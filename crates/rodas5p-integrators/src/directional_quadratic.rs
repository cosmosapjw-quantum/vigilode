//! Optional directional quadratic proposal, always checked in the original system.
//! Coefficients are hypotheses, not full-space polynomial witnesses.

use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, LinearOperator, WorkCounters, apply_counted, safe_l2,
    wrms,
};
use rodas5p_krylov::{GmresConfig, GmresIntoReport};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalQuadraticProposal {
    pub a: f64,
    pub b: f64,
}

impl DirectionalQuadraticProposal {
    pub fn new(a: f64, b: f64) -> CoreResult<Self> {
        if !a.is_finite() || !b.is_finite() {
            return Err(CoreError::InvalidInput(
                "quadratic proposal coefficients must be finite".into(),
            ));
        }
        Ok(Self { a, b })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DirectionalQuadraticStatistics {
    pub attempts: u64,
    pub accepted: u64,
    pub refused: u64,
    pub operator_actions: u64,
    pub callback_failures: u64,
}

/// Private candidate/action buffers. Clone copies both storage and cumulative work.
#[derive(Clone, Debug, Default)]
pub struct DirectionalQuadraticWorkspace {
    candidate: Vec<f64>,
    action: Vec<f64>,
    statistics: DirectionalQuadraticStatistics,
}

impl DirectionalQuadraticWorkspace {
    pub fn statistics(&self) -> DirectionalQuadraticStatistics {
        self.statistics
    }

    pub fn retained_bytes(&self) -> usize {
        (self.candidate.capacity() + self.action.capacity()) * std::mem::size_of::<f64>()
    }
}

fn norm(v: &[f64], scale: Option<&[f64]>) -> CoreResult<f64> {
    match scale {
        Some(scale) => wrms(v, scale),
        None => Ok(safe_l2(v)),
    }
}

fn action(
    op: &dyn LinearOperator,
    input: &[f64],
    output: &mut [f64],
    statistics: &mut DirectionalQuadraticStatistics,
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    output.fill(f64::NAN);
    statistics.operator_actions = statistics.operator_actions.saturating_add(1);
    let result = apply_counted(op, input, output, counters, ApplyCategory::Krylov).and_then(|()| {
        if output.iter().all(|x| x.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite(
                "quadratic proposal operator left nonfinite output".into(),
            ))
        }
    });
    if result.is_err() {
        statistics.callback_failures = statistics.callback_failures.saturating_add(1);
    }
    result
}

/// Try a correction for A=I-sigma*J, preserving the supplied initial guess.
///
/// `Some` satisfies the same computed original L2/WRMS residual threshold as
/// GMRES; it is not a physical error enclosure. `None` leaves output unchanged
/// and requests the existing solver fallback. All attempted actions remain paid.
/// Actual callback errors propagate. The caller is responsible for binding the
/// supplied sigma to its intended shifted map; the live original residual is
/// still checked even when that hypothesis is wrong.
#[allow(clippy::too_many_arguments)]
pub fn try_directional_quadratic(
    op: &dyn LinearOperator,
    sigma: f64,
    proposal: DirectionalQuadraticProposal,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    residual_scale: Option<&[f64]>,
    output: &mut [f64],
    workspace: &mut DirectionalQuadraticWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<Option<GmresIntoReport>> {
    config.validate()?;
    DirectionalQuadraticProposal::new(proposal.a, proposal.b)?;
    let n = op.dimension();
    if rhs.len() != n || output.len() != n || x0.is_some_and(|x| x.len() != n) {
        return Err(CoreError::Dimension(
            "quadratic proposal shape mismatch".into(),
        ));
    }
    if !sigma.is_finite()
        || rhs.iter().any(|x| !x.is_finite())
        || x0.is_some_and(|x| x.iter().any(|v| !v.is_finite()))
    {
        return Err(CoreError::InvalidInput(
            "nonfinite quadratic proposal input".into(),
        ));
    }
    if let Some(scale) = residual_scale
        && (scale.len() != n
            || scale.is_empty()
            || scale.iter().any(|x| !x.is_finite() || *x <= 0.0))
    {
        return Err(CoreError::InvalidInput(
            "invalid quadratic residual scale".into(),
        ));
    }
    let right_norm = norm(rhs, residual_scale)?;
    let threshold = config.atol.max(config.rtol * right_norm);
    if !threshold.is_finite() {
        return Err(CoreError::InvalidInput(
            "nonfinite quadratic residual threshold".into(),
        ));
    }
    workspace.statistics.attempts = workspace.statistics.attempts.saturating_add(1);
    if !right_norm.is_finite() {
        workspace.statistics.refused = workspace.statistics.refused.saturating_add(1);
        return Ok(None);
    }
    let a_sigma = proposal.a * sigma;
    let b_sigma2 = (proposal.b * sigma) * sigma;
    let denominator = (1.0 - a_sigma) - b_sigma2;
    let denominator_scale = 1.0 + a_sigma.abs() + b_sigma2.abs();
    if !denominator.is_finite()
        || !denominator_scale.is_finite()
        || denominator.abs() <= 64.0 * f64::EPSILON * denominator_scale
    {
        workspace.statistics.refused = workspace.statistics.refused.saturating_add(1);
        return Ok(None);
    }
    let bytes_before = workspace.retained_bytes();
    let before = *counters;
    workspace.candidate.resize(n, 0.0);
    workspace.action.resize(n, 0.0);
    let nonzero_start = x0.is_some_and(|x| x.iter().any(|v| *v != 0.0));
    if nonzero_start {
        action(
            op,
            x0.expect("nonzero start exists"),
            &mut workspace.action,
            &mut workspace.statistics,
            counters,
        )?;
        for ((v, b), ax) in workspace
            .candidate
            .iter_mut()
            .zip(rhs)
            .zip(&workspace.action)
        {
            *v = b - ax;
        }
    } else {
        workspace.candidate.copy_from_slice(rhs);
    }
    if workspace.candidate.iter().any(|x| !x.is_finite()) {
        workspace.statistics.refused = workspace.statistics.refused.saturating_add(1);
        return Ok(None);
    }
    let initial_residual_norm = norm(&workspace.candidate, residual_scale)?;
    if !initial_residual_norm.is_finite() {
        workspace.statistics.refused = workspace.statistics.refused.saturating_add(1);
        return Ok(None);
    }
    let residual_norm;
    if initial_residual_norm.is_finite() && initial_residual_norm <= threshold {
        // The initial-residual algebra matches GMRES: A*0=0 for a linear map.
        // This optional proposal does not add GMRES's final diagnostic repeat.
        match x0 {
            Some(x) => workspace.candidate.copy_from_slice(x),
            None => workspace.candidate.fill(0.0),
        }
        residual_norm = initial_residual_norm;
    } else {
        if sigma != 0.0 {
            action(
                op,
                &workspace.candidate,
                &mut workspace.action,
                &mut workspace.statistics,
                counters,
            )?;
            for (i, v) in workspace.candidate.iter_mut().enumerate() {
                let correction = ((2.0 - a_sigma) * *v - workspace.action[i]) / denominator;
                *v = x0.map_or(0.0, |x| x[i]) + correction;
            }
        } else {
            for (i, v) in workspace.candidate.iter_mut().enumerate() {
                *v += x0.map_or(0.0, |x| x[i]);
            }
        }
        if workspace.candidate.iter().any(|x| !x.is_finite()) {
            workspace.statistics.refused = workspace.statistics.refused.saturating_add(1);
            return Ok(None);
        }
        action(
            op,
            &workspace.candidate,
            &mut workspace.action,
            &mut workspace.statistics,
            counters,
        )?;
        for (r, b) in workspace.action.iter_mut().zip(rhs) {
            *r = b - *r;
        }
        residual_norm = norm(&workspace.action, residual_scale)?;
        if !residual_norm.is_finite() || residual_norm > threshold {
            workspace.statistics.refused = workspace.statistics.refused.saturating_add(1);
            return Ok(None);
        }
    }
    counters.linear_solves += 1;
    workspace.statistics.accepted = workspace.statistics.accepted.saturating_add(1);
    output.copy_from_slice(&workspace.candidate);
    let delta = counters.delta(before);
    Ok(Some(GmresIntoReport {
        residual_norm,
        relative_residual: residual_norm / right_norm.max(f64::MIN_POSITIVE),
        iterations: 0,
        matvecs: delta.linear_matvecs,
        preconditioner_apps: delta.preconditioner_apps,
        cycles: 0,
        least_squares_solves: 0,
        workspace_grew: workspace.retained_bytes() > bytes_before,
        method: "directional-quadratic",
        initial_residual_norm,
    }))
}
