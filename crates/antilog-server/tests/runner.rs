#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! The runner's side of the queue over HTTP, on the in-memory store: claim, events, approvals,
//! release and resume, with the keys and tokens that guard each route.

use std::sync::Arc;

use antilog_core::approval;
use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine};
use antilog_core::organization::Principal;
use antilog_core::runner;
use antilog_core::seed::{Seed, experiment};
use antilog_core::session::{self, NewSession};
use antilog_core::store::{ApprovalStore, RunnerStore, SessionStore};
use antilog_server::api::auth::Settings;
use antilog_server::api::router;
use antilog_server::api::runner::RunnerSettings;
use antilog_types::{
    ApprovalStatus, Command, Decision, MachineState, Session, SessionId, SessionStatus,
};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "runner-key-for-tests";

struct World {
    deps: Arc<FakeDeps>,
    app: Router,
    seed: Seed,
}

async fn world() -> World {
    world_with(None).await
}

/// `lifetime`: the most a claim may hold a machine; omitted, the server's default.
async fn world_with(lifetime: Option<jiff::SignedDuration>) -> World {
    let deps = Arc::new(FakeDeps::new(
        FakeMachine::default(),
        FakeCodeHost::default(),
    ));
    let seed = experiment(&deps.store, 10_000).await;
    let mut core = runner::Settings::new(
        "https://proxy.example/",
        "https://antilog.example",
        "vendor/model-1",
    );
    core.clone_base = "/srv/git/".to_owned();
    if let Some(lifetime) = lifetime {
        core.max_lifetime = lifetime;
    }
    let app = router(
        deps.clone(),
        Settings {
            providers: Vec::new(),
            open_organizations: false,
            secure_cookies: true,
            code_host_webhook_secret: None,
            e2e_seed: false,
            runner: Some(RunnerSettings {
                key: KEY.to_owned(),
                core,
            }),
        },
    );
    World { deps, app, seed }
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = request
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 24)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, value)
}

async fn poll(app: &Router, runner: &str, held: &[&str]) -> Value {
    let (status, body) = call(
        app,
        "POST",
        "/runner/poll",
        Some(KEY),
        json!({"runner": runner, "capacity": 4, "held": held}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn start(world: &World, prompt: &str) -> Session {
    session::start(
        &*world.deps,
        world.seed.organization.id,
        world.seed.experiment.id,
        NewSession {
            agent: "opencode".into(),
            prompt: prompt.into(),
            repos: vec!["acme/api".into()],
            environment: None,
            actor: Some(world.seed.owner.id),
        },
    )
    .await
    .unwrap()
}

fn owner(world: &World) -> Principal {
    Principal {
        organization: world.seed.organization.id,
        member: world.seed.owner.id,
    }
}

#[tokio::test]
async fn the_routes_exist_only_with_a_key_and_each_checks_it() {
    let world = world().await;
    let (status, _) = call(
        &world.app,
        "POST",
        "/runner/poll",
        None,
        json!({"runner": "r", "capacity": 1}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &world.app,
        "POST",
        "/runner/poll",
        Some("wrong"),
        json!({"runner": "r", "capacity": 1}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = call(
        &world.app,
        "POST",
        "/runner/poll",
        Some(&KEY[..KEY.len() - 1]),
        json!({"runner": "r", "capacity": 1}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    poll(&world.app, "r1", &[]).await;

    let bare = router(
        world.deps.clone(),
        Settings {
            providers: Vec::new(),
            open_organizations: false,
            secure_cookies: true,
            code_host_webhook_secret: None,
            e2e_seed: false,
            runner: None,
        },
    );
    let (status, _) = call(
        &bare,
        "POST",
        "/runner/poll",
        Some(KEY),
        json!({"runner": "r", "capacity": 1}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        !antilog_server::api::openapi_json().contains("/runner"),
        "the runner routes stay out of the public document"
    );
}

#[tokio::test]
async fn a_claim_carries_the_spec_the_contract_lists() {
    let world = world().await;
    let session = start(&world, "cut p95").await;
    let polled = poll(&world.app, "r1", &[]).await;
    let spec = &polled["claims"][0];
    assert_eq!(spec["session"], json!(session.id));
    assert_eq!(spec["model"], "vendor/model-1");
    assert_eq!(
        spec["repos"],
        json!([{"name": "acme/api", "url": "/srv/git/acme/api", "token": null}])
    );
    assert_eq!(spec["skills"], json!([]));
    assert_eq!(spec["resume"], false);
    let (model, mcp) = (
        spec["modelToken"].as_str().unwrap(),
        spec["mcpToken"].as_str().unwrap(),
    );
    assert_eq!(
        spec["opencodeConfig"],
        json!({
            "autoupdate": false,
            "small_model": "vendor/model-1",
            "experimental": {"mcp_timeout": 1_800_000},
            "provider": {
                "anthropic": {"options": {"baseURL": "https://proxy.example/anthropic/v1", "apiKey": model}},
                "openai": {"options": {"baseURL": "https://proxy.example/openai/v1", "apiKey": model}},
                "opencode": {"options": {"baseURL": "https://proxy.example/opencode/v1", "apiKey": model}},
            },
            "mcp": {"antilog": {"type": "remote", "url": format!("https://antilog.example/mcp/sessions/{}", session.id), "headers": {"Authorization": format!("Bearer {mcp}")}}},
            "skills": {"paths": ["/opt/antilog/skills"]},
            "permission": {"bash": "allow", "edit": "allow"},
        })
    );
    // The person's first words come with the claim, once.
    assert_eq!(
        polled["commands"],
        json!([{"id": polled["commands"][0]["id"], "session": session.id, "command": {"kind": "message", "text": "cut p95"}}])
    );
    assert_eq!(
        poll(&world.app, "r1", &[&session.id.to_string()]).await["commands"],
        json!([])
    );
    // Another runner finds nothing to take.
    assert_eq!(poll(&world.app, "r2", &[]).await["claims"], json!([]));
}

#[tokio::test]
async fn events_are_recorded_once_and_a_permission_becomes_a_card_that_answers_back() {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = start(&world, "go").await;
    let spec = poll(&world.app, "r1", &[]).await["claims"][0].clone();
    let token = spec["runnerToken"].as_str().unwrap();
    let events = format!("/runner/sessions/{}/events", session.id);

    let words = json!({"items": [
        {"type": "words", "key": "m1:p1", "kind": "agent", "text": "Looking at the handler."},
        {"type": "words", "key": "m1:p2", "kind": "tool", "text": "read"},
        {"type": "status", "busy": true},
    ]});
    for _ in 0..2 {
        let (status, _) = call(&world.app, "POST", &events, Some(token), words.clone()).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }
    let page = session::transcript(&world.deps.store, org, session.id, None, None)
        .await
        .unwrap();
    let texts: Vec<_> = page.events.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, ["go", "Looking at the handler.", "read"]);
    assert_eq!(
        world
            .deps
            .store
            .machine(session.id)
            .await
            .unwrap()
            .unwrap()
            .state,
        MachineState::Running
    );

    // The agent asks for bash: a card appears once and the session waits for a person.
    let ask = json!({"items": [{"type": "permission", "id": "per_1", "tool": "bash", "detail": "cargo test"}]});
    for _ in 0..2 {
        call(&world.app, "POST", &events, Some(token), ask.clone()).await;
    }
    let cards = world.deps.store.pending_approvals(org).await.unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(
        world
            .deps
            .store
            .session(org, session.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        SessionStatus::WaitingForYou
    );
    // A status report does not hide that a person still has to answer.
    call(
        &world.app,
        "POST",
        &events,
        Some(token),
        json!({"items": [{"type": "status", "busy": true}]}),
    )
    .await;
    assert_eq!(
        world
            .deps
            .store
            .session(org, session.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        SessionStatus::WaitingForYou
    );

    approval::decide(
        &world.deps.store,
        &owner(&world),
        1,
        cards[0].id,
        Decision::Approve,
    )
    .await
    .unwrap();
    let polled = poll(&world.app, "r1", &[&session.id.to_string()]).await;
    assert_eq!(
        polled["commands"][0]["command"],
        json!({"kind": "approval", "permission": "per_1", "reply": "once"})
    );
    assert_eq!(
        world
            .deps
            .store
            .session(org, session.id)
            .await
            .unwrap()
            .unwrap()
            .status,
        SessionStatus::Working
    );

    // Declining sends reject.
    call(&world.app, "POST", &events, Some(token), json!({"items": [{"type": "permission", "id": "per_2", "tool": "bash", "detail": "rm -rf /"}]})).await;
    let card = world
        .deps
        .store
        .pending_approvals(org)
        .await
        .unwrap()
        .remove(0);
    let decided = approval::decide(
        &world.deps.store,
        &owner(&world),
        2,
        card.id,
        Decision::Decline,
    )
    .await
    .unwrap();
    assert_eq!(decided.status, ApprovalStatus::Declined);
    let polled = poll(&world.app, "r1", &[&session.id.to_string()]).await;
    assert_eq!(polled["commands"][0]["command"]["reply"], "reject");
}

#[tokio::test]
async fn a_session_token_opens_only_its_own_session() {
    let world = world().await;
    let (one, two) = (start(&world, "a").await, start(&world, "b").await);
    let claims = poll(&world.app, "r1", &[]).await["claims"].clone();
    let token_of = |id: SessionId| {
        claims
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["session"] == json!(id))
            .unwrap()["runnerToken"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let (token_one, token_two) = (token_of(one.id), token_of(two.id));
    let uri = format!("/runner/sessions/{}/events", one.id);
    let items = json!({"items": []});
    assert_eq!(
        call(&world.app, "POST", &uri, Some(&token_two), items.clone())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&world.app, "POST", &uri, Some(KEY), items.clone())
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&world.app, "POST", &uri, None, items.clone()).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&world.app, "POST", &uri, Some(&token_one), items)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn a_lapsed_lease_goes_to_another_runner_and_the_old_one_is_told_to_stop() {
    let world = world().await;
    let session = start(&world, "a").await;
    let first = poll(&world.app, "r1", &[]).await;
    let old_token = first["claims"][0]["runnerToken"]
        .as_str()
        .unwrap()
        .to_owned();
    // Past the 60-second lease, with no renewal.
    world
        .deps
        .clock
        .advance(jiff::SignedDuration::from_secs(61));
    let second = poll(&world.app, "r2", &[]).await;
    assert_eq!(second["claims"][0]["session"], json!(session.id));
    // The unfinished first message is sent again, to the new holder only.
    assert_eq!(second["commands"][0]["command"]["text"], "a");
    let late = poll(&world.app, "r1", &[&session.id.to_string()]).await;
    assert_eq!(late["lost"], json!([session.id]));
    // The first runner's token no longer works.
    let uri = format!("/runner/sessions/{}/events", session.id);
    assert_eq!(
        call(
            &world.app,
            "POST",
            &uri,
            Some(&old_token),
            json!({"items": []})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn release_keeps_the_export_ends_the_tokens_and_a_message_resumes_it() {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = start(&world, "a").await;
    let spec = poll(&world.app, "r1", &[]).await["claims"][0].clone();
    let token = spec["runnerToken"].as_str().unwrap();
    let release = format!("/runner/sessions/{}/release", session.id);
    let (status, _) = call(
        &world.app,
        "POST",
        &release,
        Some(token),
        json!({"outcome": "idle", "export": "{\"info\":{}}"}),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let machine = world.deps.store.machine(session.id).await.unwrap().unwrap();
    assert_eq!(machine.state, MachineState::Released);
    // Every token the session held is dead, the model's too.
    let (status, _) = call(
        &world.app,
        "POST",
        &release,
        Some(token),
        json!({"outcome": "idle"}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    use antilog_core::credentials::{Purpose, SessionCredentials};
    assert_eq!(
        world
            .deps
            .credentials
            .verify(spec["modelToken"].as_str().unwrap(), Purpose::Model)
            .await
            .unwrap(),
        None
    );
    assert_eq!(poll(&world.app, "r1", &[]).await["claims"], json!([]));

    // A message queues it again; the next claim says to resume and serves the export.
    session::message(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "keep going",
    )
    .await
    .unwrap();
    let again = poll(&world.app, "r2", &[]).await;
    let spec = &again["claims"][0];
    assert_eq!(spec["resume"], true);
    assert_ne!(spec["runnerToken"].as_str().unwrap(), token);
    assert_eq!(again["commands"][0]["command"]["text"], "keep going");
    let uri = format!("/runner/sessions/{}/export", session.id);
    let request = Request::get(&uri)
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", spec["runnerToken"].as_str().unwrap()),
        )
        .body(Body::empty())
        .unwrap();
    let response = world.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    assert_eq!(&body[..], br#"{"info":{}}"#);

    // Stopping tells the machine to release for good, and the session is not claimed again.
    session::stop(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "Ada",
    )
    .await
    .unwrap();
    let last = poll(&world.app, "r2", &[&session.id.to_string()]).await;
    assert_eq!(
        last["commands"][0]["command"],
        json!({"kind": "release", "outcome": "done"})
    );
    let spec_token = spec["runnerToken"].as_str().unwrap();
    call(
        &world.app,
        "POST",
        &release,
        Some(spec_token),
        json!({"outcome": "done", "export": null}),
    )
    .await;
    assert_eq!(
        world
            .deps
            .store
            .machine(session.id)
            .await
            .unwrap()
            .unwrap()
            .state,
        MachineState::Done
    );
    let _: Command = serde_json::from_value(last["commands"][0]["command"].clone()).unwrap();
}

#[tokio::test]
async fn a_failed_release_marks_the_session_failed() {
    let world = world().await;
    let session = start(&world, "a").await;
    let spec = poll(&world.app, "r1", &[]).await["claims"][0].clone();
    let uri = format!("/runner/sessions/{}/release", session.id);
    call(
        &world.app,
        "POST",
        &uri,
        spec["runnerToken"].as_str(),
        json!({"outcome": "failed"}),
    )
    .await;
    let row = world
        .deps
        .store
        .session(world.seed.organization.id, session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SessionStatus::Failed);
}

#[tokio::test]
async fn a_claim_carries_the_environment_version_the_session_started_on() {
    use antilog_core::environments as env;
    use antilog_types::{EnvironmentSpec, NetworkLevel, NewEnvironment};

    let world = world().await;
    let who = owner(&world);
    let new = |network, hosts: &[&str]| NewEnvironment {
        name: "Bench".into(),
        network,
        allowed_hosts: hosts.iter().map(|h| (*h).to_owned()).collect(),
        allow_package_managers: false,
        variables: [("STRIPE_KEY".to_owned(), "sk_test".to_owned())].into(),
        setup_script: "npm ci".into(),
        cpus: 4,
        memory_mb: 16384,
    };
    env::create(
        &world.deps.store,
        &who,
        1,
        new(NetworkLevel::Limited, &["api.stripe.com"]),
    )
    .await
    .unwrap();
    let on = |environment: Option<&str>| NewSession {
        agent: "opencode".into(),
        prompt: "go".into(),
        repos: vec![],
        environment: environment.map(str::to_owned),
        actor: Some(world.seed.owner.id),
    };
    let org = world.seed.organization.id;
    let exp = world.seed.experiment.id;
    let first = session::start(&*world.deps, org, exp, on(Some("bench")))
        .await
        .unwrap();
    // An edit after the session started does not move it.
    env::edit(
        &world.deps.store,
        &who,
        2,
        "bench",
        new(NetworkLevel::None, &[]),
    )
    .await
    .unwrap();
    let second = session::start(&*world.deps, org, exp, on(Some("bench")))
        .await
        .unwrap();
    let plain = session::start(&*world.deps, org, exp, on(None))
        .await
        .unwrap();

    let polled = poll(&world.app, "r1", &[]).await;
    let spec = |id: antilog_types::SessionId| -> EnvironmentSpec {
        let claim = polled["claims"]
            .as_array()
            .unwrap()
            .iter()
            .find(|claim| claim["session"] == json!(id))
            .expect("claimed");
        serde_json::from_value(claim["environment"].clone()).expect("an environment spec")
    };
    let one = spec(first.id);
    assert_eq!(
        (one.id.as_str(), one.version, one.network),
        ("bench", 1, NetworkLevel::Limited)
    );
    assert_eq!(one.allowed_hosts, vec!["api.stripe.com".to_owned()]);
    assert_eq!(one.variables["STRIPE_KEY"], "sk_test");
    assert_eq!(
        (one.setup_script.as_str(), one.cpus, one.memory_mb),
        ("npm ci", 4, 16384)
    );
    let two = spec(second.id);
    assert_eq!((two.version, two.network), (2, NetworkLevel::None));
    assert_eq!(spec(plain.id), EnvironmentSpec::default());
}

#[tokio::test]
async fn a_machine_that_has_had_its_time_is_let_go_like_a_release_and_a_message_resumes_it() {
    let world = world_with(Some(jiff::SignedDuration::from_mins(2))).await;
    let org = world.seed.organization.id;
    let session = start(&world, "a").await;
    let id = session.id.to_string();
    let spec = poll(&world.app, "r1", &[]).await["claims"][0].clone();
    let token = spec["runnerToken"].as_str().unwrap();
    // Each poll renews the lease; the machine time runs on.
    for _ in 0..2 {
        world
            .deps
            .clock
            .advance(jiff::SignedDuration::from_secs(50));
        let body = poll(&world.app, "r1", &[&id]).await;
        assert!(
            body["commands"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["command"]["kind"] != "release"),
            "{body}"
        );
    }
    world
        .deps
        .clock
        .advance(jiff::SignedDuration::from_secs(50));
    let body = poll(&world.app, "r1", &[&id]).await;
    assert_eq!(
        body["commands"][0]["command"],
        json!({"kind": "release", "outcome": "idle"})
    );
    // Said once, and in plain words.
    let events = world
        .deps
        .store
        .session_events(org, session.id, 0, 100)
        .await
        .unwrap();
    let notice = events
        .iter()
        .find(|event| event.text.contains("2 minutes"))
        .expect("the conversation says why");
    assert!(notice.text.contains("Send a message to continue"));
    let row = world
        .deps
        .store
        .session(org, session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SessionStatus::WaitingForYou);
    let again = poll(&world.app, "r1", &[&id]).await;
    assert_eq!(again["commands"], json!([]));

    // The runner exports and lets go; a message resumes from the export.
    let (status, _) = call(
        &world.app,
        "POST",
        &format!("/runner/sessions/{}/release", session.id),
        Some(token),
        json!({"outcome": "idle", "export": "{\"info\":{}}"}),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    session::message(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "go on",
    )
    .await
    .unwrap();
    let resumed = poll(&world.app, "r2", &[]).await;
    assert_eq!(resumed["claims"][0]["resume"], true);
    // A fresh claim has its whole time again.
    let quiet = poll(&world.app, "r2", &[&id]).await;
    assert!(
        quiet["commands"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["command"]["kind"] != "release")
    );
}
