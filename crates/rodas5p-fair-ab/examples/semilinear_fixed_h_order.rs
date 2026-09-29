//! Fixed-h convergence study on the semilinear advection-diffusion family
//! (audit F-033 fix design: settles R_inner ~ 1 rows).
//!
//!     cargo run --release -p rodas5p-fair-ab --example semilinear_fixed_h_order
//!
//! RODAS5P is run at fixed step sizes span/N with every stage solved to a
//! GMRES tolerance of 1e-13 (no forcing rule, no error control), and the
//! endpoint error of each integration segment is measured against the
//! family's manufactured exact solution in case-tolerance WRMS units. The
//! observed order is log2 of successive error ratios.
//!
//! Then, for every step the adaptive dense campaign arm accepted, the true
//! local error of that step size taken from the exact state is compared with
//! the embedded estimate the controller accepted.

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, error_scale, wrms,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, IntegrationMethod, OutputSamplingPlan, OutputSchedule,
    ScientificCorpusV2, ScientificFamily, integrate_fixed,
    integrate_sequential_matrix_free_adaptive_dense_observed,
    sequential_matrix_free_step_with_inner_forcing,
};

fn main() {
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-13,
        atol: 1.0e-15,
        restart: 64,
        maxiter: 512,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    };
    for spec in ScientificCorpusV2::calibration_specs()
        .into_iter()
        .filter(|spec| {
            spec.family == ScientificFamily::SemilinearAdvectionDiffusionRamped
                && spec.dimension == 96
        })
    {
        let case = spec.build().unwrap();
        println!("{} ({} segments)", spec.id, case.integration_segments.len());
        let mut previous: Option<f64> = None;
        for steps_per_segment in [25_usize, 50, 100, 200, 400, 800] {
            let mut state = case.y0.clone();
            let mut worst = 0.0_f64;
            let mut ok = true;
            for segment in &case.integration_segments {
                let (t0, t1) = segment.t_span;
                let h = (t1 - t0) / steps_per_segment as f64;
                let result = integrate_fixed(
                    &segment.problem,
                    segment.t_span,
                    &state,
                    h,
                    IntegrationMethod::Sequential,
                    Some(&linear),
                    None,
                    spec.atol,
                    spec.rtol,
                );
                let Ok(result) = result.map_err(|e| eprintln!("  N={steps_per_segment}: {e}"))
                else {
                    ok = false;
                    break;
                };
                state = result.y.last().unwrap().clone();
                let exact = case.problem.exact(t1).expect("manufactured exact state");
                let scale = error_scale(&exact, &exact, &[spec.atol], spec.rtol).unwrap();
                let difference = state
                    .iter()
                    .zip(&exact)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>();
                worst = worst.max(wrms(&difference, &scale).unwrap());
            }
            if !ok {
                continue;
            }
            let order = previous.map(|p| (p / worst).log2());
            println!(
                "  N/segment={steps_per_segment:4}  max endpoint error = {worst:.4e} tol units  observed order = {}",
                order.map_or("-".into(), |o| format!("{o:.2}"))
            );
            previous = Some(worst);
        }

        // Per accepted step of the adaptive dense arm (campaign controller):
        // the embedded estimate it accepted, and the true local error of the
        // same step size taken from the exact state with the same forcing.
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
        };
        let campaign_linear = LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: 1.0e-10,
            atol: 1.0e-12,
            restart: 32,
            maxiter: 256,
            preconditioner: PreconditionerKind::None,
            x0_strategy: InitialGuess::Previous,
            ..LinearSolverConfig::default()
        };
        let problem = segment.problem.jvp_only_clone().unwrap();
        let run = integrate_sequential_matrix_free_adaptive_dense_observed(
            &problem,
            segment.t_span,
            &case.y0,
            &campaign_linear,
            &adaptive,
            &OutputSamplingPlan::dense(OutputSchedule::new(spec.output_times.clone()).unwrap()),
        )
        .unwrap();
        let diagnostics = &run.diagnostics;
        let mut t = segment.t_span.0;
        let mut accepted = diagnostics.accepted_step_sizes.iter();
        println!(
            "  adaptive accepted steps: {}",
            diagnostics.accepted_macro_steps
        );
        println!("  step    t_k        h          estimate   true_local  true/estimate");
        let mut index = 0;
        let (mut worst_local, mut sum_local, mut worst_ratio) = (0.0_f64, 0.0_f64, 0.0_f64);
        for (error, failure) in diagnostics
            .error_norms
            .iter()
            .zip(&diagnostics.failure_kinds)
        {
            if failure.is_some() {
                continue;
            }
            let h = *accepted.next().unwrap();
            let y_k = case.problem.exact(t).unwrap();
            let y_next = case.problem.exact(t + h).unwrap();
            let mut counters = rodas5p_core::WorkCounters::default();
            let step = sequential_matrix_free_step_with_inner_forcing(
                &problem,
                t,
                &y_k,
                h,
                &campaign_linear,
                None,
                spec.atol,
                spec.rtol,
                true,
                &mut counters,
            )
            .unwrap()
            .step;
            let scale = error_scale(&y_k, &step.y_new, &[spec.atol], spec.rtol).unwrap();
            let difference = step
                .y_new
                .iter()
                .zip(&y_next)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>();
            let true_local = wrms(&difference, &scale).unwrap();
            println!(
                "  {index:4}  {t:9.3e}  {h:9.3e}  {error:9.3e}  {true_local:9.3e}  {:9.2e}",
                true_local / error.max(f64::MIN_POSITIVE)
            );
            t += h;
            index += 1;
            worst_local = worst_local.max(true_local);
            sum_local += true_local;
            worst_ratio = worst_ratio.max(true_local / error.max(f64::MIN_POSITIVE));
        }
        println!(
            "  summary: max true local = {worst_local:.3e}, sum of true local = {sum_local:.3e}, max true/estimate = {worst_ratio:.2e} (tol units)"
        );
    }
}
