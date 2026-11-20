//! What a session's MCP endpoint serves: the Antilog tools and the tools of every connector
//! switched on for the experiment. Each connector tool is on, off or ask first. Credentials are
//! opened here, on the server, for the one call, and go nowhere else.

use antilog_types::{
    ApprovalStatus, AuditEntry, AuditOutcome, CardKind, CardPayload, Connector, ConnectorId,
    EventKind, ToolMode,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::connectors::{ConnectorClient, namespaced};
use crate::deps::Deps;
use crate::sealed::Sealer;
use crate::session::now_ms;
use crate::store::{ApprovalStore, CustomizeStore, SessionScope, SessionStore};
use crate::tools::{Caller, ToolDef, ToolHost, ToolReply, Toolbox, definitions};

pub struct Reach<'a, D: Deps, C: ConnectorClient> {
    pub deps: &'a D,
    pub client: &'a C,
    pub sealer: Option<&'a Sealer>,
}

/// What a call came to: an Antilog tool's reply, or a connector's text.
#[derive(Debug, Clone)]
pub enum Outcome {
    Antilog(ToolReply),
    Remote { text: String, refused: bool },
}

fn refused(text: impl Into<String>) -> Outcome {
    Outcome::Remote {
        text: text.into(),
        refused: true,
    }
}

fn digest(input: &Value) -> String {
    let bytes = serde_json::to_vec(input).unwrap_or_default();
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            hex.push_str(&format!("{byte:02x}"));
            hex
        })
}

/// The text blocks of an MCP `tools/call` result, and whether the server called it an error.
fn text_of(result: &Value) -> (String, bool) {
    let blocks = result.get("content").and_then(Value::as_array);
    let text = blocks
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| {
            result
                .get("structuredContent")
                .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                .unwrap_or_else(|| serde_json::to_string_pretty(result).unwrap_or_default())
        });
    let error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    (text, error)
}

impl<D: Deps, C: ConnectorClient> Reach<'_, D, C> {
    async fn switched_on(&self, scope: &SessionScope) -> Vec<Connector> {
        let store = self.deps.store();
        let ids = store
            .experiment_connectors(scope.organization, scope.experiment)
            .await
            .unwrap_or_default();
        let mut found = Vec::new();
        for id in ids {
            if let Ok(Some(connector)) = store.connector(scope.organization, id).await {
                found.push(connector);
            }
        }
        found
    }

    /// The Antilog tools, then the connectors' tools that are not off, namespaced.
    pub async fn tools(&self, scope: &SessionScope) -> Vec<ToolDef> {
        let mut defs = definitions();
        for connector in self.switched_on(scope).await {
            for tool in connector.tools.iter().filter(|t| t.mode != ToolMode::Off) {
                let ask = if tool.mode == ToolMode::Ask {
                    " (a person approves each call)"
                } else {
                    ""
                };
                defs.push(ToolDef {
                    name: namespaced(&connector, &tool.name),
                    description: format!("{} [{}]{ask}", tool.description, connector.name),
                    // The server's own schema is not kept; the agent passes what the tool needs.
                    schema: json!({"type":"object","additionalProperties":true}),
                });
            }
        }
        defs
    }

    fn audit(
        &self,
        scope: &SessionScope,
        caller: &Caller,
        target: String,
        outcome: AuditOutcome,
    ) -> AuditEntry {
        AuditEntry::new(
            scope.organization,
            caller.actor,
            "connector.use",
            target,
            outcome,
            now_ms(self.deps),
        )
    }

    pub async fn call(
        &self,
        scope: &SessionScope,
        caller: &Caller,
        name: &str,
        input: Value,
    ) -> Outcome {
        let connectors = self.switched_on(scope).await;
        let target = name.split_once("__").and_then(|(slug, tool)| {
            connectors
                .iter()
                .find(|c| c.slug == slug)
                .map(|c| (c.clone(), tool.to_owned()))
        });
        match target {
            Some((connector, tool)) => {
                self.connector_call(scope, caller, &connector, &tool, input)
                    .await
            }
            None => Outcome::Antilog(Toolbox(self.deps).call(caller, name, input).await),
        }
    }

    /// `finish` of an accepted measurement, for the endpoint to await.
    pub async fn finish(&self, caller: &Caller, ticket: &crate::tools::Ticket) -> String {
        Toolbox(self.deps).finish(caller, ticket).await
    }

    async fn connector_call(
        &self,
        scope: &SessionScope,
        caller: &Caller,
        connector: &Connector,
        tool: &str,
        input: Value,
    ) -> Outcome {
        let store = self.deps.store();
        let full = namespaced(connector, tool);
        if let Ok(Some(session)) = store.session(scope.organization, caller.session).await {
            let _ = crate::session::record(
                self.deps,
                &session,
                format!("tool:{}", caller.call),
                EventKind::Tool,
                None,
                &full,
            )
            .await;
        }
        let target = format!("{}:{tool}", connector.id);
        let Some(setting) = connector.tools.iter().find(|t| t.name == tool) else {
            return refused(format!(
                "{} has no tool named {tool}: its tools are {}",
                connector.name,
                connector
                    .tools
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };
        match setting.mode {
            ToolMode::Off => {
                let entry = self.audit(scope, caller, target, AuditOutcome::Denied);
                let _ = store.append_audit(&entry).await;
                return refused(
                    "This tool is turned off for the organization. Carry on without it.",
                );
            }
            ToolMode::Ask => {
                let wanted = digest(&input);
                match self.approval(scope, caller, connector, tool, &wanted).await {
                    Ok(true) => {}
                    Ok(false) => {
                        return match self.raise(caller, connector, tool, wanted).await {
                            Ok(answer) => Outcome::Antilog(ToolReply {
                                answer,
                                ticket: None,
                            }),
                            Err(error) => refused(error),
                        };
                    }
                    Err(error) => return refused(error),
                }
            }
            ToolMode::On => {}
        }
        let bearer = match self.bearer(scope, connector.id).await {
            Ok(bearer) => bearer,
            Err(why) => return refused(why),
        };
        let entry = self.audit(scope, caller, target, AuditOutcome::Done);
        let _ = store.append_audit(&entry).await;
        match self
            .client
            .call_tool(&connector.url, bearer.as_deref(), tool, input)
            .await
        {
            Ok(result) => {
                let (text, error) = text_of(&result);
                Outcome::Remote {
                    text,
                    refused: error,
                }
            }
            Err(error) => refused(format!(
                "{} did not answer: {error}. An owner can check it in Customize, Connectors.",
                connector.name
            )),
        }
    }

    async fn bearer(
        &self,
        scope: &SessionScope,
        id: ConnectorId,
    ) -> Result<Option<String>, String> {
        let sealed = self
            .deps
            .store()
            .connector_credential(scope.organization, id)
            .await
            .map_err(|_| "the connector's store is unavailable: retry in a minute".to_owned())?;
        match sealed {
            None => Ok(None),
            Some(sealed) => self
                .sealer
                .and_then(|s| s.open(&sealed, &format!("connector:{id}")).ok())
                .map(|secret| Some(secret.expose().to_owned()))
                .ok_or_else(|| {
                    "the connector's credential cannot be read: an owner updates it in Customize, Connectors".to_owned()
                }),
        }
    }

    /// True when a person (or policy) approved a card for exactly this call and it was not yet used.
    async fn approval(
        &self,
        scope: &SessionScope,
        caller: &Caller,
        connector: &Connector,
        tool: &str,
        wanted: &str,
    ) -> Result<bool, String> {
        let store = self.deps.store();
        let cards = store
            .approvals(scope.organization, scope.experiment)
            .await
            .map_err(|_| "the approvals are unavailable: retry in a minute".to_owned())?;
        for card in cards {
            let matches = card.session_id == Some(caller.session)
                && matches!(
                    &card.payload,
                    Some(CardPayload::UseConnectorTool { connector: c, tool: t, input_digest })
                        if *c == connector.slug && t == tool && input_digest == wanted
                );
            if !matches
                || !matches!(
                    card.status,
                    ApprovalStatus::Approved | ApprovalStatus::ApprovedByPolicy
                )
            {
                continue;
            }
            if store
                .use_approval(scope.organization, card.id)
                .await
                .map_err(|_| "the approvals are unavailable: retry in a minute".to_owned())?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn raise(
        &self,
        caller: &Caller,
        connector: &Connector,
        tool: &str,
        wanted: String,
    ) -> Result<Value, String> {
        // A card for this very call may already be waiting: do not raise a second one.
        let existing = self
            .deps
            .store()
            .approvals(caller.organization, caller.experiment)
            .await
            .unwrap_or_default();
        if let Some(card) = existing.iter().find(|card| {
            card.status == ApprovalStatus::Pending
                && card.session_id == Some(caller.session)
                && matches!(
                    &card.payload,
                    Some(CardPayload::UseConnectorTool { connector: c, tool: t, input_digest })
                        if *c == connector.slug && t == tool && *input_digest == wanted
                )
        }) {
            return Ok(json!({
                "approval": card.id,
                "status": "waiting for a person",
                "note": "A card is in front of a person. Call this tool again with the same input once it is approved."
            }));
        }
        let approval = crate::tools::ask(
            self.deps,
            caller,
            "connector tool",
            crate::tools::Card {
                kind: CardKind::UseConnectorTool,
                summary: format!("Call {tool} on {}", connector.name),
                payload: CardPayload::UseConnectorTool {
                    connector: connector.slug.clone(),
                    tool: tool.to_owned(),
                    input_digest: wanted,
                },
                cost_cents: None,
                needs_owner: false,
            },
        )
        .await
        .map_err(|e| e.0)?;
        let mut answer = crate::tools::waiting(self.deps, caller, &approval).await;
        if let Some(note) = answer.get_mut("note") {
            *note = json!(
                "Call this tool again with the same input once it is approved. Each approval pays for one call."
            );
        }
        Ok(answer)
    }
}
