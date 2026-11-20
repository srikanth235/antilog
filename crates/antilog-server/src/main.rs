#![forbid(unsafe_code)]

use std::sync::Arc;

use antilog_core::clock::SystemClock;
use antilog_core::fakes::FakeCredentials;
use antilog_server::api;
use antilog_server::deps::ServerDeps;
use antilog_server::origin::OriginGuard;
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
    let store = PgStore::new(pool);
    store.migrate().await.context("applying migrations")?;
    tracing::warn!("no runner or code host is built yet: serving on the fakes for both");

    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .with_context(|| format!("binding {listen}"))?;
    tracing::info!(%listen, "serving");
    let app = Arc::new(api::auth::App {
        deps: Arc::new(ServerDeps::on_fakes(store)),
        settings: api::auth::Settings::from_env(),
    });
    tracing::warn!("session tokens are not stored yet: the MCP endpoint refuses every token");
    let router = api::router_with_reach(
        app,
        FakeCredentials::new(Arc::new(SystemClock)),
        api::customize::Backend::from_env().await?,
    );
    let guard = OriginGuard::from_env();
    let router = if guard.is_on() {
        router.layer(axum::middleware::from_fn(move |request, next| {
            guard.clone().layer(request, next)
        }))
    } else {
        router
    };
    axum::serve(listener, router).await.context("serving")
}
