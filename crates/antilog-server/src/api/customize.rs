//! Customize: connectors, skills and plugins, and what each experiment chose. Handlers decode,
//! call `antilog-core` and encode; credentials go in sealed and never come back.

use std::sync::Arc;

use antilog_core::customize::{self, ConnectorClient, connectors, plugins, skills};
use antilog_core::deps::Deps;
use antilog_core::organization::Refusal;
use antilog_core::sealed::Sealer;
use antilog_core::store::CustomizeStore;
use antilog_types::{
    ApiError, Connector, ConnectorId, ExperimentId, NewConnector, NewPlugin, NewSkill, Plugin,
    ProjectId, RemovePlugin, SelectConnectors, SelectSkills, SessionId, SetOn, SetToolMode, Skill,
    SkillRef, SkillVersion,
};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use utoipa::{IntoParams, OpenApi};

use super::auth::{App, Authed, Fail};

#[derive(OpenApi)]
#[openapi(paths(
    connectors_list,
    connector_add,
    connector_get,
    connector_check,
    connector_credential,
    connector_tool,
    connector_remove,
    experiment_connectors,
    experiment_set_connectors,
    skills_list,
    skill_add,
    skill_version,
    skill_remove,
    project_skill,
    experiment_skills,
    experiment_set_skills,
    plugins_list,
    plugin_add,
    plugin_remove,
))]
pub struct Doc;

type Shared<D> = State<Arc<App<D>>>;

/// What the connector routes need besides the store: the remote end and the sealer.
pub struct Backend<C> {
    pub client: C,
    pub sealer: Option<Sealer>,
}

type Back<C> = Extension<Arc<Backend<C>>>;

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetCredential {
    pub credential: String,
}

#[utoipa::path(get, path = "/connectors", security(("bearer" = [])),
    responses((status = 200, description = "The organization's connectors; a credential is never shown", body = Vec<Connector>)))]
async fn connectors_list<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<Connector>>, Fail> {
    Ok(Json(connectors::list(app.store(), &authed.who).await?))
}

#[utoipa::path(post, path = "/connectors", security(("bearer" = [])), request_body = NewConnector,
    responses((status = 201, description = "Added and checked; owners", body = Connector), (status = 409, body = ApiError)))]
async fn connector_add<D: Deps + 'static, C: ConnectorClient + 'static>(
    State(app): Shared<D>,
    Extension(back): Back<C>,
    authed: Authed,
    Json(new): Json<NewConnector>,
) -> Result<(StatusCode, Json<Connector>), Fail> {
    let made = connectors::add(
        app.store(),
        &back.client,
        back.sealer.as_ref(),
        &authed.who,
        app.now(),
        new,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(made)))
}

#[utoipa::path(get, path = "/connectors/{id}", security(("bearer" = [])),
    params(("id" = ConnectorId, Path, description = "The connector")),
    responses((status = 200, body = Connector), (status = 404, body = ApiError)))]
async fn connector_get<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ConnectorId>,
) -> Result<Json<Connector>, Fail> {
    Ok(Json(connectors::get(app.store(), &authed.who, id).await?))
}

#[utoipa::path(post, path = "/connectors/{id}/check", security(("bearer" = [])),
    params(("id" = ConnectorId, Path, description = "The connector")),
    responses((status = 200, description = "Checked again: Connected, Reconnect or Not checked", body = Connector), (status = 404, body = ApiError)))]
async fn connector_check<D: Deps + 'static, C: ConnectorClient + 'static>(
    State(app): Shared<D>,
    Extension(back): Back<C>,
    authed: Authed,
    Path(id): Path<ConnectorId>,
) -> Result<Json<Connector>, Fail> {
    Ok(Json(
        connectors::recheck(
            app.store(),
            &back.client,
            back.sealer.as_ref(),
            &authed.who,
            app.now(),
            id,
        )
        .await?,
    ))
}

#[utoipa::path(put, path = "/connectors/{id}/credential", security(("bearer" = [])),
    params(("id" = ConnectorId, Path, description = "The connector")),
    request_body = SetCredential,
    responses((status = 200, description = "Replaced and checked; owners", body = Connector), (status = 404, body = ApiError)))]
async fn connector_credential<D: Deps + 'static, C: ConnectorClient + 'static>(
    State(app): Shared<D>,
    Extension(back): Back<C>,
    authed: Authed,
    Path(id): Path<ConnectorId>,
    Json(body): Json<SetCredential>,
) -> Result<Json<Connector>, Fail> {
    Ok(Json(
        connectors::update_credential(
            app.store(),
            &back.client,
            back.sealer.as_ref(),
            &authed.who,
            app.now(),
            id,
            &body.credential,
        )
        .await?,
    ))
}

#[utoipa::path(put, path = "/connectors/{id}/tools/{tool}", security(("bearer" = [])),
    params(("id" = ConnectorId, Path, description = "The connector"), ("tool" = String, Path, description = "The tool's own name")),
    request_body = SetToolMode,
    responses((status = 200, description = "On, off or ask first; owners", body = Connector), (status = 404, body = ApiError)))]
async fn connector_tool<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path((id, tool)): Path<(ConnectorId, String)>,
    Json(body): Json<SetToolMode>,
) -> Result<Json<Connector>, Fail> {
    Ok(Json(
        connectors::set_tool_mode(app.store(), &authed.who, app.now(), id, &tool, body).await?,
    ))
}

#[utoipa::path(delete, path = "/connectors/{id}", security(("bearer" = [])),
    params(("id" = ConnectorId, Path, description = "The connector")),
    responses((status = 204, description = "Removed with its credential; owners"), (status = 422, description = "An experiment has it switched on", body = ApiError)))]
async fn connector_remove<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ConnectorId>,
) -> Result<StatusCode, Fail> {
    connectors::remove(app.store(), &authed.who, app.now(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/experiments/{id}/connectors", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment")),
    responses((status = 200, description = "The connectors switched on", body = Vec<ConnectorId>)))]
async fn experiment_connectors<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ExperimentId>,
) -> Result<Json<Vec<ConnectorId>>, Fail> {
    Ok(Json(
        customize::chosen_connectors(app.store(), &authed.who, id).await?,
    ))
}

#[utoipa::path(put, path = "/experiments/{id}/connectors", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment")),
    request_body = SelectConnectors,
    responses((status = 200, description = "The connectors now switched on; editors and owners of the project", body = Vec<ConnectorId>)))]
async fn experiment_set_connectors<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ExperimentId>,
    Json(body): Json<SelectConnectors>,
) -> Result<Json<Vec<ConnectorId>>, Fail> {
    customize::select_connectors(app.store(), &authed.who, app.now(), id, body).await?;
    Ok(Json(
        customize::chosen_connectors(app.store(), &authed.who, id).await?,
    ))
}

#[derive(serde::Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct SkillsQuery {
    /// Show each skill On or Off for this project.
    project: Option<ProjectId>,
}

#[utoipa::path(get, path = "/skills", security(("bearer" = [])), params(SkillsQuery),
    responses((status = 200, description = "The organization's skills, by name", body = Vec<Skill>)))]
async fn skills_list<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Query(query): Query<SkillsQuery>,
) -> Result<Json<Vec<Skill>>, Fail> {
    Ok(Json(
        skills::list(app.store(), &authed.who, query.project).await?,
    ))
}

#[utoipa::path(post, path = "/skills", security(("bearer" = [])), request_body = NewSkill,
    responses((status = 201, description = "A new version of the skill named in SKILL.md; editors and owners", body = SkillVersion), (status = 422, body = ApiError)))]
async fn skill_add<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(new): Json<NewSkill>,
) -> Result<(StatusCode, Json<SkillVersion>), Fail> {
    let made = skills::add(app.store(), &authed.who, app.now(), new).await?;
    Ok((StatusCode::CREATED, Json(made)))
}

#[utoipa::path(get, path = "/skills/{name}/versions/{version}", security(("bearer" = [])),
    params(("name" = String, Path, description = "The skill"), ("version" = u32, Path, description = "Its version")),
    responses((status = 200, body = SkillVersion), (status = 404, body = ApiError)))]
async fn skill_version<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path((name, version)): Path<(String, u32)>,
) -> Result<Json<SkillVersion>, Fail> {
    Ok(Json(
        skills::version(app.store(), &authed.who, &name, version).await?,
    ))
}

#[utoipa::path(delete, path = "/skills/{name}", security(("bearer" = [])),
    params(("name" = String, Path, description = "The skill")),
    responses((status = 204, description = "Every version removed; owners"), (status = 422, description = "An experiment selected it", body = ApiError)))]
async fn skill_remove<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(name): Path<String>,
) -> Result<StatusCode, Fail> {
    skills::remove(app.store(), &authed.who, app.now(), &name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(put, path = "/projects/{id}/skills/{name}", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project"), ("name" = String, Path, description = "The skill")),
    request_body = SetOn,
    responses((status = 204, description = "Turned On or Off for the project; project owners")))]
async fn project_skill<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path((project, name)): Path<(ProjectId, String)>,
    Json(body): Json<SetOn>,
) -> Result<StatusCode, Fail> {
    skills::set_on(app.store(), &authed.who, app.now(), project, &name, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/experiments/{id}/skills", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment")),
    responses((status = 200, description = "The skill versions it selected", body = Vec<SkillRef>)))]
async fn experiment_skills<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ExperimentId>,
) -> Result<Json<Vec<SkillRef>>, Fail> {
    Ok(Json(
        customize::chosen_skills(app.store(), &authed.who, id).await?,
    ))
}

#[utoipa::path(put, path = "/experiments/{id}/skills", security(("bearer" = [])),
    params(("id" = ExperimentId, Path, description = "The experiment")),
    request_body = SelectSkills,
    responses((status = 200, description = "Each skill pinned to its latest version now; editors and owners of the project", body = Vec<SkillRef>)))]
async fn experiment_set_skills<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ExperimentId>,
    Json(body): Json<SelectSkills>,
) -> Result<Json<Vec<SkillRef>>, Fail> {
    Ok(Json(
        customize::select_skills(app.store(), &authed.who, app.now(), id, body).await?,
    ))
}

#[utoipa::path(get, path = "/plugins", security(("bearer" = [])),
    responses((status = 200, description = "The plugins added, oldest first", body = Vec<Plugin>)))]
async fn plugins_list<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<Plugin>>, Fail> {
    Ok(Json(plugins::list(app.store(), &authed.who).await?))
}

#[utoipa::path(post, path = "/plugins", security(("bearer" = [])), request_body = NewPlugin,
    responses((status = 201, description = "Unpacked into its skills, connectors and evaluation templates; owners", body = Plugin), (status = 409, body = ApiError)))]
async fn plugin_add<D: Deps + 'static, C: ConnectorClient + 'static>(
    State(app): Shared<D>,
    Extension(back): Back<C>,
    authed: Authed,
    Json(new): Json<NewPlugin>,
) -> Result<(StatusCode, Json<Plugin>), Fail> {
    let made = plugins::add(
        app.store(),
        back.client.allow_private(),
        &authed.who,
        app.now(),
        new,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(made)))
}

#[utoipa::path(post, path = "/plugins/{id}/remove", security(("bearer" = [])),
    params(("id" = String, Path, description = "The plugin")),
    request_body = RemovePlugin,
    responses((status = 204, description = "Removed; the parts named in keep stay; owners"), (status = 422, description = "A part is in use", body = ApiError)))]
async fn plugin_remove<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<String>,
    Json(body): Json<RemovePlugin>,
) -> Result<StatusCode, Fail> {
    plugins::remove(app.store(), &authed.who, app.now(), &id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The skill versions a session may download: those its experiment selected. The runner's route
/// (`GET /runner/sessions/{id}/skills/{name}/{version}`) authenticates the runner and calls this.
pub async fn session_skill<D: Deps>(
    deps: &D,
    session: SessionId,
    name: &str,
    version: u32,
) -> Result<SkillVersion, Refusal> {
    let store = deps.store();
    let scope = store
        .session_scope(session)
        .await?
        .ok_or(Refusal::NotFound)?;
    let selected = store
        .experiment_skills(scope.organization, scope.experiment)
        .await?;
    if !selected
        .iter()
        .any(|skill| skill.name == name && skill.version == version)
    {
        return Err(Refusal::NotFound);
    }
    store
        .skill_version(scope.organization, name, version)
        .await?
        .ok_or(Refusal::NotFound)
}

/// What the runner downloads for a session: the experiment's selected skill versions.
pub async fn session_skill_refs<D: Deps>(
    deps: &D,
    session: SessionId,
) -> Result<Vec<SkillRef>, Refusal> {
    let store = deps.store();
    let scope = store
        .session_scope(session)
        .await?
        .ok_or(Refusal::NotFound)?;
    Ok(store
        .experiment_skills(scope.organization, scope.experiment)
        .await?)
}

pub fn routes<D: Deps + 'static, C: ConnectorClient + 'static>(
    backend: Arc<Backend<C>>,
) -> Router<Arc<App<D>>> {
    Router::new()
        .route(
            "/connectors",
            get(connectors_list::<D>).post(connector_add::<D, C>),
        )
        .route(
            "/connectors/{id}",
            get(connector_get::<D>).delete(connector_remove::<D>),
        )
        .route("/connectors/{id}/check", post(connector_check::<D, C>))
        .route(
            "/connectors/{id}/credential",
            put(connector_credential::<D, C>),
        )
        .route("/connectors/{id}/tools/{tool}", put(connector_tool::<D>))
        .route(
            "/experiments/{id}/connectors",
            get(experiment_connectors::<D>).put(experiment_set_connectors::<D>),
        )
        .route("/skills", get(skills_list::<D>).post(skill_add::<D>))
        .route("/skills/{name}", axum::routing::delete(skill_remove::<D>))
        .route("/skills/{name}/versions/{version}", get(skill_version::<D>))
        .route("/projects/{id}/skills/{name}", put(project_skill::<D>))
        .route(
            "/experiments/{id}/skills",
            get(experiment_skills::<D>).put(experiment_set_skills::<D>),
        )
        .route("/plugins", get(plugins_list::<D>).post(plugin_add::<D, C>))
        .route("/plugins/{id}/remove", post(plugin_remove::<D>))
        .layer(Extension(backend))
}
