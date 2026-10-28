//! Single-run correctness/work/timing record for opt-in entry-pilot screening.
//! Run modes separately in a quiet, pinned campaign; compilation is excluded.
use serde_json::json;
use std::time::Instant;
use vigilode::{
    Atol, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig, SwitchingSolver,
};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let arg = |key: &str, default: &str| {
        args.windows(2)
            .find(|a| a[0] == key)
            .map_or(default.to_owned(), |a| a[1].clone())
    };
    let case = arg("--case", "rapid");
    let mode = arg("--mode", "screen-jvp");
    let rtol: f64 = arg("--rtol", "1e-7").parse().unwrap();
    let source = arg("--source-commit", "UNRECORDED");
    let (n, end, y0) = match case.as_str() {
        "rapid" => (2, 4.0, vec![1.0, 0.0]),
        "decay" => (1, 4.0, vec![1.0]),
        "prothero" => (1, 1.0, vec![1.0]),
        "transition" => (1, 1.0, vec![2.0]),
        _ => panic!("unknown case"),
    };
    assert!(["legacy-jvp", "legacy-fd", "screen-jvp", "screen-fd", "rhs"].contains(&mode.as_str()));
    let work_start = Instant::now();
    let rhs_case = case.clone();
    let mut problem = Problem::new(n, move |t, y, out| {
        match rhs_case.as_str() {
            "rapid" => {
                out[0] = -200.0 * y[1];
                out[1] = 200.0 * y[0];
            }
            "decay" => out[0] = -y[0],
            "prothero" => out[0] = -1e5 * (y[0] - t.cos()) - t.sin(),
            "transition" => out[0] = -rate(t) * (y[0] - t.cos()) - t.sin(),
            _ => unreachable!(),
        };
        Ok(())
    });
    if mode.ends_with("jvp") {
        let jac_case = case.clone();
        problem = problem.jacobian(Jacobian::jvp(move |t, _, v, out| {
            match jac_case.as_str() {
                "rapid" => {
                    out[0] = -200.0 * v[1];
                    out[1] = 200.0 * v[0];
                }
                "decay" => out[0] = -v[0],
                "prothero" => out[0] = -1e5 * v[0],
                "transition" => out[0] = -rate(t) * v[0],
                _ => unreachable!(),
            };
            Ok(())
        }));
    }
    if case == "prothero" || case == "transition" {
        let ft_case = case.clone();
        problem = problem.time_derivative(move |t, y, out| {
            out[0] = if ft_case == "prothero" {
                -1e5 * t.sin() - t.cos()
            } else {
                let a = (40.0 * (t - 0.3)).tanh();
                let b = (40.0 * (t - 0.7)).tanh();
                -4999.5 * 40.0 * (b * b - a * a) * (y[0] - t.cos()) - rate(t) * t.sin() - t.cos()
            };
            Ok(())
        });
    } else {
        problem = problem.autonomous();
    }
    let options = Options {
        rtol,
        atol: Atol::Scalar(rtol * 1e-3),
        h0: 1e-4,
        max_steps: 25_000,
        ..Default::default()
    };
    let config = SwitchingConfig {
        fallback: if mode == "rhs" {
            DerivativeFallback::Forbidden
        } else if mode.ends_with("fd") {
            DerivativeFallback::AllowFiniteDifference
        } else {
            DerivativeFallback::ProvidedOnly
        },
        window_count: 16,
        implicit_pilot_attempt_budget: if mode.starts_with("screen") { 16 } else { 0 },
        implicit_pilot_limit: 1,
        ..Default::default()
    };
    let result = SwitchingSolver::solve(&problem, (0.0, end), &y0, &options, &config).unwrap();
    let seconds = work_start.elapsed().as_secs_f64();
    let requested: Vec<_> = (0..=16).map(|i| end * i as f64 / 16.0).collect();
    let mut sampled_error = 0.0_f64;
    let mut all_error = 0.0_f64;
    let mut phase = 0.0_f64;
    let mut amplitude = 0.0_f64;
    for (t, y) in result.t.iter().zip(&result.y) {
        let reference = exact(&case, *t);
        let error = y
            .iter()
            .zip(&reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);
        all_error = all_error.max(error);
        if requested.contains(t) {
            sampled_error = sampled_error.max(error);
        }
        if case == "rapid" {
            phase = phase.max(
                (y[1] * reference[0] - y[0] * reference[1])
                    .atan2(y[0] * reference[0] + y[1] * reference[1])
                    .abs(),
            );
            amplitude = amplitude.max((y[0].hypot(y[1]) - 1.0).abs());
        }
    }
    let common_covered = requested.iter().all(|t| result.t.contains(t));
    println!(
        "{}",
        json!({"schema":"vigilode.frontier-switch-benchmark.v1","source_commit":source,
            "case":case,"mode":mode,"rtol":rtol,"elapsed_seconds":seconds,
            "success":result.success,"common_observations_covered":common_covered,
            "common_observation_max_abs_error":sampled_error,"all_output_max_abs_error":all_error,
            "phase_error_radians":if case=="rapid"{Some(phase)}else{None},
            "amplitude_error":if case=="rapid"{Some(amplitude)}else{None},
            "requested_times":requested,"solution":result,
            "timing_scope":"problem/config/state/workspaces/all trials; excludes reference and serialization",
            "comparison_limit":"local error tolerances do not certify equal global error; use common-observation achieved error; work score is not wall time"
        })
    );
}
fn rate(t: f64) -> f64 {
    1.0 + 4999.5 * ((40.0 * (t - 0.3)).tanh() - (40.0 * (t - 0.7)).tanh())
}
fn exact(case: &str, t: f64) -> Vec<f64> {
    match case {
        "rapid" => vec![(200.0 * t).cos(), (200.0 * t).sin()],
        "decay" => vec![(-t).exp()],
        "prothero" => vec![t.cos()],
        "transition" => {
            fn log_cosh(x: f64) -> f64 {
                x.abs() + (-2.0 * x.abs()).exp().ln_1p() - 2.0_f64.ln()
            }
            let primitive = t + 4999.5 / 40.0
                * (log_cosh(40.0 * (t - 0.3)) - log_cosh(40.0 * (t - 0.7)) - log_cosh(-12.0)
                    + log_cosh(-28.0));
            vec![t.cos() + (-primitive).exp()]
        }
        _ => unreachable!(),
    }
}
