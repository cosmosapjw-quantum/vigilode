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
    debug_assert_eq!(ledger.operator_total(), budget.used());
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
