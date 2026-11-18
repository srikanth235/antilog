#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub Vec<u8>);

pub trait Signer: Send + Sync {
    fn key_id(&self) -> &str;

    fn sign(&self, message: &[u8]) -> Signature;

    fn verify(&self, message: &[u8], signature: &Signature) -> bool;
}
