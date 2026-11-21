#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

//! The terminal relay over real sockets on the in-memory store: who may open one, how the
//! browser's and the runner's ends pair, and when it closes.

use std::sync::Arc;
use std::time::Duration;

use antilog_core::clock::Clock;
use antilog_core::deps::Deps;
use antilog_core::fakes::FakeDeps;
use antilog_core::organization::{self, Principal};
use antilog_core::runner;
use antilog_core::seed::{Seed, organization as seed_organization};
use antilog_core::session::{self, NewSession};
use antilog_core::store::{ApprovalStore, RunnerStore};
use antilog_server::api::auth::{App, Settings};
use antilog_server::api::router_with;
use antilog_server::api::runner::RunnerSettings;
use antilog_server::terminal::Terminals;
use antilog_types::{MachineState, Role, SessionId};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const KEY: &str = "runner-key-for-tests";
const RUNNER: &str = "r1";

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

struct World {
    deps: Arc<FakeDeps>,
    seed: Seed,
    http: reqwest::Client,
    base: String,
    owner: String,
    editor: String,
    viewer: String,
    /// The owner of another organization.
    stranger: String,
}

async fn token(
    deps: &FakeDeps,
    organization: antilog_types::OrganizationId,
    member: antilog_types::MemberId,
) -> String {
    let who = Principal {
        organization,
        member,
    };
    let now = deps.clock().now().as_millisecond();
    organization::create_token(&deps.store, &who, now, None, "t")
        .await
        .unwrap()
        .secret
}

async fn member(deps: &FakeDeps, seed: &Seed, name: &str, role: Role) -> String {
    let who = Principal {
        organization: seed.organization.id,
        member: seed.owner.id,
    };
    let now = deps.clock().now().as_millisecond();
    let added = organization::add_member(&deps.store, &who, now, name, None, role)
        .await
        .unwrap();
    token(deps, seed.organization.id, added.id).await
}

async fn world_with(terminals: Terminals) -> World {
    let deps = Arc::new(FakeDeps::new());
    let seed = seed_organization(&deps.store).await;
    let other = seed_organization(&deps.store).await;
    let core = runner::Settings::new(
        "https://proxy.example/",
        "https://antilog.example",
        "vendor/model-1",
    );
    let app = router_with(Arc::new(App {
        deps: deps.clone(),
        settings: Settings {
            providers: Vec::new(),
            open_organizations: false,
            secure_cookies: true,
            runner: Some(RunnerSettings {
                key: KEY.to_owned(),
                core,
                hold: Duration::ZERO,
            }),
        },
        terminals: Arc::new(terminals),
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await });
    World {
        owner: token(&deps, seed.organization.id, seed.owner.id).await,
        editor: member(&deps, &seed, "ed", Role::Editor).await,
        viewer: member(&deps, &seed, "vi", Role::Viewer).await,
        stranger: token(&deps, other.organization.id, other.owner.id).await,
        deps,
        seed,
        http: reqwest::Client::new(),
        base,
    }
}

async fn world() -> World {
    world_with(Terminals::default()).await
}

impl World {
    async fn post(&self, path: &str, token: Option<&str>, body: Value) -> (u16, Value) {
        let mut request = self
            .http
            .post(format!("http://{}{path}", self.base))
            .json(&body);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.unwrap();
        let status = response.status().as_u16();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    /// A session whose machine a runner holds and has up.
    async fn running(&self) -> (SessionId, String) {
        let session = session::start(
            &*self.deps,
            self.seed.organization.id,
            self.seed.project.id,
            NewSession {
                agent: "opencode".into(),
                model: "vendor/model-2".into(),
                prompt: "hello".into(),
                repos: vec!["acme/api".into()],
                actor: self.seed.owner.id,
                ..NewSession::default()
            },
        )
        .await
        .unwrap();
        let (status, polled) = self.poll(&[]).await;
        assert_eq!(status, 200);
        let spec = &polled["claims"][0];
        assert_eq!(spec["session"], json!(session.id));
        let runner_token = spec["runnerToken"].as_str().unwrap().to_owned();
        self.deps
            .store
            .set_machine_state(session.id, RUNNER, MachineState::Running, 1)
            .await
            .unwrap();
        (session.id, runner_token)
    }

    async fn poll(&self, held: &[SessionId]) -> (u16, Value) {
        self.post(
            "/runner/poll",
            Some(KEY),
            json!({"runner": RUNNER, "capacity": 4, "held": held}),
        )
        .await
    }

    async fn open(&self, who: &str, session: SessionId) -> (u16, Value) {
        self.post(
            &format!("/sessions/{session}/terminals"),
            Some(who),
            json!({"cols": 100, "rows": 30}),
        )
        .await
    }

    async fn browser(
        &self,
        terminal: &str,
        token: &str,
    ) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
        let url = format!(
            "ws://{}/terminals/{terminal}/connect?token={token}",
            self.base
        );
        Ok(tokio_tungstenite::connect_async(url).await?.0)
    }

    async fn machine(
        &self,
        session: SessionId,
        terminal: &str,
        runner_token: &str,
    ) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
        let url = format!(
            "ws://{}/runner/sessions/{session}/terminals/{terminal}",
            self.base
        );
        let mut request = url.into_client_request().unwrap();
        request.headers_mut().insert(
            "authorization",
            format!("Bearer {runner_token}").parse().unwrap(),
        );
        Ok(tokio_tungstenite::connect_async(request).await?.0)
    }
}

async fn next(socket: &mut Socket) -> Option<Message> {
    tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("a frame within five seconds")
        .and_then(Result::ok)
}

fn refused(result: Result<Socket, tokio_tungstenite::tungstenite::Error>) -> u16 {
    match result {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => response.status().as_u16(),
        Err(other) => panic!("not an HTTP refusal: {other}"),
        Ok(_) => 0,
    }
}

#[tokio::test]
async fn an_editor_opens_a_terminal_and_the_two_ends_carry_frames_both_ways() {
    let world = world().await;
    let (session, runner_token) = world.running().await;

    let (status, ticket) = world.open(&world.editor, session).await;
    assert_eq!(status, 201, "{ticket}");
    let terminal = ticket["terminal"].as_str().unwrap().to_owned();
    let browser_token = ticket["token"].as_str().unwrap().to_owned();

    // The machine learns of it on its next poll.
    let (_, polled) = world.poll(&[session]).await;
    let command = &polled["commands"][0]["command"];
    assert_eq!(command["kind"], "open_terminal");
    assert_eq!(command["terminal"], json!(terminal));
    assert_eq!(
        (command["cols"].as_u64(), command["rows"].as_u64()),
        (Some(100), Some(30))
    );

    // The browser may arrive before the machine does.
    let mut browser = world.browser(&terminal, &browser_token).await.unwrap();
    let mut machine = world
        .machine(session, &terminal, &runner_token)
        .await
        .unwrap();

    browser
        .send(Message::Binary(b"echo hi\r".to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        next(&mut machine).await,
        Some(Message::Binary(b"echo hi\r".to_vec().into()))
    );
    machine
        .send(Message::Binary(b"hi\r\n".to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        next(&mut browser).await,
        Some(Message::Binary(b"hi\r\n".to_vec().into()))
    );

    let resize = r#"{"type":"resize","cols":120,"rows":40}"#;
    browser.send(Message::Text(resize.into())).await.unwrap();
    assert_eq!(next(&mut machine).await, Some(Message::Text(resize.into())));

    // Opening it is audited, once, with the member and the session, and nothing typed.
    let log = world
        .deps
        .store
        .audit_log(world.seed.organization.id, None, 100)
        .await
        .unwrap();
    let opened: Vec<_> = log
        .iter()
        .filter(|entry| {
            entry.action == "terminal.opened" && entry.outcome == antilog_types::AuditOutcome::Done
        })
        .collect();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].target, session.to_string());
    assert!(opened[0].actor.is_some());
    assert!(!format!("{log:?}").contains("echo hi"));

    // The browser leaving closes the machine's end.
    browser.close(None).await.ok();
    assert!(matches!(
        next(&mut machine).await,
        None | Some(Message::Close(_))
    ));
}

#[tokio::test]
async fn who_may_open_one_and_what_each_end_must_prove() {
    let world = world().await;
    let (session, runner_token) = world.running().await;

    assert_eq!(
        world.open(&world.viewer, session).await.0,
        403,
        "a viewer may read, not type"
    );
    assert_eq!(
        world.open(&world.stranger, session).await.0,
        404,
        "another organization sees nothing"
    );
    assert_eq!(
        world
            .post(
                &format!("/sessions/{session}/terminals"),
                None,
                json!({"cols": 80, "rows": 24})
            )
            .await
            .0,
        401
    );
    let log = world
        .deps
        .store
        .audit_log(world.seed.organization.id, None, 100)
        .await
        .unwrap();
    assert!(
        log.iter()
            .filter(|entry| entry.action == "terminal.opened")
            .all(|entry| entry.outcome == antilog_types::AuditOutcome::Denied),
        "a refused open is audited as a denial, never as an opening"
    );

    let (status, ticket) = world.open(&world.owner, session).await;
    assert_eq!(status, 201);
    let terminal = ticket["terminal"].as_str().unwrap().to_owned();
    let good = ticket["token"].as_str().unwrap().to_owned();

    assert_eq!(
        refused(world.browser(&terminal, "wrong").await),
        404,
        "a wrong token"
    );
    assert_eq!(
        refused(
            world
                .browser(&uuid::Uuid::new_v4().to_string(), &good)
                .await
        ),
        404,
        "another terminal"
    );
    assert_eq!(
        refused(world.machine(session, &terminal, "wrong").await),
        401,
        "a wrong runner token"
    );

    // A runner token proves one session only.
    let (other, other_token) = {
        session::start(
            &*world.deps,
            world.seed.organization.id,
            world.seed.project.id,
            NewSession {
                agent: "opencode".into(),
                model: "vendor/model-2".into(),
                prompt: "second".into(),
                actor: world.seed.owner.id,
                ..NewSession::default()
            },
        )
        .await
        .unwrap();
        let (_, polled) = world.poll(&[session]).await;
        let spec = &polled["claims"][0];
        (
            serde_json::from_value::<SessionId>(spec["session"].clone()).unwrap(),
            spec["runnerToken"].as_str().unwrap().to_owned(),
        )
    };
    assert_eq!(
        refused(world.machine(other, &terminal, &other_token).await),
        404,
        "a terminal of another session"
    );
    assert_eq!(
        refused(world.machine(session, &terminal, &other_token).await),
        401,
        "another session's token"
    );

    let _first = world.browser(&terminal, &good).await.unwrap();
    assert_eq!(
        refused(world.browser(&terminal, &good).await),
        404,
        "the token works once"
    );
    let _machine = world
        .machine(session, &terminal, &runner_token)
        .await
        .unwrap();
    assert_eq!(
        refused(world.machine(session, &terminal, &runner_token).await),
        404,
        "so does the machine's end"
    );
}

#[tokio::test]
async fn a_machine_that_is_not_running_takes_no_terminal() {
    let world = world().await;
    let session = session::start(
        &*world.deps,
        world.seed.organization.id,
        world.seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "vendor/model-2".into(),
            prompt: "hello".into(),
            actor: world.seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    // Queued: no runner holds it yet.
    let (status, body) = world.open(&world.owner, session.id).await;
    assert_eq!((status, body["code"].as_str()), (409, Some("not_running")));
    let log = world
        .deps
        .store
        .audit_log(world.seed.organization.id, None, 100)
        .await
        .unwrap();
    assert!(!log.iter().any(|entry| entry.action == "terminal.opened"));
}

#[tokio::test]
async fn stopping_the_session_closes_its_terminals() {
    let world = world().await;
    let (session, runner_token) = world.running().await;
    let (_, ticket) = world.open(&world.owner, session).await;
    let terminal = ticket["terminal"].as_str().unwrap().to_owned();
    let mut browser = world
        .browser(&terminal, ticket["token"].as_str().unwrap())
        .await
        .unwrap();
    let mut machine = world
        .machine(session, &terminal, &runner_token)
        .await
        .unwrap();
    machine
        .send(Message::Binary(b"x".to_vec().into()))
        .await
        .unwrap();
    assert!(matches!(next(&mut browser).await, Some(Message::Binary(_))));

    let (status, _) = world
        .post(
            &format!("/sessions/{session}/stop"),
            Some(&world.owner),
            json!({}),
        )
        .await;
    assert_eq!(status, 200);
    assert!(matches!(
        next(&mut browser).await,
        None | Some(Message::Close(_))
    ));
    assert!(matches!(
        next(&mut machine).await,
        None | Some(Message::Close(_))
    ));
}

#[tokio::test]
async fn the_machine_letting_go_closes_its_terminals() {
    let world = world().await;
    let (session, runner_token) = world.running().await;
    let (_, ticket) = world.open(&world.owner, session).await;
    let terminal = ticket["terminal"].as_str().unwrap().to_owned();
    let mut browser = world
        .browser(&terminal, ticket["token"].as_str().unwrap())
        .await
        .unwrap();
    let _machine = world
        .machine(session, &terminal, &runner_token)
        .await
        .unwrap();
    let (status, _) = world
        .post(
            &format!("/runner/sessions/{session}/release"),
            Some(&runner_token),
            json!({"outcome": "idle", "export": "{}"}),
        )
        .await;
    assert_eq!(status, 204);
    assert!(matches!(
        next(&mut browser).await,
        None | Some(Message::Close(_))
    ));
}

#[tokio::test]
async fn a_terminal_nobody_uses_closes_after_the_idle_time() {
    let world = world_with(Terminals::with_limits(
        Duration::from_millis(300),
        Duration::from_secs(30),
        Duration::from_secs(60),
    ))
    .await;
    let (session, runner_token) = world.running().await;
    let (_, ticket) = world.open(&world.owner, session).await;
    let terminal = ticket["terminal"].as_str().unwrap().to_owned();
    let mut browser = world
        .browser(&terminal, ticket["token"].as_str().unwrap())
        .await
        .unwrap();
    let _machine = world
        .machine(session, &terminal, &runner_token)
        .await
        .unwrap();
    assert!(matches!(
        next(&mut browser).await,
        None | Some(Message::Close(_))
    ));
}

#[tokio::test]
async fn a_machine_that_never_connects_ends_the_browsers_wait() {
    let world = world_with(Terminals::with_limits(
        Duration::from_secs(60),
        Duration::from_millis(300),
        Duration::from_secs(60),
    ))
    .await;
    let (session, _) = world.running().await;
    let (_, ticket) = world.open(&world.owner, session).await;
    let terminal = ticket["terminal"].as_str().unwrap().to_owned();
    let mut browser = world
        .browser(&terminal, ticket["token"].as_str().unwrap())
        .await
        .unwrap();
    assert!(matches!(
        next(&mut browser).await,
        None | Some(Message::Close(_))
    ));
}

#[tokio::test]
async fn an_unclaimed_token_lapses() {
    let world = world_with(Terminals::with_limits(
        Duration::from_secs(60),
        Duration::from_secs(30),
        Duration::from_millis(100),
    ))
    .await;
    let (session, _) = world.running().await;
    let (_, ticket) = world.open(&world.owner, session).await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(
        refused(
            world
                .browser(
                    ticket["terminal"].as_str().unwrap(),
                    ticket["token"].as_str().unwrap()
                )
                .await
        ),
        404
    );
}

#[tokio::test]
async fn an_idle_poll_waits_for_a_terminal_and_answers_when_one_is_asked_for() {
    let deps = Arc::new(FakeDeps::new());
    let seed = seed_organization(&deps.store).await;
    let core = runner::Settings::new(
        "https://proxy.example/",
        "https://antilog.example",
        "vendor/model-1",
    );
    let app = router_with(Arc::new(App {
        deps: deps.clone(),
        settings: Settings {
            providers: Vec::new(),
            open_organizations: false,
            secure_cookies: true,
            runner: Some(RunnerSettings {
                key: KEY.to_owned(),
                core,
                hold: Duration::from_secs(10),
            }),
        },
        terminals: Arc::default(),
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, app).await });
    let http = reqwest::Client::new();
    let owner = token(&deps, seed.organization.id, seed.owner.id).await;
    let post = move |path: String, token: String, body: Value| {
        let http = http.clone();
        let base = base.clone();
        async move {
            http.post(format!("http://{base}{path}"))
                .bearer_auth(token)
                .json(&body)
                .send()
                .await
                .unwrap()
                .json::<Value>()
                .await
                .unwrap_or(Value::Null)
        }
    };
    let session = session::start(
        &*deps,
        seed.organization.id,
        seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "vendor/model-2".into(),
            prompt: "hi".into(),
            actor: seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    let claim = post(
        "/runner/poll".into(),
        KEY.into(),
        json!({"runner": RUNNER, "capacity": 4, "held": []}),
    )
    .await;
    assert_eq!(claim["claims"].as_array().unwrap().len(), 1);
    deps.store
        .set_machine_state(session.id, RUNNER, MachineState::Running, 1)
        .await
        .unwrap();

    let started = std::time::Instant::now();
    let held = tokio::spawn({
        let post = post.clone();
        async move {
            post(
                "/runner/poll".into(),
                KEY.into(),
                json!({"runner": RUNNER, "capacity": 0, "held": [session.id]}),
            )
            .await
        }
    });
    tokio::time::sleep(Duration::from_millis(200)).await;
    let opened = post(
        format!("/sessions/{}/terminals", session.id),
        owner,
        json!({"cols": 80, "rows": 24}),
    )
    .await;
    assert!(opened["terminal"].is_string());
    let answer = held.await.unwrap();
    assert_eq!(answer["commands"][0]["command"]["kind"], "open_terminal");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "answered at once, not after the hold"
    );
}
