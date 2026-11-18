#![expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the suite"
)]

use antilog_types::{
    Agent, Approval, ApprovalId, ApprovalStatus, AuditEntry, AuditId, AuditOutcome, CardKind,
    Cleanup, CostTier, Decision, EvaluationKind, EvaluationTemplate, EventKind, Experiment,
    ExperimentStatus, Identity, Member, MemberId, Organization, OrganizationId, Policy, PolicyMode,
    Project, ProjectRole, Role, Session, SessionStatus, Spend, SpendId, SpendKind,
};

use crate::store::{Invariant, NewEvent, SessionChange, SpendScope, SpendTotal, Store, StoreError};
use crate::token;

const T0: i64 = 1_700_000_000_000;

fn unique() -> String {
    OrganizationId::new().as_uuid().to_string()
}

fn entry(organization: OrganizationId, actor: MemberId, action: &str) -> AuditEntry {
    AuditEntry::new(
        organization,
        Some(actor),
        action,
        actor,
        AuditOutcome::Done,
        T0,
    )
}

#[track_caller]
fn refused<T>(result: Result<T, StoreError>, want: Invariant) {
    assert!(
        matches!(&result, Err(StoreError::Invariant(got)) if *got == want),
        "expected {want:?}, got {:?}",
        result.err()
    );
}

#[track_caller]
fn not_found<T>(result: Result<T, StoreError>) {
    assert!(matches!(result, Err(StoreError::NotFound)));
}

#[track_caller]
fn conflict<T>(result: Result<T, StoreError>) {
    assert!(matches!(result, Err(StoreError::Conflict)));
}

struct World {
    organization: Organization,
    owner: Member,
    project: Project,
}

impl World {
    fn id(&self) -> OrganizationId {
        self.organization.id
    }

    fn log(&self, action: &str) -> AuditEntry {
        entry(self.id(), self.owner.id, action)
    }
}

async fn person<S: Store>(store: &S, organization: OrganizationId, role: Role) -> Member {
    let member = Member::person(
        organization,
        "Grace",
        Identity {
            provider: "fake".into(),
            subject: unique(),
        },
        role,
        T0,
    );
    store
        .add_member(&member, &entry(organization, member.id, "member.add"))
        .await
        .expect("a member is added");
    member
}

async fn world<S: Store>(store: &S) -> World {
    let organization = Organization::new(format!("acme-{}", unique()), T0);
    store
        .create_organization(&organization)
        .await
        .expect("an organization is created");
    let owner = person(store, organization.id, Role::Owner).await;
    let project = Project::new(organization.id, "checkout", "the checkout path", T0);
    store
        .create_project(
            &project,
            owner.id,
            &entry(organization.id, owner.id, "project.create"),
        )
        .await
        .expect("a project is created");
    World {
        organization,
        owner,
        project,
    }
}

async fn experiment<S: Store>(
    store: &S,
    world: &World,
    cents: i64,
) -> Result<Experiment, StoreError> {
    let experiment = Experiment::new(
        world.id(),
        world.project.id,
        "cut p95",
        cents,
        world.owner.id,
        T0,
    );
    store
        .create_experiment(&experiment, &world.log("experiment.create"))
        .await
        .map(|()| experiment)
}

async fn logged<S: Store>(store: &S, organization: OrganizationId, id: AuditId) -> bool {
    store
        .audit_log(organization, None, 1000)
        .await
        .expect("the log is read")
        .iter()
        .any(|row| row.id == id)
}

pub async fn store_suite<S: Store>(store: &S) {
    store.ping().await.expect("the store answers a ping");
    organizations(store).await;
    members_and_roles(store).await;
    memberships(store).await;
    tokens(store).await;
    owners_and_sign_ins(store).await;
    budgets_and_spend(store).await;
    experiments_and_sessions(store).await;
    agents(store).await;
    evaluations(store).await;
    approvals(store).await;
    audit_log(store).await;
}

async fn organizations<S: Store>(store: &S) {
    let organization = Organization::new(format!("acme-{}", unique()), T0);
    store
        .create_organization(&organization)
        .await
        .expect("a new organization is created");
    assert_eq!(
        store.organization(organization.id).await.expect("read"),
        Some(organization.clone())
    );
    conflict(
        store
            .create_organization(&Organization {
                id: OrganizationId::new(),
                ..organization.clone()
            })
            .await,
    );
    assert_eq!(
        store
            .organization(OrganizationId::new())
            .await
            .expect("read"),
        None
    );

    let owner = person(store, organization.id, Role::Owner).await;
    store
        .set_organization_limits(
            organization.id,
            Some(10_000),
            Some(2_000),
            &entry(organization.id, owner.id, "budget.set"),
        )
        .await
        .expect("limits are set");
    let read = store
        .organization(organization.id)
        .await
        .expect("read")
        .expect("exists");
    assert_eq!(
        (read.monthly_cap_cents, read.default_budget_cents),
        (Some(10_000), Some(2_000))
    );
    refused(
        store
            .set_organization_limits(
                organization.id,
                Some(1_000),
                Some(2_000),
                &entry(organization.id, owner.id, "budget.set"),
            )
            .await,
        Invariant::OverCap,
    );
}

async fn members_and_roles<S: Store>(store: &S) {
    let world = world(store).await;
    let (org, project) = (world.id(), world.project.id);
    let identity = Identity {
        provider: "fake".into(),
        subject: unique(),
    };

    let ada = Member::person(org, "Ada", identity.clone(), Role::Editor, T0);
    store
        .add_member(&ada, &world.log("member.add"))
        .await
        .expect("added");
    conflict(
        store
            .add_member(
                &Member::person(org, "Ada again", identity.clone(), Role::Viewer, T0),
                &world.log("member.add"),
            )
            .await,
    );
    let other = world_org(store).await;
    let elsewhere = Member::person(other, "Ada elsewhere", identity.clone(), Role::Owner, T0);
    store
        .add_member(&elsewhere, &entry(other, elsewhere.id, "member.add"))
        .await
        .expect("the same identity may join another organization");
    assert_eq!(
        store
            .member_by_identity(org, &identity)
            .await
            .expect("read"),
        Some(ada.clone())
    );
    assert_eq!(store.member(other, ada.id).await.expect("read"), None);
    let service = Member::service(org, "ci", Role::Viewer, T0);
    store
        .add_member(&service, &world.log("member.add"))
        .await
        .expect("a service account has no identity");
    let listed = store.members(org).await.expect("read");
    assert_eq!(listed.len(), 3);
    assert!(listed.contains(&service));
    not_found(
        store
            .add_member(
                &Member::person(OrganizationId::new(), "x", identity, Role::Viewer, T0),
                &world.log("member.add"),
            )
            .await,
    );

    let owner = world.owner.id;
    let role = |member| store.role_in_project(org, project, member);
    assert_eq!(role(owner).await.expect("read"), Some(Role::Owner));
    assert_eq!(role(ada.id).await.expect("read"), Some(Role::Editor));
    assert_eq!(role(MemberId::new()).await.expect("read"), None);
    assert_eq!(
        store
            .role_in_project(org, antilog_types::ProjectId::new(), owner)
            .await
            .expect("read"),
        None
    );

    let backup = person(store, org, Role::Owner).await;
    store
        .set_project_role(
            org,
            project,
            backup.id,
            Some(Role::Viewer),
            &world.log("project.role"),
        )
        .await
        .expect("a second organization owner who is a viewer here");
    refused(
        store
            .set_member_role(org, backup.id, Role::Viewer, &world.log("member.role"))
            .await
            .and(
                store
                    .set_member_role(org, owner, Role::Viewer, &world.log("member.role"))
                    .await,
            ),
        Invariant::LastOwner,
    );
    store
        .set_member_role(org, backup.id, Role::Owner, &world.log("member.role"))
        .await
        .expect("unchanged");
    store
        .set_member_role(org, owner, Role::Viewer, &world.log("member.role"))
        .await
        .expect("the creator stays the project's owner by its own role");
    assert_eq!(role(owner).await.expect("read"), Some(Role::Owner));
    let roles = store.project_roles(org, project).await.expect("read");
    assert_eq!(roles.len(), 2);
    assert!(roles.contains(&ProjectRole {
        member_id: owner,
        role: Role::Owner
    }));

    let refused_entry = world.log("project.role");
    refused(
        store
            .set_project_role(org, project, owner, None, &refused_entry)
            .await,
        Invariant::LastOwner,
    );
    refused(
        store
            .set_project_role(org, project, owner, Some(Role::Editor), &refused_entry)
            .await,
        Invariant::LastOwner,
    );
    assert_eq!(role(owner).await.expect("read"), Some(Role::Owner));
    assert!(!logged(store, org, refused_entry.id).await);

    store
        .set_project_role(
            org,
            project,
            ada.id,
            Some(Role::Owner),
            &world.log("project.role"),
        )
        .await
        .expect("a second owner");
    store
        .set_project_role(org, project, owner, None, &world.log("project.role"))
        .await
        .expect("now the first may step back");
    assert_eq!(role(owner).await.expect("read"), Some(Role::Viewer));
    refused(
        store
            .set_project_role(org, project, ada.id, Some(Role::Viewer), &refused_entry)
            .await,
        Invariant::LastOwner,
    );
    store
        .set_member_role(org, owner, Role::Owner, &world.log("member.role"))
        .await
        .expect("an organization owner is an owner in every project");
    assert_eq!(role(owner).await.expect("read"), Some(Role::Owner));
    store
        .set_project_role(
            org,
            project,
            ada.id,
            Some(Role::Viewer),
            &world.log("project.role"),
        )
        .await
        .expect("a project may rank someone lower than the organization does");
    assert_eq!(role(ada.id).await.expect("read"), Some(Role::Viewer));

    not_found(
        store
            .set_project_role(
                org,
                project,
                MemberId::new(),
                Some(Role::Editor),
                &refused_entry,
            )
            .await,
    );
    not_found(
        store
            .set_member_role(other, ada.id, Role::Viewer, &refused_entry)
            .await,
    );
    let lonely = Project::new(org, "lonely", "", T0);
    not_found(
        store
            .create_project(&lonely, MemberId::new(), &world.log("project.create"))
            .await,
    );
    conflict(
        store
            .create_project(
                &Project::new(org, "checkout", "", T0),
                owner,
                &world.log("project.create"),
            )
            .await,
    );
    assert_eq!(store.projects(org).await.expect("read").len(), 1);
    assert_eq!(
        store.project(other, project).await.expect("read"),
        None,
        "a project is not reachable through another organization"
    );
}

/// One identity in two organizations is found in both, oldest membership first; a stranger in none.
async fn memberships<S: Store>(store: &S) {
    let identity = Identity {
        provider: "fake".into(),
        subject: unique(),
    };
    let mut joined = Vec::new();
    for (name, at) in [("later", T0 + 5), ("earlier", T0)] {
        let organization = Organization::new(format!("{name}-{}", unique()), at);
        store
            .create_organization(&organization)
            .await
            .expect("created");
        let member = Member::person(organization.id, "Grace", identity.clone(), Role::Owner, at);
        store
            .add_member(&member, &entry(organization.id, member.id, "member.add"))
            .await
            .expect("added");
        joined.push((organization, member));
    }
    let found = store.memberships(&identity).await.expect("looked up");
    let names: Vec<_> = found
        .iter()
        .map(|found| found.organization.name.clone())
        .collect();
    assert_eq!(
        names,
        joined
            .iter()
            .rev()
            .map(|(organization, _)| organization.name.clone())
            .collect::<Vec<_>>()
    );
    assert!(found.iter().all(
        |found| found.member.organization_id == found.organization.id
            && found.member.identity.as_ref() == Some(&identity)
    ));
    let stranger = Identity {
        provider: "fake".into(),
        subject: unique(),
    };
    assert!(store.memberships(&stranger).await.expect("none").is_empty());
}

async fn world_org<S: Store>(store: &S) -> OrganizationId {
    let organization = Organization::new(format!("other-{}", unique()), T0);
    store
        .create_organization(&organization)
        .await
        .expect("created");
    organization.id
}

async fn tokens<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let issued = token::issue(org, world.owner.id, "laptop", T0).expect("random");
    assert!(!format!("{issued:?}").contains(&issued.secret));
    assert!(!issued.secret.contains(&format!("{:?}", issued.record.hash)));
    store
        .create_access_token(&issued.record, &world.log("token.create"))
        .await
        .expect("stored");
    conflict(
        store
            .create_access_token(&issued.record, &world.log("token.create"))
            .await,
    );
    let found = token::authenticate(store, &issued.secret)
        .await
        .expect("read")
        .expect("a genuine secret authenticates");
    assert_eq!(found, issued.record.token);
    assert_eq!(found.organization_id, org);

    let wrong = format!("{}0", issued.secret);
    for bad in [wrong.as_str(), "", "alt_", "alt_x_y", "nonsense"] {
        assert_eq!(token::authenticate(store, bad).await.expect("read"), None);
    }
    let other = token::issue(org, world.owner.id, "other", T0).expect("random");
    let forged = other.secret.replacen(
        &other.record.token.id.as_uuid().simple().to_string(),
        &issued.record.token.id.as_uuid().simple().to_string(),
        1,
    );
    assert_eq!(
        token::authenticate(store, &forged).await.expect("read"),
        None
    );

    not_found(
        store
            .create_access_token(
                &token::issue(org, MemberId::new(), "ghost", T0)
                    .expect("random")
                    .record,
                &world.log("token.create"),
            )
            .await,
    );
    not_found(
        store
            .revoke_access_token(
                OrganizationId::new(),
                issued.record.token.id,
                T0,
                &world.log("token.revoke"),
            )
            .await,
    );
    store
        .revoke_access_token(
            org,
            issued.record.token.id,
            T0 + 5,
            &world.log("token.revoke"),
        )
        .await
        .expect("revoked");
    store
        .revoke_access_token(
            org,
            issued.record.token.id,
            T0 + 9,
            &world.log("token.revoke"),
        )
        .await
        .expect("revoking again changes nothing");
    assert_eq!(
        token::authenticate(store, &issued.secret)
            .await
            .expect("read"),
        None
    );
    let listed = store
        .access_tokens(org, world.owner.id)
        .await
        .expect("read");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed.first().and_then(|row| row.revoked_at), Some(T0 + 5));
}

async fn budgets_and_spend<S: Store>(store: &S) {
    let world = world(store).await;
    let (org, project) = (world.id(), world.project.id);
    let limits = async |cap, default| {
        store
            .set_project_limits(org, project, cap, default, &world.log("budget.set"))
            .await
    };

    store
        .set_organization_limits(org, Some(10_000), Some(2_000), &world.log("budget.set"))
        .await
        .expect("set");
    refused(limits(Some(20_000), None).await, Invariant::OverCap);
    refused(limits(Some(5_000), Some(6_000)).await, Invariant::OverCap);
    limits(Some(5_000), Some(1_000)).await.expect("inside");
    refused(
        store
            .set_organization_limits(org, Some(4_000), None, &world.log("budget.set"))
            .await,
        Invariant::OverCap,
    );

    let first = experiment(store, &world, 3_000).await.expect("inside");
    refused(experiment(store, &world, 6_000).await, Invariant::OverCap);
    assert_eq!(
        store.experiments(org, project).await.expect("read").len(),
        1
    );
    let set = async |cents| {
        store
            .set_experiment_budget(org, first.id, cents, &world.log("budget.set"))
            .await
    };
    set(5_000).await.expect("up to the cap");
    refused(set(5_001).await, Invariant::OverCap);
    assert_eq!(
        store
            .experiment(org, first.id)
            .await
            .expect("read")
            .map(|row| row.budget_cap_cents),
        Some(5_000)
    );
    refused(limits(Some(4_000), None).await, Invariant::OverCap);
    limits(None, None).await.expect("a project may have no cap");
    set(10_000)
        .await
        .expect("then the organization's cap holds");
    refused(set(10_001).await, Invariant::OverCap);
    not_found(
        store
            .set_experiment_budget(
                org,
                antilog_types::ExperimentId::new(),
                1,
                &world.log("budget.set"),
            )
            .await,
    );

    store
        .set_organization_limits(org, None, None, &world.log("budget.set"))
        .await
        .expect("lifting the organization cap");
    let second_project = Project::new(org, "search", "", T0);
    store
        .create_project(
            &second_project,
            world.owner.id,
            &world.log("project.create"),
        )
        .await
        .expect("created");
    let second = Experiment::new(
        org,
        second_project.id,
        "cut build time",
        100,
        world.owner.id,
        T0,
    );
    store
        .create_experiment(&second, &world.log("experiment.create"))
        .await
        .expect("created");
    let lead = Session::new(org, first.id, "claude", "agentcore", T0);
    let elsewhere = Session::new(org, second.id, "claude", "agentcore", T0);
    for session in [&lead, &elsewhere] {
        store
            .create_session(session, &world.log("session.create"))
            .await
            .expect("created");
    }
    let spend = |session: &Session, kind, cents, tokens, at| Spend {
        id: SpendId::new(),
        organization_id: org,
        session_id: session.id,
        kind,
        amount_cents: cents,
        tokens,
        occurred_at: at,
    };
    for row in [
        spend(&lead, SpendKind::Tokens, 40, 900, T0 + 10),
        spend(&lead, SpendKind::Judge, 25, 0, T0 + 20),
        spend(&elsewhere, SpendKind::Sandbox, 7, 0, T0 + 20),
    ] {
        store.record_spend(&row).await.expect("recorded");
    }
    let row = spend(&lead, SpendKind::Tokens, 1, 1, T0 + 30);
    store.record_spend(&row).await.expect("recorded");
    conflict(store.record_spend(&row).await);
    let mut stray = spend(&lead, SpendKind::Tokens, 1, 1, T0);
    stray.organization_id = world_org(store).await;
    not_found(store.record_spend(&stray).await);

    let total = async |scope, from, to| store.spend(org, scope, from, to).await.expect("read");
    let sum = |cents, tokens| SpendTotal { cents, tokens };
    assert_eq!(
        total(SpendScope::Experiment(first.id), T0, T0 + 100).await,
        sum(66, 901)
    );
    assert_eq!(
        total(SpendScope::Experiment(first.id), T0 + 10, T0 + 30).await,
        sum(65, 900)
    );
    assert_eq!(
        total(SpendScope::Project(second_project.id), T0, T0 + 100).await,
        sum(7, 0)
    );
    assert_eq!(
        total(SpendScope::Project(project), T0, T0 + 100).await,
        sum(66, 901)
    );
    assert_eq!(
        total(SpendScope::Organization, T0, T0 + 100).await,
        sum(73, 901)
    );
    assert_eq!(
        total(SpendScope::Organization, T0 + 31, T0 + 100).await,
        sum(0, 0)
    );
    let between = store
        .spend_lines_between(org, T0 + 15, T0 + 25)
        .await
        .expect("read");
    assert_eq!(
        between
            .iter()
            .map(|line| (line.session, line.kind, line.cents))
            .collect::<Vec<_>>()
            .len(),
        2
    );
    assert_eq!(between.iter().map(|line| line.cents).sum::<i64>(), 32);
    assert!(
        store
            .spend_lines_between(org, T0 + 31, T0 + 100)
            .await
            .expect("read")
            .is_empty()
    );
    assert_eq!(
        store
            .spend(
                world_org(store).await,
                SpendScope::Organization,
                T0,
                T0 + 100
            )
            .await
            .expect("read"),
        sum(0, 0)
    );

    let lines = store
        .spend_lines(org, Some(first.id))
        .await
        .expect("an experiment's lines");
    let mut got: Vec<_> = lines
        .iter()
        .map(|line| (line.session, line.kind.as_str(), line.cents, line.tokens))
        .collect();
    got.sort();
    let mut want = vec![(lead.id, "judge", 25, 0), (lead.id, "tokens", 41, 901)];
    want.sort();
    assert_eq!(got, want);
    assert!(lines.iter().all(|line| line.experiment == first.id));
    assert_eq!(
        store.spend_lines(org, None).await.expect("all").len(),
        3,
        "the other experiment's sandbox spend joins the rest"
    );
    assert!(
        store
            .spend_lines(world_org(store).await, None)
            .await
            .expect("another organization")
            .is_empty()
    );
}

async fn experiments_and_sessions<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let experiment = experiment(store, &world, 1_000).await.expect("created");
    assert_eq!(experiment.status, ExperimentStatus::NotSealed);
    assert_eq!(
        store.experiment(org, experiment.id).await.expect("read"),
        Some(experiment.clone())
    );
    assert_eq!(
        store
            .experiment(world_org(store).await, experiment.id)
            .await
            .expect("read"),
        None
    );
    let seal = async |status| {
        store
            .set_experiment_status(org, experiment.id, status, &world.log("experiment.seal"))
            .await
    };
    seal(ExperimentStatus::Sealed).await.expect("sealed");
    refused(
        seal(ExperimentStatus::NotSealed).await,
        Invariant::IllegalTransition,
    );
    not_found(
        store
            .set_experiment_status(
                org,
                antilog_types::ExperimentId::new(),
                ExperimentStatus::Sealed,
                &world.log("x"),
            )
            .await,
    );
    not_found(
        store
            .create_experiment(
                &Experiment::new(org, world.project.id, "g", 1, MemberId::new(), T0),
                &world.log("experiment.create"),
            )
            .await,
    );
    not_found(
        store
            .create_experiment(
                &Experiment::new(
                    org,
                    antilog_types::ProjectId::new(),
                    "g",
                    1,
                    world.owner.id,
                    T0,
                ),
                &world.log("experiment.create"),
            )
            .await,
    );

    let lead = Session::new(org, experiment.id, "claude", "agentcore", T0);
    let create = async |session: &Session| {
        store
            .create_session(session, &world.log("session.create"))
            .await
    };
    create(&lead).await.expect("created");
    let mut child = Session::new(org, experiment.id, "codex", "openai", T0 + 1);
    child.parent = Some(lead.id);
    create(&child).await.expect("a spawned session");
    let mut grandchild = Session::new(org, experiment.id, "codex", "openai", T0 + 2);
    grandchild.parent = Some(child.id);
    refused(create(&grandchild).await, Invariant::SpawnedCannotSpawn);
    let other = self::experiment(store, &world, 1_000)
        .await
        .expect("created");
    let mut stray = Session::new(org, other.id, "codex", "openai", T0);
    stray.parent = Some(lead.id);
    not_found(create(&stray).await);
    conflict(create(&lead).await);
    assert_eq!(
        store.sessions(org, experiment.id).await.expect("read"),
        vec![lead.clone(), child.clone()]
    );

    let update = async |change: SessionChange, at, entry: Option<&AuditEntry>| {
        store.update_session(org, lead.id, &change, at, entry).await
    };
    let waiting = update(
        SessionChange {
            status: Some(SessionStatus::WaitingForYou),
            platform_handle: Some("handle-1".into()),
            ..SessionChange::default()
        },
        T0 + 5,
        None,
    )
    .await
    .expect("waiting");
    assert_eq!(waiting.status, SessionStatus::WaitingForYou);
    assert_eq!(waiting.platform_handle.as_deref(), Some("handle-1"));
    assert_eq!(waiting.updated_at, T0 + 5);
    update(
        SessionChange {
            status: Some(SessionStatus::Working),
            ..SessionChange::default()
        },
        T0 + 6,
        None,
    )
    .await
    .expect("working again");
    let stop = world.log("session.stop");
    let done = update(
        SessionChange {
            status: Some(SessionStatus::Done),
            note: Some("Stopped by Ada".into()),
            ..SessionChange::default()
        },
        T0 + 7,
        Some(&stop),
    )
    .await
    .expect("done");
    assert_eq!(done.note.as_deref(), Some("Stopped by Ada"));
    assert_eq!(done.platform_handle.as_deref(), Some("handle-1"));
    assert!(logged(store, org, stop.id).await);
    for status in [
        SessionStatus::Working,
        SessionStatus::Failed,
        SessionStatus::WaitingForYou,
    ] {
        refused(
            update(
                SessionChange {
                    status: Some(status),
                    ..SessionChange::default()
                },
                T0 + 8,
                None,
            )
            .await,
            Invariant::IllegalTransition,
        );
    }
    let cleaned = update(
        SessionChange {
            cleanup: Some(Cleanup::Removed),
            ..SessionChange::default()
        },
        T0 + 9,
        None,
    )
    .await
    .expect("clean-up is reported after the end");
    assert_eq!(
        (cleaned.status, cleaned.cleanup),
        (SessionStatus::Done, Some(Cleanup::Removed))
    );
    not_found(
        store
            .update_session(
                world_org(store).await,
                lead.id,
                &SessionChange::default(),
                T0,
                None,
            )
            .await,
    );

    let say = |key: &str, kind, member, text: &str, at| NewEvent {
        organization: org,
        session: lead.id,
        key: key.to_owned(),
        kind,
        member,
        text: text.to_owned(),
        at,
    };
    for event in [
        say(
            "prompt",
            EventKind::Person,
            Some(world.owner.id),
            "make it faster",
            T0 + 1,
        ),
        say("agent:0", EventKind::Agent, None, "On it.", T0 + 2),
        say("tool:c1", EventKind::Tool, None, "measure", T0 + 3),
        say("agent:0", EventKind::Agent, None, "On it. (again)", T0 + 4),
    ] {
        store.append_event(&event).await.expect("appended");
    }
    let all = store
        .session_events(org, lead.id, 0, 10)
        .await
        .expect("the conversation");
    assert_eq!(
        all.iter()
            .map(|event| (event.kind, event.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (EventKind::Person, "make it faster"),
            (EventKind::Agent, "On it."),
            (EventKind::Tool, "measure"),
        ],
        "a key lands once, and the first word stays"
    );
    let seqs: Vec<i64> = all.iter().map(|event| event.seq).collect();
    assert!(seqs.windows(2).all(|pair| pair.first() < pair.get(1)));
    assert_eq!(
        all.first().and_then(|event| event.member_id),
        Some(world.owner.id)
    );
    let page = store
        .session_events(org, lead.id, seqs.first().copied().expect("one"), 1)
        .await
        .expect("a page");
    assert_eq!(page.len(), 1);
    assert_eq!(
        page.first().map(|event| event.text.as_str()),
        Some("On it.")
    );
    assert!(
        store
            .session_events(org, lead.id, seqs.last().copied().expect("one"), 10)
            .await
            .expect("the end")
            .is_empty()
    );
    assert!(
        store
            .session_events(world_org(store).await, lead.id, 0, 10)
            .await
            .expect("another organization")
            .is_empty()
    );
    not_found(
        store
            .append_event(&NewEvent {
                organization: world_org(store).await,
                ..say("x", EventKind::Agent, None, "x", T0)
            })
            .await,
    );
}

async fn agents<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let agent = |id: &str, enabled, is_default| Agent {
        id: id.to_owned(),
        name: id.to_owned(),
        platform: "agentcore".into(),
        enabled,
        is_default,
        cost_tier: CostTier::Medium,
        built_in: false,
    };
    assert!(store.agents(org).await.expect("none yet").is_empty());
    for (row, default) in [("claude-code", true), ("codex", false)] {
        store
            .put_agent(org, &agent(row, true, default), T0, &world.log("agent.set"))
            .await
            .expect("put");
    }
    let listed = store.agents(org).await.expect("listed");
    assert_eq!(
        listed
            .iter()
            .map(|a| (a.id.as_str(), a.is_default))
            .collect::<Vec<_>>(),
        vec![("claude-code", true), ("codex", false)]
    );
    // A new default clears the old one; a replaced agent changes in place.
    store
        .put_agent(
            org,
            &agent("codex", true, true),
            T0 + 1,
            &world.log("agent.set"),
        )
        .await
        .expect("put");
    store
        .put_agent(
            org,
            &agent("claude-code", false, false),
            T0 + 2,
            &world.log("agent.set"),
        )
        .await
        .expect("put");
    let listed = store.agents(org).await.expect("listed");
    assert_eq!(
        listed
            .iter()
            .map(|a| (a.id.as_str(), a.enabled, a.is_default))
            .collect::<Vec<_>>(),
        vec![("claude-code", false, false), ("codex", true, true)]
    );
    refused(
        store
            .put_agent(org, &agent("x", false, true), T0, &world.log("agent.set"))
            .await,
        Invariant::IllegalTransition,
    );
    not_found(
        store
            .put_agent(
                OrganizationId::new(),
                &agent("x", true, false),
                T0,
                &world.log("agent.set"),
            )
            .await,
    );
    assert!(
        store
            .agents(world_org(store).await)
            .await
            .expect("another organization")
            .is_empty()
    );
}

async fn evaluations<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let template = |id: &str, at| EvaluationTemplate {
        id: id.to_owned(),
        name: id.to_owned(),
        summary: String::new(),
        kind: EvaluationKind::Speed,
        metric: "p95".into(),
        lower_is_better: true,
        command: "make bench".into(),
        protected_paths: vec!["app:bench/".into(), "app:fixtures/".into()],
        created_by: world.owner.id,
        created_at: at,
    };
    let first = template("load-test", T0);
    store
        .add_evaluation(&first, org, &world.log("evaluation.add"))
        .await
        .expect("added");
    store
        .add_evaluation(
            &template("bundle", T0 + 1),
            org,
            &world.log("evaluation.add"),
        )
        .await
        .expect("added");
    conflict(
        store
            .add_evaluation(&first, org, &world.log("evaluation.add"))
            .await,
    );
    let mut stray = template("x", T0);
    stray.created_by = MemberId::new();
    not_found(
        store
            .add_evaluation(&stray, org, &world.log("evaluation.add"))
            .await,
    );
    let listed = store.evaluations(org).await.expect("listed");
    assert_eq!(listed.first(), Some(&first));
    assert_eq!(listed.len(), 2);
    assert!(
        store
            .evaluations(world_org(store).await)
            .await
            .expect("another organization")
            .is_empty()
    );
}

async fn approvals<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let experiment = experiment(store, &world, 1_000).await.expect("created");
    let policy = |card, mode, limit_cents| Policy {
        card,
        mode,
        limit_cents,
    };

    store
        .set_policy(
            org,
            &policy(CardKind::RaiseBudget, PolicyMode::AllowUpTo, Some(500)),
            &world.log("policy.set"),
        )
        .await
        .expect("set");
    store
        .set_policy(
            org,
            &policy(CardKind::RaiseBudget, PolicyMode::Allow, None),
            &world.log("policy.set"),
        )
        .await
        .expect("set again replaces");
    store
        .set_policy(
            org,
            &policy(CardKind::AddCredential, PolicyMode::AlwaysAsk, None),
            &world.log("policy.set"),
        )
        .await
        .expect("always ask is allowed");
    for mode in [PolicyMode::Allow, PolicyMode::AllowUpTo] {
        let limit = (mode == PolicyMode::AllowUpTo).then_some(1);
        refused(
            store
                .set_policy(
                    org,
                    &policy(CardKind::AddCredential, mode, limit),
                    &world.log("policy.set"),
                )
                .await,
            Invariant::AlwaysAsk,
        );
    }
    assert_eq!(
        store.policies(org).await.expect("read"),
        vec![
            policy(CardKind::AddCredential, PolicyMode::AlwaysAsk, None),
            policy(CardKind::RaiseBudget, PolicyMode::Allow, None),
        ]
    );
    assert_eq!(
        store.policies(world_org(store).await).await.expect("read"),
        vec![]
    );

    let card = |kind, status| Approval {
        id: ApprovalId::new(),
        organization_id: org,
        experiment_id: experiment.id,
        session_id: None,
        card: kind,
        summary: "Raise the budget by $5".into(),
        cost_cents: Some(500),
        needs_owner: false,
        status,
        decided_by: None,
        created_at: T0,
        decided_at: (status != ApprovalStatus::Pending).then_some(T0),
    };
    let pending = card(CardKind::RaiseBudget, ApprovalStatus::Pending);
    store
        .request_approval(&pending, &world.log("approval.request"))
        .await
        .expect("requested");
    assert_eq!(
        store.approval(org, pending.id).await.expect("read"),
        Some(pending.clone())
    );
    assert_eq!(
        store
            .approval(world_org(store).await, pending.id)
            .await
            .expect("read"),
        None
    );

    let approved_entry = AuditEntry::new(
        org,
        Some(world.owner.id),
        "approval.decide",
        pending.id,
        AuditOutcome::Approved,
        T0 + 1,
    );
    let decided = store
        .decide_approval(
            org,
            pending.id,
            Decision::Approve,
            world.owner.id,
            T0 + 1,
            &approved_entry,
        )
        .await
        .expect("approved");
    assert_eq!(
        (decided.status, decided.decided_by, decided.decided_at),
        (ApprovalStatus::Approved, Some(world.owner.id), Some(T0 + 1))
    );
    assert!(logged(store, org, approved_entry.id).await);
    let again = world.log("approval.decide");
    refused(
        store
            .decide_approval(
                org,
                pending.id,
                Decision::Decline,
                world.owner.id,
                T0 + 2,
                &again,
            )
            .await,
        Invariant::IllegalTransition,
    );
    assert!(!logged(store, org, again.id).await);
    not_found(
        store
            .decide_approval(
                org,
                ApprovalId::new(),
                Decision::Approve,
                world.owner.id,
                T0,
                &again,
            )
            .await,
    );

    let second = card(CardKind::KeepGoing, ApprovalStatus::Pending);
    store
        .request_approval(&second, &world.log("approval.request"))
        .await
        .expect("requested");
    let declined = store
        .decide_approval(
            org,
            second.id,
            Decision::Decline,
            world.owner.id,
            T0 + 3,
            &world.log("approval.decide"),
        )
        .await
        .expect("declined");
    assert_eq!(declined.status, ApprovalStatus::Declined);

    store
        .request_approval(
            &card(CardKind::RaiseBudget, ApprovalStatus::ApprovedByPolicy),
            &world.log("approval.request"),
        )
        .await
        .expect("policy may approve");
    refused(
        store
            .request_approval(
                &card(CardKind::AddCredential, ApprovalStatus::ApprovedByPolicy),
                &world.log("approval.request"),
            )
            .await,
        Invariant::AlwaysAsk,
    );
    refused(
        store
            .request_approval(
                &card(CardKind::KeepGoing, ApprovalStatus::Approved),
                &world.log("approval.request"),
            )
            .await,
        Invariant::IllegalTransition,
    );
    assert_eq!(
        store
            .approvals(org, experiment.id)
            .await
            .expect("read")
            .len(),
        3
    );
    let waiting = card(CardKind::AttachRepo, ApprovalStatus::Pending);
    store
        .request_approval(&waiting, &world.log("approval.request"))
        .await
        .expect("requested");
    let pending = store.pending_approvals(org).await.expect("pending");
    assert_eq!(
        pending.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![waiting.id]
    );
    assert!(
        store
            .pending_approvals(world_org(store).await)
            .await
            .expect("another organization")
            .is_empty()
    );
}

async fn audit_log<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let other = world_org(store).await;
    let before = store.audit_log(org, None, 1000).await.expect("read").len();
    let denied = AuditEntry::new(
        org,
        Some(world.owner.id),
        "project.delete",
        world.project.id,
        AuditOutcome::Denied,
        T0 + 1_000,
    );
    store
        .append_audit(&denied)
        .await
        .expect("a denial is recorded");
    let system = AuditEntry::new(
        org,
        None,
        "session.clean_up",
        "s",
        AuditOutcome::Done,
        T0 + 2_000,
    );
    store.append_audit(&system).await.expect("recorded");
    let foreign = AuditEntry::new(other, None, "x", "y", AuditOutcome::Done, T0 + 3_000);
    store.append_audit(&foreign).await.expect("recorded");
    conflict(store.append_audit(&denied).await);
    not_found(
        store
            .append_audit(&AuditEntry::new(
                OrganizationId::new(),
                None,
                "x",
                "y",
                AuditOutcome::Done,
                T0,
            ))
            .await,
    );

    let all = store.audit_log(org, None, 1000).await.expect("read");
    assert_eq!(all.len(), before + 2);
    assert_eq!(all.first(), Some(&system));
    assert_eq!(all.get(1), Some(&denied));
    assert!(all.iter().all(|row| row.organization_id == org));

    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let page = store.audit_log(org, cursor, 2).await.expect("read");
        let Some(last) = page.last() else { break };
        cursor = Some((last.at, last.id));
        seen.extend(page);
    }
    assert_eq!(seen, all);
}

async fn owners_and_sign_ins<S: Store>(store: &S) {
    let organization = Organization::new(format!("owners-{}", unique()), T0);
    store
        .create_organization(&organization)
        .await
        .expect("created");
    let id = organization.id;
    let member = |role| {
        Member::person(
            id,
            "Ada",
            Identity {
                provider: "fake".into(),
                subject: unique(),
            },
            role,
            T0,
        )
    };
    let viewer = member(Role::Viewer);
    refused(
        store
            .add_member(&viewer, &entry(id, viewer.id, "member.add"))
            .await,
        Invariant::LastOwner,
    );
    assert!(store.members(id).await.expect("read").is_empty());

    let first = member(Role::Owner);
    store
        .add_member(&first, &entry(id, first.id, "member.add"))
        .await
        .expect("an owner first");
    store
        .add_member(&viewer, &entry(id, first.id, "member.add"))
        .await
        .expect("then anyone");
    refused(
        store
            .set_member_role(
                id,
                first.id,
                Role::Editor,
                &entry(id, first.id, "member.set_role"),
            )
            .await,
        Invariant::LastOwner,
    );
    let second = member(Role::Owner);
    store
        .add_member(&second, &entry(id, first.id, "member.add"))
        .await
        .expect("a second owner");
    store
        .set_member_role(
            id,
            first.id,
            Role::Editor,
            &entry(id, first.id, "member.set_role"),
        )
        .await
        .expect("one of two owners steps down");
    refused(
        store
            .set_member_role(
                id,
                second.id,
                Role::Viewer,
                &entry(id, first.id, "member.set_role"),
            )
            .await,
        Invariant::LastOwner,
    );

    let issued = token::issue_session(id, viewer.id, T0).expect("random");
    let log = entry(id, viewer.id, "sign_in");
    store
        .create_sign_in_session(&issued.record, &log)
        .await
        .expect("a session is stored");
    conflict(store.create_sign_in_session(&issued.record, &log).await);
    assert!(logged(store, id, log.id).await);
    let live = token::authenticate_session(store, &issued.secret, T0 + 1)
        .await
        .expect("read")
        .expect("a live session");
    assert_eq!((live.organization_id, live.member_id), (id, viewer.id));
    let forged = format!("{}x", issued.secret);
    let other = token::issue_session(id, viewer.id, T0).expect("random");
    let spliced = format!(
        "als_{}_{}",
        issued.record.session.id.as_uuid().simple(),
        other.secret.rsplit('_').next().unwrap_or_default()
    );
    for bad in [forged.as_str(), spliced.as_str(), "als_x_y", "alt_x_y", ""] {
        assert_eq!(
            token::authenticate_session(store, bad, T0 + 1)
                .await
                .expect("read"),
            None,
            "{bad}"
        );
    }
    assert_eq!(
        token::authenticate_session(store, &issued.secret, issued.record.session.expires_at)
            .await
            .expect("read"),
        None,
        "an expired session"
    );
    not_found(
        store
            .create_sign_in_session(
                &token::issue_session(id, MemberId::new(), T0)
                    .expect("random")
                    .record,
                &log,
            )
            .await,
    );
    store
        .end_sign_in_session(id, live.id, T0 + 2, &entry(id, viewer.id, "sign_out"))
        .await
        .expect("ended");
    assert_eq!(
        token::authenticate_session(store, &issued.secret, T0 + 3)
            .await
            .expect("read"),
        None,
        "a signed-out session"
    );
    not_found(
        store
            .end_sign_in_session(
                OrganizationId::new(),
                live.id,
                T0,
                &entry(id, viewer.id, "sign_out"),
            )
            .await,
    );
}
