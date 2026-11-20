#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use antilog_core::delivery::{self, Handled, PullRequestAction, PullRequestEvent};
mod common;

use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine, Profile};
use antilog_core::judge::fixtures::definition;
use antilog_core::organization::Principal;
use antilog_core::seed::{Seed, T0, experiment};
use antilog_core::session::{self, NewSession};
use antilog_core::store::judge::JudgeStore;
use antilog_core::store::{ApprovalStore, BudgetStore};
use antilog_core::value::{self, Range};
use antilog_types::judge::{NewValueModel, ValueModelRef, ValueSource, WinEventKind};
use antilog_types::{ApprovalStatus, CardKind, CardPayload, Decision};
use common::Agent;
use serde_json::{Value, json};

fn machine() -> FakeMachine {
    let mut p = FakeMachine::default().with("base", Profile::new(100.0));
    for i in 0..4 {
        p = p.with(&format!("cheat-{i}"), Profile::new(10.0).holdout(100.0));
    }
    p.with("fast", Profile::new(80.0))
        .with("faster", Profile::new(70.0))
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

struct Run {
    deps: FakeDeps,
    seed: Seed,
    agent: Agent,
    session: antilog_types::Session,
}

async fn decide_all(run: &Run) {
    let org = run.seed.organization.id;
    for card in run
        .deps
        .store
        .approvals(org, run.seed.experiment.id)
        .await
        .unwrap()
    {
        if card.status == ApprovalStatus::Pending {
            let entry = antilog_types::AuditEntry::new(
                org,
                Some(run.seed.owner.id),
                "approval.decide",
                card.id,
                antilog_types::AuditOutcome::Approved,
                T0,
            );
            run.deps
                .store
                .decide_approval(
                    org,
                    card.id,
                    Decision::Approve,
                    run.seed.owner.id,
                    T0,
                    &entry,
                )
                .await
                .unwrap();
        }
    }
}

async fn step(run: &Run) {
    run.agent.step(&run.deps).await;
}

/// A sealed experiment worth $1 a year per ms, with one Kept trial that moved p95 from 100 to 80.
async fn kept_win(
    code: FakeCodeHost,
    commits: Value,
    repos: &[&str],
) -> (Run, antilog_types::judge::TrialId) {
    let deps = FakeDeps::new(machine(), code);
    let seed = experiment(&deps.store, 100_000).await;
    let owner = Principal {
        organization: seed.organization.id,
        member: seed.owner.id,
    };
    let model = value::set_value_model(
        &deps.store,
        &deps.judge,
        &owner,
        T0 + 1,
        seed.experiment.project_id,
        NewValueModel {
            metric: "p95".into(),
            unit: "ms".into(),
            cents_per_unit: 100.0,
            source: ValueSource {
                text: "finance".into(),
                link: None,
            },
        },
    )
    .await
    .unwrap();
    let session = session::start(
        &deps,
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
    let run = Run {
        deps,
        seed,
        agent: Agent::new(session.clone()),
        session,
    };
    let mut definition = definition();
    definition.evaluation.repos = repos
        .iter()
        .map(|repo| antilog_types::judge::RepoPin {
            repo: (*repo).into(),
            commit: "base".into(),
        })
        .collect();
    definition.value_model = Some(ValueModelRef {
        id: model.model.id,
        version: model.model.version,
    });
    run.agent.call(
        "c1",
        "seal",
        json!({"definition": definition, "cheats": cheats()}),
    );
    step(&run).await;
    decide_all(&run).await;
    step(&run).await;
    run.agent.call(
        "c2",
        "measure",
        json!({"commits": commits, "hypothesis":"cache"}),
    );
    step(&run).await;
    let sealed = run
        .deps
        .judge
        .sealed(run.seed.experiment.id, 1)
        .await
        .unwrap()
        .unwrap();
    let trial = run
        .deps
        .judge
        .trials(sealed.experiment(), 1)
        .await
        .unwrap()
        .remove(0);
    (run, trial.trial.id)
}

fn merged(delivery: &str, number: u64) -> PullRequestEvent {
    PullRequestEvent {
        delivery: delivery.into(),
        repo: "app".into(),
        number,
        action: PullRequestAction::Closed { merged: true },
    }
}

async fn figures(run: &Run) -> antilog_types::judge::ValueFigures {
    let who = Principal {
        organization: run.seed.organization.id,
        member: run.seed.owner.id,
    };
    value::project_value(
        &run.deps.store,
        &run.deps.judge,
        &who,
        run.seed.experiment.project_id,
        Range::all_time(T0),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn an_approved_card_opens_the_pull_request_and_a_merge_delivers_it_once() {
    let (run, trial) = kept_win(
        FakeCodeHost::default().with("fast", "app", "src/a.rs"),
        json!([{"repo":"app","sha":"fast"}]),
        &["app"],
    )
    .await;
    let org = run.seed.organization.id;
    run.agent.call("c3", "deliver", json!({"trial": trial}));
    step(&run).await;

    // The card carries its fields and opens nothing until a person approves.
    let card = run
        .deps
        .store
        .approvals(org, run.seed.experiment.id)
        .await
        .unwrap()
        .into_iter()
        .find(|card| card.card == CardKind::OpenPullRequest)
        .unwrap();
    assert!(matches!(
        &card.payload,
        Some(CardPayload::OpenPullRequest { trial: t, repos, .. }) if *t == trial && repos == &["app"]
    ));
    assert!(run.deps.code.opened().is_empty());
    assert_eq!(figures(&run).await.pending_cents, 0);

    decide_all(&run).await;
    step(&run).await;
    let opened = run.deps.code.opened();
    assert_eq!(opened.len(), 1);
    let (repo, sha, body) = (&opened[0].repo, &opened[0].sha, &opened[0].body);
    assert_eq!((repo.as_str(), sha.as_str()), ("app", "fast"));
    assert!(body.contains("100.00") && body.contains("80.00"), "{body}");
    let told = run.agent.inputs().last().unwrap().clone();
    assert!(told.contains("https://code.example/app/pull/1"), "{told}");
    let events = run
        .deps
        .judge
        .win_events(run.seed.experiment.id)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, WinEventKind::Opened);

    // Opened is Pending, never Delivered, and Return stays out of it.
    let open = figures(&run).await;
    assert_eq!((open.pending_cents, open.delivered_cents), (2_000, 0));
    assert!(open.return_x.is_none_or(|x| x == 0.0));

    let handled = delivery::handle(&run.deps, &merged("d1", 1)).await.unwrap();
    assert_eq!(handled, Handled::Recorded(WinEventKind::Merged));
    let done = figures(&run).await;
    assert_eq!((done.delivered_cents, done.pending_cents), (2_000, 0));
    assert!(done.return_x.is_none_or(|x| x == 0.0));
    run.deps
        .store
        .record_spend(&antilog_types::Spend {
            id: antilog_types::SpendId::new(),
            organization_id: run.seed.organization.id,
            session_id: run.session.id,
            kind: antilog_types::SpendKind::Tokens,
            amount_cents: 1_000,
            tokens: 1,
            occurred_at: T0,
            trial_id: None,
        })
        .await
        .unwrap();
    assert_eq!(figures(&run).await.return_x, Some(2.0));

    // A redelivery is a no-op, and so is an unknown pull request.
    assert_eq!(
        delivery::handle(&run.deps, &merged("d1", 1)).await.unwrap(),
        Handled::Duplicate
    );
    assert_eq!(
        delivery::handle(&run.deps, &merged("d2", 1)).await.unwrap(),
        Handled::Ignored
    );
    assert_eq!(
        delivery::handle(&run.deps, &merged("d3", 99))
            .await
            .unwrap(),
        Handled::Unknown
    );
    assert_eq!(
        run.deps
            .judge
            .win_events(run.seed.experiment.id)
            .await
            .unwrap()
            .len(),
        2
    );
    let again = figures(&run).await;
    assert_eq!(again.delivered_cents, 2_000);
}

#[tokio::test]
async fn closing_a_pull_request_takes_the_win_back_to_not_opened_and_a_reopen_restores_it() {
    let (run, trial) = kept_win(
        FakeCodeHost::default().with("fast", "app", "src/a.rs"),
        json!([{"repo":"app","sha":"fast"}]),
        &["app"],
    )
    .await;
    run.agent.call("c3", "deliver", json!({"trial": trial}));
    step(&run).await;
    decide_all(&run).await;
    step(&run).await;
    let event = |id: &str, action| PullRequestEvent {
        delivery: id.into(),
        repo: "app".into(),
        number: 1,
        action,
    };
    let closed = event("d1", PullRequestAction::Closed { merged: false });
    assert_eq!(
        delivery::handle(&run.deps, &closed).await.unwrap(),
        Handled::Recorded(WinEventKind::Closed)
    );
    assert_eq!(figures(&run).await.pending_cents, 0);
    assert_eq!(
        delivery::handle(&run.deps, &event("d2", PullRequestAction::Reopened))
            .await
            .unwrap(),
        Handled::Recorded(WinEventKind::Opened)
    );
    assert_eq!(figures(&run).await.pending_cents, 2_000);
    assert_eq!(
        delivery::handle(&run.deps, &event("d3", PullRequestAction::Other))
            .await
            .unwrap(),
        Handled::Ignored
    );
}

#[tokio::test]
async fn a_linked_set_is_delivered_only_when_every_pull_request_merged() {
    let code = FakeCodeHost::default()
        .with("fast", "app", "src/a.rs")
        .with("fast", "lib", "src/b.rs");
    let (run, trial) = kept_win(
        code,
        json!([{"repo":"app","sha":"fast"},{"repo":"lib","sha":"fast"}]),
        &["app", "lib"],
    )
    .await;
    run.agent.call("c3", "deliver", json!({"trial": trial}));
    step(&run).await;
    decide_all(&run).await;
    step(&run).await;
    let opened = run.deps.code.opened();
    assert_eq!(
        opened.iter().map(|o| o.repo.as_str()).collect::<Vec<_>>(),
        ["app", "lib"]
    );
    assert!(opened[0].body.contains("lib") && opened[1].body.contains("app"));
    assert_eq!(figures(&run).await.pending_cents, 2_000);

    let lib = |id: &str| PullRequestEvent {
        delivery: id.into(),
        repo: "lib".into(),
        number: 1,
        action: PullRequestAction::Closed { merged: true },
    };
    assert_eq!(
        delivery::handle(&run.deps, &merged("d1", 1)).await.unwrap(),
        Handled::Waiting
    );
    let half = figures(&run).await;
    assert_eq!((half.delivered_cents, half.pending_cents), (0, 2_000));
    assert_eq!(
        delivery::handle(&run.deps, &lib("d2")).await.unwrap(),
        Handled::Recorded(WinEventKind::Merged)
    );
    assert_eq!(figures(&run).await.delivered_cents, 2_000);
    // Asking to open them again opens nothing more.
    let org = run.seed.organization.id;
    let card = run
        .deps
        .store
        .approvals(org, run.seed.experiment.id)
        .await
        .unwrap()
        .into_iter()
        .find(|card| card.card == CardKind::OpenPullRequest)
        .unwrap();
    let again = delivery::open_approved(&run.deps, &card).await.unwrap();
    assert!(again.contains("already open"), "{again}");
    assert_eq!(run.deps.code.opened().len(), 2);
}

#[tokio::test]
async fn a_refusing_code_host_leaves_the_win_unopened_and_tells_the_session() {
    let (run, trial) = kept_win(
        FakeCodeHost::default().with("fast", "app", "src/a.rs"),
        json!([{"repo":"app","sha":"fast"}]),
        &["app"],
    )
    .await;
    run.deps.code.refuse_pull_requests();
    run.agent.call("c3", "deliver", json!({"trial": trial}));
    step(&run).await;
    decide_all(&run).await;
    step(&run).await;
    let told = run.agent.inputs().last().unwrap().clone();
    assert!(told.contains("opening the pull request failed"), "{told}");
    assert!(
        run.deps
            .judge
            .win_events(run.seed.experiment.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(figures(&run).await.pending_cents, 0);
}
