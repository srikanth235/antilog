use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A stable id from a seed, so a repeated wake-up or tool call writes the same row.
pub fn uuid_of(seed: &str) -> Uuid {
    let digest = Sha256::digest(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(digest.get(..16).unwrap_or(&[0; 16]));
    Uuid::from_bytes(bytes)
}
