//! F-033 attribution at a fixed mesh (external re-audit, 2.3).
//!
//!     cargo run --release -p rodas5p-fair-ab --example semilinear_f033_ablation
//!
//! The adaptive dense campaign arm is run once to fix its accepted mesh. The
//! same mesh is then replayed, step for step, by arms that differ in one
//! ingredient each:
//!
//! * `forcing-jvp`: the campaign solve (JVP-defined W, WRMS inner forcing);
//! * `tight-jvp`: JVP-defined W, every stage solved to GMRES rtol 1e-13;
//! * `direct-w`: W assembled from the Jacobian (its columns taken from the
//!   analytic JVP) and factorized by LU;
//! * `fd-jvp`: W applied through a forward-difference JVP. An FD operator is
//!   not an exact linear map (eps depends on the direction), so GMRES cannot
//!   reach 1e-13 against it; this arm solves to rtol 1e-8;
//!
//! and the global error is measured at the mesh nodes (endpoint values) and,
//! for the campaign run, at the requested output times (dense values). If
//! the node errors agree across arms, the error belongs to the RODAS5P steps
//! on that mesh, not to inexact solves, the W construction or the
//! derivative; the node/dense gap is the interpolant's share.

use std::sync::Arc;

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters, error_scale,
    safe_l2, wrms,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, OdeProblem, OutputSamplingPlan, OutputSchedule,
    ScientificCorpusV2, ScientificFamily, integrate_sequential_matrix_free_adaptive_dense_observed,
    sequential_matrix_free_step, sequential_matrix_free_step_with_inner_forcing, sequential_step,
};

fn tight_gmres() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-13,
        atol: 1.0e-15,
        restart: 64,
        maxiter: 1024,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    }
}

fn campaign_gmres() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
        ..LinearSolverConfig::default()
    }
}

/// The same problem with its JVP replaced by a forward difference
/// `[f(t, y + eps v) - f(t, y)] / eps`, eps = sqrt(u) (1 + ||y||) / ||v||.
fn forward_difference_jvp(problem: &OdeProblem) -> OdeProblem {
    let rhs_problem = problem.clone();
    let jvp_problem = problem.clone();
    let ft_problem = problem.clone();
    OdeProblem::new(
        format!("{}-fd-jvp", problem.name),
        problem.dimension,
        Arc::new(move |t, y: &[f64], out: &mut [f64]| {
            let value = rhs_problem.eval_rhs(t, y, &mut WorkCounters::default())?;
            out.copy_from_slice(&value);
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |t, y: &[f64], v: &[f64], out: &mut [f64]| {
            let norm = safe_l2(v);
            if norm == 0.0 {
                out.fill(0.0);
                return Ok(());
            }
            let eps = f64::EPSILON.sqrt() * (1.0 + safe_l2(y)) / norm;
            let shifted = y
                .iter()
                .zip(v)
                .map(|(a, b)| a + eps * b)
                .collect::<Vec<_>>();
            let mut counters = WorkCounters::default();
            let base = jvp_problem.eval_rhs(t, y, &mut counters)?;
            let moved = jvp_problem.eval_rhs(t, &shifted, &mut counters)?;
            for ((slot, a), b) in out.iter_mut().zip(&moved).zip(&base) {
                *slot = (a - b) / eps;
            }
            Ok(())
        })),
        (!problem.autonomous).then(|| {
            Arc::new(move |t, y: &[f64], out: &mut [f64]| {
                let value = ft_problem.eval_partial_t(t, y, &mut WorkCounters::default())?;
                out.copy_from_slice(&value);
                Ok(())
            }) as PartialT
        }),
        problem.autonomous,
        problem.mass_matrix.clone(),
        None,
    )
    .expect("fd-jvp problem")
}

type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> rodas5p_core::CoreResult<()> + Send + Sync>;

/// The same problem with an assembled Jacobian from its analytic JVP.
fn assembled_jacobian(problem: &OdeProblem) -> OdeProblem {
    let rhs_problem = problem.clone();
    let jacobian_problem = problem.clone();
    let jvp_problem = problem.clone();
    let ft_problem = problem.clone();
    OdeProblem::new(
        format!("{}-assembled", problem.name),
        problem.dimension,
        Arc::new(move |t, y: &[f64], out: &mut [f64]| {
            let value = rhs_problem.eval_rhs(t, y, &mut WorkCounters::default())?;
            out.copy_from_slice(&value);
            Ok(())
        }),
        None,
        Some(Arc::new(move |t, y: &[f64]| {
            jacobian_problem.dense_jacobian(t, y, &mut WorkCounters::default())
        })),
        Some(Arc::new(move |t, y: &[f64], v: &[f64], out: &mut [f64]| {
            let operator = jvp_problem.linearize_matrix_free(t, y)?;
            operator.apply(v, out)
        })),
        (!problem.autonomous).then(|| {
            Arc::new(move |t, y: &[f64], out: &mut [f64]| {
                let value = ft_problem.eval_partial_t(t, y, &mut WorkCounters::default())?;
                out.copy_from_slice(&value);
                Ok(())
            }) as PartialT
        }),
        problem.autonomous,
        problem.mass_matrix.clone(),
        None,
    )
    .expect("assembled problem")
}

fn node_error(problem: &OdeProblem, t: f64, y: &[f64], atol: f64, rtol: f64) -> f64 {
    let exact = problem.exact(t).expect("manufactured exact state");
    let scale = error_scale(&exact, &exact, &[atol], rtol).unwrap();
    let difference = y.iter().zip(&exact).map(|(a, b)| a - b).collect::<Vec<_>>();
    wrms(&difference, &scale).unwrap()
}

fn main() {
    for spec in ScientificCorpusV2::calibration_specs()
        .into_iter()
        .filter(|spec| {
            spec.family == ScientificFamily::SemilinearAdvectionDiffusionRamped
                && spec.dimension == 96
        })
    {
        let case = spec.build().unwrap();
        let segment = &case.integration_segments[0];
        let span = segment.t_span.1 - segment.t_span.0;
        let adaptive = AdaptiveStepConfig {
            atol: spec.atol,
            rtol: spec.rtol,
            initial_step: span / 100.0,
            min_step: 1.0e-12,
            max_step: span,
            max_attempts: 200_000,
            safety: 0.9,
            min_factor: 0.2,
            max_factor: 5.0,
            reject_max_factor: 0.9,
            controller: ControllerKind::Integral,
            max_step_policy: rodas5p_integrators::MaxStepPolicy::AllowClockResolutionSlack,
        };
        let matrix_free = segment.problem.jvp_only_clone().unwrap();
        let outputs = spec
            .output_times
            .iter()
            .copied()
            .filter(|t| *t >= segment.t_span.0 && *t <= segment.t_span.1)
            .collect::<Vec<_>>();
        let run = integrate_sequential_matrix_free_adaptive_dense_observed(
            &matrix_free,
            segment.t_span,
            &case.y0,
            &campaign_gmres(),
            &adaptive,
            &OutputSamplingPlan::dense(OutputSchedule::new(outputs).unwrap()),
        )
        .unwrap();
        let mesh = run.diagnostics.accepted_step_sizes.clone();
        let dense_error = run
            .observed
            .t
            .iter()
            .zip(&run.observed.y)
            .map(|(t, y)| node_error(&segment.problem, *t, y, spec.atol, spec.rtol))
            .fold(0.0_f64, f64::max);

        let fd_problem = forward_difference_jvp(&matrix_free);
        let assembled = assembled_jacobian(&matrix_free);
        let fd_gmres = LinearSolverConfig {
            rtol: 1.0e-8,
            atol: 1.0e-10,
            ..tight_gmres()
        };
        let direct = LinearSolverConfig {
            method: LinearMethod::Direct,
            ..LinearSolverConfig::default()
        };
        println!(
            "{} ({} accepted steps on segment 0, span {span:.3})",
            spec.id,
            mesh.len()
        );
        println!("  campaign dense run: max error at output times = {dense_error:.4e}");
        for arm in ["forcing-jvp", "tight-jvp", "direct-w", "fd-jvp"] {
            let mut t = segment.t_span.0;
            let mut y = case.y0.clone();
            let mut worst = 0.0_f64;
            let mut counters = WorkCounters::default();
            let mut failed = None;
            for &h in &mesh {
                let step = match arm {
                    "forcing-jvp" => sequential_matrix_free_step_with_inner_forcing(
                        &matrix_free,
                        t,
                        &y,
                        h,
                        &campaign_gmres(),
                        None,
                        spec.atol,
                        spec.rtol,
                        true,
                        &mut counters,
                    )
                    .map(|report| report.step),
                    "tight-jvp" => sequential_matrix_free_step(
                        &matrix_free,
                        t,
                        &y,
                        h,
                        &tight_gmres(),
                        None,
                        spec.atol,
                        spec.rtol,
                        true,
                        &mut counters,
                    ),
                    "direct-w" => sequential_step(
                        &assembled,
                        t,
                        &y,
                        h,
                        &direct,
                        None,
                        spec.atol,
                        spec.rtol,
                        true,
                        &mut counters,
                    ),
                    _ => sequential_matrix_free_step(
                        &fd_problem,
                        t,
                        &y,
                        h,
                        &fd_gmres,
                        None,
                        spec.atol,
                        spec.rtol,
                        true,
                        &mut counters,
                    ),
                };
                match step {
                    Ok(step) => y = step.y_new,
                    Err(error) => {
                        failed = Some(error.to_string());
                        break;
                    }
                }
                t += h;
                worst = worst.max(node_error(&segment.problem, t, &y, spec.atol, spec.rtol));
            }
            match failed {
                None => println!(
                    "  {arm:12} max error at mesh nodes = {worst:.4e}, endpoint = {:.4e}",
                    node_error(&segment.problem, t, &y, spec.atol, spec.rtol)
                ),
                Some(error) => println!("  {arm:12} failed at t = {t:.4}: {error}"),
            }
        }
    }
}
