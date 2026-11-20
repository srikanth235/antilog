//! The code host's webhook: signed, constant-time checked, mounted only with its secret.

use std::sync::Arc;

use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine};
use antilog_core::signature::sign;
use antilog_server::api::auth::Settings;
use antilog_server::api::router;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn app(secret: Option<&str>) -> Router {
    router(
        Arc::new(FakeDeps::new(
            FakeMachine::default(),
            FakeCodeHost::default(),
        )),
        Settings {
            providers: Vec::new(),
            open_organizations: false,
            secure_cookies: true,
            code_host_webhook_secret: secret.map(str::to_owned),
            e2e_seed: false,
        },
    )
}

async fn post(app: &Router, signature: Option<&str>, event: &str, body: &str) -> StatusCode {
    let mut request = Request::post("/webhooks/code-host")
        .header("x-github-event", event)
        .header("x-github-delivery", "d-1");
    if let Some(signature) = signature {
        request = request.header("x-hub-signature-256", signature);
    }
    app.clone()
        .oneshot(
            request
                .body(Body::from(body.to_owned()))
                .unwrap_or_else(|e| unreachable!("{e}")),
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e}"))
        .status()
}

const PR: &str = r#"{"action":"closed","number":9,"pull_request":{"merged":true},"repository":{"full_name":"acme/app"}}"#;

#[tokio::test]
async fn the_route_exists_only_when_the_secret_is_set() {
    assert_eq!(
        post(
            &app(None),
            Some(&sign(b"s", PR.as_bytes())),
            "pull_request",
            PR
        )
        .await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_missing_or_wrong_signature_is_refused_before_anything_is_read() {
    let app = app(Some("s3cret"));
    assert_eq!(
        post(&app, None, "pull_request", PR).await,
        StatusCode::UNAUTHORIZED
    );
    for wrong in [
        sign(b"other", PR.as_bytes()),
        sign(b"s3cret", b"{}"),
        "sha256=".to_owned(),
        "nonsense".to_owned(),
    ] {
        assert_eq!(
            post(&app, Some(&wrong), "pull_request", PR).await,
            StatusCode::UNAUTHORIZED,
            "{wrong}"
        );
    }
    // Garbage with a wrong signature is a 401, not a 400: the body is not read first.
    assert_eq!(
        post(&app, None, "pull_request", "not json").await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_signed_event_is_answered_and_one_for_a_pull_request_we_did_not_open_changes_nothing() {
    let app = app(Some("s3cret"));
    let good = sign(b"s3cret", PR.as_bytes());
    assert_eq!(
        post(&app, Some(&good), "pull_request", PR).await,
        StatusCode::OK
    );
    assert_eq!(post(&app, Some(&good), "issues", PR).await, StatusCode::OK);
    let ping = sign(b"s3cret", b"{}");
    assert_eq!(post(&app, Some(&ping), "ping", "{}").await, StatusCode::OK);
    let bad = sign(b"s3cret", b"not json");
    assert_eq!(
        post(&app, Some(&bad), "pull_request", "not json").await,
        StatusCode::BAD_REQUEST
    );
}
