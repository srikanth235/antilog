use std::num::NonZeroU32;

use antilog_types::judge::{CRITERIA_VERSION, Decision, Definition, Guardrail, Outcome, Stats};

use super::stats::{bar_confidence, confidence_95, stats, upper};

/// What the decision reads that is fixed before a trial runs.
#[derive(Debug, Clone, Copy)]
pub struct Criteria<'a> {
    pub quick_check: bool,
    pub noise: f64,
    pub k: NonZeroU32,
    pub guardrails: &'a [Guardrail],
}

impl<'a> Criteria<'a> {
    pub fn new(definition: &'a Definition, noise: f64, k: NonZeroU32) -> Self {
        Self {
            quick_check: definition.quick_check,
            noise,
            k,
            guardrails: &definition.guardrails,
        }
    }
}

/// Paired gains (positive is better) from the stages that have run. `None` has not run.
#[derive(Debug, Clone, Default)]
pub struct Evidence {
    pub touched: Vec<String>,
    pub failure: Option<String>,
    pub quick: Option<f64>,
    pub first: Option<Vec<f64>>,
    /// Paired worsening per guardrail, positive is worse.
    pub guardrails: Vec<(String, Vec<f64>)>,
    pub confirmation: Option<Vec<f64>>,
    pub holdout: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    QuickCheck,
    Full,
    Confirmation,
    Holdout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Needs(Need),
    Done(Decision),
}

enum Class {
    Clears,
    Worse,
    Within,
    Unusable,
}

fn classify(gains: &[f64], k: NonZeroU32) -> (Class, Option<Stats>) {
    let Some(s) = stats(gains, bar_confidence(k)) else {
        return (Class::Unusable, None);
    };
    let class = if s.lower > 0.0 {
        Class::Clears
    } else if upper(&s) < 0.0 {
        Class::Worse
    } else {
        Class::Within
    };
    (class, Some(s))
}

fn done(outcome: Outcome, reason: impl Into<String>) -> Verdict {
    Verdict::Done(Decision {
        outcome,
        reasons: vec![reason.into()],
        criteria: CRITERIA_VERSION,
    })
}

/// The one function that decides Kept. Pure: no I/O, no clock. Call it again with
/// more evidence until it returns `Done`. Kept needs every stage to have run.
pub fn decide(criteria: &Criteria<'_>, evidence: &Evidence) -> Verdict {
    if !evidence.touched.is_empty() {
        let paths = evidence.touched.join(", ");
        return done(
            Outcome::Rejected,
            format!("changed a protected path: {paths}"),
        );
    }
    if let Some(failure) = &evidence.failure {
        return done(Outcome::Failed, format!("did not run: {failure}"));
    }
    if criteria.quick_check {
        match evidence.quick {
            None => return Verdict::Needs(Need::QuickCheck),
            Some(gain) if !gain.is_finite() => {
                return done(
                    Outcome::Failed,
                    "the quick check measured a number that is not finite",
                );
            }
            Some(gain) if gain < -criteria.noise => {
                return done(
                    Outcome::Worse,
                    "quick check: worse than the best by more than the noise",
                );
            }
            Some(gain) if gain <= criteria.noise => {
                return done(
                    Outcome::WithinNoise,
                    "quick check: not better than the best by more than the noise",
                );
            }
            Some(_) => {}
        }
    }
    let Some(first) = &evidence.first else {
        return Verdict::Needs(Need::Full);
    };
    match classify(first, criteria.k).0 {
        Class::Clears => {}
        Class::Worse => return done(Outcome::Worse, "worse than the trial it started from"),
        Class::Within => {
            return done(
                Outcome::WithinNoise,
                "the paired gain does not clear the bar",
            );
        }
        Class::Unusable => {
            return done(
                Outcome::Failed,
                "the measurement has too few or unusable repeats",
            );
        }
    }
    for guardrail in criteria.guardrails {
        let Some((_, worsening)) = evidence
            .guardrails
            .iter()
            .find(|(n, _)| *n == guardrail.name)
        else {
            return done(
                Outcome::Failed,
                format!("guardrail {} was not measured", guardrail.name),
            );
        };
        match stats(worsening, confidence_95()) {
            None => {
                return done(
                    Outcome::Failed,
                    format!("guardrail {} has unusable repeats", guardrail.name),
                );
            }
            Some(s) if s.lower > guardrail.tolerance => {
                return done(
                    Outcome::GuardrailFailed,
                    format!("guardrail {} got worse", guardrail.name),
                );
            }
            Some(_) => {}
        }
    }
    let Some(confirmation) = &evidence.confirmation else {
        return Verdict::Needs(Need::Confirmation);
    };
    match classify(confirmation, criteria.k).0 {
        Class::Clears => {}
        Class::Worse => {
            return done(
                Outcome::Worse,
                "the confirmation in a fresh sandbox was worse",
            );
        }
        Class::Within => {
            return done(
                Outcome::WithinNoise,
                "the confirmation in a fresh sandbox did not clear the bar",
            );
        }
        Class::Unusable => {
            return done(
                Outcome::Failed,
                "the confirmation has too few or unusable repeats",
            );
        }
    }
    let Some(holdout) = &evidence.holdout else {
        return Verdict::Needs(Need::Holdout);
    };
    match classify(holdout, criteria.k) {
        (Class::Clears, _) => done(
            Outcome::Kept,
            "cleared the bar, held every guardrail, confirmed and passed the holdout",
        ),
        (Class::Unusable, _) => done(
            Outcome::Failed,
            "the holdout has too few or unusable repeats",
        ),
        _ => done(
            Outcome::FailedHoldout,
            "did not clear the bar on the holdout",
        ),
    }
}

#[cfg(test)]
mod tests {
    use antilog_types::judge::{Direction, Guardrail};
    use proptest::prelude::*;

    use super::*;

    fn k(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    const GR: [Guardrail; 0] = [];

    fn crit(quick: bool, kk: u32) -> Criteria<'static> {
        Criteria {
            quick_check: quick,
            noise: 1.0,
            k: k(kk),
            guardrails: &GR,
        }
    }

    fn win() -> Vec<f64> {
        vec![5.0, 5.2, 4.9, 5.1, 5.0]
    }

    fn full() -> Evidence {
        Evidence {
            quick: Some(5.0),
            first: Some(win()),
            confirmation: Some(win()),
            holdout: Some(win()),
            ..Evidence::default()
        }
    }

    fn outcome(v: Verdict) -> Outcome {
        match v {
            Verdict::Done(d) => d.outcome,
            Verdict::Needs(n) => panic!("needs {n:?}"),
        }
    }

    #[test]
    fn stages_are_asked_for_in_order() {
        let c = crit(true, 1);
        let mut e = Evidence::default();
        assert_eq!(decide(&c, &e), Verdict::Needs(Need::QuickCheck));
        e.quick = Some(5.0);
        assert_eq!(decide(&c, &e), Verdict::Needs(Need::Full));
        e.first = Some(win());
        assert_eq!(decide(&c, &e), Verdict::Needs(Need::Confirmation));
        e.confirmation = Some(win());
        assert_eq!(decide(&c, &e), Verdict::Needs(Need::Holdout));
        e.holdout = Some(win());
        assert_eq!(outcome(decide(&c, &e)), Outcome::Kept);
    }

    #[test]
    fn a_win_is_kept_and_carries_reasons_and_the_criteria_version() {
        let Verdict::Done(d) = decide(&crit(true, 1), &full()) else {
            panic!()
        };
        assert_eq!((d.outcome, d.criteria), (Outcome::Kept, CRITERIA_VERSION));
        assert!(!d.reasons.is_empty());
    }

    #[test]
    fn the_quick_check_never_keeps() {
        let c = crit(true, 1);
        let e = Evidence {
            quick: Some(50.0),
            ..Evidence::default()
        };
        assert!(matches!(decide(&c, &e), Verdict::Needs(Need::Full)));
        let e = Evidence {
            quick: Some(0.5),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&c, &e)), Outcome::WithinNoise);
        let e = Evidence {
            quick: Some(-1.5),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&c, &e)), Outcome::Worse);
    }

    #[test]
    fn the_quick_check_boundary_is_the_noise() {
        let at = Evidence {
            quick: Some(1.0),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&crit(true, 1), &at)), Outcome::WithinNoise);
        let above = Evidence {
            quick: Some(1.000_001),
            ..Evidence::default()
        };
        assert!(matches!(
            decide(&crit(true, 1), &above),
            Verdict::Needs(Need::Full)
        ));
        let below = Evidence {
            quick: Some(-1.0),
            ..Evidence::default()
        };
        assert_eq!(
            outcome(decide(&crit(true, 1), &below)),
            Outcome::WithinNoise
        );
    }

    #[test]
    fn the_bar_rises_with_k() {
        let borderline = vec![1.0, 1.4, 0.7, 1.1, 0.9];
        let e = Evidence {
            first: Some(borderline),
            ..Evidence::default()
        };
        assert_eq!(
            decide(&crit(false, 1), &e),
            Verdict::Needs(Need::Confirmation)
        );
        assert_eq!(outcome(decide(&crit(false, 400), &e)), Outcome::WithinNoise);
    }

    #[test]
    fn worse_and_within_noise_are_told_apart() {
        let e = Evidence {
            first: Some(vec![-5.0, -5.1, -4.9, -5.0]),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&crit(false, 1), &e)), Outcome::Worse);
        let e = Evidence {
            first: Some(vec![1.0, -1.0, 0.5, -0.5]),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&crit(false, 1), &e)), Outcome::WithinNoise);
    }

    #[test]
    fn a_failed_confirmation_or_holdout_is_never_kept() {
        let bad = vec![0.1, -0.1, 0.2, -0.2, 0.0];
        let c = crit(true, 1);
        let e = Evidence {
            confirmation: Some(bad.clone()),
            ..full()
        };
        assert_eq!(outcome(decide(&c, &e)), Outcome::WithinNoise);
        let e = Evidence {
            holdout: Some(bad),
            ..full()
        };
        assert_eq!(outcome(decide(&c, &e)), Outcome::FailedHoldout);
        let e = Evidence {
            holdout: None,
            ..full()
        };
        assert_eq!(decide(&c, &e), Verdict::Needs(Need::Holdout));
    }

    #[test]
    fn a_protected_path_rejects_before_anything_runs() {
        let e = Evidence {
            touched: vec!["app:bench/a".into()],
            ..full()
        };
        assert_eq!(outcome(decide(&crit(true, 1), &e)), Outcome::Rejected);
    }

    #[test]
    fn a_run_that_failed_is_failed_never_zero() {
        let e = Evidence {
            failure: Some("timeout".into()),
            ..Evidence::default()
        };
        let Verdict::Done(d) = decide(&crit(true, 1), &e) else {
            panic!()
        };
        assert_eq!(d.outcome, Outcome::Failed);
        let e = Evidence {
            first: Some(vec![f64::NAN, 1.0]),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&crit(false, 1), &e)), Outcome::Failed);
        let e = Evidence {
            first: Some(vec![9.0]),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&crit(false, 1), &e)), Outcome::Failed);
    }

    #[test]
    fn guardrails_must_hold_and_be_measured() {
        let g = [Guardrail {
            name: "mem".into(),
            direction: Direction::Minimize,
            tolerance: 1.0,
        }];
        let c = Criteria {
            guardrails: &g,
            ..crit(false, 1)
        };
        let base = Evidence {
            first: Some(win()),
            ..Evidence::default()
        };
        assert_eq!(outcome(decide(&c, &base)), Outcome::Failed);
        let ok = Evidence {
            guardrails: vec![("mem".into(), vec![0.5, 0.6, 0.4, 0.5])],
            ..base.clone()
        };
        assert_eq!(decide(&c, &ok), Verdict::Needs(Need::Confirmation));
        let over = Evidence {
            guardrails: vec![("mem".into(), vec![3.0, 3.1, 2.9, 3.0])],
            ..base
        };
        assert_eq!(outcome(decide(&c, &over)), Outcome::GuardrailFailed);
    }

    proptest! {
        #[test]
        fn nothing_is_kept_unless_every_stage_ran_and_cleared(
            quick in prop::option::of(-10.0f64..10.0),
            first in prop::option::of(prop::collection::vec(-10.0f64..10.0, 2..10)),
            confirmation in prop::option::of(prop::collection::vec(-10.0f64..10.0, 2..10)),
            holdout in prop::option::of(prop::collection::vec(-10.0f64..10.0, 2..10)),
            kk in 1u32..200,
        ) {
            let e = Evidence { quick, first: first.clone(), confirmation: confirmation.clone(), holdout: holdout.clone(), ..Evidence::default() };
            if let Verdict::Done(d) = decide(&crit(true, kk), &e)
                && d.outcome == Outcome::Kept
            {
                {
                    for gains in [first, confirmation, holdout] {
                        let gains = gains.unwrap();
                        let s = stats(&gains, bar_confidence(k(kk))).unwrap();
                        prop_assert!(s.lower > 0.0 && s.mean > 0.0);
                    }
                    prop_assert!(quick.unwrap() > 1.0);
                }
            }
        }
    }
}
