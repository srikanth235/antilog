use crate::clock::Clock;
use crate::code_host::CodeHost;
use crate::platform::Platform;
use crate::signer::Signer;
use crate::store::Store;
use crate::store::judge::JudgeStore;
use crate::timer::Timer;

/// Everything the session area talks to. A server holds one; a test holds the fakes.
pub trait Deps: Send + Sync {
    type Store: Store;
    type Judge: JudgeStore;
    type Platform: Platform;
    type Signer: Signer;
    type Clock: Clock;
    type Timer: Timer;
    type Code: CodeHost;

    fn store(&self) -> &Self::Store;
    fn judge(&self) -> &Self::Judge;
    fn platform(&self) -> &Self::Platform;
    fn signer(&self) -> &Self::Signer;
    fn clock(&self) -> &Self::Clock;
    fn timer(&self) -> &Self::Timer;
    fn code(&self) -> &Self::Code;
}
