use antilog_types::judge::hex;
use ed25519_dalek::{Signature as Ed, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::secrets::{Secrets, SecretsError};
use crate::signer::{Signature, Signer};

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error(transparent)]
    Secrets(#[from] SecretsError),
    #[error("the signing key is not 64 hex characters")]
    Malformed,
}

/// The judge's receipt signer. The key is a 32-byte seed held as 64 hex characters in `Secrets`.
pub struct Ed25519Signer {
    key: SigningKey,
    key_id: String,
}

impl Ed25519Signer {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let key = SigningKey::from_bytes(&seed);
        let digest = Sha256::digest(key.verifying_key().as_bytes());
        let key_id = hex(digest.get(..8).unwrap_or_default());
        Self { key, key_id }
    }

    pub async fn from_secrets(secrets: &impl Secrets, name: &str) -> Result<Self, KeyError> {
        let secret = secrets.get(name).await?;
        let text = secret.expose().trim();
        let bytes: Option<Vec<u8>> = (text.len() == 64)
            .then(|| {
                (0..32)
                    .map(|i| {
                        text.get(2 * i..2 * i + 2)
                            .and_then(|b| u8::from_str_radix(b, 16).ok())
                    })
                    .collect()
            })
            .flatten();
        let seed = bytes
            .and_then(|b| <[u8; 32]>::try_from(b).ok())
            .ok_or(KeyError::Malformed)?;
        Ok(Self::from_seed(seed))
    }

    pub fn public_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }
}

impl Signer for Ed25519Signer {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.key.sign(message).to_bytes().to_vec())
    }

    fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        Ed::from_slice(&signature.0)
            .is_ok_and(|sig| self.key.verifying_key().verify(message, &sig).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::FakeSecrets;

    #[test]
    fn signs_and_rejects_a_changed_message_a_short_signature_and_a_foreign_key() {
        let signer = Ed25519Signer::from_seed([7; 32]);
        let signature = signer.sign(b"receipt");
        assert_eq!(signature.0.len(), 64);
        assert!(signer.verify(b"receipt", &signature));
        assert!(!signer.verify(b"receipt!", &signature));
        assert!(!signer.verify(b"receipt", &Signature(vec![1, 2, 3])));
        let other = Ed25519Signer::from_seed([8; 32]);
        assert!(!other.verify(b"receipt", &signature));
        assert_ne!(signer.key_id(), other.key_id());
        assert_eq!(signer.key_id().len(), 16);
    }

    #[tokio::test]
    async fn the_key_comes_from_secrets_and_a_bad_one_is_refused() {
        let secrets = FakeSecrets::default()
            .with("judge-key", &"07".repeat(32))
            .with("short", "abcd")
            .with("nothex", &"zz".repeat(32));
        let signer = Ed25519Signer::from_secrets(&secrets, "judge-key")
            .await
            .unwrap();
        assert_eq!(signer.key_id(), Ed25519Signer::from_seed([7; 32]).key_id());
        assert!(matches!(
            Ed25519Signer::from_secrets(&secrets, "short").await,
            Err(KeyError::Malformed)
        ));
        assert!(matches!(
            Ed25519Signer::from_secrets(&secrets, "nothex").await,
            Err(KeyError::Malformed)
        ));
        assert!(matches!(
            Ed25519Signer::from_secrets(&secrets, "none").await,
            Err(KeyError::Secrets(_))
        ));
    }
}
