use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};

use antilog_core::platform::{
    PairedRun, Platform, PlatformError, PlatformEvent, SessionHandle, SessionSpec,
};
use antilog_core::platform_suite::agent_suite;
use antilog_types::judge::{CommitRef, DatasetRef};
use serde_json::{Value, json};

use super::*;

#[derive(Default)]
struct Live {
    log: Vec<Value>,
    answered: HashSet<String>,
}

/// A stand-in for the two images: the agent keeps its event log in memory per runtime
/// session, the judge answers a paired run with fixed numbers.
#[derive(Default)]
struct StubRuntime {
    sessions: Mutex<HashMap<String, Live>>,
    calls: Mutex<Vec<(String, String, Value)>>,
    stopped: Mutex<Vec<(String, String)>>,
    down: Mutex<bool>,
}

impl StubRuntime {
    fn calls(&self) -> Vec<(String, String, Value)> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Runtime for StubRuntime {
    async fn invoke(
        &self,
        runtime: &str,
        session: &str,
        body: Vec<u8>,
    ) -> Result<Vec<u8>, RuntimeError> {
        if *self.down.lock().unwrap_or_else(PoisonError::into_inner) {
            return Err(RuntimeError::Unavailable("throttled".into()));
        }
        let request: Value = serde_json::from_slice(&body).expect("the adapter sends JSON");
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((runtime.to_owned(), session.to_owned(), request.clone()));
        let mut sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
        let unknown = || json!({"ok": false, "error": "unknown_session"});
        let answer = match request["op"].as_str().unwrap_or_default() {
            "start" => {
                sessions.insert(
                    session.to_owned(),
                    Live {
                        log: vec![json!({"type": "message", "text": "ready"})],
                        ..Live::default()
                    },
                );
                json!({"ok": true})
            }
            "paired" => json!({
                "metric": [[10.0, 8.0], [10.0, 8.0]],
                "guardrails": [{"name": "memory", "pairs": [[1.0, 1.0], [1.0, 1.0]]}],
            }),
            op => match sessions.get_mut(session) {
                None => unknown(),
                Some(live) => match op {
                    "input" => json!({"ok": true}),
                    "answer" => {
                        let call = request["call"].as_str().unwrap_or_default().to_owned();
                        live.answered.insert(call);
                        json!({"ok": true})
                    }
                    "stop" => {
                        live.log.push(json!({"type": "ended", "failed": null}));
                        json!({"ok": true})
                    }
                    "events" => {
                        let from = request["cursor"]
                            .as_str()
                            .and_then(|c| c.parse::<usize>().ok())
                            .unwrap_or(0)
                            .min(live.log.len());
                        json!({"events": live.log[from..], "next_cursor": live.log.len().to_string()})
                    }
                    _ => json!({"ok": false, "error": "unknown op"}),
                },
            },
        };
        Ok(serde_json::to_vec(&answer).unwrap())
    }

    async fn stop(&self, runtime: &str, session: &str) -> Result<(), RuntimeError> {
        self.sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(session);
        self.stopped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((runtime.to_owned(), session.to_owned()));
        Ok(())
    }
}

fn adapter() -> AgentCore<StubRuntime> {
    AgentCore::new(
        StubRuntime::default(),
        &Config::new("agent-arn", "judge-arn"),
        "agent-arn".to_owned(),
        "judge-arn".to_owned(),
    )
}

#[tokio::test]
async fn meets_the_platform_contract() {
    agent_suite(&adapter()).await;
}

#[tokio::test]
async fn a_session_starts_on_the_agent_runtime_with_the_configured_model() {
    let platform = adapter();
    let spec = SessionSpec {
        agent: "coder".to_owned(),
        prompt: "go".to_owned(),
        repos: vec!["acme/app".to_owned()],
        environment: Some("py".to_owned()),
        tools: vec![],
    };
    let handle = platform.start_session(&spec).await.unwrap();
    assert!(
        handle.0.len() >= 33,
        "AgentCore needs 33 characters or more: {}",
        handle.0
    );
    let calls = platform.runtime.calls();
    let (runtime, session, request) = &calls[0];
    assert_eq!(runtime, "agent-arn");
    assert_eq!(session, &handle.0);
    assert_eq!(request["model"], "zai.glm-4.7-flash");
    assert_eq!(request["fallback_model"], "qwen.qwen3-coder-30b-a3b-v1:0");
    assert_eq!(request["repos"], json!(["acme/app"]));
}

#[tokio::test]
async fn delete_ends_the_microvm_and_the_log_goes_with_it() {
    let platform = adapter();
    let handle = platform
        .start_session(&SessionSpec {
            agent: "a".to_owned(),
            prompt: "p".to_owned(),
            repos: vec![],
            environment: None,
            tools: vec![],
        })
        .await
        .unwrap();
    platform.delete(&handle).await.unwrap();
    assert_eq!(
        platform
            .runtime
            .stopped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_slice(),
        [("agent-arn".to_owned(), handle.0)]
    );
}

#[tokio::test]
async fn an_unreachable_runtime_is_unavailable_not_failed() {
    let platform = adapter();
    *platform.runtime.down.lock().unwrap() = true;
    let result = platform
        .send_input(&SessionHandle("x".to_owned()), "hello")
        .await;
    assert!(
        matches!(result, Err(PlatformError::Unavailable(_))),
        "{result:?}"
    );
}

#[tokio::test]
async fn a_paired_run_uses_the_judge_runtime_in_a_fresh_session_each_time() {
    let platform = adapter();
    let evaluation = DatasetRef {
        id: "eval".to_owned(),
        version: 1,
    };
    let holdout = DatasetRef {
        id: "hold".to_owned(),
        version: 1,
    };
    let base = [CommitRef {
        repo: "acme/app".to_owned(),
        sha: "aaa".to_owned(),
    }];
    let trial = [CommitRef {
        repo: "acme/app".to_owned(),
        sha: "bbb".to_owned(),
    }];
    let run = |data| PairedRun {
        seal: None,
        command: "make bench",
        files: &[],
        data,
        base: &base,
        trial: &trial,
        repeats: 2,
        guardrails: &[],
    };
    let pairs = platform
        .run_paired(&run(antilog_core::platform::Data::Evaluation(&evaluation)))
        .await
        .unwrap();
    assert_eq!(pairs.metric, vec![(10.0, 8.0), (10.0, 8.0)]);
    assert_eq!(pairs.guardrails[0].0, "memory");
    platform
        .run_paired(&run(antilog_core::platform::Data::Holdout(&holdout)))
        .await
        .unwrap();

    let calls = platform.runtime.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|(runtime, _, _)| runtime == "judge-arn"));
    assert_ne!(calls[0].1, calls[1].1, "each run is its own sandbox");
    assert_eq!(calls[0].2["data"]["kind"], "evaluation");
    assert_eq!(calls[1].2["data"]["kind"], "holdout");
    assert_eq!(calls[0].2["repeats"], 2);
    assert_eq!(
        platform
            .runtime
            .stopped
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len(),
        2,
        "every judge sandbox is ended"
    );
}

#[tokio::test]
async fn an_event_keeps_its_shape() {
    let platform = adapter();
    let handle = platform
        .start_session(&SessionSpec {
            agent: "a".to_owned(),
            prompt: "p".to_owned(),
            repos: vec![],
            environment: None,
            tools: vec![],
        })
        .await
        .unwrap();
    let page = platform
        .session_events(&handle, None, Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(
        page.events,
        vec![PlatformEvent::Message {
            text: "ready".to_owned()
        }]
    );
    let sent = platform.runtime.calls();
    assert_eq!(sent.last().unwrap().2["wait_ms"], 1000);
}
