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

/// What every `CardStore` must do: write a proposal once, apply a card once.
pub async fn card_suite<S: Store>(store: &S) {
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
    let session = Session::new(org, seed.experiment.id, "claude", T0);
    store
        .create_session(&session, &log("session.start"))
        .await
        .expect("a session is created");
    let approval = Approval {
        id: ApprovalId::new(),
        organization_id: org,
        experiment_id: seed.experiment.id,
        session_id: Some(session.id),
        card: CardKind::SealExperiment,
        summary: "Seal p95".into(),
        payload: None,
        cost_cents: None,
        needs_owner: false,
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
    assert!(store.mark_applied(org, approval.id).await.expect("marked"));
    assert!(!store.mark_applied(org, approval.id).await.expect("marked"));
}

/// What the win-delivery tables must do on any store, in the ledger and the judge's tables together:
/// pull requests found by name, a delivery marked once, spend that names its trial, a card's
/// typed fields kept, and a value model that lands with its audit row or not at all.
pub async fn wins_suite<S: Store, J: crate::store::judge::JudgeStore>(
    store: &S,
    judge_for: impl Fn(antilog_types::OrganizationId) -> J,
) {
    use antilog_types::judge::{PullRequestState, SealedExperiment, ValueSource};
    use antilog_types::{
        Approval, ApprovalId, ApprovalStatus, CardKind, CardPayload, RepoAccess, Session, Spend,
        SpendId, SpendKind,
    };

    use crate::judge::fixtures;
    use crate::store::{PullRequestRow, StoreError};

    let seed = experiment(store, 10_000).await;
    let org = seed.organization.id;
    let judge = judge_for(org);
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

    let sealed = SealedExperiment::seal(
        seed.experiment.id,
        1,
        fixtures::definition(),
        vec![],
        fixtures::checks(),
        T0,
    )
    .expect("seals");
    judge.put_sealed(&sealed).await.expect("sealed");
    let session = Session::new(org, seed.experiment.id, "claude", T0);
    store
        .create_session(&session, &log("session.create"))
        .await
        .expect("session");
    let mut trial = fixtures::trial(&sealed, None, "a");
    trial.session = session.id;
    trial.at = T0 + 50;
    judge.begin_trial(&trial).await.expect("trial");
    assert_eq!(
        judge
            .trial(trial.id)
            .await
            .expect("read")
            .map(|r| r.trial.at),
        Some(T0 + 50),
        "a trial keeps when it ran"
    );

    // Pull requests are found by the code host's own name, without an organization.
    let row = |number: u64, state| PullRequestRow {
        organization: org,
        repo: "acme/app".into(),
        number,
        experiment: seed.experiment.id,
        version: 1,
        trial: trial.id,
        url: format!("https://code.example/acme/app/pull/{number}"),
        state,
    };
    assert_eq!(store.pull_request("acme/app", 1).await.expect("read"), None);
    store
        .put_pull_requests(
            org,
            &[
                row(1, PullRequestState::Open),
                row(2, PullRequestState::Open),
            ],
        )
        .await
        .expect("put");
    assert!(matches!(
        store
            .put_pull_requests(org, &[row(2, PullRequestState::Open)])
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.pull_request("acme/app", 2).await.expect("read"),
        Some(row(2, PullRequestState::Open))
    );
    assert_eq!(store.pull_request("acme/app", 3).await.expect("read"), None);
    store
        .set_pull_request_state(org, "acme/app", 2, PullRequestState::Merged)
        .await
        .expect("state");
    let listed = store.pull_requests_of(org, trial.id).await.expect("list");
    assert_eq!(
        listed
            .iter()
            .map(|r| (r.number, r.state))
            .collect::<Vec<_>>(),
        vec![(1, PullRequestState::Open), (2, PullRequestState::Merged)]
    );
    assert!(matches!(
        store
            .set_pull_request_state(org, "acme/app", 9, PullRequestState::Closed)
            .await,
        Err(StoreError::NotFound)
    ));

    assert!(!store.delivery_seen(org, "d-1").await.expect("read"));
    store.put_delivery(org, "d-1", T0).await.expect("mark");
    store
        .put_delivery(org, "d-1", T0)
        .await
        .expect("marking twice is fine");
    assert!(store.delivery_seen(org, "d-1").await.expect("read"));
    assert!(!store.delivery_seen(org, "d-2").await.expect("read"));

    // Spend may name the trial it paid for, and the rows come back with it.
    let spend = |cents, at, trial_id| Spend {
        id: SpendId::new(),
        organization_id: org,
        session_id: session.id,
        kind: SpendKind::Tokens,
        amount_cents: cents,
        tokens: 1,
        occurred_at: at,
        trial_id,
    };
    store
        .record_spend(&spend(30, T0 + 10, None))
        .await
        .expect("spend");
    store
        .record_spend(&spend(70, T0 + 60, Some(trial.id)))
        .await
        .expect("spend");
    let rows = store
        .spend_rows_between(org, T0, T0 + 1_000)
        .await
        .expect("rows");
    assert_eq!(
        rows.iter()
            .map(|r| (r.cents, r.at, r.trial))
            .collect::<Vec<_>>(),
        vec![(30, T0 + 10, None), (70, T0 + 60, Some(trial.id))]
    );
    assert_eq!(
        store
            .spend_rows_between(org, T0 + 20, T0 + 1_000)
            .await
            .expect("rows")
            .len(),
        1
    );

    // A card keeps its typed fields.
    let card = Approval {
        id: ApprovalId::new(),
        organization_id: org,
        experiment_id: seed.experiment.id,
        session_id: Some(session.id),
        card: CardKind::AttachRepo,
        summary: "Attach repo acme/app".into(),
        payload: Some(CardPayload::AttachRepo {
            repo: "acme/app".into(),
            access: RepoAccess::Push,
            branch: Some("antilog/p95".into()),
        }),
        cost_cents: None,
        needs_owner: false,
        status: ApprovalStatus::Pending,
        decided_by: None,
        created_at: T0,
        decided_at: None,
    };
    store
        .request_approval(&card, &log("approval.raise"))
        .await
        .expect("card");
    assert_eq!(
        store.approval(org, card.id).await.expect("read"),
        Some(card)
    );

    // A value model and its audit row land together, or neither does.
    let model = |version| antilog_types::judge::ValueModel {
        id: antilog_types::judge::ValueModelId::from_uuid(uuid::Uuid::from_u128(7)),
        version,
        metric: "p95".into(),
        unit: "ms".into(),
        cents_per_unit: 100.0,
        source: ValueSource {
            text: "finance".into(),
            link: None,
        },
        set_by: seed.owner.id.as_uuid(),
    };
    let audited = |target: &str| {
        AuditEntry::new(
            org,
            Some(seed.owner.id),
            "value_model.set",
            target,
            AuditOutcome::Done,
            T0,
        )
    };
    let count = |entries: Vec<AuditEntry>| {
        entries
            .iter()
            .filter(|e| e.action == "value_model.set")
            .count()
    };
    let project = seed.experiment.project_id;
    judge
        .put_value_model_audited(project, &model(1), T0, &audited("p95@1"))
        .await
        .expect("first version");
    assert_eq!(
        count(store.audit_log(org, None, 100).await.expect("log")),
        1
    );
    let again = judge
        .put_value_model_audited(project, &model(1), T0, &audited("p95@1 again"))
        .await;
    assert!(again.is_err(), "a version that exists is refused");
    assert_eq!(
        count(store.audit_log(org, None, 100).await.expect("log")),
        1,
        "a refused write leaves no audit row"
    );
}

/// A Kept trial, its signed receipt and an open pull request, made without a running agent. Only
/// for the server on its fakes, so an end-to-end test can start from "a win exists" and then
/// let the code host's webhook merge it: the same sealing, receipt and pull-request code runs.
#[derive(Debug, Clone)]
pub struct SeededWin {
    pub trial: antilog_types::judge::TrialId,
    pub repo: String,
    pub pull_requests: Vec<(String, u64)>,
}

pub async fn kept_win<D: crate::deps::Deps>(
    deps: &D,
    organization: antilog_types::OrganizationId,
    experiment: antilog_types::ExperimentId,
    actor: Option<antilog_types::MemberId>,
) -> Result<SeededWin, String> {
    use antilog_types::judge::{
        Decision, Finished, HoldoutResult, Measurement, Outcome, RepoPin, SealedExperiment, Stage,
        Trial, TrialId, ValueModelRef,
    };
    use antilog_types::{
        Approval, ApprovalId, ApprovalStatus, CardKind, CardPayload, ExperimentStatus,
    };

    use crate::judge::fixtures;
    use crate::store::SessionStore as _;
    use crate::store::judge::JudgeStore;

    let now = crate::session::now_ms(deps);
    let store = deps.store();
    let judge = deps.judge(organization);
    let row = store
        .experiment(organization, experiment)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no such experiment")?;
    let repo = format!("e2e/{}", experiment.as_uuid().simple());
    let mut definition = fixtures::definition();
    definition.evaluation.repos = vec![RepoPin {
        repo: repo.clone(),
        commit: "base".into(),
    }];
    for path in &mut definition.evaluation.protected {
        path.repo.clone_from(&repo);
    }
    definition.value_model = judge
        .value_models(row.project_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|m| m.model.metric == definition.metric.name)
        .map(|m| ValueModelRef {
            id: m.model.id,
            version: m.model.version,
        });
    let mut version = 1;
    while judge
        .sealed(experiment, version)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        version += 1;
    }
    let sealed = SealedExperiment::seal(
        experiment,
        version,
        definition,
        vec![],
        fixtures::checks(),
        now,
    )
    .map_err(|e| format!("{e:?}"))?;
    judge.put_sealed(&sealed).await.map_err(|e| e.to_string())?;
    let entry = AuditEntry::new(
        organization,
        actor,
        "experiment.seal",
        experiment,
        AuditOutcome::Done,
        now,
    );
    store
        .set_experiment_status(organization, experiment, ExperimentStatus::Sealed, &entry)
        .await
        .map_err(|e| e.to_string())?;

    let session = crate::session::open_external(deps, organization, experiment, actor)
        .await
        .map_err(|e| e.to_string())?;
    let trial = Trial {
        id: TrialId::new(),
        experiment,
        version,
        session: session.id,
        parent: None,
        commits: vec![antilog_types::judge::CommitRef {
            repo: repo.clone(),
            sha: "fast".into(),
        }],
        hypothesis: "cache the cart".into(),
        at: now,
    };
    judge.begin_trial(&trial).await.map_err(|e| e.to_string())?;
    judge
        .finish_trial(
            trial.id,
            &Finished {
                measurement: Some(Measurement {
                    trial: trial.id,
                    experiment,
                    version,
                    seal: sealed.seal_hash().clone(),
                    stage: Stage::Full,
                    k: 1,
                    noise: 2.0,
                    parent_value: sealed.checks().baseline(),
                    value: 80.0,
                    gain: None,
                    confirmation: None,
                    guardrails: vec![],
                    holdout: Some(HoldoutResult::Passed),
                }),
                decision: Decision {
                    outcome: Outcome::Kept,
                    reasons: vec![],
                    criteria: 1,
                },
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    let record = judge
        .trial(trial.id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("the trial was lost")?;
    let caller = crate::tools::Caller {
        organization,
        experiment,
        session: session.id,
        actor,
        call: "seed".into(),
    };
    if !crate::tools::issue_receipt(deps, &caller, &sealed, &record).await {
        return Err("the receipt could not be signed".into());
    }
    let card = Approval {
        id: ApprovalId::new(),
        organization_id: organization,
        experiment_id: experiment,
        session_id: Some(session.id),
        card: CardKind::OpenPullRequest,
        summary: "Open pull requests".into(),
        payload: Some(CardPayload::OpenPullRequest {
            trial: trial.id,
            hypothesis: trial.hypothesis.clone(),
            repos: vec![repo.clone()],
        }),
        cost_cents: None,
        needs_owner: false,
        status: ApprovalStatus::Approved,
        decided_by: actor,
        created_at: now,
        decided_at: Some(now),
    };
    crate::delivery::open_approved(deps, &card)
        .await
        .map_err(|e| e.to_string())?;
    let pull_requests =
        crate::store::CodeHostStore::pull_requests_of(store, organization, trial.id)
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|row| (row.repo, row.number))
            .collect();
    Ok(SeededWin {
        trial: trial.id,
        repo,
        pull_requests,
    })
}

/// What every `RunnerStore` must do: lease sessions to a runner, hand each
/// command over once, send an unfinished one again after a requeue, and keep results and exports.
pub async fn runner_suite<S: Store>(store: &S) {
    use antilog_types::{
        Command, MachineState, Reply, RunResult, Seed, Session, SessionId, SessionStatus,
    };
    use uuid::Uuid;

    use crate::store::{SessionChange, StoreError};

    let mut sessions = Vec::new();
    // A runner serves one organization, so both sessions are in the same one.
    let seed = experiment(store, 1_000).await;
    for _ in 0..2 {
        let org = seed.organization.id;
        let session = Session::new(org, seed.experiment.id, "opencode", T0);
        let log = AuditEntry::new(
            org,
            Some(seed.owner.id),
            "session.start",
            session.id,
            AuditOutcome::Done,
            T0,
        );
        store
            .create_session(&session, &log)
            .await
            .expect("a session is created");
        let seed_of = Seed {
            repos: vec!["acme/api".into()],
            environment: None,
        };
        store
            .enqueue_machine(org, session.id, &seed_of, T0)
            .await
            .expect("queued");
        assert!(matches!(
            store.enqueue_machine(org, session.id, &seed_of, T0).await,
            Err(StoreError::Conflict)
        ));
        sessions.push((org, session));
    }
    let mut both = sessions.into_iter();
    let (org, first) = both.next().expect("two sessions");
    let (_, second) = both.next().expect("two sessions");
    let (org, first, second) = (&org, &first, &second);
    let queued = store
        .machine(first.id)
        .await
        .expect("read")
        .expect("a machine");
    assert_eq!(queued.state, MachineState::Queued);
    assert_eq!(queued.seed.repos, vec!["acme/api".to_owned()]);
    assert_eq!(queued.agent, "opencode");
    assert_eq!(
        store
            .machine(Session::new(*org, first.experiment_id, "x", T0).id)
            .await
            .expect("read"),
        None
    );

    // One runner takes work up to its limit, once each.
    let claimed = store.claim("r1", 1, 1_000, 61_000).await.expect("claim");
    assert_eq!(claimed.len(), 1);
    let mine = claimed.first().expect("one");
    assert_eq!(mine.state, MachineState::Claimed);
    assert_eq!(mine.holder.as_deref(), Some("r1"));
    let rest = store.claim("r1", 5, 1_000, 61_000).await.expect("claim");
    assert_eq!(rest.len(), 1);
    assert_ne!(rest.first().expect("one").session, mine.session);
    assert!(
        store
            .claim("r2", 5, 1_000, 61_000)
            .await
            .expect("claim")
            .is_empty()
    );

    // A lease is renewed by its holder and by nobody else.
    assert_eq!(
        store
            .renew("r1", 30_000, 90_000)
            .await
            .expect("renew")
            .len(),
        2
    );
    assert!(
        store
            .renew("r2", 30_000, 90_000)
            .await
            .expect("renew")
            .is_empty()
    );

    // Commands go to the holder once, oldest first.
    let message = Command::Message {
        text: "hello".into(),
    };
    let one = store
        .push_command(*org, first.id, &message, T0)
        .await
        .expect("push");
    let two = store
        .push_command(*org, first.id, &Command::Abort, T0)
        .await
        .expect("push");
    assert!(store.take_commands("r2").await.expect("take").is_empty());
    let taken = store.take_commands("r1").await.expect("take");
    assert_eq!(
        taken.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![one, two]
    );
    assert_eq!(taken.first().expect("one").command, message);
    assert!(store.take_commands("r1").await.expect("take").is_empty());
    assert!(matches!(
        store
            .push_command(*org, SessionId::new(), &Command::Abort, T0)
            .await,
        Err(StoreError::NotFound)
    ));

    // A finished command is not sent again after a requeue; an unfinished one is.
    store.finish_commands("r1", &[one]).await.expect("finish");
    store.finish_commands("r2", &[two]).await.expect("finish");
    assert_eq!(
        store
            .claim("r2", 5, 200_000, 260_000)
            .await
            .expect("claim")
            .len(),
        2
    );
    let again = store.take_commands("r2").await.expect("take");
    let ids: Vec<Uuid> = again
        .iter()
        .filter(|e| e.session == first.id)
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, vec![two]);
    assert!(
        !store
            .set_machine_state(first.id, "r1", MachineState::Idle, 200_000)
            .await
            .expect("state"),
        "the lapsed holder no longer holds it"
    );

    // A command's outcome is kept for the one waiting.
    let run = Command::Approval {
        permission: "p1".into(),
        reply: Reply::Once,
    };
    let run_id = store
        .push_command(*org, first.id, &run, T0)
        .await
        .expect("push");
    assert_eq!(store.result(run_id).await.expect("read"), None);
    let result = RunResult {
        command: run_id,
        exit: 0,
        output: "ok\n".into(),
    };
    assert!(!store.put_result(second.id, &result).await.expect("put"));
    assert!(store.put_result(first.id, &result).await.expect("put"));
    assert_eq!(store.result(run_id).await.expect("read"), Some(result));
    store.drop_command(run_id).await.expect("drop");
    assert_eq!(store.result(run_id).await.expect("read"), None);

    // Idle, then let go with an export; a later message queues it again to resume.
    assert!(
        store
            .set_machine_state(first.id, "r2", MachineState::Idle, 200_000)
            .await
            .expect("state")
    );
    assert!(!store.wake_machine(first.id, 200_000).await.expect("wake"));
    store
        .put_export(*org, first.id, "{\"export\":1}", 200_000)
        .await
        .expect("export");
    assert!(
        store
            .set_machine_state(first.id, "r2", MachineState::Released, 200_000)
            .await
            .expect("state")
    );
    assert!(
        store
            .machine(first.id)
            .await
            .expect("read")
            .expect("row")
            .holder
            .is_none()
    );
    assert!(
        store
            .take_commands("r2")
            .await
            .expect("take")
            .iter()
            .all(|e| e.session != first.id)
    );
    assert!(store.wake_machine(first.id, 210_000).await.expect("wake"));
    let woken = store.machine(first.id).await.expect("read").expect("row");
    assert_eq!((woken.state, woken.resume), (MachineState::Queued, true));
    assert_eq!(
        store.export(first.id).await.expect("read").as_deref(),
        Some("{\"export\":1}")
    );
    assert_eq!(store.export(second.id).await.expect("read"), None);

    // A session that has ended is not claimed again.
    let ended = SessionChange {
        status: Some(SessionStatus::Done),
        note: None,
    };
    store
        .update_session(*org, first.id, &ended, 220_000, None)
        .await
        .expect("done");
    assert!(
        store
            .claim("r3", 5, 220_000, 280_000)
            .await
            .expect("claim")
            .is_empty()
    );
}

/// A runner locks to the organization of the first session it claims, and claims only that
/// organization's sessions until it exits (`unlock_runner`), so customers never share a host.
pub async fn runner_lock_suite<S: Store>(store: &S) {
    use antilog_types::{MachineState, Seed, Session};

    let mut queued = Vec::new();
    for _ in 0..2 {
        let seed = experiment(store, 1_000).await;
        let org = seed.organization.id;
        for _ in 0..2 {
            let session = Session::new(org, seed.experiment.id, "opencode", T0);
            let log = AuditEntry::new(
                org,
                Some(seed.owner.id),
                "session.start",
                session.id,
                AuditOutcome::Done,
                T0,
            );
            store
                .create_session(&session, &log)
                .await
                .expect("a session is created");
            let seed_of = Seed {
                repos: Vec::new(),
                environment: None,
            };
            store
                .enqueue_machine(org, session.id, &seed_of, T0)
                .await
                .expect("queued");
            queued.push((org, session.id));
        }
    }
    let at = |n: usize| *queued.get(n).expect("four sessions");
    let (org_a, org_b) = (at(0).0, at(2).0);
    assert_ne!(org_a, org_b);

    // A runner with no room takes nothing and locks to nothing.
    assert!(
        store
            .claim("busy", 0, 1_000, 61_000)
            .await
            .expect("claim")
            .is_empty()
    );

    // The first claim locks r1 to that session's organization; it never takes the other's.
    let first = store.claim("r1", 1, 1_000, 61_000).await.expect("claim");
    assert_eq!(first.len(), 1);
    let locked_to = first.first().expect("one").organization;
    let (other, other_org) = if locked_to == org_a {
        (at(2).1, org_b)
    } else {
        (at(0).1, org_a)
    };
    let more = store.claim("r1", 10, 1_000, 61_000).await.expect("claim");
    assert_eq!(more.len(), 1, "the rest of its own organization's sessions");
    assert!(more.iter().all(|m| m.organization == locked_to));
    assert!(
        store
            .claim("r1", 10, 1_000, 61_000)
            .await
            .expect("claim")
            .is_empty()
    );
    let b = store.machine(other).await.expect("read").expect("row");
    assert_eq!((b.state, b.organization), (MachineState::Queued, other_org));

    // Another runner takes the other organization, and only that.
    let theirs = store.claim("r2", 10, 1_000, 61_000).await.expect("claim");
    assert_eq!(theirs.len(), 2);
    assert!(theirs.iter().all(|m| m.organization == other_org));

    // Everything is let go and queued again, both organizations' sessions.
    for (holder, held) in [
        ("r1", first.iter().chain(more.iter()).collect::<Vec<_>>()),
        ("r2", theirs.iter().collect()),
    ] {
        for machine in held {
            assert!(
                store
                    .set_machine_state(machine.session, holder, MachineState::Released, 2_000)
                    .await
                    .expect("state")
            );
            assert!(
                store
                    .wake_machine(machine.session, 2_000)
                    .await
                    .expect("wake")
            );
        }
    }

    // The lock outlives the leases: r2 holds nothing, but the other organization's sessions are
    // still not its to take. r1 takes one of its own.
    let again = store.claim("r1", 1, 3_000, 63_000).await.expect("claim");
    assert_eq!(again.len(), 1);
    assert_eq!(again.first().expect("one").organization, locked_to);
    let b_again = store.claim("r2", 10, 3_000, 63_000).await.expect("claim");
    assert_eq!(b_again.len(), 2);
    assert!(b_again.iter().all(|m| m.organization == other_org));

    // A runner that still holds a session is not unlocked by saying it is leaving.
    store.unlock_runner("r1").await.expect("unlock");
    let rest = store.claim("r1", 10, 3_100, 63_100).await.expect("claim");
    assert!(rest.iter().all(|m| m.organization == locked_to));

    // Once it holds nothing, leaving lets the lock go, and it locks afresh to whoever is next.
    for machine in again.iter().chain(rest.iter()) {
        store
            .set_machine_state(machine.session, "r1", MachineState::Done, 4_000)
            .await
            .expect("state");
    }
    // The other organization's sessions wait again.
    for machine in &b_again {
        store
            .set_machine_state(machine.session, "r2", MachineState::Released, 4_050)
            .await
            .expect("state");
        assert!(
            store
                .wake_machine(machine.session, 4_050)
                .await
                .expect("wake")
        );
    }
    assert!(
        store
            .claim("r1", 10, 4_100, 64_100)
            .await
            .expect("claim")
            .is_empty(),
        "r1 is still locked to the first organization, which has nothing queued"
    );
    store.unlock_runner("r1").await.expect("unlock");
    let afresh = store.claim("r1", 10, 4_200, 64_200).await.expect("claim");
    assert_eq!(afresh.len(), 2);
    assert!(afresh.iter().all(|m| m.organization == other_org));
}
