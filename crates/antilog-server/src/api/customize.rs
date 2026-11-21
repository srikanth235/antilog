//! Customize: connectors, skills and plugins. Handlers decode,
//! call `antilog-core` and encode; credentials go in sealed and never come back.

use std::sync::Arc;

use antilog_core::customize::{ConnectorClient, connectors, plugins, skills};
use antilog_core::deps::Deps;
use antilog_core::organization::Refusal;
use antilog_core::sealed::Sealer;
use antilog_core::store::CustomizeStore;
use antilog_types::{
    ApiError, Connector, ConnectorId, NewConnector, NewPlugin, NewSkill, Plugin, RemovePlugin,
    SessionId, SetToolMode, Skill, SkillVersion,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use utoipa::OpenApi;

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
    skills_list,
    skill_add,
    skill_version,
    skill_remove,
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
    responses((status = 204, description = "Removed with its credential; owners"), (status = 404, body = ApiError)))]
async fn connector_remove<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ConnectorId>,
) -> Result<StatusCode, Fail> {
    connectors::remove(app.store(), &authed.who, app.now(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/skills", security(("bearer" = [])),
    responses((status = 200, description = "The organization's skills, by name", body = Vec<Skill>)))]
async fn skills_list<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Result<Json<Vec<Skill>>, Fail> {
    Ok(Json(skills::list(app.store(), &authed.who).await?))
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
    responses((status = 204, description = "Every version removed; owners"), (status = 404, body = ApiError)))]
async fn skill_remove<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(name): Path<String>,
) -> Result<StatusCode, Fail> {
    skills::remove(app.store(), &authed.who, app.now(), &name).await?;
    Ok(StatusCode::NO_CONTENT)
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
    responses((status = 201, description = "Unpacked into its skills and connectors; owners", body = Plugin), (status = 409, body = ApiError)))]
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
    responses((status = 204, description = "Removed; the parts named in keep stay; owners"), (status = 404, body = ApiError)))]
async fn plugin_remove<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<String>,
    Json(body): Json<RemovePlugin>,
) -> Result<StatusCode, Fail> {
    plugins::remove(app.store(), &authed.who, app.now(), &id, body).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The skill versions a session may download: any version of its organization's skills. The
/// runner's route (`GET /runner/sessions/{id}/skills/{name}/{version}`) authenticates the runner
/// and calls this.
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
    store
        .skill_version(scope.organization, name, version)
        .await?
        .ok_or(Refusal::NotFound)
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
        .route("/skills", get(skills_list::<D>).post(skill_add::<D>))
        .route("/skills/{name}", axum::routing::delete(skill_remove::<D>))
        .route("/skills/{name}/versions/{version}", get(skill_version::<D>))
        .route("/plugins", get(plugins_list::<D>).post(plugin_add::<D, C>))
        .route("/plugins/{id}/remove", post(plugin_remove::<D>))
        .layer(Extension(backend))
}
