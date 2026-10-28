//! DP-02 (`docs/reviews/20261018_production_audit/DEFAULT_PATH_KO.md`):
//! user callback work is charged when it is attempted, before user code
//! runs, so a callback that returns `Err` or a non-finite value still
//! appears in the counters. The residues closed here are the JVP-assembled
//! dense Jacobian and the small (static-dispatch) driver with its lane-batched
//! variant. Each test checks that the counter equals the number of times user
//! code was entered and that the failure itself is unchanged: the same error,
//! or the same unsuccessful run with no accepted step.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, BatchProblem, OdeProblem, OutputSchedule, Rodas5pFastOptions,
    Rodas5pFastSmallOptions, Rodas5pFastSmallResult, SmallBatchMember, SmallProblem,
    integrate_rodas5p_fast_observed, integrate_rodas5p_fast_small_batch,
    integrate_rodas5p_fast_small_observed_with_small_options,
};

const N: usize = 3;

/// How the JVP callback misbehaves on the column `fail_column` of the
/// assembled Jacobian (every other column is `-v`).
#[derive(Clone, Copy, Debug)]
enum JvpFault {
    None,
    Err,
    NaN,
}

/// A JVP-only problem `y' = -y` whose JVP counts every entry into user code.
fn jvp_only_problem(fault: JvpFault, fail_column: usize) -> (OdeProblem, Arc<AtomicUsize>) {
    let entries = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&entries);
    let problem = OdeProblem::new(
        "dp02-jvp-only",
        N,
        Arc::new(|_, y, out| {
            for (o, v) in out.iter_mut().zip(y) {
                *o = -v;
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            seen.fetch_add(1, Ordering::SeqCst);
            for (o, x) in out.iter_mut().zip(v) {
                *o = -x;
            }
            if v[fail_column] == 1.0 {
                match fault {
                    JvpFault::None => {}
                    JvpFault::Err => {
                        return Err(CoreError::InvalidInput("synthetic JVP failure".into()));
                    }
                    JvpFault::NaN => out[fail_column] = f64::NAN,
                }
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    (problem, entries)
}

fn jvp_counts(counters: &WorkCounters) -> (u64, u64, u64) {
    (
        counters.jvp_calls,
        counters.jvp_vectors,
        counters.jacobian_builds,
    )
}

#[test]
fn jvp_assembled_jacobian_charges_attempted_columns_on_err_and_nonfinite() {
    let y = [1.0, 2.0, 3.0];
    for (fault, fail_column) in [(JvpFault::Err, 1), (JvpFault::NaN, 1), (JvpFault::Err, 0)] {
        let (problem, entries) = jvp_only_problem(fault, fail_column);
        let mut counters = WorkCounters::default();
        let error = problem.dense_jacobian(0.0, &y, &mut counters).unwrap_err();
        match fault {
            JvpFault::Err => assert!(matches!(error, CoreError::InvalidInput(_)), "{error:?}"),
            _ => assert!(matches!(error, CoreError::NonFinite(_)), "{error:?}"),
        }
        let attempted = fail_column as u64 + 1;
        assert_eq!(entries.load(Ordering::SeqCst) as u64, attempted);
        // Every attempted column is charged, and the build was attempted once.
        assert_eq!(jvp_counts(&counters), (attempted, attempted, 1));

        // The in-place entry falls back to the same assembly.
        let mut out = DenseMatrix::zeros(N, N);
        let mut counters = WorkCounters::default();
        assert!(
            problem
                .dense_jacobian_into(0.0, &y, &mut out, &mut counters)
                .is_err()
        );
        assert_eq!(jvp_counts(&counters), (attempted, attempted, 1));
    }

    // Success charges what it always did: one JVP per column and one build.
    let (problem, entries) = jvp_only_problem(JvpFault::None, 0);
    let mut counters = WorkCounters::default();
    let matrix = problem.dense_jacobian(0.0, &y, &mut counters).unwrap();
    assert_eq!(matrix[(1, 1)], -1.0);
    assert_eq!(entries.load(Ordering::SeqCst), N);
    assert_eq!(jvp_counts(&counters), (N as u64, N as u64, 1));

    // A library-side shape refusal enters no user code and charges nothing.
    let mut counters = WorkCounters::default();
    assert!(problem.dense_jacobian(0.0, &[1.0], &mut counters).is_err());
    assert_eq!(jvp_counts(&counters), (0, 0, 0));
    assert_eq!(entries.load(Ordering::SeqCst), N);
}

fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-8,
        initial_step: 1.0e-2,
        min_step: 1.0e-6,
        max_step: 1.0,
        max_attempts: 10_000,
        ..AdaptiveStepConfig::default()
    }
}

#[test]
fn dense_fast_driver_charges_failed_jvp_columns() {
    let output = OutputSchedule::new(vec![0.0, 1.0]).unwrap();

    // A non-finite column is a typed step failure: the run shrinks the step
    // to the minimum and ends unsuccessfully, every attempt charged.
    let (problem, entries) = jvp_only_problem(JvpFault::NaN, 0);
    let run =
        integrate_rodas5p_fast_observed(&problem, (0.0, 1.0), &[1.0; N], &adaptive(), &output)
            .unwrap();
    let observed = &run.observed;
    assert!(!observed.success);
    assert_eq!(run.accepted_steps, 0);
    assert!(run.attempts >= 1);
    assert_eq!(run.rejected_steps, run.attempts);
    assert_eq!(
        observed.counters.nonfinite_step_failures,
        run.attempts as u64
    );
    let entered = entries.load(Ordering::SeqCst) as u64;
    assert!(entered >= 1);
    assert_eq!(observed.counters.jvp_calls, entered);
    assert_eq!(observed.counters.jvp_vectors, entered);
    assert_eq!(observed.counters.jacobian_builds, run.attempts as u64);

    // A callback `Err` is not a step failure: it ends the run with that error.
    let (problem, entries) = jvp_only_problem(JvpFault::Err, 0);
    let error =
        integrate_rodas5p_fast_observed(&problem, (0.0, 1.0), &[1.0; N], &adaptive(), &output)
            .unwrap_err();
    assert!(matches!(error, CoreError::InvalidInput(_)), "{error:?}");
    assert_eq!(entries.load(Ordering::SeqCst), 1);
}

/// `y' = -y` for the small driver; the right-hand side is NaN wherever
/// `y[0] < poison`, and every call is counted.
struct Poisoned {
    poison: f64,
    entries: AtomicUsize,
}

impl Poisoned {
    fn new(poison: f64) -> Self {
        Self {
            poison,
            entries: AtomicUsize::new(0),
        }
    }
    fn eval(poison: f64, y: &[f64; 2], out: &mut [f64; 2]) {
        out[0] = -y[0];
        out[1] = -2.0 * y[1];
        if y[0] < poison {
            out[1] = f64::NAN;
        }
    }
}

impl SmallProblem<2> for Poisoned {
    fn rhs(&self, y: &[f64; 2], out: &mut [f64; 2]) {
        self.entries.fetch_add(1, Ordering::SeqCst);
        Self::eval(self.poison, y, out);
    }
    fn jacobian(&self, _: &[f64; 2], out: &mut [[f64; 2]; 2]) {
        out[0][0] = -1.0;
        out[1][1] = -2.0;
    }
}

impl<const B: usize> BatchProblem<2, B> for Poisoned {
    type Lanes = [f64; B];
    fn lanes(member: &Self) -> [f64; B] {
        [member.poison; B]
    }
    fn set_lane(lanes: &mut [f64; B], lane: usize, member: &Self) {
        lanes[lane] = member.poison;
    }
    fn rhs_batch(lanes: &[f64; B], y: &[[f64; B]; 2], out: &mut [[f64; B]; 2]) {
        for (l, poison) in lanes.iter().enumerate() {
            let mut lane_out = [0.0; 2];
            Self::eval(*poison, &[y[0][l], y[1][l]], &mut lane_out);
            out[0][l] = lane_out[0];
            out[1][l] = lane_out[1];
        }
    }
}

fn small_options() -> [Rodas5pFastSmallOptions; 2] {
    [
        Rodas5pFastSmallOptions::default(),
        Rodas5pFastSmallOptions {
            fast: Rodas5pFastOptions::default(),
            static_stages: true,
            static_solves: true,
        },
    ]
}

fn run_small(problem: &Poisoned, options: Rodas5pFastSmallOptions) -> Rodas5pFastSmallResult {
    let output = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    integrate_rodas5p_fast_small_observed_with_small_options(
        problem,
        (0.0, 1.0),
        &[1.0, 1.0],
        &adaptive(),
        &output,
        options,
    )
    .unwrap()
}

#[test]
fn small_driver_charges_nonfinite_rhs_attempts() {
    for options in small_options() {
        // NaN from the first call: no step is ever accepted.
        let problem = Poisoned::new(f64::INFINITY);
        let run = run_small(&problem, options);
        let counters = &run.observed.counters;
        assert!(!run.observed.success);
        assert_eq!(run.accepted_steps, 0);
        assert!(run.attempts >= 1);
        assert_eq!(counters.nonfinite_step_failures, run.attempts as u64);
        let entered = problem.entries.load(Ordering::SeqCst) as u64;
        assert!(entered >= 1);
        assert_eq!(
            (counters.rhs_calls, counters.rhs_evaluations),
            (entered, entered)
        );

        // NaN only past `y[0] < 0.5`: the run stops there, unsuccessful, after
        // accepted steps and finite stage calls. Every call is charged.
        let problem = Poisoned::new(0.5);
        let run = run_small(&problem, options);
        let counters = &run.observed.counters;
        assert!(!run.observed.success);
        assert!(run.accepted_steps > 0);
        assert!(counters.nonfinite_step_failures > 0);
        let entered = problem.entries.load(Ordering::SeqCst) as u64;
        assert_eq!(
            (counters.rhs_calls, counters.rhs_evaluations),
            (entered, entered)
        );

        // A clean run is charged as before: every call, all finite.
        let problem = Poisoned::new(f64::NEG_INFINITY);
        let run = run_small(&problem, options);
        assert!(run.observed.success);
        let entered = problem.entries.load(Ordering::SeqCst) as u64;
        assert_eq!(run.observed.counters.rhs_evaluations, entered);
    }
}

fn same_run(
    member: usize,
    scalar: &CoreResult<Rodas5pFastSmallResult>,
    batch: &CoreResult<Rodas5pFastSmallResult>,
) {
    let (s, b) = (scalar.as_ref().unwrap(), batch.as_ref().unwrap());
    assert_eq!(
        (s.attempts, s.accepted_steps, s.rejected_steps),
        (b.attempts, b.accepted_steps, b.rejected_steps),
        "member {member}"
    );
    let (s, b) = (&s.observed, &b.observed);
    assert_eq!(s.counters, b.counters, "member {member}: counters");
    assert_eq!(
        (s.success, &s.message),
        (b.success, &b.message),
        "member {member}"
    );
    let bits = |v: &[Vec<f64>]| -> Vec<Vec<u64>> {
        v.iter()
            .map(|r| r.iter().map(|x| x.to_bits()).collect())
            .collect()
    };
    assert_eq!(bits(&s.y), bits(&b.y), "member {member}: outputs");
}

#[test]
fn small_batch_lanes_charge_nonfinite_rhs_like_the_scalar_driver() {
    let output = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    let adaptive = adaptive();
    let poisons = [
        f64::NEG_INFINITY,
        f64::INFINITY,
        0.5,
        f64::NEG_INFINITY,
        0.7,
    ];
    let members: Vec<SmallBatchMember<2, Poisoned>> = poisons
        .iter()
        .map(|&poison| SmallBatchMember {
            problem: Poisoned::new(poison),
            t_span: (0.0, 1.0),
            y0: [1.0, 1.0],
        })
        .collect();
    let scalar: Vec<_> = members
        .iter()
        .map(|m| {
            integrate_rodas5p_fast_small_observed_with_small_options(
                &m.problem,
                m.t_span,
                &m.y0,
                &adaptive,
                &output,
                Rodas5pFastSmallOptions::default(),
            )
        })
        .collect();
    // The always-NaN member is charged for its failed attempts.
    let poisoned = scalar[1].as_ref().unwrap();
    assert!(!poisoned.observed.success);
    assert!(poisoned.observed.counters.rhs_evaluations >= 1);
    for batch in [
        integrate_rodas5p_fast_small_batch::<2, 2, _>(&members, &adaptive, &output),
        integrate_rodas5p_fast_small_batch::<2, 4, _>(&members, &adaptive, &output),
    ] {
        for (k, (s, b)) in scalar.iter().zip(&batch).enumerate() {
            same_run(k, s, b);
        }
    }
}
