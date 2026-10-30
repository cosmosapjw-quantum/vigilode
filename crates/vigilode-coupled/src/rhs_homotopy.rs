//! RHS-map-only homotopy with a screened depth-one Anderson extrapolation.
//!
//! The callback computes the fixed-point map itself. This module requests no
//! Jacobian, JVP, finite-difference derivative, AD, matrix factorization, or
//! Newton correction. A caller that builds those operations into its map must
//! account for them separately; the interface does not erase that work.
//!
//! At each continuation station, the map is
//! `G_lambda(x) = (1-lambda) anchor + lambda G(x)`. History is reset when lambda
//! changes. An extrapolation replaces a fully evaluated Picard candidate only
//! when its original-map residual is smaller. For an exact contraction of
//! factor q on a common invariant domain, the selected residual decreases by
//! at least q per update, and the root error is at most residual / (1-q).
//! The binary64 residual returned here is an operational measurement: callers
//! supplying a rigorous error bound must also bound evaluation and rounding.

use serde::Serialize;

/// A direct fixed-point map. Callback outputs must be fully overwritten.
/// The callback and domain predicate must describe one unchanged map/epoch.
pub trait RhsOnlyMap {
    fn dimension(&self) -> usize;
    fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String>;
    fn admits(&self, x: &[f64]) -> bool {
        x.len() == self.dimension() && x.iter().all(|v| v.is_finite())
    }
    fn model_epoch(&self) -> Option<u64> {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RhsAcceleration {
    Picard,
    /// Accept a finite, in-domain trial whenever its measured residual is no
    /// greater than the Picard candidate's. No coefficient-magnitude, observed
    /// contraction, rejection-count, or gain-margin heuristic is applied.
    ResidualDominance,
    /// Try every admissible depth-one extrapolation until its rejection cap.
    Screened,
    /// Try only when the last accepted residual ratio exceeds the threshold.
    /// This is a work heuristic; the actual residual screen still decides.
    SlowOnly,
}

#[derive(Clone, Debug, Serialize)]
pub struct RhsHomotopyConfig {
    /// Strictly increasing values in (0,1], ending at exactly 1.
    pub lambdas: Vec<f64>,
    pub residual_l2_tolerance: f64,
    pub max_iterations_per_lambda: usize,
    /// Includes the starting residual, baseline candidates, rejected trials,
    /// and every continuation station. Charged before callback dispatch.
    pub max_map_evaluations: u64,
    pub acceleration: RhsAcceleration,
    pub min_residual_ratio: f64,
    /// Required actual-residual improvement over the Picard candidate.
    pub improvement_factor: f64,
    pub max_coefficient: f64,
    pub max_rejected_trials_per_lambda: usize,
}
impl Default for RhsHomotopyConfig {
    fn default() -> Self {
        Self {
            lambdas: vec![1.0],
            residual_l2_tolerance: 1e-11,
            max_iterations_per_lambda: 4096,
            max_map_evaluations: 16384,
            acceleration: RhsAcceleration::SlowOnly,
            min_residual_ratio: 0.5,
            improvement_factor: 0.9,
            max_coefficient: 1024.0,
            max_rejected_trials_per_lambda: 2,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RhsHomotopyWork {
    pub map_attempted: u64,
    pub map_completed: u64,
    pub picard_updates: u64,
    pub extrapolation_trials: u64,
    pub extrapolations_accepted: u64,
    pub extrapolations_rejected: u64,
    pub domain_skipped_trials: u64,
    pub coefficient_skips: u64,
    pub policy_skips: u64,
    pub continuation_stations: u64,
    /// Scalar dot-product passes over the state; no least-squares matrix.
    pub residual_dot_passes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RhsHomotopyFailureKind {
    Input,
    Budget,
    Domain,
    Callback,
    NonFinite,
    ModelChanged,
    IterationLimit,
}

#[derive(Clone, Debug, Serialize)]
pub struct RhsHomotopyFailure {
    pub kind: RhsHomotopyFailureKind,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct RhsHomotopyPoint {
    pub lambda: f64,
    pub state: Vec<f64>,
    pub residual_l2: f64,
    pub iterations: usize,
    pub map_evaluations: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RhsHomotopyAttempt {
    /// Present only after a lambda=1 residual passes the requested tolerance.
    pub root: Option<Vec<f64>>,
    pub points: Vec<RhsHomotopyPoint>,
    pub work: RhsHomotopyWork,
    pub failure: Option<RhsHomotopyFailure>,
}

fn failure(kind: RhsHomotopyFailureKind, message: &str) -> RhsHomotopyFailure {
    RhsHomotopyFailure {
        kind,
        message: message.into(),
    }
}
fn l2(x: &[f64]) -> f64 {
    x.iter().fold(0.0_f64, |r, &v| r.hypot(v))
}

struct Value {
    state: Vec<f64>,
    mapped: Vec<f64>,
    residual: Vec<f64>,
    norm: f64,
}

fn evaluate<M: RhsOnlyMap + ?Sized>(
    map: &M,
    anchor: &[f64],
    state: Vec<f64>,
    lambda: f64,
    epoch: Option<u64>,
    cfg: &RhsHomotopyConfig,
    work: &mut RhsHomotopyWork,
) -> Result<Value, RhsHomotopyFailure> {
    if state.len() != anchor.len() || state.iter().any(|v| !v.is_finite()) {
        return Err(failure(
            RhsHomotopyFailureKind::NonFinite,
            "candidate contains nonfinite arithmetic",
        ));
    }
    if map.model_epoch() != epoch {
        return Err(failure(
            RhsHomotopyFailureKind::ModelChanged,
            "map epoch changed before callback",
        ));
    }
    if !map.admits(&state) {
        return Err(failure(
            RhsHomotopyFailureKind::Domain,
            "candidate is outside the callback domain",
        ));
    }
    if work.map_attempted >= cfg.max_map_evaluations {
        return Err(failure(
            RhsHomotopyFailureKind::Budget,
            "map evaluation budget exhausted before dispatch",
        ));
    }
    work.map_attempted += 1;
    let mut mapped = vec![f64::NAN; state.len()];
    map.evaluate(&state, &mut mapped)
        .map_err(|e| RhsHomotopyFailure {
            kind: RhsHomotopyFailureKind::Callback,
            message: e,
        })?;
    if map.model_epoch() != epoch {
        return Err(failure(
            RhsHomotopyFailureKind::ModelChanged,
            "map epoch changed during callback",
        ));
    }
    if mapped.iter().any(|v| !v.is_finite()) {
        return Err(failure(
            RhsHomotopyFailureKind::NonFinite,
            "map did not fully write finite output",
        ));
    }
    work.map_completed += 1;
    for (g, a) in mapped.iter_mut().zip(anchor) {
        *g = (1.0 - lambda) * a + lambda * *g;
    }
    let residual: Vec<_> = mapped.iter().zip(&state).map(|(g, x)| g - x).collect();
    let norm = l2(&residual);
    if !norm.is_finite() {
        return Err(failure(
            RhsHomotopyFailureKind::NonFinite,
            "homotopy map or residual overflowed",
        ));
    }
    Ok(Value {
        state,
        mapped,
        residual,
        norm,
    })
}

fn valid(cfg: &RhsHomotopyConfig, dimension: usize, anchor: &[f64]) -> bool {
    let heuristic_fields_valid = cfg.acceleration == RhsAcceleration::ResidualDominance
        || (cfg.min_residual_ratio.is_finite()
            && cfg.min_residual_ratio >= 0.0
            && cfg.improvement_factor.is_finite()
            && cfg.improvement_factor > 0.0
            && cfg.improvement_factor <= 1.0
            && cfg.max_coefficient.is_finite()
            && cfg.max_coefficient > 0.0);
    dimension > 0
        && dimension == anchor.len()
        && anchor.iter().all(|v| v.is_finite())
        && cfg.residual_l2_tolerance.is_finite()
        && cfg.residual_l2_tolerance > 0.0
        && cfg.max_iterations_per_lambda > 0
        && cfg.max_map_evaluations > 0
        && heuristic_fields_valid
        && cfg.lambdas.last() == Some(&1.0)
        && cfg
            .lambdas
            .iter()
            .all(|v| v.is_finite() && *v > 0.0 && *v <= 1.0)
        && cfg.lambdas.windows(2).all(|v| v[0] < v[1])
}

/// Solve a direct fixed-point target using only callback values and vector
/// arithmetic. No input state is mutated; partial station successes and all
/// attempted callback work survive a failure. A callback error is fatal even
/// when it occurs at an optional extrapolated candidate.
pub fn solve_rhs_homotopy<M: RhsOnlyMap + ?Sized>(
    map: &M,
    anchor: &[f64],
    cfg: &RhsHomotopyConfig,
) -> RhsHomotopyAttempt {
    let mut out = RhsHomotopyAttempt::default();
    if !valid(cfg, map.dimension(), anchor) {
        out.failure = Some(failure(
            RhsHomotopyFailureKind::Input,
            "invalid RHS homotopy input/configuration",
        ));
        return out;
    }
    let epoch = map.model_epoch();
    let result = run(map, anchor, cfg, epoch, &mut out);
    match result {
        Ok(root) => out.root = Some(root),
        Err(e) => out.failure = Some(e),
    }
    out
}

fn run<M: RhsOnlyMap + ?Sized>(
    map: &M,
    anchor: &[f64],
    cfg: &RhsHomotopyConfig,
    epoch: Option<u64>,
    out: &mut RhsHomotopyAttempt,
) -> Result<Vec<f64>, RhsHomotopyFailure> {
    let mut seed = anchor.to_vec();
    for &lambda in &cfg.lambdas {
        out.work.continuation_stations += 1;
        let calls_before = out.work.map_attempted;
        // Residuals from different lambda values cannot share Anderson history.
        let mut previous: Option<Value> = None;
        let mut rejected = 0;
        let mut current = evaluate(map, anchor, seed, lambda, epoch, cfg, &mut out.work)?;
        let mut iteration = 0;
        while current.norm > cfg.residual_l2_tolerance {
            if iteration == cfg.max_iterations_per_lambda {
                return Err(failure(
                    RhsHomotopyFailureKind::IterationLimit,
                    "station exhausted its update limit",
                ));
            }
            let baseline = evaluate(
                map,
                anchor,
                current.mapped.clone(),
                lambda,
                epoch,
                cfg,
                &mut out.work,
            )?;
            let mut selected = baseline;
            out.work.picard_updates += 1;
            // Do not spend a speculative map call after the Picard root passes.
            if selected.norm > cfg.residual_l2_tolerance
                && cfg.acceleration != RhsAcceleration::Picard
                && let Some(old) = &previous
            {
                let slow = current.norm >= cfg.min_residual_ratio * old.norm;
                let exact_dominance = cfg.acceleration == RhsAcceleration::ResidualDominance;
                if exact_dominance
                    || (rejected < cfg.max_rejected_trials_per_lambda
                        && (cfg.acceleration == RhsAcceleration::Screened || slow))
                {
                    // Normalize both vectors before forming dot products.
                    // This prevents overflow/underflow from squaring their
                    // common absolute residual scale.
                    let scale = current
                        .residual
                        .iter()
                        .chain(&old.residual)
                        .fold(0.0_f64, |s, &v| s.max(v.abs()));
                    let mut numerator = 0.0;
                    let mut denominator = 0.0;
                    if scale > 0.0 {
                        for (&r, &r_old) in current.residual.iter().zip(&old.residual) {
                            let dr = r / scale - r_old / scale;
                            numerator += (r / scale) * dr;
                            denominator += dr * dr;
                        }
                    }
                    out.work.residual_dot_passes += 1;
                    let gamma = numerator / denominator;
                    if gamma.is_finite() && (exact_dominance || gamma.abs() <= cfg.max_coefficient)
                    {
                        let trial: Vec<_> = current
                            .mapped
                            .iter()
                            .zip(&old.mapped)
                            .map(|(g, g_old)| g - gamma * (g - g_old))
                            .collect();
                        if trial.iter().all(|v| v.is_finite()) && map.admits(&trial) {
                            out.work.extrapolation_trials += 1;
                            let extrapolated =
                                evaluate(map, anchor, trial, lambda, epoch, cfg, &mut out.work)?;
                            let limit = if exact_dominance {
                                selected.norm
                            } else {
                                cfg.improvement_factor * selected.norm
                            };
                            if extrapolated.norm <= limit {
                                selected = extrapolated;
                                out.work.extrapolations_accepted += 1;
                            } else {
                                rejected += 1;
                                out.work.extrapolations_rejected += 1;
                            }
                        } else {
                            rejected += 1;
                            out.work.domain_skipped_trials += 1;
                        }
                    } else {
                        out.work.coefficient_skips += 1;
                    }
                } else {
                    out.work.policy_skips += 1;
                }
            }
            previous = Some(current);
            current = selected;
            iteration += 1;
        }
        seed = current.state;
        out.points.push(RhsHomotopyPoint {
            lambda,
            state: seed.clone(),
            residual_l2: current.norm,
            iterations: iteration,
            map_evaluations: out.work.map_attempted - calls_before,
        });
    }
    Ok(seed)
}

/// An analytically planned, model-specific physical-accuracy consumer.
///
/// The model is exactly `y'=-k*y-g*y^3`, with represented `k>0`, `g>=0`,
/// `y0>=0`. The smallest admitted dyadic time grid is selected from proved
/// truncation and contraction bounds. The remaining physical error budget is
/// assigned to actual algebraic step errors, verified by outward intervals.
/// Resource limits are caller-owned limits, not accuracy tuning parameters.
#[derive(Clone, Debug, Serialize)]
pub struct CubicDecayAccuracyPlan {
    k: f64,
    g: f64,
    y0: f64,
    horizon: f64,
    physical_tolerance: f64,
    steps: usize,
    h: f64,
    derivative_bound: f64,
    truncation_constant: f64,
    contraction_bound: f64,
    inverse_denominator_lower: f64,
    local_truncation_bound: f64,
    total_truncation_bound: f64,
    algebraic_step_budget: f64,
    stage_residual_target: f64,
    max_polynomial_evaluations: u64,
}

impl CubicDecayAccuracyPlan {
    pub fn new(
        k: f64,
        g: f64,
        y0: f64,
        horizon: f64,
        physical_tolerance: f64,
        max_steps: usize,
        max_polynomial_evaluations: u64,
    ) -> Result<Self, String> {
        use rodas5p_core::directed::{
            add_down, add_up, div_down, div_up, mul_down, mul_up, sub_down,
        };
        if ![k, g, y0, horizon, physical_tolerance]
            .iter()
            .all(|v| v.is_finite())
            || k <= 0.0
            || g < 0.0
            || y0 < 0.0
            || horizon <= 0.0
            || physical_tolerance <= 0.0
            || max_steps == 0
            || max_polynomial_evaluations == 0
        {
            return Err("invalid cubic-decay model, physical budget, or resource limit".into());
        }
        let calculate = || -> rodas5p_core::CoreResult<Self> {
            let y2 = mul_up(y0, y0)?;
            let phi = mul_up(y0, add_up(k, mul_up(g, y2)?)?)?;
            let ell = add_up(k, mul_up(3.0, mul_up(g, y2)?)?)?;
            // C=L*M2/8=L^2*phi(Y)/8. Positivity permits the difference of
            // two positive midpoint quadrature defects, not their sum.
            let c = div_up(mul_up(mul_up(ell, ell)?, phi)?, 8.0)?;
            let enlarged_y = add_up(y0, physical_tolerance)?;
            let enlarged_l = add_up(k, mul_up(3.0, mul_up(g, mul_up(enlarged_y, enlarged_y)?)?)?)?;
            let mut n = 1usize;
            loop {
                if n > max_steps || n as u128 > (1u128 << 52) {
                    return Err(rodas5p_core::CoreError::InvalidInput(
                        "proved dyadic grid exceeds caller's step resource limit".into(),
                    ));
                }
                let nf = n as f64;
                let h_lo = div_down(horizon, nf)?;
                let h = div_up(horizon, nf)?;
                if h <= 0.0 || h != h_lo || mul_down(h, nf)? != horizon || mul_up(h, nf)? != horizon
                {
                    return Err(rodas5p_core::CoreError::InvalidInput(
                        "dyadic time subdivision is not exact in the represented range".into(),
                    ));
                }
                let q = mul_up(mul_up(0.5, h)?, enlarged_l)?;
                let denominator = add_down(1.0, mul_down(mul_down(0.5, h)?, k)?)?;
                let local = div_up(mul_up(c, mul_up(mul_up(h, h)?, h)?)?, denominator)?;
                let total = mul_up(nf, local)?;
                if q < 1.0 && total < physical_tolerance {
                    let remainder = sub_down(physical_tolerance, total)?;
                    let algebraic = div_down(remainder, nf)?;
                    let residual = mul_down(denominator, algebraic)?;
                    if algebraic > 0.0 && residual > 0.0 {
                        return Ok(Self {
                            k,
                            g,
                            y0,
                            horizon,
                            physical_tolerance,
                            steps: n,
                            h,
                            derivative_bound: phi,
                            truncation_constant: c,
                            contraction_bound: q,
                            inverse_denominator_lower: denominator,
                            local_truncation_bound: local,
                            total_truncation_bound: total,
                            algebraic_step_budget: algebraic,
                            stage_residual_target: residual,
                            max_polynomial_evaluations,
                        });
                    }
                }
                n = n.checked_mul(2).ok_or_else(|| {
                    rodas5p_core::CoreError::InvalidInput("dyadic step count overflow".into())
                })?;
            }
        };
        calculate().map_err(|e| e.to_string())
    }
    pub fn steps(&self) -> usize {
        self.steps
    }
    pub fn step_size(&self) -> f64 {
        self.h
    }
    pub fn stage_residual_target(&self) -> f64 {
        self.stage_residual_target
    }
    pub fn physical_tolerance(&self) -> f64 {
        self.physical_tolerance
    }
    pub fn contraction_bound(&self) -> f64 {
        self.contraction_bound
    }
    pub fn truncation_bound(&self) -> f64 {
        self.total_truncation_bound
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CubicDecayAccuracyPoint {
    pub step: usize,
    pub time: f64,
    /// Encloses the exact mathematical time step*h.
    pub exact_time_interval: [f64; 2],
    pub state: f64,
    /// Error bound at the represented `time`, including its displacement.
    pub physical_error_bound: f64,
    pub algebraic_step_error_bound: f64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CubicDecayAccuracyResult {
    pub completed: bool,
    pub points: Vec<CubicDecayAccuracyPoint>,
    pub map_evaluations: u64,
    pub interval_polynomial_evaluations: u64,
    pub extrapolation_trials: u64,
    pub extrapolations_accepted: u64,
    pub polish_evaluations: u64,
    pub warm_seed_domain_resets: u64,
    /// Algebraic cubic inverse candidates, counted separately from polynomial
    /// evaluations because cube roots and square roots have different costs.
    pub algebraic_predictors: u64,
    /// Rational inverse candidates, with no square root or cube root.
    pub rational_predictors: u64,
    /// Candidates whose outward original residual needed an iterative repair.
    pub algebraic_predictor_fallbacks: u64,
    pub failure: Option<String>,
}

/// Value-only stage implementations for the sealed cubic-decay consumer.
/// All variants use the same plan and outward original-polynomial certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CubicStageSolver {
    /// Original Vec-valued homotopy implementation, retained as a control.
    GenericHomotopy(RhsAcceleration),
    /// The same scalar operations and callback order on stack-valued history.
    ScalarHomotopy(RhsAcceleration),
    /// A cancellation-free algebraic inverse, then the original residual
    /// certificate. Scalar Picard and bisection repair an inaccurate candidate.
    /// This specializes the declared polynomial, not an arbitrary RHS.
    AlgebraicPredictor,
    /// The [1/1] inverse-series rational candidate (1+2 beta)/(1+3 beta).
    /// Its exact normalized residual is beta^3*(3+8 beta)/(1+3 beta)^3.
    /// Acceptance and repair use the same outward original-polynomial check.
    RationalPredictor,
}

struct CubicStepMap<'a> {
    plan: &'a CubicDecayAccuracyPlan,
    y: f64,
}
impl RhsOnlyMap for CubicStepMap<'_> {
    fn dimension(&self) -> usize {
        1
    }
    fn admits(&self, x: &[f64]) -> bool {
        let end = -2.0 * self.y;
        x.len() == 1 && x[0].is_finite() && x[0] >= end.min(0.0) && x[0] <= end.max(0.0)
    }
    fn evaluate(&self, x: &[f64], out: &mut [f64]) -> Result<(), String> {
        let m = self.y + 0.5 * x[0];
        let nonlinear = if self.plan.g == 0.0 {
            0.0
        } else {
            self.plan.g * m * m * m
        };
        out[0] = -self.plan.h * (self.plan.k * m + nonlinear);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct CubicScalarValue {
    state: f64,
    mapped: f64,
    residual: f64,
    norm: f64,
}

fn cubic_scalar_value(
    map: &CubicStepMap<'_>,
    anchor: f64,
    state: f64,
    out: &mut CubicDecayAccuracyResult,
) -> Result<CubicScalarValue, String> {
    if !map.admits(&[state]) {
        return Err("scalar stage candidate is outside its exact increment bracket".into());
    }
    if out
        .map_evaluations
        .checked_add(out.interval_polynomial_evaluations)
        .is_none_or(|v| v >= map.plan.max_polynomial_evaluations)
    {
        return Err("polynomial evaluation resource limit reached before scalar dispatch".into());
    }
    out.map_evaluations += 1;
    let mut value = [f64::NAN];
    map.evaluate(&[state], &mut value)?;
    if !value[0].is_finite() {
        return Err("scalar polynomial map overflowed".into());
    }
    // Retain the original lambda=1 homotopy arithmetic, including signed zero.
    let mapped = 0.0 * anchor + 1.0 * value[0];
    let residual = mapped - state;
    let norm = 0.0_f64.hypot(residual);
    if !norm.is_finite() {
        return Err("scalar polynomial residual overflowed".into());
    }
    Ok(CubicScalarValue {
        state,
        mapped,
        residual,
        norm,
    })
}

fn cubic_scalar_stage(
    map: &CubicStepMap<'_>,
    seed: f64,
    acceleration: RhsAcceleration,
    out: &mut CubicDecayAccuracyResult,
) -> Result<f64, String> {
    let mut current = cubic_scalar_value(map, seed, seed, out)?;
    let mut previous: Option<CubicScalarValue> = None;
    let mut rejected = 0usize;
    while current.norm > map.plan.stage_residual_target {
        let mut selected = cubic_scalar_value(map, seed, current.mapped, out)?;
        if selected.norm > map.plan.stage_residual_target
            && acceleration != RhsAcceleration::Picard
            && let Some(old) = previous
        {
            let exact = acceleration == RhsAcceleration::ResidualDominance;
            // Legacy heuristic modes retain their existing defaults.
            if exact
                || (rejected < 2
                    && (acceleration == RhsAcceleration::Screened
                        || current.norm >= 0.5 * old.norm))
            {
                let scale = current.residual.abs().max(old.residual.abs());
                let (mut numerator, mut denominator) = (0.0, 0.0);
                if scale > 0.0 {
                    let dr = current.residual / scale - old.residual / scale;
                    numerator += (current.residual / scale) * dr;
                    denominator += dr * dr;
                }
                let gamma = numerator / denominator;
                if gamma.is_finite() && (exact || gamma.abs() <= 1024.0) {
                    let trial = current.mapped - gamma * (current.mapped - old.mapped);
                    if trial.is_finite() && map.admits(&[trial]) {
                        out.extrapolation_trials += 1;
                        let extrapolated = cubic_scalar_value(map, seed, trial, out)?;
                        let limit = if exact {
                            selected.norm
                        } else {
                            0.9 * selected.norm
                        };
                        if extrapolated.norm <= limit {
                            selected = extrapolated;
                            out.extrapolations_accepted += 1;
                        } else {
                            rejected += 1;
                        }
                    } else {
                        rejected += 1;
                    }
                }
            }
        }
        previous = Some(current);
        current = selected;
    }
    Ok(current.state)
}

fn cubic_model_predictor(
    map: &CubicStepMap<'_>,
    rational: bool,
    out: &mut CubicDecayAccuracyResult,
) -> Result<f64, String> {
    let p = map.plan;
    if rational {
        out.rational_predictors += 1;
    } else {
        out.algebraic_predictors += 1;
    }
    // a*m+b*m^3=y; m=(y/a)*s, s+beta*s^3=1.
    // u^3=z+sqrt(1+z^2), z=3*sqrt(3*beta)/2,
    // s=3/(u^2+1+u^-2). All sums are positive: no u-u^-1 subtraction.
    // Under q_B<1 and |y|<=Y+epsilon, beta<1/3 and 0<=z<3/2.
    let a = 1.0 + (0.5 * p.h) * p.k;
    let beta = ((0.5 * p.h) * (p.g * (map.y * map.y))) / (a * a * a);
    let s = if rational {
        // Coefficients follow by matching 1-beta+3 beta^2 in the inverse
        // series. The exact residual is beta^3*(3+8 beta)/(1+3 beta)^3.
        (1.0 + 2.0 * beta) / (1.0 + 3.0 * beta)
    } else {
        let z = 1.5 * (3.0 * beta).sqrt();
        let u = (z + z.hypot(1.0)).cbrt();
        let u2 = u * u;
        3.0 / (u2 + 1.0 + 1.0 / u2)
    };
    let midpoint = (map.y / a) * s;
    if !midpoint.is_finite() || !beta.is_finite() {
        return Err("algebraic cubic predictor overflowed before polynomial dispatch".into());
    }
    // Form a small increment by evaluating the RHS at the predicted midpoint,
    // rather than subtracting two nearly equal endpoint-scale numbers.
    if out
        .map_evaluations
        .checked_add(out.interval_polynomial_evaluations)
        .is_none_or(|v| v >= p.max_polynomial_evaluations)
    {
        return Err(
            "polynomial evaluation resource limit reached before predictor dispatch".into(),
        );
    }
    out.map_evaluations += 1;
    let nonlinear = if p.g == 0.0 {
        0.0
    } else {
        p.g * midpoint * midpoint * midpoint
    };
    let increment = -p.h * (p.k * midpoint + nonlinear);
    if !map.admits(&[increment]) {
        return Err("algebraic cubic predictor is outside its exact increment bracket".into());
    }
    Ok(increment)
}

fn cubic_verified_step(
    p: &CubicDecayAccuracyPlan,
    y: f64,
    increment: f64,
    out: &mut CubicDecayAccuracyResult,
) -> Result<(f64, f64, rodas5p_core::directed::Interval), String> {
    use rodas5p_core::directed::{Interval as I, add_up, div_up};
    if out
        .map_evaluations
        .checked_add(out.interval_polynomial_evaluations)
        .is_none_or(|v| v >= p.max_polynomial_evaluations)
    {
        return Err(
            "polynomial evaluation resource limit reached before interval validation".into(),
        );
    }
    out.interval_polynomial_evaluations += 1;
    let check = || -> rodas5p_core::CoreResult<(f64, f64, I)> {
        let m = I::point(y)?.add(I::point(increment)?.scale(0.5)?)?;
        let nonlinear = if p.g == 0.0 {
            I::point(0.0)?
        } else {
            m.mul(m)?.mul(m)?.scale(p.g)?
        };
        let phi = m.scale(p.k)?.add(nonlinear)?;
        let residual = I::point(increment)?.add(phi.scale(p.h)?)?;
        let next = y + increment;
        let displacement = I::exact_sum(y, increment)?.sub(I::point(next)?)?.mag();
        let algebraic = add_up(
            div_up(residual.mag(), p.inverse_denominator_lower)?,
            displacement,
        )?;
        Ok((next, algebraic, residual))
    };
    check().map_err(|e| e.to_string())
}

/// Integrate the sealed polynomial model on the plan's exact dyadic grid.
/// The physical bound uses nonexpansivity plus exact-model truncation bounds,
/// outward original-polynomial residuals and represented-update displacement.
/// It is not a generic callback certificate. Every map and interval polynomial
/// evaluation, including polishing work, is charged to the caller's resource cap.
pub fn integrate_cubic_decay_accuracy(
    plan: &CubicDecayAccuracyPlan,
    acceleration: RhsAcceleration,
    warm_previous_stage: bool,
) -> CubicDecayAccuracyResult {
    integrate_cubic_decay_accuracy_with_solver(
        plan,
        CubicStageSolver::GenericHomotopy(acceleration),
        warm_previous_stage,
    )
}

/// Integrate with an explicitly selected value-only stage implementation.
/// The algebraic predictor's library cube root is an untrusted candidate;
/// acceptance comes exclusively from the original outward residual bound.
pub fn integrate_cubic_decay_accuracy_with_solver(
    plan: &CubicDecayAccuracyPlan,
    solver: CubicStageSolver,
    warm_previous_stage: bool,
) -> CubicDecayAccuracyResult {
    let mut out = CubicDecayAccuracyResult::default();
    let run = cubic_integrate(plan, solver, warm_previous_stage, &mut out);
    match run {
        Ok(()) => out.completed = true,
        Err(e) => out.failure = Some(e),
    }
    out
}

fn cubic_integrate(
    p: &CubicDecayAccuracyPlan,
    solver: CubicStageSolver,
    warm: bool,
    out: &mut CubicDecayAccuracyResult,
) -> Result<(), String> {
    use rodas5p_core::directed::{Interval as I, add_up, mul_up};
    let mut y = p.y0;
    let mut previous_stage = 0.0;
    let mut accumulated_algebraic = 0.0;
    out.points.push(CubicDecayAccuracyPoint {
        step: 0,
        time: 0.0,
        exact_time_interval: [0.0, 0.0],
        state: y,
        physical_error_bound: 0.0,
        algebraic_step_error_bound: 0.0,
    });
    for i in 1..=p.steps {
        let used = out
            .map_evaluations
            .checked_add(out.interval_polynomial_evaluations)
            .ok_or("polynomial evaluation counter overflow")?;
        let remaining = p
            .max_polynomial_evaluations
            .checked_sub(used)
            .ok_or("polynomial evaluation resource limit exceeded")?;
        if remaining == 0 {
            return Err("polynomial evaluation resource limit exhausted".into());
        }
        let map = CubicStepMap { plan: p, y };
        let mut seed = if warm { previous_stage } else { 0.0 };
        if !map.admits(&[seed]) {
            seed = 0.0;
            out.warm_seed_domain_resets += 1;
        }
        let mut increment = match solver {
            CubicStageSolver::GenericHomotopy(acceleration) => {
                let cfg = RhsHomotopyConfig {
                    lambdas: vec![1.0],
                    residual_l2_tolerance: p.stage_residual_target,
                    max_iterations_per_lambda: usize::try_from(remaining).unwrap_or(usize::MAX),
                    max_map_evaluations: remaining,
                    acceleration,
                    ..RhsHomotopyConfig::default()
                };
                let attempt = solve_rhs_homotopy(&map, &[seed], &cfg);
                out.map_evaluations += attempt.work.map_attempted;
                out.extrapolation_trials += attempt.work.extrapolation_trials;
                out.extrapolations_accepted += attempt.work.extrapolations_accepted;
                attempt
                    .root
                    .ok_or_else(|| format!("stage {i}: {:?}", attempt.failure))?[0]
            }
            CubicStageSolver::ScalarHomotopy(acceleration) => {
                cubic_scalar_stage(&map, seed, acceleration, out)?
            }
            CubicStageSolver::AlgebraicPredictor => cubic_model_predictor(&map, false, out)?,
            CubicStageSolver::RationalPredictor => cubic_model_predictor(&map, true, out)?,
        };
        let mut verified = cubic_verified_step(p, y, increment, out)?;
        if verified.1 > p.algebraic_step_budget
            && matches!(
                solver,
                CubicStageSolver::AlgebraicPredictor | CubicStageSolver::RationalPredictor
            )
        {
            out.algebraic_predictor_fallbacks += 1;
            increment = cubic_scalar_stage(&map, seed, RhsAcceleration::Picard, out)?;
            verified = cubic_verified_step(p, y, increment, out)?;
        }
        if verified.1 > p.algebraic_step_budget {
            // Monotone bisection is value-only. Its exact bracket comes from
            // F(0)=h*phi(y), F(-2y)=-2y. No guessed radius is introduced.
            let endpoint = -2.0 * y;
            let (mut lo, mut hi) = (endpoint.min(0.0), endpoint.max(0.0));
            loop {
                let mid = lo + 0.5 * (hi - lo);
                if mid == lo || mid == hi {
                    return Err(
                        "stage residual budget reaches the represented rounding floor".into(),
                    );
                }
                verified = cubic_verified_step(p, y, mid, out)?;
                out.polish_evaluations += 1;
                increment = mid;
                if verified.1 <= p.algebraic_step_budget {
                    break;
                }
                if verified.2.lo > 0.0 {
                    hi = mid;
                } else if verified.2.hi < 0.0 {
                    lo = mid;
                } else {
                    return Err(
                        "outward stage residual straddles zero above the remaining physical budget"
                            .into(),
                    );
                }
            }
        }
        accumulated_algebraic =
            add_up(accumulated_algebraic, verified.1).map_err(|e| e.to_string())?;
        let elapsed = I::point(p.h)
            .and_then(|h| h.scale(i as f64))
            .map_err(|e| e.to_string())?;
        let label = p.h * i as f64;
        let time_displacement = elapsed
            .sub(I::point(label).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
            .mag();
        let bound = add_up(
            add_up(
                mul_up(i as f64, p.local_truncation_bound).map_err(|e| e.to_string())?,
                accumulated_algebraic,
            )
            .map_err(|e| e.to_string())?,
            mul_up(p.derivative_bound, time_displacement).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if bound > p.physical_tolerance {
            return Err(
                "accumulated outward physical error bound exceeds requested tolerance".into(),
            );
        }
        y = verified.0;
        previous_stage = increment;
        out.points.push(CubicDecayAccuracyPoint {
            step: i,
            time: label,
            exact_time_interval: [elapsed.lo, elapsed.hi],
            state: y,
            physical_error_bound: bound,
            algebraic_step_error_bound: verified.1,
        });
    }
    Ok(())
}

/// Opt-in RHS-only depth-one Anderson acceleration with a caller-proved
/// contraction factor, evaluating a speculative candidate before the usual
/// Picard residual when that can save a complete callback.
///
/// Preconditions beyond the checked numeric input: on a common invariant
/// domain the unchanged exact G must be L2-contractive by base_q, and the
/// homotopy stations must map that domain into itself. A caller-supplied
/// number is not itself a contraction certificate. At station lambda,
/// q_lambda=lambda*base_q. If the evaluated Anderson trial z has
/// ||H_lambda(z)-z|| <= q_lambda*||H_lambda(x)-x||, it can replace
/// Picard H_lambda(x) without evaluating H_lambda(H_lambda(x)).
///
/// This does NOT claim dominance over the uncomputed Picard residual.
/// Each accepted exact-real residual instead satisfies the same geometric
/// contraction envelope. If the trial misses that envelope, the original
/// Picard candidate is evaluated and the smaller measured residual chosen.
/// This is a numerical/operational test: outward callback and arithmetic
/// rounding bounds are still needed for a rigorous physical-error claim.
///
/// This function does not alter solve_rhs_homotopy or any default route;
/// it never requests a Jacobian/JVP/FD derivative. A trial is attempted
/// only if enough budget remains to evaluate a fallback Picard candidate.
pub fn solve_rhs_homotopy_contraction_fast_first<M: RhsOnlyMap + ?Sized>(
    map: &M,
    anchor: &[f64],
    cfg: &RhsHomotopyConfig,
    base_q: f64,
) -> RhsHomotopyAttempt {
    let mut out = RhsHomotopyAttempt::default();
    if !valid(cfg, map.dimension(), anchor)
        || cfg.acceleration != RhsAcceleration::ResidualDominance
        || !base_q.is_finite()
        || !(0.0..1.0).contains(&base_q)
    {
        out.failure = Some(failure(
            RhsHomotopyFailureKind::Input,
            "fast-first requires valid residual-dominance config and 0<=q<1",
        ));
        return out;
    }
    let epoch = map.model_epoch();
    let result = run_contraction_fast_first(map, anchor, cfg, epoch, base_q, &mut out);
    match result {
        Ok(root) => out.root = Some(root),
        Err(err) => out.failure = Some(err),
    }
    out
}

fn run_contraction_fast_first<M: RhsOnlyMap + ?Sized>(
    map: &M,
    anchor: &[f64],
    cfg: &RhsHomotopyConfig,
    epoch: Option<u64>,
    base_q: f64,
    out: &mut RhsHomotopyAttempt,
) -> Result<Vec<f64>, RhsHomotopyFailure> {
    let mut seed = anchor.to_vec();
    for &lambda in &cfg.lambdas {
        let q = lambda * base_q;
        out.work.continuation_stations += 1;
        let calls_before = out.work.map_attempted;
        let mut previous: Option<Value> = None;
        let mut current = evaluate(map, anchor, seed, lambda, epoch, cfg, &mut out.work)?;
        let mut iteration = 0;
        while current.norm > cfg.residual_l2_tolerance {
            if iteration == cfg.max_iterations_per_lambda {
                return Err(failure(
                    RhsHomotopyFailureKind::IterationLimit,
                    "fast-first station exhausted its update limit",
                ));
            }
            let mut trial_value: Option<Value> = None;
            // Leave at least one full callback charge for fallback Picard.
            if cfg.max_map_evaluations - out.work.map_attempted >= 2
                && let Some(old) = &previous
            {
                let scale = current
                    .residual
                    .iter()
                    .chain(&old.residual)
                    .fold(0.0_f64, |s, &v| s.max(v.abs()));
                let mut numerator = 0.0;
                let mut denominator = 0.0;
                if scale > 0.0 {
                    for (&r, &r_old) in current.residual.iter().zip(&old.residual) {
                        let dr = r / scale - r_old / scale;
                        numerator += (r / scale) * dr;
                        denominator += dr * dr;
                    }
                }
                out.work.residual_dot_passes += 1;
                let gamma = numerator / denominator;
                if gamma.is_finite() {
                    let trial: Vec<_> = current
                        .mapped
                        .iter()
                        .zip(&old.mapped)
                        .map(|(g, g_old)| g - gamma * (g - g_old))
                        .collect();
                    if trial.iter().all(|v| v.is_finite()) && map.admits(&trial) {
                        out.work.extrapolation_trials += 1;
                        let candidate =
                            evaluate(map, anchor, trial, lambda, epoch, cfg, &mut out.work)?;
                        if candidate.norm <= q * current.norm {
                            out.work.extrapolations_accepted += 1;
                            previous = Some(current);
                            current = candidate;
                            iteration += 1;
                            continue;
                        }
                        trial_value = Some(candidate);
                    } else {
                        out.work.domain_skipped_trials += 1;
                    }
                } else {
                    out.work.coefficient_skips += 1;
                }
            } else {
                out.work.policy_skips += 1;
            }
            let baseline = evaluate(
                map,
                anchor,
                current.mapped.clone(),
                lambda,
                epoch,
                cfg,
                &mut out.work,
            )?;
            out.work.picard_updates += 1;
            let selected = match trial_value {
                Some(candidate) if candidate.norm <= baseline.norm => {
                    out.work.extrapolations_accepted += 1;
                    candidate
                }
                Some(_) => {
                    out.work.extrapolations_rejected += 1;
                    baseline
                }
                None => baseline,
            };
            previous = Some(current);
            current = selected;
            iteration += 1;
        }
        seed = current.state;
        out.points.push(RhsHomotopyPoint {
            lambda,
            state: seed.clone(),
            residual_l2: current.norm,
            iterations: iteration,
            map_evaluations: out.work.map_attempted - calls_before,
        });
    }
    Ok(seed)
}
