//! Equal-observation serial microbenchmark of persistence versus restarting.
//!
//! Run with `cargo run -p vigilode --release --example production_session_bench -- 31`.
//! Optional arguments are timed repetitions and observation intervals (31, 64
//! by default). All methods land at the same observations; no dense-output comparison is
//! mixed in. This measures repeated observation delivery, not general solver
//! superiority. No other process or thread is launched by the benchmark.

use std::{hint::black_box, time::Instant};

use rodas5p_integrators::{
    AdaptiveStepConfig, Dopri5Config, OutputSchedule, integrate_dopri5_observed,
};
use serde_json::json;
use vigilode::{Atol, Dopri5, Options, Problem, SessionAdvance, SessionEpochs};

#[derive(Clone, Copy)]
enum Case {
    Decay1,
    Decay32,
    Oscillator2,
}

impl Case {
    fn name(self) -> &'static str {
        match self {
            Self::Decay1 => "scalar-decay",
            Self::Decay32 => "diagonal-decay-32",
            Self::Oscillator2 => "rotation-frequency-3",
        }
    }
    fn initial(self) -> Vec<f64> {
        match self {
            Self::Decay1 => vec![1.0],
            Self::Decay32 => vec![1.0; 32],
            Self::Oscillator2 => vec![1.0, 0.0],
        }
    }
    fn rhs(self, _: f64, y: &[f64], out: &mut [f64]) {
        match self {
            Self::Decay1 | Self::Decay32 => {
                for (i, (dy, &state)) in out.iter_mut().zip(y).enumerate() {
                    *dy = -(1.0 + i as f64 / 32.0) * state;
                }
            }
            Self::Oscillator2 => {
                out[0] = -3.0 * y[1];
                out[1] = 3.0 * y[0];
            }
        }
    }
    fn exact(self, t: f64, i: usize) -> f64 {
        match self {
            Self::Decay1 | Self::Decay32 => (-(1.0 + i as f64 / 32.0) * t).exp(),
            Self::Oscillator2 if i == 0 => (3.0 * t).cos(),
            Self::Oscillator2 => (3.0 * t).sin(),
        }
    }
}

#[derive(Clone, Copy)]
enum Mode {
    NativeOneShot,
    FacadeOneShot,
    Persistent,
    Restart,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::NativeOneShot => "native-one-shot",
            Self::FacadeOneShot => "facade-one-shot",
            Self::Persistent => "facade-persistent-observation-delivery",
            Self::Restart => "facade-restart-per-observation",
        }
    }
}

struct Run {
    y: Vec<Vec<f64>>,
    rhs_calls: u64,
    attempts: usize,
    accepted: usize,
    rejected: usize,
    fsal_reuses: u64,
}

fn execute(case: Case, mode: Mode, times: &[f64], opts: &Options) -> Run {
    let initial = case.initial();
    let span = (times[0], *times.last().unwrap());
    if matches!(mode, Mode::NativeOneShot) {
        let callback = |t, y: &[f64], out: &mut [f64]| {
            case.rhs(t, y, out);
            Ok(())
        };
        let config = Dopri5Config {
            adaptive: AdaptiveStepConfig {
                atol: 1e-12,
                rtol: opts.rtol,
                initial_step: opts.h0,
                max_attempts: opts.max_steps,
                ..AdaptiveStepConfig::default()
            },
            atol: None,
            fsal: true,
            recover_nonfinite_rhs: true,
        };
        let result = integrate_dopri5_observed(
            &callback,
            span,
            &initial,
            &config,
            &OutputSchedule::new(times.to_vec()).unwrap(),
        )
        .unwrap();
        assert!(result.observed.success, "native integration failed");
        assert_eq!(result.observed.t, times);
        return Run {
            y: result.observed.y,
            rhs_calls: result.observed.counters.rhs_calls,
            attempts: result.attempts,
            accepted: result.accepted_steps,
            rejected: result.rejected_steps,
            fsal_reuses: result.fsal_reuses,
        };
    }
    let problem = Problem::new(initial.len(), move |t, y, out| {
        case.rhs(t, y, out);
        Ok(())
    });
    if matches!(mode, Mode::Restart) {
        let mut state = initial.clone();
        let mut run = Run {
            y: vec![initial],
            rhs_calls: 0,
            attempts: 0,
            accepted: 0,
            rejected: 0,
            fsal_reuses: 0,
        };
        for pair in times.windows(2) {
            let result = Dopri5::solve_observed(&problem, (pair[0], pair[1]), &state, opts, pair)
                .unwrap()
                .ensure_success()
                .unwrap();
            run.rhs_calls += result.stats.counters.rhs_calls;
            run.attempts += result.stats.attempts;
            run.accepted += result.stats.accepted_steps;
            run.rejected += result.stats.rejected_steps;
            run.fsal_reuses += result.stats.fsal_reuses;
            state = result.final_state;
            run.y.push(state.clone());
        }
        return run;
    }
    let result = if matches!(mode, Mode::Persistent) {
        Dopri5::with_session(
            &problem,
            span,
            &initial,
            opts,
            times,
            SessionEpochs::default(),
            |session| {
                for &target in &times[1..] {
                    let outcome = session
                        .advance_to(target, usize::MAX, SessionEpochs::default())
                        .unwrap();
                    assert!(matches!(
                        outcome,
                        SessionAdvance::Checkpoint | SessionAdvance::Complete
                    ));
                    black_box(session.checkpoint().y);
                }
            },
        )
        .unwrap()
        .1
    } else {
        Dopri5::solve_observed(&problem, span, &initial, opts, times).unwrap()
    }
    .ensure_success()
    .unwrap();
    assert_eq!(result.t, times);
    Run {
        y: result.y,
        rhs_calls: result.stats.counters.rhs_calls,
        attempts: result.stats.attempts,
        accepted: result.stats.accepted_steps,
        rejected: result.stats.rejected_steps,
        fsal_reuses: result.stats.fsal_reuses,
    }
}

fn main() {
    let repetitions = std::env::args()
        .nth(1)
        .map(|s| s.parse::<usize>().expect("positive repetition count"))
        .unwrap_or(31);
    assert!(repetitions > 0);
    let intervals = std::env::args()
        .nth(2)
        .map(|s| {
            s.parse::<usize>()
                .expect("positive observation interval count")
        })
        .unwrap_or(64);
    assert!(intervals > 0);
    let opts = Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-12),
        h0: 0.001,
        ..Options::default()
    };
    let times: Vec<f64> = (0..=intervals)
        .map(|i| 4.0 * (i as f64 / intervals as f64))
        .collect();
    let mut rows = Vec::new();
    for case in [Case::Decay1, Case::Decay32, Case::Oscillator2] {
        let reference = execute(case, Mode::FacadeOneShot, &times, &opts);
        let modes = [
            Mode::NativeOneShot,
            Mode::FacadeOneShot,
            Mode::Persistent,
            Mode::Restart,
        ];
        let mut runs = Vec::new();
        let mut errors = Vec::new();
        let mut parities = Vec::new();
        let mut solves_per_sample = Vec::new();
        for mode in modes {
            let result = execute(case, mode, &times, &opts);
            let max_error = times
                .iter()
                .zip(&result.y)
                .flat_map(|(&t, y)| {
                    y.iter()
                        .enumerate()
                        .map(move |(i, value)| (value - case.exact(t, i)).abs())
                })
                .fold(0.0_f64, f64::max);
            assert!(
                max_error < 1e-7,
                "accuracy failure: {} {}",
                case.name(),
                mode.name()
            );
            let identical = result.y == reference.y
                && result.rhs_calls == reference.rhs_calls
                && result.attempts == reference.attempts
                && result.fsal_reuses == reference.fsal_reuses;
            if !matches!(mode, Mode::Restart) {
                assert!(identical, "persistent/native step path differs");
            }
            // Calibrate at least 3 ms of work, then target 5 ms per sample.
            // Calibration is excluded from measured samples and each mode
            // receives its own batch size; per-solve time is normalized below.
            black_box(execute(case, mode, &times, &opts));
            let calibration = Instant::now();
            let mut calibration_count = 0_usize;
            while calibration.elapsed().as_nanos() < 3_000_000 {
                black_box(execute(case, mode, &times, &opts));
                calibration_count += 1;
            }
            let count = ((calibration_count as f64 * 5_000_000.0
                / calibration.elapsed().as_nanos() as f64)
                .ceil() as usize)
                .max(1);
            solves_per_sample.push(count);
            runs.push(result);
            errors.push(max_error);
            parities.push(identical);
        }
        let mut elapsed_batches: Vec<Vec<u128>> = (0..modes.len())
            .map(|_| Vec::with_capacity(repetitions))
            .collect();
        // Rotate mode order every sample to avoid assigning all early/late
        // thermal or scheduler conditions to one implementation.
        for sample in 0..repetitions {
            for offset in 0..modes.len() {
                let index = (sample + offset) % modes.len();
                let start = Instant::now();
                for _ in 0..solves_per_sample[index] {
                    black_box(execute(case, modes[index], &times, &opts));
                }
                elapsed_batches[index].push(start.elapsed().as_nanos());
            }
        }
        for (index, mode) in modes.iter().enumerate() {
            let result = &runs[index];
            let elapsed: Vec<f64> = elapsed_batches[index]
                .iter()
                .map(|&ns| ns as f64 / solves_per_sample[index] as f64)
                .collect();
            let mut ordered = elapsed.clone();
            ordered.sort_by(f64::total_cmp);
            rows.push(json!({
                "case": case.name(), "dimension": case.initial().len(), "method": mode.name(),
                "elapsed_ns_median": ordered[repetitions / 2], "elapsed_ns_min": ordered[0],
                "elapsed_ns_max": ordered[repetitions - 1], "elapsed_ns_samples": elapsed,
                "solves_per_sample": solves_per_sample[index], "batch_elapsed_ns_samples": elapsed_batches[index],
                "rhs_calls": result.rhs_calls, "attempts": result.attempts,
                "accepted_steps": result.accepted, "rejected_steps": result.rejected,
                "fsal_reuses": result.fsal_reuses, "max_absolute_observation_error": errors[index],
                "identical_output_and_work_to_facade_one_shot": parities[index],
            }));
        }
    }
    println!("{}", serde_json::to_string_pretty(&json!({
        "schema": "vigilode.production-session-benchmark.v1",
        "benchmark_kind": "serial-equal-observation-persistence-microbenchmark",
        "repetitions": repetitions, "observations": times, "options": opts,
        "accuracy_requirement_max_abs": 1e-7, "solver_threads": 1,
        "timing_protocol": { "calibration_minimum_ns": 3_000_000,
            "target_batch_ns": 5_000_000, "mode_order": "cyclic rotation per sample",
            "reported_elapsed_ns": "per-solve batch normalization" },
        "limitations": ["Timing is host-specific; independent broad work-precision comparisons are still required.",
            "Restart mode repeats construction and resets h0/controller/FSAL at each observation.",
            "Native/facade/session paths are required to produce identical observation states and work."],
        "results": rows,
    })).unwrap());
}
