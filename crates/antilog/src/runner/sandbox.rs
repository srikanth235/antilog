//! Where a session runs. A `Sandbox` starts one machine for a session; a `Machine` runs commands
//! in it, writes files into it, starts a long-running process in it and stops it. Everything the
//! runner does to a session goes through these calls, so a microVM or VM backend can replace the
//! container one without touching the rest (`docs/V0.md`).

use std::future::Future;
use std::path::{Path, PathBuf};

use antilog_types::{EnvironmentSpec, SessionId, SessionSpec};
use anyhow::{Result, bail};

/// A command to run in the machine. `args[0]` is the program.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Exec {
    pub args: Vec<String>,
    /// The working directory, inside the machine.
    pub dir: Option<PathBuf>,
    /// Added to the machine's own environment for this command.
    pub env: Vec<(String, String)>,
    pub stdin: Option<Vec<u8>>,
}

impl Exec {
    pub fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            args: args.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn dir(mut self, dir: &Path) -> Self {
        self.dir = Some(dir.to_path_buf());
        self
    }

    #[must_use]
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.env.push((key.to_owned(), value.to_owned()));
        self
    }

    #[must_use]
    pub fn stdin(mut self, bytes: Vec<u8>) -> Self {
        self.stdin = Some(bytes);
        self
    }
}

/// What a finished command left. A command killed by a signal has status 1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Output {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl Output {
    /// The output when the command succeeded, else an error that says what failed and why.
    pub fn ok(self, what: &str) -> Result<Self> {
        if self.status != 0 {
            bail!(
                "{what} failed ({}): {}",
                self.status,
                String::from_utf8_lossy(&self.stderr).trim()
            );
        }
        Ok(self)
    }
}

/// Where things live inside the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// The session's repos are cloned under here.
    pub work: PathBuf,
    /// OpenCode's machine-wide config, which a repo's own `opencode.json` cannot override.
    pub config: PathBuf,
    /// Where the skill packs are unpacked.
    pub skills: PathBuf,
}

/// An address a process in the machine can listen on, and the URL the runner reaches it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listener {
    /// What the process binds to.
    pub bind: String,
    pub port: u16,
    pub base: String,
}

/// What a machine is made from. Built from `EnvironmentSpec::default()` and the session's own
/// variables until the claim carries a whole environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    pub session: SessionId,
    pub environment: EnvironmentSpec,
    /// The password of the machine's login at the proxy: the session's model token.
    pub proxy_token: String,
}

impl SandboxSpec {
    pub fn new(spec: &SessionSpec) -> Self {
        Self {
            session: spec.session,
            environment: EnvironmentSpec {
                variables: spec.variables.clone(),
                ..EnvironmentSpec::default()
            },
            proxy_token: spec.model_token.clone(),
        }
    }
}

/// A running machine. Dropping it must not leave processes behind, but only `stop` takes the
/// machine and its disk away.
pub trait Machine: Send + Sync + 'static {
    fn paths(&self) -> &Paths;

    /// Runs a command to its end. A command that ran and failed is an `Ok` with its status.
    fn exec(&self, exec: Exec) -> impl Future<Output = Result<Output>> + Send;

    /// Starts a process that stays up until the machine stops.
    fn spawn(&self, exec: Exec) -> impl Future<Output = Result<()>> + Send;

    /// An address to start a server on.
    fn listener(&self) -> impl Future<Output = Result<Listener>> + Send;

    /// Writes a file as root, making its folder.
    fn put(&self, path: &Path, bytes: &[u8]) -> impl Future<Output = Result<()>> + Send {
        async move {
            self.exec(
                Exec::new([
                    "sh",
                    "-c",
                    "mkdir -p \"$(dirname \"$1\")\" && cat > \"$1\"",
                    "sh",
                    &path.to_string_lossy(),
                ])
                .stdin(bytes.to_vec()),
            )
            .await?
            .ok(&format!("writing {}", path.display()))?;
            Ok(())
        }
    }

    /// Stops everything in the machine and takes it away, with its disk. Safe to call twice.
    fn stop(&self) -> impl Future<Output = ()> + Send;
}

pub trait Sandbox: Send + Sync + 'static {
    type Machine: Machine;

    fn start(&self, spec: &SandboxSpec) -> impl Future<Output = Result<Self::Machine>> + Send;
}

/// Runs a command and requires it to succeed.
pub async fn run<M: Machine>(machine: &M, exec: Exec, what: &str) -> Result<Output> {
    machine.exec(exec).await?.ok(what)
}

#[cfg(test)]
pub mod plain {
    //! The plain-process backend, for tests: the "machine" is this host, in folders the test
    //! chooses. It isolates nothing.

    use std::process::Stdio;
    use std::sync::Mutex;

    use anyhow::Context;
    use tokio::io::AsyncWriteExt;
    use tokio::process::{Child, Command};

    use super::*;

    pub struct Plain {
        /// Each session works in `<workdir>/<session>`.
        pub workdir: PathBuf,
        pub config: PathBuf,
        pub skills: PathBuf,
    }

    impl Plain {
        pub fn under(root: &Path) -> Self {
            Self {
                workdir: root.join("work"),
                config: root.join("etc/opencode/opencode.json"),
                skills: root.join("skills"),
            }
        }
    }

    pub struct PlainMachine {
        paths: Paths,
        env: Vec<(String, String)>,
        children: Mutex<Vec<Child>>,
    }

    impl PlainMachine {
        pub fn new(paths: Paths, env: Vec<(String, String)>) -> Self {
            Self {
                paths,
                env,
                children: Mutex::new(Vec::new()),
            }
        }

        fn command(&self, exec: &Exec) -> Result<Command> {
            let (program, rest) = exec.args.split_first().context("an empty command")?;
            let mut command = Command::new(program);
            command
                .args(rest)
                .env_remove("ANTILOG_RUNNER_KEY")
                .envs(self.env.iter().map(|(k, v)| (k, v)))
                .envs(exec.env.iter().map(|(k, v)| (k, v)))
                .kill_on_drop(true);
            if let Some(dir) = &exec.dir {
                command.current_dir(dir);
            }
            Ok(command)
        }
    }

    impl Sandbox for Plain {
        type Machine = PlainMachine;

        async fn start(&self, spec: &SandboxSpec) -> Result<PlainMachine> {
            Ok(PlainMachine::new(
                Paths {
                    work: self.workdir.join(spec.session.to_string()),
                    config: self.config.clone(),
                    skills: self.skills.clone(),
                },
                spec.environment
                    .variables
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ))
        }
    }

    impl Machine for PlainMachine {
        fn paths(&self) -> &Paths {
            &self.paths
        }

        async fn exec(&self, exec: Exec) -> Result<Output> {
            let mut command = self.command(&exec)?;
            command
                .stdin(if exec.stdin.is_some() {
                    Stdio::piped()
                } else {
                    Stdio::null()
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut child = command.spawn().context("starting the command")?;
            if let (Some(bytes), Some(mut stdin)) = (exec.stdin, child.stdin.take()) {
                stdin.write_all(&bytes).await?;
            }
            let output = child.wait_with_output().await?;
            Ok(Output {
                status: output.status.code().unwrap_or(1),
                stdout: output.stdout,
                stderr: output.stderr,
            })
        }

        async fn spawn(&self, exec: Exec) -> Result<()> {
            let child = self
                .command(&exec)?
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("starting the process")?;
            self.children
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(child);
            Ok(())
        }

        async fn listener(&self) -> Result<Listener> {
            let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let port = socket.local_addr()?.port();
            Ok(Listener {
                bind: "127.0.0.1".into(),
                port,
                base: format!("http://127.0.0.1:{port}"),
            })
        }

        async fn stop(&self) {
            let children: Vec<Child> = std::mem::take(
                &mut *self
                    .children
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            );
            for mut child in children {
                child.kill().await.ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use antilog_types::SessionId;

    use super::plain::Plain;
    use super::*;

    fn temp() -> PathBuf {
        std::env::temp_dir().join(format!("antilog-sandbox-{}", uuid::Uuid::now_v7()))
    }

    #[tokio::test]
    async fn the_plain_backend_runs_commands_writes_files_and_stops_what_it_started() {
        let root = temp();
        let sandbox = Plain::under(&root);
        let mut environment = EnvironmentSpec::default();
        environment
            .variables
            .insert("FROM_ENV".into(), "yes".into());
        let spec = SandboxSpec {
            session: SessionId::new(),
            environment,
            proxy_token: String::new(),
        };
        let machine = sandbox.start(&spec).await.unwrap();
        assert_eq!(
            machine.paths().work,
            root.join("work").join(spec.session.to_string())
        );

        // Output, status, the machine's variables and the command's own.
        let out = machine
            .exec(
                Exec::new([
                    "sh",
                    "-c",
                    "echo \"$FROM_ENV $EXTRA\"; echo oops >&2; exit 3",
                ])
                .env("EXTRA", "x"),
            )
            .await
            .unwrap();
        assert_eq!(out.status, 3);
        assert_eq!(String::from_utf8_lossy(&out.stdout), "yes x\n");
        assert_eq!(String::from_utf8_lossy(&out.stderr), "oops\n");
        let failed = out.ok("the thing").unwrap_err().to_string();
        assert!(failed.contains("the thing failed (3): oops"), "{failed}");

        // Stdin, the working directory, and a file written with its folders.
        let file = root.join("deep/er/file.txt");
        machine.put(&file, b"hello").await.unwrap();
        let out = machine
            .exec(
                Exec::new(["sh", "-c", "pwd; cat"])
                    .dir(&root)
                    .stdin(b"in".to_vec()),
            )
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&out.stdout).ends_with("\nin"));
        assert_eq!(std::fs::read(&file).unwrap(), b"hello");

        // A process that stays up is killed by stop, and stop can be called again.
        let pidfile = root.join("pid");
        machine
            .spawn(Exec::new([
                "sh",
                "-c",
                &format!("echo $$ > {}; sleep 60", pidfile.display()),
            ]))
            .await
            .unwrap();
        let pid = loop {
            if let Ok(text) = std::fs::read_to_string(&pidfile)
                && !text.trim().is_empty()
            {
                break text.trim().to_owned();
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        };
        machine.stop().await;
        machine.stop().await;
        for _ in 0..100 {
            if !Path::new("/proc").join(&pid).exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(
            !Path::new("/proc").join(&pid).exists(),
            "the process is gone"
        );

        let listener = machine.listener().await.unwrap();
        assert_eq!(listener.bind, "127.0.0.1");
        assert_eq!(listener.base, format!("http://127.0.0.1:{}", listener.port));
        std::fs::remove_dir_all(&root).ok();
    }
}
