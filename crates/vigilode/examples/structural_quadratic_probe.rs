//! Full-IVP comparison of an explicit quadratic proposal and ordinary GMRES.
//! Both arms use the same native matrix-free driver, starts, tolerances and
//! seventeen requested output times. Model setup, error analysis and JSON
//! serialization are outside the solver timer; workspace setup is inside.
use rodas5p_core::{CoreResult, InitialGuess, LinearSolverConfig};
use rodas5p_integrators::{
    AdaptiveStepConfig, KrylovWorkspaceOptions, OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace,
    RoutingOptions,
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;

#[derive(Clone, Default)]
struct Calls {
    rhs: Arc<AtomicU64>,
    jvp: Arc<AtomicU64>,
}

struct Model {
    problem: OdeProblem,
    y0: Vec<f64>,
    coefficients: (f64, f64),
    calls: Calls,
    damping: f64,
    time_dependent: bool,
}

fn model(case: &str) -> CoreResult<Model> {
    let n = if case == "rotation-large" { 4096 } else { 16 };
    let damping = if case == "damped-rotation" { 0.5 } else { 0.0 };
    let time_dependent = case == "time-jvp";
    let calls = Calls::default();
    let c = calls.clone();
    let j = calls.clone();
    let problem = OdeProblem::new(
        case,
        n,
        Arc::new(move |t, y, out| {
            c.rhs.fetch_add(1, Ordering::Relaxed);
            if time_dependent {
                for i in 0..n {
                    let target = (-t).exp() * (1.0 + i as f64 / 16.0);
                    out[i] = -(2.0 + i as f64 / 8.0) * (y[i] - target) - target;
                }
            } else {
                for k in 0..n / 2 {
                    out[2 * k] = -damping * y[2 * k] - 3.0 * y[2 * k + 1];
                    out[2 * k + 1] = 3.0 * y[2 * k] - damping * y[2 * k + 1];
                }
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_t, _y, v, out| {
            j.jvp.fetch_add(1, Ordering::Relaxed);
            if time_dependent {
                for i in 0..n {
                    out[i] = -(2.0 + i as f64 / 8.0) * v[i];
                }
            } else {
                for k in 0..n / 2 {
                    out[2 * k] = -damping * v[2 * k] - 3.0 * v[2 * k + 1];
                    out[2 * k + 1] = 3.0 * v[2 * k] - damping * v[2 * k + 1];
                }
            }
            Ok(())
        })),
        None, // Preserve the original nonautonomous finite-difference f_t path.
        !time_dependent,
        None,
        None,
    )?;
    let y0 = (0..n)
        .map(|i| {
            if time_dependent {
                1.0 + i as f64 / 16.0
            } else if i % 2 == 0 {
                // Repeat the original n=16 model's amplitudes, also at n=4096.
                1.0 + (i % 16) as f64 / 32.0
            } else {
                0.0
            }
        })
        .collect();
    let coefficients = if time_dependent {
        (-47.0 / 8.0, -31.0 / 4.0)
    } else if case == "badcoeff" {
        (0.0, 9.0)
    } else {
        (-2.0 * damping, -(damping * damping + 9.0))
    };
    Ok(Model {
        problem,
        y0,
        coefficients,
        calls,
        damping,
        time_dependent,
    })
}

fn exact(model: &Model, t: f64, i: usize) -> f64 {
    if model.time_dependent {
        (-t).exp() * model.y0[i]
    } else {
        let (s, c) = (3.0 * t).sin_cos();
        let amplitude = model.y0[(i / 2) * 2];
        (-model.damping * t).exp() * amplitude * if i % 2 == 0 { c } else { s }
    }
}

fn one(
    case: &str,
    arm: &str,
    start: &str,
    mode: &str,
    span: f64,
    output: &OutputSchedule,
) -> Result<Value, String> {
    let model = model(case).map_err(|e| e.to_string())?;
    let linear = LinearSolverConfig {
        rtol: 1.0e-10,
        atol: 1.0e-14,
        x0_strategy: if start == "previous" {
            InitialGuess::Previous
        } else {
            InitialGuess::Zero
        },
        ..RoutingOptions::default().matrix_free
    };
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-10,
        rtol: 1.0e-7,
        initial_step: 1.0e-3,
        ..AdaptiveStepConfig::default()
    };
    let begin = Instant::now();
    let mut workspace =
        Rodas5pMfFastWorkspace::new(&model.problem, &linear).map_err(|e| e.to_string())?;
    workspace
        .set_krylov_workspace(KrylovWorkspaceOptions::ls_workspace())
        .map_err(|e| e.to_string())?;
    workspace.set_solve_log(mode == "probe");
    if arm == "proposal" {
        workspace
            .set_directional_quadratic_proposal(Some(model.coefficients))
            .map_err(|e| e.to_string())?;
    }
    let (run, _, _, log) = workspace
        .integrate_observed_with_observer(
            &model.problem,
            (0.0, span),
            &model.y0,
            &adaptive,
            output,
            &mut |_, _, _, _, _| {},
        )
        .map_err(|e| e.to_string())?;
    let elapsed_ns = begin.elapsed().as_nanos();

    let observed = &run.observed;
    let mut max_abs_error = 0.0_f64;
    let mut scaled_sum = 0.0_f64;
    let mut component_count = 0usize;
    let mut finite = true;
    for (&t, row) in observed.t.iter().zip(&observed.y) {
        finite &= t.is_finite();
        for (i, &v) in row.iter().enumerate() {
            let target = exact(&model, t, i);
            let error = (v - target).abs();
            finite &= v.is_finite() && target.is_finite() && error.is_finite();
            max_abs_error = max_abs_error.max(error);
            let scaled = error / (adaptive.atol + adaptive.rtol * target.abs().max(v.abs()));
            scaled_sum += scaled * scaled;
            component_count += 1;
        }
    }
    let max_logged_relative_residual = log.as_ref().map(|entries| {
        entries
            .iter()
            .map(|entry| f64::from_bits(entry.relative_residual_bits))
            .fold(0.0_f64, f64::max)
    });
    let w = &observed.counters;
    // Same existing integer objective as the strict switcher, all attempts.
    let strict_work_units = w.rhs_calls as u128
        + w.jvp_vectors as u128
        + w.ft_calls as u128
        + w.linear_solves as u128
        + w.jacobian_builds as u128 * model.y0.len() as u128;
    let mut sample = json!({
        "success": observed.success,
        "message": observed.message,
        "finite": finite,
        "gridpoint_count": observed.t.len(),
        "max_abs_error": max_abs_error,
        "scaled_rms_error": (scaled_sum / component_count.max(1) as f64).sqrt(),
        "callback_counts": {
            "rhs": model.calls.rhs.load(Ordering::Relaxed),
            "jvp": model.calls.jvp.load(Ordering::Relaxed)
        },
        "work": w,
        "strict_work_units": strict_work_units,
        "attempts": run.attempts,
        "accepted_steps": run.accepted_steps,
        "rejected_steps": run.rejected_steps,
        "proposal_statistics": workspace.directional_quadratic_statistics(),
        "proposal_workspace_bytes": workspace.directional_quadratic_workspace_bytes(),
        "krylov_workspace_bytes": workspace.krylov_workspace_bytes(),
        "gmres_cycles": workspace.gmres_into_cycles(),
        "max_logged_relative_residual": max_logged_relative_residual,
        "elapsed_ns": elapsed_ns
    });
    if mode == "probe" {
        sample["t"] = json!(observed.t);
        sample["y"] = json!(observed.y);
        sample["t_bits"] = json!(
            observed
                .t
                .iter()
                .map(|v| format!("{:016x}", v.to_bits()))
                .collect::<Vec<_>>()
        );
        sample["y_bits"] = json!(
            observed
                .y
                .iter()
                .map(|row| row
                    .iter()
                    .map(|v| format!("{:016x}", v.to_bits()))
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        sample["solve_log"] = json!(log);
    }
    let drop_begin = Instant::now();
    drop(run);
    drop(workspace);
    drop(log);
    let drop_ns = drop_begin.elapsed().as_nanos();
    sample["drop_ns"] = json!(drop_ns);
    sample["solve_plus_drop_ns"] = json!(elapsed_ns + drop_ns);
    Ok(sample)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("structural_quadratic_probe: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help") {
        println!(
            "structural_quadratic_probe --case rotation|rotation-large|damped-rotation|time-jvp|badcoeff --arm gmres|proposal --start zero|previous --mode probe|timing --repeats N [--span T]"
        );
        return Ok(());
    }
    let flags = [
        "--case",
        "--arm",
        "--start",
        "--mode",
        "--repeats",
        "--span",
    ];
    if (args.len() - 1) % 2 != 0 {
        return Err("each option requires a value".into());
    }
    let mut seen = std::collections::HashSet::new();
    for pair in args[1..].chunks_exact(2) {
        if !flags.contains(&pair[0].as_str()) || !seen.insert(pair[0].as_str()) {
            return Err(format!("unknown or duplicate option {}", pair[0]));
        }
    }
    let get = |key: &str, default: &str| {
        args.windows(2)
            .find(|w| w[0] == key)
            .map_or_else(|| default.to_owned(), |w| w[1].clone())
    };
    let case = get("--case", "rotation");
    let arm = get("--arm", "gmres");
    let start = get("--start", "previous");
    let mode = get("--mode", "probe");
    let repeats: usize = get("--repeats", "1").parse().map_err(|_| "bad repeats")?;
    let span: f64 = get("--span", "0.25").parse().map_err(|_| "bad span")?;
    if ![
        "rotation",
        "rotation-large",
        "damped-rotation",
        "time-jvp",
        "badcoeff",
    ]
    .contains(&case.as_str())
        || !["gmres", "proposal"].contains(&arm.as_str())
        || !["zero", "previous"].contains(&start.as_str())
        || !["probe", "timing"].contains(&mode.as_str())
        || repeats == 0
        || !span.is_finite()
        || span <= 0.0
    {
        return Err("invalid case, arm, start, mode, repeats or span".into());
    }
    let output_grid: Vec<f64> = (0..=16).map(|i| span * i as f64 / 16.0).collect();
    let output = OutputSchedule::new(output_grid.clone()).map_err(|e| e.to_string())?;
    let mut samples = Vec::with_capacity(repeats);
    for _ in 0..repeats {
        samples.push(one(&case, &arm, &start, &mode, span, &output)?);
    }
    let parameters = model(&case).map_err(|e| e.to_string())?;
    println!(
        "{}",
        json!({
            "schema": "directional-quadratic-full-ivp-v1",
            "case": case, "dimension": parameters.y0.len(), "arm": arm,
            "start": start, "mode": mode, "repeats": repeats, "span": span,
            "coefficients": {"a":parameters.coefficients.0,"b":parameters.coefficients.1},
            "output_grid": output_grid,
            "tolerances": {"rtol":1.0e-7,"atol":1.0e-10,"linear_rtol":1.0e-10,"linear_atol":1.0e-14,"h0":1.0e-3},
            "physical_error_cap": 1.0e-5,
            "solve_log_enabled": mode == "probe",
            "counter_scope": "dot/axpy counters describe existing instrumented kernels, not total FLOPs; quadratic candidate/residual loops and norms also perform O(n) work, included in elapsed time; proposal operator_actions count all attempted callback products",
            "timer_scope": "workspace construction plus integration; separate owned result/workspace drop; no model setup, audit or JSON",
            "samples": samples
        })
    );
    Ok(())
}
