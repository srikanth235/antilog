//! The container backend: one container per session, driven through the `docker` command line.
//!
//! - **Network.** Each session gets its own `--internal` network, which has no route out. The
//!   runner binds a relay on that network's gateway address that carries raw TCP to the proxy, so
//!   the proxy is the only thing the container can reach; `HTTPS_PROXY` and `HTTP_PROXY` name the
//!   relay and carry the session's model token as the CONNECT login. What the proxy lets through
//!   (the environment's hosts, or only the model gateway at `NetworkLevel::None`) is the proxy's
//!   decision.
//! - **Docker for the agent.** With the sysbox runtime present on the host the container runs
//!   under it and a `dockerd` starts inside, so the agent has its own Docker without
//!   `--privileged`. Without sysbox the container has no Docker daemon; it is never started
//!   privileged.
//! - **Setup scripts** run once per environment (`id`, `version`) in a container of the session
//!   image; the result is committed as an image tagged with both and reused by later sessions. A
//!   tag not used for seven days is removed.
//! - **Secrets stay out of the process list**: values reach `docker` through its own environment
//!   (`-e NAME`), never as arguments.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use antilog_types::SessionId;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::task::JoinHandle;

use super::sandbox::{Exec, Listener, Machine, Output, Paths, Sandbox, SandboxSpec};

/// Where things are inside every session container.
const WORK: &str = "/work";
const CONFIG: &str = "/etc/opencode/opencode.json";
const SKILLS: &str = "/opt/antilog/skills";
/// The port OpenCode listens on inside the container.
const OPENCODE_PORT: u16 = 4096;
const SYSBOX: &str = "sysbox-runc";
/// A setup image not used for this long is removed.
const SETUP_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// One `docker` call. Values in `env` reach the command through the `docker` process's own
/// environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Call {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub stdin: Option<Vec<u8>>,
}

/// The `docker` command line, so a test can record what would run.
pub trait Cli: Send + Sync + 'static {
    fn run(&self, call: Call) -> impl std::future::Future<Output = Result<Output>> + Send;
}

pub struct Docker {
    pub binary: String,
}

impl Cli for Docker {
    async fn run(&self, call: Call) -> Result<Output> {
        let mut command = Command::new(&self.binary);
        command
            .args(&call.args)
            .env_remove("ANTILOG_RUNNER_KEY")
            .envs(call.env.iter().map(|(k, v)| (k, v)))
            .stdin(if call.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .with_context(|| format!("running {}", self.binary))?;
        if let (Some(bytes), Some(mut stdin)) = (call.stdin, child.stdin.take()) {
            stdin.write_all(&bytes).await?;
        }
        let output = child.wait_with_output().await?;
        Ok(Output {
            status: output.status.code().unwrap_or(1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

pub struct ContainerConfig {
    /// The session image: OpenCode pinned, git and the Docker client (`session-image/`).
    pub image: String,
    /// `host:port` of the proxy, which the per-session relay carries traffic to.
    pub proxy_addr: String,
    /// Where the runner keeps what it must remember between sessions.
    pub state_dir: PathBuf,
}

/// The setup images this runner made and when each was last used.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Images {
    used: BTreeMap<String, i64>,
}

impl Images {
    fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(bytes) = serde_json::to_vec(self) {
            std::fs::write(path, bytes).ok();
        }
    }

    fn touch(&mut self, tag: &str, now: i64) {
        self.used.insert(tag.to_owned(), now);
    }

    fn expired(&self, now: i64, ttl: Duration) -> Vec<String> {
        let ttl = i64::try_from(ttl.as_millis()).unwrap_or(i64::MAX);
        self.used
            .iter()
            .filter(|(_, used)| now.saturating_sub(**used) >= ttl)
            .map(|(tag, _)| tag.clone())
            .collect()
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// The image of an environment's setup: tagged with the environment's id and version.
fn setup_tag(id: &str, version: u32) -> String {
    let clean: String = id
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let mut tag = format!("antilog-env:{clean}-v{version}");
    if clean != id {
        // A name changed by cleaning could clash with another: tell them apart.
        use sha2::{Digest, Sha256};
        let hash: String = Sha256::digest(id.as_bytes())
            .iter()
            .take(4)
            .map(|byte| format!("{byte:02x}"))
            .collect();
        tag = format!("antilog-env:{clean}-{hash}-v{version}");
    }
    tag
}

fn encode(part: &str) -> String {
    part.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// What makes every process in the machine use the proxy, with the CONNECT login.
fn proxy_env(gateway: &str, port: u16, token: &str) -> Vec<(String, String)> {
    let url = format!("http://antilog:{}@{gateway}:{port}", encode(token));
    let mut env = Vec::new();
    for name in ["HTTPS_PROXY", "HTTP_PROXY", "https_proxy", "http_proxy"] {
        env.push((name.to_owned(), url.clone()));
    }
    for name in ["NO_PROXY", "no_proxy"] {
        env.push((name.to_owned(), "localhost,127.0.0.1".to_owned()));
    }
    env
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with("ANTILOG_RUNNER")
}

/// The environment's variables, then the proxy's, which win.
fn machine_env(spec: &SandboxSpec, proxy: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = spec
        .environment
        .variables
        .iter()
        .filter(|(name, _)| valid_name(name))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    env.retain(|(name, _)| !proxy.iter().any(|(p, _)| p == name));
    env.extend(proxy);
    env
}

fn env_flags(env: &[(String, String)]) -> Vec<String> {
    env.iter()
        .flat_map(|(name, _)| ["-e".to_owned(), name.clone()])
        .collect()
}

/// Carries raw TCP from the session's network to the proxy.
async fn relay(bind: IpAddr, upstream: String) -> Result<(u16, JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind((bind, 0)).await?;
    let port = listener.local_addr()?.port();
    let task = tokio::spawn(async move {
        while let Ok((mut inbound, _)) = listener.accept().await {
            let upstream = upstream.clone();
            tokio::spawn(async move {
                if let Ok(mut outbound) = tokio::net::TcpStream::connect(&upstream).await {
                    tokio::io::copy_bidirectional(&mut inbound, &mut outbound)
                        .await
                        .ok();
                }
            });
        }
    });
    Ok((port, task))
}

pub struct ContainerSandbox<C: Cli> {
    cli: Arc<C>,
    config: ContainerConfig,
    images: Mutex<Images>,
}

impl<C: Cli> ContainerSandbox<C> {
    pub fn new(cli: C, config: ContainerConfig) -> Self {
        let images = Images::load(&config.state_dir.join("setup-images.json"));
        Self {
            cli: Arc::new(cli),
            config,
            images: Mutex::new(images),
        }
    }

    fn images(&self) -> std::sync::MutexGuard<'_, Images> {
        self.images
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn remember(&self, edit: impl FnOnce(&mut Images)) {
        let mut images = self.images();
        edit(&mut images);
        images.save(&self.config.state_dir.join("setup-images.json"));
    }

    async fn docker<const N: usize>(&self, args: [&str; N]) -> Result<Output> {
        self.cli
            .run(Call {
                args: args.iter().map(|arg| (*arg).to_owned()).collect(),
                ..Call::default()
            })
            .await
    }

    /// Removes the setup images nobody has used for seven days.
    async fn reap(&self) {
        let expired = self.images().expired(now_ms(), SETUP_TTL);
        for tag in expired {
            match self.docker(["rmi", &tag]).await {
                Ok(out) if out.status == 0 => {
                    self.remember(|images| {
                        images.used.remove(&tag);
                    });
                    tracing::info!(%tag, "removed a setup image unused for 7 days");
                }
                Ok(out) if String::from_utf8_lossy(&out.stderr).contains("No such image") => {
                    self.remember(|images| {
                        images.used.remove(&tag);
                    });
                }
                _ => tracing::warn!(%tag, "could not remove an unused setup image"),
            }
        }
    }

    /// The session's own internal network, and the relay on its gateway.
    async fn network(&self, session: SessionId) -> Result<Net> {
        let name = format!("antilog-{session}");
        let label = format!("antilog.session={session}");
        self.docker(["network", "create", "--internal", "--label", &label, &name])
            .await?
            .ok("creating the network")?;
        let found = async {
            let out = self
                .docker([
                    "network",
                    "inspect",
                    "-f",
                    "{{(index .IPAM.Config 0).Gateway}}",
                    &name,
                ])
                .await?
                .ok("inspecting the network")?;
            let gateway = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            let address: IpAddr = gateway
                .parse()
                .with_context(|| format!("the network's gateway {gateway:?} is not an address"))?;
            let (port, task) = relay(address, self.config.proxy_addr.clone())
                .await
                .context("starting the relay to the proxy")?;
            anyhow::Ok((gateway, port, task))
        }
        .await;
        match found {
            Ok((gateway, port, task)) => Ok(Net {
                name,
                gateway,
                port,
                relay: task,
            }),
            Err(error) => {
                self.docker(["network", "rm", &name]).await.ok();
                Err(error)
            }
        }
    }

    async fn remove_net(&self, net: Net) {
        net.relay.abort();
        if let Err(error) = self.docker(["network", "rm", &net.name]).await {
            tracing::warn!(%error, "could not remove the session's network");
        }
    }

    fn limits(spec: &SandboxSpec) -> Vec<String> {
        let memory = format!("{}m", spec.environment.memory_mb);
        vec![
            "--cpus".to_owned(),
            spec.environment.cpus.to_string(),
            "--memory".to_owned(),
            memory.clone(),
            "--memory-swap".to_owned(),
            memory,
            "--pids-limit".to_owned(),
            "4096".to_owned(),
        ]
    }

    /// The image a session starts from: the environment's setup snapshot when it has a script,
    /// made now if this runner has not made it yet.
    async fn image_for(
        &self,
        spec: &SandboxSpec,
        net: &Net,
        env: &[(String, String)],
    ) -> Result<String> {
        let environment = &spec.environment;
        if environment.setup_script.trim().is_empty() {
            return Ok(self.config.image.clone());
        }
        let tag = setup_tag(&environment.id, environment.version);
        let present = self.docker(["image", "inspect", &tag]).await?.status == 0;
        if !present {
            let name = format!("antilog-setup-{}", spec.session);
            let mut args = vec![
                "run".to_owned(),
                "--name".to_owned(),
                name.clone(),
                "--network".to_owned(),
                net.name.clone(),
            ];
            args.extend(Self::limits(spec));
            args.extend(env_flags(env));
            args.extend([
                self.config.image.clone(),
                "sh".to_owned(),
                "-c".to_owned(),
                environment.setup_script.clone(),
            ]);
            let built = async {
                let out = self
                    .cli
                    .run(Call {
                        args,
                        env: env.to_vec(),
                        stdin: None,
                    })
                    .await?;
                if out.status != 0 {
                    let log = String::from_utf8_lossy(&out.stderr);
                    let tail: String = log
                        .chars()
                        .rev()
                        .take(2000)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    bail!(
                        "the environment's setup script failed ({}): {}",
                        out.status,
                        tail.trim()
                    );
                }
                self.docker(["commit", &name, &tag])
                    .await?
                    .ok("saving the setup image")?;
                anyhow::Ok(())
            }
            .await;
            self.docker(["rm", "-f", "-v", &name]).await.ok();
            built?;
        }
        self.remember(|images| images.touch(&tag, now_ms()));
        Ok(tag)
    }

    async fn boot(&self, spec: &SandboxSpec, net: &Net) -> Result<Booted> {
        let env = machine_env(spec, proxy_env(&net.gateway, net.port, &spec.proxy_token));
        let image = self.image_for(spec, net, &env).await?;
        let name = format!("antilog-{}", spec.session);
        let sysbox = self
            .docker(["info", "--format", "{{json .Runtimes}}"])
            .await
            .is_ok_and(|out| {
                out.status == 0 && String::from_utf8_lossy(&out.stdout).contains(SYSBOX)
            });
        let mut args: Vec<String> = [
            "run",
            "-d",
            "--init",
            "--name",
            &name,
            "--network",
            &net.name,
            "--label",
        ]
        .iter()
        .map(|arg| (*arg).to_owned())
        .collect();
        args.push(format!("antilog.session={}", spec.session));
        args.extend(Self::limits(spec));
        if sysbox {
            args.push(format!("--runtime={SYSBOX}"));
        }
        args.extend(env_flags(&env));
        args.extend([image, "sleep".to_owned(), "infinity".to_owned()]);
        self.cli
            .run(Call {
                args,
                env: env.clone(),
                stdin: None,
            })
            .await?
            .ok("starting the container")?;
        Ok(Booted { name, sysbox })
    }
}

struct Net {
    name: String,
    gateway: String,
    port: u16,
    relay: JoinHandle<()>,
}

struct Booted {
    name: String,
    sysbox: bool,
}

impl<C: Cli> Sandbox for ContainerSandbox<C> {
    type Machine = ContainerMachine<C>;

    async fn start(&self, spec: &SandboxSpec) -> Result<ContainerMachine<C>> {
        self.reap().await;
        let net = self.network(spec.session).await?;
        let booted = match self.boot(spec, &net).await {
            Ok(booted) => booted,
            Err(error) => {
                self.docker(["rm", "-f", "-v", &format!("antilog-{}", spec.session)])
                    .await
                    .ok();
                self.remove_net(net).await;
                return Err(error);
            }
        };
        let machine = ContainerMachine {
            cli: self.cli.clone(),
            name: booted.name,
            net: Mutex::new(Some(net)),
            paths: Paths {
                work: PathBuf::from(WORK),
                config: PathBuf::from(CONFIG),
                skills: PathBuf::from(SKILLS),
            },
            address: Mutex::new(None),
        };
        let prepared = async {
            let out = machine
                .cli
                .run(Call {
                    args: vec![
                        "inspect".into(),
                        "-f".into(),
                        "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}".into(),
                        machine.name.clone(),
                    ],
                    ..Call::default()
                })
                .await?
                .ok("finding the container's address")?;
            let ip = String::from_utf8_lossy(&out.stdout).trim().to_owned();
            if ip.parse::<IpAddr>().is_err() {
                bail!("the container has no address: {ip:?}");
            }
            *machine
                .address
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(ip);
            if booted.sysbox {
                // The agent's own Docker, inside the container's own user namespace.
                machine.spawn(Exec::new(["dockerd"])).await?;
                let mut up = false;
                for _ in 0..30 {
                    if machine
                        .exec(Exec::new(["docker", "info"]))
                        .await
                        .is_ok_and(|out| out.status == 0)
                    {
                        up = true;
                        break;
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                if !up {
                    tracing::warn!(session = %spec.session, "dockerd did not come up in the container");
                }
            } else {
                tracing::warn!(
                    session = %spec.session,
                    "the sysbox runtime is not installed on this host: the agent has no Docker"
                );
            }
            anyhow::Ok(())
        }
        .await;
        if let Err(error) = prepared {
            machine.stop().await;
            return Err(error);
        }
        Ok(machine)
    }
}

pub struct ContainerMachine<C: Cli> {
    cli: Arc<C>,
    name: String,
    net: Mutex<Option<Net>>,
    paths: Paths,
    address: Mutex<Option<String>>,
}

impl<C: Cli> ContainerMachine<C> {
    fn exec_call(&self, exec: &Exec, detached: bool) -> Call {
        let mut args = vec!["exec".to_owned()];
        if detached {
            args.push("-d".to_owned());
        } else if exec.stdin.is_some() {
            args.push("-i".to_owned());
        }
        if let Some(dir) = &exec.dir {
            args.push("-w".to_owned());
            args.push(dir.to_string_lossy().into_owned());
        }
        args.extend(env_flags(&exec.env));
        args.push(self.name.clone());
        args.extend(exec.args.iter().cloned());
        Call {
            args,
            env: exec.env.clone(),
            stdin: exec.stdin.clone(),
        }
    }
}

impl<C: Cli> Machine for ContainerMachine<C> {
    fn paths(&self) -> &Paths {
        &self.paths
    }

    async fn exec(&self, exec: Exec) -> Result<Output> {
        self.cli.run(self.exec_call(&exec, false)).await
    }

    async fn spawn(&self, exec: Exec) -> Result<()> {
        self.cli
            .run(self.exec_call(&exec, true))
            .await?
            .ok("starting a process in the container")?;
        Ok(())
    }

    async fn listener(&self) -> Result<Listener> {
        let ip = self
            .address
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .context("the container has no address yet")?;
        Ok(Listener {
            bind: "0.0.0.0".to_owned(),
            port: OPENCODE_PORT,
            base: format!("http://{ip}:{OPENCODE_PORT}"),
        })
    }

    async fn stop(&self) {
        let net = self
            .net
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let Some(net) = net else { return };
        // The container's disk goes with it: `-v` also removes its anonymous volumes.
        let removed = self
            .cli
            .run(Call {
                args: vec!["rm".into(), "-f".into(), "-v".into(), self.name.clone()],
                ..Call::default()
            })
            .await;
        if let Err(error) = removed {
            tracing::warn!(%error, "could not remove the session's container");
        }
        net.relay.abort();
        let gone = self
            .cli
            .run(Call {
                args: vec!["network".into(), "rm".into(), net.name.clone()],
                ..Call::default()
            })
            .await;
        if let Err(error) = gone {
            tracing::warn!(%error, "could not remove the session's network");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use antilog_types::EnvironmentSpec;

    use super::*;

    /// Records every call and answers from a script; a call with no scripted answer succeeds with
    /// no output.
    #[derive(Default)]
    struct Recorder {
        calls: Mutex<Vec<Call>>,
        answers: Mutex<VecDeque<(String, Output)>>,
    }

    impl Recorder {
        /// The next call whose arguments start with `prefix` gets this answer.
        fn answer(&self, prefix: &str, status: i32, stdout: &str, stderr: &str) {
            self.answers.lock().unwrap().push_back((
                prefix.to_owned(),
                Output {
                    status,
                    stdout: stdout.as_bytes().to_vec(),
                    stderr: stderr.as_bytes().to_vec(),
                },
            ));
        }

        fn lines(&self) -> Vec<String> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .map(|call| call.args.join(" "))
                .collect()
        }
    }

    impl Cli for Arc<Recorder> {
        async fn run(&self, call: Call) -> Result<Output> {
            let line = call.args.join(" ");
            self.calls.lock().unwrap().push(call);
            let mut answers = self.answers.lock().unwrap();
            let found = answers
                .iter()
                .position(|(prefix, _)| line.starts_with(prefix));
            Ok(found
                .and_then(|at| answers.remove(at))
                .map_or_else(Output::default, |(_, output)| output))
        }
    }

    fn temp() -> PathBuf {
        std::env::temp_dir().join(format!("antilog-container-{}", uuid::Uuid::now_v7()))
    }

    fn sandbox(recorder: &Arc<Recorder>, state: &Path) -> ContainerSandbox<Arc<Recorder>> {
        ContainerSandbox::new(
            recorder.clone(),
            ContainerConfig {
                image: "antilog-session:test".into(),
                proxy_addr: "127.0.0.1:9".into(),
                state_dir: state.to_path_buf(),
            },
        )
    }

    fn spec(environment: EnvironmentSpec) -> SandboxSpec {
        SandboxSpec {
            session: SessionId::new(),
            environment,
            proxy_token: "alc_tok/en".into(),
        }
    }

    /// The gateway the recorder reports for the network: where the test's relay can bind.
    fn network_answers(recorder: &Recorder, ip: &str) {
        recorder.answer("network inspect", 0, "127.0.0.1\n", "");
        recorder.answer("inspect", 0, &format!("{ip}\n"), "");
    }

    #[tokio::test]
    async fn a_container_starts_with_limits_an_internal_network_and_the_proxy() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder, &state);
        let mut environment = EnvironmentSpec {
            cpus: 3,
            memory_mb: 4096,
            ..EnvironmentSpec::default()
        };
        environment
            .variables
            .insert("SERVICE_KEY".into(), "s3cret".into());
        environment.variables.insert("bad-name".into(), "x".into());
        environment
            .variables
            .insert("HTTPS_PROXY".into(), "http://evil".into());
        let spec = spec(environment);
        let session = spec.session;
        let machine = sandbox.start(&spec).await.unwrap();

        let lines = recorder.lines();
        let calls = recorder.calls.lock().unwrap().clone();
        assert_eq!(
            lines[0],
            format!(
                "network create --internal --label antilog.session={session} antilog-{session}"
            )
        );
        assert!(lines[1].starts_with("network inspect"), "{}", lines[1]);
        let run = lines.iter().position(|l| l.starts_with("run ")).unwrap();
        let line = &lines[run];
        for want in [
            "-d --init",
            &format!("--name antilog-{session}"),
            &format!("--network antilog-{session}"),
            "--cpus 3",
            "--memory 4096m",
            "--memory-swap 4096m",
            "--pids-limit 4096",
            "-e SERVICE_KEY",
            "-e HTTPS_PROXY -e HTTP_PROXY",
            "antilog-session:test sleep infinity",
        ] {
            assert!(line.contains(want), "{want} is missing from {line}");
        }
        assert!(!line.contains("--privileged"), "{line}");
        assert!(!line.contains("--runtime"), "no sysbox here: {line}");
        assert!(!line.contains("bad-name"), "{line}");
        // No value is an argument: they travel in the docker process's environment.
        assert!(
            !line.contains("s3cret") && !line.contains("alc_tok"),
            "{line}"
        );
        let env = &calls[run].env;
        let var = |name: &str| {
            env.iter()
                .rev()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(var("SERVICE_KEY").as_deref(), Some("s3cret"));
        let proxy = var("HTTPS_PROXY").unwrap();
        assert!(
            proxy.starts_with("http://antilog:alc_tok%2Fen@127.0.0.1:"),
            "the CONNECT login is the model token: {proxy}"
        );
        assert_eq!(
            var("HTTP_PROXY"),
            Some(proxy),
            "the environment cannot replace the proxy"
        );

        // No sysbox: no dockerd; OpenCode is reached by the container's address.
        assert!(!lines.iter().any(|l| l.contains("dockerd")));
        let listener = machine.listener().await.unwrap();
        assert_eq!(listener.base, "http://172.20.0.2:4096");
        assert_eq!(listener.bind, "0.0.0.0");
        assert_eq!(
            machine.paths().config,
            Path::new("/etc/opencode/opencode.json")
        );
        assert_eq!(machine.paths().skills, Path::new("/opt/antilog/skills"));
        std::fs::remove_dir_all(&state).ok();
    }

    #[tokio::test]
    async fn commands_run_through_docker_exec_and_secrets_stay_out_of_the_arguments() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder, &state);
        let machine = sandbox
            .start(&spec(EnvironmentSpec::default()))
            .await
            .unwrap();
        let name = machine.name.clone();
        let before = recorder.lines().len();

        machine
            .exec(
                Exec::new(["git", "status"])
                    .dir(Path::new("/work/api"))
                    .env("TOKEN", "abc"),
            )
            .await
            .unwrap();
        machine
            .put(Path::new("/etc/opencode/opencode.json"), b"{}")
            .await
            .unwrap();
        machine
            .spawn(Exec::new(["opencode", "serve"]).env("OPENCODE_SERVER_PASSWORD", "pw"))
            .await
            .unwrap();
        let lines = recorder.lines();
        assert_eq!(
            lines[before],
            format!("exec -w /work/api -e TOKEN {name} git status")
        );
        assert!(
            lines[before + 1].starts_with(&format!("exec -i {name} sh -c")),
            "{}",
            lines[before + 1]
        );
        assert_eq!(
            recorder.calls.lock().unwrap()[before + 1].stdin.as_deref(),
            Some(&b"{}"[..])
        );
        assert_eq!(
            lines[before + 2],
            format!("exec -d -e OPENCODE_SERVER_PASSWORD {name} opencode serve")
        );
        assert!(
            !lines
                .iter()
                .any(|l| l.contains("pw") && l.contains("OPENCODE") && l.contains("=pw"))
        );
        std::fs::remove_dir_all(&state).ok();
    }

    #[tokio::test]
    async fn with_sysbox_the_agent_gets_docker_without_privileges() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        recorder.answer(
            "info",
            0,
            "{\"runc\":{},\"sysbox-runc\":{\"path\":\"/usr/bin/sysbox-runc\"}}",
            "",
        );
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder, &state);
        let spec = spec(EnvironmentSpec::default());
        let machine = sandbox.start(&spec).await.unwrap();
        let lines = recorder.lines();
        let run = lines.iter().find(|l| l.starts_with("run ")).unwrap();
        assert!(run.contains("--runtime=sysbox-runc"), "{run}");
        assert!(!run.contains("--privileged"), "{run}");
        let name = &machine.name;
        assert!(
            lines.contains(&format!("exec -d {name} dockerd")),
            "{lines:?}"
        );
        assert!(
            lines.contains(&format!("exec {name} docker info")),
            "{lines:?}"
        );
        std::fs::remove_dir_all(&state).ok();
    }

    #[tokio::test]
    async fn stopping_removes_the_container_its_disk_and_its_network_once() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder, &state);
        let spec = spec(EnvironmentSpec::default());
        let machine = sandbox.start(&spec).await.unwrap();
        let before = recorder.lines().len();
        machine.stop().await;
        machine.stop().await;
        assert_eq!(
            recorder.lines()[before..],
            [
                format!("rm -f -v antilog-{}", spec.session),
                format!("network rm antilog-{}", spec.session),
            ]
        );
        std::fs::remove_dir_all(&state).ok();
    }

    #[tokio::test]
    async fn a_container_that_will_not_start_leaves_nothing_behind() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        recorder.answer("network inspect", 0, "127.0.0.1\n", "");
        recorder.answer("run -d", 125, "", "no such image");
        let sandbox = sandbox(&recorder, &state);
        let spec = spec(EnvironmentSpec::default());
        let error = sandbox.start(&spec).await.err().unwrap();
        assert!(format!("{error:#}").contains("no such image"), "{error:#}");
        let lines = recorder.lines();
        assert!(lines.contains(&format!("rm -f -v antilog-{}", spec.session)));
        assert!(lines.contains(&format!("network rm antilog-{}", spec.session)));
        std::fs::remove_dir_all(&state).ok();
    }

    #[tokio::test]
    async fn a_setup_script_runs_once_per_id_and_version_and_the_image_is_reused() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        let sandbox = sandbox(&recorder, &state);
        let environment = EnvironmentSpec {
            id: "py".into(),
            version: 4,
            setup_script: "pip install numpy".into(),
            ..EnvironmentSpec::default()
        };

        // First session: no image yet (inspect fails), so the script runs and is committed.
        network_answers(&recorder, "172.20.0.2");
        recorder.answer("image inspect", 1, "", "No such image");
        let first = spec(environment.clone());
        let machine = sandbox.start(&first).await.unwrap();
        let lines = recorder.lines();
        let setup = lines
            .iter()
            .find(|l| l.starts_with("run --name antilog-setup-"))
            .expect("the script ran in a container of the session image");
        assert!(
            setup.contains("--network") && setup.contains("--cpus 2"),
            "{setup}"
        );
        assert!(
            setup.ends_with("antilog-session:test sh -c pip install numpy"),
            "{setup}"
        );
        assert!(lines.contains(&format!(
            "commit antilog-setup-{} antilog-env:py-v4",
            first.session
        )));
        assert!(lines.contains(&format!("rm -f -v antilog-setup-{}", first.session)));
        let run = lines.iter().find(|l| l.starts_with("run -d")).unwrap();
        assert!(
            run.contains("antilog-env:py-v4 sleep infinity"),
            "the session starts from the snapshot: {run}"
        );
        machine.stop().await;

        // Second session, same id and version: the image is there, the script does not run.
        let before = recorder.lines().len();
        network_answers(&recorder, "172.20.0.3");
        let second = sandbox.start(&spec_for(&environment)).await.unwrap();
        let lines = recorder.lines();
        assert!(
            !lines[before..]
                .iter()
                .any(|l| l.contains("antilog-setup-") || l.starts_with("commit")),
            "{:?}",
            &lines[before..]
        );
        assert!(
            lines[before..]
                .iter()
                .any(|l| l.contains("antilog-env:py-v4 sleep infinity"))
        );
        second.stop().await;

        // A new version is another image.
        let before = recorder.lines().len();
        network_answers(&recorder, "172.20.0.4");
        recorder.answer("image inspect", 1, "", "No such image");
        let bumped = EnvironmentSpec {
            version: 5,
            ..environment
        };
        sandbox.start(&spec_for(&bumped)).await.unwrap();
        assert!(
            recorder.lines()[before..]
                .iter()
                .any(|l| l.contains("antilog-env:py-v5") && l.starts_with("commit"))
        );
        std::fs::remove_dir_all(&state).ok();
    }

    fn spec_for(environment: &EnvironmentSpec) -> SandboxSpec {
        spec(environment.clone())
    }

    #[tokio::test]
    async fn a_failed_setup_script_fails_the_start_and_keeps_no_image() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        recorder.answer("image inspect", 1, "", "");
        recorder.answer("run --name antilog-setup-", 7, "", "boom");
        let sandbox = sandbox(&recorder, &state);
        let environment = EnvironmentSpec {
            setup_script: "false".into(),
            ..EnvironmentSpec::default()
        };
        let error = sandbox.start(&spec(environment)).await.err().unwrap();
        assert!(format!("{error:#}").contains("setup script failed (7): boom"));
        let lines = recorder.lines();
        assert!(!lines.iter().any(|l| l.starts_with("commit")));
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("rm -f -v antilog-setup-"))
        );
        assert!(!sandbox.images().used.contains_key("antilog-env:default-v1"));
        std::fs::remove_dir_all(&state).ok();
    }

    #[tokio::test]
    async fn a_setup_image_unused_for_seven_days_is_removed() {
        let state = temp();
        let recorder = Arc::new(Recorder::default());
        let sandbox = sandbox(&recorder, &state);
        let day = 24 * 60 * 60 * 1000;
        sandbox.remember(|images| {
            images.touch("antilog-env:old-v1", now_ms() - 8 * day);
            images.touch("antilog-env:fresh-v1", now_ms() - 6 * day);
        });
        // What the runner remembers survives a restart.
        let sandbox = self::sandbox(&recorder, &state);
        sandbox.reap().await;
        assert_eq!(recorder.lines(), ["rmi antilog-env:old-v1"]);
        assert_eq!(
            sandbox.images().used.keys().collect::<Vec<_>>(),
            ["antilog-env:fresh-v1"]
        );
        std::fs::remove_dir_all(&state).ok();
    }

    #[tokio::test]
    async fn the_relay_carries_bytes_to_the_proxy_and_only_there() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream = proxy.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            let (mut socket, _) = proxy.accept().await.unwrap();
            let mut buffer = [0_u8; 7];
            socket.read_exact(&mut buffer).await.unwrap();
            socket.write_all(&buffer).await.unwrap();
        });
        let (port, task) = relay("127.0.0.1".parse().unwrap(), upstream).await.unwrap();
        let mut client = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        client.write_all(b"CONNECT").await.unwrap();
        let mut answer = [0_u8; 7];
        client.read_exact(&mut answer).await.unwrap();
        assert_eq!(&answer, b"CONNECT");
        task.abort();
    }

    #[test]
    fn a_setup_tag_carries_the_id_and_the_version() {
        assert_eq!(setup_tag("default", 1), "antilog-env:default-v1");
        assert_ne!(setup_tag("Py 3", 2), setup_tag("py-3", 2));
        assert!(setup_tag("Py 3", 2).starts_with("antilog-env:py-3-"));
    }
}
