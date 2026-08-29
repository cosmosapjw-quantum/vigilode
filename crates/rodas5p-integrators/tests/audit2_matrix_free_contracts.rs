#![cfg(feature = "audit2-research")]

use rodas5p_core::{
    CoreError, CoreResult, IdentityPreconditioner, LinearOperator, Preconditioner, WorkCounters,
    safe_l2,
};
use rodas5p_integrators::audit2_research::matrix_free::{
    MatrixFreeFailurePhase, run_audit2_matrix_free_correction,
};
use rodas5p_integrators::audit2_research::{
    Audit2OriginalTargetAccuracyDisposition, Audit2ResearchConfig, run_audit2_research_correction,
};
use rodas5p_integrators::{
    OdeProblem, StepContext, StructuredBlockSystem, build_step_context,
    build_step_context_matrix_free, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem,
};
use rodas5p_krylov::GmresConfig;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

fn config() -> GmresConfig {
    GmresConfig {
        rtol: 1e-11,
        atol: 1e-13,
        restart: 32,
        max_arnoldi: 256,
    }
}
fn identity(c: &StepContext<'_>, _: &mut WorkCounters) -> CoreResult<Box<dyn Preconditioner>> {
    Ok(Box::new(IdentityPreconditioner::new(c.problem.dimension)))
}
fn trial(n: usize) -> Vec<Vec<f64>> {
    (0..8)
        .map(|i| {
            (0..n)
                .map(|j| 1e-5 * ((i * n + j + 1) as f64).sin())
                .collect()
        })
        .collect()
}
fn sparse_problem(n: usize, zero: bool) -> OdeProblem {
    OdeProblem::new(
        "matrix-free-stencil",
        n,
        Arc::new(move |_, y, out| {
            for i in 0..n {
                out[i] = if zero {
                    0.0
                } else {
                    -200.0 * y[i]
                        + if i > 0 { 30.0 * y[i - 1] } else { 0.0 }
                        + if i + 1 < n { 40.0 * y[i + 1] } else { 0.0 }
                };
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            for i in 0..n {
                out[i] = if zero {
                    0.0
                } else {
                    -200.0 * v[i]
                        + if i > 0 { 30.0 * v[i - 1] } else { 0.0 }
                        + if i + 1 < n { 40.0 * v[i + 1] } else { 0.0 }
                };
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn matrix_free_correction_reuses_one_setup_and_checks_every_row() {
    let (p, y) = manufactured_vector_problem(4, 100.0, 0.5, 0.1, 0.0).unwrap();
    let mut entry = WorkCounters::default();
    let c = build_step_context_matrix_free(&p, 0.0, &y, 0.01, &mut entry).unwrap();
    assert!(c.shifted.explicit().is_none());
    let calls = AtomicUsize::new(0);
    let r = run_audit2_matrix_free_correction(&c, &trial(4), &config(), |ctx, w| {
        calls.fetch_add(1, Ordering::Relaxed);
        identity(ctx, w)
    });
    assert!(r.completed, "{:?}", r.failure);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(r.work.preconditioner_setup_attempts, 1);
    assert_eq!(r.work.preconditioner_setup_completed, 1);
    assert_eq!(r.work.solve_attempts, 8);
    assert_eq!(r.work.solve_completed, 8);
    assert_eq!(r.rows.len(), 8);
    assert_eq!(r.correction.len(), 8);
    assert_eq!(r.linear_reports.len(), 8);
    assert_eq!(r.work.true_residual_completed, 8);
    assert!(r.rows.iter().all(|v| v.residual_l2 <= v.threshold));
    assert!(r.work.operator.jvp_attempts > 0);
    assert_eq!(r.total_counters().jacobian_builds, 0);
    assert_eq!(r.total_counters().direct_factorizations, 0);
    assert_eq!(
        r.accuracy_disposition,
        Audit2OriginalTargetAccuracyDisposition::BudgetNotSpecified
    );
}

#[test]
fn all_small_coordinates_have_independent_full_target_residuals() {
    for n in [4, 8, 16] {
        for h in [0.001, 0.01, 0.05, 0.1] {
            let (p, y) = manufactured_vector_problem(n, 100.0, 0.5, 0.1, 0.0).unwrap();
            let k = trial(n);
            let mut context_work = WorkCounters::default();
            let c = build_step_context_matrix_free(&p, 0.0, &y, h, &mut context_work).unwrap();
            let explicit =
                build_step_context(&p, 0.0, &y, h, &mut WorkCounters::default()).unwrap();
            let oracle =
                run_audit2_research_correction(&explicit, &k, Audit2ResearchConfig::default());
            let oracle = oracle.completed().unwrap();
            let r = run_audit2_matrix_free_correction(&c, &k, &config(), identity);
            assert!(r.completed, "n={n} h={h}: {:?}", r.failure);
            let diff = safe_l2(
                &r.correction
                    .iter()
                    .flatten()
                    .zip(oracle.correction.iter().flatten())
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
            );
            let denom = safe_l2(
                &oracle
                    .correction
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>(),
            );
            let block = StructuredBlockSystem::new(&explicit);
            let mut oracle_work = WorkCounters::default();
            let original_rhs = block.target_residual(&k, &mut oracle_work).unwrap();
            let original_snapshot = block
                .nonlinear_remainder_snapshot(&k, &mut oracle_work)
                .unwrap();
            let original_matrix = block
                .target_jacobian_matrix(&k, &original_snapshot, &mut oracle_work)
                .unwrap();
            let z = r.correction.iter().flatten().copied().collect::<Vec<_>>();
            let original_image = original_matrix.matvec(&z).unwrap();
            oracle_work.diagnostic_matvecs += 1;
            let original_residual = safe_l2(
                &original_image
                    .iter()
                    .zip(original_rhs.iter().flatten())
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
            );
            let original_rhs_norm =
                safe_l2(&original_rhs.iter().flatten().copied().collect::<Vec<_>>());
            assert!(original_residual <= 1e-8 * original_rhs_norm.max(1.0));
            save(
                &format!("small_n{n}_h{h}"),
                serde_json::json!({"n":n,"h":h,"relative_correction_difference":diff/denom,"original_action_residual_l2":original_residual,"original_rhs_l2":original_rhs_norm,"projected_residual_l2":r.projected_linear_residual_l2,"context_work":context_work,"independent_oracle_work":oracle_work,"candidate":r}),
            );
            println!(
                "MFW_SMALL n={n} h={h} relative_difference={} residual={:?} work={}",
                diff / denom,
                r.projected_linear_residual_l2,
                serde_json::to_string(&r.work).unwrap()
            );
            assert!(
                r.projected_linear_residual_l2.unwrap()
                    <= 1e-8 * r.initial_residual_l2.unwrap().max(1.0)
            );
            assert!(diff / denom < 1e-7);
        }
    }
}

#[test]
fn matrix_free_storage_probes_do_not_assemble_a_jacobian() {
    for n in [32, 128, 512] {
        let p = sparse_problem(n, false);
        let y = vec![1.0; n];
        let c = build_step_context_matrix_free(&p, 0.0, &y, 0.01, &mut WorkCounters::default())
            .unwrap();
        let r = run_audit2_matrix_free_correction(&c, &trial(n), &config(), identity);
        assert!(r.completed, "n={n} {:?}", r.failure);
        assert_eq!(r.total_counters().jacobian_builds, 0);
        assert_eq!(r.total_counters().direct_factorizations, 0);
        assert!(r.workspace_capacity_f64 < 100 * n + 2000);
        save(
            &format!("storage_n{n}"),
            serde_json::json!({"n":n,"workspace_capacity_f64":r.workspace_capacity_f64,"total_counters":r.total_counters(),"work":r.work,"row_checks":r.rows,"matrix_free_input":c.shifted.explicit().is_none(),"completed":r.completed}),
        );
        println!(
            "MFW_STORAGE n={n} workspace={} jvp={} row_checks={}",
            r.workspace_capacity_f64,
            r.total_counters().jvp_vectors,
            r.rows.len()
        );
    }
}

#[test]
fn zero_rhs_has_no_relative_residual_and_no_accuracy_admission() {
    let p = sparse_problem(4, true);
    let c = build_step_context_matrix_free(&p, 0.0, &[0.0; 4], 0.01, &mut WorkCounters::default())
        .unwrap();
    let r = run_audit2_matrix_free_correction(&c, &vec![vec![0.0; 4]; 8], &config(), identity);
    assert!(r.completed, "{:?}", r.failure);
    assert!(r.rows.iter().all(|v| v.relative_residual.is_none()));
    assert_eq!(
        r.accuracy_disposition,
        Audit2OriginalTargetAccuracyDisposition::BudgetNotSpecified
    );
}

#[test]
fn explicit_input_is_rejected_instead_of_disguised_as_matrix_free() {
    let (p, y) = manufactured_vector_problem(4, 100.0, 0.5, 0.1, 0.0).unwrap();
    let c = build_step_context(&p, 0.0, &y, 0.01, &mut WorkCounters::default()).unwrap();
    let r = run_audit2_matrix_free_correction(&c, &trial(4), &config(), identity);
    assert!(!r.completed);
    assert_eq!(
        r.failure.unwrap().phase,
        MatrixFreeFailurePhase::InputValidation
    );
    assert_eq!(r.work.preconditioner_setup_attempts, 0);
}

#[test]
fn invalid_configuration_and_trial_do_not_call_setup() {
    let p = sparse_problem(4, false);
    let c = build_step_context_matrix_free(&p, 0.0, &[1.0; 4], 0.01, &mut WorkCounters::default())
        .unwrap();
    for bad in [
        GmresConfig {
            rtol: f64::NAN,
            ..config()
        },
        GmresConfig {
            restart: 0,
            ..config()
        },
    ] {
        let r = run_audit2_matrix_free_correction(&c, &trial(4), &bad, identity);
        assert!(!r.completed);
        assert_eq!(r.work.preconditioner_setup_attempts, 0);
    }
    for k in [vec![vec![1.0; 3]; 8], vec![vec![f64::NAN; 4]; 8]] {
        let r = run_audit2_matrix_free_correction(&c, &k, &config(), identity);
        assert!(!r.completed);
        assert_eq!(r.work.preconditioner_setup_attempts, 0);
    }
}

#[test]
fn failed_setup_retains_its_attempt_and_reported_work() {
    let p = sparse_problem(4, false);
    let c = build_step_context_matrix_free(&p, 0.0, &[1.0; 4], 0.01, &mut WorkCounters::default())
        .unwrap();
    let r = run_audit2_matrix_free_correction(&c, &trial(4), &config(), |_, w| {
        w.preconditioner_apps += 3;
        Err(CoreError::LinearSolve("injected setup failure".into()))
    });
    assert!(!r.completed);
    assert_eq!(r.work.preconditioner_setup_attempts, 1);
    assert_eq!(r.work.preconditioner_setup_completed, 0);
    assert_eq!(r.setup_counters.preconditioner_apps, 3);
    assert_eq!(
        r.failure.unwrap().phase,
        MatrixFreeFailurePhase::PreconditionerSetup
    );
}

fn save(name: &str, value: serde_json::Value) {
    if let Ok(root) = std::env::var("AUDIT2_MATRIX_FREE_OUTPUT_DIR") {
        use std::io::Write;
        let root = std::path::Path::new(&root);
        std::fs::create_dir_all(root).unwrap();
        let path = root.join(format!("{name}.json"));
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        f.write_all(serde_json::to_string_pretty(&value).unwrap().as_bytes())
            .unwrap();
    }
}
fn scalar(
    armed: Arc<AtomicBool>,
    jvp_calls: Arc<AtomicUsize>,
    fail_call: usize,
    bad_rhs: bool,
    finite_wrong: bool,
) -> OdeProblem {
    let rhs_flag = armed.clone();
    OdeProblem::new(
        "scalar-failure-probe",
        1,
        Arc::new(move |_, y, out| {
            if bad_rhs && rhs_flag.load(Ordering::Relaxed) {
                return Err(CoreError::InvalidInput("injected RHS failure".into()));
            }
            out[0] = -2.0 * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            if armed.load(Ordering::Relaxed) {
                let count = jvp_calls.fetch_add(1, Ordering::Relaxed) + 1;
                if count == fail_call {
                    if finite_wrong {
                        out[0] = -20.0 * v[0];
                        return Ok(());
                    }
                    return Err(CoreError::LinearSolve("injected JVP failure".into()));
                }
            }
            out[0] = -2.0 * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn failed_jvp_is_counted_before_the_first_solve_returns() {
    let armed = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let p = scalar(armed.clone(), calls.clone(), 1, false, false);
    let c =
        build_step_context_matrix_free(&p, 0.0, &[1.0], 0.1, &mut WorkCounters::default()).unwrap();
    let r = run_audit2_matrix_free_correction(&c, &vec![vec![0.0]; 8], &config(), |ctx, w| {
        armed.store(true, Ordering::Relaxed);
        identity(ctx, w)
    });
    assert!(!r.completed);
    assert_eq!(r.work.solve_attempts, 1);
    assert_eq!(r.work.solve_completed, 0);
    assert_eq!(r.work.operator.jvp_attempts, 1);
    assert_eq!(r.work.operator.jvp_completed, 0);
    assert!(r.correction.is_empty());
    assert!(!r.failed_kernel_iterate_available);
    assert!(r.inherited_work_complete);
    save("failed_jvp", serde_json::to_value(&r).unwrap());
}

struct FailingPc {
    n: usize,
    calls: AtomicUsize,
    fail_at: usize,
    zero: bool,
    nan: bool,
}
impl Preconditioner for FailingPc {
    fn dimension(&self) -> usize {
        self.n
    }
    fn apply(&self, x: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let call = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
        if call == self.fail_at {
            return Err(CoreError::LinearSolve("injected PC failure".into()));
        }
        for (a, b) in out.iter_mut().zip(x) {
            *a = if self.nan {
                f64::NAN
            } else if self.zero {
                0.0
            } else {
                *b
            };
        }
        Ok(())
    }
}

#[test]
fn late_preconditioner_failure_preserves_the_first_verified_row() {
    let p = scalar(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicUsize::new(0)),
        usize::MAX,
        false,
        false,
    );
    let c =
        build_step_context_matrix_free(&p, 0.0, &[1.0], 0.1, &mut WorkCounters::default()).unwrap();
    let r = run_audit2_matrix_free_correction(&c, &vec![vec![0.0]; 8], &config(), |_, _| {
        Ok(Box::new(FailingPc {
            n: 1,
            calls: AtomicUsize::new(0),
            fail_at: 3,
            zero: false,
            nan: false,
        }))
    });
    assert!(!r.completed);
    assert_eq!(r.work.solve_attempts, 2);
    assert_eq!(r.work.solve_completed, 1);
    assert_eq!(r.correction.len(), 1);
    assert_eq!(r.linear_reports.len(), 1);
    assert_eq!(r.work.operator.preconditioner_attempts, 3);
    assert_eq!(r.work.operator.preconditioner_completed, 2);
    assert_eq!(r.total_counters().preconditioner_apps, 3);
    save("late_pc_failure", serde_json::to_value(&r).unwrap());
}

#[test]
fn zero_or_nan_preconditioner_cannot_certify_a_nonzero_true_residual() {
    let p = sparse_problem(4, false);
    let c = build_step_context_matrix_free(&p, 0.0, &[1.0; 4], 0.01, &mut WorkCounters::default())
        .unwrap();
    for nan in [false, true] {
        let r = run_audit2_matrix_free_correction(&c, &trial(4), &config(), |_, _| {
            Ok(Box::new(FailingPc {
                n: 4,
                calls: AtomicUsize::new(0),
                fail_at: usize::MAX,
                zero: !nan,
                nan,
            }))
        });
        assert!(!r.completed);
        assert!(r.correction.is_empty());
        assert!(r.work.operator.preconditioner_attempts > 0);
    }
}

#[test]
fn additional_true_residual_check_retains_a_report_before_rejecting() {
    let armed = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let p = scalar(armed.clone(), calls, 4, false, true);
    let c =
        build_step_context_matrix_free(&p, 0.0, &[1.0], 0.1, &mut WorkCounters::default()).unwrap();
    let r = run_audit2_matrix_free_correction(&c, &vec![vec![0.0]; 8], &config(), |ctx, w| {
        armed.store(true, Ordering::Relaxed);
        identity(ctx, w)
    });
    assert!(!r.completed);
    assert_eq!(
        r.failure.as_ref().unwrap().phase,
        MatrixFreeFailurePhase::TrueResidual
    );
    assert_eq!(r.linear_reports.len(), 1);
    assert!(r.correction.is_empty());
    assert_eq!(r.rows.len(), 1);
    assert!(r.rows[0].residual_l2 > r.rows[0].threshold);
    save(
        "true_residual_counterexample",
        serde_json::to_value(&r).unwrap(),
    );
}

#[test]
fn exhausted_inner_solve_preserves_work_without_inventing_an_iterate() {
    let p = sparse_problem(4, false);
    let c = build_step_context_matrix_free(&p, 0.0, &[1.0; 4], 0.01, &mut WorkCounters::default())
        .unwrap();
    let r = run_audit2_matrix_free_correction(
        &c,
        &trial(4),
        &GmresConfig {
            max_arnoldi: 1,
            restart: 1,
            ..config()
        },
        identity,
    );
    assert!(!r.completed);
    assert!(r.work.operator.w_attempts > 0);
    assert!(r.workspace_capacity_f64 > 0);
    assert_eq!(
        r.failure.as_ref().unwrap().phase,
        MatrixFreeFailurePhase::LinearSolve
    );
    assert!(!r.failed_kernel_iterate_available);
    assert!(r.linear_reports.is_empty());
    save("exhausted", serde_json::to_value(&r).unwrap());
}

#[test]
fn late_nonlinear_diagnostic_failure_keeps_all_linear_results() {
    let armed = Arc::new(AtomicBool::new(false));
    let p = scalar(
        armed.clone(),
        Arc::new(AtomicUsize::new(0)),
        usize::MAX,
        true,
        false,
    );
    let c =
        build_step_context_matrix_free(&p, 0.0, &[1.0], 0.1, &mut WorkCounters::default()).unwrap();
    let r = run_audit2_matrix_free_correction(&c, &vec![vec![0.0]; 8], &config(), |ctx, w| {
        armed.store(true, Ordering::Relaxed);
        identity(ctx, w)
    });
    assert!(!r.completed);
    assert_eq!(
        r.failure.as_ref().unwrap().phase,
        MatrixFreeFailurePhase::NonlinearDiagnostic
    );
    assert_eq!(r.correction.len(), 8);
    assert_eq!(r.rows.len(), 8);
    assert_eq!(r.linear_reports.len(), 8);
    assert!(r.projected_linear_residual_l2.is_some());
    assert!(r.nonlinear_residual_after_l2.is_none());
    assert!(!r.inherited_work_complete);
    save("late_nonlinear_failure", serde_json::to_value(&r).unwrap());
}

#[test]
fn inherited_preparation_failure_is_explicitly_incomplete_not_zero_work() {
    let armed = Arc::new(AtomicBool::new(false));
    let p = scalar(
        armed.clone(),
        Arc::new(AtomicUsize::new(0)),
        usize::MAX,
        true,
        false,
    );
    let c =
        build_step_context_matrix_free(&p, 0.0, &[1.0], 0.1, &mut WorkCounters::default()).unwrap();
    armed.store(true, Ordering::Relaxed);
    let r = run_audit2_matrix_free_correction(&c, &vec![vec![0.0]; 8], &config(), identity);
    assert!(!r.completed);
    assert!(!r.inherited_work_complete);
    assert_eq!(r.work.preparation_attempts, 1);
    assert_eq!(r.work.preparation_completed, 0);
    assert_eq!(r.work.preconditioner_setup_attempts, 0);
}

#[test]
fn nonidentity_mass_uses_jvp_w_but_retains_the_dense_mass_caveat() {
    let (p, y, _, _) = manufactured_mass_nonlinear_problem(1000.0, 50.0, 20.0, 0.0).unwrap();
    let c =
        build_step_context_matrix_free(&p, 0.0, &y, 0.01, &mut WorkCounters::default()).unwrap();
    let k = trial(2);
    let r = run_audit2_matrix_free_correction(&c, &k, &config(), identity);
    assert!(r.completed, "{:?}", r.failure);
    assert!(r.work.operator.mass_attempts > 0);
    assert_eq!(
        r.work.operator.mass_attempts,
        r.work.operator.mass_completed
    );
    let explicit = build_step_context(&p, 0.0, &y, 0.01, &mut WorkCounters::default()).unwrap();
    let block = StructuredBlockSystem::new(&explicit);
    let mut work = WorkCounters::default();
    let rhs = block.target_residual(&k, &mut work).unwrap();
    let snapshot = block.nonlinear_remainder_snapshot(&k, &mut work).unwrap();
    let matrix = block
        .target_jacobian_matrix(&k, &snapshot, &mut work)
        .unwrap();
    let image = matrix
        .matvec(&r.correction.iter().flatten().copied().collect::<Vec<_>>())
        .unwrap();
    work.diagnostic_matvecs += 1;
    let residual = safe_l2(
        &image
            .iter()
            .zip(rhs.iter().flatten())
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    let rhs_norm = safe_l2(&rhs.iter().flatten().copied().collect::<Vec<_>>());
    assert!(residual <= 1e-8 * rhs_norm.max(1.0));
    save(
        "mass_nonnormal",
        serde_json::json!({"original_action_residual_l2":residual,"original_rhs_l2":rhs_norm,"independent_oracle_work":work,"candidate":r}),
    );
}

#[test]
fn no_setup_can_be_reused_across_a_changed_context() {
    let p = sparse_problem(4, false);
    let count = AtomicUsize::new(0);
    for (h, y) in [(0.01, vec![1.0; 4]), (0.02, vec![2.0; 4])] {
        let c =
            build_step_context_matrix_free(&p, 0.0, &y, h, &mut WorkCounters::default()).unwrap();
        let r = run_audit2_matrix_free_correction(&c, &trial(4), &config(), |ctx, w| {
            count.fetch_add(1, Ordering::Relaxed);
            identity(ctx, w)
        });
        assert!(r.completed);
    }
    assert_eq!(count.load(Ordering::Relaxed), 2);
}

struct AnalyticDiagonalPc {
    inverse: Vec<f64>,
}
impl Preconditioner for AnalyticDiagonalPc {
    fn dimension(&self) -> usize {
        self.inverse.len()
    }
    fn apply(&self, x: &[f64], out: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.inverse.len() || out.len() != x.len() {
            return Err(CoreError::Dimension("diagonal PC shape".into()));
        }
        for ((v, &a), &b) in out.iter_mut().zip(&self.inverse).zip(x) {
            *v = a * b;
        }
        Ok(())
    }
}

#[test]
fn nontrivial_analytic_preconditioner_is_built_once_without_dense_assembly() {
    let n = 128;
    let rates: Arc<Vec<f64>> = Arc::new((0..n).map(|i| 1.0 + 100.0 * (i as f64).powi(2)).collect());
    let rr = rates.clone();
    let jr = rates.clone();
    let p = OdeProblem::new(
        "diagonal-preconditioned-stiff",
        n,
        Arc::new(move |_, y, out| {
            for i in 0..n {
                out[i] = -rr[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            for i in 0..n {
                out[i] = -jr[i] * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let mut entry = WorkCounters::default();
    let c = build_step_context_matrix_free(&p, 0.0, &vec![1.0; n], 0.01, &mut entry).unwrap();
    let setup_calls = AtomicUsize::new(0);
    let r = run_audit2_matrix_free_correction(&c, &trial(n), &config(), |ctx, _| {
        setup_calls.fetch_add(1, Ordering::Relaxed);
        let inverse = rates
            .iter()
            .map(|a| 1.0 / (1.0 + ctx.h * ctx.coeffs.gamma * a))
            .collect();
        Ok(Box::new(AnalyticDiagonalPc { inverse }))
    });
    assert!(r.completed, "{:?}", r.failure);
    assert_eq!(setup_calls.load(Ordering::Relaxed), 1);
    assert_eq!(r.work.solve_completed, 8);
    assert_eq!(r.total_counters().direct_factorizations, 0);
    assert_eq!(r.total_counters().jacobian_builds, 0);
    assert!(r.linear_reports.iter().all(|v| v.iterations <= 2));
    assert!(
        r.rows
            .iter()
            .all(|v| v.workspace_capacity_f64 == r.rows[0].workspace_capacity_f64)
    );
    save(
        "analytic_diagonal_pc",
        serde_json::json!({"n":n,"preconditioner_retained_f64":n,"setup_calls":1,"context_work":entry,"candidate":r}),
    );
}

#[test]
fn observed_jvp_callbacks_equal_the_available_success_ledger() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let p = OdeProblem::new(
        "counted-jvp",
        1,
        Arc::new(|_, y, out| {
            out[0] = -2.0 * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, out| {
            counted.fetch_add(1, Ordering::Relaxed);
            out[0] = -2.0 * v[0];
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let c =
        build_step_context_matrix_free(&p, 0.0, &[1.0], 0.1, &mut WorkCounters::default()).unwrap();
    let before = calls.load(Ordering::Relaxed);
    let r = run_audit2_matrix_free_correction(&c, &vec![vec![0.0]; 8], &config(), identity);
    assert!(r.completed);
    assert_eq!(
        r.total_counters().jvp_vectors as usize,
        calls.load(Ordering::Relaxed) - before
    );
}
