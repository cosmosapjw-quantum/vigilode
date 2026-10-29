//! Re-audit 2026-10-10 finding F101, development node AS02: the predictive
//! term `f_P` of `ControllerKind::{Predictive, PredictiveCapped,
//! PredictiveCapped2}` for extreme but positive finite inputs.
//!
//! Contract: for positive finite `h`, `h_acc`, `error`, with
//! `err_acc = max(err_prev, 1e-2)`,
//! `f_P = clamp(exp(ln safety + ln h - ln h_acc + (ln err_acc - 2 ln error) / p),
//! min_factor, max_factor)`, clamped in the log domain. The ALG02 expression
//! `safety * (h / h_acc) * (err_acc / (error * error)).powf(1 / p)` is kept
//! bit for bit whenever all its intermediates are normal binary64 numbers and
//! the product is positive; only otherwise is the log form used. The
//! `err = 0`, post-rejection, sliver and informative-clipping rules and the
//! `Integral` / `Pi` arithmetic are unchanged.

use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, CLIPPED_SAMPLE_INFORMATIVE_RATIO, ControllerKind,
    adaptive_next_step_after_attempt,
};

const PREDICTIVE: [ControllerKind; 3] = [
    ControllerKind::Predictive,
    ControllerKind::PredictiveCapped,
    ControllerKind::PredictiveCapped2,
];

fn config(controller: ControllerKind) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        controller,
        ..AdaptiveStepConfig::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn step(
    state: &mut AdaptiveControllerState,
    config: &AdaptiveStepConfig,
    requested: f64,
    trial: f64,
    error: f64,
    order: usize,
    accepted: bool,
    clipped: bool,
) -> f64 {
    adaptive_next_step_after_attempt(
        state, config, requested, trial, error, order, accepted, clipped,
    )
    .unwrap()
}

/// The production accepted/rejected factor (`Integral`, `Pi`, and `f_I` of
/// the predictive kinds), written out as in `propose_factor`.
fn integral(
    c: &AdaptiveStepConfig,
    previous: Option<f64>,
    error: f64,
    order: usize,
    accepted: bool,
) -> f64 {
    if error == 0.0 {
        return if accepted {
            c.max_factor
        } else {
            c.reject_max_factor
        };
    }
    let order = order as f64;
    let raw = match (c.controller, accepted, previous) {
        (ControllerKind::Pi, true, Some(p)) if p > 0.0 => {
            c.safety * error.powf(-0.7 / order) * p.powf(0.4 / order)
        }
        _ => c.safety * error.powf(-1.0 / order),
    };
    assert!(raw.is_finite() && raw > 0.0, "reference f_I must be finite");
    if accepted {
        raw.clamp(c.min_factor, c.max_factor)
    } else {
        raw.clamp(c.min_factor, c.reject_max_factor)
    }
}

/// The ALG02 predictive expression, operation for operation, and whether all
/// its intermediates were normal with a positive product (the AS02 switch
/// condition for keeping it).
fn old_predictive(
    c: &AdaptiveStepConfig,
    h: f64,
    h_acc: f64,
    err_prev: f64,
    error: f64,
    order: usize,
) -> (f64, bool) {
    let err_acc = err_prev.max(1.0e-2);
    let order = order as f64;
    let ratio = h / h_acc;
    let scaled = c.safety * ratio;
    let squared = error * error;
    let quotient = err_acc / squared;
    let power = quotient.powf(1.0 / order);
    let raw = scaled * power;
    // Same association as `c.safety * (h / h_acc) * (...).powf(1 / order)`.
    assert_eq!(
        raw.to_bits(),
        (c.safety * (h / h_acc) * (err_acc / (error * error)).powf(1.0 / order)).to_bits()
    );
    let normal = raw > 0.0
        && [ratio, scaled, squared, quotient, power, raw]
            .iter()
            .all(|x| x.is_normal());
    (raw, normal)
}

/// The unclamped log of `f_P`, written independently of the implementation
/// (a different association of the same terms).
fn log_oracle(
    c: &AdaptiveStepConfig,
    h: f64,
    h_acc: f64,
    err_prev: f64,
    error: f64,
    p: usize,
) -> f64 {
    let err_acc = err_prev.max(1.0e-2);
    (err_acc.ln() - 2.0 * error.ln()) / p as f64 + c.safety.ln() + (h.ln() - h_acc.ln())
}

/// `f_P` from the log oracle: the clamp bounds exactly when the oracle is
/// clearly outside, `None` within `1e-9` of a bound (rounding-ambiguous).
fn oracle_factor(c: &AdaptiveStepConfig, log: f64) -> Option<f64> {
    let (lo, hi) = (c.min_factor.ln(), c.max_factor.ln());
    if log < lo - 1.0e-9 {
        Some(c.min_factor)
    } else if log > hi + 1.0e-9 {
        Some(c.max_factor)
    } else if log > lo + 1.0e-9 && log < hi - 1.0e-9 {
        Some(log.exp())
    } else {
        None
    }
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs()
}

/// A fresh state that accepted one non-clipped trial `h_acc` with `err_prev`
/// and, if `reject`, then rejected a trial (error 2) from there.
fn history(
    c: &AdaptiveStepConfig,
    h_acc: f64,
    err_prev: f64,
    order: usize,
    reject: bool,
) -> AdaptiveControllerState {
    let mut state = AdaptiveControllerState::default();
    step(&mut state, c, h_acc, h_acc, err_prev, order, true, false);
    if reject {
        step(&mut state, c, h_acc, h_acc, 2.0, order, false, false);
    }
    assert_eq!(state.last_accepted_step(), Some(h_acc));
    state
}

/// Expected accepted-step factor of the predictive kinds from `f_I` and an
/// `f_P`, with the post-rejection cap.
fn combine(c: &AdaptiveStepConfig, f_i: f64, f_p: f64, pending: bool) -> f64 {
    let mut factor = f_i.min(f_p);
    if pending && c.controller != ControllerKind::Predictive {
        factor = factor.min(1.0);
    }
    factor
}

/// F101 as registered: previous accepted h = 1, error = 0.5; current
/// h = 1e-100, error = 1e-200, order 5. The exact factor 7.83e-21 clamps to
/// min_factor = 0.2; ALG02 returned 5.0.
#[test]
fn registered_tiny_error_case_selects_min_factor() {
    for kind in PREDICTIVE {
        let c = config(kind);
        let (old, normal) = old_predictive(&c, 1.0e-100, 1.0, 0.5, 1.0e-200, 5);
        assert!(!normal, "error^2 underflows: the log path must be used");
        assert_eq!(old, f64::INFINITY, "the ALG02 expression overflows");
        assert_eq!(old.clamp(c.min_factor, c.max_factor), 5.0);
        let log = log_oracle(&c, 1.0e-100, 1.0, 0.5, 1.0e-200, 5);
        assert!((log - -46.295_691_811_650_73).abs() < 1.0e-9);

        for reject in [false, true] {
            let mut state = history(&c, 1.0, 0.5, 5, reject);
            let next = step(&mut state, &c, 1.0e-100, 1.0e-100, 1.0e-200, 5, true, false);
            assert_eq!(next.to_bits(), (1.0e-100 * 0.2_f64).to_bits(), "{kind:?}");
            assert_eq!(next, 2.0e-101);
        }

        // The same exponent with a unit trial step, so the factor itself is
        // the next step: h_acc = 1e100, h = 1.
        let mut state = history(&c, 1.0e100, 0.5, 5, false);
        let next = step(&mut state, &c, 1.0, 1.0, 1.0e-200, 5, true, false);
        assert_eq!(next.to_bits(), 0.2_f64.to_bits(), "{kind:?}");
    }
}

/// NATIVE.json `predictive_moderate_control`: h = 0.1, error = 0.5 after
/// h = 1, error = 0.5 gives factor 0.20000000000000004 (direct path, as
/// before).
#[test]
fn registered_moderate_control_is_unchanged() {
    for kind in PREDICTIVE {
        let c = config(kind);
        let mut state = history(&c, 1.0, 0.5, 5, false);
        let next = step(&mut state, &c, 0.1, 0.1, 0.5, 5, true, false);
        assert_eq!(next, 0.020_000_000_000_000_004, "{kind:?}");
        let (old, normal) = old_predictive(&c, 0.1, 1.0, 0.5, 0.5, 5);
        assert!(normal);
        let expected = combine(
            &c,
            integral(&c, None, 0.5, 5, true),
            old.clamp(0.2, 5.0),
            false,
        );
        // f_P clamps to 0.2; NATIVE.json's 0.20000000000000004 is next_h / h
        // with next_h = 0.1 * 0.2 = 0.020000000000000004.
        assert_eq!(expected, 0.2);
        assert_eq!(next.to_bits(), (0.1 * expected).to_bits());
    }
}

/// Deterministic 64-bit LCG.
struct Lcg(u64);

impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1_u64 << 53) as f64)
    }

    fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + (hi.ln() - lo.ln()) * self.unit()).exp()
    }
}

/// Ordinary inputs (all ALG02 intermediates normal) agree with the ALG02
/// expression bit for bit, for every predictive kind, with and without a
/// pending rejection.
#[test]
fn ordinary_inputs_agree_with_alg02_bit_for_bit() {
    let mut rng = Lcg(0x5eed_a502_u64);
    let mut checked = 0_usize;
    for kind in PREDICTIVE {
        let c = config(kind);
        for _ in 0..20_000 {
            let h_acc = rng.log_uniform(1.0e-12, 1.0e2);
            let h = h_acc * rng.log_uniform(0.05, 20.0);
            let err_prev = rng.log_uniform(1.0e-14, 1.0e3);
            let error = rng.log_uniform(1.0e-14, 1.0e3);
            let order = [2_usize, 3, 4, 5][(rng.unit() * 4.0) as usize];
            let reject = rng.unit() < 0.3;
            let mut state = history(&c, h_acc, err_prev, order, reject);
            let prev_recorded = err_prev.max(1.0e-16);
            let (old, normal) = old_predictive(&c, h, h_acc, prev_recorded, error, order);
            assert!(normal, "ordinary sample left the direct path");
            let f_i = integral(&c, Some(prev_recorded), error, order, true);
            let expected = combine(&c, f_i, old.clamp(c.min_factor, c.max_factor), reject);
            let next = step(&mut state, &c, h, h, error, order, true, false);
            assert_eq!(next.to_bits(), (h * expected).to_bits(), "{kind:?}");
            checked += 1;
        }
    }
    assert_eq!(checked, 60_000);
}

/// Over a wide log-uniform sample (errors from subnormal to 1e300, step
/// ratios beyond 1e+-300) every proposal is finite and inside the factor
/// bounds; it is ALG02 bit for bit exactly when the ALG02 intermediates are
/// normal, and otherwise matches the log oracle.
#[test]
fn wide_sample_switch_condition_and_log_oracle() {
    let mut rng = Lcg(0x00f1_0101_u64);
    let (mut direct, mut log_path) = (0_usize, 0_usize);
    for kind in PREDICTIVE {
        let c = config(kind);
        for _ in 0..30_000 {
            let h_acc = rng.log_uniform(1.0e-300, 1.0e300);
            let h = rng.log_uniform(1.0e-300, 1.0e300);
            let err_prev = rng.log_uniform(1.0e-320, 1.0e300);
            let error = rng.log_uniform(1.0e-320, 1.0e300);
            let order = [2_usize, 3, 5][(rng.unit() * 3.0) as usize];
            let reject = rng.unit() < 0.3;
            let mut state = history(&c, h_acc, err_prev, order, reject);
            let prev_recorded = err_prev.max(1.0e-16);
            let f_i = integral(&c, Some(prev_recorded), error, order, true);
            let next = step(&mut state, &c, h, h, error, order, true, false);
            assert!(next.is_finite() && next > 0.0);
            assert!(next >= h * c.min_factor && next <= h * c.max_factor);
            let (old, normal) = old_predictive(&c, h, h_acc, prev_recorded, error, order);
            if normal {
                direct += 1;
                let expected = combine(&c, f_i, old.clamp(c.min_factor, c.max_factor), reject);
                assert_eq!(next.to_bits(), (h * expected).to_bits(), "{kind:?}");
            } else {
                log_path += 1;
                let log = log_oracle(&c, h, h_acc, prev_recorded, error, order);
                if let Some(f_p) = oracle_factor(&c, log) {
                    let expected = combine(&c, f_i, f_p, reject);
                    assert!(
                        close(next / h, expected, 1.0e-11),
                        "{kind:?} h={h:e} h_acc={h_acc:e} err_prev={err_prev:e} error={error:e}: \
                         {} vs {expected}",
                        next / h
                    );
                }
            }
        }
    }
    assert!(direct > 10_000 && log_path > 10_000, "{direct} {log_path}");
}

/// Near-underflow and near-overflow neighbours, deterministic grid: errors
/// down to the smallest subnormal and up to f64::MAX, step ratios near
/// 1e+-300 and h_acc subnormal, all three kinds, with and without a pending
/// rejection.
#[test]
fn near_underflow_and_overflow_grid_gives_bounded_factors() {
    let errors = [
        f64::from_bits(1),
        1.0e-320,
        1.0e-310,
        f64::MIN_POSITIVE,
        f64::MIN_POSITIVE * (1.0 + f64::EPSILON),
        1.0e-200,
        1.5e-154,
        f64::MIN_POSITIVE.sqrt(),
        1.0e-150,
        1.0e-16,
        0.5,
        1.0e150,
        f64::MAX.sqrt(),
        1.0e200,
        f64::MAX,
    ];
    // (h_acc, h)
    let steps = [
        (1.0, 1.0),
        (1.0, 1.0e-300),
        (1.0e-300, 1.0),
        (1.0e-150, 1.0e150),
        (1.0e150, 1.0e-150),
        (1.0e300, 1.0e-300),
        (1.0e-300, 1.0e300),
        (f64::from_bits(1), 1.0),
        (f64::MIN_POSITIVE, 1.0e300),
        (f64::MAX, 1.0e-300),
    ];
    let err_prevs = [1.0e-16, 0.5, 1.0e300];
    for kind in PREDICTIVE {
        let c = config(kind);
        for order in [2_usize, 3, 5] {
            for &(h_acc, h) in &steps {
                for &err_prev in &err_prevs {
                    for &error in &errors {
                        for reject in [false, true] {
                            let f_i = integral(&c, Some(err_prev), error, order, true);
                            let mut state = history(&c, h_acc, err_prev, order, reject);
                            let next = step(&mut state, &c, h, h, error, order, true, false);
                            assert!(next.is_finite() && next > 0.0, "{kind:?} {h:e} {error:e}");
                            assert!(next >= h * c.min_factor && next <= h * c.max_factor);
                            let (old, normal) =
                                old_predictive(&c, h, h_acc, err_prev, error, order);
                            let log = log_oracle(&c, h, h_acc, err_prev, error, order);
                            let expected = if normal {
                                Some(old.clamp(c.min_factor, c.max_factor))
                            } else {
                                oracle_factor(&c, log)
                            };
                            if let Some(f_p) = expected {
                                let factor = combine(&c, f_i, f_p, reject);
                                if normal {
                                    assert_eq!(next.to_bits(), (h * factor).to_bits());
                                } else {
                                    assert!(
                                        close(next / h, factor, 1.0e-11),
                                        "{kind:?} p={order} h_acc={h_acc:e} h={h:e} \
                                         err_prev={err_prev:e} error={error:e}: {} vs {factor}",
                                        next / h
                                    );
                                }
                            }
                            if reject && kind != ControllerKind::Predictive {
                                assert!(next <= h, "post-rejection cap");
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Former ALG02 failure modes on positive finite inputs: `error^2`
/// overflowing to infinity made the product zero (an error return), and
/// `h / h_acc` underflowing to zero times an infinite power made it NaN.
/// Both now give min_factor.
#[test]
fn former_error_returns_now_clamp_to_min_factor() {
    for kind in PREDICTIVE {
        let c = config(kind);
        // error^2 = inf: quotient 0, product 0.
        let (old, normal) = old_predictive(&c, 1.0, 1.0, 0.5, 1.0e200, 5);
        assert!(!normal && old == 0.0);
        let mut state = history(&c, 1.0, 0.5, 5, false);
        assert_eq!(step(&mut state, &c, 1.0, 1.0, 1.0e200, 5, true, false), 0.2);

        // h / h_acc = 0 and error^2 = 0: 0 * inf.
        let (old, normal) = old_predictive(&c, 1.0e-300, 1.0e300, 0.5, 1.0e-200, 5);
        assert!(!normal && old.is_nan());
        let mut state = history(&c, 1.0e300, 0.5, 5, false);
        let next = step(&mut state, &c, 1.0e-300, 1.0e-300, 1.0e-200, 5, true, false);
        assert_eq!(next.to_bits(), (1.0e-300 * 0.2_f64).to_bits());
    }
}

/// `err = 0` keeps its policy under an extreme history: `max_factor`, the
/// post-rejection cap only for `PredictiveCapped2` (ALG05), and no predictive
/// term.
#[test]
fn zero_error_policy_is_unchanged_under_extreme_history() {
    for (h_acc, h) in [(1.0e-300, 1.0), (1.0e300, 1.0), (1.0, 1.0e-300)] {
        for kind in PREDICTIVE {
            let c = config(kind);
            for reject in [false, true] {
                let mut state = history(&c, h_acc, 1.0e-200, 5, reject);
                let next = step(&mut state, &c, h, h, 0.0, 5, true, false);
                let factor = if reject && kind == ControllerKind::PredictiveCapped2 {
                    1.0
                } else {
                    c.max_factor
                };
                assert_eq!(next.to_bits(), (h * factor).to_bits(), "{kind:?} {reject}");
                assert!(!state.rejection_pending());
                assert_eq!(state.last_accepted_step(), Some(h));
            }
        }
    }
}

/// Rejections scale the actual trial by the unchanged production factor, and
/// the first acceptance after one is capped at one for the capped kinds even
/// when the predictive term is at max_factor on the log path.
#[test]
fn rejection_policy_is_unchanged_under_extreme_inputs() {
    for kind in PREDICTIVE {
        let c = config(kind);
        let mut state = history(&c, 1.0, 0.5, 5, false);
        for error in [2.0, 1.0e300, f64::MAX, f64::INFINITY] {
            let next = step(&mut state, &c, 1.0e-100, 1.0e-100, error, 5, false, false);
            let expected = if error.is_finite() {
                1.0e-100 * integral(&c, Some(0.5), error, 5, false)
            } else {
                1.0e-100 * c.min_factor
            };
            assert_eq!(next.to_bits(), expected.to_bits());
        }
        if kind == ControllerKind::PredictiveCapped2 {
            assert!(state.rejection_pending());
        }
        // f_P exponent is far above ln 5 and error^2 underflows.
        let (_, normal) = old_predictive(&c, 1.0e200, 1.0, 0.5, 1.0e-200, 5);
        assert!(!normal);
        let next = step(&mut state, &c, 1.0e200, 1.0e200, 1.0e-200, 5, true, false);
        let factor = if kind == ControllerKind::Predictive {
            c.max_factor
        } else {
            1.0
        };
        assert_eq!(next.to_bits(), (1.0e200 * factor).to_bits(), "{kind:?}");
        assert!(!state.rejection_pending());
    }
}

/// A sliver landing returns the request unchanged and leaves the history and
/// a pending rejection untouched, also where ALG02 produced NaN.
#[test]
fn sliver_policy_is_unchanged_under_extreme_inputs() {
    for kind in PREDICTIVE {
        let c = config(kind);
        for reject in [false, true] {
            let mut state = history(&c, 1.0e300, 0.5, 5, reject);
            let before = state.clone();
            let requested = 1.0e-299;
            let trial = 1.0e-300;
            assert!(trial / requested < CLIPPED_SAMPLE_INFORMATIVE_RATIO);
            for error in [1.0e-200, f64::from_bits(1), 1.0e200, 0.0] {
                let next = step(&mut state, &c, requested, trial, error, 5, true, true);
                assert_eq!(next, requested);
                assert_eq!(
                    state.previous_accepted_error(),
                    before.previous_accepted_error()
                );
                assert_eq!(state.last_accepted_step(), Some(1.0e300));
                assert_eq!(
                    state.rejection_pending(),
                    reject && kind == ControllerKind::PredictiveCapped2
                );
            }
        }
    }
}

/// Informative clipping keeps request restoration (the registered ALG02 /
/// ALG05 policy): the post-rejection cap is "factor <= 1", not "no step
/// increase after the actual trial". NATIVE.json
/// `informative_clip_cap_semantics`: after h = 1 accepted (error 0.5) and
/// h = 1.1 rejected (error 3), a clipped trial 0.75 of request 1 with error
/// 1e-3 gives next h = 1 (4/3 of the trial) and clears the pending flag
/// for both capped kinds.
#[test]
fn informative_clipping_keeps_request_restoration() {
    for kind in [
        ControllerKind::PredictiveCapped,
        ControllerKind::PredictiveCapped2,
    ] {
        let c = config(kind);
        let mut state = AdaptiveControllerState::default();
        let first = step(&mut state, &c, 1.0, 1.0, 0.5, 5, true, false);
        assert_eq!(first, 1.033_828_519_497_331_6);
        let second = step(&mut state, &c, 1.1, 1.1, 3.0, 5, false, false);
        assert_eq!(second, 0.794_714_146_142_628_5);
        if kind == ControllerKind::PredictiveCapped2 {
            assert!(state.rejection_pending());
        }
        let next = step(&mut state, &c, 1.0, 0.75, 1.0e-3, 5, true, true);
        assert_eq!(next, 1.0, "{kind:?}");
        assert!(next > 0.75);
        assert!(!state.rejection_pending());
        assert_eq!(state.last_accepted_step(), Some(0.75));
        assert_eq!(state.previous_accepted_error(), Some(1.0e-3));
    }

    // Extreme informative samples: one that does not predict rejection
    // restores the request, one that does keeps the actual trial (factor at
    // min_factor on the log path in both).
    for kind in PREDICTIVE {
        let c = config(kind);
        let mut state = history(&c, 1.0, 0.5, 5, false);
        let next = step(&mut state, &c, 1.0e-100, 0.75e-100, 1.0e-200, 5, true, true);
        assert_eq!(next, 1.0e-100);
        let mut state = history(&c, 1.0, 0.5, 5, false);
        let next = step(&mut state, &c, 1.0e-100, 0.75e-100, 1.0e200, 5, true, true);
        assert_eq!(next, 0.75e-100);
        assert_eq!(state.last_accepted_step(), Some(0.75e-100));
    }
}

/// The shared update for `Integral` and `Pi`, written out as before AS02.
#[derive(Default)]
struct IntegralReference {
    previous: Option<f64>,
}

impl IntegralReference {
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        c: &AdaptiveStepConfig,
        requested: f64,
        trial: f64,
        error: f64,
        order: usize,
        accepted: bool,
        clipped: bool,
    ) -> f64 {
        if accepted {
            let factor = integral(c, self.previous, error, order, true);
            if !clipped {
                self.previous = Some(error.max(1.0e-16));
                return trial * factor;
            }
            let ratio = trial / requested;
            if ratio < CLIPPED_SAMPLE_INFORMATIVE_RATIO {
                return requested;
            }
            self.previous = Some(error.max(1.0e-16));
            let candidate = trial * factor;
            let predicted = error * ratio.powf(-(order as f64));
            if predicted > 1.0 {
                return candidate.max(trial).min(requested);
            }
            return requested.max(candidate);
        }
        if error.is_finite() {
            trial * integral(c, self.previous, error.max(1.0e-16), order, false)
        } else {
            trial * c.min_factor
        }
    }
}

/// `Integral` and `Pi` are bit for bit the pre-AS02 update on a large random
/// attempt stream with extreme magnitudes, rejections, failures, `err = 0`,
/// informative clipped samples and slivers.
#[test]
fn integral_and_pi_are_unchanged_on_a_large_random_sample() {
    let mut rng = Lcg(0x01a7_e9a1);
    for kind in [ControllerKind::Integral, ControllerKind::Pi] {
        let c = config(kind);
        for order in [3_usize, 5] {
            let mut state = AdaptiveControllerState::default();
            let mut reference = IntegralReference::default();
            for _ in 0..100_000 {
                let requested = rng.log_uniform(1.0e-280, 1.0e280);
                let clip = rng.unit();
                let (trial, clipped) = if clip < 0.15 {
                    (requested * (0.01 + 0.45 * rng.unit()), true)
                } else if clip < 0.3 {
                    (requested * (0.5 + 0.49 * rng.unit()), true)
                } else {
                    (requested, false)
                };
                let pick = rng.unit();
                let error = if pick < 0.05 {
                    0.0
                } else if pick < 0.1 {
                    f64::INFINITY
                } else if pick < 0.5 {
                    rng.log_uniform(1.0e-320, 1.0e300)
                } else {
                    rng.log_uniform(1.0e-6, 1.0e2)
                };
                let accepted = error.is_finite() && (error <= 1.0 || rng.unit() < 0.1);
                let got = step(
                    &mut state, &c, requested, trial, error, order, accepted, clipped,
                );
                let want = reference.update(&c, requested, trial, error, order, accepted, clipped);
                assert_eq!(got.to_bits(), want.to_bits(), "{kind:?}");
                assert_eq!(state.previous_accepted_error(), reference.previous);
                assert_eq!(state.last_accepted_step(), None);
                assert!(!state.rejection_pending());
            }
        }
    }
}
