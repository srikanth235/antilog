#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

mod common;

use antilog_core::fakes::FakeDeps;
use antilog_core::seed::{Seed, T0, organization};
use antilog_core::session::{self, NewSession};
use antilog_core::store::RunnerStore;
use antilog_core::store::{ApprovalStore, SessionStore};
use antilog_types::{Command, Decision, SessionStatus};
use common::Agent;
use serde_json::json;

async fn world() -> (FakeDeps, Seed) {
    let deps = FakeDeps::new();
    let seed = organization(&deps.store).await;
    (deps, seed)
}

async fn lead_with(deps: &FakeDeps, seed: &Seed, budget: Option<i64>) -> Agent {
    let session = session::start(
        deps,
        seed.organization.id,
        seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "opencode/test".into(),
            prompt: "cut p95".into(),
            budget_cap_cents: budget,
            actor: seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    Agent::new(session)
}

async fn lead(deps: &FakeDeps, seed: &Seed) -> Agent {
    lead_with(deps, seed, Some(1_000)).await
}

#[tokio::test]
async fn a_started_session_is_queued_and_spend_is_booked_once() {
    let (deps, seed) = world().await;
    let session = lead(&deps, &seed).await.session;
    let org = seed.organization.id;
    assert_eq!(session.status, SessionStatus::Queued);
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
    let spent = antilog_core::spend::spent(&deps.store, &deps.clock, org, session.id)
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
    let (deps, seed) = world().await;
    let session = lead(&deps, &seed).await.session;
    let org = seed.organization.id;
    let resumed = session::message(&deps, org, session.id, None, "try caching")
        .await
        .unwrap();
    assert_eq!(
        resumed.status,
        SessionStatus::Queued,
        "no runner holds it yet"
    );
    let stopped = session::stop(&deps, org, session.id, Some(seed.owner.id), "Grace")
        .await
        .unwrap();
    assert_eq!(stopped.status, SessionStatus::Stopped);
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
async fn setup_tools_ask_and_refuse_with_a_next_step() {
    let (deps, seed) = world().await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;
    agent.call("a", "attach_repo", json!({"repo": "acme/app"}));
    agent.call("b", "attach_repo", json!({"repo": "nonsense"}));
    agent.call(
        "c",
        "request_credential",
        json!({"connector":"snowflake","reason":"read orders","scope":"admin"}),
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
    let cards = deps.store.approvals(org, agent.session.id).await.unwrap();
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
        .session(org, agent.session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.budget_cap_cents, Some(1_500));
    agent.step(&deps).await;
    let told = agent
        .inputs()
        .iter()
        .filter(|m| m.starts_with("Approved"))
        .count();
    assert_eq!(told, 1, "told once");
}

#[tokio::test]
async fn a_spent_budget_leaves_no_room_and_the_organization_cap_stops_new_sessions() {
    let (deps, seed) = world().await;
    let agent = lead_with(&deps, &seed, Some(100)).await;
    let org = seed.organization.id;
    let room = antilog_core::budget::ensure_room(&deps.store, org, agent.session.id, T0)
        .await
        .unwrap();
    assert_eq!(room.remaining(), Some(100));
    antilog_core::spend::record_usage(&deps.store, org, agent.session.id, 100, 1, T0, "u")
        .await
        .unwrap();
    assert!(matches!(
        antilog_core::budget::ensure_room(&deps.store, org, agent.session.id, T0).await,
        Err(antilog_core::organization::Refusal::Spent(
            antilog_core::budget::Spent::Session
        ))
    ));
    agent.call(
        "r",
        "request_budget",
        json!({"amount_cents": 50, "reason": "one more try"}),
    );
    agent.step(&deps).await;
    assert_eq!(agent.answer("r")["status"], "waiting for a person");

    // A session with no cap of its own is held only by the organization's monthly cap.
    let free = lead_with(&deps, &seed, None).await;
    free.call(
        "r",
        "request_budget",
        json!({"amount_cents": 50, "reason": "more"}),
    );
    free.step(&deps).await;
    assert!(
        free.answer("r")["error"]
            .as_str()
            .unwrap()
            .contains("no budget of its own")
    );
    let owner = antilog_core::organization::Principal {
        organization: org,
        member: seed.owner.id,
    };
    antilog_core::budget::set_organization_limits(&deps.store, &owner, T0, Some(120), None, None)
        .await
        .unwrap();
    assert!(
        antilog_core::budget::set_session_budget(&deps.store, &owner, T0, free.session.id, 121)
            .await
            .is_err(),
        "a session's budget stays inside the organization's cap"
    );
    let now = antilog_core::clock::Clock::now(&deps.clock).as_millisecond();
    antilog_core::spend::record_usage(&deps.store, org, free.session.id, 120, 1, now, "m")
        .await
        .unwrap();
    let again = session::start(
        &deps,
        org,
        seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "opencode/test".into(),
            actor: seed.owner.id,
            ..NewSession::default()
        },
    )
    .await;
    assert!(matches!(again, Err(session::SessionError::CapReached)));
}

#[tokio::test]
async fn a_decided_card_reaches_the_agent_once_and_an_approved_repo_reaches_the_machine() {
    let (deps, seed) = world().await;
    let agent = lead(&deps, &seed).await;
    let (org, id) = (seed.organization.id, agent.session.id);
    agent.call("a", "attach_repo", json!({"repo": "acme/lib"}));
    agent.step(&deps).await;
    // A runner holds the machine: the repo is cloned now.
    let held = deps.store.claim("host", 1, T0, T0 + 60_000).await.unwrap();
    assert_eq!(held.len(), 1);
    let card = deps.store.approvals(org, id).await.unwrap()[0].id;
    let entry = antilog_types::AuditEntry::new(
        org,
        Some(seed.owner.id),
        "card.approve",
        card,
        antilog_types::AuditOutcome::Approved,
        T0,
    );
    deps.store
        .decide_approval(org, card, Decision::Approve, seed.owner.id, T0, &entry)
        .await
        .unwrap();
    // Settling twice, as the decision and the agent's next call both do, tells it once.
    for _ in 0..2 {
        session::settle(&deps, org, id).await.unwrap();
    }
    let machine = deps.store.machine(id).await.unwrap().unwrap();
    assert_eq!(machine.seed.repos, ["acme/lib"], "a later claim clones it");
    let commands = deps.store.take_commands("host").await.unwrap();
    let attach: Vec<_> = commands
        .iter()
        .filter_map(|e| match &e.command {
            Command::AttachRepo { repo } => Some(repo.name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(attach, ["acme/lib"]);
    let messages: Vec<_> = commands
        .iter()
        .filter_map(|e| match &e.command {
            Command::Message { text } if text.starts_with("Approved") => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 1, "one message wakes the agent");
    assert!(messages[0].contains("being cloned into the machine's working folder as lib"));
    // The conversation has it as a note from Antilog, not as the person's words.
    let page = session::transcript(&deps.store, org, id, None, None)
        .await
        .unwrap();
    let notes: Vec<_> = page
        .events
        .iter()
        .filter(|e| e.text.starts_with("Approved"))
        .collect();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].kind, antilog_types::EventKind::Agent);
    assert_eq!(notes[0].member_id, None);
}

#[tokio::test]
async fn the_conversation_keeps_words_and_a_cleaned_record_of_each_tool_call() {
    let (deps, seed) = world().await;
    let agent = lead(&deps, &seed).await;
    let org = seed.organization.id;
    agent.call(
        "c1",
        "attach_repo",
        json!({"repo": "acme/app", "note": "use alc_0123abcdef", "secret-looking": "kept"}),
    );
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
            ("tool", "attach_repo"),
            ("person", "try caching"),
        ]
    );
    assert_eq!(page.events[0].member_id, Some(seed.owner.id));
    assert_eq!(page.events[2].member_id, Some(seed.owner.id));
    let record = page.events[1]
        .tool
        .as_ref()
        .expect("a tool call carries its record");
    assert_eq!(record.label, "attach_repo");
    let input = record.input.as_deref().unwrap_or_default();
    assert!(input.contains("kept"), "the input is kept: {input}");
    assert!(!input.contains("alc_0123"), "a credential is not: {input}");
    assert!(record.output.is_some(), "the answer is kept, cut small");
    assert!(page.events[0].tool.is_none() && page.events[2].tool.is_none());
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
