//! Sample sealed experiments for tests and the store suite.
#![expect(
    clippy::expect_used,
    reason = "test support: a bad fixture fails the test"
)]

use antilog_types::SessionId;
use antilog_types::judge::*;

pub fn definition() -> Definition {
    Definition {
        metric: Metric {
            name: "p95".into(),
            unit: "ms".into(),
            direction: Direction::Minimize,
        },
        kind: EvaluationKind::Speed,
        evaluation: Evaluation {
            command: "make bench".into(),
            repos: vec![RepoPin {
                repo: "app".into(),
                commit: "base".into(),
            }],
            protected: vec![ProtectedPath {
                repo: "app".into(),
                glob: "bench/**".into(),
            }],
        },
        guardrails: vec![],
        data: DataRefs {
            evaluation: DatasetRef {
                id: "checkout".into(),
                version: 1,
            },
            holdout: DatasetRef {
                id: "checkout-holdout".into(),
                version: 1,
            },
        },
        repeats: 5,
        quick_check: true,
        patience: 6,
        holdout_cap: 20,
        value_model: None,
        criteria: CRITERIA_VERSION,
    }
}

pub fn checks() -> SealChecks {
    let cheats = STANDARD_CHEATS
        .iter()
        .map(|name| CheatCheck {
            name: (*name).into(),
            rejected: true,
        })
        .collect();
    SealChecks::passed(100.0, 2.0, cheats).expect("the fixture checks pass")
}

pub fn sealed_with(definition: Definition) -> SealedExperiment {
    SealedExperiment::seal(ExperimentId::new(), 1, definition, vec![], checks(), 0)
        .expect("the fixture definition is sealable")
}

pub fn sealed() -> SealedExperiment {
    sealed_with(definition())
}

pub fn sealed_for(experiment: ExperimentId) -> SealedExperiment {
    SealedExperiment::seal(experiment, 1, definition(), vec![], checks(), 0)
        .expect("the fixture definition is sealable")
}

pub fn value_model(version: u32) -> ValueModel {
    ValueModel {
        id: ValueModelId::new(),
        version,
        metric: "p95".into(),
        unit: "ms".into(),
        cents_per_unit: 360.0,
        source: ValueSource {
            text: "finance estimate".into(),
            link: None,
        },
        set_by: uuid::Uuid::now_v7(),
    }
}

pub fn receipt(sealed: &SealedExperiment, trial: TrialId) -> Receipt {
    let body = ReceiptBody {
        id: ReceiptId::new(),
        experiment: ExperimentSection {
            experiment: sealed.experiment(),
            version: sealed.version(),
            seal: sealed.seal_hash().clone(),
            trial,
            value_model: None,
            baseline: 100.0,
            value: 90.0,
            repeats: 5,
            noise: 2.0,
            trials_tried: 1,
            holdout: HoldoutResult::Passed,
            second_opinion: None,
            spend: Cents(100),
            cost_of_win: Cents(100),
            win_return: None,
        },
        data: DataSection {
            datasets: vec![],
            skill_packs: vec![],
            commits: vec![],
            credential_scopes: vec![],
        },
        compute: ComputeSection {
            environment: "default".into(),
            judge_image: "judge".into(),
            judge_digest: "sha256:0".into(),
            agents: vec![],
        },
    };
    Receipt::sign(body, |_| ("fixture".into(), vec![1, 2, 3])).expect("the fixture body signs")
}

pub fn trial(sealed: &SealedExperiment, parent: Option<TrialId>, sha: &str) -> Trial {
    Trial {
        id: TrialId::new(),
        experiment: sealed.experiment(),
        version: sealed.version(),
        session: SessionId::new(),
        parent,
        commits: vec![CommitRef {
            repo: "app".into(),
            sha: sha.into(),
        }],
        hypothesis: format!("try {sha}"),
        at: 0,
    }
}
