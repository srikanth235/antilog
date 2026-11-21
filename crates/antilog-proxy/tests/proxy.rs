#![expect(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "tests read a JSON body and a fixed block, and a failed call fails the test"
)]

//! The proxy against fake upstreams for both APIs, over real sockets on localhost: no real model
//! call is made and no key is a real one.

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use antilog_core::clock::SystemClock;
use antilog_core::credentials::{Purpose, SessionCredentials};
use antilog_core::fakes::{FakeCredentials, FakeStore};
use antilog_core::secrets::Secret;
use antilog_core::seed::{self, Seed};
use antilog_core::store::{
    BudgetStore, EnvironmentStore, ModelKeyStore, RunnerStore, SessionStore,
};
use antilog_proxy::price::{self, Tokens, ZenPrices};
use antilog_proxy::{Proxy, SessionNetworks, Upstreams, server};
use antilog_types::{
    AuditEntry, AuditOutcome, Environment, ModelProvider, NetworkLevel, Seed as MachineSeed,
    Session, SessionId, SessionParams,
};
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::response::Response;
use axum::routing::any;
use bytes::Bytes;
use jiff::SignedDuration;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::wrappers::ReceiverStream;

/// A model the vendored table prices, for each provider.
fn model_of(provider: ModelProvider) -> String {
    price::models(provider)
        .into_iter()
        .find(|m| {
            price::lookup(provider, Some(m), &ZenPrices::default())
                .0
                .input
                > 0
        })
        .unwrap()
        .to_owned()
}

fn am() -> String {
    model_of(ModelProvider::Anthropic)
}

fn om() -> String {
    model_of(ModelProvider::Openai)
}

/// What the tokens cost under a model, in cents, as the proxy should book it.
fn cost(provider: ModelProvider, model: &str, tokens: Tokens) -> i64 {
    i64::try_from(price::cents(
        price::lookup(provider, Some(model), &ZenPrices::default()).0,
        tokens,
    ))
    .unwrap()
}

const A_TOKENS: Tokens = Tokens {
    input: 1000,
    output: 500,
    cache_read: 0,
    cache_write: 0,
};
const O_TOKENS: Tokens = Tokens {
    input: 2000,
    output: 800,
    cache_read: 0,
    cache_write: 0,
};

const ANTHROPIC_KEY: &str = "sk-ant-real-org-key-0001";
const OPENAI_KEY: &str = "sk-oai-real-org-key-0002";
const ZEN_KEY: &str = "zen-real-org-key-0003";

// ---- the fake providers -------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Seen {
    path: String,
    headers: HashMap<String, String>,
    body: Value,
}

type Log = Arc<Mutex<Vec<Seen>>>;

fn sse(events: &[&str]) -> Response {
    let text: String = events.iter().map(|e| format!("{e}\n\n")).collect();
    Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(text))
        .unwrap()
}

async fn provider(State(log): State<Log>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, 1 << 20).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    let seen = Seen {
        path: parts.uri.path().to_owned(),
        headers: parts
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_owned()))
            .collect(),
        body: body.clone(),
    };
    log.lock().unwrap().push(seen);
    let model = body["model"].as_str().unwrap_or("m").to_owned();
    let stream = body["stream"] == true;

    if parts.uri.path().ends_with("/messages") {
        if model == "slow-stream" {
            let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, std::io::Error>>(4);
            tokio::spawn(async move {
                let start = json!({"type":"message_start","message":{"model":am(),"usage":{"input_tokens":700,"output_tokens":1}}});
                let _ = tx
                    .send(Ok(Bytes::from(format!(
                        "event: message_start\ndata: {start}\n\n"
                    ))))
                    .await;
                for _ in 0..100 {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let delta =
                        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\n\n";
                    if tx.send(Ok(Bytes::from(delta))).await.is_err() {
                        return;
                    }
                }
            });
            return Response::builder()
                .header(header::CONTENT_TYPE, "text/event-stream")
                .body(Body::from_stream(ReceiverStream::new(rx)))
                .unwrap();
        }
        let usage = json!({"input_tokens": 1000, "output_tokens": 500});
        if stream {
            let start = json!({"type":"message_start","message":{"model":model,"usage":{"input_tokens":1000,"output_tokens":1}}});
            let text = json!({"type":"content_block_delta","delta":{"text":"hello"}});
            let delta = json!({"type":"message_delta","usage":{"output_tokens":500}});
            return sse(&[
                &format!("event: message_start\ndata: {start}"),
                &format!("event: content_block_delta\ndata: {text}"),
                &format!("event: message_delta\ndata: {delta}"),
                "event: message_stop\ndata: {\"type\":\"message_stop\"}",
            ]);
        }
        return axum::Json(json!({"type":"message","model":model,"content":[{"type":"text","text":"hello"}],"usage":usage})).into_response_();
    }

    if parts.uri.path().ends_with("/chat/completions") {
        let usage = json!({"prompt_tokens": 2000, "completion_tokens": 800, "total_tokens": 2800});
        if stream {
            let chunk = json!({"model":model,"choices":[{"delta":{"content":"hello"}}]});
            let last = json!({"model":model,"choices":[],"usage":usage});
            let mut events = vec![format!("data: {chunk}")];
            if body["stream_options"]["include_usage"] == true {
                events.push(format!("data: {last}"));
            }
            events.push("data: [DONE]".to_owned());
            let refs: Vec<&str> = events.iter().map(String::as_str).collect();
            return sse(&refs);
        }
        return axum::Json(
            json!({"model":model,"choices":[{"message":{"content":"hello"}}],"usage":usage}),
        )
        .into_response_();
    }
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap()
}

trait IntoResponse_ {
    fn into_response_(self) -> Response;
}
impl IntoResponse_ for axum::Json<Value> {
    fn into_response_(self) -> Response {
        axum::response::IntoResponse::into_response(self)
    }
}

async fn start_provider() -> (String, Log) {
    let log = Log::default();
    let app = Router::new()
        .route("/{*path}", any(provider))
        .with_state(log.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (url, log)
}

// ---- logs ---------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Capture {
    type Writer = Capture;
    fn make_writer(&'a self) -> Capture {
        self.clone()
    }
}

impl Capture {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

// ---- the rig ------------------------------------------------------------------------------

type TestProxy = Proxy<FakeStore, FakeCredentials, SystemClock>;

struct Rig {
    proxy: Arc<TestProxy>,
    addr: std::net::SocketAddr,
    seed: Seed,
    session: SessionId,
    token: String,
    log: Log,
    logs: Capture,
    http: reqwest::Client,
    _guard: tracing::subscriber::DefaultGuard,
}

struct Options {
    budget_cents: i64,
    network: NetworkLevel,
    registries: bool,
    hosts: Vec<String>,
    platform_hosts: Vec<String>,
    allow_private: bool,
    keys: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            budget_cents: 10_000,
            network: NetworkLevel::Limited,
            registries: false,
            hosts: Vec::new(),
            platform_hosts: Vec::new(),
            allow_private: false,
            keys: true,
        }
    }
}

fn audit(seed: &Seed, action: &str) -> AuditEntry {
    AuditEntry::new(
        seed.organization.id,
        Some(seed.owner.id),
        action,
        "x",
        AuditOutcome::Done,
        seed::T0,
    )
}

impl Rig {
    async fn start(options: Options) -> Self {
        let logs = Capture::default();
        let guard = tracing::subscriber::set_default(
            tracing_subscriber::fmt()
                .with_writer(logs.clone())
                .with_max_level(tracing::Level::TRACE)
                .with_ansi(false)
                .finish(),
        );
        let (upstream, log) = start_provider().await;
        let store = FakeStore::default();
        let seed = seed::organization(&store).await;
        let session = Session::new(
            seed.organization.id,
            seed.owner.id,
            SessionParams {
                title: "cut p95".into(),
                agent: "opencode".into(),
                model: "opencode/test".into(),
                budget_cap_cents: Some(options.budget_cents),
                ..SessionParams::default()
            },
            seed::T0,
        );
        store
            .create_session(&session, &audit(&seed, "session.start"))
            .await
            .unwrap();
        if options.keys {
            for (provider, key) in [
                (ModelProvider::Anthropic, ANTHROPIC_KEY),
                (ModelProvider::Openai, OPENAI_KEY),
                (ModelProvider::Opencode, ZEN_KEY),
            ] {
                store
                    .put_model_key(
                        seed.organization.id,
                        provider,
                        &Secret::new(key),
                        seed::T0,
                        &audit(&seed, "model_key.set"),
                    )
                    .await
                    .unwrap();
            }
        }
        // The session's machine was started on a stored environment, which says what it reaches.
        let environment = Environment {
            id: "bench".into(),
            name: "Bench".into(),
            version: 1,
            network: options.network,
            allowed_hosts: options.hosts,
            allow_package_managers: options.registries,
            variables: Default::default(),
            setup_script: String::new(),
            cpus: 2,
            memory_mb: 8192,
            built_in: false,
            archived: false,
            created_by: Some(seed.owner.id),
            created_at: seed::T0,
        };
        store
            .add_environment(
                seed.organization.id,
                &environment,
                &audit(&seed, "environment.create"),
            )
            .await
            .unwrap();
        store
            .enqueue_machine(
                seed.organization.id,
                session.id,
                &MachineSeed {
                    repos: Vec::new(),
                    environment: Some("bench".into()),
                    environment_version: Some(1),
                },
                seed::T0,
            )
            .await
            .unwrap();
        let credentials = FakeCredentials::new(Arc::new(SystemClock));
        let token = credentials
            .mint(session.id, Purpose::Model, SignedDuration::from_mins(30))
            .await
            .unwrap();
        let proxy = Arc::new(Proxy {
            store,
            credentials,
            clock: SystemClock,
            networks: SessionNetworks::default(),
            zen: ZenPrices::default(),
            upstreams: Upstreams {
                anthropic: upstream.clone(),
                openai: upstream.clone(),
                opencode: upstream.clone(),
                server: Some(upstream),
            },
            http: reqwest::Client::new(),
            platform_hosts: options.platform_hosts.clone(),
            allow_private_addresses: options.allow_private,
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(server::serve(listener, proxy.clone()));
        Self {
            proxy,
            addr,
            seed,
            session: session.id,
            token,
            log,
            logs,
            http: reqwest::Client::new(),
            _guard: guard,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    async fn anthropic(&self, token: &str, body: Value) -> reqwest::Response {
        self.http
            .post(self.url("/anthropic/v1/messages"))
            .header("x-api-key", token)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    async fn openai(&self, token: &str, body: Value) -> reqwest::Response {
        self.http
            .post(self.url("/openai/v1/chat/completions"))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    /// What the session has been booked: cents and tokens.
    async fn booked(&self) -> (i64, i64) {
        let lines = self
            .proxy
            .store
            .spend_lines(self.seed.organization.id, self.session)
            .await
            .unwrap();
        (
            lines.iter().map(|l| l.cents).sum(),
            lines.iter().map(|l| l.tokens).sum(),
        )
    }

    fn calls(&self) -> Vec<Seen> {
        self.log.lock().unwrap().clone()
    }
}

// ---- the gateway --------------------------------------------------------------------------

#[tokio::test]
async fn an_anthropic_call_carries_the_organizations_key_and_books_its_usage() {
    let rig = Rig::start(Options::default()).await;
    let reply = rig
        .anthropic(
            &rig.token,
            json!({"model":format!("{}-20250929", am()),"max_tokens":10,"messages":[]}),
        )
        .await;
    assert_eq!(reply.status(), StatusCode::OK);
    let body: Value = reply.json().await.unwrap();
    assert_eq!(body["content"][0]["text"], "hello");

    // The token was swapped: the provider saw the organization's key and never the token.
    let calls = rig.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/v1/messages");
    assert_eq!(calls[0].headers["x-api-key"], ANTHROPIC_KEY);
    assert_eq!(calls[0].headers["anthropic-version"], "2023-06-01");
    assert!(!calls[0].headers.values().any(|v| v.contains(&rig.token)));
    // 1000 input at $3 and 500 output at $15 per million is 1.05 cents, booked as 2.
    assert_eq!(
        rig.booked().await,
        (cost(ModelProvider::Anthropic, &am(), A_TOKENS), 1500)
    );
}

#[tokio::test]
async fn an_anthropic_stream_is_relayed_whole_and_books_its_final_usage() {
    let rig = Rig::start(Options::default()).await;
    let reply = rig
        .anthropic(
            &rig.token,
            json!({"model":am(),"stream":true,"messages":[]}),
        )
        .await;
    assert_eq!(reply.status(), StatusCode::OK);
    assert!(
        reply.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
    let text = reply.text().await.unwrap();
    for event in [
        "message_start",
        "content_block_delta",
        "message_delta",
        "message_stop",
    ] {
        assert!(
            text.contains(&format!("event: {event}")),
            "{event} missing from {text}"
        );
    }
    assert_eq!(rig.calls()[0].headers["x-api-key"], ANTHROPIC_KEY);
    // The start reports 1000 in; the last delta says 500 out.
    assert_eq!(
        rig.booked().await,
        (cost(ModelProvider::Anthropic, &am(), A_TOKENS), 1500)
    );
}

const ZEN_LIST: &str = r#"{"data":[
    {"id":"big-pickle","cost":{"text":{"input":{"price":0},"output":{"price":0}}}},
    {"id":"glm-5.3-flash","cost":{"text":{"input":{"price":0.15,"cache":{"read":0.03}},"output":{"price":0.5}}},"multipliers":[{"hours":"peak","x":2}]}
]}"#;

/// A fake Zen model list whose status and body a test can change.
async fn start_zen_list(initial: &str) -> (String, Arc<Mutex<(u16, String)>>) {
    let state = Arc::new(Mutex::new((200u16, initial.to_owned())));
    let shared = state.clone();
    let app = Router::new().route(
        "/v1/models",
        any(move || {
            let (status, body) = shared.lock().unwrap().clone();
            async move {
                Response::builder()
                    .status(status)
                    .body(Body::from(body))
                    .unwrap()
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await });
    (url, state)
}

#[tokio::test]
async fn an_opencode_call_carries_the_zen_key_and_its_headers_and_books_zens_price() {
    let rig = Rig::start(Options::default()).await;
    let (list, _) = start_zen_list(ZEN_LIST).await;
    rig.proxy.zen.refresh(&rig.http, &list).await;
    let call = |model: &str| {
        rig.http
            .post(rig.url("/opencode/v1/chat/completions"))
            .bearer_auth(&rig.token)
            .header("x-opencode-session", "ses_1")
            .header("x-opencode-project", "prj_1")
            .json(&json!({"model":model,"messages":[]}))
            .send()
    };
    // A zero-priced model books nothing.
    assert_eq!(call("big-pickle").await.unwrap().status(), StatusCode::OK);
    let seen = &rig.calls()[0];
    assert_eq!(seen.path, "/v1/chat/completions");
    assert_eq!(seen.headers["authorization"], format!("Bearer {ZEN_KEY}"));
    assert_eq!(seen.headers["x-opencode-session"], "ses_1");
    assert_eq!(seen.headers["x-opencode-project"], "prj_1");
    assert_eq!(rig.booked().await, (0, 2800));

    // A priced model books Zen's price: 2000 in at $0.15 and 800 out at $0.50 per million is
    // 0.07 cents, booked as 1.
    assert_eq!(
        call("glm-5.3-flash").await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(rig.booked().await, (1, 5600));

    // A model the list does not have is booked at the highest known price.
    assert_eq!(
        call("some-paid-model").await.unwrap().status(),
        StatusCode::OK
    );
    let (cents, tokens) = rig.booked().await;
    assert!(cents > 1);
    assert_eq!(tokens, 8400);
}

#[tokio::test]
async fn a_failed_zen_refresh_keeps_the_last_table_and_logs_once() {
    let rig = Rig::start(Options::default()).await;
    let (list, state) = start_zen_list(ZEN_LIST).await;
    let zen = &rig.proxy.zen;
    let price = |model: &str| price::lookup(ModelProvider::Opencode, Some(model), zen);
    // No table yet: everything is the highest price.
    assert_eq!(price("glm-5.3-flash"), (price::highest(), false));
    zen.refresh(&rig.http, &list).await;
    let known = price("glm-5.3-flash");
    assert!(known.1);
    assert_eq!(known.0.input, 150);

    *state.lock().unwrap() = (500, "down".to_owned());
    zen.refresh(&rig.http, &list).await;
    *state.lock().unwrap() = (200, "not json".to_owned());
    zen.refresh(&rig.http, &list).await;
    assert_eq!(price("glm-5.3-flash"), known);
    assert_eq!(
        rig.logs
            .text()
            .matches("could not fetch the Zen model list")
            .count(),
        1,
        "{}",
        rig.logs.text()
    );
}

#[tokio::test]
async fn an_openai_call_and_its_stream_carry_the_key_and_book_their_usage() {
    let rig = Rig::start(Options::default()).await;
    let plain = rig
        .openai(&rig.token, json!({"model":om(),"messages":[]}))
        .await;
    assert_eq!(plain.status(), StatusCode::OK);
    assert_eq!(
        plain.json::<Value>().await.unwrap()["choices"][0]["message"]["content"],
        "hello"
    );
    assert_eq!(
        rig.calls()[0].headers["authorization"],
        format!("Bearer {OPENAI_KEY}")
    );
    // 2000 in at $1.25 and 800 out at $10 per million: 1.05 cents, booked as 2.
    assert_eq!(
        rig.booked().await,
        (cost(ModelProvider::Openai, &om(), O_TOKENS), 2800)
    );

    let streamed = rig
        .openai(
            &rig.token,
            json!({"model":om(),"stream":true,"messages":[]}),
        )
        .await;
    let text = streamed.text().await.unwrap();
    assert!(text.contains("hello") && text.contains("[DONE]"));
    // The one field that makes a chat stream report usage was turned on; nothing else was.
    let sent = &rig.calls()[1];
    assert_eq!(sent.body["stream_options"]["include_usage"], true);
    assert_eq!(sent.body["model"], om());
    assert_eq!(
        rig.booked().await,
        (2 * cost(ModelProvider::Openai, &om(), O_TOKENS), 5600)
    );
}

#[tokio::test]
async fn a_token_of_the_wrong_purpose_or_one_that_expired_is_refused_before_anything_is_forwarded()
{
    let rig = Rig::start(Options::default()).await;
    let creds = &rig.proxy.credentials;
    let mcp = creds
        .mint(rig.session, Purpose::Mcp, SignedDuration::from_mins(5))
        .await
        .unwrap();
    let expired = creds
        .mint(rig.session, Purpose::Model, SignedDuration::from_secs(-1))
        .await
        .unwrap();

    for token in [mcp.as_str(), expired.as_str(), "alc_not_a_token"] {
        let reply = rig
            .anthropic(token, json!({"model":am(),"messages":[]}))
            .await;
        assert_eq!(reply.status(), StatusCode::UNAUTHORIZED, "{token}");
        let body: Value = reply.json().await.unwrap();
        assert_eq!(body["type"], "error");
        assert_eq!(body["error"]["type"], "authentication_error");

        let reply = rig.openai(token, json!({"model":om(),"messages":[]})).await;
        assert_eq!(reply.status(), StatusCode::UNAUTHORIZED, "{token}");
        assert_eq!(
            reply.json::<Value>().await.unwrap()["error"]["type"],
            "authentication_error"
        );
    }
    // No key at all.
    let bare = rig
        .http
        .post(rig.url("/openai/v1/chat/completions"))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(bare.status(), StatusCode::UNAUTHORIZED);
    // Each provider reads its own header: a Bearer token does not open the Anthropic route.
    let wrong_header = rig
        .http
        .post(rig.url("/anthropic/v1/messages"))
        .bearer_auth(&rig.token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong_header.status(), StatusCode::UNAUTHORIZED);

    assert!(rig.calls().is_empty());
    assert_eq!(rig.booked().await, (0, 0));
}

#[tokio::test]
async fn an_organization_with_no_key_is_told_so_and_nothing_is_forwarded() {
    let rig = Rig::start(Options {
        keys: false,
        ..Options::default()
    })
    .await;
    let reply = rig
        .anthropic(&rig.token, json!({"model":am(),"messages":[]}))
        .await;
    assert_eq!(reply.status(), StatusCode::FORBIDDEN);
    let message = reply.json::<Value>().await.unwrap()["error"]["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        message.contains("no anthropic key") && message.contains("owner"),
        "{message}"
    );
    assert!(rig.calls().is_empty());
}

#[tokio::test]
async fn once_the_budget_is_spent_the_call_is_refused_in_the_providers_shape() {
    // A budget of exactly one call.
    let rig = Rig::start(Options {
        budget_cents: cost(ModelProvider::Anthropic, &am(), A_TOKENS),
        ..Options::default()
    })
    .await;
    let first = rig
        .anthropic(&rig.token, json!({"model":am(),"messages":[]}))
        .await;
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(
        rig.booked().await.0,
        cost(ModelProvider::Anthropic, &am(), A_TOKENS)
    );

    let refused = rig
        .anthropic(&rig.token, json!({"model":am(),"messages":[]}))
        .await;
    assert_eq!(refused.status(), StatusCode::PAYMENT_REQUIRED);
    let body: Value = refused.json().await.unwrap();
    assert_eq!(body["type"], "error");
    assert_eq!(body["error"]["type"], "billing_error");
    let message = body["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("budget is spent") && message.contains("request_budget"),
        "{message}"
    );

    let refused = rig
        .openai(&rig.token, json!({"model":om(),"messages":[]}))
        .await;
    assert_eq!(refused.status(), StatusCode::PAYMENT_REQUIRED);
    let body: Value = refused.json().await.unwrap();
    assert_eq!(body["error"]["code"], "insufficient_quota");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("budget is spent")
    );

    // Only the first call reached a provider, and nothing more was booked.
    assert_eq!(rig.calls().len(), 1);
    assert_eq!(
        rig.booked().await.0,
        cost(ModelProvider::Anthropic, &am(), A_TOKENS)
    );
}

#[tokio::test]
async fn a_stream_the_session_abandons_books_what_was_seen_and_logs_it() {
    let rig = Rig::start(Options::default()).await;
    let mut reply = rig
        .anthropic(
            &rig.token,
            json!({"model":"slow-stream","stream":true,"messages":[]}),
        )
        .await;
    let first = reply.chunk().await.unwrap().unwrap();
    assert!(String::from_utf8_lossy(&first).contains("message_start"));
    drop(reply);

    let mut booked = (0, 0);
    for _ in 0..60 {
        booked = rig.booked().await;
        if booked.1 > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // The start reported 700 input tokens and one output token; nothing later was reported.
    assert_eq!(
        booked,
        (
            cost(
                ModelProvider::Anthropic,
                &am(),
                Tokens {
                    input: 700,
                    output: 1,
                    cache_read: 0,
                    cache_write: 0
                }
            ),
            701
        )
    );
    assert!(
        rig.logs.text().contains("stream ended early"),
        "{}",
        rig.logs.text()
    );
}

#[tokio::test]
async fn an_unknown_model_is_booked_at_the_highest_known_price_and_logged() {
    let rig = Rig::start(Options::default()).await;
    let reply = rig
        .anthropic(&rig.token, json!({"model":"no-such-model-9","messages":[]}))
        .await;
    assert_eq!(reply.status(), StatusCode::OK);
    // 1000 in and 500 out at the table's highest ($15 and $75 per million): 5.25 cents, booked as 6.
    assert_eq!(
        rig.booked().await,
        (
            i64::try_from(price::cents(price::highest(), A_TOKENS)).unwrap(),
            1500
        )
    );
    assert!(
        rig.logs.text().contains("unknown model"),
        "{}",
        rig.logs.text()
    );
}

#[tokio::test]
async fn a_call_with_no_bill_passes_through_unmetered() {
    let rig = Rig::start(Options::default()).await;
    let reply = rig
        .http
        .get(rig.url("/anthropic/v1/models"))
        .header("x-api-key", &rig.token)
        .send()
        .await
        .unwrap();
    // The fake provider has no such route; the point is that it was forwarded, not booked.
    assert_eq!(reply.status(), StatusCode::NOT_FOUND);
    assert_eq!(rig.calls()[0].headers["x-api-key"], ANTHROPIC_KEY);
    assert_eq!(rig.booked().await, (0, 0));
}

// ---- egress -------------------------------------------------------------------------------

async fn echo_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 4];
                if stream.read_exact(&mut buf).await.is_ok() && &buf == b"ping" {
                    let _ = stream.write_all(b"pong").await;
                }
            });
        }
    });
    port
}

fn basic(token: &str) -> String {
    // "agent:<token>" in base64.
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = format!("agent:{token}");
    let mut out = String::new();
    for chunk in input.as_bytes().chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Sends a CONNECT and returns the status line's code and the open stream.
async fn connect(rig: &Rig, target: &str, token: Option<&str>) -> (u16, TcpStream) {
    let mut stream = TcpStream::connect(rig.addr).await.unwrap();
    let auth = token.map_or(String::new(), |t| {
        format!("Proxy-Authorization: Basic {}\r\n", basic(t))
    });
    stream
        .write_all(format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n{auth}\r\n").as_bytes())
        .await
        .unwrap();
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).await.unwrap();
        head.push(byte[0]);
    }
    let status = String::from_utf8_lossy(&head)
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (status, stream)
}

#[tokio::test]
async fn a_listed_host_is_tunnelled_and_the_bytes_pass_untouched() {
    let rig = Rig::start(Options {
        hosts: vec!["127.0.0.1".into()],
        allow_private: true,
        ..Options::default()
    })
    .await;
    let port = echo_server().await;
    let (status, mut tunnel) = connect(&rig, &format!("127.0.0.1:{port}"), Some(&rig.token)).await;
    assert_eq!(status, 200);
    tunnel.write_all(b"ping").await.unwrap();
    let mut answer = [0u8; 4];
    tunnel.read_exact(&mut answer).await.unwrap();
    assert_eq!(&answer, b"pong");
    assert!(rig.logs.text().contains("egress allowed"));
}

#[tokio::test]
async fn a_host_that_is_not_listed_is_refused_and_logged() {
    let rig = Rig::start(Options {
        hosts: vec!["api.stripe.com".into()],
        allow_private: true,
        ..Options::default()
    })
    .await;
    let port = echo_server().await;
    let (status, _) = connect(&rig, &format!("127.0.0.1:{port}"), Some(&rig.token)).await;
    assert_eq!(status, 403);
    let (status, _) = connect(&rig, "evil.example:443", Some(&rig.token)).await;
    assert_eq!(status, 403);
    let logs = rig.logs.text();
    assert!(
        logs.contains("egress refused: not on the allowlist"),
        "{logs}"
    );
    assert!(logs.contains("evil.example"), "{logs}");
}

#[tokio::test]
async fn a_tunnel_needs_a_live_session_token_and_never_reaches_a_private_address() {
    let rig = Rig::start(Options {
        hosts: vec!["127.0.0.1".into()],
        ..Options::default()
    })
    .await;
    let port = echo_server().await;
    let target = format!("127.0.0.1:{port}");
    let (status, _) = connect(&rig, &target, None).await;
    assert_eq!(status, 407);
    let (status, _) = connect(&rig, &target, Some("alc_wrong")).await;
    assert_eq!(status, 407);
    let mcp = rig
        .proxy
        .credentials
        .mint(rig.session, Purpose::Mcp, SignedDuration::from_mins(5))
        .await
        .unwrap();
    let (status, _) = connect(&rig, &target, Some(&mcp)).await;
    assert_eq!(status, 407);
    // Listed, but it leads to this machine's own network: refused all the same.
    let (status, _) = connect(&rig, &target, Some(&rig.token)).await;
    assert_eq!(status, 403);
    assert!(rig.logs.text().contains("only to a private address"));
}

/// A tunnel to the echo server, with the answer if it was opened.
async fn ping(rig: &Rig, target: &str) -> (u16, Option<[u8; 4]>) {
    let (status, mut tunnel) = connect(rig, target, Some(&rig.token)).await;
    if status != 200 {
        return (status, None);
    }
    tunnel.write_all(b"ping").await.unwrap();
    let mut answer = [0u8; 4];
    tunnel.read_exact(&mut answer).await.unwrap();
    (status, Some(answer))
}

#[tokio::test]
async fn no_network_refuses_every_tunnel_and_the_gateway_still_answers() {
    let rig = Rig::start(Options {
        network: NetworkLevel::None,
        // A list at this level is ignored, and so is the switch for registries.
        hosts: vec!["127.0.0.1".into()],
        registries: true,
        allow_private: true,
        ..Options::default()
    })
    .await;
    let port = echo_server().await;
    assert_eq!(ping(&rig, &format!("127.0.0.1:{port}")).await.0, 403);
    assert_eq!(ping(&rig, "registry.npmjs.org:443").await.0, 403);
    assert!(
        rig.logs
            .text()
            .contains("egress refused: not on the allowlist")
    );
    // The model gateway is a route of the proxy itself, not a tunnel.
    let reply = rig
        .anthropic(
            &rig.token,
            json!({"model":format!("{}-20250929", am()),"max_tokens":10,"messages":[]}),
        )
        .await;
    assert_eq!(reply.status(), StatusCode::OK);
}

#[tokio::test]
async fn an_mcp_call_is_passed_to_the_server_with_its_own_token() {
    let rig = Rig::start(Options::default()).await;
    rig.http
        .post(format!("http://{}/mcp/sessions/abc?x=1", rig.addr))
        .header("authorization", "Bearer the-mcp-token")
        .header("mcp-session-id", "m1")
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .send()
        .await
        .unwrap();
    let seen = rig.log.lock().unwrap().clone();
    let call = seen
        .iter()
        .find(|s| s.path == "/mcp/sessions/abc")
        .expect("the server got the call");
    assert_eq!(call.headers["authorization"], "Bearer the-mcp-token");
    assert_eq!(call.headers["mcp-session-id"], "m1");
    assert_eq!(call.body["method"], "tools/list");
}

#[tokio::test]
async fn a_platform_host_is_reached_even_with_no_network() {
    let rig = Rig::start(Options {
        network: NetworkLevel::None,
        platform_hosts: vec!["LocalHost.".into()],
        allow_private: true,
        ..Options::default()
    })
    .await;
    let port = echo_server().await;
    assert_eq!(ping(&rig, &format!("localhost:{port}")).await.0, 200);
    assert_eq!(ping(&rig, &format!("127.0.0.1:{port}")).await.0, 403);
}

#[tokio::test]
async fn a_limited_network_allows_its_listed_hosts_only() {
    let rig = Rig::start(Options {
        hosts: vec!["127.0.0.1".into()],
        allow_private: true,
        ..Options::default()
    })
    .await;
    let port = echo_server().await;
    assert_eq!(
        ping(&rig, &format!("127.0.0.1:{port}")).await,
        (200, Some(*b"pong"))
    );
    assert_eq!(ping(&rig, "pypi.org:443").await.0, 403);

    // Registries are one switch, covered where the allowlist is (`egress` unit tests); a name
    // that is neither listed nor a registry is refused.
    assert_eq!(ping(&rig, "evil.example:443").await.0, 403);
}

#[tokio::test]
async fn a_full_network_allows_any_host_but_never_a_private_address() {
    let open = Rig::start(Options {
        network: NetworkLevel::Full,
        allow_private: true,
        ..Options::default()
    })
    .await;
    let port = echo_server().await;
    // Only the test's switch lets a tunnel reach this machine; nothing is listed.
    assert_eq!(
        ping(&open, &format!("127.0.0.1:{port}")).await,
        (200, Some(*b"pong"))
    );

    let rig = Rig::start(Options {
        network: NetworkLevel::Full,
        ..Options::default()
    })
    .await;
    for target in [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        "10.1.2.3:443".to_owned(),
        "192.168.0.10:443".to_owned(),
        "169.254.169.254:80".to_owned(),
        "[::1]:443".to_owned(),
    ] {
        assert_eq!(ping(&rig, &target).await.0, 403, "{target}");
    }
    assert!(rig.logs.text().contains("only to a private address"));
}

#[tokio::test]
async fn a_session_with_no_machine_gets_no_network() {
    let rig = Rig::start(Options {
        network: NetworkLevel::Full,
        allow_private: true,
        ..Options::default()
    })
    .await;
    // A session no runner has claimed has no machine row here.
    let other = seed::session(&rig.seed, "opencode", seed::T0);
    rig.proxy
        .store
        .create_session(&other, &audit(&rig.seed, "session.start"))
        .await
        .unwrap();
    let token = rig
        .proxy
        .credentials
        .mint(other.id, Purpose::Model, SignedDuration::from_mins(5))
        .await
        .unwrap();
    let port = echo_server().await;
    let (status, _) = connect(&rig, &format!("127.0.0.1:{port}"), Some(&token)).await;
    assert_eq!(status, 403);
}

// ---- secrets stay out of logs -------------------------------------------------------------

#[tokio::test]
async fn no_key_or_token_appears_in_a_log_line() {
    let rig = Rig::start(Options {
        budget_cents: cost(ModelProvider::Anthropic, &am(), A_TOKENS),
        hosts: vec!["127.0.0.1".into()],
        ..Options::default()
    })
    .await;
    let expired = rig
        .proxy
        .credentials
        .mint(rig.session, Purpose::Model, SignedDuration::from_secs(-1))
        .await
        .unwrap();
    // Every path: a good call, a stream, a bad token, a spent budget, an openai call, a tunnel.
    rig.anthropic(&rig.token, json!({"model":am(),"messages":[]}))
        .await;
    rig.anthropic(&rig.token, json!({"model":am(),"messages":[]}))
        .await;
    rig.anthropic(&expired, json!({})).await;
    rig.openai(
        &rig.token,
        json!({"model":om(),"stream":true,"messages":[]}),
    )
    .await;
    connect(&rig, "evil.example:443", Some(&rig.token)).await;
    connect(&rig, "evil.example:443", Some(&expired)).await;

    let logs = rig.logs.text();
    assert!(!logs.is_empty());
    for secret in [
        ANTHROPIC_KEY,
        OPENAI_KEY,
        rig.token.as_str(),
        expired.as_str(),
        &basic(&rig.token),
    ] {
        assert!(!logs.contains(secret), "a secret is in the log:\n{logs}");
    }
    // The log does say who and what, so a person can follow a session.
    assert!(logs.contains(&rig.session.to_string()));
}
