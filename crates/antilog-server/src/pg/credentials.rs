use antilog_core::credentials::{CredentialsError, Purpose, SessionCredentials};
use antilog_core::token::entropy;
use antilog_types::SessionId;
use jiff::SignedDuration;
use sha2::{Digest, Sha256};

use super::PgStore;

fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

const fn purpose(purpose: Purpose) -> &'static str {
    match purpose {
        Purpose::Model => "model",
        Purpose::Mcp => "mcp",
        Purpose::Runner => "runner",
    }
}

fn unavailable(error: sqlx::Error) -> CredentialsError {
    CredentialsError::Unavailable(Box::new(error))
}

impl SessionCredentials for PgStore {
    async fn mint(
        &self,
        session: SessionId,
        purpose_of: Purpose,
        ttl: SignedDuration,
    ) -> Result<String, CredentialsError> {
        let token = format!(
            "alc_{}",
            entropy()?
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        sqlx::query!(
            "select mint_session_credential($1, $2, $3, $4)",
            hash(&token),
            session.as_uuid(),
            purpose(purpose_of),
            ttl.as_secs_f64(),
        )
        .execute(&self.pool)
        .await
        .map_err(unavailable)?;
        Ok(token)
    }

    async fn verify(
        &self,
        token: &str,
        purpose_of: Purpose,
    ) -> Result<Option<SessionId>, CredentialsError> {
        let found = sqlx::query_scalar!(
            "select verify_session_credential($1, $2)",
            hash(token),
            purpose(purpose_of),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(unavailable)?;
        Ok(found.map(SessionId::from_uuid))
    }

    async fn revoke_all(&self, session: SessionId) -> Result<(), CredentialsError> {
        sqlx::query!("select revoke_session_credentials($1)", session.as_uuid())
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(unavailable)
    }
}
