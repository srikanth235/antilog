use std::fmt;

use antilog_types::{AccessToken, MemberId, OrganizationId, TokenId};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::store::{OrganizationStore, StoreError, TokenRecord};

const PREFIX: &str = "alt";

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

/// 32 random bytes: two version-4 UUIDs, 244 random bits and no new dependency.
fn entropy() -> [u8; 32] {
    let mut bytes = [0; 32];
    for (half, uuid) in bytes.chunks_mut(16).zip([Uuid::new_v4(), Uuid::new_v4()]) {
        half.copy_from_slice(uuid.as_bytes());
    }
    bytes
}

pub fn issue(
    organization_id: OrganizationId,
    member_id: MemberId,
    name: impl Into<String>,
    created_at: i64,
) -> IssuedToken {
    issue_with(entropy(), organization_id, member_id, name, created_at)
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

fn parse(secret: &str) -> Option<TokenId> {
    let mut parts = secret.split('_');
    let (prefix, id, _random) = (parts.next()?, parts.next()?, parts.next()?);
    if prefix != PREFIX || parts.next().is_some() {
        return None;
    }
    Uuid::parse_str(id).ok().map(TokenId::from_uuid)
}

/// The token a presented secret belongs to, if it is genuine and not revoked.
pub async fn authenticate<S: OrganizationStore>(
    store: &S,
    secret: &str,
) -> Result<Option<AccessToken>, StoreError> {
    let Some(id) = parse(secret) else {
        return Ok(None);
    };
    let Some(record) = store.access_token(id).await? else {
        return Ok(None);
    };
    let genuine: bool = record.hash.ct_eq(&hash(secret)).into();
    Ok((genuine && record.token.revoked_at.is_none()).then_some(record.token))
}
