use std::sync::Mutex;

use serde_json::{Value, json};

use super::locked;
use crate::customize::{ClientError, ConnectorClient, RemoteTool};

#[derive(Default)]
struct State {
    tools: Vec<RemoteTool>,
    down: bool,
    requires: Option<String>,
    calls: Vec<Call>,
}

/// One call a connector received: the bearer it was given, the tool and its arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub url: String,
    pub bearer: Option<String>,
    pub tool: String,
    pub arguments: Value,
}

/// A scripted remote MCP server: the tools it lists, the calls it saw, whether it is down, and
/// the bearer it insists on. It answers a call with a text block naming the tool.
#[derive(Default)]
pub struct FakeConnectors(Mutex<State>);

impl FakeConnectors {
    pub fn with_tool(self, name: &str, description: &str) -> Self {
        locked(&self.0).tools.push(RemoteTool {
            name: name.to_owned(),
            description: description.to_owned(),
            schema: json!({"type": "object"}),
        });
        self
    }

    pub fn requiring(self, bearer: &str) -> Self {
        locked(&self.0).requires = Some(bearer.to_owned());
        self
    }

    pub fn set_down(&self, down: bool) {
        locked(&self.0).down = down;
    }

    pub fn calls(&self) -> Vec<Call> {
        locked(&self.0).calls.clone()
    }

    fn admit(&self, bearer: Option<&str>) -> Result<(), ClientError> {
        let state = locked(&self.0);
        if state.down {
            return Err(ClientError::Unreachable("the fake is down".to_owned()));
        }
        match &state.requires {
            Some(want) if bearer != Some(want.as_str()) => Err(ClientError::Refused),
            _ => Ok(()),
        }
    }
}

impl ConnectorClient for FakeConnectors {
    fn allow_private(&self) -> bool {
        false
    }

    async fn list_tools(
        &self,
        _: &str,
        bearer: Option<&str>,
    ) -> Result<Vec<RemoteTool>, ClientError> {
        self.admit(bearer)?;
        Ok(locked(&self.0).tools.clone())
    }

    async fn call_tool(
        &self,
        url: &str,
        bearer: Option<&str>,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, ClientError> {
        self.admit(bearer)?;
        locked(&self.0).calls.push(Call {
            url: url.to_owned(),
            bearer: bearer.map(str::to_owned),
            tool: tool.to_owned(),
            arguments,
        });
        Ok(json!({"content": [{"type": "text", "text": format!("{tool} ran")}]}))
    }
}
