//! Verified common-damping packet windows by an exact damping frame.
//!
//! The transformed cubic coefficient is `chi*exp(-2*kappa*t)`. A damped
//! moment supplies its window average. The existing frozen polynomial packet
//! evaluates that averaged equation, while an explicit variation enclosure
//! accounts for the difference from the original time-dependent coefficient.
//! This is an opt-in bounded client, not a claim that coefficient freezing is
//! exact for unequal frequencies. Only the source's rotating frame is admitted.

use rodas5p_core::CoreError;
use rodas5p_core::damped_moments::{DampedForcingMomentWork, compute_damped_forcing_moment};
use rodas5p_core::directed::{
    Interval, add_up, div_up, exp_interval, mul_up, sqrt_down, sqrt_up, sub_down, sub_up,
};
use thiserror::Error;

use crate::packet_window::{
    CausalWeights, PacketCurveOutput, PacketError, RepresentedPacketCurve, VerifiedPacketConfig,
    compile_packet, generate_candidate,
};
use crate::{CollectiveModel, PacketWorkCounters};

#[derive(Clone, Copy, Debug)]
pub struct DampedPacketConfig {
    pub packet: VerifiedPacketConfig,
    /// At most 2^levels subwindows in each requested physical window.
    pub max_subdivision_levels: usize,
    /// Absolute width budget for the physical averaged-forcing moment.
    pub moment_width_limit: f64,
}

/// Attempt counters include rejected trials. These are not allocator counts.
#[derive(Clone, Debug, Default)]
pub struct DampedPacketWork {
    pub packet: PacketWorkCounters,
    pub moments: DampedForcingMomentWork,
    pub validation_attempts: u64,
    pub window_attempts: u64,
    pub subwindow_attempts: u64,
    pub rejected_windows: u64,
    pub scalar_operations: u64,
    pub moment_fallback_attempts: u64,
    pub copied_mode_items: u64,
    pub allocation_attempts: u64,
}

#[derive(Debug, Error)]
pub enum DampedPacketError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error("damped packet requires the source rotating frame (carrier=0)")]
    UnsupportedCarrier,
    #[error("invalid damped packet input: {0}")]
    InvalidInput(&'static str),
    #[error("physical duration is not exactly represented")]
    InexactDuration,
    #[error("damped moment width {width} exceeds {limit}")]
    MomentWidth { width: f64, limit: f64 },
    #[error("damped packet center leaves the source constructor domain")]
    CenterAdmission,
    #[error("damped packet budgets unmet: mean={mean_radius}, mode={mode_radius}")]
    BudgetExceeded { mean_radius: f64, mode_radius: f64 },
}

macro_rules! scalar {
    ($w:expr, $e:expr) => {{
        $w.scalar_operations = $w.scalar_operations.saturating_add(1);
        $e.map_err(DampedPacketError::Core)?
    }};
}

/// Immutable returned-center enclosure of the original damped source flow.
#[derive(Clone, Debug)]
pub struct VerifiedDampedPacketPoint {
    time: f64,
    modes: Vec<(f64, f64)>,
    mode_radii: Vec<f64>,
    mean: (f64, f64),
    mean_radius: f64,
    weighted_radius: f64,
    subwindows: usize,
}

impl VerifiedDampedPacketPoint {
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn modes(&self) -> &[(f64, f64)] {
        &self.modes
    }
    pub fn mode_radii(&self) -> &[f64] {
        &self.mode_radii
    }
    pub fn mean(&self) -> (f64, f64) {
        self.mean
    }
    pub fn mean_radius(&self) -> f64 {
        self.mean_radius
    }
    pub fn weighted_radius(&self) -> f64 {
        self.weighted_radius
    }
    pub fn subwindows(&self) -> usize {
        self.subwindows
    }
}

#[derive(Clone, Debug)]
pub struct VerifiedDampedPacketTrajectory {
    source: CollectiveModel,
    initial_time: f64,
    initial_radii: Vec<f64>,
    points: Vec<VerifiedDampedPacketPoint>,
}

impl VerifiedDampedPacketTrajectory {
    pub fn source_model(&self) -> &CollectiveModel {
        &self.source
    }
    pub fn initial_time(&self) -> f64 {
        self.initial_time
    }
    pub fn initial_radii(&self) -> &[f64] {
        &self.initial_radii
    }
    pub fn points(&self) -> &[VerifiedDampedPacketPoint] {
        &self.points
    }
}

/// An immutable descriptor: caller changes cannot rebind its source or horizon.
pub struct DampedPacketDescriptor {
    source: CollectiveModel,
    horizon: f64,
    config: DampedPacketConfig,
}

impl DampedPacketDescriptor {
    pub fn new(
        source: &CollectiveModel,
        horizon: f64,
        config: DampedPacketConfig,
        work: &mut DampedPacketWork,
    ) -> Result<Self, DampedPacketError> {
        validate(source, horizon, 0.0, config, work)?;
        work.allocation_attempts = work.allocation_attempts.saturating_add(3);
        work.copied_mode_items = work
            .copied_mode_items
            .saturating_add(source.num_modes() as u64);
        Ok(Self {
            source: source.clone(),
            horizon,
            config,
        })
    }

    pub fn source_model(&self) -> &CollectiveModel {
        &self.source
    }

    pub fn reconstruct(
        &self,
        initial_weighted_radius: f64,
        work: &mut DampedPacketWork,
    ) -> Result<VerifiedDampedPacketPoint, DampedPacketError> {
        verify_damped_packet_window(
            &self.source,
            self.horizon,
            initial_weighted_radius,
            self.config,
            work,
        )
    }
}

fn validate(
    model: &CollectiveModel,
    h: f64,
    radius: f64,
    config: DampedPacketConfig,
    work: &mut DampedPacketWork,
) -> Result<(), DampedPacketError> {
    work.validation_attempts = work.validation_attempts.saturating_add(1);
    if model.carrier() != 0.0 {
        return Err(DampedPacketError::UnsupportedCarrier);
    }
    if !h.is_finite() || h <= 0.0 || !radius.is_finite() || radius < 0.0 {
        return Err(DampedPacketError::InvalidInput("horizon or initial radius"));
    }
    let p = config.packet;
    if p.degree < p.order
        || p.sweeps == 0
        || config.max_subdivision_levels > 20
        || !p.mean_tolerance.is_finite()
        || p.mean_tolerance <= 0.0
        || !p.mode_tolerance.is_finite()
        || p.mode_tolerance <= 0.0
        || !config.moment_width_limit.is_finite()
        || config.moment_width_limit < 0.0
    {
        return Err(DampedPacketError::InvalidInput(
            "order, budgets or subdivision limit",
        ));
    }
    p.degree
        .checked_mul(3)
        .and_then(|d| d.checked_add(1))
        .and_then(|d| p.order.checked_add(1).and_then(|o| o.checked_mul(d)))
        .filter(|&n| n <= isize::MAX as usize / std::mem::size_of::<f64>())
        .ok_or(DampedPacketError::InvalidInput("polynomial size overflow"))?;
    Ok(())
}

fn copied<T: Copy>(items: &[T], work: &mut DampedPacketWork) -> Result<Vec<T>, DampedPacketError> {
    work.allocation_attempts = work.allocation_attempts.saturating_add(1);
    let mut out = Vec::new();
    out.try_reserve_exact(items.len())
        .map_err(|_| DampedPacketError::InvalidInput("output allocation refused"))?;
    out.extend_from_slice(items);
    work.copied_mode_items = work.copied_mode_items.saturating_add(items.len() as u64);
    Ok(out)
}

fn rebind(
    source: &CollectiveModel,
    centers: &[(f64, f64)],
    chi: f64,
    kappa: f64,
    work: &mut DampedPacketWork,
) -> Result<CollectiveModel, DampedPacketError> {
    CollectiveModel::new(
        copied(source.xi(), work)?,
        copied(source.weights(), work)?,
        copied(centers, work)?,
        source.width(),
        chi,
        0.0,
        kappa,
    )
    .map_err(|_| DampedPacketError::CenterAdmission)
}

fn exp_negative_product(
    rate: f64,
    h: f64,
    work: &mut DampedPacketWork,
) -> Result<Interval, DampedPacketError> {
    if rate == 0.0 {
        return Ok(scalar!(work, Interval::point(1.0)));
    }
    let r = scalar!(work, Interval::point(rate));
    let t = scalar!(work, Interval::point(h));
    let z = scalar!(work, r.mul(t));
    let low = scalar!(work, exp_interval(-z.hi));
    let high = scalar!(work, exp_interval(-z.lo.max(0.0)));
    Ok(scalar!(
        work,
        Interval::new(low.lo.max(0.0), high.hi.min(1.0))
    ))
}

/// The alternate Q00 formula is stable when 2*kappa*h>1. At small argument
/// retain the existing confluent Taylor kernel, avoiding cancellation.
fn averaged_coefficient(
    source: &CollectiveModel,
    h: f64,
    config: DampedPacketConfig,
    work: &mut DampedPacketWork,
) -> Result<(f64, f64, Interval), DampedPacketError> {
    let kappa = source.kappa();
    let damping = exp_negative_product(kappa, h, work)?;
    if kappa == 0.0 || source.chi() == 0.0 {
        return Ok((source.chi(), 0.0, damping));
    }
    let rate = 2.0 * kappa;
    if !rate.is_finite() {
        return Err(DampedPacketError::InvalidInput("twice damping overflows"));
    }
    let attempted =
        compute_damped_forcing_moment(rate, h, 0, 0, config.moment_width_limit, &mut work.moments);
    let moment = match attempted {
        Ok(value) => value.interval,
        Err(_) => {
            work.moment_fallback_attempts = work.moment_fallback_attempts.saturating_add(1);
            let a = scalar!(work, Interval::point(rate));
            let t = scalar!(work, Interval::point(h));
            let z = scalar!(work, a.mul(t));
            if z.lo <= 1.0 {
                return Err(DampedPacketError::InvalidInput(
                    "small-argument damped moment refused",
                ));
            }
            let exponential = exp_negative_product(rate, h, work)?;
            let one = scalar!(work, Interval::point(1.0));
            let numerator = scalar!(work, one.sub(exponential));
            let alternative = scalar!(work, numerator.div(a));
            let width = scalar!(work, sub_up(alternative.hi, alternative.lo));
            if width > config.moment_width_limit {
                return Err(DampedPacketError::MomentWidth {
                    width,
                    limit: config.moment_width_limit,
                });
            }
            alternative
        }
    };
    let t = scalar!(work, Interval::point(h));
    let average = scalar!(work, moment.div(t));
    // Positivity and damping yield these exact analytical intersections.
    let average = scalar!(
        work,
        Interval::new(average.lo.max(0.0), average.hi.min(1.0))
    );
    let chi = scalar!(work, Interval::point(source.chi()));
    let coefficient = scalar!(work, chi.mul(average));
    let center = coefficient.lo * 0.5 + coefficient.hi * 0.5;
    if !center.is_finite() {
        return Err(DampedPacketError::InvalidInput(
            "average coefficient center",
        ));
    }
    let center_i = scalar!(work, Interval::point(center));
    let difference = scalar!(work, coefficient.sub(center_i));
    Ok((center, difference.mag(), damping))
}

fn initial_bounds(
    source: &CollectiveModel,
    radius: f64,
    work: &mut DampedPacketWork,
) -> Result<(f64, f64), DampedPacketError> {
    let mut s = 0.0;
    let mut energy = 0.0;
    for (&w, &(re, im)) in source.weights().iter().zip(source.initial_amplitudes()) {
        s = scalar!(work, add_up(s, w));
        let re2 = scalar!(work, mul_up(re, re));
        let im2 = scalar!(work, mul_up(im, im));
        let a2 = scalar!(work, add_up(re2, im2));
        let term = scalar!(work, mul_up(w, a2));
        energy = scalar!(work, add_up(energy, term));
    }
    let center_r = scalar!(work, sqrt_up(energy));
    let r = scalar!(work, add_up(center_r, radius));
    Ok((s, r))
}

fn variation_bound(
    source: &CollectiveModel,
    h: f64,
    radius: f64,
    average_chi: f64,
    chi_radius: f64,
    work: &mut DampedPacketWork,
) -> Result<f64, DampedPacketError> {
    if source.kappa() == 0.0 || source.chi() == 0.0 {
        return Ok(0.0);
    }
    let (s, r) = initial_bounds(source, radius, work)?;
    let s2 = scalar!(work, mul_up(s, s));
    let r2 = scalar!(work, mul_up(r, r));
    let r3 = scalar!(work, mul_up(r2, r));
    let forcing_norm = scalar!(work, mul_up(s2, r3));
    // Equal frequencies commute with the cubic flow, so the exact average
    // reproduces its whole endpoint. Only represented-coefficient error remains.
    let equal_frequency = source.width() == 0.0 || source.xi().iter().all(|&x| x == source.xi()[0]);
    let variation = if equal_frequency {
        0.0
    } else {
        let exp2 = exp_negative_product(2.0 * source.kappa(), h, work)?;
        let range = scalar!(work, sub_up(1.0, exp2.lo));
        let half = scalar!(work, mul_up(0.5, range));
        scalar!(work, mul_up(source.chi().abs(), half))
    };
    let coefficient_error = scalar!(work, add_up(variation, chi_radius));
    let integrated = scalar!(work, mul_up(h, coefficient_error));
    let forcing_error = scalar!(work, mul_up(forcing_norm, integrated));
    if equal_frequency {
        return Ok(forcing_error);
    }
    let nonlinear = scalar!(work, mul_up(s2, r2));
    let gain_rate = scalar!(
        work,
        mul_up(nonlinear, source.chi().abs().max(average_chi.abs()))
    );
    let gain_argument = scalar!(work, mul_up(gain_rate, h));
    let gain = scalar!(work, exp_interval(gain_argument));
    Ok(scalar!(work, mul_up(gain.hi, forcing_error)))
}

fn one_subwindow(
    source: &CollectiveModel,
    h: f64,
    radius: f64,
    config: DampedPacketConfig,
    work: &mut DampedPacketWork,
) -> Result<VerifiedDampedPacketPoint, DampedPacketError> {
    work.subwindow_attempts = work.subwindow_attempts.saturating_add(1);
    let (average_chi, chi_radius, damping) = averaged_coefficient(source, h, config, work)?;
    let model_error = variation_bound(source, h, radius, average_chi, chi_radius, work)?;
    let transformed = rebind(source, source.initial_amplitudes(), average_chi, 0.0, work)?;
    let force_degree = config
        .packet
        .degree
        .checked_mul(3)
        .ok_or(DampedPacketError::InvalidInput("forcing degree"))?;
    let table = CausalWeights::compile(config.packet.order, force_degree);
    let packet = compile_packet(&transformed, h, config.packet.order, &mut work.packet)?;
    let candidate = generate_candidate(
        &packet,
        config.packet.degree,
        config.packet.sweeps,
        &table,
        &mut work.packet,
    )?;
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work.packet)?;
    let output = PacketCurveOutput::new(curve);
    let mut workspace = output.workspace(&mut work.packet)?;
    let view = output.enclose_at_weighted(h, radius, &mut workspace, &mut work.packet)?;
    let inner_radius = view
        .centered_weighted_radius()
        .ok_or(DampedPacketError::InvalidInput(
            "packet weighted-center radius missing",
        ))?;
    let mut modes = copied(view.modes(), work)?;
    let mut scale_square = 0.0;
    let scale_center = damping.lo * 0.5 + damping.hi * 0.5;
    if source.kappa() != 0.0 {
        for (j, z) in modes.iter_mut().enumerate() {
            let original = *z;
            *z = (scale_center * original.0, scale_center * original.1);
            if !z.0.is_finite() || !z.1.is_finite() {
                return Err(DampedPacketError::InvalidInput("scaled mode center"));
            }
            let re = scalar!(work, Interval::point(original.0));
            let im = scalar!(work, Interval::point(original.1));
            let exact_re = scalar!(work, damping.mul(re));
            let exact_im = scalar!(work, damping.mul(im));
            let center_re = scalar!(work, Interval::point(z.0));
            let center_im = scalar!(work, Interval::point(z.1));
            let err_re = scalar!(work, exact_re.sub(center_re)).mag();
            let err_im = scalar!(work, exact_im.sub(center_im)).mag();
            let re2 = scalar!(work, mul_up(err_re, err_re));
            let im2 = scalar!(work, mul_up(err_im, err_im));
            let square = scalar!(work, add_up(re2, im2));
            let weighted = scalar!(work, mul_up(source.weights()[j], square));
            scale_square = scalar!(work, add_up(scale_square, weighted));
        }
    }
    let propagated = scalar!(work, add_up(inner_radius, model_error));
    let scaled = scalar!(work, mul_up(damping.hi, propagated));
    let scale_radius = scalar!(work, sqrt_up(scale_square));
    let weighted_radius = scalar!(work, add_up(scaled, scale_radius));
    make_point(source, h, modes, weighted_radius, 1, work)
}

fn make_point(
    source: &CollectiveModel,
    time: f64,
    modes: Vec<(f64, f64)>,
    weighted_radius: f64,
    subwindows: usize,
    work: &mut DampedPacketWork,
) -> Result<VerifiedDampedPacketPoint, DampedPacketError> {
    work.allocation_attempts = work.allocation_attempts.saturating_add(1);
    let mut mode_radii = Vec::new();
    mode_radii
        .try_reserve_exact(modes.len())
        .map_err(|_| DampedPacketError::InvalidInput("radii allocation"))?;
    let mut s = 0.0;
    let mut mean = (0.0, 0.0);
    let mut mean_re = scalar!(work, Interval::point(0.0));
    let mut mean_im = scalar!(work, Interval::point(0.0));
    for (&weight, &z) in source.weights().iter().zip(&modes) {
        let lower_sqrt = scalar!(work, sqrt_down(weight));
        let mode_radius = scalar!(work, div_up(weighted_radius, lower_sqrt));
        mode_radii.push(mode_radius);
        s = scalar!(work, add_up(s, weight));
        mean.0 += weight * z.0;
        mean.1 += weight * z.1;
        let w = scalar!(work, Interval::point(weight));
        let re = scalar!(work, Interval::point(z.0));
        let im = scalar!(work, Interval::point(z.1));
        let term_re = scalar!(work, w.mul(re));
        let term_im = scalar!(work, w.mul(im));
        mean_re = scalar!(work, mean_re.add(term_re));
        mean_im = scalar!(work, mean_im.add(term_im));
    }
    let mr = scalar!(work, Interval::point(mean.0));
    let mi = scalar!(work, Interval::point(mean.1));
    let dr = scalar!(work, mean_re.sub(mr)).mag();
    let di = scalar!(work, mean_im.sub(mi)).mag();
    let dr2 = scalar!(work, mul_up(dr, dr));
    let di2 = scalar!(work, mul_up(di, di));
    let d2 = scalar!(work, add_up(dr2, di2));
    let displacement = scalar!(work, sqrt_up(d2));
    let root_s = scalar!(work, sqrt_up(s));
    let weighted_mean = scalar!(work, mul_up(root_s, weighted_radius));
    let mean_radius = scalar!(work, add_up(weighted_mean, displacement));
    Ok(VerifiedDampedPacketPoint {
        time,
        modes,
        mode_radii,
        mean,
        mean_radius,
        weighted_radius,
        subwindows,
    })
}

fn within_budget(point: &VerifiedDampedPacketPoint, config: DampedPacketConfig) -> bool {
    point.mean_radius <= config.packet.mean_tolerance
        && point
            .mode_radii
            .iter()
            .all(|&r| r <= config.packet.mode_tolerance)
}

/// Verified original-source endpoint, with fixed budgets and bounded retries.
/// Each failed trial is discarded; its work remains charged. Initial uncertainty
/// is carried as a weighted ball, without repeated ball/box inflation.
pub fn verify_damped_packet_window(
    model: &CollectiveModel,
    h: f64,
    initial_weighted_radius: f64,
    config: DampedPacketConfig,
    work: &mut DampedPacketWork,
) -> Result<VerifiedDampedPacketPoint, DampedPacketError> {
    validate(model, h, initial_weighted_radius, config, work)?;
    let mut last = DampedPacketError::InvalidInput("no attempted window");
    let mut delta = h;
    for level in 0..=config.max_subdivision_levels {
        work.window_attempts = work.window_attempts.saturating_add(1);
        let count = 1usize << level;
        let mut current = rebind(
            model,
            model.initial_amplitudes(),
            model.chi(),
            model.kappa(),
            work,
        )?;
        let mut radius = initial_weighted_radius;
        let mut endpoint = None;
        for index in 0..count {
            match one_subwindow(&current, delta, radius, config, work) {
                Ok(mut point) => {
                    radius = point.weighted_radius;
                    point.time = h;
                    point.subwindows = count;
                    if index + 1 < count {
                        current = rebind(model, &point.modes, model.chi(), model.kappa(), work)?;
                    }
                    endpoint = Some(point);
                }
                Err(error) => {
                    last = error;
                    endpoint = None;
                    break;
                }
            }
        }
        if let Some(point) = endpoint {
            if within_budget(&point, config) {
                return Ok(point);
            }
            last = DampedPacketError::BudgetExceeded {
                mean_radius: point.mean_radius,
                mode_radius: point.mode_radii.iter().copied().fold(0.0_f64, f64::max),
            };
        }
        work.rejected_windows = work.rejected_windows.saturating_add(1);
        if level < config.max_subdivision_levels {
            let half = delta * 0.5;
            if half <= 0.0 || half * 2.0 != delta {
                return Err(DampedPacketError::InexactDuration);
            }
            delta = half;
        }
    }
    Err(last)
}

/// Requested-mesh common-damping trajectory. Only a complete budget-satisfying
/// trajectory is returned; the output center/radius ownership is immutable.
pub fn integrate_verified_damped_packet(
    model: &CollectiveModel,
    initial_radii: &[f64],
    mesh: &[f64],
    config: DampedPacketConfig,
    work: &mut DampedPacketWork,
) -> Result<VerifiedDampedPacketTrajectory, DampedPacketError> {
    work.validation_attempts = work.validation_attempts.saturating_add(1);
    if mesh.len() < 2
        || mesh.iter().any(|t| !t.is_finite())
        || mesh.windows(2).any(|w| w[1] <= w[0])
        || initial_radii.len() != model.num_modes()
        || initial_radii.iter().any(|&r| !r.is_finite() || r < 0.0)
    {
        return Err(DampedPacketError::InvalidInput("mesh or initial radii"));
    }
    let mut square = 0.0;
    for (&w, &r) in model.weights().iter().zip(initial_radii) {
        let r2 = scalar!(work, mul_up(r, r));
        let term = scalar!(work, mul_up(w, r2));
        square = scalar!(work, add_up(square, term));
    }
    let mut radius = scalar!(work, sqrt_up(square));
    let mut current = rebind(
        model,
        model.initial_amplitudes(),
        model.chi(),
        model.kappa(),
        work,
    )?;
    work.allocation_attempts = work.allocation_attempts.saturating_add(1);
    let mut points = Vec::new();
    points
        .try_reserve_exact(mesh.len() - 1)
        .map_err(|_| DampedPacketError::InvalidInput("trajectory allocation"))?;
    for (index, pair) in mesh.windows(2).enumerate() {
        let low = scalar!(work, sub_down(pair[1], pair[0]));
        let high = scalar!(work, sub_up(pair[1], pair[0]));
        if low != high || low <= 0.0 {
            return Err(DampedPacketError::InexactDuration);
        }
        let mut point = verify_damped_packet_window(&current, low, radius, config, work)?;
        point.time = pair[1];
        radius = point.weighted_radius;
        if index + 2 < mesh.len() {
            current = rebind(model, &point.modes, model.chi(), model.kappa(), work)?;
        }
        points.push(point);
    }
    Ok(VerifiedDampedPacketTrajectory {
        source: model.clone(),
        initial_time: mesh[0],
        initial_radii: copied(initial_radii, work)?,
        points,
    })
}
