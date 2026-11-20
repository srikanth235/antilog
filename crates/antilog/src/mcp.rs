use std::future::Future;
use std::sync::{Mutex, PoisonError};

use antilog_types::{SessionId, ToolDefinition, ToolResult};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    JsonObject, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use serde_json::Value;

/// The Antilog API as the MCP server sees it. Each tool is one call, made with the caller's token.
pub trait Api: Send + Sync + 'static {
    fn tools(&self) -> impl Future<Output = Result<Vec<ToolDefinition>, String>> + Send;

    fn call(
        &self,
        tool: &str,
        session: Option<SessionId>,
        input: Value,
    ) -> impl Future<Output = Result<ToolResult, String>> + Send;
}

pub struct Http {
    client: reqwest::Client,
    base: String,
    token: String,
    experiment: String,
}

impl Http {
    pub fn new(base: &str, token: &str, experiment: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base: base.trim_end_matches('/').to_owned(),
            token: token.to_owned(),
            experiment: experiment.to_owned(),
        }
    }

    async fn send<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<T, String> {
        let response = request
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| {
                format!("the Antilog server did not answer: {error}. Check --server and retry")
            })?;
        let status = response.status();
        if status.is_success() {
            return response
                .json()
                .await
                .map_err(|error| format!("the Antilog server sent something unreadable: {error}"));
        }
        let body: Value = response.json().await.unwrap_or(Value::Null);
        let message = body
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("no reason given");
        Err(format!("the Antilog server refused ({status}): {message}"))
    }
}

impl Api for Http {
    async fn tools(&self) -> Result<Vec<ToolDefinition>, String> {
        self.send(self.client.get(format!("{}/tools", self.base)))
            .await
    }

    async fn call(
        &self,
        tool: &str,
        session: Option<SessionId>,
        input: Value,
    ) -> Result<ToolResult, String> {
        let url = format!("{}/experiments/{}/tools/{tool}", self.base, self.experiment);
        let body = serde_json::json!({ "session": session, "input": input });
        self.send(self.client.post(url).json(&body)).await
    }
}

pub struct Mcp<A: Api> {
    api: A,
    session: Mutex<Option<SessionId>>,
}

impl<A: Api> Mcp<A> {
    pub fn new(api: A) -> Self {
        Self {
            api,
            session: Mutex::new(None),
        }
    }

    pub async fn list(&self) -> Result<Vec<Tool>, String> {
        let tools = self.api.tools().await?;
        Ok(tools
            .into_iter()
            .map(|t| {
                let schema = match t.input_schema {
                    Value::Object(map) => map,
                    _ => JsonObject::new(),
                };
                Tool::new(t.name, t.description, schema)
            })
            .collect())
    }

    /// The text an agent reads, and whether it is a refusal. The session opened by the first
    /// call is reused, so every trial of this agent belongs to one session.
    pub async fn run(&self, name: &str, arguments: Option<JsonObject>) -> (String, bool) {
        let session = *self.session.lock().unwrap_or_else(PoisonError::into_inner);
        let input = Value::Object(arguments.unwrap_or_default());
        let result = match self.api.call(name, session, input).await {
            Ok(result) => result,
            Err(error) => return (error, true),
        };
        *self.session.lock().unwrap_or_else(PoisonError::into_inner) = Some(result.session);
        if let Some(error) = result.answer.get("error").and_then(Value::as_str) {
            return (error.to_owned(), true);
        }
        let mut text = serde_json::to_string_pretty(&result.answer).unwrap_or_default();
        if let Some(followup) = result.followup {
            text.push_str("\n\n");
            text.push_str(&followup);
        }
        (text, false)
    }
}

impl<A: Api> ServerHandler for Mcp<A> {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("antilog", env!("CARGO_PKG_VERSION")))
            .with_instructions(
            "Antilog measures a change and only its numbers count. Read history, then measure a commit you pushed.",
        )
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools = self
            .list()
            .await
            .map_err(|message| ErrorData::internal_error(message, None))?;
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let (text, refused) = self.run(&request.name, request.arguments).await;
        let content = vec![ContentBlock::text(text)];
        Ok(if refused {
            CallToolResult::error(content)
        } else {
            CallToolResult::success(content)
        }
        .into())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use std::sync::Arc;

    use antilog_core::fakes::{FakeCodeHost, FakeDeps, FakeMachine};
    use antilog_core::seed::experiment;
    use serde_json::json;

    use super::*;

    async fn serve() -> (Mcp<Http>, Arc<FakeDeps>, antilog_core::seed::Seed) {
        let deps = Arc::new(FakeDeps::new(
            FakeMachine::default(),
            FakeCodeHost::default(),
        ));
        let seed = experiment(&deps.store, 1_000).await;
        let now = antilog_core::clock::Clock::now(&deps.clock).as_millisecond();
        let issued =
            antilog_core::token::issue(seed.organization.id, seed.owner.id, "laptop", now).unwrap();
        let entry = antilog_types::AuditEntry::new(
            seed.organization.id,
            Some(seed.owner.id),
            "token.create",
            seed.owner.id,
            antilog_types::AuditOutcome::Done,
            now,
        );
        antilog_core::store::OrganizationStore::create_access_token(
            &deps.store,
            &issued.record,
            &entry,
        )
        .await
        .unwrap();
        let app = antilog_server::api::router(
            deps.clone(),
            antilog_server::api::auth::Settings {
                providers: Vec::new(),
                open_organizations: false,
                secure_cookies: true,
                code_host_webhook_secret: None,
                e2e_seed: false,
                runner: None,
            },
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let http = Http::new(
            &format!("http://127.0.0.1:{port}"),
            &issued.secret,
            &seed.experiment.id.to_string(),
        );
        (Mcp::new(http), deps, seed)
    }

    #[tokio::test]
    async fn the_tools_are_the_ones_every_session_gets() {
        let (mcp, _, _) = serve().await;
        let names: Vec<String> = mcp
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        for tool in [
            "seal",
            "measure",
            "history",
            "deliver",
            "attach_repo",
            "request_credential",
            "set_environment",
            "request_budget",
        ] {
            assert!(names.iter().any(|n| n == tool), "{tool} is served");
        }
    }

    #[tokio::test]
    async fn a_refusal_says_what_to_do_and_calls_share_one_session() {
        let (mcp, deps, seed) = serve().await;
        let args = json!({"commits": [{"repo": "app", "sha": "x"}], "hypothesis": "h"});
        let (text, refused) = mcp.run("measure", args.as_object().cloned()).await;
        assert!(refused);
        assert!(text.contains("call seal"), "{text}");
        let (text, refused) = mcp.run("history", None).await;
        assert!(!refused, "{text}");
        let (text, refused) = mcp.run("nope", None).await;
        assert!(refused && text.contains("the tools are"), "{text}");
        use antilog_core::store::SessionStore;
        let sessions = deps
            .store
            .sessions(seed.organization.id, seed.experiment.id)
            .await
            .unwrap();
        assert_eq!(sessions.len(), 1, "one session for the whole connection");
    }
}
