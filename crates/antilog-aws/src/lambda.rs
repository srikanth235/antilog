//! The server's `Router` on Lambda, behind a function URL with response streaming.
//!
//! Two callers reach it. The function URL carries browsers and the CLI. EventBridge
//! Scheduler invokes the function directly with an event shaped like a function URL
//! request, so the same `Router` handles a wake-up as `POST /internal/wake/{job}`. A
//! direct invoke needs `lambda:InvokeFunction`, which only the scheduler role holds. A
//! path under `/internal/` is therefore refused unless the event says the scheduler sent
//! it: a function URL request always carries the URL's own domain name, so no outside
//! caller can forge the scheduler's.

use axum::Router;
use lambda_http::http::{Request, Response, StatusCode};
use lambda_http::request::RequestContext;
use lambda_http::{Body, Error, RequestExt, service_fn};
use tower::ServiceExt;

/// The domain name a scheduler-sent event carries in its request context.
pub const SCHEDULER_DOMAIN: &str = "scheduler.antilog.internal";
pub const INTERNAL_PREFIX: &str = "/internal/";
pub const WAKE_PREFIX: &str = "/internal/wake/";

/// The event Scheduler hands the function for one job: a `POST` to `/internal/wake/{job}`.
pub fn wake_event(job: &str) -> String {
    let path = format!("{WAKE_PREFIX}{job}");
    serde_json::json!({
        "version": "2.0",
        "routeKey": "$default",
        "rawPath": path,
        "rawQueryString": "",
        "headers": {"content-type": "application/json"},
        "requestContext": {
            "accountId": "anonymous",
            "apiId": "antilog-scheduler",
            "domainName": SCHEDULER_DOMAIN,
            "domainPrefix": "scheduler",
            "http": {
                "method": "POST",
                "path": path,
                "protocol": "HTTP/1.1",
                "sourceIp": "127.0.0.1",
                "userAgent": "antilog-scheduler"
            },
            "requestId": "scheduler",
            "routeKey": "$default",
            "stage": "$default",
            "time": "01/Jan/2026:00:00:00 +0000",
            "timeEpoch": 0
        },
        "body": "",
        "isBase64Encoded": false
    })
    .to_string()
}

fn from_scheduler(request: &Request<Body>) -> bool {
    matches!(
        request.request_context_ref(),
        Some(RequestContext::ApiGatewayV2(context))
            if context.domain_name.as_deref() == Some(SCHEDULER_DOMAIN)
    )
}

/// Runs one request through the router, keeping `/internal/` for the scheduler.
pub async fn handle(
    router: Router,
    request: Request<Body>,
) -> Result<Response<axum::body::Body>, Error> {
    if request.uri().path().starts_with(INTERNAL_PREFIX) && !from_scheduler(&request) {
        let mut refused = Response::new(axum::body::Body::from(
            r#"{"code":"not_found","message":"No such route.","fields":[]}"#,
        ));
        *refused.status_mut() = StatusCode::NOT_FOUND;
        refused.headers_mut().insert(
            lambda_http::http::header::CONTENT_TYPE,
            lambda_http::http::HeaderValue::from_static("application/json"),
        );
        return Ok(refused);
    }
    let (parts, body) = request.into_parts();
    let body = match body {
        Body::Empty => axum::body::Body::empty(),
        Body::Text(text) => axum::body::Body::from(text),
        Body::Binary(bytes) => axum::body::Body::from(bytes),
        // `Body` is non-exhaustive in spirit; anything new is treated as empty.
        _ => axum::body::Body::empty(),
    };
    match router.oneshot(Request::from_parts(parts, body)).await {
        Ok(response) => Ok(response),
        Err(never) => match never {},
    }
}

/// Serves `router` on Lambda with response streaming, until the runtime stops.
/// Build the router first (connections are lazy, so a paused database does not block
/// start-up) and pass it in; nothing here knows how it was wired.
pub async fn serve(router: Router) -> Result<(), Error> {
    lambda_http::run_with_streaming_response(service_fn(move |request| {
        handle(router.clone(), request)
    }))
    .await
}

#[cfg(test)]
mod tests {
    use axum::routing::{get, post};
    use lambda_http::request::from_str;

    use super::*;

    fn router() -> Router {
        Router::new()
            .route("/health", get(|| async { "ok" }))
            .route("/internal/wake/{job}", post(|| async { "woke" }))
    }

    async fn text(response: Response<axum::body::Body>) -> (StatusCode, String) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap_or_default();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    fn url_request(path: &str, method: &str) -> Request<Body> {
        from_str(
            &serde_json::json!({
                "version": "2.0",
                "routeKey": "$default",
                "rawPath": path,
                "rawQueryString": "",
                "headers": {},
                "requestContext": {
                    "accountId": "anonymous",
                    "apiId": "abc",
                    "domainName": "abc.lambda-url.us-east-1.on.aws",
                    "domainPrefix": "abc",
                    "http": {"method": method, "path": path, "protocol": "HTTP/1.1", "sourceIp": "1.2.3.4", "userAgent": "curl"},
                    "requestId": "r",
                    "routeKey": "$default",
                    "stage": "$default",
                    "time": "01/Jan/2026:00:00:00 +0000",
                    "timeEpoch": 0
                },
                "isBase64Encoded": false
            })
            .to_string(),
        )
        .unwrap_or_else(|e| unreachable!("{e}"))
    }

    #[tokio::test]
    async fn a_function_url_request_reaches_the_router() {
        let response = handle(router(), url_request("/health", "GET"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(text(response).await, (StatusCode::OK, "ok".to_owned()));
    }

    #[tokio::test]
    async fn the_scheduler_event_is_a_wake_request() {
        let request = from_str(&wake_event("00ff")).unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(request.uri().path(), "/internal/wake/00ff");
        let response = handle(router(), request)
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(text(response).await, (StatusCode::OK, "woke".to_owned()));
    }

    #[tokio::test]
    async fn the_internet_cannot_reach_internal_paths() {
        let response = handle(router(), url_request("/internal/wake/00ff", "POST"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        let (status, body) = text(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("not_found"));
    }
}
