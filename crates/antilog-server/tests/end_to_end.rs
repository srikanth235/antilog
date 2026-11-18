#![expect(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "tests read JSON bodies and fail loudly"
)]

//! A person signs in, sets up an experiment, has the agent seal it, measures a win and reads
//! its receipt: the whole chain over HTTP on the fakes.

use std::sync::Arc;

use antilog_core::auth::FakeSignIn;
use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakePlatform};
use antilog_core::judge::fixtures::definition;
use antilog_core::judge::sandbox::Profile;
use antilog_server::api::auth::{Provider, Settings};
use antilog_server::api::router;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    auth: &str,
    body: Value,
) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {auth}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn sign_in(app: &Router) -> String {
    let get = Request::get("/auth/fake/start")
        .body(Body::empty())
        .unwrap();
    let started = app.clone().oneshot(get).await.unwrap();
    let cookie = started
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find_map(|v| {
            v.to_str()
                .ok()?
                .strip_prefix("antilog_challenge=")?
                .split(';')
                .next()
                .map(str::to_owned)
        })
        .unwrap();
    let bytes = axum::body::to_bytes(started.into_body(), 1 << 20)
        .await
        .unwrap();
    let state = serde_json::from_slice::<Value>(&bytes).unwrap()["state"]
        .as_str()
        .unwrap()
        .to_owned();
    let request = Request::post("/auth/fake/organization")
        .header(header::COOKIE, format!("antilog_challenge={cookie}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"name": "acme", "code": "fake:ada", "state": state}).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    serde_json::from_slice::<Value>(&bytes).unwrap()["session"]["secret"]
        .as_str()
        .unwrap()
        .to_owned()
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
            json!({"name": name, "commits": [{"repo": "app", "sha": format!("cheat-{i}")}],
                "changed": [["app", path]]})
        })
        .collect();
    Value::Array(cheats)
}

fn app() -> Router {
    app_with(None)
}

fn app_with(wake_secret: Option<String>) -> Router {
    let mut platform = FakePlatform::default().with("base", Profile::new(100.0));
    for i in 0..4 {
        platform = platform.with(&format!("cheat-{i}"), Profile::new(10.0).holdout(100.0));
    }
    let platform = platform.with("fast", Profile::new(80.0));
    router(
        Arc::new(FakeDeps::new(
            platform,
            FakeCodeHost::default().with("fast", "app", "src/a.rs"),
        )),
        Settings {
            providers: vec![Provider::Fake(FakeSignIn)],
            open_organizations: true,
            secure_cookies: true,
            wake_secret,
            code_host_webhook_secret: None,
            e2e_seed: false,
        },
    )
}

#[tokio::test]
async fn from_sign_in_to_a_signed_receipt() {
    let app = app();
    let owner = sign_in(&app).await;
    let post = |uri: String, body: Value| {
        let (app, owner) = (app.clone(), owner.clone());
        async move { send(&app, Method::POST, &uri, &owner, body).await }
    };
    let get = |uri: String| {
        let (app, owner) = (app.clone(), owner.clone());
        async move { send(&app, Method::GET, &uri, &owner, json!({})).await }
    };

    let (status, project) = post(
        "/projects".into(),
        json!({"name": "checkout", "description": "the path"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{project}");
    let (status, experiment) = post(
        format!("/projects/{}/experiments", project["id"].as_str().unwrap()),
        json!({"goal": "cut p95", "budgetCapCents": 100_000}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{experiment}");
    let exp = experiment["id"].as_str().unwrap().to_owned();

    let (status, session) = post(
        format!("/experiments/{exp}/sessions"),
        json!({"agent": "claude", "prompt": "cut p95"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{session}");
    let sid = session["id"].as_str().unwrap().to_owned();
    let tool = |name: &str, input: Value| {
        post(
            format!("/experiments/{exp}/tools/{name}"),
            json!({"session": sid, "input": input}),
        )
    };

    let (_, sealed) = tool(
        "seal",
        json!({"definition": definition(), "cheats": cheats()}),
    )
    .await;
    assert_eq!(
        sealed["answer"]["status"], "waiting for a person",
        "{sealed}"
    );
    let (_, cards) = get(format!("/experiments/{exp}/approvals")).await;
    let card = cards[0]["id"].as_str().unwrap();
    assert_eq!(cards[0]["status"], "pending");
    let (status, decided) = post(
        format!("/approvals/{card}/decision"),
        json!({"decision": "approve"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    let (status, stepped) = post(format!("/sessions/{sid}/step"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{stepped}");
    let (_, row) = get(format!("/experiments/{exp}")).await;
    assert_eq!(row["status"], "sealed");

    let (_, measured) = tool(
        "measure",
        json!({"commits": [{"repo": "app", "sha": "fast"}], "hypothesis": "cache"}),
    )
    .await;
    let followup = measured["followup"].as_str().unwrap();
    assert!(
        followup.contains("kept") && followup.contains("receipt"),
        "{followup}"
    );
    let (_, history) = tool("history", json!({})).await;
    let trial = history["answer"]["best"].as_str().unwrap();
    let (status, receipt) = get(format!("/trials/{trial}/receipt")).await;
    assert_eq!(status, StatusCode::OK, "{receipt}");
    assert_eq!(receipt["body"]["experiment"]["trial"], trial);
    assert_eq!(receipt["body"]["experiment"]["value"], 80.0);
    assert_eq!(receipt["body"]["experiment"]["holdout"], "passed");
    let (status, _) = get(format!(
        "/trials/{}/receipt",
        "018f3b1e-0000-7000-8000-000000000000"
    ))
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, audit) = get("/audit".into()).await;
    let started = audit["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["action"] == "session.start")
        .collect::<Vec<_>>();
    assert_eq!(started.len(), 1);
    assert!(
        started[0]["actor"].is_string(),
        "the audit names the member: {started:?}"
    );
}

#[tokio::test]
async fn a_scheduler_wake_up_steps_the_session_with_the_shared_secret() {
    let app = app_with(Some("wake-secret".into()));
    let owner = sign_in(&app).await;
    let (_, project) = send(
        &app,
        Method::POST,
        "/projects",
        &owner,
        json!({"name": "p", "description": ""}),
    )
    .await;
    let (_, experiment) = send(
        &app,
        Method::POST,
        &format!("/projects/{}/experiments", project["id"].as_str().unwrap()),
        &owner,
        json!({"goal": "g", "budgetCapCents": 1000}),
    )
    .await;
    let (_, session) = send(
        &app,
        Method::POST,
        &format!(
            "/experiments/{}/sessions",
            experiment["id"].as_str().unwrap()
        ),
        &owner,
        json!({"agent": "claude", "prompt": "go"}),
    )
    .await;
    let (_, me) = send(&app, Method::GET, "/organization", &owner, json!({})).await;
    let org = me["id"].as_str().unwrap().to_owned();
    let wake = |org: String, job: String, secret: Option<&'static str>| {
        let mut request = Request::post(format!("/internal/wake/{org}/{job}"));
        if let Some(secret) = secret {
            request = request.header("x-antilog-wake", secret);
        }
        let request = request.body(Body::empty()).unwrap();
        let app = app.clone();
        async move { app.oneshot(request).await.unwrap().status() }
    };
    let id = session["id"].as_str().unwrap().to_owned();
    assert_eq!(
        wake(org.clone(), id.clone(), None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        wake(org.clone(), id.clone(), Some("wrong")).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        wake(org.clone(), id.clone(), Some("wake-secret")).await,
        StatusCode::OK
    );
    assert_eq!(
        wake(org.clone(), id, Some("wake-secret")).await,
        StatusCode::OK,
        "a second wake-up is safe"
    );
    let ghost = "018f3b1e-0000-7000-8000-000000000000";
    assert_eq!(
        wake(org, ghost.into(), Some("wake-secret")).await,
        StatusCode::NO_CONTENT
    );
    let doc: Value = serde_json::from_str(&antilog_server::api::openapi_json()).unwrap();
    assert!(
        doc["paths"]
            .as_object()
            .unwrap()
            .keys()
            .all(|p| !p.starts_with("/internal"))
    );
}

#[tokio::test]
async fn the_wake_route_is_absent_without_a_secret() {
    let app = app();
    let request = Request::post(
        "/internal/wake/018f3b1e-0000-7000-8000-000000000000/018f3b1e-0000-7000-8000-000000000000",
    )
    .body(Body::empty())
    .unwrap();
    assert_eq!(
        app.oneshot(request).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
}
