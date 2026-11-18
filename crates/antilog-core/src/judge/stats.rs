use std::num::NonZeroU32;

use antilog_types::judge::{ALPHA, Stats};

fn ln_gamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let t = x + 7.5;
    let sum = G.iter().enumerate().skip(1).fold(G[0], |acc, (i, g)| {
        acc + g / (x + f64::from(u8::try_from(i).unwrap_or(0)))
    });
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + sum.ln()
}

fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..500 {
        let m = f64::from(m);
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-15 {
            break;
        }
    }
    h
}

/// Regularized incomplete beta function.
fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front =
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_cf(a, b, x) / a
    } else {
        1.0 - front * beta_cf(b, a, 1.0 - x) / b
    }
}

pub fn t_cdf(t: f64, df: f64) -> f64 {
    let tail = inc_beta(df / 2.0, 0.5, df / (df + t * t)) / 2.0;
    if t > 0.0 { 1.0 - tail } else { tail }
}

pub fn t_quantile(p: f64, df: f64) -> f64 {
    if p < 0.5 {
        return -t_quantile(1.0 - p, df);
    }
    if p == 0.5 {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    while t_cdf(hi, df) < p && hi < 1e15 {
        lo = hi;
        hi *= 2.0;
    }
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if t_cdf(mid, df) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// The confidence the k-th measured trial of an experiment version must clear.
pub fn bar_confidence(k: NonZeroU32) -> f64 {
    1.0 - ALPHA / f64::from(k.get())
}

pub fn confidence_95() -> f64 {
    1.0 - ALPHA
}

fn mean_sd(xs: &[f64]) -> Option<(u32, f64, f64)> {
    let n = u32::try_from(xs.len()).ok().filter(|n| *n >= 2)?;
    if !xs.iter().all(|x| x.is_finite()) {
        return None;
    }
    let nf = f64::from(n);
    let mean = xs.iter().sum::<f64>() / nf;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (nf - 1.0);
    Some((n, mean, var.sqrt()))
}

/// Mean of the paired differences and its one-sided lower confidence bound.
/// `None` for fewer than two values or any value that is not finite.
pub fn stats(diffs: &[f64], confidence: f64) -> Option<Stats> {
    let (n, mean, sd) = mean_sd(diffs)?;
    let nf = f64::from(n);
    let lower = mean - t_quantile(confidence, nf - 1.0) * sd / nf.sqrt();
    Some(Stats {
        n,
        mean,
        sd,
        confidence,
        lower,
    })
}

/// The matching one-sided upper bound.
pub fn upper(s: &Stats) -> f64 {
    2.0 * s.mean - s.lower
}

/// 95% half-width of one paired difference, from differences of a commit with itself.
pub fn noise(diffs: &[f64]) -> Option<f64> {
    let (n, _, sd) = mean_sd(diffs)?;
    let nf = f64::from(n);
    Some(t_quantile(1.0 - ALPHA / 2.0, nf - 1.0) * sd * (1.0 + 1.0 / nf).sqrt())
}

/// A regression: the lower bound of the paired loss is above zero at 95%.
pub fn regressed(losses: &[f64]) -> bool {
    stats(losses, confidence_95()).is_some_and(|s| s.lower > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{a} vs {b}");
    }

    #[test]
    fn t_quantiles_match_published_tables() {
        close(t_quantile(0.95, 9.0), 1.833_113, 1e-5);
        close(t_quantile(0.975, 4.0), 2.776_445, 1e-5);
        close(t_quantile(0.99, 1.0), 31.820_516, 1e-4);
        close(t_quantile(0.95, 1e6), 1.644_854, 1e-4);
        close(t_quantile(0.05, 9.0), -1.833_113, 1e-5);
    }

    #[test]
    fn the_bar_rises_with_k() {
        let k = |n| bar_confidence(NonZeroU32::new(n).unwrap());
        close(k(1), 0.95, 1e-12);
        close(k(5), 0.99, 1e-12);
        assert!(k(2) > k(1) && k(40) > k(2));
    }

    #[test]
    fn too_few_or_bad_values_give_no_stats() {
        assert!(stats(&[1.0], 0.95).is_none());
        assert!(stats(&[1.0, f64::NAN], 0.95).is_none());
        assert!(stats(&[1.0, f64::INFINITY], 0.95).is_none());
    }

    #[test]
    fn noise_and_regression() {
        assert!(noise(&[0.1, -0.1, 0.0, 0.05]).unwrap() > 0.0);
        assert!(regressed(&[5.0, 5.1, 4.9, 5.0]));
        assert!(!regressed(&[5.0, -5.0, 4.0, -4.0]));
    }

    fn sample() -> impl Strategy<Value = Vec<f64>> {
        prop::collection::vec(-1000.0f64..1000.0, 2..40)
    }

    proptest! {
        #[test]
        fn cdf_is_a_cdf(t in -50.0f64..50.0, df in 1.0f64..200.0) {
            let p = t_cdf(t, df);
            prop_assert!((0.0..=1.0).contains(&p));
            prop_assert!(t_cdf(t + 0.5, df) >= p);
            prop_assert!((t_cdf(-t, df) - (1.0 - p)).abs() < 1e-9);
        }

        #[test]
        fn quantile_inverts_cdf(p in 0.01f64..0.99999, df in 1.0f64..100.0) {
            prop_assert!((t_cdf(t_quantile(p, df), df) - p).abs() < 1e-9);
        }

        #[test]
        fn lower_bound_is_at_most_the_mean(xs in sample(), c in 0.5f64..0.9999) {
            let s = stats(&xs, c).unwrap();
            prop_assert!(s.lower <= s.mean + 1e-9);
            prop_assert!(upper(&s) >= s.mean - 1e-9);
        }

        #[test]
        fn a_harder_bar_never_lowers_the_bound_up(xs in sample(), k in 1u32..500) {
            let easy = stats(&xs, bar_confidence(NonZeroU32::new(k).unwrap())).unwrap();
            let hard = stats(&xs, bar_confidence(NonZeroU32::new(k + 1).unwrap())).unwrap();
            prop_assert!(hard.lower <= easy.lower + 1e-9);
        }

        #[test]
        fn clearing_the_bar_needs_a_positive_mean(xs in sample(), k in 1u32..500) {
            let s = stats(&xs, bar_confidence(NonZeroU32::new(k).unwrap())).unwrap();
            prop_assert!(s.lower <= 0.0 || s.mean > 0.0);
        }

        #[test]
        fn bound_shifts_with_the_data(xs in sample(), shift in -100.0f64..100.0) {
            let a = stats(&xs, 0.95).unwrap();
            let moved: Vec<f64> = xs.iter().map(|x| x + shift).collect();
            let b = stats(&moved, 0.95).unwrap();
            prop_assert!((b.lower - (a.lower + shift)).abs() < 1e-6);
        }

        #[test]
        fn a_pure_loss_never_clears_the_bar(xs in prop::collection::vec(-1000.0f64..-0.001, 2..40), k in 1u32..100) {
            let s = stats(&xs, bar_confidence(NonZeroU32::new(k).unwrap())).unwrap();
            prop_assert!(s.lower <= 0.0);
        }
    }
}
