//! OpenCode, driven over its HTTP API. `Opencode` starts it in a session's machine and runs its
//! CLI there; `Api` is what it speaks once running.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::Duration;

use antilog_types::{ContextUse, Item, ItemKind, Reply, ToolRecord, ToolStatus};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::sandbox::{Exec, Machine};

/// The real thing, pinned by the session image to one version.
pub struct Opencode {
    pub binary: String,
    /// `--pure`: no external plugins.
    pub pure: bool,
    /// Sets `OPENCODE_DISABLE_PROJECT_CONFIG`.
    pub disable_project_config: bool,
}

impl Opencode {
    fn command(&self, dir: &Path, password: &str, args: &[&str]) -> Exec {
        let mut exec = Exec::new(std::iter::once(self.binary.as_str()).chain(args.iter().copied()));
        if self.pure {
            exec.args.push("--pure".to_owned());
        }
        exec = exec
            .dir(dir)
            .env("OPENCODE_SERVER_PASSWORD", password)
            .env("OPENCODE_DISABLE_MODELS_FETCH", "1");
        if self.disable_project_config {
            exec = exec.env("OPENCODE_DISABLE_PROJECT_CONFIG", "1");
        }
        exec
    }

    /// Starts `opencode serve` in the machine and returns the URL the runner reaches it by.
    pub async fn serve<M: Machine>(
        &self,
        machine: &M,
        dir: &Path,
        password: &str,
    ) -> Result<String> {
        let listener = machine.listener().await?;
        let port = listener.port.to_string();
        machine
            .spawn(self.command(
                dir,
                password,
                &["serve", "--hostname", &listener.bind, "--port", &port],
            ))
            .await
            .with_context(|| format!("starting {}", self.binary))?;
        Ok(listener.base)
    }

    /// `opencode export`: the session as JSON, for a later claim to import.
    pub async fn export<M: Machine>(
        &self,
        machine: &M,
        dir: &Path,
        password: &str,
        session: &str,
    ) -> Result<String> {
        let output = machine
            .exec(self.command(dir, password, &["export", session]))
            .await
            .context("running opencode export")?
            .ok("opencode export")?;
        let text = String::from_utf8(output.stdout).context("the export is not text")?;
        // The export is JSON; anything the CLI printed before it is dropped.
        match text.find('{') {
            Some(start) => Ok(text.get(start..).unwrap_or_default().to_owned()),
            None => bail!("opencode export printed no JSON"),
        }
    }

    /// `opencode import`: puts an export back and returns the id of the session it made.
    pub async fn import<M: Machine>(
        &self,
        machine: &M,
        dir: &Path,
        password: &str,
        file: &Path,
    ) -> Result<String> {
        let file = file.to_string_lossy().into_owned();
        let output = machine
            .exec(self.command(dir, password, &["import", &file]))
            .await
            .context("running opencode import")?
            .ok("opencode import")?;
        let text = String::from_utf8_lossy(&output.stdout);
        text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .find(|word| word.starts_with("ses_"))
            .map(str::to_owned)
            .context("opencode import did not say which session it made")
    }
}

/// OpenCode's HTTP API, for the calls the runner makes.
#[derive(Clone)]
pub struct Api {
    http: reqwest::Client,
    base: String,
    password: String,
}

impl Api {
    pub fn new(base: &str, password: &str) -> Self {
        Self {
            // It is on this machine: never through a proxy.
            http: reqwest::Client::builder()
                .no_proxy()
                .build()
                .unwrap_or_default(),
            base: base.trim_end_matches('/').to_owned(),
            password: password.to_owned(),
        }
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{}{path}", self.base))
            .basic_auth("opencode", Some(&self.password))
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .post(format!("{}{path}", self.base))
            .basic_auth("opencode", Some(&self.password))
    }

    /// A shell in the machine: OpenCode's PTY, started in `cwd` with the session's variables.
    pub async fn pty_create(&self, cwd: &str, env: &BTreeMap<String, String>) -> Result<String> {
        let made: Value = self
            .post("/pty")
            .json(&json!({"cwd": cwd, "env": env, "title": "Antilog terminal"}))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        made.get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .context("OpenCode made a terminal without an id")
    }

    pub async fn pty_resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        self.http
            .put(format!("{}/pty/{id}", self.base))
            .basic_auth("opencode", Some(&self.password))
            .json(&json!({"size": {"rows": rows, "cols": cols}}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn pty_remove(&self, id: &str) -> Result<()> {
        self.http
            .delete(format!("{}/pty/{id}", self.base))
            .basic_auth("opencode", Some(&self.password))
            .send()
            .await?;
        Ok(())
    }

    /// The WebSocket request for a terminal: the same address, over `ws`, with the password.
    pub fn pty_connect_request(
        &self,
        id: &str,
    ) -> Result<tokio_tungstenite::tungstenite::handshake::client::Request> {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let url = websocket_url(&format!("{}/pty/{id}/connect", self.base));
        let mut request = url.into_client_request()?;
        let auth = self
            .http
            .get(&self.base)
            .basic_auth("opencode", Some(&self.password))
            .build()?
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .cloned()
            .context("no authorization header")?;
        request.headers_mut().insert("authorization", auth);
        Ok(request)
    }

    /// Waits until the server answers.
    pub async fn wait_healthy(&self, within: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + within;
        loop {
            // A request sent while it is still starting can stall: ask again.
            let got = self
                .get("/global/health")
                .timeout(Duration::from_secs(2))
                .send()
                .await;
            if let Ok(response) = got
                && response.status().is_success()
            {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                bail!("OpenCode did not come up");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn create_session(&self) -> Result<String> {
        let made: Value = self
            .post("/session")
            .json(&json!({}))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        made.get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .context("OpenCode made a session without an id")
    }

    /// A person's words for the agent. `model` is `provider/model`; `effort` is one of the model's
    /// OpenCode variants (the prompt's `variant`), or none for the model's default.
    pub async fn prompt(
        &self,
        session: &str,
        text: &str,
        model: &str,
        effort: Option<&str>,
    ) -> Result<()> {
        let mut body = json!({"parts": [{"type": "text", "text": text}]});
        if let (Some(effort), Some(object)) = (effort, body.as_object_mut()) {
            object.insert("variant".into(), json!(effort));
        }
        if let (Some((provider, id)), Some(object)) = (model.split_once('/'), body.as_object_mut())
        {
            object.insert(
                "model".into(),
                json!({"providerID": provider, "modelID": id}),
            );
        }
        self.post(&format!("/session/{session}/prompt_async"))
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Each model's context window in tokens, by `provider/model`, from the providers OpenCode
    /// is configured with. A model that reports none is left out.
    pub async fn context_limits(&self) -> Result<HashMap<String, u64>> {
        let body: Value = self
            .get("/config/providers")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(limits(&body))
    }

    /// Compacts the conversation now. OpenCode's v2 route first, the older summarize route
    /// (which names the model) when the server does not have it.
    pub async fn compact(&self, session: &str, model: &str) -> Result<()> {
        let response = self
            .post(&format!("/api/session/{session}/compact"))
            .json(&json!({}))
            .send()
            .await?;
        if response.status() != reqwest::StatusCode::NOT_FOUND {
            response.error_for_status()?;
            return Ok(());
        }
        let (provider, id) = model.split_once('/').unwrap_or(("", model));
        self.post(&format!("/session/{session}/summarize"))
            .json(&json!({"providerID": provider, "modelID": id}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn abort(&self, session: &str) -> Result<()> {
        self.post(&format!("/session/{session}/abort"))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn reply(&self, permission: &str, reply: Reply) -> Result<()> {
        let reply = match reply {
            Reply::Once => "once",
            Reply::Reject => "reject",
        };
        self.post(&format!("/permission/{permission}/reply"))
            .json(&json!({"reply": reply}))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn pending_permissions(&self) -> Result<Vec<Value>> {
        Ok(self
            .get("/permission")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Each session's status, by id.
    pub async fn statuses(&self) -> Result<Value> {
        Ok(self
            .get("/session/status")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// The whole conversation, following `x-next-cursor` back through the pages.
    pub async fn messages(&self, session: &str) -> Result<Vec<Value>> {
        let mut all = Vec::new();
        let mut before: Option<String> = None;
        for _ in 0..1_000 {
            let mut path = format!("/session/{session}/message?limit=100");
            if let Some(cursor) = &before {
                path.push_str("&before=");
                path.push_str(&percent(cursor));
            }
            let request = self.get(&path);
            let response = request.send().await?.error_for_status()?;
            let next = response
                .headers()
                .get("x-next-cursor")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let page: Vec<Value> = response.json().await?;
            all.extend(page);
            match next {
                Some(cursor) => before = Some(cursor),
                None => return Ok(all),
            }
        }
        bail!("the conversation has more pages than the runner will read")
    }

    /// Opens the live stream. It has no replay: rebuild from `messages` after each connect.
    pub async fn events(&self) -> Result<Events> {
        let response = self.get("/event").send().await?.error_for_status()?;
        Ok(Events {
            response,
            buffer: String::new(),
        })
    }
}

fn percent(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The server-sent events of an open stream.
pub struct Events {
    response: reqwest::Response,
    buffer: String,
}

impl Events {
    /// The next batch of events; `None` when the stream has closed.
    pub async fn next(&mut self) -> Result<Option<Vec<Value>>> {
        let Some(chunk) = self.response.chunk().await? else {
            return Ok(None);
        };
        self.buffer.push_str(&String::from_utf8_lossy(&chunk));
        Ok(Some(drain(&mut self.buffer)))
    }
}

/// The complete events at the front of `buffer`, removed from it.
fn drain(buffer: &mut String) -> Vec<Value> {
    let normalized = buffer.replace("\r\n", "\n");
    *buffer = normalized;
    let mut events = Vec::new();
    while let Some(end) = buffer.find("\n\n") {
        let block: String = buffer.drain(..end + 2).collect();
        let data: Vec<&str> = block
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim_start)
            .collect();
        if let Ok(event) = serde_json::from_str::<Value>(&data.join("\n")) {
            events.push(event);
        }
    }
    events
}

/// Reads `GET /config/providers`: `providers` is a list, each with its `models` as a record
/// (or a list) whose `limit.context` is the window.
fn limits(body: &Value) -> HashMap<String, u64> {
    let mut out = HashMap::new();
    let providers = body
        .get("providers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    for provider in providers {
        let Some(provider_id) = provider.get("id").and_then(Value::as_str) else {
            continue;
        };
        let models: Vec<(Option<&str>, &Value)> = match provider.get("models") {
            Some(Value::Object(map)) => map.iter().map(|(k, v)| (Some(k.as_str()), v)).collect(),
            Some(Value::Array(list)) => list.iter().map(|v| (None, v)).collect(),
            _ => Vec::new(),
        };
        for (key, model) in models {
            let id = key.or_else(|| model.get("id").and_then(Value::as_str));
            let window = model.pointer("/limit/context").and_then(Value::as_u64);
            if let (Some(id), Some(window)) = (id, window.filter(|w| *w > 0)) {
                out.insert(format!("{provider_id}/{id}"), window);
            }
        }
    }
    out
}

/// What has been said and what has been sent, so that an event seen twice, or again after a
/// reconnect, is sent to the server once. The server also ignores a key it has.
pub struct Tracker {
    session: String,
    roles: HashMap<String, String>,
    sent: HashSet<String>,
    /// What the errors of the turn in progress said, so a failure that arrives twice (on the
    /// message and as a session error) is shown once.
    errors: HashSet<String>,
    /// The `provider/model` each assistant message was made with.
    models: HashMap<String, String>,
    /// Each model's context window, from OpenCode's providers.
    limits: HashMap<String, u64>,
}

impl Tracker {
    pub fn new(session: &str) -> Self {
        Self {
            session: session.to_owned(),
            roles: HashMap::new(),
            sent: HashSet::new(),
            errors: HashSet::new(),
            models: HashMap::new(),
            limits: HashMap::new(),
        }
    }

    /// The windows of the configured models, so a step's use can be reported with its window.
    pub fn set_limits(&mut self, limits: HashMap<String, u64>) {
        self.limits = limits;
    }

    fn mine(&self, properties: &Value) -> bool {
        properties.get("sessionID").and_then(Value::as_str) == Some(&self.session)
    }

    /// What one live event comes to.
    pub fn event(&mut self, event: &Value) -> Vec<Item> {
        let Some(properties) = event.get("properties") else {
            return Vec::new();
        };
        match event.get("type").and_then(Value::as_str) {
            Some("message.updated") if self.mine(properties) => properties
                .get("info")
                .map(|info| self.know(info))
                .unwrap_or_default(),
            Some("message.part.updated") if self.mine(properties) => properties
                .get("part")
                .map(|part| self.part(part))
                .unwrap_or_default(),
            Some("permission.asked") if self.mine(properties) => {
                permission(properties).into_iter().collect()
            }
            Some("session.status") if self.mine(properties) => {
                let kind = properties
                    .pointer("/status/type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if kind == "busy" {
                    self.errors.clear();
                }
                vec![Item::Status {
                    busy: kind != "idle",
                }]
            }
            Some("session.idle") if self.mine(properties) => vec![Item::Status { busy: false }],
            // A failed turn, such as a call the provider refused: the person reads why.
            Some("session.error") if self.mine(properties) => {
                let key = event
                    .get("id")
                    .and_then(Value::as_str)
                    .map_or_else(|| uuid::Uuid::now_v7().to_string(), str::to_owned);
                properties
                    .get("error")
                    .and_then(|error| self.failure(key, error))
                    .into_iter()
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// What a message read back from `GET /session/{id}/message` comes to.
    pub fn message(&mut self, message: &Value) -> Vec<Item> {
        let mut items = message
            .get("info")
            .map(|info| self.know(info))
            .unwrap_or_default();
        if let Some(parts) = message.get("parts").and_then(Value::as_array) {
            items.extend(parts.iter().flat_map(|part| self.part(part)));
        }
        items
    }

    /// Of the context uses in `items` only the last matters: a rebuild reads every step again.
    pub fn latest_context(items: &mut Vec<Item>) {
        let last = items
            .iter()
            .rposition(|item| matches!(item, Item::Context { .. }));
        let mut at = 0;
        items.retain(|item| {
            at += 1;
            !matches!(item, Item::Context { .. }) || Some(at - 1) == last
        });
    }

    /// Remembers whose message this is; an assistant message that ended in an error is an item.
    fn know(&mut self, info: &Value) -> Vec<Item> {
        let (Some(id), Some(role)) = (
            info.get("id").and_then(Value::as_str),
            info.get("role").and_then(Value::as_str),
        ) else {
            return Vec::new();
        };
        self.roles.insert(id.to_owned(), role.to_owned());
        if role != "assistant" {
            return Vec::new();
        }
        let model = |pointer: &str| info.pointer(pointer).and_then(Value::as_str);
        if let (Some(provider), Some(model)) = (
            model("/providerID").or_else(|| model("/model/providerID")),
            model("/modelID").or_else(|| model("/model/modelID")),
        ) {
            self.models
                .insert(id.to_owned(), format!("{provider}/{model}"));
        }
        info.get("error")
            .and_then(|error| self.failure(id.to_owned(), error))
            .into_iter()
            .collect()
    }

    /// A failure as the person reads it, once: the same text twice in a turn is one error.
    fn failure(&mut self, key: String, error: &Value) -> Option<Item> {
        let key = format!("error:{key}");
        if !self.sent.insert(key.clone()) {
            return None;
        }
        let text = error_text(error);
        self.errors
            .insert(text.clone())
            .then_some(Item::Error { key, text })
    }

    /// Only what an agent said or called is sent: a person's words are kept by the server when
    /// they are written, and a part still being written is sent when it is finished.
    fn part(&mut self, part: &Value) -> Vec<Item> {
        let text = |name: &str| part.get(name).and_then(Value::as_str);
        let (Some(message), Some(id)) = (text("messageID"), text("id")) else {
            return Vec::new();
        };
        let key = format!("{message}:{id}");
        // A compaction marker sits in a message of the person's side: it is Antilog's note.
        if text("type") == Some("compaction") {
            if !self.sent.insert(key.clone()) {
                return Vec::new();
            }
            let how = if flag(part, "overflow") {
                " (the window overflowed)"
            } else if flag(part, "auto") {
                " (automatic)"
            } else {
                ""
            };
            return vec![Item::Note {
                key: format!("compaction:{key}"),
                text: format!("Context compacted{how}."),
            }];
        }
        if self.roles.get(message).map(String::as_str) != Some("assistant") {
            return Vec::new();
        }
        if self.sent.contains(&key) {
            return Vec::new();
        }
        // A step of the agent ended: its input, cached or not, is what sat in the window.
        if text("type") == Some("step-finish") {
            self.sent.insert(key);
            let count = |pointer: &str| part.pointer(pointer).and_then(Value::as_u64).unwrap_or(0);
            let used =
                count("/tokens/input") + count("/tokens/cache/read") + count("/tokens/cache/write");
            if used == 0 {
                return Vec::new();
            }
            let window = self
                .models
                .get(message)
                .and_then(|model| self.limits.get(model))
                .copied();
            return vec![Item::Context {
                context: ContextUse { used, window },
            }];
        }
        // The Antilog tools reach the agent through our own MCP endpoint, which records each call
        // with its input and answer; recording OpenCode's view of it too would show it twice.
        if text("type") == Some("tool") && text("tool").is_some_and(|n| n.starts_with(MCP_PREFIX)) {
            self.sent.insert(key);
            return Vec::new();
        }
        let item = match text("type") {
            Some("text")
                if !flag(part, "synthetic")
                    && !flag(part, "ignored")
                    && part.pointer("/time/end").is_some() =>
            {
                text("text")
                    .filter(|words| !words.trim().is_empty())
                    .map(|words| (ItemKind::Agent, words.to_owned()))
            }
            Some("tool") => matches!(
                part.pointer("/state/status").and_then(Value::as_str),
                Some("completed" | "error")
            )
            .then(|| text("tool").map(|name| (ItemKind::Tool, name.to_owned())))
            .flatten(),
            _ => None,
        };
        match item {
            Some((kind, text)) => {
                self.sent.insert(key.clone());
                let tool = (kind == ItemKind::Tool).then(|| tool_record(&text, part));
                vec![Item::Words {
                    key,
                    kind,
                    text,
                    tool,
                }]
            }
            None => Vec::new(),
        }
    }
}

/// What a provider's refusal says, as OpenCode reports it: the error's message, and the
/// provider's own words from the response body when they add to it.
fn error_text(error: &Value) -> String {
    let data = error.get("data").unwrap_or(error);
    let message = data
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty());
    let body: Option<String> = data
        .get("responseBody")
        .and_then(Value::as_str)
        .and_then(|raw| {
            serde_json::from_str::<Value>(raw).ok().and_then(|json| {
                ["/error/message", "/message", "/error"]
                    .iter()
                    .find_map(|path| {
                        json.pointer(path)
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
            })
        });
    let text = match (message, body) {
        (Some(message), Some(body)) if !message.contains(&body) => format!("{message}: {body}"),
        (Some(message), _) => message.to_owned(),
        (None, Some(body)) => body,
        (None, None) => error
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("the model call failed")
            .to_owned(),
    };
    text.chars().take(2000).collect()
}

/// What OpenCode names the tools of the `antilog` MCP entry in the managed config.
const MCP_PREFIX: &str = "antilog_";

/// A finished tool part as the conversation keeps it: a short label, how it ended, and the input
/// and output. The server cuts and cleans both again, so this only keeps the message small.
fn tool_record(name: &str, part: &Value) -> ToolRecord {
    let state = part.get("state").unwrap_or(&Value::Null);
    let input = state.get("input").unwrap_or(&Value::Null);
    let field = |key: &str| input.get(key).and_then(Value::as_str);
    let label = match name {
        "bash" => field("command").or_else(|| field("description")),
        "read" | "edit" | "write" | "patch" | "multiedit" => {
            field("filePath").or_else(|| field("path"))
        }
        "grep" | "glob" => field("pattern"),
        "webfetch" => field("url"),
        _ => None,
    }
    .unwrap_or(name);
    let shown = match (name, field("command")) {
        ("bash", Some(command)) => command.to_owned(),
        _ if input.is_null() => String::new(),
        _ => serde_json::to_string_pretty(input).unwrap_or_default(),
    };
    let exit = state.pointer("/metadata/exit").and_then(Value::as_i64);
    let failed = state.get("status").and_then(Value::as_str) == Some("error")
        || exit.is_some_and(|code| code != 0);
    let output = state
        .get("output")
        .or_else(|| state.get("error"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    ToolRecord {
        label: label.to_owned(),
        status: if failed {
            ToolStatus::Failed
        } else {
            ToolStatus::Done
        },
        input: (!shown.is_empty()).then_some(shown),
        output,
    }
    .cleaned()
}

fn flag(part: &Value, name: &str) -> bool {
    part.get(name).and_then(Value::as_bool).unwrap_or(false)
}

/// A permission the agent asked for, as an ask-first card needs it.
pub fn permission(request: &Value) -> Option<Item> {
    let id = request.get("id").and_then(Value::as_str)?;
    let tool = request
        .get("permission")
        .and_then(Value::as_str)
        .unwrap_or("a tool");
    let patterns: Vec<&str> = request
        .get("patterns")
        .and_then(Value::as_array)
        .map(|all| all.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    Some(Item::Permission {
        id: id.to_owned(),
        tool: tool.to_owned(),
        detail: patterns.join(", "),
    })
}

/// `http` becomes `ws`, `https` becomes `wss`.
pub fn websocket_url(url: &str) -> String {
    match url.split_once("://") {
        Some(("https", rest)) => format!("wss://{rest}"),
        Some(("http", rest)) => format!("ws://{rest}"),
        _ => url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant(tracker: &mut Tracker, id: &str) {
        let none = tracker.event(&json!({
            "type": "message.updated",
            "properties": {"sessionID": "ses_1", "info": {"id": id, "role": "assistant"}}
        }));
        assert!(none.is_empty());
    }

    #[test]
    fn a_stream_is_cut_into_events_wherever_the_chunks_fall() {
        let mut buffer = String::from("data: {\"type\":\"a\"}\n\ndata: {\"ty");
        assert_eq!(drain(&mut buffer), vec![json!({"type": "a"})]);
        buffer.push_str("pe\":\"b\"}\r\n\r\n: keep-alive\n\n");
        assert_eq!(drain(&mut buffer), vec![json!({"type": "b"})]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn only_what_the_agent_finished_is_sent_and_only_once() {
        let mut tracker = Tracker::new("ses_1");
        assistant(&mut tracker, "msg_a");
        let part = |id: &str, kind: &str, extra: Value| {
            let mut part =
                json!({"id": id, "messageID": "msg_a", "sessionID": "ses_1", "type": kind});
            if let (Some(base), Some(extra)) = (part.as_object_mut(), extra.as_object()) {
                base.extend(extra.clone());
            }
            json!({"type": "message.part.updated", "properties": {"sessionID": "ses_1", "part": part}})
        };
        // Still being written: nothing yet.
        assert!(
            tracker
                .event(&part(
                    "prt_1",
                    "text",
                    json!({"text": "Hel", "time": {"start": 1}})
                ))
                .is_empty()
        );
        let done = part(
            "prt_1",
            "text",
            json!({"text": "Hello", "time": {"start": 1, "end": 2}}),
        );
        assert_eq!(
            tracker.event(&done),
            vec![Item::Words {
                key: "msg_a:prt_1".into(),
                kind: ItemKind::Agent,
                text: "Hello".into(),
                tool: None,
            }]
        );
        assert!(
            tracker.event(&done).is_empty(),
            "the same part is not sent twice"
        );
        let tool = |status: &str| {
            part(
                "prt_2",
                "tool",
                json!({"tool": "bash", "callID": "c", "state": {"status": status}}),
            )
        };
        assert!(tracker.event(&tool("running")).is_empty());
        assert_eq!(
            tracker.event(&tool("completed")),
            vec![Item::Words {
                key: "msg_a:prt_2".into(),
                kind: ItemKind::Tool,
                text: "bash".into(),
                tool: Some(ToolRecord {
                    label: "bash".into(),
                    status: ToolStatus::Done,
                    input: None,
                    output: None,
                }),
            }]
        );
    }

    #[test]
    fn a_step_finish_is_the_context_the_call_filled_with_the_models_window() {
        let mut tracker = Tracker::new("ses_1");
        tracker.set_limits(limits(&json!({"providers": [
            {"id": "vendor", "models": {"m": {"limit": {"context": 1_000_000}}, "none": {}}},
            {"id": "other", "models": [{"id": "n", "limit": {"context": 8000}}]},
        ]})));
        assert_eq!(tracker.limits.len(), 2);
        assert_eq!(tracker.limits["vendor/m"], 1_000_000);
        assert_eq!(tracker.limits["other/n"], 8000);
        let none = tracker.event(&json!({
            "type": "message.updated",
            "properties": {"sessionID": "ses_1", "info": {
                "id": "msg_a", "role": "assistant", "providerID": "vendor", "modelID": "m"}}
        }));
        assert!(none.is_empty());
        let step = |id: &str, input: u64| {
            json!({"type": "message.part.updated", "properties": {"sessionID": "ses_1", "part": {
                "id": id, "messageID": "msg_a", "type": "step-finish",
                "tokens": {"input": input, "output": 9, "reasoning": 0, "cache": {"read": 100, "write": 10}}}}})
        };
        assert_eq!(
            tracker.event(&step("p1", 1)),
            vec![Item::Context {
                context: ContextUse {
                    used: 111,
                    window: Some(1_000_000)
                }
            }]
        );
        assert!(tracker.event(&step("p1", 1)).is_empty(), "once");
        // A model whose window is unknown is reported without one.
        let mut unknown = Tracker::new("ses_1");
        assistant(&mut unknown, "msg_a");
        assert_eq!(
            unknown.event(&step("p2", 5)),
            vec![Item::Context {
                context: ContextUse {
                    used: 115,
                    window: None
                }
            }]
        );
        // Of many steps read back, only the last is kept.
        let mut items = vec![
            Item::Context {
                context: ContextUse {
                    used: 1,
                    window: None,
                },
            },
            Item::Status { busy: true },
            Item::Context {
                context: ContextUse {
                    used: 2,
                    window: None,
                },
            },
        ];
        Tracker::latest_context(&mut items);
        assert_eq!(
            items,
            vec![
                Item::Status { busy: true },
                Item::Context {
                    context: ContextUse {
                        used: 2,
                        window: None
                    }
                },
            ]
        );
    }

    #[test]
    fn a_compaction_is_a_note_once() {
        let mut tracker = Tracker::new("ses_1");
        let part = |auto: bool, overflow: bool| {
            json!({"type": "message.part.updated", "properties": {"sessionID": "ses_1", "part": {
                "id": "prt_c", "messageID": "msg_u", "type": "compaction",
                "auto": auto, "overflow": overflow}}})
        };
        let first = tracker.event(&part(true, false));
        let [Item::Note { text, .. }] = first.as_slice() else {
            panic!("one note")
        };
        assert_eq!(text, "Context compacted (automatic).");
        assert!(tracker.event(&part(true, false)).is_empty());
        let mut other = Tracker::new("ses_1");
        let plain = other.event(&part(false, false));
        let [Item::Note { text, .. }] = plain.as_slice() else {
            panic!("one note")
        };
        assert_eq!(text, "Context compacted.");
    }

    #[test]
    fn a_tool_call_carries_what_it_ran_and_how_it_ended() {
        let mut tracker = Tracker::new("ses_1");
        assistant(&mut tracker, "msg_a");
        let tool = |id: &str, name: &str, state: Value| {
            json!({"type": "message.part.updated", "properties": {"sessionID": "ses_1", "part": {
                "id": id, "messageID": "msg_a", "type": "tool", "tool": name, "callID": id, "state": state}}})
        };
        let failed = tracker.event(&tool(
            "prt_1",
            "bash",
            json!({"status": "completed", "input": {"command": "cargo test\n--all"}, "output": "boom", "metadata": {"exit": 101}}),
        ));
        let [
            Item::Words {
                tool: Some(record), ..
            },
        ] = failed.as_slice()
        else {
            panic!("one tool item, got {failed:?}")
        };
        assert_eq!(record.label, "cargo test");
        assert_eq!(record.status, ToolStatus::Failed);
        assert_eq!(record.input.as_deref(), Some("cargo test\n--all"));
        assert_eq!(record.output.as_deref(), Some("boom"));
        let edit = tracker.event(&tool(
            "prt_2",
            "edit",
            json!({"status": "error", "input": {"filePath": "src/a.rs"}, "error": "no match"}),
        ));
        let [
            Item::Words {
                tool: Some(record), ..
            },
        ] = edit.as_slice()
        else {
            panic!("one tool item")
        };
        assert_eq!(
            (record.label.as_str(), record.status),
            ("src/a.rs", ToolStatus::Failed)
        );
        // An Antilog tool is recorded by the server's MCP endpoint, not twice.
        let ours = tool(
            "prt_3",
            "antilog_seal",
            json!({"status": "completed", "input": {}}),
        );
        assert!(tracker.event(&ours).is_empty());
    }

    #[test]
    fn a_persons_words_and_other_sessions_are_not_sent() {
        let mut tracker = Tracker::new("ses_1");
        tracker.event(&json!({"type": "message.updated", "properties": {"sessionID": "ses_1", "info": {"id": "msg_u", "role": "user"}}}));
        let person = json!({"type": "message.part.updated", "properties": {"sessionID": "ses_1", "part": {"id": "prt_9", "messageID": "msg_u", "type": "text", "text": "hi", "time": {"start": 1, "end": 2}}}});
        assert!(tracker.event(&person).is_empty());
        let elsewhere = json!({"type": "session.status", "properties": {"sessionID": "ses_2", "status": {"type": "busy"}}});
        assert!(tracker.event(&elsewhere).is_empty());
    }

    #[test]
    fn a_rebuild_from_the_messages_sends_what_a_dropped_stream_missed() {
        let mut tracker = Tracker::new("ses_1");
        let listed = json!({"info": {"id": "msg_a", "role": "assistant"}, "parts": [
            {"id": "prt_1", "messageID": "msg_a", "type": "text", "text": "Done", "time": {"start": 1, "end": 2}},
        ]});
        assert_eq!(tracker.message(&listed).len(), 1);
        assert!(
            tracker.message(&listed).is_empty(),
            "a second rebuild adds nothing"
        );
    }

    #[test]
    fn a_failed_turn_says_why() {
        let mut tracker = Tracker::new("ses_1");
        let failed = json!({"id": "evt_9", "type": "session.error", "properties": {"sessionID": "ses_1", "error": {"name": "UnknownError", "data": {"message": "budget spent"}}}});
        assert_eq!(
            tracker.event(&failed),
            vec![Item::Error {
                key: "error:evt_9".into(),
                text: "budget spent".into(),
            }]
        );
    }

    #[test]
    fn a_refused_call_is_an_error_once_with_the_providers_words() {
        let mut tracker = Tracker::new("ses_1");
        let info = json!({"id": "msg_1", "role": "assistant", "error": {"name": "APIError", "data": {"message": "Payment Required", "statusCode": 402, "responseBody": "{\"error\":{\"message\":\"Insufficient balance\"}}"}}});
        let on_message = tracker.event(
            &json!({"type": "message.updated", "properties": {"sessionID": "ses_1", "info": info}}),
        );
        assert_eq!(
            on_message,
            vec![Item::Error {
                key: "error:msg_1".into(),
                text: "Payment Required: Insufficient balance".into(),
            }]
        );
        // The same failure as a session error, and again from the read-back after a reconnect.
        let again = json!({"id": "evt_1", "type": "session.error", "properties": {"sessionID": "ses_1", "error": info["error"]}});
        assert_eq!(tracker.event(&again), Vec::new());
        assert_eq!(
            tracker.message(&json!({"info": info, "parts": []})),
            Vec::new()
        );
    }

    #[test]
    fn a_permission_and_the_status_become_items() {
        let mut tracker = Tracker::new("ses_1");
        let asked = json!({"type": "permission.asked", "properties": {"id": "per_1", "sessionID": "ses_1", "permission": "bash", "patterns": ["cargo test", "cargo fmt"], "metadata": {}, "always": []}});
        assert_eq!(
            tracker.event(&asked),
            vec![Item::Permission {
                id: "per_1".into(),
                tool: "bash".into(),
                detail: "cargo test, cargo fmt".into()
            }]
        );
        let status = |kind: &str| json!({"type": "session.status", "properties": {"sessionID": "ses_1", "status": {"type": kind}}});
        assert_eq!(
            tracker.event(&status("busy")),
            vec![Item::Status { busy: true }]
        );
        assert_eq!(
            tracker.event(&status("idle")),
            vec![Item::Status { busy: false }]
        );
    }
}
