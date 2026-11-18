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
    }
}
