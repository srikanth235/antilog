//! The one MCP endpoint of a session: `/mcp/sessions/{id}`, streamable HTTP, served without
//! protocol sessions (each request stands alone). A bearer token for `Purpose::Mcp` names the
//! session; the tools are the Antilog setup tools and those of the organization's connectors.

use std::sync::Arc;

use antilog_core::credentials::{Purpose, SessionCredentials};
use antilog_core::customize::ConnectorClient;
use antilog_core::customize::reach::{Outcome, Reach};
use antilog_core::deps::Deps;
use antilog_core::store::{CustomizeStore, SessionScope};
use antilog_core::tools::Caller;
use antilog_types::SessionId;
use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Extension, Router};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;

use super::auth::App;
use super::customize::Backend;

/// What the endpoint verifies tokens with, and what it calls connectors with.
pub struct Mcp<V, C> {
    pub credentials: V,
    pub backend: Arc<Backend<C>>,
}

struct SessionMcp<D, C> {
    app: Arc<App<D>>,
    backend: Arc<Backend<C>>,
    scope: SessionScope,
    session: SessionId,
}

impl<D, C> Clone for SessionMcp<D, C> {
    fn clone(&self) -> Self {
        Self {
            app: self.app.clone(),
            backend: self.backend.clone(),
            scope: self.scope,
            session: self.session,
        }
    }
}

impl<D: Deps + 'static, C: ConnectorClient + 'static> ServerHandler for SessionMcp<D, C> {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("antilog", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Antilog sets this session up: attach a repo, ask for a credential, change the environment, ask for more budget. A person approves each request.",
            )
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let reach = Reach {
            deps: &*self.app.deps,
            client: &self.backend.client,
            sealer: self.backend.sealer.as_ref(),
        };
        let tools = reach
            .tools(&self.scope)
            .await
            .into_iter()
            .map(|def| {
                let schema = match def.schema {
                    Value::Object(map) => map,
                    _ => JsonObject::new(),
                };
                Tool::new(def.name, def.description, schema)
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let reach = Reach {
            deps: &*self.app.deps,
            client: &self.backend.client,
            sealer: self.backend.sealer.as_ref(),
        };
        let caller = Caller {
            organization: self.scope.organization,
            session: self.session,
            actor: self.scope.member,
            call: uuid::Uuid::now_v7().to_string(),
        };
        let input = Value::Object(request.arguments.unwrap_or_default());
        let (text, refused) = match reach.call(&self.scope, &caller, &request.name, input).await {
            Outcome::Remote { text, refused } => (text, refused),
            Outcome::Antilog(reply) => {
                if let Some(error) = reply.answer.get("error").and_then(Value::as_str) {
                    (error.to_owned(), true)
                } else {
                    (
                        serde_json::to_string_pretty(&reply.answer).unwrap_or_default(),
                        false,
                    )
                }
            }
        };
        let content = vec![ContentBlock::text(text)];
        Ok(if refused {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        }
        .into())
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
        "The session's MCP token is missing, wrong or expired.",
    )
        .into_response()
}

async fn endpoint<D, V, C>(
    State(app): State<Arc<App<D>>>,
    Extension(mcp): Extension<Arc<Mcp<V, C>>>,
    Path(id): Path<SessionId>,
    request: Request,
) -> Response
where
    D: Deps + 'static,
    V: SessionCredentials + 'static,
    C: ConnectorClient + 'static,
{
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim);
    let Some(token) = token else {
        return unauthorized();
    };
    match mcp.credentials.verify(token, Purpose::Mcp).await {
        Ok(Some(session)) if session == id => {}
        Ok(_) => return unauthorized(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    let scope = match app.store().session_scope(id).await {
        Ok(Some(scope)) => scope,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let handler = SessionMcp {
        app,
        backend: mcp.backend.clone(),
        scope,
        session: id,
    };
    // The bearer token is the guard, so the loopback-only Host check does not apply.
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .disable_allowed_hosts();
    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    service.handle(request).await.map(Body::new)
}

pub fn routes<D, V, C>(mcp: Arc<Mcp<V, C>>) -> Router<Arc<App<D>>>
where
    D: Deps + 'static,
    V: SessionCredentials + 'static,
    C: ConnectorClient + 'static,
{
    Router::new()
        .route("/mcp/sessions/{id}", any(endpoint::<D, V, C>))
        .layer(Extension(mcp))
}
