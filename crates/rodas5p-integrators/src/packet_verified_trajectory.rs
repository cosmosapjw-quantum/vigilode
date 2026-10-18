//! Requested-time, undamped collective flow with sealed weighted error transport.
//!
//! Each requested interval is one represented packet window. A returned object
//! contains all requested outputs and all tolerances passed. Refusal carries no
//! partially certified trajectory. This is an opt-in route, independent of the
//! historical diagnostic `PacketCertificate`/`integrate_packet_trajectory`.

use rodas5p_core::directed::{add_up, mul_up, sqrt_up, sub_down, sub_up};
use thiserror::Error;

use super::{PacketFrozenOutput, RepresentedPacketCurve};
use crate::packet_window::PacketError;
use crate::{
    CausalWeights, CollectiveModel, PacketWorkCounters, compile_packet, generate_candidate,
};

#[derive(Clone, Copy, Debug, PartialEq)]
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
    #[error(
        "window {index} needs weighted radius {weighted_radius}, above reserved headroom {limit}"
    )]
    HeadroomExceeded {
        index: usize,
        weighted_radius: f64,
        limit: f64,
    },
    #[error("adaptive packet attempt limit {attempts} reached")]
    AttemptLimit { attempts: usize },
    #[error("adaptive packet interval {index} exhausted {depth} bisections")]
    RefinementExhausted { index: usize, depth: usize },
    #[error("adaptive packet interval {index} has no representable exact subdivision")]
    UnrepresentableSubdivision { index: usize },
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
        let output = PacketFrozenOutput::from_weighted_radius(curve, radius, work)?;
        let mut workspace = output.workspace(work)?;
        let view = output.enclose_at(h, &mut workspace, work)?;
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

/// Transactional state of the same undamped original-model trajectory.
///
/// The physical budgets, source model and initial uncertainty are immutable.
/// An accepted window replaces the current center, time and weighted ball
/// together. A rejected window changes none of these three quantities.
pub struct VerifiedPacketState {
    source: CollectiveModel,
    current: CollectiveModel,
    initial_time: f64,
    initial_radii: Vec<f64>,
    time: f64,
    radius: f64,
    config: VerifiedPacketConfig,
    table: CausalWeights,
    accepted_windows: usize,
}

impl VerifiedPacketState {
    pub fn new(
        model: &CollectiveModel,
        initial_radii: &[f64],
        initial_time: f64,
        config: VerifiedPacketConfig,
        work: &mut PacketWorkCounters,
    ) -> Result<Self, VerifiedPacketError> {
        if model.kappa() != 0.0 || model.carrier() != 0.0 {
            return Err(VerifiedPacketError::UnsupportedModel);
        }
        if !initial_time.is_finite()
            || initial_radii.len() != model.num_modes()
            || initial_radii.iter().any(|r| !r.is_finite() || *r < 0.0)
            || config.degree < config.order
            || config.sweeps == 0
            || !config.mean_tolerance.is_finite()
            || config.mean_tolerance <= 0.0
            || !config.mode_tolerance.is_finite()
            || config.mode_tolerance <= 0.0
        {
            return Err(VerifiedPacketError::InvalidInput(
                "time, shape, order, sweeps or budgets",
            ));
        }
        let force_degree = config
            .degree
            .checked_mul(3)
            .ok_or(PacketError::SizeOverflow("verified causal forcing degree"))?;
        config
            .order
            .checked_add(1)
            .and_then(|r| force_degree.checked_add(1).and_then(|c| r.checked_mul(c)))
            .filter(|&items| items <= isize::MAX as usize / std::mem::size_of::<f64>())
            .ok_or(PacketError::SizeOverflow("verified causal table"))?;
        let mut square = 0.0;
        for (&w, &r) in model.weights().iter().zip(initial_radii) {
            work.curve_output_certificate_ops += 1;
            let r2 =
                mul_up(r, r).map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
            work.curve_output_certificate_ops += 1;
            let term =
                mul_up(w, r2).map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
            work.curve_output_certificate_ops += 1;
            square = add_up(square, term)
                .map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
        }
        work.curve_output_certificate_ops += 1;
        let radius =
            sqrt_up(square).map_err(|_| PacketError::OutwardRefusal("verified initial ball"))?;
        Ok(Self {
            source: model.clone(),
            current: model.clone(),
            initial_time,
            initial_radii: copy_slice(initial_radii)?,
            time: initial_time,
            radius,
            config,
            table: CausalWeights::compile(config.order, force_degree),
            accepted_windows: 0,
        })
    }

    pub fn source_model(&self) -> &CollectiveModel {
        &self.source
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn modes(&self) -> &[(f64, f64)] {
        self.current.initial_amplitudes()
    }
    pub fn weighted_radius(&self) -> f64 {
        self.radius
    }
    pub fn config(&self) -> VerifiedPacketConfig {
        self.config
    }
    pub fn accepted_windows(&self) -> usize {
        self.accepted_windows
    }
}

/// One actual candidate attempt. Work snapshots include failed arithmetic.
/// `outcome=Ok(r)` means the state was committed with centered weighted radius r.
/// The snapshots cover selected packet counters, not complete runtime or memory.
#[derive(Clone, Debug)]
pub struct VerifiedPacketAttempt {
    pub start_time: f64,
    pub end_time: f64,
    pub incoming_weighted_radius: f64,
    /// Initial cheap-bound refusal retained if Bernstein replay was attempted.
    pub pre_refinement_error: Option<VerifiedPacketError>,
    pub outcome: Result<f64, VerifiedPacketError>,
    pub work_before: PacketWorkCounters,
    pub work_after: PacketWorkCounters,
}

/// Caller-owned attempt history retained even when an adaptive solve refuses.
#[derive(Clone, Debug, Default)]
pub struct VerifiedPacketDiagnostics {
    attempts: Vec<VerifiedPacketAttempt>,
}

impl VerifiedPacketDiagnostics {
    pub fn attempts(&self) -> &[VerifiedPacketAttempt] {
        &self.attempts
    }
    pub fn accepted(&self) -> usize {
        self.attempts.iter().filter(|a| a.outcome.is_ok()).count()
    }
    pub fn rejected(&self) -> usize {
        self.attempts.iter().filter(|a| a.outcome.is_err()).count()
    }
}

/// Attempt one finite exact-duration window, retaining work and rejection reason.
///
/// On every error the state time, modes, weighted radius and immutable physical
/// budgets are unchanged. The attempt log reserves its record before any packet
/// work, so a logging-allocation refusal does not run an unrecorded candidate.
/// Constructor admission of the proposed next center is checked before commit.
pub fn advance_verified_packet(
    state: &mut VerifiedPacketState,
    end_time: f64,
    diagnostics: &mut VerifiedPacketDiagnostics,
    work: &mut PacketWorkCounters,
) -> Result<VerifiedPacketPoint, VerifiedPacketError> {
    advance_with_headroom(state, end_time, None, None, diagnostics, work)
}

fn advance_with_headroom(
    state: &mut VerifiedPacketState,
    end_time: f64,
    weighted_limit: Option<f64>,
    bernstein_max_depth: Option<usize>,
    diagnostics: &mut VerifiedPacketDiagnostics,
    work: &mut PacketWorkCounters,
) -> Result<VerifiedPacketPoint, VerifiedPacketError> {
    diagnostics
        .attempts
        .try_reserve(1)
        .map_err(|_| PacketError::SizeOverflow("verified attempt log"))?;
    let start_time = state.time;
    let incoming_weighted_radius = state.radius;
    let work_before = *work;
    let mut pre_refinement_error = None;
    let result = prepare_verified_advance(
        state,
        end_time,
        weighted_limit,
        bernstein_max_depth,
        &mut pre_refinement_error,
        work,
    );
    let outcome = match &result {
        Ok((point, _)) => Ok(point.weighted_radius),
        Err(error) => Err(error.clone()),
    };
    diagnostics.attempts.push(VerifiedPacketAttempt {
        start_time,
        end_time,
        incoming_weighted_radius,
        pre_refinement_error,
        outcome,
        work_before,
        work_after: *work,
    });
    let (point, next_model) = result?;
    // The only state mutation: all fallible numerical/admission/allocation work
    // above completed, and this same returned center owns the transported ball.
    state.current = next_model;
    state.time = end_time;
    state.radius = point.weighted_radius;
    state.accepted_windows += 1;
    Ok(point)
}

fn prepare_verified_advance(
    state: &VerifiedPacketState,
    end_time: f64,
    weighted_limit: Option<f64>,
    bernstein_max_depth: Option<usize>,
    pre_refinement_error: &mut Option<VerifiedPacketError>,
    work: &mut PacketWorkCounters,
) -> Result<(VerifiedPacketPoint, CollectiveModel), VerifiedPacketError> {
    if !end_time.is_finite() || end_time <= state.time {
        return Err(VerifiedPacketError::InvalidInput(
            "strict finite forward window",
        ));
    }
    let index = state.accepted_windows;
    let h = exact_duration(state.time, end_time, index, work)?;
    let packet = compile_packet(&state.current, h, state.config.order, work)?;
    let candidate = generate_candidate(
        &packet,
        state.config.degree,
        state.config.sweeps,
        &state.table,
        work,
    )?;
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, work)?;
    let mut output = PacketFrozenOutput::from_weighted_radius(curve, state.radius, work)?;
    let mut point = query_verified_point(&output, state, end_time, h, weighted_limit, work);
    if let Some(depth) = bernstein_max_depth
        && matches!(
            point,
            Err(VerifiedPacketError::BudgetExceeded { .. }
                | VerifiedPacketError::HeadroomExceeded { .. })
        )
        && output.residual_sup_upper() > 0.0
    {
        *pre_refinement_error = point.as_ref().err().cloned();
        // Half-rho is an efficiency target only. Even a refinement which misses
        // it supplies a valid monotone bound; all original gates run again.
        let target = 0.5 * output.residual_sup_upper();
        (output, _) = output.with_bernstein_residual_bound(target, depth, work)?;
        point = query_verified_point(&output, state, end_time, h, weighted_limit, work);
    }
    let point = point?;
    let next_model = CollectiveModel::new(
        copy_slice(state.source.xi())?,
        copy_slice(state.source.weights())?,
        copy_slice(point.modes())?,
        state.source.width(),
        state.source.chi(),
        0.0,
        0.0,
    )
    .map_err(|_| VerifiedPacketError::CenterAdmission { index })?;
    Ok((point, next_model))
}

fn query_verified_point(
    output: &PacketFrozenOutput,
    state: &VerifiedPacketState,
    end_time: f64,
    h: f64,
    weighted_limit: Option<f64>,
    work: &mut PacketWorkCounters,
) -> Result<VerifiedPacketPoint, VerifiedPacketError> {
    let index = state.accepted_windows;
    let mut workspace = output.workspace(work)?;
    let view = output.enclose_at(h, &mut workspace, work)?;
    let max_mode_radius = view.mode_radii().iter().copied().fold(0.0_f64, f64::max);
    if view.mean_radius() > state.config.mean_tolerance
        || max_mode_radius > state.config.mode_tolerance
    {
        return Err(VerifiedPacketError::BudgetExceeded {
            index,
            mean_radius: view.mean_radius(),
            max_mode_radius,
        });
    }
    let radius = view
        .centered_weighted_radius()
        .ok_or(PacketError::OutwardRefusal(
            "verified weighted carry missing",
        ))?;
    if let Some(limit) = weighted_limit
        && radius > limit
    {
        return Err(VerifiedPacketError::HeadroomExceeded {
            index,
            weighted_radius: radius,
            limit,
        });
    }
    Ok(VerifiedPacketPoint {
        time: end_time,
        modes: copy_slice(view.modes())?,
        mode_radii: copy_slice(view.mode_radii())?,
        mean: view.mean(),
        mean_radius: view.mean_radius(),
        weighted_radius: radius,
    })
}

/// Resource bounds for dyadic refinement. No field changes a physical budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedPacketAdaptiveConfig {
    pub max_bisections: usize,
    pub max_attempts: usize,
    /// Optional A10 refinement of a refused candidate before time subdivision.
    /// None skips Bernstein work. Some(depth) permits one replay per candidate.
    pub bernstein_max_depth: Option<usize>,
}

/// Complete the original requested mesh using transactionally accepted subwindows.
///
/// Only budget/domain/arithmetic numerical refusals trigger bisection; malformed
/// input and identity/shape errors return immediately. Every accepted subwindow
/// meets the original physical mean/mode budgets. Requested output times are
/// preserved bit-for-bit, and the returned object has one point per mesh entry
/// after the initial time. Internal accepted endpoints remain in `diagnostics`.
/// Every subdivision checks exact binary64 durations. A stricter elapsed-time
/// weighted-radius schedule reserves capacity for later windows; the original
/// outward physical gates remain mandatory. Exhausting resources or floating
/// time resolution returns an error with the complete attempt log.
///
/// Successful completion is not promised for arbitrary precision requests:
/// inherited uncertainty, rounding floors, or the constructor's energy domain
/// can require a typed refusal. It never scales or relaxes a physical tolerance.
pub fn integrate_verified_packet_adaptive(
    model: &CollectiveModel,
    initial_radii: &[f64],
    mesh: &[f64],
    config: VerifiedPacketConfig,
    adaptive: VerifiedPacketAdaptiveConfig,
    diagnostics: &mut VerifiedPacketDiagnostics,
    work: &mut PacketWorkCounters,
) -> Result<VerifiedPacketTrajectory, VerifiedPacketError> {
    if mesh.len() < 2
        || mesh.iter().any(|t| !t.is_finite())
        || mesh.windows(2).any(|pair| pair[1] <= pair[0])
        || adaptive.max_attempts == 0
    {
        return Err(VerifiedPacketError::InvalidInput(
            "strict finite mesh and positive attempt limit",
        ));
    }
    for (index, pair) in mesh.windows(2).enumerate() {
        exact_duration(pair[0], pair[1], index, work)?;
    }
    let stack_capacity = adaptive
        .max_bisections
        .checked_add(1)
        .ok_or(PacketError::SizeOverflow("adaptive packet stack"))?;
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(stack_capacity)
        .map_err(|_| PacketError::SizeOverflow("adaptive packet stack"))?;
    let mut points = Vec::new();
    points
        .try_reserve_exact(mesh.len() - 1)
        .map_err(|_| PacketError::SizeOverflow("verified trajectory outputs"))?;
    let first_attempt = diagnostics.attempts.len();
    let mut state = VerifiedPacketState::new(model, initial_radii, mesh[0], config, work)?;
    let initial_radius = state.radius;
    // This controller target is deliberately stricter than the final budgets.
    // Ordinary arithmetic here selects steps; the outward physical gates above
    // remain the only certificate authority.
    let min_weight = model
        .weights()
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let weight_sum = model.weights().iter().sum::<f64>();
    let final_headroom = 0.5
        * (config.mode_tolerance * min_weight.sqrt())
            .min(config.mean_tolerance / weight_sum.sqrt());
    for (index, &requested_time) in mesh.iter().skip(1).enumerate() {
        stack.push((requested_time, 0_usize));
        while let Some((end_time, depth)) = stack.pop() {
            let attempts = diagnostics.attempts.len() - first_attempt;
            if attempts >= adaptive.max_attempts {
                return Err(VerifiedPacketError::AttemptLimit { attempts });
            }
            let limit = scheduled_headroom(
                mesh[0],
                *mesh.last().expect("validated mesh"),
                end_time,
                initial_radius,
                final_headroom,
            );
            match advance_with_headroom(
                &mut state,
                end_time,
                limit,
                adaptive.bernstein_max_depth,
                diagnostics,
                work,
            ) {
                Ok(point) => {
                    if end_time == requested_time {
                        points.push(point);
                    }
                }
                Err(error) => {
                    if !is_refinable(&error) {
                        return Err(error);
                    }
                    if depth >= adaptive.max_bisections {
                        return Err(VerifiedPacketError::RefinementExhausted { index, depth });
                    }
                    let h = exact_duration(state.time, end_time, index, work)?;
                    let mid = state.time + 0.5 * h;
                    if !mid.is_finite() || mid <= state.time || mid >= end_time {
                        return Err(VerifiedPacketError::UnrepresentableSubdivision { index });
                    }
                    // Rounded midpoint construction is harmless: both actual
                    // represented subinterval durations are checked separately.
                    exact_duration(state.time, mid, index, work)?;
                    exact_duration(mid, end_time, index, work)?;
                    stack.push((end_time, depth + 1));
                    stack.push((mid, depth + 1));
                }
            }
        }
    }
    Ok(VerifiedPacketTrajectory {
        source: state.source,
        initial_time: state.initial_time,
        initial_radii: state.initial_radii,
        points,
    })
}

fn is_refinable(error: &VerifiedPacketError) -> bool {
    matches!(
        error,
        VerifiedPacketError::BudgetExceeded { .. }
            | VerifiedPacketError::HeadroomExceeded { .. }
            | VerifiedPacketError::CenterAdmission { .. }
            | VerifiedPacketError::Packet(
                PacketError::DomainExceeded { .. }
                    | PacketError::GronwallConditionViolated { .. }
                    | PacketError::NonFinite(_)
                    | PacketError::OutwardRefusal(_)
                    | PacketError::BernsteinBudgetExhausted { .. }
            )
    )
}

// Reserve error capacity for later windows. This is a step-selection heuristic,
// not a mathematical certificate. A candidate always faces the original hard
// physical gates as well. If no positive reserve exists, use those gates alone.
fn scheduled_headroom(
    start: f64,
    finish: f64,
    time: f64,
    initial_radius: f64,
    final_radius: f64,
) -> Option<f64> {
    if !final_radius.is_finite() || final_radius <= initial_radius {
        return None;
    }
    let span = finish - start;
    let fraction = if span.is_finite() {
        (time - start) / span
    } else {
        (0.5 * time - 0.5 * start) / (0.5 * finish - 0.5 * start)
    };
    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
        return None;
    }
    Some(initial_radius + (final_radius - initial_radius) * fraction)
}
