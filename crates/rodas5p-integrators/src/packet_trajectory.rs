//! Uncertainty propagation and adaptive window transaction for collective packet trajectories.
//!
//! Research node LP04 of `research/promotion_loop12_20261011/PORTING_DAG.json`.
//! Implements multi-window causal packet advancement with:
//! 1. Inherited uncertainty transfer across windows:
//!    \[
//!    \rho_{\text{mean}}^{(k)} = \sum_j w_j \rho_j^{(k)}, \quad E_{\text{mean}}^{(k+1)} = \frac{D + \tau(A_0 + h G / (p+2)) + \rho_{\text{mean}}^{(k)}}{1 - h L}
//!    \]
//!    \[
//!    E_{\text{mode}, j}^{(k+1)} = B(|z_j| + h G / (p+2)) + h L_F E_{\text{mean}}^{(k+1)} + \rho_j^{(k)}
//!    \]
//! 2. Strict transactional atomicity: rejected trials reduce step size but cannot mutate accepted
//!    time, state, or error bounds; all work counters accumulate monotonically across attempts.
//! 3. Output policy separating mean-only and full-mode trajectory records.
//!
//! Radii are analytic estimates evaluated in ordinary binary64. The returned
//! `roundoff_enclosed = false` keeps production certification explicitly open.

use serde::{Deserialize, Serialize};

use crate::collective_model::{CollectiveAdmissionError, CollectiveAdmissionWork, CollectiveModel};
use crate::packet_window::{
    CausalWeights, OutwardPacketRequest, PacketError, compile_packet, compute_certificate,
    generate_candidate, reconstruct_modes, reconstruct_outward_at_times,
};
use crate::packet_work::PacketWorkCounters;
use rodas5p_core::directed::{Interval, add_up, mul_up, sqrt_up};
use rodas5p_core::{CoreError, CoreResult};
use thiserror::Error;

/// Output reporting policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputPolicy {
    /// Records time, mean, and diagnostic mean radius.
    MeanOnly,
    /// Records time, mean, mean radius, plus all microscopic modes and mode radii.
    AllModes,
}

/// A recorded step in the packet trajectory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub t: f64,
    pub h: f64,
    pub mean: (f64, f64),
    pub mean_radius: f64,
    pub modes: Option<Vec<(f64, f64)>>,
    pub mode_radii: Option<Vec<f64>>,
    pub local_defect: f64,
    pub total_mean_bound: f64,
}

/// Configuration for adaptive multi-window packet integration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryConfig {
    pub t_start: f64,
    pub t_end: f64,
    pub initial_h: f64,
    pub min_h: f64,
    pub max_h: f64,
    pub order: usize,
    pub degree: usize,
    pub sweeps: usize,
    pub mean_tolerance: f64,
    pub mode_tolerance: f64,
    pub output_policy: OutputPolicy,
    pub max_steps: usize,
}

impl TrajectoryConfig {
    pub fn new(
        t_start: f64,
        t_end: f64,
        initial_h: f64,
        mean_tolerance: f64,
        mode_tolerance: f64,
    ) -> CoreResult<Self> {
        if !t_start.is_finite() || !t_end.is_finite() || t_end <= t_start {
            return Err(CoreError::InvalidInput(format!(
                "packet_trajectory: t_end ({t_end}) must be strictly greater than t_start ({t_start})"
            )));
        }
        if !initial_h.is_finite() || initial_h <= 0.0 {
            return Err(CoreError::InvalidInput(format!(
                "packet_trajectory: initial_h must be positive finite, got {initial_h}"
            )));
        }
        if !mean_tolerance.is_finite() || mean_tolerance <= 0.0 {
            return Err(CoreError::InvalidInput(format!(
                "packet_trajectory: mean_tolerance must be positive finite, got {mean_tolerance}"
            )));
        }
        if !mode_tolerance.is_finite() || mode_tolerance <= 0.0 {
            return Err(CoreError::InvalidInput(format!(
                "packet_trajectory: mode_tolerance must be positive finite, got {mode_tolerance}"
            )));
        }

        Ok(Self {
            t_start,
            t_end,
            initial_h,
            min_h: 1e-6,
            max_h: initial_h * 2.0,
            order: 8,
            degree: 10,
            sweeps: 4,
            mean_tolerance,
            mode_tolerance,
            output_policy: OutputPolicy::MeanOnly,
            max_steps: 1000,
        })
    }
}

/// Outcome of a full trajectory integration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PacketTrajectoryResult {
    /// False: the current packet arithmetic does not enclose floating-point error.
    pub roundoff_enclosed: bool,
    pub final_t: f64,
    pub final_mean: (f64, f64),
    pub final_mean_radius: f64,
    pub final_modes: Vec<(f64, f64)>,
    pub final_mode_radii: Vec<f64>,
    pub steps: Vec<TrajectoryStep>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub work: PacketWorkCounters,
}

/// A provisional point on the linear outward trajectory. Its radii are
/// computed around the returned centers and transport initial uncertainty,
/// but have no certified authority while `roundoff_enclosed` is false.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearOutwardPoint {
    pub t: f64,
    pub mean: (f64, f64),
    pub mean_radius: f64,
    pub modes: Option<Vec<(f64, f64)>>,
    pub mode_radii: Option<Vec<f64>>,
}

/// Scoped result for the linear member of the packet family. Authority stays
/// false until the local outward primitive and its composition are reviewed.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearOutwardTrajectory {
    pub roundoff_enclosed: bool,
    pub observations: Vec<LinearOutwardPoint>,
    pub endpoints: Vec<LinearOutwardPoint>,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub work: PacketWorkCounters,
}

/// Typed refusal for unsupported models, time grids, and adaptive exhaustion.
#[derive(Clone, Debug, PartialEq, Error)]
pub enum LinearTrajectoryRefusal {
    #[error("linear outward trajectory requires chi = kappa = 0")]
    UnsupportedModel,
    #[error("invalid linear outward trajectory request: {0}")]
    InvalidInput(&'static str),
    #[error("requested times do not admit exact binary64 local offsets")]
    NonExactTimeGrid,
    #[error("linear outward packet refused: {0}")]
    Packet(#[from] PacketError),
    #[error("adaptive linear outward trajectory exhausted its step limit")]
    StepLimit,
    #[error("adaptive linear outward trajectory reached its minimum step")]
    MinimumStep,
}

fn exact_offset(t: f64, origin: f64) -> Result<f64, LinearTrajectoryRefusal> {
    let offset = t - origin;
    let exact = Interval::point(t)
        .and_then(|a| a.sub(Interval::point(origin)?))
        .map_err(|_| LinearTrajectoryRefusal::NonExactTimeGrid)?;
    if !offset.is_finite() || exact.lo != offset || exact.hi != offset {
        return Err(LinearTrajectoryRefusal::NonExactTimeGrid);
    }
    Ok(offset)
}

fn outward_point(
    output: &crate::packet_window::OutwardPacketOutput,
    index: usize,
    t: f64,
    n: usize,
    policy: OutputPolicy,
) -> Result<LinearOutwardPoint, LinearTrajectoryRefusal> {
    let start = index
        .checked_mul(n)
        .ok_or(LinearTrajectoryRefusal::InvalidInput(
            "output index overflow",
        ))?;
    let end = start
        .checked_add(n)
        .ok_or(LinearTrajectoryRefusal::InvalidInput(
            "output index overflow",
        ))?;
    let modes = output
        .modes
        .get(start..end)
        .ok_or(LinearTrajectoryRefusal::InvalidInput(
            "outward output shape",
        ))?;
    let radii = output
        .mode_radii
        .get(start..end)
        .ok_or(LinearTrajectoryRefusal::InvalidInput(
            "outward radius shape",
        ))?;
    let mean = *output
        .means
        .get(index)
        .ok_or(LinearTrajectoryRefusal::InvalidInput("outward mean shape"))?;
    let mean_radius =
        *output
            .mean_radii
            .get(index)
            .ok_or(LinearTrajectoryRefusal::InvalidInput(
                "outward mean radius shape",
            ))?;
    Ok(LinearOutwardPoint {
        t,
        mean,
        mean_radius,
        modes: (policy == OutputPolicy::AllModes).then(|| modes.to_vec()),
        mode_radii: (policy == OutputPolicy::AllModes).then(|| radii.to_vec()),
    })
}

/// Composes A01's local pointwise outward enclosures for the exactly linear
/// family. The exact time-offset gate avoids silently relabeling a local
/// enclosure at a rounded time as one at the requested global time. A rejected
/// trial commits no state or output; its packet work remains in `work`.
pub fn run_linear_outward_trajectory(
    model: &CollectiveModel,
    initial_radii: Option<&[f64]>,
    config: &TrajectoryConfig,
    requested_times: &[f64],
    work: &mut PacketWorkCounters,
) -> Result<LinearOutwardTrajectory, LinearTrajectoryRefusal> {
    if model.chi() != 0.0 || model.kappa() != 0.0 {
        return Err(LinearTrajectoryRefusal::UnsupportedModel);
    }
    TrajectoryConfig::new(
        config.t_start,
        config.t_end,
        config.initial_h,
        config.mean_tolerance,
        config.mode_tolerance,
    )
    .map_err(|_| LinearTrajectoryRefusal::InvalidInput("nonfinite time, step or tolerance"))?;
    if !config.min_h.is_finite()
        || config.min_h <= 0.0
        || !config.max_h.is_finite()
        || config.max_h < config.min_h
        || config.degree < config.order
        || config.sweeps == 0
        || config.max_steps == 0
        || config.degree.checked_mul(3).is_none()
        || config.order.checked_add(1).is_none()
        || config.degree.checked_add(1).is_none()
        || config
            .degree
            .checked_mul(3)
            .and_then(|n| n.checked_add(1))
            .is_none()
    {
        return Err(LinearTrajectoryRefusal::InvalidInput(
            "invalid packet configuration",
        ));
    }
    if requested_times
        .iter()
        .any(|t| !t.is_finite() || *t < config.t_start || *t > config.t_end)
        || requested_times.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(LinearTrajectoryRefusal::InvalidInput(
            "requested times must be finite, ordered and in range",
        ));
    }
    let n = model.num_modes();
    let mut radii = match initial_radii {
        Some(input) if input.len() == n && input.iter().all(|r| r.is_finite() && *r >= 0.0) => {
            input.to_vec()
        }
        Some(_) => {
            return Err(LinearTrajectoryRefusal::InvalidInput(
                "invalid initial radii",
            ));
        }
        None => vec![0.0; n],
    };
    let table = CausalWeights::compile(config.order, 3 * config.degree);
    let mut current = model.clone();
    let mut t = config.t_start;
    let mut h = config.initial_h.min(config.max_h);
    let mut observations = Vec::with_capacity(requested_times.len() + 2);
    let mut endpoints = Vec::new();
    let mut next_request = 0;
    let mut accepted_steps = 0;
    let mut rejected_steps = 0;
    let mut last_packet_error = None;
    while t < config.t_end {
        if accepted_steps + rejected_steps >= config.max_steps {
            return Err(LinearTrajectoryRefusal::StepLimit);
        }
        let remaining = exact_offset(config.t_end, t)?;
        let step_h = h.min(remaining);
        if step_h < config.min_h || t + step_h <= t {
            return Err(last_packet_error
                .map(LinearTrajectoryRefusal::Packet)
                .unwrap_or(LinearTrajectoryRefusal::MinimumStep));
        }
        let next_t = t + step_h;
        let exact_next = Interval::exact_sum(t, step_h)
            .map_err(|_| LinearTrajectoryRefusal::NonExactTimeGrid)?;
        if exact_next.lo != next_t || exact_next.hi != next_t {
            return Err(LinearTrajectoryRefusal::NonExactTimeGrid);
        }
        let mut local_times = Vec::new();
        let first = endpoints.is_empty();
        if first {
            local_times.push(0.0);
        }
        let mut count_request = 0;
        while next_request + count_request < requested_times.len()
            && requested_times[next_request + count_request] < next_t
        {
            let requested = requested_times[next_request + count_request];
            if requested > t {
                local_times.push(exact_offset(requested, t)?);
            }
            count_request += 1;
        }
        local_times.push(step_h);
        let attempt = (|| -> Result<_, PacketError> {
            let packet = compile_packet(&current, step_h, config.order, work)?;
            let candidate =
                generate_candidate(&packet, config.degree, config.sweeps, &table, work)?;
            reconstruct_outward_at_times(
                &candidate,
                &packet,
                OutwardPacketRequest {
                    times: &local_times,
                    initial_radii: &radii,
                },
                work,
            )
        })();
        let output = match attempt {
            Ok(output) => output,
            Err(error) => {
                last_packet_error = Some(error);
                rejected_steps += 1;
                h = step_h * 0.5;
                continue;
            }
        };
        last_packet_error = None;
        let last = local_times.len() - 1;
        let start = last * n;
        let endpoint_radii = output.mode_radii.get(start..start + n).ok_or(
            LinearTrajectoryRefusal::InvalidInput("outward endpoint shape"),
        )?;
        let endpoint_mean_radius =
            output
                .mean_radii
                .get(last)
                .ok_or(LinearTrajectoryRefusal::InvalidInput(
                    "outward endpoint mean shape",
                ))?;
        if *endpoint_mean_radius > config.mean_tolerance
            || endpoint_radii.iter().any(|r| *r > config.mode_tolerance)
        {
            rejected_steps += 1;
            h = step_h * 0.5;
            continue;
        }
        let endpoint_modes =
            output
                .modes
                .get(start..start + n)
                .ok_or(LinearTrajectoryRefusal::InvalidInput(
                    "outward endpoint shape",
                ))?;
        let next_model = match CollectiveModel::new(
            current.xi().to_vec(),
            current.weights().to_vec(),
            endpoint_modes.to_vec(),
            current.width(),
            current.chi(),
            current.carrier(),
            current.kappa(),
        ) {
            Ok(next) => next,
            Err(_) => {
                rejected_steps += 1;
                h = step_h * 0.5;
                continue;
            }
        };
        let mut index = 0;
        if first {
            let initial = outward_point(&output, 0, t, n, config.output_policy)?;
            observations.push(initial.clone());
            endpoints.push(initial);
            index += 1;
        }
        for requested in &requested_times[next_request..next_request + count_request] {
            if *requested > t {
                observations.push(outward_point(
                    &output,
                    index,
                    *requested,
                    n,
                    config.output_policy,
                )?);
                index += 1;
            }
        }
        let endpoint = outward_point(&output, last, next_t, n, config.output_policy)?;
        if requested_times.get(next_request + count_request) == Some(&next_t) {
            observations.push(endpoint.clone());
            count_request += 1;
        }
        endpoints.push(endpoint.clone());
        if next_t == config.t_end && observations.last().map(|p| p.t) != Some(next_t) {
            observations.push(endpoint);
        }
        next_request += count_request;
        t = next_t;
        current = next_model;
        radii.copy_from_slice(endpoint_radii);
        accepted_steps += 1;
        h = step_h;
    }
    Ok(LinearOutwardTrajectory {
        roundoff_enclosed: false,
        observations,
        endpoints,
        accepted_steps,
        rejected_steps,
        work: *work,
    })
}

fn complex_mean(weights: &[f64], z: &[(f64, f64)]) -> (f64, f64) {
    let mut re = 0.0;
    let mut im = 0.0;
    for (&w, &(u, v)) in weights.iter().zip(z.iter()) {
        re += w * u;
        im += w * v;
    }
    (re, im)
}

/// Executes a transactional, adaptive multi-window packet trajectory.
pub fn run_packet_trajectory(
    model: &CollectiveModel,
    initial_radii: Option<&[f64]>,
    config: &TrajectoryConfig,
    work: &mut PacketWorkCounters,
) -> CoreResult<PacketTrajectoryResult> {
    // Config is public and deserializable, so constructor checks alone do not
    // establish the contract at this execution boundary.
    TrajectoryConfig::new(
        config.t_start,
        config.t_end,
        config.initial_h,
        config.mean_tolerance,
        config.mode_tolerance,
    )?;
    if !config.min_h.is_finite()
        || config.min_h <= 0.0
        || !config.max_h.is_finite()
        || config.max_h < config.min_h
        || config.degree < config.order
        || config.sweeps == 0
        || config.max_steps == 0
        || config.degree.checked_mul(3).is_none()
    {
        return Err(CoreError::InvalidInput(
            "packet_trajectory: invalid step bounds, degree, sweeps or step limit".into(),
        ));
    }
    if model.kappa() != 0.0 {
        return Err(CoreError::InvalidInput(
            "packet_trajectory: kappa > 0 requires an implemented damped packet route".into(),
        ));
    }
    let num_modes = model.num_modes();

    // Validate initial radii if provided
    let mut current_radii = if let Some(radii) = initial_radii {
        if radii.len() != num_modes {
            return Err(CoreError::InvalidInput(format!(
                "packet_trajectory: initial_radii length {} != model modes {num_modes}",
                radii.len()
            )));
        }
        for (idx, &r) in radii.iter().enumerate() {
            if !r.is_finite() || r < 0.0 {
                return Err(CoreError::InvalidInput(format!(
                    "packet_trajectory: initial_radii[{idx}] must be non-negative finite, got {r}"
                )));
            }
        }
        radii.to_vec()
    } else {
        vec![0.0; num_modes]
    };

    // Energy conservation and |mean|^2 <= (sum weights) * energy prove the
    // true mean stays in the unit disk for every permitted initial state.
    // Enclose the energy of the product of complex uncertainty disks outward.
    let mut initial_energy_upper = 0.0;
    let mut weight_sum_upper = 0.0;
    for ((&w, &(re, im)), &radius) in model
        .weights()
        .iter()
        .zip(model.initial_amplitudes())
        .zip(&current_radii)
    {
        weight_sum_upper = add_up(weight_sum_upper, w)?;
        let magnitude = sqrt_up(add_up(mul_up(re, re)?, mul_up(im, im)?)?)?;
        let outer_radius = add_up(magnitude, radius)?;
        initial_energy_upper = add_up(
            initial_energy_upper,
            mul_up(w, mul_up(outer_radius, outer_radius)?)?,
        )?;
    }
    let initial_mean_squared_upper = mul_up(weight_sum_upper, initial_energy_upper)?;
    if initial_mean_squared_upper > 1.0 {
        return Err(CoreError::InvalidInput(format!(
            "packet_trajectory: initial uncertainty mean-squared upper bound {initial_mean_squared_upper} exceeds proved unit domain"
        )));
    }

    let weights_table = CausalWeights::compile(config.order, 3 * config.degree);

    let mut t = config.t_start;
    let mut current_model = model.clone();
    let mut h = config.initial_h.min(config.max_h).min(config.t_end - t);

    let mut steps = Vec::new();
    let mut accepted_steps = 0;
    let mut rejected_steps = 0;

    // Record initial step t_start
    let initial_mean = complex_mean(current_model.weights(), current_model.initial_amplitudes());
    let mut initial_mean_radius = 0.0;
    for (&w, &r) in current_model.weights().iter().zip(current_radii.iter()) {
        initial_mean_radius += w * r;
    }

    steps.push(TrajectoryStep {
        t,
        h: 0.0,
        mean: initial_mean,
        mean_radius: initial_mean_radius,
        modes: if config.output_policy == OutputPolicy::AllModes {
            Some(current_model.initial_amplitudes().to_vec())
        } else {
            None
        },
        mode_radii: if config.output_policy == OutputPolicy::AllModes {
            Some(current_radii.clone())
        } else {
            None
        },
        local_defect: 0.0,
        total_mean_bound: initial_mean_radius,
    });

    while t < config.t_end {
        if accepted_steps + rejected_steps >= config.max_steps {
            return Err(CoreError::NonlinearSolve(format!(
                "packet_trajectory: exceeded max_steps limit {}",
                config.max_steps
            )));
        }

        // Clamp step to not overshoot t_end
        let step_h = h.min(config.t_end - t);
        if t + step_h <= t || !(t + step_h).is_finite() {
            return Err(CoreError::InvalidInput(
                "packet_trajectory: step cannot advance representable time".into(),
            ));
        }
        if step_h < config.min_h {
            return Err(CoreError::NonlinearSolve(format!(
                "packet_trajectory: step size {step_h:e} fell below min_h {:e} at t={t}",
                config.min_h
            )));
        }

        // --- Transactional Attempt ---
        // Compute inherited mean radius
        let mut inherited_mean_radius = 0.0;
        for (&w, &r) in current_model.weights().iter().zip(current_radii.iter()) {
            inherited_mean_radius += w * r;
        }

        let packet_res = compile_packet(&current_model, step_h, config.order, work);
        let packet = match packet_res {
            Ok(p) => p,
            Err(_) => {
                // Rejection: shrink step and retry
                rejected_steps += 1;
                h = (step_h * 0.5).max(config.min_h * 0.5);
                continue;
            }
        };

        let cand_res =
            generate_candidate(&packet, config.degree, config.sweeps, &weights_table, work);
        let cand = match cand_res {
            Ok(c) => c,
            Err(_) => {
                rejected_steps += 1;
                h = (step_h * 0.5).max(config.min_h * 0.5);
                continue;
            }
        };

        let cert_res = compute_certificate(&cand, &packet, &weights_table, work);
        let cert = match cert_res {
            Ok(c) => c,
            Err(_) => {
                rejected_steps += 1;
                h = (step_h * 0.5).max(config.min_h * 0.5);
                continue;
            }
        };

        // Transfer theorem: total mean bound incorporating inherited uncertainty
        let denom = 1.0 - step_h * cert.lipschitz_l;
        if denom <= 0.0 {
            rejected_steps += 1;
            h = (step_h * 0.5).max(config.min_h * 0.5);
            continue;
        }

        // kernel_model already equals tau * (A0 + h*G/(p+2)).
        // Add it once; it is not the force magnitude G.
        let num = cert.volterra_defect + cert.kernel_model + inherited_mean_radius;
        let total_mean_bound = num / denom;

        // Transfer theorem: mode bounds
        let mut new_mode_radii = Vec::with_capacity(num_modes);
        let mut max_mode_bound: f64 = 0.0;

        for (&local_bound, &prev_r) in cert.mode_bounds.iter().zip(&current_radii) {
            let mode_b = local_bound
                + step_h * cert.microscopic_lipschitz_l * inherited_mean_radius / denom
                + prev_r;
            max_mode_bound = max_mode_bound.max(mode_b);
            new_mode_radii.push(mode_b);
        }

        // Budget check
        let accepted =
            total_mean_bound <= config.mean_tolerance && max_mode_bound <= config.mode_tolerance;

        if accepted {
            // Reconstruct new microscopic mode amplitudes
            let new_modes = match reconstruct_modes(&cand, &packet, work) {
                Ok(modes) => modes,
                Err(_) => {
                    rejected_steps += 1;
                    h = (step_h * 0.5).max(config.min_h * 0.5);
                    continue;
                }
            };

            // Construct new admitted model for the subsequent window
            let next_model_res = CollectiveModel::new(
                current_model.xi().to_vec(),
                current_model.weights().to_vec(),
                new_modes.clone(),
                current_model.width(),
                current_model.chi(),
                current_model.carrier(),
                current_model.kappa(),
            );

            let next_model = match next_model_res {
                Ok(m) => m,
                Err(_) => {
                    // Refusal to admit: treat as rejected step
                    rejected_steps += 1;
                    h = (step_h * 0.5).max(config.min_h * 0.5);
                    continue;
                }
            };

            // Commit transaction
            t += step_h;
            current_model = next_model;
            current_radii = new_mode_radii.clone();
            accepted_steps += 1;

            let step_mean =
                complex_mean(current_model.weights(), current_model.initial_amplitudes());
            steps.push(TrajectoryStep {
                t,
                h: step_h,
                mean: step_mean,
                mean_radius: total_mean_bound,
                modes: if config.output_policy == OutputPolicy::AllModes {
                    Some(new_modes)
                } else {
                    None
                },
                mode_radii: if config.output_policy == OutputPolicy::AllModes {
                    Some(new_mode_radii)
                } else {
                    None
                },
                local_defect: cert.volterra_defect,
                total_mean_bound,
            });

            // Conservative step adjustment
            h = (step_h * 1.25).min(config.max_h);
        } else {
            // Rejected transaction: accepted state remains completely pristine
            rejected_steps += 1;
            h = (step_h * 0.5).max(config.min_h * 0.5);
        }
    }

    let final_mean = complex_mean(current_model.weights(), current_model.initial_amplitudes());
    let mut final_mean_radius = 0.0;
    for (&w, &r) in current_model.weights().iter().zip(current_radii.iter()) {
        final_mean_radius += w * r;
    }

    Ok(PacketTrajectoryResult {
        roundoff_enclosed: false,
        final_t: t,
        final_mean,
        final_mean_radius,
        final_modes: current_model.initial_amplitudes().to_vec(),
        final_mode_radii: current_radii,
        steps,
        accepted_steps,
        rejected_steps,
        work: *work,
    })
}

// A02-TXN-01: immutable seeds and staged, verified nonlinear kappa=0 windows.
// Legacy estimate-only trajectories above retain their existing authority.
use crate::packet_window::{
    PacketCurveOutputWorkspace, PacketFrozenOutput, RepresentedPacketCurve,
};
use rodas5p_core::directed::{div_up, exp_interval, sqrt_down};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static VERIFIED_LINEAGE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedTrajectoryWork {
    pub packet: PacketWorkCounters,
    pub validation_items: u64,
    pub identity_items: u64,
    pub arithmetic_attempts: u64,
    pub copied_items: u64,
    pub allocation_requests: u64,
    pub prepare_attempts: u64,
    pub query_attempts: u64,
    pub commit_attempts: u64,
    pub rejected_attempts: u64,
}
#[derive(Debug)]
pub enum VerifiedTrajectoryRefusal {
    NextCenterAdmission(CollectiveAdmissionError),
    InvalidInput(&'static str),
    Arithmetic(&'static str),
    Packet(PacketError),
    StalePreparedWindow,
    ToleranceRejected {
        time: f64,
        mode: Option<usize>,
        required: f64,
        allowed: f64,
    },
    Capacity,
}
impl From<PacketError> for VerifiedTrajectoryRefusal {
    fn from(e: PacketError) -> Self {
        Self::Packet(e)
    }
}
type VerifiedResult<T> = Result<T, VerifiedTrajectoryRefusal>;
fn va<T>(
    w: &mut VerifiedTrajectoryWork,
    stage: &'static str,
    f: impl FnOnce() -> CoreResult<T>,
) -> VerifiedResult<T> {
    w.arithmetic_attempts += 1;
    f().map_err(|_| VerifiedTrajectoryRefusal::Arithmetic(stage))
}
fn vreserve<T>(n: usize, w: &mut VerifiedTrajectoryWork) -> VerifiedResult<Vec<T>> {
    if n > isize::MAX as usize / std::mem::size_of::<T>().max(1) {
        return Err(VerifiedTrajectoryRefusal::Capacity);
    }
    w.allocation_requests += 1;
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| VerifiedTrajectoryRefusal::Capacity)?;
    Ok(v)
}
fn vcopy<T: Copy>(s: &[T], w: &mut VerifiedTrajectoryWork) -> VerifiedResult<Vec<T>> {
    let mut v = vreserve(s.len(), w)?;
    v.extend_from_slice(s);
    w.copied_items += (s.len() * (std::mem::size_of::<T>() / 8)) as u64;
    Ok(v)
}
fn vmagnitude(z: (f64, f64), w: &mut VerifiedTrajectoryWork) -> VerifiedResult<f64> {
    let re = va(w, "magnitude real square", || mul_up(z.0, z.0))?;
    let im = va(w, "magnitude imaginary square", || mul_up(z.1, z.1))?;
    let sq = va(w, "magnitude sum", || add_up(re, im))?;
    va(w, "magnitude sqrt", || sqrt_up(sq))
}
fn venergy(
    model: &CollectiveModel,
    centers: &[(f64, f64)],
    radii: &[f64],
    w: &mut VerifiedTrajectoryWork,
) -> VerifiedResult<(f64, f64, f64)> {
    let (mut s, mut energy, mut error) = (0.0, 0.0, 0.0);
    for ((&weight, &center), &radius) in model.weights().iter().zip(centers).zip(radii) {
        w.validation_items += 4;
        if !radius.is_finite() || radius < 0.0 {
            return Err(VerifiedTrajectoryRefusal::InvalidInput("initial radius"));
        }
        s = va(w, "weight sum", || add_up(s, weight))?;
        let mag = vmagnitude(center, w)?;
        let outer = va(w, "energy outer radius", || add_up(mag, radius))?;
        let sq = va(w, "energy square", || mul_up(outer, outer))?;
        let term = va(w, "weighted energy", || mul_up(weight, sq))?;
        energy = va(w, "energy sum", || add_up(energy, term))?;
        let sq = va(w, "radius square", || mul_up(radius, radius))?;
        let term = va(w, "weighted radius", || mul_up(weight, sq))?;
        error = va(w, "radius sum", || add_up(error, term))?;
    }
    Ok((s, energy, va(w, "weighted radius sqrt", || sqrt_up(error))?))
}
fn config_bits(c: &TrajectoryConfig) -> [u64; 12] {
    [
        c.t_start.to_bits(),
        c.t_end.to_bits(),
        c.initial_h.to_bits(),
        c.min_h.to_bits(),
        c.max_h.to_bits(),
        c.mean_tolerance.to_bits(),
        c.mode_tolerance.to_bits(),
        c.order as u64,
        c.degree as u64,
        c.sweeps as u64,
        c.max_steps as u64,
        c.output_policy as u64,
    ]
}
fn vvalidate_config(
    c: &TrajectoryConfig,
    grid: &[f64],
    w: &mut VerifiedTrajectoryWork,
) -> VerifiedResult<()> {
    w.validation_items += 12;
    if !c.t_start.is_finite()
        || !c.t_end.is_finite()
        || c.t_end <= c.t_start
        || !c.initial_h.is_finite()
        || c.initial_h <= 0.0
        || !c.min_h.is_finite()
        || c.min_h <= 0.0
        || !c.max_h.is_finite()
        || c.max_h < c.min_h
        || !c.mean_tolerance.is_finite()
        || c.mean_tolerance <= 0.0
        || !c.mode_tolerance.is_finite()
        || c.mode_tolerance <= 0.0
        || c.degree < c.order
        || c.sweeps == 0
        || c.max_steps == 0
    {
        return Err(VerifiedTrajectoryRefusal::InvalidInput("configuration"));
    }
    c.degree
        .checked_mul(3)
        .and_then(|x| x.checked_add(1))
        .and_then(|x| x.checked_mul(c.order.checked_add(1)?))
        .filter(|&x| x <= isize::MAX as usize / std::mem::size_of::<f64>())
        .ok_or(VerifiedTrajectoryRefusal::Capacity)?;
    for (i, &t) in grid.iter().enumerate() {
        w.validation_items += 1;
        if !t.is_finite() || t < c.t_start || t > c.t_end || (i > 0 && grid[i - 1] >= t) {
            return Err(VerifiedTrajectoryRefusal::InvalidInput("requested grid"));
        }
    }
    Ok(())
}

/// Immutable numerical result. Authority cannot be constructed from public fields.
/// ```compile_fail
/// use rodas5p_integrators::packet_trajectory::VerifiedTrajectoryPoint;
/// fn forge(p: &mut VerifiedTrajectoryPoint) { p.mean_radius = 0.0; }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedTrajectoryPoint {
    time: f64,
    local_time: f64,
    offset: Interval,
    temporal_delta: f64,
    lineage: u64,
    generation: u64,
    mean: (f64, f64),
    mean_radius: f64,
    centers: Vec<(f64, f64)>,
    radii: Vec<f64>,
    weighted_seed: f64,
    public_modes: bool,
}
impl VerifiedTrajectoryPoint {
    fn empty(n: usize, w: &mut VerifiedTrajectoryWork) -> VerifiedResult<Self> {
        let mut centers = vreserve(n, w)?;
        centers.resize(n, (0.0, 0.0));
        let mut radii = vreserve(n, w)?;
        radii.resize(n, 0.0);
        Ok(Self {
            time: 0.0,
            local_time: 0.0,
            offset: Interval { lo: 0.0, hi: 0.0 },
            temporal_delta: 0.0,
            lineage: 0,
            generation: 0,
            mean: (0.0, 0.0),
            mean_radius: 0.0,
            centers,
            radii,
            weighted_seed: 0.0,
            public_modes: true,
        })
    }
    fn copy_from(&mut self, p: &Self, w: &mut VerifiedTrajectoryWork) {
        self.time = p.time;
        self.local_time = p.local_time;
        self.offset = p.offset;
        self.temporal_delta = p.temporal_delta;
        self.lineage = p.lineage;
        self.generation = p.generation;
        self.mean = p.mean;
        self.mean_radius = p.mean_radius;
        self.weighted_seed = p.weighted_seed;
        self.public_modes = p.public_modes;
        self.centers.copy_from_slice(&p.centers);
        self.radii.copy_from_slice(&p.radii);
        w.copied_items += 11 + 3 * p.radii.len() as u64;
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn local_time(&self) -> f64 {
        self.local_time
    }
    pub fn offset_interval(&self) -> Interval {
        self.offset
    }
    pub fn temporal_delta(&self) -> f64 {
        self.temporal_delta
    }
    pub fn identity(&self) -> (u64, u64) {
        (self.lineage, self.generation)
    }
    pub fn mean(&self) -> (f64, f64) {
        self.mean
    }
    pub fn mean_radius(&self) -> f64 {
        self.mean_radius
    }
    pub fn modes(&self) -> Option<&[(f64, f64)]> {
        self.public_modes.then_some(&self.centers)
    }
    pub fn mode_radii(&self) -> Option<&[f64]> {
        self.public_modes.then_some(&self.radii)
    }
    pub fn weighted_seed(&self) -> f64 {
        self.weighted_seed
    }
}

struct WindowStartSeed {
    lineage: u64,
    generation: u64,
    time: f64,
    end: f64,
    span: Interval,
    horizon: f64,
    centers: Vec<(f64, f64)>,
    radii: Vec<f64>,
    weighted: f64,
    global_energy: f64,
    center_energy: f64,
    model: CollectiveModel,
    config: [u64; 12],
    grid: Vec<f64>,
    cursor: usize,
    s: f64,
    lambda: f64,
    q: f64,
    speed: f64,
}
/// Prepared tokens can be used only with their original complete accepted seed.
/// ```compile_fail
/// use rodas5p_integrators::packet_trajectory::PreparedVerifiedWindow;
/// fn forge(s: &mut PreparedVerifiedWindow) { s.seed.generation = 0; }
/// ```
pub struct PreparedVerifiedWindow {
    seed: Arc<WindowStartSeed>,
    owner: Option<PacketFrozenOutput>,
    local: PacketCurveOutputWorkspace,
    staged: Vec<VerifiedTrajectoryPoint>,
    endpoint: VerifiedTrajectoryPoint,
    query: VerifiedTrajectoryPoint,
    request_count: usize,
    committed: bool,
}
/// ```compile_fail
/// use rodas5p_integrators::packet_trajectory::RetainedVerifiedWindow;
/// fn forge(s: &mut RetainedVerifiedWindow) { s.seed.time = 0.0; }
/// ```
pub struct RetainedVerifiedWindow {
    seed: Arc<WindowStartSeed>,
    owner: PacketFrozenOutput,
}
pub struct VerifiedQueryWorkspace {
    local: PacketCurveOutputWorkspace,
    point: VerifiedTrajectoryPoint,
}

/// ```compile_fail
/// use rodas5p_integrators::packet_trajectory::VerifiedTrajectoryState;
/// fn forge(s: &mut VerifiedTrajectoryState) { s.weighted = 0.0; }
/// ```
/// Accepted state owns its model/config/grid/uncertainty; no setter accepts a
/// user asserted weighted seed, energy bound or certificate.
pub struct VerifiedTrajectoryState {
    lineage: u64,
    generation: u64,
    time: f64,
    centers: Vec<(f64, f64)>,
    radii: Vec<f64>,
    weighted: f64,
    energy: f64,
    model: CollectiveModel,
    config: TrajectoryConfig,
    grid: Vec<f64>,
    cursor: usize,
    outputs: Vec<VerifiedTrajectoryPoint>,
    retained: Vec<RetainedVerifiedWindow>,
}
impl VerifiedTrajectoryState {
    pub fn new(
        model: &CollectiveModel,
        radii: &[f64],
        config: &TrajectoryConfig,
        grid: &[f64],
        w: &mut VerifiedTrajectoryWork,
    ) -> VerifiedResult<Self> {
        vvalidate_config(config, grid, w)?;
        if model.kappa() != 0.0 {
            return Err(VerifiedTrajectoryRefusal::InvalidInput(
                "unsupported damping",
            ));
        }
        if radii.len() != model.num_modes() {
            return Err(VerifiedTrajectoryRefusal::InvalidInput(
                "initial radius shape",
            ));
        }
        let (s, energy, weighted) = venergy(model, model.initial_amplitudes(), radii, w)?;
        let tube = va(w, "initial tube", || mul_up(s, energy))?;
        if tube > 1.0 {
            return Err(VerifiedTrajectoryRefusal::InvalidInput(
                "initial energy tube",
            ));
        }
        let lineage = VERIFIED_LINEAGE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .map_err(|_| VerifiedTrajectoryRefusal::Capacity)?;
        let centers = vcopy(model.initial_amplitudes(), w)?;
        let radii = vcopy(radii, w)?;
        let grid = vcopy(grid, w)?;
        // CollectiveModel::clone copies its three private Vec payloads.
        w.allocation_requests += 3;
        w.copied_items += (4 * model.num_modes()) as u64;
        let mut outputs = vreserve(grid.len(), w)?;
        for _ in &grid {
            outputs.push(VerifiedTrajectoryPoint::empty(model.num_modes(), w)?);
        }
        let retained = vreserve(config.max_steps, w)?;
        Ok(Self {
            lineage,
            generation: 0,
            time: config.t_start,
            centers,
            radii,
            weighted,
            energy,
            model: model.clone(),
            config: config.clone(),
            grid,
            cursor: 0,
            outputs,
            retained,
        })
    }
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn centers(&self) -> &[(f64, f64)] {
        &self.centers
    }
    pub fn component_radii(&self) -> &[f64] {
        &self.radii
    }
    pub fn weighted_radius(&self) -> f64 {
        self.weighted
    }
    pub fn global_energy(&self) -> f64 {
        self.energy
    }
    pub fn output_cursor(&self) -> usize {
        self.cursor
    }
    pub fn observations(&self) -> &[VerifiedTrajectoryPoint] {
        &self.outputs[..self.cursor]
    }
    pub fn retained_windows(&self) -> &[RetainedVerifiedWindow] {
        &self.retained
    }
    pub fn output_policy(&self) -> OutputPolicy {
        self.config.output_policy
    }
    pub fn prepare(
        &self,
        end: f64,
        w: &mut VerifiedTrajectoryWork,
    ) -> VerifiedResult<PreparedVerifiedWindow> {
        prepare_verified_window(self, end, w)
    }
}

fn seed_matches(
    state: &VerifiedTrajectoryState,
    seed: &WindowStartSeed,
    w: &mut VerifiedTrajectoryWork,
) -> bool {
    let mut same = state.lineage == seed.lineage
        && state.generation == seed.generation
        && state.time.to_bits() == seed.time.to_bits()
        && state.weighted.to_bits() == seed.weighted.to_bits()
        && state.energy.to_bits() == seed.global_energy.to_bits()
        && state.cursor == seed.cursor
        && config_bits(&state.config) == seed.config;
    w.identity_items += 18;
    for (a, b) in state.centers.iter().zip(&seed.centers) {
        w.identity_items += 2;
        same &= a.0.to_bits() == b.0.to_bits() && a.1.to_bits() == b.1.to_bits();
    }
    for (a, b) in state.radii.iter().zip(&seed.radii) {
        w.identity_items += 1;
        same &= a.to_bits() == b.to_bits();
    }
    for (a, b) in state.grid.iter().zip(&seed.grid) {
        w.identity_items += 1;
        same &= a.to_bits() == b.to_bits();
    }
    // Both private model copies are immutable; compare all static payload bits.
    for (a, b) in state
        .model
        .xi()
        .iter()
        .zip(seed.model.xi())
        .chain(state.model.weights().iter().zip(seed.model.weights()))
    {
        w.identity_items += 1;
        same &= a.to_bits() == b.to_bits();
    }
    for (a, b) in [
        (state.model.width(), seed.model.width()),
        (state.model.chi(), seed.model.chi()),
        (state.model.carrier(), seed.model.carrier()),
        (state.model.kappa(), seed.model.kappa()),
    ] {
        w.identity_items += 1;
        same &= a.to_bits() == b.to_bits();
    }
    same
}

pub fn prepare_verified_window(
    state: &VerifiedTrajectoryState,
    end: f64,
    w: &mut VerifiedTrajectoryWork,
) -> VerifiedResult<PreparedVerifiedWindow> {
    w.prepare_attempts += 1;
    if !end.is_finite() || end <= state.time || end > state.config.t_end {
        return Err(VerifiedTrajectoryRefusal::InvalidInput("window end"));
    }
    let span = va(w, "physical span", || {
        Interval::point(end)?.sub(Interval::point(state.time)?)
    })?;
    let horizon = span.hi;
    if span.lo < 0.0 || !horizon.is_finite() || horizon <= 0.0 {
        return Err(VerifiedTrajectoryRefusal::Arithmetic("physical span"));
    }
    let zeros = {
        let mut x = vreserve(state.radii.len(), w)?;
        x.resize(state.radii.len(), 0.0);
        x
    };
    let (s, center_energy, _) = venergy(&state.model, &state.centers, &zeros, w)?;
    let s2 = va(w, "S square", || mul_up(s, s))?;
    let chi = state.model.chi().abs();
    let se = va(w, "gain energy", || mul_up(s2, chi))?;
    let lambda = va(w, "flow lambda", || {
        mul_up(se, state.energy.max(center_energy))
    })?;
    let sqrt_s = va(w, "sqrt S", || sqrt_up(s))?;
    let q0 = va(w, "Duhamel S", || mul_up(s, sqrt_s))?;
    let q1 = va(w, "Duhamel chi", || mul_up(q0, chi))?;
    let q2 = va(w, "Duhamel energy", || {
        mul_up(q1, state.energy.max(center_energy))
    })?;
    let q = va(w, "Duhamel factor3", || mul_up(q2, 3.0))?;
    let mut omega: f64 = 0.0;
    for &xi in state.model.xi() {
        let v = va(w, "frequency", || {
            Interval::point(state.model.width())?.mul(Interval::point(xi)?)
        })?;
        omega = omega.max(v.mag());
    }
    let vnl = va(w, "speed nonlinear", || mul_up(se, state.energy))?;
    let velocity = va(w, "speed sum", || add_up(omega, vnl))?;
    let root_e = va(w, "sqrt energy", || sqrt_up(state.energy))?;
    let speed = va(w, "speed", || mul_up(root_e, velocity))?;
    let xi = vcopy(state.model.xi(), w)?;
    let weights = vcopy(state.model.weights(), w)?;
    let initial = vcopy(&state.centers, w)?;
    // Constructor arithmetic is charged by its published implementation formula.
    w.packet.curve_model_constructor_payload_items += (4 * state.centers.len() + 4) as u64;
    w.packet.curve_model_constructor_arithmetic_ops += (6 * state.centers.len() + 2) as u64;
    let local_model = CollectiveModel::new(
        xi,
        weights,
        initial,
        state.model.width(),
        state.model.chi(),
        state.model.carrier(),
        0.0,
    )
    .map_err(|_| VerifiedTrajectoryRefusal::InvalidInput("next model admission"))?;
    let packet = compile_packet(&local_model, horizon, state.config.order, &mut w.packet)?;
    // Existing causal-table construction has one outer Vec plus one row Vec.
    w.allocation_requests += state.config.order as u64 + 2;
    w.arithmetic_attempts += ((state.config.order + 1) * (1 + 2 * 3 * state.config.degree)) as u64;
    let table = CausalWeights::compile(state.config.order, 3 * state.config.degree);
    let candidate = generate_candidate(
        &packet,
        state.config.degree,
        state.config.sweeps,
        &table,
        &mut w.packet,
    )?;
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut w.packet)?;
    let owner = PacketFrozenOutput::new(curve, &zeros, &mut w.packet)?;
    let local = owner.workspace(&mut w.packet)?;
    let mut request_count = 0;
    while state.cursor + request_count < state.grid.len()
        && state.grid[state.cursor + request_count] <= end
    {
        request_count += 1;
    }
    let mut staged = vreserve(request_count, w)?;
    for _ in 0..request_count {
        staged.push(VerifiedTrajectoryPoint::empty(state.centers.len(), w)?);
    }
    let endpoint = VerifiedTrajectoryPoint::empty(state.centers.len(), w)?;
    let query = VerifiedTrajectoryPoint::empty(state.centers.len(), w)?;
    let seed = WindowStartSeed {
        lineage: state.lineage,
        generation: state.generation,
        time: state.time,
        end,
        span,
        horizon,
        centers: vcopy(&state.centers, w)?,
        radii: vcopy(&state.radii, w)?,
        weighted: state.weighted,
        global_energy: state.energy,
        center_energy,
        model: local_model,
        config: config_bits(&state.config),
        grid: vcopy(&state.grid, w)?,
        cursor: state.cursor,
        s,
        lambda,
        q,
        speed,
    };
    w.allocation_requests += 1;
    Ok(PreparedVerifiedWindow {
        seed: Arc::new(seed),
        owner: Some(owner),
        local,
        staged,
        endpoint,
        query,
        request_count,
        committed: false,
    })
}

fn verified_query(
    seed: &WindowStartSeed,
    owner: &PacketFrozenOutput,
    time: f64,
    local: &mut PacketCurveOutputWorkspace,
    p: &mut VerifiedTrajectoryPoint,
    w: &mut VerifiedTrajectoryWork,
) -> VerifiedResult<()> {
    w.query_attempts += 1;
    w.validation_items += 1;
    if !time.is_finite() || time < seed.time || time > seed.end {
        return Err(VerifiedTrajectoryRefusal::InvalidInput("global query time"));
    }
    let offset = va(w, "global offset", || {
        Interval::point(time)?.sub(Interval::point(seed.time)?)
    })?;
    let u = (time - seed.time).max(0.0).min(seed.horizon);
    if offset.lo < 0.0 || offset.hi > seed.horizon {
        return Err(VerifiedTrajectoryRefusal::Arithmetic("offset domain"));
    }
    let difference = va(w, "temporal displacement", || {
        offset.sub(Interval::point(u)?)
    })?;
    let delta = difference.mag();
    let exponent = va(w, "flow exponent", || mul_up(seed.lambda, u))?;
    let gain = va(w, "flow exponential", || exp_interval(exponent))?.hi;
    let ge = va(w, "transported seed", || mul_up(gain, seed.weighted))?;
    let ktime = va(w, "temporal speed", || mul_up(seed.speed, delta))?;
    let sqrt_s = va(w, "mean sqrt S", || sqrt_up(seed.s))?;
    let u_q = va(w, "Duhamel time", || mul_up(u, seed.q))?;
    let forced = va(w, "Duhamel inherited", || mul_up(u_q, ge))?;
    let view = owner.enclose_at(u, local, &mut w.packet)?;
    let base = view.weighted_radius();
    let displacement = view.weighted_center_displacement();
    for j in 0..seed.radii.len() {
        let denominator = va(w, "mode sqrt weight", || sqrt_down(seed.model.weights()[j]))?;
        if denominator <= 0.0 {
            return Err(VerifiedTrajectoryRefusal::Arithmetic(
                "vanished weight denominator",
            ));
        }
        let norm = va(w, "mode weighted gain", || div_up(ge, denominator))?;
        let duhamel = va(w, "mode Duhamel", || add_up(seed.radii[j], forced))?;
        let lift = va(w, "mode temporal lift", || div_up(ktime, denominator))?;
        let inherited = va(w, "mode inherited lift", || add_up(norm.min(duhamel), lift))?;
        p.radii[j] = va(w, "mode total", || add_up(view.mode_radii()[j], inherited))?;
        p.centers[j] = view.modes()[j];
        w.copied_items += 3;
    }
    let total = va(w, "mean inherited lift", || add_up(ge, ktime))?;
    let mean_inherited = va(w, "mean transport", || mul_up(sqrt_s, total))?;
    p.mean_radius = va(w, "mean total", || {
        add_up(view.mean_radius(), mean_inherited)
    })?;
    let bound = va(w, "endpoint local flow", || add_up(base, ge))?;
    let bound = va(w, "endpoint time lift", || add_up(bound, ktime))?;
    p.weighted_seed = va(w, "endpoint center displacement", || {
        add_up(bound, displacement)
    })?;
    p.time = time;
    p.local_time = u;
    p.offset = offset;
    p.temporal_delta = delta;
    p.lineage = seed.lineage;
    p.generation = seed.generation;
    p.mean = view.mean();
    p.public_modes = seed.config[11] == OutputPolicy::AllModes as u64;
    w.copied_items += 10;
    Ok(())
}
fn check_tolerance(
    p: &VerifiedTrajectoryPoint,
    config: [u64; 12],
    w: &mut VerifiedTrajectoryWork,
) -> VerifiedResult<()> {
    w.validation_items += 1;
    let mean = f64::from_bits(config[5]);
    let mode = f64::from_bits(config[6]);
    if p.mean_radius > mean {
        return Err(VerifiedTrajectoryRefusal::ToleranceRejected {
            time: p.time,
            mode: None,
            required: p.mean_radius,
            allowed: mean,
        });
    }
    for (j, &r) in p.radii.iter().enumerate() {
        w.validation_items += 1;
        if r > mode {
            return Err(VerifiedTrajectoryRefusal::ToleranceRejected {
                time: p.time,
                mode: Some(j),
                required: r,
                allowed: mode,
            });
        }
    }
    Ok(())
}

pub fn advance_verified(
    state: &mut VerifiedTrajectoryState,
    prepared: &mut PreparedVerifiedWindow,
    w: &mut VerifiedTrajectoryWork,
) -> VerifiedResult<()> {
    advance_verified_impl(state, prepared, w, None)
}
#[derive(Clone, Copy, PartialEq)]
enum VerifiedTrialFault {
    BeforeQuery(usize),
    EndpointAdmission,
}
fn advance_verified_impl(
    state: &mut VerifiedTrajectoryState,
    p: &mut PreparedVerifiedWindow,
    w: &mut VerifiedTrajectoryWork,
    fail_before: Option<VerifiedTrialFault>,
) -> VerifiedResult<()> {
    w.commit_attempts += 1;
    let result = (|| {
        if !seed_matches(state, &p.seed, w) || p.committed {
            return Err(VerifiedTrajectoryRefusal::StalePreparedWindow);
        }
        let next = state
            .generation
            .checked_add(1)
            .ok_or(VerifiedTrajectoryRefusal::Capacity)?;
        if state.retained.len() == state.retained.capacity() {
            return Err(VerifiedTrajectoryRefusal::Capacity);
        }
        let owner = p
            .owner
            .as_ref()
            .ok_or(VerifiedTrajectoryRefusal::StalePreparedWindow)?;
        for index in 0..p.request_count {
            if fail_before == Some(VerifiedTrialFault::BeforeQuery(index)) {
                return Err(VerifiedTrajectoryRefusal::Packet(
                    PacketError::OutwardRefusal("private transaction test seam"),
                ));
            }
            let t = p.seed.grid[p.seed.cursor + index];
            verified_query(&p.seed, owner, t, &mut p.local, &mut p.query, w)?;
            check_tolerance(&p.query, p.seed.config, w)?;
            p.staged[index].copy_from(&p.query, w);
        }
        if fail_before == Some(VerifiedTrialFault::BeforeQuery(p.request_count)) {
            return Err(VerifiedTrajectoryRefusal::Packet(
                PacketError::OutwardRefusal("private transaction test seam"),
            ));
        }
        verified_query(&p.seed, owner, p.seed.end, &mut p.local, &mut p.endpoint, w)?;
        check_tolerance(&p.endpoint, p.seed.config, w)?;
        // Admission uses borrowed immutable static model inputs and the same
        // returned endpoint centers. All attempted work survives typed refusal.
        #[cfg(test)]
        if fail_before == Some(VerifiedTrialFault::EndpointAdmission) {
            // Private rejected-trial seam only: finite center outside the same
            // constructor energy domain, after all endpoint tolerance checks.
            p.endpoint.centers[0] = (2.0, 0.0);
        }
        let mut admission_work = CollectiveAdmissionWork::default();
        let admitted = state
            .model
            .validate_candidate_centers_with_work(&p.endpoint.centers, &mut admission_work);
        w.packet.curve_model_constructor_payload_items += admission_work.payload_items;
        w.packet.curve_model_constructor_arithmetic_ops += admission_work.arithmetic_ops;
        admitted.map_err(VerifiedTrajectoryRefusal::NextCenterAdmission)?;

        // Immutable private tokens cannot change during evaluation; nevertheless
        // full identity is checked again immediately before the atomic commit.
        if !seed_matches(state, &p.seed, w) {
            return Err(VerifiedTrajectoryRefusal::StalePreparedWindow);
        }
        let owner = p
            .owner
            .take()
            .ok_or(VerifiedTrajectoryRefusal::StalePreparedWindow)?;
        // Move the private seed without allocation; replace only after all
        // fallible work succeeds, using a prepared move slot.
        state.time = p.seed.end;
        state.centers.copy_from_slice(&p.endpoint.centers);
        state.radii.copy_from_slice(&p.endpoint.radii);
        state.weighted = p.endpoint.weighted_seed;
        w.copied_items += 3 * state.centers.len() as u64 + 2;
        for index in 0..p.request_count {
            state.outputs[state.cursor + index].copy_from(&p.staged[index], w);
        }
        state.cursor += p.request_count;
        state.generation = next;
        // Owner/seed storage is transferred using a private Option slot below.
        p.committed = true;
        state.retained.push(RetainedVerifiedWindow {
            seed: Arc::clone(&p.seed),
            owner,
        });
        Ok(())
    })();
    if result.is_err() {
        w.rejected_attempts += 1;
    }
    result
}

impl RetainedVerifiedWindow {
    pub fn start_time(&self) -> f64 {
        self.seed.time
    }
    pub fn end_time(&self) -> f64 {
        self.seed.end
    }
    pub fn physical_span(&self) -> Interval {
        self.seed.span
    }
    pub fn represented_horizon(&self) -> f64 {
        self.seed.horizon
    }
    pub fn identity(&self) -> (u64, u64) {
        (self.seed.lineage, self.seed.generation)
    }
    pub fn start_weighted_radius(&self) -> f64 {
        self.seed.weighted
    }
    pub fn center_energy(&self) -> f64 {
        self.seed.center_energy
    }
    pub fn workspace(
        &self,
        w: &mut VerifiedTrajectoryWork,
    ) -> VerifiedResult<VerifiedQueryWorkspace> {
        Ok(VerifiedQueryWorkspace {
            local: self.owner.workspace(&mut w.packet)?,
            point: VerifiedTrajectoryPoint::empty(self.seed.radii.len(), w)?,
        })
    }
    pub fn query<'a>(
        &self,
        time: f64,
        workspace: &'a mut VerifiedQueryWorkspace,
        w: &mut VerifiedTrajectoryWork,
    ) -> VerifiedResult<&'a VerifiedTrajectoryPoint> {
        verified_query(
            &self.seed,
            &self.owner,
            time,
            &mut workspace.local,
            &mut workspace.point,
            w,
        )?;
        Ok(&workspace.point)
    }
}

impl VerifiedTrajectoryState {
    /// Caller-owned diagnostic snapshot; allocate outside measured operations.
    pub fn snapshot_bits(&self) -> Vec<u64> {
        let mut v = vec![
            self.lineage,
            self.generation,
            self.time.to_bits(),
            self.weighted.to_bits(),
            self.energy.to_bits(),
            self.cursor as u64,
            self.retained.len() as u64,
        ];
        for &(re, im) in &self.centers {
            v.extend([re.to_bits(), im.to_bits()]);
        }
        v.extend(self.radii.iter().map(|x| x.to_bits()));
        for p in self.observations() {
            p.append_bits(&mut v);
        }
        for r in &self.retained {
            v.extend([
                r.seed.lineage,
                r.seed.generation,
                r.seed.time.to_bits(),
                r.seed.end.to_bits(),
                r.seed.weighted.to_bits(),
                r.seed.global_energy.to_bits(),
            ]);
        }
        v
    }
}
impl VerifiedTrajectoryPoint {
    fn append_bits(&self, v: &mut Vec<u64>) {
        v.extend([
            self.time.to_bits(),
            self.local_time.to_bits(),
            self.offset.lo.to_bits(),
            self.offset.hi.to_bits(),
            self.temporal_delta.to_bits(),
            self.lineage,
            self.generation,
            self.mean.0.to_bits(),
            self.mean.1.to_bits(),
            self.mean_radius.to_bits(),
            self.weighted_seed.to_bits(),
        ]);
        for &(re, im) in &self.centers {
            v.extend([re.to_bits(), im.to_bits()]);
        }
        v.extend(self.radii.iter().map(|x| x.to_bits()));
    }
    pub fn snapshot_bits(&self) -> Vec<u64> {
        let mut v = Vec::new();
        self.append_bits(&mut v);
        v
    }
}

/// Execute the prescribed adaptive schedule. Only physical tolerance rejection
/// reduces h; arithmetic/input failures return immediately with the caller's
/// accepted state preserved. All rejected work remains in the supplied counters.
pub fn complete_verified_trajectory(
    state: &mut VerifiedTrajectoryState,
    w: &mut VerifiedTrajectoryWork,
) -> VerifiedResult<()> {
    let mut h = state.config.initial_h.min(state.config.max_h);
    let mut attempts = 0;
    while state.time < state.config.t_end {
        if attempts >= state.config.max_steps {
            return Err(VerifiedTrajectoryRefusal::Capacity);
        }
        attempts += 1;
        let end = (state.time + h).min(state.config.t_end);
        if !end.is_finite() || end <= state.time || h < state.config.min_h {
            return Err(VerifiedTrajectoryRefusal::InvalidInput(
                "minimum advancing step",
            ));
        }
        let mut prepared = prepare_verified_window(state, end, w)?;
        match advance_verified(state, &mut prepared, w) {
            Ok(()) => {}
            Err(VerifiedTrajectoryRefusal::ToleranceRejected { .. }) => {
                h *= 0.5;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod verified_transaction_private_tests {
    use super::*;
    mod packet_counter_contract {
        use super::{PacketWorkCounters, VerifiedTrajectoryWork};
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/a02_packet_counter_contract.rs"
        ));
    }
    fn fixed() -> (CollectiveModel, TrajectoryConfig, Vec<f64>) {
        let model = CollectiveModel::new(
            vec![-0.5, 0.0, 0.5],
            vec![0.25, 0.5, 0.25],
            vec![(0.25, -0.125), (0.125, 0.0625), (-0.0625, 0.25)],
            1.0,
            0.0,
            0.0,
            0.0,
        )
        .unwrap();
        let mut config = TrajectoryConfig::new(
            0.0,
            0.25,
            0.0625,
            f64::from_bits(0x3ee4f8b588e368f1),
            f64::from_bits(0x3ee4f8b588e368f1),
        )
        .unwrap();
        config.order = 8;
        config.degree = 10;
        config.sweeps = 4;
        config.min_h = f64::from_bits(0x3eb0c6f7a0b5ed8d);
        config.max_h = 0.125;
        config.max_steps = 1000;
        (model, config, vec![0.03125, 0.09375, 0.15625, 0.25])
    }
    #[test]
    fn txn_fail_first_and_second_preserve_all_accepted_bits() {
        let (model, config, grid) = fixed();
        for fault in [0, 1] {
            let mut work = VerifiedTrajectoryWork::default();
            let mut state =
                VerifiedTrajectoryState::new(&model, &[0.0; 3], &config, &grid, &mut work).unwrap();
            let mut prepared = state.prepare(0.0625, &mut work).unwrap();
            let snapshot = state.snapshot_bits();
            let before = work.clone();
            let result = advance_verified_impl(
                &mut state,
                &mut prepared,
                &mut work,
                Some(VerifiedTrialFault::BeforeQuery(fault)),
            );
            assert!(matches!(
                result,
                Err(VerifiedTrajectoryRefusal::Packet(
                    PacketError::OutwardRefusal(_)
                ))
            ));
            assert_eq!(snapshot, state.snapshot_bits());
            packet_counter_contract::assert_delta(
                &before.packet,
                &work.packet,
                &packet_counter_contract::query(3, 8, 10, fault as u64),
                if fault == 0 {
                    "TXN_FAIL_FIRST"
                } else {
                    "TXN_FAIL_SECOND"
                },
            );
            println!(
                "private_transaction_packet_delta before={} after={} expected={}",
                serde_json::to_string(&before.packet).unwrap(),
                serde_json::to_string(&work.packet).unwrap(),
                serde_json::to_string(&packet_counter_contract::query(3, 8, 10, fault as u64))
                    .unwrap()
            );
            packet_counter_contract::assert_full_delta(
                &before,
                &work,
                &packet_counter_contract::fault_adapter(3, 8, 10, grid.len(), fault),
                "private fault full",
            );
            assert_eq!(work.commit_attempts - before.commit_attempts, 1);
            assert_eq!(work.rejected_attempts - before.rejected_attempts, 1);
            assert_eq!(work.query_attempts - before.query_attempts, fault as u64);
            assert_eq!(
                work.arithmetic_attempts - before.arithmetic_attempts,
                (15 + 6 * 3) * fault as u64
            );
        }
    }
    #[test]
    fn endpoint_admission_refusal_preserves_transaction_and_full_work() {
        let (model, config, grid) = fixed();
        let mut work = VerifiedTrajectoryWork::default();
        let mut state =
            VerifiedTrajectoryState::new(&model, &[0.0; 3], &config, &grid, &mut work).unwrap();
        let mut prepared = state.prepare(0.0625, &mut work).unwrap();
        let bits = state.snapshot_bits();
        let before = work.clone();
        let result = advance_verified_impl(
            &mut state,
            &mut prepared,
            &mut work,
            Some(VerifiedTrialFault::EndpointAdmission),
        );
        assert!(matches!(
            result,
            Err(VerifiedTrajectoryRefusal::NextCenterAdmission(
                CollectiveAdmissionError::EnergyExceedsUnitBall { .. }
            ))
        ));
        assert_eq!(state.snapshot_bits(), bits);
        let expected =
            packet_counter_contract::admission_rejection_adapter(3, 8, 10, grid.len(), 1);
        packet_counter_contract::assert_full_delta(
            &before,
            &work,
            &expected,
            "endpoint admission rejection full",
        );
        println!(
            "endpoint_admission_rejection before={} after={} expected={} inclusion_credit=NONE",
            serde_json::to_string(&before).unwrap(),
            serde_json::to_string(&work).unwrap(),
            serde_json::to_string(&expected).unwrap()
        );
    }

    #[test]
    fn inherited_and_stationary_seeds_use_same_center_displacement_and_fixed_energy() {
        let (_, mut config, _) = fixed();
        config.t_end = 0.125;
        config.mean_tolerance = f64::from_bits(0x3f1a36e2eb1c432d);
        config.mode_tolerance = config.mean_tolerance;
        let cases = [
            (
                CollectiveModel::new(
                    vec![-0.5, 0.5],
                    vec![0.5, 0.5],
                    vec![(0.25, 0.0), (0.0, 0.125)],
                    1.0,
                    0.0,
                    0.0,
                    0.0,
                )
                .unwrap(),
                vec![
                    f64::from_bits(0x3eb0c6f7a0b5ed8d),
                    f64::from_bits(0x3ec0c6f7a0b5ed8d),
                ],
                false,
            ),
            (
                CollectiveModel::new(
                    vec![0.0; 3],
                    vec![f64::from_bits(0x3d70000000000000), 0.5, 0.5],
                    vec![(0.25, 0.0), (0.0, 0.125), (-0.125, 0.0625)],
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                )
                .unwrap(),
                vec![
                    f64::from_bits(0x3e10000000000000),
                    f64::from_bits(0x3e20000000000000),
                    f64::from_bits(0x3e30000000000000),
                ],
                true,
            ),
        ];
        for (model, radii, stationary) in cases {
            let grid = if stationary {
                vec![0.03125, 0.09375, 0.125]
            } else {
                vec![0.03125, 0.125]
            };
            let mut work = VerifiedTrajectoryWork::default();
            let mut state =
                VerifiedTrajectoryState::new(&model, &radii, &config, &grid, &mut work).unwrap();
            let energy = state.global_energy().to_bits();
            for end in [0.0625, 0.125] {
                let mut p = state.prepare(end, &mut work).unwrap();
                let (expected, expected_radii, displacement) = {
                    let mut pw = PacketWorkCounters::default();
                    let owner = p.owner.as_ref().unwrap();
                    let mut ws = owner.workspace(&mut pw).unwrap();
                    let u = end - p.seed.time;
                    let view = owner.enclose_at(u, &mut ws, &mut pw).unwrap();
                    let gain = exp_interval(mul_up(p.seed.lambda, u).unwrap()).unwrap().hi;
                    let ge = mul_up(gain, p.seed.weighted).unwrap();
                    let delta = Interval::point(end)
                        .unwrap()
                        .sub(Interval::point(p.seed.time).unwrap())
                        .unwrap()
                        .sub(Interval::point(u).unwrap())
                        .unwrap()
                        .mag();
                    let kt = mul_up(p.seed.speed, delta).unwrap();
                    assert_eq!(kt, 0.0);
                    let disp = view.weighted_center_displacement();
                    let expected = add_up(
                        add_up(add_up(view.weighted_radius(), ge).unwrap(), kt).unwrap(),
                        disp,
                    )
                    .unwrap();
                    let mut rr = Vec::new();
                    for j in 0..radii.len() {
                        let root = sqrt_down(model.weights()[j]).unwrap();
                        let norm = div_up(ge, root).unwrap();
                        let forced = mul_up(mul_up(u, p.seed.q).unwrap(), ge).unwrap();
                        let duhamel = add_up(p.seed.radii[j], forced).unwrap();
                        assert_eq!(
                            duhamel.to_bits(),
                            p.seed.radii[j].to_bits(),
                            "chi=0 must preserve the inherited marginal branch"
                        );
                        let lifted = add_up(norm.min(duhamel), div_up(kt, root).unwrap()).unwrap();
                        rr.push(add_up(view.mode_radii()[j], lifted).unwrap());
                    }
                    (expected, rr, disp)
                };
                let before = work.clone();
                let k = p.request_count;
                advance_verified(&mut state, &mut p, &mut work).unwrap();
                packet_counter_contract::assert_full_delta(
                    &before,
                    &work,
                    &packet_counter_contract::commit_adapter(
                        model.num_modes(),
                        config.order,
                        config.degree,
                        grid.len(),
                        k,
                    ),
                    "inherited/stationary commit full",
                );
                assert_eq!(state.global_energy().to_bits(), energy);
                assert_eq!(state.weighted_radius().to_bits(), expected.to_bits());
                assert_eq!(
                    state
                        .component_radii()
                        .iter()
                        .map(|x| x.to_bits())
                        .collect::<Vec<_>>(),
                    expected_radii
                        .iter()
                        .map(|x| x.to_bits())
                        .collect::<Vec<_>>()
                );
                println!(
                    "seed_gate stationary={} end={} delta_w={} e_next={} E_global_bits={} inherited_source_expression=PASS",
                    stationary, end, displacement, expected, energy
                );
            }
        }
    }
}
