#![expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the test"
)]

use antilog_types::{
    AuditEntry, AuditOutcome, Experiment, Identity, Member, Organization, Project, Role,
};

use crate::store::Store;

pub const T0: i64 = 1_800_000_000_000;

pub struct Seed {
    pub organization: Organization,
    pub owner: Member,
    pub experiment: Experiment,
}

/// An organization with an owner, a project and one experiment with a budget in cents.
pub async fn experiment<S: Store>(store: &S, budget_cents: i64) -> Seed {
    let organization = Organization::new(format!("acme-{}", uuid::Uuid::now_v7()), T0);
    store
        .create_organization(&organization)
        .await
        .expect("an organization is created");
    let owner = Member::person(
        organization.id,
        "Grace",
        Identity {
            provider: "fake".into(),
            subject: uuid::Uuid::now_v7().to_string(),
        },
        Role::Owner,
        T0,
    );
    let log = |action: &str, target: String| {
        AuditEntry::new(
            organization.id,
            Some(owner.id),
            action,
            target,
            AuditOutcome::Done,
            T0,
        )
    };
    store
        .add_member(&owner, &log("member.add", owner.id.to_string()))
        .await
        .expect("an owner is added");
    let project = Project::new(organization.id, "checkout", "the checkout path", T0);
    store
        .create_project(
            &project,
            owner.id,
            &log("project.create", project.id.to_string()),
        )
        .await
        .expect("a project is created");
    let experiment = Experiment::new(
        organization.id,
        project.id,
        "cut p95",
        budget_cents,
        owner.id,
        T0,
    );
    store
        .create_experiment(
            &experiment,
            &log("experiment.create", experiment.id.to_string()),
        )
        .await
        .expect("an experiment is created");
    Seed {
        organization,
        owner,
        experiment,
    }
}

/// What every `RelayStore` must do: keep the cursor, write a proposal once, tell once.
pub async fn relay_suite<S: Store>(store: &S) {
    use antilog_types::{Approval, ApprovalId, ApprovalStatus, CardKind, Session};

    use crate::store::StoreError;

    let seed = experiment(store, 1_000).await;
    let org = seed.organization.id;
    let log = |action: &str| {
        AuditEntry::new(
            org,
            Some(seed.owner.id),
            action,
            "x",
            AuditOutcome::Done,
            T0,
        )
    };
    let session = Session::new(org, seed.experiment.id, "claude", "fake", T0);
    store
        .create_session(&session, &log("session.start"))
        .await
        .expect("a session is created");
    assert_eq!(
        store.relay_cursor(org, session.id).await.expect("read"),
        None
    );
    for cursor in ["3", "9"] {
        store
            .save_relay_cursor(org, session.id, cursor, T0)
            .await
            .expect("saved");
    }
    assert_eq!(
        store
            .relay_cursor(org, session.id)
            .await
            .expect("read")
            .as_deref(),
        Some("9")
    );

    let approval = Approval {
        id: ApprovalId::new(),
        organization_id: org,
        experiment_id: seed.experiment.id,
        session_id: Some(session.id),
        card: CardKind::SealExperiment,
        summary: "Seal p95".into(),
        cost_cents: None,
        status: ApprovalStatus::Pending,
        decided_by: None,
        created_at: T0,
        decided_at: None,
    };
    store
        .request_approval(&approval, &log("tool.seal"))
        .await
        .expect("requested");
    store
        .put_proposal(org, approval.id, "{}")
        .await
        .expect("stored");
    assert!(matches!(
        store.put_proposal(org, approval.id, "{}").await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store
            .proposal(org, approval.id)
            .await
            .expect("read")
            .as_deref(),
        Some("{}")
    );
    let elsewhere = experiment(store, 1).await.organization.id;
    assert_eq!(
        store.proposal(elsewhere, approval.id).await.expect("read"),
        None
    );
    assert!(store.mark_told(org, approval.id).await.expect("marked"));
    assert!(!store.mark_told(org, approval.id).await.expect("marked"));
}
