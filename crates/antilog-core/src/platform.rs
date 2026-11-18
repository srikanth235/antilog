use std::future::Future;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSpec {
    pub agent: String,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionHandle(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEvents {
    pub events: Vec<String>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeSpec {
    pub sealed_command: String,
    pub repeats: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JudgeRun {
    pub samples: Vec<f64>,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("unknown session")]
    UnknownSession,
    #[error("platform unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait Platform: Send + Sync {
    fn start_session(
        &self,
        spec: &SessionSpec,
    ) -> impl Future<Output = Result<SessionHandle, PlatformError>> + Send;

    fn session_events(
        &self,
        session: &SessionHandle,
        cursor: Option<&str>,
    ) -> impl Future<Output = Result<SessionEvents, PlatformError>> + Send;

    fn run_judge(
        &self,
        spec: &JudgeSpec,
    ) -> impl Future<Output = Result<JudgeRun, PlatformError>> + Send;
}
