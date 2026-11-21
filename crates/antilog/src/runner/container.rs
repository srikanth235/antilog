//! The container backend: one container per session, driven through the `docker` command line.
//!
//! - **Network.** Each session gets its own `--internal` network, which has no route out. The
//!   runner binds a relay on that network's gateway address that carries raw TCP to the proxy, so
//!   the proxy is the only thing the container can reach; `HTTPS_PROXY` and `HTTP_PROXY` name the
//!   relay and carry the session's model token as the CONNECT login. What the proxy lets through
//!   (the platform hosts always, plus the environment's) is the proxy's
//!   decision.
//! - **Docker for the agent.** With the sysbox runtime present on the host the container runs
//!   under it and a `dockerd` starts inside, so the agent has its own Docker without
//!   `--privileged`. Without sysbox the container has no Docker daemon; it is never started
//!   privileged.
//! - **Variables** are set when the container starts, so every `docker exec` in it, the setup
//!   script included, sees them. The setup script itself is the session's to run, after the
//!   clones (`session.rs`): the container is always the session image.
//! - **Secrets stay out of the process list**: values reach `docker` through its own environment
//!   (`-e NAME`), never as arguments.

use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
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
    /// The most disk one container may write, in GiB; 0 for no limit. Docker enforces it only
    /// where the storage driver supports `--storage-opt size=` (`probe_disk`).
    pub disk_gb: u64,
}

/// Whether `docker run --storage-opt size=` can work with this storage driver: `driver` is
/// `docker info`'s `Driver` and `status` its `DriverStatus` as JSON. Overlay2 does only on XFS
/// (and then only with project quotas, which the first refusal at `docker run` tells).
fn supports_disk_size(driver: &str, status: &str) -> bool {
    match driver {
        "devicemapper" | "btrfs" | "zfs" | "windowsfilter" => true,
        "overlay2" => status
            .replace(' ', "")
            .contains("\"BackingFilesystem\",\"xfs\""),
        _ => false,
    }
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
    /// Containers get `--storage-opt size=`: the storage driver was found able (`probe_disk`) and
    /// has not refused it since.
    disk_limited: AtomicBool,
}

impl<C: Cli> ContainerSandbox<C> {
    pub fn new(cli: C, config: ContainerConfig) -> Self {
        Self {
            cli: Arc::new(cli),
            config,
            disk_limited: AtomicBool::new(false),
        }
    }

    /// Asks Docker once, when the runner starts, whether its storage driver can limit a
    /// container's disk. When it cannot the runner says so once and runs sessions without the
    /// limit; it never fails a session for it.
    pub async fn probe_disk(&self) {
        if self.config.disk_gb == 0 {
            tracing::info!("the container disk is not limited: the limit is 0");
            return;
        }
        let info = self
            .docker(["info", "--format", "{{.Driver}}|{{json .DriverStatus}}"])
            .await
            .ok()
            .filter(|out| out.status == 0)
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned());
        let (driver, status) = info
            .as_deref()
            .and_then(|text| text.split_once('|'))
            .unwrap_or(("unknown", ""));
        if supports_disk_size(driver, status) {
            self.disk_limited.store(true, Ordering::SeqCst);
            tracing::info!(gb = self.config.disk_gb, %driver, "each container's disk is limited");
        } else {
            tracing::warn!(%driver, "the container disk is not limited: this storage driver does not support --storage-opt size");
        }
    }

    /// `docker run` with the disk limit when there is one; Docker refusing it (an XFS without
    /// project quotas) drops the limit for this runner, once, and the run goes on without it.
    async fn run_sized(&self, args: Vec<String>, env: Vec<(String, String)>) -> Result<Output> {
        if self.disk_limited.load(Ordering::SeqCst) {
            let mut sized = args.clone();
            sized.splice(
                1..1,
                [
                    "--storage-opt".to_owned(),
                    format!("size={}g", self.config.disk_gb),
                ],
            );
            let out = self
                .cli
                .run(Call {
                    args: sized,
                    env: env.clone(),
                    stdin: None,
                })
                .await?;
            if out.status == 0 || !String::from_utf8_lossy(&out.stderr).contains("storage-opt") {
                return Ok(out);
            }
            self.disk_limited.store(false, Ordering::SeqCst);
            tracing::warn!("the container disk is not limited: Docker refused --storage-opt size");
            // The refused run may have made the container before it failed.
            if let Some(name) = args
                .iter()
                .position(|arg| arg == "--name")
                .and_then(|at| args.get(at + 1))
            {
                self.docker(["rm", "-f", "-v", name]).await.ok();
            }
        }
        self.cli
            .run(Call {
                args,
                env,
                stdin: None,
            })
            .await
    }

    async fn docker(&self, args: impl AsRef<[&str]>) -> Result<Output> {
        self.cli
            .run(Call {
                args: args.as_ref().iter().map(|arg| (*arg).to_owned()).collect(),
                ..Call::default()
            })
            .await
    }

    /// The session's own internal network, and the relay on its gateway.
    async fn network(&self, spec: &SandboxSpec) -> Result<Net> {
        let name = format!("antilog-{}", spec.name());
        let mut args = vec![
            "network".to_owned(),
            "create".to_owned(),
            "--internal".to_owned(),
            "--label".to_owned(),
            format!("antilog.session={}", spec.session),
        ];
        args.push(name.clone());
        self.cli
            .run(Call {
                args,
                ..Call::default()
            })
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

    async fn boot(&self, spec: &SandboxSpec, net: &Net) -> Result<Booted> {
        let env = machine_env(spec, proxy_env(&net.gateway, net.port, &spec.proxy_token));
        let name = format!("antilog-{}", spec.name());
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
        args.extend([
            self.config.image.clone(),
            "sleep".to_owned(),
            "infinity".to_owned(),
        ]);
        self.run_sized(args, env.clone())
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
        let net = self.network(spec).await?;
        let booted = match self.boot(spec, &net).await {
            Ok(booted) => booted,
            Err(error) => {
                self.docker(["rm", "-f", "-v", &format!("antilog-{}", spec.name())])
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
    use std::path::Path;

    use antilog_types::{EnvironmentSpec, SessionId};

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

    fn sandbox(recorder: &Arc<Recorder>) -> ContainerSandbox<Arc<Recorder>> {
        ContainerSandbox::new(
            recorder.clone(),
            ContainerConfig {
                image: "antilog-session:test".into(),
                proxy_addr: "127.0.0.1:9".into(),
                disk_gb: 20,
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

    #[test]
    fn only_some_storage_drivers_can_limit_a_container_disk() {
        let xfs = r#"[["Backing Filesystem","xfs"],["Supports d_type","true"]]"#;
        let ext4 = r#"[["Backing Filesystem","extfs"],["Supports d_type","true"]]"#;
        assert!(supports_disk_size("overlay2", xfs));
        assert!(!supports_disk_size("overlay2", ext4));
        assert!(supports_disk_size("zfs", "[]"));
        assert!(supports_disk_size("btrfs", "[]"));
        assert!(!supports_disk_size("vfs", "[]"));
    }

    /// The `docker run` line of a session started on a recorder that answered `info`.
    async fn run_line(
        info: Option<(i32, &str, &str)>,
        run_answer: Option<(i32, &str)>,
    ) -> Vec<String> {
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        if let Some((status, driver, details)) = info {
            recorder.answer("info", status, &format!("{driver}|{details}\n"), "");
        }
        if let Some((status, stderr)) = run_answer {
            recorder.answer("run ", status, "", stderr);
        }
        let sandbox = sandbox(&recorder);
        sandbox.probe_disk().await;
        let _machine = sandbox.start(&spec(EnvironmentSpec::default())).await;
        recorder.lines()
    }

    #[tokio::test]
    async fn the_disk_is_limited_only_where_the_storage_driver_can() {
        let xfs = r#"[["Backing Filesystem","xfs"]]"#;
        let lines = run_line(Some((0, "overlay2", xfs)), None).await;
        let run = lines.iter().find(|l| l.starts_with("run ")).unwrap();
        assert!(run.starts_with("run --storage-opt size=20g -d"), "{run}");

        let ext4 = r#"[["Backing Filesystem","extfs"]]"#;
        let lines = run_line(Some((0, "overlay2", ext4)), None).await;
        let run = lines.iter().find(|l| l.starts_with("run ")).unwrap();
        assert!(!run.contains("storage-opt"), "{run}");

        // Docker cannot be asked: no limit, and the session still starts.
        let lines = run_line(Some((1, "", "")), None).await;
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("run ") && !l.contains("storage-opt"))
        );
    }

    #[tokio::test]
    async fn a_refused_disk_limit_does_not_fail_the_session() {
        let xfs = r#"[["Backing Filesystem","xfs"]]"#;
        let lines = run_line(
            Some((0, "overlay2", xfs)),
            Some((
                125,
                "--storage-opt is supported only for overlay over xfs with 'pquota' mount option",
            )),
        )
        .await;
        let runs: Vec<&String> = lines.iter().filter(|l| l.starts_with("run ")).collect();
        assert_eq!(runs.len(), 2, "{lines:?}");
        assert!(runs[0].contains("storage-opt") && !runs[1].contains("storage-opt"));
    }

    #[tokio::test]
    async fn a_container_starts_with_limits_an_internal_network_and_the_proxy() {
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder);
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
    }

    #[tokio::test]
    async fn commands_run_through_docker_exec_and_secrets_stay_out_of_the_arguments() {
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder);
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
    }

    #[tokio::test]
    async fn with_sysbox_the_agent_gets_docker_without_privileges() {
        let recorder = Arc::new(Recorder::default());
        recorder.answer(
            "info",
            0,
            "{\"runc\":{},\"sysbox-runc\":{\"path\":\"/usr/bin/sysbox-runc\"}}",
            "",
        );
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder);
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
    }

    #[tokio::test]
    async fn stopping_removes_the_container_its_disk_and_its_network_once() {
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder);
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
    }

    #[tokio::test]
    async fn a_container_that_will_not_start_leaves_nothing_behind() {
        let recorder = Arc::new(Recorder::default());
        recorder.answer("network inspect", 0, "127.0.0.1\n", "");
        recorder.answer("run -d", 125, "", "no such image");
        let sandbox = sandbox(&recorder);
        let spec = spec(EnvironmentSpec::default());
        let error = sandbox.start(&spec).await.err().unwrap();
        assert!(format!("{error:#}").contains("no such image"), "{error:#}");
        let lines = recorder.lines();
        assert!(lines.contains(&format!("rm -f -v antilog-{}", spec.session)));
        assert!(lines.contains(&format!("network rm antilog-{}", spec.session)));
    }

    #[tokio::test]
    async fn the_container_is_always_the_session_image_with_the_variables_set_and_no_setup() {
        let recorder = Arc::new(Recorder::default());
        network_answers(&recorder, "172.20.0.2");
        let sandbox = sandbox(&recorder);
        let mut environment = EnvironmentSpec {
            id: "py".into(),
            version: 4,
            setup_script: "pip install numpy".into(),
            ..EnvironmentSpec::default()
        };
        environment.variables.insert("TOKEN".into(), "v".into());
        sandbox.start(&spec(environment)).await.unwrap();
        let lines = recorder.lines();
        // The script is the session's to run after the clones: no setup container, no commit, no
        // tagged image, and the container starts from the session image with the variables set.
        assert!(
            !lines
                .iter()
                .any(|l| l.contains("antilog-setup-") || l.starts_with("commit")),
            "{lines:?}"
        );
        let run = lines.iter().find(|l| l.starts_with("run -d")).unwrap();
        assert!(
            run.ends_with("antilog-session:test sleep infinity"),
            "{run}"
        );
        assert!(run.contains("-e TOKEN"), "{run}");
        assert!(!run.contains("pip install"), "{run}");
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
}
