use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;

use super::locked;
use crate::judge::sandbox::{Profile, Script};
use crate::platform::{
    PairedRun, Pairs, Platform, PlatformError, PlatformEvent, SessionEvents, SessionHandle,
    SessionSpec,
};

#[derive(Default)]
struct Live {
    events: Vec<PlatformEvent>,
    inputs: Vec<String>,
    answers: Vec<(String, Value)>,
    deleted: bool,
}

#[derive(Default)]
struct State {
    sessions: HashMap<String, Live>,
    next: u64,
    down: bool,
}

/// An agent platform that does what a test tells it, and a scripted judge sandbox.
#[derive(Default)]
pub struct FakePlatform {
    state: Mutex<State>,
    script: Script,
}

impl FakePlatform {
    pub fn with(self, sha: &str, profile: Profile) -> Self {
        self.script.with(sha, profile);
        self
    }

    pub fn failing(self, sha: &str) -> Self {
        self.script.failing(sha);
        self
    }

    /// `(on the holdout, repeats)` for each judge run so far.
    pub fn calls(&self) -> Vec<(bool, u32)> {
        self.script.calls()
    }

    /// What the agent does next, as the platform would report it.
    pub fn emit(&self, session: &SessionHandle, event: PlatformEvent) {
        if let Some(live) = locked(&self.state).sessions.get_mut(&session.0) {
            live.events.push(event);
        }
    }

    pub fn set_down(&self, down: bool) {
        locked(&self.state).down = down;
    }

    pub fn inputs(&self, session: &SessionHandle) -> Vec<String> {
        locked(&self.state)
            .sessions
            .get(&session.0)
            .map(|s| s.inputs.clone())
            .unwrap_or_default()
    }

    pub fn answers(&self, session: &SessionHandle) -> Vec<(String, Value)> {
        locked(&self.state)
            .sessions
            .get(&session.0)
            .map(|s| s.answers.clone())
            .unwrap_or_default()
    }

    pub fn is_deleted(&self, session: &SessionHandle) -> bool {
        locked(&self.state)
            .sessions
            .get(&session.0)
            .is_some_and(|s| s.deleted)
    }

    fn live<T>(
        &self,
        session: &SessionHandle,
        f: impl FnOnce(&mut Live) -> T,
    ) -> Result<T, PlatformError> {
        let mut state = locked(&self.state);
        if state.down {
            return Err(PlatformError::Unavailable(
                "the fake platform is down".into(),
            ));
        }
        match state.sessions.get_mut(&session.0) {
            Some(live) if !live.deleted => Ok(f(live)),
            _ => Err(PlatformError::UnknownSession),
        }
    }
}

impl Platform for FakePlatform {
    fn name(&self) -> &str {
        "fake"
    }

    async fn start_session(&self, spec: &SessionSpec) -> Result<SessionHandle, PlatformError> {
        let mut state = locked(&self.state);
        if state.down {
            return Err(PlatformError::Unavailable(
                "the fake platform is down".into(),
            ));
        }
        state.next += 1;
        let handle = format!("fake-session-{}", state.next);
        let live = Live {
            events: vec![PlatformEvent::Message {
                text: format!("started {}: {}", spec.agent, spec.prompt),
            }],
            ..Live::default()
        };
        state.sessions.insert(handle.clone(), live);
        Ok(SessionHandle(handle))
    }

    async fn send_input(&self, session: &SessionHandle, text: &str) -> Result<(), PlatformError> {
        self.live(session, |live| live.inputs.push(text.to_owned()))
    }

    async fn session_events(
        &self,
        session: &SessionHandle,
        cursor: Option<&str>,
        _wait: Duration,
    ) -> Result<SessionEvents, PlatformError> {
        self.live(session, |live| {
            let from = cursor
                .and_then(|c| c.parse::<usize>().ok())
                .unwrap_or(0)
                .min(live.events.len());
            SessionEvents {
                events: live.events.get(from..).unwrap_or_default().to_vec(),
                next_cursor: Some(live.events.len().to_string()),
            }
        })
    }

    async fn answer_tool(
        &self,
        session: &SessionHandle,
        call: &str,
        result: &Value,
    ) -> Result<(), PlatformError> {
        self.live(session, |live| {
            if !live.answers.iter().any(|(id, _)| id == call) {
                live.answers.push((call.to_owned(), result.clone()));
            }
        })
    }

    async fn stop(&self, session: &SessionHandle) -> Result<(), PlatformError> {
        self.live(session, |live| {
            live.events.push(PlatformEvent::Ended { failed: None });
        })
    }

    async fn delete(&self, session: &SessionHandle) -> Result<(), PlatformError> {
        let mut state = locked(&self.state);
        match state.sessions.get_mut(&session.0) {
            Some(live) => {
                live.deleted = true;
                Ok(())
            }
            None => Err(PlatformError::UnknownSession),
        }
    }

    async fn run_paired(&self, run: &PairedRun<'_>) -> Result<Pairs, PlatformError> {
        self.script.pairs(run)
    }
}
