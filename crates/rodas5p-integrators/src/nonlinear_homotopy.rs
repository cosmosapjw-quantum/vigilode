//! Source-bound nonlinear window shooting with an independent ODE certificate.
//!
//! Candidate rounds use exact-subflow formulas evaluated in ordinary binary64:
//! those formulas and the complex vector prefix are NOT rounding certificates.
//! A final independent outward packet enclosure at every represented input,
//! followed by the scalar outward causal error prefix, certifies the returned
//! centers. Consequently failure of candidate convergence cannot forge success.
//! Only requested mesh endpoints are certified by this API.

use rayon::{join, prelude::*};
use rodas5p_core::CoreError;
use rodas5p_core::directed::{
    Interval, add_up, div_down, div_up, exp_interval, mul_up, sqrt_down, sqrt_up, sub_down, sub_up,
};
use serde::Serialize;
use thiserror::Error;

use crate::affine_transport::{AffineErrorMap, parallel_error_prefix};
use crate::damped_packet::{
    DampedPacketConfig, DampedPacketError, DampedPacketWork, verify_damped_packet_window,
};
use crate::packet_window::{
    PacketCurveOutput, PacketError, RepresentedPacketCurve, VerifiedPacketConfig,
};
use crate::{
    CausalWeights, CollectiveModel, PacketWorkCounters, compile_packet, generate_candidate,
};

type Complex = (f64, f64);

#[derive(Clone, Debug, Serialize)]
pub struct NonlinearHomotopyConfig {
    /// Positive number of exact-split candidate substeps per window.
    pub split_substeps: usize,
    /// Initial number of complete correction rounds per homotopy stage.
    pub correction_rounds: usize,
    /// Number of budget-driven retries; each doubles substeps and adds a round.
    pub max_refinements: usize,
    /// Strictly increasing values in [0,1], ending at exactly 1.0.
    pub lambdas: Vec<f64>,
    /// Absolute weighted-norm radius at EVERY requested mesh output.
    pub weighted_tolerance: f64,
    pub packet_order: usize,
    pub packet_degree: usize,
    pub packet_sweeps: usize,
    /// Local damped verifier may use at most 2^levels certified subwindows.
    pub damped_subdivision_levels: usize,
}

#[derive(Debug, Error)]
pub enum NonlinearHomotopyError {
    #[error("nonlinear homotopy invalid input: {0}")]
    InvalidInput(&'static str),
    #[error("nonlinear homotopy packet certificate requires zero carrier")]
    UnsupportedModel,
    #[error("nonlinear homotopy resource/work counter overflow")]
    ResourceLimit,
    #[error("nonlinear homotopy candidate produced nonfinite arithmetic")]
    NonFiniteCandidate,
    #[error("nonlinear homotopy could not project a represented center into its admitted ball")]
    ProjectionRefused,
    #[error("nonlinear homotopy center is outside the model constructor domain")]
    CenterAdmission,
    #[error("mesh interval {index} has no exact positive binary64 duration")]
    InexactDuration { index: usize },
    #[error("endpoint budgets exceeded after {attempts} attempts: {radius} > {tolerance}")]
    BudgetExceeded {
        attempts: usize,
        radius: f64,
        tolerance: f64,
    },
    #[error(transparent)]
    Arithmetic(#[from] CoreError),
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error(transparent)]
    Damped(#[from] DampedPacketError),
}

/// Attempted work survives typed failures. These are algorithm event counts,
/// not a FLOP, allocation, scheduling, or elapsed-time measurement.
#[derive(Clone, Debug, Default, Serialize)]
pub struct NonlinearHomotopyWork {
    pub attempts: u64,
    pub homotopy_stages: u64,
    pub correction_rounds: u64,
    pub fine_window_calls: u64,
    pub attempted_split_substeps: u64,
    pub vector_affine_compositions: u64,
    pub vector_mode_compositions: u64,
    pub projected_states: u64,
    pub projection_mode_inputs: u64,
    pub verification_window_calls: u64,
    pub scalar_prefix_compositions: u64,
    /// One entry per local verification attempt, in deterministic window order.
    pub packet_verification_work: Vec<PacketWorkCounters>,
    /// Additional damped verifier counts, one per verification attempt.
    pub damped_verification_work: Vec<DampedWindowWorkReceipt>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DampedWindowWorkReceipt {
    pub validation_attempts: u64,
    pub window_attempts: u64,
    pub subwindow_attempts: u64,
    pub rejected_windows: u64,
    pub scalar_operations: u64,
    pub moment_fallback_attempts: u64,
    pub copied_mode_items: u64,
    pub allocation_attempts: u64,
    pub moment_validation_attempts: u64,
    pub moment_kernel_attempts: u64,
    pub moment_width_checks: u64,
}
impl From<&DampedPacketWork> for DampedWindowWorkReceipt {
    fn from(w: &DampedPacketWork) -> Self {
        Self {
            validation_attempts: w.validation_attempts,
            window_attempts: w.window_attempts,
            subwindow_attempts: w.subwindow_attempts,
            rejected_windows: w.rejected_windows,
            scalar_operations: w.scalar_operations,
            moment_fallback_attempts: w.moment_fallback_attempts,
            copied_mode_items: w.copied_mode_items,
            allocation_attempts: w.allocation_attempts,
            moment_validation_attempts: w.moments.validation_attempts,
            moment_kernel_attempts: w.moments.kernel_attempts,
            moment_width_checks: w.moments.width_checks,
        }
    }
}

fn charge(counter: &mut u64, amount: usize) -> Result<(), NonlinearHomotopyError> {
    *counter = counter
        .checked_add(u64::try_from(amount).map_err(|_| NonlinearHomotopyError::ResourceLimit)?)
        .ok_or(NonlinearHomotopyError::ResourceLimit)?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct VerifiedHomotopyPoint {
    time: f64,
    modes: Vec<Complex>,
    weighted_radius: f64,
    mode_radii: Vec<f64>,
}
impl VerifiedHomotopyPoint {
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn modes(&self) -> &[Complex] {
        &self.modes
    }
    pub fn weighted_radius(&self) -> f64 {
        self.weighted_radius
    }
    pub fn mode_radii(&self) -> &[f64] {
        &self.mode_radii
    }
}

/// Constructible only after all final lambda=1 endpoint certificates pass.
#[derive(Clone, Debug)]
pub struct VerifiedNonlinearHomotopy {
    source: CollectiveModel,
    mesh: Vec<f64>,
    config: NonlinearHomotopyConfig,
    points: Vec<VerifiedHomotopyPoint>,
    final_substeps: usize,
    final_rounds: usize,
}
impl VerifiedNonlinearHomotopy {
    pub fn source_model(&self) -> &CollectiveModel {
        &self.source
    }
    pub fn mesh(&self) -> &[f64] {
        &self.mesh
    }
    pub fn config(&self) -> &NonlinearHomotopyConfig {
        &self.config
    }
    pub fn points(&self) -> &[VerifiedHomotopyPoint] {
        &self.points
    }
    pub fn final_substeps(&self) -> usize {
        self.final_substeps
    }
    pub fn final_rounds(&self) -> usize {
        self.final_rounds
    }
}

fn finite(z: Complex) -> Result<Complex, NonlinearHomotopyError> {
    if z.0.is_finite() && z.1.is_finite() {
        Ok(z)
    } else {
        Err(NonlinearHomotopyError::NonFiniteCandidate)
    }
}
fn cmul(a: Complex, b: Complex) -> Result<Complex, NonlinearHomotopyError> {
    finite((a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0))
}
fn cadd(a: Complex, b: Complex) -> Result<Complex, NonlinearHomotopyError> {
    finite((a.0 + b.0, a.1 + b.1))
}
fn rotation(model: &CollectiveModel, h: f64) -> Result<Vec<Complex>, NonlinearHomotopyError> {
    let attenuation = (-model.kappa() * h).exp();
    model
        .xi()
        .iter()
        .map(|&x| {
            let angle = model.width() * x * h;
            if !angle.is_finite() {
                return Err(NonlinearHomotopyError::NonFiniteCandidate);
            }
            let (s, c) = angle.sin_cos();
            finite((attenuation * c, attenuation * s))
        })
        .collect()
}
fn apply_diagonal(q: &[Complex], y: &[Complex]) -> Result<Vec<Complex>, NonlinearHomotopyError> {
    q.iter().zip(y).map(|(&q, &z)| cmul(q, z)).collect()
}

// Ordinary binary64 candidate; exact B-flow formula uses the actual S.
fn split_candidate(
    model: &CollectiveModel,
    input: &[Complex],
    h: f64,
    m: usize,
    lambda: f64,
) -> Result<Vec<Complex>, NonlinearHomotopyError> {
    let tau = h / m as f64;
    if tau <= 0.0 || !tau.is_finite() {
        return Err(NonlinearHomotopyError::NonFiniteCandidate);
    }
    let half = rotation(model, tau * 0.5)?;
    let sum_w: f64 = model.weights().iter().sum();
    let mut y = input.to_vec();
    for _ in 0..m {
        y = apply_diagonal(&half, &y)?;
        let mut mean = (0.0, 0.0);
        for (&w, &z) in model.weights().iter().zip(&y) {
            mean = cadd(mean, (w * z.0, w * z.1))?;
        }
        let angle = lambda * model.chi() * sum_w * (mean.0 * mean.0 + mean.1 * mean.1) * tau;
        if !angle.is_finite() {
            return Err(NonlinearHomotopyError::NonFiniteCandidate);
        }
        // cos(angle)-1 via -2sin(angle/2)^2 avoids cancellation in the kick.
        let half_s = (angle * 0.5).sin();
        let multiplier = (-2.0 * half_s * half_s, angle.sin());
        let kick = cmul((mean.0 / sum_w, mean.1 / sum_w), multiplier)?;
        for z in &mut y {
            *z = cadd(*z, kick)?;
        }
        y = apply_diagonal(&half, &y)?;
    }
    Ok(y)
}

#[derive(Clone, Debug)]
struct VectorAffine {
    q: Vec<Complex>,
    b: Vec<Complex>,
}
impl VectorAffine {
    fn identity(n: usize) -> Self {
        Self {
            q: vec![(1.0, 0.0); n],
            b: vec![(0.0, 0.0); n],
        }
    }
    fn after(&self, before: &Self) -> Result<Self, NonlinearHomotopyError> {
        let q = self
            .q
            .iter()
            .zip(&before.q)
            .map(|(&a, &b)| cmul(a, b))
            .collect::<Result<Vec<_>, _>>()?;
        let b = self
            .q
            .iter()
            .zip(&before.b)
            .zip(&self.b)
            .map(|((&q, &b), &own)| cadd(cmul(q, b)?, own))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { q, b })
    }
    fn apply(&self, y: &[Complex]) -> Result<Vec<Complex>, NonlinearHomotopyError> {
        self.q
            .iter()
            .zip(y)
            .zip(&self.b)
            .map(|((&q, &y), &b)| cadd(cmul(q, y)?, b))
            .collect()
    }
}
fn build_prefix(
    maps: &[VectorAffine],
    tree: &mut [VectorAffine],
) -> Result<(), NonlinearHomotopyError> {
    if maps.len() == 1 {
        tree[0] = maps[0].clone();
        return Ok(());
    }
    let mid = maps.len() / 2;
    let (root, rest) = tree.split_at_mut(1);
    let (left, right) = rest.split_at_mut(2 * mid - 1);
    let (a, b) = join(
        || build_prefix(&maps[..mid], left),
        || build_prefix(&maps[mid..], right),
    );
    a?;
    b?;
    root[0] = right[0].after(&left[0])?;
    Ok(())
}
fn distribute_prefix(
    tree: &[VectorAffine],
    prefix: &VectorAffine,
    y0: &[Complex],
    out: &mut [Vec<Complex>],
) -> Result<(), NonlinearHomotopyError> {
    if out.len() == 1 {
        out[0] = tree[0].after(prefix)?.apply(y0)?;
        return Ok(());
    }
    let mid = out.len() / 2;
    let (left, right) = tree[1..].split_at(2 * mid - 1);
    let rp = left[0].after(prefix)?;
    let (lo, hi) = out.split_at_mut(mid);
    let (a, b) = join(
        || distribute_prefix(left, prefix, y0, lo),
        || distribute_prefix(right, &rp, y0, hi),
    );
    a?;
    b?;
    Ok(())
}
fn vector_prefix(
    maps: &[VectorAffine],
    initial: &[Complex],
    work: &mut NonlinearHomotopyWork,
) -> Result<Vec<Vec<Complex>>, NonlinearHomotopyError> {
    let m = maps.len();
    let n = initial.len();
    let compositions = m
        .checked_mul(3)
        .and_then(|v| v.checked_sub(2))
        .ok_or(NonlinearHomotopyError::ResourceLimit)?;
    // Charge a complete attempted scan before any fallible arithmetic. This is
    // deliberately an attempted upper count if a branch refuses early.
    charge(&mut work.vector_affine_compositions, compositions)?;
    charge(
        &mut work.vector_mode_compositions,
        compositions
            .checked_mul(n)
            .ok_or(NonlinearHomotopyError::ResourceLimit)?,
    )?;
    let mut tree = vec![VectorAffine::identity(n); 2 * m - 1];
    let mut out = vec![Vec::new(); m];
    build_prefix(maps, &mut tree)?;
    distribute_prefix(&tree, &VectorAffine::identity(n), initial, &mut out)?;
    Ok(out)
}

fn energy_upper(weights: &[f64], y: &[Complex]) -> Result<f64, NonlinearHomotopyError> {
    let mut e = 0.0;
    for (&w, &z) in weights.iter().zip(y) {
        e = add_up(e, mul_up(w, add_up(mul_up(z.0, z.0)?, mul_up(z.1, z.1)?)?)?)?;
    }
    Ok(e)
}
fn difference_upper(
    weights: &[f64],
    a: &[Complex],
    b: &[Complex],
) -> Result<f64, NonlinearHomotopyError> {
    let mut e = 0.0;
    for ((&w, &a), &b) in weights.iter().zip(a).zip(b) {
        let re = Interval::point(a.0)?.sub(Interval::point(b.0)?)?.mag();
        let im = Interval::point(a.1)?.sub(Interval::point(b.1)?)?.mag();
        e = add_up(e, mul_up(w, add_up(mul_up(re, re)?, mul_up(im, im)?)?)?)?;
    }
    Ok(sqrt_up(e)?)
}
fn project_inward(
    weights: &[f64],
    y: &mut [Complex],
    energy: f64,
) -> Result<(), NonlinearHomotopyError> {
    for _ in 0..4 {
        let current = energy_upper(weights, y)?;
        if current <= energy {
            return Ok(());
        }
        let scale = sqrt_down(div_down(energy, current)?.max(0.0))?;
        // A small inward margin is checked below; it is not assumed sufficient.
        let scale = scale * (1.0 - 16.0 * f64::EPSILON);
        for z in y.iter_mut() {
            *z = finite((scale * z.0, scale * z.1))?;
        }
    }
    if energy_upper(weights, y)? <= energy {
        Ok(())
    } else {
        Err(NonlinearHomotopyError::ProjectionRefused)
    }
}

fn local_ode_endpoint(
    model: &CollectiveModel,
    input: &[Complex],
    h: f64,
    config: &NonlinearHomotopyConfig,
    verification_tolerance: f64,
    damped_work: &mut DampedPacketWork,
) -> Result<(Vec<Complex>, f64), NonlinearHomotopyError> {
    let rebound = CollectiveModel::new(
        model.xi().to_vec(),
        model.weights().to_vec(),
        input.to_vec(),
        model.width(),
        model.chi(),
        0.0,
        model.kappa(),
    )
    .map_err(|_| NonlinearHomotopyError::CenterAdmission)?;
    if model.kappa() > 0.0 {
        let cfg = DampedPacketConfig {
            packet: VerifiedPacketConfig {
                order: config.packet_order,
                degree: config.packet_degree,
                sweeps: config.packet_sweeps,
                mean_tolerance: verification_tolerance,
                mode_tolerance: verification_tolerance,
            },
            max_subdivision_levels: config.damped_subdivision_levels,
            moment_width_limit: f64::MAX,
        };
        let point = verify_damped_packet_window(&rebound, h, 0.0, cfg, damped_work)?;
        return Ok((point.modes().to_vec(), point.weighted_radius()));
    }
    let packet_work = &mut damped_work.packet;
    let packet = compile_packet(&rebound, h, config.packet_order, packet_work)?;
    let force_degree = config
        .packet_degree
        .checked_mul(3)
        .ok_or(NonlinearHomotopyError::ResourceLimit)?;
    let table = CausalWeights::compile(config.packet_order, force_degree);
    let candidate = generate_candidate(
        &packet,
        config.packet_degree,
        config.packet_sweeps,
        &table,
        packet_work,
    )?;
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, packet_work)?;
    let output = PacketCurveOutput::new(curve);
    let mut ws = output.workspace(packet_work)?;
    let view = output.enclose_at_weighted(h, 0.0, &mut ws, packet_work)?;
    let radius = view
        .centered_weighted_radius()
        .ok_or(NonlinearHomotopyError::InvalidInput(
            "packet centered weighted radius absent",
        ))?;
    Ok((view.modes().to_vec(), radius))
}

/// Build, refine, and independently verify a nonlinear window candidate.
///
/// The returned model is the original immutable lambda=1 source. Projection is
/// after the entire affine vector scan, and every represented center is checked
/// to lie in the fixed energy ball before it becomes a fine-map input. This API
/// admits exact represented initial data and zero carrier, with nonnegative common damping.
/// All final source-specific enclosures must pass unchanged absolute tolerance.
/// Candidate transcendental/scan rounding is included through the independently
/// recomputed final defect, not through an unimplemented iteration-roundoff bound.
pub fn integrate_nonlinear_homotopy(
    model: &CollectiveModel,
    mesh: &[f64],
    config: &NonlinearHomotopyConfig,
    work: &mut NonlinearHomotopyWork,
) -> Result<VerifiedNonlinearHomotopy, NonlinearHomotopyError> {
    if model.carrier() != 0.0 {
        return Err(NonlinearHomotopyError::UnsupportedModel);
    }
    if mesh.len() < 2
        || mesh.iter().any(|t| !t.is_finite())
        || mesh.windows(2).any(|p| p[1] <= p[0])
        || config.split_substeps == 0
        || config.correction_rounds == 0
        || config.packet_sweeps == 0
        || config.packet_degree < config.packet_order
        || !config.weighted_tolerance.is_finite()
        || config.weighted_tolerance <= 0.0
        || config.lambdas.is_empty()
        || config.lambdas.last() != Some(&1.0)
        || config
            .lambdas
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0 || *x > 1.0)
        || config.lambdas.windows(2).any(|p| p[1] <= p[0])
    {
        return Err(NonlinearHomotopyError::InvalidInput(
            "mesh, schedule, iteration, packet, or absolute budget",
        ));
    }
    let m = mesh.len() - 1;
    let n = model.num_modes();
    m.checked_mul(n)
        .and_then(|x| x.checked_mul(8))
        .filter(|&x| x <= isize::MAX as usize / std::mem::size_of::<Complex>())
        .ok_or(NonlinearHomotopyError::ResourceLimit)?;
    let force_degree = config
        .packet_degree
        .checked_mul(3)
        .ok_or(NonlinearHomotopyError::ResourceLimit)?;
    config
        .packet_order
        .checked_add(1)
        .and_then(|r| force_degree.checked_add(1).and_then(|c| r.checked_mul(c)))
        .filter(|&x| x <= isize::MAX as usize / std::mem::size_of::<f64>())
        .ok_or(NonlinearHomotopyError::ResourceLimit)?;
    let mut durations = Vec::with_capacity(m);
    for (index, pair) in mesh.windows(2).enumerate() {
        let lo = sub_down(pair[1], pair[0])?;
        let hi = sub_up(pair[1], pair[0])?;
        if lo != hi || lo <= 0.0 {
            return Err(NonlinearHomotopyError::InexactDuration { index });
        }
        durations.push(lo);
    }
    let source_initial = model.initial_amplitudes();
    let energy = energy_upper(model.weights(), source_initial)?;
    let mut s = 0.0;
    for &w in model.weights() {
        s = add_up(s, w)?;
    }
    let c = mul_up(model.chi().abs(), mul_up(mul_up(s, s)?, energy)?)?;
    let mut horizon = 0.0;
    for &h in &durations {
        horizon = add_up(horizon, h)?;
    }
    let total_gain = exp_interval(mul_up(c, horizon)?)?.hi;
    let budget_divisor = mul_up(
        mul_up(4.0, add_up(m as f64, 1.0)?)?,
        mul_up(total_gain, sqrt_up(s)?.max(1.0))?,
    )?;
    let verification_tolerance = div_down(config.weighted_tolerance, budget_divisor)?;
    if verification_tolerance <= 0.0 {
        return Err(NonlinearHomotopyError::InvalidInput(
            "local verification budget underflow",
        ));
    }
    let q: Vec<_> = durations
        .iter()
        .map(|&h| rotation(model, h))
        .collect::<Result<_, _>>()?;
    let mut last_radius = f64::INFINITY;
    for refinement in 0..=config.max_refinements {
        charge(&mut work.attempts, 1)?;
        let factor = 1usize
            .checked_shl(
                u32::try_from(refinement).map_err(|_| NonlinearHomotopyError::ResourceLimit)?,
            )
            .ok_or(NonlinearHomotopyError::ResourceLimit)?;
        let substeps = config
            .split_substeps
            .checked_mul(factor)
            .ok_or(NonlinearHomotopyError::ResourceLimit)?;
        if substeps == 0 {
            return Err(NonlinearHomotopyError::ResourceLimit);
        }
        let rounds = config
            .correction_rounds
            .checked_add(refinement)
            .ok_or(NonlinearHomotopyError::ResourceLimit)?
            .min(m);
        let coarse: Vec<_> = q
            .iter()
            .map(|q| VectorAffine {
                q: q.clone(),
                b: vec![(0.0, 0.0); n],
            })
            .collect();
        let mut z = Vec::with_capacity(m + 1);
        z.push(source_initial.to_vec());
        z.extend(vector_prefix(&coarse, source_initial, work)?);
        charge(&mut work.projected_states, m)?;
        charge(
            &mut work.projection_mode_inputs,
            m.checked_mul(n)
                .ok_or(NonlinearHomotopyError::ResourceLimit)?,
        )?;
        for y in &mut z[1..] {
            project_inward(model.weights(), y, energy)?;
        }
        for &lambda in &config.lambdas {
            charge(&mut work.homotopy_stages, 1)?;
            for _ in 0..rounds {
                charge(&mut work.correction_rounds, 1)?;
                charge(&mut work.fine_window_calls, m)?;
                charge(
                    &mut work.attempted_split_substeps,
                    m.checked_mul(substeps)
                        .ok_or(NonlinearHomotopyError::ResourceLimit)?,
                )?;
                let maps: Vec<Result<VectorAffine, NonlinearHomotopyError>> = (0..m)
                    .into_par_iter()
                    .map(|j| {
                        let fine = split_candidate(model, &z[j], durations[j], substeps, lambda)?;
                        let coarse = apply_diagonal(&q[j], &z[j])?;
                        let b = fine
                            .iter()
                            .zip(coarse)
                            .map(|(&a, b)| finite((a.0 - b.0, a.1 - b.1)))
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok(VectorAffine { q: q[j].clone(), b })
                    })
                    .collect();
                let maps = maps.into_iter().collect::<Result<Vec<_>, _>>()?;
                let mut next = vector_prefix(&maps, source_initial, work)?;
                charge(&mut work.projected_states, m)?;
                charge(
                    &mut work.projection_mode_inputs,
                    m.checked_mul(n)
                        .ok_or(NonlinearHomotopyError::ResourceLimit)?,
                )?;
                next.par_iter_mut()
                    .try_for_each(|y| project_inward(model.weights(), y, energy))?;
                z.truncate(1);
                z.extend(next);
            }
        }
        // These windows are independent because each input is the FINAL fixed
        // represented center, not a concurrently changing predecessor.
        charge(&mut work.verification_window_calls, m)?;
        let verified: Vec<_> = (0..m)
            .into_par_iter()
            .map(|j| {
                let mut dw = DampedPacketWork::default();
                let answer = local_ode_endpoint(
                    model,
                    &z[j],
                    durations[j],
                    config,
                    verification_tolerance,
                    &mut dw,
                );
                (answer, dw)
            })
            .collect();
        let mut local = Vec::with_capacity(m);
        for (answer, dw) in verified {
            work.packet_verification_work.push(dw.packet);
            work.damped_verification_work.push((&dw).into());
            local.push(answer);
        }
        let local = local.into_iter().collect::<Result<Vec<_>, _>>()?;
        let maps: Vec<_> = local
            .iter()
            .enumerate()
            .map(|(j, (center, radius))| {
                let residual = add_up(
                    difference_upper(model.weights(), &z[j + 1], center)?,
                    *radius,
                )?;
                let gain = exp_interval(mul_up(c, durations[j])?)?.hi;
                Ok(AffineErrorMap::new(gain, residual)?)
            })
            .collect::<Result<Vec<_>, NonlinearHomotopyError>>()?;
        let prefix = parallel_error_prefix(&maps, 0.0)?;
        charge(&mut work.scalar_prefix_compositions, prefix.compositions())?;
        last_radius = prefix.radii()[1..].iter().copied().fold(0.0, f64::max);
        if last_radius <= config.weighted_tolerance {
            let points = z
                .into_iter()
                .skip(1)
                .enumerate()
                .map(|(j, modes)| {
                    let radius = prefix.radii()[j + 1];
                    let mode_radii = model
                        .weights()
                        .iter()
                        .map(|&w| Ok(div_up(radius, sqrt_down(w)?)?))
                        .collect::<Result<Vec<_>, NonlinearHomotopyError>>()?;
                    Ok(VerifiedHomotopyPoint {
                        time: mesh[j + 1],
                        modes,
                        weighted_radius: radius,
                        mode_radii,
                    })
                })
                .collect::<Result<Vec<_>, NonlinearHomotopyError>>()?;
            return Ok(VerifiedNonlinearHomotopy {
                source: model.clone(),
                mesh: mesh.to_vec(),
                config: config.clone(),
                points,
                final_substeps: substeps,
                final_rounds: rounds,
            });
        }
    }
    Err(NonlinearHomotopyError::BudgetExceeded {
        attempts: config.max_refinements + 1,
        radius: last_radius,
        tolerance: config.weighted_tolerance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complex_prefix_preserves_causal_order_and_tree_shape() {
        let maps = vec![
            VectorAffine {
                q: vec![(0.0, 1.0)],
                b: vec![(1.0, 0.0)],
            },
            VectorAffine {
                q: vec![(2.0, 0.0)],
                b: vec![(0.0, 1.0)],
            },
            VectorAffine {
                q: vec![(0.0, -1.0)],
                b: vec![(2.0, 2.0)],
            },
        ];
        let mut work = NonlinearHomotopyWork::default();
        let out = vector_prefix(&maps, &[(1.0, 2.0)], &mut work).unwrap();
        let mut serial = vec![(1.0, 2.0)];
        for (map, got) in maps.iter().zip(&out) {
            serial = map.apply(&serial).unwrap();
            assert_eq!(&serial, got);
        }
        assert_eq!(work.vector_affine_compositions, 7);
    }
    #[test]
    fn inward_projection_proves_represented_membership() {
        let mut y = vec![(10.0, 5.0), (-20.0, 2.0)];
        project_inward(&[0.25, 0.75], &mut y, 0.5).unwrap();
        assert!(energy_upper(&[0.25, 0.75], &y).unwrap() <= 0.5);
    }
}
