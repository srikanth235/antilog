use antilog_types::OrganizationId;

use crate::clock::Clock;
use crate::code_host::CodeHost;
use crate::machine::SessionMachine;
use crate::signer::Signer;
use crate::store::Store;
use crate::store::judge::JudgeStore;

/// Everything the session area talks to. A server holds one; a test holds the fakes.
pub trait Deps: Send + Sync {
    type Store: Store;
    type Judge: JudgeStore;
    type Machine: SessionMachine;
    type Signer: Signer;
    type Clock: Clock;
    type Code: CodeHost;

    fn store(&self) -> &Self::Store;
    /// The judge's store, scoped to one organization.
    fn judge(&self, organization: OrganizationId) -> Self::Judge;
    fn machine(&self) -> &Self::Machine;
    fn signer(&self) -> &Self::Signer;
    fn clock(&self) -> &Self::Clock;
    fn code(&self) -> &Self::Code;
}
