//! Original-target stage-error budgets for opt-in causal RK/Rosenbrock clients.
//!
//! Let `d_i` bound the inverse-diagonal action on stage residual `i`, and let
//! `B_ij >= 0` bound propagation from an earlier stage `j` to `i`. Strict
//! causality gives `e = d + B e` without a contraction assumption. This is the
//! caller-buffer counterpart of `rodas5p_integrators::upper_causal_solve`, with
//! output transport, context checks and adjoint-weighted residual allowances.
//!
//! The caller supplies valid residual, inverse and Lipschitz upper bounds in
//! one fixed physical norm, including rounding/evaluation error and a domain
//! containing both the exact and candidate stages. This API checks structure
//! and identity; it cannot establish those model-specific hypotheses. Bounds
//! use the core's outward binary64 operations and inherit their IEEE arithmetic
//! assumptions. A local embedded estimator remains an estimator: the returned
//! bound concerns perturbation of the specified stage equations/output only.

use rodas5p_core::{
    CoreError,
    directed::{add_up, div_down, mul_up},
};
use serde::Serialize;

use crate::SessionEpochs;

/// Identity of one physical stage target. Any changed binding invalidates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CausalTargetKey {
    /// Model, callback, physical norm and tolerance versions.
    pub epochs: SessionEpochs,
    /// Coordinate reconstruction version, including time-dependent charts.
    pub chart: u64,
    /// Diagonal operator/tableau/provider-plan version.
    pub operator: u64,
    /// Caller version of the unchanged accepted initial state.
    pub state: u64,
    /// Exact represented initial time.
    pub time_bits: u64,
    /// Exact represented trial step. Changing h invalidates the old majorant.
    pub step_bits: u64,
}

impl CausalTargetKey {
    /// Bind a finite forward step. Epochs also cover concealed provider changes.
    pub fn new(
        epochs: SessionEpochs,
        chart: u64,
        operator: u64,
        state: u64,
        time: f64,
        step: f64,
    ) -> Result<Self, CausalBudgetError> {
        if !time.is_finite()
            || !step.is_finite()
            || step <= 0.0
            || !(time + step).is_finite()
            || time + step <= time
        {
            return Err(CausalBudgetError::Invalid(
                "finite progressing forward step required",
            ));
        }
        Ok(Self {
            epochs,
            chart,
            operator,
            state,
            time_bits: time.to_bits(),
            step_bits: step.to_bits(),
        })
    }
}

/// Refusal without accepting a numerical candidate.
#[derive(Debug, thiserror::Error)]
pub enum CausalBudgetError {
    /// Malformed bounds, dimensions, target or budget.
    #[error("causal budget: {0}")]
    Invalid(&'static str),
    /// The record belongs to a different physical stage target.
    #[error("causal budget: stale target identity")]
    StaleTarget,
    /// A continuation root at lambda below one is not the requested target.
    #[error("causal budget: original target requires lambda exactly one")]
    NotOriginalTarget,
    /// The inherited directed arithmetic failed closed, e.g. on overflow.
    #[error(transparent)]
    Arithmetic(#[from] CoreError),
}

/// Supplied original-target data in a single fixed stage norm.
pub struct CausalBudgetInput<'a> {
    /// Identity of the target the adapter currently requests.
    pub expected: CausalTargetKey,
    /// Identity attached to the residuals and propagation majorant.
    pub observed: CausalTargetKey,
    /// Continuation parameter at which these residuals were evaluated.
    pub lambda: f64,
    /// Full row-major s by s nonnegative strictly lower propagation majorant.
    pub propagation: &'a [f64],
    /// s nonnegative bounds on inverse-diagonal-action residuals.
    pub residual_seeds: &'a [f64],
    /// s nonnegative physical-output Lipschitz weights.
    pub output_weights: &'a [f64],
    /// Nonnegative reconstruction and evaluation error not already in seeds.
    pub reconstruction_error: f64,
    /// Allowed physical-output perturbation of the exact stage target.
    pub output_budget: f64,
}

/// A conditional, original-target perturbation decision, separate from ODE LTE.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct CausalBudgetReceipt {
    /// Original physical output error relative to the exact stage equations.
    pub output_error_upper: f64,
    /// The unchanged requested perturbation budget.
    pub output_budget: f64,
    /// True exactly when the supplied-bounds result fits the budget.
    pub admitted: bool,
    /// Directed scalar arithmetic operations, including output reconstruction.
    pub directed_operations: usize,
    /// Maximum stage-error bound, useful for a declared domain-closure check.
    pub maximum_stage_error_upper: f64,
}

fn nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

fn validate(propagation: &[f64], weights: &[f64], stages: usize) -> Result<(), CausalBudgetError> {
    if stages == 0
        || stages.checked_mul(stages) != Some(propagation.len())
        || weights.len() != stages
    {
        return Err(CausalBudgetError::Invalid(
            "nonempty square propagation and matching weights required",
        ));
    }
    if !weights.iter().copied().all(nonnegative) {
        return Err(CausalBudgetError::Invalid(
            "weights must be finite and nonnegative",
        ));
    }
    for i in 0..stages {
        for j in 0..stages {
            let b = propagation[i * stages + j];
            if !nonnegative(b) || (j >= i && b != 0.0) {
                return Err(CausalBudgetError::Invalid(
                    "propagation must be finite, nonnegative and strictly lower triangular",
                ));
            }
        }
    }
    Ok(())
}

/// Evaluate the finite causal path sum in O(s²) work and caller-owned O(s)
/// storage. No allocation or RHS/derivative/Newton callback is performed.
///
/// Structural validation occurs before writing `stage_errors`. A subsequent
/// arithmetic error can leave a partial workspace; it never returns admission.
pub fn assess_causal_budget(
    input: &CausalBudgetInput<'_>,
    stage_errors: &mut [f64],
) -> Result<CausalBudgetReceipt, CausalBudgetError> {
    if input.expected != input.observed {
        return Err(CausalBudgetError::StaleTarget);
    }
    if input.lambda != 1.0 {
        return Err(CausalBudgetError::NotOriginalTarget);
    }
    // Keys have public fields for serialization; validate their physical values.
    let t = f64::from_bits(input.expected.time_bits);
    let h = f64::from_bits(input.expected.step_bits);
    CausalTargetKey::new(
        input.expected.epochs,
        input.expected.chart,
        input.expected.operator,
        input.expected.state,
        t,
        h,
    )?;
    let s = input.residual_seeds.len();
    validate(input.propagation, input.output_weights, s)?;
    if stage_errors.len() != s
        || !input.residual_seeds.iter().copied().all(nonnegative)
        || !nonnegative(input.reconstruction_error)
        || !nonnegative(input.output_budget)
    {
        return Err(CausalBudgetError::Invalid(
            "finite nonnegative seeds, budget and matching workspace required",
        ));
    }
    let mut operations = 0;
    for i in 0..s {
        let mut e = input.residual_seeds[i];
        for (j, &prior_stage_error) in stage_errors.iter().enumerate().take(i) {
            let b = input.propagation[i * s + j];
            if b != 0.0 && prior_stage_error != 0.0 {
                e = add_up(e, mul_up(b, prior_stage_error)?)?;
                operations += 2;
            }
        }
        stage_errors[i] = e;
    }
    let mut output = input.reconstruction_error;
    let mut maximum: f64 = 0.0;
    for (e, w) in stage_errors.iter().zip(input.output_weights) {
        maximum = maximum.max(*e);
        if *e != 0.0 && *w != 0.0 {
            output = add_up(output, mul_up(*e, *w)?)?;
            operations += 2;
        }
    }
    Ok(CausalBudgetReceipt {
        output_error_upper: output,
        output_budget: input.output_budget,
        admitted: output <= input.output_budget,
        directed_operations: operations,
        maximum_stage_error_upper: maximum,
    })
}

/// Compute outward adjoint influences `v = w + Bᵀ v` and downward seed
/// allowances `tau/(m v_i)` for m positive influences. If each seed stays
/// within its allowance, the propagated output contribution is at most tau.
/// Zero-influence stages receive zero allowance: no undefined infinity is used.
///
/// The allowances control the seeds M_i*rho_i, not bare residuals rho_i.
/// Separate reconstruction/LTE budgets must be reserved before passing tau.
pub fn causal_residual_allowances(
    propagation: &[f64],
    weights: &[f64],
    budget: f64,
    influences: &mut [f64],
    allowances: &mut [f64],
) -> Result<(), CausalBudgetError> {
    let s = weights.len();
    validate(propagation, weights, s)?;
    if influences.len() != s || allowances.len() != s || !nonnegative(budget) {
        return Err(CausalBudgetError::Invalid(
            "finite nonnegative budget and matching workspaces required",
        ));
    }
    for i in (0..s).rev() {
        let mut v = weights[i];
        for j in i + 1..s {
            let b = propagation[j * s + i];
            if b != 0.0 && influences[j] != 0.0 {
                v = add_up(v, mul_up(b, influences[j])?)?;
            }
        }
        influences[i] = v;
    }
    let active = influences.iter().filter(|v| **v > 0.0).count();
    // Realistic stage counts are exactly representable. Refuse an impossible
    // gigantic caller slice rather than silently round an allocation divisor.
    if active as u128 > (1u128 << 53) {
        return Err(CausalBudgetError::Invalid(
            "stage count exceeds exact binary64 integer range",
        ));
    }
    for i in 0..s {
        allowances[i] = if influences[i] == 0.0 || budget == 0.0 {
            0.0
        } else {
            div_down(div_down(budget, active as f64)?.max(0.0), influences[i])?.max(0.0)
        };
    }
    Ok(())
}
