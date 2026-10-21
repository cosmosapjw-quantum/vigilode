use rodas5p_core::{CoreError, CoreResult, DenseMatrix};
use rodas5p_integrators::OdeProblem;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Case {
    pub case_id: String,
    pub model: String,
    pub parameters: Value,
    pub initial_state: Vec<f64>,
    pub t_span: [f64; 2],
    pub output_times: Vec<f64>,
    pub rtol: f64,
    pub atol: Vec<f64>,
    pub mass_matrix: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    pub algorithm_config: Value,
    #[serde(skip)]
    derivative_rhs_calls: Arc<AtomicU64>,
    #[serde(skip)]
    stencil_jacobian_builds: Arc<AtomicU64>,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub case: Case,
    #[serde(default)]
    pub solver_id: String,
    #[serde(default)]
    pub solver_config: Value,
    #[serde(default)]
    pub probes: Vec<ProbeSpec>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProbeSpec {
    pub time: f64,
    pub state: Vec<f64>,
    pub direction: Vec<f64>,
}

impl Case {
    pub fn derivative_policy(&self) -> &'static str {
        if self.algorithm_config["jacobian_assembly"] == "analytic_stencil" {
            "analytic_jvp_with_opt_in_stencil_dense_assembly"
        } else {
            "analytic_jvp_columns"
        }
    }
    pub fn derivative_rhs_calls(&self) -> u64 {
        self.derivative_rhs_calls.load(Ordering::Relaxed)
    }
    pub fn stencil_jacobian_builds(&self) -> u64 {
        self.stencil_jacobian_builds.load(Ordering::Relaxed)
    }
    fn p(&self, key: &str) -> f64 {
        self.parameters[key].as_f64().unwrap_or(f64::NAN)
    }
    pub fn supported(&self) -> bool {
        matches!(
            self.model.as_str(),
            "scalar_decay"
                | "logistic"
                | "kepler"
                | "robertson"
                | "vanderpol"
                | "hires"
                | "pr_linear"
                | "pr_nonlinear"
                | "rotation"
                | "damping"
                | "diagonal_stiff"
                | "jordan"
                | "nonnormal"
                | "periodic_heat"
                | "advection_diffusion"
                | "brusselator_1d"
                | "gray_scott_2d"
                | "fourier_path"
                | "sabr"
        )
    }
    pub fn autonomous(&self) -> bool {
        !matches!(
            self.model.as_str(),
            "pr_linear" | "pr_nonlinear" | "fourier_path"
        )
    }
    pub fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        if y.len() != self.initial_state.len() || out.len() != y.len() {
            return Err(CoreError::Dimension(
                "benchmark RHS dimension mismatch".into(),
            ));
        }
        match self.model.as_str() {
            "scalar_decay" => out[0] = self.p("rate") * y[0],
            "sabr" => out[0] = self.p("lambda") * y[0],
            "logistic" => out[0] = self.p("rate") * y[0] * (1.0 - y[0] / self.p("capacity")),
            "kepler" => {
                let r2 = y[0] * y[0] + y[1] * y[1];
                let q = -self.p("mu") / (r2 * r2.sqrt());
                out[0] = y[2];
                out[1] = y[3];
                out[2] = q * y[0];
                out[3] = q * y[1];
            }
            "robertson" => {
                let a = self.p("k1") * y[0];
                let b = self.p("k2") * y[1] * y[2];
                let c = self.p("k3") * y[1] * y[1];
                out[0] = -a + b;
                out[1] = a - b - c;
                out[2] = c;
            }
            "vanderpol" => {
                out[0] = y[1];
                out[1] = self.p("mu") * (1.0 - y[0] * y[0]) * y[1] - y[0];
            }
            "hires" => {
                out[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
                out[1] = 1.71 * y[0] - 8.75 * y[1];
                out[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
                out[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
                out[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
                out[5] =
                    -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
                out[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
                out[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
            }
            "pr_linear" | "pr_nonlinear" => {
                let d = y[0] - t.sin();
                out[0] = self.p("lambda") * d + t.cos() + self.p("beta") * d * d;
            }
            "rotation" => {
                let w = self.p("omega");
                out[0] = -w * y[1];
                out[1] = w * y[0];
            }
            "damping" => {
                let w = self.p("omega");
                out[0] = y[1];
                out[1] = -w * w * y[0] - 2.0 * self.p("zeta") * w * y[1];
            }
            "diagonal_stiff" => {
                let rates = self.parameters["rates"]
                    .as_array()
                    .ok_or_else(|| CoreError::InvalidInput("rates absent".into()))?;
                for i in 0..y.len() {
                    out[i] = rates[i].as_f64().unwrap_or(f64::NAN) * y[i];
                }
            }
            "jordan" => {
                let a = self.p("lambda");
                out[0] = a * y[0] + self.p("offdiag") * y[1];
                out[1] = a * y[1];
            }
            "nonnormal" => {
                out[0] = self.p("a") * y[0] + self.p("b") * y[1];
                out[1] = self.p("d") * y[1];
            }
            "periodic_heat" | "advection_diffusion" => {
                let n = y.len();
                let d = self.p("diffusion") * (n * n) as f64;
                let v = if self.model == "advection_diffusion" {
                    self.p("velocity") * n as f64
                } else {
                    0.0
                };
                for i in 0..n {
                    let left = y[(i + n - 1) % n];
                    out[i] = d * (left - 2.0 * y[i] + y[(i + 1) % n]) - v * (y[i] - left);
                }
            }
            "brusselator_1d" => {
                let n = y.len() / 2;
                let d = self.p("diffusion_factor");
                for i in 0..n {
                    let u = y[2 * i];
                    let v = y[2 * i + 1];
                    let ul = if i == 0 {
                        self.p("boundary_u")
                    } else {
                        y[2 * i - 2]
                    };
                    let ur = if i + 1 == n {
                        self.p("boundary_u")
                    } else {
                        y[2 * i + 2]
                    };
                    let vl = if i == 0 {
                        self.p("boundary_v")
                    } else {
                        y[2 * i - 1]
                    };
                    let vr = if i + 1 == n {
                        self.p("boundary_v")
                    } else {
                        y[2 * i + 3]
                    };
                    out[2 * i] = 1.0 + u * u * v - 4.0 * u + d * (ul - 2.0 * u + ur);
                    out[2 * i + 1] = 3.0 * u - u * u * v + d * (vl - 2.0 * v + vr);
                }
            }
            "gray_scott_2d" => {
                let n = self.p("grid") as usize;
                let n2 = n * n;
                if y.len() != 2 * n2 {
                    return Err(CoreError::Dimension("Gray Scott grid mismatch".into()));
                }
                let du = self.p("du");
                let dv = self.p("dv");
                let feed = self.p("feed");
                let kill = self.p("kill");
                for k in 0..n2 {
                    let i = k % n;
                    let j = k / n;
                    let a = ((i + n - 1) % n) + j * n;
                    let b = ((i + 1) % n) + j * n;
                    let c = i + ((j + 1) % n) * n;
                    let d = i + ((j + n - 1) % n) * n;
                    let u = y[k];
                    let v = y[n2 + k];
                    let uv2 = u * v * v;
                    let lu = (y[a] + y[b] + y[c] + y[d] - 4.0 * u) * (n2 as f64);
                    let lv =
                        (y[n2 + a] + y[n2 + b] + y[n2 + c] + y[n2 + d] - 4.0 * v) * (n2 as f64);
                    out[k] = du * lu - uv2 + feed * (1.0 - u);
                    out[n2 + k] = dv * lv + uv2 - (feed + kill) * v;
                }
            }
            "fourier_path" => {
                let ar = y[0];
                let ai = y[1];
                let br = y[2];
                let bi = y[3];
                let phase = self.p("omega") * t;
                let c = self.p("g") + self.p("eps") * (phase.cos() * ar - phase.sin() * ai);
                let q = 1.25 * self.p("sigma");
                out[0] = -q * c * (ar * bi - ai * br);
                out[1] = q * c * (ar * br + ai * bi);
                out[2] = -q * c * ar * ai;
                out[3] = 0.5 * q * c * (ar * ar - ai * ai);
            }
            "mass_invertible" => {
                out[0] = -2.0 * y[0] - y[1];
                out[1] = -3.0 * y[1];
            }
            "mass_ill_conditioned" => {
                out[0] = -y[0];
                out[1] = -self.p("epsilon") * y[1];
            }
            "dae_index1" | "dae_index1_inconsistent" => {
                out[0] = -y[0] + y[1];
                out[1] = y[0] + y[1] - (2.0 * t.sin() + t.cos());
            }
            "robertson_constraint" => {
                let a = self.p("k1") * y[0];
                let b = self.p("k2") * y[1] * y[2];
                let c = self.p("k3") * y[1] * y[1];
                out[0] = -a + b;
                out[1] = a - b - c;
                out[2] = y[0] + y[1] + y[2] - 1.0;
            }
            "pendulum_index3" => {
                out[0] = y[2];
                out[1] = y[3];
                out[2] = -y[4] * y[0];
                out[3] = -self.p("g") - y[4] * y[1];
                out[4] = y[0] * y[0] + y[1] * y[1] - self.p("length").powi(2);
            }
            _ => {
                return Err(CoreError::InvalidInput(
                    "unsupported benchmark model".into(),
                ));
            }
        }
        if out.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite("benchmark RHS nonfinite".into()))
        }
    }
    pub fn jvp_into(&self, t: f64, y: &[f64], v: &[f64], out: &mut [f64]) -> CoreResult<()> {
        let n = y.len();
        if v.len() != n || out.len() != n || n != self.initial_state.len() {
            return Err(CoreError::Dimension(
                "benchmark JVP dimension mismatch".into(),
            ));
        }
        match self.model.as_str() {
            "scalar_decay" => out[0] = self.p("rate") * v[0],
            "sabr" => out[0] = self.p("lambda") * v[0],
            "logistic" => out[0] = self.p("rate") * (1.0 - 2.0 * y[0] / self.p("capacity")) * v[0],
            "kepler" => {
                let r2 = y[0] * y[0] + y[1] * y[1];
                let q = -self.p("mu") / (r2 * r2.sqrt());
                let dot = y[0] * v[0] + y[1] * v[1];
                out[0] = v[2];
                out[1] = v[3];
                out[2] = q * (v[0] - 3.0 * y[0] * dot / r2);
                out[3] = q * (v[1] - 3.0 * y[1] * dot / r2);
            }
            "robertson" => {
                let k1 = self.p("k1");
                let k2 = self.p("k2");
                let k3 = self.p("k3");
                let a = -k1 * v[0] + k2 * (v[1] * y[2] + y[1] * v[2]);
                let c = 2.0 * k3 * y[1] * v[1];
                out[0] = a;
                out[1] = -a - c;
                out[2] = c;
            }
            "vanderpol" => {
                let mu = self.p("mu");
                out[0] = v[1];
                out[1] = (-2.0 * mu * y[0] * y[1] - 1.0) * v[0] + mu * (1.0 - y[0] * y[0]) * v[1];
            }
            "hires" => {
                let dq = 280.0 * (v[5] * y[7] + y[5] * v[7]);
                out[0] = -1.71 * v[0] + 0.43 * v[1] + 8.32 * v[2];
                out[1] = 1.71 * v[0] - 8.75 * v[1];
                out[2] = -10.03 * v[2] + 0.43 * v[3] + 0.035 * v[4];
                out[3] = 8.32 * v[1] + 1.71 * v[2] - 1.12 * v[3];
                out[4] = -1.745 * v[4] + 0.43 * v[5] + 0.43 * v[6];
                out[5] = -dq + 0.69 * v[3] + 1.71 * v[4] - 0.43 * v[5] + 0.69 * v[6];
                out[6] = dq - 1.81 * v[6];
                out[7] = -dq + 1.81 * v[6];
            }
            "pr_linear" | "pr_nonlinear" => {
                out[0] = (self.p("lambda") + 2.0 * self.p("beta") * (y[0] - t.sin())) * v[0]
            }
            "rotation" => {
                let w = self.p("omega");
                out[0] = -w * v[1];
                out[1] = w * v[0];
            }
            "damping" => {
                let w = self.p("omega");
                out[0] = v[1];
                out[1] = -w * w * v[0] - 2.0 * self.p("zeta") * w * v[1];
            }
            "diagonal_stiff" => {
                let rates = self.parameters["rates"]
                    .as_array()
                    .ok_or_else(|| CoreError::InvalidInput("rates absent".into()))?;
                for i in 0..n {
                    out[i] = rates[i].as_f64().unwrap_or(f64::NAN) * v[i];
                }
            }
            "jordan" => {
                let a = self.p("lambda");
                out[0] = a * v[0] + self.p("offdiag") * v[1];
                out[1] = a * v[1];
            }
            "nonnormal" => {
                out[0] = self.p("a") * v[0] + self.p("b") * v[1];
                out[1] = self.p("d") * v[1];
            }
            "periodic_heat" | "advection_diffusion" => {
                self.rhs(t, v, out)?;
                self.derivative_rhs_calls.fetch_add(1, Ordering::Relaxed);
            }
            "brusselator_1d" => {
                let d = self.p("diffusion_factor");
                for i in 0..n / 2 {
                    let u = y[2 * i];
                    let z = y[2 * i + 1];
                    let du = v[2 * i];
                    let dz = v[2 * i + 1];
                    let dul = if i == 0 { 0.0 } else { v[2 * i - 2] };
                    let dur = if i + 1 == n / 2 { 0.0 } else { v[2 * i + 2] };
                    let dzl = if i == 0 { 0.0 } else { v[2 * i - 1] };
                    let dzr = if i + 1 == n / 2 { 0.0 } else { v[2 * i + 3] };
                    out[2 * i] = (2.0 * u * z - 4.0) * du + u * u * dz + d * (dul - 2.0 * du + dur);
                    out[2 * i + 1] =
                        (3.0 - 2.0 * u * z) * du - u * u * dz + d * (dzl - 2.0 * dz + dzr);
                }
            }
            "gray_scott_2d" => {
                let side = self.p("grid") as usize;
                let n2 = side * side;
                let du = self.p("du");
                let dv = self.p("dv");
                let feed = self.p("feed");
                let kill = self.p("kill");
                for k in 0..n2 {
                    let i = k % side;
                    let j = k / side;
                    let a = ((i + side - 1) % side) + j * side;
                    let b = ((i + 1) % side) + j * side;
                    let c = i + ((j + 1) % side) * side;
                    let d = i + ((j + side - 1) % side) * side;
                    let u = y[k];
                    let z = y[n2 + k];
                    let x = v[k];
                    let w = v[n2 + k];
                    let lu = (v[a] + v[b] + v[c] + v[d] - 4.0 * x) * (n2 as f64);
                    let lv =
                        (v[n2 + a] + v[n2 + b] + v[n2 + c] + v[n2 + d] - 4.0 * w) * (n2 as f64);
                    let reaction = z * z * x + 2.0 * u * z * w;
                    out[k] = du * lu - reaction - feed * x;
                    out[n2 + k] = dv * lv + reaction - (feed + kill) * w;
                }
            }
            "fourier_path" => {
                let ar = y[0];
                let ai = y[1];
                let br = y[2];
                let bi = y[3];
                let var = v[0];
                let vai = v[1];
                let vbr = v[2];
                let vbi = v[3];
                let phase = self.p("omega") * t;
                let cs = phase.cos();
                let sn = phase.sin();
                let c = self.p("g") + self.p("eps") * (cs * ar - sn * ai);
                let dc = self.p("eps") * (cs * var - sn * vai);
                let q = 1.25 * self.p("sigma");
                let p = ar * br + ai * bi;
                let dp = var * br + ar * vbr + vai * bi + ai * vbi;
                let z = ar * bi - ai * br;
                let dz = var * bi + ar * vbi - vai * br - ai * vbr;
                let r = ar * ar - ai * ai;
                let dr = 2.0 * ar * var - 2.0 * ai * vai;
                let s = 2.0 * ar * ai;
                let ds = 2.0 * (var * ai + ar * vai);
                out[0] = -q * (dc * z + c * dz);
                out[1] = q * (dc * p + c * dp);
                out[2] = -0.5 * q * (dc * s + c * ds);
                out[3] = 0.5 * q * (dc * r + c * dr);
            }
            "mass_invertible" => {
                out[0] = -2.0 * v[0] - v[1];
                out[1] = -3.0 * v[1];
            }
            "mass_ill_conditioned" => {
                out[0] = -v[0];
                out[1] = -self.p("epsilon") * v[1];
            }
            "dae_index1" | "dae_index1_inconsistent" => {
                out[0] = -v[0] + v[1];
                out[1] = v[0] + v[1];
            }
            "robertson_constraint" => {
                let k1 = self.p("k1");
                let k2 = self.p("k2");
                let k3 = self.p("k3");
                let a = -k1 * v[0] + k2 * (v[1] * y[2] + y[1] * v[2]);
                let c = 2.0 * k3 * y[1] * v[1];
                out[0] = a;
                out[1] = -a - c;
                out[2] = v[0] + v[1] + v[2];
            }
            "pendulum_index3" => {
                out[0] = v[2];
                out[1] = v[3];
                out[2] = -y[4] * v[0] - y[0] * v[4];
                out[3] = -y[4] * v[1] - y[1] * v[4];
                out[4] = 2.0 * y[0] * v[0] + 2.0 * y[1] * v[1];
            }
            _ => {
                return Err(CoreError::InvalidInput(
                    "unsupported derivative model".into(),
                ));
            }
        }
        if out.iter().all(|x| x.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite("benchmark JVP nonfinite".into()))
        }
    }
    pub fn jac_into(
        &self,
        t: f64,
        y: &[f64],
        out: &mut DenseMatrix,
        basis: &mut [f64],
        column: &mut [f64],
    ) -> CoreResult<()> {
        let n = y.len();
        if n == 0
            || n != self.initial_state.len()
            || out.nrows() != n
            || out.ncols() != n
            || basis.len() != n
            || column.len() != n
        {
            return Err(CoreError::Dimension(
                "benchmark Jacobian dimension mismatch".into(),
            ));
        }
        if self.algorithm_config["jacobian_assembly"] == "analytic_stencil" {
            if !matches!(self.model.as_str(), "periodic_heat" | "advection_diffusion") {
                return Err(CoreError::InvalidInput(
                    "analytic_stencil requires periodic_heat or advection_diffusion".into(),
                ));
            }
            let d = self.p("diffusion") * (n as f64) * (n as f64);
            let v = if self.model == "advection_diffusion" {
                self.p("velocity") * n as f64
            } else {
                0.0
            };
            let center = -2.0 * d - v;
            let left = d + v;
            if ![d, v, center, left].iter().all(|x| x.is_finite()) {
                return Err(CoreError::NonFinite("benchmark stencil nonfinite".into()));
            }
            // Dense clearing remains O(n^2); only the nonzero stencil assembly
            // is O(n). Add contributions: for n=1/2 periodic neighbors coincide.
            out.as_mut_slice().fill(0.0);
            for i in 0..n {
                if n == 1 {
                    continue; // Both periodic flux differences vanish identically.
                }
                if n == 2 {
                    out[(i, i)] = center;
                    out[(i, 1 - i)] = -center;
                    continue;
                }
                out[(i, i)] += center;
                out[(i, (i + n - 1) % n)] += left;
                out[(i, (i + 1) % n)] += d;
            }
            self.stencil_jacobian_builds.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
        basis.fill(0.0);
        for j in 0..n {
            basis[j] = 1.0;
            self.jvp_into(t, y, basis, column)?;
            for i in 0..n {
                out[(i, j)] = column[i];
            }
            basis[j] = 0.0;
        }
        Ok(())
    }
    pub fn probe(&self, points: &[ProbeSpec]) -> Value {
        let points: Vec<ProbeSpec> = if points.is_empty() {
            let n = self.initial_state.len();
            vec![
                ProbeSpec {
                    time: self.t_span[0],
                    state: self.initial_state.clone(),
                    direction: (0..n).map(|i| 0.5 + 0.01 * i as f64).collect(),
                },
                ProbeSpec {
                    time: self.t_span[0] + 0.37 * (self.t_span[1] - self.t_span[0]),
                    state: self
                        .initial_state
                        .iter()
                        .enumerate()
                        .map(|(i, x)| x + 0.01 * (i + 1) as f64)
                        .collect(),
                    direction: (0..n).map(|i| 0.5 + 0.01 * i as f64).collect(),
                },
            ]
        } else {
            points.to_vec()
        };
        let n = self.initial_state.len();
        let mut rows = Vec::new();
        for p in points {
            if p.state.len() != n || p.direction.len() != n {
                return json!({"status":"ENVIRONMENT_FAILURE","reason":"probe dimension mismatch","case_id":self.case_id});
            }
            let mut rhs = vec![0.0; n];
            let mut jvp = vec![0.0; n];
            if let Err(e) = self.rhs(p.time, &p.state, &mut rhs) {
                return json!({"status":"NUMERICAL_FAILURE","reason":e.to_string(),"case_id":self.case_id});
            }
            if let Err(e) = self.jvp_into(p.time, &p.state, &p.direction, &mut jvp) {
                return json!({"status":"NUMERICAL_FAILURE","reason":e.to_string(),"case_id":self.case_id});
            }
            let indices = if n > 512 {
                vec![0, n / 2, n - 1]
            } else {
                (0..n).collect()
            };
            let mut cols = Vec::with_capacity(indices.len());
            let mut basis = vec![0.0; n];
            let mut col = vec![0.0; n];
            for &j in &indices {
                basis[j] = 1.0;
                if let Err(e) = self.jvp_into(p.time, &p.state, &basis, &mut col) {
                    return json!({"status":"NUMERICAL_FAILURE","reason":e.to_string(),"case_id":self.case_id});
                }
                cols.push(col.clone());
                basis[j] = 0.0;
            }
            let jac = if n <= 512 {
                let mut m = vec![vec![0.0; n]; n];
                for (j, c) in cols.iter().enumerate() {
                    for i in 0..n {
                        m[i][j] = c[i];
                    }
                }
                json!(m)
            } else {
                Value::Null
            };
            rows.push(json!({"time":p.time,"state":p.state,"direction":p.direction,"rhs":rhs,"jacobian":jac,"jacobian_columns":if n>512{json!(cols)}else{Value::Null},"jacobian_column_indices":if n>512{json!(indices)}else{Value::Null},"jvp":jvp}));
        }
        json!({"status":"OK","case_id":self.case_id,"probes":rows,"derivative_mode":self.derivative_policy(),"derivative_rhs_calls":self.derivative_rhs_calls(),"rhs_semantics":"original_f_with_algebraic_rows_for_DAE"})
    }
    pub fn problem(&self) -> CoreResult<OdeProblem> {
        let n = self.initial_state.len();
        let rhs_case = self.clone();
        let rhs = Arc::new(move |t, y: &[f64], out: &mut [f64]| rhs_case.rhs(t, y, out));
        // Jacobian columns and JVP use the same analytic derivative lane.
        // The caller-owned matrix and reusable basis/column buffers avoid callback allocations.
        let scratch = Arc::new(Mutex::new((vec![0.0; n], vec![0.0; n])));
        let jc = self.clone();
        let js = Arc::clone(&scratch);
        let fill = Arc::new(move |t: f64, y: &[f64], mat: &mut DenseMatrix| {
            let mut guard = js
                .lock()
                .map_err(|_| CoreError::InvalidInput("Jacobian scratch poisoned".into()))?;
            let (basis, column) = &mut *guard;
            jc.jac_into(t, y, mat, basis, column)
        });
        let jc = self.clone();
        let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
            jc.jvp_into(t, y, v, out)
        });
        let j = self.clone();
        let jac = Arc::new(move |t: f64, y: &[f64]| {
            let mut mat = DenseMatrix::zeros(n, n);
            let mut basis = vec![0.0; n];
            let mut column = vec![0.0; n];
            j.jac_into(t, y, &mut mat, &mut basis, &mut column)?;
            Ok(mat)
        });
        OdeProblem::new(
            self.case_id.clone(),
            n,
            rhs,
            None,
            Some(jac),
            Some(jvp),
            None,
            self.autonomous(),
            None,
            None,
        )
        .map(|p| p.with_jacobian_into(fill))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn case(model: &str, parameters: Value, y: Vec<f64>) -> Case {
        Case {
            case_id: model.into(),
            model: model.into(),
            parameters,
            initial_state: y.clone(),
            t_span: [0.0, 1.0],
            output_times: vec![0.0, 1.0],
            rtol: 1e-8,
            atol: vec![1e-11; y.len()],
            mass_matrix: None,
            algorithm_config: Value::Null,
            derivative_rhs_calls: Arc::new(AtomicU64::new(0)),
            stencil_jacobian_builds: Arc::new(AtomicU64::new(0)),
        }
    }
    #[test]
    fn fourier_complex_equations_at_zero() {
        let c = case(
            "fourier_path",
            serde_json::json!({"omega":40.0,"g":0.125,"eps":0.125,"sigma":1}),
            vec![0.25, 0.0, 0.5, 0.0],
        );
        let mut out = [0.0; 4];
        c.rhs(0.0, &c.initial_state, &mut out).unwrap();
        assert_eq!(out[0], 0.0);
        assert_eq!(out[1], 1.25 * 0.15625 * 0.125);
        assert_eq!(out[2], 0.0);
        assert_eq!(out[3], 0.5 * 1.25 * 0.15625 * 0.0625);
    }
    #[test]
    fn diagonal_fixture_preserves_sparsity() {
        let c = case(
            "diagonal_stiff",
            serde_json::json!({"rates":[-1.0,-100.0,-10000.0]}),
            vec![1.0, 1.0, 1.0],
        );
        let mut out = [0.0; 3];
        c.rhs(0.0, &c.initial_state, &mut out).unwrap();
        assert_eq!(out, [-1.0, -100.0, -10000.0]);
    }
    #[test]
    fn analytic_stencil_matches_basis_jvp_including_periodic_collisions() {
        for n in [1, 2, 3, 8, 128] {
            for model in ["periodic_heat", "advection_diffusion"] {
                let mut c = case(
                    model,
                    json!({"diffusion":0.125,"velocity":-0.25}),
                    vec![0.7; n],
                );
                let mut basis = vec![0.; n];
                let mut column = vec![0.; n];
                let mut reference = DenseMatrix::zeros(n, n);
                c.jac_into(
                    0.,
                    &c.initial_state,
                    &mut reference,
                    &mut basis,
                    &mut column,
                )
                .unwrap();
                assert_eq!(c.derivative_rhs_calls(), n as u64);
                c.algorithm_config = json!({"jacobian_assembly":"analytic_stencil"});
                let mut candidate = DenseMatrix::zeros(n, n);
                c.jac_into(
                    0.,
                    &c.initial_state,
                    &mut candidate,
                    &mut basis,
                    &mut column,
                )
                .unwrap();
                assert_eq!(candidate, reference);
                assert_eq!(c.derivative_rhs_calls(), n as u64);
                assert_eq!(c.stencil_jacobian_builds(), 1);
                // Constant mode remains in the discrete transport nullspace.
                for i in 0..n {
                    assert_eq!((0..n).map(|j| candidate[(i, j)]).sum::<f64>(), 0.);
                }
            }
        }
    }
    #[test]
    fn stencil_refuses_nonfinite_parameters_without_modifying_output() {
        let mut c = case("periodic_heat", json!({"diffusion":1e308}), vec![0.; 4]);
        c.algorithm_config = json!({"jacobian_assembly":"analytic_stencil"});
        let mut out = DenseMatrix::zeros(4, 4);
        out.as_mut_slice().fill(7.);
        assert!(
            c.jac_into(0., &c.initial_state, &mut out, &mut [0.; 4], &mut [0.; 4])
                .is_err()
        );
        assert!(out.as_slice().iter().all(|x| *x == 7.));
        assert_eq!(c.stencil_jacobian_builds(), 0);
    }
    #[test]
    fn analytic_heat_assembly_runs_through_fast_driver_with_same_trajectory() {
        use rodas5p_integrators::{
            AdaptiveStepConfig, OutputSchedule, integrate_rodas5p_fast_observed,
        };
        let n = 8;
        let initial: Vec<f64> = (0..n)
            .map(|i| 1.0 + 0.1 * (std::f64::consts::TAU * i as f64 / n as f64).cos())
            .collect();
        let mut c = case("periodic_heat", json!({"diffusion":0.125}), initial);
        let config = AdaptiveStepConfig {
            rtol: 1e-7,
            atol: 1e-10,
            initial_step: 0.001,
            max_step: 0.1,
            ..Default::default()
        };
        let schedule = OutputSchedule::new(vec![0.0, 0.05, 0.1]).unwrap();
        let base = integrate_rodas5p_fast_observed(
            &c.problem().unwrap(),
            (0.0, 0.1),
            &c.initial_state,
            &config,
            &schedule,
        )
        .unwrap();
        let old_calls = c.derivative_rhs_calls();
        assert!(old_calls > 0);
        c.algorithm_config = json!({"jacobian_assembly":"analytic_stencil"});
        let candidate = integrate_rodas5p_fast_observed(
            &c.problem().unwrap(),
            (0.0, 0.1),
            &c.initial_state,
            &config,
            &schedule,
        )
        .unwrap();
        assert!(base.observed.success && candidate.observed.success);
        assert_eq!(base.observed.t, candidate.observed.t);
        assert_eq!(base.observed.y, candidate.observed.y);
        assert_eq!(base.observed.counters, candidate.observed.counters);
        let lambda =
            -4.0 * 0.125 * (n * n) as f64 * (std::f64::consts::PI / n as f64).sin().powi(2);
        for (time, state) in candidate.observed.t.iter().zip(&candidate.observed.y) {
            for (i, value) in state.iter().enumerate() {
                let exact = 1.0
                    + 0.1
                        * (lambda * time).exp()
                        * (std::f64::consts::TAU * i as f64 / n as f64).cos();
                assert!((value - exact).abs() / (config.atol + config.rtol * exact.abs()) < 1.0);
            }
        }
        assert_eq!(old_calls, c.derivative_rhs_calls());
        assert!(c.stencil_jacobian_builds() > 0);
    }
    #[test]
    fn one_periodic_cell_remains_exact_zero_under_large_advection() {
        let mut c = case(
            "advection_diffusion",
            json!({"diffusion":1.0,"velocity":1e16}),
            vec![3.0],
        );
        c.algorithm_config = json!({"jacobian_assembly":"analytic_stencil"});
        let mut out = DenseMatrix::zeros(1, 1);
        out[(0, 0)] = 7.0;
        c.jac_into(0., &c.initial_state, &mut out, &mut [0.], &mut [0.])
            .unwrap();
        assert_eq!(out[(0, 0)], 0.0);
    }
}
