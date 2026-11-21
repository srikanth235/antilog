use crate::clock::Clock;
use crate::credentials::SessionCredentials;
use crate::store::Store;

/// Everything the session area talks to. A server holds one; a test holds the fakes.
pub trait Deps: Send + Sync {
    type Store: Store;
    type Clock: Clock;
    type Credentials: SessionCredentials;

    fn store(&self) -> &Self::Store;
    fn clock(&self) -> &Self::Clock;
    fn credentials(&self) -> &Self::Credentials;
}
