//! Research node CT01 (`research/ct01_controller_grid_holdout_20261010`):
//! the opt-in controller telemetry of the dense fast driver
//! (`integrate_rodas5p_fast_observed_with_telemetry`) is a pure observer.
//!
//! - `telemetry_on_and_off_runs_are_bitwise_identical`: for all three
//!   controller kinds of CT01 (`Integral`, `PredictiveCapped`,
//!   `PredictiveCapped2`), the four CT01 problems, the CT01 interior grid
//!   and the endpoint-only schedule, and several tolerances and initial
//!   steps, every output time and state (bits), every counter, the attempt,
//!   acceptance, rejection, reuse and clipping counts, the message, the LU
//!   kind and the driver id are the same with the telemetry off, on, and on
//!   with a decision trace.
//! - `traced_decisions_replay_through_the_controller`: every recorded
//!   decision is the controller's own: replaying the traced inputs through a
//!   fresh `AdaptiveControllerState` gives every next step bit for bit, and
//!   under `PredictiveCapped2` the observer's pending flag equals the
//!   controller's `rejection_pending()` after every update.

#[path = "../src/stiff_benchmark.rs"]
#[allow(dead_code, unused_imports)]
mod stiff_benchmark;

use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, ControllerTelemetry,
    OutputSchedule, Rodas5pFastResult, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_observed_with_telemetry, rodas_next_step_after_attempt,
};
use stiff_benchmark::{BenchmarkProblem, benchmark_problems};

const PROBLEMS: [&str; 4] = [
    "van-der-pol-mu1000",
    "hires",
    "robertson",
    "brusselator-1d-50",
];
const KINDS: [ControllerKind; 3] = [
    ControllerKind::Integral,
    ControllerKind::PredictiveCapped,
    ControllerKind::PredictiveCapped2,
];
/// (rtol, initial step) pairs; the steps are CT01 seeds.
const SETTINGS: [(f64, f64); 2] = [(1.0e-4, 7.0e-5), (1.0e-6, 2.0e-6)];

fn problems() -> Vec<BenchmarkProblem> {
    let mut all = benchmark_problems().unwrap();
    PROBLEMS
        .iter()
        .map(|id| {
            let at = all.iter().position(|p| p.id == *id).unwrap();
            all.swap_remove(at)
        })
        .collect()
}

fn config(p: &BenchmarkProblem, kind: ControllerKind, rtol: f64, h0: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * p.atol_scale,
        rtol,
        initial_step: h0,
        min_step: 1.0e-14,
        max_step: p.t_span.1 - p.t_span.0,
        max_attempts: 1_000_000,
        controller: kind,
        ..AdaptiveStepConfig::default()
    }
}

/// The CT01 grid (t0 and the 40 interior points) and the endpoint schedule.
fn schedules(p: &BenchmarkProblem) -> [OutputSchedule; 2] {
    let (t0, tf) = p.t_span;
    let mut grid = vec![t0];
    grid.extend((1..=40).map(|k| t0 + (k as f64) * (tf - t0) / 40.0));
    [
        OutputSchedule::new(grid).unwrap(),
        OutputSchedule::new(vec![t0, tf]).unwrap(),
    ]
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Everything a run reports, with floats as bits.
fn fingerprint(r: &Rodas5pFastResult) -> String {
    let o = &r.observed;
    format!(
        "t={:?} y={:?} success={} message={:?} counters={:?} internal={} clipped={} \
         attempts={} accepted={} rejected={} reuses={} lu={:?} driver={}",
        bits(&o.t),
        o.y.iter().map(|y| bits(y)).collect::<Vec<_>>(),
        o.success,
        o.message,
        o.counters,
        o.internal_steps,
        o.output_clipped_steps,
        r.attempts,
        r.accepted_steps,
        r.rejected_steps,
        r.jacobian_reuses,
        r.lu,
        r.driver
    )
}

#[test]
fn telemetry_on_and_off_runs_are_bitwise_identical() {
    let mut runs = 0;
    let mut clipped_runs = 0;
    for p in problems() {
        for schedule in schedules(&p) {
            for kind in KINDS {
                for (rtol, h0) in SETTINGS {
                    let c = config(&p, kind, rtol, h0);
                    let off =
                        integrate_rodas5p_fast_observed(&p.problem, p.t_span, &p.y0, &c, &schedule)
                            .unwrap();
                    let (on, counts) = integrate_rodas5p_fast_observed_with_telemetry(
                        &p.problem,
                        p.t_span,
                        &p.y0,
                        &c,
                        &schedule,
                        ControllerTelemetry::default(),
                    )
                    .unwrap();
                    let (traced, with_trace) = integrate_rodas5p_fast_observed_with_telemetry(
                        &p.problem,
                        p.t_span,
                        &p.y0,
                        &c,
                        &schedule,
                        ControllerTelemetry::with_trace(),
                    )
                    .unwrap();
                    let reference = fingerprint(&off);
                    assert_eq!(fingerprint(&on), reference, "{} {kind:?} {rtol:e}", p.id);
                    assert_eq!(
                        fingerprint(&traced),
                        reference,
                        "{} {kind:?} {rtol:e}",
                        p.id
                    );
                    assert!(off.observed.success, "{} {kind:?} {rtol:e}", p.id);
                    // The trace changes nothing in the counts either.
                    assert_eq!(
                        ControllerTelemetry {
                            trace: None,
                            ..with_trace.clone()
                        },
                        counts
                    );
                    assert_eq!(with_trace.trace.as_ref().unwrap().len(), off.attempts);
                    assert_eq!(counts.updates, off.attempts);
                    assert_eq!(counts.clipped_landings, off.observed.output_clipped_steps);
                    runs += 1;
                    clipped_runs += usize::from(counts.clipped_landings > 0);
                }
            }
        }
    }
    assert_eq!(runs, 4 * 2 * 3 * SETTINGS.len());
    // The grid schedule clips (the endpoint schedule never does).
    assert!(clipped_runs >= 4 * 3 * SETTINGS.len());
}

#[test]
fn traced_decisions_replay_through_the_controller() {
    for p in problems() {
        let [grid, _] = schedules(&p);
        for kind in KINDS {
            for (rtol, h0) in SETTINGS {
                let c = config(&p, kind, rtol, h0);
                let (result, telemetry) = integrate_rodas5p_fast_observed_with_telemetry(
                    &p.problem,
                    p.t_span,
                    &p.y0,
                    &c,
                    &grid,
                    ControllerTelemetry::with_trace(),
                )
                .unwrap();
                let trace = telemetry.trace.clone().unwrap();
                assert_eq!(trace.len(), result.attempts);
                assert_eq!(
                    trace.iter().filter(|d| d.accepted).count(),
                    result.accepted_steps
                );
                let mut controller = AdaptiveControllerState::default();
                let mut observer = ControllerTelemetry::default();
                for (i, d) in trace.iter().enumerate() {
                    let next = rodas_next_step_after_attempt(
                        &mut controller,
                        &c,
                        d.requested_h,
                        d.trial_h,
                        d.error,
                        d.accepted,
                        d.clipped,
                    )
                    .unwrap();
                    assert_eq!(next.to_bits(), d.next_h.to_bits(), "{} {kind:?} #{i}", p.id);
                    observer.observe(*d);
                    if kind == ControllerKind::PredictiveCapped2 {
                        assert_eq!(
                            observer.rejection_pending,
                            controller.rejection_pending(),
                            "{} #{i}",
                            p.id
                        );
                    } else {
                        assert!(!controller.rejection_pending());
                    }
                }
                assert_eq!(
                    observer,
                    ControllerTelemetry {
                        trace: None,
                        ..telemetry
                    }
                );
            }
        }
    }
}
