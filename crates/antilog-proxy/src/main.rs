#![forbid(unsafe_code)]

use std::sync::Arc;

use antilog_core::clock::SystemClock;
use antilog_core::secrets::EnvSecrets;
use antilog_proxy::{NetworkAccess, Proxy, StaticEnvironment, Upstreams, server};
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

    // Environments are not stored yet, so every session gets the network the proxy is told:
    // the hosts in ANTILOG_PROXY_ALLOWED_HOSTS (comma separated) and, unless switched off with
    // ANTILOG_PROXY_PACKAGE_REGISTRIES=0, the package registries. Nothing else.
    let hosts = setting("ANTILOG_PROXY_ALLOWED_HOSTS")
        .map(|list| {
            list.split(',')
                .map(|host| host.trim().to_owned())
                .filter(|host| !host.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let environments = StaticEnvironment(NetworkAccess::Limited {
        hosts,
        package_registries: setting("ANTILOG_PROXY_PACKAGE_REGISTRIES").as_deref() != Some("0"),
    });
    let upstreams = Upstreams {
        anthropic: setting("ANTILOG_PROXY_ANTHROPIC_URL")
            .unwrap_or_else(|| Upstreams::default().anthropic),
        openai: setting("ANTILOG_PROXY_OPENAI_URL").unwrap_or_else(|| Upstreams::default().openai),
    };
    let proxy = Arc::new(Proxy {
        credentials: store.clone(),
        store,
        clock: SystemClock,
        environments,
        upstreams,
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("building the HTTP client")?,
        allow_private_addresses: false,
    });

    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .with_context(|| format!("binding {listen}"))?;
    tracing::info!(%listen, "proxy serving");
    server::serve(listener, proxy).await.context("serving")
}
