use rodas5p_core::{
    CoreError, CoreResult, LuFactorization, WorkCounters, error_scale, safe_l2, wrms,
};
use serde::Serialize;

use crate::{StructuredBlockSystem, flatten};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CorrectionDiagnostic {
    pub first_output_wrms: f64,
    pub second_output_wrms: f64,
    pub output_ratio: f64,
    pub initial_residual_norm: f64,
    pub residual_after_first_norm: f64,
    pub residual_after_second_norm: f64,
    pub residual_ratio: f64,
    pub empirical_tail_wrms: Option<f64>,
    pub contraction_evidence: bool,
    pub refreshed_jacobian: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RefinedRootConfig {
    pub max_iterations: usize,
    pub residual_rtol: f64,
    pub residual_atol: f64,
    pub correction_wrms_tolerance: f64,
    pub max_backtracks: usize,
    pub refresh_jacobian_each_iteration: bool,
}

impl Default for RefinedRootConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            residual_rtol: 1.0e-13,
            residual_atol: 1.0e-14,
            correction_wrms_tolerance: 1.0e-8,
            max_backtracks: 10,
            refresh_jacobian_each_iteration: true,
        }
    }
}

impl RefinedRootConfig {
    pub fn validate(&self) -> CoreResult<()> {
        if self.max_iterations == 0 {
            return Err(CoreError::InvalidInput(
                "refined-root iteration count must be positive".into(),
            ));
        }
        if self.max_backtracks == 0 {
            return Err(CoreError::InvalidInput(
                "refined-root backtrack count must be positive".into(),
            ));
        }
        for (label, value, positive) in [
            ("residual rtol", self.residual_rtol, false),
            ("residual atol", self.residual_atol, false),
            (
                "correction WRMS tolerance",
                self.correction_wrms_tolerance,
                true,
            ),
        ] {
            if !value.is_finite() {
                return Err(CoreError::NonFinite(format!(
                    "refined-root {label} contains NaN/Inf"
                )));
            }
            if (positive && value <= 0.0) || (!positive && value < 0.0) {
                return Err(CoreError::InvalidInput(format!(
                    "refined-root {label} has invalid sign"
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RefinedRootCertificate {
    pub converged: bool,
    pub termination: String,
    pub iterations: usize,
    pub stages: Vec<Vec<f64>>,
    pub initial_residual_norm: f64,
    pub final_residual_norm: f64,
    pub relative_residual: f64,
    pub residual_tolerance: f64,
    pub last_correction_wrms: f64,
    pub candidate_output_wrms: f64,
    pub backtracks: usize,
}

struct TargetEvaluation {
    residual: Vec<f64>,
    rhs_norm: f64,
    snapshot: crate::NonlinearRemainderSnapshot,
}

fn evaluate_target(
    block: &StructuredBlockSystem<'_, '_>,
    stages: &[Vec<f64>],
    counters: &mut WorkCounters,
) -> CoreResult<TargetEvaluation> {
    let snapshot = block.nonlinear_remainder_snapshot(stages, counters)?;
    let applied = block.apply(stages, counters)?;
    let residual_rows: Vec<Vec<f64>> = applied
        .iter()
        .zip(&snapshot.rhs)
        .map(|(lhs, rhs)| lhs.iter().zip(rhs).map(|(a, b)| a - b).collect())
        .collect();
    let residual = flatten(&residual_rows);
    let rhs_norm = safe_l2(&flatten(&snapshot.rhs)).max(f64::MIN_POSITIVE);
    Ok(TargetEvaluation {
        residual,
        rhs_norm,
        snapshot,
    })
}

fn apply_correction(stages: &[Vec<f64>], correction: &[f64], damping: f64) -> Vec<Vec<f64>> {
    let mut updated = stages.to_vec();
    let n = stages.first().map_or(0, Vec::len);
    for (stage, correction_row) in updated.iter_mut().zip(correction.chunks_exact(n)) {
        for (value, delta) in stage.iter_mut().zip(correction_row) {
            *value -= damping * delta;
        }
    }
    updated
}

fn output_from_stages(block: &StructuredBlockSystem<'_, '_>, stages: &[Vec<f64>]) -> Vec<f64> {
    let mut output = block.context.y.clone();
    for (&weight, stage) in block.context.coeffs.b.iter().zip(stages) {
        for (value, increment) in output.iter_mut().zip(stage) {
            *value += weight * increment;
        }
    }
    output
}

fn projected_correction(
    block: &StructuredBlockSystem<'_, '_>,
    correction: &[f64],
) -> CoreResult<Vec<f64>> {
    if correction.len() != block.s * block.n {
        return Err(CoreError::Dimension(
            "target correction shape does not match stage system".into(),
        ));
    }
    let mut output = vec![0.0; block.n];
    for stage in 0..block.s {
        for component in 0..block.n {
            output[component] +=
                block.context.coeffs.b[stage] * correction[stage * block.n + component];
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn certify_second_correction(
    block: &StructuredBlockSystem<'_, '_>,
    stages: &[Vec<f64>],
    atol: f64,
    rtol: f64,
    refresh_jacobian: bool,
    counters: &mut WorkCounters,
) -> CoreResult<CorrectionDiagnostic> {
    if !(atol > 0.0 && atol.is_finite() && rtol >= 0.0 && rtol.is_finite()) {
        return Err(CoreError::InvalidInput(
            "second-correction tolerances are invalid".into(),
        ));
    }
    let initial = evaluate_target(block, stages, counters)?;
    let initial_norm = safe_l2(&initial.residual);
    let jacobian0 = block.target_jacobian_matrix(stages, &initial.snapshot, counters)?;
    counters.direct_factorizations += 1;
    let factor0 = LuFactorization::new(&jacobian0)?;
    counters.direct_solve_calls += 1;
    let correction1 = factor0.solve(&initial.residual)?;
    let stages1 = apply_correction(stages, &correction1, 1.0);
    let after_first = evaluate_target(block, &stages1, counters)?;
    let residual1 = safe_l2(&after_first.residual);

    let correction2 = if refresh_jacobian {
        let jacobian1 = block.target_jacobian_matrix(&stages1, &after_first.snapshot, counters)?;
        counters.direct_factorizations += 1;
        let factor1 = LuFactorization::new(&jacobian1)?;
        counters.direct_solve_calls += 1;
        factor1.solve(&after_first.residual)?
    } else {
        counters.direct_solve_calls += 1;
        factor0.solve(&after_first.residual)?
    };
    let stages2 = apply_correction(&stages1, &correction2, 1.0);
    let after_second = evaluate_target(block, &stages2, counters)?;
    let residual2 = safe_l2(&after_second.residual);

    let candidate_output = output_from_stages(block, stages);
    let scale = error_scale(&block.context.y, &candidate_output, &[atol], rtol)?;
    let first_output_wrms = wrms(&projected_correction(block, &correction1)?, &scale)?;
    let second_output_wrms = wrms(&projected_correction(block, &correction2)?, &scale)?;
    let output_ratio = if first_output_wrms > f64::MIN_POSITIVE {
        second_output_wrms / first_output_wrms
    } else if second_output_wrms <= f64::MIN_POSITIVE {
        0.0
    } else {
        f64::INFINITY
    };
    let residual_ratio = if initial_norm > f64::MIN_POSITIVE {
        residual1 / initial_norm
    } else if residual1 <= f64::MIN_POSITIVE {
        0.0
    } else {
        f64::INFINITY
    };
    let contraction_evidence = output_ratio.is_finite()
        && residual_ratio.is_finite()
        && output_ratio < 1.0
        && residual_ratio < 1.0
        && residual2 <= residual1;
    let empirical_tail_wrms = contraction_evidence
        .then(|| first_output_wrms + second_output_wrms / (1.0 - output_ratio).max(f64::EPSILON));

    if [
        first_output_wrms,
        second_output_wrms,
        initial_norm,
        residual1,
        residual2,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        Ok(CorrectionDiagnostic {
            first_output_wrms,
            second_output_wrms,
            output_ratio,
            initial_residual_norm: initial_norm,
            residual_after_first_norm: residual1,
            residual_after_second_norm: residual2,
            residual_ratio,
            empirical_tail_wrms,
            contraction_evidence,
            refreshed_jacobian: refresh_jacobian,
        })
    } else {
        Err(CoreError::NonFinite(
            "second-correction diagnostic contains NaN/Inf".into(),
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn refine_target_root(
    block: &StructuredBlockSystem<'_, '_>,
    candidate_stages: &[Vec<f64>],
    atol: f64,
    rtol: f64,
    config: &RefinedRootConfig,
    counters: &mut WorkCounters,
) -> CoreResult<RefinedRootCertificate> {
    config.validate()?;
    if !(atol > 0.0 && atol.is_finite() && rtol >= 0.0 && rtol.is_finite()) {
        return Err(CoreError::InvalidInput(
            "refined-root output tolerances are invalid".into(),
        ));
    }
    block.validate_stage_rows(candidate_stages)?;

    let candidate_output = output_from_stages(block, candidate_stages);
    let mut stages = candidate_stages.to_vec();
    let initial = evaluate_target(block, &stages, counters)?;
    let initial_residual_norm = safe_l2(&initial.residual);
    let mut evaluation = initial;
    let mut last_correction_wrms: Option<f64> = None;
    let mut total_backtracks = 0_usize;
    let mut termination = "iteration budget exhausted".to_string();
    let mut converged = false;
    let mut iterations = 0_usize;
    let mut fixed_factor: Option<LuFactorization> = None;

    for iteration in 0..=config.max_iterations {
        iterations = iteration;
        let residual_norm = safe_l2(&evaluation.residual);
        let relative_residual = residual_norm / evaluation.rhs_norm;
        let residual_tolerance = config
            .residual_rtol
            .max(config.residual_atol / evaluation.rhs_norm);
        if relative_residual <= residual_tolerance
            && (iteration == 0
                || last_correction_wrms
                    .is_some_and(|value| value <= config.correction_wrms_tolerance))
        {
            converged = true;
            termination = "residual and correction tolerances satisfied".into();
            break;
        }
        if iteration == config.max_iterations {
            break;
        }

        if config.refresh_jacobian_each_iteration || fixed_factor.is_none() {
            let jacobian = block.target_jacobian_matrix(&stages, &evaluation.snapshot, counters)?;
            counters.direct_factorizations += 1;
            fixed_factor = Some(LuFactorization::new(&jacobian)?);
        }
        counters.direct_solve_calls += 1;
        let correction = fixed_factor
            .as_ref()
            .expect("factor initialized")
            .solve(&evaluation.residual)?;
        let current_output = output_from_stages(block, &stages);
        let scale = error_scale(&block.context.y, &current_output, &[atol], rtol)?;

        let mut accepted_trial: Option<(Vec<Vec<f64>>, TargetEvaluation, f64)> = None;
        let mut damping = 1.0;
        for backtrack in 0..config.max_backtracks {
            let trial = apply_correction(&stages, &correction, damping);
            if !trial.iter().flatten().all(|value| value.is_finite()) {
                damping *= 0.5;
                total_backtracks += 1;
                continue;
            }
            let trial_evaluation = evaluate_target(block, &trial, counters)?;
            let trial_norm = safe_l2(&trial_evaluation.residual);
            if trial_norm < residual_norm || trial_norm <= config.residual_atol {
                let projected: Vec<f64> = projected_correction(block, &correction)?
                    .into_iter()
                    .map(|value| damping * value)
                    .collect();
                let correction_wrms = wrms(&projected, &scale)?;
                accepted_trial = Some((trial, trial_evaluation, correction_wrms));
                total_backtracks += backtrack;
                break;
            }
            damping *= 0.5;
        }

        let Some((trial, trial_evaluation, correction_wrms)) = accepted_trial else {
            termination = "line search failed to reduce target residual".into();
            break;
        };
        stages = trial;
        evaluation = trial_evaluation;
        last_correction_wrms = Some(correction_wrms);
    }

    let final_residual_norm = safe_l2(&evaluation.residual);
    let relative_residual = final_residual_norm / evaluation.rhs_norm;
    let residual_tolerance = config
        .residual_rtol
        .max(config.residual_atol / evaluation.rhs_norm);
    let refined_output = output_from_stages(block, &stages);
    let scale = error_scale(&block.context.y, &refined_output, &[atol], rtol)?;
    let candidate_output_wrms = wrms(
        &candidate_output
            .iter()
            .zip(&refined_output)
            .map(|(candidate, refined)| candidate - refined)
            .collect::<Vec<_>>(),
        &scale,
    )?;
    let last_correction_wrms = last_correction_wrms.unwrap_or(0.0);

    if ![
        initial_residual_norm,
        final_residual_norm,
        relative_residual,
        residual_tolerance,
        last_correction_wrms,
        candidate_output_wrms,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(CoreError::NonFinite(
            "refined-root certificate contains NaN/Inf".into(),
        ));
    }

    Ok(RefinedRootCertificate {
        converged,
        termination,
        iterations,
        stages,
        initial_residual_norm,
        final_residual_norm,
        relative_residual,
        residual_tolerance,
        last_correction_wrms,
        candidate_output_wrms,
        backtracks: total_backtracks,
    })
}
