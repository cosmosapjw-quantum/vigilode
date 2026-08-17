use std::{sync::Arc, time::Instant};

use rodas5p_core::{
    CoreError, CoreResult, LinearMethod, LinearSolverConfig, WorkCounters, error_scale, safe_l2,
    wrms,
};
use serde::{Deserialize, Serialize};

use crate::{
    AdaptiveControllerState, AdaptiveStepConfig, ControllerKind, FusedOrthogonalization,
    FusedPhiKrylovConfig, FusedPhiPrefixSession, OdeProblem, ParallelExecution,
    pexprb54s4_fused_step, pexprb54s4_tableau, sequential_matrix_free_step,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum G4S5B0Profile {
    Smoke,
    Canonical,
    Calibration128,
    Holdout512,
}

impl G4S5B0Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Canonical => "canonical",
            Self::Calibration128 => "calibration-128",
            Self::Holdout512 => "holdout-512",
        }
    }

    pub fn dimensions(self) -> &'static [usize] {
        match self {
            Self::Smoke => &[32],
            Self::Canonical => &[128, 512],
            Self::Calibration128 => &[128],
            Self::Holdout512 => &[512],
        }
    }

    pub fn uses_canonical_tolerances(self) -> bool {
        !matches!(self, Self::Smoke)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0StepRow {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub step_index: usize,
    pub t_start: f64,
    pub h: f64,
    pub transition_level: f64,

    pub rodas_embedded_error: f64,
    pub rodas_wall_seconds: f64,
    pub rodas_rhs_evaluations: u64,
    pub rodas_jvp_vectors: u64,
    pub rodas_linear_matvecs: u64,

    pub exponential_completed: bool,
    pub exponential_total_error: Option<f64>,
    pub exponential_locally_admissible: bool,
    pub exponential_wall_seconds: Option<f64>,
    pub exponential_prefix_wall_seconds: Option<f64>,
    pub exponential_rhs_evaluations: Option<u64>,
    pub exponential_jvp_vectors: Option<u64>,
    pub exponential_maximum_krylov_dimension: Option<usize>,
    pub exponential_phi_substeps: Option<usize>,
    pub exponential_failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0TrajectorySummary {
    pub trajectory_id: String,
    pub family: String,
    pub dimension: usize,
    pub rtol: f64,
    pub success: bool,
    pub failure: Option<String>,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub endpoint_time: f64,
    pub explicit_jacobian_builds: u64,
    pub direct_factorizations: u64,
    pub newton_iterations: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct G4S5B0Report {
    pub schema: &'static str,
    pub status: &'static str,
    pub profile: &'static str,
    pub switching_active: bool,
    pub committed_method: &'static str,
    pub rows: Vec<G4S5B0StepRow>,
    pub trajectories: Vec<G4S5B0TrajectorySummary>,
    pub limitations: Vec<String>,
}

type TransitionMetric = Arc<dyn Fn(f64, &[f64]) -> f64 + Send + Sync>;

struct AtlasProblem {
    family: &'static str,
    id: String,
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    transition: TransitionMetric,
}

fn smooth_ramp(t: f64, center: f64, width: f64) -> (f64, f64) {
    let z = (t - center) / width;
    let th = z.tanh();
    (0.5 * (1.0 + th), 0.5 * (1.0 - th * th) / width)
}

fn block_padding_rhs(y: &[f64], out: &mut [f64], first_padding: usize, rate: f64) {
    for i in first_padding..y.len() {
        out[i] = -rate * y[i];
    }
}

fn robertson_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    if n < 3 {
        return Err(CoreError::InvalidInput(
            "Robertson atlas requires n>=3".into(),
        ));
    }
    let blocks = n / 3;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.045, 0.010);
        let activity = 0.05 + 0.95 * ramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = 1.0 + 0.02 * (block % 5) as f64;
            let k1 = 0.04 * scale;
            let k2 = 1.0e4 * activity * scale;
            let k3 = 3.0e7 * activity * scale;
            let y1 = y[i];
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            out[i] = -k1 * y1 + k2 * y2 * y3;
            out[i + 1] = k1 * y1 - k2 * y2 * y3 - k3 * y2 * y2;
            out[i + 2] = k3 * y2 * y2;
        }
        block_padding_rhs(y, out, 3 * blocks, 20.0 * activity);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.045, 0.010);
        let activity = 0.05 + 0.95 * ramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = 1.0 + 0.02 * (block % 5) as f64;
            let k1 = 0.04 * scale;
            let k2 = 1.0e4 * activity * scale;
            let k3 = 3.0e7 * activity * scale;
            out[i] = -k1 * v[i] + k2 * y[i + 2] * v[i + 1] + k2 * y[i + 1] * v[i + 2];
            out[i + 1] = k1 * v[i] + (-k2 * y[i + 2] - 2.0 * k3 * y[i + 1]) * v[i + 1]
                - k2 * y[i + 1] * v[i + 2];
            out[i + 2] = 2.0 * k3 * y[i + 1] * v[i + 1];
        }
        for i in 3 * blocks..n {
            out[i] = -20.0 * activity * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.045, 0.010);
        let dactivity = 0.95 * dramp;
        for block in 0..blocks {
            let i = 3 * block;
            let scale = 1.0 + 0.02 * (block % 5) as f64;
            let dk2 = 1.0e4 * dactivity * scale;
            let dk3 = 3.0e7 * dactivity * scale;
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            out[i] = dk2 * y2 * y3;
            out[i + 1] = -dk2 * y2 * y3 - dk3 * y2 * y2;
            out[i + 2] = dk3 * y2 * y2;
        }
        for i in 3 * blocks..n {
            out[i] = -20.0 * dactivity * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[3 * block] = 1.0;
    }
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.045, 0.010).0);
    Ok(AtlasProblem {
        family: "robertson-ramped",
        id: format!("robertson-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("robertson-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 0.10),
        transition,
    })
}

fn hires_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    if n < 8 {
        return Err(CoreError::InvalidInput("HIRES atlas requires n>=8".into()));
    }
    let blocks = n / 8;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.08);
        let activity = 0.1 + 0.9 * ramp;
        for block in 0..blocks {
            let i = 8 * block;
            let y1 = y[i];
            let y2 = y[i + 1];
            let y3 = y[i + 2];
            let y4 = y[i + 3];
            let y5 = y[i + 4];
            let y6 = y[i + 5];
            let y7 = y[i + 6];
            let y8 = y[i + 7];
            let q = 280.0 * activity * y6 * y8;
            out[i] = -1.71 * y1 + 0.43 * y2 + 8.32 * y3 + 0.0007;
            out[i + 1] = 1.71 * y1 - 8.75 * y2;
            out[i + 2] = -10.03 * y3 + 0.43 * y4 + 0.035 * y5;
            out[i + 3] = 8.32 * y2 + 1.71 * y3 - 1.12 * y4;
            out[i + 4] = -1.745 * y5 + 0.43 * y6 + 0.43 * y7;
            out[i + 5] = -q + 0.69 * y4 + 1.71 * y5 - 0.43 * y6 + 0.69 * y7;
            out[i + 6] = q - 1.81 * y7;
            out[i + 7] = -q + 1.81 * y7;
        }
        block_padding_rhs(y, out, 8 * blocks, 2.0 + 20.0 * activity);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.08);
        let activity = 0.1 + 0.9 * ramp;
        for block in 0..blocks {
            let i = 8 * block;
            let qv = 280.0 * activity * (y[i + 7] * v[i + 5] + y[i + 5] * v[i + 7]);
            out[i] = -1.71 * v[i] + 0.43 * v[i + 1] + 8.32 * v[i + 2];
            out[i + 1] = 1.71 * v[i] - 8.75 * v[i + 1];
            out[i + 2] = -10.03 * v[i + 2] + 0.43 * v[i + 3] + 0.035 * v[i + 4];
            out[i + 3] = 8.32 * v[i + 1] + 1.71 * v[i + 2] - 1.12 * v[i + 3];
            out[i + 4] = -1.745 * v[i + 4] + 0.43 * v[i + 5] + 0.43 * v[i + 6];
            out[i + 5] =
                -qv + 0.69 * v[i + 3] + 1.71 * v[i + 4] - 0.43 * v[i + 5] + 0.69 * v[i + 6];
            out[i + 6] = qv - 1.81 * v[i + 6];
            out[i + 7] = -qv + 1.81 * v[i + 6];
        }
        for i in 8 * blocks..n {
            out[i] = -(2.0 + 20.0 * activity) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.45, 0.08);
        let dactivity = 0.9 * dramp;
        out.fill(0.0);
        for block in 0..blocks {
            let i = 8 * block;
            let dq = 280.0 * dactivity * y[i + 5] * y[i + 7];
            out[i + 5] = -dq;
            out[i + 6] = dq;
            out[i + 7] = -dq;
        }
        for i in 8 * blocks..n {
            out[i] = -20.0 * dactivity * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[8 * block] = 1.0;
        y0[8 * block + 7] = 0.0057;
    }
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.45, 0.08).0);
    Ok(AtlasProblem {
        family: "hires-ramped",
        id: format!("hires-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("hires-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn vdp_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    if n < 2 {
        return Err(CoreError::InvalidInput("VDP atlas requires n>=2".into()));
    }
    let blocks = n / 2;
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let mu = 10.0 + 490.0 * ramp;
        for block in 0..blocks {
            let i = 2 * block;
            let local_mu = mu * (1.0 + 0.01 * (block % 7) as f64);
            out[i] = y[i + 1];
            out[i + 1] = local_mu * (1.0 - y[i] * y[i]) * y[i + 1] - y[i];
        }
        block_padding_rhs(y, out, 2 * blocks, 5.0 + mu);
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let mu = 10.0 + 490.0 * ramp;
        for block in 0..blocks {
            let i = 2 * block;
            let local_mu = mu * (1.0 + 0.01 * (block % 7) as f64);
            out[i] = v[i + 1];
            out[i + 1] = (-2.0 * local_mu * y[i] * y[i + 1] - 1.0) * v[i]
                + local_mu * (1.0 - y[i] * y[i]) * v[i + 1];
        }
        for i in 2 * blocks..n {
            out[i] = -(5.0 + mu) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (_, dramp) = smooth_ramp(t, 0.50, 0.08);
        let dmu = 490.0 * dramp;
        out.fill(0.0);
        for block in 0..blocks {
            let i = 2 * block;
            let local_dmu = dmu * (1.0 + 0.01 * (block % 7) as f64);
            out[i + 1] = local_dmu * (1.0 - y[i] * y[i]) * y[i + 1];
        }
        for i in 2 * blocks..n {
            out[i] = -dmu * y[i];
        }
        Ok(())
    });
    let mut y0 = vec![0.0; n];
    for block in 0..blocks {
        y0[2 * block] = 2.0;
    }
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.50, 0.08).0);
    Ok(AtlasProblem {
        family: "van-der-pol-ramped",
        id: format!("van-der-pol-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("van-der-pol-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn exact_shape(n: usize, t: f64) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let k = 1.0 + (i % 7) as f64;
            0.4 * (k * t).sin() + 0.2 * (0.5 * k * t).cos()
        })
        .collect()
}

fn exact_shape_first(n: usize, t: f64) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let k = 1.0 + (i % 7) as f64;
            0.4 * k * (k * t).cos() - 0.1 * k * (0.5 * k * t).sin()
        })
        .collect()
}

fn apply_rotating_operator(n: usize, t: f64, x: &[f64], out: &mut [f64]) {
    let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
    let stiffness = 20.0 + 480.0 * ramp;
    let eta = 0.1 + 0.8 * ramp;
    let theta = 8.0 * t + 0.4 * (4.0 * t).sin();
    let c = theta.cos();
    let s = theta.sin();
    let blocks = n / 2;
    for block in 0..blocks {
        let i = 2 * block;
        let xr0 = c * x[i] + s * x[i + 1];
        let xr1 = -s * x[i] + c * x[i + 1];
        let ar0 = -stiffness * xr0 + eta * stiffness * xr1;
        let ar1 = -0.35 * stiffness * xr1;
        out[i] = c * ar0 - s * ar1;
        out[i + 1] = s * ar0 + c * ar1;
    }
    for i in 2 * blocks..n {
        out[i] = -stiffness * x[i];
    }
}

fn rotating_nonnormal_problem(n: usize) -> CoreResult<AtlasProblem> {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = exact_shape(n, t);
        let dphi = exact_shape_first(n, t);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_operator(n, t, &defect, out);
        let (ramp, _) = smooth_ramp(t, 0.60, 0.06);
        let nonlinear = 40.0 * ramp;
        for i in 0..n {
            out[i] += dphi[i] + nonlinear * (y[i] * y[i] - phi[i] * phi[i]);
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        apply_rotating_operator(n, t, v, out);
        let (ramp, _) = smooth_ramp(t, 0.60, 0.06);
        let nonlinear = 40.0 * ramp;
        for i in 0..n {
            out[i] += 2.0 * nonlinear * y[i] * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let eps = 1.0e-6;
        let mut plus = vec![0.0; n];
        let mut minus = vec![0.0; n];
        let phi_p = exact_shape(n, t + eps);
        let dphi_p = exact_shape_first(n, t + eps);
        let defect_p = y.iter().zip(&phi_p).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_operator(n, t + eps, &defect_p, &mut plus);
        let (rp, _) = smooth_ramp(t + eps, 0.60, 0.06);
        for i in 0..n {
            plus[i] += dphi_p[i] + 40.0 * rp * (y[i] * y[i] - phi_p[i] * phi_p[i]);
        }
        let phi_m = exact_shape(n, t - eps);
        let dphi_m = exact_shape_first(n, t - eps);
        let defect_m = y.iter().zip(&phi_m).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_rotating_operator(n, t - eps, &defect_m, &mut minus);
        let (rm, _) = smooth_ramp(t - eps, 0.60, 0.06);
        for i in 0..n {
            minus[i] += dphi_m[i] + 40.0 * rm * (y[i] * y[i] - phi_m[i] * phi_m[i]);
            out[i] = (plus[i] - minus[i]) / (2.0 * eps);
        }
        Ok(())
    });
    let y0 = exact_shape(n, 0.0);
    let exact = Arc::new(move |t: f64| exact_shape(n, t));
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.55, 0.08).0);
    Ok(AtlasProblem {
        family: "rotating-nonnormal",
        id: format!("rotating-nonnormal-n{n}"),
        problem: OdeProblem::new(
            format!("rotating-nonnormal-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            Some(exact),
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn nonautonomous_forcing_problem(n: usize) -> CoreResult<AtlasProblem> {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.07);
        let stiffness = 30.0 + 470.0 * ramp;
        let frequency = 2.0 + 28.0 * ramp;
        for i in 0..n {
            let phase = (i % 11) as f64 * 0.17;
            let phi = (frequency * t + phase).sin();
            let dphi = frequency * (frequency * t + phase).cos();
            let defect = y[i] - phi;
            out[i] = -stiffness * defect + dphi + 20.0 * ramp * defect * defect;
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        let (ramp, _) = smooth_ramp(t, 0.45, 0.07);
        let stiffness = 30.0 + 470.0 * ramp;
        let frequency = 2.0 + 28.0 * ramp;
        for i in 0..n {
            let phase = (i % 11) as f64 * 0.17;
            let phi = (frequency * t + phase).sin();
            let defect = y[i] - phi;
            out[i] = (-stiffness + 40.0 * ramp * defect) * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let eps = 1.0e-6;
        let evaluate = |time: f64, state: &[f64]| {
            let (ramp, _) = smooth_ramp(time, 0.45, 0.07);
            let stiffness = 30.0 + 470.0 * ramp;
            let frequency = 2.0 + 28.0 * ramp;
            (0..n)
                .map(|i| {
                    let phase = (i % 11) as f64 * 0.17;
                    let phi = (frequency * time + phase).sin();
                    let dphi = frequency * (frequency * time + phase).cos();
                    let defect = state[i] - phi;
                    -stiffness * defect + dphi + 20.0 * ramp * defect * defect
                })
                .collect::<Vec<_>>()
        };
        let plus = evaluate(t + eps, y);
        let minus = evaluate(t - eps, y);
        for i in 0..n {
            out[i] = (plus[i] - minus[i]) / (2.0 * eps);
        }
        Ok(())
    });
    let y0 = (0..n)
        .map(|i| ((i % 11) as f64 * 0.17).sin())
        .collect::<Vec<_>>();
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.45, 0.07).0);
    Ok(AtlasProblem {
        family: "nonautonomous-stiff-forcing",
        id: format!("nonautonomous-stiff-forcing-n{n}"),
        problem: OdeProblem::new(
            format!("nonautonomous-stiff-forcing-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            None,
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn apply_advection_diffusion(n: usize, t: f64, x: &[f64], out: &mut [f64]) {
    let dx = 1.0 / (n + 1) as f64;
    let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
    let diffusion = 0.002;
    let advection = 0.5 + 3.5 * ramp;
    let reaction = -1.0;
    for i in 0..n {
        let left = if i == 0 { 0.0 } else { x[i - 1] };
        let right = if i + 1 == n { 0.0 } else { x[i + 1] };
        out[i] = diffusion * (left - 2.0 * x[i] + right) / (dx * dx)
            - advection * (x[i] - left) / dx
            + reaction * x[i];
    }
}

fn semilinear_transition_problem(n: usize) -> CoreResult<AtlasProblem> {
    let dx = 1.0 / (n + 1) as f64;
    let exact = Arc::new(move |t: f64| {
        (1..=n)
            .map(|i| (-t).exp() * (std::f64::consts::PI * i as f64 * dx).sin())
            .collect::<Vec<_>>()
    });
    let rhs_exact = exact.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let phi = rhs_exact(t);
        let defect = y.iter().zip(&phi).map(|(a, b)| a - b).collect::<Vec<_>>();
        apply_advection_diffusion(n, t, &defect, out);
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let nonlinear = 2.0 + 48.0 * ramp;
        for i in 0..n {
            out[i] += -phi[i] + nonlinear * (y[i] * y[i] - phi[i] * phi[i]);
        }
        Ok(())
    });
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        apply_advection_diffusion(n, t, v, out);
        let (ramp, _) = smooth_ramp(t, 0.50, 0.08);
        let nonlinear = 2.0 + 48.0 * ramp;
        for i in 0..n {
            out[i] += 2.0 * nonlinear * y[i] * v[i];
        }
        Ok(())
    });
    let partial_t = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let eps = 1.0e-6;
        let evaluate = |time: f64, state: &[f64]| {
            let phi = (1..=n)
                .map(|i| (-time).exp() * (std::f64::consts::PI * i as f64 * dx).sin())
                .collect::<Vec<_>>();
            let defect = state
                .iter()
                .zip(&phi)
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>();
            let mut value = vec![0.0; n];
            apply_advection_diffusion(n, time, &defect, &mut value);
            let (ramp, _) = smooth_ramp(time, 0.50, 0.08);
            let nonlinear = 2.0 + 48.0 * ramp;
            for i in 0..n {
                value[i] += -phi[i] + nonlinear * (state[i] * state[i] - phi[i] * phi[i]);
            }
            value
        };
        let plus = evaluate(t + eps, y);
        let minus = evaluate(t - eps, y);
        for i in 0..n {
            out[i] = (plus[i] - minus[i]) / (2.0 * eps);
        }
        Ok(())
    });
    let y0 = exact(0.0);
    let transition = Arc::new(|t: f64, _y: &[f64]| smooth_ramp(t, 0.50, 0.08).0);
    Ok(AtlasProblem {
        family: "semilinear-advection-diffusion-ramped",
        id: format!("semilinear-advection-diffusion-ramped-n{n}"),
        problem: OdeProblem::new(
            format!("semilinear-advection-diffusion-ramped-n{n}"),
            n,
            rhs,
            None,
            None,
            Some(jvp),
            Some(partial_t),
            false,
            None,
            Some(exact),
        )?,
        y0,
        t_span: (0.0, 1.0),
        transition,
    })
}

fn build_problems(profile: G4S5B0Profile) -> CoreResult<Vec<AtlasProblem>> {
    let mut problems = Vec::new();
    for &n in profile.dimensions() {
        problems.push(robertson_transition_problem(n)?);
        problems.push(hires_transition_problem(n)?);
        problems.push(vdp_transition_problem(n)?);
        problems.push(rotating_nonnormal_problem(n)?);
        problems.push(nonautonomous_forcing_problem(n)?);
        problems.push(semilinear_transition_problem(n)?);
    }
    Ok(problems)
}

fn adaptive_config(profile: G4S5B0Profile, span: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: if profile.uses_canonical_tolerances() {
            1.0e-7
        } else {
            1.0e-6
        },
        rtol: if profile.uses_canonical_tolerances() {
            1.0e-5
        } else {
            1.0e-4
        },
        initial_step: (span / 20.0).max(1.0e-8),
        min_step: 1.0e-12,
        max_step: span / 5.0,
        max_attempts: 20_000,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 4.0,
        reject_max_factor: 0.8,
        controller: ControllerKind::Pi,
    }
}

fn phi_config(rtol: f64, dimension: usize) -> FusedPhiKrylovConfig {
    FusedPhiKrylovConfig {
        minimum_dimension: 2,
        maximum_dimension: (dimension + 4).min(32),
        dimension_increment: 2,
        relative_tolerance: (0.03 * rtol).max(1.0e-12),
        absolute_tolerance: (3.0e-4 * rtol).max(1.0e-14),
        orthogonalization: FusedOrthogonalization::FullMgs,
        maximum_substeps: 16,
    }
}

fn linear_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-10,
        atol: 1.0e-12,
        restart: 32,
        maxiter: 256,
        ..LinearSolverConfig::default()
    }
}

struct ExponentialShadow {
    completed: bool,
    total_error: Option<f64>,
    admissible: bool,
    wall: Option<f64>,
    prefix_wall: Option<f64>,
    work: Option<WorkCounters>,
    max_dimension: Option<usize>,
    substeps: Option<usize>,
    failure: Option<String>,
}

fn benchmark_operation<F>(minimum_sample_seconds: f64, mut operation: F) -> CoreResult<f64>
where
    F: FnMut() -> CoreResult<()>,
{
    operation()?;
    let mut batch = 1usize;
    loop {
        let start = Instant::now();
        for _ in 0..batch {
            operation()?;
        }
        if start.elapsed().as_secs_f64() >= minimum_sample_seconds || batch >= (1 << 16) {
            break;
        }
        batch = batch.saturating_mul(2);
    }
    let mut samples = Vec::with_capacity(3);
    for _ in 0..3 {
        let start = Instant::now();
        for _ in 0..batch {
            operation()?;
        }
        samples.push(start.elapsed().as_secs_f64() / batch as f64);
    }
    samples.sort_by(f64::total_cmp);
    Ok(samples[1])
}

fn shadow_problem_state(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let base = problem.jvp_only_clone()?;
    if problem.autonomous {
        Ok((base, y.to_vec()))
    } else {
        let augmented = base.time_augmented_clone()?;
        let mut state = y.to_vec();
        state.push(t);
        Ok((augmented, state))
    }
}

fn measure_e_prefix(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    minimum_sample_seconds: f64,
) -> CoreResult<f64> {
    let (shadow_problem, state) = shadow_problem_state(problem, t, y)?;
    let mut prep = WorkCounters::default();
    let f0 = shadow_problem.eval_rhs(t, &state, &mut prep)?;
    let tableau = pexprb54s4_tableau();
    let scale = tableau.c2 * h;
    let mut vectors = vec![vec![0.0; shadow_problem.dimension]; 2];
    for i in 0..shadow_problem.dimension {
        vectors[1][i] = tableau.c2 * f0[i] / scale;
    }
    benchmark_operation(minimum_sample_seconds, || {
        let operator = shadow_problem.linearize_matrix_free(t, &state)?;
        let mut counters = WorkCounters::default();
        let _ = FusedPhiPrefixSession::begin(
            operator,
            scale,
            &vectors,
            FusedPhiKrylovConfig {
                minimum_dimension: 1,
                maximum_dimension: config.maximum_dimension,
                dimension_increment: 1,
                relative_tolerance: config.relative_tolerance,
                absolute_tolerance: config.absolute_tolerance,
                orthogonalization: FusedOrthogonalization::FullMgs,
                maximum_substeps: 1,
            },
            2,
            &mut counters,
        )?;
        Ok(())
    })
}

fn run_exponential_shadow(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    atol: f64,
    rtol: f64,
    profile: G4S5B0Profile,
) -> ExponentialShadow {
    let result = (|| -> CoreResult<ExponentialShadow> {
        let (shadow_problem, state) = shadow_problem_state(problem, t, y)?;
        let config = phi_config(rtol, shadow_problem.dimension + 4);
        let execution = ParallelExecution::sequential();
        let start = Instant::now();
        let report = pexprb54s4_fused_step(&shadow_problem, t, &state, h, config, &execution)?;
        let wall = start.elapsed().as_secs_f64();
        let physical_n = problem.dimension;
        let y_new = &report.y_new[..physical_n];
        let error_vector = report
            .error_estimate
            .as_ref()
            .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 omitted embedded error".into()))?;
        let scale = error_scale(y, y_new, &[atol], rtol)?;
        let time_error = wrms(&error_vector[..physical_n], &scale)?;
        let phi_error = h.abs()
            * report
                .fused_phi_reports
                .iter()
                .map(|entry| entry.error_estimate)
                .filter(|value| value.is_finite())
                .sum::<f64>()
            / safe_l2(&scale).max(f64::MIN_POSITIVE);
        let total_error = time_error.max(phi_error);
        let max_dimension = report
            .fused_phi_reports
            .iter()
            .map(|entry| entry.maximum_krylov_dimension)
            .max();
        let substeps = report
            .fused_phi_reports
            .iter()
            .map(|entry| entry.substeps)
            .sum();
        let prefix_wall = measure_e_prefix(
            problem,
            t,
            y,
            h,
            config,
            if profile.uses_canonical_tolerances() {
                0.001
            } else {
                0.0002
            },
        )?;
        Ok(ExponentialShadow {
            completed: true,
            total_error: Some(total_error),
            admissible: total_error.is_finite() && total_error <= 1.0,
            wall: Some(wall),
            prefix_wall: Some(prefix_wall),
            work: Some(report.work),
            max_dimension,
            substeps: Some(substeps),
            failure: None,
        })
    })();
    result.unwrap_or_else(|error| ExponentialShadow {
        completed: false,
        total_error: None,
        admissible: false,
        wall: None,
        prefix_wall: None,
        work: None,
        max_dimension: None,
        substeps: None,
        failure: Some(error.to_string()),
    })
}

fn run_trajectory(
    problem: AtlasProblem,
    profile: G4S5B0Profile,
    include_exponential_shadow: bool,
) -> (Vec<G4S5B0StepRow>, G4S5B0TrajectorySummary) {
    let adaptive = adaptive_config(profile, problem.t_span.1 - problem.t_span.0);
    let linear = linear_config();
    let mut controller = AdaptiveControllerState::default();
    let mut t = problem.t_span.0;
    let tf = problem.t_span.1;
    let mut y = problem.y0.clone();
    let mut h = adaptive.initial_step.min(tf - t);
    let tolerance = 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let mut attempts = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut rows = Vec::new();
    let mut total_jacobian_builds = 0u64;
    let mut total_factorizations = 0u64;
    let mut failure = None;

    while t < tf - tolerance && attempts < adaptive.max_attempts {
        attempts += 1;
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step {
            failure = Some("minimum step reached".into());
            break;
        }
        let mut step_counters = WorkCounters::default();
        let start = Instant::now();
        let trial = sequential_matrix_free_step(
            &problem.problem,
            t,
            &y,
            h,
            &linear,
            None,
            adaptive.atol,
            adaptive.rtol,
            false,
            &mut step_counters,
        );
        let rodas_wall = start.elapsed().as_secs_f64();
        total_jacobian_builds += step_counters.jacobian_builds;
        total_factorizations += step_counters.direct_factorizations;
        let report = match trial {
            Ok(report) => report,
            Err(CoreError::NonFinite(_) | CoreError::LinearSolve(_)) => {
                rejected += 1;
                h *= adaptive.min_factor;
                continue;
            }
            Err(error) => {
                failure = Some(error.to_string());
                break;
            }
        };
        let error = report.error_norm;
        let step_accepted = report.accepted
            && error.is_finite()
            && error <= 1.0
            && report.y_new.iter().all(|value| value.is_finite());
        if !step_accepted {
            rejected += 1;
            let _ = controller.record_rejection(error.max(1.0e-16));
            h *= controller
                .propose_factor(&adaptive, error.max(1.0e-16), 5, false)
                .unwrap_or(adaptive.min_factor);
            continue;
        }

        let shadow = if include_exponential_shadow {
            run_exponential_shadow(
                &problem.problem,
                t,
                &y,
                h,
                adaptive.atol,
                adaptive.rtol,
                profile,
            )
        } else {
            ExponentialShadow {
                completed: false,
                total_error: None,
                admissible: false,
                wall: None,
                prefix_wall: None,
                work: None,
                max_dimension: None,
                substeps: None,
                failure: None,
            }
        };
        let transition_level = (problem.transition)(t, &y);
        let exp_work = shadow.work.unwrap_or_default();
        rows.push(G4S5B0StepRow {
            trajectory_id: problem.id.clone(),
            family: problem.family.into(),
            dimension: problem.problem.dimension,
            rtol: adaptive.rtol,
            step_index: accepted,
            t_start: t,
            h,
            transition_level,
            rodas_embedded_error: error,
            rodas_wall_seconds: rodas_wall,
            rodas_rhs_evaluations: step_counters.rhs_evaluations,
            rodas_jvp_vectors: step_counters.jvp_vectors,
            rodas_linear_matvecs: step_counters.linear_matvecs,
            exponential_completed: shadow.completed,
            exponential_total_error: shadow.total_error,
            exponential_locally_admissible: shadow.admissible,
            exponential_wall_seconds: shadow.wall,
            exponential_prefix_wall_seconds: shadow.prefix_wall,
            exponential_rhs_evaluations: shadow.completed.then_some(exp_work.rhs_evaluations),
            exponential_jvp_vectors: shadow.completed.then_some(exp_work.jvp_vectors),
            exponential_maximum_krylov_dimension: shadow.max_dimension,
            exponential_phi_substeps: shadow.substeps,
            exponential_failure: shadow.failure,
        });

        t = report.t_new;
        y = report.y_new;
        accepted += 1;
        let _ = controller.record_acceptance(error);
        h *= controller
            .propose_factor(&adaptive, error.max(1.0e-16), 5, true)
            .unwrap_or(1.0);
    }

    let success = t >= tf - tolerance;
    if !success && failure.is_none() {
        failure = Some("maximum attempts reached".into());
    }
    let summary = G4S5B0TrajectorySummary {
        trajectory_id: problem.id,
        family: problem.family.into(),
        dimension: problem.problem.dimension,
        rtol: adaptive.rtol,
        success,
        failure,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        endpoint_time: t,
        explicit_jacobian_builds: total_jacobian_builds,
        direct_factorizations: total_factorizations,
        newton_iterations: 0,
    };
    (rows, summary)
}

pub fn run_g4_s5b0_regime_atlas(profile: G4S5B0Profile) -> CoreResult<G4S5B0Report> {
    let mut rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        let (mut local_rows, summary) = run_trajectory(problem, profile, true);
        rows.append(&mut local_rows);
        trajectories.push(summary);
    }
    let status = if trajectories.iter().all(|row| row.success) {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0Report {
        schema: "g4-s5b0-expanded-paired-regime-atlas-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        rows,
        trajectories,
        limitations: vec![
            "The six families are transition-bearing scaled/replicated atlas variants, not canonical work-precision reproductions of the original benchmarks.".into(),
            "The committed trajectory is protected matrix-free RODAS5P; E-K is read-only shadow telemetry.".into(),
            "One-shot full-step wall times require repeated campaigns and median aggregation before residence labels are authoritative.".into(),
            "N=2048 remains sealed for the later selector holdout.".into(),
            "No active method switching or physical-client execution occurs in this node.".into(),
        ],
    })
}

pub fn run_g4_s5b0_rjf_only(profile: G4S5B0Profile) -> CoreResult<G4S5B0Report> {
    let mut rows = Vec::new();
    let mut trajectories = Vec::new();
    for problem in build_problems(profile)? {
        let (mut local_rows, summary) = run_trajectory(problem, profile, false);
        rows.append(&mut local_rows);
        trajectories.push(summary);
    }
    let status = if trajectories.iter().all(|row| row.success) {
        "complete"
    } else {
        "complete-with-failures"
    };
    Ok(G4S5B0Report {
        schema: "g4-s5b0-rjf-only-regime-replay-v1",
        status,
        profile: profile.as_str(),
        switching_active: false,
        committed_method: "protected-sequential-matrix-free-rodas5p",
        rows,
        trajectories,
        limitations: vec![
            "This runner executes only the committed protected R-JF trajectory; all E-K fields are intentionally empty.".into(),
            "E-K labels and optimized timing may be joined only from separately provenance-checked durable audit data after exact R-JF numerical parity checks.".into(),
            "No active method switching or E-K prefix/full-step work occurs in this runner.".into(),
        ],
    })
}
