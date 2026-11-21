//! One session on this machine: clone, configure, start OpenCode, carry the conversation to the
//! server and the server's commands to OpenCode, and let go.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use antilog_types::{
    Command, Envelope, Item, Release, ReleaseOutcome, RepoSpec, SessionId, SessionSpec,
};
use anyhow::{Context, Result, bail};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::api::{NotHeld, Server};

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
        self.flush().await
    }

    async fn command(&mut self, envelope: Envelope) -> Result<bool> {
        match envelope.command {
            Command::Message { text } => {
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
                            self.spec.repos.push(repo);
                        }
                    }
                    Err(error) => tracing::warn!(%error, "could not clone an attached repo"),
                }
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
        self.stop().await;
        self.server
            .release(
                self.spec.session,
                &self.spec.runner_token,
                &Release { outcome, export },
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
                    for event in batch {
                        let items = self.tracker.event(&event);
                        self.outbox.extend(items);
                    }
                    self.flush().await?;
                }
                message = commands.recv() => match message {
                    Some(Msg::Command(envelope)) => {
                        if self.command(envelope).await? {
                            return Ok(true);
                        }
                    }
                    Some(Msg::Stop) | None => bail!(NotHeld),
                },
                _ = tick.tick() => self.flush().await?,
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
) -> Result<Live<S>> {
    let machine = Arc::new(sandbox.start(&SandboxSpec::new(spec)).await?);
    match prepare(&machine, opencode, server, spec, done).await {
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
) -> Result<Live<S>> {
    let root = machine.paths().work.clone();
    clone_all(&**machine, &root, spec).await?;
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
        outbox: Vec::new(),
        done: done.clone(),
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
) {
    let session: SessionId = spec.session;
    let mut live = match start(&sandbox, &opencode, &server, &spec, &done).await {
        Ok(live) => live,
        Err(error) => {
            tracing::error!(%session, error = %format!("{error:#}"), "the session did not start");
            let release = Release {
                outcome: ReleaseOutcome::Failed,
                export: None,
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
