#![expect(clippy::unwrap_used, reason = "tests")]

use antilog_core::clock::SystemClock;
use antilog_core::fakes::FakeSigner;
use antilog_core::fakes::{FakeMachine, Profile};
use antilog_core::judge::ed25519::Ed25519Signer;
use antilog_core::judge::fixtures::{definition, trial};
use antilog_core::judge::measure::{MeasureError, Refusal, measure};
use antilog_core::judge::receipt::{Context, ReceiptError, VerifyError, issue, verify};
use antilog_core::judge::seal::{PlantedCheat, Proposal, SealError, seal};
use antilog_core::store::judge::{FakeJudgeStore, JudgeStore};
use antilog_types::SessionId;
use antilog_types::judge::*;

fn cheats(rejected_by_holdout_only: bool) -> Vec<PlantedCheat> {
    STANDARD_CHEATS
        .iter()
        .enumerate()
        .map(|(i, name)| PlantedCheat {
            name: (*name).into(),
            commits: vec![CommitRef {
                repo: "app".into(),
                sha: format!("cheat-{i}"),
            }],
            changed: if rejected_by_holdout_only && i == 3 {
                vec![("app".into(), "src/x.rs".into())]
            } else {
                vec![("app".into(), "bench/data.json".into())]
            },
        })
        .collect()
}

fn machine() -> FakeMachine {
    let mut s = FakeMachine::default().with("base", Profile::new(100.0));
    for i in 0..4 {
        s = s.with(&format!("cheat-{i}"), Profile::new(10.0).holdout(100.0));
    }
    s.with("fast", Profile::new(80.0))
        .with("faster", Profile::new(60.0))
        .with("fits-eval", Profile::new(70.0).holdout(123.456))
        .with("slow", Profile::new(120.0))
        .with("same", Profile::new(100.0))
        .with("meh", Profile::new(99.2))
        .with("broken", Profile::new(1.0))
}

fn proposal(def: Definition) -> Proposal {
    Proposal {
        experiment: ExperimentId::new(),
        version: 1,
        definition: def,
        files: vec![],
        cheats: cheats(true),
    }
}

async fn sealed_with(def: Definition) -> (FakeMachine, FakeJudgeStore, SealedExperiment) {
    let (machine, store) = (machine(), FakeJudgeStore::default());
    let sealed = seal(
        &machine,
        SessionId::new(),
        &store,
        &SystemClock,
        proposal(def),
    )
    .await
    .unwrap();
    (machine, store, sealed)
}

fn changed(path: &str) -> Vec<(String, String)> {
    vec![("app".into(), path.into())]
}

#[tokio::test]
async fn sealing_runs_the_checks_and_stores_one_version() {
    let (_, store, sealed) = sealed_with(definition()).await;
    assert!((sealed.checks().baseline() - 100.0).abs() < 1.0);
    assert!(sealed.checks().noise() > 0.0);
    assert_eq!(sealed.checks().cheats().len(), 4);
    let stored = store.sealed(sealed.experiment(), 1).await.unwrap();
    assert_eq!(stored.as_ref(), Some(&sealed));
}

#[tokio::test]
async fn a_cheat_the_evaluation_accepts_blocks_the_seal() {
    let (machine, store) = (
        machine().with("sneaky", Profile::new(10.0)),
        FakeJudgeStore::default(),
    );
    let mut p = proposal(definition());
    p.cheats[3].commits = vec![CommitRef {
        repo: "app".into(),
        sha: "sneaky".into(),
    }];
    let err = seal(&machine, SessionId::new(), &store, &SystemClock, p)
        .await
        .unwrap_err();
    assert!(
        matches!(err, SealError::Refused(SealRefused::CheatNotRejected(_))),
        "{err}"
    );
}

#[tokio::test]
async fn a_missing_standard_cheat_blocks_the_seal() {
    let (machine, store) = (machine(), FakeJudgeStore::default());
    let mut p = proposal(definition());
    p.cheats.pop();
    let err = seal(&machine, SessionId::new(), &store, &SystemClock, p)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SealError::Refused(SealRefused::CheatMissing(_))
    ));
}

#[tokio::test]
async fn a_win_is_confirmed_and_checked_on_the_holdout() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let before = machine.calls().len();
    let record = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "fast"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    let finished = record.finished.unwrap();
    assert_eq!(finished.decision.outcome, Outcome::Kept);
    assert_eq!(finished.decision.criteria, CRITERIA_VERSION);
    let m = finished.measurement.unwrap();
    assert_eq!(
        (m.stage, m.k, m.holdout),
        (Stage::Full, 1, Some(HoldoutResult::Passed))
    );
    assert!(m.confirmation.is_some() && m.gain.unwrap().lower > 0.0);
    let runs = |n: usize, holdout: bool| std::iter::repeat_n(holdout, n);
    assert_eq!(
        machine.calls()[before..],
        runs(2, false)
            .chain(runs(10, false))
            .chain(runs(10, false))
            .chain(runs(10, true))
            .collect::<Vec<_>>(),
        "quick check, full, confirmation in a fresh checkout, holdout: each a base and a trial run per repeat"
    );
}

#[tokio::test]
async fn weak_trials_stop_early_and_are_never_kept() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let before = machine.calls().len();
    let slow = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "slow"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    assert_eq!(slow.outcome(), Outcome::Worse);
    let m = slow.finished.unwrap().measurement.unwrap();
    assert_eq!(m.stage, Stage::QuickCheck);
    let same = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "same"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    assert_eq!(same.outcome(), Outcome::WithinNoise);
    assert_eq!(
        machine.calls().len() - before,
        4,
        "one quick pair each, nothing more"
    );
}

#[tokio::test]
async fn a_trial_that_fits_the_evaluation_fails_the_holdout_and_no_holdout_value_leaks() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let record = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "fits-eval"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    assert_eq!(record.outcome(), Outcome::FailedHoldout);
    let json = serde_json::to_string(&record).unwrap();
    assert!(!json.contains("123.456"));
    assert_eq!(
        record.finished.unwrap().measurement.unwrap().holdout,
        Some(HoldoutResult::Failed)
    );
}

#[tokio::test]
async fn a_protected_path_is_rejected_before_anything_runs_and_has_no_number() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let before = machine.calls().len();
    let record = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "fast"),
        &changed("bench/run.sh"),
    )
    .await
    .unwrap();
    assert_eq!(record.outcome(), Outcome::Rejected);
    assert!(record.finished.unwrap().measurement.is_none());
    assert_eq!(machine.calls().len(), before);
}

#[tokio::test]
async fn a_run_that_fails_is_failed_and_has_no_number() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let machine = machine.failing("broken");
    let record = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "broken"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    assert_eq!(record.outcome(), Outcome::Failed);
    assert!(record.finished.unwrap().measurement.is_none());
}

#[tokio::test]
async fn the_bar_rises_with_each_trial_measured() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    for expect_k in 1..=3 {
        let r = measure(
            &machine,
            &store,
            &sealed,
            trial(&sealed, None, "meh"),
            &changed("src/a.rs"),
        )
        .await
        .unwrap();
        assert_eq!(r.finished.unwrap().measurement.unwrap().k, expect_k);
    }
}

#[tokio::test]
async fn lineage_must_run_through_kept_trials_of_this_version() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let first = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "fast"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    let child = trial(&sealed, Some(first.trial.id), "faster");
    let second = measure(&machine, &store, &sealed, child, &changed("src/a.rs"))
        .await
        .unwrap();
    assert_eq!(second.outcome(), Outcome::Kept);
    assert!(
        (second
            .finished
            .as_ref()
            .unwrap()
            .measurement
            .as_ref()
            .unwrap()
            .parent_value
            - 80.0)
            .abs()
            < 1.0
    );
    let orphan = trial(&sealed, Some(TrialId::new()), "fast");
    assert!(matches!(
        measure(&machine, &store, &sealed, orphan, &[]).await,
        Err(MeasureError::Refused(Refusal::BadParent))
    ));
    let slow = measure(&machine, &store, &sealed, trial(&sealed, None, "slow"), &[])
        .await
        .unwrap();
    let from_unkept = trial(&sealed, Some(slow.trial.id), "fast");
    assert!(matches!(
        measure(&machine, &store, &sealed, from_unkept, &[]).await,
        Err(MeasureError::Refused(Refusal::BadParent))
    ));
}

#[tokio::test]
async fn an_unknown_repo_and_another_version_are_refused() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let mut t = trial(&sealed, None, "fast");
    t.commits[0].repo = "other".into();
    assert!(matches!(
        measure(&machine, &store, &sealed, t, &[]).await,
        Err(MeasureError::Refused(Refusal::UnknownRepo))
    ));
    let mut t = trial(&sealed, None, "fast");
    t.version = 9;
    assert!(matches!(
        measure(&machine, &store, &sealed, t, &[]).await,
        Err(MeasureError::Refused(Refusal::WrongVersion))
    ));
}

#[tokio::test]
async fn patience_and_the_holdout_cap_refuse_measure() {
    let mut def = definition();
    def.patience = 2;
    let (machine, store, sealed) = sealed_with(def).await;
    for _ in 0..2 {
        measure(&machine, &store, &sealed, trial(&sealed, None, "same"), &[])
            .await
            .unwrap();
    }
    assert!(matches!(
        measure(&machine, &store, &sealed, trial(&sealed, None, "fast"), &[]).await,
        Err(MeasureError::Refused(Refusal::PatienceReached(2)))
    ));

    let mut def = definition();
    def.holdout_cap = 1;
    let (machine, store, sealed) = sealed_with(def).await;
    measure(&machine, &store, &sealed, trial(&sealed, None, "fast"), &[])
        .await
        .unwrap();
    assert!(matches!(
        measure(&machine, &store, &sealed, trial(&sealed, None, "fast"), &[]).await,
        Err(MeasureError::Refused(Refusal::HoldoutCapReached(1)))
    ));
}

fn context(digest: &str) -> Context {
    Context {
        value_model: None,
        second_opinion: None,
        spend: Cents(5_000),
        cost_of_win: Cents(5_000),
        data: DataSection {
            datasets: vec![],
            skill_packs: vec![],
            commits: vec![],
            credential_scopes: vec![],
        },
        compute: ComputeSection {
            environment: "default".into(),
            judge_image: "judge:1".into(),
            judge_digest: digest.into(),
            agents: vec![],
        },
    }
}

#[tokio::test]
async fn only_the_best_kept_trial_gets_a_signed_receipt_that_verifies() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let a = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, None, "fast"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    let b = measure(
        &machine,
        &store,
        &sealed,
        trial(&sealed, Some(a.trial.id), "faster"),
        &changed("src/a.rs"),
    )
    .await
    .unwrap();
    let slow = measure(&machine, &store, &sealed, trial(&sealed, None, "slow"), &[])
        .await
        .unwrap();
    let records = store.trials(sealed.experiment(), 1).await.unwrap();
    let signer = Ed25519Signer::from_seed([3; 32]);

    assert!(matches!(
        issue(&sealed, &records, a.trial.id, context("sha256:1"), &signer),
        Err(ReceiptError::NotTheWin)
    ));
    assert!(matches!(
        issue(
            &sealed,
            &records,
            slow.trial.id,
            context("sha256:1"),
            &signer
        ),
        Err(ReceiptError::NotKept)
    ));
    let receipt = issue(&sealed, &records, b.trial.id, context("sha256:1"), &signer).unwrap();
    store.put_receipt(&receipt).await.unwrap();
    assert_eq!(
        store.receipt(b.trial.id).await.unwrap().as_ref(),
        Some(&receipt)
    );

    let e = &receipt.body().experiment;
    assert_eq!(
        (e.holdout, e.trials_tried, e.repeats),
        (HoldoutResult::Passed, 3, 5)
    );
    assert!((e.value - 60.0).abs() < 1.0);
    assert_eq!(e.seal, *sealed.seal_hash());

    assert_eq!(verify(&receipt, &signer, "sha256:1"), Ok(()));
    assert_eq!(
        verify(&receipt, &signer, "sha256:2"),
        Err(VerifyError::DigestMismatch)
    );
    assert_eq!(
        verify(&receipt, &Ed25519Signer::from_seed([4; 32]), "sha256:1"),
        Err(VerifyError::UnknownSigner)
    );

    let tampered = serde_json::to_string(&receipt)
        .unwrap()
        .replace("\"spend\":5000", "\"spend\":1");
    let tampered: Receipt = serde_json::from_str(&tampered).unwrap();
    assert_eq!(
        verify(&tampered, &signer, "sha256:1"),
        Err(VerifyError::BadSignature)
    );
}

#[tokio::test]
async fn a_receipt_must_use_the_sealed_value_model() {
    let (machine, store, sealed) = sealed_with(definition()).await;
    let win = measure(&machine, &store, &sealed, trial(&sealed, None, "fast"), &[])
        .await
        .unwrap();
    let records = store.trials(sealed.experiment(), 1).await.unwrap();
    let mut ctx = context("d");
    ctx.value_model = Some(ValueModel {
        id: ValueModelId::new(),
        version: 1,
        metric: "p95".into(),
        unit: "ms".into(),
        cents_per_unit: 1.0,
        source: ValueSource {
            text: "x".into(),
            link: None,
        },
        set_by: uuid::Uuid::nil(),
    });
    let err = issue(&sealed, &records, win.trial.id, ctx, &FakeSigner::new("k")).unwrap_err();
    assert!(matches!(err, ReceiptError::ValueModelMismatch));
}
