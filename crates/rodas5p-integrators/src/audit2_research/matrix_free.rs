//! Opt-in, same-trial matrix-free correction. Not a step-acceptance policy.
//!
//! One factory call and one reusable Givens workspace serve all lower-block
//! rows of ONE frozen context. No cross-step cache or stale-identity heuristic
//! exists. A user factory must be deterministic and account for its setup work;
//! its internal storage/cost and an existing dense mass are not made scalable
//! by this wrapper. Failed GMRES calls do not expose their unfinished iterate.

use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::{ApplyCategory, LinearSolveReport, Preconditioner, apply_counted};
use rodas5p_krylov::{GmresConfig, GmresGivensWorkspace, solve_gmres_givens_with_workspace};

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatrixFreeFailurePhase {
    InputValidation,
    Preparation,
    PreconditionerSetup,
    CorrectionJvp,
    CorrectionRhs,
    LinearSolve,
    TrueResidual,
    LinearDiagnostic,
    NonlinearDiagnostic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixFreeFailure {
    pub phase: MatrixFreeFailurePhase,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixFreeOperatorWork {
    pub w_attempts: u64,
    pub w_completed: u64,
    pub jvp_attempts: u64,
    pub jvp_completed: u64,
    pub mass_attempts: u64,
    pub mass_completed: u64,
    pub preconditioner_attempts: u64,
    pub preconditioner_completed: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MatrixFreeCorrectionWork {
    pub preparation_attempts: u64,
    pub preparation_completed: u64,
    pub preconditioner_setup_attempts: u64,
    pub preconditioner_setup_completed: u64,
    pub solve_attempts: u64,
    /// Kernel calls returning a report, BEFORE independent row verification.
    pub solve_completed: u64,
    pub true_residual_attempts: u64,
    pub true_residual_completed: u64,
    pub operator: MatrixFreeOperatorWork,
    pub kernel_counters: WorkCounters,
    pub correction_and_diagnostic: Audit2CorrectionWork,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatrixFreeRowCheck {
    pub stage: usize,
    pub rhs_l2: f64,
    pub threshold: f64,
    pub residual_l2: f64,
    pub relative_residual: Option<f64>,
    /// Retained workspace capacity, not peak memory or allocation count.
    pub workspace_capacity_f64: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixFreeCorrectionReport {
    pub completed: bool,
    pub failure: Option<MatrixFreeFailure>,
    pub config: GmresConfig,
    pub accuracy_disposition: Audit2OriginalTargetAccuracyDisposition,
    pub projection: Option<Audit2CoefficientProjection>,
    pub preparation_counters: WorkCounters,
    pub setup_counters: WorkCounters,
    /// Inherited preparation/nonlinear APIs count successful callbacks. If they fail,
    /// their internal failed-callback work is unknown, not silently zero.
    pub inherited_work_complete: bool,
    pub work: MatrixFreeCorrectionWork,
    /// Only independently verified rows are used by subsequent block rows.
    pub correction: Vec<Vec<f64>>,
    /// Includes a returned row even if its later independent check fails.
    pub linear_reports: Vec<LinearSolveReport>,
    pub rows: Vec<MatrixFreeRowCheck>,
    pub initial_residual_l2: Option<f64>,
    pub projected_linear_residual_l2: Option<f64>,
    pub nonlinear_residual_after_l2: Option<f64>,
    pub workspace_capacity_f64: usize,
    /// The existing kernel returns Err without its in-progress iterate.
    pub failed_kernel_iterate_available: bool,
}

impl MatrixFreeCorrectionReport {
    fn new(config: &GmresConfig) -> Self {
        Self {
            completed: false,
            failure: None,
            config: config.clone(),
            accuracy_disposition: Audit2OriginalTargetAccuracyDisposition::BudgetNotSpecified,
            projection: None,
            preparation_counters: WorkCounters::default(),
            setup_counters: WorkCounters::default(),
            inherited_work_complete: true,
            work: MatrixFreeCorrectionWork::default(),
            correction: Vec::new(),
            linear_reports: Vec::new(),
            rows: Vec::new(),
            initial_residual_l2: None,
            projected_linear_residual_l2: None,
            nonlinear_residual_after_l2: None,
            workspace_capacity_f64: 0,
            failed_kernel_iterate_available: false,
        }
    }

    fn fail(&mut self, phase: MatrixFreeFailurePhase, error: impl ToString) {
        self.failure = Some(MatrixFreeFailure {
            phase,
            message: error.to_string(),
        });
    }

    /// Available work, with attempted W/JVP/mass/PC callbacks charged once.
    /// Not exhaustive when inherited_work_complete is false. Kernel iteration
    /// counters on Err remain the kernel's reported completed-cycle counts.
    pub fn total_counters(&self) -> WorkCounters {
        let mut total = self.preparation_counters;
        total.accumulate(self.setup_counters);
        total.accumulate(self.work.kernel_counters);
        total.accumulate(self.work.correction_and_diagnostic.counters);
        total.jvp_calls = total
            .jvp_calls
            .saturating_add(self.work.operator.jvp_attempts);
        total.jvp_vectors = total
            .jvp_vectors
            .saturating_add(self.work.operator.jvp_attempts);
        total.mass_matvecs = total
            .mass_matvecs
            .saturating_add(self.work.operator.mass_attempts);
        total.preconditioner_apps = total.preconditioner_apps.saturating_add(
            self.work
                .operator
                .preconditioner_attempts
                .saturating_sub(self.work.kernel_counters.preconditioner_apps),
        );
        total
    }
}

#[derive(Default)]
struct ApplicationLedger {
    w_attempts: AtomicU64,
    w_completed: AtomicU64,
    jvp_attempts: AtomicU64,
    jvp_completed: AtomicU64,
    mass_attempts: AtomicU64,
    mass_completed: AtomicU64,
    pc_attempts: AtomicU64,
    pc_completed: AtomicU64,
}
fn tick(x: &AtomicU64) {
    let _ = x.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
        Some(v.saturating_add(1))
    });
}
impl ApplicationLedger {
    fn snapshot(&self) -> MatrixFreeOperatorWork {
        let get = |v: &AtomicU64| v.load(Ordering::Relaxed);
        MatrixFreeOperatorWork {
            w_attempts: get(&self.w_attempts),
            w_completed: get(&self.w_completed),
            jvp_attempts: get(&self.jvp_attempts),
            jvp_completed: get(&self.jvp_completed),
            mass_attempts: get(&self.mass_attempts),
            mass_completed: get(&self.mass_completed),
            preconditioner_attempts: get(&self.pc_attempts),
            preconditioner_completed: get(&self.pc_completed),
        }
    }
}

struct MatrixFreeW<'a, 'p> {
    context: &'a StepContext<'p>,
    ledger: &'a ApplicationLedger,
}
impl LinearOperator for MatrixFreeW<'_, '_> {
    fn token(&self) -> u64 {
        self.context.shifted.token()
    }
    fn dimension(&self) -> usize {
        self.context.problem.dimension
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        if x.len() != self.dimension() || y.len() != self.dimension() {
            return Err(CoreError::Dimension("matrix-free W input shape".into()));
        }
        tick(&self.ledger.w_attempts);
        tick(&self.ledger.jvp_attempts);
        self.context.jacobian.apply(x, y)?;
        tick(&self.ledger.jvp_completed);
        let hg = self.context.h * self.context.coeffs.gamma;
        if let Some(mass) = &self.context.problem.mass_matrix {
            tick(&self.ledger.mass_attempts);
            for (i, out) in y.iter_mut().enumerate() {
                let mx: f64 = mass.row(i).iter().zip(x).map(|(a, b)| a * b).sum();
                *out = mx - hg * (*out);
            }
            tick(&self.ledger.mass_completed);
        } else {
            for (out, &input) in y.iter_mut().zip(x) {
                *out = input - hg * (*out);
            }
        }
        finite(y, "matrix-free W output")?;
        tick(&self.ledger.w_completed);
        Ok(())
    }
    // Physical application metadata stays zero: the ledger charges attempts,
    // including failures, once. GMRES separately counts successful categories.
}

struct TrackedPreconditioner<'a> {
    inner: &'a dyn Preconditioner,
    ledger: &'a ApplicationLedger,
}
impl Preconditioner for TrackedPreconditioner<'_> {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        tick(&self.ledger.pc_attempts);
        self.inner.apply(x, y)?;
        tick(&self.ledger.pc_completed);
        finite(y, "preconditioner output")
    }
    fn is_identity(&self) -> bool {
        self.inner.is_identity()
    }
}

fn finite(values: &[f64], name: &str) -> CoreResult<()> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(CoreError::NonFinite(format!("{name} contains NaN/Inf")))
    }
}

/// Compute a linearized correction without assembling J, W, or a block matrix.
///
/// The supplied context must have no explicit Jacobian/W. Identity mass is
/// storage-scalable; the existing API's nonidentity mass remains a dense input.
/// `setup` is called once after preparation; it must return a fixed deterministic
/// left preconditioner, report setup work, and must not mutate the physics.
/// A reference correction and external observable budget are deliberately absent.
/// The initial caller's StepContext construction cost is outside this receipt.
pub fn run_audit2_matrix_free_correction<F>(
    context: &StepContext<'_>,
    trial_stages: &[Vec<f64>],
    config: &GmresConfig,
    setup: F,
) -> MatrixFreeCorrectionReport
where
    F: FnOnce(&StepContext<'_>, &mut WorkCounters) -> CoreResult<Box<dyn Preconditioner>>,
{
    let mut report = MatrixFreeCorrectionReport::new(config);
    let validation = (|| -> CoreResult<()> {
        config.validate()?;
        if !config.rtol.is_finite()
            || !config.atol.is_finite()
            || !context.t.is_finite()
            || !context.h.is_finite()
            || context.h == 0.0
            || !(context.h * context.coeffs.gamma).is_finite()
        {
            return Err(CoreError::InvalidInput(
                "nonfinite tolerance or context".into(),
            ));
        }
        if !context.problem.has_jvp()
            || context.jacobian.explicit().is_some()
            || context.shifted.explicit().is_some()
        {
            return Err(CoreError::InvalidInput(
                "explicit analytic-JVP matrix-free context required".into(),
            ));
        }
        if context.y.len() != context.problem.dimension
            || context.f0.len() != context.problem.dimension
            || context.ft0.len() != context.problem.dimension
        {
            return Err(CoreError::Dimension("matrix-free context shape".into()));
        }
        finite(&context.y, "context state")?;
        finite(&context.f0, "context RHS")?;
        finite(&context.ft0, "context partial t")?;
        Ok(())
    })();
    if let Err(e) = validation {
        report.fail(MatrixFreeFailurePhase::InputValidation, e);
        return report;
    }
    report.work.preparation_attempts = 1;
    let prepared = match prepare_target(context, trial_stages) {
        Ok(p) => p,
        Err(e) => {
            report.preparation_counters = e.preparation_counters;
            report.inherited_work_complete = false;
            report.fail(MatrixFreeFailurePhase::Preparation, e.message);
            return report;
        }
    };
    report.work.preparation_completed = 1;
    report.preparation_counters = prepared.preparation_counters;
    report.projection = Some(prepared.projection.clone());
    let initial_norm = rows_l2(&prepared.residual);
    if !initial_norm.is_finite() {
        report.fail(
            MatrixFreeFailurePhase::Preparation,
            "nonfinite initial residual norm",
        );
        return report;
    }
    report.initial_residual_l2 = Some(initial_norm);
    report.work.preconditioner_setup_attempts = 1;
    let pc = match setup(&prepared.context, &mut report.setup_counters) {
        Ok(pc) if pc.dimension() == context.problem.dimension => pc,
        Ok(_) => {
            report.fail(
                MatrixFreeFailurePhase::PreconditionerSetup,
                "preconditioner dimension mismatch",
            );
            return report;
        }
        Err(e) => {
            report.fail(MatrixFreeFailurePhase::PreconditionerSetup, e);
            return report;
        }
    };
    report.work.preconditioner_setup_completed = 1;
    let ledger = ApplicationLedger::default();
    let op = MatrixFreeW {
        context: &prepared.context,
        ledger: &ledger,
    };
    let tracked_pc = TrackedPreconditioner {
        inner: pc.as_ref(),
        ledger: &ledger,
    };
    let mut workspace = GmresGivensWorkspace::default();
    run_rows(
        &prepared,
        config,
        &op,
        &tracked_pc,
        &mut workspace,
        &mut report,
    );
    report.work.operator = ledger.snapshot();
    report.workspace_capacity_f64 = workspace.capacity_f64();
    if report.failure.is_some() {
        return report;
    }

    let image = match common_linear_diagnostic(
        &prepared,
        &report.correction,
        &mut report.work.correction_and_diagnostic,
    ) {
        Ok(v) => v,
        Err(e) => {
            report.inherited_work_complete = false;
            report.fail(MatrixFreeFailurePhase::LinearDiagnostic, e);
            return report;
        }
    };
    let residual: Vec<f64> = image
        .iter()
        .flatten()
        .zip(prepared.residual.iter().flatten())
        .map(|(a, b)| a - b)
        .collect();
    let norm = safe_l2(&residual);
    if !norm.is_finite() {
        report.fail(
            MatrixFreeFailurePhase::LinearDiagnostic,
            "nonfinite block residual",
        );
        return report;
    }
    report.projected_linear_residual_l2 = Some(norm);
    match nonlinear_residual_after(
        &prepared,
        trial_stages,
        &report.correction,
        &mut report.work.correction_and_diagnostic,
    ) {
        Ok(value) if value.is_finite() => report.nonlinear_residual_after_l2 = Some(value),
        Ok(_) => {
            report.fail(
                MatrixFreeFailurePhase::NonlinearDiagnostic,
                "nonfinite nonlinear diagnostic",
            );
            return report;
        }
        Err(e) => {
            report.inherited_work_complete = false;
            report.fail(MatrixFreeFailurePhase::NonlinearDiagnostic, e);
            return report;
        }
    }
    report.completed = true;
    report
}

fn run_rows(
    prepared: &PreparedTarget<'_>,
    config: &GmresConfig,
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    workspace: &mut GmresGivensWorkspace,
    report: &mut MatrixFreeCorrectionReport,
) {
    let c = &prepared.context;
    let n = c.problem.dimension;
    let mut p = vec![0.0; n];
    let mut q = vec![0.0; n];
    let mut image = vec![0.0; n];
    for i in 0..c.coeffs.stages() {
        p.fill(0.0);
        q.fill(0.0);
        for (j, z) in report.correction.iter().enumerate() {
            for k in 0..n {
                p[k] += c.coeffs.alpha[(i, j)] * z[k];
                q[k] += c.coeffs.gamma_matrix[(i, j)] * z[k];
            }
        }
        let mut rhs = prepared.residual[i].clone();
        if i > 0 {
            let update = (|| -> CoreResult<()> {
                finite(&p, "alpha correction mix")?;
                finite(&q, "gamma correction mix")?;
                let stage = c.problem.linearize_matrix_free(
                    c.t + c.coeffs.c[i] * c.h,
                    &prepared.snapshot.states[i],
                )?;
                apply_jvp_attempt(
                    stage.as_ref(),
                    &p,
                    &mut image,
                    &mut report.work.correction_and_diagnostic,
                    false,
                )?;
                finite(&image, "stage correction JVP")?;
                for k in 0..n {
                    rhs[k] += c.h * image[k];
                }
                apply_jvp_attempt(
                    c.jacobian.as_ref(),
                    &q,
                    &mut image,
                    &mut report.work.correction_and_diagnostic,
                    false,
                )?;
                finite(&image, "frozen correction JVP")?;
                for k in 0..n {
                    rhs[k] += c.h * image[k];
                }
                Ok(())
            })();
            if let Err(e) = update {
                report.fail(MatrixFreeFailurePhase::CorrectionJvp, e);
                return;
            }
        }
        if let Err(e) = finite(&rhs, "correction RHS") {
            report.fail(MatrixFreeFailurePhase::CorrectionRhs, e);
            return;
        }
        let rhs_l2 = safe_l2(&rhs);
        let threshold = config.atol.max(config.rtol * rhs_l2);
        if !rhs_l2.is_finite() || !threshold.is_finite() {
            report.fail(
                MatrixFreeFailurePhase::CorrectionRhs,
                "nonfinite norm/threshold",
            );
            return;
        }
        report.work.solve_attempts += 1;
        let linear = match solve_gmres_givens_with_workspace(
            op,
            pc,
            &rhs,
            None,
            config,
            workspace,
            &mut report.work.kernel_counters,
        ) {
            Ok(v) => v,
            Err(e) => {
                report.work.kernel_counters.linear_solve_failures += 1;
                report.fail(MatrixFreeFailurePhase::LinearSolve, e);
                return;
            }
        };
        report.work.solve_completed += 1;
        report.linear_reports.push(linear);
        let linear = report.linear_reports.last().expect("just appended");
        report.work.true_residual_attempts += 1;
        if let Err(e) = apply_counted(
            op,
            &linear.x,
            &mut image,
            &mut report.work.kernel_counters,
            ApplyCategory::Diagnostic,
        ) {
            report.fail(MatrixFreeFailurePhase::TrueResidual, e);
            return;
        }
        for k in 0..n {
            image[k] -= rhs[k];
        }
        let residual_l2 = safe_l2(&image);
        report.work.true_residual_completed += 1;
        report.rows.push(MatrixFreeRowCheck {
            stage: i,
            rhs_l2,
            threshold,
            residual_l2,
            relative_residual: if rhs_l2 == 0.0 {
                None
            } else {
                Some(residual_l2 / rhs_l2)
            },
            workspace_capacity_f64: workspace.capacity_f64(),
        });
        if !linear.converged || !residual_l2.is_finite() || residual_l2 > threshold {
            report.fail(
                MatrixFreeFailurePhase::TrueResidual,
                "unpreconditioned residual failed the supplied linear tolerance",
            );
            return;
        }
        report.correction.push(linear.x.clone());
    }
}
