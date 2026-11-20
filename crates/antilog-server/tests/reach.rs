#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::expect_used)]

//! The reach routes end to end on the fakes: Customize, the per-session MCP endpoint, and a mock
//! MCP server as the connector.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use antilog_core::clock::{Clock, SystemClock};
use antilog_core::credentials::{Purpose, SessionCredentials};
use antilog_core::fakes::{FakeCodeHost, FakeCredentials, FakeDeps, FakeMachine};
use antilog_core::sealed::Sealer;
use antilog_core::secrets::Secret;
use antilog_core::seed;
use antilog_server::api::{self, auth, customize::Backend};
use antilog_server::connectors::HttpConnectors;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use jiff::SignedDuration;
use serde_json::{Value, json};

type Seen = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A remote MCP server that wants the bearer `s3cret`. `tools/call` answers as an event stream.
async fn mock(
    State(seen): State<Seen>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let method = body["method"].as_str().unwrap_or_default().to_owned();
    seen.lock().unwrap().push((method.clone(), bearer.clone()));
    if bearer.as_deref() != Some("Bearer s3cret") {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let id = body["id"].clone();
    let result = match method.as_str() {
        "initialize" => {
            json!({"protocolVersion": "2025-03-26", "capabilities": {"tools": {}}, "serverInfo": {"name": "mock", "version": "1"}})
        }
        "tools/list" => json!({"tools": [
            {"name": "lookup", "description": "Look a thing up", "inputSchema": {"type": "object"}},
            {"name": "wipe", "description": "Wipe", "inputSchema": {"type": "object"}}]}),
        "tools/call" => {
            let text = format!("looked up {}", body["params"]["arguments"]["what"]);
            let message = json!({"jsonrpc": "2.0", "id": id, "result": {"content": [{"type": "text", "text": text}]}});
            return (
                [("content-type", "text/event-stream")],
                format!("event: message\ndata: {message}\n\n"),
            )
                .into_response();
        }
        _ => return StatusCode::ACCEPTED.into_response(),
    };
    (
        [("mcp-session-id", "mock-1")],
        Json(json!({"jsonrpc": "2.0", "id": id, "result": result})),
    )
        .into_response()
}

async fn serve(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    format!("http://127.0.0.1:{port}")
}

struct World {
    base: String,
    http: reqwest::Client,
    bearer: String,
    seed: seed::Seed,
    deps: Arc<FakeDeps>,
    credentials: Arc<FakeCredentials>,
    seen: Seen,
    mock_url: String,
}

/// The credentials are shared so the test can mint tokens the endpoint then verifies.
struct Shared(Arc<FakeCredentials>);

impl SessionCredentials for Shared {
    async fn mint(
        &self,
        s: antilog_types::SessionId,
        p: Purpose,
        ttl: SignedDuration,
    ) -> Result<String, antilog_core::credentials::CredentialsError> {
        self.0.mint(s, p, ttl).await
    }
    async fn verify(
        &self,
        t: &str,
        p: Purpose,
    ) -> Result<Option<antilog_types::SessionId>, antilog_core::credentials::CredentialsError> {
        self.0.verify(t, p).await
    }
    async fn revoke_all(
        &self,
        s: antilog_types::SessionId,
    ) -> Result<(), antilog_core::credentials::CredentialsError> {
        self.0.revoke_all(s).await
    }
}

async fn world() -> World {
    let seen = Seen::default();
    let mock_url = serve(
        Router::new()
            .route("/mcp", post(mock))
            .with_state(seen.clone()),
    )
    .await;
    let deps = Arc::new(FakeDeps::new(
        FakeMachine::default(),
        FakeCodeHost::default(),
    ));
    let seed = seed::experiment(&deps.store, 1_000).await;
    let now = deps.clock.now().as_millisecond();
    let issued = antilog_core::token::issue(seed.organization.id, seed.owner.id, "t", now).unwrap();
    let entry = antilog_types::AuditEntry::new(
        seed.organization.id,
        Some(seed.owner.id),
        "token.create",
        seed.owner.id,
        antilog_types::AuditOutcome::Done,
        now,
    );
    antilog_core::store::OrganizationStore::create_access_token(
        &deps.store,
        &issued.record,
        &entry,
    )
    .await
    .unwrap();
    let credentials = Arc::new(FakeCredentials::new(Arc::new(SystemClock)));
    let app = Arc::new(auth::App {
        deps: deps.clone(),
        settings: auth::Settings {
            providers: Vec::new(),
            open_organizations: false,
            secure_cookies: true,
            code_host_webhook_secret: None,
            e2e_seed: false,
        },
    });
    let router = api::router_with_reach(
        app,
        Shared(credentials.clone()),
        Backend {
            client: HttpConnectors::new(true).unwrap(),
            sealer: Some(Sealer::new(&Secret::new("cd".repeat(32))).unwrap()),
        },
    );
    World {
        base: serve(router).await,
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap(),
        bearer: issued.secret,
        seed,
        deps,
        credentials,
        seen,
        mock_url: format!("{mock_url}/mcp"),
    }
}

impl World {
    async fn api(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.base))
            .bearer_auth(&self.bearer);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    async fn rpc(
        &self,
        session: antilog_types::SessionId,
        token: &str,
        id: u64,
        method: &str,
        params: Value,
    ) -> (StatusCode, Value) {
        let response = self
            .http
            .post(format!("{}/mcp/sessions/{session}", self.base))
            .bearer_auth(token)
            .header("accept", "application/json, text/event-stream")
            .json(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let text = response.text().await.unwrap();
        let value = serde_json::from_str(&text).or_else(|_| {
            let data: String = text
                .lines()
                .filter_map(|l| l.strip_prefix("data:"))
                .collect();
            serde_json::from_str(data.trim())
        });
        (status, value.unwrap_or(Value::Null))
    }
}

#[tokio::test]
async fn a_connector_is_added_switched_on_and_served_to_a_session_without_its_credential() {
    let w = world().await;
    let (status, added) = w
        .api(
            reqwest::Method::POST,
            "/connectors",
            Some(json!({"name": "Lookup", "url": w.mock_url, "credential": "s3cret"})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{added}");
    assert_eq!(added["health"], "connected");
    assert_eq!(added["hasCredential"], true);
    assert!(!added.to_string().contains("s3cret"));
    let id = added["id"].as_str().unwrap().to_owned();
    let (_, listed) = w.api(reqwest::Method::GET, "/connectors", None).await;
    assert!(!listed.to_string().contains("s3cret"));
    let (status, _) = w
        .api(
            reqwest::Method::PUT,
            &format!("/connectors/{id}/tools/lookup"),
            Some(json!({"mode": "on"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = w
        .api(
            reqwest::Method::PUT,
            &format!("/connectors/{id}/tools/wipe"),
            Some(json!({"mode": "off"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let experiment = w.seed.experiment.id;
    let (status, _) = w
        .api(
            reqwest::Method::PUT,
            &format!("/experiments/{experiment}/connectors"),
            Some(json!({"connectors": [id]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let session = antilog_core::session::open_external(
        &*w.deps,
        w.seed.organization.id,
        experiment,
        Some(w.seed.owner.id),
    )
    .await
    .unwrap();
    let token = w
        .credentials
        .mint(session.id, Purpose::Mcp, SignedDuration::from_secs(600))
        .await
        .unwrap();

    // The endpoint needs the session's own MCP token.
    let model = w
        .credentials
        .mint(session.id, Purpose::Model, SignedDuration::from_secs(600))
        .await
        .unwrap();
    let other =
        antilog_core::session::open_external(&*w.deps, w.seed.organization.id, experiment, None)
            .await
            .unwrap();
    let other_token = w
        .credentials
        .mint(other.id, Purpose::Mcp, SignedDuration::from_secs(600))
        .await
        .unwrap();
    for bad in ["nonsense", model.as_str(), other_token.as_str()] {
        let (status, _) = w.rpc(session.id, bad, 1, "tools/list", json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{bad}");
    }

    let (status, listed) = w.rpc(session.id, &token, 1, "tools/list", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.contains(&"seal") && names.contains(&"measure"),
        "{names:?}"
    );
    assert!(names.contains(&"lookup__lookup"), "{names:?}");
    assert!(
        !names.contains(&"lookup__wipe"),
        "off is not offered: {names:?}"
    );

    let (status, called) = w
        .rpc(
            session.id,
            &token,
            2,
            "tools/call",
            json!({"name": "lookup__lookup", "arguments": {"what": "x"}}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{called}");
    assert_eq!(called["result"]["content"][0]["text"], "looked up \"x\"");
    assert_eq!(called["result"]["isError"], json!(false));
    let seen = w.seen.lock().unwrap().clone();
    assert!(
        seen.iter()
            .any(|(m, b)| m == "tools/call" && b.as_deref() == Some("Bearer s3cret")),
        "the server sent the credential: {seen:?}"
    );

    let (_, history) = w
        .rpc(
            session.id,
            &token,
            3,
            "tools/call",
            json!({"name": "history", "arguments": {}}),
        )
        .await;
    assert_eq!(history["result"]["isError"], json!(false), "{history}");

    let (_, wiped) = w
        .rpc(
            session.id,
            &token,
            4,
            "tools/call",
            json!({"name": "lookup__wipe", "arguments": {}}),
        )
        .await;
    assert_eq!(
        wiped["result"]["isError"],
        json!(true),
        "an off tool is refused: {wiped}"
    );
}

#[tokio::test]
async fn a_skill_is_uploaded_selected_and_downloaded_by_version() {
    let w = world().await;
    let body = json!({"files": [
        {"path": "SKILL.md", "content": "---\nname: pg-tuning\ndescription: Tunes Postgres\n---\n# Tuning"},
        {"path": "refs/plans.md", "content": "Read EXPLAIN."}]});
    let (status, made) = w
        .api(reqwest::Method::POST, "/skills", Some(body.clone()))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    assert_eq!(made["version"], 1);
    let (status, bad) = w
        .api(
            reqwest::Method::POST,
            "/skills",
            Some(json!({"files": [{"path": "x.md", "content": "x"}]})),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    let experiment = w.seed.experiment.id;
    let (status, chosen) = w
        .api(
            reqwest::Method::PUT,
            &format!("/experiments/{experiment}/skills"),
            Some(json!({"skills": ["pg-tuning"]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{chosen}");
    assert_eq!(chosen[0]["version"], 1);
    let project = w.seed.experiment.project_id;
    let (status, _) = w
        .api(
            reqwest::Method::PUT,
            &format!("/projects/{project}/skills/pg-tuning"),
            Some(json!({"on": false})),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, skills) = w
        .api(
            reqwest::Method::GET,
            &format!("/skills?project={project}"),
            None,
        )
        .await;
    assert_eq!(skills[0]["on"], false);
    let (status, refused) = w
        .api(reqwest::Method::DELETE, "/skills/pg-tuning", None)
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");

    // The runner's route reads what its session's experiment selected, and nothing else.
    let session =
        antilog_core::session::open_external(&*w.deps, w.seed.organization.id, experiment, None)
            .await
            .unwrap();
    let got = api::customize::session_skill(&*w.deps, session.id, "pg-tuning", 1)
        .await
        .unwrap();
    assert_eq!(got.files.len(), 2);
    assert!(
        api::customize::session_skill(&*w.deps, session.id, "pg-tuning", 2)
            .await
            .is_err()
    );
    assert!(
        api::customize::session_skill(&*w.deps, session.id, "other", 1)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_plugin_is_unpacked_and_removed_by_part() {
    let w = world().await;
    let (status, plugin) = w
        .api(reqwest::Method::POST, "/plugins", Some(json!({
            "name": "Kit", "version": "1", "description": "d",
            "skills": [{"files": [{"path": "SKILL.md", "content": "---\nname: kit-skill\ndescription: d\n---\n"}]}],
            "connectors": [{"name": "Kit MCP", "url": w.mock_url}],
            "evaluations": []})))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{plugin}");
    let (_, connectors) = w.api(reqwest::Method::GET, "/connectors", None).await;
    assert_eq!(connectors[0]["health"], "not_checked");
    let (status, _) = w
        .api(
            reqwest::Method::POST,
            "/plugins/kit/remove",
            Some(json!({"keep": [{"kind": "skill", "name": "kit-skill"}]})),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, skills) = w.api(reqwest::Method::GET, "/skills", None).await;
    assert_eq!(skills.as_array().unwrap().len(), 1, "the kept skill stays");
    let (_, connectors) = w.api(reqwest::Method::GET, "/connectors", None).await;
    assert!(connectors.as_array().unwrap().is_empty());
}
