//! The organization floor: members, tokens, policy, approvals, budgets, audit.
//! Handlers decode, call the action layer in `antilog-core` and encode; they test no role.

use std::sync::Arc;

use antilog_core::deps::Deps;
use antilog_core::organization::{self as floor, Refusal};
use antilog_core::store::OrganizationStore;
use antilog_core::{approval, budget};
use antilog_types::{
    AccessToken, ApiError, Approval, ApprovalId, AuditCursor, AuditId, AuditPage, BudgetSummary,
    Decide, LimitSpend, Limits, Member, MemberId, NewMember, NewProject, NewToken, Organization,
    Policy, Project, ProjectId, ProjectRole, ProjectSpend, SessionId, SetProjectRole, SetRole,
    TokenId, TokenIssued,
};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use utoipa::{IntoParams, OpenApi};

use super::auth::{App, Authed, Fail};

type Shared<D> = State<Arc<App<D>>>;
type Reply<T> = Result<Json<T>, Fail>;

#[derive(OpenApi)]
#[openapi(paths(
    organization,
    set_organization_limits,
    budgets,
    members,
    add_member,
    set_member_role,
    projects,
    create_project,
    project_roles,
    set_project_role,
    set_project_limits,
    tokens,
    create_token,
    revoke_token,
    policies,
    set_policy,
    approvals,
    pending,
    decide,
    audit
))]
pub struct ApiDoc;

#[utoipa::path(
    get, path = "/organization", security(("bearer" = [])),
    responses((status = 200, description = "The caller's organization", body = Organization))
)]
async fn organization<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
) -> Reply<Organization> {
    Ok(Json(floor::organization(app.store(), &authed.who).await?))
}

#[utoipa::path(
    put, path = "/organization/limits", security(("bearer" = [])),
    request_body = Limits,
    responses(
        (status = 204, description = "Set; organization owners only"),
        (status = 409, description = "The cap would pass a limit inside it", body = ApiError),
    )
)]
async fn set_organization_limits<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(limits): Json<Limits>,
) -> Result<StatusCode, Fail> {
    budget::set_organization_limits(
        app.store(),
        &authed.who,
        app.now(),
        limits.monthly_cap_cents,
        limits.default_budget_cents,
        limits.max_running_sessions,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/budgets", security(("bearer" = [])),
    responses((status = 200, description = "This month's spend against the caps", body = BudgetSummary))
)]
async fn budgets<D: Deps + 'static>(State(app): Shared<D>, authed: Authed) -> Reply<BudgetSummary> {
    let summary = budget::summary(app.store(), &authed.who, app.now()).await?;
    Ok(Json(BudgetSummary {
        organization: LimitSpend {
            cap_cents: summary.organization.cap_cents,
            spent_cents: summary.organization.spent_cents,
        },
        projects: summary
            .projects
            .into_iter()
            .map(|(project_id, scope)| ProjectSpend {
                project_id,
                cap_cents: scope.cap_cents,
                spent_cents: scope.spent_cents,
            })
            .collect(),
    }))
}

#[utoipa::path(
    get, path = "/members", security(("bearer" = [])),
    responses((status = 200, description = "The organization's members", body = Vec<Member>))
)]
async fn members<D: Deps + 'static>(State(app): Shared<D>, authed: Authed) -> Reply<Vec<Member>> {
    floor::member_of(app.store(), &authed.who).await?;
    Ok(Json(app.store().members(authed.who.organization).await?))
}

#[utoipa::path(
    post, path = "/members", security(("bearer" = [])),
    request_body = NewMember,
    responses(
        (status = 201, description = "Added; organization owners only", body = Member),
        (status = 409, description = "That identity is already a member", body = ApiError),
    )
)]
async fn add_member<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(new): Json<NewMember>,
) -> Result<(StatusCode, Json<Member>), Fail> {
    let member = floor::add_member(
        app.store(),
        &authed.who,
        app.now(),
        &new.name,
        new.identity,
        new.role,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(member)))
}

#[utoipa::path(
    put, path = "/members/{id}/role", security(("bearer" = [])),
    params(("id" = MemberId, Path, description = "The member")),
    request_body = SetRole,
    responses(
        (status = 204, description = "Set; organization owners only"),
        (status = 409, description = "The organization or a project would be left without an owner", body = ApiError),
    )
)]
async fn set_member_role<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<MemberId>,
    Json(set): Json<SetRole>,
) -> Result<StatusCode, Fail> {
    floor::set_member_role(app.store(), &authed.who, app.now(), id, set.role).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/projects", security(("bearer" = [])),
    responses((status = 200, description = "The organization's projects", body = Vec<Project>))
)]
async fn projects<D: Deps + 'static>(State(app): Shared<D>, authed: Authed) -> Reply<Vec<Project>> {
    floor::member_of(app.store(), &authed.who).await?;
    Ok(Json(app.store().projects(authed.who.organization).await?))
}

#[utoipa::path(
    post, path = "/projects", security(("bearer" = [])),
    request_body = NewProject,
    responses(
        (status = 201, description = "Created; the caller is its first owner", body = Project),
        (status = 409, description = "The name is taken", body = ApiError),
    )
)]
async fn create_project<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(new): Json<NewProject>,
) -> Result<(StatusCode, Json<Project>), Fail> {
    let project = floor::create_project(
        app.store(),
        &authed.who,
        app.now(),
        &new.name,
        &new.description,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(project)))
}

#[utoipa::path(
    get, path = "/projects/{id}/roles", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project")),
    responses((status = 200, description = "Roles set for this project; the rest have their organization role", body = Vec<ProjectRole>))
)]
async fn project_roles<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ProjectId>,
) -> Reply<Vec<ProjectRole>> {
    Ok(Json(
        floor::project_roles(app.store(), &authed.who, id).await?,
    ))
}

#[utoipa::path(
    put, path = "/projects/{id}/roles/{member}", security(("bearer" = [])),
    params(
        ("id" = ProjectId, Path, description = "The project"),
        ("member" = MemberId, Path, description = "The member"),
    ),
    request_body = SetProjectRole,
    responses(
        (status = 204, description = "Set; project owners only"),
        (status = 409, description = "The project would be left without an owner", body = ApiError),
    )
)]
async fn set_project_role<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path((id, member)): Path<(ProjectId, MemberId)>,
    Json(set): Json<SetProjectRole>,
) -> Result<StatusCode, Fail> {
    floor::set_project_role(app.store(), &authed.who, app.now(), id, member, set.role).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put, path = "/projects/{id}/limits", security(("bearer" = [])),
    params(("id" = ProjectId, Path, description = "The project")),
    request_body = Limits,
    responses(
        (status = 204, description = "Set; project owners only"),
        (status = 409, description = "The cap would pass the organization's, or a budget inside it", body = ApiError),
    )
)]
async fn set_project_limits<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ProjectId>,
    Json(limits): Json<Limits>,
) -> Result<StatusCode, Fail> {
    budget::set_project_limits(
        app.store(),
        &authed.who,
        app.now(),
        id,
        limits.monthly_cap_cents,
        limits.default_budget_cents,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
struct TokenQuery {
    /// Whose tokens (owners only); omitted: the caller's.
    member_id: Option<MemberId>,
}

#[utoipa::path(
    get, path = "/tokens", security(("bearer" = [])),
    params(TokenQuery),
    responses((status = 200, description = "Tokens without their secrets", body = Vec<AccessToken>))
)]
async fn tokens<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Query(query): Query<TokenQuery>,
) -> Reply<Vec<AccessToken>> {
    Ok(Json(
        floor::tokens(app.store(), &authed.who, query.member_id).await?,
    ))
}

#[utoipa::path(
    post, path = "/tokens", security(("bearer" = [])),
    request_body = NewToken,
    responses((status = 201, description = "Created; the secret is shown only now", body = TokenIssued))
)]
async fn create_token<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(new): Json<NewToken>,
) -> Result<(StatusCode, Json<TokenIssued>), Fail> {
    let issued = floor::create_token(
        app.store(),
        &authed.who,
        app.now(),
        new.member_id,
        &new.name,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(TokenIssued {
            token: issued.record.token.clone(),
            secret: issued.secret.clone(),
        }),
    ))
}

#[utoipa::path(
    delete, path = "/tokens/{id}", security(("bearer" = [])),
    params(("id" = TokenId, Path, description = "The token")),
    responses((status = 204, description = "Revoked; your own, or any if an organization owner"))
)]
async fn revoke_token<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<TokenId>,
) -> Result<StatusCode, Fail> {
    floor::revoke_token(app.store(), &authed.who, app.now(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/policies", security(("bearer" = [])),
    responses((status = 200, description = "One policy per card kind; what was never set always asks", body = Vec<Policy>))
)]
async fn policies<D: Deps + 'static>(State(app): Shared<D>, authed: Authed) -> Reply<Vec<Policy>> {
    Ok(Json(approval::policies(app.store(), &authed.who).await?))
}

#[utoipa::path(
    put, path = "/policies", security(("bearer" = [])),
    request_body = Policy,
    responses(
        (status = 204, description = "Set; organization owners only"),
        (status = 409, description = "Add credential only ever asks", body = ApiError),
    )
)]
async fn set_policy<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Json(policy): Json<Policy>,
) -> Result<StatusCode, Fail> {
    approval::set_policy(app.store(), &authed.who, app.now(), policy).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get, path = "/sessions/{id}/approvals", security(("bearer" = [])),
    params(("id" = SessionId, Path, description = "The session")),
    responses((status = 200, description = "Its cards, oldest first", body = Vec<Approval>))
)]
async fn approvals<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<SessionId>,
) -> Reply<Vec<Approval>> {
    Ok(Json(
        approval::approvals(app.store(), &authed.who, id).await?,
    ))
}

#[utoipa::path(
    get, path = "/approvals/pending", security(("bearer" = [])),
    responses((status = 200, description = "Every pending card in the organization if the caller may decide (an editor or an owner), oldest first", body = Vec<Approval>))
)]
async fn pending<D: Deps + 'static>(State(app): Shared<D>, authed: Authed) -> Reply<Vec<Approval>> {
    Ok(Json(approval::pending(app.store(), &authed.who).await?))
}

#[utoipa::path(
    post, path = "/approvals/{id}/decision", security(("bearer" = [])),
    params(("id" = ApprovalId, Path, description = "The card")),
    request_body = Decide,
    responses(
        (status = 200, description = "Decided", body = Approval),
        (status = 403, description = "Editors and owners decide", body = ApiError),
        (status = 409, description = "It was already decided", body = ApiError),
    )
)]
async fn decide<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Path(id): Path<ApprovalId>,
    Json(decide): Json<Decide>,
) -> Reply<Approval> {
    let decided =
        approval::decide(app.store(), &authed.who, app.now(), id, decide.decision).await?;
    // The decision stands even when applying it fails now. Settling also tells the agent.
    if let Err(error) =
        antilog_core::session::settle(&*app.deps, decided.organization_id, decided.session_id).await
    {
        tracing::warn!(%error, "applying a decided card failed");
    }
    Ok(Json(decided))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
struct AuditQuery {
    /// The cursor of the previous page: its `at` and `id`, together.
    before_at: Option<i64>,
    before_id: Option<AuditId>,
    /// 1 to 200; default 50.
    limit: Option<u32>,
}

#[utoipa::path(
    get, path = "/audit", security(("bearer" = [])),
    params(AuditQuery),
    responses((status = 200, description = "Newest first; organization owners only", body = AuditPage))
)]
async fn audit<D: Deps + 'static>(
    State(app): Shared<D>,
    authed: Authed,
    Query(query): Query<AuditQuery>,
) -> Reply<AuditPage> {
    let before = match (query.before_at, query.before_id) {
        (Some(at), Some(id)) => Some((at, id)),
        (None, None) => None,
        _ => return Err(Refusal::Invalid("give beforeAt and beforeId together").into()),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let entries = floor::audit_log(app.store(), &authed.who, app.now(), before, limit).await?;
    let next = (entries.len() == usize::try_from(limit).unwrap_or(usize::MAX))
        .then(|| {
            entries.last().map(|last| AuditCursor {
                at: last.at,
                id: last.id,
            })
        })
        .flatten();
    Ok(Json(AuditPage { entries, next }))
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route("/organization", get(organization::<D>))
        .route("/organization/limits", put(set_organization_limits::<D>))
        .route("/budgets", get(budgets::<D>))
        .route("/members", get(members::<D>).post(add_member::<D>))
        .route("/members/{id}/role", put(set_member_role::<D>))
        .route("/projects", get(projects::<D>).post(create_project::<D>))
        .route("/projects/{id}/roles", get(project_roles::<D>))
        .route("/projects/{id}/roles/{member}", put(set_project_role::<D>))
        .route("/projects/{id}/limits", put(set_project_limits::<D>))
        .route("/sessions/{id}/approvals", get(approvals::<D>))
        .route("/tokens", get(tokens::<D>).post(create_token::<D>))
        .route("/tokens/{id}", delete(revoke_token::<D>))
        .route("/policies", get(policies::<D>).put(set_policy::<D>))
        .route("/approvals/pending", get(pending::<D>))
        .route("/approvals/{id}/decision", post(decide::<D>))
        .route("/audit", get(audit::<D>))
}
