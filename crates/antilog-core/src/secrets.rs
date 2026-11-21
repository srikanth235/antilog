use std::fmt;
use std::future::Future;

#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("secret {0} not found")]
    NotFound(String),
    #[error("secrets unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait Secrets: Send + Sync {
    fn get(&self, name: &str) -> impl Future<Output = Result<Secret, SecretsError>> + Send;
}

/// Secrets from the process environment or from mounted files, nothing else.
/// Antilog cloud puts them in the function's environment at deploy time and the
/// chart mounts them as files, so no cloud's secret manager is called at request time.
///
/// A secret named `database-url` is read from the variable `ANTILOG_SECRET_DATABASE_URL`,
/// then from the file `database-url` in the directory named by `ANTILOG_SECRETS_DIR`.
type Lookup = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;

pub struct EnvSecrets {
    lookup: Lookup,
}

impl EnvSecrets {
    /// Reads the real environment.
    pub fn from_env() -> Self {
        Self::with_lookup(|name| std::env::var(name).ok())
    }

    /// Reads variables through `lookup`; a test passes a map.
    pub fn with_lookup(lookup: impl Fn(&str) -> Option<String> + Send + Sync + 'static) -> Self {
        Self {
            lookup: Box::new(lookup),
        }
    }

    /// `database-url` becomes `ANTILOG_SECRET_DATABASE_URL`.
    pub fn variable(name: &str) -> String {
        let upper: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect();
        format!("ANTILOG_SECRET_{upper}")
    }

    fn valid(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
            && !name.starts_with('.')
    }
}

impl Secrets for EnvSecrets {
    async fn get(&self, name: &str) -> Result<Secret, SecretsError> {
        if !Self::valid(name) {
            return Err(SecretsError::NotFound(name.to_owned()));
        }
        if let Some(value) = (self.lookup)(&Self::variable(name)).filter(|v| !v.is_empty()) {
            return Ok(Secret::new(value));
        }
        if let Some(dir) = (self.lookup)("ANTILOG_SECRETS_DIR").filter(|d| !d.is_empty()) {
            let path = std::path::Path::new(&dir).join(name);
            return match std::fs::read_to_string(&path) {
                // A mounted file usually ends in a newline the value does not have.
                Ok(text) => Ok(Secret::new(text.trim_end_matches(['\n', '\r']))),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Err(SecretsError::NotFound(name.to_owned()))
                }
                Err(error) => Err(SecretsError::Unavailable(Box::new(error))),
            };
        }
        Err(SecretsError::NotFound(name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn secrets(vars: &[(&str, &str)]) -> EnvSecrets {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        EnvSecrets::with_lookup(move |name| map.get(name).cloned())
    }

    #[test]
    fn variable_names_are_upper_snake() {
        assert_eq!(
            EnvSecrets::variable("database-url"),
            "ANTILOG_SECRET_DATABASE_URL"
        );
    }

    #[tokio::test]
    async fn reads_a_variable() {
        let s = secrets(&[("ANTILOG_SECRET_DATABASE_URL", "postgres://x")]);
        let got = s.get("database-url").await.map(|v| v.expose().to_owned());
        assert_eq!(got.ok().as_deref(), Some("postgres://x"));
    }

    #[tokio::test]
    async fn missing_is_not_found_and_never_shows_a_value() {
        let s = secrets(&[]);
        assert!(matches!(
            s.get("database-url").await,
            Err(SecretsError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn reads_a_mounted_file_without_its_newline() {
        let dir = std::env::temp_dir().join(format!("antilog-secrets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        std::fs::write(dir.join("receipt-key"), "abc\n").ok();
        let s = secrets(&[("ANTILOG_SECRETS_DIR", dir.to_str().unwrap_or_default())]);
        let got = s.get("receipt-key").await.map(|v| v.expose().to_owned());
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(got.ok().as_deref(), Some("abc"));
    }

    #[tokio::test]
    async fn a_path_is_not_a_name() {
        let s = secrets(&[("ANTILOG_SECRETS_DIR", "/etc")]);
        assert!(matches!(
            s.get("../passwd").await,
            Err(SecretsError::NotFound(_))
        ));
    }
}
