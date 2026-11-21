//! The model keys an organization brings. A key goes in and never comes out: the list says which
//! providers have one and when it was set.

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::model_keys;
use antilog_types::{ApiError, ModelInfo, ModelKeyStatus, ModelProvider, NewModelKey};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use utoipa::OpenApi;

use super::auth::{App, Authed, Fail};

type Shared<D> = State<Arc<App<D>>>;

#[derive(OpenApi)]
#[openapi(paths(list_model_keys, set_model_key, remove_model_key, list_models))]
pub struct Doc;

#[utoipa::path(
    get, path = "/model-keys", security(("bearer" = [])),
    responses((status = 200, description = "The providers that have a key, with when it was set; never a key", body = Vec<ModelKeyStatus>))
)]
async fn list_model_keys<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<ModelKeyStatus>>, Fail> {
    Ok(Json(model_keys::list(app.store(), &authed.who).await?))
}

#[utoipa::path(
    get, path = "/models", security(("bearer" = [])),
    responses((status = 200, description = "The models of the providers that have a key, by provider then name, each with its context window, prices and effort levels. Listed from the vendored LiteLLM table and OpenCode's own lists, which are kept for an hour", body = Vec<ModelInfo>))
)]
async fn list_models<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<ModelInfo>>, Fail> {
    let keyed: Vec<ModelProvider> = model_keys::list(app.store(), &authed.who)
        .await?
        .into_iter()
        .map(|key| key.provider)
        .collect();
    Ok(Json(app.settings.models.models(Some(&keyed)).await))
}

#[utoipa::path(
    put, path = "/model-keys/{provider}", operation_id = "set_model_key", security(("bearer" = [])),
    params(("provider" = ModelProvider, Path, description = "The provider the key is for")),
    request_body = NewModelKey,
    responses(
        (status = 200, description = "Set or replaced; organization owners only. The key is not returned", body = ModelKeyStatus),
        (status = 422, description = "Not a usable key", body = ApiError),
        (status = 503, description = "The server has no master key to seal it with", body = ApiError),
    )
)]
async fn set_model_key<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(provider): Path<ModelProvider>,
    Json(new): Json<NewModelKey>,
) -> Result<Json<ModelKeyStatus>, Fail> {
    Ok(Json(
        model_keys::set(app.store(), &authed.who, app.now(), provider, new).await?,
    ))
}

#[utoipa::path(
    delete, path = "/model-keys/{provider}", operation_id = "remove_model_key", security(("bearer" = [])),
    params(("provider" = ModelProvider, Path, description = "The provider whose key to remove")),
    responses(
        (status = 204, description = "Removed; organization owners only"),
        (status = 404, description = "There is no key for that provider", body = ApiError),
    )
)]
async fn remove_model_key<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(provider): Path<ModelProvider>,
) -> Result<StatusCode, Fail> {
    model_keys::remove(app.store(), &authed.who, app.now(), provider).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route("/models", get(list_models::<D>))
        .route("/model-keys", get(list_model_keys::<D>))
        .route(
            "/model-keys/{provider}",
            put(set_model_key::<D>).delete(remove_model_key::<D>),
        )
}
