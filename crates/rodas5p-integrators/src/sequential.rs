use std::sync::Arc;

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, IdentityPreconditioner, InitialGuess,
    JacobiPreconditioner, LinearMethod, LinearOperator, LinearSolveReport, LinearSolverConfig,
    LuFactorization, Preconditioner, PreconditionerKind, Rodas5pCoefficients, ShiftedOperator,
    WorkCounters, apply_counted, apply_jvp_counted, error_scale, load_rodas5p_coefficients,
    safe_l2, wrms,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GmresConfig, LgmresConfig, LgmresState, solve_gcrodr,
    solve_gcrodr_with_residual_scale, solve_gmres, solve_gmres_with_residual_scale, solve_lgmres,
    solve_lgmres_with_residual_scale,
};

use crate::{OdeProblem, RODAS5P_INNER_FORCING_FLOOR, rodas5p_inner_forcing_target};

#[derive(Clone, Debug, PartialEq)]
pub enum KrylovState {
    Lgmres(LgmresState),
    Gcrodr(GcrodrState),
}

impl KrylovState {
    pub fn for_method(method: LinearMethod) -> Option<Self> {
        match method {
            LinearMethod::Lgmres => Some(Self::Lgmres(LgmresState::default())),
            LinearMethod::Gcrodr => Some(Self::Gcrodr(GcrodrState::default())),
            _ => None,
        }
    }
    pub fn invalidate_products(&mut self) {
        match self {
            Self::Lgmres(s) => {
                for image in &mut s.images {
                    *image = None;
                }
                s.operator_token = None;
                s.system_identity = None;
                s.previous_solution = None;
            }
            Self::Gcrodr(s) => {
                s.image.clear();
                s.operator_token = None;
                s.system_identity = None;
                s.previous_solution = None;
            }
        }
    }
}

pub struct StepContext<'a> {
    pub problem: &'a OdeProblem,
    pub t: f64,
    pub y: Vec<f64>,
    pub h: f64,
    pub coeffs: Rodas5pCoefficients,
    pub f0: Vec<f64>,
    pub ft0: Vec<f64>,
    pub jacobian: Arc<dyn LinearOperator>,
    pub shifted: ShiftedOperator,
}

#[derive(Clone, Debug)]
pub struct StageSolveData {
    pub stages: Vec<Vec<f64>>,
    pub reports: Vec<LinearSolveReport>,
    pub stage_states: Vec<Vec<f64>>,
    pub stage_rhs_values: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StageInnerForcingReport {
    pub stage_index: usize,
    pub flow_wrms: f64,
    pub rhs_wrms: f64,
    pub eta: f64,
    pub tau: f64,
    pub achieved_residual_wrms: f64,
}

#[derive(Clone, Debug)]
pub struct InnerForcedStepResult {
    pub step: StepResult,
    pub stage_forcing: Vec<StageInnerForcingReport>,
}

#[derive(Clone, Debug)]
pub struct InnerForcedStageSolveData {
    pub stage_data: StageSolveData,
    pub stage_forcing: Vec<StageInnerForcingReport>,
}

#[derive(Clone, Debug, Default)]
pub struct StepCertificate {
    pub accepted: bool,
    pub reason: String,
    pub iterations: usize,
    pub embedded_error: f64,
    pub fixed_point_error: f64,
    pub residual_proxy_error: f64,
    pub contraction_tail_error: Option<f64>,
    pub observed_contraction: Option<f64>,
    pub stage_residual_norm: f64,
    pub stage_relative_residual: f64,
}

#[derive(Clone, Debug)]
pub struct StepResult {
    pub t_old: f64,
    pub t_new: f64,
    pub y_old: Vec<f64>,
    pub y_new: Vec<f64>,
    pub h: f64,
    pub stages: Vec<Vec<f64>>,
    pub error_vector: Vec<f64>,
    pub error_norm: f64,
    pub accepted: bool,
    pub method: String,
    pub used_fallback: bool,
    pub certificate: Option<StepCertificate>,
    pub counters: WorkCounters,
}

struct FactorPreconditioner {
    factor: LuFactorization,
}
impl Preconditioner for FactorPreconditioner {
    fn dimension(&self) -> usize {
        self.factor.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let sol = self.factor.solve(x)?;
        y.copy_from_slice(&sol);
        Ok(())
    }
}

pub fn build_step_context<'a>(
    problem: &'a OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    counters: &mut WorkCounters,
) -> CoreResult<StepContext<'a>> {
    if !h.is_finite() || h == 0.0 {
        return Err(CoreError::InvalidInput(
            "step size must be finite and nonzero".into(),
        ));
    }
    if y.len() != problem.dimension || !y.iter().all(|v| v.is_finite()) {
        return Err(CoreError::InvalidInput("invalid initial state".into()));
    }
    let coeffs = load_rodas5p_coefficients()?;
    let f0 = problem.eval_rhs(t, y, counters)?;
    let ft0 = problem.eval_partial_t(t, y, counters)?;
    let jacobian = problem.linearize(t, y, counters)?;
    let shifted = ShiftedOperator::new(
        problem.mass_matrix.clone(),
        jacobian.clone(),
        h,
        coeffs.gamma,
    )?;
    Ok(StepContext {
        problem,
        t,
        y: y.to_vec(),
        h,
        coeffs,
        f0,
        ft0,
        jacobian,
        shifted,
    })
}

/// Build the RODAS5P step context without materializing an explicit Jacobian.
///
/// The RHS and time-derivative accounting is identical to [`build_step_context`], but the frozen
/// linearization is required to come from a user-provided JVP.  Consequently the shifted operator
/// exposes no explicit matrix and any direct-factorization request fails closed.
pub fn build_step_context_matrix_free<'a>(
    problem: &'a OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    counters: &mut WorkCounters,
) -> CoreResult<StepContext<'a>> {
    if !h.is_finite() || h == 0.0 {
        return Err(CoreError::InvalidInput(
            "step size must be finite and nonzero".into(),
        ));
    }
    if y.len() != problem.dimension || !y.iter().all(|value| value.is_finite()) {
        return Err(CoreError::InvalidInput("invalid initial state".into()));
    }
    if !problem.supports_matrix_free_jvp() {
        return Err(CoreError::InvalidInput(
            "strict matrix-free integration requires a user-supplied JVP".into(),
        ));
    }
    let coeffs = load_rodas5p_coefficients()?;
    let f0 = problem.eval_rhs(t, y, counters)?;
    let ft0 = problem.eval_partial_t(t, y, counters)?;
    let jacobian = problem.linearize_matrix_free(t, y)?;
    let shifted = ShiftedOperator::new_counted_jvp(
        problem.mass_matrix.clone(),
        jacobian.clone(),
        h,
        coeffs.gamma,
    )?;
    debug_assert!(shifted.explicit().is_none());
    Ok(StepContext {
        problem,
        t,
        y: y.to_vec(),
        h,
        coeffs,
        f0,
        ft0,
        jacobian,
        shifted,
    })
}

fn row_combination(weights: &[f64], rows: &[Vec<f64>], n: usize) -> Vec<f64> {
    let mut out = vec![0.0; n];
    for (&a, row) in weights.iter().zip(rows) {
        for i in 0..n {
            out[i] += a * row[i];
        }
    }
    out
}

fn make_pc(
    context: &StepContext<'_>,
    kind: PreconditionerKind,
    factor: &Option<LuFactorization>,
) -> CoreResult<Box<dyn Preconditioner>> {
    let n = context.problem.dimension;
    match kind {
        PreconditionerKind::None => Ok(Box::new(IdentityPreconditioner::new(n))),
        PreconditionerKind::Jacobi => {
            let w = context.shifted.explicit().ok_or_else(|| {
                CoreError::LinearSolve("Jacobi preconditioner needs explicit W".into())
            })?;
            Ok(Box::new(JacobiPreconditioner::from_matrix(w)?))
        }
        PreconditionerKind::Direct => {
            let f = factor.clone().ok_or_else(|| {
                CoreError::LinearSolve("direct preconditioner needs explicit W".into())
            })?;
            Ok(Box::new(FactorPreconditioner { factor: f }))
        }
    }
}

fn direct_report(
    context: &StepContext<'_>,
    factor: &LuFactorization,
    rhs: &[f64],
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    let before = *counters;
    counters.linear_solves += 1;
    counters.direct_solve_calls += 1;
    let x = factor.solve(rhs)?;
    let mut ax = vec![0.0; rhs.len()];
    apply_counted(
        &context.shifted,
        &x,
        &mut ax,
        counters,
        ApplyCategory::Diagnostic,
    )?;
    let residual: Vec<f64> = rhs.iter().zip(ax).map(|(b, a)| b - a).collect();
    let rn = safe_l2(&residual);
    let bn = safe_l2(rhs);
    let d = counters.delta(before);
    Ok(LinearSolveReport {
        x,
        converged: true,
        info: 0,
        residual_norm: rn,
        relative_residual: rn / bn.max(f64::MIN_POSITIVE),
        iterations: 0,
        matvecs: d.linear_matvecs,
        preconditioner_apps: d.preconditioner_apps,
        method: "direct".into(),
    })
}

fn sequential_stages_impl(
    context: &StepContext<'_>,
    config: &LinearSolverConfig,
    mut recycle: Option<&mut KrylovState>,
    outer_tolerances: Option<(f64, f64)>,
    counters: &mut WorkCounters,
) -> CoreResult<InnerForcedStageSolveData> {
    config.validate().map_err(CoreError::InvalidInput)?;
    if outer_tolerances.is_some() && config.method == LinearMethod::Direct {
        return Err(CoreError::InvalidInput(
            "RODAS5P inner forcing requires a Krylov linear solver".into(),
        ));
    }
    let s = context.coeffs.stages();
    let n = context.problem.dimension;
    let mut stages = vec![vec![0.0; n]; s];
    let mut states = vec![vec![0.0; n]; s];
    let mut fvals = vec![vec![0.0; n]; s];
    let mut reports = Vec::with_capacity(s);
    let mut stage_forcing = Vec::with_capacity(if outer_tolerances.is_some() { s } else { 0 });
    let output_weight_l1 = context
        .coeffs
        .b
        .iter()
        .map(|weight| weight.abs())
        .sum::<f64>();
    let need_factor = config.method == LinearMethod::Direct
        || config.preconditioner == PreconditionerKind::Direct;
    let factor = if need_factor {
        let w = context.shifted.explicit().ok_or_else(|| {
            CoreError::LinearSolve("direct action needs explicit shifted matrix".into())
        })?;
        counters.direct_factorizations += 1;
        Some(LuFactorization::new(w)?)
    } else {
        None
    };
    let pc = make_pc(context, config.preconditioner, &factor)?;
    let mut owned_state = KrylovState::for_method(config.method);
    let state: &mut Option<&mut KrylovState> = &mut recycle;
    for i in 0..s {
        let delta = if i == 0 {
            vec![0.0; n]
        } else {
            row_combination(&context.coeffs.alpha.row(i)[..i], &stages[..i], n)
        };
        let yi: Vec<f64> = context.y.iter().zip(&delta).map(|(a, b)| a + b).collect();
        states[i] = yi.clone();
        let fi = if i == 0 {
            context.f0.clone()
        } else {
            context
                .problem
                .eval_rhs(context.t + context.coeffs.c[i] * context.h, &yi, counters)?
        };
        fvals[i] = fi.clone();
        let gmix = if i == 0 {
            vec![0.0; n]
        } else {
            row_combination(&context.coeffs.gamma_matrix.row(i)[..i], &stages[..i], n)
        };
        let mut jg = vec![0.0; n];
        if gmix.iter().any(|v| *v != 0.0) {
            apply_jvp_counted(context.jacobian.as_ref(), &gmix, &mut jg, counters)?;
        }
        let mut rhs = vec![0.0; n];
        for q in 0..n {
            rhs[q] = context.h * fi[q]
                + context.h * jg[q]
                + context.h * context.h * context.coeffs.gamma_rows[i] * context.ft0[q];
        }
        let forcing = if let Some((outer_atol, outer_rtol)) = outer_tolerances {
            let scale = error_scale(&context.y, &yi, &[outer_atol], outer_rtol)?;
            let flow_wrms = context.h.abs() * wrms(&fi, &scale)?;
            let rhs_wrms = wrms(&rhs, &scale)?;
            let target = rodas5p_inner_forcing_target(flow_wrms, rhs_wrms, output_weight_l1)?;
            Some((scale, flow_wrms, rhs_wrms, target))
        } else {
            None
        };
        let residual_scale = forcing.as_ref().map(|(scale, _, _, _)| scale.as_slice());
        let linear_rtol = forcing
            .as_ref()
            .map_or(config.rtol, |(_, _, _, target)| target.eta);
        let linear_atol = if forcing.is_some() {
            RODAS5P_INNER_FORCING_FLOOR
        } else {
            config.atol
        };
        if forcing.is_some() {
            counters.forced_stage_solves = counters.forced_stage_solves.saturating_add(1);
        }
        let x0 = if i > 0
            && config.method != LinearMethod::Direct
            && config.x0_strategy == InitialGuess::Previous
        {
            Some(stages[i - 1].as_slice())
        } else {
            None
        };
        let report = match config.method {
            LinearMethod::Direct => direct_report(
                context,
                factor.as_ref().expect("factor created"),
                &rhs,
                counters,
            )?,
            LinearMethod::Gmres => {
                let gmres = GmresConfig {
                    restart: config.restart,
                    max_arnoldi: config.maxiter.max(config.restart),
                    rtol: linear_rtol,
                    atol: linear_atol,
                };
                match residual_scale {
                    Some(scale) => solve_gmres_with_residual_scale(
                        &context.shifted,
                        pc.as_ref(),
                        &rhs,
                        x0,
                        &gmres,
                        Some(scale),
                        counters,
                    )?,
                    None => solve_gmres(&context.shifted, pc.as_ref(), &rhs, x0, &gmres, counters)?,
                }
            }
            LinearMethod::Lgmres => {
                let st = match state.as_deref_mut() {
                    Some(KrylovState::Lgmres(s)) => s,
                    Some(_) => {
                        return Err(CoreError::InvalidInput(
                            "LGMRES received GCRO-DR state".into(),
                        ));
                    }
                    None => match owned_state.as_mut() {
                        Some(KrylovState::Lgmres(s)) => s,
                        _ => unreachable!(),
                    },
                };
                let lgmres = LgmresConfig {
                    inner_m: config.inner_m,
                    max_outer: config.maxiter,
                    outer_k: config.outer_k,
                    rtol: linear_rtol,
                    atol: linear_atol,
                };
                match residual_scale {
                    Some(scale) => solve_lgmres_with_residual_scale(
                        &context.shifted,
                        pc.as_ref(),
                        &rhs,
                        x0,
                        &lgmres,
                        st,
                        Some(scale),
                        counters,
                    )?,
                    None => solve_lgmres(
                        &context.shifted,
                        pc.as_ref(),
                        &rhs,
                        x0,
                        &lgmres,
                        st,
                        counters,
                    )?,
                }
            }
            LinearMethod::Gcrodr => {
                let st = match state.as_deref_mut() {
                    Some(KrylovState::Gcrodr(s)) => s,
                    Some(_) => {
                        return Err(CoreError::InvalidInput(
                            "GCRO-DR received LGMRES state".into(),
                        ));
                    }
                    None => match owned_state.as_mut() {
                        Some(KrylovState::Gcrodr(s)) => s,
                        _ => unreachable!(),
                    },
                };
                let gcrodr = GcrodrConfig {
                    restart: config.restart,
                    max_arnoldi: config.maxiter.max(config.restart),
                    recycle_dim: config.recycle_dim,
                    rank_tol: config.recycle_rank_tol,
                    rtol: linear_rtol,
                    atol: linear_atol,
                };
                match residual_scale {
                    Some(scale) => solve_gcrodr_with_residual_scale(
                        &context.shifted,
                        pc.as_ref(),
                        &rhs,
                        x0,
                        &gcrodr,
                        st,
                        Some(scale),
                        counters,
                    )?,
                    None => solve_gcrodr(
                        &context.shifted,
                        pc.as_ref(),
                        &rhs,
                        x0,
                        &gcrodr,
                        st,
                        counters,
                    )?,
                }
            }
        };
        if let Some((_, flow_wrms, rhs_wrms, target)) = forcing {
            if report.residual_norm > target.tau {
                return Err(CoreError::LinearSolve(format!(
                    "RODAS5P stage {i} achieved WRMS residual {:.3e} exceeds forcing target {:.3e}",
                    report.residual_norm, target.tau
                )));
            }
            stage_forcing.push(StageInnerForcingReport {
                stage_index: i,
                flow_wrms,
                rhs_wrms,
                eta: target.eta,
                tau: target.tau,
                achieved_residual_wrms: report.residual_norm,
            });
        }
        stages[i] = report.x.clone();
        reports.push(report);
    }
    if !stages.iter().flatten().all(|v| v.is_finite()) {
        return Err(CoreError::NonFinite("non-finite RODAS5P stages".into()));
    }
    Ok(InnerForcedStageSolveData {
        stage_data: StageSolveData {
            stages,
            reports,
            stage_states: states,
            stage_rhs_values: fvals,
        },
        stage_forcing,
    })
}

pub fn sequential_stages(
    context: &StepContext<'_>,
    config: &LinearSolverConfig,
    recycle: Option<&mut KrylovState>,
    counters: &mut WorkCounters,
) -> CoreResult<StageSolveData> {
    sequential_stages_impl(context, config, recycle, None, counters).map(|data| data.stage_data)
}

#[allow(clippy::too_many_arguments)]
pub fn sequential_stages_with_inner_forcing(
    context: &StepContext<'_>,
    config: &LinearSolverConfig,
    recycle: Option<&mut KrylovState>,
    outer_atol: f64,
    outer_rtol: f64,
    counters: &mut WorkCounters,
) -> CoreResult<InnerForcedStageSolveData> {
    sequential_stages_impl(
        context,
        config,
        recycle,
        Some((outer_atol, outer_rtol)),
        counters,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn finish_step(
    context: &StepContext<'_>,
    stages: Vec<Vec<f64>>,
    atol: f64,
    rtol: f64,
    method: String,
    accepted: Option<bool>,
    used_fallback: bool,
    certificate: Option<StepCertificate>,
    before: WorkCounters,
    counters: &WorkCounters,
) -> CoreResult<StepResult> {
    let n = context.problem.dimension;
    let update = row_combination(&context.coeffs.b, &stages, n);
    let y_new: Vec<f64> = context.y.iter().zip(update).map(|(a, b)| a + b).collect();
    let error_vector = row_combination(&context.coeffs.btilde, &stages, n);
    let scale = error_scale(&context.y, &y_new, &[atol], rtol)?;
    let error_norm = wrms(&error_vector, &scale)?;
    let accepted = accepted.unwrap_or(error_norm.is_finite() && error_norm <= 1.0)
        && y_new.iter().all(|v| v.is_finite());
    Ok(StepResult {
        t_old: context.t,
        t_new: context.t + context.h,
        y_old: context.y.clone(),
        y_new,
        h: context.h,
        stages,
        error_vector,
        error_norm,
        accepted,
        method,
        used_fallback,
        certificate,
        counters: counters.delta(before),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn sequential_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &LinearSolverConfig,
    mut recycle: Option<&mut KrylovState>,
    atol: f64,
    rtol: f64,
    force_accept: bool,
    counters: &mut WorkCounters,
) -> CoreResult<StepResult> {
    let before = *counters;
    let snapshot = recycle.as_deref().cloned();
    let result = (|| {
        let context = build_step_context(problem, t, y, h, counters)?;
        let data = sequential_stages(&context, config, recycle.as_deref_mut(), counters)?;
        let mut r = finish_step(
            &context,
            data.stages,
            atol,
            rtol,
            format!("RODAS5P-sequential-{:?}", config.method),
            if force_accept { Some(true) } else { None },
            false,
            None,
            before,
            counters,
        )?;
        if r.accepted {
            counters.accepted_steps += 1;
        } else {
            counters.rejected_steps += 1;
        }
        r.counters = counters.delta(before);
        Ok(r)
    })();
    if result.as_ref().map_or(true, |r| !r.accepted)
        && let (Some(target), Some(saved)) = (recycle, snapshot)
    {
        *target = saved;
    }
    result
}
/// Execute one protected sequential RODAS5P step through a strict matrix-free JVP operator.
///
/// This path is the load-bearing comparator for the generic vectorized/JF branch.  It never
/// materializes an explicit Jacobian or shifted matrix, and it counts every shifted-operator
/// application as one JVP (and one mass matvec when a mass matrix is present).
#[allow(clippy::too_many_arguments)]
pub fn sequential_matrix_free_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &LinearSolverConfig,
    mut recycle: Option<&mut KrylovState>,
    atol: f64,
    rtol: f64,
    force_accept: bool,
    counters: &mut WorkCounters,
) -> CoreResult<StepResult> {
    if config.method == LinearMethod::Direct || config.preconditioner == PreconditionerKind::Direct
    {
        return Err(CoreError::InvalidInput(
            "strict matrix-free sequential RODAS5P forbids direct factorization".into(),
        ));
    }
    let before = *counters;
    let snapshot = recycle.as_deref().cloned();
    let result = (|| {
        let context = build_step_context_matrix_free(problem, t, y, h, counters)?;
        let data = sequential_stages(&context, config, recycle.as_deref_mut(), counters)?;
        let mut report = finish_step(
            &context,
            data.stages,
            atol,
            rtol,
            format!("RODAS5P-protected-sequential-JF-{:?}", config.method),
            if force_accept { Some(true) } else { None },
            false,
            None,
            before,
            counters,
        )?;
        if report.accepted {
            counters.accepted_steps += 1;
        } else {
            counters.rejected_steps += 1;
        }
        report.counters = counters.delta(before);
        debug_assert_eq!(report.counters.jacobian_builds, 0);
        debug_assert_eq!(report.counters.direct_factorizations, 0);
        Ok(report)
    })();
    if result.as_ref().map_or(true, |report| !report.accepted)
        && let (Some(target), Some(saved)) = (recycle, snapshot)
    {
        *target = saved;
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn sequential_matrix_free_step_with_inner_forcing(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &LinearSolverConfig,
    mut recycle: Option<&mut KrylovState>,
    atol: f64,
    rtol: f64,
    force_accept: bool,
    counters: &mut WorkCounters,
) -> CoreResult<InnerForcedStepResult> {
    if config.method == LinearMethod::Direct || config.preconditioner == PreconditionerKind::Direct
    {
        return Err(CoreError::InvalidInput(
            "strict matrix-free sequential RODAS5P forbids direct factorization".into(),
        ));
    }
    let before = *counters;
    let snapshot = recycle.as_deref().cloned();
    let result = (|| {
        let context = build_step_context_matrix_free(problem, t, y, h, counters)?;
        let data = sequential_stages_with_inner_forcing(
            &context,
            config,
            recycle.as_deref_mut(),
            atol,
            rtol,
            counters,
        )?;
        let mut step = finish_step(
            &context,
            data.stage_data.stages,
            atol,
            rtol,
            format!(
                "RODAS5P-protected-sequential-JF-{:?}-WRMS-forcing-v2",
                config.method
            ),
            if force_accept { Some(true) } else { None },
            false,
            None,
            before,
            counters,
        )?;
        if step.accepted {
            counters.accepted_steps += 1;
        } else {
            counters.rejected_steps += 1;
        }
        step.counters = counters.delta(before);
        debug_assert_eq!(step.counters.jacobian_builds, 0);
        debug_assert_eq!(step.counters.direct_factorizations, 0);
        Ok(InnerForcedStepResult {
            step,
            stage_forcing: data.stage_forcing,
        })
    })();
    if result.as_ref().map_or(true, |report| !report.step.accepted)
        && let (Some(target), Some(saved)) = (recycle, snapshot)
    {
        *target = saved;
    }
    result
}
