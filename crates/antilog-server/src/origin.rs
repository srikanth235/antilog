//! Only the front door may reach the server.
//!
//! On Antilog cloud the Lambda function URL is public, so anyone who finds it could skip
//! CloudFront. CloudFront adds a secret header to every `/api/*` request and the server
//! refuses a request without it. The rule:
//!
//! - **On Lambda the check is always on** ([`OriginGuard::required_from_env`]). With no
//!   secret set it refuses everything except the deploy's own invocations: it fails
//!   closed. There is no switch to turn it off.
//! - **The local server** ([`OriginGuard::from_env`]) checks only when a secret is set, and
//!   says so in its log when it does not. Tests build [`OriginGuard::off`] explicitly.
//!
//! Two secrets are accepted at once so one can be rotated while the other still works
//! (`ANTILOG_ORIGIN_SECRET` and `ANTILOG_ORIGIN_SECRET_NEXT`). Every comparison is
//! constant time and no secret is ever printed.

use antilog_core::secrets::Secret;
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use subtle::ConstantTimeEq;

/// The header CloudFront adds to API requests.
pub const ORIGIN_HEADER: &str = "x-antilog-origin";
const PRIMARY: &str = "ANTILOG_ORIGIN_SECRET";
const NEXT: &str = "ANTILOG_ORIGIN_SECRET_NEXT";

#[derive(Clone, Debug)]
pub struct OriginGuard {
    /// `None`: the check is off. `Some`: a request must carry one of these (none: refuse all).
    accepted: Option<Vec<Secret>>,
}

fn setting(name: &str) -> Option<Secret> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .map(Secret::new)
}

impl OriginGuard {
    /// No check. For tests and nothing else.
    pub fn off() -> Self {
        Self { accepted: None }
    }

    /// Accepts these secrets. Empty values are dropped, so an empty list refuses everyone.
    pub fn requiring(secrets: impl IntoIterator<Item = Secret>) -> Self {
        Self {
            accepted: Some(
                secrets
                    .into_iter()
                    .filter(|secret| !secret.expose().is_empty())
                    .collect(),
            ),
        }
    }

    /// Lambda: always on, whatever the environment holds.
    pub fn required_from_env() -> Self {
        Self::requiring([setting(PRIMARY), setting(NEXT)].into_iter().flatten())
    }

    /// The local server: on when a secret is set, off otherwise.
    pub fn from_env() -> Self {
        let guard = Self::required_from_env();
        if guard.accepted.as_deref().is_some_and(<[Secret]>::is_empty) {
            tracing::warn!("{PRIMARY} is not set: any caller may reach this server");
            return Self::off();
        }
        guard
    }

    pub fn is_on(&self) -> bool {
        self.accepted.is_some()
    }

    /// Whether the request's headers carry an accepted secret (always true when off).
    /// Checks every accepted secret, with no early exit.
    pub fn allows(&self, headers: &HeaderMap) -> bool {
        let Some(accepted) = &self.accepted else {
            return true;
        };
        let given = headers
            .get(ORIGIN_HEADER)
            .map_or(&[][..], |value| value.as_bytes());
        let mut ok = subtle::Choice::from(0);
        for secret in accepted {
            ok |= given.ct_eq(secret.expose().as_bytes());
        }
        bool::from(ok)
    }

    /// The refusal for a request that does not carry it.
    pub fn refusal() -> Response {
        (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"code":"forbidden","message":"Not available.","fields":[]}"#,
        )
            .into_response()
    }

    /// A middleware for a plain server (not Lambda, whose entry checks before the router).
    pub async fn layer(self, request: Request, next: Next) -> Response {
        if self.allows(request.headers()) {
            next.run(request).await
        } else {
            Self::refusal()
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn with(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN_HEADER, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn off_lets_everything_through() {
        assert!(OriginGuard::off().allows(&HeaderMap::new()));
    }

    #[test]
    fn a_missing_or_wrong_header_is_refused() {
        let guard = OriginGuard::requiring([Secret::new("one")]);
        assert!(!guard.allows(&HeaderMap::new()));
        assert!(!guard.allows(&with("two")));
        assert!(!guard.allows(&with("")));
        assert!(!guard.allows(&with("one-and-more")));
        assert!(guard.allows(&with("one")));
    }

    #[test]
    fn two_secrets_are_accepted_for_a_rotation() {
        let guard = OriginGuard::requiring([Secret::new("old"), Secret::new("new")]);
        assert!(guard.allows(&with("old")));
        assert!(guard.allows(&with("new")));
        assert!(!guard.allows(&with("older")));
    }

    #[test]
    fn no_secret_set_refuses_everyone_instead_of_opening() {
        let guard = OriginGuard::requiring([]);
        assert!(guard.is_on());
        assert!(!guard.allows(&HeaderMap::new()));
        assert!(!guard.allows(&with("")));
        let blank = OriginGuard::requiring([Secret::new("")]);
        assert!(!blank.allows(&with("")));
        assert!(!blank.allows(&HeaderMap::new()));
    }
}
