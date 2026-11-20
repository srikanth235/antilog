//! The server's `Router` on Lambda, behind a function URL with response streaming.
//!
//! Two callers reach it. The function URL carries browsers and the CLI. The deploy workflow
//! invokes the function directly with an event shaped like a function URL request, so the
//! same `Router` handles `POST /internal/migrate`. A direct invoke needs
//! `lambda:InvokeFunction`, which only the deploy role holds. A path under `/internal/` is
//! therefore refused unless the event says the deploy sent it: a function URL request always
//! carries the URL's own domain name, so no outside caller can forge the deploy's.
//!
//! Everything else must carry CloudFront's origin secret (`antilog_server::origin`), with
//! no exception for `/health`; with no secret set the function refuses all of it.

use antilog_server::origin::OriginGuard;
use axum::Router;
use lambda_http::http::{Request, Response, StatusCode};
use lambda_http::request::RequestContext;
use lambda_http::{Body, Error, RequestExt, service_fn};
use tower::ServiceExt;

/// The domain name a deploy-sent event carries in its request context.
pub const DEPLOY_DOMAIN: &str = "deploy.antilog.internal";
pub const INTERNAL_PREFIX: &str = "/internal/";

fn from_deploy(request: &Request<Body>) -> bool {
    matches!(
        request.request_context_ref(),
        Some(RequestContext::ApiGatewayV2(context))
            if context.domain_name.as_deref() == Some(DEPLOY_DOMAIN)
    )
}

/// Runs one request through the router, keeping `/internal/` for the deploy.
pub async fn handle(
    router: Router,
    guard: &OriginGuard,
    request: Request<Body>,
) -> Result<Response<axum::body::Body>, Error> {
    let deploy = from_deploy(&request);
    // Only CloudFront (which adds the origin secret) and the deploy may reach the server.
    if !deploy && !guard.allows(request.headers()) {
        return Ok(OriginGuard::refusal());
    }
    if request.uri().path().starts_with(INTERNAL_PREFIX) && !deploy {
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
pub async fn serve(router: Router, guard: OriginGuard) -> Result<(), Error> {
    let guard = std::sync::Arc::new(guard);
    lambda_http::run_with_streaming_response(service_fn(move |request| {
        let (router, guard) = (router.clone(), guard.clone());
        async move { handle(router, &guard, request).await }
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
            .route("/internal/migrate", post(|| async { "migrated" }))
    }

    fn open() -> OriginGuard {
        OriginGuard::off()
    }

    fn with_secret(mut request: Request<Body>, secret: &str) -> Request<Body> {
        request.headers_mut().insert(
            antilog_server::origin::ORIGIN_HEADER,
            secret.parse().unwrap_or_else(|e| unreachable!("{e}")),
        );
        request
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
        let response = handle(router(), &open(), url_request("/health", "GET"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(text(response).await, (StatusCode::OK, "ok".to_owned()));
    }

    fn deploy_event() -> Request<Body> {
        from_str(include_str!("../../../deploy/tofu/aws/events/migrate.json"))
            .unwrap_or_else(|e| unreachable!("{e}"))
    }

    #[tokio::test]
    async fn the_deploy_event_reaches_an_internal_path() {
        let request = deploy_event();
        assert_eq!(request.uri().path(), "/internal/migrate");
        let response = handle(router(), &open(), request)
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            text(response).await,
            (StatusCode::OK, "migrated".to_owned())
        );
    }

    #[tokio::test]
    async fn the_internet_cannot_reach_internal_paths() {
        let response = handle(router(), &open(), url_request("/internal/migrate", "POST"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        let (status, body) = text(response).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("not_found"));
    }

    #[tokio::test]
    async fn without_the_origin_secret_the_function_refuses_everything_but_the_deploy() {
        let guard = OriginGuard::requiring([antilog_core::secrets::Secret::new("front")]);
        for request in [
            url_request("/health", "GET"),
            with_secret(url_request("/health", "GET"), "back"),
        ] {
            let response = handle(router(), &guard, request)
                .await
                .unwrap_or_else(|e| unreachable!("{e}"));
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
        let through = handle(
            router(),
            &guard,
            with_secret(url_request("/health", "GET"), "front"),
        )
        .await
        .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(text(through).await, (StatusCode::OK, "ok".to_owned()));
        let response = handle(router(), &guard, deploy_event())
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(
            text(response).await,
            (StatusCode::OK, "migrated".to_owned())
        );
    }

    #[tokio::test]
    async fn an_unset_secret_fails_closed() {
        let guard = OriginGuard::requiring([]);
        let response = handle(router(), &guard, url_request("/health", "GET"))
            .await
            .unwrap_or_else(|e| unreachable!("{e}"));
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
