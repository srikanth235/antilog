pub mod session;

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::store::Store;
use antilog_types::{ApiError, Health, HealthStatus};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use utoipa::OpenApi;

pub mod auth;
pub mod e2e;
pub mod live;
pub mod organization;
pub mod value;
pub mod webhook;

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
async fn health<D: Deps + 'static>(
    State(app): State<Arc<auth::App<D>>>,
) -> Result<Json<Health>, (StatusCode, Json<ApiError>)> {
    match app.store().ping().await {
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

pub fn router<D: Deps + 'static>(deps: Arc<D>, settings: auth::Settings) -> Router {
    router_with(Arc::new(auth::App { deps, settings }))
}

pub fn router_with<D: Deps + 'static>(app: Arc<auth::App<D>>) -> Router {
    let wake = app.settings.wake_secret.is_some();
    let router = Router::new()
        .route("/health", get(health::<D>))
        .merge(auth::routes())
        .merge(organization::routes())
        .merge(session::routes())
        .merge(value::routes())
        .merge(live::routes());
    let router = if wake {
        router.merge(session::wake_routes())
    } else {
        router
    };
    let router = if app.settings.e2e_seed {
        router.merge(e2e::routes())
    } else {
        router
    };
    let router = if app.settings.code_host_webhook_secret.is_some() {
        router.merge(webhook::routes())
    } else {
        router
    };
    router.with_state(app)
}

pub fn openapi_json() -> String {
    let mut doc = ApiDoc::openapi();
    doc.merge(auth::ApiDoc::openapi());
    doc.merge(organization::ApiDoc::openapi());
    doc.merge(session::Doc::openapi());
    doc.merge(value::Doc::openapi());
    doc.merge(live::Doc::openapi());
    doc.to_pretty_json()
        .unwrap_or_else(|error| unreachable!("the document serializes: {error}"))
}

#[cfg(test)]
mod tests {
    use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakePlatform};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_answers_ok_from_the_fake_store() {
        let app = router(
            Arc::new(FakeDeps::new(
                FakePlatform::default(),
                FakeCodeHost::default(),
            )),
            auth::Settings {
                providers: Vec::new(),
                open_organizations: false,
                secure_cookies: true,
                wake_secret: None,
                code_host_webhook_secret: None,
                e2e_seed: false,
            },
        );
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
