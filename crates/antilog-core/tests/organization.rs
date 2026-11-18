#![expect(clippy::unwrap_used, reason = "tests")]

use antilog_core::approval::{self, CardRequest};
use antilog_core::auth::{self, Challenge, Credential, FakeSignIn, Proven, SignInProvider};
use antilog_core::budget::{self, Spent, month_bounds};
use antilog_core::fakes::FakeStore;
use antilog_core::organization::{self as org, Action, Principal, Refusal};
use antilog_core::store::{ApprovalStore, BudgetStore, Invariant, SessionStore, Store};
use antilog_core::token;
use antilog_types::{
    ApprovalStatus, AuditOutcome, CardKind, Decision, Experiment, Identity, Member, Organization,
    Policy, PolicyMode, Project, Role, Session, Spend, SpendId, SpendKind,
};

const T0: i64 = 1_700_000_000_000;

fn proven(subject: &str) -> Proven {
    Proven {
        identity: Identity {
            provider: "fake".into(),
            subject: subject.into(),
        },
        name: subject.into(),
    }
}

struct World {
    store: FakeStore,
    organization: Organization,
    owner: Principal,
}

impl World {
    async fn new() -> Self {
        let store = FakeStore::default();
        let (organization, signed) = auth::create_organization(&store, T0, "acme", &proven("ada"))
            .await
            .unwrap();
        let owner = Principal {
            organization: organization.id,
            member: signed.member.id,
        };
        Self {
            store,
            organization,
            owner,
        }
    }

    async fn member(&self, subject: &str, role: Role) -> (Principal, Member) {
        let member = org::add_member(
            &self.store,
            &self.owner,
            T0,
            subject,
            Some(proven(subject).identity),
            role,
        )
        .await
        .unwrap();
        (
            Principal {
                organization: self.organization.id,
                member: member.id,
            },
            member,
        )
    }

    async fn project(&self) -> Project {
        org::create_project(&self.store, &self.owner, T0, "checkout", "")
            .await
            .unwrap()
    }

    async fn experiment(&self, project: &Project, cents: i64) -> Experiment {
        let experiment = Experiment::new(
            self.organization.id,
            project.id,
            "cut p95",
            cents,
            self.owner.member,
            T0,
        );
        self.store
            .create_experiment(
                &experiment,
                &antilog_types::AuditEntry::new(
                    self.organization.id,
                    Some(self.owner.member),
                    "experiment.create",
                    experiment.id,
                    AuditOutcome::Done,
                    T0,
                ),
            )
            .await
            .unwrap();
        experiment
    }

    async fn audited(&self, action: &str, outcome: AuditOutcome) -> usize {
        self.store
            .audit_log(self.organization.id, None, 1000)
            .await
            .unwrap()
            .iter()
            .filter(|row| row.action == action && row.outcome == outcome)
            .count()
    }
}

#[tokio::test]
async fn the_permission_matrix_is_applied_in_one_place() {
    let world = World::new().await;
    let project = world.project().await;
    for role in Role::ALL {
        let (who, _) = world.member(&format!("{role:?}"), *role).await;
        for action in Action::ALL {
            let scope = action.in_project().then_some(project.id);
            let before = world.audited(action.name(), AuditOutcome::Denied).await;
            let result = org::permit(&world.store, &who, *action, scope, "target", T0).await;
            let allowed = *role >= action.minimum();
            assert_eq!(result.is_ok(), allowed, "{role:?} {action:?}");
            let after = world.audited(action.name(), AuditOutcome::Denied).await;
            assert_eq!(after, before + usize::from(!allowed), "{role:?} {action:?}");
        }
    }
}

#[tokio::test]
async fn a_project_role_overrides_the_organization_role_in_that_project_only() {
    let world = World::new().await;
    let (editor, editor_member) = world.member("ed", Role::Editor).await;
    let (viewer, viewer_member) = world.member("vi", Role::Viewer).await;
    let (a, b) = (world.project().await, {
        org::create_project(&world.store, &world.owner, T0, "search", "")
            .await
            .unwrap()
    });
    org::set_project_role(
        &world.store,
        &world.owner,
        T0,
        a.id,
        editor_member.id,
        Some(Role::Viewer),
    )
    .await
    .unwrap();
    org::set_project_role(
        &world.store,
        &world.owner,
        T0,
        a.id,
        viewer_member.id,
        Some(Role::Owner),
    )
    .await
    .unwrap();
    let can = |who: Principal, project: Project| {
        let store = &world.store;
        async move {
            org::permit(
                store,
                &who,
                Action::ApprovalDecide,
                Some(project.id),
                "x",
                T0,
            )
            .await
            .is_ok()
        }
    };
    assert!(!can(editor, a.clone()).await);
    assert!(can(editor, b.clone()).await);
    assert!(can(viewer, a).await);
    assert!(!can(viewer, b).await);
}

#[tokio::test]
async fn an_organization_keeps_an_owner_through_the_action_layer() {
    let world = World::new().await;
    let result = org::set_member_role(
        &world.store,
        &world.owner,
        T0,
        world.owner.member,
        Role::Editor,
    )
    .await;
    assert!(matches!(
        result,
        Err(Refusal::Invariant(Invariant::LastOwner))
    ));
    assert_eq!(
        world.audited("member.set_role", AuditOutcome::Done).await,
        0
    );
}

async fn raised(
    world: &World,
    experiment: &Experiment,
    card: CardKind,
    cost: i64,
    follow: bool,
    needs_owner: bool,
) -> antilog_types::Approval {
    approval::raise(
        &world.store,
        T0,
        Some(world.owner.member),
        CardRequest {
            organization: world.organization.id,
            experiment: experiment.id,
            session: None,
            card,
            summary: "Raise the budget by $5".into(),
            payload: None,
            cost_cents: Some(cost),
            needs_owner,
            follow_policy: follow,
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn policy_pre_approves_but_never_skips_the_audit_row() {
    let world = World::new().await;
    let project = world.project().await;
    let experiment = world.experiment(&project, 1_000).await;
    let policy = |card, mode, limit| Policy {
        card,
        mode,
        limit_cents: limit,
    };

    assert_eq!(
        raised(&world, &experiment, CardKind::RaiseBudget, 100, true, false)
            .await
            .status,
        ApprovalStatus::Pending,
        "nothing is set: always ask"
    );
    org::permit(&world.store, &world.owner, Action::PolicySet, None, "x", T0)
        .await
        .unwrap();
    approval::set_policy(
        &world.store,
        &world.owner,
        T0,
        policy(CardKind::RaiseBudget, PolicyMode::AllowUpTo, Some(500)),
    )
    .await
    .unwrap();
    approval::set_policy(
        &world.store,
        &world.owner,
        T0,
        policy(CardKind::AttachRepo, PolicyMode::Allow, None),
    )
    .await
    .unwrap();
    let step = |card, cost, follow, owner| raised(&world, &experiment, card, cost, follow, owner);
    assert_eq!(
        step(CardKind::RaiseBudget, 500, true, false).await.status,
        ApprovalStatus::ApprovedByPolicy
    );
    assert_eq!(
        step(CardKind::RaiseBudget, 501, true, false).await.status,
        ApprovalStatus::Pending
    );
    assert_eq!(
        step(CardKind::RaiseBudget, 100, false, false).await.status,
        ApprovalStatus::Pending,
        "Ask mode asks"
    );
    assert_eq!(
        step(CardKind::AttachRepo, 0, true, false).await.status,
        ApprovalStatus::ApprovedByPolicy
    );
    assert_eq!(
        step(CardKind::AttachRepo, 0, true, true).await.status,
        ApprovalStatus::Pending,
        "a value model is never pre-approved"
    );
    assert_eq!(
        world
            .audited("approval.raise", AuditOutcome::ApprovedByPolicy)
            .await,
        2
    );

    let credential = approval::set_policy(
        &world.store,
        &world.owner,
        T0,
        policy(CardKind::AddCredential, PolicyMode::Allow, None),
    )
    .await;
    assert!(matches!(
        credential,
        Err(Refusal::Invariant(Invariant::AlwaysAsk))
    ));
    assert_eq!(
        step(CardKind::AddCredential, 0, true, false).await.status,
        ApprovalStatus::Pending
    );

    for bad in [
        policy(CardKind::KeepGoing, PolicyMode::AllowUpTo, None),
        policy(CardKind::KeepGoing, PolicyMode::Allow, Some(5)),
    ] {
        assert!(matches!(
            approval::set_policy(&world.store, &world.owner, T0, bad).await,
            Err(Refusal::Invalid(_))
        ));
    }
    let (editor, _) = world.member("ed", Role::Editor).await;
    assert!(matches!(
        approval::set_policy(
            &world.store,
            &editor,
            T0,
            policy(CardKind::KeepGoing, PolicyMode::Allow, None)
        )
        .await,
        Err(Refusal::Denied)
    ));
    let all = approval::policies(&world.store, &editor).await.unwrap();
    assert_eq!(all.len(), CardKind::ALL.len());
}

#[tokio::test]
async fn who_may_decide_a_card() {
    let world = World::new().await;
    let project = world.project().await;
    let experiment = world.experiment(&project, 1_000).await;
    let (editor, _) = world.member("ed", Role::Editor).await;
    let (viewer, _) = world.member("vi", Role::Viewer).await;

    let plain = raised(
        &world,
        &experiment,
        CardKind::RaiseBudget,
        100,
        false,
        false,
    )
    .await;
    assert!(matches!(
        approval::decide(&world.store, &viewer, T0, plain.id, Decision::Approve).await,
        Err(Refusal::Denied)
    ));
    let decided = approval::decide(&world.store, &editor, T0, plain.id, Decision::Approve)
        .await
        .unwrap();
    assert_eq!(
        (decided.status, decided.decided_by),
        (ApprovalStatus::Approved, Some(editor.member))
    );
    assert!(
        matches!(
            approval::decide(&world.store, &world.owner, T0, plain.id, Decision::Decline).await,
            Err(Refusal::Invariant(Invariant::IllegalTransition))
        ),
        "a card is decided once"
    );
    assert_eq!(
        world
            .audited("approval.decide", AuditOutcome::Approved)
            .await,
        1
    );
    assert_eq!(
        world.audited("approval.decide", AuditOutcome::Denied).await,
        1
    );

    let model = raised(&world, &experiment, CardKind::SealExperiment, 0, true, true).await;
    assert!(matches!(
        approval::decide(&world.store, &editor, T0, model.id, Decision::Approve).await,
        Err(Refusal::Denied)
    ));
    let by_owner = approval::decide(&world.store, &world.owner, T0, model.id, Decision::Approve)
        .await
        .unwrap();
    assert_eq!(by_owner.status, ApprovalStatus::Approved);
    assert!(matches!(
        approval::decide(
            &world.store,
            &world.owner,
            T0,
            antilog_types::ApprovalId::new(),
            Decision::Approve
        )
        .await,
        Err(Refusal::NotFound)
    ));
    assert_eq!(
        approval::approvals(&world.store, &viewer, experiment.id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn pending_cards_are_the_ones_this_person_may_decide_across_experiments() {
    let world = World::new().await;
    let one = world.project().await;
    let two = org::create_project(&world.store, &world.owner, T0, "search", "")
        .await
        .unwrap();
    let first = world.experiment(&one, 1_000).await;
    let second = world.experiment(&two, 1_000).await;
    let (editor, editor_member) = world.member("ed", Role::Editor).await;
    let (viewer, _) = world.member("vi", Role::Viewer).await;
    // Ed is only a viewer in the second project.
    org::set_project_role(
        &world.store,
        &world.owner,
        T0,
        two.id,
        editor_member.id,
        Some(Role::Viewer),
    )
    .await
    .unwrap();

    let plain = raised(&world, &first, CardKind::RaiseBudget, 100, false, false).await;
    let elsewhere = raised(&world, &second, CardKind::RaiseBudget, 100, false, false).await;
    let model = raised(&world, &first, CardKind::SealExperiment, 0, true, true).await;
    let decided = raised(&world, &first, CardKind::KeepGoing, 0, false, false).await;
    approval::decide(
        &world.store,
        &world.owner,
        T0,
        decided.id,
        Decision::Decline,
    )
    .await
    .unwrap();

    let ids =
        |cards: Vec<antilog_types::Approval>| cards.into_iter().map(|c| c.id).collect::<Vec<_>>();
    assert_eq!(
        ids(approval::pending(&world.store, &world.owner).await.unwrap()),
        vec![plain.id, elsewhere.id, model.id]
    );
    assert_eq!(
        ids(approval::pending(&world.store, &editor).await.unwrap()),
        vec![plain.id],
        "an editor of one project, a viewer of the other, and no value-model card"
    );
    assert!(
        approval::pending(&world.store, &viewer)
            .await
            .unwrap()
            .is_empty()
    );
    let stranger = Principal {
        organization: world.organization.id,
        member: antilog_types::MemberId::new(),
    };
    assert!(matches!(
        approval::pending(&world.store, &stranger).await,
        Err(Refusal::Denied)
    ));
}

#[tokio::test]
async fn me_names_the_member_their_role_and_every_organization_of_their_identity() {
    let world = World::new().await;
    let me = org::me(&world.store, &world.owner).await.unwrap();
    assert_eq!(me.member.id, world.owner.member);
    assert_eq!(
        (me.role, me.organization.id),
        (Role::Owner, world.organization.id)
    );
    assert_eq!(me.organizations.len(), 1);

    // The same person is invited to a second organization.
    let (other, signed) = auth::create_organization(&world.store, T0 + 1, "globex", &proven("ada"))
        .await
        .unwrap();
    let there = org::me(
        &world.store,
        &Principal {
            organization: other.id,
            member: signed.member.id,
        },
    )
    .await
    .unwrap();
    let names: Vec<_> = there
        .organizations
        .iter()
        .map(|o| o.name.as_str())
        .collect();
    assert_eq!(names, ["acme", "globex"]);

    // A service account has only its own organization.
    let service = org::add_member(&world.store, &world.owner, T0, "ci", None, Role::Editor)
        .await
        .unwrap();
    let ci = org::me(
        &world.store,
        &Principal {
            organization: world.organization.id,
            member: service.id,
        },
    )
    .await
    .unwrap();
    assert_eq!(ci.organizations.len(), 1);
    assert!(matches!(
        org::me(
            &world.store,
            &Principal {
                organization: world.organization.id,
                member: antilog_types::MemberId::new()
            }
        )
        .await,
        Err(Refusal::Denied)
    ));
}

#[tokio::test]
async fn signing_in_without_an_organization_finds_the_oldest_membership() {
    let world = World::new().await;
    let signed = auth::sign_in(&world.store, T0 + 5, None, &proven("ada"))
        .await
        .unwrap();
    assert_eq!(signed.member.organization_id, world.organization.id);

    let (other, _) = auth::create_organization(&world.store, T0 + 10, "globex", &proven("ada"))
        .await
        .unwrap();
    let again = auth::sign_in(&world.store, T0 + 11, None, &proven("ada"))
        .await
        .unwrap();
    assert_eq!(
        again.member.organization_id, world.organization.id,
        "the oldest membership signs in; a named one is still reachable"
    );
    let named = auth::sign_in(&world.store, T0 + 12, Some(other.id), &proven("ada"))
        .await
        .unwrap();
    assert_eq!(named.member.organization_id, other.id);
    assert!(matches!(
        auth::sign_in(&world.store, T0 + 13, None, &proven("mallory")).await,
        Err(Refusal::Denied)
    ));
}

#[tokio::test]
async fn spend_is_read_by_experiment_kind_and_session() {
    let world = World::new().await;
    let project = world.project().await;
    let first = world.experiment(&project, 1_000).await;
    let second = world.experiment(&project, 1_000).await;
    spend(&world, &first, 40, T0).await;
    spend(&world, &first, 5, T0 + 1).await;
    spend(&world, &second, 7, T0).await;
    let (viewer, _) = world.member("vi", Role::Viewer).await;

    let one = budget::experiment_spend(&world.store, &viewer, first.id)
        .await
        .unwrap();
    assert_eq!((one.spent_cents, one.budget_cap_cents), (45, 1_000));
    assert_eq!(one.by_session.len(), 2);
    assert_eq!(one.by_kind.iter().map(|k| k.cents).sum::<i64>(), 45);
    let all = budget::all_spend(&world.store, &viewer).await.unwrap();
    assert_eq!(
        all.iter().map(|s| s.spent_cents).sum::<i64>(),
        52,
        "every experiment of the organization"
    );
    assert!(matches!(
        budget::experiment_spend(&world.store, &viewer, antilog_types::ExperimentId::new()).await,
        Err(Refusal::NotFound)
    ));
}

async fn spend(world: &World, experiment: &Experiment, cents: i64, at: i64) {
    let session = Session::new(world.organization.id, experiment.id, "claude", "fake", at);
    world
        .store
        .create_session(
            &session,
            &antilog_types::AuditEntry::new(
                world.organization.id,
                None,
                "session.create",
                session.id,
                AuditOutcome::Done,
                at,
            ),
        )
        .await
        .unwrap();
    world
        .store
        .record_spend(&Spend {
            id: SpendId::new(),
            organization_id: world.organization.id,
            session_id: session.id,
            kind: SpendKind::Tokens,
            amount_cents: cents,
            tokens: 1,
            occurred_at: at,
            trial_id: None,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn budgets_and_caps_are_enforced_and_nested() {
    let world = World::new().await;
    let project = world.project().await;
    let experiment = world.experiment(&project, 1_000).await;
    let id = world.organization.id;
    let (editor, _) = world.member("ed", Role::Editor).await;

    budget::set_organization_limits(&world.store, &world.owner, T0, Some(5_000), Some(1_000))
        .await
        .unwrap();
    budget::set_project_limits(
        &world.store,
        &world.owner,
        T0,
        project.id,
        Some(3_000),
        Some(500),
    )
    .await
    .unwrap();
    assert!(matches!(
        budget::set_organization_limits(&world.store, &editor, T0, None, None).await,
        Err(Refusal::Denied)
    ));
    assert!(matches!(
        budget::set_project_limits(
            &world.store,
            &world.owner,
            T0,
            project.id,
            Some(6_000),
            None
        )
        .await,
        Err(Refusal::Invariant(Invariant::OverCap))
    ));
    assert!(matches!(
        budget::set_experiment_budget(&world.store, &editor, T0, experiment.id, 3_001).await,
        Err(Refusal::Invariant(Invariant::OverCap))
    ));
    assert!(matches!(
        budget::set_organization_limits(&world.store, &world.owner, T0, Some(-1), None).await,
        Err(Refusal::Invalid(_))
    ));
    budget::set_experiment_budget(&world.store, &editor, T0, experiment.id, 2_000)
        .await
        .unwrap();

    let room = budget::ensure_room(&world.store, id, experiment.id, T0)
        .await
        .unwrap();
    assert_eq!(
        (room.experiment, room.project, room.organization),
        (2_000, Some(3_000), Some(5_000))
    );

    spend(&world, &experiment, 1_500, T0).await;
    assert_eq!(
        budget::headroom(&world.store, id, experiment.id, T0 + 1)
            .await
            .unwrap()
            .remaining(),
        500
    );
    spend(&world, &experiment, 500, T0 + 1).await;
    assert!(matches!(
        budget::ensure_room(&world.store, id, experiment.id, T0 + 2).await,
        Err(Refusal::Spent(Spent::Experiment))
    ));
    budget::set_experiment_budget(&world.store, &editor, T0, experiment.id, 2_500)
        .await
        .unwrap();
    assert_eq!(
        budget::ensure_room(&world.store, id, experiment.id, T0 + 2)
            .await
            .unwrap()
            .remaining(),
        500
    );

    let sibling = world.experiment(&project, 2_000).await;
    spend(&world, &sibling, 1_000, T0 + 3).await;
    assert!(matches!(
        budget::ensure_room(&world.store, id, sibling.id, T0 + 4).await,
        Err(Refusal::Spent(Spent::Project))
    ));

    let (_, next_month) = month_bounds(T0).unwrap();
    assert!(
        budget::headroom(&world.store, id, sibling.id, next_month)
            .await
            .unwrap()
            .project
            .unwrap()
            > 0,
        "a cap resets with the month"
    );
    let summary = budget::summary(&world.store, &editor, T0 + 4)
        .await
        .unwrap();
    assert_eq!(
        (
            summary.organization.cap_cents,
            summary.organization.spent_cents
        ),
        (Some(5_000), 3_000)
    );
    assert_eq!(summary.projects.len(), 1);

    let other = world.project_second().await;
    budget::set_project_limits(&world.store, &world.owner, T0, other.id, Some(4_000), None)
        .await
        .unwrap();
    let elsewhere = world.experiment(&other, 3_000).await;
    spend(&world, &elsewhere, 2_000, T0 + 5).await;
    let room = budget::headroom(&world.store, id, elsewhere.id, T0 + 6)
        .await
        .unwrap();
    assert_eq!(
        (room.experiment, room.project, room.organization),
        (1_000, Some(2_000), Some(0))
    );
    assert!(matches!(
        budget::ensure_room(&world.store, id, elsewhere.id, T0 + 6).await,
        Err(Refusal::Spent(Spent::Organization))
    ));
}

impl World {
    async fn project_second(&self) -> Project {
        org::create_project(&self.store, &self.owner, T0, "search", "")
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn month_bounds_are_the_utc_month() {
    let (from, to) = month_bounds(1_700_000_000_000).unwrap();
    assert_eq!((from, to), (1_698_796_800_000, 1_701_388_800_000));
    assert_eq!(month_bounds(from).unwrap().0, from);
    assert_eq!(month_bounds(to - 1).unwrap().1, to);
}

#[tokio::test]
async fn sign_in_admits_members_and_audits_strangers() {
    let world = World::new().await;
    let id = world.organization.id;
    let (_, ada) = world.member("grace", Role::Viewer).await;
    let signed = auth::sign_in(&world.store, T0, Some(id), &proven("grace"))
        .await
        .unwrap();
    assert_eq!(signed.member.id, ada.id);
    let (principal, credential) = auth::authenticate(&world.store, &signed.issued.secret, T0 + 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(principal.member, ada.id);
    assert!(matches!(credential, Credential::SignIn(_)));
    assert!(matches!(
        auth::sign_in(&world.store, T0, Some(id), &proven("mallory")).await,
        Err(Refusal::Denied)
    ));
    assert_eq!(world.audited("sign_in", AuditOutcome::Denied).await, 1);
    assert!(matches!(
        auth::sign_in(
            &world.store,
            T0,
            Some(antilog_types::OrganizationId::new()),
            &proven("grace")
        )
        .await,
        Err(Refusal::NotFound)
    ));

    if let Credential::SignIn(session) = credential {
        auth::sign_out(&world.store, &session, T0 + 2)
            .await
            .unwrap();
    }
    assert!(
        auth::authenticate(&world.store, &signed.issued.secret, T0 + 3)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(world.audited("sign_out", AuditOutcome::Done).await, 1);
    for secret in ["", "nope", "als_", "alt_a_b"] {
        assert!(
            auth::authenticate(&world.store, secret, T0)
                .await
                .unwrap()
                .is_none()
        );
    }
    assert!(
        auth::authenticate(
            &world.store,
            &signed.issued.secret,
            T0 + token::SESSION_TTL_MS
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[tokio::test]
async fn tokens_belong_to_their_member_and_service_accounts_to_owners() {
    let world = World::new().await;
    let (viewer, _) = world.member("vi", Role::Viewer).await;
    let mine = org::create_token(&world.store, &viewer, T0, None, "laptop")
        .await
        .unwrap();
    let (principal, credential) = auth::authenticate(&world.store, &mine.secret, T0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((principal, credential), (viewer, Credential::AccessToken));
    assert!(!format!("{mine:?}").contains(&mine.secret));

    let service = org::add_member(&world.store, &world.owner, T0, "ci", None, Role::Editor)
        .await
        .unwrap();
    assert!(matches!(
        org::create_token(&world.store, &viewer, T0, Some(service.id), "x").await,
        Err(Refusal::Denied)
    ));
    let theirs = org::create_token(&world.store, &world.owner, T0, Some(service.id), "ci")
        .await
        .unwrap();
    assert!(matches!(
        org::revoke_token(&world.store, &viewer, T0, theirs.record.token.id).await,
        Err(Refusal::Denied)
    ));
    assert!(matches!(
        org::tokens(&world.store, &viewer, Some(service.id)).await,
        Err(Refusal::Denied)
    ));
    org::revoke_token(&world.store, &viewer, T0, mine.record.token.id)
        .await
        .unwrap();
    assert!(
        auth::authenticate(&world.store, &mine.secret, T0)
            .await
            .unwrap()
            .is_none()
    );
    let audit = org::audit_log(&world.store, &world.owner, T0, None, 1000)
        .await
        .unwrap();
    assert!(
        !audit
            .iter()
            .any(|row| row.target.contains(&mine.secret) || row.action.contains("alt_"))
    );
    assert!(matches!(
        org::audit_log(&world.store, &viewer, T0, None, 10).await,
        Err(Refusal::Denied)
    ));
}

#[tokio::test]
async fn the_fake_provider_proves_only_a_named_subject() {
    let challenge = Challenge::new().unwrap();
    assert_eq!(
        Challenge::from_cookie(&challenge.cookie()),
        Some(challenge.clone())
    );
    assert_ne!(challenge, Challenge::new().unwrap());
    assert!(
        FakeSignIn
            .authorize_url(&challenge)
            .await
            .unwrap()
            .contains(&challenge.state)
    );
    let proved = FakeSignIn.complete("fake:ada", &challenge).await.unwrap();
    assert_eq!(proved.identity.subject, "ada");
    for bad in ["", "fake:", "ada", "github:1"] {
        assert!(FakeSignIn.complete(bad, &challenge).await.is_err());
    }
}

#[test]
fn no_password_exists_anywhere_in_the_types() {
    let schema = serde_json::to_string(&antilog_types::Member::service(
        antilog_types::OrganizationId::new(),
        "x",
        Role::Viewer,
        0,
    ))
    .unwrap();
    assert!(!schema.to_lowercase().contains("password"));
}

#[allow(dead_code)]
fn store_is_used<S: Store>(_: &S) {}

#[tokio::test]
async fn auto_picks_the_default_the_only_agent_or_asks_and_only_an_owner_sets_agents() {
    use antilog_core::agents;
    use antilog_types::{CostTier, NewAgent};

    let world = World::new().await;
    let org = world.organization.id;
    let new = |enabled, is_default| NewAgent {
        name: "Agent".into(),
        platform: "agentcore".into(),
        enabled,
        is_default,
        cost_tier: CostTier::Low,
    };
    let (editor, _) = world.member("ed", Role::Editor).await;

    // Nothing set up: the built-in agent keeps a first experiment from waiting on a settings visit.
    let (agent, reason) = agents::auto(&world.store, org, "fake").await.unwrap();
    assert_eq!((agent.id.as_str(), agent.built_in), ("agentcore-glm", true));
    assert!(reason.contains("No agent is set up yet"));
    assert_eq!(
        agents::resolve(&world.store, org, "fake", "auto")
            .await
            .unwrap(),
        "agentcore-glm"
    );
    assert_eq!(
        agents::resolve(&world.store, org, "fake", "codex")
            .await
            .unwrap(),
        "codex"
    );

    assert!(matches!(
        agents::set(&world.store, &editor, T0, "codex", new(true, false)).await,
        Err(Refusal::Denied)
    ));
    assert_eq!(world.audited("agent.set", AuditOutcome::Denied).await, 1);
    for bad in ["auto", "Codex", "-x", ""] {
        assert!(matches!(
            agents::set(&world.store, &world.owner, T0, bad, new(true, false)).await,
            Err(Refusal::Invalid(_))
        ));
    }
    assert!(matches!(
        agents::set(&world.store, &world.owner, T0, "codex", new(false, true)).await,
        Err(Refusal::Invalid(_))
    ));

    agents::set(&world.store, &world.owner, T0, "codex", new(true, false))
        .await
        .unwrap();
    let (only, reason) = agents::auto(&world.store, org, "fake").await.unwrap();
    assert_eq!(
        (only.id.as_str(), reason.as_str()),
        ("codex", "The only agent turned on.")
    );
    agents::set(&world.store, &world.owner, T0, "gemini", new(true, false))
        .await
        .unwrap();
    assert!(matches!(
        agents::auto(&world.store, org, "fake").await,
        Err(Refusal::Invalid(_))
    ));
    agents::set(&world.store, &world.owner, T0, "gemini", new(true, true))
        .await
        .unwrap();
    let (chosen, reason) = agents::auto(&world.store, org, "fake").await.unwrap();
    assert_eq!(chosen.id, "gemini");
    assert!(reason.contains("default"));
    agents::set(&world.store, &world.owner, T0, "gemini", new(false, false))
        .await
        .unwrap();
    agents::set(&world.store, &world.owner, T0, "codex", new(false, false))
        .await
        .unwrap();
    assert!(matches!(
        agents::auto(&world.store, org, "fake").await,
        Err(Refusal::Invalid(_))
    ));
    assert_eq!(
        agents::list(&world.store, &editor, "fake")
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(world.audited("agent.set", AuditOutcome::Done).await, 5);
}
