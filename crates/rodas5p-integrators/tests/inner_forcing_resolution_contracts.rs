//! The forcing refinement reports whether its final stages meet the budget
//! derived from their own embedded estimate (external re-audit RA-05).

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    InnerForcingResolution, semilinear_advection_diffusion_problem,
    sequential_matrix_free_step_with_inner_forcing,
};

#[test]
fn every_forced_step_reports_its_final_budget_status() {
    // Matrix-free n = 64, where GMRES stops at the forcing tolerance. Over
    // h = 1/8 .. 1/256 and rtol 1e-4 .. 1e-8 all 1512 steps measured
    // self-consistent; this contract keeps a smaller ladder.
    let (problem, y0) =
        semilinear_advection_diffusion_problem(64, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
    let problem = problem.jvp_only_clone().unwrap();
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 8,
        maxiter: 64,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    };
    for rtol in [1.0e-4, 1.0e-8] {
        let steps = 16;
        let h = 1.0 / steps as f64;
        let mut y = y0.clone();
        let mut work = WorkCounters::default();
        for step in 0..steps {
            let report = sequential_matrix_free_step_with_inner_forcing(
                &problem,
                step as f64 * h,
                &y,
                h,
                &config,
                None,
                1.0e-2 * rtol,
                rtol,
                true,
                &mut work,
            )
            .unwrap();
            assert_eq!(
                report.resolution,
                InnerForcingResolution::SelfConsistent,
                "rtol {rtol:e}, step {step}"
            );
            y = report.step.y_new;
        }
    }
}
