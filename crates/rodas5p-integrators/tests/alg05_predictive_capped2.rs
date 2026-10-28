//! Research node ALG05 (`research/alg05_predictive_controller_v2_20261010`):
//! `ControllerKind::PredictiveCapped2` is `PredictiveCapped` with two
//! changes, and the older kinds keep their arithmetic bit for bit.
//!
//! 1. The cap applies when `err = 0`: after a rejected or failed attempt an
//!    accepted step with `err = 0` gets `min(max_factor, 1) = 1`.
//! 2. The rejection flag survives sliver landings: it is cleared only by an
//!    accepted non-sliver step (a non-clipped trial, or a clipped trial of at
//!    least `CLIPPED_SAMPLE_INFORMATIVE_RATIO` times the request).
//!
//! The reference model below writes out the shared update of
//! `adaptive_next_step_after_attempt` as it was before ALG05 (commit
//! b8964fc) and the registered ALG05 rule, operation for operation.

use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, CLIPPED_SAMPLE_INFORMATIVE_RATIO, ControllerKind,
    RODAS5P_ESTIMATOR_ORDER, adaptive_next_step_after_attempt,
};

const ORDER: usize = RODAS5P_ESTIMATOR_ORDER;
/// f_I = 0.9 * 0.01^(-1/5): the uncapped factor at err = 0.01 used below.
const F_I_AT_1E_2: f64 = 2.260_697_788_358_622_3;

fn config(controller: ControllerKind) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        controller,
        ..AdaptiveStepConfig::default()
    }
}

fn step(
    state: &mut AdaptiveControllerState,
    config: &AdaptiveStepConfig,
    requested: f64,
    trial: f64,
    error: f64,
    accepted: bool,
    clipped: bool,
) -> f64 {
    adaptive_next_step_after_attempt(
        state, config, requested, trial, error, ORDER, accepted, clipped,
    )
    .unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1.0e-14 * b.abs()
}

/// The shared update written out: ALG02's rule for `Integral`, `Pi`,
/// `Predictive` and `PredictiveCapped`, and the registered ALG05 rule for
/// `PredictiveCapped2`.
#[derive(Default)]
struct Reference {
    previous: Option<f64>,
    h_acc: Option<f64>,
    last_rejected: bool,
    pending: bool,
}

impl Reference {
    fn integral(c: &AdaptiveStepConfig, previous: Option<f64>, error: f64, accepted: bool) -> f64 {
        if error == 0.0 {
            return if accepted {
                c.max_factor
            } else {
                c.reject_max_factor
            };
        }
        let order = ORDER as f64;
        let raw = match (c.controller, accepted, previous) {
            (ControllerKind::Pi, true, Some(p)) if p > 0.0 => {
                c.safety * error.powf(-0.7 / order) * p.powf(0.4 / order)
            }
            _ => c.safety * error.powf(-1.0 / order),
        };
        if accepted {
            raw.clamp(c.min_factor, c.max_factor)
        } else {
            raw.clamp(c.min_factor, c.reject_max_factor)
        }
    }

    fn update(
        &mut self,
        c: &AdaptiveStepConfig,
        requested: f64,
        trial: f64,
        error: f64,
        accepted: bool,
        clipped: bool,
    ) -> f64 {
        let kind = c.controller;
        let predictive = matches!(
            kind,
            ControllerKind::Predictive
                | ControllerKind::PredictiveCapped
                | ControllerKind::PredictiveCapped2
        );
        let v2 = kind == ControllerKind::PredictiveCapped2;
        let previous_failed = self.last_rejected;
        self.last_rejected = !accepted;
        if v2 && !accepted {
            self.pending = true;
        }
        if accepted {
            let capped_by = if v2 { self.pending } else { previous_failed };
            let integral = Self::integral(c, self.previous, error, true);
            let factor = if !predictive {
                integral
            } else if error == 0.0 {
                if v2 && capped_by {
                    integral.min(1.0)
                } else {
                    integral
                }
            } else {
                let mut factor = integral;
                if let (Some(h_acc), Some(err_prev)) = (self.h_acc, self.previous) {
                    let err_acc = err_prev.max(1.0e-2);
                    let order = ORDER as f64;
                    let raw =
                        c.safety * (trial / h_acc) * (err_acc / (error * error)).powf(1.0 / order);
                    factor = factor.min(raw.clamp(c.min_factor, c.max_factor));
                }
                if matches!(
                    kind,
                    ControllerKind::PredictiveCapped | ControllerKind::PredictiveCapped2
                ) && capped_by
                {
                    factor = factor.min(1.0);
                }
                factor
            };
            if !clipped {
                self.previous = Some(error.max(1.0e-16));
                if predictive {
                    self.h_acc = Some(trial);
                }
                if v2 {
                    self.pending = false;
                }
                return trial * factor;
            }
            let ratio = trial / requested;
            if ratio < CLIPPED_SAMPLE_INFORMATIVE_RATIO {
                return requested;
            }
            self.previous = Some(error.max(1.0e-16));
            if predictive {
                self.h_acc = Some(trial);
            }
            if v2 {
                self.pending = false;
            }
            let candidate = trial * factor;
            let predicted = error * ratio.powf(-(ORDER as f64));
            if predicted > 1.0 {
                return candidate.max(trial).min(requested);
            }
            return requested.max(candidate);
        }
        if error.is_finite() {
            trial * Self::integral(c, self.previous, error.max(1.0e-16), false)
        } else {
            trial * c.min_factor
        }
    }
}

/// A deterministic attempt stream (64-bit LCG) with rejections, failures,
/// `err = 0`, informative clipped samples and slivers.
struct Stream(u64);

impl Stream {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1_u64 << 53) as f64)
    }

    /// (requested, trial, error, accepted, clipped) for the current request.
    fn attempt(&mut self, request: f64) -> (f64, f64, f64, bool, bool) {
        let clip = self.unit();
        let (trial, clipped) = if clip < 0.15 {
            // Sliver landing.
            (request * (0.01 + 0.45 * self.unit()), true)
        } else if clip < 0.3 {
            // Informative clipped sample.
            (request * (0.5 + 0.49 * self.unit()), true)
        } else {
            (request, false)
        };
        let kind = self.unit();
        let error = if kind < 0.05 {
            0.0
        } else if kind < 0.1 {
            f64::INFINITY
        } else {
            10.0_f64.powf(-9.0 + 11.0 * self.unit())
        };
        let accepted = error <= 1.0;
        (request, trial, error, accepted, clipped && accepted)
    }
}

fn run_stream(kind: ControllerKind, seed: u64, attempts: usize) -> (usize, usize, usize) {
    let c = config(kind);
    let mut state = AdaptiveControllerState::default();
    let mut reference = Reference::default();
    let mut stream = Stream(seed);
    let mut request = 1.0e-3;
    let (mut sliver_after_rejection, mut zero_after_rejection, mut rejections) = (0, 0, 0);
    for i in 0..attempts {
        let (requested, trial, error, accepted, clipped) = stream.attempt(request);
        let pending_before = state.rejection_pending();
        if accepted && pending_before && clipped && trial / requested < 0.5 {
            sliver_after_rejection += 1;
        }
        if accepted && pending_before && error == 0.0 {
            zero_after_rejection += 1;
        }
        if !accepted {
            rejections += 1;
        }
        let expected = reference.update(&c, requested, trial, error, accepted, clipped);
        let next = step(&mut state, &c, requested, trial, error, accepted, clipped);
        assert_eq!(
            next.to_bits(),
            expected.to_bits(),
            "{kind:?} seed {seed} attempt {i}: {next} != {expected}"
        );
        assert_eq!(state.previous_accepted_error(), reference.previous);
        let predictive = kind != ControllerKind::Integral && kind != ControllerKind::Pi;
        assert_eq!(
            state.last_accepted_step(),
            if predictive { reference.h_acc } else { None }
        );
        assert_eq!(state.rejection_pending(), reference.pending);
        assert_eq!(state.last_rejected_trial().is_some(), !accepted);
        if kind != ControllerKind::PredictiveCapped2 {
            assert!(!state.rejection_pending());
        }
        // Keep the request in a sane range so the stream stays varied.
        request = next.clamp(1.0e-8, 1.0);
    }
    (rejections, sliver_after_rejection, zero_after_rejection)
}

#[test]
fn the_older_kinds_keep_their_arithmetic_bit_for_bit() {
    for kind in [
        ControllerKind::Integral,
        ControllerKind::Pi,
        ControllerKind::Predictive,
        ControllerKind::PredictiveCapped,
    ] {
        for seed in [1_u64, 7, 2026, 0xdead_beef] {
            let (rejections, _, _) = run_stream(kind, seed, 4000);
            assert!(rejections > 100, "{kind:?}: the stream must reject");
        }
    }
}

#[test]
fn predictive_capped2_follows_the_registered_rule_bit_for_bit() {
    let mut slivers = 0;
    let mut zeros = 0;
    for seed in [1_u64, 7, 2026, 0xdead_beef] {
        let (_, s, z) = run_stream(ControllerKind::PredictiveCapped2, seed, 4000);
        slivers += s;
        zeros += z;
    }
    // The stream exercises both registered changes.
    assert!(slivers > 20, "{slivers}");
    assert!(zeros > 5, "{zeros}");
}

#[test]
fn predictive_capped2_serializes_like_the_production_state() {
    let c = config(ControllerKind::PredictiveCapped2);
    let mut state = AdaptiveControllerState::default();
    step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
    step(&mut state, &c, 0.11, 0.11, 3.0, false, false);
    assert!(state.rejection_pending());
    assert_eq!(
        serde_json::to_value(&state).unwrap(),
        serde_json::json!({"previous_accepted_error": 0.5})
    );
    assert_eq!(
        serde_json::to_value(ControllerKind::PredictiveCapped2).unwrap(),
        serde_json::json!("predictive-capped2")
    );
    // The older kinds' names are unchanged.
    assert_eq!(
        serde_json::to_value(ControllerKind::PredictiveCapped).unwrap(),
        serde_json::json!("predictive-capped")
    );
    assert_eq!(
        serde_json::to_value(ControllerKind::Integral).unwrap(),
        serde_json::json!("integral")
    );
}

/// Gate item 6, first behaviour: err = 0 after a rejection (or failure)
/// gives factor 1; `PredictiveCapped` still gives `max_factor`.
#[test]
fn err_zero_after_a_rejection_gives_factor_one() {
    for failure in [3.0, f64::INFINITY] {
        for (kind, expected) in [
            (ControllerKind::PredictiveCapped2, 1.0),
            (ControllerKind::PredictiveCapped, 5.0),
            (ControllerKind::Predictive, 5.0),
        ] {
            let c = config(kind);
            let mut state = AdaptiveControllerState::default();
            step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
            step(&mut state, &c, 0.11, 0.11, failure, false, false);
            let next = step(&mut state, &c, 0.02, 0.02, 0.0, true, false);
            assert_eq!(next, 0.02 * expected, "{kind:?} {failure}");
            // The err = 0 step enters the history as before.
            assert_eq!(state.last_accepted_step(), Some(0.02));
            assert_eq!(state.previous_accepted_error(), Some(1.0e-16));
            assert!(!state.rejection_pending());
            // The flag is consumed: the next err = 0 step may grow again.
            let after = step(&mut state, &c, next, next, 0.0, true, false);
            assert_eq!(after, next * c.max_factor, "{kind:?}");
        }
    }
    // Without a pending rejection, err = 0 still gives max_factor.
    let c = config(ControllerKind::PredictiveCapped2);
    let mut state = AdaptiveControllerState::default();
    assert_eq!(
        step(&mut state, &c, 0.1, 0.1, 0.0, true, false),
        0.1 * c.max_factor
    );
    // min(max_factor, 1) is 1 for any valid max_factor.
    let c = AdaptiveStepConfig {
        max_factor: 1.0,
        ..config(ControllerKind::PredictiveCapped2)
    };
    let mut state = AdaptiveControllerState::default();
    step(&mut state, &c, 0.1, 0.1, 3.0, false, false);
    assert_eq!(step(&mut state, &c, 0.09, 0.09, 0.0, true, false), 0.09);
}

/// Gate item 6, second behaviour: a sliver landing after a rejection keeps
/// the rejection flag, so the next full step is capped;
/// `PredictiveCapped` consumes the flag on the sliver and grows.
#[test]
fn a_sliver_landing_keeps_the_rejection_flag() {
    for kind in [
        ControllerKind::PredictiveCapped2,
        ControllerKind::PredictiveCapped,
    ] {
        let c = config(kind);
        let mut state = AdaptiveControllerState::default();
        step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
        step(&mut state, &c, 0.11, 0.11, 3.0, false, false);
        assert_eq!(
            state.rejection_pending(),
            kind == ControllerKind::PredictiveCapped2
        );
        // Sliver landing: trial 0.01 of request 0.4 (< 0.5 of it).
        let next = step(&mut state, &c, 0.4, 0.01, 0.2, true, true);
        assert_eq!(next, 0.4);
        assert_eq!(state.last_accepted_step(), Some(0.1));
        assert_eq!(state.previous_accepted_error(), Some(0.5));
        assert_eq!(
            state.rejection_pending(),
            kind == ControllerKind::PredictiveCapped2
        );
        // The next full step: f_I = 2.26070 and
        // f_P = 0.9 * 0.5 * (0.5 / 1e-4)^(1/5) = 2.47176, uncapped 2.26070.
        let full = step(&mut state, &c, 0.05, 0.05, 0.01, true, false);
        let expected = if kind == ControllerKind::PredictiveCapped2 {
            1.0
        } else {
            F_I_AT_1E_2
        };
        assert!(close(full / 0.05, expected), "{kind:?}: {}", full / 0.05);
        assert!(!state.rejection_pending());
        // Cleared by the accepted full step: the step after it may grow.
        let after = step(&mut state, &c, full, full, 0.01, true, false);
        assert!(close(after / full, F_I_AT_1E_2), "{kind:?}");
    }
    // Several slivers in a row, one with err = 0, keep the flag too.
    let c = config(ControllerKind::PredictiveCapped2);
    let mut state = AdaptiveControllerState::default();
    step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
    step(&mut state, &c, 0.11, 0.11, f64::INFINITY, false, false);
    for error in [0.2, 0.0, 1.0e-6] {
        assert_eq!(step(&mut state, &c, 0.4, 0.1, error, true, true), 0.4);
        assert!(state.rejection_pending());
    }
    let next = step(&mut state, &c, 0.05, 0.05, 0.0, true, false);
    assert_eq!(next, 0.05);
}

/// An informative clipped acceptance (trial >= 0.5 request) clears the
/// flag, and the cap binds that acceptance itself.
#[test]
fn an_informative_clipped_acceptance_clears_the_rejection_flag() {
    // Exactly the informative ratio counts as non-sliver.
    let requested = 0.08;
    let trial = requested * CLIPPED_SAMPLE_INFORMATIVE_RATIO;
    for kind in [
        ControllerKind::PredictiveCapped2,
        ControllerKind::Predictive,
    ] {
        let c = config(kind);
        let mut state = AdaptiveControllerState::default();
        step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
        step(&mut state, &c, 0.11, 0.11, 3.0, false, false);
        // err = 0.05 predicts 0.05 * 2^5 = 1.6 > 1 for the request, so the
        // next request is min(max(trial * f, trial), request). Uncapped,
        // f = min(f_I, f_P) = min(1.63453, 0.36 (0.5 / 0.0025)^(1/5)) =
        // 1.03874; capped, f = 1 and the request becomes the trial.
        let next = step(&mut state, &c, requested, trial, 0.05, true, true);
        if kind == ControllerKind::PredictiveCapped2 {
            assert_eq!(next, trial);
        } else {
            assert!(
                close(next / trial, 1.038_743_932_253_193_7),
                "{}",
                next / trial
            );
        }
        assert!(!state.rejection_pending());
        assert_eq!(state.last_accepted_step(), Some(trial));
        assert_eq!(state.previous_accepted_error(), Some(0.05));
        // The next full step is uncapped: f_I = 2.26070 and
        // f_P = 0.9 * 2 * (0.05 / 1e-4)^(1/5) = 6.24 > f_I.
        let full = step(&mut state, &c, 0.08, 0.08, 0.01, true, false);
        assert!(close(full / 0.08, F_I_AT_1E_2), "{}", full / 0.08);
    }
    let c = config(ControllerKind::PredictiveCapped2);
    // A rejection while pending keeps it pending; rejection proposals are
    // the production ones.
    let mut state = AdaptiveControllerState::default();
    step(&mut state, &c, 0.1, 0.1, 0.5, true, false);
    let a = step(&mut state, &c, 0.11, 0.11, 3.0, false, false);
    let b = step(&mut state, &c, a, a, 2.0, false, false);
    assert!(state.rejection_pending());
    let reference = config(ControllerKind::Integral);
    let mut integral = AdaptiveControllerState::default();
    step(&mut integral, &reference, 0.1, 0.1, 0.5, true, false);
    let ai = step(&mut integral, &reference, 0.11, 0.11, 3.0, false, false);
    let bi = step(&mut integral, &reference, ai, ai, 2.0, false, false);
    assert_eq!((a.to_bits(), b.to_bits()), (ai.to_bits(), bi.to_bits()));
}

/// Without rejections the new kind equals `PredictiveCapped` (and
/// `Predictive`) bit for bit, slivers included.
#[test]
fn without_rejections_predictive_capped2_equals_the_older_predictive_kinds() {
    let attempts = [
        (0.1, 0.1, 0.5, false),
        (0.12, 0.12, 0.02, false),
        (0.4, 0.05, 0.3, true),
        (0.4, 0.3, 0.0, true),
        (0.4, 0.4, 0.0, false),
        (2.0, 2.0, 0.9, false),
        (2.1, 1.5, 1.0e-7, true),
    ];
    let mut states = [
        AdaptiveControllerState::default(),
        AdaptiveControllerState::default(),
        AdaptiveControllerState::default(),
    ];
    let kinds = [
        ControllerKind::PredictiveCapped2,
        ControllerKind::PredictiveCapped,
        ControllerKind::Predictive,
    ];
    for (requested, trial, error, clipped) in attempts {
        let next: Vec<u64> = states
            .iter_mut()
            .zip(kinds)
            .map(|(state, kind)| {
                step(state, &config(kind), requested, trial, error, true, clipped).to_bits()
            })
            .collect();
        assert_eq!(next[0], next[1]);
        assert_eq!(next[0], next[2]);
        assert_eq!(states[0], states[1]);
    }
}

/// The history-free proposal of the new kind is the integral factor, as
/// for the other predictive kinds.
#[test]
fn history_free_proposal_of_predictive_capped2_is_the_integral_factor() {
    let mut state = AdaptiveControllerState::default();
    state.record_acceptance(0.05).unwrap();
    for error in [0.0, 0.5, 3.0] {
        for accepted in [true, false] {
            let p = state
                .propose_factor(
                    &config(ControllerKind::PredictiveCapped2),
                    error,
                    ORDER,
                    accepted,
                )
                .unwrap();
            let i = state
                .propose_factor(&config(ControllerKind::Integral), error, ORDER, accepted)
                .unwrap();
            assert_eq!(p.to_bits(), i.to_bits());
        }
    }
}
