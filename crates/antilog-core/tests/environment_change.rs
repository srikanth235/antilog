#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! A running session changes its environment: the agent proposes, a person approves, the change
//! is the environment's next version and the session moves to it.

mod common;

use antilog_core::environments;
use antilog_core::fakes::FakeDeps;
use antilog_core::organization::Principal;
use antilog_core::seed::{Seed, T0, organization};
use antilog_core::session::{self, NewSession};
use antilog_core::store::{ApprovalStore, EnvironmentStore, RunnerStore, SessionStore};
use antilog_types::{
    ApprovalStatus, AuditEntry, AuditOutcome, CardPayload, Command, Decision, NetworkLevel,
    NewEnvironment, Session,
};
use common::Agent;
use serde_json::json;

async fn world() -> (FakeDeps, Seed) {
    let deps = FakeDeps::new();
    let seed = organization(&deps.store).await;
    (deps, seed)
}

fn who(seed: &Seed) -> Principal {
    Principal {
        organization: seed.organization.id,
        member: seed.owner.id,
    }
}

fn bench() -> NewEnvironment {
    NewEnvironment {
        name: "Bench".into(),
        network: NetworkLevel::Limited,
        allowed_hosts: vec!["api.stripe.com".into()],
        allow_package_managers: false,
        variables: [("STRIPE_KEY".to_owned(), "sk_test".to_owned())].into(),
        setup_script: "npm ci".into(),
        repos: vec!["acme/app".into()],
        network_secrets: Vec::new(),
        cpus: 2,
        memory_mb: 4096,
    }
}

async fn start(deps: &FakeDeps, seed: &Seed, environment: Option<&str>) -> Agent {
    let session = session::start(
        deps,
        seed.organization.id,
        seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "opencode/test".into(),
            prompt: "go".into(),
            environment: environment.map(str::to_owned),
            actor: seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    Agent::new(session)
}

async fn row(deps: &FakeDeps, seed: &Seed, session: &Session) -> Session {
    deps.store
        .session(seed.organization.id, session.id)
        .await
        .unwrap()
        .unwrap()
}

async fn approve(deps: &FakeDeps, seed: &Seed, agent: &Agent) {
    let org = seed.organization.id;
    let card = deps
        .store
        .approvals(org, agent.session.id)
        .await
        .unwrap()
        .last()
        .unwrap()
        .id;
    let entry = AuditEntry::new(
        org,
        Some(seed.owner.id),
        "approval.decide",
        card,
        AuditOutcome::Approved,
        T0,
    );
    deps.store
        .decide_approval(org, card, Decision::Approve, seed.owner.id, T0, &entry)
        .await
        .unwrap();
    agent.step(deps).await;
}

fn proposal() -> serde_json::Value {
    json!({
        "network": "limited",
        "add_hosts": ["registry.example.com", "API.Example.com"],
        "remove_hosts": ["api.stripe.com"],
        "set_variables": {"REGION": "eu", "STRIPE_KEY": "sk_live"},
        "unset_variables": ["OLD"],
        "setup_script": "npm ci\nnpm run build",
        "add_repos": ["acme/web"],
        "remove_repos": ["acme/app"]
    })
}

#[tokio::test]
async fn the_card_carries_the_whole_proposal_and_the_environment_before_and_after() {
    let (deps, seed) = world().await;
    environments::create(&deps.store, &who(&seed), T0, bench())
        .await
        .unwrap();
    let agent = start(&deps, &seed, Some("bench")).await;
    agent.call("a", "set_environment", proposal());
    agent.step(&deps).await;
    assert_eq!(agent.answer("a")["status"], "waiting for a person");

    let cards = deps
        .store
        .approvals(seed.organization.id, agent.session.id)
        .await
        .unwrap();
    assert_eq!(cards.len(), 1);
    let Some(CardPayload::ChangeEnvironment(proposal)) = &cards[0].payload else {
        panic!("the card has the proposal: {:?}", cards[0].payload);
    };
    let antilog_types::EnvironmentProposal {
        environment,
        creates_environment,
        session_version,
        base_version,
        change,
        before,
        after,
        ..
    } = &**proposal;
    assert_eq!(environment, "bench");
    assert!(!creates_environment);
    assert_eq!((*session_version, *base_version), (1, 1));
    assert_eq!(
        change.setup_script.as_deref(),
        Some("npm ci\nnpm run build")
    );
    assert_eq!(change.set_variables["STRIPE_KEY"], "sk_live");
    assert_eq!(before.allowed_hosts, ["api.stripe.com"]);
    assert_eq!(before.variables["STRIPE_KEY"], "sk_test");
    assert_eq!(
        after.allowed_hosts,
        ["registry.example.com", "api.example.com"]
    );
    assert_eq!(after.variables["STRIPE_KEY"], "sk_live");
    assert_eq!(after.variables["REGION"], "eu");
    assert_eq!(after.setup_script, "npm ci\nnpm run build");
    assert_eq!(after.repos, ["acme/web"]);
    // The summary names counts and no value.
    assert!(
        !cards[0].summary.contains("sk_live"),
        "{}",
        cards[0].summary
    );
    // Nothing changed before approval.
    let now = row(&deps, &seed, &agent.session).await;
    assert_eq!(now.environment_version, Some(1));
}

#[tokio::test]
async fn reserved_names_and_loose_hosts_are_refused_before_a_card_is_raised() {
    let (deps, seed) = world().await;
    let agent = start(&deps, &seed, None).await;
    for (id, input, why) in [
        (
            "a",
            json!({"set_variables": {"ANTILOG_TOKEN": "x"}}),
            "ANTILOG_",
        ),
        (
            "b",
            json!({"set_variables": {"HTTPS_PROXY": "http://evil"}}),
            "proxy",
        ),
        (
            "c",
            json!({"network": "limited", "add_hosts": ["*.example.com"]}),
            "exact host",
        ),
        (
            "d",
            json!({"network": "full", "add_hosts": ["api.example.com"]}),
            "limited",
        ),
        ("e", json!({}), "name what to change"),
        ("f", json!({"network": "wide"}), "none, limited or full"),
    ] {
        agent.call(id, "set_environment", input);
        agent.step(&deps).await;
        let error = agent.answer(id)["error"].as_str().unwrap().to_owned();
        assert!(error.contains(why), "{id}: {error}");
    }
    assert!(
        deps.store
            .approvals(seed.organization.id, agent.session.id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn approving_makes_the_next_version_moves_the_session_and_tells_a_running_machine() {
    let (deps, seed) = world().await;
    let org = seed.organization.id;
    environments::create(&deps.store, &who(&seed), T0, bench())
        .await
        .unwrap();
    let agent = start(&deps, &seed, Some("bench")).await;
    let id = agent.session.id;
    agent.call("a", "set_environment", proposal());
    agent.step(&deps).await;
    let held = deps.store.claim("host", 1, T0, T0 + 60_000).await.unwrap();
    assert_eq!(held.len(), 1);
    approve(&deps, &seed, &agent).await;

    // Version 2 exists, and the session and its machine's seed are on it.
    let newest = deps.store.environments(org).await.unwrap();
    let bench = newest.iter().find(|e| e.id == "bench").unwrap();
    assert_eq!(bench.version, 2);
    assert_eq!(bench.setup_script, "npm ci\nnpm run build");
    assert_eq!(bench.variables["REGION"], "eu");
    let moved = row(&deps, &seed, &agent.session).await;
    assert_eq!(moved.environment.as_deref(), Some("bench"));
    assert_eq!(moved.environment_version, Some(2));
    let machine = deps.store.machine(id).await.unwrap().unwrap();
    assert_eq!(machine.seed.environment_version, Some(2));
    assert!(machine.seed.repos.contains(&"acme/web".to_owned()));

    // The machine is told the version, and the agent what applies now and what later.
    let commands = deps.store.take_commands("host").await.unwrap();
    let applied: Vec<_> = commands
        .iter()
        .filter_map(|e| match &e.command {
            Command::ApplyEnvironment { environment } => Some(environment),
            _ => None,
        })
        .collect();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].version, 2);
    assert_eq!(applied[0].variables["STRIPE_KEY"], "sk_live");
    assert_eq!(applied[0].setup_script, "npm ci\nnpm run build");
    let told = agent.inputs().join("\n");
    for want in [
        "version 2 of Bench",
        "other sessions on that environment get it when they start",
        "The network change applies now",
        "The variables are set for commands run from now on",
        "The setup script is running now",
    ] {
        assert!(told.contains(want), "{want} is missing from {told}");
    }
    // Audited: the new version and the move, by the person who approved.
    let log = deps.store.audit_log(org, None, 200).await.unwrap();
    for action in ["environment.edit", "session.set_environment"] {
        assert!(
            log.iter()
                .any(|e| e.action == action && e.actor == Some(seed.owner.id)),
            "{action}"
        );
    }
    // A second step changes nothing twice.
    agent.step(&deps).await;
    let again = deps.store.environments(org).await.unwrap();
    assert_eq!(again.iter().find(|e| e.id == "bench").unwrap().version, 2);
}

#[tokio::test]
async fn a_session_on_the_default_gets_an_environment_of_its_own() {
    let (deps, seed) = world().await;
    let org = seed.organization.id;
    let agent = start(&deps, &seed, None).await;
    agent.call(
        "a",
        "set_environment",
        json!({"set_variables": {"REGION": "eu"}}),
    );
    agent.step(&deps).await;
    let card = &deps.store.approvals(org, agent.session.id).await.unwrap()[0];
    let Some(CardPayload::ChangeEnvironment(proposal)) = &card.payload else {
        panic!("a change environment card");
    };
    let (creates_environment, environment_name) =
        (&proposal.creates_environment, &proposal.environment_name);
    assert!(creates_environment);
    approve(&deps, &seed, &agent).await;
    let moved = row(&deps, &seed, &agent.session).await;
    assert!(moved.environment.is_some_and(|id| !id.is_empty()));
    assert_eq!(moved.environment_version, Some(1));
    let all = deps.store.environments(org).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, *environment_name);
    assert_eq!(all[0].variables["REGION"], "eu");
    // The machine is not held yet: it applies when it next starts.
    assert!(
        agent
            .inputs()
            .join(" ")
            .contains("when the machine next starts")
    );
    let machine = deps.store.machine(agent.session.id).await.unwrap().unwrap();
    assert_eq!(
        machine.seed.environment.as_deref(),
        Some(all[0].id.as_str())
    );
}

#[tokio::test]
async fn a_person_moves_a_session_to_another_environment() {
    let (deps, seed) = world().await;
    let org = seed.organization.id;
    environments::create(&deps.store, &who(&seed), T0, bench())
        .await
        .unwrap();
    let agent = start(&deps, &seed, None).await;
    let id = agent.session.id;
    let held = deps.store.claim("host", 1, T0, T0 + 60_000).await.unwrap();
    assert_eq!(held.len(), 1);
    let moved = session::move_to_environment(&deps, org, id, Some(seed.owner.id), "bench")
        .await
        .unwrap();
    assert_eq!(moved.environment.as_deref(), Some("bench"));
    assert_eq!(moved.environment_version, Some(1));
    let commands = deps.store.take_commands("host").await.unwrap();
    assert!(commands.iter().any(|e| matches!(
        &e.command,
        Command::ApplyEnvironment { environment } if environment.id == "bench"
    )));
    // Its default repo is cloned, and the agent is told.
    assert!(commands.iter().any(|e| matches!(
        &e.command,
        Command::AttachRepo { repo } if repo.name == "acme/app"
    )));
    assert!(commands.iter().any(|e| matches!(
        &e.command,
        Command::Message { text } if text.contains("moved this session to the environment Bench")
    )));
    // The same environment again changes nothing; an unknown or archived one is refused.
    let again = session::move_to_environment(&deps, org, id, Some(seed.owner.id), "bench")
        .await
        .unwrap();
    assert_eq!(again, moved);
    assert!(
        session::move_to_environment(&deps, org, id, Some(seed.owner.id), "nope")
            .await
            .is_err()
    );
    environments::archive(&deps.store, &who(&seed), T0, "bench")
        .await
        .unwrap();
    session::move_to_environment(&deps, org, id, Some(seed.owner.id), "default")
        .await
        .unwrap();
    assert!(
        session::move_to_environment(&deps, org, id, Some(seed.owner.id), "bench")
            .await
            .is_err(),
        "an archived environment is not offered"
    );
    let back = row(&deps, &seed, &agent.session).await;
    assert_eq!(back.environment, None);
    let status = approvals_pending(&deps, &seed, &agent).await;
    assert_eq!(status, 0);
}

async fn approvals_pending(deps: &FakeDeps, seed: &Seed, agent: &Agent) -> usize {
    deps.store
        .approvals(seed.organization.id, agent.session.id)
        .await
        .unwrap()
        .iter()
        .filter(|a| a.status == ApprovalStatus::Pending)
        .count()
}

#[tokio::test]
async fn policy_may_pre_approve_it() {
    use antilog_types::{CardKind, Policy, PolicyMode};
    let (deps, seed) = world().await;
    let org = seed.organization.id;
    antilog_core::approval::set_policy(
        &deps.store,
        &who(&seed),
        T0,
        Policy {
            card: CardKind::ChangeEnvironment,
            mode: PolicyMode::Allow,
            limit_cents: None,
        },
    )
    .await
    .unwrap();
    let session = session::start(
        &deps,
        org,
        seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "opencode/test".into(),
            prompt: "go".into(),
            follow_policy: true,
            actor: seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    let agent = Agent::new(session);
    agent.call(
        "a",
        "set_environment",
        json!({"set_variables": {"REGION": "eu"}}),
    );
    agent.step(&deps).await;
    assert_eq!(agent.answer("a")["status"], "approved by policy");
    let moved = row(&deps, &seed, &agent.session).await;
    assert!(moved.environment.is_some(), "policy applied it at once");
    assert_eq!(deps.store.environments(org).await.unwrap().len(), 1);
}
