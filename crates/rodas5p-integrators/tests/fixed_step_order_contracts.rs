//! Fixed-step order of RODAS5P where the suite had no evidence (audit F-018,
//! F-020): stiff problems, a mass matrix M != I, finite-difference f_t,
//! preconditioned GMRES, and the production forcing rule at an outer
//! tolerance that does not depend on h.
//!
//! h = 1/2^k; slope = log2(e_k / e_{k+1}); only errors above 1e-12 count as
//! pre-floor. Two-sided bands mean that an improvement also forces a re-seal.
//! Band sources: audit harness runs E-04 / E-09 (n = 256 / 512), and one
//! measurement of these in-tree variants (recorded next to each band).

use std::sync::Arc;

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters, error_scale,
    safe_l2, wrms,
};
use rodas5p_integrators::{
    OdeProblem, constant_affine_mass_problem, prothero_robinson_problem,
    semilinear_advection_diffusion_problem, sequential_matrix_free_step_with_inner_forcing,
    sequential_step,
};

const FLOOR: f64 = 1.0e-12;

fn direct() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Direct,
        ..LinearSolverConfig::default()
    }
}

/// Fixed-step RODAS5P with the given linear solver; returns the final state.
fn integrate(
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    steps: usize,
    linear: &LinearSolverConfig,
) -> Vec<f64> {
    let h = final_time / steps as f64;
    let mut y = y0.to_vec();
    let mut counters = WorkCounters::default();
    for step in 0..steps {
        let report = sequential_step(
            problem,
            step as f64 * h,
            &y,
            h,
            linear,
            None,
            1.0e-12,
            1.0e-12,
            true,
            &mut counters,
        )
        .unwrap();
        y = report.y_new;
    }
    y
}

fn distance(a: &[f64], b: &[f64]) -> f64 {
    safe_l2(&a.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<_>>())
}

/// Endpoint errors against the exact solution for h = final_time / 2^k.
fn exact_errors(
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    ks: std::ops::RangeInclusive<u32>,
    linear: &LinearSolverConfig,
) -> Vec<f64> {
    let exact = problem.exact(final_time).expect("exact solution");
    ks.map(|k| distance(&integrate(problem, y0, final_time, 1 << k, linear), &exact))
        .collect()
}

/// Differences y_{2^k} - y_{2^{k+1}}; same asymptotic slope, no exact
/// solution needed.
fn richardson_errors(
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    ks: std::ops::RangeInclusive<u32>,
    linear: &LinearSolverConfig,
) -> Vec<f64> {
    let (first, last) = (*ks.start(), *ks.end());
    let solutions = (first..=last + 1)
        .map(|k| integrate(problem, y0, final_time, 1 << k, linear))
        .collect::<Vec<_>>();
    solutions
        .windows(2)
        .map(|w| distance(&w[0], &w[1]))
        .collect()
}

fn pre_floor_slopes(errors: &[f64]) -> Vec<f64> {
    errors
        .windows(2)
        .filter(|pair| pair[1] > FLOOR)
        .map(|pair| (pair[0] / pair[1]).log2())
        .collect()
}

fn has_consecutive_slopes_at_least(slopes: &[f64], minimum: f64, count: usize) -> bool {
    slopes
        .windows(count)
        .any(|window| window.iter().all(|slope| *slope >= minimum))
}

#[test]
fn direct_fixed_step_nonstiff_order_is_five() {
    // E-04 p2 direct: 5.33 / 5.27 / 5.15 / 5.05.
    let (semilinear, y0) =
        semilinear_advection_diffusion_problem(16, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
    let errors = exact_errors(&semilinear, &y0, 1.0, 3..=6, &direct());
    let slopes = pre_floor_slopes(&errors);
    assert!(
        has_consecutive_slopes_at_least(&slopes, 4.8, 2),
        "semilinear slopes {slopes:?} errors {errors:?}"
    );
}

#[test]
fn direct_fixed_step_with_mass_matrix_is_order_five() {
    // Audit F-020: no order test used M != I. E-09 constant_affine_mass:
    // 5.18 / 5.64 / 5.83. The problem has no closed-form solution, so the
    // slope is taken from successive differences.
    let (problem, y0, _, _) = constant_affine_mass_problem();
    let differences = richardson_errors(&problem, &y0, 1.0, 1..=5, &direct());
    let slopes = pre_floor_slopes(&differences);
    assert!(
        has_consecutive_slopes_at_least(&slopes, 4.8, 2),
        "mass slopes {slopes:?} differences {differences:?}"
    );
}

#[test]
fn direct_fixed_step_stiff_pr_order_is_reduced_band() {
    // E-09: 2.95 / 3.03 / 3.11 / 3.25, err(h = 1/64) = 1.75e-12.
    let (problem, y0) = prothero_robinson_problem(-1.0e4, 0.0, 0.0);
    let errors = exact_errors(&problem, &y0, 1.0, 2..=6, &direct());
    let slopes = pre_floor_slopes(&errors);
    assert!(!slopes.is_empty());
    assert!(
        slopes.iter().all(|slope| (2.7..=3.6).contains(slope)),
        "stiff PR slopes {slopes:?} errors {errors:?}"
    );
    assert!(errors[4] <= 1.0e-11, "err(1/64) = {:e}", errors[4]);
}

#[test]
fn direct_fixed_step_mildly_stiff_pr_band() {
    // E-09: 4.03, 4.69.
    let (problem, y0) = prothero_robinson_problem(-1.0e2, 0.0, 0.0);
    let errors = exact_errors(&problem, &y0, 1.0, 2..=7, &direct());
    let slopes = pre_floor_slopes(&errors);
    assert!(slopes.len() >= 2, "{slopes:?}");
    for slope in &slopes[slopes.len() - 2..] {
        assert!(
            (3.8..=5.2).contains(slope),
            "mildly stiff PR slopes {slopes:?} errors {errors:?}"
        );
    }
}

/// Prothero-Robinson without an analytic f_t, so the step differentiates the
/// right-hand side in t by finite differences.
fn prothero_robinson_fd_t(lambda: f64) -> OdeProblem {
    OdeProblem::new(
        "PR-fd-t",
        1,
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = lambda * (y[0] - t.sin()) + t.cos();
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, _y: &[f64]| {
            rodas5p_core::DenseMatrix::new(1, 1, vec![lambda])
        })),
        None,
        None,
        false,
        None,
        Some(Arc::new(|t: f64| vec![t.sin()])),
    )
    .unwrap()
}

#[test]
fn finite_difference_partial_t_is_accurate_and_keeps_the_order() {
    // Audit F-020: FD f_t accuracy was never measured.
    let lambda = -1.0e2;
    let fd = prothero_robinson_fd_t(lambda);
    let (analytic, y0) = prothero_robinson_problem(lambda, 0.0, 0.0);
    let mut counters = WorkCounters::default();
    for &(t, y) in &[(0.3, 0.2), (0.9, 0.8), (2.5, -0.4)] {
        let approx = fd.eval_partial_t(t, &[y], &mut counters).unwrap()[0];
        let exact = analytic.eval_partial_t(t, &[y], &mut counters).unwrap()[0];
        let scale = exact.abs().max(1.0);
        assert!(
            (approx - exact).abs() <= 1.0e-6 * scale,
            "t {t}: fd {approx:e} vs analytic {exact:e}"
        );
    }
    let fd_errors = exact_errors(&fd, &y0, 1.0, 2..=7, &direct());
    let analytic_errors = exact_errors(&analytic, &y0, 1.0, 2..=7, &direct());
    for (fd_error, analytic_error) in fd_errors.iter().zip(&analytic_errors) {
        assert!(
            *fd_error <= 2.0 * analytic_error + 1.0e-10,
            "fd {fd_errors:?} vs analytic {analytic_errors:?}"
        );
    }
}

#[test]
fn preconditioned_gmres_paths_keep_the_direct_order() {
    // Audit F-020: no integrator-level test set a preconditioner.
    let (problem, y0) =
        semilinear_advection_diffusion_problem(16, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
    let reference = exact_errors(&problem, &y0, 1.0, 3..=6, &direct());
    for preconditioner in [PreconditionerKind::Jacobi, PreconditionerKind::Direct] {
        let gmres = LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: 1.0e-13,
            atol: 1.0e-15,
            restart: 32,
            maxiter: 256,
            preconditioner,
            x0_strategy: InitialGuess::Zero,
            ..LinearSolverConfig::default()
        };
        let errors = exact_errors(&problem, &y0, 1.0, 3..=6, &gmres);
        for (error, direct_error) in errors.iter().zip(&reference) {
            assert!(
                (error - direct_error).abs() <= 1.0e-3 * direct_error + 1.0e-12,
                "{preconditioner:?}: {errors:?} vs direct {reference:?}"
            );
        }
        let slopes = pre_floor_slopes(&errors);
        assert!(
            has_consecutive_slopes_at_least(&slopes, 4.8, 2),
            "{preconditioner:?} slopes {slopes:?}"
        );
    }
}

/// The production forcing rule at an outer tolerance that is NOT a function
/// of h; returns the endpoint error in outer WRMS units.
fn forcing_fixed_outer_error(
    problem: &OdeProblem,
    y0: &[f64],
    final_time: f64,
    steps: usize,
    rtol: f64,
) -> Result<f64, String> {
    let atol = 1.0e-2 * rtol;
    let h = final_time / steps as f64;
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 8,
        maxiter: 64,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    };
    let mut y = y0.to_vec();
    let mut work = WorkCounters::default();
    for step in 0..steps {
        let report = sequential_matrix_free_step_with_inner_forcing(
            problem,
            step as f64 * h,
            &y,
            h,
            &config,
            None,
            atol,
            rtol,
            true,
            &mut work,
        )
        .map_err(|error| format!("step {step}: {error}"))?;
        y = report.step.y_new;
    }
    let exact = problem.exact(final_time).unwrap();
    let scale = error_scale(&exact, &exact, &[atol], rtol).unwrap();
    let difference = y.iter().zip(&exact).map(|(a, b)| a - b).collect::<Vec<_>>();
    Ok(wrms(&difference, &scale).unwrap())
}

fn outer_wrms(problem: &OdeProblem, y: &[f64], final_time: f64, rtol: f64) -> f64 {
    let exact = problem.exact(final_time).unwrap();
    let scale = error_scale(&exact, &exact, &[1.0e-2 * rtol], rtol).unwrap();
    let difference = y.iter().zip(&exact).map(|(a, b)| a - b).collect::<Vec<_>>();
    wrms(&difference, &scale).unwrap()
}

/// n = 64 semilinear advection-diffusion, matrix-free: large enough that
/// GMRES stops at the forcing tolerance instead of converging exactly.
/// Measured at rtol 1e-6, h = 1/8 .. 1/256: errors 4.7e-2, 2.6e-2, 2.0e-2,
/// 6.6e-5, 4.3e-3, 5.1e-5 outer WRMS units, a tolerance floor with no
/// order. The small in-tree problems (n <= 32) keep order five down to
/// roundoff because their GMRES solves are exact.
fn forcing_problem() -> (OdeProblem, Vec<f64>) {
    let (problem, y0) =
        semilinear_advection_diffusion_problem(64, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
    (problem.jvp_only_clone().unwrap(), y0)
}

const FORCING_FINAL_TIME: f64 = 1.0;

#[test]
fn forcing_fixed_outer_rtol_error_is_bounded_by_tolerance_floor() {
    // A tolerance-floor contract, not an order-5 claim (E-04 floors
    // 0.01-0.8 rtol on P2).
    let (problem, y0) = forcing_problem();
    for rtol in [1.0e-4, 1.0e-6] {
        for k in 3..=8 {
            let steps = 1 << k;
            let forcing =
                forcing_fixed_outer_error(&problem, &y0, FORCING_FINAL_TIME, steps, rtol).unwrap();
            let (dense, _) =
                semilinear_advection_diffusion_problem(64, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
            let direct_state = integrate(&dense, &y0, FORCING_FINAL_TIME, steps, &direct());
            let direct_error = outer_wrms(&dense, &direct_state, FORCING_FINAL_TIME, rtol);
            assert!(
                forcing <= (3.0 * direct_error).max(1.0),
                "rtol {rtol:e}, h = 1/{steps}: forcing {forcing:e} vs direct {direct_error:e}"
            );
        }
    }
}

#[test]
fn forcing_fixed_outer_rtol_does_not_claim_order_five() {
    // Expected-failure sentinel (audit F-018): at a fixed outer rtol the
    // forcing arm's error stalls at a tolerance floor, so at least one
    // pre-floor slope is below 4.5 today. Invert this test (>= 4.5 for at
    // least two halvings until the floor) when a forcing rule that retains
    // order five lands.
    let (problem, y0) = forcing_problem();
    let rtol = 1.0e-6;
    let errors = (3..=8)
        .map(|k| {
            forcing_fixed_outer_error(&problem, &y0, FORCING_FINAL_TIME, 1 << k, rtol).unwrap()
                * rtol
        })
        .collect::<Vec<_>>();
    let slopes = pre_floor_slopes(&errors);
    assert!(!slopes.is_empty(), "{errors:?}");
    assert!(
        slopes.iter().any(|slope| *slope < 4.5),
        "the forcing arm now keeps order five: invert this sentinel. slopes {slopes:?}"
    );
}

#[test]
fn forcing_rule_tight_tolerance_abort_is_typed() {
    // At rtol 1e-10 the forcing rule's roundoff floor exceeds its residual
    // allocation. The failure must be the typed refusal at step 0, not a
    // silent wrong answer. E-04 (P1, n = 256) refused at every h; on this
    // 1-D problem the stage right-hand side h f shrinks with h, and the
    // measured refusals are h = 1/4 .. 1/32. At h = 1/64 and 1/128 the run
    // completes within the tolerance (1.7e-2 and 1.5e-3 WRMS units).
    let (problem, y0) = prothero_robinson_problem(-1.0e4, 0.0, 0.0);
    let problem = problem.jvp_only_clone().unwrap();
    for k in 2..=5 {
        let error = forcing_fixed_outer_error(&problem, &y0, 1.0, 1 << k, 1.0e-10)
            .expect_err("rtol 1e-10 must refuse");
        assert!(
            error.starts_with("step 0:") && error.contains("roundoff floor"),
            "h = 1/{}: {error}",
            1 << k
        );
    }
    for k in 6..=7 {
        let error = forcing_fixed_outer_error(&problem, &y0, 1.0, 1 << k, 1.0e-10).unwrap();
        assert!(error <= 1.0, "h = 1/{}: {error:e}", 1 << k);
    }
}
