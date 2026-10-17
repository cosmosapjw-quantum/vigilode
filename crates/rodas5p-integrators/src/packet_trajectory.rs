//! Uncertainty propagation and adaptive window transaction for collective packet trajectories.
//!
//! Research node LP04 of `research/promotion_loop12_20261011/PORTING_DAG.json`.
//! Implements multi-window causal packet advancement with:
//! 1. Inherited uncertainty transfer across windows:
//!    \[
//!    \rho_{\text{mean}}^{(k)} = \sum_j w_j \rho_j^{(k)}, \quad E_{\text{mean}}^{(k+1)} = \frac{D + \tau(A_0 + h G / (p+2)) + \rho_{\text{mean}}^{(k)}}{1 - h L}
//!    \]
//!    \[
//!    E_{\text{mode}, j}^{(k+1)} = \tau(|z_j| + h G / (p+2)) + h L E_{\text{mean}}^{(k+1)} + \rho_j^{(k)}
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
    CausalWeights, compile_packet, compute_certificate, generate_candidate, reconstruct_modes,
};
use crate::packet_work::PacketWorkCounters;
use rodas5p_core::directed::{add_up, mul_up, sqrt_up};
use rodas5p_core::{CoreError, CoreResult};

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
            let mode_b =
                local_bound + step_h * cert.lipschitz_l * inherited_mean_radius / denom + prev_r;
            max_mode_bound = max_mode_bound.max(mode_b);
            new_mode_radii.push(mode_b);
        }

        // Budget check
        let accepted =
            total_mean_bound <= config.mean_tolerance && max_mode_bound <= config.mode_tolerance;

        if accepted {
            // Reconstruct new microscopic mode amplitudes
            let new_modes = reconstruct_modes(&cand, &packet, work);

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
