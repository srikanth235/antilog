#![expect(
    clippy::expect_used,
    reason = "test support: a failed call fails the suite"
)]

use antilog_types::{
    Agent, Approval, ApprovalId, ApprovalStatus, AuditEntry, AuditId, AuditOutcome, CardKind,
    CostTier, Decision, Environment, EventKind, Identity, Member, MemberId, NetworkLevel,
    Organization, OrganizationId, Policy, PolicyMode, Project, ProjectRole, Role, Session,
    SessionId, SessionParams, SessionStatus, Spend, SpendId, SpendKind,
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

async fn session<S: Store>(
    store: &S,
    world: &World,
    agent: &str,
    cents: Option<i64>,
) -> Result<Session, StoreError> {
    let session = Session::new(
        world.id(),
        world.project.id,
        world.owner.id,
        SessionParams {
            title: "cut p95".into(),
            agent: agent.into(),
            model: "opencode/test".into(),
            budget_cap_cents: cents,
            ..SessionParams::default()
        },
        T0,
    );
    store
        .create_session(&session, &world.log("session.start"))
        .await
        .map(|()| session)
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
    sessions(store).await;
    agents(store).await;
    environments(store).await;
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
            Some(3),
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
                None,
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
    let org = world.id();

    store
        .set_organization_limits(
            org,
            Some(10_000),
            Some(2_000),
            None,
            &world.log("budget.set"),
        )
        .await
        .expect("set");
    refused(
        store
            .set_organization_limits(
                org,
                Some(1_000),
                Some(2_000),
                None,
                &world.log("budget.set"),
            )
            .await,
        Invariant::OverCap,
    );
    let limits = async |cap, default| {
        store
            .set_project_limits(
                org,
                world.project.id,
                cap,
                default,
                &world.log("budget.set"),
            )
            .await
    };
    refused(limits(Some(20_000), None).await, Invariant::OverCap);
    refused(limits(Some(5_000), Some(6_000)).await, Invariant::OverCap);
    limits(Some(9_000), Some(1_000)).await.expect("inside");

    let lead = session(store, &world, "claude", Some(3_000))
        .await
        .expect("inside");
    refused(
        session(store, &world, "claude", Some(9_001)).await,
        Invariant::OverCap,
    );
    let set = async |cents| {
        store
            .set_session_budget(org, lead.id, cents, &world.log("budget.set"))
            .await
    };
    set(9_000).await.expect("up to the project's cap");
    refused(set(9_001).await, Invariant::OverCap);
    assert_eq!(
        store
            .session(org, lead.id)
            .await
            .expect("read")
            .and_then(|row| row.budget_cap_cents),
        Some(9_000)
    );
    refused(
        store
            .set_organization_limits(org, Some(5_000), None, None, &world.log("budget.set"))
            .await,
        Invariant::OverCap,
    );
    not_found(
        store
            .set_session_budget(org, SessionId::new(), 1, &world.log("budget.set"))
            .await,
    );

    store
        .set_organization_limits(org, None, None, None, &world.log("budget.set"))
        .await
        .expect("lifting the organization cap");
    let elsewhere = session(store, &world, "claude", None)
        .await
        .expect("no cap");
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
        spend(&lead, SpendKind::Machine, 25, 0, T0 + 20),
        spend(&elsewhere, SpendKind::Machine, 7, 0, T0 + 20),
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
        total(SpendScope::Session(lead.id), T0, T0 + 100).await,
        sum(66, 901)
    );
    assert_eq!(
        total(SpendScope::Session(lead.id), T0 + 10, T0 + 30).await,
        sum(65, 900)
    );
    assert_eq!(
        total(SpendScope::Session(elsewhere.id), T0, T0 + 100).await,
        sum(7, 0)
    );
    assert_eq!(
        total(SpendScope::Organization, T0, T0 + 100).await,
        sum(73, 901)
    );
    assert_eq!(
        total(SpendScope::Organization, T0 + 31, T0 + 100).await,
        sum(0, 0)
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
        .spend_lines(org, lead.id)
        .await
        .expect("a session's lines");
    let mut got: Vec<_> = lines
        .iter()
        .map(|line| (line.kind.as_str(), line.cents, line.tokens))
        .collect();
    got.sort();
    assert_eq!(got, vec![("machine", 25, 0), ("tokens", 41, 901)]);
    assert!(
        store
            .spend_lines(world_org(store).await, lead.id)
            .await
            .expect("another organization")
            .is_empty()
    );
}

async fn sessions<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let lead = session(store, &world, "claude", Some(1_000))
        .await
        .expect("created");
    assert_eq!(
        store.session(org, lead.id).await.expect("read"),
        Some(lead.clone())
    );
    assert_eq!(
        store
            .session(world_org(store).await, lead.id)
            .await
            .expect("read"),
        None
    );
    let create = async |session: &Session| {
        store
            .create_session(session, &world.log("session.start"))
            .await
    };
    not_found(
        create(&Session::new(
            org,
            world.project.id,
            MemberId::new(),
            SessionParams {
                title: "x".into(),
                agent: "claude".into(),
                ..SessionParams::default()
            },
            T0,
        ))
        .await,
    );
    let other_person = person(store, org, Role::Editor).await;
    let child = Session::new(
        org,
        world.project.id,
        other_person.id,
        SessionParams {
            title: "second".into(),
            agent: "codex".into(),
            ..SessionParams::default()
        },
        T0 + 1,
    );
    create(&child).await.expect("a second session");
    conflict(create(&lead).await);
    assert_eq!(
        store.sessions(org, None, None).await.expect("read"),
        vec![child.clone(), lead.clone()],
        "newest first"
    );
    assert_eq!(
        store
            .sessions(org, Some(world.project.id), Some(world.owner.id))
            .await
            .expect("read"),
        vec![lead.clone()]
    );
    assert!(
        store
            .sessions(world_org(store).await, None, None)
            .await
            .expect("another organization")
            .is_empty()
    );

    let update = async |change: SessionChange, at, entry: Option<&AuditEntry>| {
        store.update_session(org, lead.id, &change, at, entry).await
    };
    let waiting = update(
        SessionChange {
            status: Some(SessionStatus::WaitingForYou),
            ..SessionChange::default()
        },
        T0 + 5,
        None,
    )
    .await
    .expect("waiting");
    assert_eq!(waiting.status, SessionStatus::WaitingForYou);
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
        },
        T0 + 7,
        Some(&stop),
    )
    .await
    .expect("done");
    assert_eq!(done.note.as_deref(), Some("Stopped by Ada"));
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
        tool: None,
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
        NewEvent {
            tool: Some(antilog_types::ToolRecord {
                label: "measure".to_owned(),
                status: antilog_types::ToolStatus::Failed,
                input: Some("{}".to_owned()),
                output: Some("refused".to_owned()),
            }),
            ..say("tool:c1", EventKind::Tool, None, "measure", T0 + 3)
        },
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
    assert_eq!(
        all.iter()
            .map(|event| event.tool.is_some())
            .collect::<Vec<_>>(),
        vec![false, false, true],
        "only a tool call carries its record"
    );
    assert_eq!(
        all.get(2)
            .and_then(|event| event.tool.as_ref())
            .map(|t| (t.status, t.output.as_deref())),
        Some((antilog_types::ToolStatus::Failed, Some("refused")))
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
    // The live stream's marks: seq and session, only this organization's, oldest first.
    let marks = store.event_marks(org, 0, 10).await.expect("marks");
    assert_eq!(
        marks.iter().map(|(seq, _)| *seq).collect::<Vec<_>>(),
        seqs,
        "one mark per conversation item"
    );
    assert!(marks.iter().all(|(_, session)| *session == lead.id));
    let after_first = store
        .event_marks(org, seqs.first().copied().expect("one"), 1)
        .await
        .expect("a page of marks");
    assert_eq!(after_first.len(), 1);
    assert_eq!(
        store.latest_event_seq(org).await.expect("latest"),
        seqs.last().copied().expect("one")
    );
    let stranger = world_org(store).await;
    assert!(
        store
            .event_marks(stranger, 0, 10)
            .await
            .expect("another organization")
            .is_empty()
    );
    assert_eq!(store.latest_event_seq(stranger).await.expect("none"), 0);
    let changed = store
        .sessions_changed(org, T0, 10)
        .await
        .expect("changed sessions");
    assert!(changed.iter().any(|row| row.id == lead.id));
    assert!(
        store
            .sessions_changed(org, T0 + 1_000_000, 10)
            .await
            .expect("none changed since")
            .is_empty()
    );
    assert!(
        store
            .sessions_changed(stranger, 0, 10)
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

async fn environments<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let version = |id: &str, version: u32, network: NetworkLevel| Environment {
        id: id.to_owned(),
        name: format!("{id} {version}"),
        version,
        network,
        allowed_hosts: vec!["api.stripe.com".to_owned()],
        allow_package_managers: version % 2 == 1,
        variables: [("STRIPE_KEY".to_owned(), "sk_test".to_owned())].into(),
        setup_script: "echo ready".to_owned(),
        cpus: 4,
        memory_mb: 16384,
        built_in: false,
        archived: false,
        created_by: Some(world.owner.id),
        created_at: T0 + i64::from(version),
    };
    assert!(store.environments(org).await.expect("none yet").is_empty());

    // A new environment is version 1; its id is taken for good.
    let first = version("bench", 1, NetworkLevel::Limited);
    store
        .add_environment(org, &first, &world.log("environment.create"))
        .await
        .expect("created");
    conflict(
        store
            .add_environment(org, &first, &world.log("environment.create"))
            .await,
    );
    store
        .add_environment(
            org,
            &version("full", 1, NetworkLevel::Full),
            &world.log("environment.create"),
        )
        .await
        .expect("a second environment");

    // An edit is the next version; the old one stays readable as it was.
    let second = version("bench", 2, NetworkLevel::None);
    store
        .add_environment_version(org, &second, &world.log("environment.edit"))
        .await
        .expect("edited");
    // Two edits made from one version cannot both land, and a version cannot be skipped.
    conflict(
        store
            .add_environment_version(org, &second, &world.log("environment.edit"))
            .await,
    );
    conflict(
        store
            .add_environment_version(
                org,
                &version("bench", 5, NetworkLevel::None),
                &world.log("environment.edit"),
            )
            .await,
    );
    not_found(
        store
            .add_environment_version(
                org,
                &version("nothing", 2, NetworkLevel::None),
                &world.log("environment.edit"),
            )
            .await,
    );
    assert_eq!(
        store
            .environment_version(org, "bench", 1)
            .await
            .expect("read"),
        Some(first.clone())
    );
    assert_eq!(
        store
            .environment_version(org, "bench", 2)
            .await
            .expect("read"),
        Some(second.clone())
    );
    assert_eq!(
        store
            .environment_version(org, "bench", 3)
            .await
            .expect("read"),
        None
    );
    let listed = store.environments(org).await.expect("listed");
    assert_eq!(
        listed
            .iter()
            .map(|row| (row.id.as_str(), row.version, row.network, row.archived))
            .collect::<Vec<_>>(),
        vec![
            ("bench", 2, NetworkLevel::None, false),
            ("full", 1, NetworkLevel::Full, false)
        ]
    );
    assert_eq!(listed.first(), Some(&second));

    // Archiving hides nothing that ran: every version stays readable, marked archived, and the
    // environment takes no new version. Archiving again changes nothing.
    store
        .archive_environment(org, "bench", &world.log("environment.archive"))
        .await
        .expect("archived");
    store
        .archive_environment(org, "bench", &world.log("environment.archive"))
        .await
        .expect("archived again");
    not_found(
        store
            .archive_environment(org, "nothing", &world.log("environment.archive"))
            .await,
    );
    refused(
        store
            .add_environment_version(
                org,
                &version("bench", 3, NetworkLevel::None),
                &world.log("environment.edit"),
            )
            .await,
        Invariant::IllegalTransition,
    );
    let kept = store
        .environment_version(org, "bench", 1)
        .await
        .expect("read")
        .expect("kept");
    assert!(kept.archived);
    assert_eq!(
        Environment {
            archived: false,
            ..kept
        },
        first
    );
    let listed = store.environments(org).await.expect("listed");
    assert_eq!(
        listed
            .iter()
            .map(|row| (row.id.as_str(), row.archived))
            .collect::<Vec<_>>(),
        vec![("bench", true), ("full", false)]
    );

    // Another organization sees none of it.
    let other = world_org(store).await;
    assert!(store.environments(other).await.expect("read").is_empty());
    assert_eq!(
        store
            .environment_version(other, "bench", 1)
            .await
            .expect("read"),
        None
    );
    not_found(
        store
            .archive_environment(other, "bench", &world.log("environment.archive"))
            .await,
    );
}

async fn approvals<S: Store>(store: &S) {
    let world = world(store).await;
    let org = world.id();
    let session = session(store, &world, "claude", Some(1_000))
        .await
        .expect("created");
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
        session_id: session.id,
        card: kind,
        summary: "Raise the budget by $5".into(),
        payload: None,
        cost_cents: Some(500),
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

    let second = card(CardKind::ChangeEnvironment, ApprovalStatus::Pending);
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
                &card(CardKind::ChangeEnvironment, ApprovalStatus::Approved),
                &world.log("approval.request"),
            )
            .await,
        Invariant::IllegalTransition,
    );
    assert_eq!(
        store.approvals(org, session.id).await.expect("read").len(),
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
        "session.delete",
        world.owner.id,
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
