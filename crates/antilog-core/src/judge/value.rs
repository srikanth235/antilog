use antilog_types::judge::{Cents, Direction, ValueModel};

/// One win, as value reads it. `regressed_to` is the latest watch value, if it watched.
#[derive(Debug, Clone, PartialEq)]
pub struct WinValue {
    pub direction: Direction,
    pub baseline: f64,
    pub best: f64,
    pub value_model: Option<ValueModel>,
    pub delivery: Delivery,
    pub regressed_to: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    NotOpened,
    Open,
    Merged,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Figures {
    pub delivered: Cents,
    pub pending: Cents,
    pub at_risk: Cents,
    pub spend: Cents,
    /// Delivered over everything spent. `None` when nothing was spent.
    pub return_x: Option<f64>,
    /// Wins with no value model: they show the metric and count in no dollar figure.
    pub unvalued: u32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("a dollar figure overflowed")]
pub struct Overflow;

impl WinValue {
    fn gained(&self) -> f64 {
        self.direction.gain(self.baseline, self.best).max(0.0)
    }

    /// Yearly value of the win, from its sealed value model only.
    pub fn yearly(&self) -> Option<Cents> {
        self.value_model.as_ref()?.yearly(self.gained())
    }

    fn lost(&self) -> Option<Cents> {
        let model = self.value_model.as_ref()?;
        let back = self
            .regressed_to
            .map_or(0.0, |now| self.direction.gain(now, self.best));
        model.yearly(back.max(0.0).min(self.gained()))
    }
}

fn ratio(top: Cents, bottom: Cents) -> Option<f64> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a display ratio, cents stay far below 2^53"
    )]
    (bottom.0 > 0).then(|| top.0 as f64 / bottom.0 as f64)
}

/// Delivered counts a merged pull request only, less what regressed; Pending is apart
/// and out of Return; Return divides by all spend in scope, misses included.
pub fn figures(wins: &[WinValue], spend: Cents) -> Result<Figures, Overflow> {
    let (mut delivered, mut pending, mut at_risk) = (Cents::ZERO, Cents::ZERO, Cents::ZERO);
    let mut unvalued = 0;
    for win in wins {
        let Some(value) = win.yearly() else {
            unvalued += 1;
            continue;
        };
        match win.delivery {
            Delivery::NotOpened => {}
            Delivery::Open => pending = pending.checked_add(value).ok_or(Overflow)?,
            Delivery::Merged => {
                let lost = win.lost().unwrap_or(Cents::ZERO).min(value);
                at_risk = at_risk.checked_add(lost).ok_or(Overflow)?;
                let kept = value.checked_sub(lost).ok_or(Overflow)?;
                delivered = delivered.checked_add(kept).ok_or(Overflow)?;
            }
        }
    }
    Ok(Figures {
        delivered,
        pending,
        at_risk,
        spend,
        return_x: ratio(delivered, spend),
        unvalued,
    })
}

/// One win's own Return: its yearly value over the cost of the win.
pub fn win_return(win: &WinValue, cost_of_win: Cents) -> Option<f64> {
    ratio(win.yearly()?, cost_of_win)
}

#[cfg(test)]
mod tests {
    use antilog_types::judge::{ValueModelId, ValueSource};
    use proptest::prelude::*;
    use uuid::Uuid;

    use super::*;

    fn model(cents_per_unit: f64) -> ValueModel {
        ValueModel {
            id: ValueModelId::new(),
            version: 1,
            metric: "p95".into(),
            cents_per_unit,
            source: ValueSource {
                text: "finance".into(),
                link: None,
            },
            set_by: Uuid::nil(),
        }
    }

    fn win(delivery: Delivery) -> WinValue {
        WinValue {
            direction: Direction::Minimize,
            baseline: 1200.0,
            best: 1100.0,
            value_model: Some(model(3600.0)),
            delivery,
            regressed_to: None,
        }
    }

    #[test]
    fn a_merged_win_is_delivered_and_an_open_one_is_only_pending() {
        let f = figures(
            &[
                win(Delivery::Merged),
                win(Delivery::Open),
                win(Delivery::NotOpened),
            ],
            Cents(1_000_000),
        )
        .unwrap();
        assert_eq!(f.delivered, Cents(360_000));
        assert_eq!(f.pending, Cents(360_000));
        assert_eq!(f.return_x, Some(0.36));
    }

    #[test]
    fn return_counts_every_dollar_spent_including_misses() {
        let f = figures(&[win(Delivery::Merged)], Cents(36_000)).unwrap();
        assert_eq!(f.return_x, Some(10.0));
        let f = figures(&[win(Delivery::Merged)], Cents(360_000)).unwrap();
        assert_eq!(f.return_x, Some(1.0));
        assert_eq!(figures(&[], Cents(500)).unwrap().return_x, Some(0.0));
        assert_eq!(figures(&[], Cents::ZERO).unwrap().return_x, None);
    }

    #[test]
    fn a_regression_moves_value_from_delivered_to_at_risk() {
        let mut w = win(Delivery::Merged);
        w.regressed_to = Some(1150.0);
        let f = figures(&[w.clone()], Cents(1000)).unwrap();
        assert_eq!((f.delivered, f.at_risk), (Cents(180_000), Cents(180_000)));
        w.regressed_to = Some(5000.0);
        let f = figures(&[w], Cents(1000)).unwrap();
        assert_eq!((f.delivered, f.at_risk), (Cents::ZERO, Cents(360_000)));
    }

    #[test]
    fn a_win_with_no_value_model_counts_in_no_dollar_figure() {
        let mut w = win(Delivery::Merged);
        w.value_model = None;
        let f = figures(&[w], Cents(1000)).unwrap();
        assert_eq!((f.delivered, f.unvalued), (Cents::ZERO, 1));
    }

    #[test]
    fn a_win_has_its_own_return() {
        assert_eq!(win_return(&win(Delivery::Open), Cents(36_000)), Some(10.0));
        assert_eq!(win_return(&win(Delivery::Open), Cents::ZERO), None);
    }

    #[test]
    fn overflow_is_an_error_not_a_wrap() {
        let mut w = win(Delivery::Merged);
        w.value_model = Some(model(5.0e16));
        w.baseline = 100.0;
        w.best = 0.0;
        assert_eq!(figures(&[w.clone(), w], Cents(1)), Err(Overflow));
    }

    proptest! {
        #[test]
        fn delivered_plus_at_risk_is_the_merged_value(
            baseline in 100.0f64..2000.0, gain in 0.0f64..99.0, back in 0.0f64..200.0, spend in 1i64..1_000_000,
        ) {
            let w = WinValue {
                direction: Direction::Minimize,
                baseline,
                best: baseline - gain,
                value_model: Some(model(100.0)),
                delivery: Delivery::Merged,
                regressed_to: Some(baseline - gain + back),
            };
            let f = figures(std::slice::from_ref(&w), Cents(spend)).unwrap();
            prop_assert_eq!(f.delivered.0 + f.at_risk.0, w.yearly().unwrap().0);
            prop_assert!(f.delivered.0 >= 0 && f.at_risk.0 >= 0);
            prop_assert_eq!(f.pending, Cents::ZERO);
        }

        #[test]
        fn return_never_ignores_spend(spend_a in 1i64..1_000_000, extra in 1i64..1_000_000) {
            let wins = [win(Delivery::Merged)];
            let a = figures(&wins, Cents(spend_a)).unwrap().return_x.unwrap();
            let b = figures(&wins, Cents(spend_a + extra)).unwrap().return_x.unwrap();
            prop_assert!(b < a);
        }
    }
}
