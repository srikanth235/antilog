use std::num::NonZeroU32;

use antilog_types::judge::{
    CRITERIA_VERSION, CommitRef, Decision, Definition, GuardrailReading, HoldoutResult, Outcome,
    ProtectedFile, SealHash, Stage, Stats,
};

use super::acceptance::{Criteria, Evidence, Need, Verdict, decide};
use crate::platform::{Data, PairedRun, Pairs, Platform};
use super::stats::{bar_confidence, confidence_95, stats};

#[derive(Clone, Copy)]
pub struct Env<'a> {
    pub definition: &'a Definition,
    pub files: &'a [ProtectedFile],
    pub seal: Option<&'a SealHash>,
}

/// Where a trial starts from and what it changed, as full commit sets.
pub struct Sides<'a> {
    pub best: &'a [CommitRef],
    pub parent: &'a [CommitRef],
    pub trial: &'a [CommitRef],
}

/// What one evaluation found. `measured` is false when nothing ran.
pub struct Ran {
    pub measured: bool,
    pub stage: Stage,
    pub parent_value: f64,
    pub value: f64,
    pub gain: Option<Stats>,
    pub confirmation: Option<Stats>,
    pub guardrails: Vec<GuardrailReading>,
    pub holdout: Option<HoldoutResult>,
    pub decision: Decision,
}

fn mean(xs: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = xs.fold((0.0, 0u32), |(s, n), x| (s + x, n + 1));
    if n == 0 { f64::NAN } else { sum / f64::from(n) }
}

fn usable(pairs: &Pairs, repeats: u32) -> Result<(), String> {
    let n = usize::try_from(repeats).unwrap_or(0);
    let all = std::iter::once(&pairs.metric).chain(pairs.guardrails.iter().map(|(_, p)| p));
    for series in all {
        let finite = series.iter().all(|(b, t)| b.is_finite() && t.is_finite());
        if series.len() != n || !finite {
            return Err("the judge sandbox returned an unusable result".into());
        }
    }
    Ok(())
}

/// Runs the stages `decide` asks for, one fresh sandbox each, until it is done.
pub async fn evaluate<S: Platform>(
    sandbox: &S,
    env: Env<'_>,
    k: NonZeroU32,
    noise: f64,
    touched: Vec<String>,
    sides: Sides<'_>,
) -> Ran {
    let def = env.definition;
    let direction = def.metric.direction;
    let criteria = Criteria::new(def, noise, k);
    let mut evidence = Evidence {
        touched,
        ..Evidence::default()
    };
    let mut ran = Ran {
        measured: false,
        stage: Stage::Full,
        parent_value: f64::NAN,
        value: f64::NAN,
        gain: None,
        confirmation: None,
        guardrails: vec![],
        holdout: None,
        decision: Decision {
            outcome: Outcome::Failed,
            reasons: vec![],
            criteria: CRITERIA_VERSION,
        },
    };
    let run = |data, base, repeats| PairedRun {
        seal: env.seal,
        command: &def.evaluation.command,
        files: env.files,
        data,
        base,
        trial: sides.trial,
        repeats,
        guardrails: &def.guardrails,
    };
    loop {
        let need = match decide(&criteria, &evidence) {
            Verdict::Done(decision) => {
                ran.decision = decision;
                break;
            }
            Verdict::Needs(need) => need,
        };
        let (data, base, repeats) = match need {
            Need::QuickCheck => (Data::Evaluation(&def.data.evaluation), sides.best, 1),
            Need::Full | Need::Confirmation => (
                Data::Evaluation(&def.data.evaluation),
                sides.parent,
                def.repeats,
            ),
            Need::Holdout => (Data::Holdout(&def.data.holdout), sides.parent, def.repeats),
        };
        let pairs = match sandbox.run_paired(&run(data, base, repeats)).await {
            Ok(pairs) => usable(&pairs, repeats).map(|()| pairs),
            Err(error) => Err(error.to_string()),
        };
        let pairs = match pairs {
            Ok(pairs) => pairs,
            Err(failure) => {
                evidence.failure = Some(failure);
                continue;
            }
        };
        let gains: Vec<f64> = pairs
            .metric
            .iter()
            .map(|(b, t)| direction.gain(*b, *t))
            .collect();
        match need {
            Need::QuickCheck => {
                ran.measured = true;
                ran.stage = Stage::QuickCheck;
                ran.parent_value = mean(pairs.metric.iter().map(|p| p.0));
                ran.value = mean(pairs.metric.iter().map(|p| p.1));
                evidence.quick = gains.first().copied();
            }
            Need::Full => {
                ran.measured = true;
                ran.stage = Stage::Full;
                ran.parent_value = mean(pairs.metric.iter().map(|p| p.0));
                ran.value = mean(pairs.metric.iter().map(|p| p.1));
                ran.gain = stats(&gains, bar_confidence(k));
                for guardrail in &def.guardrails {
                    let worsening: Vec<f64> = pairs
                        .guardrails
                        .iter()
                        .find(|(name, _)| *name == guardrail.name)
                        .map(|(_, p)| {
                            p.iter()
                                .map(|(b, t)| guardrail.direction.gain(*t, *b))
                                .collect()
                        })
                        .unwrap_or_default();
                    if let Some(worsening_stats) = stats(&worsening, confidence_95()) {
                        ran.guardrails.push(GuardrailReading {
                            name: guardrail.name.clone(),
                            worsening: worsening_stats,
                        });
                    }
                    evidence
                        .guardrails
                        .push((guardrail.name.clone(), worsening));
                }
                evidence.first = Some(gains);
            }
            Need::Confirmation => {
                ran.confirmation = stats(&gains, bar_confidence(k));
                evidence.confirmation = Some(gains);
            }
            Need::Holdout => {
                let passed = stats(&gains, bar_confidence(k)).is_some_and(|s| s.lower > 0.0);
                ran.holdout = Some(if passed {
                    HoldoutResult::Passed
                } else {
                    HoldoutResult::Failed
                });
                evidence.holdout = Some(gains);
            }
        }
    }
    ran
}
