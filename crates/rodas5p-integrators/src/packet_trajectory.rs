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

use crate::collective_model::CollectiveModel;
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
