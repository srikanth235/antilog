//! One session on this machine: clone, configure, start OpenCode, carry the conversation to the
//! server and the server's commands to OpenCode, and let go.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use antilog_types::{
    Command, Envelope, Item, Release, ReleaseOutcome, RepoSpec, SessionId, SessionSpec,
};
use anyhow::{Context, Result, bail};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::api::{NotHeld, Server};
use super::changes::{self, Base};

use super::opencode::{Api, Opencode, Tracker, permission};
use super::sandbox::{Exec, Machine, Sandbox, SandboxSpec, run as run_in_machine};

/// Where a repo of the session is cloned.
pub fn repo_dir(workdir: &Path, repo: &str) -> PathBuf {
    workdir.join(repo.rsplit('/').next().unwrap_or(repo))
}

/// What the poll loop tells a session's task.
pub enum Msg {
    Command(Envelope),
    /// The lease is gone: stop without a word to the server.
    Stop,
}

/// How long the session's own steps may take.
#[derive(Clone, Copy)]
pub struct Timing {
    /// The environment's setup script.
    pub setup: Duration,
    /// How long the agent may sit idle before the machine is let go; zero keeps it.
    pub idle: Duration,
}

/// Command ids finished since the last poll.
pub type Done = Arc<Mutex<Vec<Uuid>>>;

fn finish(done: &Done, id: Uuid) {
    done.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(id);
}

/// A clone address with the short-lived credential in it, for an https host.
fn clone_url(repo: &RepoSpec) -> String {
    match (&repo.token, repo.url.strip_prefix("https://")) {
        (Some(token), Some(rest)) => format!("https://x-access-token:{token}@{rest}"),
        _ => repo.url.clone(),
    }
}

pub(super) async fn clone_all<M: Machine>(
    machine: &M,
    dir: &Path,
    spec: &SessionSpec,
) -> Result<()> {
    run_in_machine(
        machine,
        Exec::new([
            "mkdir".to_owned(),
            "-p".to_owned(),
            dir.to_string_lossy().into_owned(),
        ]),
        "making the session's folder",
    )
    .await?;
    for repo in &spec.repos {
        let target = repo_dir(dir, &repo.name);
        let target = target.to_string_lossy().into_owned();
        let present = machine
            .exec(Exec::new([
                "test".to_owned(),
                "-d".to_owned(),
                format!("{target}/.git"),
            ]))
            .await?
            .status
            == 0;
        if present {
            continue;
        }
        run_in_machine(
            machine,
            Exec::new([
                "git".to_owned(),
                "clone".to_owned(),
                "--quiet".to_owned(),
                clone_url(repo),
                target.clone(),
            ])
            .dir(dir),
            "git clone",
        )
        .await
        // The clone address carries a credential; keep it out of the failure's text.
        .map_err(|error| match &repo.token {
            Some(token) => anyhow::anyhow!("{}", format!("{error:#}").replace(token, "***")),
            None => error,
        })?;
    }
    Ok(())
}

/// What the setup script left behind, in a file in the session's folder: a machine that is
/// started again in the same container does not run the script a second time.
const SETUP_MARKER: &str = ".antilog-setup";
/// How much of the script's output the note keeps: the end, where the reason is.
const NOTE_BYTES: usize = 2048;

/// How the environment's setup script went.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Setup {
    /// There is none, or it already ran in this container.
    Skipped,
    Done,
    /// It failed or timed out: the session goes on without it, and the person and the agent are
    /// told.
    Failed(Failure),
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Failure {
    /// The exit status; `None` when the script could not be run at all.
    pub exit: Option<i32>,
    pub timed_out: bool,
    /// The last 2 KB of what it printed.
    pub tail: String,
}

impl Failure {
    /// The conversation's note: what happened, the exit code and the end of the output.
    pub(super) fn note(&self, limit: Duration) -> String {
        let how = if self.timed_out {
            format!("did not finish in {} and was stopped", span(limit))
        } else {
            "failed".to_owned()
        };
        let code = self.exit.map_or_else(
            || "it could not be run".to_owned(),
            |code| format!("exit code {code}"),
        );
        let mut text = format!(
            "The environment's setup script {how} ({code}). The session started without it, so tools it installs may be missing."
        );
        if !self.tail.is_empty() {
            text.push_str("\n\nLast output:\n");
            text.push_str(&self.tail);
        }
        text
    }

    /// The line the agent reads before the person's first message.
    pub(super) fn line(&self) -> String {
        let how = if self.timed_out {
            "timed out"
        } else {
            "failed"
        };
        format!(
            "[Antilog] The environment's setup script {how}, so tools it installs may be missing."
        )
    }
}

fn span(limit: Duration) -> String {
    let secs = limit.as_secs();
    if secs >= 60 && secs.is_multiple_of(60) {
        format!("{} minutes", secs / 60)
    } else {
        format!("{secs} seconds")
    }
}

/// The last `NOTE_BYTES` bytes of `bytes` as text, from a whole character, with each repo's
/// clone credential taken out.
fn tail(bytes: &[u8], spec: &SessionSpec) -> String {
    let start = bytes.len().saturating_sub(NOTE_BYTES);
    let mut text = String::from_utf8_lossy(bytes.get(start..).unwrap_or_default())
        .trim_start_matches('\u{FFFD}')
        .trim()
        .to_owned();
    for token in spec.repos.iter().filter_map(|repo| repo.token.as_deref()) {
        text = text.replace(token, "***");
    }
    text
}

const REASON_CHARS: usize = 400;

/// Why a session did not start, as the person reads it: each repo's clone credential and any
/// Antilog token taken out, the first lines only, cut short.
pub(super) fn reason(error: &anyhow::Error, spec: &SessionSpec) -> String {
    let mut text = format!("{error:#}");
    for secret in spec
        .repos
        .iter()
        .filter_map(|repo| repo.token.as_deref())
        .chain([
            spec.model_token.as_str(),
            spec.mcp_token.as_str(),
            spec.runner_token.as_str(),
        ])
        .filter(|secret| secret.len() >= 8)
    {
        text = text.replace(secret, "***");
    }
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let cut: String = text.chars().take(REASON_CHARS).collect();
    if cut.len() < text.len() {
        format!("{cut}…")
    } else {
        cut
    }
}

/// Runs the environment's setup script as root, from the folder that holds the repo checkouts
/// (each repo a subfolder named after it), with the environment's variables set in the machine.
/// It runs once per container, after the clones and before the agent starts. A script that fails
/// or runs out of time is not the session's end: the caller says so and goes on.
pub(super) async fn run_setup<M: Machine>(
    machine: &M,
    root: &Path,
    spec: &SessionSpec,
    limit: Duration,
) -> Setup {
    let script = &spec.environment.setup_script;
    if script.trim().is_empty() {
        return Setup::Skipped;
    }
    let marker = root.join(SETUP_MARKER);
    let ran = machine
        .exec(Exec::new([
            "test".to_owned(),
            "-e".to_owned(),
            marker.to_string_lossy().into_owned(),
        ]))
        .await
        .is_ok_and(|out| out.status == 0);
    if ran {
        return Setup::Skipped;
    }
    let result = run_script(machine, root, script, &[], spec, limit).await;
    // Whatever happened, this container has had its turn.
    machine.put(&marker, b"").await.ok();
    result
}

/// Runs `script` as root from `root`, under `timeout`, with `env` added to the machine's own.
/// `timeout` ends the script and what it started; the second timer is for a machine that cannot
/// be asked. stderr is folded into stdout so the tail reads as it happened.
async fn run_script<M: Machine>(
    machine: &M,
    root: &Path,
    script: &str,
    env: &[(String, String)],
    spec: &SessionSpec,
    limit: Duration,
) -> Setup {
    let secs = limit.as_secs().max(1);
    let mut exec = Exec::new([
        "sh".to_owned(),
        "-c".to_owned(),
        "exec timeout -k 10 \"$1\" bash -c \"$2\" 2>&1".to_owned(),
        "sh".to_owned(),
        secs.to_string(),
        script.to_owned(),
    ])
    .dir(root);
    exec.env = env.to_vec();
    let started = Instant::now();
    let result = tokio::time::timeout(limit + Duration::from_secs(30), machine.exec(exec)).await;
    match result {
        Err(_) => Setup::Failed(Failure {
            exit: None,
            timed_out: true,
            tail: String::new(),
        }),
        Ok(Err(error)) => Setup::Failed(Failure {
            exit: None,
            timed_out: false,
            tail: tail(format!("{error:#}").as_bytes(), spec),
        }),
        Ok(Ok(out)) if out.status == 0 => Setup::Done,
        Ok(Ok(out)) => Setup::Failed(Failure {
            exit: Some(out.status),
            // 124 is `timeout`'s own answer; 137 is its follow-up kill.
            timed_out: out.status == 124 || (out.status == 137 && started.elapsed() >= limit),
            tail: tail(&out.stdout, spec),
        }),
    }
}

/// The variables of an environment as a shell script that new bash processes source: each name
/// set, and each name an earlier version set that this one does not unset. A name the machine
/// cannot hold is left out, as the container's own start leaves it out.
fn env_script(variables: &BTreeMap<String, String>, removed: &[String]) -> String {
    let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
    let ok = |name: &str| {
        let mut chars = name.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !name.starts_with("ANTILOG_")
    };
    let mut text = String::from("# The session's environment variables, written by Antilog.\n");
    for name in removed.iter().filter(|name| ok(name)) {
        text.push_str(&format!("unset {name}\n"));
    }
    for (name, value) in variables.iter().filter(|(name, _)| ok(name)) {
        text.push_str(&format!("export {name}={}\n", quote(value)));
    }
    text
}

/// Where OpenCode works: the clone when there is one, else the folder of clones.
fn work_dir(dir: &Path, spec: &SessionSpec) -> PathBuf {
    match spec.repos.as_slice() {
        [only] => repo_dir(dir, &only.name),
        _ => dir.to_path_buf(),
    }
}

/// A path inside a skill pack: relative, with nothing that climbs out of the folder.
fn inside(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
}

#[derive(serde::Deserialize)]
struct Pack {
    files: Vec<PackFile>,
}

#[derive(serde::Deserialize)]
struct PackFile {
    path: String,
    content: String,
}

/// Writes each pack to `<skills dir>/<name>/`. A pack is JSON: text files with their paths. One
/// that cannot be fetched, or names a path outside its folder, is skipped whole.
pub(super) async fn install_skills<M: Machine>(machine: &M, server: &Server, spec: &SessionSpec) {
    for skill in &spec.skills {
        let pack = async {
            if !inside(&skill.name) || Path::new(&skill.name).components().count() != 1 {
                bail!("the name is not a folder name");
            }
            let Some(bytes) = server.skill(&skill.path, &spec.runner_token).await? else {
                bail!("not available");
            };
            let pack: Pack = serde_json::from_slice(&bytes).context("reading the pack")?;
            if let Some(bad) = pack.files.iter().find(|file| !inside(&file.path)) {
                bail!("{} is outside the pack's folder", bad.path);
            }
            let target = machine.paths().skills.join(&skill.name);
            machine
                .exec(Exec::new([
                    "rm".to_owned(),
                    "-rf".to_owned(),
                    target.to_string_lossy().into_owned(),
                ]))
                .await?
                .ok("clearing the pack's folder")?;
            for file in &pack.files {
                machine
                    .put(&target.join(&file.path), file.content.as_bytes())
                    .await?;
            }
            anyhow::Ok(())
        }
        .await;
        if let Err(error) = pack {
            tracing::warn!(skill = %skill.name, version = %skill.version, %error, "skill pack not installed");
        }
    }
}

fn password() -> Result<String> {
    let mut bytes = [0_u8; 24];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut random| std::io::Read::read_exact(&mut random, &mut bytes))
        .context("reading random bytes")?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

struct Live<S: Sandbox> {
    machine: Arc<S::Machine>,
    opencode: Arc<Opencode>,
    server: Server,
    spec: SessionSpec,
    /// Where OpenCode works, and its server's password.
    dir: PathBuf,
    password: String,
    api: Api,
    oc_session: String,
    tracker: Tracker,
    outbox: Vec<Item>,
    done: Done,
    /// Said to the agent ahead of the first message it is sent: the setup script failed.
    heads_up: Option<String>,
    /// Each repo of the machine with where it was cloned from, for the Changes panel.
    repos: Vec<(String, PathBuf, Base)>,
    setup_timeout: Duration,
    /// How long the agent may sit idle before the machine is let go; zero keeps it.
    idle_limit: Duration,
    /// Since when nothing has happened: the agent is not busy and no one has written to it.
    quiet_since: Option<Instant>,
}

impl<S: Sandbox> Live<S> {
    /// Sends what is waiting. A server that cannot be reached leaves it for the next try: the
    /// server ignores an item it already has.
    async fn flush(&mut self) -> Result<()> {
        if self.outbox.is_empty() {
            return Ok(());
        }
        match self
            .server
            .items(self.spec.session, &self.spec.runner_token, &self.outbox)
            .await
        {
            Ok(()) => {
                self.outbox.clear();
                Ok(())
            }
            Err(error) if error.is::<NotHeld>() => Err(error),
            Err(error) => {
                tracing::warn!(%error, "could not send events; will try again");
                Ok(())
            }
        }
    }

    /// Reads what the agent changed in each repo and sends it, replacing the last report. A repo
    /// or a server that cannot answer leaves the old report: the next turn tries again.
    async fn report_changes(&self) -> Result<()> {
        let report = changes::report(&*self.machine, &self.repos).await;
        if report.repos.is_empty() {
            return Ok(());
        }
        match self
            .server
            .changes(self.spec.session, &self.spec.runner_token, &report)
            .await
        {
            Err(error) if error.is::<NotHeld>() => Err(error),
            Err(error) => {
                tracing::warn!(%error, "could not send the changes");
                Ok(())
            }
            Ok(()) => Ok(()),
        }
    }

    /// After a (re)connect: what the stream missed is read back from the messages, de-duplicated
    /// by message id and part id, with the permissions and the status as they are now.
    async fn rebuild(&mut self) -> Result<()> {
        for message in self.api.messages(&self.oc_session).await? {
            let items = self.tracker.message(&message);
            self.outbox.extend(items);
        }
        for request in self.api.pending_permissions().await? {
            if request.get("sessionID").and_then(serde_json::Value::as_str)
                == Some(&self.oc_session)
                && let Some(item) = permission(&request)
            {
                self.outbox.push(item);
            }
        }
        let statuses = self.api.statuses().await?;
        let busy = statuses
            .get(&self.oc_session)
            .and_then(|status| status.get("type"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|kind| kind != "idle");
        self.outbox.push(Item::Status { busy });
        self.quiet_since = (!busy).then(Instant::now);
        self.flush().await
    }

    async fn command(&mut self, envelope: Envelope) -> Result<bool> {
        match envelope.command {
            Command::Message { text } => {
                let text = match self.heads_up.take() {
                    Some(line) => format!("{line}\n\n{text}"),
                    None => text,
                };
                self.api
                    .prompt(&self.oc_session, &text, &self.spec.model)
                    .await?;
            }
            Command::AttachRepo { repo } => {
                // A failed clone is not the session's end: the agent can still work without it.
                let only = SessionSpec {
                    repos: vec![repo.clone()],
                    ..self.spec.clone()
                };
                match clone_all(&*self.machine, &self.machine.paths().work.clone(), &only).await {
                    Ok(()) => {
                        if !self.spec.repos.iter().any(|have| have.name == repo.name) {
                            let dir = repo_dir(&self.machine.paths().work, &repo.name);
                            match changes::base(&*self.machine, &dir).await {
                                Ok(base) => self.repos.push((repo.name.clone(), dir, base)),
                                Err(error) => {
                                    tracing::warn!(%error, "could not read the attached repo's base");
                                }
                            }
                            self.spec.repos.push(repo);
                        }
                    }
                    Err(error) => tracing::warn!(%error, "could not clone an attached repo"),
                }
            }
            Command::ApplyEnvironment { environment } => {
                self.apply_environment(*environment).await;
            }
            Command::Approval { permission, reply } => {
                self.api.reply(&permission, reply).await?;
            }
            Command::Abort => self.api.abort(&self.oc_session).await?,
            Command::Release { outcome } => {
                self.release(outcome).await?;
                finish(&self.done, envelope.id);
                return Ok(true);
            }
        }
        finish(&self.done, envelope.id);
        Ok(false)
    }

    /// The session moved to another version of its environment. The network is the proxy's and
    /// needs nothing here. The variables go into the file new bash processes source (the
    /// container's own environment is fixed at its start, so a process already running keeps its
    /// old one), and the setup script runs now if it changed, with the new variables, as at the
    /// start: a failure is a note and a line for the agent, never the session's end.
    async fn apply_environment(&mut self, environment: antilog_types::EnvironmentSpec) {
        let removed: Vec<String> = self
            .spec
            .environment
            .variables
            .keys()
            .filter(|name| !environment.variables.contains_key(*name))
            .cloned()
            .collect();
        let file = self.machine.paths().env.clone();
        if let Err(error) = self
            .machine
            .put(
                &file,
                env_script(&environment.variables, &removed).as_bytes(),
            )
            .await
        {
            tracing::warn!(%error, "could not write the session's variables");
        }
        let script = environment.setup_script.clone();
        let changed = script != self.spec.environment.setup_script && !script.trim().is_empty();
        let mut next = self.spec.clone();
        next.environment = environment;
        if changed {
            let root = self.machine.paths().work.clone();
            let env: Vec<(String, String)> = next
                .environment
                .variables
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect();
            if let Setup::Failed(failure) = run_script(
                &*self.machine,
                &root,
                &script,
                &env,
                &next,
                self.setup_timeout,
            )
            .await
            {
                tracing::warn!(session = %self.spec.session, exit = ?failure.exit, timed_out = failure.timed_out, "the setup script failed after an environment change");
                self.outbox.push(Item::Note {
                    key: format!("setup:{}", Uuid::now_v7()),
                    text: failure.note(self.setup_timeout),
                });
                self.heads_up = Some(failure.line());
                self.flush().await.ok();
            }
        }
        self.spec = next;
    }

    /// Exports the session for a later claim, stops OpenCode and lets the machine go.
    async fn release(&mut self, outcome: ReleaseOutcome) -> Result<()> {
        let export = match self
            .opencode
            .export(&*self.machine, &self.dir, &self.password, &self.oc_session)
            .await
        {
            Ok(export) => Some(export),
            Err(error) => {
                tracing::warn!(%error, "could not export the session");
                None
            }
        };
        self.flush().await.ok();
        self.report_changes().await.ok();
        self.stop().await;
        self.server
            .release(
                self.spec.session,
                &self.spec.runner_token,
                &Release {
                    outcome,
                    export,
                    reason: None,
                },
            )
            .await
    }

    /// Takes the machine away, with its disk.
    async fn stop(&mut self) {
        self.machine.stop().await;
    }

    /// The stream, until it ends, the session is let go (true) or the lease is lost.
    async fn serve(&mut self, commands: &mut mpsc::UnboundedReceiver<Msg>) -> Result<bool> {
        let mut events = self.api.events().await?;
        self.rebuild().await?;
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                batch = events.next() => {
                    let Some(batch) = batch? else { return Ok(false) };
                    let mut turn_ended = false;
                    for event in batch {
                        let items = self.tracker.event(&event);
                        turn_ended |= items.iter().any(|item| matches!(item, Item::Status { busy: false }));
                        if items.iter().any(|item| matches!(item, Item::Status { busy: true })) {
                            self.quiet_since = None;
                        } else if turn_ended {
                            self.quiet_since = Some(Instant::now());
                        }
                        self.outbox.extend(items);
                    }
                    self.flush().await?;
                    if turn_ended {
                        self.report_changes().await?;
                    }
                }
                message = commands.recv() => match message {
                    Some(Msg::Command(envelope)) => {
                        self.quiet_since = None;
                        if self.command(envelope).await? {
                            return Ok(true);
                        }
                    }
                    Some(Msg::Stop) | None => bail!(NotHeld),
                },
                _ = tick.tick() => {
                    self.flush().await?;
                    if !self.idle_limit.is_zero()
                        && self.quiet_since.is_some_and(|since| since.elapsed() >= self.idle_limit)
                    {
                        tracing::info!(session = %self.spec.session, "idle: exporting the session and letting the machine go");
                        self.release(ReleaseOutcome::Idle).await?;
                        return Ok(true);
                    }
                }
            }
        }
    }
}

async fn start<S: Sandbox>(
    sandbox: &Arc<S>,
    opencode: &Arc<Opencode>,
    server: &Server,
    spec: &SessionSpec,
    done: &Done,
    timing: Timing,
) -> Result<Live<S>> {
    let machine = Arc::new(sandbox.start(&SandboxSpec::new(spec)).await?);
    match prepare(&machine, opencode, server, spec, done, timing).await {
        Ok(live) => Ok(live),
        Err(error) => {
            machine.stop().await;
            Err(error)
        }
    }
}

async fn prepare<S: Sandbox>(
    machine: &Arc<S::Machine>,
    opencode: &Arc<Opencode>,
    server: &Server,
    spec: &SessionSpec,
    done: &Done,
    timing: Timing,
) -> Result<Live<S>> {
    let setup_timeout = timing.setup;
    let root = machine.paths().work.clone();
    clone_all(&**machine, &root, spec).await?;
    // The container has its variables and the repos are in: the setup script can use both.
    let mut outbox = Vec::new();
    let mut heads_up = None;
    if let Setup::Failed(failure) = run_setup(&**machine, &root, spec, setup_timeout).await {
        tracing::warn!(session = %spec.session, exit = ?failure.exit, timed_out = failure.timed_out, "the setup script failed: the session starts without it");
        outbox.push(Item::Note {
            key: format!("setup:{}", Uuid::now_v7()),
            text: failure.note(setup_timeout),
        });
        heads_up = Some(failure.line());
    }
    let file = machine.paths().env.clone();
    machine
        .put(
            &file,
            env_script(&spec.environment.variables, &[]).as_bytes(),
        )
        .await
        .with_context(|| format!("writing {}", file.display()))?;
    let config = machine.paths().config.clone();
    machine
        .put(&config, &serde_json::to_vec_pretty(&spec.opencode_config)?)
        .await
        .with_context(|| format!("writing {}", config.display()))?;
    install_skills(&**machine, server, spec).await;
    let dir = work_dir(&root, spec);
    let password = password()?;
    // A session let go earlier comes back from its export.
    let mut imported = None;
    if spec.resume
        && let Some(export) = server.export(spec.session, &spec.runner_token).await?
    {
        let file = root.join(".antilog-export.json");
        machine.put(&file, export.as_bytes()).await?;
        imported = Some(opencode.import(&**machine, &dir, &password, &file).await?);
        machine
            .exec(Exec::new([
                "rm".to_owned(),
                "-f".to_owned(),
                file.to_string_lossy().into_owned(),
            ]))
            .await
            .ok();
    }
    let mut repos = Vec::new();
    for repo in &spec.repos {
        let at = repo_dir(&root, &repo.name);
        match changes::base(&**machine, &at).await {
            Ok(base) => repos.push((repo.name.clone(), at, base)),
            Err(error) => {
                tracing::warn!(repo = %repo.name, %error, "could not read the repo's base")
            }
        }
    }
    let base = opencode.serve(&**machine, &dir, &password).await?;
    let api = Api::new(&base, &password);
    api.wait_healthy(Duration::from_secs(60)).await?;
    let oc_session = match imported {
        Some(id) => id,
        None => api.create_session().await?,
    };
    Ok(Live {
        machine: machine.clone(),
        opencode: opencode.clone(),
        server: server.clone(),
        spec: spec.clone(),
        dir,
        password,
        api,
        tracker: Tracker::new(&oc_session),
        oc_session,
        outbox,
        done: done.clone(),
        heads_up,
        repos,
        setup_timeout,
        idle_limit: timing.idle,
        quiet_since: None,
    })
}

/// Runs one session until it is let go or the lease is lost.
pub async fn drive<S: Sandbox>(
    sandbox: Arc<S>,
    opencode: Arc<Opencode>,
    server: Server,
    spec: SessionSpec,
    mut commands: mpsc::UnboundedReceiver<Msg>,
    done: Done,
    timing: Timing,
) {
    let session: SessionId = spec.session;
    let mut live = match start(&sandbox, &opencode, &server, &spec, &done, timing).await {
        Ok(live) => live,
        Err(error) => {
            let why = reason(&error, &spec);
            tracing::error!(%session, error = %why, "the session did not start");
            let release = Release {
                outcome: ReleaseOutcome::Failed,
                export: None,
                reason: Some(why),
            };
            server
                .release(session, &spec.runner_token, &release)
                .await
                .ok();
            return;
        }
    };
    let mut attempts = 0;
    loop {
        match live.serve(&mut commands).await {
            Ok(true) => return,
            Ok(false) => {
                // The stream closed: connect again and rebuild.
                attempts += 1;
            }
            Err(error) if error.is::<NotHeld>() => {
                tracing::warn!(%session, "the lease is gone: stopping");
                live.stop().await;
                return;
            }
            Err(error) => {
                attempts += 1;
                tracing::warn!(%session, error = %format!("{error:#}"), "lost OpenCode's stream");
            }
        }
        if attempts > 30 {
            tracing::error!(%session, "OpenCode does not answer: releasing as failed");
            live.stop().await;
            let release = Release {
                outcome: ReleaseOutcome::Failed,
                export: None,
                reason: Some(
                    "OpenCode stopped answering and could not be reached again.".to_owned(),
                ),
            };
            server
                .release(session, &spec.runner_token, &release)
                .await
                .ok();
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
