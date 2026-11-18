use std::sync::Arc;

use antilog_core::store::Store;
use antilog_types::{ApiError, Health, HealthStatus};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use utoipa::OpenApi;

pub mod auth;
pub mod organization;

#[derive(OpenApi)]
#[openapi(
    info(title = "Antilog API", version = env!("CARGO_PKG_VERSION")),
    paths(health)
)]
struct ApiDoc;

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "The server and its store are up", body = Health),
        (status = 503, description = "The store does not answer", body = ApiError),
    )
)]
async fn health<S: Store + 'static>(
    State(store): State<Arc<S>>,
) -> Result<Json<Health>, (StatusCode, Json<ApiError>)> {
    match store.ping().await {
        Ok(()) => Ok(Json(Health {
            status: HealthStatus::Ok,
        })),
        Err(error) => {
            tracing::error!(%error, "store ping failed");
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiError {
                    code: "store_unavailable".to_owned(),
                    message: "The store does not answer; retry shortly.".to_owned(),
                    fields: Vec::new(),
                }),
            ))
        }
    }
}

pub fn router<S: Store + 'static>(store: Arc<S>) -> Router {
    let app = Arc::new(auth::App {
        store: store.clone(),
        clock: Arc::new(antilog_core::clock::SystemClock),
        settings: auth::Settings::from_env(),
    });
    router_with(store, app)
}

pub fn router_with<S: Store + 'static>(store: Arc<S>, app: Arc<auth::App<S>>) -> Router {
    Router::new()
        .route("/health", get(health::<S>))
        .with_state(store)
        .merge(auth::routes().merge(organization::routes()).with_state(app))
}

pub fn openapi_json() -> String {
    let mut doc = ApiDoc::openapi();
    doc.merge(auth::ApiDoc::openapi());
    doc.merge(organization::ApiDoc::openapi());
    doc.to_pretty_json()
        .unwrap_or_else(|error| unreachable!("the document serializes: {error}"))
}

#[cfg(test)]
mod tests {
    use antilog_core::fakes::FakeStore;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_answers_ok_from_the_fake_store() {
        let app = router(Arc::new(FakeStore::default()));
        let response = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), br#"{"status":"ok"}"#);
    }

    #[test]
    fn committed_openapi_document_is_current() {
        let committed = include_str!("../../../openapi/antilog.json");
        assert_eq!(
            committed.trim_end(),
            openapi_json(),
            "run `cargo run -p antilog-server -- openapi > openapi/antilog.json`"
        );
    }
}
