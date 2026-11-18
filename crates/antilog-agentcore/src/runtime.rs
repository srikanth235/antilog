//! The one call the adapter makes to AgentCore, behind a trait so a test can stand in.

use std::future::Future;

use aws_sdk_bedrockagentcore::error::SdkError;
use aws_sdk_bedrockagentcore::operation::invoke_agent_runtime::InvokeAgentRuntimeError;
use aws_sdk_bedrockagentcore::operation::stop_runtime_session::StopRuntimeSessionError;
use aws_sdk_bedrockagentcore::primitives::Blob;

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("no such runtime session")]
    NotFound,
    /// Throttled, a server error, a timeout or no route: worth trying again later.
    #[error("runtime unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// Refused for a reason retrying does not change (access, quota, a bad request).
    #[error("runtime refused the call: {0}")]
    Refused(String),
}

/// AgentCore Runtime's data plane, reduced to what the adapter uses.
pub trait Runtime: Send + Sync {
    /// `POST /invocations` on one runtime session; the same session id is the same microVM.
    fn invoke(
        &self,
        runtime: &str,
        session: &str,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<Vec<u8>, RuntimeError>> + Send;

    /// Ends the session's microVM. A session that is already gone is not an error.
    fn stop(
        &self,
        runtime: &str,
        session: &str,
    ) -> impl Future<Output = Result<(), RuntimeError>> + Send;
}

/// The real client.
pub struct SdkRuntime {
    client: aws_sdk_bedrockagentcore::Client,
    qualifier: Option<String>,
}

impl SdkRuntime {
    pub fn new(client: aws_sdk_bedrockagentcore::Client, qualifier: Option<String>) -> Self {
        Self { client, qualifier }
    }
}

impl Runtime for SdkRuntime {
    async fn invoke(
        &self,
        runtime: &str,
        session: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, RuntimeError> {
        let mut call = self
            .client
            .invoke_agent_runtime()
            .agent_runtime_arn(runtime)
            .runtime_session_id(session)
            .content_type("application/json")
            .accept("application/json")
            .payload(Blob::new(body));
        if let Some(qualifier) = &self.qualifier {
            call = call.qualifier(qualifier);
        }
        let output = call.send().await.map_err(invoke_error)?;
        let bytes = output
            .response
            .collect()
            .await
            .map_err(|e| RuntimeError::Unavailable(Box::new(e)))?;
        Ok(bytes.into_bytes().to_vec())
    }

    async fn stop(&self, runtime: &str, session: &str) -> Result<(), RuntimeError> {
        let mut call = self
            .client
            .stop_runtime_session()
            .agent_runtime_arn(runtime)
            .runtime_session_id(session);
        if let Some(qualifier) = &self.qualifier {
            call = call.qualifier(qualifier);
        }
        match call.send().await {
            Ok(_) => Ok(()),
            Err(error) => match stop_error(error) {
                RuntimeError::NotFound => Ok(()),
                other => Err(other),
            },
        }
    }
}

fn invoke_error(error: SdkError<InvokeAgentRuntimeError>) -> RuntimeError {
    use InvokeAgentRuntimeError as E;
    match error {
        SdkError::ServiceError(context) => match context.err() {
            E::ResourceNotFoundException(_) => RuntimeError::NotFound,
            E::AccessDeniedException(e) => RuntimeError::Refused(format!("access denied: {e}")),
            E::ValidationException(e) => RuntimeError::Refused(format!("invalid request: {e}")),
            E::ServiceQuotaExceededException(e) => {
                RuntimeError::Refused(format!("quota exceeded: {e}"))
            }
            other => RuntimeError::Unavailable(other.to_string().into()),
        },
        other => RuntimeError::Unavailable(Box::new(other)),
    }
}

fn stop_error(error: SdkError<StopRuntimeSessionError>) -> RuntimeError {
    use StopRuntimeSessionError as E;
    match error {
        SdkError::ServiceError(context) => match context.err() {
            E::ResourceNotFoundException(_) => RuntimeError::NotFound,
            E::AccessDeniedException(e) => RuntimeError::Refused(format!("access denied: {e}")),
            E::UnauthorizedException(e) => RuntimeError::Refused(format!("unauthorized: {e}")),
            E::ValidationException(e) => RuntimeError::Refused(format!("invalid request: {e}")),
            other => RuntimeError::Unavailable(other.to_string().into()),
        },
        other => RuntimeError::Unavailable(Box::new(other)),
    }
}
