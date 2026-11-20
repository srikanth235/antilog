#![forbid(unsafe_code)]

//! The Lambda binary: `cargo lambda build --release --arm64`.
//!
//! Connections are lazy, so a paused Aurora does not hold up the function's start; the
//! first request that needs the database waits for it to resume (about 15 seconds).
//! One connection per instance, because instances times pool size must stay under the
//! database's limit and there is no RDS Proxy.

use std::sync::Arc;
use std::time::Duration;

use antilog_aws::deps::AwsDeps;
use antilog_aws::lambda;
use antilog_core::secrets::{EnvSecrets, Secrets};
use antilog_server::api;
use antilog_server::origin::OriginGuard;
use antilog_server::pg::PgStore;
use anyhow::Context;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Lambda collects stdout into CloudWatch Logs; the time is added there.
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .init();

    let database_url = match EnvSecrets::from_env().get("database-url").await {
        Ok(secret) => secret.expose().to_owned(),
        Err(_) => std::env::var("DATABASE_URL").context("database-url is not set")?,
    };
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .idle_timeout(Duration::from_secs(30))
        .acquire_timeout(Duration::from_secs(45))
        .connect_lazy(&database_url)
        .context("reading the database url")?;
    let deps = Arc::new(AwsDeps::from_env(PgStore::new(pool)).await?);

    let backend = api::customize::Backend::from_env().await?;
    lambda::serve(
        build_router(deps, backend),
        OriginGuard::required_from_env(),
    )
    .await
    .map_err(|e| anyhow::anyhow!(e))
}

/// The server's routes plus the one operation the deploy needs: migrations, run once by
/// the deploy workflow before traffic, never at start-up (each instance would race).
/// Both are routes of the same `Router`; `/internal/` is closed to the internet.
fn build_router(
    deps: Arc<AwsDeps>,
    backend: api::customize::Backend<antilog_server::connectors::HttpConnectors>,
) -> Router {
    let operations = Router::new()
        .route("/internal/migrate", post(migrate))
        .with_state(deps.clone());
    let app = Arc::new(api::auth::App {
        deps,
        settings: api::auth::Settings::from_env(),
    });
    // Session tokens are not stored yet, so the MCP endpoint refuses every token until the
    // credentials store replaces this fake.
    let credentials =
        antilog_core::fakes::FakeCredentials::new(Arc::new(antilog_core::clock::SystemClock));
    api::router_with_reach(app, credentials, backend).merge(operations)
}

async fn migrate(State(deps): State<Arc<AwsDeps>>) -> Result<&'static str, (StatusCode, String)> {
    deps.store
        .migrate()
        .await
        .map(|()| "migrated")
        .map_err(|error| {
            tracing::error!(%error, "migration failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "migration failed".to_owned(),
            )
        })
}
