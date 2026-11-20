//! Sign-in and sign-out, and who is calling: a session cookie or a bearer token.

use std::sync::Arc;

use antilog_core::auth::{
    self, Challenge, Credential, FakeSignIn, Proven, SignInError, SignInProvider,
};
use antilog_core::clock::Clock;
use antilog_core::deps::Deps;
use antilog_core::organization::{Principal, Refusal};
use antilog_core::store::Invariant;
use antilog_core::token::SESSION_TTL_MS;
use antilog_types::{
    ApiError, Me, OrganizationCreated, SessionIssued, SignInCallback, SignInProviders, SignInStart,
};
use axum::extract::{FromRequestParts, Path, State};
use axum::http::header::{AUTHORIZATION, COOKIE, SET_COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use subtle::ConstantTimeEq;
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

use crate::signin::{GitHub, GitHubConfig, Oidc, OidcConfig};

const SESSION_COOKIE: &str = "antilog_session";
const CHALLENGE_COOKIE: &str = "antilog_challenge";

pub enum Provider {
    Fake(FakeSignIn),
    GitHub(GitHub),
    Oidc(Oidc),
}

impl Provider {
    fn name(&self) -> &str {
        match self {
            Self::Fake(provider) => provider.name(),
            Self::GitHub(provider) => provider.name(),
            Self::Oidc(provider) => provider.name(),
        }
    }

    async fn authorize_url(&self, challenge: &Challenge) -> Result<String, SignInError> {
        match self {
            Self::Fake(provider) => provider.authorize_url(challenge).await,
            Self::GitHub(provider) => provider.authorize_url(challenge).await,
            Self::Oidc(provider) => provider.authorize_url(challenge).await,
        }
    }

    async fn complete(&self, code: &str, challenge: &Challenge) -> Result<Proven, SignInError> {
        match self {
            Self::Fake(provider) => provider.complete(code, challenge).await,
            Self::GitHub(provider) => provider.complete(code, challenge).await,
            Self::Oidc(provider) => provider.complete(code, challenge).await,
        }
    }
}

pub struct Settings {
    pub providers: Vec<Provider>,
    /// Whether a person signed in at a provider may create a new organization.
    pub open_organizations: bool,
    /// Cookies are `Secure` unless this server is reached over plain http (local development).
    pub secure_cookies: bool,
    /// When set, `/webhooks/code-host` is mounted and checks `x-hub-signature-256` against it.
    pub code_host_webhook_secret: Option<String>,
    /// `ANTILOG_E2E_SEED=1` mounts `/internal/e2e/win`, which makes a win without an agent. For the
    /// server on its fakes in end-to-end tests only.
    pub e2e_seed: bool,
}

fn setting(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// A secret is read from the environment or, as a reference, from the file the `_FILE` variable names.
fn secret(name: &str) -> Option<String> {
    setting(name).or_else(|| {
        let path = setting(&format!("{name}_FILE"))?;
        std::fs::read_to_string(path)
            .ok()
            .map(|text| text.trim().to_owned())
    })
}

impl Settings {
    /// Nothing is offered unless it is configured; `ANTILOG_FAKE_SIGN_IN=1` is for tests and agents only.
    pub fn from_env() -> Self {
        let mut providers = Vec::new();
        if let (Some(client_id), Some(client_secret), Some(redirect)) = (
            setting("ANTILOG_GITHUB_CLIENT_ID"),
            secret("ANTILOG_GITHUB_CLIENT_SECRET"),
            setting("ANTILOG_GITHUB_REDIRECT"),
        ) {
            providers.push(Provider::GitHub(GitHub(GitHubConfig {
                client_id,
                client_secret,
                redirect,
                auth_url: setting("ANTILOG_GITHUB_AUTH_URL")
                    .unwrap_or_else(|| "https://github.com/login/oauth/authorize".to_owned()),
                token_url: setting("ANTILOG_GITHUB_TOKEN_URL")
                    .unwrap_or_else(|| "https://github.com/login/oauth/access_token".to_owned()),
                user_url: setting("ANTILOG_GITHUB_USER_URL")
                    .unwrap_or_else(|| "https://api.github.com/user".to_owned()),
            })));
        }
        for (name, issuer) in [
            ("GOOGLE", Some("https://accounts.google.com".to_owned())),
            ("OIDC", None),
        ] {
            let issuer = setting(&format!("ANTILOG_{name}_ISSUER")).or(issuer);
            if let (Some(issuer), Some(client_id), Some(client_secret), Some(redirect)) = (
                issuer,
                setting(&format!("ANTILOG_{name}_CLIENT_ID")),
                secret(&format!("ANTILOG_{name}_CLIENT_SECRET")),
                setting(&format!("ANTILOG_{name}_REDIRECT")),
            ) {
                providers.push(Provider::Oidc(Oidc(OidcConfig {
                    name: name.to_lowercase(),
                    issuer,
                    client_id,
                    client_secret,
                    redirect,
                })));
            }
        }
        if setting("ANTILOG_FAKE_SIGN_IN").as_deref() == Some("1") {
            providers.push(Provider::Fake(FakeSignIn));
        }
        Self {
            providers,
            open_organizations: setting("ANTILOG_OPEN_ORGANIZATIONS").as_deref() == Some("1"),
            secure_cookies: setting("ANTILOG_INSECURE_COOKIES").as_deref() != Some("1"),
            code_host_webhook_secret: secret("ANTILOG_CODE_HOST_WEBHOOK_SECRET"),
            e2e_seed: setting("ANTILOG_E2E_SEED").as_deref() == Some("1"),
        }
    }
}

pub struct App<D> {
    pub deps: Arc<D>,
    pub settings: Settings,
}

impl<D: Deps> App<D> {
    pub fn store(&self) -> &D::Store {
        self.deps.store()
    }

    pub fn now(&self) -> i64 {
        self.deps.clock().now().as_millisecond()
    }

    fn provider(&self, name: &str) -> Result<&Provider, Fail> {
        self.settings
            .providers
            .iter()
            .find(|provider| provider.name() == name)
            .ok_or(Fail::Refused(Refusal::NotFound))
    }
}

/// Why a request was turned away; the body is always the same `ApiError` shape.
pub enum Fail {
    Unauthenticated,
    Refused(Refusal),
    SignIn(SignInError),
    Session(antilog_core::session::SessionError),
}

impl From<Refusal> for Fail {
    fn from(refusal: Refusal) -> Self {
        Self::Refused(refusal)
    }
}

impl From<antilog_core::store::StoreError> for Fail {
    fn from(error: antilog_core::store::StoreError) -> Self {
        Self::Refused(error.into())
    }
}

fn body(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            code: code.to_owned(),
            message: message.to_owned(),
            fields: Vec::new(),
        }),
    )
        .into_response()
}

impl IntoResponse for Fail {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthenticated => body(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Sign in or present an access token.",
            ),
            Self::Session(error) => super::session::refuse(error),
            Self::SignIn(SignInError::Refused) => body(
                StatusCode::UNAUTHORIZED,
                "sign_in_refused",
                "The provider did not accept the sign-in.",
            ),
            Self::SignIn(SignInError::Unavailable(error)) => {
                tracing::error!(%error, "sign-in provider unavailable");
                body(
                    StatusCode::BAD_GATEWAY,
                    "provider_unavailable",
                    "The sign-in provider does not answer; retry shortly.",
                )
            }
            Self::Refused(refusal) => match refusal {
                Refusal::Denied => body(
                    StatusCode::FORBIDDEN,
                    "forbidden",
                    "Your role does not allow this.",
                ),
                Refusal::NotFound => body(StatusCode::NOT_FOUND, "not_found", "Not found."),
                Refusal::Conflict => body(StatusCode::CONFLICT, "conflict", "It already exists."),
                Refusal::Invalid(message) => {
                    body(StatusCode::UNPROCESSABLE_ENTITY, "invalid", message)
                }
                Refusal::Invariant(invariant) => {
                    let code = match invariant {
                        Invariant::LastOwner => "last_owner",
                        Invariant::OverCap => "over_cap",
                        Invariant::IllegalTransition => "illegal_transition",
                        Invariant::AlwaysAsk => "always_ask",
                        Invariant::AppendOnly => "append_only",
                    };
                    body(StatusCode::CONFLICT, code, &invariant.to_string())
                }
                Refusal::Spent(spent) => {
                    body(StatusCode::CONFLICT, "budget_spent", &spent.to_string())
                }
                Refusal::Unavailable(error) => {
                    tracing::error!(%error, "store unavailable");
                    body(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "store_unavailable",
                        "The store does not answer; retry shortly.",
                    )
                }
            },
        }
    }
}

/// The secret a caller presented: a bearer token, else the session cookie.
pub(super) fn presented(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .or_else(|| cookie(headers, SESSION_COOKIE))
}

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then_some(value))
}

/// The caller, proven by `Authorization: Bearer <secret>` or the session cookie.
pub struct Authed {
    pub who: Principal,
    pub credential: Credential,
}

impl<D: Deps + 'static> FromRequestParts<Arc<App<D>>> for Authed {
    type Rejection = Fail;

    async fn from_request_parts(
        parts: &mut Parts,
        app: &Arc<App<D>>,
    ) -> Result<Self, Self::Rejection> {
        let presented = presented(&parts.headers).ok_or(Fail::Unauthenticated)?;
        let (who, credential) = auth::authenticate(app.store(), presented, app.now())
            .await?
            .ok_or(Fail::Unauthenticated)?;
        Ok(Self { who, credential })
    }
}

fn set_cookie(name: &str, value: &str, max_age: i64, secure: bool) -> HeaderValue {
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{name}={value}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age}{secure}"
    ))
    .unwrap_or_else(|_| HeaderValue::from_static(""))
}

#[derive(OpenApi)]
#[openapi(
    paths(providers, start, callback, create_organization, sign_out, me),
    modifiers(&Bearer)
)]
pub struct ApiDoc;

struct Bearer;

impl utoipa::Modify for Bearer {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
        );
    }
}

#[utoipa::path(
    get, path = "/auth/providers",
    responses((status = 200, description = "The sign-in providers this server offers", body = SignInProviders))
)]
async fn providers<D: Deps + 'static>(State(app): State<Arc<App<D>>>) -> Json<SignInProviders> {
    Json(SignInProviders {
        providers: app
            .settings
            .providers
            .iter()
            .map(|provider| provider.name().to_owned())
            .collect(),
    })
}

#[utoipa::path(
    get, path = "/auth/{provider}/start", operation_id = "start_sign_in",
    params(("provider" = String, Path, description = "A provider from /auth/providers")),
    responses(
        (status = 200, description = "Where to send the person; the state is also kept in a cookie", body = SignInStart),
        (status = 404, description = "No such provider", body = ApiError),
    )
)]
async fn start<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    Path(provider): Path<String>,
) -> Result<Response, Fail> {
    let provider = app.provider(&provider)?;
    let challenge = Challenge::new().map_err(Refusal::from)?;
    let authorize_url = provider
        .authorize_url(&challenge)
        .await
        .map_err(Fail::SignIn)?;
    let cookie = set_cookie(
        CHALLENGE_COOKIE,
        &challenge.cookie(),
        600,
        app.settings.secure_cookies,
    );
    Ok((
        AppendHeaders([(SET_COOKIE, cookie)]),
        Json(SignInStart {
            authorize_url,
            state: challenge.state,
        }),
    )
        .into_response())
}

async fn prove<D: Deps + 'static>(
    app: &App<D>,
    provider: &str,
    headers: &HeaderMap,
    code: &str,
    state: &str,
) -> Result<Proven, Fail> {
    let provider = app.provider(provider)?;
    let challenge = cookie(headers, CHALLENGE_COOKIE)
        .and_then(Challenge::from_cookie)
        .filter(|challenge| bool::from(challenge.state.as_bytes().ct_eq(state.as_bytes())))
        .ok_or(Fail::SignIn(SignInError::Refused))?;
    provider
        .complete(code, &challenge)
        .await
        .map_err(Fail::SignIn)
}

fn issued(
    app: &App<impl Deps>,
    signed: &auth::SignedIn,
) -> (SessionIssued, [(axum::http::HeaderName, HeaderValue); 2]) {
    let session = &signed.issued.record.session;
    let body = SessionIssued {
        member: signed.member.clone(),
        secret: signed.issued.secret.clone(),
        expires_at: session.expires_at,
    };
    let secure = app.settings.secure_cookies;
    (
        body,
        [
            (
                SET_COOKIE,
                set_cookie(
                    SESSION_COOKIE,
                    &signed.issued.secret,
                    SESSION_TTL_MS / 1000,
                    secure,
                ),
            ),
            (SET_COOKIE, set_cookie(CHALLENGE_COOKIE, "", 0, secure)),
        ],
    )
}

#[utoipa::path(
    post, path = "/auth/{provider}/callback",
    params(("provider" = String, Path, description = "A provider from /auth/providers")),
    request_body = SignInCallback,
    responses(
        (status = 200, description = "Signed in; the session is also set as a cookie", body = SessionIssued),
        (status = 401, description = "The provider did not accept the sign-in", body = ApiError),
        (status = 403, description = "Not a member of that organization, or of any when none is named", body = ApiError),
    )
)]
async fn callback<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Json(request): Json<SignInCallback>,
) -> Result<Response, Fail> {
    let proven = prove(&app, &provider, &headers, &request.code, &request.state).await?;
    let signed = auth::sign_in(app.store(), app.now(), request.organization, &proven).await?;
    let (body, cookies) = issued(&app, &signed);
    Ok((AppendHeaders(cookies), Json(body)).into_response())
}

#[utoipa::path(
    post, path = "/auth/{provider}/organization",
    params(("provider" = String, Path, description = "A provider from /auth/providers")),
    request_body = antilog_types::NewOrganization,
    responses(
        (status = 201, description = "The organization, and its first owner signed in", body = OrganizationCreated),
        (status = 403, description = "This server does not let people create organizations", body = ApiError),
        (status = 409, description = "The name is taken", body = ApiError),
    )
)]
async fn create_organization<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Json(request): Json<antilog_types::NewOrganization>,
) -> Result<Response, Fail> {
    if !app.settings.open_organizations {
        return Err(Refusal::Denied.into());
    }
    let proven = prove(&app, &provider, &headers, &request.code, &request.state).await?;
    let (organization, signed) =
        auth::create_organization(app.store(), app.now(), &request.name, &proven).await?;
    let (session, cookies) = issued(&app, &signed);
    Ok((
        StatusCode::CREATED,
        AppendHeaders(cookies),
        Json(OrganizationCreated {
            organization,
            session,
        }),
    )
        .into_response())
}

#[utoipa::path(
    post, path = "/auth/sign-out", security(("bearer" = [])),
    responses(
        (status = 204, description = "The session ended"),
        (status = 401, description = "Not signed in", body = ApiError),
    )
)]
async fn sign_out<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    authed: Authed,
) -> Result<Response, Fail> {
    if let Credential::SignIn(session) = &authed.credential {
        auth::sign_out(app.store(), session, app.now()).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        AppendHeaders([(
            SET_COOKIE,
            set_cookie(SESSION_COOKIE, "", 0, app.settings.secure_cookies),
        )]),
    )
        .into_response())
}

#[utoipa::path(
    get, path = "/me", security(("bearer" = [])),
    responses(
        (status = 200, description = "The signed-in member, their organization and role, and the organizations their identity belongs to", body = Me),
        (status = 401, description = "Not signed in", body = ApiError),
    )
)]
async fn me<D: Deps + 'static>(
    State(app): State<Arc<App<D>>>,
    authed: Authed,
) -> Result<Json<Me>, Fail> {
    Ok(Json(
        antilog_core::organization::me(app.store(), &authed.who).await?,
    ))
}

pub fn routes<D: Deps + 'static>() -> Router<Arc<App<D>>> {
    Router::new()
        .route("/auth/providers", get(providers::<D>))
        .route("/auth/{provider}/start", get(start::<D>))
        .route("/auth/{provider}/callback", post(callback::<D>))
        .route(
            "/auth/{provider}/organization",
            post(create_organization::<D>),
        )
        .route("/auth/sign-out", post(sign_out::<D>))
        .route("/me", get(me::<D>))
}
