//! Export for the residual-to-output budget of the raw-U and K drivers
//! (research node `research/rnext01_residual_output_20261003`,
//! remaining-only DAG node R-NEXT-01): the 36 L-0038 one-step comparisons,
//! with both drivers' stages, outputs and error norms bit for bit, and the
//! coefficients. `tools/rnext01_residual_output.py` computes the budgets.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{cases, linear_config, write_output};
use rodas5p_core::{
    DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters, rodas5p_coefficients,
};
use rodas5p_integrators::{KrylovState, Rodas5pMfFastWorkspace, sequential_matrix_free_step};
use serde_json::{Value, json};

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

fn matrix_hex(m: &DenseMatrix) -> Vec<Vec<String>> {
    (0..m.nrows())
        .map(|i| (0..m.ncols()).map(|j| hex(m[(i, j)])).collect())
        .collect()
}

#[test]
#[ignore = "export of research/rnext01_residual_output_20261003; release build"]
fn export_one_step_comparisons() {
    let coeffs = rodas5p_coefficients().unwrap();
    let coefficients = json!({
        "gamma": hex(coeffs.gamma), "a": matrix_hex(&coeffs.a), "c_matrix": matrix_hex(&coeffs.c_matrix),
        "c": hexes(&coeffs.c), "b_code": hexes(&coeffs.b_code), "gamma_rows": hexes(&coeffs.gamma_rows),
        "alpha": matrix_hex(&coeffs.alpha), "gamma_matrix": matrix_hex(&coeffs.gamma_matrix),
        "b": hexes(&coeffs.b), "btilde": hexes(&coeffs.btilde),
    });
    let mut rows: Vec<Value> = Vec::new();
    for case in cases() {
        for h in case.steps {
            for method in [
                LinearMethod::Gmres,
                LinearMethod::Lgmres,
                LinearMethod::Gcrodr,
            ] {
                let linear = LinearSolverConfig {
                    maxiter: 4000,
                    ..linear_config(method, 1.0e-12)
                };
                let (atol, rtol) = (1.0e-6 * case.atol_scale, 1.0e-6);
                let t = case.t_span.0;
                let mut recycle = KrylovState::for_method(method);
                let k = sequential_matrix_free_step(
                    &case.problem,
                    t,
                    &case.y0,
                    h,
                    &linear,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    true,
                    &mut WorkCounters::default(),
                );
                let mut work = Rodas5pMfFastWorkspace::new(&case.problem, &linear).unwrap();
                let mut recycle = KrylovState::for_method(method);
                let u = work.attempt(
                    &case.problem,
                    t,
                    &case.y0,
                    h,
                    true,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    &mut WorkCounters::default(),
                );
                let k_json = match &k {
                    Ok(step) => json!({
                        "stages": step.stages.iter().map(|s| hexes(s)).collect::<Vec<_>>(),
                        "y_new": hexes(&step.y_new), "error_vector": hexes(&step.error_vector),
                        "error_norm": hex(step.error_norm),
                    }),
                    Err(error) => json!({"error": error.to_string()}),
                };
                let u_json = match &u {
                    Ok(norm) => json!({
                        "stages": (0..8).map(|i| hexes(work.stage(i))).collect::<Vec<_>>(),
                        "y_new": hexes(work.y_new()), "error_norm": hex(*norm),
                    }),
                    Err(error) => json!({"error": error.to_string()}),
                };
                println!(
                    "{} h={h:e} {method:?}: K {} U {}",
                    case.id,
                    k.is_ok(),
                    u.is_ok()
                );
                rows.push(json!({
                    "problem": case.id, "method": format!("{method:?}"), "t": hex(t), "h": hex(h),
                    "y": hexes(&case.y0), "atol": hex(atol), "rtol": hex(rtol),
                    "k": k_json, "u": u_json,
                }));
            }
        }
    }
    write_output(
        "RNEXT01_EXPORT",
        &json!({"schema": "vigilode-rnext01-export-v1", "coefficients": coefficients, "cases": rows}),
    );
}
