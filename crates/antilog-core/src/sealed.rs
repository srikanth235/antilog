//! Values stored encrypted in Postgres: customer model keys and connector credentials.
//! XChaCha20-Poly1305 with a random 24-byte nonce for each value. The master key comes from the
//! `Secrets` store, never from the database. The associated data binds a value to its row, so a
//! value copied into another row does not open.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};

use crate::secrets::Secret;

const NONCE: usize = 24;

#[derive(Debug, thiserror::Error)]
pub enum SealedError {
    #[error("the master key must be 32 bytes, hex encoded")]
    BadKey,
    #[error("the system random source is unavailable")]
    Random,
    #[error("the value does not open with this key and row")]
    Open,
}

pub struct Sealer(XChaCha20Poly1305);

impl Sealer {
    /// `master` is 64 hex characters.
    pub fn new(master: &Secret) -> Result<Self, SealedError> {
        let hex = master.expose().trim();
        if hex.len() != 64 {
            return Err(SealedError::BadKey);
        }
        let mut key = [0u8; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte =
                u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| SealedError::BadKey)?;
        }
        XChaCha20Poly1305::new_from_slice(&key)
            .map(Self)
            .map_err(|_| SealedError::BadKey)
    }

    /// Nonce followed by ciphertext. `row` names where the value lives, such as "model_key:<org>:anthropic".
    pub fn seal(&self, plain: &str, row: &str) -> Result<Vec<u8>, SealedError> {
        let mut nonce = [0u8; NONCE];
        getrandom::fill(&mut nonce).map_err(|_| SealedError::Random)?;
        let sealed = self
            .0
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: plain.as_bytes(),
                    aad: row.as_bytes(),
                },
            )
            .map_err(|_| SealedError::Open)?;
        Ok([nonce.as_slice(), &sealed].concat())
    }

    pub fn open(&self, stored: &[u8], row: &str) -> Result<Secret, SealedError> {
        if stored.len() <= NONCE {
            return Err(SealedError::Open);
        }
        let (nonce, sealed) = stored.split_at(NONCE);
        let nonce: [u8; NONCE] = nonce.try_into().map_err(|_| SealedError::Open)?;
        let plain = self
            .0
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: sealed,
                    aad: row.as_bytes(),
                },
            )
            .map_err(|_| SealedError::Open)?;
        String::from_utf8(plain)
            .map(Secret::new)
            .map_err(|_| SealedError::Open)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sealer(fill: char) -> Sealer {
        Sealer::new(&Secret::new(fill.to_string().repeat(64))).unwrap()
    }

    #[test]
    fn a_value_opens_only_with_its_key_and_row() {
        let s = sealer('a');
        let stored = s.seal("sk-live", "model_key:org1:anthropic").unwrap();
        assert!(!stored.windows(7).any(|w| w == b"sk-live"));
        assert_eq!(
            s.open(&stored, "model_key:org1:anthropic")
                .unwrap()
                .expose(),
            "sk-live"
        );
        assert!(s.open(&stored, "model_key:org2:anthropic").is_err());
        assert!(
            sealer('b')
                .open(&stored, "model_key:org1:anthropic")
                .is_err()
        );
        assert_ne!(
            stored,
            s.seal("sk-live", "model_key:org1:anthropic").unwrap()
        );
    }

    #[test]
    fn a_bad_master_key_is_refused() {
        assert!(Sealer::new(&Secret::new("short")).is_err());
        assert!(Sealer::new(&Secret::new("z".repeat(64))).is_err());
    }
}
