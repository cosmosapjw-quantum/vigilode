use std::{sync::Arc, time::Instant};

use rodas5p_core::{
    ApplyCategory, CoreError, IdentityPreconditioner, JacobiPreconditioner, LinearOperator,
    Preconditioner, WorkCounters, apply_counted, safe_l2,
};
use rodas5p_krylov::{
    GcrodrConfig, GmresConfig, LgmresConfig, solve_gcrodr_with_workspace,
    solve_gmres_with_workspace, solve_lgmres_with_workspace,
};

use crate::{
    BUDGET_EXHAUSTED_MARKER, BudgetedOperator, FairError, FairResult, FairSolveConfig,
    FairSolveResult, LinearSystemCase, PreconditionerKind, ResidualCertificate, SolveStatus,
    SolverKind, SolverSession, StableDenseOperator, TimingLedger, WorkLedger,
    relative_solution_error,
};

fn failure_certificate(config: &FairSolveConfig, rhs: &[f64]) -> ResidualCertificate {
    ResidualCertificate {
        passed: false,
        residual_norm: f64::INFINITY,
        relative_residual: f64::INFINITY,
        threshold: config.atol.max(config.rtol * safe_l2(rhs)),
    }
}

fn classify_error(error: &CoreError) -> SolveStatus {
    match error {
        CoreError::LinearSolve(message) if message.contains(BUDGET_EXHAUSTED_MARKER) => {
            SolveStatus::BudgetExhausted
        }
        CoreError::NonFinite(_) => SolveStatus::NumericalFailure,
        _ => SolveStatus::Failed,
    }
}

pub fn solve_case(
    case: &LinearSystemCase,
    config: &FairSolveConfig,
    session: Option<&mut SolverSession>,
    initial_guess: Option<&[f64]>,
) -> FairResult<FairSolveResult> {
    config.validate()?;
    if let Some(existing) = session.as_ref()
        && existing.solver != config.solver
    {
        return Err(FairError::Invalid(
            "session solver does not match fair solve configuration".into(),
        ));
    }
    if let Some(x0) = initial_guess
        && (x0.len() != case.dimension() || !x0.iter().all(|value| value.is_finite()))
    {
        return Err(FairError::Invalid("invalid explicit initial guess".into()));
    }

    let total_start = Instant::now();
    let stable: Arc<dyn LinearOperator> = Arc::new(StableDenseOperator::new(
        case.matrix.clone(),
        &case.operator_id,
    )?);
    solve_case_with_operator(case, config, session, initial_guess, stable, total_start)
}

fn solve_case_with_operator(
    case: &LinearSystemCase,
    config: &FairSolveConfig,
    session: Option<&mut SolverSession>,
    initial_guess: Option<&[f64]>,
    stable: Arc<dyn LinearOperator>,
    total_start: Instant,
) -> FairResult<FairSolveResult> {
    let operator = BudgetedOperator::new(stable, config.hard_operator_budget);
    let budget = Arc::clone(operator.budget());

    let setup_start = Instant::now();
    let preconditioner: Box<dyn Preconditioner> = match config.preconditioner {
        PreconditionerKind::None => Box::new(IdentityPreconditioner::new(case.dimension())),
        PreconditionerKind::Jacobi => Box::new(JacobiPreconditioner::from_matrix(&case.matrix)?),
    };
    let setup_seconds = setup_start.elapsed().as_secs_f64();

    let mut owned_session = SolverSession::new(config.solver);
    let active = session.unwrap_or(&mut owned_session);
    let SolverSession {
        previous_solution,
        lgmres,
        gcrodr,
        gmres_workspace,
        lgmres_workspace,
        gcrodr_workspace,
        certificate_output,
        certificate_residual,
        ..
    } = active;
    let x0 = initial_guess.or(previous_solution.as_deref());

    let mut counters = WorkCounters::default();
    let solve_start = Instant::now();
    let solve_result = match config.solver {
        SolverKind::Gmres => solve_gmres_with_workspace(
            &operator,
            preconditioner.as_ref(),
            &case.rhs,
            x0,
            &GmresConfig {
                restart: config.restart,
                max_arnoldi: config.hard_operator_budget as usize,
                rtol: config.rtol,
                atol: config.atol,
            },
            gmres_workspace,
            &mut counters,
        ),
        SolverKind::Lgmres => solve_lgmres_with_workspace(
            &operator,
            preconditioner.as_ref(),
            &case.rhs,
            x0,
            &LgmresConfig {
                inner_m: config.restart.saturating_sub(config.recycle_dim).max(1),
                max_outer: (config.hard_operator_budget as usize / config.restart).max(1) + 1,
                outer_k: config.recycle_dim,
                rtol: config.rtol,
                atol: config.atol,
            },
            lgmres,
            lgmres_workspace,
            &mut counters,
        ),
        SolverKind::Gcrodr => solve_gcrodr_with_workspace(
            &operator,
            preconditioner.as_ref(),
            &case.rhs,
            x0,
            &GcrodrConfig {
                restart: config.restart,
                max_arnoldi: config.hard_operator_budget as usize,
                recycle_dim: config.recycle_dim,
                rank_tol: 1e-12,
                rtol: config.rtol,
                atol: config.atol,
            },
            gcrodr,
            gcrodr_workspace,
            &mut counters,
        ),
    };
    let solve_seconds = solve_start.elapsed().as_secs_f64();

    let mut ledger = WorkLedger::from_counters(counters);
    ledger.preconditioner_setup = u64::from(config.preconditioner != PreconditionerKind::None);
    let mut timing = TimingLedger {
        setup_seconds,
        solve_seconds,
        residual_seconds: 0.0,
        total_seconds: 0.0,
    };

    let (status, solution, certificate, iterations, message) = match solve_result {
        Ok(report) => {
            let residual_start = Instant::now();
            certificate_output.resize(case.dimension(), 0.0);
            let certificate_result = apply_counted(
                &operator,
                &report.x,
                certificate_output,
                &mut counters,
                ApplyCategory::Diagnostic,
            );
            timing.residual_seconds = residual_start.elapsed().as_secs_f64();
            ledger = WorkLedger::from_counters(counters);
            ledger.preconditioner_setup =
                u64::from(config.preconditioner != PreconditionerKind::None);
            match certificate_result {
                Ok(()) => {
                    certificate_residual.resize(case.dimension(), 0.0);
                    for index in 0..case.dimension() {
                        certificate_residual[index] = case.rhs[index] - certificate_output[index];
                    }
                    let residual_norm = safe_l2(certificate_residual);
                    let rhs_norm = safe_l2(&case.rhs);
                    let threshold = config.atol.max(config.rtol * rhs_norm);
                    let certificate = ResidualCertificate {
                        passed: residual_norm.is_finite() && residual_norm <= threshold,
                        residual_norm,
                        relative_residual: residual_norm / rhs_norm.max(f64::MIN_POSITIVE),
                        threshold,
                    };
                    let status = if certificate.passed {
                        SolveStatus::Converged
                    } else {
                        SolveStatus::Failed
                    };
                    (
                        status,
                        report.x,
                        certificate,
                        report.iterations,
                        if status == SolveStatus::Converged {
                            String::new()
                        } else {
                            "external true-residual certificate failed".into()
                        },
                    )
                }
                Err(error) => (
                    classify_error(&error),
                    report.x,
                    failure_certificate(config, &case.rhs),
                    report.iterations,
                    error.to_string(),
                ),
            }
        }
        Err(error) => (
            classify_error(&error),
            vec![0.0; case.dimension()],
            failure_certificate(config, &case.rhs),
            counters.linear_iterations,
            error.to_string(),
        ),
    };

    if status == SolveStatus::BudgetExhausted {
        ledger.budget_exhaustions = 1;
    }
    // Solver categories count requested applications, including refusals before
    // the inner callback. Admission includes callbacks that return an error.
    debug_assert_eq!(ledger.operator_total(), budget.used() + budget.refused());
    timing.total_seconds = total_start.elapsed().as_secs_f64();
    let relative_error = if solution.iter().all(|value| value.is_finite()) {
        relative_solution_error(&solution, &case.oracle_solution)
    } else {
        f64::INFINITY
    };

    if status == SolveStatus::Converged {
        active.operator_id = Some(case.operator_id.clone());
        active.previous_solution = Some(solution.clone());
        active.generation += 1;
    }

    Ok(FairSolveResult {
        solver: config.solver,
        status,
        solution,
        certificate,
        ledger,
        timing,
        system_id: case.system_id.clone(),
        operator_id: case.operator_id.clone(),
        relative_solution_error: relative_error,
        iterations,
        message,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use rodas5p_core::{CoreResult, DenseMatrix};

    use super::*;

    struct ObservedOperator {
        matrix: DenseMatrix,
        entered: AtomicU64,
        completed: AtomicU64,
        failed: AtomicU64,
        fail_at: u64,
    }

    impl LinearOperator for ObservedOperator {
        fn dimension(&self) -> usize {
            self.matrix.nrows()
        }

        fn token(&self) -> u64 {
            1
        }

        fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
            let entry = self.entered.fetch_add(1, Ordering::Relaxed) + 1;
            if entry == self.fail_at {
                self.failed.fetch_add(1, Ordering::Relaxed);
                return Err(CoreError::NonFinite("injected callback failure".into()));
            }
            self.matrix.matvec_into(x, y)?;
            self.completed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn run_observed(
        budget: u64,
        exact_guess: bool,
        fail_at: u64,
    ) -> (FairSolveResult, Arc<ObservedOperator>) {
        let matrix = DenseMatrix::from_rows(&[&[4.0, 1.0], &[1.0, 3.0]]).unwrap();
        let case = LinearSystemCase::from_matrix(matrix.clone(), vec![2.0, -5.0], 0, 0).unwrap();
        let config = FairSolveConfig {
            solver: SolverKind::Gmres,
            rtol: 1.0e-9,
            atol: 1.0e-12,
            restart: 2,
            recycle_dim: 1,
            hard_operator_budget: budget,
            preconditioner: PreconditionerKind::None,
            use_previous_oracle_guess: false,
        };
        let observed = Arc::new(ObservedOperator {
            matrix,
            entered: AtomicU64::new(0),
            completed: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            fail_at,
        });
        let guess = [1.0, -2.0];
        let result = solve_case_with_operator(
            &case,
            &config,
            None,
            exact_guess.then_some(guess.as_slice()),
            observed.clone(),
            Instant::now(),
        )
        .unwrap();
        (result, observed)
    }

    fn check_entry_accounting(
        result: &FairSolveResult,
        observed: &ObservedOperator,
        budget: u64,
        refused: u64,
    ) {
        let entered = observed.entered.load(Ordering::Relaxed);
        assert_eq!(
            result.ledger.operator_total(),
            entered + refused,
            "A = E + R"
        );
        assert!(entered <= budget, "E <= B");
        assert_eq!(
            entered,
            observed.completed.load(Ordering::Relaxed) + observed.failed.load(Ordering::Relaxed),
            "E = C + F"
        );
    }

    #[test]
    fn solver_refusal_counts_the_attempt_without_entering_the_callback() {
        let (result, observed) = run_observed(1, false, 0);
        assert_eq!(result.status, SolveStatus::BudgetExhausted);
        assert!(!result.certificate.passed);
        assert_eq!(result.ledger.operator_krylov, 2);
        assert_eq!(result.ledger.operator_diagnostic, 0);
        assert_eq!(result.ledger.budget_exhaustions, 1);
        check_entry_accounting(&result, &observed, 1, 1);
    }

    #[test]
    fn external_diagnostic_refusal_counts_the_attempt_without_false_certification() {
        let (result, observed) = run_observed(2, true, 0);
        assert_eq!(result.status, SolveStatus::BudgetExhausted);
        assert!(!result.certificate.passed);
        assert_eq!(result.solution, [1.0, -2.0]);
        assert_eq!(result.ledger.operator_krylov, 1);
        assert_eq!(result.ledger.operator_diagnostic, 2);
        assert_eq!(result.ledger.budget_exhaustions, 1);
        check_entry_accounting(&result, &observed, 2, 1);
    }

    #[test]
    fn admitted_solver_and_diagnostic_failures_remain_charged_and_preserve_the_error() {
        for (exact_guess, fail_at) in [(false, 1), (true, 3)] {
            let (result, observed) = run_observed(5, exact_guess, fail_at);
            assert_eq!(result.status, SolveStatus::NumericalFailure);
            assert!(!result.certificate.passed);
            assert!(result.message.contains("injected callback failure"));
            assert_eq!(result.ledger.budget_exhaustions, 0);
            assert_eq!(observed.failed.load(Ordering::Relaxed), 1);
            check_entry_accounting(&result, &observed, 5, 0);
        }
    }
}
