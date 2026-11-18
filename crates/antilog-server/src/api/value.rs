//! Value models, value and Return, and the monthly Report. Handlers decode, call
//! `antilog_core::value` and encode; they test no role. Every figure is "measured offline".

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::value::{self, Range};
use antilog_types::judge::{
    ExperimentValue, NewValueModel, ProjectValueModel, Report, ValueFigures, ValueModels,
};
use antilog_types::{ApiError, ExperimentId, ProjectId};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use utoipa::{IntoParams, OpenApi};

use super::auth::{App, Authed, Fail};

type Shared<D> = State<Arc<App<D>>>;
type Reply<T> = Result<Json<T>, Fail>;

#[derive(OpenApi)]
#[openapi(paths(
    value_models,
    set_value_model,
    project_value,
    organization_value,
    experiment_value,
    report
))]
pub struct Doc;

/// A time range in epoch milliseconds, both ends optional: `from` included, `to` excluded.
/// Left out, it is all time.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
struct Between {
    from: Option<i64>,
    to: Option<i64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
struct ReportQuery {
    /// The month as `YYYY-MM`, in UTC; the current month when left out.
    month: Option<String>,
    /// One project; left out, the organization.
    project: Option<ProjectId>,
}

#[utoipa::path(
    get, path = "/projects/{id}/value-models", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project")),
    responses(
        (status = 200, description = "The newest version of each metric's value model", body = ValueModels),
        (status = 404, body = ApiError),
    )
)]
async fn value_models<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(project): Path<ProjectId>,
) -> Reply<ValueModels> {
    let judge = app.deps.judge(authed.who.organization);
    Ok(Json(
        value::value_models(app.store(), &judge, &authed.who, project).await?,
    ))
}

#[utoipa::path(
    post, path = "/projects/{id}/value-models", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = NewValueModel,
    responses(
        (status = 201, description = "Set: a project owner only, a new version when the metric had one", body = ProjectValueModel),
        (status = 403, description = "Not a project owner", body = ApiError),
        (status = 422, body = ApiError),
    )
)]
async fn set_value_model<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(project): Path<ProjectId>,
    Json(new): Json<NewValueModel>,
) -> Result<(StatusCode, Json<ProjectValueModel>), Fail> {
    let judge = app.deps.judge(authed.who.organization);
    let model =
        value::set_value_model(app.store(), &judge, &authed.who, app.now(), project, new).await?;
    Ok((StatusCode::CREATED, Json(model)))
}

#[utoipa::path(
    get, path = "/projects/{id}/value", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project"), Between),
    responses(
        (status = 200, description = "Delivered, Pending, At risk and Return of the project, measured offline", body = ValueFigures),
        (status = 404, body = ApiError),
    )
)]
async fn project_value<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(project): Path<ProjectId>,
    Query(between): Query<Between>,
) -> Reply<ValueFigures> {
    let judge = app.deps.judge(authed.who.organization);
    let range = Range::new(app.now(), between.from, between.to)?;
    Ok(Json(
        value::project_value(app.store(), &judge, &authed.who, project, range).await?,
    ))
}

#[utoipa::path(
    get, path = "/value", security(("bearer" = [])),
    params(Between),
    responses((status = 200, description = "Delivered, Pending, At risk and Return of the organization, measured offline; organization owners and editors", body = ValueFigures), (status = 403, body = ApiError))
)]
async fn organization_value<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Query(between): Query<Between>,
) -> Reply<ValueFigures> {
    let judge = app.deps.judge(authed.who.organization);
    let range = Range::new(app.now(), between.from, between.to)?;
    Ok(Json(
        value::organization_value(app.store(), &judge, &authed.who, app.now(), range).await?,
    ))
}

#[utoipa::path(
    get, path = "/experiments/{id}/value", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment"), Between),
    responses(
        (status = 200, description = "Its value model, its wins and what they are worth, measured offline", body = ExperimentValue),
        (status = 404, body = ApiError),
    )
)]
async fn experiment_value<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ExperimentId>,
    Query(between): Query<Between>,
) -> Reply<ExperimentValue> {
    let judge = app.deps.judge(authed.who.organization);
    let range = Range::new(app.now(), between.from, between.to)?;
    Ok(Json(
        value::experiment_value(app.store(), &judge, &authed.who, id, range).await?,
    ))
}

#[utoipa::path(
    get, path = "/report", security(("bearer" = [])),
    params(ReportQuery),
    responses(
        (status = 200, description = "The month's report: spent, Delivered, Pending, At risk, Return, where the spend went, the agent scoreboard, top wins and experiments stopped with no win", body = Report),
        (status = 403, description = "The organization report is for owners and editors", body = ApiError),
        (status = 404, body = ApiError),
        (status = 422, body = ApiError),
    )
)]
async fn report<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Query(query): Query<ReportQuery>,
) -> Reply<Report> {
    let judge = app.deps.judge(authed.who.organization);
    let month = match query.month {
        Some(month) => month,
        None => value::month_of(app.now()).ok_or(antilog_core::organization::Refusal::Invalid(
            "the time is out of range",
        ))?,
    };
    Ok(Json(
        value::report(
            app.store(),
            &judge,
            &authed.who,
            app.now(),
            &month,
            query.project,
        )
        .await?,
    ))
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route(
            "/projects/{id}/value-models",
            get(value_models::<D>).post(set_value_model::<D>),
        )
        .route("/projects/{id}/value", get(project_value::<D>))
        .route("/value", get(organization_value::<D>))
        .route("/experiments/{id}/value", get(experiment_value::<D>))
        .route("/report", get(report::<D>))
}
