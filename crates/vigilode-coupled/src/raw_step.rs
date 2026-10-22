//! Same-target raw-U dispatch and conditional algebraic-error accounting.
//!
//! The optional norm bounds are supplied by the caller, not verified here.
//! Floating point residuals and the recurrence below are not interval bounds.
use crate::algebra::{finite, norm};
use crate::{Attempt, Config, Failure, FailureKind, Work, coupled_step_residual_first};
use rodas5p_core::{
    CoreError, CoreResult, LinearSolverConfig, WorkCounters, rodas5p_coefficients,
    scaled_error_component,
};
use rodas5p_integrators::{
    AdaptiveFailureKind, OdeProblem, ParallelExecution, Rodas5pMfFastWorkspace,
};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Norm assumptions must hold for every attempted h and all line segments
/// between approximate and exact stage states, in the unweighted Euclidean norm.
#[derive(Clone, Debug, Serialize)]
pub struct L2Assumptions {
    pub inverse_norm_bound: f64,
    pub rhs_lipschitz_bound: f64,
    /// Caller-supplied allowance for each computed residual's evaluation error.
    /// Zero means no allowance; this implementation does not establish one.
    pub residual_roundoff_l2: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub enum RootErrorPolicy {
    /// Original-space residual screening plus the usual embedded estimate.
    #[default]
    Operational,
    /// Add conditional embedded perturbation and a separate endpoint budget.
    /// These are estimates conditional on externally justified assumptions.
    ConditionalL2 {
        assumptions: L2Assumptions,
        max_endpoint_wrms: f64,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct ConditionalErrorEstimate {
    pub stage_l2: [f64; 8],
    pub endpoint_l2: f64,
    pub embedded_l2: f64,
    pub endpoint_wrms: f64,
    pub embedded_wrms: f64,
    pub authority: &'static str,
}

fn input(message: &str) -> Failure {
    Failure::new(FailureKind::Input, message)
}
fn validate_assumptions(a: &L2Assumptions) -> crate::Result<()> {
    if !a.inverse_norm_bound.is_finite()
        || a.inverse_norm_bound <= 0.0
        || !a.rhs_lipschitz_bound.is_finite()
        || a.rhs_lipschitz_bound < 0.0
        || !a.residual_roundoff_l2.is_finite()
        || a.residual_roundoff_l2 < 0.0
    {
        return Err(input(
            "conditional L2 assumptions must be finite and nonnegative, with positive inverse bound",
        ));
    }
    Ok(())
}
fn validate_policy(policy: &RootErrorPolicy) -> crate::Result<()> {
    if let RootErrorPolicy::ConditionalL2 {
        assumptions,
        max_endpoint_wrms,
    } = policy
    {
        validate_assumptions(assumptions)?;
        if !max_endpoint_wrms.is_finite() || *max_endpoint_wrms <= 0.0 || *max_endpoint_wrms > 1.0 {
            return Err(input("conditional endpoint budget must lie in (0, 1]"));
        }
    }
    Ok(())
}

/// Evaluate the finite causal recurrence for the eight raw-U stages.
/// No small-coupling assumption is needed: the strictly lower triangular
/// propagation matrix is nilpotent. Its entries can nevertheless be very large.
/// The scale is held fixed for this estimate. A zero scale is allowed only if
/// the entire corresponding scalar output bound is zero.
pub fn conditional_l2_error_estimate(
    h: f64,
    residual_l2: [f64; 8],
    assumptions: &L2Assumptions,
    scale: &[f64],
) -> crate::Result<ConditionalErrorEstimate> {
    validate_assumptions(assumptions)?;
    if !h.is_finite()
        || h <= 0.0
        || scale.is_empty()
        || scale.iter().any(|s| !s.is_finite() || *s < 0.0)
        || residual_l2.iter().any(|r| !r.is_finite() || *r < 0.0)
    {
        return Err(input(
            "invalid step, scale, or stage residual for conditional bound",
        ));
    }
    let c = rodas5p_coefficients().map_err(|e| input(&e.to_string()))?;
    let mut stage_l2 = [0.0; 8];
    for i in 0..8 {
        let mut rhs = residual_l2[i] + assumptions.residual_roundoff_l2;
        for j in 0..i {
            let coupling = (c.gamma * c.c_matrix[(i, j)]).abs()
                + (h * c.gamma).abs() * assumptions.rhs_lipschitz_bound * c.a[(i, j)].abs();
            rhs += coupling * stage_l2[j];
        }
        stage_l2[i] = assumptions.inverse_norm_bound * rhs;
        if !stage_l2[i].is_finite() {
            return Err(Failure::new(
                FailureKind::NonFinite,
                "conditional causal recurrence overflow",
            ));
        }
    }
    let endpoint_l2: f64 = c
        .b_code
        .iter()
        .zip(stage_l2)
        .map(|(b, e)| b.abs() * e)
        .sum();
    let embedded_l2 = stage_l2[7];
    let min_scale = scale.iter().copied().fold(f64::INFINITY, f64::min);
    let to_wrms = |v: f64| {
        if v == 0.0 {
            0.0
        } else {
            v / min_scale / (scale.len() as f64).sqrt()
        }
    };
    let endpoint_wrms = to_wrms(endpoint_l2);
    let embedded_wrms = to_wrms(embedded_l2);
    if !endpoint_l2.is_finite() || !endpoint_wrms.is_finite() || !embedded_wrms.is_finite() {
        return Err(Failure::new(
            FailureKind::NonFinite,
            "conditional output bound is unrepresentable in requested scale",
        ));
    }
    Ok(ConditionalErrorEstimate {
        stage_l2,
        endpoint_l2,
        embedded_l2,
        endpoint_wrms,
        embedded_wrms,
        authority: "conditional-f64-estimate-user-assumptions-not-certified",
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct RawSelectedStep {
    pub selection: &'static str,
    pub target_id: &'static str,
    pub y_new: Vec<f64>,
    pub error_vector: Vec<f64>,
    pub embedded_wrms: f64,
    /// Driver accepts only <= 1. This includes the operational residual screen
    /// and, when requested, the conditional root and embedded error budgets.
    pub effective_error: f64,
    pub full_residual_wrms: f64,
    pub conditional_bound: Option<ConditionalErrorEstimate>,
}

#[derive(Debug, Default, Serialize)]
pub struct RawStepAttempt {
    pub selected: Option<RawSelectedStep>,
    pub candidate_attempt: Attempt,
    pub candidate_bound: Option<ConditionalErrorEstimate>,
    pub fallback_attempted: bool,
    pub fallback_counters: WorkCounters,
    /// Additional same-target validation is charged separately from GMRES.
    pub fallback_validation_work: Work,
    pub terminal_failure: Option<Failure>,
    /// Only trusted solver/conditional-estimate failures permit a smaller-h
    /// retry. Callback errors retain their original text but never set this.
    pub retryable_failure: bool,
    /// Trusted source category for adaptive counters, distinct from a user
    /// callback's arbitrary CoreError variant.
    pub retry_failure_kind: Option<AdaptiveFailureKind>,
}

/// Reuses allocated fallback buffers, never a stale operator or rejected-step
/// history. Every fallback refreshes the frozen operator; recycling is disabled.
pub struct RawStepSession {
    fallback: Rodas5pMfFastWorkspace,
    dimension: usize,
    pending_fallback: bool,
}
impl RawStepSession {
    pub fn new(p: &OdeProblem, linear: &LinearSolverConfig) -> CoreResult<Self> {
        if !p.autonomous {
            return Err(CoreError::InvalidInput(
                "coupled raw session requires an autonomous ODE".into(),
            ));
        }
        Ok(Self {
            fallback: Rodas5pMfFastWorkspace::new(p, linear)?,
            dimension: p.dimension,
            pending_fallback: false,
        })
    }

    /// Must be called only after the outer adaptive driver commits this attempt.
    pub fn commit_accepted(&mut self, h: f64, error: f64) {
        if self.pending_fallback {
            self.fallback.record_accepted_step(h);
            self.fallback.record_accepted_error(error);
        } else {
            self.fallback.clear_accepted_step();
        }
        self.pending_fallback = false;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn attempt(
        &mut self,
        p: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        cfg: &Config,
        execution: &ParallelExecution,
        policy: &RootErrorPolicy,
    ) -> RawStepAttempt {
        self.pending_fallback = false;
        let mut out = RawStepAttempt::default();
        if p.dimension != self.dimension {
            out.terminal_failure = Some(input("raw session/problem dimensions differ"));
            return out;
        }
        if !p.autonomous || p.mass_matrix.is_some() || !p.supports_matrix_free_jvp() {
            out.terminal_failure = Some(Failure::new(
                FailureKind::Unsupported,
                "raw session requires autonomous identity-mass ODE with supplied JVP",
            ));
            return out;
        }
        // A storage-budget decline can precede the candidate kernel's own
        // h*gamma check. The common input contract must therefore guard this
        // before either algorithm can be selected.
        let gamma = match rodas5p_coefficients() {
            Ok(c) => c.gamma,
            Err(e) => {
                out.terminal_failure = Some(input(&e.to_string()));
                return out;
            }
        };
        if !(h * gamma).is_finite() || h * gamma <= 0.0 {
            out.terminal_failure = Some(input(
                "raw step requires a finite positive representable h*gamma",
            ));
            return out;
        }
        if let Err(e) = validate_policy(policy) {
            out.terminal_failure = Some(e);
            return out;
        }
        let epoch = p.model_epoch();
        out.candidate_attempt = coupled_step_residual_first(p, t, y, h, cfg, execution);
        if p.model_epoch() != epoch {
            out.terminal_failure = Some(Failure::new(
                FailureKind::ModelChanged,
                "model changed before raw dispatch",
            ));
            return out;
        }
        if let Some(c) = out.candidate_attempt.candidate.as_ref() {
            let scale = scale(y, &c.y_new, cfg);
            match assess(policy, h, c.full_residual_l2, &scale, c.embedded_wrms) {
                Ok((effective, bound)) => {
                    out.candidate_bound = bound.clone();
                    let root_budget_passes = match (&bound, policy) {
                        (
                            Some(b),
                            RootErrorPolicy::ConditionalL2 {
                                max_endpoint_wrms, ..
                            },
                        ) => b.endpoint_wrms <= *max_endpoint_wrms,
                        _ => true,
                    };
                    if root_budget_passes {
                        out.selected = Some(RawSelectedStep {
                            selection: "coupled-raw-u",
                            target_id: crate::TARGET_ID,
                            y_new: c.y_new.clone(),
                            error_vector: c.error_vector.clone(),
                            embedded_wrms: c.embedded_wrms,
                            effective_error: effective,
                            full_residual_wrms: c.full_residual_wrms,
                            conditional_bound: bound,
                        });
                        return out;
                    }
                }
                Err(e) if e.kind == FailureKind::Input => {
                    out.terminal_failure = Some(e);
                    return out;
                }
                Err(_) => {} // Unusable conditional estimate tries the independent raw solve.
            }
        } else if !out.candidate_attempt.failure.as_ref().is_some_and(|e| {
            matches!(
                e.kind,
                FailureKind::Budget
                    | FailureKind::ProjectionLeakage
                    | FailureKind::ProjectionSingular
                    | FailureKind::RootIteration
            )
        }) {
            out.terminal_failure = out.candidate_attempt.failure.clone();
            return out;
        }
        out.fallback_attempted = true;
        // Distinguish an error emitted by user code from one emitted by the
        // Krylov solver, even if a callback chooses CoreError::LinearSolve.
        let callback_failed = Arc::new(AtomicBool::new(false));
        let guarded = match guarded_problem(p, callback_failed.clone()) {
            Ok(p) => p,
            Err(e) => {
                out.terminal_failure = Some(input(&e.to_string()));
                return out;
            }
        };
        let fallback_result = self.fallback.attempt(
            &guarded,
            t,
            y,
            h,
            true,
            None,
            cfg.atol,
            cfg.rtol,
            &mut out.fallback_counters,
        );
        if p.model_epoch() != epoch {
            out.terminal_failure = Some(Failure::new(
                FailureKind::ModelChanged,
                "model changed in raw fallback",
            ));
            return out;
        }
        let embedded = match fallback_result {
            Ok(e) => e,
            Err(e) => {
                let kind = if callback_failed.load(Ordering::SeqCst) {
                    FailureKind::Callback
                } else {
                    match &e {
                        CoreError::LinearSolve(_) => {
                            out.retryable_failure = true;
                            out.retry_failure_kind = Some(AdaptiveFailureKind::LinearSolve);
                            FailureKind::RootIteration
                        }
                        CoreError::NonlinearSolve(_) => {
                            out.retryable_failure = true;
                            out.retry_failure_kind = Some(AdaptiveFailureKind::NonlinearSolve);
                            FailureKind::RootIteration
                        }
                        CoreError::NonFinite(_) => {
                            out.retryable_failure = true;
                            out.retry_failure_kind = Some(AdaptiveFailureKind::NonFinite);
                            FailureKind::NonFinite
                        }
                        _ => FailureKind::Input,
                    }
                };
                out.terminal_failure =
                    Some(Failure::new(kind, format!("raw fallback failed: {e}")));
                return out;
            }
        };
        let yn = self.fallback.y_new().to_vec();
        let stages: Vec<_> = (0..8).map(|i| self.fallback.stage(i).to_vec()).collect();
        let scale = scale(y, &yn, cfg);
        let checked = validate_raw(
            p,
            t,
            y,
            h,
            &stages,
            &scale,
            epoch,
            &mut out.fallback_validation_work,
        );
        let (residuals, full_residual_wrms) = match checked {
            Ok(x) => x,
            Err(e) => {
                out.terminal_failure = Some(e);
                return out;
            }
        };
        let (effective, bound) = match assess(policy, h, residuals, &scale, embedded) {
            Ok(x) => x,
            Err(e) => {
                out.retryable_failure = e.kind == FailureKind::NonFinite;
                if out.retryable_failure {
                    out.retry_failure_kind = Some(AdaptiveFailureKind::NonFinite);
                }
                out.terminal_failure = Some(e);
                return out;
            }
        };
        out.selected = Some(RawSelectedStep {
            selection: "sequential-raw-u-fallback",
            target_id: crate::TARGET_ID,
            y_new: yn,
            error_vector: stages[7].clone(),
            embedded_wrms: embedded,
            effective_error: effective.max(full_residual_wrms / cfg.full_residual_wrms_limit),
            full_residual_wrms,
            conditional_bound: bound,
        });
        self.pending_fallback = true;
        out
    }
}

fn guarded_problem(p: &OdeProblem, failed: Arc<AtomicBool>) -> CoreResult<OdeProblem> {
    let original = Arc::new(p.clone());
    let rhs_p = original.clone();
    let rhs_failed = failed.clone();
    let jvp = p.jvp_callback().expect("capability checked").clone();
    let mut guarded = OdeProblem::new(
        p.name.clone(),
        p.dimension,
        Arc::new(move |t, y, out| {
            out.fill(f64::NAN);
            let result = rhs_p.eval_rhs_into(t, y, out, &mut WorkCounters::default());
            if result.is_err() {
                rhs_failed.store(true, Ordering::SeqCst);
            }
            result
        }),
        None,
        None,
        Some(Arc::new(move |t, y, v, out| {
            out.fill(f64::NAN);
            let result = jvp(t, y, v, out).and_then(|()| {
                if out.iter().all(|v| v.is_finite()) {
                    Ok(())
                } else {
                    Err(CoreError::NonFinite(
                        "JVP produced nonfinite or incomplete output".into(),
                    ))
                }
            });
            if result.is_err() {
                failed.store(true, Ordering::SeqCst);
            }
            result
        })),
        None,
        true,
        None,
        None,
    )?;
    if p.model_epoch().is_some() {
        guarded = guarded.with_model_epoch(Arc::new(move || original.model_epoch().unwrap_or(0)));
    }
    Ok(guarded)
}

fn scale(y: &[f64], yn: &[f64], cfg: &Config) -> Vec<f64> {
    y.iter()
        .zip(yn)
        .map(|(a, b)| cfg.atol + cfg.rtol * a.abs().max(b.abs()))
        .collect()
}
fn assess(
    policy: &RootErrorPolicy,
    h: f64,
    residuals: [f64; 8],
    scale: &[f64],
    embedded: f64,
) -> crate::Result<(f64, Option<ConditionalErrorEstimate>)> {
    match policy {
        RootErrorPolicy::Operational => Ok((embedded, None)),
        RootErrorPolicy::ConditionalL2 {
            assumptions,
            max_endpoint_wrms,
        } => {
            let b = conditional_l2_error_estimate(h, residuals, assumptions, scale)?;
            Ok((
                (embedded + b.embedded_wrms).max(b.endpoint_wrms / max_endpoint_wrms),
                Some(b),
            ))
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn validate_raw(
    p: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    u: &[Vec<f64>],
    scale: &[f64],
    epoch: Option<u64>,
    work: &mut Work,
) -> crate::Result<([f64; 8], f64)> {
    let c = rodas5p_coefficients().map_err(|e| input(&e.to_string()))?;
    let op = p
        .linearize_matrix_free(t, y)
        .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
    let mut residuals = [0.0; 8];
    let mut maximum = 0.0_f64;
    work.validation_batches += 1;
    for i in 0..8 {
        work.stage_tasks_scheduled += 1;
        let mut state = y.to_vec();
        let mut mix = vec![0.0; y.len()];
        for j in 0..i {
            for k in 0..y.len() {
                state[k] += c.a[(i, j)] * u[j][k];
                mix[k] += c.c_matrix[(i, j)] * u[j][k];
            }
        }
        finite(&state)?;
        let mut rhs = vec![f64::NAN; y.len()];
        work.rhs_attempted += 1;
        p.eval_rhs_into(t, &state, &mut rhs, &mut WorkCounters::default())
            .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
        finite(&rhs)?;
        work.rhs_completed += 1;
        let mut ju = vec![f64::NAN; y.len()];
        work.jvp_attempted += 1;
        work.validation_jvp += 1;
        op.apply(&u[i], &mut ju)
            .map_err(|e| Failure::new(FailureKind::Callback, e.to_string()))?;
        finite(&ju)?;
        work.jvp_completed += 1;
        let r: Vec<_> = (0..y.len())
            .map(|k| u[i][k] - h * c.gamma * ju[k] - c.gamma * mix[k] - h * c.gamma * rhs[k])
            .collect();
        finite(&r)?;
        residuals[i] = norm(&r);
        let weighted: Vec<_> = r
            .iter()
            .zip(scale)
            .map(|(v, s)| scaled_error_component(*v, *s))
            .collect();
        maximum = maximum.max(norm(&weighted) / (y.len() as f64).sqrt());
        work.stage_tasks_completed += 1;
        if p.model_epoch() != epoch {
            return Err(Failure::new(
                FailureKind::ModelChanged,
                "model changed in raw residual validation",
            ));
        }
    }
    Ok((residuals, maximum))
}
