//! For end-to-end tests of the server on its fakes: a win without an agent. Mounted only when
//! `ANTILOG_E2E_SEED=1`, never in OpenAPI, and only a project owner may call it.

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::organization::{Action, Refusal};
use antilog_core::seed::kept_win;
use antilog_core::store::SessionStore;
use antilog_types::ExperimentId;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};

use super::auth::{App, Authed, Fail};

async fn win<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    authed: Authed,
    Path(experiment): Path<ExperimentId>,
) -> Result<Json<Value>, Fail> {
    let row = app
        .store()
        .experiment(authed.who.organization, experiment)
        .await?
        .ok_or(Refusal::NotFound)?;
    antilog_core::organization::permit(
        app.store(),
        &authed.who,
        Action::ValueModelSet,
        Some(row.project_id),
        experiment,
        app.now(),
    )
    .await?;
    let seeded = kept_win(
        &*app.deps,
        authed.who.organization,
        experiment,
        Some(authed.who.member),
    )
    .await
    .map_err(|error| {
        tracing::error!(%error, "the seed failed");
        Fail::Refused(Refusal::Invalid("the seed failed"))
    })?;
    Ok(Json(json!({
        "trial": seeded.trial,
        "repo": seeded.repo,
        "pullRequests": seeded.pull_requests.iter().map(|(repo, number)| json!({"repo": repo, "number": number})).collect::<Vec<_>>(),
    })))
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new().route("/internal/e2e/{experiment}/win", post(win::<D>))
}
