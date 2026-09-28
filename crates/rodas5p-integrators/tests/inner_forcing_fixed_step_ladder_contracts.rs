//! WU-3 (audit F-009, F-008, F-030, F-031): the production inner-forcing rule
//! at a fixed outer tolerance.
//!
//! The existing order test (`inner_forcing_v2_contracts.rs`) ties `rtol` to
//! `h^6`, so it cannot see a stage-residual budget that does not shrink with
//! `h`.  These contracts hold `rtol` fixed while `h` is halved and compare the
//! matrix-free inexact arm with the direct-LU arm on the same fixed-step
//! ladder.  The pass criterion is relative to the direct arm, not a slope of
//! at least 4.5: on stiff Prothero-Robinson even direct LU shows the classical
//! Rosenbrock order reduction to about 4 (audit E-04).

use std::sync::Arc;

use rodas5p_core::{
    CoreResult, DenseMatrix, InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind,
    WorkCounters,
};
use rodas5p_integrators::{
    OdeProblem, RODAS5P_INNER_FORCING_ERROR_EXPONENT, RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION,
    rodas5p_inner_forcing_error_limit, rodas5p_inner_forcing_target,
    semilinear_advection_diffusion_problem, sequential_matrix_free_step_with_inner_forcing,
    sequential_step,
};

/// Stated tracking factor: the inexact arm may lose at most this factor
/// against direct LU at every rung (the E-04 crossover rule).
const TRACKING_FACTOR: f64 = 3.0;
/// Absolute allowance below which both arms sit at their roundoff floor.
const ROUNDOFF_ALLOWANCE: f64 = 1.0e-13;
const OUTER_RTOL: f64 = 1.0e-6;
const OUTER_ATOL: f64 = 1.0e-2 * OUTER_RTOL;

/// Vector Prothero-Robinson problem `y' = Lambda (y - g) + g'` with a diagonal
/// `Lambda` log-spaced in `[-lambda_max, -1]` and `g_i(t) = sin(t + phi_i)`.
///
/// This is the E-04 `p1pr` problem with `Q = I`: the modes decouple, but an
/// unpreconditioned Krylov solve still needs a polynomial over the whole
/// spectrum, so the inner residual budget, not the dimension, ends GMRES.
fn diagonal_prothero_robinson(n: usize, lambda_max: f64) -> (OdeProblem, Vec<f64>) {
    let lambda: Arc<Vec<f64>> = Arc::new(
        (0..n)
            .map(|k| -(lambda_max.ln() * k as f64 / (n - 1) as f64).exp())
            .collect(),
    );
    let phase: Arc<Vec<f64>> = Arc::new(
        (0..n)
            .map(|k| 2.0 * std::f64::consts::PI * ((k as f64 * 0.618_033_988_749_895) % 1.0))
            .collect(),
    );
    let rhs = {
        let (lambda, phase) = (lambda.clone(), phase.clone());
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            for i in 0..n {
                out[i] = lambda[i] * (y[i] - (t + phase[i]).sin()) + (t + phase[i]).cos();
            }
            Ok(())
        })
    };
    let jacobian = {
        let lambda = lambda.clone();
        Arc::new(move |_t: f64, _y: &[f64]| -> CoreResult<DenseMatrix> {
            let mut matrix = DenseMatrix::zeros(n, n);
            for i in 0..n {
                matrix[(i, i)] = lambda[i];
            }
            Ok(matrix)
        })
    };
    let jvp = {
        let lambda = lambda.clone();
        Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
            for i in 0..n {
                out[i] = lambda[i] * v[i];
            }
            Ok(())
        })
    };
    let partial_t = {
        let (lambda, phase) = (lambda.clone(), phase.clone());
        Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
            for i in 0..n {
                out[i] = -lambda[i] * (t + phase[i]).cos() - (t + phase[i]).sin();
            }
            Ok(())
        })
    };
    let exact = {
        let phase = phase.clone();
        Arc::new(move |t: f64| phase.iter().map(|p| (t + p).sin()).collect::<Vec<_>>())
    };
    let y0 = exact(0.0);
    let problem = OdeProblem::new(
        format!("diagonal-prothero-robinson-n{n}"),
        n,
        rhs,
        None,
        Some(jacobian),
        Some(jvp),
        Some(partial_t),
        false,
        None,
        Some(exact),
    )
    .unwrap();
    (problem, y0)
}

fn gmres_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 32,
        maxiter: 20_000,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
        ..LinearSolverConfig::default()
    }
}

#[derive(Clone, Copy)]
enum Arm {
    DirectLu,
    InexactForcing,
}

/// Relative max-norm error at the end of `2^k` fixed steps on `[0, 1]`.
fn fixed_step_endpoint_error(problem: &OdeProblem, y0: &[f64], k: u32, arm: Arm) -> f64 {
    let steps = 1usize << k;
    let h = 1.0 / steps as f64;
    let direct = LinearSolverConfig::default();
    let krylov = gmres_config();
    let mut work = WorkCounters::default();
    let mut t = 0.0;
    let mut y = y0.to_vec();
    for step in 0..steps {
        let t_new = (step + 1) as f64 * h;
        let step_h = t_new - t;
        y = match arm {
            Arm::DirectLu => {
                sequential_step(
                    problem, t, &y, step_h, &direct, None, OUTER_ATOL, OUTER_RTOL, true, &mut work,
                )
                .unwrap_or_else(|error| panic!("direct arm k={k} step {step}: {error}"))
                .y_new
            }
            Arm::InexactForcing => {
                sequential_matrix_free_step_with_inner_forcing(
                    problem, t, &y, step_h, &krylov, None, OUTER_ATOL, OUTER_RTOL, true, &mut work,
                )
                .unwrap_or_else(|error| panic!("inexact arm k={k} step {step}: {error}"))
                .step
                .y_new
            }
        };
        t = t_new;
    }
    let exact = problem.exact(t).unwrap();
    let scale = exact.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    y.iter()
        .zip(&exact)
        .fold(0.0_f64, |m, (a, b)| m.max((a - b).abs()))
        / scale
}

fn assert_inexact_tracks_direct(label: &str, problem: &OdeProblem, y0: &[f64], rungs: &[u32]) {
    let mut table = Vec::new();
    for &k in rungs {
        let direct = fixed_step_endpoint_error(problem, y0, k, Arm::DirectLu);
        let inexact = fixed_step_endpoint_error(problem, y0, k, Arm::InexactForcing);
        table.push((k, direct, inexact));
    }
    for &(k, direct, inexact) in &table {
        assert!(
            inexact <= TRACKING_FACTOR * direct + ROUNDOFF_ALLOWANCE,
            "{label}: at h = 2^-{k} the inexact arm error {inexact:.3e} exceeds \
             {TRACKING_FACTOR} x direct-LU error {direct:.3e}; ladder (k, direct, inexact) = {table:?}"
        );
    }
}

#[test]
fn inexact_arm_tracks_direct_lu_on_a_fixed_rtol_halving_ladder_prothero_robinson() {
    // E-04 p1pr: the production rule leaves an h-independent floor near
    // 0.1-0.3 rtol while direct LU keeps converging.
    let (problem, y0) = diagonal_prothero_robinson(128, 1.0e6);
    assert_inexact_tracks_direct("diagonal Prothero-Robinson", &problem, &y0, &[3, 4, 5]);
}

#[test]
fn inexact_arm_tracks_direct_lu_on_a_fixed_rtol_halving_ladder_advection_diffusion() {
    // E-04 p2 (n reduced from 512 to 128): the floor sits at 0.01-0.8 rtol.
    let (problem, y0) =
        semilinear_advection_diffusion_problem(128, 0.02, 3.0, -1.0, 10.0, 0.0).unwrap();
    assert_inexact_tracks_direct("semilinear advection-diffusion", &problem, &y0, &[3, 4, 5]);
}

#[test]
fn forcing_target_on_a_stiff_stage_returns_a_floor_instead_of_an_error() {
    // F-009: rhs_wrms > 0.1 / (l1 * 64 eps) ~ 1.4e12 used to abort the step
    // before any GMRES iteration, although LU solves the same stage.
    let output_weight_l1 = 4.9455;
    let target = rodas5p_inner_forcing_target(1.0e10, 1.0e13, output_weight_l1)
        .expect("a stiff stage must get a residual floor, not an error");
    assert!(target.tau.is_finite() && target.tau > 0.0);
    assert!(target.tau >= 64.0 * f64::EPSILON * 1.0e13 * (1.0 - 1.0e-12));
    assert!(
        target.floor_active,
        "a floor-limited target must be flagged"
    );
}

#[test]
fn error_scaled_residual_limit_shrinks_with_the_embedded_estimate() {
    // F-008: the admitted stage residual must follow the truncation error.
    let l1 = 4.9455;
    let cap = RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION / l1;
    let at = |error: f64| rodas5p_inner_forcing_error_limit(error, l1).unwrap();
    assert_eq!(at(1.0).to_bits(), cap.to_bits());
    assert_eq!(at(4.0).to_bits(), cap.to_bits());
    let expected = cap * 1.0e-3_f64.powf(RODAS5P_INNER_FORCING_ERROR_EXPONENT);
    assert_eq!(at(1.0e-3).to_bits(), expected.to_bits());
    assert!(at(1.0e-6) < at(1.0e-3) && at(1.0e-3) < at(0.5));
    assert_eq!(at(0.0), 0.0);
    assert!(rodas5p_inner_forcing_error_limit(f64::NAN, l1).is_err());
    assert!(rodas5p_inner_forcing_error_limit(-1.0, l1).is_err());
    assert!(rodas5p_inner_forcing_error_limit(0.5, 0.0).is_err());
}

#[test]
fn refined_stage_reports_are_flagged_and_within_the_error_scaled_limit() {
    let (problem, y0) = diagonal_prothero_robinson(64, 1.0e6);
    let config = gmres_config();
    let mut work = WorkCounters::default();
    let report = sequential_matrix_free_step_with_inner_forcing(
        &problem,
        0.0,
        &y0,
        1.0 / 32.0,
        &config,
        None,
        OUTER_ATOL,
        OUTER_RTOL,
        true,
        &mut work,
    )
    .unwrap();
    assert!(
        report
            .stage_forcing
            .iter()
            .any(|row| row.refinement_pass > 0)
    );
    let l1 = 4.9455;
    let limit = rodas5p_inner_forcing_error_limit(report.step.error_norm, l1).unwrap();
    for row in &report.stage_forcing {
        let floor = 64.0 * f64::EPSILON * row.flow_wrms.max(row.rhs_wrms).max(1.0);
        assert!(row.achieved_residual_wrms <= row.tau);
        assert!(row.tau <= RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION / l1);
        // The final pass meets the limit of the estimate it was refined against;
        // that estimate and the final one agree to well within a factor of two.
        assert!(
            row.achieved_residual_wrms <= 2.0 * limit.max(floor),
            "{row:?} limit {limit:e}"
        );
    }
}

#[test]
fn tight_rtol_stiff_fixed_steps_do_not_abort_at_step_zero() {
    // F-009 in E-04: P1PR at rtol = 1e-10 failed at step 0 for h >= 1/32.
    let (problem, y0) = diagonal_prothero_robinson(32, 1.0e6);
    let config = gmres_config();
    let mut work = WorkCounters::default();
    let h = 0.125;
    let report = sequential_matrix_free_step_with_inner_forcing(
        &problem, 0.0, &y0, h, &config, None, 1.0e-12, 1.0e-10, true, &mut work,
    )
    .expect("a legal stiff input at rtol = 1e-10 must not abort at step 0");
    let direct = sequential_step(
        &problem,
        0.0,
        &y0,
        h,
        &LinearSolverConfig::default(),
        None,
        1.0e-12,
        1.0e-10,
        true,
        &mut work,
    )
    .unwrap();
    let exact = problem.exact(h).unwrap();
    let error = |y: &[f64]| {
        y.iter()
            .zip(&exact)
            .fold(0.0_f64, |m, (a, b)| m.max((a - b).abs()))
    };
    assert!(
        error(&report.step.y_new) <= TRACKING_FACTOR * error(&direct.y_new) + ROUNDOFF_ALLOWANCE,
        "inexact {:.3e} direct {:.3e}",
        error(&report.step.y_new),
        error(&direct.y_new)
    );
}
