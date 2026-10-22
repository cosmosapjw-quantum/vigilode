//! Contracts of the accept control of the fast driver (library step L3,
//! [`AcceptedStepView`], [`AcceptControl`]): a control that always continues
//! leaves the run bit for bit; a stop at the k-th accepted step returns that
//! step's state, proposal and counters as a `ClientStop` partial; a restart
//! from it reaches `tf`; a stop at `tf` is a success; the control is called
//! once per accepted step and allocates nothing. Allocations are counted per
//! thread, as in `rodas5p_fast_allocations.rs`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, AdaptiveStepConfig, BandedJacobian, BandedStageLu, BandedWork,
    ControllerKind, DriverPartial, DriverStopReason, OdeProblem, OutputSchedule, Rodas5pFastResult,
    StageLinearSystem, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_banded_observed_with_atol_and_control,
    integrate_rodas5p_fast_client_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
};

#[path = "b1_problems/mod.rs"]
mod b1_problems;
use b1_problems::{Case, brusselator_case, config, hires_case, schedule};

struct Counting;

thread_local! {
    // A const-initialized `Cell` needs no allocation and no destructor, so
    // the allocator can use it on any thread at any time.
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// Allocations made by this thread during `f`.
fn allocations_during<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.with(Cell::get);
    let value = f();
    (value, ALLOCATIONS.with(Cell::get) - before)
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

type Control<'c> = &'c mut dyn FnMut(&AcceptedStepView<'_>) -> AcceptControl;

/// The 1D Brusselator's band (u and v interleaved: lower = upper = 2), as
/// in `fast_vector_atol_contracts.rs`.
fn brusselator_band(cells: usize) -> BandedJacobian {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |_t, y: &[f64], band: &mut [f64]| {
            let at = |band: &mut [f64], i: usize, j: usize, v: f64| {
                band[i * 5 + (j + 2 - i)] = v;
            };
            for i in 0..cells {
                let (u, v) = (y[2 * i], y[2 * i + 1]);
                let (a, b) = (2 * i, 2 * i + 1);
                at(band, a, a, 2.0 * u * v - 4.0 - 2.0 * c);
                at(band, a, b, u * u);
                at(band, b, a, 3.0 - 2.0 * u * v);
                at(band, b, b, -u * u - 2.0 * c);
                if i > 0 {
                    at(band, a, a - 2, c);
                    at(band, b, b - 2, c);
                }
                if i + 1 < cells {
                    at(band, a, a + 2, c);
                    at(band, b, b + 2, c);
                }
            }
            Ok(())
        }),
    }
}

/// The band `lower = upper = 2` of the problem's dense Jacobian, as in
/// `client_stage_system_contracts.rs` (HIRES has no entry outside it).
fn dense_band(problem: &OdeProblem) -> BandedJacobian {
    let problem = problem.clone();
    BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |t, y: &[f64], band: &mut [f64]| {
            let n = problem.dimension;
            let j = problem.dense_jacobian(t, y, &mut WorkCounters::default())?;
            for i in 0..n {
                for c in 0..n {
                    if i.abs_diff(c) <= 2 {
                        band[i * 5 + (c + 2 - i)] = j[(i, c)];
                    } else if j[(i, c)] != 0.0 {
                        return Err(CoreError::InvalidInput(format!(
                            "Jacobian entry ({i}, {c}) outside the band"
                        )));
                    }
                }
            }
            Ok(())
        }),
    }
}

/// A nonuniform per-component tolerance: `atol`, `2 atol`, `3 atol`, ...
fn atol_vector(n: usize, adaptive: &AdaptiveStepConfig) -> Vec<f64> {
    (0..n).map(|i| adaptive.atol * (1 + i % 3) as f64).collect()
}

/// A problem and the entry point it runs through: the banded driver
/// (Brusselator) or the client driver with [`BandedStageLu`] (HIRES).
struct Setup {
    case: Case,
    band: BandedJacobian,
    client: bool,
}

/// The parts of a run the contracts compare.
struct Run {
    fast: Rodas5pFastResult,
    partial: Option<DriverPartial>,
    banded: Option<BandedWork>,
}

impl Setup {
    fn brusselator() -> Self {
        Self {
            case: brusselator_case(),
            band: brusselator_band(50),
            client: false,
        }
    }

    fn hires() -> Self {
        let case = hires_case();
        let band = dense_band(&case.problem);
        Self {
            case,
            band,
            client: true,
        }
    }

    fn label(&self, rtol: f64) -> String {
        let arm = if self.client { "client" } else { "banded" };
        format!("{} ({arm}) at rtol {rtol:e}", self.case.id)
    }

    /// The run from `(span.0, y0)` with the case's band and the
    /// nonuniform tolerance of `adaptive`, without a control (`None`) or
    /// through the `_and_control` entry point.
    fn run(
        &self,
        span: (f64, f64),
        y0: &[f64],
        adaptive: &AdaptiveStepConfig,
        output: &OutputSchedule,
        control: Option<Control<'_>>,
    ) -> Run {
        let problem = &self.case.problem;
        let n = problem.dimension;
        let atol = atol_vector(n, adaptive);
        if self.client {
            let client = Box::new(BandedStageLu::new(n, &self.band).unwrap());
            let (fast, partial, _client) = match control {
                None => integrate_rodas5p_fast_client_observed_with_atol(
                    problem, client, span, y0, adaptive, output, &atol,
                ),
                Some(control) => integrate_rodas5p_fast_client_observed_with_atol_and_control(
                    problem, client, span, y0, adaptive, output, &atol, control,
                ),
            }
            .unwrap();
            Run {
                fast,
                partial,
                banded: None,
            }
        } else {
            let banded = match control {
                None => integrate_rodas5p_fast_banded_observed_with_atol(
                    problem, &self.band, span, y0, adaptive, output, &atol,
                ),
                Some(control) => integrate_rodas5p_fast_banded_observed_with_atol_and_control(
                    problem, &self.band, span, y0, adaptive, output, &atol, control,
                ),
            }
            .unwrap();
            Run {
                fast: banded.fast,
                partial: banded.partial,
                banded: Some(banded.work),
            }
        }
    }
}

/// The run `b` equals the run `a`: outputs bit for bit, counters, step
/// statistics, success, partial record and band work.
fn assert_same_run(a: &Run, b: &Run, label: &str) {
    let (x, y) = (&a.fast, &b.fast);
    assert_eq!(bits(&x.observed.t), bits(&y.observed.t), "{label}");
    assert_eq!(x.observed.y.len(), y.observed.y.len(), "{label}");
    for (p, q) in x.observed.y.iter().zip(&y.observed.y) {
        assert_eq!(bits(p), bits(q), "{label}");
    }
    assert_eq!(x.observed.counters, y.observed.counters, "{label}");
    assert_eq!(x.observed.success, y.observed.success, "{label}");
    assert_eq!(x.observed.message, y.observed.message, "{label}");
    assert_eq!(
        x.observed.internal_steps, y.observed.internal_steps,
        "{label}"
    );
    assert_eq!(
        x.observed.output_clipped_steps, y.observed.output_clipped_steps,
        "{label}"
    );
    assert_eq!(x.attempts, y.attempts, "{label}");
    assert_eq!(x.accepted_steps, y.accepted_steps, "{label}");
    assert_eq!(x.rejected_steps, y.rejected_steps, "{label}");
    assert_eq!(x.jacobian_reuses, y.jacobian_reuses, "{label}");
    assert_eq!((x.lu, x.driver), (y.lu, y.driver), "{label}");
    assert_eq!(a.partial, b.partial, "{label}");
    assert_eq!(a.banded, b.banded, "{label}");
}

/// A copy of one [`AcceptedStepView`] (the test's control may allocate).
#[derive(Clone, Debug)]
struct Seen {
    t: f64,
    y: Vec<f64>,
    h_used: f64,
    h_next: f64,
    accepted_steps: usize,
    attempts: usize,
    counters: WorkCounters,
}

impl Seen {
    fn of(view: &AcceptedStepView<'_>) -> Self {
        Self {
            t: view.t,
            y: view.y.to_vec(),
            h_used: view.h_used,
            h_next: view.h_next,
            accepted_steps: view.accepted_steps,
            attempts: view.attempts,
            counters: *view.counters,
        }
    }
}

/// The full run under a control that records every view and continues.
fn recorded_run(
    setup: &Setup,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> (Run, Vec<Seen>) {
    let mut seen = Vec::new();
    let mut record = |view: &AcceptedStepView<'_>| {
        seen.push(Seen::of(view));
        AcceptControl::Continue
    };
    let case = &setup.case;
    let run = setup.run(case.span, &case.y0, adaptive, output, Some(&mut record));
    (run, seen)
}

/// The run that stops at its `k`-th accepted step, and the control's calls.
fn stopped_run(
    setup: &Setup,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    k: usize,
) -> (Run, usize) {
    let mut calls = 0;
    let mut stop_at_k = |view: &AcceptedStepView<'_>| {
        calls += 1;
        if view.accepted_steps == k {
            AcceptControl::Stop
        } else {
            AcceptControl::Continue
        }
    };
    let case = &setup.case;
    let run = setup.run(case.span, &case.y0, adaptive, output, Some(&mut stop_at_k));
    (run, calls)
}

/// The first accepted step (1-based) at or after the first output time
/// after `t0`: a stop there has collected an output beside `t0`.
fn first_step_past_an_output(seen: &[Seen], output: &OutputSchedule) -> usize {
    seen.iter()
        .position(|view| view.t >= output.times()[1])
        .expect("a step past the first output")
        + 1
}

/// (a) and (e): a control that always continues gives the run without one,
/// bit for bit, and is called once per accepted step.
#[test]
fn a_continuing_control_leaves_the_run_bit_identical() {
    let mut rejected = 0;
    for setup in [Setup::brusselator(), Setup::hires()] {
        let case = &setup.case;
        let output = schedule(case.span, 10);
        for rtol in [1.0e-6, 1.0e-9] {
            let label = setup.label(rtol);
            let adaptive = config(case, rtol);
            let reference = setup.run(case.span, &case.y0, &adaptive, &output, None);
            let mut calls = 0;
            let mut last = None;
            let mut continue_always = |view: &AcceptedStepView<'_>| {
                calls += 1;
                last = Some((view.t, view.accepted_steps, view.attempts, *view.counters));
                AcceptControl::Continue
            };
            let controlled = setup.run(
                case.span,
                &case.y0,
                &adaptive,
                &output,
                Some(&mut continue_always),
            );
            assert!(reference.fast.observed.success, "{label}");
            assert!(reference.partial.is_none(), "{label}");
            assert_same_run(&reference, &controlled, &label);
            let fast = &controlled.fast;
            assert_eq!(calls, fast.accepted_steps, "{label}");
            // The last call saw the step that reached tf and the final work.
            assert_eq!(
                last,
                Some((
                    case.span.1,
                    fast.accepted_steps,
                    fast.attempts,
                    fast.observed.counters
                )),
                "{label}"
            );
            eprintln!(
                "{label}: attempts {}, accepted {}, rejected {}, reuses {}",
                fast.attempts, fast.accepted_steps, fast.rejected_steps, fast.jacobian_reuses
            );
            rejected += fast.rejected_steps;
        }
    }
    // Rejected attempts happened, so "once per accepted step" is not "once
    // per attempt".
    assert!(rejected > 0);
}

/// (e): the views run over the accepted steps in order, with the attempts,
/// the counters and the state of each.
#[test]
fn the_control_sees_every_accepted_step_once_in_order() {
    for setup in [Setup::brusselator(), Setup::hires()] {
        let case = &setup.case;
        let output = schedule(case.span, 10);
        let label = setup.label(1.0e-6);
        let adaptive = config(case, 1.0e-6);
        let (run, seen) = recorded_run(&setup, &adaptive, &output);
        let fast = &run.fast;
        assert_eq!(seen.len(), fast.accepted_steps, "{label}");
        let mut t = case.span.0;
        for (i, view) in seen.iter().enumerate() {
            assert_eq!(view.accepted_steps, i + 1, "{label}");
            assert_eq!(view.counters.accepted_steps, (i + 1) as u64, "{label}");
            assert!(view.attempts >= view.accepted_steps, "{label}");
            assert_eq!(
                view.counters.rejected_steps,
                (view.attempts - view.accepted_steps) as u64,
                "{label}"
            );
            assert_eq!(view.t, t + view.h_used, "{label}");
            assert!(view.h_next > 0.0 && view.h_next.is_finite(), "{label}");
            assert_eq!(view.y.len(), case.problem.dimension, "{label}");
            if i > 0 {
                assert!(view.attempts > seen[i - 1].attempts, "{label}");
            }
            t = view.t;
        }
        let last = seen.last().unwrap();
        assert_eq!(last.t, case.span.1, "{label}");
        assert_eq!(last.attempts, fast.attempts, "{label}");
        assert_eq!(last.counters, fast.observed.counters, "{label}");
        assert_eq!(
            bits(&last.y),
            bits(fast.observed.y.last().unwrap()),
            "{label}"
        );
    }
}

/// (b): a stop at the k-th accepted step ends the run there: the partial
/// record holds that step's state and proposal, the counters and step
/// statistics are the full run's at that step, and the output holds every
/// output time at or before it.
#[test]
fn a_stop_at_the_kth_accepted_step_returns_that_step() {
    for setup in [Setup::brusselator(), Setup::hires()] {
        let case = &setup.case;
        let output = schedule(case.span, 10);
        let adaptive = config(case, 1.0e-6);
        let (full, seen) = recorded_run(&setup, &adaptive, &output);
        let collected_by = |t: f64| full.fast.observed.t.iter().filter(|&&s| s <= t).count();
        for k in [1, 5, 17, first_step_past_an_output(&seen, &output)] {
            let label = format!("{} stopped at step {k}", setup.label(1.0e-6));
            let at = &seen[k - 1];
            assert!(at.t < case.span.1, "{label}");
            let (stopped, calls) = stopped_run(&setup, &adaptive, &output, k);
            assert_eq!(calls, k, "{label}");
            let partial = stopped.partial.as_ref().expect("a partial record");
            assert_eq!(partial.reason, DriverStopReason::ClientStop, "{label}");
            assert_eq!(partial.last_failure, None, "{label}");
            assert_eq!(partial.t.to_bits(), at.t.to_bits(), "{label}");
            assert_eq!(bits(&partial.y), bits(&at.y), "{label}");
            assert_eq!(
                partial.h_next.map(f64::to_bits),
                Some(at.h_next.to_bits()),
                "{label}"
            );
            let fast = &stopped.fast;
            assert!(!fast.observed.success, "{label}");
            assert!(fast.observed.message.contains("client"), "{label}");
            assert_eq!(fast.observed.counters, at.counters, "{label}");
            assert_eq!(fast.attempts, at.attempts, "{label}");
            assert_eq!(fast.accepted_steps, k, "{label}");
            assert_eq!(fast.rejected_steps, at.attempts - k, "{label}");
            assert_eq!(fast.observed.internal_steps, k, "{label}");
            // Every output time at or before t_k, as the full run took them.
            let m = collected_by(at.t);
            assert_eq!(
                bits(&fast.observed.t),
                bits(&full.fast.observed.t[..m]),
                "{label}"
            );
            for (a, b) in fast.observed.y.iter().zip(&full.fast.observed.y[..m]) {
                assert_eq!(bits(a), bits(b), "{label}");
            }
            assert_eq!(fast.observed.y.len(), m, "{label}");
        }
    }
}

/// (c): integrating from the stop `(t_k, y_k)` with `initial_step = h_next`
/// to `tf` succeeds, and its endpoint agrees with the uninterrupted run
/// within a weighted RMS of one. Whether it is bit for bit depends on the
/// controller. The restart keeps the state, the time and the proposal, and
/// linearizes afresh as the run does after every accepted step; what it
/// loses is the controller history, which the first controller update after
/// `t_k` would have read. The default `Integral` controller keeps none past
/// an accepted step (its factor depends on the current error alone and its
/// reject memory is cleared on acceptance), so there the restart is the
/// uninterrupted run bit for bit, work included. `Pi` reads the previous
/// accepted error, so its first update after `t_k` differs unless its
/// factor is clamped, and the steps and the endpoint differ (asserted for
/// at least one restart). `Predictive` reads the previous accepted step and
/// error, so it differs only where its predictive term would have bound
/// that update (not in these runs).
#[test]
fn a_restart_from_the_stop_reaches_tf() {
    let mut history_changed_a_restart = false;
    for setup in [Setup::brusselator(), Setup::hires()] {
        let case = &setup.case;
        let output = schedule(case.span, 10);
        let rtol = 1.0e-6;
        for controller in [
            ControllerKind::Integral,
            ControllerKind::Pi,
            ControllerKind::Predictive,
        ] {
            let adaptive = AdaptiveStepConfig {
                controller,
                ..config(case, rtol)
            };
            let (full, seen) = recorded_run(&setup, &adaptive, &output);
            assert!(full.fast.observed.success);
            let reference = full.fast.observed.y.last().unwrap();
            let atol = atol_vector(case.problem.dimension, &adaptive);
            for k in [1, 5, 17, first_step_past_an_output(&seen, &output)] {
                let label = format!(
                    "{} under {controller:?} restarted after step {k}",
                    setup.label(rtol)
                );
                let (stopped, _) = stopped_run(&setup, &adaptive, &output, k);
                let partial = stopped.partial.unwrap();
                // The remaining output: t_k, then every later output time.
                let mut times = vec![partial.t];
                times.extend(output.times().iter().copied().filter(|&s| s > partial.t));
                let rest = OutputSchedule::new(times).unwrap();
                let restart_config = AdaptiveStepConfig {
                    initial_step: partial.h_next.unwrap(),
                    ..adaptive.clone()
                };
                let restart = setup.run(
                    (partial.t, case.span.1),
                    &partial.y,
                    &restart_config,
                    &rest,
                    None,
                );
                let fast = &restart.fast;
                assert!(fast.observed.success, "{label}");
                assert!(restart.partial.is_none(), "{label}");
                assert_eq!(*fast.observed.t.last().unwrap(), case.span.1, "{label}");
                let y = fast.observed.y.last().unwrap();
                let sum: f64 = y
                    .iter()
                    .zip(reference)
                    .zip(&atol)
                    .map(|((a, b), atol)| {
                        let z = (a - b) / (atol + rtol * a.abs().max(b.abs()));
                        z * z
                    })
                    .sum();
                let wrms = (sum / y.len() as f64).sqrt();
                let bitwise = bits(y) == bits(reference);
                eprintln!(
                    "{label}: WRMS {wrms:.3e} (bitwise {bitwise}), attempts {} + {} vs {}",
                    stopped.fast.attempts, fast.attempts, full.fast.attempts
                );
                assert!(wrms <= 1.0, "{label}: WRMS {wrms:e}");
                if controller == ControllerKind::Integral {
                    // The later outputs and the work of the two pieces are
                    // the uninterrupted run's.
                    let later = full.fast.observed.t.len() - (fast.observed.t.len() - 1);
                    assert_eq!(
                        bits(&fast.observed.t[1..]),
                        bits(&full.fast.observed.t[later..]),
                        "{label}"
                    );
                    for (a, b) in fast.observed.y[1..]
                        .iter()
                        .zip(&full.fast.observed.y[later..])
                    {
                        assert_eq!(bits(a), bits(b), "{label}");
                    }
                    let mut work = stopped.fast.observed.counters;
                    work.accumulate(fast.observed.counters);
                    assert_eq!(work, full.fast.observed.counters, "{label}");
                    assert_eq!(
                        stopped.fast.attempts + fast.attempts,
                        full.fast.attempts,
                        "{label}"
                    );
                    assert_eq!(
                        stopped.fast.jacobian_reuses + fast.jacobian_reuses,
                        full.fast.jacobian_reuses,
                        "{label}"
                    );
                } else {
                    history_changed_a_restart |= !bitwise;
                }
            }
        }
    }
    assert!(history_changed_a_restart);
}

/// (f): a stop on the step that reaches `tf` changes nothing: the run is
/// complete, reported as a success without a partial record, bit for bit
/// the run without a control.
#[test]
fn a_stop_at_tf_is_a_success() {
    for setup in [Setup::brusselator(), Setup::hires()] {
        let case = &setup.case;
        let output = schedule(case.span, 10);
        let label = setup.label(1.0e-6);
        let adaptive = config(case, 1.0e-6);
        let reference = setup.run(case.span, &case.y0, &adaptive, &output, None);
        let (mut calls, mut stops) = (0, 0);
        let mut stop_at_tf = |view: &AcceptedStepView<'_>| {
            calls += 1;
            if view.t == case.span.1 {
                stops += 1;
                AcceptControl::Stop
            } else {
                AcceptControl::Continue
            }
        };
        let stopped = setup.run(
            case.span,
            &case.y0,
            &adaptive,
            &output,
            Some(&mut stop_at_tf),
        );
        assert_eq!(stops, 1, "{label}");
        assert_eq!(calls, reference.fast.accepted_steps, "{label}");
        assert!(stopped.fast.observed.success, "{label}");
        assert!(stopped.partial.is_none(), "{label}");
        assert_eq!(stopped.fast.observed.message, "success", "{label}");
        assert_same_run(&reference, &stopped, &label);
    }
}

/// A stop before `tf` on the attempt that spends the budget is the
/// client's: the reason is `ClientStop`, not `AttemptBudget`.
#[test]
fn a_stop_on_the_last_budgeted_attempt_is_a_client_stop() {
    let setup = Setup::brusselator();
    let case = &setup.case;
    let output = schedule(case.span, 10);
    let adaptive = config(case, 1.0e-6);
    let (_, seen) = recorded_run(&setup, &adaptive, &output);
    let k = 5;
    let budget = AdaptiveStepConfig {
        max_attempts: seen[k - 1].attempts,
        ..adaptive.clone()
    };
    let spent = setup.run(case.span, &case.y0, &budget, &output, None);
    let spent = spent.partial.unwrap();
    assert_eq!(spent.reason, DriverStopReason::AttemptBudget);
    assert_eq!(spent.h_next, None);
    let (stopped, _) = stopped_run(&setup, &budget, &output, k);
    let stopped = stopped.partial.unwrap();
    assert_eq!(stopped.reason, DriverStopReason::ClientStop);
    assert_eq!(bits(&stopped.y), bits(&spent.y));
    assert_eq!(stopped.h_next, Some(seen[k - 1].h_next));
}

/// The record serializes the new reason in kebab case and `h_next` only
/// when it is set, so the existing records serialize as before.
#[test]
fn the_client_stop_record_serializes() {
    let setup = Setup::brusselator();
    let case = &setup.case;
    let output = schedule(case.span, 10);
    let adaptive = config(case, 1.0e-6);
    let (stopped, _) = stopped_run(&setup, &adaptive, &output, 5);
    let partial = stopped.partial.unwrap();
    let value = serde_json::to_value(&partial).unwrap();
    assert_eq!(value["reason"], "client-stop");
    assert_eq!(value["h_next"], partial.h_next.unwrap());
    let budget = DriverPartial {
        reason: DriverStopReason::AttemptBudget,
        h_next: None,
        ..partial
    };
    let value = serde_json::to_value(&budget).unwrap();
    assert_eq!(value["reason"], "attempt-budget");
    assert!(value.get("h_next").is_none(), "{value}");
}

/// `y_i' = -k_i (y_i - 1) - y_i^3`, `k_i` from 1 to 1e4: the diagonal stiff
/// problem of `client_stage_system_contracts.rs`.
fn diagonal_problem() -> (OdeProblem, Vec<f64>, Vec<f64>) {
    let n = 8;
    let rates: Vec<f64> = (0..n)
        .map(|i| 10f64.powf(4.0 * i as f64 / (n - 1) as f64))
        .collect();
    let k = rates.clone();
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for ((o, y), k) in out.iter_mut().zip(y).zip(&k) {
            *o = -k * (y - 1.0) - y * y * y;
        }
        Ok(())
    });
    let k = rates.clone();
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        for i in 0..n {
            j[(i, i)] = -k[i] - 3.0 * y[i] * y[i];
        }
        Ok(j)
    });
    let problem = OdeProblem::new(
        "diagonal-cubic",
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let y0 = (0..n).map(|i| 3.0 - 0.25 * i as f64).collect();
    (problem, y0, rates)
}

/// The diagonal `W` of `client_stage_system_contracts.rs`: no allocation
/// after construction.
struct Diagonal {
    rates: Vec<f64>,
    jacobian: Vec<f64>,
    inverse: Vec<f64>,
}

impl Diagonal {
    fn new(rates: &[f64]) -> Self {
        Self {
            rates: rates.to_vec(),
            jacobian: vec![0.0; rates.len()],
            inverse: vec![0.0; rates.len()],
        }
    }
}

impl StageLinearSystem for Diagonal {
    fn dimension(&self) -> usize {
        self.rates.len()
    }

    fn linearize(
        &mut self,
        _problem: &OdeProblem,
        _t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        for ((j, k), y) in self.jacobian.iter_mut().zip(&self.rates).zip(y) {
            *j = -k - 3.0 * y * y;
        }
        Ok(())
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.direct_factorizations += 1;
        for (w, j) in self.inverse.iter_mut().zip(&self.jacobian) {
            *w = 1.0 / (inv_h_gamma - j);
        }
        Ok(())
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        counters.direct_solve_calls += 1;
        for (x, w) in b.iter_mut().zip(&self.inverse) {
            *x *= w;
        }
        Ok(())
    }
}

/// (d): with an allocation-free client and an allocation-free control, the
/// run allocates as much at two tolerances whose attempt counts differ at
/// least twice, and exactly as much as the run without a control.
#[test]
fn the_control_allocates_nothing() {
    let (problem, y0, rates) = diagonal_problem();
    let output = OutputSchedule::new(vec![0.0, 10.0]).unwrap();
    let adaptive = |rtol: f64| AdaptiveStepConfig {
        atol: 1.0e-4 * rtol,
        rtol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: 10.0,
        max_attempts: 1_000_000,
        ..AdaptiveStepConfig::default()
    };
    let controlled = |rtol: f64| {
        let adaptive = adaptive(rtol);
        let atol = vec![adaptive.atol; rates.len()];
        let client = Box::new(Diagonal::new(&rates));
        let mut calls = 0_usize;
        let ((fast, partial, _client), allocations) = allocations_during(|| {
            let mut count_calls = |_view: &AcceptedStepView<'_>| {
                calls += 1;
                AcceptControl::Continue
            };
            integrate_rodas5p_fast_client_observed_with_atol_and_control(
                &problem,
                client,
                (0.0, 10.0),
                &y0,
                &adaptive,
                &output,
                &atol,
                &mut count_calls,
            )
            .unwrap()
        });
        assert!(fast.observed.success && partial.is_none());
        assert_eq!(calls, fast.accepted_steps);
        (fast.attempts, allocations)
    };
    let plain = |rtol: f64| {
        let adaptive = adaptive(rtol);
        let atol = vec![adaptive.atol; rates.len()];
        let client = Box::new(Diagonal::new(&rates));
        let ((fast, _, _client), allocations) = allocations_during(|| {
            integrate_rodas5p_fast_client_observed_with_atol(
                &problem,
                client,
                (0.0, 10.0),
                &y0,
                &adaptive,
                &output,
                &atol,
            )
            .unwrap()
        });
        (fast.attempts, allocations)
    };
    controlled(1.0e-6); // warm the coefficient cache and any lazy statics
    let runs: Vec<_> = [1.0e-6, 1.0e-10].map(controlled).into();
    assert!(runs[1].0 >= 2 * runs[0].0, "{runs:?}");
    assert_eq!(
        runs[0].1, runs[1].1,
        "allocations depend on the step count: {runs:?}"
    );
    let without: Vec<_> = [1.0e-6, 1.0e-10].map(plain).into();
    assert_eq!(runs, without, "the control changed the run or allocated");
    eprintln!("control: {runs:?} (attempts, allocations)");
}
