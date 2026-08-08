use rodas5p_core::{
    CoreError, CoreResult, LinearMethod, LinearOperator, LinearSolverConfig, PreconditionerKind,
    WorkCounters, error_scale, safe_l2, wrms,
};

use crate::{
    BlockMethod, BlockPreconditioner, KrylovState, OdeProblem, StepCertificate, StepResult,
    StructuredBlockSystem, build_step_context, finish_step, sequential_stages,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredictorKind {
    Zero,
    ScaledLast,
    LinearHistory,
}

#[derive(Clone, Debug, Default)]
pub struct StageHistory {
    pub entries: Vec<(f64, Vec<Vec<f64>>)>,
    pub fast_failure_streak: usize,
}
impl StageHistory {
    pub fn push(&mut self, h: f64, stages: Vec<Vec<f64>>) {
        self.entries.insert(0, (h, stages));
        self.entries.truncate(2);
    }
    pub fn predictor(&self, h: f64, kind: PredictorKind, shape: (usize, usize)) -> Vec<Vec<f64>> {
        if kind == PredictorKind::Zero || self.entries.is_empty() {
            return vec![vec![0.0; shape.1]; shape.0];
        }
        let (h1, k1) = &self.entries[0];
        if *h1 == 0.0 || k1.len() != shape.0 || k1.iter().any(|r| r.len() != shape.1) {
            return vec![vec![0.0; shape.1]; shape.0];
        }
        let mut p1 = k1.clone();
        for row in &mut p1 {
            for v in row {
                *v *= h / h1;
            }
        }
        if kind == PredictorKind::ScaledLast || self.entries.len() < 2 {
            return p1;
        }
        let (h2, k2) = &self.entries[1];
        if *h2 == 0.0 || k2.len() != shape.0 || k2.iter().any(|r| r.len() != shape.1) {
            return p1;
        }
        let mut out = vec![vec![0.0; shape.1]; shape.0];
        for i in 0..shape.0 {
            for q in 0..shape.1 {
                out[i][q] = h * (2.0 * k1[i][q] / h1 - k2[i][q] / h2);
            }
        }
        out
    }
    pub fn eligible_ratio(&self, h: f64, lower: f64, upper: f64) -> bool {
        self.entries
            .first()
            .is_none_or(|(hp, _)| *hp != 0.0 && lower <= (h / hp).abs() && (h / hp).abs() <= upper)
    }
}

#[derive(Clone, Debug)]
pub struct SabrConfig {
    pub predictor: PredictorKind,
    pub max_iterations: usize,
    pub block_method: BlockMethod,
    pub block_rtol: f64,
    pub block_atol: f64,
    pub block_restart: usize,
    pub block_max_arnoldi: usize,
    pub block_preconditioner: BlockPreconditioner,
    pub defect_budget_fraction: f64,
    pub contraction_limit: f64,
    pub residual_safety: f64,
    pub affine_closure_rtol: f64,
    pub step_ratio_min: f64,
    pub step_ratio_max: f64,
    pub max_failure_streak: usize,
}
impl Default for SabrConfig {
    fn default() -> Self {
        Self {
            predictor: PredictorKind::LinearHistory,
            max_iterations: 3,
            block_method: BlockMethod::Gmres,
            block_rtol: 1e-11,
            block_atol: 1e-13,
            block_restart: 40,
            block_max_arnoldi: 100,
            block_preconditioner: BlockPreconditioner::Direct,
            defect_budget_fraction: 0.10,
            contraction_limit: 0.85,
            residual_safety: 1.25,
            affine_closure_rtol: 5e-11,
            step_ratio_min: 0.4,
            step_ratio_max: 2.5,
            max_failure_streak: 2,
        }
    }
}

fn combine_rows(weights: &[f64], rows: &[Vec<f64>], n: usize) -> Vec<f64> {
    let mut out = vec![0.0; n];
    for (&a, row) in weights.iter().zip(rows) {
        for q in 0..n {
            out[q] += a * row[q];
        }
    }
    out
}
fn subtract_rows(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.iter().zip(y).map(|(u, v)| u - v).collect())
        .collect()
}
fn max_component_abs(rows: &[Vec<f64>], n: usize) -> Vec<f64> {
    (0..n)
        .map(|q| rows.iter().map(|r| r[q].abs()).fold(0.0, f64::max))
        .collect()
}

fn stage_inverse_coefficient_bound(context: &crate::StepContext<'_>) -> CoreResult<f64> {
    let l = &context.coeffs.l;
    let mut power = rodas5p_core::DenseMatrix::identity(l.nrows());
    let mut sum = 0.0;
    let mut gp = 1.0;
    for _ in 0..l.nrows() {
        let norm = (0..power.nrows())
            .map(|i| power.row(i).iter().map(|v| v.abs()).sum::<f64>())
            .fold(0.0, f64::max);
        sum += norm / gp;
        power = power.matmul(l)?;
        gp *= context.coeffs.gamma;
    }
    Ok(sum)
}

fn residual_proxy(
    block: &StructuredBlockSystem<'_, '_>,
    residual: &[Vec<f64>],
    scale: &[f64],
    cfg: &SabrConfig,
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    let coeff = stage_inverse_coefficient_bound(block.context)?;
    if let Some(w) = block.context.shifted.explicit() {
        counters.direct_factorizations += 1;
        let factor = rodas5p_core::LuFactorization::new(w)?;
        let pre = factor.solve_rows(residual)?;
        counters.direct_solve_calls += 1;
        let sum_b: f64 = block.context.coeffs.b.iter().map(|v| v.abs()).sum();
        let component: Vec<f64> = max_component_abs(&pre, block.n)
            .into_iter()
            .map(|v| sum_b * v)
            .collect();
        Ok(cfg.residual_safety * coeff * wrms(&component, scale)?)
    } else {
        let base = block.rhs_base();
        Ok(
            cfg.residual_safety * coeff * safe_l2(&crate::flatten(residual))
                / safe_l2(&crate::flatten(&base)).max(f64::MIN_POSITIVE),
        )
    }
}

fn tail_error(
    context: &crate::StepContext<'_>,
    inc: &[Vec<f64>],
    prev: Option<&[Vec<f64>]>,
    scale: &[f64],
    limit: f64,
) -> CoreResult<(Option<f64>, Option<f64>)> {
    let Some(prev) = prev else {
        return Ok((None, None));
    };
    let d = safe_l2(&crate::flatten(prev));
    let n = safe_l2(&crate::flatten(inc));
    if d <= f64::MIN_POSITIVE {
        return if n <= f64::EPSILON {
            Ok((Some(0.0), Some(0.0)))
        } else {
            Ok((None, None))
        };
    }
    let q = n / d;
    if !q.is_finite() || q >= limit {
        return Ok((Some(q), None));
    }
    let component: Vec<f64> = max_component_abs(inc, context.problem.dimension)
        .into_iter()
        .map(|v| context.coeffs.b.iter().map(|x| x.abs()).sum::<f64>() * v)
        .collect();
    Ok((
        Some(q),
        Some((q / (1.0 - q).max(f64::EPSILON)) * wrms(&component, scale)?),
    ))
}

fn block_solve(
    block: &StructuredBlockSystem<'_, '_>,
    rhs: &[Vec<f64>],
    cfg: &SabrConfig,
    x0: Option<&[Vec<f64>]>,
    counters: &mut WorkCounters,
) -> CoreResult<crate::BlockSolveReport> {
    match cfg.block_method {
        BlockMethod::Gmres => block.gmres_solve(
            rhs,
            cfg.block_rtol,
            cfg.block_atol,
            cfg.block_restart,
            cfg.block_max_arnoldi,
            cfg.block_preconditioner,
            x0,
            counters,
        ),
        m => block.solve(rhs, m, counters),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sabr_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    cfg: &SabrConfig,
    fallback_config: Option<&LinearSolverConfig>,
    history: &mut StageHistory,
    mut recycle: Option<&mut KrylovState>,
    atol: f64,
    rtol: f64,
    force_accept: bool,
    counters: &mut WorkCounters,
) -> CoreResult<StepResult> {
    let before = *counters;
    let state_snapshot = recycle.as_deref().cloned();
    let history_snapshot = history.clone();
    let context = build_step_context(problem, t, y, h, counters)?;
    counters.fast_attempts += 1;
    let fallback = fallback_config
        .cloned()
        .unwrap_or_else(|| LinearSolverConfig {
            method: if context.shifted.explicit().is_some() {
                LinearMethod::Direct
            } else {
                LinearMethod::Gmres
            },
            preconditioner: PreconditionerKind::None,
            ..Default::default()
        });
    let eligible = history.eligible_ratio(h, cfg.step_ratio_min, cfg.step_ratio_max)
        && history.fast_failure_streak < cfg.max_failure_streak;
    let mut reason = "eligibility gate".to_string();
    if eligible {
        let fast = (|| {
            let block = StructuredBlockSystem::new(&context);
            let mut k = history.predictor(
                h,
                cfg.predictor,
                (context.coeffs.stages(), problem.dimension),
            );
            if !k.iter().flatten().all(|v| v.is_finite()) {
                return Err(CoreError::NonFinite("non-finite stage predictor".into()));
            }
            let mut prev: Option<Vec<Vec<f64>>> = None;
            let (mut rhs, _, _, _) = block.nonlinear_rhs(&k, counters)?;
            for iteration in 1..=cfg.max_iterations {
                let rep = block_solve(&block, &rhs, cfg, Some(&k), counters)?;
                let knew = rep.stages.clone();
                if !knew.iter().flatten().all(|v| v.is_finite()) {
                    return Err(CoreError::NonFinite("non-finite block stages".into()));
                }
                let (rhs_new, _, _, _) = block.nonlinear_rhs(&knew, counters)?;
                let applied = block.apply(&knew, counters)?;
                let residual = subtract_rows(&applied, &rhs_new);
                let residual_norm = safe_l2(&crate::flatten(&residual));
                let rhs_norm = safe_l2(&crate::flatten(&rhs_new)).max(f64::MIN_POSITIVE);
                let rel_stage = residual_norm / rhs_norm;
                let yc_update = combine_rows(&context.coeffs.b, &knew, problem.dimension);
                let yc: Vec<f64> = context
                    .y
                    .iter()
                    .zip(yc_update)
                    .map(|(a, b)| a + b)
                    .collect();
                let scale = error_scale(&context.y, &yc, &[atol], rtol)?;
                let embedded = wrms(
                    &combine_rows(&context.coeffs.btilde, &knew, problem.dimension),
                    &scale,
                )?;
                let residual_error = residual_proxy(&block, &residual, &scale, cfg, counters)?;
                let inc = subtract_rows(&knew, &k);
                let (q, tail) = tail_error(
                    &context,
                    &inc,
                    prev.as_deref(),
                    &scale,
                    cfg.contraction_limit,
                )?;
                let closure = rep.relative_residual <= cfg.affine_closure_rtol
                    && rel_stage <= cfg.affine_closure_rtol;
                let contraction_valid = tail.is_some();
                let fp = residual_error.max(tail.unwrap_or(0.0));
                let combined = embedded + fp;
                let accept = rep.converged
                    && combined.is_finite()
                    && fp <= cfg.defect_budget_fraction
                    && (force_accept || combined <= 1.0)
                    && (closure || contraction_valid);
                let cert = StepCertificate {
                    accepted: accept,
                    reason: if accept {
                        "certificate passed".into()
                    } else {
                        "certificate not yet passed".into()
                    },
                    iterations: iteration,
                    embedded_error: embedded,
                    fixed_point_error: fp,
                    residual_proxy_error: residual_error,
                    contraction_tail_error: tail,
                    observed_contraction: q,
                    stage_residual_norm: residual_norm,
                    stage_relative_residual: rel_stage,
                };
                if accept {
                    let mut r = finish_step(
                        &context,
                        knew.clone(),
                        atol,
                        rtol,
                        format!("SABR5P-fast-{}", rep.method),
                        Some(true),
                        false,
                        Some(cert),
                        before,
                        counters,
                    )?;
                    counters.fast_accepts += 1;
                    counters.accepted_steps += 1;
                    history.fast_failure_streak = 0;
                    history.push(h, knew);
                    r.counters = counters.delta(before);
                    return Ok(Some(r));
                }
                if q.is_some_and(|v| !v.is_finite() || v >= cfg.contraction_limit) {
                    reason = format!("observed contraction {q:?} outside gate");
                    return Ok(None);
                }
                prev = Some(inc);
                k = knew;
                rhs = rhs_new;
                reason = "iteration budget exhausted".into();
            }
            Ok(None)
        })();
        match fast {
            Ok(Some(r)) => return Ok(r),
            Ok(None) => {}
            Err(e) => reason = format!("fast-path failure: {e}"),
        }
    }
    counters.fallback_steps += 1;
    history.fast_failure_streak += 1;
    let data = match sequential_stages(&context, &fallback, recycle.as_deref_mut(), counters) {
        Ok(d) => d,
        Err(e) => {
            if let (Some(target), Some(saved)) = (recycle, state_snapshot) {
                *target = saved;
            }
            *history = history_snapshot;
            return Err(e);
        }
    };
    let mut r = finish_step(
        &context,
        data.stages.clone(),
        atol,
        rtol,
        format!("SABR5P-fallback-RODAS5P-{:?}", fallback.method),
        if force_accept { Some(true) } else { None },
        true,
        Some(StepCertificate {
            reason,
            ..Default::default()
        }),
        before,
        counters,
    )?;
    if r.accepted {
        counters.accepted_steps += 1;
        history.push(h, data.stages);
    } else {
        counters.rejected_steps += 1;
        if let (Some(target), Some(saved)) = (recycle, state_snapshot) {
            *target = saved;
        }
        *history = history_snapshot;
    }
    r.counters = counters.delta(before);
    Ok(r)
}
