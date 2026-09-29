//! Matrix-free common-W correction against the sequential baseline on the
//! same RODAS5P target and the same h (external re-audit RA-06).
//! Pre-registration: research/common_w_accelerator_parity_20260929/.
//!
//!     cargo run --release -p rodas5p-integrators --features audit2-research \
//!         --example common_w_parity
//!
//! Every step of one adaptive sequential run is replayed from the baseline
//! state by three arms: the sequential baseline, predictor + 1 correction,
//! and predictor + 2 corrections with a charged fallback. Acceptance in arm
//! 3 is causal (the target residual); the baseline stages are used only for
//! the parity columns.

use std::sync::Arc;

use rodas5p_core::{
    InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters, error_scale,
    safe_l2, wrms,
};
use rodas5p_integrators::{
    Audit2MatrixFreeCommonWConfig, Audit2MatrixFreeCorrectionOutcome, IntegrationMethod,
    OdeProblem, StepContext, StructuredBlockSystem, build_step_context_matrix_free, finish_step,
    integrate_adaptive, rodas5p_dense_output, run_audit2_matrix_free_common_w_correction,
    semilinear_advection_diffusion_problem, sequential_stages,
};

const ACCEPT_RELATIVE_RESIDUAL: f64 = 1.0e-9;

fn gmres() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-11,
        atol: 1.0e-13,
        restart: 24,
        maxiter: 192,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Zero,
        ..LinearSolverConfig::default()
    }
}

/// y1' = -y1 + y2^2, y2' = -10 y2 + y1 y3, y3' = -100 (y3 - y1).
fn dissipative() -> (OdeProblem, Vec<f64>) {
    let problem = OdeProblem::new(
        "dissipative-noncommuting-3",
        3,
        Arc::new(|_, y: &[f64], out: &mut [f64]| {
            out[0] = -y[0] + y[1] * y[1];
            out[1] = -10.0 * y[1] + y[0] * y[2];
            out[2] = -100.0 * (y[2] - y[0]);
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|_, y: &[f64], v: &[f64], out: &mut [f64]| {
            out[0] = -v[0] + 2.0 * y[1] * v[1];
            out[1] = y[2] * v[0] - 10.0 * v[1] + y[0] * v[2];
            out[2] = 100.0 * v[0] - 100.0 * v[2];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    (problem, vec![1.0, 0.5, 0.0])
}

#[derive(Default)]
struct Arm {
    work: WorkCounters,
    solves: u64,
    corrections: u64,
    fallbacks: u64,
    fallback_work: WorkCounters,
    failures: u64,
    max_endpoint: f64,
    max_embedded: f64,
    max_dense: f64,
    max_relative_residual: f64,
}

impl Arm {
    fn print(&self, name: &str) {
        println!(
            "  {name:16} rhs {:6} jvp {:6} W-apply {:6} P-apply {:6} W-solves {:5} corrections {:4} \
             fallbacks {:3} (+rhs {} +W-apply {}) failures {} | max diff endpoint {:.2e} \
             embedded {:.2e} dense {:.2e} | max rel residual {:.2e}",
            self.work.rhs_evaluations,
            self.work.jvp_vectors,
            self.work.linear_matvecs,
            self.work.preconditioner_apps,
            self.solves,
            self.corrections,
            self.fallbacks,
            self.fallback_work.rhs_evaluations,
            self.fallback_work.linear_matvecs,
            self.failures,
            self.max_endpoint,
            self.max_embedded,
            self.max_dense,
            self.max_relative_residual,
        );
    }
}

fn relative_residual(
    context: &StepContext<'_>,
    stages: &[Vec<f64>],
    counters: &mut WorkCounters,
) -> f64 {
    let block = StructuredBlockSystem::new(context);
    let applied = block.apply(stages, counters).unwrap();
    let snapshot = block
        .nonlinear_remainder_snapshot(stages, counters)
        .unwrap();
    let residual = applied
        .iter()
        .zip(&snapshot.rhs)
        .flat_map(|(lhs, rhs)| lhs.iter().zip(rhs).map(|(a, b)| a - b))
        .collect::<Vec<_>>();
    let rhs = snapshot.rhs.iter().flatten().copied().collect::<Vec<_>>();
    safe_l2(&residual) / safe_l2(&rhs).max(f64::MIN_POSITIVE)
}

/// Apply `count` corrections; returns the stages, or None on a failed
/// correction. All correction work is charged to `arm`.
fn corrected(
    context: &StepContext<'_>,
    mut stages: Vec<Vec<f64>>,
    count: usize,
    arm: &mut Arm,
) -> Option<Vec<Vec<f64>>> {
    for _ in 0..count {
        let outcome = run_audit2_matrix_free_common_w_correction(
            context,
            &stages,
            Audit2MatrixFreeCommonWConfig::default(),
        );
        arm.corrections += 1;
        let (work, correction) = match outcome {
            Audit2MatrixFreeCorrectionOutcome::Completed(success) => {
                (success.work, Some(success.correction))
            }
            Audit2MatrixFreeCorrectionOutcome::Failed(failure) => (failure.work, None),
        };
        arm.work.accumulate(work.preparation_counters);
        arm.work.accumulate(work.coupling_counters);
        if let Some(session) = &work.session {
            arm.work.accumulate(session.counters);
            arm.solves += session.solve_attempts;
        }
        let correction = correction?;
        for (stage, delta) in stages.iter_mut().zip(&correction) {
            for (value, d) in stage.iter_mut().zip(delta) {
                *value -= d;
            }
        }
    }
    Some(stages)
}

fn parity(
    context: &StepContext<'_>,
    stages: &[Vec<f64>],
    baseline: &[Vec<f64>],
    atol: f64,
    rtol: f64,
    arm: &mut Arm,
) {
    let scratch = WorkCounters::default();
    let finish = |stages: &[Vec<f64>]| {
        finish_step(
            context,
            stages.to_vec(),
            atol,
            rtol,
            "parity".into(),
            Some(true),
            false,
            None,
            scratch,
            &scratch,
        )
        .unwrap()
    };
    let (step, reference) = (finish(stages), finish(baseline));
    let scale = error_scale(&context.y, &reference.y_new, &[atol], rtol).unwrap();
    let difference = |a: &[f64], b: &[f64]| {
        wrms(
            &a.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<_>>(),
            &scale,
        )
        .unwrap()
    };
    arm.max_endpoint = arm
        .max_endpoint
        .max(difference(&step.y_new, &reference.y_new));
    arm.max_embedded = arm
        .max_embedded
        .max(difference(&step.error_vector, &reference.error_vector));
    let dense = rodas5p_dense_output(&step, 0.5).unwrap();
    let dense_reference = rodas5p_dense_output(&reference, 0.5).unwrap();
    arm.max_dense = arm.max_dense.max(difference(&dense, &dense_reference));
}

fn run_case(name: &str, problem: &OdeProblem, y0: &[f64], t_end: f64, rtol: f64) {
    let atol = 1.0e-2 * rtol;
    let mesh = integrate_adaptive(
        problem,
        (0.0, t_end),
        y0,
        t_end / 100.0,
        IntegrationMethod::Sequential,
        Some(&gmres()),
        None,
        atol,
        rtol,
        100_000,
        t_end,
    )
    .unwrap();
    assert!(mesh.success, "{}", mesh.message);
    let mut baseline_arm = Arm::default();
    let mut one = Arm::default();
    let mut two = Arm::default();
    let mut previous: Option<(Vec<Vec<f64>>, f64)> = None;
    for (index, &h) in mesh.step_sizes.iter().enumerate() {
        let (t, y) = (mesh.t[index], &mesh.y[index]);
        let mut setup = WorkCounters::default();
        let context = build_step_context_matrix_free(problem, t, y, h, &mut setup).unwrap();
        let mut baseline_work = WorkCounters::default();
        let baseline = sequential_stages(&context, &gmres(), None, &mut baseline_work)
            .unwrap()
            .stages;
        baseline_arm.work.accumulate(baseline_work);
        baseline_arm.solves += baseline.len() as u64;

        let predictor = match &previous {
            Some((stages, h_previous)) => stages
                .iter()
                .map(|row| row.iter().map(|v| v * h / h_previous).collect())
                .collect(),
            None => vec![vec![0.0; problem.dimension]; baseline.len()],
        };
        // Arm 2: predictor + 1 correction, parity only.
        match corrected(&context, predictor.clone(), 1, &mut one) {
            Some(stages) => {
                let r = relative_residual(&context, &stages, &mut one.work);
                one.max_relative_residual = one.max_relative_residual.max(r);
                parity(&context, &stages, &baseline, atol, rtol, &mut one);
            }
            None => one.failures += 1,
        }
        // Arm 3: predictor + 2 corrections, causal acceptance, fallback.
        let candidate = corrected(&context, predictor, 2, &mut two);
        let accepted = candidate.and_then(|stages| {
            let r = relative_residual(&context, &stages, &mut two.work);
            two.max_relative_residual = two.max_relative_residual.max(r);
            (r <= ACCEPT_RELATIVE_RESIDUAL).then_some(stages)
        });
        let stages = match accepted {
            Some(stages) => stages,
            None => {
                let mut fallback = WorkCounters::default();
                let stages = sequential_stages(&context, &gmres(), None, &mut fallback)
                    .unwrap()
                    .stages;
                two.fallbacks += 1;
                two.solves += stages.len() as u64;
                two.work.accumulate(fallback);
                two.fallback_work.accumulate(fallback);
                stages
            }
        };
        parity(&context, &stages, &baseline, atol, rtol, &mut two);
        previous = Some((baseline, h));
    }
    println!(
        "{name} rtol {rtol:e}: {} steps (baseline mesh)",
        mesh.step_sizes.len()
    );
    baseline_arm.print("sequential");
    one.print("pred+1corr");
    two.print("pred+2corr+fb");
}

fn main() {
    let (dissipative, y0) = dissipative();
    let (semilinear, s0) =
        semilinear_advection_diffusion_problem(32, 0.05, 0.5, -1.0, 0.5, 0.0).unwrap();
    let semilinear = semilinear.jvp_only_clone().unwrap();
    for rtol in [1.0e-4, 1.0e-6] {
        run_case("dissipative-3", &dissipative, &y0, 2.0, rtol);
        run_case("semilinear-32", &semilinear, &s0, 1.0, rtol);
    }
}
