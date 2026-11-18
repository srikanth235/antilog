use std::fmt;

use antilog_types::{AccessToken, MemberId, OrganizationId, SignInId, SignInSession, TokenId};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::store::{OrganizationStore, SignInRecord, StoreError, TokenRecord};

const PREFIX: &str = "alt";
const SESSION_PREFIX: &str = "als";

/// A sign-in session ends this long after it starts: twelve hours.
pub const SESSION_TTL_MS: i64 = 12 * 60 * 60 * 1000;

/// A new token: store the record, show the secret once, then forget it.
pub struct IssuedToken {
    pub record: TokenRecord,
    pub secret: String,
}

impl fmt::Debug for IssuedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IssuedToken")
            .field("record", &self.record)
            .field("secret", &"[redacted]")
            .finish()
    }
}

fn hash(secret: &str) -> [u8; 32] {
    Sha256::digest(secret.as_bytes()).into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The system's random source failed; nothing is issued without it.
#[derive(Debug, thiserror::Error)]
#[error("the system random source is unavailable")]
pub struct RandomUnavailable;

/// 32 bytes from the operating system's random source.
pub fn entropy() -> Result<[u8; 32], RandomUnavailable> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| RandomUnavailable)?;
    Ok(bytes)
}

pub fn issue(
    organization_id: OrganizationId,
    member_id: MemberId,
    name: impl Into<String>,
    created_at: i64,
) -> Result<IssuedToken, RandomUnavailable> {
    Ok(issue_with(
        entropy()?,
        organization_id,
        member_id,
        name,
        created_at,
    ))
}

pub fn issue_with(
    entropy: [u8; 32],
    organization_id: OrganizationId,
    member_id: MemberId,
    name: impl Into<String>,
    created_at: i64,
) -> IssuedToken {
    let id = TokenId::new();
    let secret = format!("{PREFIX}_{}_{}", id.as_uuid().simple(), hex(&entropy));
    IssuedToken {
        record: TokenRecord {
            token: AccessToken {
                id,
                organization_id,
                member_id,
                name: name.into(),
                created_at,
                revoked_at: None,
            },
            hash: hash(&secret),
        },
        secret,
    }
}

fn parse(prefix: &str, secret: &str) -> Option<Uuid> {
    let mut parts = secret.split('_');
    let (found, id, _random) = (parts.next()?, parts.next()?, parts.next()?);
    if found != prefix || parts.next().is_some() {
        return None;
    }
    Uuid::parse_str(id).ok()
}

/// Which kind of secret this is, so a presented secret is looked up once.
pub fn kind(secret: &str) -> Option<SecretKind> {
    match secret.split('_').next()? {
        PREFIX => Some(SecretKind::AccessToken),
        SESSION_PREFIX => Some(SecretKind::SignIn),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    AccessToken,
    SignIn,
}

/// A new sign-in session: store the record, set the secret as a cookie, forget it.
pub struct IssuedSession {
    pub record: SignInRecord,
    pub secret: String,
}

impl fmt::Debug for IssuedSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IssuedSession")
            .field("record", &self.record)
            .field("secret", &"[redacted]")
            .finish()
    }
}

pub fn issue_session(
    organization_id: OrganizationId,
    member_id: MemberId,
    now: i64,
) -> Result<IssuedSession, RandomUnavailable> {
    Ok(issue_session_with(
        entropy()?,
        organization_id,
        member_id,
        now,
    ))
}

pub fn issue_session_with(
    entropy: [u8; 32],
    organization_id: OrganizationId,
    member_id: MemberId,
    now: i64,
) -> IssuedSession {
    let id = SignInId::new();
    let secret = format!(
        "{SESSION_PREFIX}_{}_{}",
        id.as_uuid().simple(),
        hex(&entropy)
    );
    IssuedSession {
        record: SignInRecord {
            session: SignInSession {
                id,
                organization_id,
                member_id,
                created_at: now,
                expires_at: now.saturating_add(SESSION_TTL_MS),
                ended_at: None,
            },
            hash: hash(&secret),
        },
        secret,
    }
}

/// The session a presented secret belongs to, if it is genuine, not ended and not expired.
pub async fn authenticate_session<S: OrganizationStore>(
    store: &S,
    secret: &str,
    now: i64,
) -> Result<Option<SignInSession>, StoreError> {
    let Some(id) = parse(SESSION_PREFIX, secret).map(SignInId::from_uuid) else {
        return Ok(None);
    };
    let Some(record) = store.sign_in_session(id).await? else {
        return Ok(None);
    };
    let genuine: bool = record.hash.ct_eq(&hash(secret)).into();
    let live = record.session.ended_at.is_none() && now < record.session.expires_at;
    Ok((genuine && live).then_some(record.session))
}

/// The token a presented secret belongs to, if it is genuine and not revoked.
pub async fn authenticate<S: OrganizationStore>(
    store: &S,
    secret: &str,
) -> Result<Option<AccessToken>, StoreError> {
    let Some(id) = parse(PREFIX, secret).map(TokenId::from_uuid) else {
        return Ok(None);
    };
    let Some(record) = store.access_token(id).await? else {
        return Ok(None);
    };
    let genuine: bool = record.hash.ct_eq(&hash(secret)).into();
    Ok((genuine && record.token.revoked_at.is_none()).then_some(record.token))
}
