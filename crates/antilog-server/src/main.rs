#![forbid(unsafe_code)]

use std::sync::Arc;

use antilog_server::api;
use antilog_server::deps::ServerDeps;
use antilog_server::pg::PgStore;
use anyhow::Context;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::args().nth(1).as_deref() == Some("openapi") {
        println!("{}", api::openapi_json());
        return Ok(());
    }

    tracing_subscriber::fmt().init();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?;
    let listen = std::env::var("ANTILOG_LISTEN").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await
        .context("connecting to Postgres")?;
    let store = PgStore::new(pool)
        .sealed_by(&antilog_core::secrets::EnvSecrets::from_env())
        .await;
    store.migrate().await.context("applying migrations")?;
    let settings = api::auth::Settings::from_env();
    if settings.runner.is_none() {
        tracing::warn!("ANTILOG_RUNNER_KEY is not set: no runner can claim a session");
    }

    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .with_context(|| format!("binding {listen}"))?;
    tracing::info!(%listen, "serving");
    let app = Arc::new(api::auth::App {
        deps: Arc::new(ServerDeps::new(store.clone())),
        settings,
    });
    let router = api::router_with_reach(app, store, api::customize::Backend::from_env().await?);
    axum::serve(listener, router).await.context("serving")
}
