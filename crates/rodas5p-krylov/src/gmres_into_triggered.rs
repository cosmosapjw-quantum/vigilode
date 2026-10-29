//! Opt-in right flexible GMRES with Givens trigger before candidate construction.
//! This path deliberately does not change standard dispatch. The monitor must
//! verify the original-equation residual and any physical stage-error budget.
//! For nonstationary right preconditioning we store z_j=P_j^-1 v_j explicitly.
use crate::{
    common::{true_residual_into, validate_system},
    gmres::GmresConfig,
    gmres_into::{
        CapacityGrowth, ColumnSide, GmresCapacity, GmresColumnMonitor, GmresMonitorDecision,
        GmresMonitoredProgress, GmresMonitoredReport, arnoldi_column, begin_cycle,
    },
    kernels::{axpy, linear_combination_into},
    small::least_squares,
    workspace::GmresWorkspace,
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, LinearOperator, Preconditioner,
    PreconditionerSide, WorkCounters, safe_l2,
};

struct ProjectedGivens {
    cosine: Vec<f64>,
    sine: Vec<f64>,
    rotated_rhs: Vec<f64>,
    column: Vec<f64>,
}
impl ProjectedGivens {
    fn new(columns: usize, beta: f64) -> Self {
        let mut rotated_rhs = vec![0.0; columns + 1];
        rotated_rhs[0] = beta;
        Self {
            cosine: vec![0.0; columns],
            sine: vec![0.0; columns],
            rotated_rhs,
            column: vec![0.0; columns + 1],
        }
    }
    fn update(&mut self, h: &DenseMatrix, j: usize) -> CoreResult<f64> {
        for row in 0..=j + 1 {
            self.column[row] = h[(row, j)];
        }
        for row in 0..j {
            let a = self.column[row];
            let b = self.column[row + 1];
            self.column[row] = self.cosine[row] * a + self.sine[row] * b;
            self.column[row + 1] = -self.sine[row] * a + self.cosine[row] * b;
        }
        if self.column[..j + 2].iter().any(|v| !v.is_finite()) {
            return Err(CoreError::NonFinite("nonfinite GMRES Givens column".into()));
        }
        let radius = self.column[j].hypot(self.column[j + 1]);
        if !radius.is_finite() {
            return Err(CoreError::NonFinite("nonfinite GMRES Givens radius".into()));
        }
        let (c, s) = if radius == 0.0 {
            (1.0, 0.0)
        } else {
            (self.column[j] / radius, self.column[j + 1] / radius)
        };
        self.cosine[j] = c;
        self.sine[j] = s;
        let old_rhs = self.rotated_rhs[j];
        self.rotated_rhs[j] = c * old_rhs;
        self.rotated_rhs[j + 1] = -s * old_rhs;
        let projected = self.rotated_rhs[j + 1].abs();
        if !projected.is_finite() {
            return Err(CoreError::NonFinite(
                "nonfinite GMRES projected residual".into(),
            ));
        }
        Ok(projected)
    }
}

fn flexible_least_squares_correction(
    workspace: &mut GmresWorkspace,
    actual: usize,
    beta: f64,
) -> CoreResult<()> {
    let arnoldi = &mut workspace.arnoldi;
    arnoldi.hessenberg_prefix.resize_zeros(actual + 1, actual)?;
    for row in 0..actual + 1 {
        for column in 0..actual {
            arnoldi.hessenberg_prefix[(row, column)] = arnoldi.hessenberg[(row, column)];
        }
    }
    arnoldi.rhs_small[..actual + 1].fill(0.0);
    arnoldi.rhs_small[0] = beta;
    match workspace.least_squares.as_deref_mut() {
        Some(small) => {
            let sol = &mut workspace.least_squares_solution;
            small.solve_into(
                &arnoldi.hessenberg_prefix,
                &arnoldi.rhs_small[..actual + 1],
                sol,
            )?;
            linear_combination_into(&arnoldi.directions[..actual], sol, &mut arnoldi.correction)?;
        }
        None => {
            let sol = least_squares(&arnoldi.hessenberg_prefix, &arnoldi.rhs_small[..actual + 1])?;
            linear_combination_into(&arnoldi.directions[..actual], &sol, &mut arnoldi.correction)?;
        }
    }
    Ok(())
}

/// Research-only right flexible GMRES. A column calls the preconditioner
/// exactly once and saves the resulting direction, including variable/nonlinear
/// preconditioners. Givens detects a *candidate opportunity*, never a certificate.
/// The supplied monitor MUST certify original residual and stage-error bounds.
///
/// Unlike the existing per-column monitored solver, no least-squares or
/// candidate is constructed until a projected trigger, breakdown or cycle end.
/// Restart, callback error accounting, and output transaction remain explicit.
#[allow(clippy::too_many_arguments)]
pub fn solve_gmres_into_right_flexible_triggered_monitored(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: &[f64],
    config: &GmresConfig,
    projected_trigger: f64,
    capacity: GmresCapacity,
    output: &mut [f64],
    workspace: &mut GmresWorkspace,
    counters: &mut WorkCounters,
    monitor: &mut GmresColumnMonitor<'_>,
) -> CoreResult<GmresMonitoredReport> {
    workspace.monitored_progress = GmresMonitoredProgress::default();
    config.validate()?;
    if !projected_trigger.is_finite() || projected_trigger < 0.0 {
        return Err(CoreError::InvalidInput(
            "invalid GMRES projected trigger".into(),
        ));
    }
    if pc.side() != PreconditionerSide::Right {
        return Err(CoreError::InvalidInput(
            "flexible triggered GMRES requires a right preconditioner".into(),
        ));
    }
    let n = validate_system(op, pc, rhs, Some(x0))?;
    if output.len() != n {
        return Err(CoreError::Dimension(
            "GMRES output length differs from system".into(),
        ));
    }
    let columns = config.restart.min(config.max_arnoldi).min(n.max(1));
    if capacity.growth == CapacityGrowth::Refuse
        && (n > capacity.max_dimension || columns > capacity.max_columns)
    {
        return Err(CoreError::InvalidInput(format!(
            "KRYLOV_CAPACITY_EXCEEDED: GMRES needs dimension {n} and {columns} columns"
        )));
    }
    workspace.common.prepare(n);
    workspace.common.x.copy_from_slice(x0);
    let (mut total, mut cycles, mut solves) = (0usize, 0u64, 0u64);
    while total < config.max_arnoldi {
        if workspace.common.x.iter().all(|v| *v == 0.0) {
            workspace.common.residual.copy_from_slice(rhs);
        } else {
            true_residual_into(
                op,
                rhs,
                &workspace.common.x,
                &mut workspace.common.operator_output,
                &mut workspace.common.residual,
                counters,
                ApplyCategory::Krylov,
            )?;
        }
        workspace
            .common
            .preconditioned
            .copy_from_slice(&workspace.common.residual);
        let beta = safe_l2(&workspace.common.preconditioned);
        if beta == 0.0 {
            break;
        }
        if !(beta > f64::MIN_POSITIVE && beta.is_finite()) {
            return Err(CoreError::LinearSolve(
                "right GMRES residual breakdown".into(),
            ));
        }
        let steps = config.restart.min(config.max_arnoldi - total).min(n.max(1));
        // This opt-in path deliberately owns an additional Z basis.
        workspace.arnoldi.prepare(n, steps)?;
        begin_cycle(
            &mut workspace.arnoldi,
            &workspace.common.preconditioned,
            n,
            steps,
        )?;
        let mut givens = ProjectedGivens::new(steps, beta);
        cycles += 1;
        workspace.monitored_progress.cycles += 1;
        let mut actual = 0usize;
        let mut cannot_extend = false;
        let mut decision = GmresMonitorDecision::Continue;
        let mut candidate_exists = false;
        for j in 0..steps {
            workspace.monitored_progress.attempted_columns += 1;
            let happy = arnoldi_column(
                op,
                pc,
                ColumnSide::Right,
                &mut workspace.arnoldi,
                j,
                counters,
                Some(&mut workspace.monitored_progress.column_operator_applications),
            )?;
            // Preconditioned direction is the actual P_j^-1 v_j from this call.
            workspace.arnoldi.directions[j].copy_from_slice(&workspace.arnoldi.raw_operator_output);
            workspace.monitored_progress.completed_columns += 1;
            counters.linear_iterations += 1;
            actual = j + 1;
            let projected = givens.update(&workspace.arnoldi.hessenberg, j)?;
            if projected <= projected_trigger || happy || actual == steps {
                workspace.monitored_progress.least_squares_attempts += 1;
                flexible_least_squares_correction(workspace, actual, beta)?;
                workspace.monitored_progress.completed_least_squares_solves += 1;
                solves += 1;
                workspace
                    .common
                    .scratch_b
                    .copy_from_slice(&workspace.common.x);
                axpy(
                    1.0,
                    &workspace.arnoldi.correction,
                    &mut workspace.common.scratch_b,
                    counters,
                )?;
                candidate_exists = true;
                decision = monitor(
                    (total + actual) as u64,
                    &workspace.common.scratch_b,
                    counters,
                )?;
                if decision != GmresMonitorDecision::Continue {
                    break;
                }
            }
            if happy {
                cannot_extend = true;
                break;
            }
        }
        total += actual;
        match decision {
            GmresMonitorDecision::Accept => {
                counters.linear_solves += 1;
                output.copy_from_slice(&workspace.common.scratch_b);
                return Ok(GmresMonitoredReport {
                    accepted: true,
                    iterations: total as u64,
                    cycles,
                    least_squares_solves: solves,
                });
            }
            GmresMonitorDecision::Abandon => break,
            GmresMonitorDecision::Continue => {
                if cannot_extend || !candidate_exists {
                    break;
                }
                workspace
                    .common
                    .x
                    .copy_from_slice(&workspace.common.scratch_b);
            }
        }
    }
    Ok(GmresMonitoredReport {
        accepted: false,
        iterations: total as u64,
        cycles,
        least_squares_solves: solves,
    })
}
