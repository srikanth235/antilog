#![forbid(unsafe_code)]

mod mcp;

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
    /// Serve the Antilog tools on stdio for an agent on this machine.
    Mcp {
        /// The Antilog server, such as https://antilog.example.com.
        #[arg(long)]
        server: String,
        #[arg(long)]
        organization: String,
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
        Some(Command::Mcp {
            server,
            organization,
            experiment,
            token,
        }) => {
            let token = token
                .or_else(|| std::env::var("ANTILOG_TOKEN").ok())
                .ok_or_else(|| anyhow::anyhow!("give --token or set ANTILOG_TOKEN"))?;
            let api = mcp::Http::new(&server, &token, &organization, &experiment);
            let running = mcp::Mcp::new(api).serve(rmcp::transport::stdio()).await?;
            running.waiting().await?;
            Ok(())
        }
    }
}
