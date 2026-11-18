use std::future::Future;

use jiff::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub u128);

#[derive(Debug, thiserror::Error)]
pub enum TimerError {
    #[error("timer unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait Timer: Send + Sync {
    fn wake_at(
        &self,
        job: JobId,
        at: Timestamp,
    ) -> impl Future<Output = Result<(), TimerError>> + Send;

    fn cancel(&self, job: JobId) -> impl Future<Output = Result<(), TimerError>> + Send;
}
