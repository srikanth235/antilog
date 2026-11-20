//! A runner against the real server routes on the in-memory store, with a fake OpenCode HTTP
//! server in place of the binary and a local git repo to clone.

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use antilog_core::approval;
use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine};
use antilog_core::organization::Principal;
use antilog_core::runner::Settings as CoreSettings;
use antilog_core::seed::{Seed, experiment};
use antilog_core::session::{self, NewSession};
use antilog_core::store::{ApprovalStore, RunnerStore, SessionStore};
use antilog_server::api::auth::Settings;
use antilog_server::api::router;
use antilog_server::api::runner::RunnerSettings;
use antilog_types::judge::{CommitRef, DatasetRef};
use antilog_types::{Command, DataSpec, Decision, MachineState, RunSpec, Session, SessionStatus};
use axum::body::{Body, Bytes};
use axum::extract::{Path as UrlPath, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use super::sandbox::plain::{Plain, PlainMachine};
use super::sandbox::{Exec, Listener, Machine, Output, Paths, Sandbox, SandboxSpec};
use super::{Config, Opencode, run};

const KEY: &str = "runner-key";

// ---- the fake OpenCode --------------------------------------------------------------------

#[derive(Default)]
struct Fake {
    prompts: Vec<Value>,
    replies: Vec<(String, String)>,
    aborts: u32,
    /// Messages as `GET /session/{id}/message` lists them, oldest first.
    messages: Vec<Value>,
    permissions: Vec<Value>,
    status: Option<&'static str>,
    streams: Vec<mpsc::Sender<Result<Bytes, Infallible>>>,
    streams_opened: u32,
    imported: Vec<String>,
    /// Every request's authorization header.
    auth: Vec<String>,
}

type Shared = Arc<Mutex<Fake>>;

fn locked(shared: &Shared) -> std::sync::MutexGuard<'_, Fake> {
    shared.lock().unwrap()
}

async fn serve_fake(shared: Shared) -> String {
    async fn note(shared: &Shared, headers: &HeaderMap) {
        let auth = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        locked(shared).auth.push(auth);
    }
    let app =
        Router::new()
            .route(
                "/global/health",
                get(|| async { Json(json!({"healthy": true})) }),
            )
            .route(
                "/session",
                post(|State(s): State<Shared>, h: HeaderMap| async move {
                    note(&s, &h).await;
                    Json(json!({"id": "ses_fake"}))
                }),
            )
            .route(
                "/session/{id}/prompt_async",
                post(
                    |State(s): State<Shared>, Json(body): Json<Value>| async move {
                        locked(&s).prompts.push(body);
                        axum::http::StatusCode::NO_CONTENT
                    },
                ),
            )
            .route(
                "/session/{id}/abort",
                post(|State(s): State<Shared>| async move {
                    locked(&s).aborts += 1;
                    Json(true)
                }),
            )
            .route(
                "/session/{id}/message",
                get(|State(s): State<Shared>| async move { Json(locked(&s).messages.clone()) }),
            )
            .route(
                "/session/status",
                get(|State(s): State<Shared>| async move {
                    let status = locked(&s).status;
                    Json(match status {
                        Some(kind) => json!({"ses_fake": {"type": kind}}),
                        None => json!({}),
                    })
                }),
            )
            .route(
                "/permission",
                get(|State(s): State<Shared>| async move { Json(locked(&s).permissions.clone()) }),
            )
            .route(
                "/permission/{id}/reply",
                post(
                    |State(s): State<Shared>,
                     UrlPath(id): UrlPath<String>,
                     Json(body): Json<Value>| async move {
                        locked(&s)
                            .replies
                            .push((id, body["reply"].as_str().unwrap().to_owned()));
                        Json(true)
                    },
                ),
            )
            .route(
                "/event",
                get(|State(s): State<Shared>| async move {
                    let (tx, rx) = mpsc::channel(64);
                    {
                        let mut fake = locked(&s);
                        fake.streams.push(tx);
                        fake.streams_opened += 1;
                    }
                    let mut response: Response =
                        Body::from_stream(ReceiverStream::new(rx)).into_response();
                    response.headers_mut().insert(
                        "content-type",
                        HeaderValue::from_static("text/event-stream"),
                    );
                    response
                }),
            )
            .with_state(shared);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    base
}

async fn emit(shared: &Shared, event: Value) {
    let line = Bytes::from(format!("data: {event}\n\n"));
    let streams = locked(shared).streams.clone();
    for stream in streams {
        stream.send(Ok(line.clone())).await.ok();
    }
}

fn assistant_text(message: &str, part: &str, text: &str) -> Value {
    json!({"type": "message.part.updated", "properties": {"sessionID": "ses_fake", "part": {
        "id": part, "messageID": message, "sessionID": "ses_fake", "type": "text",
        "text": text, "time": {"start": 1, "end": 2}}}})
}

fn assistant_message(message: &str) -> Value {
    json!({"type": "message.updated", "properties": {"sessionID": "ses_fake", "info": {"id": message, "role": "assistant"}}})
}

/// A machine on this host whose `opencode` is the fake HTTP server: `serve` does nothing and the
/// CLI answers from `Fake`.
struct FakeSandbox {
    base: String,
    shared: Shared,
    plain: Plain,
}

struct ScriptedMachine {
    base: String,
    shared: Shared,
    plain: PlainMachine,
}

fn fake_opencode() -> Opencode {
    Opencode {
        binary: "opencode".into(),
        pure: false,
        disable_project_config: false,
    }
}

impl Sandbox for FakeSandbox {
    type Machine = ScriptedMachine;

    async fn start(&self, spec: &SandboxSpec) -> anyhow::Result<ScriptedMachine> {
        Ok(ScriptedMachine {
            base: self.base.clone(),
            shared: self.shared.clone(),
            plain: self.plain.start(spec).await?,
        })
    }
}

impl Machine for ScriptedMachine {
    fn paths(&self) -> &Paths {
        self.plain.paths()
    }

    async fn exec(&self, exec: Exec) -> anyhow::Result<Output> {
        let words: Vec<&str> = exec.args.iter().map(String::as_str).collect();
        match words.as_slice() {
            ["opencode", "export", session, ..] => {
                assert!(
                    exec.env
                        .iter()
                        .any(|(k, v)| k == "OPENCODE_SERVER_PASSWORD" && !v.is_empty())
                );
                Ok(Output {
                    stdout: json!({"info": {"id": session}, "messages": []})
                        .to_string()
                        .into_bytes(),
                    ..Output::default()
                })
            }
            ["opencode", "import", file, ..] => {
                locked(&self.shared)
                    .imported
                    .push(std::fs::read_to_string(file)?);
                Ok(Output {
                    stdout: b"imported ses_fake".to_vec(),
                    ..Output::default()
                })
            }
            _ => self.plain.exec(exec).await,
        }
    }

    async fn spawn(&self, exec: Exec) -> anyhow::Result<()> {
        if exec.args.first().map(String::as_str) == Some("opencode") {
            assert!(
                exec.env
                    .iter()
                    .any(|(k, v)| k == "OPENCODE_SERVER_PASSWORD" && !v.is_empty())
            );
            return Ok(());
        }
        self.plain.spawn(exec).await
    }

    async fn listener(&self) -> anyhow::Result<Listener> {
        Ok(Listener {
            bind: "127.0.0.1".into(),
            port: 0,
            base: self.base.clone(),
        })
    }

    async fn stop(&self) {
        self.plain.stop().await;
    }
}

// ---- the world ------------------------------------------------------------------------------

struct World {
    deps: Arc<FakeDeps>,
    seed: Seed,
    shared: Shared,
    config_path: PathBuf,
    workdir: PathBuf,
    temp: PathBuf,
    runner: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Drop for World {
    fn drop(&mut self) {
        self.runner.abort();
        std::fs::remove_dir_all(&self.temp).ok();
    }
}

fn sh(dir: &Path, script: &str) -> String {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{script}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The server on the in-memory store, and a repo `acme/api` to clone.
async fn backend(proxy: &str, model: &str) -> (Arc<FakeDeps>, Seed, String, PathBuf) {
    let temp = std::env::temp_dir().join(format!("antilog-runner-{}", Uuid::now_v7()));
    let origin = temp.join("origin/acme/api");
    std::fs::create_dir_all(&origin).unwrap();
    sh(
        &origin,
        "git init -q . && git config user.email a@b && git config user.name n && echo base > file.txt && git add . && git commit -qm base",
    );

    let deps = Arc::new(FakeDeps::new(
        FakeMachine::default(),
        FakeCodeHost::default(),
    ));
    let seed = experiment(&deps.store, 10_000).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = format!("http://{}", listener.local_addr().unwrap());
    let mut core = CoreSettings::new(proxy, &server, model);
    core.clone_base = format!("{}/", temp.join("origin").display());
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
    tokio::spawn(async move { axum::serve(listener, app).await });
    (deps, seed, server, temp)
}

async fn world() -> World {
    world_with(|_| {}, Arc::default()).await
}

async fn world_with(
    adjust: impl FnOnce(&mut Config),
    stop: Arc<std::sync::atomic::AtomicBool>,
) -> World {
    let (deps, seed, server, temp) = backend("https://proxy.example", "vendor/model-1").await;
    let shared: Shared = Shared::default();
    let base = serve_fake(shared.clone()).await;
    let mut config = Config::new(&server, KEY);
    config.poll = Duration::from_millis(50);
    adjust(&mut config);
    let plain = Plain::under(&temp);
    let (config_path, workdir) = (plain.config.clone(), plain.workdir.clone());
    let runner = tokio::spawn(super::run_until(
        config,
        fake_opencode(),
        FakeSandbox {
            base,
            shared: shared.clone(),
            plain,
        },
        stop,
    ));
    World {
        deps,
        seed,
        shared,
        config_path,
        workdir,
        temp,
        runner,
    }
}

async fn until<T>(what: &str, mut check: impl AsyncFnMut() -> Option<T>) -> T {
    for _ in 0..400 {
        if let Some(found) = check().await {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {what}");
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

async fn transcript(world: &World, session: &Session) -> Vec<String> {
    session::transcript(
        &world.deps.store,
        world.seed.organization.id,
        session.id,
        None,
        None,
    )
    .await
    .unwrap()
    .events
    .into_iter()
    .map(|e| e.text)
    .collect()
}

async fn state(world: &World, session: &Session) -> MachineState {
    world
        .deps
        .store
        .machine(session.id)
        .await
        .unwrap()
        .unwrap()
        .state
}

// ---- the tests ------------------------------------------------------------------------------

#[tokio::test]
async fn a_session_runs_from_the_queue_through_opencode_and_back() {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = start(&world, "cut p95").await;

    // Claimed, cloned, configured, started; the person's first words reach the agent.
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    let prompt = locked(&world.shared).prompts[0].clone();
    assert_eq!(prompt["parts"][0]["text"], "cut p95");
    assert_eq!(
        prompt["model"],
        json!({"providerID": "vendor", "modelID": "model-1"})
    );
    assert!(
        world
            .workdir
            .join(session.id.to_string())
            .join("api/file.txt")
            .exists()
    );
    let config: Value =
        serde_json::from_slice(&std::fs::read(&world.config_path).unwrap()).unwrap();
    assert_eq!(config["autoupdate"], false);
    assert_eq!(
        config["permission"],
        json!({"bash": "ask", "edit": "allow"})
    );
    assert_eq!(
        config["provider"]["anthropic"]["options"]["baseURL"],
        "https://proxy.example/anthropic/v1"
    );
    assert!(
        locked(&world.shared)
            .auth
            .iter()
            .all(|a| a.starts_with("Basic ")),
        "every call carries OpenCode's password"
    );

    // The agent speaks; the stream's first words are recorded.
    until("a stream", async || {
        (locked(&world.shared).streams_opened > 0).then_some(())
    })
    .await;
    emit(&world.shared, assistant_message("msg_1")).await;
    emit(
        &world.shared,
        assistant_text("msg_1", "prt_1", "Profiling the handler."),
    )
    .await;
    until("the words", async || {
        transcript(&world, &session)
            .await
            .contains(&"Profiling the handler.".to_owned())
            .then_some(())
    })
    .await;

    // A permission becomes a card; the person's answer goes back to OpenCode.
    emit(&world.shared, json!({"type": "permission.asked", "properties": {"id": "per_1", "sessionID": "ses_fake", "permission": "bash", "patterns": ["cargo test"], "metadata": {}, "always": []}})).await;
    let card = until("the card", async || {
        world.deps.store.pending_approvals(org).await.unwrap().pop()
    })
    .await;
    approval::decide(
        &world.deps.store,
        &Principal {
            organization: org,
            member: world.seed.owner.id,
        },
        1,
        card.id,
        Decision::Approve,
    )
    .await
    .unwrap();
    until("the reply", async || {
        (!locked(&world.shared).replies.is_empty()).then_some(())
    })
    .await;
    assert_eq!(
        locked(&world.shared).replies[0],
        ("per_1".to_owned(), "once".to_owned())
    );

    // The stream drops and a part is written while it is down; the rebuild finds it, once.
    let sessions_before = locked(&world.shared).streams_opened;
    {
        let mut fake = locked(&world.shared);
        fake.messages = vec![
            json!({"info": {"id": "msg_1", "role": "assistant"}, "parts": [
                {"id": "prt_1", "messageID": "msg_1", "type": "text", "text": "Profiling the handler.", "time": {"start": 1, "end": 2}},
                {"id": "prt_2", "messageID": "msg_1", "type": "text", "text": "Found it: a quadratic loop.", "time": {"start": 3, "end": 4}},
            ]}),
        ];
        fake.status = Some("idle");
        fake.streams.clear();
    }
    until("a reconnect", async || {
        (locked(&world.shared).streams_opened > sessions_before).then_some(())
    })
    .await;
    until("the missed words", async || {
        transcript(&world, &session)
            .await
            .contains(&"Found it: a quadratic loop.".to_owned())
            .then_some(())
    })
    .await;
    let words = transcript(&world, &session).await;
    assert_eq!(
        words
            .iter()
            .filter(|w| *w == "Profiling the handler.")
            .count(),
        1
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
        SessionStatus::WaitingForYou
    );

    // A person's message and an abort reach OpenCode.
    session::message(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "try the cache",
    )
    .await
    .unwrap();
    until("the message", async || {
        (locked(&world.shared).prompts.len() == 2).then_some(())
    })
    .await;
    assert_eq!(
        locked(&world.shared).prompts[1]["parts"][0]["text"],
        "try the cache"
    );
    world
        .deps
        .store
        .push_command(org, session.id, &Command::Abort, 3)
        .await
        .unwrap();
    until("the abort", async || {
        (locked(&world.shared).aborts == 1).then_some(())
    })
    .await;

    // The judge's command runs in the machine on the commit asked for, and its output comes back.
    let repo = world.workdir.join(session.id.to_string()).join("api");
    let sha = sh(&repo, "git rev-parse HEAD");
    let run = Command::Run {
        run: RunSpec {
            command: "cat file.txt; echo '{\"metric\": 1.5}'".into(),
            commits: vec![CommitRef {
                repo: "acme/api".into(),
                sha,
            }],
            files: vec![],
            data: DataSpec {
                holdout: false,
                dataset: DatasetRef {
                    id: "d".into(),
                    version: 1,
                },
            },
        },
    };
    let id = world
        .deps
        .store
        .push_command(org, session.id, &run, 4)
        .await
        .unwrap();
    let result = until("the run", async || {
        world.deps.store.result(id).await.unwrap()
    })
    .await;
    assert_eq!(
        (result.exit, result.output.as_str()),
        (0, "base\n{\"metric\": 1.5}\n")
    );

    // Stopping releases it: exported, stopped, done.
    session::stop(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "Ada",
    )
    .await
    .unwrap();
    until("done", async || {
        (state(&world, &session).await == MachineState::Done).then_some(())
    })
    .await;
    let export = world.deps.store.export(session.id).await.unwrap().unwrap();
    assert!(export.contains("ses_fake"));
}

#[tokio::test]
async fn a_released_session_comes_back_from_its_export() {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = start(&world, "go").await;
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    until("a stream", async || {
        (locked(&world.shared).streams_opened > 0).then_some(())
    })
    .await;

    let release = Command::Release {
        outcome: antilog_types::ReleaseOutcome::Idle,
    };
    world
        .deps
        .store
        .push_command(org, session.id, &release, 1)
        .await
        .unwrap();
    until("released", async || {
        (state(&world, &session).await == MachineState::Released).then_some(())
    })
    .await;
    assert!(world.deps.store.export(session.id).await.unwrap().is_some());

    session::message(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "keep going",
    )
    .await
    .unwrap();
    until("the second prompt", async || {
        (locked(&world.shared).prompts.len() == 2).then_some(())
    })
    .await;
    assert_eq!(
        locked(&world.shared).imported.len(),
        1,
        "the export was imported"
    );
    assert_eq!(
        locked(&world.shared).prompts[1]["parts"][0]["text"],
        "keep going"
    );
    assert!(
        world
            .deps
            .store
            .machine(session.id)
            .await
            .unwrap()
            .unwrap()
            .resume
    );
    // The clone from the first claim is reused, not cloned again.
    assert!(
        world
            .workdir
            .join(session.id.to_string())
            .join("api/file.txt")
            .exists()
    );
}

#[tokio::test]
async fn a_session_that_cannot_start_is_released_as_failed() {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = session::start(
        &*world.deps,
        org,
        world.seed.experiment.id,
        NewSession {
            agent: "opencode".into(),
            prompt: "go".into(),
            repos: vec!["acme/missing".into()],
            environment: None,
            actor: Some(world.seed.owner.id),
        },
    )
    .await
    .unwrap();
    until("failed", async || {
        (state(&world, &session).await == MachineState::Failed).then_some(())
    })
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
        SessionStatus::Failed
    );
}

/// A second organization's queued session, for a runner that has let its lock go to take.
async fn other_organizations_session(world: &World) -> (antilog_types::OrganizationId, Session) {
    let other = experiment(&world.deps.store, 10_000).await;
    let session = session::start(
        &*world.deps,
        other.organization.id,
        other.experiment.id,
        NewSession {
            agent: "opencode".into(),
            prompt: "go".into(),
            repos: vec![],
            environment: None,
            actor: Some(other.owner.id),
        },
    )
    .await
    .unwrap();
    (other.organization.id, session)
}

#[tokio::test]
async fn a_runner_started_to_exit_leaves_after_its_last_session_and_lets_its_lock_go() {
    let world = world_with(
        |config| {
            config.name = "wipe-me".into();
            config.exit_after_last_session = true;
        },
        Arc::default(),
    )
    .await;
    let org = world.seed.organization.id;
    let session = start(&world, "cut p95").await;
    until("running", async || {
        matches!(
            state(&world, &session).await,
            MachineState::Running | MachineState::Idle
        )
        .then_some(())
    })
    .await;
    // It is locked to this organization: another organization's session is not its to take.
    let (other_org, other) = other_organizations_session(&world).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(state(&world, &other).await, MachineState::Queued);
    assert!(!world.runner.is_finished());

    world
        .deps
        .store
        .push_command(
            org,
            session.id,
            &Command::Release {
                outcome: antilog_types::ReleaseOutcome::Done,
            },
            1,
        )
        .await
        .unwrap();
    until("the runner exits", async || {
        world.runner.is_finished().then_some(())
    })
    .await;
    assert_eq!(state(&world, &session).await, MachineState::Done);
    // Its lock went with it: the name takes any organization's work again.
    let claimed = world
        .deps
        .store
        .claim("wipe-me", 1, 1_900_000_000_000, 1_900_000_060_000)
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].organization, other_org);
}

#[tokio::test]
async fn a_runner_told_to_stop_gives_its_sessions_the_grace_then_stops_them_and_leaves() {
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let world = world_with(
        |config| {
            config.name = "draining".into();
            config.drain_grace = Duration::from_millis(400);
        },
        stop.clone(),
    )
    .await;
    let session = start(&world, "cut p95").await;
    until("running", async || {
        matches!(
            state(&world, &session).await,
            MachineState::Running | MachineState::Idle
        )
        .then_some(())
    })
    .await;
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    // Within the grace it still holds the session, and claims nothing new.
    let (_, other) = other_organizations_session(&world).await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(!world.runner.is_finished());
    assert_eq!(state(&world, &other).await, MachineState::Queued);
    // After the grace it stops the session and exits.
    until("the runner exits", async || {
        world.runner.is_finished().then_some(())
    })
    .await;
    assert_eq!(state(&world, &other).await, MachineState::Queued);
}

#[tokio::test]
async fn a_skill_pack_is_unpacked_into_the_skills_folder() {
    let temp = std::env::temp_dir().join(format!("antilog-skill-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&temp).unwrap();
    let bytes = json!({"files": [
        {"path": "SKILL.md", "content": "# perf\n"},
        {"path": "notes/more.md", "content": "more\n"},
    ]})
    .to_string();
    let escape = json!({"files": [{"path": "../../evil", "content": "x"}]}).to_string();
    let app = Router::new()
        .route(
            "/runner/sessions/{id}/skills/escape/1",
            get(move || {
                let escape = escape.clone();
                async move { escape }
            }),
        )
        .route(
            "/runner/sessions/{id}/skills/perf/3",
            get(move |headers: HeaderMap| {
                let bytes = bytes.clone();
                async move {
                    assert_eq!(headers["authorization"], "Bearer tok");
                    bytes
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });

    let skills_dir = temp.join("skills");
    let machine = PlainMachine::new(
        Paths {
            work: temp.join("work"),
            config: temp.join("opencode.json"),
            skills: skills_dir.clone(),
        },
        Vec::new(),
    );
    let spec = antilog_types::SessionSpec {
        session: antilog_types::SessionId::new(),
        organization: antilog_types::OrganizationId::new(),
        experiment: antilog_types::ExperimentId::new(),
        agent: "opencode".into(),
        model: "v/m".into(),
        repos: vec![],
        environment: antilog_types::EnvironmentSpec::default(),
        skills: vec![
            antilog_types::SkillDownload {
                name: "perf".into(),
                version: "3".into(),
                path: format!("/runner/sessions/{}/skills/perf/3", "x"),
            },
            antilog_types::SkillDownload {
                name: "escape".into(),
                version: "1".into(),
                path: format!("/runner/sessions/{}/skills/escape/1", "x"),
            },
            antilog_types::SkillDownload {
                name: "gone".into(),
                version: "1".into(),
                path: "/runner/sessions/x/skills/gone/1".into(),
            },
        ],
        opencode_config: json!({}),
        model_token: String::new(),
        mcp_token: String::new(),
        runner_token: "tok".into(),
        resume: false,
    };
    super::session::install_skills(&machine, &super::api::Server::new(&server), &spec).await;
    assert_eq!(
        std::fs::read_to_string(skills_dir.join("perf/SKILL.md")).unwrap(),
        "# perf\n"
    );
    assert!(
        !skills_dir.join("gone").exists(),
        "a pack that is not there is skipped"
    );
    std::fs::remove_dir_all(&temp).ok();
}

// ---- the real OpenCode, by hand ---------------------------------------------------------------
//
// `ANTILOG_OPENCODE=/path/to/opencode ANTILOG_OPENCODE_MODEL=provider/model \
//    cargo test -p antilog real_opencode -- --ignored --nocapture`
// runs a session end to end through the pinned OpenCode against a scripted model server that
// asks for `bash` once and then says it is done.

async fn mock_model() -> (String, Arc<Mutex<Vec<Value>>>) {
    let seen: Arc<Mutex<Vec<Value>>> = Arc::default();
    fn stream(blocks: Vec<(&str, Value)>) -> Response {
        let mut text = String::new();
        for (name, data) in blocks {
            text.push_str(&format!("event: {name}\ndata: {data}\n\n"));
        }
        let mut response = text.into_response();
        response.headers_mut().insert(
            "content-type",
            HeaderValue::from_static("text/event-stream"),
        );
        response
    }
    let app = Router::new()
        .route(
            "/anthropic/v1/messages",
            post(|State(seen): State<Arc<Mutex<Vec<Value>>>>, headers: HeaderMap, Json(body): Json<Value>| async move {
                let mut record = body.clone();
                record["_key"] = json!(headers.get("x-api-key").and_then(|v| v.to_str().ok()));
                seen.lock().unwrap().push(record);
                let messages = body["messages"].as_array().cloned().unwrap_or_default();
                let answered = messages.last().is_some_and(|m| m.to_string().contains("tool_result"));
                let has_tools = body["tools"].as_array().is_some_and(|t| !t.is_empty());
                let start = ("message_start", json!({"type": "message_start", "message": {"id": "msg_m", "type": "message", "role": "assistant", "model": body["model"], "content": [], "stop_reason": null, "usage": {"input_tokens": 3, "output_tokens": 1}}}));
                if has_tools && !answered {
                    stream(vec![
                        start,
                        ("content_block_start", json!({"type": "content_block_start", "index": 0, "content_block": {"type": "tool_use", "id": "toolu_1", "name": "bash", "input": {}}})),
                        ("content_block_delta", json!({"type": "content_block_delta", "index": 0, "delta": {"type": "input_json_delta", "partial_json": "{\"command\":\"echo from-the-agent\",\"description\":\"say hi\"}"}})),
                        ("content_block_stop", json!({"type": "content_block_stop", "index": 0})),
                        ("message_delta", json!({"type": "message_delta", "delta": {"stop_reason": "tool_use"}, "usage": {"output_tokens": 5}})),
                        ("message_stop", json!({"type": "message_stop"})),
                    ])
                } else {
                    let words = if has_tools { "All done." } else { "A title" };
                    stream(vec![
                        start,
                        ("content_block_start", json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}})),
                        ("content_block_delta", json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": words}})),
                        ("content_block_stop", json!({"type": "content_block_stop", "index": 0})),
                        ("message_delta", json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 2}})),
                        ("message_stop", json!({"type": "message_stop"})),
                    ])
                }
            }),
        )
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (base, seen)
}

#[tokio::test]
#[ignore = "needs the pinned OpenCode: see the comment above"]
async fn real_opencode_runs_a_session_end_to_end() {
    tracing_subscriber::fmt().with_test_writer().try_init().ok();
    let binary = std::env::var("ANTILOG_OPENCODE").expect("ANTILOG_OPENCODE");
    let model = std::env::var("ANTILOG_OPENCODE_MODEL").expect("ANTILOG_OPENCODE_MODEL");
    let (proxy, seen) = mock_model().await;
    let (deps, seed, server, temp) = backend(&proxy, &model).await;
    let org = seed.organization.id;
    let mut config = Config::new(&server, KEY);
    config.poll = Duration::from_millis(100);
    let mut plain = Plain::under(&temp);
    // OpenCode reads its managed config from this path only.
    plain.config = PathBuf::from("/etc/opencode/opencode.json");
    let runner = tokio::spawn(run(
        config,
        super::Opencode {
            binary,
            pure: false,
            disable_project_config: false,
        },
        plain,
    ));
    let session = session::start(
        &*deps,
        org,
        seed.experiment.id,
        NewSession {
            agent: "opencode".into(),
            prompt: "say hi with bash".into(),
            repos: vec!["acme/api".into()],
            environment: None,
            actor: Some(seed.owner.id),
        },
    )
    .await
    .unwrap();
    let card = {
        let mut found = None;
        for _ in 0..600 {
            found = deps.store.pending_approvals(org).await.unwrap().pop();
            if found.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        found.expect("the bash permission became a card")
    };
    println!("card: {}", card.summary);
    approval::decide(
        &deps.store,
        &Principal {
            organization: org,
            member: seed.owner.id,
        },
        1,
        card.id,
        Decision::Approve,
    )
    .await
    .unwrap();
    let mut said = Vec::new();
    for _ in 0..600 {
        said = session::transcript(&deps.store, org, session.id, None, None)
            .await
            .unwrap()
            .events
            .into_iter()
            .map(|e| format!("{:?}: {}", e.kind, e.text))
            .collect();
        if said.iter().any(|line| line.contains("All done.")) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("transcript: {said:#?}");
    assert!(said.iter().any(|line| line.contains("All done.")));
    assert!(said.iter().any(|line| line.contains("bash")));
    let requests = seen.lock().unwrap().clone();
    assert!(requests.iter().all(|r| {
        r["_key"]
            .as_str()
            .is_some_and(|k| k.starts_with("alc_") || !k.is_empty())
    }));
    println!(
        "model calls: {}, key: {}",
        requests.len(),
        requests[0]["_key"]
    );

    // Release exports; a message resumes through import.
    deps.store
        .push_command(
            org,
            session.id,
            &Command::Release {
                outcome: antilog_types::ReleaseOutcome::Idle,
            },
            2,
        )
        .await
        .unwrap();
    for _ in 0..300 {
        if deps.store.machine(session.id).await.unwrap().unwrap().state == MachineState::Released {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let export = deps
        .store
        .export(session.id)
        .await
        .unwrap()
        .expect("an export was kept");
    println!("export bytes: {}", export.len());
    session::message(&*deps, org, session.id, Some(seed.owner.id), "again")
        .await
        .unwrap();
    let calls = requests.len();
    for _ in 0..300 {
        if seen.lock().unwrap().len() > calls {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        seen.lock().unwrap().len() > calls,
        "the resumed session called the model again"
    );
    let last = seen
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|r| r["tools"].is_array())
        .cloned()
        .unwrap();
    assert!(
        last["messages"].to_string().contains("say hi with bash"),
        "the imported history came along"
    );
    runner.abort();
    std::fs::remove_dir_all(&temp).ok();
}
