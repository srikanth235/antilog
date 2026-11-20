#![forbid(unsafe_code)]

use std::sync::Arc;

use antilog_core::clock::SystemClock;
use antilog_core::secrets::EnvSecrets;
use antilog_proxy::{Proxy, SessionNetworks, Upstreams, egress, server};
use antilog_server::pg::PgStore;
use anyhow::Context;
use sqlx::postgres::PgPoolOptions;

fn setting(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let database_url = setting("DATABASE_URL").context("DATABASE_URL is not set")?;
    let listen = setting("ANTILOG_PROXY_LISTEN").unwrap_or_else(|| "0.0.0.0:8081".to_owned());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await
        .context("connecting to Postgres")?;
    let store = PgStore::new(pool).sealed_by(&EnvSecrets::from_env()).await;

    let upstreams = Upstreams {
        anthropic: setting("ANTILOG_PROXY_ANTHROPIC_URL")
            .unwrap_or_else(|| Upstreams::default().anthropic),
        openai: setting("ANTILOG_PROXY_OPENAI_URL").unwrap_or_else(|| Upstreams::default().openai),
        opencode: setting("ANTILOG_PROXY_OPENCODE_URL")
            .unwrap_or_else(|| Upstreams::default().opencode),
    };
    let proxy = Arc::new(Proxy {
        credentials: store.clone(),
        store,
        clock: SystemClock,
        networks: SessionNetworks::default(),
        upstreams,
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("building the HTTP client")?,
        platform_hosts: setting("ANTILOG_PROXY_PLATFORM_HOSTS")
            .map(|hosts| egress::platform_hosts(&hosts))
            .unwrap_or_default(),
        allow_private_addresses: false,
    });

    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .with_context(|| format!("binding {listen}"))?;
    tracing::info!(%listen, "proxy serving");
    server::serve(listener, proxy).await.context("serving")
}
