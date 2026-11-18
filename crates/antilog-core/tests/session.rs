#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakePlatform};
use antilog_core::judge::fixtures::definition;
use antilog_core::judge::sandbox::Profile;
use antilog_core::platform::{PlatformEvent, SessionHandle};
use antilog_core::seed::{Seed, T0, experiment};
use antilog_core::session::{self, NewSession, STEP};
use antilog_core::store::{ApprovalStore, RelayStore, SessionStore};
use antilog_core::tools::Toolbox;
use antilog_types::{ApprovalStatus, Decision, SessionStatus};
use serde_json::{Value, json};

fn platform() -> FakePlatform {
    let mut p = FakePlatform::default().with("base", Profile::new(100.0));
    for i in 0..4 {
        p = p.with(&format!("cheat-{i}"), Profile::new(10.0).holdout(100.0));
    }
    p.with("fast", Profile::new(80.0))
        .with("slow", Profile::new(120.0))
}

async fn world(budget: i64) -> (FakeDeps, Seed) {
    let deps = FakeDeps::new(
        platform(),
        FakeCodeHost::default()
            .with("fast", "app", "src/a.rs")
            .with("slow", "app", "src/b.rs"),
    );
    let seed = experiment(&deps.store, budget).await;
    (deps, seed)
}

async fn lead(deps: &FakeDeps, seed: &Seed) -> (antilog_types::Session, SessionHandle) {
    let session = session::start(
        deps,
        seed.organization.id,
        seed.experiment.id,
        NewSession {
            agent: "claude".into(),
            prompt: "cut p95".into(),
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    let handle = SessionHandle(session.platform_handle.clone().unwrap());
    (session, handle)
}

fn call(deps: &FakeDeps, handle: &SessionHandle, id: &str, name: &str, input: Value) {
    deps.platform.emit(
        handle,
        PlatformEvent::ToolCall {
            id: id.into(),
            name: name.into(),
            input,
        },
    );
}

fn answer(deps: &FakeDeps, handle: &SessionHandle, id: &str) -> Value {
    deps.platform
        .answers(handle)
        .into_iter()
        .find(|(call, _)| call == id)
        .expect("the agent was answered")
        .1
}

fn cheats() -> Value {
    let cheats: Vec<Value> = antilog_types::judge::STANDARD_CHEATS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let path = if i == 3 {
                "src/x.rs"
            } else {
                "bench/data.json"
            };
            json!({"name": name, "commits": [{"repo":"app","sha":format!("cheat-{i}")}],
                "changed": [["app", path]]})
        })
        .collect();
    Value::Array(cheats)
}

#[tokio::test]
async fn start_books_the_first_step_and_a_step_meters_spend_once() {
    let (deps, seed) = world(1_000).await;
    let (session, handle) = lead(&deps, &seed).await;
    let org = seed.organization.id;
    assert_eq!(session.status, SessionStatus::Working);
    let wake = deps.timer.booked();
    assert_eq!(wake.len(), 1);
    assert_eq!(wake[0].1, deps.clock.now_plus(STEP));

    deps.platform.emit(
        &handle,
        PlatformEvent::Usage {
            cents: 120,
            tokens: 5_000,
        },
    );
    deps.platform.emit(
        &handle,
        PlatformEvent::Usage {
            cents: 30,
            tokens: 1_000,
        },
    );
    let step = session::step(&deps, &Toolbox(&deps), org, session.id)
        .await
        .unwrap();
    assert_eq!(step.events, 3);
    let spent = antilog_core::spend::spent(&deps.store, &deps.clock, org, seed.experiment.id)
        .await
        .unwrap();
    assert_eq!((spent.cents, spent.tokens), (150, 6_000));

    let saved = deps
        .store
        .relay_cursor(org, session.id)
        .await
        .unwrap()
        .unwrap();
    deps.platform.emit(
        &handle,
        PlatformEvent::Usage {
            cents: 10,
            tokens: 1,
        },
    );
    session::step(&deps, &Toolbox(&deps), org, session.id)
        .await
        .unwrap();
    deps.store
        .save_relay_cursor(org, session.id, &saved, T0)
        .await
        .unwrap();
    session::step(&deps, &Toolbox(&deps), org, session.id)
        .await
        .unwrap();
    let again = antilog_core::spend::spent(&deps.store, &deps.clock, org, seed.experiment.id)
        .await
        .unwrap();
    assert_eq!(
        (again.cents, again.tokens),
        (160, 6_001),
        "a replayed step books nothing twice"
    );
    assert_eq!(deps.timer.booked().len(), 1, "one job per session");
}

#[tokio::test]
async fn idle_waits_a_message_resumes_and_stop_is_final() {
    let (deps, seed) = world(1_000).await;
    let (session, handle) = lead(&deps, &seed).await;
    let org = seed.organization.id;
    deps.platform.emit(&handle, PlatformEvent::Idle);
    let step = session::step(&deps, &Toolbox(&deps), org, session.id)
        .await
        .unwrap();
    assert_eq!(step.status, SessionStatus::WaitingForYou);
    let resumed = session::message(&deps, org, session.id, None, "try caching")
        .await
        .unwrap();
    assert_eq!(resumed.status, SessionStatus::Working);
    assert_eq!(
        deps.platform.inputs(&handle),
        vec!["try caching".to_owned()]
    );
    let stopped = session::stop(&deps, org, session.id, Some(seed.owner.id), "Grace")
        .await
        .unwrap();
    assert_eq!(stopped.status, SessionStatus::Done);
    assert_eq!(stopped.note.as_deref(), Some("Stopped by Grace"));
    assert_eq!(stopped.cleanup, Some(antilog_types::Cleanup::Removed));
    assert!(deps.platform.is_deleted(&handle));
    assert!(deps.timer.booked().is_empty());
    assert!(
        session::message(&deps, org, session.id, None, "again")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_platform_that_is_down_leaves_waiting_for_you_and_a_lost_session_fails() {
    let (deps, seed) = world(1_000).await;
    deps.platform.set_down(true);
    let session = session::start(
        &deps,
        seed.organization.id,
        seed.experiment.id,
        NewSession::default(),
    )
    .await
    .unwrap();
    assert_eq!(session.status, SessionStatus::WaitingForYou);
    assert!(session.platform_handle.is_none());
    deps.platform.set_down(false);
    let (second, handle) = lead(&deps, &seed).await;
    deps.platform.set_down(true);
    let step = session::step(&deps, &Toolbox(&deps), seed.organization.id, second.id)
        .await
        .unwrap();
    assert_eq!(step.status, SessionStatus::WaitingForYou);
    deps.platform.set_down(false);
    deps.platform.emit(
        &handle,
        PlatformEvent::Ended {
            failed: Some("out of memory".into()),
        },
    );
    let step = session::step(&deps, &Toolbox(&deps), seed.organization.id, second.id)
        .await
        .unwrap();
    assert_eq!(
        step.status,
        SessionStatus::Failed,
        "a waiting session still ends"
    );
}

#[tokio::test]
async fn seal_measure_history_deliver_through_the_session() {
    let (deps, seed) = world(100_000).await;
    let (session, handle) = lead(&deps, &seed).await;
    let org = seed.organization.id;
    let host = Toolbox(&deps);

    call(
        &deps,
        &handle,
        "c1",
        "measure",
        json!({"commits":[{"repo":"app","sha":"fast"}],"hypothesis":"cache"}),
    );
    session::step(&deps, &host, org, session.id).await.unwrap();
    assert!(
        answer(&deps, &handle, "c1")["error"]
            .as_str()
            .unwrap()
            .contains("call seal")
    );

    call(
        &deps,
        &handle,
        "c2",
        "seal",
        json!({"definition": definition(), "cheats": cheats()}),
    );
    session::step(&deps, &host, org, session.id).await.unwrap();
    let sealed = answer(&deps, &handle, "c2");
    assert_eq!(sealed["status"], "waiting for a person", "{sealed}");
    let approval = deps
        .store
        .approvals(org, seed.experiment.id)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(approval.status, ApprovalStatus::Pending);
    let waiting = deps.store.session(org, session.id).await.unwrap().unwrap();
    assert_eq!(waiting.status, SessionStatus::WaitingForYou);

    let entry = antilog_types::AuditEntry::new(
        org,
        Some(seed.owner.id),
        "card.approve",
        approval.id,
        antilog_types::AuditOutcome::Approved,
        T0,
    );
    deps.store
        .decide_approval(
            org,
            approval.id,
            Decision::Approve,
            seed.owner.id,
            T0,
            &entry,
        )
        .await
        .unwrap();
    let step = session::step(&deps, &host, org, session.id).await.unwrap();
    assert_eq!(step.status, SessionStatus::Working);
    assert!(
        deps.platform
            .inputs(&handle)
            .last()
            .unwrap()
            .contains("Sealed as version 1")
    );
    assert_eq!(
        deps.store
            .experiment(org, seed.experiment.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        antilog_types::ExperimentStatus::Sealed
    );

    call(
        &deps,
        &handle,
        "c3",
        "measure",
        json!({"commits":[{"repo":"app","sha":"fast"}],"hypothesis":"cache"}),
    );
    session::step(&deps, &host, org, session.id).await.unwrap();
    assert_eq!(answer(&deps, &handle, "c3")["status"], "measuring");
    let told = deps.platform.inputs(&handle).last().unwrap().clone();
    assert!(told.contains("kept"), "{told}");
    assert!(told.contains("p95 80.00 ms"), "{told}");

    call(&deps, &handle, "c4", "history", json!({}));
    session::step(&deps, &host, org, session.id).await.unwrap();
    let history = answer(&deps, &handle, "c4");
    assert_eq!(history["trials"].as_array().unwrap().len(), 1);
    let trial = history["trials"][0]["trial"].clone();
    assert_eq!(history["best"], trial);

    call(&deps, &handle, "c5", "deliver", json!({"trial": trial}));
    call(
        &deps,
        &handle,
        "c6",
        "deliver",
        json!({"trial": "00000000-0000-7000-8000-000000000000"}),
    );
    session::step(&deps, &host, org, session.id).await.unwrap();
    assert_eq!(
        answer(&deps, &handle, "c5")["status"],
        "waiting for a person"
    );
    assert!(
        answer(&deps, &handle, "c6")["error"]
            .as_str()
            .unwrap()
            .contains("history")
    );
}

#[tokio::test]
async fn setup_tools_ask_and_refuse_with_a_next_step() {
    let (deps, seed) = world(1_000).await;
    let (session, handle) = lead(&deps, &seed).await;
    let org = seed.organization.id;
    let host = Toolbox(&deps);
    call(
        &deps,
        &handle,
        "a",
        "attach_repo",
        json!({"repo": "acme/app"}),
    );
    call(
        &deps,
        &handle,
        "b",
        "attach_repo",
        json!({"repo": "nonsense"}),
    );
    call(
        &deps,
        &handle,
        "c",
        "request_credential",
        json!({"connector":"snowflake","reason":"read orders","scope":"holdout read"}),
    );
    call(
        &deps,
        &handle,
        "d",
        "request_credential",
        json!({"connector":"snowflake","reason":"read orders","scope":"read","value":"hunter2"}),
    );
    call(&deps, &handle, "e", "set_environment", json!({}));
    call(
        &deps,
        &handle,
        "f",
        "request_budget",
        json!({"amount_cents": 500, "reason": "more trials"}),
    );
    call(&deps, &handle, "g", "nope", json!({}));
    session::step(&deps, &host, org, session.id).await.unwrap();
    assert_eq!(
        answer(&deps, &handle, "a")["status"],
        "waiting for a person"
    );
    for id in ["b", "c", "d", "e", "g"] {
        assert!(answer(&deps, &handle, id)["error"].is_string(), "{id}");
    }
    assert!(
        answer(&deps, &handle, "d")["error"]
            .as_str()
            .unwrap()
            .contains("never a secret")
    );
    let cards = deps.store.approvals(org, seed.experiment.id).await.unwrap();
    assert_eq!(cards.len(), 2);

    let raise = cards
        .iter()
        .find(|c| c.card == antilog_types::CardKind::RaiseBudget)
        .unwrap();
    let entry = antilog_types::AuditEntry::new(
        org,
        Some(seed.owner.id),
        "card.approve",
        raise.id,
        antilog_types::AuditOutcome::Approved,
        T0,
    );
    deps.store
        .decide_approval(org, raise.id, Decision::Approve, seed.owner.id, T0, &entry)
        .await
        .unwrap();
    session::step(&deps, &host, org, session.id).await.unwrap();
    let row = deps
        .store
        .experiment(org, seed.experiment.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.budget_cap_cents, 1_500);
    session::step(&deps, &host, org, session.id).await.unwrap();
    let told = deps
        .platform
        .inputs(&handle)
        .iter()
        .filter(|m| m.starts_with("Approved"))
        .count();
    assert_eq!(told, 1, "told once");
}

#[tokio::test]
async fn spawn_hands_over_history_and_a_spawned_session_cannot_spawn() {
    let (deps, seed) = world(1_000).await;
    let (lead_session, handle) = lead(&deps, &seed).await;
    let org = seed.organization.id;
    let host = Toolbox(&deps);
    call(
        &deps,
        &handle,
        "s",
        "spawn",
        json!({"agent":"codex","prompt":"try indexes"}),
    );
    session::step(&deps, &host, org, lead_session.id)
        .await
        .unwrap();
    let spawned = answer(&deps, &handle, "s");
    let child: antilog_types::SessionId =
        serde_json::from_value(spawned["session"].clone()).unwrap();
    let child_row = deps.store.session(org, child).await.unwrap().unwrap();
    assert_eq!(child_row.parent, Some(lead_session.id));
    let child_handle = SessionHandle(child_row.platform_handle.unwrap());
    call(
        &deps,
        &child_handle,
        "x",
        "spawn",
        json!({"agent":"codex","prompt":"again"}),
    );
    session::step(&deps, &host, org, child).await.unwrap();
    assert!(
        answer(&deps, &child_handle, "x")["error"]
            .as_str()
            .unwrap()
            .contains("spawned session cannot spawn")
    );
}

#[tokio::test]
async fn a_spent_budget_stops_new_sessions_and_measurements() {
    let (deps, seed) = world(100).await;
    let (session, handle) = lead(&deps, &seed).await;
    let org = seed.organization.id;
    deps.platform.emit(
        &handle,
        PlatformEvent::Usage {
            cents: 100,
            tokens: 1,
        },
    );
    call(
        &deps,
        &handle,
        "m",
        "measure",
        json!({"commits":[{"repo":"app","sha":"fast"}],"hypothesis":"x"}),
    );
    session::step(&deps, &Toolbox(&deps), org, session.id)
        .await
        .unwrap();
    assert!(
        answer(&deps, &handle, "m")["error"]
            .as_str()
            .unwrap()
            .contains("request_budget")
    );
    let again = session::start(
        &deps,
        org,
        seed.experiment.id,
        NewSession {
            agent: "claude".into(),
            ..NewSession::default()
        },
    )
    .await;
    assert!(matches!(again, Err(session::SessionError::Budget(_))));
}
