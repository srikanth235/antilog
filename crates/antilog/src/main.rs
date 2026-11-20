#![forbid(unsafe_code)]

mod mcp;
mod runner;

use clap::{Parser, Subcommand};
use rmcp::ServiceExt;

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
        /// Where sessions are cloned.
        #[arg(long, default_value = "/var/lib/antilog/sessions")]
        workdir: std::path::PathBuf,
        /// OpenCode's machine-wide config, which a repo's own config cannot override.
        #[arg(long, default_value = "/etc/opencode/opencode.json")]
        config_path: std::path::PathBuf,
        /// Where skill packs are unpacked.
        #[arg(long, default_value = "/opt/antilog/skills")]
        skills_dir: std::path::PathBuf,
        /// The OpenCode binary, pinned by the machine image.
        #[arg(long, default_value = "opencode")]
        opencode: String,
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
    /// Serve the Antilog tools on stdio for an agent on this machine.
    Mcp {
        /// The Antilog server, such as https://antilog.example.com.
        #[arg(long)]
        server: String,
        #[arg(long)]
        experiment: String,
        /// An access token; defaults to the ANTILOG_TOKEN environment variable.
        #[arg(long)]
        token: Option<String>,
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
            workdir,
            config_path,
            skills_dir,
            opencode,
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
            config.workdir = workdir;
            config.config_path = config_path;
            config.skills_dir = skills_dir;
            config.poll = std::time::Duration::from_secs(poll_secs.max(1));
            runner::run(
                config,
                runner::Opencode {
                    binary: opencode,
                    pure,
                    disable_project_config,
                },
            )
            .await
        }
        Some(Command::Mcp {
            server,
            experiment,
            token,
        }) => {
            let token = token
                .or_else(|| std::env::var("ANTILOG_TOKEN").ok())
                .ok_or_else(|| anyhow::anyhow!("give --token or set ANTILOG_TOKEN"))?;
            let api = mcp::Http::new(&server, &token, &experiment);
            let running = mcp::Mcp::new(api).serve(rmcp::transport::stdio()).await?;
            running.waiting().await?;
            Ok(())
        }
    }
}
