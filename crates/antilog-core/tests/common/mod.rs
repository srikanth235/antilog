#![allow(dead_code, reason = "each test file uses part of this")]

//! A stand-in for an agent that calls the tools, and for the server that serves them.

use std::collections::HashMap;
use std::sync::Mutex;

use antilog_core::fakes::FakeDeps;
use antilog_core::session;
use antilog_core::tools::{Caller, ToolHost, Toolbox};
use antilog_types::{Session, SessionStatus};
use serde_json::Value;

pub struct Agent {
    pub session: Session,
    queue: Mutex<Vec<(String, String, Value)>>,
    answers: Mutex<HashMap<String, Value>>,
    inputs: Mutex<Vec<String>>,
}

impl Agent {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            queue: Mutex::default(),
            answers: Mutex::default(),
            inputs: Mutex::default(),
        }
    }

    /// The agent calls a tool; it is served at the next `step`.
    pub fn call(&self, id: &str, name: &str, input: Value) {
        self.queue
            .lock()
            .unwrap()
            .push((id.into(), name.into(), input));
    }

    /// Serves the queued calls as the API does, then applies the cards a person decided.
    /// What an agent would be told in words lands in `inputs`.
    pub async fn step(&self, deps: &FakeDeps) -> SessionStatus {
        let host = Toolbox(deps);
        let queued: Vec<_> = self.queue.lock().unwrap().drain(..).collect();
        for (id, name, input) in queued {
            let caller = Caller {
                organization: self.session.organization_id,
                experiment: self.session.experiment_id,
                session: self.session.id,
                actor: None,
                call: id.clone(),
            };
            let reply = host.call(&caller, &name, input).await;
            self.answers.lock().unwrap().insert(id, reply.answer);
            if let Some(ticket) = reply.ticket {
                let text = host.finish(&caller, &ticket).await;
                self.inputs.lock().unwrap().push(text);
            }
        }
        let told = session::settle(deps, self.session.organization_id, self.session.id)
            .await
            .unwrap();
        self.inputs.lock().unwrap().extend(told);
        antilog_core::store::SessionStore::session(
            &deps.store,
            self.session.organization_id,
            self.session.id,
        )
        .await
        .unwrap()
        .unwrap()
        .status
    }

    pub fn answer(&self, id: &str) -> Value {
        self.answers
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .expect("the agent was answered")
    }

    pub fn inputs(&self) -> Vec<String> {
        self.inputs.lock().unwrap().clone()
    }
}
