use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use antilog_types::SessionId;
use jiff::{SignedDuration, Timestamp};
use sha2::{Digest, Sha256};

use super::locked;
use crate::clock::Clock;
use crate::credentials::{CredentialsError, Purpose, SessionCredentials};
use crate::token::entropy;

struct Entry {
    session: SessionId,
    purpose: Purpose,
    expires: Timestamp,
}

/// In-memory `SessionCredentials`, keyed by the token's hash as the real store is.
pub struct FakeCredentials {
    clock: Arc<dyn Clock>,
    entries: Mutex<HashMap<[u8; 32], Entry>>,
}

impl FakeCredentials {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            entries: Mutex::default(),
        }
    }
}

fn hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

impl SessionCredentials for FakeCredentials {
    async fn mint(
        &self,
        session: SessionId,
        purpose: Purpose,
        ttl: SignedDuration,
    ) -> Result<String, CredentialsError> {
        let token = format!(
            "alc_{}",
            entropy()?
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let expires = self.clock.now().checked_add(ttl).unwrap_or(Timestamp::MAX);
        locked(&self.entries).insert(
            hash(&token),
            Entry {
                session,
                purpose,
                expires,
            },
        );
        Ok(token)
    }

    async fn verify(
        &self,
        token: &str,
        purpose: Purpose,
    ) -> Result<Option<SessionId>, CredentialsError> {
        let now = self.clock.now();
        Ok(locked(&self.entries)
            .get(&hash(token))
            .filter(|e| e.purpose == purpose && e.expires > now)
            .map(|e| e.session))
    }

    async fn revoke_all(&self, session: SessionId) -> Result<(), CredentialsError> {
        locked(&self.entries).retain(|_, e| e.session != session);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::FakeClock;

    #[tokio::test]
    async fn a_token_works_for_its_purpose_until_it_expires_or_is_revoked() {
        let clock = Arc::new(FakeClock::new(Timestamp::UNIX_EPOCH));
        let creds = FakeCredentials::new(clock.clone());
        let session = SessionId::new();
        let token = creds
            .mint(session, Purpose::Model, SignedDuration::from_mins(10))
            .await
            .unwrap();

        assert_eq!(
            creds.verify(&token, Purpose::Model).await.unwrap(),
            Some(session)
        );
        assert_eq!(creds.verify(&token, Purpose::Mcp).await.unwrap(), None);
        assert_eq!(
            creds.verify("alc_wrong", Purpose::Model).await.unwrap(),
            None
        );

        clock.advance(SignedDuration::from_mins(11));
        assert_eq!(creds.verify(&token, Purpose::Model).await.unwrap(), None);

        let fresh = creds
            .mint(session, Purpose::Mcp, SignedDuration::from_mins(10))
            .await
            .unwrap();
        creds.revoke_all(session).await.unwrap();
        assert_eq!(creds.verify(&fresh, Purpose::Mcp).await.unwrap(), None);
    }
}
