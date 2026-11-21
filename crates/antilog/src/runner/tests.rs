//! A runner against the real server routes on the in-memory store, with a fake OpenCode HTTP
//! server in place of the binary and a local git repo to clone.

use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use antilog_core::approval;
use antilog_core::fakes::FakeDeps;
use antilog_core::organization::Principal;
use antilog_core::runner::Settings as CoreSettings;
use antilog_core::seed::{Seed, organization};
use antilog_core::session::{self, NewSession};
use antilog_core::store::{ApprovalStore, ChangesStore, RunnerStore, SessionStore};
use antilog_server::api::auth::Settings;
use antilog_server::api::router;
use antilog_server::api::runner::RunnerSettings;
use antilog_types::{Command, Decision, MachineState, Session, SessionStatus};
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
    compacts: u32,
    /// Messages as `GET /session/{id}/message` lists them, oldest first.
    messages: Vec<Value>,
    permissions: Vec<Value>,
    status: Option<&'static str>,
    streams: Vec<mpsc::Sender<Result<Bytes, Infallible>>>,
    streams_opened: u32,
    imported: Vec<String>,
    /// Every request's authorization header.
    auth: Vec<String>,
    /// The fake PTY: what `POST /pty` was given, each resize, each removal and every text frame
    /// the runner wrote to a shell. The fake shell echoes `out:` and the input; `exit` ends it.
    ptys: Vec<Value>,
    resizes: Vec<(String, u64, u64)>,
    removed: Vec<String>,
    typed: Vec<String>,
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
                "/api/session/{id}/compact",
                post(|State(s): State<Shared>| async move {
                    locked(&s).compacts += 1;
                    Json(true)
                }),
            )
            .route(
                "/config/providers",
                get(|| async {
                    Json(json!({"providers": [{"id": "vendor", "models": {
                        "model-1": {"limit": {"context": 200_000, "output": 8_000}}}}]}))
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
                "/pty",
                post(
                    |State(s): State<Shared>, h: HeaderMap, Json(body): Json<Value>| async move {
                        note(&s, &h).await;
                        let mut fake = locked(&s);
                        fake.ptys.push(body);
                        Json(json!({"id": format!("pty_fake{}", fake.ptys.len())}))
                    },
                ),
            )
            .route(
                "/pty/{id}",
                axum::routing::put(
                    |State(s): State<Shared>,
                     UrlPath(id): UrlPath<String>,
                     Json(body): Json<Value>| async move {
                        let size = &body["size"];
                        locked(&s).resizes.push((
                            id,
                            size["cols"].as_u64().unwrap(),
                            size["rows"].as_u64().unwrap(),
                        ));
                        Json(json!({}))
                    },
                )
                .delete(
                    |State(s): State<Shared>, UrlPath(id): UrlPath<String>| async move {
                        locked(&s).removed.push(id);
                        Json(true)
                    },
                ),
            )
            .route(
                "/pty/{id}/connect",
                get(
                    |State(s): State<Shared>, upgrade: axum::extract::WebSocketUpgrade| async move {
                        upgrade.on_upgrade(move |mut socket| async move {
                            use axum::extract::ws::Message;
                            socket
                                .send(Message::Binary(Bytes::from_static(b"\0{\"cursor\":0}")))
                                .await
                                .ok();
                            while let Some(Ok(message)) = socket.recv().await {
                                if let Message::Text(text) = message {
                                    locked(&s).typed.push(text.to_string());
                                    if text.as_str() == "exit" {
                                        break;
                                    }
                                    socket
                                        .send(Message::Text(format!("out:{text}").into()))
                                        .await
                                        .ok();
                                }
                            }
                        })
                    },
                ),
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
    server: String,
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

    let lib = temp.join("origin/acme/lib");
    std::fs::create_dir_all(&lib).unwrap();
    sh(
        &lib,
        "git init -q . && git config user.email a@b && git config user.name n && echo lib > lib.txt && git add . && git commit -qm lib && git checkout -qb release && echo r > release.txt && git add . && git commit -qm release && git checkout -q -",
    );

    let deps = Arc::new(FakeDeps::new());
    let seed = organization(&deps.store).await;
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
            runner: Some(RunnerSettings {
                key: KEY.to_owned(),
                core,
                hold: std::time::Duration::ZERO,
            }),
            models: antilog_server::models::ModelLists::offline(),
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
        server,
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
        world.seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "vendor/model-1".into(),
            prompt: prompt.into(),
            repos: vec!["acme/api".into()],
            actor: world.seed.owner.id,
            ..NewSession::default()
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
        json!({
            "bash": "allow",
            "edit": "allow",
            "external_directory": {"/work/**": "allow", "/tmp/**": "allow"},
        })
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

    // An approved repo is cloned into the live machine, beside the first one. The queue holds
    // only its name; the server says where it is cloned from.
    world
        .deps
        .store
        .push_command(
            org,
            session.id,
            &Command::AttachRepo {
                repo: antilog_types::RepoSpec {
                    name: "acme/lib".into(),
                    url: String::new(),
                    token: None,
                    branch: None,
                },
            },
            3,
        )
        .await
        .unwrap();
    let lib = world
        .workdir
        .join(session.id.to_string())
        .join("lib/lib.txt");
    until("the attached repo", async || lib.exists().then_some(())).await;

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
async fn the_end_of_a_turn_reports_the_changes_against_the_clone() {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = start(&world, "cut p95").await;
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    until("a stream", async || {
        (locked(&world.shared).streams_opened > 0).then_some(())
    })
    .await;
    let repo = world.workdir.join(session.id.to_string()).join("api");
    sh(
        &repo,
        "echo second > file.txt && git -c user.email=a@b -c user.name=n commit -qam 'change the file'",
    );
    sh(
        &repo,
        "echo third > file.txt && echo new > new.txt && echo '*.log' > .gitignore && echo x > noise.log && printf 'a\\0b' > blob.bin",
    );
    emit(
        &world.shared,
        json!({"type": "session.idle", "properties": {"sessionID": "ses_fake"}}),
    )
    .await;
    let repos = until("the report", async || {
        let found = world.deps.store.changes(org, session.id).await.unwrap();
        (!found.is_empty()).then_some(found)
    })
    .await;
    let api = &repos[0];
    assert_eq!(api.repo, "acme/api");
    assert_eq!(api.base_branch, api.head_branch);
    assert_ne!(api.base_sha, api.head_sha);
    assert_eq!(api.commits.len(), 1);
    assert_eq!(api.commits[0].subject, "change the file");
    let file = |name: &str| api.files.iter().find(|file| file.path == name);
    let changed = file("file.txt").expect("the edited file");
    assert_eq!((changed.additions, changed.deletions), (1, 1));
    assert!(changed.patch.starts_with("@@"));
    assert!(changed.patch.contains("+third"));
    assert_eq!(
        file("new.txt").expect("a new file counts").status,
        antilog_types::FileStatus::Added
    );
    assert!(file("blob.bin").expect("a binary file").binary);
    assert!(file("blob.bin").unwrap().patch.is_empty());
    assert!(file("noise.log").is_none(), "an ignored file is left out");
    assert!(file(".gitignore").is_some());

    let commit = world
        .deps
        .store
        .commit_changes(org, session.id, None, &api.commits[0].sha)
        .await
        .unwrap()
        .expect("the commit's own diff");
    assert_eq!(commit.files.len(), 1);
    assert!(commit.files[0].patch.contains("+second"));
    let status = sh(&repo, "git status --short");
    assert!(
        status.contains("?? new.txt"),
        "the work tree's index is left as the agent had it: {status}"
    );
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
        world.seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "opencode/test".into(),
            prompt: "go".into(),
            repos: vec!["acme/missing".into()],
            actor: world.seed.owner.id,
            ..NewSession::default()
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
    let row = world
        .deps
        .store
        .session(org, session.id)
        .await
        .unwrap()
        .unwrap();
    let note = row.note.unwrap();
    assert!(note.starts_with("git clone failed"), "{note}");
    let events = session::transcript(&world.deps.store, org, session.id, None, None)
        .await
        .unwrap()
        .events;
    let error = events
        .iter()
        .find(|event| event.kind == antilog_types::EventKind::Error)
        .expect("the reason is in the transcript");
    assert!(
        error
            .text
            .starts_with("The session failed: git clone failed"),
        "{}",
        error.text
    );
}

/// A second organization's queued session, for a runner that has let its lock go to take.
async fn other_organizations_session(world: &World) -> (antilog_types::OrganizationId, Session) {
    let other = organization(&world.deps.store).await;
    let session = session::start(
        &*world.deps,
        other.organization.id,
        other.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "opencode/test".into(),
            prompt: "go".into(),
            actor: other.owner.id,
            ..NewSession::default()
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
            env: temp.join("env.sh"),
        },
        Vec::new(),
    );
    let spec = antilog_types::SessionSpec {
        session: antilog_types::SessionId::new(),
        organization: antilog_types::OrganizationId::new(),
        agent: "opencode".into(),
        model: "v/m".into(),
        effort: None,
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

// ---- the setup script -------------------------------------------------------------------------

/// A machine of the plain backend in a folder of its own, with the variables given.
fn setup_machine(variables: &[(&str, &str)]) -> (PlainMachine, PathBuf) {
    let temp = std::env::temp_dir().join(format!("antilog-setup-{}", Uuid::now_v7()));
    let work = temp.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let machine = PlainMachine::new(
        Paths {
            work: work.clone(),
            config: temp.join("opencode.json"),
            skills: temp.join("skills"),
            env: temp.join("env.sh"),
        },
        variables
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect(),
    );
    (machine, work)
}

fn setup_spec(script: &str) -> antilog_types::SessionSpec {
    antilog_types::SessionSpec {
        session: antilog_types::SessionId::new(),
        organization: antilog_types::OrganizationId::new(),
        agent: "opencode".into(),
        model: "v/m".into(),
        effort: None,
        repos: vec![antilog_types::RepoSpec {
            name: "acme/api".into(),
            url: "x".into(),
            token: Some("ghs_secret".into()),
            branch: None,
        }],
        environment: antilog_types::EnvironmentSpec {
            setup_script: script.into(),
            ..antilog_types::EnvironmentSpec::default()
        },
        skills: vec![],
        opencode_config: json!({}),
        model_token: String::new(),
        mcp_token: String::new(),
        runner_token: "tok".into(),
        resume: false,
    }
}

#[test]
fn a_reason_a_session_did_not_start_is_short_and_has_no_credentials() {
    let spec = setup_spec("");
    let long = "x".repeat(2000);
    let error = anyhow::anyhow!(
        "git clone failed (128): fatal: unable to access 'https://x-access-token:ghs_secret@host/acme/api/': {long}\nsecond line"
    );
    let text = super::session::reason(&error, &spec);
    assert!(text.starts_with("git clone failed (128): fatal: unable to access"));
    assert!(!text.contains("ghs_secret"), "{text}");
    assert!(text.contains("x-access-token:***@host"), "{text}");
    assert!(text.chars().count() <= 401, "{}", text.chars().count());
    assert!(text.ends_with('…'));
}

#[tokio::test]
async fn the_setup_script_runs_in_the_folder_of_the_checkouts_with_the_variables() {
    use super::session::{Setup, run_setup};
    let (machine, work) = setup_machine(&[("GREETING", "hello")]);
    std::fs::create_dir_all(work.join("api")).unwrap();
    std::fs::write(work.join("api/file.txt"), "base").unwrap();
    let spec = setup_spec("echo \"$GREETING $(cat api/file.txt) $(basename \"$PWD\")\" > ran.txt");
    let outcome = run_setup(&machine, &work, &spec, Duration::from_secs(30)).await;
    assert_eq!(outcome, Setup::Done);
    assert_eq!(
        std::fs::read_to_string(work.join("ran.txt"))
            .unwrap()
            .trim(),
        "hello base work"
    );
    // Once per container: the same machine does not run it again.
    std::fs::remove_file(work.join("ran.txt")).unwrap();
    assert_eq!(
        run_setup(&machine, &work, &spec, Duration::from_secs(30)).await,
        Setup::Skipped
    );
    assert!(!work.join("ran.txt").exists());
    // No script, nothing to run.
    assert_eq!(
        run_setup(
            &machine,
            &work,
            &setup_spec("  \n"),
            Duration::from_secs(30)
        )
        .await,
        Setup::Skipped
    );
    std::fs::remove_dir_all(work.parent().unwrap()).ok();
}

#[tokio::test]
async fn a_failing_setup_script_is_reported_with_its_exit_code_and_the_end_of_its_output() {
    use super::session::{Setup, run_setup};
    let (machine, work) = setup_machine(&[]);
    // Ten kilobytes first, so only the end can be kept; the clone credential is taken out.
    let spec = setup_spec(
        "head -c 10000 /dev/zero | tr '\\0' 'x'; echo; echo building ghs_secret; echo oops >&2; exit 7",
    );
    let Setup::Failed(failure) = run_setup(&machine, &work, &spec, Duration::from_secs(30)).await
    else {
        panic!("it failed");
    };
    assert_eq!((failure.exit, failure.timed_out), (Some(7), false));
    assert!(failure.tail.len() <= 2048, "{}", failure.tail.len());
    assert!(failure.tail.contains("building ***") && failure.tail.ends_with("oops"));
    assert!(!failure.tail.contains("ghs_secret"));
    let note = failure.note(Duration::from_secs(600));
    assert!(note.contains("failed (exit code 7)"), "{note}");
    assert!(note.contains("tools it installs may be missing"), "{note}");
    assert!(failure.line().contains("setup script failed"));
    std::fs::remove_dir_all(work.parent().unwrap()).ok();
}

#[tokio::test]
async fn a_setup_script_that_runs_out_of_time_is_stopped_and_reported_like_a_failure() {
    use super::session::{Setup, run_setup};
    let (machine, work) = setup_machine(&[]);
    let spec = setup_spec("echo started; sleep 60; echo never");
    let began = std::time::Instant::now();
    let Setup::Failed(failure) = run_setup(&machine, &work, &spec, Duration::from_secs(1)).await
    else {
        panic!("it timed out");
    };
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "{:?}",
        began.elapsed()
    );
    assert!(failure.timed_out);
    assert_eq!(failure.exit, Some(124));
    assert!(failure.tail.contains("started") && !failure.tail.contains("never"));
    let note = failure.note(Duration::from_secs(1));
    assert!(
        note.contains("did not finish in 1 seconds and was stopped (exit code 124)"),
        "{note}"
    );
    assert!(failure.line().contains("timed out"));
    std::fs::remove_dir_all(work.parent().unwrap()).ok();
}

/// A session on a stored environment with a setup script and default repos.
async fn start_on_environment(world: &World, script: &str, repos: &[&str]) -> Session {
    use antilog_types::{NetworkLevel, NewEnvironment};
    let who = Principal {
        organization: world.seed.organization.id,
        member: world.seed.owner.id,
    };
    antilog_core::environments::create(
        &world.deps.store,
        &who,
        1,
        NewEnvironment {
            name: "Bench".into(),
            network: NetworkLevel::Limited,
            allowed_hosts: Vec::new(),
            allow_package_managers: true,
            variables: [("GREETING".to_owned(), "hello".to_owned())].into(),
            setup_script: script.into(),
            repos: repos.iter().map(|repo| (*repo).to_owned()).collect(),
            network_secrets: Vec::new(),
            cpus: 2,
            memory_mb: 4096,
        },
    )
    .await
    .unwrap();
    session::start(
        &*world.deps,
        world.seed.organization.id,
        world.seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "vendor/model-1".into(),
            prompt: "go".into(),
            repos: vec!["acme/api".into()],
            environment: Some("bench".into()),
            actor: world.seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_session_clones_the_branch_it_was_started_with_for_the_repos_picked() {
    let world = world().await;
    let session = session::start(
        &*world.deps,
        world.seed.organization.id,
        world.seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "vendor/model-1".into(),
            prompt: "go".into(),
            repos: vec!["acme/lib".into()],
            branch: Some("release".into()),
            actor: world.seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    let folder = world.workdir.join(session.id.to_string());
    assert!(folder.join("lib/release.txt").exists(), "lib is on release");
}

#[tokio::test]
async fn a_session_runs_the_setup_script_after_the_clones_and_before_the_agent() {
    let world = world().await;
    // The script sees the default repo and the picked one, and the environment's variable.
    let session = start_on_environment(
        &world,
        "echo \"$GREETING $(cat api/file.txt) $(cat lib/lib.txt)\" > ran.txt",
        &["acme/lib"],
    )
    .await;
    assert_eq!(session.repos, vec!["acme/lib", "acme/api"]);
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    let folder = world.workdir.join(session.id.to_string());
    assert_eq!(
        std::fs::read_to_string(folder.join("ran.txt"))
            .unwrap()
            .trim(),
        "hello base lib"
    );
    // It worked: nothing to tell.
    assert_eq!(locked(&world.shared).prompts[0]["parts"][0]["text"], "go");
    assert!(
        transcript(&world, &session)
            .await
            .iter()
            .all(|text| !text.contains("setup script"))
    );
}

#[tokio::test]
async fn a_failing_setup_script_does_not_fail_the_session_and_everyone_is_told() {
    let world = world().await;
    let session = start_on_environment(&world, "echo installing; echo boom >&2; exit 3", &[]).await;
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    // The agent starts, and its first message says what went wrong with the machine.
    let prompt = locked(&world.shared).prompts[0]["parts"][0]["text"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        prompt.starts_with("[Antilog] The environment's setup script failed"),
        "{prompt}"
    );
    assert!(prompt.ends_with("\n\ngo"), "{prompt}");
    // The person reads a note with the exit code and the end of the output.
    let note = until("the note", async || {
        transcript(&world, &session)
            .await
            .into_iter()
            .find(|text| text.contains("setup script"))
    })
    .await;
    assert!(note.contains("exit code 3"), "{note}");
    assert!(
        note.contains("installing") && note.contains("boom"),
        "{note}"
    );
    assert_ne!(state(&world, &session).await, MachineState::Failed);
    // Only the first message carries the line.
    session::message(
        &*world.deps,
        world.seed.organization.id,
        session.id,
        Some(world.seed.owner.id),
        "next",
    )
    .await
    .unwrap();
    until("the second prompt", async || {
        (locked(&world.shared).prompts.len() == 2).then_some(())
    })
    .await;
    assert_eq!(locked(&world.shared).prompts[1]["parts"][0]["text"], "next");
}

/// Another environment to move a session to: no `GREETING`, one variable with a quote in it, and
/// the given script.
async fn make_other_environment(world: &World, script: &str) {
    use antilog_types::{NetworkLevel, NewEnvironment};
    let who = Principal {
        organization: world.seed.organization.id,
        member: world.seed.owner.id,
    };
    antilog_core::environments::create(
        &world.deps.store,
        &who,
        2,
        NewEnvironment {
            name: "Other".into(),
            network: NetworkLevel::Limited,
            allowed_hosts: Vec::new(),
            allow_package_managers: true,
            variables: [("EXTRA".to_owned(), "it's \"fine\" $HOME".to_owned())].into(),
            setup_script: script.into(),
            repos: Vec::new(),
            network_secrets: Vec::new(),
            cpus: 2,
            memory_mb: 4096,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn a_session_moved_to_another_environment_gets_its_variables_and_runs_the_changed_script() {
    let world = world().await;
    let session = start_on_environment(&world, "", &[]).await;
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    make_other_environment(&world, "echo \"$EXTRA\" > moved.txt").await;
    session::move_to_environment(
        &*world.deps,
        world.seed.organization.id,
        session.id,
        Some(world.seed.owner.id),
        "other",
    )
    .await
    .unwrap();

    // The script ran now, from the folder of the checkouts, with the new variables.
    let folder = world.workdir.join(session.id.to_string());
    let moved = folder.join("moved.txt");
    until("the script", async || moved.exists().then_some(())).await;
    assert_eq!(
        std::fs::read_to_string(&moved).unwrap().trim(),
        "it's \"fine\" $HOME"
    );
    // New bash processes source the file: the new variable is set verbatim and the one the new
    // version lacks is gone, though the machine's own environment still has it.
    let file = world.workdir.join(format!("{}.env.sh", session.id));
    let out = std::process::Command::new("bash")
        .args(["-c", "echo \"[$GREETING][$EXTRA]\""])
        .env("BASH_ENV", &file)
        .env("GREETING", "hello")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "[][it's \"fine\" $HOME]"
    );
    // The agent is told, once, and the session carries on.
    until("the agent's message", async || {
        locked(&world.shared)
            .prompts
            .iter()
            .any(|p| {
                p["parts"][0]["text"]
                    .as_str()
                    .is_some_and(|t| t.contains("moved this session to the environment Other"))
            })
            .then_some(())
    })
    .await;
    assert_ne!(state(&world, &session).await, MachineState::Failed);
}

#[tokio::test]
async fn a_failing_script_after_a_move_is_a_note_and_a_line_for_the_agent_not_a_failure() {
    let world = world().await;
    let session = start_on_environment(&world, "", &[]).await;
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    make_other_environment(&world, "echo nope >&2; exit 5").await;
    session::move_to_environment(
        &*world.deps,
        world.seed.organization.id,
        session.id,
        Some(world.seed.owner.id),
        "other",
    )
    .await
    .unwrap();
    let note = until("the note", async || {
        transcript(&world, &session)
            .await
            .into_iter()
            .find(|text| text.contains("setup script failed"))
    })
    .await;
    assert!(
        note.contains("exit code 5") && note.contains("nope"),
        "{note}"
    );
    let told = until("the agent's message", async || {
        locked(&world.shared)
            .prompts
            .iter()
            .filter_map(|p| p["parts"][0]["text"].as_str().map(str::to_owned))
            .find(|t| t.contains("moved this session"))
    })
    .await;
    assert!(
        told.starts_with("[Antilog] The environment's setup script failed"),
        "{told}"
    );
    assert_ne!(state(&world, &session).await, MachineState::Failed);
}

#[tokio::test]
async fn a_setup_script_that_times_out_does_not_fail_the_session_either() {
    let world = world_with(
        |config| config.setup_timeout = Duration::from_secs(1),
        Arc::default(),
    )
    .await;
    let session = start_on_environment(&world, "echo slow; sleep 60", &[]).await;
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    let prompt = locked(&world.shared).prompts[0]["parts"][0]["text"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(prompt.contains("setup script timed out"), "{prompt}");
    let note = until("the note", async || {
        transcript(&world, &session)
            .await
            .into_iter()
            .find(|text| text.contains("setup script"))
    })
    .await;
    assert!(
        note.contains("did not finish in 1 seconds and was stopped (exit code 124)"),
        "{note}"
    );
    assert!(note.contains("slow"), "{note}");
}

#[tokio::test]
async fn a_runner_holds_as_many_sessions_as_it_has_room_for() {
    let world = world_with(|config| config.sessions = 2, Arc::default()).await;
    let org = world.seed.organization.id;
    let status = async |session: &Session| {
        world
            .deps
            .store
            .session(org, session.id)
            .await
            .unwrap()
            .unwrap()
            .status
    };
    let first = start(&world, "one").await;
    let second = start(&world, "two").await;
    let third = start(&world, "three").await;
    assert_eq!(status(&third).await, SessionStatus::Queued);
    until("two claimed", async || {
        (state(&world, &first).await != MachineState::Queued
            && state(&world, &second).await != MachineState::Queued)
            .then_some(())
    })
    .await;
    assert_ne!(status(&first).await, SessionStatus::Queued);
    assert_ne!(status(&second).await, SessionStatus::Queued);
    // There is no room for a third until one lets go; until then it reads Queued, not Working.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(state(&world, &third).await, MachineState::Queued);
    assert_eq!(status(&third).await, SessionStatus::Queued);

    let stopped = session::stop(
        &*world.deps,
        org,
        first.id,
        Some(world.seed.owner.id),
        "Ada",
    )
    .await
    .unwrap();
    assert_eq!(stopped.status, SessionStatus::Stopped);
    until("the third claimed", async || {
        (state(&world, &third).await != MachineState::Queued).then_some(())
    })
    .await;
    assert_ne!(status(&third).await, SessionStatus::Queued);
}

#[tokio::test]
async fn an_idle_session_lets_its_machine_go_and_a_message_brings_it_back() {
    let world = world_with(
        |config| config.idle_timeout = Duration::from_secs(1),
        Arc::default(),
    )
    .await;
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
    emit(
        &world.shared,
        json!({"type": "session.status", "properties": {"sessionID": "ses_fake", "status": {"type": "idle"}}}),
    )
    .await;
    until("released", async || {
        (state(&world, &session).await == MachineState::Released).then_some(())
    })
    .await;
    assert!(world.deps.store.export(session.id).await.unwrap().is_some());
    let row = world
        .deps
        .store
        .session(org, session.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, SessionStatus::WaitingForYou);

    let woken = session::message(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "keep going",
    )
    .await
    .unwrap();
    assert_eq!(woken.status, SessionStatus::Queued);
    until("the second prompt", async || {
        (locked(&world.shared).prompts.len() == 2).then_some(())
    })
    .await;
    assert_eq!(locked(&world.shared).imported.len(), 1);
}

#[tokio::test]
async fn every_prompt_carries_the_sessions_model_and_effort_and_a_switch_applies_from_the_next_message()
 {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = session::start(
        &*world.deps,
        org,
        world.seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "vendor/model-1".into(),
            effort: Some("high".into()),
            prompt: "first".into(),
            repos: vec!["acme/api".into()],
            actor: world.seed.owner.id,
            ..NewSession::default()
        },
    )
    .await
    .unwrap();
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    let first = locked(&world.shared).prompts[0].clone();
    assert_eq!(
        first["model"],
        json!({"providerID": "vendor", "modelID": "model-1"})
    );
    assert_eq!(first["variant"], "high");

    session::set_model(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "other/model-2",
        Some("max"),
        "Switched to Model 2 · Max",
    )
    .await
    .unwrap();
    session::message(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "second",
    )
    .await
    .unwrap();
    until("the second prompt", async || {
        (locked(&world.shared).prompts.len() == 2).then_some(())
    })
    .await;
    let second = locked(&world.shared).prompts[1].clone();
    assert_eq!(second["parts"][0]["text"], "second");
    assert_eq!(
        second["model"],
        json!({"providerID": "other", "modelID": "model-2"})
    );
    assert_eq!(second["variant"], "max");

    // Back to the model's own default: no variant at all.
    session::set_model(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "other/model-2",
        None,
        "Switched to Model 2",
    )
    .await
    .unwrap();
    session::message(
        &*world.deps,
        org,
        session.id,
        Some(world.seed.owner.id),
        "third",
    )
    .await
    .unwrap();
    until("the third prompt", async || {
        (locked(&world.shared).prompts.len() == 3).then_some(())
    })
    .await;
    assert!(locked(&world.shared).prompts[2].get("variant").is_none());
}

#[tokio::test]
async fn a_step_reports_the_context_it_filled_and_compaction_is_a_note_and_a_command() {
    use antilog_core::store::ContextStore;

    let world = world().await;
    let org = world.seed.organization.id;
    let session = start(&world, "go").await;
    until("a stream", async || {
        (locked(&world.shared).streams_opened > 0).then_some(())
    })
    .await;
    emit(
        &world.shared,
        json!({"type": "message.updated", "properties": {"sessionID": "ses_fake", "info": {
            "id": "msg_a", "role": "assistant", "providerID": "vendor", "modelID": "model-1"}}}),
    )
    .await;
    emit(
        &world.shared,
        json!({"type": "message.part.updated", "properties": {"sessionID": "ses_fake", "part": {
            "id": "prt_s", "messageID": "msg_a", "sessionID": "ses_fake", "type": "step-finish",
            "tokens": {"input": 1000, "output": 50, "reasoning": 0, "cache": {"read": 20_000, "write": 3_000}},
            "cost": 0.01}}}),
    )
    .await;
    let context = until("the context", async || {
        let context = world.deps.store.context(org, session.id).await.unwrap();
        context.used.is_some().then_some(context)
    })
    .await;
    // The window of the latest call: input, cache read and cache write; not the output.
    assert_eq!(context.used, Some(24_000));
    assert_eq!(context.window, Some(200_000));

    emit(
        &world.shared,
        json!({"type": "message.part.updated", "properties": {"sessionID": "ses_fake", "part": {
            "id": "prt_c", "messageID": "msg_u", "sessionID": "ses_fake", "type": "compaction",
            "auto": true, "overflow": true}}}),
    )
    .await;
    until("the note", async || {
        transcript(&world, &session)
            .await
            .iter()
            .any(|line| line.contains("Context compacted (the window overflowed)"))
            .then_some(())
    })
    .await;

    session::compact(&*world.deps, org, session.id, Some(world.seed.owner.id))
        .await
        .unwrap();
    until("OpenCode told to compact", async || {
        (locked(&world.shared).compacts == 1).then_some(())
    })
    .await;
}

#[tokio::test]
async fn a_busy_session_keeps_its_machine() {
    let world = world_with(
        |config| config.idle_timeout = Duration::from_secs(1),
        Arc::default(),
    )
    .await;
    locked(&world.shared).status = Some("busy");
    let session = start(&world, "go").await;
    until("the first prompt", async || {
        (!locked(&world.shared).prompts.is_empty()).then_some(())
    })
    .await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_ne!(state(&world, &session).await, MachineState::Released);
}

#[tokio::test]
async fn a_refused_model_call_is_an_error_in_the_transcript_and_the_session_waits() {
    let world = world().await;
    let org = world.seed.organization.id;
    let session = start(&world, "go").await;
    until("a stream", async || {
        (locked(&world.shared).streams_opened > 0).then_some(())
    })
    .await;
    emit(
        &world.shared,
        json!({"type": "message.updated", "properties": {"sessionID": "ses_fake", "info": {
            "id": "msg_err", "role": "assistant",
            "error": {"name": "APIError", "data": {"message": "Insufficient balance", "statusCode": 402}}}}}),
    )
    .await;
    emit(
        &world.shared,
        json!({"type": "session.status", "properties": {"sessionID": "ses_fake", "status": {"type": "idle"}}}),
    )
    .await;
    let error = until("the error", async || {
        session::transcript(&world.deps.store, org, session.id, None, None)
            .await
            .unwrap()
            .events
            .into_iter()
            .find(|event| event.kind == antilog_types::EventKind::Error)
    })
    .await;
    assert_eq!(error.text, "Insufficient balance");
    until("waiting", async || {
        let row = world
            .deps
            .store
            .session(org, session.id)
            .await
            .unwrap()
            .unwrap();
        (row.status == SessionStatus::WaitingForYou).then_some(())
    })
    .await;
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
        seed.project.id,
        NewSession {
            agent: "opencode".into(),
            model: "opencode/test".into(),
            prompt: "say hi with bash".into(),
            repos: vec!["acme/api".into()],
            actor: seed.owner.id,
            ..NewSession::default()
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

// ---- terminals -----------------------------------------------------------------------------

type Browser =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// What a person's browser does: ask for a terminal over HTTP, then open its WebSocket.
async fn open_terminal(world: &World, session: &Session, who: &str) -> Browser {
    let ticket: Value = reqwest::Client::new()
        .post(format!(
            "{}/sessions/{}/terminals",
            world.server, session.id
        ))
        .bearer_auth(who)
        .json(&json!({"cols": 100, "rows": 30}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let url = format!(
        "{}/terminals/{}/connect?token={}",
        world.server.replacen("http", "ws", 1),
        ticket["terminal"].as_str().unwrap(),
        ticket["token"].as_str().unwrap()
    );
    tokio_tungstenite::connect_async(url).await.unwrap().0
}

async fn token_for(world: &World) -> String {
    let who = Principal {
        organization: world.seed.organization.id,
        member: world.seed.owner.id,
    };
    antilog_core::organization::create_token(&world.deps.store, &who, 1_800_000_000_000, None, "t")
        .await
        .unwrap()
        .secret
}

async fn machine_up(world: &World, session: &Session) {
    until("the machine up", async || {
        matches!(
            state(world, session).await,
            MachineState::Running | MachineState::Idle
        )
        .then_some(())
    })
    .await;
    until("a stream", async || {
        (locked(&world.shared).streams_opened > 0).then_some(())
    })
    .await;
}

async fn frame(socket: &mut Browser) -> Option<tokio_tungstenite::tungstenite::Message> {
    use futures_util::StreamExt as _;
    tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("a frame within five seconds")
        .and_then(Result::ok)
}

#[tokio::test]
async fn a_terminal_opened_by_a_person_is_a_shell_in_the_machine_carried_over_an_outbound_socket() {
    use futures_util::SinkExt as _;
    use tokio_tungstenite::tungstenite::Message;
    let world = world().await;
    let session = start(&world, "hi").await;
    machine_up(&world, &session).await;
    let token = token_for(&world).await;

    let mut browser = open_terminal(&world, &session, &token).await;
    // The shell starts in the folder the agent works in, at the size the panel had, with the session's
    // variables, and the runner's key is nowhere in it.
    until("the shell", async || {
        (!locked(&world.shared).resizes.is_empty()).then_some(())
    })
    .await;
    {
        let fake = locked(&world.shared);
        let cwd = fake.ptys[0]["cwd"].as_str().unwrap().to_owned();
        assert!(cwd.contains(&session.id.to_string()), "{cwd}");
        assert_eq!(fake.resizes[0], ("pty_fake1".to_owned(), 100, 30));
        assert!(!fake.ptys[0].to_string().contains(KEY));
    }

    // Keys reach the shell and its output reaches the browser, as bytes; the shell's own
    // bookkeeping frame does not.
    browser
        .send(Message::Binary(b"echo hi\r".to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        frame(&mut browser).await,
        Some(Message::Binary(b"out:echo hi\r".to_vec().into()))
    );
    assert_eq!(locked(&world.shared).typed, vec!["echo hi\r".to_owned()]);

    // A resize from the browser resizes the shell.
    browser
        .send(Message::Text(
            r#"{"type":"resize","cols":120,"rows":40}"#.into(),
        ))
        .await
        .unwrap();
    until("the resize", async || {
        locked(&world.shared)
            .resizes
            .contains(&("pty_fake1".to_owned(), 120, 40))
            .then_some(())
    })
    .await;

    // The shell ending tells the browser and closes; the shell is removed.
    browser
        .send(Message::Binary(b"exit".to_vec().into()))
        .await
        .unwrap();
    assert_eq!(
        frame(&mut browser).await,
        Some(Message::Text(r#"{"type":"exit"}"#.into()))
    );
    until("the shell removed", async || {
        locked(&world.shared)
            .removed
            .contains(&"pty_fake1".to_owned())
            .then_some(())
    })
    .await;
}

#[tokio::test]
async fn closing_the_browser_removes_the_shell_and_stopping_the_session_closes_the_rest() {
    let world = world().await;
    let session = start(&world, "hi").await;
    machine_up(&world, &session).await;
    let token = token_for(&world).await;

    let mut first = open_terminal(&world, &session, &token).await;
    until("the first shell", async || {
        (locked(&world.shared).ptys.len() == 1).then_some(())
    })
    .await;
    first.close(None).await.ok();
    until("the first shell removed", async || {
        locked(&world.shared)
            .removed
            .contains(&"pty_fake1".to_owned())
            .then_some(())
    })
    .await;

    let mut second = open_terminal(&world, &session, &token).await;
    until("the second shell", async || {
        (locked(&world.shared).ptys.len() == 2).then_some(())
    })
    .await;
    let stopped = reqwest::Client::new()
        .post(format!("{}/sessions/{}/stop", world.server, session.id))
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(stopped.status(), 200);
    assert!(matches!(
        frame(&mut second).await,
        None | Some(tokio_tungstenite::tungstenite::Message::Close(_))
    ));
}
