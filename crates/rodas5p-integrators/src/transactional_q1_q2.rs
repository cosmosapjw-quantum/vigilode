use rodas5p_core::{
    CoreError, CoreResult, IdentityPreconditioner, InitialGuess, LinearMethod, LinearOperator,
    LinearSolverConfig, PreconditionerKind, WorkCounters, error_scale, wrms,
};
use rodas5p_krylov::{GmresConfig, solve_gmres};
use serde::Serialize;

use crate::homotopy::{add_scaled_rows, evaluate_partial_path};
use crate::{
    HomotopyWorkLedger, OdeProblem, ParallelExecution, StepCertificate, StepContext, StepResult,
    StructuredBlockSystem, build_step_context_matrix_free, finish_step, sequential_stages,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionalQ1Q2Lane {
    Q1Fast,
    Q2Escalated,
    SequentialFallback,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OperationalGateReport {
    pub accepted: bool,
    pub reason: String,
    pub finite: bool,
    pub target_residual_before: f64,
    pub target_residual_after: f64,
    pub target_residual_ratio: f64,
    pub target_relative_residual: f64,
    pub previous_output_correction_wrms: f64,
    pub last_output_correction_wrms: f64,
    pub output_contraction: f64,
    pub contraction_tail_wrms: f64,
    pub embedded_error: f64,
    pub output_budget: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransactionalQ1Q2Config {
    pub threads: usize,
    pub gmres_restart: usize,
    pub gmres_max_arnoldi: usize,
    pub gmres_rtol: f64,
    pub gmres_atol: f64,
    pub absolute_output_budget: f64,
    pub embedded_budget_fraction: f64,
    pub max_output_contraction: f64,
    pub max_residual_contraction: f64,
    pub max_target_relative_residual: f64,
}

impl Default for TransactionalQ1Q2Config {
    fn default() -> Self {
        Self {
            threads: 1,
            gmres_restart: 32,
            gmres_max_arnoldi: 256,
            gmres_rtol: 1.0e-10,
            gmres_atol: 1.0e-12,
            absolute_output_budget: 0.1,
            embedded_budget_fraction: 0.2,
            max_output_contraction: 0.8,
            max_residual_contraction: 0.9,
            max_target_relative_residual: 1.0e-5,
        }
    }
}

impl TransactionalQ1Q2Config {
    pub fn validate(&self) -> CoreResult<()> {
        if self.threads == 0 || self.gmres_restart == 0 || self.gmres_max_arnoldi == 0 {
            return Err(CoreError::InvalidInput(
                "transactional q1/q2 thread and Krylov limits must be positive".into(),
            ));
        }
        if self.gmres_max_arnoldi < self.gmres_restart {
            return Err(CoreError::InvalidInput(
                "transactional q1/q2 max Arnoldi count must be at least the restart length".into(),
            ));
        }
        if ![
            self.gmres_rtol,
            self.gmres_atol,
            self.absolute_output_budget,
            self.embedded_budget_fraction,
            self.max_output_contraction,
            self.max_residual_contraction,
            self.max_target_relative_residual,
        ]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(CoreError::InvalidInput(
                "transactional q1/q2 tolerances contain invalid values".into(),
            ));
        }
        if !(self.absolute_output_budget > 0.0
            && self.embedded_budget_fraction > 0.0
            && self.max_output_contraction < 1.0
            && self.max_residual_contraction < 1.0
            && self.max_target_relative_residual > 0.0)
        {
            return Err(CoreError::InvalidInput(
                "transactional q1/q2 gate parameters lie outside their admissible ranges".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TransactionalQ1Q2StepReport {
    pub step: StepResult,
    pub lane: TransactionalQ1Q2Lane,
    pub q1_gate: OperationalGateReport,
    pub q2_gate: Option<OperationalGateReport>,
    pub fast_accepted: bool,
    pub escalated: bool,
    pub fallback_reason: Option<String>,
    pub q1_candidate_y: Vec<f64>,
    pub q2_candidate_y: Option<Vec<f64>>,
    pub critical_path_depth: u64,
    pub work: HomotopyWorkLedger,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TransactionalQ1Q2RunDiagnostics {
    /// Every transactional attempt evaluates the q=1 path before any escalation or fallback.
    pub q1_path_attempts: usize,
    /// Attempts that reached the q=2 causal escalation, including attempts that later fell back.
    pub q2_path_attempts: usize,
    /// Final lane selected for the attempt after all transactional gates.
    pub selected_q1_fast_attempts: usize,
    pub selected_q2_escalated_attempts: usize,
    pub selected_sequential_fallback_attempts: usize,
    pub accepted_q1_fast_steps: usize,
    pub accepted_q2_escalated_steps: usize,
    pub accepted_sequential_fallback_steps: usize,
    pub total_w_solve_batches: u64,
    pub total_w_solve_vectors: u64,
    pub max_w_solve_batches_per_attempt: u64,
    pub accepted_w_solve_batches: Vec<u64>,
    pub total_critical_path_depth: u64,
    pub max_critical_path_depth_per_attempt: u64,
    pub accepted_critical_path_depths: Vec<u64>,
    pub attempted_lanes: Vec<TransactionalQ1Q2Lane>,
}

impl TransactionalQ1Q2RunDiagnostics {
    pub(crate) fn record(&mut self, report: &TransactionalQ1Q2StepReport, accepted: bool) {
        self.total_w_solve_batches = self
            .total_w_solve_batches
            .saturating_add(report.work.w_solve_batches);
        self.total_w_solve_vectors = self
            .total_w_solve_vectors
            .saturating_add(report.work.w_solve_vectors);
        self.max_w_solve_batches_per_attempt = self
            .max_w_solve_batches_per_attempt
            .max(report.work.w_solve_batches);
        self.total_critical_path_depth = self
            .total_critical_path_depth
            .saturating_add(report.critical_path_depth);
        self.max_critical_path_depth_per_attempt = self
            .max_critical_path_depth_per_attempt
            .max(report.critical_path_depth);
        self.q1_path_attempts += 1;
        if report.escalated {
            self.q2_path_attempts += 1;
        }
        self.attempted_lanes.push(report.lane);
        match report.lane {
            TransactionalQ1Q2Lane::Q1Fast => {
                self.selected_q1_fast_attempts += 1;
                if accepted {
                    self.accepted_q1_fast_steps += 1;
                }
            }
            TransactionalQ1Q2Lane::Q2Escalated => {
                self.selected_q2_escalated_attempts += 1;
                if accepted {
                    self.accepted_q2_escalated_steps += 1;
                }
            }
            TransactionalQ1Q2Lane::SequentialFallback => {
                self.selected_sequential_fallback_attempts += 1;
                if accepted {
                    self.accepted_sequential_fallback_steps += 1;
                }
            }
        }
        if accepted {
            self.accepted_w_solve_batches
                .push(report.work.w_solve_batches);
            self.accepted_critical_path_depths
                .push(report.critical_path_depth);
        }
    }

    pub fn accepted_steps(&self) -> usize {
        self.accepted_q1_fast_steps
            + self.accepted_q2_escalated_steps
            + self.accepted_sequential_fallback_steps
    }

    pub fn fast_fraction(&self) -> f64 {
        let accepted = self.accepted_steps();
        if accepted == 0 {
            0.0
        } else {
            (self.accepted_q1_fast_steps + self.accepted_q2_escalated_steps) as f64
                / accepted as f64
        }
    }

    pub fn fallback_fraction(&self) -> f64 {
        let accepted = self.accepted_steps();
        if accepted == 0 {
            0.0
        } else {
            self.accepted_sequential_fallback_steps as f64 / accepted as f64
        }
    }
}

struct MatrixFreeCommonWSolver<'ctx, 'problem> {
    context: &'ctx StepContext<'problem>,
    execution: ParallelExecution,
    gmres: GmresConfig,
}

impl<'ctx, 'problem> MatrixFreeCommonWSolver<'ctx, 'problem> {
    fn new(
        context: &'ctx StepContext<'problem>,
        config: &TransactionalQ1Q2Config,
    ) -> CoreResult<Self> {
        if context.shifted.explicit().is_some() {
            return Err(CoreError::InvalidInput(
                "transactional q1/q2 fast path received an explicit shifted matrix".into(),
            ));
        }
        Ok(Self {
            context,
            execution: ParallelExecution::rayon(config.threads)?,
            gmres: GmresConfig {
                restart: config.gmres_restart.min(context.problem.dimension.max(1)),
                max_arnoldi: config.gmres_max_arnoldi,
                rtol: config.gmres_rtol,
                atol: config.gmres_atol,
            },
        })
    }

    fn solve_rows(
        &self,
        rhs: &[Vec<f64>],
        counters: &mut WorkCounters,
        work: &mut HomotopyWorkLedger,
    ) -> CoreResult<Vec<Vec<f64>>> {
        let n = self.context.problem.dimension;
        if rhs.is_empty() || rhs.iter().any(|row| row.len() != n) {
            return Err(CoreError::Dimension(
                "transactional common-W RHS batch shape mismatch".into(),
            ));
        }
        let indices = (0..rhs.len()).collect::<Vec<_>>();
        let solved = self.execution.map_ordered(&indices, |&index| {
            let pc = IdentityPreconditioner::new(n);
            let mut local = WorkCounters::default();
            let report = solve_gmres(
                &self.context.shifted,
                &pc,
                &rhs[index],
                None,
                &self.gmres,
                &mut local,
            );
            Ok((report.map(|report| report.x), local))
        })?;
        counters.block_linear_solves += 1;
        work.w_solve_batches += 1;
        work.w_solve_vectors += rhs.len() as u64;
        let mut rows = Vec::with_capacity(rhs.len());
        let mut first_error = None;
        for (row, local) in solved {
            counters.block_linear_iterations = counters
                .block_linear_iterations
                .saturating_add(local.linear_iterations);
            counters.accumulate(local);
            match row {
                Ok(row) => rows.push(row),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        Ok(rows)
    }
}

struct TruncatedInverseRows {
    sum: Vec<Vec<f64>>,
    last_term: Vec<Vec<f64>>,
}

fn truncated_inverse_rows_with_last_term(
    block: &StructuredBlockSystem<'_, '_>,
    solver: &MatrixFreeCommonWSolver<'_, '_>,
    eta: f64,
    q: usize,
    rhs: &[Vec<f64>],
    counters: &mut WorkCounters,
    work: &mut HomotopyWorkLedger,
) -> CoreResult<TruncatedInverseRows> {
    if !eta.is_finite() || q >= block.s {
        return Err(CoreError::InvalidInput(
            "invalid matrix-free truncated-inverse parameters".into(),
        ));
    }
    let mut term = solver.solve_rows(rhs, counters, work)?;
    let mut sum = term.clone();
    if eta != 0.0 {
        for _ in 1..=q {
            let mut coupled = block.coupling_apply(&term, counters)?;
            work.coupling_actions += 1;
            for value in coupled.iter_mut().flatten() {
                *value *= eta;
            }
            term = solver.solve_rows(&coupled, counters, work)?;
            for (sum_row, term_row) in sum.iter_mut().zip(&term) {
                for (sum_value, term_value) in sum_row.iter_mut().zip(term_row) {
                    *sum_value += term_value;
                }
            }
        }
    }
    if sum.iter().flatten().all(|value| value.is_finite())
        && term.iter().flatten().all(|value| value.is_finite())
    {
        Ok(TruncatedInverseRows {
            sum,
            last_term: term,
        })
    } else {
        Err(CoreError::NonFinite(
            "matrix-free truncated inverse produced NaN/Inf".into(),
        ))
    }
}

fn truncated_inverse_rows(
    block: &StructuredBlockSystem<'_, '_>,
    solver: &MatrixFreeCommonWSolver<'_, '_>,
    eta: f64,
    q: usize,
    rhs: &[Vec<f64>],
    counters: &mut WorkCounters,
    work: &mut HomotopyWorkLedger,
) -> CoreResult<Vec<Vec<f64>>> {
    Ok(truncated_inverse_rows_with_last_term(block, solver, eta, q, rhs, counters, work)?.sum)
}

fn weighted_stage_update(weights: &[f64], rows: &[Vec<f64>], n: usize) -> Vec<f64> {
    let mut output = vec![0.0; n];
    for (&weight, row) in weights.iter().zip(rows) {
        for (value, increment) in output.iter_mut().zip(row) {
            *value += weight * increment;
        }
    }
    output
}

fn candidate_output(context: &StepContext<'_>, stages: &[Vec<f64>]) -> Vec<f64> {
    let update = weighted_stage_update(&context.coeffs.b, stages, context.problem.dimension);
    context
        .y
        .iter()
        .zip(update)
        .map(|(base, increment)| base + increment)
        .collect()
}

fn embedded_error(
    context: &StepContext<'_>,
    stages: &[Vec<f64>],
    atol: f64,
    rtol: f64,
) -> CoreResult<f64> {
    let output = candidate_output(context, stages);
    let error = weighted_stage_update(&context.coeffs.btilde, stages, context.problem.dimension);
    let scale = error_scale(&context.y, &output, &[atol], rtol)?;
    wrms(&error, &scale)
}

fn output_correction_wrms(
    context: &StepContext<'_>,
    candidate: &[Vec<f64>],
    correction: &[Vec<f64>],
    atol: f64,
    rtol: f64,
) -> CoreResult<f64> {
    let output = candidate_output(context, candidate);
    let projected = weighted_stage_update(&context.coeffs.b, correction, context.problem.dimension);
    let scale = error_scale(&context.y, &output, &[atol], rtol)?;
    wrms(&projected, &scale)
}

fn operational_gate(
    config: &TransactionalQ1Q2Config,
    residual_before: f64,
    residual_after: f64,
    target_rhs_norm: f64,
    previous_output_correction_wrms: f64,
    last_output_correction_wrms: f64,
    embedded_error: f64,
) -> OperationalGateReport {
    let finite = [
        residual_before,
        residual_after,
        target_rhs_norm,
        previous_output_correction_wrms,
        last_output_correction_wrms,
        embedded_error,
    ]
    .iter()
    .all(|value| value.is_finite());
    let target_residual_ratio = residual_after / residual_before.max(f64::MIN_POSITIVE);
    let target_relative_residual = residual_after / target_rhs_norm.max(f64::MIN_POSITIVE);
    let output_contraction = if previous_output_correction_wrms <= f64::MIN_POSITIVE {
        if last_output_correction_wrms <= f64::MIN_POSITIVE {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        last_output_correction_wrms / previous_output_correction_wrms
    };
    let contraction_tail_wrms = if output_contraction < 1.0 {
        last_output_correction_wrms / (1.0 - output_contraction)
    } else {
        f64::INFINITY
    };
    // The embedded RODAS5P estimator is dimensionless.  Raising it to 6/5 forces the
    // algebraic-tail allowance to shrink like a fifth-order method's local O(h^6) defect when
    // the embedded difference scales as O(h^5), while the absolute cap preserves a bounded
    // tolerance-relative fallback in coarse regimes.
    let output_budget = config.absolute_output_budget.min(
        config.embedded_budget_fraction * embedded_error.max(1024.0 * f64::EPSILON).powf(6.0 / 5.0),
    );
    let residual_gate = target_relative_residual <= config.max_target_relative_residual
        || target_residual_ratio <= config.max_residual_contraction;
    let accepted = finite
        && output_contraction <= config.max_output_contraction
        && residual_gate
        && contraction_tail_wrms <= output_budget;
    let reason = if !finite {
        "nonfinite diagnostic"
    } else if output_contraction > config.max_output_contraction {
        "output correction did not contract"
    } else if !residual_gate {
        "target residual is neither small nor contracting"
    } else if contraction_tail_wrms > output_budget {
        "correction tail above order-aware budget"
    } else {
        "operational gate passed"
    }
    .to_string();
    OperationalGateReport {
        accepted,
        reason,
        finite,
        target_residual_before: residual_before,
        target_residual_after: residual_after,
        target_residual_ratio,
        target_relative_residual,
        previous_output_correction_wrms,
        last_output_correction_wrms,
        output_contraction,
        contraction_tail_wrms,
        embedded_error,
        output_budget,
    }
}

fn build_step_certificate(
    gate: &OperationalGateReport,
    iterations: usize,
    reason_prefix: &str,
) -> StepCertificate {
    StepCertificate {
        accepted: gate.accepted,
        reason: format!("{reason_prefix}: {}", gate.reason),
        iterations,
        embedded_error: gate.embedded_error,
        fixed_point_error: gate.contraction_tail_wrms,
        residual_proxy_error: gate.target_relative_residual,
        contraction_tail_error: Some(gate.contraction_tail_wrms),
        observed_contraction: Some(gate.output_contraction),
        stage_residual_norm: gate.target_residual_after,
        stage_relative_residual: gate.target_relative_residual,
    }
}

struct FastPathSuccess {
    stages: Vec<Vec<f64>>,
    q1_candidate_y: Vec<f64>,
    q2_candidate_y: Option<Vec<f64>>,
    lane: TransactionalQ1Q2Lane,
    q1_gate: OperationalGateReport,
    q2_gate: Option<OperationalGateReport>,
    active_gate: OperationalGateReport,
    escalated: bool,
}

struct FastPathFailure {
    error: CoreError,
    q1_gate: Option<OperationalGateReport>,
    q2_gate: Option<OperationalGateReport>,
    escalated: bool,
}

fn failed_operational_gate(reason: impl Into<String>) -> OperationalGateReport {
    OperationalGateReport {
        accepted: false,
        reason: reason.into(),
        finite: false,
        target_residual_before: f64::INFINITY,
        target_residual_after: f64::INFINITY,
        target_residual_ratio: f64::INFINITY,
        target_relative_residual: f64::INFINITY,
        previous_output_correction_wrms: f64::INFINITY,
        last_output_correction_wrms: f64::INFINITY,
        output_contraction: f64::INFINITY,
        contraction_tail_wrms: f64::INFINITY,
        embedded_error: f64::INFINITY,
        output_budget: 0.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn attempt_fast_path(
    context: &StepContext<'_>,
    block: &StructuredBlockSystem<'_, '_>,
    solver: &MatrixFreeCommonWSolver<'_, '_>,
    config: &TransactionalQ1Q2Config,
    atol: f64,
    rtol: f64,
    counters: &mut WorkCounters,
    work: &mut HomotopyWorkLedger,
) -> Result<FastPathSuccess, Box<FastPathFailure>> {
    let mut q1_gate: Option<OperationalGateReport> = None;
    let mut q2_gate: Option<OperationalGateReport> = None;
    let mut escalated = false;
    macro_rules! fast_try {
        ($expression:expr) => {
            match $expression {
                Ok(value) => value,
                Err(error) => {
                    return Err(Box::new(FastPathFailure {
                        error,
                        q1_gate: q1_gate.clone(),
                        q2_gate: q2_gate.clone(),
                        escalated,
                    }));
                }
            }
        };
    }

    // Decoupled lambda=0 start and direct endpoint predictor.  eta=0 makes the q=1 tangent
    // solve one common-W batch; no q-dependent work is hidden in the start state.
    let mut stages = fast_try!(truncated_inverse_rows(
        block,
        solver,
        0.0,
        1,
        &block.rhs_base(),
        counters,
        work,
    ));
    let start = fast_try!(evaluate_partial_path(
        block, &stages, 0.0, 0.0, counters, work,
    ));
    let tangent = fast_try!(truncated_inverse_rows(
        block,
        solver,
        0.0,
        1,
        &start.tangent_rhs,
        counters,
        work,
    ));
    stages = fast_try!(add_scaled_rows(&stages, 1.0, &tangent, 0.0, None));

    let mut q1_corrections = Vec::with_capacity(2);
    let mut last_q1_causal_term = None;
    let mut residuals = Vec::with_capacity(3);
    for correction_index in 0..2 {
        let evaluation = fast_try!(evaluate_partial_path(
            block, &stages, 0.0, 1.0, counters, work,
        ));
        residuals.push(evaluation.target_residual_norm);
        let inverse = fast_try!(truncated_inverse_rows_with_last_term(
            block,
            solver,
            1.0,
            1,
            &evaluation.homotopy_residual,
            counters,
            work,
        ));
        if correction_index == 1 {
            last_q1_causal_term = Some(inverse.last_term.clone());
        }
        let correction_wrms = fast_try!(output_correction_wrms(
            context,
            &stages,
            &inverse.sum,
            atol,
            rtol,
        ));
        q1_corrections.push(correction_wrms);
        stages = fast_try!(add_scaled_rows(&stages, -1.0, &inverse.sum, 0.0, None));
        work.correction_rounds += 1;
    }
    let q1_final = fast_try!(evaluate_partial_path(
        block, &stages, 0.0, 1.0, counters, work,
    ));
    let q1_candidate_y = candidate_output(context, &stages);
    residuals.push(q1_final.target_residual_norm);
    let q1_embedded = fast_try!(embedded_error(context, &stages, atol, rtol));
    let q1_report = operational_gate(
        config,
        residuals[1],
        residuals[2],
        q1_final.target_rhs_norm,
        q1_corrections[0],
        q1_corrections[1],
        q1_embedded,
    );
    q1_gate = Some(q1_report.clone());
    if q1_report.accepted {
        return Ok(FastPathSuccess {
            stages,
            q1_candidate_y,
            q2_candidate_y: None,
            lane: TransactionalQ1Q2Lane::Q1Fast,
            q1_gate: q1_report.clone(),
            q2_gate,
            active_gate: q1_report,
            escalated,
        });
    }

    escalated = true;
    let q2_before = q1_final.target_residual_norm;
    // The q=1 endpoint and all prior common-W work are retained.  The second q=1 correction
    // already contains D^{-1}r + QD^{-1}r for its own residual.  One additional common-W
    // application computes Q^2D^{-1}r and upgrades that correction to q=2 without restarting
    // from the decoupled predictor.  This is the load-bearing 6 -> 7 batch escalation.
    let mut q2_rhs = fast_try!(block.coupling_apply(
        &last_q1_causal_term.expect("second q1 causal term is available"),
        counters,
    ));
    work.coupling_actions += 1;
    // eta=1 at the target endpoint.  Retain the explicit scaling loop so that a future
    // nonunit-endpoint extension cannot silently inherit this assumption.
    for value in q2_rhs.iter_mut().flatten() {
        *value *= 1.0;
    }
    let q2_correction = fast_try!(solver.solve_rows(&q2_rhs, counters, work));
    let q2_correction_wrms = fast_try!(output_correction_wrms(
        context,
        &stages,
        &q2_correction,
        atol,
        rtol,
    ));
    stages = fast_try!(add_scaled_rows(&stages, -1.0, &q2_correction, 0.0, None,));
    work.correction_rounds += 1;
    let q2_final = fast_try!(evaluate_partial_path(
        block, &stages, 0.0, 1.0, counters, work,
    ));
    let q2_candidate_y = Some(candidate_output(context, &stages));
    // One q=0 common-W diagnostic at the post-q2 residual estimates the *remaining* output
    // defect rather than reusing the magnitude of the correction that was already applied.
    // It is not applied to the state, so q=2 remains a single causal escalation; the eighth
    // batch is explicitly certificate work and is retained in the critical path ledger.
    let q2_diagnostic = fast_try!(solver.solve_rows(&q2_final.homotopy_residual, counters, work,));
    let q2_remaining_wrms = fast_try!(output_correction_wrms(
        context,
        &stages,
        &q2_diagnostic,
        atol,
        rtol,
    ));
    let q2_embedded = fast_try!(embedded_error(context, &stages, atol, rtol));
    let q2_report = operational_gate(
        config,
        q2_before,
        q2_final.target_residual_norm,
        q2_final.target_rhs_norm,
        q2_correction_wrms,
        q2_remaining_wrms,
        q2_embedded,
    );
    q2_gate = Some(q2_report.clone());
    Ok(FastPathSuccess {
        stages,
        q1_candidate_y,
        q2_candidate_y,
        lane: TransactionalQ1Q2Lane::Q2Escalated,
        q1_gate: q1_report,
        q2_gate,
        active_gate: q2_report,
        escalated,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn transactional_q1_q2_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &TransactionalQ1Q2Config,
    atol: f64,
    rtol: f64,
    force_accept: bool,
    counters: &mut WorkCounters,
) -> CoreResult<TransactionalQ1Q2StepReport> {
    config.validate()?;
    let before = *counters;
    counters.fast_attempts += 1;
    let context = build_step_context_matrix_free(problem, t, y, h, counters)?;
    if context.shifted.explicit().is_some() || counters.jacobian_builds != before.jacobian_builds {
        return Err(CoreError::InvalidInput(
            "transactional q1/q2 path violated the explicit-J-free contract".into(),
        ));
    }
    let block = StructuredBlockSystem::new(&context);
    if block.s != 8 {
        return Err(CoreError::InvalidInput(
            "transactional q1/q2 path requires the 8-stage RODAS5P tableau".into(),
        ));
    }
    let solver = MatrixFreeCommonWSolver::new(&context, config)?;
    let mut work = HomotopyWorkLedger {
        path_rounds: 1,
        ..HomotopyWorkLedger::default()
    };

    let fast = attempt_fast_path(
        &context, &block, &solver, config, atol, rtol, counters, &mut work,
    );
    let (
        mut lane,
        q1_gate,
        q2_gate,
        active_gate,
        fast_stages,
        q1_candidate_y,
        q2_candidate_y,
        escalated,
        mut fallback_reason,
    ) = match fast {
        Ok(result) => (
            result.lane,
            result.q1_gate,
            result.q2_gate,
            result.active_gate,
            Some(result.stages),
            result.q1_candidate_y,
            result.q2_candidate_y,
            result.escalated,
            None,
        ),
        Err(failure) => match failure.error {
            CoreError::NonFinite(_) | CoreError::LinearSolve(_) => {
                let reason = format!("speculative q1/q2 path failed: {}", failure.error);
                let gate = failure
                    .q2_gate
                    .clone()
                    .or_else(|| failure.q1_gate.clone())
                    .unwrap_or_else(|| failed_operational_gate(reason.clone()));
                (
                    TransactionalQ1Q2Lane::SequentialFallback,
                    failure
                        .q1_gate
                        .unwrap_or_else(|| failed_operational_gate(reason.clone())),
                    failure.q2_gate,
                    gate,
                    None,
                    Vec::new(),
                    None,
                    failure.escalated,
                    Some(reason),
                )
            }
            error => return Err(error),
        },
    };

    let fast_path_accepted = active_gate.accepted && fast_stages.is_some();
    let mut step = if fast_path_accepted {
        counters.fast_accepts += 1;
        let certificate = build_step_certificate(
            &active_gate,
            work.correction_rounds,
            match lane {
                TransactionalQ1Q2Lane::Q1Fast => "q1-c2",
                TransactionalQ1Q2Lane::Q2Escalated => "q1-c2-plus-q2-c1",
                TransactionalQ1Q2Lane::SequentialFallback => unreachable!(),
            },
        );
        let mut step = finish_step(
            &context,
            fast_stages.expect("fast stages present for accepted fast path"),
            atol,
            rtol,
            match lane {
                TransactionalQ1Q2Lane::Q1Fast => "RODAS5P-transactional-q1-c2".into(),
                TransactionalQ1Q2Lane::Q2Escalated => "RODAS5P-transactional-q1-c2-q2-c1".into(),
                TransactionalQ1Q2Lane::SequentialFallback => unreachable!(),
            },
            None,
            false,
            Some(certificate),
            before,
            counters,
        )?;
        step.accepted = active_gate.accepted && (force_accept || step.error_norm <= 1.0);
        step
    } else {
        if fallback_reason.is_none() {
            fallback_reason = Some(active_gate.reason.clone());
        }
        lane = TransactionalQ1Q2Lane::SequentialFallback;
        counters.fallback_steps += 1;
        let fallback_config = LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: config.gmres_rtol,
            atol: config.gmres_atol,
            restart: config.gmres_restart,
            maxiter: config.gmres_max_arnoldi,
            preconditioner: PreconditionerKind::None,
            x0_strategy: InitialGuess::Previous,
            ..LinearSolverConfig::default()
        };
        let data = sequential_stages(&context, &fallback_config, None, counters)?;
        finish_step(
            &context,
            data.stages,
            atol,
            rtol,
            "RODAS5P-protected-sequential-JF-fallback".into(),
            if force_accept { Some(true) } else { None },
            true,
            None,
            before,
            counters,
        )?
    };

    if step.accepted {
        counters.accepted_steps += 1;
    } else {
        counters.rejected_steps += 1;
    }
    step.used_fallback = lane == TransactionalQ1Q2Lane::SequentialFallback;
    step.counters = counters.delta(before);
    debug_assert_eq!(step.counters.jacobian_builds, 0);
    debug_assert_eq!(step.counters.direct_factorizations, 0);
    let fast_accepted = lane != TransactionalQ1Q2Lane::SequentialFallback && active_gate.accepted;
    let critical_path_depth = work.w_solve_batches
        + if lane == TransactionalQ1Q2Lane::SequentialFallback {
            context.coeffs.stages() as u64
        } else {
            0
        };
    Ok(TransactionalQ1Q2StepReport {
        step,
        lane,
        q1_gate,
        q2_gate,
        fast_accepted,
        escalated,
        fallback_reason,
        q1_candidate_y,
        q2_candidate_y,
        critical_path_depth,
        work,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[test]
    fn multi_row_failure_retains_later_row_shifted_work() {
        let applies = Arc::new(AtomicUsize::new(0));
        let jvp_applies = applies.clone();
        let problem = OdeProblem::new(
            "multi-row-accounting",
            1,
            Arc::new(|_t, _y, out| {
                out[0] = 0.0;
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(move |_t, _y, v, out| {
                if jvp_applies.fetch_add(1, Ordering::SeqCst) == 0 {
                    return Err(CoreError::LinearSolve("first row injected failure".into()));
                }
                out[0] = v[0];
                Ok(())
            })),
            Some(Arc::new(|_t, _y, out| {
                out[0] = 0.0;
                Ok(())
            })),
            false,
            None,
            None,
        )
        .unwrap();
        let mut counters = WorkCounters::default();
        let context =
            build_step_context_matrix_free(&problem, 0.0, &[1.0], 0.1, &mut counters).unwrap();
        let solver =
            MatrixFreeCommonWSolver::new(&context, &TransactionalQ1Q2Config::default()).unwrap();
        let mut work = HomotopyWorkLedger::default();

        let error = solver
            .solve_rows(&[vec![1.0], vec![2.0]], &mut counters, &mut work)
            .unwrap_err();

        assert!(error.to_string().contains("first row injected failure"));
        assert!(applies.load(Ordering::SeqCst) > 1);
        assert!(
            counters.linear_matvecs > 0,
            "later successful row applications must survive the first-row error"
        );
    }
}
