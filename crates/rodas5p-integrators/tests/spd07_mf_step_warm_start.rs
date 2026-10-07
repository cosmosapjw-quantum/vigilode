//! Research node `research/spd07_mf_step_warm_start_20261007` (speed
//! research node SPD07): stage-indexed warm start of the matrix-free U-form
//! driver's Krylov stage solves from the previous accepted step.
//! `export_base` was run on the registration commit's solver source before
//! any source change.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{adaptive, brusselator, cases, linear_config, write_output};
use rodas5p_core::{InitialGuess, LinearMethod, LinearSolverConfig};
use rodas5p_integrators::{
    GcrodrRecyclePolicy, OdeProblem, OutputSchedule, Rodas5pMfFastResult,
    integrate_rodas5p_fast_observed, integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_traced,
    integrate_rodas5p_mf_fast_observed_with_gcrodr_policy, sequential_matrix_free_step,
};
use serde_json::{Value, json};

struct Run {
    id: &'static str,
    /// JVP-only clone.
    problem: OdeProblem,
    /// With an explicit Jacobian, for the dense reference run.
    full: Option<OdeProblem>,
    exact: Option<Box<dyn Fn(f64) -> Vec<f64>>>,
    y0: Vec<f64>,
    t_span: (f64, f64),
    atol_scale: f64,
}

/// The SAFE-RECYCLE problem set.
fn runs() -> Vec<Run> {
    let mut out: Vec<Run> = cases()
        .into_iter()
        .map(|c| Run {
            id: c.id,
            problem: c.problem,
            full: c.full,
            exact: c.exact,
            y0: c.y0,
            t_span: c.t_span,
            atol_scale: c.atol_scale,
        })
        .collect();
    let (p, y0) = brusselator(160).unwrap();
    out.push(Run {
        id: "brusselator-1d-160",
        problem: p.jvp_only_clone().unwrap(),
        full: Some(p),
        exact: None,
        y0,
        t_span: (0.0, 10.0),
        atol_scale: 1.0,
    });
    out
}

const RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];
const LINEAR_RTOL: f64 = 1.0e-10;
const REFERENCE_RTOL: f64 = 1.0e-12;

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| format!("{:016x}", x.to_bits())).collect()
}

fn result_json(result: &rodas5p_core::CoreResult<Rodas5pMfFastResult>) -> Value {
    match result {
        Ok(r) => json!({
            "ok": true,
            "success": r.observed.success,
            "message": r.observed.message,
            "t": hexes(&r.observed.t),
            "y_last": hexes(r.observed.y.last().unwrap()),
            "attempts": r.attempts,
            "accepted": r.accepted_steps,
            "rejected": r.rejected_steps,
            "state_reuses": r.state_reuses,
            "internal_steps": r.observed.internal_steps,
            "output_clipped_steps": r.observed.output_clipped_steps,
            "counters": serde_json::to_value(r.observed.counters).unwrap(),
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

fn config(method: LinearMethod, x0: InitialGuess) -> LinearSolverConfig {
    LinearSolverConfig {
        x0_strategy: x0,
        ..linear_config(method, LINEAR_RTOL)
    }
}

fn gmres_into(run: &Run, rtol: f64, x0: InitialGuess) -> Value {
    let span = run.t_span.1 - run.t_span.0;
    let schedule = OutputSchedule::new(vec![run.t_span.0, run.t_span.1]).unwrap();
    result_json(&integrate_rodas5p_mf_fast_observed_gmres_into(
        &run.problem,
        run.t_span,
        &run.y0,
        &config(LinearMethod::Gmres, x0),
        &adaptive(rtol, run.atol_scale, span),
        &schedule,
    ))
}

fn gcrodr_cold(run: &Run, rtol: f64, x0: InitialGuess) -> Value {
    let span = run.t_span.1 - run.t_span.0;
    let schedule = OutputSchedule::new(vec![run.t_span.0, run.t_span.1]).unwrap();
    result_json(&integrate_rodas5p_mf_fast_observed_with_gcrodr_policy(
        &run.problem,
        run.t_span,
        &run.y0,
        &config(LinearMethod::Gcrodr, x0),
        &adaptive(rtol, run.atol_scale, span),
        &schedule,
        GcrodrRecyclePolicy::Cold,
    ))
}

/// The final-state reference: the exact solution where one exists, else the
/// dense fast driver at `REFERENCE_RTOL`.
fn reference(run: &Run) -> Value {
    let tf = run.t_span.1;
    if let Some(exact) = &run.exact {
        return json!({"kind": "exact", "y": hexes(&exact(tf))});
    }
    let full = run.full.as_ref().expect("reference problem");
    let span = tf - run.t_span.0;
    let schedule = OutputSchedule::new(vec![run.t_span.0, tf]).unwrap();
    let r = integrate_rodas5p_fast_observed(
        full,
        run.t_span,
        &run.y0,
        &rodas5p_integrators::AdaptiveStepConfig {
            max_attempts: 1_000_000,
            ..adaptive(REFERENCE_RTOL, run.atol_scale, span)
        },
        &schedule,
    )
    .expect("reference run");
    assert!(r.observed.success, "reference run of {} failed", run.id);
    json!({
        "kind": "dense-fast-rtol-1e-12",
        "attempts": r.attempts,
        "y": hexes(r.observed.y.last().unwrap()),
    })
}

/// Base export, run on the registration commit's solver source: GMRES
/// `solve_into` with `Zero` and `Previous`, GCRO-DR cold with `Previous`.
#[test]
#[ignore = "base export of research/spd07_mf_step_warm_start_20261007; release build"]
fn export_base() {
    let mut rows = Vec::new();
    for run in runs() {
        let reference = reference(&run);
        for rtol in RTOLS {
            rows.push(json!({
                "case": run.id,
                "rtol": rtol,
                "dimension": run.y0.len(),
                "reference": reference,
                "gmres_into_zero": gmres_into(&run, rtol, InitialGuess::Zero),
                "gmres_into_previous": gmres_into(&run, rtol, InitialGuess::Previous),
                "gcrodr_cold_previous": gcrodr_cold(&run, rtol, InitialGuess::Previous),
            }));
        }
    }
    write_output("SPD07_BASE", &json!({"rows": rows}));
}

/// GMRES `solve_into` through the traced entry point (the same driver call
/// as [`gmres_into`]), with the restart cycles of all stage solves.
fn gmres_into_traced(run: &Run, rtol: f64, x0: InitialGuess) -> Value {
    let span = run.t_span.1 - run.t_span.0;
    let schedule = OutputSchedule::new(vec![run.t_span.0, run.t_span.1]).unwrap();
    let mut cycles = 0_u64;
    let result = integrate_rodas5p_mf_fast_observed_traced(
        &run.problem,
        run.t_span,
        &run.y0,
        &config(LinearMethod::Gmres, x0),
        &adaptive(rtol, run.atol_scale, span),
        &schedule,
        true,
        &mut |_, _, _, _, work| cycles = work.gmres_into_cycles(),
    );
    let mut row = result_json(&result);
    row["gmres_into_cycles"] = json!(cycles);
    row
}

const GUESSES: [(&str, InitialGuess); 4] = [
    ("zero", InitialGuess::Zero),
    ("previous", InitialGuess::Previous),
    ("previous_step", InitialGuess::PreviousStep),
    ("previous_step_scaled", InitialGuess::PreviousStepScaled),
];

/// The recorded run: every initial guess for GMRES `solve_into` and
/// GCRO-DR cold on the base cases.
#[test]
#[ignore = "recorded run of research/spd07_mf_step_warm_start_20261007; release build"]
fn export_runs() {
    let mut rows = Vec::new();
    for run in runs() {
        for rtol in RTOLS {
            let mut row = json!({"case": run.id, "rtol": rtol, "dimension": run.y0.len()});
            for (name, x0) in GUESSES {
                row[format!("gmres_into_{name}")] = gmres_into_traced(&run, rtol, x0);
                row[format!("gcrodr_cold_{name}")] = gcrodr_cold(&run, rtol, x0);
            }
            rows.push(row);
        }
    }
    write_output("SPD07_RUNS", &json!({"rows": rows}));
}

fn base_rows() -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/spd07_mf_step_warm_start_20261007/BASE.json");
    let base: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    base["rows"].as_array().unwrap().clone()
}

/// The unchanged initial guesses reproduce the base export on the two
/// smallest cases (the recorded run checks all fourteen).
#[test]
fn default_guesses_reproduce_the_base_export() {
    let base = base_rows();
    for run in runs()
        .into_iter()
        .filter(|r| r.id == "quadratic-4" || r.id == "prothero-robinson-forced")
    {
        for rtol in RTOLS {
            let row = base
                .iter()
                .find(|r| r["case"] == run.id && r["rtol"] == rtol)
                .unwrap();
            assert_eq!(
                gmres_into(&run, rtol, InitialGuess::Zero),
                row["gmres_into_zero"]
            );
            let mut traced = gmres_into_traced(&run, rtol, InitialGuess::Previous);
            traced.as_object_mut().unwrap().remove("gmres_into_cycles");
            assert_eq!(traced, row["gmres_into_previous"]);
            assert_eq!(
                gcrodr_cold(&run, rtol, InitialGuess::Previous),
                row["gcrodr_cold_previous"]
            );
        }
    }
}

/// The step-indexed starts change the Krylov work (they are used) and keep
/// the run successful on a small case.
#[test]
fn step_indexed_starts_are_used() {
    let run = runs().into_iter().find(|r| r.id == "quadratic-4").unwrap();
    let previous = gmres_into_traced(&run, 1.0e-6, InitialGuess::Previous);
    for x0 in [InitialGuess::PreviousStep, InitialGuess::PreviousStepScaled] {
        let step = gmres_into_traced(&run, 1.0e-6, x0);
        assert_eq!(step["success"], true);
        assert_ne!(
            step["counters"]["linear_matvecs"],
            previous["counters"]["linear_matvecs"]
        );
    }
}

/// Every other stage solver refuses the step-indexed starts.
#[test]
fn sequential_step_refuses_step_indexed_starts() {
    let run = runs().into_iter().find(|r| r.id == "quadratic-4").unwrap();
    for x0 in [InitialGuess::PreviousStep, InitialGuess::PreviousStepScaled] {
        let mut counters = rodas5p_core::WorkCounters::default();
        let result = sequential_matrix_free_step(
            &run.problem,
            0.0,
            &run.y0,
            1.0e-3,
            &config(LinearMethod::Gmres, x0),
            None,
            1.0e-6,
            1.0e-6,
            false,
            &mut counters,
        );
        let error = match result {
            Ok(_) => panic!("the sequential step accepted {x0:?}"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("step-indexed"), "{error}");
    }
}

/// Stage indexing and scaling of the step-indexed starts through the
/// workspace: re-attempting the recorded step's system (same `t`, `y`, `h`)
/// starts every stage from its own solution, so no stage needs a Krylov
/// iteration; with `h` changed, the scaled start differs from the unscaled
/// one; and after `clear_accepted_step` the start is `Previous` again.
#[test]
fn recorded_step_starts_each_stage_from_its_own_solution() {
    use rodas5p_core::WorkCounters;
    use rodas5p_integrators::Rodas5pMfFastWorkspace;
    let run = runs().into_iter().find(|r| r.id == "quadratic-4").unwrap();
    let (t, h, atol, rtol) = (0.0, 1.0e-2, 1.0e-8, 1.0e-8);
    let attempt = |work: &mut Rodas5pMfFastWorkspace, h: f64| {
        let mut counters = WorkCounters::default();
        work.attempt(
            &run.problem,
            t,
            &run.y0,
            h,
            true,
            None,
            atol,
            rtol,
            &mut counters,
        )
        .unwrap();
        counters
    };
    for x0 in [InitialGuess::PreviousStep, InitialGuess::PreviousStepScaled] {
        let mut work =
            Rodas5pMfFastWorkspace::new(&run.problem, &config(LinearMethod::Gmres, x0)).unwrap();
        let first = attempt(&mut work, h);
        assert!(first.linear_iterations > 0);
        let stages: Vec<Vec<f64>> = (0..8).map(|i| work.stage(i).to_vec()).collect();
        work.record_accepted_step(h);
        let again = attempt(&mut work, h);
        assert_eq!(
            again.linear_iterations, 0,
            "{x0:?}: a stage started elsewhere"
        );
        for (i, stage) in stages.iter().enumerate() {
            assert_eq!(work.stage(i), &stage[..], "{x0:?}: stage {i}");
        }
        work.clear_accepted_step();
        let cleared = attempt(&mut work, h);
        assert_eq!(cleared.linear_iterations, first.linear_iterations, "{x0:?}");
    }
    // A changed step: the scaled and unscaled starts differ.
    let mut runs_at_half = Vec::new();
    for x0 in [InitialGuess::PreviousStep, InitialGuess::PreviousStepScaled] {
        let mut work =
            Rodas5pMfFastWorkspace::new(&run.problem, &config(LinearMethod::Gmres, x0)).unwrap();
        attempt(&mut work, h);
        work.record_accepted_step(h);
        let half = attempt(&mut work, 0.5 * h);
        runs_at_half.push((half.linear_matvecs, work.stage(0).to_vec()));
    }
    assert_ne!(runs_at_half[0], runs_at_half[1]);
}
