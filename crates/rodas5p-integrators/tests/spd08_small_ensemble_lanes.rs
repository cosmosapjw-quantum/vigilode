//! Identity contracts of the lane-batched small driver (speed research node
//! `research/spd08_small_ensemble_lanes_20261007`): every member of a batch
//! run equals the scalar small driver bitwise (final and output states,
//! output times, attempts, accepted and rejected steps, Jacobian reuses,
//! every counter, internal and clipped steps, success and message, errors),
//! on the van der Pol ensemble with 4 and 8 lanes, with fewer members than
//! lanes and a ragged last round, and with injected failures: an oversized
//! initial step that causes typed linear-solve rejections, and a non-finite
//! right-hand side in one lane.
//!
//! With `SPD08_CONTRACT_OUTCOME=<path>`, the failure-masking test writes its
//! outcome as JSON for `tools/spd08_ensemble_check.py`.

use rodas5p_core::CoreResult;
use rodas5p_integrators::{
    AdaptiveStepConfig, BatchProblem, OutputSchedule, RODAS5P_FAST_SMALL_BATCH_DRIVER_ID,
    RODAS5P_FAST_SMALL_DRIVER_ID, Rodas5pFastSmallResult, SmallBatchMember, SmallProblem,
    integrate_rodas5p_fast_small_batch, integrate_rodas5p_fast_small_observed,
};

/// van der Pol with the expression order of the CLI's `SmallVanDerPol`.
#[derive(Clone, Copy, Debug)]
struct VanDerPol {
    mu: f64,
}

impl SmallProblem<2> for VanDerPol {
    fn rhs(&self, y: &[f64; 2], out: &mut [f64; 2]) {
        let mu = self.mu;
        out[0] = y[1];
        out[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
    }
    fn jacobian(&self, y: &[f64; 2], out: &mut [[f64; 2]; 2]) {
        let mu = self.mu;
        out[0][0] = 0.0;
        out[0][1] = 1.0;
        out[1][0] = -2.0 * mu * y[0] * y[1] - 1.0;
        out[1][1] = mu * (1.0 - y[0] * y[0]);
    }
}

impl<const B: usize> BatchProblem<2, B> for VanDerPol {
    type Lanes = [f64; B];
    fn lanes(member: &Self) -> [f64; B] {
        [member.mu; B]
    }
    fn set_lane(lanes: &mut [f64; B], lane: usize, member: &Self) {
        lanes[lane] = member.mu;
    }
    fn rhs_batch(mu: &[f64; B], y: &[[f64; B]; 2], out: &mut [[f64; B]; 2]) {
        for l in 0..B {
            let mu = mu[l];
            out[0][l] = y[1][l];
            out[1][l] = mu * (1.0 - y[0][l] * y[0][l]) * y[1][l] - y[0][l];
        }
    }
}

/// van der Pol with two failure injections. Where `|y0| > cap` (only
/// reached by the stage states of oversized steps) the right-hand side is
/// huge but finite, so the stage solve overflows: a typed linear-solve
/// failure. Where `y1 > poison` the right-hand side is NaN: a non-finite
/// failure. With `singular`, the Jacobian is NaN, so every factorization
/// fails (a typed linear-solve failure before the stages) until the step
/// falls below the minimum step.
#[derive(Clone, Copy, Debug)]
struct Trap {
    mu: f64,
    cap: f64,
    poison: f64,
    singular: bool,
}

impl Trap {
    fn eval(&self, y0: f64, y1: f64) -> (f64, f64) {
        let mu = self.mu;
        let mut out = (y1, mu * (1.0 - y0 * y0) * y1 - y0);
        if y0.abs() > self.cap {
            out = (1.0e308, -1.0e308);
        }
        if y1 > self.poison {
            out.1 = f64::NAN;
        }
        out
    }
}

impl SmallProblem<2> for Trap {
    fn rhs(&self, y: &[f64; 2], out: &mut [f64; 2]) {
        (out[0], out[1]) = self.eval(y[0], y[1]);
    }
    fn jacobian(&self, y: &[f64; 2], out: &mut [[f64; 2]; 2]) {
        VanDerPol { mu: self.mu }.jacobian(y, out);
        if self.singular {
            out[1][1] = f64::NAN;
        }
    }
}

impl<const B: usize> BatchProblem<2, B> for Trap {
    type Lanes = [Trap; B];
    fn lanes(member: &Self) -> [Trap; B] {
        [*member; B]
    }
    fn set_lane(lanes: &mut [Trap; B], lane: usize, member: &Self) {
        lanes[lane] = *member;
    }
    fn rhs_batch(lanes: &[Trap; B], y: &[[f64; B]; 2], out: &mut [[f64; B]; 2]) {
        for (l, problem) in lanes.iter().enumerate() {
            (out[0][l], out[1][l]) = problem.eval(y[0][l], y[1][l]);
        }
    }
}

fn config(rtol: f64, initial_step: f64, span: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol,
        rtol,
        initial_step,
        min_step: 1.0e-14,
        max_step: span,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Bitwise equality of a batch member with the scalar driver's result.
fn assert_same(
    member: usize,
    scalar: &CoreResult<Rodas5pFastSmallResult>,
    batch: &CoreResult<Rodas5pFastSmallResult>,
) {
    match (scalar, batch) {
        (Ok(s), Ok(b)) => {
            assert_eq!(s.driver, RODAS5P_FAST_SMALL_DRIVER_ID);
            assert_eq!(b.driver, RODAS5P_FAST_SMALL_BATCH_DRIVER_ID);
            assert_eq!(s.attempts, b.attempts, "member {member}: attempts");
            assert_eq!(
                s.accepted_steps, b.accepted_steps,
                "member {member}: accepted"
            );
            assert_eq!(
                s.rejected_steps, b.rejected_steps,
                "member {member}: rejected"
            );
            assert_eq!(
                s.jacobian_reuses, b.jacobian_reuses,
                "member {member}: reuses"
            );
            let (s, b) = (&s.observed, &b.observed);
            assert_eq!(s.counters, b.counters, "member {member}: counters");
            assert_eq!(s.success, b.success, "member {member}: success");
            assert_eq!(s.message, b.message, "member {member}: message");
            assert_eq!(
                s.internal_steps, b.internal_steps,
                "member {member}: internal steps"
            );
            assert_eq!(
                s.output_clipped_steps, b.output_clipped_steps,
                "member {member}: clipped steps"
            );
            assert_eq!(bits(&s.t), bits(&b.t), "member {member}: output times");
            assert_eq!(s.y.len(), b.y.len(), "member {member}: outputs");
            for (ys, yb) in s.y.iter().zip(&b.y) {
                assert_eq!(bits(ys), bits(yb), "member {member}: output states");
            }
        }
        (Err(s), Err(b)) => assert_eq!(format!("{s:?}"), format!("{b:?}"), "member {member}"),
        _ => panic!("member {member}: scalar {scalar:?} vs batch {batch:?}"),
    }
}

fn scalar_runs<P: SmallProblem<2>>(
    members: &[SmallBatchMember<2, P>],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> Vec<CoreResult<Rodas5pFastSmallResult>> {
    members
        .iter()
        .map(|m| {
            integrate_rodas5p_fast_small_observed(&m.problem, m.t_span, &m.y0, adaptive, output)
        })
        .collect()
}

fn assert_lanes_equal_scalar<const B: usize, P: BatchProblem<2, B>>(
    members: &[SmallBatchMember<2, P>],
    scalar: &[CoreResult<Rodas5pFastSmallResult>],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) {
    let batch = integrate_rodas5p_fast_small_batch::<2, B, P>(members, adaptive, output);
    assert_eq!(batch.len(), scalar.len());
    for (k, (s, b)) in scalar.iter().zip(&batch).enumerate() {
        assert_same(k, s, b);
    }
}

fn vdp_members(count: usize, span: (f64, f64)) -> Vec<SmallBatchMember<2, VanDerPol>> {
    (0..count)
        .map(|k| SmallBatchMember {
            problem: VanDerPol {
                mu: 1000.0 * (1.0 + k as f64 / count as f64),
            },
            t_span: span,
            y0: [2.0, 0.0],
        })
        .collect()
}

/// The registered ensemble: 64 members, `mu = 1000 (1 + k / 64)`, rtol 1e-6,
/// with the registered output schedule and with interior output times (steps
/// clipped to an output time).
#[test]
fn the_vdp_ensemble_on_lanes_equals_the_scalar_driver() {
    let members = vdp_members(64, (0.0, 2000.0));
    let adaptive = config(1.0e-6, 1.0e-6, 2000.0);
    for (interior, output) in [
        (false, OutputSchedule::new(vec![0.0, 2000.0]).unwrap()),
        (
            true,
            OutputSchedule::new(vec![0.0, 0.3, 700.0, 2000.0]).unwrap(),
        ),
    ] {
        let scalar = scalar_runs(&members, &adaptive, &output);
        let ok: Vec<_> = scalar.iter().map(|r| r.as_ref().unwrap()).collect();
        assert!(ok.iter().all(|r| r.observed.success));
        assert!(
            ok.iter()
                .all(|r| r.rejected_steps > 0 && r.jacobian_reuses > 0)
        );
        if interior {
            assert!(ok.iter().all(|r| r.observed.output_clipped_steps > 0));
        }
        assert_lanes_equal_scalar::<4, _>(&members, &scalar, &adaptive, &output);
        assert_lanes_equal_scalar::<8, _>(&members, &scalar, &adaptive, &output);
    }
}

/// Fewer members than lanes, member counts that are not a multiple of the
/// lane count, members with different initial states (lanes finish in
/// different rounds), and a single lane.
#[test]
fn ragged_ensembles_on_lanes_equal_the_scalar_driver() {
    let adaptive = config(1.0e-5, 1.0e-6, 2000.0);
    let output = OutputSchedule::new(vec![0.0, 2000.0]).unwrap();
    for count in [1, 3, 5, 13] {
        let mut members = vdp_members(count, (0.0, 2000.0));
        for (k, m) in members.iter_mut().enumerate() {
            if k % 3 == 1 {
                m.y0 = [-1.5, 0.25];
            }
        }
        let scalar = scalar_runs(&members, &adaptive, &output);
        assert!(scalar.iter().all(|r| r.as_ref().unwrap().observed.success));
        assert_lanes_equal_scalar::<1, _>(&members, &scalar, &adaptive, &output);
        assert_lanes_equal_scalar::<4, _>(&members, &scalar, &adaptive, &output);
        assert_lanes_equal_scalar::<8, _>(&members, &scalar, &adaptive, &output);
    }
}

/// A member the scalar driver rejects (a reversed span, a non-finite initial
/// state) gets the scalar driver's error at its position; an attempt limit
/// ends members unsuccessfully with the partial outputs.
#[test]
fn rejected_and_exhausted_members_equal_the_scalar_driver() {
    let output = OutputSchedule::new(vec![0.0, 300.0]).unwrap();
    let mut members = vdp_members(7, (0.0, 300.0));
    members[2].y0 = [f64::NAN, 0.0];
    members[5].t_span = (300.0, 0.0);
    let adaptive = config(1.0e-5, 1.0e-6, 300.0);
    let scalar = scalar_runs(&members, &adaptive, &output);
    assert!(scalar[2].is_err() && scalar[5].is_err());
    assert_lanes_equal_scalar::<4, _>(&members, &scalar, &adaptive, &output);
    assert_lanes_equal_scalar::<8, _>(&members, &scalar, &adaptive, &output);
    let limited = AdaptiveStepConfig {
        max_attempts: 9,
        ..adaptive
    };
    let scalar = scalar_runs(&members, &limited, &output);
    assert!(
        scalar
            .iter()
            .any(|r| r.as_ref().is_ok_and(|r| !r.observed.success))
    );
    assert_lanes_equal_scalar::<4, _>(&members, &scalar, &limited, &output);
    assert_lanes_equal_scalar::<8, _>(&members, &scalar, &limited, &output);
}

fn trap_members(
    count: usize,
    poisoned: Option<usize>,
    singular: Option<usize>,
) -> Vec<SmallBatchMember<2, Trap>> {
    (0..count)
        .map(|k| SmallBatchMember {
            problem: Trap {
                mu: 1000.0 * (1.0 + k as f64 / count as f64),
                cap: 2.5,
                poison: if Some(k) == poisoned {
                    0.5
                } else {
                    f64::INFINITY
                },
                singular: Some(k) == singular,
            },
            t_span: (0.0, 2000.0),
            y0: [2.0, 0.0],
        })
        .collect()
}

/// Failure masking (gate item 3): typed linear-solve failures from an
/// oversized initial step in some lanes of the same round, a member whose
/// every factorization fails, and a non-finite right-hand side in one lane;
/// every member equals the scalar driver. With `SPD08_CONTRACT_OUTCOME` set,
/// the outcome is written there as JSON once every assertion has held.
#[test]
fn injected_failures_are_masked_per_lane() {
    let output = OutputSchedule::new(vec![0.0, 2000.0]).unwrap();
    let mut compared = 0;
    // Oversized initial step (the whole span): the first attempt of members
    // 0 and 1 overflows in the stage solve while the other lanes of the
    // round go on; member 5 never factors and ends unsuccessfully.
    let members = trap_members(10, None, Some(5));
    let adaptive = config(1.0e-6, 2000.0, 2000.0);
    let scalar = scalar_runs(&members, &adaptive, &output);
    let counters: Vec<_> = scalar
        .iter()
        .map(|r| r.as_ref().unwrap().observed.counters)
        .collect();
    let solve_failures: Vec<u64> = counters.iter().map(|c| c.linear_solve_failures).collect();
    let failing_lanes = solve_failures.iter().filter(|f| **f > 0).count();
    assert!(
        failing_lanes >= 2 && failing_lanes < members.len(),
        "typed linear-solve failures in some lanes only: {solve_failures:?}"
    );
    let singular = scalar[5].as_ref().unwrap();
    assert!(!singular.observed.success);
    assert_eq!(singular.rejected_steps, singular.attempts);
    assert_eq!(
        counters[5].linear_solve_failures as usize,
        singular.attempts
    );
    assert!(
        scalar
            .iter()
            .enumerate()
            .all(|(k, r)| k == 5 || r.as_ref().unwrap().observed.success)
    );
    assert_lanes_equal_scalar::<4, _>(&members, &scalar, &adaptive, &output);
    assert_lanes_equal_scalar::<8, _>(&members, &scalar, &adaptive, &output);
    compared += 2 * members.len();
    let linear_solve_failures: u64 = solve_failures.iter().sum();
    // A non-finite right-hand side in member 2 only (lane 2 of the first
    // round): it is rejected with a fresh Jacobian afterwards until its step
    // falls below the minimum step; the other lanes are unaffected.
    let members = trap_members(10, Some(2), None);
    let adaptive = config(1.0e-6, 1.0e-6, 2000.0);
    let scalar = scalar_runs(&members, &adaptive, &output);
    let nonfinite: Vec<u64> = scalar
        .iter()
        .map(|r| {
            r.as_ref()
                .unwrap()
                .observed
                .counters
                .nonfinite_step_failures
        })
        .collect();
    assert!(nonfinite[2] > 0, "{nonfinite:?}");
    assert!(nonfinite.iter().enumerate().all(|(k, n)| k == 2 || *n == 0));
    assert_lanes_equal_scalar::<4, _>(&members, &scalar, &adaptive, &output);
    assert_lanes_equal_scalar::<8, _>(&members, &scalar, &adaptive, &output);
    compared += 2 * members.len();
    if let Some(path) = std::env::var_os("SPD08_CONTRACT_OUTCOME") {
        let outcome = serde_json::json!({
            "schema": "vigilode-spd08-contract-outcome-v1",
            "test": "injected_failures_are_masked_per_lane",
            "lanes": [4, 8],
            "members_compared": compared,
            "oversized_step_linear_solve_failures": linear_solve_failures,
            "oversized_step_failing_lanes": failing_lanes,
            "singular_member_factorization_failures": counters[5].linear_solve_failures,
            "nonfinite_failures_poisoned_lane": nonfinite[2],
            "nonfinite_failures_other_lanes": nonfinite.iter().sum::<u64>() - nonfinite[2],
            "identical": true,
        });
        std::fs::write(path, serde_json::to_string_pretty(&outcome).unwrap() + "\n").unwrap();
    }
}
