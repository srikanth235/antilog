use std::time::Duration;

use antilog_core::store::{ApprovalStore, OrganizationStore};
use antilog_core::testing::store_suite;
use antilog_server::pg::{MIGRATOR, PgStore};
use antilog_types::{
    ApprovalStatus, AuditEntry, AuditOutcome, CardKind, Cleanup, ExperimentStatus, Member,
    MemberKind, Organization, OrganizationId, PolicyMode, Role, SessionStatus, SpendKind,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

async fn database() -> anyhow::Result<(PgPool, ContainerAsync<GenericImage>)> {
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
    Ok((pool, container))
}

async fn migrated_store() -> anyhow::Result<(PgStore, PgPool, ContainerAsync<GenericImage>)> {
    let (pool, container) = database().await?;
    let store = PgStore::new(pool.clone());
    store.migrate().await?;
    Ok((store, pool, container))
}

#[tokio::test]
async fn postgres_store_passes_the_store_suite() -> anyhow::Result<()> {
    let (store, _pool, _container) = migrated_store().await?;
    store_suite(&store).await;
    Ok(())
}

#[tokio::test]
async fn migrations_apply_again_without_change() -> anyhow::Result<()> {
    let (store, _pool, _container) = migrated_store().await?;
    store.migrate().await?;
    Ok(())
}

#[tokio::test]
async fn migrations_apply_on_top_of_the_foundation_schema() -> anyhow::Result<()> {
    let (pool, _container) = database().await?;
    MIGRATOR.run_to(20_261_118_000_000, &pool).await?;
    let organization = Organization::new("old-acme", 1_700_000_000_000);
    sqlx::query("insert into organizations (id, name, created_at) values ($1, $2, now())")
        .bind(organization.id.as_uuid())
        .bind(&organization.name)
        .execute(&pool)
        .await?;
    let store = PgStore::new(pool);
    store.migrate().await?;
    let read = store.organization(organization.id).await?;
    assert_eq!(read.map(|row| row.monthly_cap_cents), Some(None));
    Ok(())
}

#[tokio::test]
async fn audit_log_and_spend_refuse_every_change() -> anyhow::Result<()> {
    let (store, pool, _container) = migrated_store().await?;
    let organization = Organization::new("acme", 1_700_000_000_000);
    store.create_organization(&organization).await?;
    let entry = AuditEntry::new(
        organization.id,
        None,
        "project.create",
        "p",
        AuditOutcome::Done,
        1_700_000_000_000,
    );
    store.append_audit(&entry).await?;
    for statement in [
        "update audit_log set action = 'x'",
        "delete from audit_log",
        "truncate audit_log",
        "update spend set amount_cents = 0",
        "delete from spend",
        "truncate spend",
    ] {
        let error = sqlx::query(statement).execute(&pool).await.err();
        let code = error.and_then(|error| {
            error
                .as_database_error()
                .and_then(|db| db.code().map(String::from))
        });
        assert_eq!(code.as_deref(), Some("AL005"), "{statement}");
    }
    assert_eq!(
        store.audit_log(organization.id, None, 10).await?,
        vec![entry]
    );
    Ok(())
}

#[tokio::test]
async fn every_organization_table_carries_its_organization() -> anyhow::Result<()> {
    let (_store, pool, _container) = migrated_store().await?;
    let tables: Vec<String> = sqlx::query_scalar(
        "select t.table_name::text from information_schema.tables t
         where t.table_schema = 'public' and t.table_type = 'BASE TABLE'
           and t.table_name not in ('organizations', '_sqlx_migrations')
           and not exists (select 1 from information_schema.columns c
               where c.table_schema = 'public' and c.table_name = t.table_name
                 and c.column_name = 'organization_id')",
    )
    .fetch_all(&pool)
    .await?;
    assert!(tables.is_empty(), "no organization column: {tables:?}");
    Ok(())
}

fn texts<T: Copy>(all: &[T], text: impl Fn(T) -> &'static str) -> Vec<&'static str> {
    let mut texts: Vec<_> = all.iter().map(|value| text(*value)).collect();
    texts.sort_unstable();
    texts
}

#[tokio::test]
async fn check_constraints_list_the_values_of_the_enums() -> anyhow::Result<()> {
    let (_store, pool, _container) = migrated_store().await?;
    let expected = [
        ("members_kind", texts(MemberKind::ALL, MemberKind::as_str)),
        ("members_role", texts(Role::ALL, Role::as_str)),
        ("project_roles_role", texts(Role::ALL, Role::as_str)),
        (
            "experiments_status",
            texts(ExperimentStatus::ALL, ExperimentStatus::as_str),
        ),
        (
            "sessions_status",
            texts(SessionStatus::ALL, SessionStatus::as_str),
        ),
        ("sessions_cleanup", texts(Cleanup::ALL, Cleanup::as_str)),
        ("spend_kind", texts(SpendKind::ALL, SpendKind::as_str)),
        ("policies_card", texts(CardKind::ALL, CardKind::as_str)),
        ("approvals_card", texts(CardKind::ALL, CardKind::as_str)),
        ("policies_mode", texts(PolicyMode::ALL, PolicyMode::as_str)),
        (
            "approvals_status",
            texts(ApprovalStatus::ALL, ApprovalStatus::as_str),
        ),
        (
            "audit_outcome",
            texts(AuditOutcome::ALL, AuditOutcome::as_str),
        ),
    ];
    for (name, values) in expected {
        let definition: String = sqlx::query_scalar(
            "select pg_get_constraintdef(oid) from pg_constraint where conname = $1",
        )
        .bind(name)
        .fetch_one(&pool)
        .await?;
        let mut listed: Vec<&str> = definition.split('\'').skip(1).step_by(2).collect();
        listed.sort_unstable();
        assert_eq!(listed, values, "{name}");
    }
    Ok(())
}

#[tokio::test]
async fn concurrent_demotions_cannot_orphan_a_project() -> anyhow::Result<()> {
    use antilog_core::store::{Invariant, StoreError};
    use antilog_types::{Identity, Project};

    let (store, _pool, _container) = migrated_store().await?;
    let org = OrganizationId::new();
    store
        .create_organization(&Organization {
            id: org,
            ..Organization::new("acme", 0)
        })
        .await?;
    let log = |member| {
        AuditEntry::new(
            org,
            Some(member),
            "member.add",
            member,
            AuditOutcome::Done,
            0,
        )
    };
    let mut owners = Vec::new();
    for subject in ["a", "b"] {
        let member = Member::person(
            org,
            subject,
            Identity {
                provider: "fake".into(),
                subject: subject.into(),
            },
            Role::Owner,
            0,
        );
        store.add_member(&member, &log(member.id)).await?;
        owners.push(member);
    }
    let (a, b) = (owners[0].id, owners[1].id);
    let project = Project::new(org, "p", "", 0);
    store.create_project(&project, a, &log(a)).await?;
    store
        .set_project_role(org, project.id, a, None, &log(a))
        .await?;
    let demote = |member| {
        let store = store.clone();
        async move {
            store
                .set_member_role(org, member, Role::Viewer, &log(member))
                .await
        }
    };
    let (first, second) = tokio::join!(demote(a), demote(b));
    let outcomes = [first, second];
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(
        outcomes
            .iter()
            .any(|result| matches!(result, Err(StoreError::Invariant(Invariant::LastOwner))))
    );
    Ok(())
}
