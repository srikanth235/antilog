use antilog_core::clock::SystemClock;
use antilog_core::deps::Deps;

use crate::pg::PgStore;

/// What the server runs on: Postgres for the ledger, the runner queue and session credentials
/// (so a token minted at claim verifies at the proxy and the MCP endpoint).
pub struct ServerDeps {
    pub store: PgStore,
}

impl ServerDeps {
    pub fn new(store: PgStore) -> Self {
        Self { store }
    }
}

impl Deps for ServerDeps {
    type Store = PgStore;
    type Clock = SystemClock;
    type Credentials = PgStore;

    fn store(&self) -> &PgStore {
        &self.store
    }
    fn clock(&self) -> &SystemClock {
        &SystemClock
    }
    fn credentials(&self) -> &PgStore {
        &self.store
    }
}
