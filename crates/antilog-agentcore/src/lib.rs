#![forbid(unsafe_code)]

//! The agent platform adapter for AWS AgentCore: agent sessions and judge sandboxes
//! as runtime sessions on two Antilog runtimes. The server relays; the loop runs in the
//! runtime. The wire contract with the images is in [`protocol`].

mod config;
mod control;
pub mod protocol;
mod runtime;

use std::time::Duration;

use antilog_core::platform::{
    Data, PairedRun, Pairs, Platform, PlatformError, PlatformEvent, SessionEvents, SessionHandle,
    SessionSpec,
};
use serde_json::{Value, json};
use uuid::Uuid;

pub use config::{Config, DEFAULT_FALLBACK_MODEL, DEFAULT_MODEL, Missing};
pub use control::resolve_runtime_arn;
pub use runtime::{Runtime, RuntimeError, SdkRuntime};

use protocol::{Answer, Request};

/// AgentCore wants a runtime session id of at least 33 characters.
const SESSION_PREFIX: &str = "antilog-session-";
const JUDGE_PREFIX: &str = "antilog-judge-";

pub struct AgentCore<R> {
    runtime: R,
    agent_runtime: String,
    judge_runtime: String,
    model: String,
    fallback_model: Option<String>,
}

impl<R: Runtime> AgentCore<R> {
    /// `agent_runtime` and `judge_runtime` are ARNs.
    pub fn new(runtime: R, config: &Config, agent_runtime: String, judge_runtime: String) -> Self {
        Self {
            runtime,
            agent_runtime,
            judge_runtime,
            model: config.model.clone(),
            fallback_model: config.fallback_model.clone(),
        }
    }

    async fn call(
        &self,
        runtime: &str,
        session: &str,
        request: &Request<'_>,
    ) -> Result<Answer, PlatformError> {
        let body = serde_json::to_vec(request)
            .map_err(|e| PlatformError::Failed(format!("the request does not serialize: {e}")))?;
        let bytes = self
            .runtime
            .invoke(runtime, session, body)
            .await
            .map_err(platform_error)?;
        let answer: Answer = serde_json::from_slice(&bytes).map_err(|e| {
            PlatformError::Failed(format!("the runtime answered something unreadable: {e}"))
        })?;
        match answer {
            Answer::Refused { error, .. } if error == "unknown_session" => {
                Err(PlatformError::UnknownSession)
            }
            Answer::Refused { error, .. } => Err(PlatformError::Failed(error)),
            other => Ok(other),
        }
    }

    async fn done(
        &self,
        session: &SessionHandle,
        request: &Request<'_>,
    ) -> Result<(), PlatformError> {
        match self.call(&self.agent_runtime, &session.0, request).await? {
            Answer::Done { .. } => Ok(()),
            _ => Err(PlatformError::Failed(
                "the runtime answered a different question".to_owned(),
            )),
        }
    }
}

fn platform_error(error: RuntimeError) -> PlatformError {
    match error {
        RuntimeError::NotFound => PlatformError::UnknownSession,
        RuntimeError::Unavailable(source) => PlatformError::Unavailable(source),
        RuntimeError::Refused(reason) => PlatformError::Failed(reason),
    }
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}{}", Uuid::new_v4().simple())
}

impl<R: Runtime> Platform for AgentCore<R> {
    fn name(&self) -> &str {
        "agentcore"
    }

    async fn start_session(&self, spec: &SessionSpec) -> Result<SessionHandle, PlatformError> {
        let handle = SessionHandle(new_id(SESSION_PREFIX));
        let request = Request::Start {
            agent: &spec.agent,
            prompt: &spec.prompt,
            repos: &spec.repos,
            environment: spec.environment.as_deref(),
            tools: spec.tools.iter().map(Into::into).collect(),
            model: &self.model,
            fallback_model: self.fallback_model.as_deref(),
        };
        self.done(&handle, &request).await?;
        Ok(handle)
    }

    async fn send_input(&self, session: &SessionHandle, text: &str) -> Result<(), PlatformError> {
        self.done(session, &Request::Input { text }).await
    }

    async fn session_events(
        &self,
        session: &SessionHandle,
        cursor: Option<&str>,
        wait: Duration,
    ) -> Result<SessionEvents, PlatformError> {
        let wait_ms = u64::try_from(wait.as_millis()).unwrap_or(u64::MAX);
        let request = Request::Events { cursor, wait_ms };
        match self.call(&self.agent_runtime, &session.0, &request).await? {
            Answer::Events(answer) => Ok(SessionEvents {
                events: answer.events.into_iter().map(PlatformEvent::from).collect(),
                // A runtime with nothing new keeps the cursor it was given.
                next_cursor: answer.next_cursor.or_else(|| cursor.map(str::to_owned)),
            }),
            _ => Err(PlatformError::Failed(
                "the runtime answered a different question".to_owned(),
            )),
        }
    }

    async fn answer_tool(
        &self,
        session: &SessionHandle,
        call: &str,
        result: &Value,
    ) -> Result<(), PlatformError> {
        self.done(session, &Request::Answer { call, result }).await
    }

    async fn stop(&self, session: &SessionHandle) -> Result<(), PlatformError> {
        // Tell the agent first so its last events (Ended) are readable, then end the microVM
        // only on delete: the log lives in the microVM.
        self.done(session, &Request::Stop).await
    }

    async fn delete(&self, session: &SessionHandle) -> Result<(), PlatformError> {
        self.runtime
            .stop(&self.agent_runtime, &session.0)
            .await
            .map_err(platform_error)
    }

    async fn run_paired(&self, run: &PairedRun<'_>) -> Result<Pairs, PlatformError> {
        // A fresh runtime session is a fresh microVM: nothing from an earlier run is there.
        let session = new_id(JUDGE_PREFIX);
        let (kind, data) = match run.data {
            Data::Evaluation(data) => ("evaluation", data),
            Data::Holdout(data) => ("holdout", data),
        };
        let payload = json!({
            "seal": run.seal,
            "command": run.command,
            "files": run.files,
            "data": {"kind": kind, "dataset": data},
            "base": run.base,
            "trial": run.trial,
            "repeats": run.repeats,
            "guardrails": run.guardrails,
        });
        let result = self
            .call(&self.judge_runtime, &session, &Request::Paired(&payload))
            .await;
        // The sandbox ends with the run, whatever came back.
        if let Err(error) = self.runtime.stop(&self.judge_runtime, &session).await {
            tracing::warn!(%error, session, "could not end a judge sandbox");
        }
        match result? {
            Answer::Pairs(answer) => Ok(Pairs {
                metric: answer.metric,
                guardrails: answer
                    .guardrails
                    .into_iter()
                    .map(|g| (g.name, g.pairs))
                    .collect(),
            }),
            _ => Err(PlatformError::Failed(
                "the judge runtime answered a different question".to_owned(),
            )),
        }
    }
}

impl AgentCore<SdkRuntime> {
    /// Real clients from the default credential chain, with runtime names resolved to ARNs.
    /// Retries are off: a repeated `input` would be sent twice, and the relay retries a step.
    pub async fn connect(config: &Config) -> Result<Self, PlatformError> {
        use aws_config::retry::RetryConfig;
        use aws_config::timeout::TimeoutConfig;

        let shared = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .retry_config(RetryConfig::standard().with_max_attempts(1))
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_attempt_timeout(config.call_timeout)
                    .build(),
            )
            .load()
            .await;
        let control = aws_sdk_bedrockagentcorecontrol::Client::new(&shared);
        let agent = resolve_runtime_arn(&control, &config.agent_runtime).await?;
        let judge = resolve_runtime_arn(&control, &config.judge_runtime).await?;
        let data = aws_sdk_bedrockagentcore::Client::new(&shared);
        Ok(Self::new(
            SdkRuntime::new(data, config.qualifier.clone()),
            config,
            agent,
            judge,
        ))
    }
}

#[cfg(test)]
mod tests;
