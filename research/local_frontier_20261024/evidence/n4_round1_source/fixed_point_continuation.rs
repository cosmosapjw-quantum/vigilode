//! Opt-in, source-bound continuation using only F values and fixed P actions.
//!
//! A declared patch covers an infinity ball and a closed parameter interval.
//! Its provider must justify a uniform contraction bound for T(x)=x-P F(x),
//! the center displacement bound, continuity in lambda, and invertibility of
//! P. These are whole-domain declarations, not inferred from sampled calls.
//! Callback-supplied error bounds and outward scalar arithmetic bound the
//! original residual and the root error. No derivative, JVP, Jacobian, or
//! Newton callback exists in this interface. It does not replace the existing
//! general homotopy corrector or certify an ODE trajectory.

use rodas5p_core::directed::{Interval, add_up, div_up, mul_down, mul_up, sub_down};
use serde::Serialize;
use thiserror::Error;

/// Caller-owned identity of the immutable target and selected root branch.
/// IDs bind declarations; they do not prove that an opaque callback is truthful.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ContinuationIdentity {
    pub target_id: u64,
    pub model_epoch: u64,
    pub branch_id: u64,
}

/// Analytic/interval provider obligations in the infinity norm.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct FixedPointBounds {
    /// Uniform Lipschitz bound of x-P F(x,lambda), strictly below one.
    pub contraction_upper: f64,
    /// sup_lambda ||P F(center,lambda)||_infinity over the entire patch.
    pub center_correction_upper: f64,
    /// ||P||_infinity upper bound, used to transport F evaluation errors.
    pub inverse_action_norm_upper: f64,
    /// ||P^-1||_infinity upper bound; declares P nonsingular.
    pub inverse_action_inverse_norm_upper: f64,
}

/// Immutable admitted patch. Constructor checks scalar consistency only.
/// The provider remains responsible for the whole-domain mathematical bounds.
#[derive(Clone, Debug, Serialize)]
pub struct DeclaredFixedPointPatch {
    identity: ContinuationIdentity,
    patch_id: u64,
    inverse_id: u64,
    lambda_start: f64,
    lambda_end: f64,
    center: Vec<f64>,
    radius: f64,
    bounds: FixedPointBounds,
}

impl DeclaredFixedPointPatch {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: ContinuationIdentity,
        patch_id: u64,
        inverse_id: u64,
        lambda_start: f64,
        lambda_end: f64,
        center: Vec<f64>,
        radius: f64,
        bounds: FixedPointBounds,
    ) -> Result<Self, FixedPointContinuationError> {
        let b = bounds;
        if center.is_empty()
            || center.iter().any(|x| !x.is_finite())
            || !radius.is_finite() || radius <= 0.0
            || !lambda_start.is_finite() || !lambda_end.is_finite()
            || lambda_start < 0.0 || lambda_end > 1.0 || lambda_start >= lambda_end
            || !b.contraction_upper.is_finite()
            || !(0.0..1.0).contains(&b.contraction_upper)
            || !b.center_correction_upper.is_finite() || b.center_correction_upper < 0.0
            || !b.inverse_action_norm_upper.is_finite() || b.inverse_action_norm_upper <= 0.0
            || !b.inverse_action_inverse_norm_upper.is_finite()
            || b.inverse_action_inverse_norm_upper <= 0.0
        {
            return Err(FixedPointContinuationError::Input("invalid patch or bound"));
        }
        let margin = sub_down(1.0, b.contraction_upper).map_err(arithmetic)?;
        if b.center_correction_upper > mul_down(margin, radius).map_err(arithmetic)? {
            return Err(FixedPointContinuationError::Input("self-map bound is not satisfied"));
        }
        Ok(Self { identity, patch_id, inverse_id, lambda_start, lambda_end,
            center, radius, bounds })
    }

    pub fn patch_id(&self) -> u64 { self.patch_id }
    pub fn inverse_id(&self) -> u64 { self.inverse_id }
    pub fn parameter_interval(&self) -> (f64, f64) { (self.lambda_start, self.lambda_end) }
    pub fn center(&self) -> &[f64] { &self.center }
    pub fn radius(&self) -> f64 { self.radius }
    pub fn bounds(&self) -> FixedPointBounds { self.bounds }
}

/// Ordered patches covering exactly [0,1], with immutable branch identity.
#[derive(Clone, Debug, Serialize)]
pub struct DeclaredFixedPointAtlas {
    identity: ContinuationIdentity,
    patches: Vec<DeclaredFixedPointPatch>,
    dimension: usize,
}

impl DeclaredFixedPointAtlas {
    pub fn new(identity: ContinuationIdentity, patches: Vec<DeclaredFixedPointPatch>)
        -> Result<Self, FixedPointContinuationError>
    {
        let Some(first) = patches.first() else {
            return Err(FixedPointContinuationError::Input("empty atlas"));
        };
        let dimension = first.center.len();
        if first.lambda_start != 0.0 || patches.last().unwrap().lambda_end != 1.0 {
            return Err(FixedPointContinuationError::Input("atlas must cover exactly [0,1]"));
        }
        for (i, p) in patches.iter().enumerate() {
            if p.identity != identity || p.center.len() != dimension
                || (i > 0 && patches[i-1].lambda_end != p.lambda_start)
                || patches[..i].iter().any(|old| old.patch_id == p.patch_id)
            {
                return Err(FixedPointContinuationError::Input("patch identity, adjacency or dimension mismatch"));
            }
        }
        Ok(Self { identity, patches, dimension })
    }

    pub fn identity(&self) -> ContinuationIdentity { self.identity }
    pub fn dimension(&self) -> usize { self.dimension }
    pub fn patches(&self) -> &[DeclaredFixedPointPatch] { &self.patches }
}

/// Mathematical providers must be pure for one identity/epoch. F is jointly
/// continuous on the declared patches; fixed P is linear and nonsingular.
/// Both callback outputs must be fully written. Each returned scalar bounds
/// infinity-norm evaluation error against the *mathematical* F or P action at
/// the represented inputs. Zero is valid only for an exact evaluation.
pub trait FixedPointHomotopyProvider {
    fn identity(&self) -> ContinuationIdentity;
    fn dimension(&self) -> usize;
    fn inverse_identity(&self, patch_id: u64) -> u64;
    fn residual(&self, x: &[f64], lambda: f64, output: &mut [f64]) -> Result<f64, String>;
    fn apply_inverse(&self, patch_id: u64, input: &[f64], output: &mut [f64]) -> Result<f64, String>;
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct FixedPointContinuationConfig {
    pub residual_tolerance: f64,
    pub substeps_per_patch: usize,
    pub max_iterations_per_station: usize,
    /// Cumulative limits apply to the supplied work object, including failures.
    pub max_residual_calls: usize,
    pub max_inverse_calls: usize,
    pub max_stations: usize,
    pub max_dimension: usize,
}

impl Default for FixedPointContinuationConfig {
    fn default() -> Self {
        Self { residual_tolerance: 1e-10, substeps_per_patch: 1,
            max_iterations_per_station: 128, max_residual_calls: 4096,
            max_inverse_calls: 4096, max_stations: 256, max_dimension: 4096 }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FixedPointContinuationWork {
    pub residual_calls: usize,
    pub inverse_calls: usize,
    pub state_updates: usize,
    pub completed_stations: usize,
    pub branch_overlap_checks: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixedPointStation {
    pub patch_id: u64,
    pub lambda: f64,
    pub state_updates: usize,
    /// Original ||F(x,lambda)|| upper bound, including provider evaluation error.
    pub residual_upper: f64,
    /// Conditional on the declared whole-domain bounds and callback enclosures.
    pub conditional_root_error_upper: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixedPointContinuationReport {
    pub identity: ContinuationIdentity,
    pub stations: Vec<FixedPointStation>,
    pub final_residual_upper: f64,
    pub final_conditional_root_error_upper: f64,
}

#[derive(Debug, Error, PartialEq)]
pub enum FixedPointContinuationError {
    #[error("fixed-P continuation invalid input: {0}")]
    Input(&'static str),
    #[error("fixed-P continuation target, epoch, branch or inverse identity changed")]
    IdentityMismatch,
    #[error("fixed-P continuation exhausted {0} budget")]
    Budget(&'static str),
    #[error("fixed-P continuation callback {kind}: {message}")]
    Callback { kind: &'static str, message: String },
    #[error("fixed-P continuation callback did not supply finite values and a finite nonnegative bound")]
    InvalidCallbackOutput,
    #[error("fixed-P continuation iterate is outside patch {patch_id}")]
    OutsidePatch { patch_id: u64 },
    #[error("fixed-P continuation cannot transport the previous root enclosure into patch {patch_id}")]
    BranchOverlap { patch_id: u64 },
    #[error("fixed-P continuation failed to reach original residual tolerance at lambda={lambda}")]
    IterationLimit { lambda: f64 },
    #[error("fixed-P continuation arithmetic: {0}")]
    Arithmetic(String),
}

fn arithmetic(e: rodas5p_core::CoreError) -> FixedPointContinuationError {
    FixedPointContinuationError::Arithmetic(e.to_string())
}

fn distance_upper(a: &[f64], b: &[f64]) -> Result<f64, FixedPointContinuationError> {
    let mut distance = 0.0_f64;
    for (&x, &y) in a.iter().zip(b) {
        let d = Interval::point(x).and_then(|x| x.sub(Interval::point(y)?)).map_err(arithmetic)?;
        distance = distance.max(d.mag());
    }
    Ok(distance)
}

fn bound_ok(error: f64, values: &[f64]) -> bool {
    error.is_finite() && error >= 0.0 && values.iter().all(|x| x.is_finite())
}

fn norm(values: &[f64]) -> f64 { values.iter().fold(0.0, |n, v| n.max(v.abs())) }

fn identity<P: FixedPointHomotopyProvider + ?Sized>(
    provider: &P, atlas: &DeclaredFixedPointAtlas, patch: &DeclaredFixedPointPatch,
) -> Result<(), FixedPointContinuationError> {
    if provider.identity() != atlas.identity || provider.dimension() != atlas.dimension
        || provider.inverse_identity(patch.patch_id) != patch.inverse_id
    { return Err(FixedPointContinuationError::IdentityMismatch); }
    Ok(())
}

fn charge(count: &mut usize, limit: usize, kind: &'static str)
    -> Result<(), FixedPointContinuationError>
{
    if *count >= limit { return Err(FixedPointContinuationError::Budget(kind)); }
    *count += 1;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn station<P: FixedPointHomotopyProvider + ?Sized>(
    provider: &P, atlas: &DeclaredFixedPointAtlas, patch: &DeclaredFixedPointPatch,
    lambda: f64, config: &FixedPointContinuationConfig, x: &mut [f64],
    f: &mut [f64], delta: &mut [f64], trial: &mut [f64],
    work: &mut FixedPointContinuationWork,
) -> Result<FixedPointStation, FixedPointContinuationError> {
    for updates in 0..=config.max_iterations_per_station {
        identity(provider, atlas, patch)?;
        if distance_upper(x, &patch.center)? > patch.radius {
            return Err(FixedPointContinuationError::OutsidePatch { patch_id: patch.patch_id });
        }
        charge(&mut work.residual_calls, config.max_residual_calls, "F calls")?;
        f.fill(f64::NAN);
        let f_error = provider.residual(x, lambda, f)
            .map_err(|message| FixedPointContinuationError::Callback { kind: "F", message })?;
        identity(provider, atlas, patch)?;
        if !bound_ok(f_error, f) { return Err(FixedPointContinuationError::InvalidCallbackOutput); }
        let residual_upper = add_up(norm(f), f_error).map_err(arithmetic)?;
        charge(&mut work.inverse_calls, config.max_inverse_calls, "P calls")?;
        delta.fill(f64::NAN);
        let p_error = provider.apply_inverse(patch.patch_id, f, delta)
            .map_err(|message| FixedPointContinuationError::Callback { kind: "P", message })?;
        identity(provider, atlas, patch)?;
        if !bound_ok(p_error, delta) { return Err(FixedPointContinuationError::InvalidCallbackOutput); }
        let error = add_up(p_error,
            mul_up(patch.bounds.inverse_action_norm_upper, f_error).map_err(arithmetic)?)
            .map_err(arithmetic)?;
        let numerator = add_up(norm(delta), error).map_err(arithmetic)?;
        let denominator = sub_down(1.0, patch.bounds.contraction_upper).map_err(arithmetic)?;
        let root_error = div_up(numerator, denominator).map_err(arithmetic)?;
        if residual_upper <= config.residual_tolerance {
            charge(&mut work.completed_stations, config.max_stations, "stations")?;
            return Ok(FixedPointStation { patch_id: patch.patch_id, lambda,
                state_updates: updates, residual_upper, conditional_root_error_upper: root_error });
        }
        if updates == config.max_iterations_per_station { break; }
        for ((next, &value), &correction) in trial.iter_mut().zip(x.iter()).zip(delta.iter()) {
            *next = value - correction;
            if !next.is_finite() { return Err(FixedPointContinuationError::Arithmetic("nonfinite update".into())); }
        }
        if distance_upper(trial, &patch.center)? > patch.radius {
            return Err(FixedPointContinuationError::OutsidePatch { patch_id: patch.patch_id });
        }
        work.state_updates = work.state_updates.checked_add(1)
            .ok_or(FixedPointContinuationError::Budget("update counter"))?;
        x.copy_from_slice(trial);
    }
    Err(FixedPointContinuationError::IterationLimit { lambda })
}

/// Follow the connected declared branch from lambda=0 to exactly lambda=1.
/// The initial point is corrected at zero. Between patches, the old root's
/// enclosed ball must fit inside the new ball, proving equal roots at their
/// common parameter under truthful declarations. Merely sharing branch_id is
/// insufficient. Only after a fresh final F(x,1) check is `output` changed.
/// All other failures leave output untouched; attempted callback work remains.
pub fn continue_fixed_point<P: FixedPointHomotopyProvider + ?Sized>(
    provider: &P,
    atlas: &DeclaredFixedPointAtlas,
    config: &FixedPointContinuationConfig,
    initial: &[f64],
    output: &mut [f64],
    work: &mut FixedPointContinuationWork,
) -> Result<FixedPointContinuationReport, FixedPointContinuationError> {
    if initial.len() != atlas.dimension || output.len() != atlas.dimension
        || initial.iter().any(|x| !x.is_finite()) || atlas.dimension > config.max_dimension
        || !config.residual_tolerance.is_finite() || config.residual_tolerance <= 0.0
        || config.substeps_per_patch == 0 || config.max_iterations_per_station == 0
    { return Err(FixedPointContinuationError::Input("invalid state, tolerance or budget")); }
    let stations = atlas.patches.len().checked_mul(config.substeps_per_patch)
        .and_then(|n| n.checked_add(1)).ok_or(FixedPointContinuationError::Budget("station count"))?;
    if work.completed_stations.checked_add(stations).is_none_or(|n| n > config.max_stations) {
        return Err(FixedPointContinuationError::Budget("stations"));
    }
    // Admission for every inverse precedes all F/P callbacks.
    for patch in &atlas.patches { identity(provider, atlas, patch)?; }
    let mut x = initial.to_vec();
    let mut f = vec![0.0; atlas.dimension];
    let mut delta = vec![0.0; atlas.dimension];
    let mut trial = vec![0.0; atlas.dimension];
    let mut receipts = Vec::with_capacity(stations);
    receipts.push(station(provider, atlas, &atlas.patches[0], 0.0, config,
        &mut x, &mut f, &mut delta, &mut trial, work)?);
    for (index, patch) in atlas.patches.iter().enumerate() {
        if index > 0 {
            work.branch_overlap_checks = work.branch_overlap_checks.checked_add(1)
                .ok_or(FixedPointContinuationError::Budget("overlap counter"))?;
            let old_error = receipts.last().unwrap().conditional_root_error_upper;
            let inclusion = add_up(distance_upper(&x, &patch.center)?, old_error).map_err(arithmetic)?;
            if inclusion > patch.radius {
                return Err(FixedPointContinuationError::BranchOverlap { patch_id: patch.patch_id });
            }
        }
        for step in 1..=config.substeps_per_patch {
            let lambda = if step == config.substeps_per_patch { patch.lambda_end }
                else { patch.lambda_start + (patch.lambda_end-patch.lambda_start)
                    * (step as f64 / config.substeps_per_patch as f64) };
            if lambda <= receipts.last().unwrap().lambda || lambda > patch.lambda_end {
                return Err(FixedPointContinuationError::Input("unrepresentable parameter station"));
            }
            receipts.push(station(provider, atlas, patch, lambda, config,
                &mut x, &mut f, &mut delta, &mut trial, work)?);
        }
    }
    identity(provider, atlas, atlas.patches.last().unwrap())?;
    let final_station = receipts.last().unwrap();
    debug_assert_eq!(final_station.lambda, 1.0);
    let final_residual_upper = final_station.residual_upper;
    let final_conditional_root_error_upper = final_station.conditional_root_error_upper;
    output.copy_from_slice(&x);
    Ok(FixedPointContinuationReport { identity: atlas.identity, stations: receipts,
        final_residual_upper, final_conditional_root_error_upper })
}
