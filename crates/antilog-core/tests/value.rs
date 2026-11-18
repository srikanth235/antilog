#![expect(clippy::unwrap_used, reason = "tests")]

use antilog_core::fakes::FakeStore;
use antilog_core::judge::fixtures;
use antilog_core::organization::{Principal, Refusal};
use antilog_core::seed::{self, T0};
use antilog_core::store::judge::{FakeJudgeStore, JudgeStore};
use antilog_core::store::{ApprovalStore, BudgetStore, OrganizationStore, SessionStore};
use antilog_core::value::{self, Range};
use antilog_types::judge::*;
use antilog_types::{
    AuditEntry, AuditOutcome, Experiment, Identity, Member, Role, Session, Spend, SpendId,
    SpendKind,
};

struct World {
    store: FakeStore,
    judge: FakeJudgeStore,
    owner: Principal,
    experiment: Experiment,
}

impl World {
    async fn new() -> Self {
        let store = FakeStore::default();
        let seed = seed::experiment(&store, 100_000).await;
        Self {
            owner: Principal {
                organization: seed.organization.id,
                member: seed.owner.id,
            },
            experiment: seed.experiment,
            store,
            judge: FakeJudgeStore::default(),
        }
    }

    async fn editor(&self) -> Principal {
        let member = Member::person(
            self.owner.organization,
            "Eve",
            Identity {
                provider: "fake".into(),
                subject: "eve".into(),
            },
            Role::Editor,
            T0,
        );
        self.store
            .add_member(
                &member,
                &AuditEntry::new(
                    self.owner.organization,
                    None,
                    "member.add",
                    member.id,
                    AuditOutcome::Done,
                    T0,
                ),
            )
            .await
            .unwrap();
        Principal {
            organization: self.owner.organization,
            member: member.id,
        }
    }

    async fn set(&self, who: &Principal, per_unit: f64) -> Result<ProjectValueModel, Refusal> {
        value::set_value_model(
            &self.store,
            &self.judge,
            who,
            T0 + 1,
            self.experiment.project_id,
            NewValueModel {
                metric: "p95".into(),
                unit: "ms".into(),
                cents_per_unit: per_unit,
                source: ValueSource {
                    text: "finance".into(),
                    link: Some("https://example.com/study".into()),
                },
            },
        )
        .await
    }

    async fn session(&self, agent: &str, experiment: &Experiment) -> Session {
        let session = Session::new(self.owner.organization, experiment.id, agent, "fake", T0);
        self.store
            .create_session(
                &session,
                &AuditEntry::new(
                    self.owner.organization,
                    None,
                    "session.create",
                    session.id,
                    AuditOutcome::Done,
                    T0,
                ),
            )
            .await
            .unwrap();
        session
    }

    async fn spend(&self, session: &Session, cents: i64, at: i64) {
        self.store
            .record_spend(&Spend {
                id: SpendId::new(),
                organization_id: self.owner.organization,
                session_id: session.id,
                kind: SpendKind::Tokens,
                amount_cents: cents,
                tokens: 10,
                occurred_at: at,
            })
            .await
            .unwrap();
    }

    /// Seals the experiment with the model (or none) and keeps one trial at `value`.
    async fn win(
        &self,
        experiment: &Experiment,
        model: Option<&ProjectValueModel>,
        session: &Session,
        value: f64,
    ) -> TrialId {
        let mut definition = fixtures::definition();
        definition.value_model = model.map(|row| ValueModelRef {
            id: row.model.id,
            version: row.model.version,
        });
        let sealed =
            SealedExperiment::seal(experiment.id, 1, definition, vec![], fixtures::checks(), T0)
                .unwrap();
        self.judge.put_sealed(&sealed).await.unwrap();
        let mut trial = fixtures::trial(&sealed, None, "a");
        trial.session = session.id;
        self.judge.begin_trial(&trial).await.unwrap();
        self.judge
            .finish_trial(
                trial.id,
                &Finished {
                    measurement: Some(Measurement {
                        trial: trial.id,
                        experiment: experiment.id,
                        version: 1,
                        seal: sealed.seal_hash().clone(),
                        stage: Stage::Full,
                        k: 1,
                        noise: 2.0,
                        parent_value: 100.0,
                        value,
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
            .unwrap();
        trial.id
    }

    async fn event(
        &self,
        experiment: &Experiment,
        trial: TrialId,
        kind: WinEventKind,
        value: Option<f64>,
        at: i64,
    ) -> Result<(), Refusal> {
        value::record_win_event(
            &self.store,
            &self.judge,
            self.owner.organization,
            WinEvent {
                experiment: experiment.id,
                version: 1,
                trial,
                kind,
                value,
                link: Some("https://github.com/acme/app/pull/7".into()),
                at,
            },
        )
        .await
    }

    async fn project(&self, range: Range) -> ValueFigures {
        value::project_value(
            &self.store,
            &self.judge,
            &self.owner,
            self.experiment.project_id,
            range,
        )
        .await
        .unwrap()
    }
}

fn all() -> Range {
    Range::all_time(T0 + 1_000_000)
}

// Baseline 100, best 90: a gain of 10 ms. At 360 cents a year per ms that is $36.00.
const WIN_CENTS: i64 = 3_600;

#[tokio::test]
async fn only_a_project_owner_sets_a_value_model_and_each_change_is_a_new_version() {
    let w = World::new().await;
    let eve = w.editor().await;
    assert!(matches!(w.set(&eve, 360.0).await, Err(Refusal::Denied)));
    let denied = w
        .store
        .audit_log(w.owner.organization, None, 50)
        .await
        .unwrap()
        .iter()
        .any(|row| row.action == "value_model.set" && row.outcome == AuditOutcome::Denied);
    assert!(denied, "the refusal is audited");

    let first = w.set(&w.owner, 360.0).await.unwrap();
    assert_eq!(
        (first.model.version, first.model.set_by),
        (1, w.owner.member.as_uuid())
    );
    let second = w.set(&w.owner, 720.0).await.unwrap();
    assert_eq!((second.model.id, second.model.version), (first.model.id, 2));

    let listed = value::value_models(&w.store, &w.judge, &eve, w.experiment.project_id)
        .await
        .unwrap();
    assert_eq!(listed.models, vec![second]);
    assert!(
        w.store
            .audit_log(w.owner.organization, None, 50)
            .await
            .unwrap()
            .iter()
            .any(|row| row.action == "value_model.set" && row.outcome == AuditOutcome::Done)
    );

    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            w.set(&w.owner, bad).await,
            Err(Refusal::Invalid(_))
        ));
    }
}

#[tokio::test]
async fn delivered_needs_a_merge_pending_stays_out_of_return_and_a_miss_counts_in_spend() {
    let w = World::new().await;
    let model = w.set(&w.owner, 360.0).await.unwrap();
    let session = w.session("Claude Code", &w.experiment).await;
    let trial = w.win(&w.experiment, Some(&model), &session, 90.0).await;
    w.spend(&session, 1_800, T0 + 10).await;

    // A second experiment that found nothing still counts in what was spent.
    let miss = Experiment::new(
        w.owner.organization,
        w.experiment.project_id,
        "shrink the bundle",
        10_000,
        w.owner.member,
        T0,
    );
    w.store
        .create_experiment(
            &miss,
            &AuditEntry::new(
                w.owner.organization,
                None,
                "experiment.create",
                miss.id,
                AuditOutcome::Done,
                T0,
            ),
        )
        .await
        .unwrap();
    let idle = w.session("Codex", &miss).await;
    w.spend(&idle, 1_800, T0 + 20).await;

    // No pull request: a win, but nothing delivered or pending.
    let f = w.project(all()).await;
    assert_eq!(
        (f.delivered_cents, f.pending_cents, f.merged_wins),
        (0, 0, 0)
    );
    assert_eq!((f.spent_cents, f.return_x), (3_600, Some(0.0)));
    assert_eq!(f.basis, ValueBasis::MeasuredOffline);

    // Merging needs an open pull request first.
    assert!(matches!(
        w.event(&w.experiment, trial, WinEventKind::Merged, None, T0 + 30)
            .await,
        Err(Refusal::Invalid(_))
    ));
    w.event(&w.experiment, trial, WinEventKind::Opened, None, T0 + 30)
        .await
        .unwrap();
    let f = w.project(all()).await;
    assert_eq!(
        (f.delivered_cents, f.pending_cents, f.pending_wins),
        (0, WIN_CENTS, 1)
    );
    assert_eq!(f.return_x, Some(0.0), "pending is not in Return");

    w.event(&w.experiment, trial, WinEventKind::Merged, None, T0 + 40)
        .await
        .unwrap();
    let f = w.project(all()).await;
    assert_eq!(
        (f.delivered_cents, f.pending_cents, f.merged_wins),
        (WIN_CENTS, 0, 1)
    );
    assert_eq!(
        f.return_x,
        Some(1.0),
        "$36.00 over $36.00 spent, the miss included"
    );

    let org = value::organization_value(&w.store, &w.judge, &w.owner, T0 + 1, all())
        .await
        .unwrap();
    assert_eq!(org, f);

    // A regression moves value from Delivered to At risk; a recovery moves it back.
    w.event(
        &w.experiment,
        trial,
        WinEventKind::Regressed,
        Some(95.0),
        T0 + 50,
    )
    .await
    .unwrap();
    let f = w.project(all()).await;
    assert_eq!(
        (f.delivered_cents, f.at_risk_cents),
        (WIN_CENTS / 2, WIN_CENTS / 2)
    );
    let seen = value::experiment_value(&w.store, &w.judge, &w.owner, w.experiment.id, all())
        .await
        .unwrap();
    assert_eq!(
        seen.wins.first().map(|win| win.state),
        Some(WinState::AtRisk)
    );
    assert_eq!(seen.value_model, Some(model));
    w.event(&w.experiment, trial, WinEventKind::Recovered, None, T0 + 60)
        .await
        .unwrap();
    assert_eq!(w.project(all()).await.at_risk_cents, 0);

    // A time range counts what was merged in it, and spend only from it.
    let before_merge = Range {
        from: 0,
        to: T0 + 35,
    };
    let f = w.project(before_merge).await;
    assert_eq!((f.delivered_cents, f.pending_cents), (0, WIN_CENTS));
    let later = Range {
        from: T0 + 45,
        to: T0 + 1_000,
    };
    let f = w.project(later).await;
    assert_eq!((f.delivered_cents, f.spent_cents, f.return_x), (0, 0, None));
}

#[tokio::test]
async fn with_no_value_model_a_win_counts_in_no_dollar_figure() {
    let w = World::new().await;
    let session = w.session("Claude Code", &w.experiment).await;
    let trial = w.win(&w.experiment, None, &session, 90.0).await;
    w.spend(&session, 500, T0 + 10).await;
    w.event(&w.experiment, trial, WinEventKind::Opened, None, T0 + 20)
        .await
        .unwrap();
    w.event(&w.experiment, trial, WinEventKind::Merged, None, T0 + 30)
        .await
        .unwrap();
    let f = w.project(all()).await;
    assert_eq!(
        (
            f.delivered_cents,
            f.pending_cents,
            f.at_risk_cents,
            f.unvalued_wins
        ),
        (0, 0, 0, 1)
    );
    let seen = value::experiment_value(&w.store, &w.judge, &w.owner, w.experiment.id, all())
        .await
        .unwrap();
    assert_eq!(seen.value_model, None);
    assert_eq!(seen.wins.first().and_then(|win| win.yearly_cents), None);
}

#[tokio::test]
async fn a_fact_must_follow_the_one_before() {
    let w = World::new().await;
    let session = w.session("Claude Code", &w.experiment).await;
    let trial = w.win(&w.experiment, None, &session, 90.0).await;
    let refused = async |kind, value, at| w.event(&w.experiment, trial, kind, value, at).await;
    assert!(
        refused(WinEventKind::Regressed, Some(95.0), 1)
            .await
            .is_err()
    );
    assert!(refused(WinEventKind::Closed, None, 1).await.is_err());
    assert!(refused(WinEventKind::Opened, Some(1.0), 1).await.is_err());
    assert!(refused(WinEventKind::Regressed, None, 1).await.is_err());
    refused(WinEventKind::Opened, None, 1).await.unwrap();
    assert!(refused(WinEventKind::Opened, None, 2).await.is_err());
    refused(WinEventKind::Closed, None, 3).await.unwrap();
    refused(WinEventKind::Opened, None, 4).await.unwrap();
    refused(WinEventKind::Merged, None, 5).await.unwrap();
    assert!(refused(WinEventKind::Recovered, None, 6).await.is_err());
    // A trial that was not Kept has no pull request to report on.
    let stray = w
        .event(&w.experiment, TrialId::new(), WinEventKind::Opened, None, 1)
        .await;
    assert!(matches!(stray, Err(Refusal::Invalid(_))));
}

#[tokio::test]
async fn the_report_adds_up_the_month_and_names_what_found_nothing() {
    let w = World::new().await;
    let model = w.set(&w.owner, 360.0).await.unwrap();
    let session = w.session("Claude Code", &w.experiment).await;
    let trial = w.win(&w.experiment, Some(&model), &session, 90.0).await;
    // 2027-01 starts at 1_798_761_600_000 ms (UTC).
    let month = value::month_range("2027-01").unwrap();
    assert_eq!(month.to - month.from, 31 * 24 * 3600 * 1000);
    let inside = month.from + 1_000;
    w.spend(&session, 1_000, inside).await;
    w.spend(&session, 4_000, month.from - 1).await;
    w.event(&w.experiment, trial, WinEventKind::Opened, None, inside)
        .await
        .unwrap();
    w.event(&w.experiment, trial, WinEventKind::Merged, None, inside + 1)
        .await
        .unwrap();

    let stopped = Experiment::new(
        w.owner.organization,
        w.experiment.project_id,
        "autocomplete latency",
        2_000,
        w.owner.member,
        T0,
    );
    w.store
        .create_experiment(
            &stopped,
            &AuditEntry::new(
                w.owner.organization,
                None,
                "experiment.create",
                stopped.id,
                AuditOutcome::Done,
                T0,
            ),
        )
        .await
        .unwrap();
    let ended = w.session("Codex", &stopped).await;
    // Sealed, tried, nothing kept; its session ended.
    let sealed = SealedExperiment::seal(
        stopped.id,
        1,
        fixtures::definition(),
        vec![],
        fixtures::checks(),
        T0,
    )
    .unwrap();
    w.judge.put_sealed(&sealed).await.unwrap();
    w.store
        .set_experiment_status(
            w.owner.organization,
            stopped.id,
            antilog_types::ExperimentStatus::Sealed,
            &AuditEntry::new(
                w.owner.organization,
                None,
                "experiment.seal",
                stopped.id,
                AuditOutcome::Done,
                T0,
            ),
        )
        .await
        .unwrap();
    w.spend(&ended, 2_000, inside + 5).await;
    w.store
        .update_session(
            w.owner.organization,
            ended.id,
            &antilog_core::store::SessionChange {
                status: Some(antilog_types::SessionStatus::Done),
                ..antilog_core::store::SessionChange::default()
            },
            inside + 6,
            None,
        )
        .await
        .unwrap();

    let report = value::report(&w.store, &w.judge, &w.owner, T0, "2027-01", None)
        .await
        .unwrap();
    assert_eq!(report.month, "2027-01");
    assert_eq!(
        report.figures.spent_cents, 3_000,
        "last month's spend is not in it"
    );
    assert_eq!(report.figures.delivered_cents, WIN_CENTS);
    assert_eq!(report.figures.return_x, Some(1.2));
    assert_eq!(report.spent_with_win_cents, 1_000);
    assert_eq!(report.spent_no_win_cents, 2_000);
    assert_eq!(report.spent_not_sealed_cents, 0);
    assert_eq!(report.spend_by_kind.len(), 1);
    let names: Vec<_> = report.agents.iter().map(|row| row.agent.as_str()).collect();
    assert_eq!(names, vec!["Claude Code", "Codex"]);
    assert_eq!(
        report
            .agents
            .iter()
            .map(|row| row.delivered_cents)
            .collect::<Vec<_>>(),
        vec![WIN_CENTS, 0]
    );
    assert_eq!(report.top_wins.len(), 1);
    assert_eq!(
        report
            .top_wins
            .first()
            .and_then(|win| win.agent.clone())
            .as_deref(),
        Some("Claude Code")
    );
    assert_eq!(
        report
            .stopped
            .iter()
            .map(|row| (row.spent_cents, row.budget_spent))
            .collect::<Vec<_>>(),
        vec![(2_000, true)]
    );

    // A month nobody spent in is empty, not an error; a bad month is refused.
    let empty = value::report(&w.store, &w.judge, &w.owner, T0, "2026-02", None)
        .await
        .unwrap();
    assert_eq!(
        (empty.figures.spent_cents, empty.figures.return_x),
        (0, None)
    );
    assert!(matches!(
        value::report(&w.store, &w.judge, &w.owner, T0, "2027-13", None).await,
        Err(Refusal::Invalid(_))
    ));
}
