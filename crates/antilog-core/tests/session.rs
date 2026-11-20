#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine, Profile};
use antilog_core::judge::fixtures::definition;
use antilog_core::seed::{Seed, T0, experiment};
use antilog_core::session::{self, NewSession};
use antilog_core::store::{ApprovalStore, OrganizationStore, SessionStore};
use antilog_types::{ApprovalStatus, Decision, SessionStatus};
use common::Agent;
use serde_json::{Value, json};

fn machine() -> FakeMachine {
    let mut p = FakeMachine::default().with("base", Profile::new(100.0));
    for i in 0..4 {
        p = p.with(&format!("cheat-{i}"), Profile::new(10.0).holdout(100.0));
    }
    p.with("fast", Profile::new(80.0))
        .with("slow", Profile::new(120.0))
}

async fn world(budget: i64) -> (FakeDeps, Seed) {
    let deps = FakeDeps::new(
        machine(),
        FakeCodeHost::default()
            .with("fast", "app", "src/a.rs")
            .with("slow", "app", "src/b.rs"),
    );
    let seed = experiment(&deps.store, budget).await;
    (deps, seed)
}

async fn lead(deps: &FakeDeps, seed: &Seed) -> Agent {
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
    Agent::new(session)
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
async fn a_started_session_is_working_and_spend_is_booked_once() {
    let (deps, seed) = world(1_000).await;
    let session = lead(&deps, &seed).await.session;
    let org = seed.organization.id;
    assert_eq!(session.status, SessionStatus::Working);
    for (seed_name, cents, tokens) in [("a", 120, 5_000), ("b", 30, 1_000), ("a", 120, 5_000)] {
        antilog_core::spend::record_usage(
            &deps.store,
            org,
            session.id,
            cents,
            tokens,
            T0,
            seed_name,
        )
        .await
        .unwrap();
    }
    let spent = antilog_core::spend::spent(&deps.store, &deps.clock, org, seed.experiment.id)
        .await
        .unwrap();
    assert_eq!(
        (spent.cents, spent.tokens),
        (150, 6_000),
        "a replayed report books nothing twice"
    );
}

#[tokio::test]
async fn a_message_resumes_and_stop_is_final() {
    let (deps, seed) = world(1_000).await;
    let session = lead(&deps, &seed).await.session;
    let org = seed.organization.id;
    let resumed = session::message(&deps, org, session.id, None, "try caching")
        .await
        .unwrap();
    assert_eq!(resumed.status, SessionStatus::Working);
    let stopped = session::stop(&deps, org, session.id, Some(seed.owner.id), "Grace")
        .await
        .unwrap();
    assert_eq!(stopped.status, SessionStatus::Done);
    assert_eq!(stopped.note.as_deref(), Some("Stopped by Grace"));
    assert!(
        session::message(&deps, org, session.id, None, "again")
            .await
            .is_err()
    );
    assert!(
        session::stop(&deps, org, session.id, None, "Grace")
            .await
            .is_err(),
        "a stopped session cannot be stopped again"
    );
}

#[tokio::test]
async fn seal_measure_history_deliver_through_the_session() {
    let (deps, seed) = world(100_000).await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;

    agent.call(
        "c1",
        "measure",
        json!({"commits":[{"repo":"app","sha":"fast"}],"hypothesis":"cache"}),
    );
    agent.step(&deps).await;
    assert!(
        agent.answer("c1")["error"]
            .as_str()
            .unwrap()
            .contains("call seal")
    );

    agent.call(
        "c2",
        "seal",
        json!({"definition": definition(), "cheats": cheats()}),
    );
    agent.step(&deps).await;
    let sealed = agent.answer("c2");
    assert_eq!(sealed["status"], "waiting for a person", "{sealed}");
    let approval = deps
        .store
        .approvals(org, seed.experiment.id)
        .await
        .unwrap()
        .remove(0);
    assert_eq!(approval.status, ApprovalStatus::Pending);
    let waiting = deps
        .store
        .session(org, agent.session.id)
        .await
        .unwrap()
        .unwrap();
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
    let step = agent.step(&deps).await;
    assert_eq!(step, SessionStatus::Working);
    assert!(
        agent
            .inputs()
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

    agent.call(
        "c3",
        "measure",
        json!({"commits":[{"repo":"app","sha":"fast"}],"hypothesis":"cache"}),
    );
    agent.step(&deps).await;
    assert_eq!(agent.answer("c3")["status"], "measuring");
    let told = agent.inputs().last().unwrap().clone();
    assert!(told.contains("kept"), "{told}");
    assert!(told.contains("p95 80.00 ms"), "{told}");

    agent.call("c4", "history", json!({}));
    agent.step(&deps).await;
    let history = agent.answer("c4");
    assert_eq!(history["trials"].as_array().unwrap().len(), 1);
    let trial = history["trials"][0]["trial"].clone();
    assert_eq!(history["best"], trial);

    agent.call("c5", "deliver", json!({"trial": trial}));
    agent.call(
        "c6",
        "deliver",
        json!({"trial": "00000000-0000-7000-8000-000000000000"}),
    );
    agent.step(&deps).await;
    assert_eq!(agent.answer("c5")["status"], "waiting for a person");
    assert!(
        agent.answer("c6")["error"]
            .as_str()
            .unwrap()
            .contains("history")
    );
}

#[tokio::test]
async fn setup_tools_ask_and_refuse_with_a_next_step() {
    let (deps, seed) = world(1_000).await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;
    agent.call("a", "attach_repo", json!({"repo": "acme/app"}));
    agent.call("b", "attach_repo", json!({"repo": "nonsense"}));
    agent.call(
        "c",
        "request_credential",
        json!({"connector":"snowflake","reason":"read orders","scope":"holdout read"}),
    );
    agent.call(
        "d",
        "request_credential",
        json!({"connector":"snowflake","reason":"read orders","scope":"read","value":"hunter2"}),
    );
    agent.call("e", "set_environment", json!({}));
    agent.call(
        "f",
        "request_budget",
        json!({"amount_cents": 500, "reason": "more trials"}),
    );
    agent.call("g", "nope", json!({}));
    agent.step(&deps).await;
    assert_eq!(agent.answer("a")["status"], "waiting for a person");
    for id in ["b", "c", "d", "e", "g"] {
        assert!(agent.answer(id)["error"].is_string(), "{id}");
    }
    assert!(
        agent.answer("d")["error"]
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
    agent.step(&deps).await;
    let row = deps
        .store
        .experiment(org, seed.experiment.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.budget_cap_cents, 1_500);
    agent.step(&deps).await;
    let told = agent
        .inputs()
        .iter()
        .filter(|m| m.starts_with("Approved"))
        .count();
    assert_eq!(told, 1, "told once");
}

#[tokio::test]
async fn spawn_hands_over_history_and_a_spawned_session_cannot_spawn() {
    let (deps, seed) = world(1_000).await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;
    agent.call(
        "s",
        "spawn",
        json!({"agent":"codex","prompt":"try indexes"}),
    );
    agent.step(&deps).await;
    let spawned = agent.answer("s");
    let child: antilog_types::SessionId =
        serde_json::from_value(spawned["session"].clone()).unwrap();
    let child_row = deps.store.session(org, child).await.unwrap().unwrap();
    assert_eq!(child_row.parent, Some(agent.session.id));
    let child_agent = Agent::new(child_row);
    child_agent.call("x", "spawn", json!({"agent":"codex","prompt":"again"}));
    child_agent.step(&deps).await;
    assert!(
        child_agent.answer("x")["error"]
            .as_str()
            .unwrap()
            .contains("spawned session cannot spawn")
    );
}

#[tokio::test]
async fn a_spent_budget_stops_new_sessions_and_measurements() {
    let (deps, seed) = world(100).await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;
    antilog_core::spend::record_usage(&deps.store, org, agent.session.id, 100, 1, T0, "u")
        .await
        .unwrap();
    agent.call(
        "m",
        "measure",
        json!({"commits":[{"repo":"app","sha":"fast"}],"hypothesis":"x"}),
    );
    agent.step(&deps).await;
    assert!(
        agent.answer("m")["error"]
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

#[tokio::test]
async fn the_conversation_keeps_words_and_tool_names_and_never_what_a_tool_returned() {
    let (deps, seed) = world(1_000).await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;
    agent.call("c1", "history", json!({"secret-looking": "never stored"}));
    agent.step(&deps).await;
    session::message(
        &deps,
        org,
        agent.session.id,
        Some(seed.owner.id),
        "try caching",
    )
    .await
    .unwrap();

    let page = session::transcript(&deps.store, org, agent.session.id, None, None)
        .await
        .unwrap();
    let said: Vec<_> = page
        .events
        .iter()
        .map(|event| (event.kind.as_str(), event.text.as_str()))
        .collect();
    assert_eq!(
        said,
        vec![
            ("person", "cut p95"),
            ("tool", "history"),
            ("person", "try caching"),
        ]
    );
    assert_eq!(page.events[0].member_id, None);
    assert_eq!(page.events[2].member_id, Some(seed.owner.id));
    assert!(
        !serde_json::to_string(&page)
            .unwrap()
            .contains("never stored"),
        "a tool's input is not kept"
    );
    assert_eq!(page.next, None);

    let first = session::transcript(&deps.store, org, agent.session.id, None, Some(2))
        .await
        .unwrap();
    assert_eq!(first.events.len(), 2);
    let next = first.next.expect("more follow");
    let rest = session::transcript(&deps.store, org, agent.session.id, Some(next), Some(3))
        .await
        .unwrap();
    assert_eq!(rest.events.len(), 1);
    assert_eq!(rest.next, None);
    assert!(
        session::transcript(
            &deps.store,
            antilog_types::OrganizationId::new(),
            agent.session.id,
            None,
            None
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn sealing_only_points_at_the_projects_value_model_and_says_so_on_the_card() {
    use antilog_core::store::judge::JudgeStore;
    use antilog_types::judge::{ValueModelId, ValueSource};

    let (deps, seed) = world(100_000).await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;
    let project = seed.experiment.project_id;
    let model = |metric: &str, version| antilog_types::judge::ValueModel {
        id: ValueModelId::new(),
        version,
        metric: metric.into(),
        unit: "ms".into(),
        cents_per_unit: 360.0,
        source: ValueSource {
            text: "finance".into(),
            link: None,
        },
        set_by: seed.owner.id.as_uuid(),
    };
    let p95 = model("p95", 1);
    deps.judge.put_value_model(project, &p95, T0).await.unwrap();
    let other_metric = model("bundle", 1);
    deps.judge
        .put_value_model(project, &other_metric, T0)
        .await
        .unwrap();
    let other_project = {
        let p = antilog_types::Project::new(org, "other", "x", T0);
        deps.store
            .create_project(
                &p,
                seed.owner.id,
                &antilog_types::AuditEntry::new(
                    org,
                    Some(seed.owner.id),
                    "project.create",
                    p.id,
                    antilog_types::AuditOutcome::Done,
                    T0,
                ),
            )
            .await
            .unwrap();
        p.id
    };
    let foreign = model("p95", 1);
    deps.judge
        .put_value_model(other_project, &foreign, T0)
        .await
        .unwrap();

    let seal = |id: &str, reference: Value| {
        let mut def = serde_json::to_value(definition()).unwrap();
        def["valueModel"] = reference;
        agent.call(id, "seal", json!({"definition": def, "cheats": cheats()}));
    };
    let reference =
        |m: &antilog_types::judge::ValueModel| json!({"id": m.id, "version": m.version});

    seal("nobody", json!({"id": ValueModelId::new(), "version": 1}));
    seal("metric", reference(&other_metric));
    seal("project", reference(&foreign));
    agent.step(&deps).await;
    for id in ["nobody", "metric", "project"] {
        assert!(agent.answer(id)["error"].is_string(), "{id}");
    }
    assert!(
        deps.store
            .approvals(org, seed.experiment.id)
            .await
            .unwrap()
            .is_empty(),
        "a refused seal raises no card"
    );

    // The project's own model needs no owner; leaving it out or an old one does.
    seal("ok", reference(&p95));
    agent.step(&deps).await;
    assert_eq!(agent.answer("ok")["status"], "waiting for a person");
    seal("dropped", Value::Null);
    agent.step(&deps).await;
    let cards = deps.store.approvals(org, seed.experiment.id).await.unwrap();
    assert_eq!(cards.len(), 2);
    let value = cards.iter().find(|c| c.summary.contains("Value:")).unwrap();
    assert!(
        value.summary.contains("measured offline"),
        "{}",
        value.summary
    );
    assert!(!value.needs_owner);
    let none = cards
        .iter()
        .find(|c| c.summary.contains("No value set"))
        .unwrap();
    assert!(
        none.needs_owner,
        "dropping the project's model needs an owner"
    );
}
