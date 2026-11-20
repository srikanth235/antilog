use std::time::Duration;

use antilog_core::store::judge::judge_store_suite;
use antilog_core::store::{ApprovalStore, ModelKeyStore, OrganizationStore, SessionStore};
use antilog_core::testing::store_suite;
use antilog_server::pg::{MIGRATOR, PgStore};
use antilog_types::{
    ApprovalStatus, AuditEntry, AuditOutcome, CardKind, ExperimentStatus, Member, MemberKind,
    Organization, OrganizationId, PolicyMode, Role, SessionStatus, SpendKind,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

/// Keeps the container alive for a test. Without Docker, `ANTILOG_TEST_POSTGRES` names a local
/// server's admin URL and each test gets a fresh database on it instead.
struct Container(
    #[expect(dead_code, reason = "held so the container lives as long as the test")]
    Option<ContainerAsync<GenericImage>>,
);

async fn database() -> anyhow::Result<(PgPool, Container)> {
    let (pool, container, _url) = database_at().await?;
    Ok((pool, container))
}

async fn connect(url: &str) -> anyhow::Result<PgPool> {
    let mut attempts = 0;
    loop {
        match PgPoolOptions::new().max_connections(4).connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(_) if attempts < 60 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

async fn database_at() -> anyhow::Result<(PgPool, Container, String)> {
    if let Ok(admin) = std::env::var("ANTILOG_TEST_POSTGRES") {
        let name = format!("antilog_test_{}", uuid::Uuid::now_v7().simple());
        let pool = connect(&admin).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("create database {name}")))
            .execute(&pool)
            .await?;
        let url = format!(
            "{}/{name}",
            admin
                .rsplit_once('/')
                .map_or(admin.as_str(), |(base, _)| base)
        );
        return Ok((connect(&url).await?, Container(None), url));
    }
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
    Ok((connect(&url).await?, Container(Some(container)), url))
}

fn sealer() -> anyhow::Result<antilog_core::sealed::Sealer> {
    antilog_core::sealed::Sealer::new(&antilog_core::secrets::Secret::new("ab".repeat(32)))
        .map_err(|e| anyhow::anyhow!("{e}"))
}

async fn migrated_store() -> anyhow::Result<(PgStore, PgPool, Container)> {
    let (pool, container) = database().await?;
    let store = PgStore::new(pool.clone()).with_sealer(sealer()?);
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
async fn postgres_store_keeps_the_card_state() -> anyhow::Result<()> {
    let (store, _pool, _container) = migrated_store().await?;
    antilog_core::seed::card_suite(&store).await;
    Ok(())
}

#[tokio::test]
async fn postgres_keeps_pull_requests_spend_cards_and_audited_models() -> anyhow::Result<()> {
    // As the role the server runs as, so row-level security is in force.
    let (store, _pool, _container) = app_store().await?;
    antilog_core::seed::wins_suite(&store, |organization| store.judge(organization)).await;
    Ok(())
}

#[tokio::test]
async fn a_pull_request_two_organizations_share_belongs_to_neither() -> anyhow::Result<()> {
    use antilog_core::judge::fixtures;
    use antilog_core::store::judge::JudgeStore;
    use antilog_core::store::{CodeHostStore, PullRequestRow};
    use antilog_types::judge::{PullRequestState, SealedExperiment};

    let (store, _pool, _container) = app_store().await?;
    let mut owners = Vec::new();
    for _ in 0..2 {
        let seed = antilog_core::seed::experiment(&store, 1_000).await;
        let judge = store.judge(seed.organization.id);
        let sealed = SealedExperiment::seal(
            seed.experiment.id,
            1,
            fixtures::definition(),
            vec![],
            fixtures::checks(),
            0,
        )
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        judge.put_sealed(&sealed).await?;
        let trial = fixtures::trial(&sealed, None, "a");
        judge.begin_trial(&trial).await?;
        owners.push((seed, trial));
    }
    let row =
        |seed: &antilog_core::seed::Seed, trial: &antilog_types::judge::Trial| PullRequestRow {
            organization: seed.organization.id,
            repo: "acme/shared".into(),
            number: 5,
            experiment: seed.experiment.id,
            version: 1,
            trial: trial.id,
            url: "https://code.example/acme/shared/pull/5".into(),
            state: PullRequestState::Open,
        };
    let (first, trial) = &owners[0];
    store
        .put_pull_requests(first.organization.id, &[row(first, trial)])
        .await?;
    assert!(store.pull_request("acme/shared", 5).await?.is_some());
    let (second, trial) = &owners[1];
    store
        .put_pull_requests(second.organization.id, &[row(second, trial)])
        .await?;
    assert_eq!(store.pull_request("acme/shared", 5).await?, None);
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
        "update session_events set text = 'x'",
        "delete from session_events",
        "truncate session_events",
        "update evaluation_templates set name = 'x'",
        "delete from evaluation_templates",
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

/// A pool that connects as a role that owns nothing and is no superuser, as the server does.
async fn app_store() -> anyhow::Result<(PgStore, PgPool, Container)> {
    let (admin, container, url) = database_at().await?;
    PgStore::new(admin.clone()).migrate().await?;
    for statement in [
        "do $$ begin if not exists (select from pg_roles where rolname = 'antilog_app') then create role antilog_app login password 'app' nosuperuser nobypassrls; end if; end $$",
        "grant usage on schema public to antilog_app",
        "grant select, insert, update, delete, truncate on all tables in schema public to antilog_app",
    ] {
        sqlx::query(statement).execute(&admin).await?;
    }
    let app_url = url.replacen("postgres:pw@", "antilog_app:app@", 1);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&app_url)
        .await?;
    Ok((
        PgStore::new(pool.clone()).with_sealer(sealer()?),
        pool,
        container,
    ))
}

#[tokio::test]
async fn the_store_suite_passes_as_a_role_without_bypass() -> anyhow::Result<()> {
    let (store, _pool, _container) = app_store().await?;
    store_suite(&store).await;
    Ok(())
}

#[tokio::test]
async fn a_query_that_forgets_its_organization_sees_nothing() -> anyhow::Result<()> {
    let (store, pool, _container) = app_store().await?;
    let a = Organization::new("a", 0);
    let b = Organization::new("b", 0);
    for organization in [&a, &b] {
        store.create_organization(organization).await?;
        let owner = Member::person(
            organization.id,
            "owner",
            antilog_types::Identity {
                provider: "fake".into(),
                subject: organization.name.clone(),
            },
            Role::Owner,
            0,
        );
        let entry = AuditEntry::new(
            organization.id,
            Some(owner.id),
            "member.add",
            owner.id,
            AuditOutcome::Done,
            0,
        );
        store.add_member(&owner, &entry).await?;
    }
    for table in [
        "organizations",
        "members",
        "audit_log",
        "projects",
        "policies",
        "access_tokens",
        "sign_in_sessions",
    ] {
        let unscoped: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("select count(*) from {table}")))
                .fetch_one(&pool)
                .await?;
        assert_eq!(unscoped, 0, "{table} without an organization set");
    }

    // The identity window shows the rows of one identity and the organizations they belong to, no more.
    let mut tx = pool.begin().await?;
    for (setting, value) in [
        ("antilog.identity_provider", "fake"),
        ("antilog.identity_subject", "a"),
    ] {
        sqlx::query("select set_config($1, $2, true)")
            .bind(setting)
            .bind(value)
            .execute(&mut *tx)
            .await?;
    }
    let members: i64 = sqlx::query_scalar("select count(*) from members")
        .fetch_one(&mut *tx)
        .await?;
    let organizations: Vec<String> = sqlx::query_scalar("select name from organizations")
        .fetch_all(&mut *tx)
        .await?;
    let projects: i64 = sqlx::query_scalar("select count(*) from projects")
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(
        (members, organizations, projects),
        (1, vec!["a".to_owned()], 0)
    );
    // Read-only: nothing changes through the window, and a write is refused.
    let changed = sqlx::query("update members set name = 'x'")
        .execute(&mut *tx)
        .await?;
    assert_eq!(changed.rows_affected(), 0);
    let insert = sqlx::query(
        "insert into projects (id, organization_id, name, description, created_at) select gen_random_uuid(), organization_id, 'p', '', now() from members",
    )
    .execute(&mut *tx)
    .await;
    assert!(insert.is_err());
    tx.rollback().await?;
    // Each identity is shown only its own membership, never another's.
    for (subject, organization) in [("a", &a), ("b", &b)] {
        let found = store
            .memberships(&antilog_types::Identity {
                provider: "fake".into(),
                subject: subject.into(),
            })
            .await?;
        assert_eq!(
            found
                .iter()
                .map(|found| (found.organization.id, found.member.name.as_str()))
                .collect::<Vec<_>>(),
            vec![(organization.id, "owner")]
        );
    }

    let mut tx = pool.begin().await?;
    sqlx::query("select set_config('antilog.organization_id', $1, true)")
        .bind(a.id.to_string())
        .execute(&mut *tx)
        .await?;
    let seen: Vec<String> = sqlx::query_scalar("select name from members")
        .fetch_all(&mut *tx)
        .await?;
    assert_eq!(seen, ["owner"], "only organization a's member");
    let audit: i64 =
        sqlx::query_scalar("select count(*) from audit_log where organization_id = $1")
            .bind(b.id.as_uuid())
            .fetch_one(&mut *tx)
            .await?;
    assert_eq!(audit, 0, "b's rows are invisible even when asked for by id");
    let forged = sqlx::query("insert into audit_log (id, organization_id, action, target, outcome, at) values (gen_random_uuid(), $1, 'x', 'y', 'done', now())")
        .bind(b.id.as_uuid())
        .execute(&mut *tx)
        .await;
    let code = forged.err().and_then(|e| {
        e.as_database_error()
            .and_then(|d| d.code().map(String::from))
    });
    assert_eq!(
        code.as_deref(),
        Some("42501"),
        "a write into another organization is refused"
    );
    drop(tx);

    let moved: u64 = {
        let mut tx = pool.begin().await?;
        sqlx::query("select set_config('antilog.organization_id', $1, true)")
            .bind(a.id.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("update members set organization_id = $1")
            .bind(b.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0)
    };
    assert_eq!(moved, 0, "a row cannot be handed to another organization");
    Ok(())
}

#[tokio::test]
async fn the_session_area_and_the_judge_see_only_their_organization() -> anyhow::Result<()> {
    let (store, pool, _container) = app_store().await?;
    let a = antilog_core::seed::experiment(&store, 1_000).await;
    let b = antilog_core::seed::experiment(&store, 1_000).await;
    let tables = [
        "experiments",
        "sessions",
        "spend",
        "seal_proposals",
        "approvals_told",
        "sealed_experiments",
        "trials",
        "receipts",
        "value_models",
        "win_events",
    ];
    judge_store_suite(
        &store.judge(a.organization.id),
        a.experiment.project_id,
        a.experiment.id,
    )
    .await;
    judge_store_suite(
        &store.judge(b.organization.id),
        b.experiment.project_id,
        b.experiment.id,
    )
    .await;
    for table in tables {
        let unscoped: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("select count(*) from {table}")))
                .fetch_one(&pool)
                .await?;
        assert_eq!(unscoped, 0, "{table} without an organization set");
    }
    let mut tx = pool.begin().await?;
    sqlx::query("select set_config('antilog.organization_id', $1, true)")
        .bind(a.organization.id.to_string())
        .execute(&mut *tx)
        .await?;
    for table in tables {
        let foreign: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "select count(*) from {table} where organization_id = $1"
        )))
        .bind(b.organization.id.as_uuid())
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(foreign, 0, "{table} shows another organization's rows");
    }
    let own: i64 = sqlx::query_scalar("select count(*) from trials")
        .fetch_one(&mut *tx)
        .await?;
    assert_eq!(own, 2, "only a's two trials");
    let forged = sqlx::query("insert into experiments (id, organization_id, project_id, goal, status, budget_cap_cents, created_by, created_at) select gen_random_uuid(), $1, project_id, 'x', 'not_sealed', 0, created_by, now() from experiments")
        .bind(b.organization.id.as_uuid())
        .execute(&mut *tx)
        .await;
    let code = forged.err().and_then(|e| {
        e.as_database_error()
            .and_then(|d| d.code().map(String::from))
    });
    assert_eq!(
        code.as_deref(),
        Some("42501"),
        "a write into another organization is refused"
    );
    Ok(())
}

#[tokio::test]
async fn postgres_judge_store_passes_the_suite() -> anyhow::Result<()> {
    let (store, _pool, _container) = migrated_store().await?;
    let seed = antilog_core::seed::experiment(&store, 1_000).await;
    judge_store_suite(
        &store.judge(seed.organization.id),
        seed.experiment.project_id,
        seed.experiment.id,
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn every_table_with_an_organization_has_row_level_security() -> anyhow::Result<()> {
    let (_store, pool, _container) = migrated_store().await?;
    let without: Vec<String> = sqlx::query_scalar(
        "select c.relname::text from pg_class c join pg_namespace n on n.oid = c.relnamespace
         where n.nspname = 'public' and c.relkind = 'r' and not c.relrowsecurity
           and c.relname <> '_sqlx_migrations'
         order by 1",
    )
    .fetch_all(&pool)
    .await?;
    assert!(without.is_empty(), "no row-level security: {without:?}");
    Ok(())
}

#[tokio::test]
async fn access_tokens_and_sign_in_sessions_are_found_by_id_alone() -> anyhow::Result<()> {
    let (store, _pool, _container) = app_store().await?;
    let organization = Organization::new("acme", 0);
    store.create_organization(&organization).await?;
    let owner = Member::person(
        organization.id,
        "ada",
        antilog_types::Identity {
            provider: "fake".into(),
            subject: "ada".into(),
        },
        Role::Owner,
        0,
    );
    let entry = |action: &str| {
        AuditEntry::new(
            organization.id,
            Some(owner.id),
            action,
            owner.id,
            AuditOutcome::Done,
            0,
        )
    };
    store.add_member(&owner, &entry("member.add")).await?;
    let issued = antilog_core::token::issue(organization.id, owner.id, "laptop", 0)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    store
        .create_access_token(&issued.record, &entry("token.create"))
        .await?;
    let found = antilog_core::token::authenticate(&store, &issued.secret).await?;
    assert_eq!(found.map(|token| token.member_id), Some(owner.id));
    let session = antilog_core::token::issue_session(organization.id, owner.id, 0)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    store
        .create_sign_in_session(&session.record, &entry("sign_in"))
        .await?;
    let live = antilog_core::token::authenticate_session(&store, &session.secret, 1).await?;
    assert_eq!(live.map(|row| row.organization_id), Some(organization.id));
    Ok(())
}

#[tokio::test]
async fn postgres_keeps_session_credentials_as_a_role_without_bypass() -> anyhow::Result<()> {
    let (store, pool, _container) = app_store().await?;
    antilog_core::gateway_suite::credentials_suite(&store, &store).await;
    // Only a hash is kept, and a token is not a column.
    let token = {
        use antilog_core::credentials::{Purpose, SessionCredentials};
        let seed = antilog_core::seed::experiment(&store, 100).await;
        let session = antilog_types::Session::new(
            seed.organization.id,
            seed.experiment.id,
            "opencode",
            antilog_core::seed::T0,
        );
        store
            .create_session(
                &session,
                &AuditEntry::new(
                    seed.organization.id,
                    Some(seed.owner.id),
                    "session.start",
                    session.id.to_string(),
                    AuditOutcome::Done,
                    antilog_core::seed::T0,
                ),
            )
            .await?;
        store
            .mint(
                session.id,
                Purpose::Model,
                jiff::SignedDuration::from_mins(5),
            )
            .await?
    };
    // The app role, with no organization set, sees no credential row at all.
    let rows: i64 = sqlx::query_scalar("select count(*) from session_credentials")
        .fetch_one(&pool)
        .await?;
    assert_eq!(rows, 0);
    assert!(!token.is_empty());
    Ok(())
}

#[tokio::test]
async fn postgres_seals_model_keys_and_scopes_them_to_their_organization() -> anyhow::Result<()> {
    let (store, pool, _container) = app_store().await?;
    antilog_core::gateway_suite::model_key_suite(&store).await;

    let seed = antilog_core::seed::experiment(&store, 100).await;
    let org = seed.organization.id;
    let audit = AuditEntry::new(
        org,
        Some(seed.owner.id),
        "model_key.set",
        "anthropic",
        AuditOutcome::Done,
        1,
    );
    store
        .put_model_key(
            org,
            antilog_types::ModelProvider::Anthropic,
            &antilog_core::secrets::Secret::new("sk-ant-plain-text-marker"),
            1,
            &audit,
        )
        .await?;
    // As the role the server runs as: with no organization set the row is invisible, and with
    // it set the stored bytes are not the key.
    let seen: i64 = sqlx::query_scalar("select count(*) from model_keys")
        .fetch_one(&pool)
        .await?;
    assert_eq!(seen, 0);
    let mut tx = pool.begin().await?;
    sqlx::query("select set_config('antilog.organization_id', $1, true)")
        .bind(org.as_uuid().to_string())
        .execute(&mut *tx)
        .await?;
    let sealed: Vec<u8> = sqlx::query_scalar("select sealed from model_keys")
        .fetch_one(&mut *tx)
        .await?;
    assert!(
        !sealed
            .windows(b"plain-text-marker".len())
            .any(|w| w == b"plain-text-marker")
    );

    // Without a master key the store refuses rather than keep a key in the clear.
    let bare = PgStore::new(pool.clone());
    assert!(
        bare.put_model_key(
            org,
            antilog_types::ModelProvider::Openai,
            &antilog_core::secrets::Secret::new("sk"),
            1,
            &audit
        )
        .await
        .is_err()
    );
    Ok(())
}
