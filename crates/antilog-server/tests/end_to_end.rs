#![expect(
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "tests read JSON bodies and fail loudly"
)]

//! A person signs in, starts a session, talks to it, reads what it spent and stops it: the whole
//! chain over HTTP on the fakes.

use std::sync::Arc;

use antilog_core::auth::FakeSignIn;
use antilog_core::fakes::FakeDeps;
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

fn app() -> Router {
    router(
        Arc::new(FakeDeps::new()),
        Settings {
            providers: vec![Provider::Fake(FakeSignIn)],
            open_organizations: true,
            secure_cookies: true,
            runner: None,
        },
    )
}

#[tokio::test]
async fn from_sign_in_to_a_stopped_session() {
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

    let (status, empty) = post(
        "/sessions".into(),
        json!({"prompt": "cut p95", "repos": ["acme/app"]}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "this server has no default model: {empty}"
    );
    let (status, session) = post(
        "/sessions".into(),
        json!({
            "prompt": "cut p95 on the checkout path\nmore detail",
            "repos": ["acme/app"],
            "model": "opencode/big-pickle",
            "budgetCapCents": 500
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{session}");
    let sid = session["id"].as_str().unwrap().to_owned();
    assert_eq!(session["title"], "cut p95 on the checkout path");
    assert_eq!(session["status"], "working");
    assert_eq!(session["repos"], json!(["acme/app"]));
    assert_eq!(session["budgetCapCents"], 500);
    assert_eq!(session["model"], "opencode/big-pickle");

    let (_, listed) = get("/sessions".into()).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["id"], sid.as_str());
    let (_, mine) = get("/sessions?mine=true".into()).await;
    assert_eq!(mine.as_array().unwrap().len(), 1);

    let (status, after) = post(
        format!("/sessions/{sid}/messages"),
        json!({"text": "try caching"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    let (_, events) = get(format!("/sessions/{sid}/events")).await;
    let said: Vec<&str> = events["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        said,
        ["cut p95 on the checkout path\nmore detail", "try caching"]
    );
    let (_, spend) = get(format!("/sessions/{sid}/spend")).await;
    assert_eq!(spend["spentCents"], 0);
    assert_eq!(spend["budgetCapCents"], 500);

    let (status, stopped) = post(format!("/sessions/{sid}/stop"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert_eq!(stopped["status"], "done");
    let (status, _) = post(
        format!("/sessions/{sid}/messages"),
        json!({"text": "again"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "a stopped session stays so");

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
