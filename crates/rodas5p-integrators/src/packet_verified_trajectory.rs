//! Requested-time, undamped collective flow with sealed weighted error transport.
//!
//! Each requested interval is one represented packet window. A returned object
//! contains all requested outputs and all tolerances passed. Refusal carries no
//! partially certified trajectory. This is an opt-in route, independent of the
//! historical diagnostic `PacketCertificate`/`integrate_packet_trajectory`.

use rodas5p_core::directed::{add_up, mul_up, sqrt_up, sub_down, sub_up};
use thiserror::Error;

use super::{PacketCurveOutput, RepresentedPacketCurve};
use crate::packet_window::PacketError;
use crate::{
    CausalWeights, CollectiveModel, PacketWorkCounters, compile_packet, generate_candidate,
};

#[derive(Clone, Copy, Debug)]
pub struct VerifiedPacketConfig {
    pub order: usize,
    pub degree: usize,
    pub sweeps: usize,
    pub mean_tolerance: f64,
    pub mode_tolerance: f64,
}

#[derive(Clone, Debug, PartialEq, Error)]
pub enum VerifiedPacketError {
    #[error(transparent)]
    Packet(#[from] PacketError),
    #[error("verified packet requires zero carrier and zero damping")]
    UnsupportedModel,
    #[error("verified packet mesh/configuration/initial ball is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("interval {index} has no exact binary64 duration; insert closer mesh points")]
    InexactDuration { index: usize },
    #[error("window {index} exceeds output budgets: mean {mean_radius}, mode {max_mode_radius}")]
    BudgetExceeded {
        index: usize,
        mean_radius: f64,
        max_mode_radius: f64,
    },
    #[error("window {index} center leaves the collective-model constructor domain")]
    CenterAdmission { index: usize },
}

/// Immutable output at one requested absolute time.
#[derive(Clone, Debug)]
pub struct VerifiedPacketPoint {
    time: f64,
    modes: Vec<(f64, f64)>,
    mode_radii: Vec<f64>,
    mean: (f64, f64),
    mean_radius: f64,
    weighted_radius: f64,
}

impl VerifiedPacketPoint {
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
    /// Radius in the exact source-weight norm about these returned centers.
    pub fn weighted_radius(&self) -> f64 {
        self.weighted_radius
    }
}

/// Complete successful requested-time result, bound to its immutable source.
#[derive(Clone, Debug)]
pub struct VerifiedPacketTrajectory {
    source: CollectiveModel,
    initial_time: f64,
    initial_radii: Vec<f64>,
    points: Vec<VerifiedPacketPoint>,
}

impl VerifiedPacketTrajectory {
    pub fn source_model(&self) -> &CollectiveModel {
        &self.source
    }
    pub fn initial_time(&self) -> f64 {
        self.initial_time
    }
    pub fn initial_radii(&self) -> &[f64] {
        &self.initial_radii
    }
    /// One point per mesh entry after the initial time.
    pub fn points(&self) -> &[VerifiedPacketPoint] {
        &self.points
    }
}

/// Solve the native co-rotating undamped equation at an increasing finite mesh.
///
/// Each difference must be exactly representable as binary64. This makes the
/// composition times exact; ordinary rounded subtraction is never silently
/// substituted. A suitably scaled uniform power-of-two mesh has exact
/// differences; every supplied mesh is checked explicitly.
/// Component input disks are converted once to a weighted ball. Subsequent
/// windows carry that ball directly, including evaluation-rounding displacement.
/// This avoids the factor sqrt(N) introduced by repeated ball-to-box conversion.
///
/// Setup is allocating. `PacketWorkCounters` records the underlying packet and
/// interval arithmetic; it is not a complete allocator or wall-time receipt.
/// The function neither relaxes tolerances nor silently retries a refused mesh.
pub fn integrate_verified_packet(
    model: &CollectiveModel,
    initial_radii: &[f64],
    mesh: &[f64],
    config: VerifiedPacketConfig,
    work: &mut PacketWorkCounters,
) -> Result<VerifiedPacketTrajectory, VerifiedPacketError> {
    let n = model.num_modes();
    if model.kappa() != 0.0 || model.carrier() != 0.0 {
        return Err(VerifiedPacketError::UnsupportedModel);
    }
    if mesh.len() < 2
        || mesh.iter().any(|t| !t.is_finite())
        || mesh.windows(2).any(|t| t[1] <= t[0])
    {
        return Err(VerifiedPacketError::InvalidInput("strict finite mesh"));
    }
    if initial_radii.len() != n
        || initial_radii.iter().any(|r| !r.is_finite() || *r < 0.0)
        || config.degree < config.order
        || config.sweeps == 0
        || !config.mean_tolerance.is_finite()
        || config.mean_tolerance <= 0.0
        || !config.mode_tolerance.is_finite()
        || config.mode_tolerance <= 0.0
    {
        return Err(VerifiedPacketError::InvalidInput(
            "shape, order, sweeps or budgets",
        ));
    }
    let force_degree = config
        .degree
        .checked_mul(3)
        .ok_or(PacketError::SizeOverflow("verified causal forcing degree"))?;
    // Guard the table's unchecked integer shape before calling its existing
    // constructor. Oversized requests refuse before any allocation.
    config
        .order
        .checked_add(1)
        .and_then(|r| force_degree.checked_add(1).and_then(|c| r.checked_mul(c)))
        .filter(|&items| items <= isize::MAX as usize / std::mem::size_of::<f64>())
        .ok_or(PacketError::SizeOverflow("verified causal table"))?;
    for (index, pair) in mesh.windows(2).enumerate() {
        exact_duration(pair[0], pair[1], index, work)?;
    }
    let mut square = 0.0;
    for (&w, &radius) in model.weights().iter().zip(initial_radii) {
        work.curve_output_certificate_ops += 1;
        let r2 = mul_up(radius, radius)
            .map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
        work.curve_output_certificate_ops += 1;
        let term =
            mul_up(w, r2).map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
        work.curve_output_certificate_ops += 1;
        square = add_up(square, term)
            .map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
    }
    work.curve_output_certificate_ops += 1;
    let mut radius =
        sqrt_up(square).map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
    let mut current = model.clone();
    let table = CausalWeights::compile(config.order, force_degree);
    let mut points = Vec::new();
    points
        .try_reserve_exact(mesh.len() - 1)
        .map_err(|_| PacketError::SizeOverflow("verified trajectory outputs"))?;
    for (index, pair) in mesh.windows(2).enumerate() {
        let h = exact_duration(pair[0], pair[1], index, work)?;
        let packet = compile_packet(&current, h, config.order, work)?;
        let candidate = generate_candidate(&packet, config.degree, config.sweeps, &table, work)?;
        let curve = RepresentedPacketCurve::freeze(&candidate, &packet, work)?;
        let output = PacketCurveOutput::new(curve);
        let mut workspace = output.workspace(work)?;
        let view = output.enclose_at_weighted(h, radius, &mut workspace, work)?;
        let max_mode_radius = view.mode_radii().iter().copied().fold(0.0_f64, f64::max);
        if view.mean_radius() > config.mean_tolerance || max_mode_radius > config.mode_tolerance {
            return Err(VerifiedPacketError::BudgetExceeded {
                index,
                mean_radius: view.mean_radius(),
                max_mode_radius,
            });
        }
        radius = view
            .centered_weighted_radius()
            .ok_or(PacketError::OutwardRefusal(
                "verified weighted carry missing",
            ))?;
        let point = VerifiedPacketPoint {
            time: pair[1],
            modes: copy_slice(view.modes())?,
            mode_radii: copy_slice(view.mode_radii())?,
            mean: view.mean(),
            mean_radius: view.mean_radius(),
            weighted_radius: radius,
        };
        if index + 2 < mesh.len() {
            current = CollectiveModel::new(
                copy_slice(model.xi())?,
                copy_slice(model.weights())?,
                copy_slice(point.modes())?,
                model.width(),
                model.chi(),
                0.0,
                0.0,
            )
            .map_err(|_| VerifiedPacketError::CenterAdmission { index })?;
        }
        points.push(point);
    }
    Ok(VerifiedPacketTrajectory {
        source: model.clone(),
        initial_time: mesh[0],
        initial_radii: copy_slice(initial_radii)?,
        points,
    })
}

fn exact_duration(
    a: f64,
    b: f64,
    index: usize,
    work: &mut PacketWorkCounters,
) -> Result<f64, VerifiedPacketError> {
    work.curve_time_normalization_ops += 1;
    let lo = sub_down(b, a).map_err(|_| VerifiedPacketError::InexactDuration { index })?;
    work.curve_time_normalization_ops += 1;
    let hi = sub_up(b, a).map_err(|_| VerifiedPacketError::InexactDuration { index })?;
    if lo != hi || lo <= 0.0 {
        return Err(VerifiedPacketError::InexactDuration { index });
    }
    Ok(lo)
}

fn copy_slice<T: Copy>(values: &[T]) -> Result<Vec<T>, PacketError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(values.len())
        .map_err(|_| PacketError::SizeOverflow("verified trajectory copy"))?;
    copy.extend_from_slice(values);
    Ok(copy)
}
