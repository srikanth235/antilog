use std::time::Duration;

use antilog_core::testing::store_suite;
use antilog_server::pg::PgStore;
use sqlx::postgres::PgPoolOptions;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

async fn migrated_store() -> anyhow::Result<(PgStore, ContainerAsync<GenericImage>)> {
    let container = GenericImage::new("postgres", "18")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "pw")
        .start()
        .await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let url = format!("postgres://postgres:pw@127.0.0.1:{port}/postgres");

    let mut attempts = 0;
    let pool = loop {
        match PgPoolOptions::new().max_connections(4).connect(&url).await {
            Ok(pool) => break pool,
            Err(_) if attempts < 60 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(error) => return Err(error.into()),
        }
    };
    let store = PgStore::new(pool);
    store.migrate().await?;
    Ok((store, container))
}

#[tokio::test]
async fn postgres_store_passes_the_store_suite() -> anyhow::Result<()> {
    let (store, _container) = migrated_store().await?;
    store_suite(&store).await;
    Ok(())
}

#[tokio::test]
async fn migrations_apply_again_without_change() -> anyhow::Result<()> {
    let (store, _container) = migrated_store().await?;
    store.migrate().await?;
    Ok(())
}
