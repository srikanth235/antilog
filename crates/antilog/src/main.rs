#![forbid(unsafe_code)]

mod runner;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "antilog", version, about = "Drive Antilog from the shell")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Poll an Antilog server's queue and run its sessions on this machine.
    Runner {
        /// The Antilog server, such as https://antilog.example.com.
        #[arg(long)]
        server: String,
        /// The runner key; defaults to the ANTILOG_RUNNER_KEY environment variable.
        #[arg(long)]
        key: Option<String>,
        /// A name for this runner.
        #[arg(long)]
        name: Option<String>,
        /// The session image each session's container starts from (`session-image/`).
        #[arg(long, default_value = "antilog-session:latest")]
        image: String,
        /// The proxy's `host:port`, which is the only way out of a session's container; defaults
        /// to the ANTILOG_PROXY_ADDR environment variable.
        #[arg(long)]
        proxy: Option<String>,
        /// The Docker command line.
        #[arg(long, default_value = "docker")]
        docker: String,
        /// The OpenCode binary inside the session image.
        #[arg(long, default_value = "opencode")]
        opencode: String,
        /// Exit after the last session is let go, so the host can be wiped. The runner serves
        /// one organization until it exits.
        #[arg(long)]
        exit_after_last_session: bool,
        /// How long a draining runner (after SIGTERM, or after its last session with
        /// --exit-after-last-session) waits for its sessions to be let go before it stops them.
        #[arg(long, default_value_t = 30)]
        drain_grace_sec: u64,
        /// How long an environment's setup script may run before it is stopped. A script that
        /// fails or times out does not fail the session; the agent and the person are told.
        #[arg(long, default_value_t = 600)]
        setup_timeout_sec: u64,
        /// The most disk one session's container may write, in GiB (0: no limit); defaults to
        /// the ANTILOG_DISK_GB environment variable, then 20. Docker enforces it only where the
        /// storage driver supports `--storage-opt size=`; elsewhere the runner says so once at
        /// start and runs without it.
        #[arg(long)]
        disk_gb: Option<u64>,
        /// How many sessions this runner holds at once, each in its own container; defaults to
        /// the ANTILOG_RUNNER_SESSIONS environment variable, then 4 bounded by the host's CPUs
        /// and memory (at most one session per CPU and per 2 GiB).
        #[arg(long)]
        sessions: Option<u32>,
        /// Seconds an idle session (waiting for a person) keeps its machine before the runner
        /// exports it and lets the machine go; a person's message starts it again (0: never).
        #[arg(long, default_value_t = 600)]
        idle_timeout_sec: u64,
        /// Seconds between polls.
        #[arg(long, default_value_t = 5)]
        poll_secs: u64,
        /// Start OpenCode with --pure: no external plugins.
        #[arg(long)]
        pure: bool,
        /// Start OpenCode with OPENCODE_DISABLE_PROJECT_CONFIG set.
        #[arg(long)]
        disable_project_config: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        None => Ok(()),
        Some(Command::Runner {
            server,
            key,
            name,
            image,
            proxy,
            docker,
            opencode,
            exit_after_last_session,
            drain_grace_sec,
            setup_timeout_sec,
            disk_gb,
            sessions,
            idle_timeout_sec,
            poll_secs,
            pure,
            disable_project_config,
        }) => {
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .init();
            let key = key
                .or_else(|| std::env::var("ANTILOG_RUNNER_KEY").ok())
                .ok_or_else(|| anyhow::anyhow!("give --key or set ANTILOG_RUNNER_KEY"))?;
            let mut config = runner::Config::new(&server, &key);
            if let Some(name) = name {
                config.name = name;
            }
            config.poll = std::time::Duration::from_secs(poll_secs.max(1));
            config.exit_after_last_session = exit_after_last_session;
            config.drain_grace = std::time::Duration::from_secs(drain_grace_sec);
            config.setup_timeout = std::time::Duration::from_secs(setup_timeout_sec.max(1));
            config.idle_timeout = std::time::Duration::from_secs(idle_timeout_sec);
            let sessions = match sessions {
                Some(count) => count,
                None => match std::env::var("ANTILOG_RUNNER_SESSIONS") {
                    Ok(text) => text.trim().parse().map_err(|_| {
                        anyhow::anyhow!("ANTILOG_RUNNER_SESSIONS is not a whole number")
                    })?,
                    Err(_) => runner::default_sessions(),
                },
            };
            if sessions == 0 {
                anyhow::bail!("--sessions must be at least 1");
            }
            config.sessions = sessions;
            let proxy_addr = proxy
                .or_else(|| std::env::var("ANTILOG_PROXY_ADDR").ok())
                .ok_or_else(|| anyhow::anyhow!("give --proxy or set ANTILOG_PROXY_ADDR"))?;
            let disk_gb = match disk_gb {
                Some(gb) => gb,
                None => match std::env::var("ANTILOG_DISK_GB") {
                    Ok(text) => text.trim().parse().map_err(|_| {
                        anyhow::anyhow!("ANTILOG_DISK_GB is not a whole number of GiB")
                    })?,
                    Err(_) => 20,
                },
            };
            let sandbox = runner::ContainerSandbox::new(
                runner::Docker { binary: docker },
                runner::ContainerConfig {
                    image,
                    proxy_addr,
                    disk_gb,
                },
            );
            sandbox.probe_disk().await;
            runner::run(
                config,
                runner::Opencode {
                    binary: opencode,
                    pure,
                    disable_project_config,
                },
                sandbox,
            )
            .await
        }
    }
}
