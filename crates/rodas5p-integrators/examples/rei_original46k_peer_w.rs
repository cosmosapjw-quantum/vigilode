//! Cross-branch Peer-W coefficients + protected low-rank W on full original REI point RHS.
//! Selected v/f closure, full 46,080 cells. Not interval/official source-owner admission.
#![allow(dead_code)]
#![recursion_limit = "256"]
pub use rodas5p_integrators::{OdeProblem, ParallelExecution};
#[path = "rei_peer_support/centered_inputs.rs"]
mod centered;
#[path = "rei_original46k_native_nonphoto.rs"]
mod chemistry;
#[path = "rei_peer_support/ppsw1_0caa3ed2.rs"]
mod peer;
#[path = "rei_peer_support/coupled_rank3.rs"]
mod rank3;
use rodas5p_core::low_rank_bordered::{LowRankApplyWorkspace, LowRankBorderedPreconditioner};
use rodas5p_core::{CoreError, CoreResult};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};
type R<T> = Result<T, String>;
const N: usize = 46080;
const D: usize = 4 * N;
const KB: f64 = 1.380649e-16;
const STATE_SHA: &str = "20b6066eade8a31ecfe46a47081a2e3a7fef7353dba4ac83204456c24ac5f628";
const SPEC_SHA: &str = "0608145d1ab7c61f9b596ad1e407bb93b496df6f8fc2e79d5a33dba1df1fb581";
const ORACLE_SHA: &str = "4e51dd4555c82fa5d13e2837fa86b4d0ceda5a7701c54f7057ca63f9ffa2dc66";
fn err<T>(x: CoreResult<T>) -> R<T> {
    x.map_err(|e| e.to_string())
}
fn core<T>(x: R<T>) -> CoreResult<T> {
    x.map_err(CoreError::InvalidInput)
}
fn sha(x: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(x);
    format!("{:x}", h.finalize())
}
fn sum(x: impl Iterator<Item = f64>) -> f64 {
    let (mut s, mut c) = (0.0, 0.0);
    for z in x {
        let y = z - c;
        let t = s + y;
        c = (t - s) - y;
        s = t;
    }
    s
}
#[derive(Deserialize)]
struct Poly {
    knots: Vec<f64>,
    coefficients: Vec<Vec<f64>>,
}
impl Poly {
    fn at(&self, t: f64) -> R<f64> {
        if self.knots.len() < 2 || t < self.knots[0] || t > *self.knots.last().unwrap() {
            return Err("ORIGINAL_PPOLY_TIME_OUT_OF_DOMAIN".into());
        }
        let i = self
            .knots
            .partition_point(|x| *x <= t)
            .saturating_sub(1)
            .min(self.knots.len() - 2);
        let x = t - self.knots[i];
        let v = self.coefficients[i].iter().fold(0.0, |y, c| y * x + c);
        if !v.is_finite() {
            return Err("ORIGINAL_PPOLY_NONFINITE".into());
        }
        Ok(v)
    }
}
#[derive(Deserialize)]
struct Spec {
    schema: String,
    owner_head: String,
    source_tar_sha256: String,
    kernel_sha256: String,
    accepted_array_sha256: String,
    cells: usize,
    state_sha256: String,
    start_s: f64,
    stop_s: f64,
    duration_s: f64,
    n_h_total: f64,
    n_he_total: f64,
    nh0_cm3: f64,
    #[serde(rename = "YHe")]
    yhe: f64,
    #[serde(rename = "Mpc_cm")]
    mpc_cm: f64,
    #[serde(rename = "H0_s")]
    h0: f64,
    omega_m: f64,
    omega_l: f64,
    #[serde(rename = "eV_erg")]
    ev: f64,
    sigma: Vec<Vec<f64>>,
    support: Vec<Vec<u8>>,
    #[serde(rename = "excess_eV")]
    excess: Vec<Vec<f64>>,
    current: Vec<Poly>,
    external: Vec<Poly>,
    z: Poly,
    f_closure: f64,
}
#[derive(Clone, Copy)]
struct Cell {
    h: f64,
    he: f64,
    volume: f64,
    v: f64,
    table: i32,
}
struct Model {
    spec: Spec,
    cells: Vec<Cell>,
    initial: Vec<f64>,
    rhs_calls: AtomicU64,
    local_calls: AtomicU64,
}
#[derive(Clone, Copy)]
struct Photo {
    k: [f64; 3],
    heat: [f64; 3],
    dk: [[f64; 3]; 3],
    dq: [[f64; 3]; 3],
    hubble: f64,
    volume_factor: f64,
}
fn table(x: f64) -> i32 {
    if x < (1e4_f64).ln() {
        -1
    } else {
        ((x / std::f64::consts::LN_10 - 4.0) * 4.0).floor() as i32
    }
}
fn bytes_f64(b: &[u8], offset: usize) -> R<f64> {
    if offset + 8 > b.len() {
        return Err("INPUT_TRUNCATED".into());
    }
    let x = f64::from_le_bytes(b[offset..offset + 8].try_into().unwrap());
    if !x.is_finite() {
        return Err("INPUT_NONFINITE".into());
    }
    Ok(x)
}
impl Model {
    fn load(spec_path: &Path, source_path: &Path) -> R<Self> {
        let raw = fs::read(spec_path).map_err(|e| e.to_string())?;
        let data = fs::read(source_path).map_err(|e| e.to_string())?;
        if sha(&raw) != SPEC_SHA || sha(&data) != STATE_SHA {
            return Err("SOURCE_SHA_DRIFT".into());
        }
        let spec: Spec = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        if spec.cells != N
            || spec.state_sha256 != STATE_SHA
            || spec.owner_head != "f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa"
            || spec.kernel_sha256
                != "579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644"
            || data.len() != 24 + N * 8 * 8
            || &data[..8] != b"REIPW001"
        {
            return Err("SOURCE_IDENTITY_SCHEMA".into());
        }
        let mut cells = Vec::with_capacity(N);
        let mut initial = Vec::with_capacity(D);
        for i in 0..N {
            let a = (0..8)
                .map(|j| bytes_f64(&data, 24 + 8 * (8 * i + j)))
                .collect::<R<Vec<_>>>()?;
            cells.push(Cell {
                h: a[0],
                he: a[1],
                volume: a[2],
                v: a[7],
                table: table(a[6]),
            });
            initial.extend_from_slice(&a[3..7]);
        }
        Ok(Self {
            spec,
            cells,
            initial,
            rhs_calls: AtomicU64::new(0),
            local_calls: AtomicU64::new(0),
        })
    }
    fn moments(&self, y: &[f64]) -> [f64; 3] {
        [
            sum(self
                .cells
                .iter()
                .enumerate()
                .map(|(i, c)| c.h * y[4 * i] / self.spec.n_h_total)),
            sum(self
                .cells
                .iter()
                .enumerate()
                .map(|(i, c)| c.he * y[4 * i + 1] / self.spec.n_he_total)),
            sum(self.cells.iter().enumerate().map(|(i, c)| {
                c.he * (1.0 - y[4 * i + 1]) * (1.0 - y[4 * i + 2]) / self.spec.n_he_total
            })),
        ]
    }
    fn photo(&self, tau: f64, y: &[f64]) -> R<Photo> {
        if tau < -1e-13 || tau > 1.0 + 1e-13 || !tau.is_finite() {
            return Err("F05_TICK_CLOCK_VIOLATION".into());
        }
        let time = self.spec.start_s + self.spec.duration_s * tau;
        let z = self.spec.z.at(time)?;
        let scale = self.spec.nh0_cm3 * (1.0 + z).powi(2) * self.spec.mpc_cm;
        let totals = [
            self.spec.n_h_total,
            self.spec.n_he_total,
            self.spec.n_he_total,
        ];
        let denom = [
            self.spec.n_h_total,
            self.spec.n_he_total,
            self.spec.n_h_total,
        ];
        let m = self.moments(y);
        let mut out = Photo {
            k: [0.0; 3],
            heat: [0.0; 3],
            dk: [[0.0; 3]; 3],
            dq: [[0.0; 3]; 3],
            hubble: self.spec.h0
                * (self.spec.omega_m * (1.0 + z).powi(3) + self.spec.omega_l).sqrt(),
            volume_factor: (1.0 + z).powi(-3),
        };
        for g in 0..4 {
            let j = self.spec.current[g].at(time)?.max(0.0);
            let e = self.spec.external[g].at(time)?.max(0.0);
            let a: [f64; 3] = std::array::from_fn(|s| {
                if self.spec.support[s + 1][g] == 0 {
                    0.0
                } else {
                    scale * self.spec.sigma[s][g] * (if s == 0 { 1.0 } else { self.spec.yhe })
                        / denom[s]
                }
            });
            let an: [f64; 3] = std::array::from_fn(|s| a[s] * totals[s]);
            let den = e + (0..3).map(|s| an[s] * m[s]).sum::<f64>();
            if den <= 0.0 || !den.is_finite() {
                return Err("PHOTON_DENOM_NONPOSITIVE".into());
            }
            for s in 0..3 {
                let rate = (j / den) * a[s];
                out.k[s] += rate;
                out.heat[s] += rate * self.spec.excess[s][g] * self.spec.ev;
                for q in 0..3 {
                    let d = -rate * an[q] / den;
                    out.dk[s][q] += d;
                    out.dq[s][q] += d * self.spec.excess[s][g] * self.spec.ev;
                }
            }
        }
        Ok(out)
    }
    fn local(&self, i: usize, y: &[f64], p: &Photo) -> R<[f64; 4]> {
        self.local_calls.fetch_add(1, Ordering::Relaxed);
        let c = self.cells[i];
        if y.len() != 4
            || !y.iter().all(|x| x.is_finite())
            || y[..3].iter().any(|x| *x <= 0.0 || *x >= 1.0)
            || table(y[3]) != c.table
        {
            return Err(format!("SOURCE_PHYSICAL_CONE_OR_TABLE_CELL_{i}"));
        }
        let pop = [
            c.h * y[0],
            c.h * (1.0 - y[0]),
            c.he * y[1],
            c.he * (1.0 - y[1]) * (1.0 - y[2]),
            c.he * (1.0 - y[1]) * y[2],
        ];
        let neutral = [pop[0], pop[2], pop[3]];
        let photons = std::array::from_fn(|s| neutral[s] * p.k[s]);
        let heat = (0..3).map(|s| neutral[s] * p.heat[s]).sum();
        let t = y[3].exp();
        let e = chemistry::time_map_event(
            pop,
            t,
            c.volume * p.volume_factor,
            p.hubble,
            c.v,
            self.spec.f_closure,
            photons,
            heat,
        )?;
        let np = c.h + c.he + pop[1] + pop[3] + 2.0 * pop[4];
        let u = 1.5 * KB * np * t;
        let rate = [
            e.net[0] / c.h,
            e.net[2] / c.he,
            (e.net[4] - y[2] * (e.net[3] + e.net[4])) / (c.he * (1.0 - y[1])),
            e.thermal_rhs_erg_s / u - e.electron_rate_s / np,
        ];
        Ok(rate.map(|x| x * self.spec.duration_s))
    }
    fn rhs(&self, t: f64, y: &[f64]) -> R<Vec<f64>> {
        self.rhs_calls.fetch_add(1, Ordering::Relaxed);
        if y.len() != D {
            return Err("FULL_46K_DIMENSION_REQUIRED".into());
        }
        let ph = self.photo(t, y)?;
        let mut out = Vec::with_capacity(D);
        for i in 0..N {
            out.extend_from_slice(&self.local(i, &y[4 * i..4 * i + 4], &ph)?);
        }
        Ok(out)
    }
    fn jac(&self, t: f64, y: &[f64]) -> R<Jac> {
        let ph = self.photo(t, y)?;
        let mut blocks = Vec::with_capacity(N);
        let mut u = vec![[0.0; 3]; D];
        let mut v = vec![[0.0; 3]; D];
        for i in 0..N {
            let c = self.cells[i];
            let a: &[f64] = &y[4 * i..4 * i + 4];
            let mut d = [[0.0; 4]; 4];
            for s in 0..4 {
                let eps = if s == 3 {
                    1e-5
                } else {
                    (1e-5 * a[s].abs().max(1e-4))
                        .min(0.2 * a[s])
                        .min(0.2 * (1.0 - a[s]))
                };
                let mut lo = [a[0], a[1], a[2], a[3]];
                let mut hi = lo;
                lo[s] -= eps;
                hi[s] += eps;
                let fl = self.local(i, &lo, &ph)?;
                let fh = self.local(i, &hi, &ph)?;
                for r in 0..4 {
                    d[r][s] = (fh[r] - fl[r]) / (hi[s] - lo[s]);
                }
            }
            blocks.push(d);
            v[4 * i][0] = c.h / self.spec.n_h_total;
            v[4 * i + 1][1] = c.he / self.spec.n_he_total;
            v[4 * i + 1][2] = -c.he * (1.0 - a[2]) / self.spec.n_he_total;
            v[4 * i + 2][2] = -c.he * (1.0 - a[1]) / self.spec.n_he_total;
            let pop = [
                c.h * a[0],
                c.h * (1.0 - a[0]),
                c.he * a[1],
                c.he * (1.0 - a[1]) * (1.0 - a[2]),
                c.he * (1.0 - a[1]) * a[2],
            ];
            let neutral = [pop[0], pop[2], pop[3]];
            let np = c.h + c.he + pop[1] + pop[3] + 2.0 * pop[4];
            let energy = 1.5 * KB * np * a[3].exp();
            for q in 0..3 {
                let dp: [f64; 3] = std::array::from_fn(|s| neutral[s] * ph.dk[s][q]);
                let dq = (0..3).map(|s| neutral[s] * ph.dq[s][q]).sum::<f64>();
                let vals = [
                    -dp[0] / c.h,
                    -dp[1] / c.he,
                    (dp[2] - a[2] * dp[1]) / (c.he * (1.0 - a[1])),
                    dq / energy - dp.iter().sum::<f64>() / np,
                ];
                for r in 0..4 {
                    u[4 * i + r][q] = self.spec.duration_s * vals[r];
                }
            }
        }
        Ok(Jac { blocks, u, v })
    }
    fn observables(&self, y: &[f64]) -> serde_json::Value {
        let ne = sum(self.cells.iter().enumerate().map(|(i, c)| {
            c.h * (1.0 - y[4 * i]) + c.he * (1.0 - y[4 * i + 1]) * (1.0 + y[4 * i + 2])
        }));
        let energy = sum(self.cells.iter().enumerate().map(|(i, c)| {
            let n = c.h
                + c.he
                + c.h * (1.0 - y[4 * i])
                + c.he * (1.0 - y[4 * i + 1]) * (1.0 + y[4 * i + 2]);
            1.5 * KB * n * y[4 * i + 3].exp()
        }));
        let min = (0..N)
            .flat_map(|i| y[4 * i..4 * i + 3].iter().map(|x| x.min(1.0 - x)))
            .fold(f64::INFINITY, f64::min);
        json!({"electrons":ne,"gas_U_erg":energy,"minimum_fraction_cone_margin":min})
    }
}
struct Jac {
    blocks: Vec<[[f64; 4]; 4]>,
    u: Vec<[f64; 3]>,
    v: Vec<[f64; 3]>,
}
impl Jac {
    fn apply(&self, x: &[f64]) -> Vec<f64> {
        rank3::apply(&self.blocks, &self.u, &self.v, x)
    }
    fn factor(&self, shift: f64) -> R<LowRankBorderedPreconditioner> {
        let n = self.u.len();
        let mut band = vec![0.0; n * 7];
        for (i, b) in self.blocks.iter().enumerate() {
            for r in 0..4 {
                for c in 0..4 {
                    band[(4 * i + r) * 7 + c + 3 - r] = if r == c {
                        1.0 - shift * b[r][c]
                    } else {
                        -shift * b[r][c]
                    };
                }
            }
        }
        let uu = self
            .u
            .iter()
            .flat_map(|r| r.iter().map(|v| -shift * v))
            .collect::<Vec<_>>();
        let vv = self
            .v
            .iter()
            .flat_map(|r| r.iter().copied())
            .collect::<Vec<_>>();
        let mut pre = err(LowRankBorderedPreconditioner::new(n, 0, 3, 3, 3))?;
        err(pre.refactor(n, 0, 3, &band, &[], &[], &[], &uu, &vv))?;
        Ok(pre)
    }
    fn residual(&self, x: &[f64], b: &[f64], shift: f64) -> f64 {
        let jx = self.apply(x);
        let num = x
            .iter()
            .zip(jx)
            .zip(b)
            .map(|((x, j), b)| (x - shift * j - b).abs())
            .fold(0.0, f64::max);
        let den = b.iter().map(|x| x.abs()).fold(1e-300, f64::max);
        num / den
    }
}
fn rk4_step(m: &Model, t: f64, y: &[f64], h: f64) -> R<Vec<f64>> {
    let k1 = m.rhs(t, y)?;
    let a = y
        .iter()
        .zip(&k1)
        .map(|(x, k)| x + h * 0.5 * k)
        .collect::<Vec<_>>();
    let k2 = m.rhs(t + 0.5 * h, &a)?;
    let b = y
        .iter()
        .zip(&k2)
        .map(|(x, k)| x + h * 0.5 * k)
        .collect::<Vec<_>>();
    let k3 = m.rhs(t + 0.5 * h, &b)?;
    let c = y
        .iter()
        .zip(&k3)
        .map(|(x, k)| x + h * k)
        .collect::<Vec<_>>();
    let k4 = m.rhs(t + h, &c)?;
    let out = (0..D)
        .map(|i| y[i] + h * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0)
        .collect();
    Ok(out)
}
fn rk4_path(m: &Model, steps: usize) -> R<Vec<(f64, Vec<f64>)>> {
    let mut y = m.initial.clone();
    let mut path = vec![(0.0, y.clone())];
    let h = 1.0 / steps as f64;
    for k in 0..steps {
        y = rk4_step(m, k as f64 * h, &y, h)?;
        if (k + 1) % (steps / 16).max(1) == 0 {
            path.push(((k + 1) as f64 * h, y.clone()));
        }
    }
    Ok(path)
}
fn seed(m: &Model, h: f64) -> R<peer::State> {
    let mut y = m.initial.clone();
    let mut t = 0.0;
    let mut ys = Vec::new();
    let mut fs = Vec::new();
    for c in peer::nodes() {
        let target = (1.0 + c) * h;
        let steps = ((target - t) / (h / 8.0) - 1e-12).ceil().max(0.0) as usize;
        if steps > 0 {
            let step = (target - t) / steps as f64;
            for j in 0..steps {
                y = rk4_step(m, t + j as f64 * step, &y, step)?;
            }
        }
        t = target;
        ys.push(y.clone());
        fs.push(m.rhs(t, &y)?);
    }
    Ok(peer::State {
        t: 2.0 * h,
        y: ys.try_into().unwrap(),
        f: fs.try_into().unwrap(),
        h_prev: h,
    })
}
fn peer_run(
    m: &Model,
    steps: usize,
    threads: usize,
) -> R<(Vec<(f64, Vec<f64>)>, serde_json::Value)> {
    if steps < 4 {
        return Err("PEER_AT_LEAST_FOUR_STEPS".into());
    }
    let now = Instant::now();
    let centered_mode = match env::var("REI_PEER_FORM").as_deref() {
        Ok("centered") => true,
        Ok("absolute") | Err(_) => false,
        Ok(_) => return Err("REI_PEER_FORM_MUST_BE_absolute_OR_centered".into()),
    };
    let rc0 = m.rhs_calls.load(Ordering::Relaxed);
    let lc0 = m.local_calls.load(Ordering::Relaxed);
    let h = 1.0 / steps as f64;
    let mut state = seed(m, h)?;
    let starter = now.elapsed().as_secs_f64();
    let exec = err(ParallelExecution::rayon(threads))?;
    let mut path = vec![(0.0, m.initial.clone()), (state.t, state.y[5].clone())];
    let mut residual = 0.0_f64;
    let mut facts = 0usize;
    let mut solves = 0usize;
    for _ in 2..steps {
        let inputs = err(if centered_mode {
            centered::build(&state, h)
        } else {
            peer::stage_inputs(&state, h)
        })?;
        let j = m.jac(state.t, &state.y[5])?;
        let shift = h * peer::gamma();
        let pre = j.factor(shift)?;
        facts += 1;
        let rhs = inputs
            .iter()
            .map(|s| {
                let jt = j.apply(&s.transfer);
                s.explicit
                    .iter()
                    .zip(jt)
                    .map(|(e, j)| e + h * j)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let original_rhs = if centered_mode {
            Some(
                err(peer::stage_inputs(&state, h))?
                    .iter()
                    .map(|s| {
                        let jt = j.apply(&s.transfer);
                        s.explicit
                            .iter()
                            .zip(jt)
                            .map(|(a, b)| a + h * b)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        let jobs = err(exec.map_ordered(&[0usize, 1, 2, 3, 4, 5], |&i| {
            let mut ws = LowRankApplyWorkspace::new(D, 0, 3)?;
            let mut y = vec![0.0; D];
            pre.apply_with_workspace(&rhs[i], &mut y, &mut ws)?;
            let mut r = j.residual(&y, &rhs[i], shift);
            if let Some(original) = &original_rhs {
                for (value, anchor) in y.iter_mut().zip(&state.y[5]) {
                    *value += anchor;
                }
                r = r.max(j.residual(&y, &original[i], shift));
            }
            if r > 1e-10 || !r.is_finite() {
                return Err(CoreError::LinearSolve(format!(
                    "ORIGINAL_FULL_W_RESIDUAL_{r:e}"
                )));
            }
            let f = core(m.rhs(inputs[i].t, &y))?;
            Ok((y, f, r))
        }))?;
        solves += 6;
        let mut yy = Vec::new();
        let mut ff = Vec::new();
        for (y, f, r) in jobs {
            residual = residual.max(r);
            yy.push(y);
            ff.push(f);
        }
        state = peer::State {
            t: state.t + h,
            y: yy.try_into().unwrap(),
            f: ff.try_into().unwrap(),
            h_prev: h,
        };
        path.push((state.t, state.y[5].clone()));
    }
    let record = json!({"method":"PPSW1_6b_imported_tableau_plus_original_full_HHe_rank3_W","steps":steps,"threads":threads,"elapsed_including_start_seconds":now.elapsed().as_secs_f64(),"centered_increment_form":centered_mode,"starter_seconds":starter,"starter":"charged_RK4_h_over_8_history_not_generic_dense_Radau","rhs_calls":m.rhs_calls.load(Ordering::Relaxed)-rc0,"local_source_calls":m.local_calls.load(Ordering::Relaxed)-lc0,"W_factorizations":facts,"W_solves":solves,"max_original_W_relative_residual":residual,"endpoint_time_tau":state.t,"observables":m.observables(&state.y[5]),"closure":"source-admissible_point_v_mid_f_0.55","official_tick161":"NOT_ACCEPTED"});
    Ok((path, record))
}
fn errors(path: &[(f64, Vec<f64>)], reference: &[(f64, Vec<f64>)]) -> R<serde_json::Value> {
    let mut maxima = [0.0_f64; 4];
    let mut compared = 0;
    for (t, y) in path {
        let r = reference
            .iter()
            .find(|(q, _)| (q - t).abs() < 1e-12)
            .ok_or_else(|| format!("REFERENCE_TIME_MISSING_{t}"))?;
        for i in 0..N {
            let f = |a: &[f64]| {
                [
                    1.0 - a[0],
                    (1.0 - a[1]) * (1.0 - a[2]),
                    (1.0 - a[1]) * a[2],
                    a[3],
                ]
            };
            let a = f(&y[4 * i..4 * i + 4]);
            let b = f(&r.1[4 * i..4 * i + 4]);
            for k in 0..4 {
                maxima[k] = maxima[k].max((a[k] - b[k]).abs());
            }
        }
        compared += 1;
    }
    Ok(
        json!({"max_abs_public_xHII_xHeII_xHeIII_logT":maxima,"matched_output_epochs":compared,"formal_global_error_certificate":false}),
    )
}
fn save(path: &Path, states: &[(f64, Vec<f64>)]) -> R<String> {
    let mut raw = Vec::new();
    raw.extend_from_slice(b"REIPWOUT");
    raw.extend_from_slice(&(N as u64).to_le_bytes());
    raw.extend_from_slice(&(states.len() as u64).to_le_bytes());
    for (t, y) in states {
        raw.extend_from_slice(&t.to_le_bytes());
        for x in y {
            raw.extend_from_slice(&x.to_le_bytes());
        }
    }
    fs::write(path, &raw).map_err(|e| e.to_string())?;
    Ok(sha(&raw))
}
fn oracle(m: &Model, path: &Path) -> R<serde_json::Value> {
    let raw = fs::read(path).map_err(|e| e.to_string())?;
    if sha(&raw) != ORACLE_SHA || raw.len() != 24 + 3 * D * 8 {
        return Err("ORIGINAL_RHS_ORACLE_DRIFT".into());
    }
    let mut out = Vec::new();
    for (k, t) in [0.0, 0.5, 1.0].iter().enumerate() {
        let r = m.rhs(*t, &m.initial)?;
        let mut abs = 0.0_f64;
        let mut rel = 0.0_f64;
        for j in 0..D {
            let a = bytes_f64(&raw, 24 + 8 * (k * D + j))?;
            abs = abs.max((a - r[j]).abs());
            rel = rel.max((a - r[j]).abs() / a.abs().max(1e-12));
        }
        if rel > 2e-8 {
            return Err(format!(
                "ORIGINAL_FULL_RHS_MISMATCH_tau={t}_rel={rel}_abs={abs}"
            ));
        }
        out.push(json!({"tau":t,"max_abs":abs,"max_scaled_relative":rel,"cells":N}));
    }
    Ok(json!(out))
}
fn jac_probe(m: &Model) -> R<serde_json::Value> {
    let y = &m.initial;
    let dir = (0..D)
        .map(|k| {
            let i = k / 4;
            let s = k % 4;
            let sign = if i % 2 == 0 { 1.0 } else { -0.7 };
            sign * if s == 3 {
                0.01
            } else {
                0.1 * y[k].min(1.0 - y[k])
            }
        })
        .collect::<Vec<_>>();
    let eps = 1e-4;
    let lo = y
        .iter()
        .zip(&dir)
        .map(|(a, d)| a - eps * d)
        .collect::<Vec<_>>();
    let hi = y
        .iter()
        .zip(&dir)
        .map(|(a, d)| a + eps * d)
        .collect::<Vec<_>>();
    let fl = m.rhs(0.5, &lo)?;
    let fh = m.rhs(0.5, &hi)?;
    let j = m.jac(0.5, y)?;
    let jv = j.apply(&dir);
    let num = (0..D)
        .map(|k| (jv[k] - (fh[k] - fl[k]) / (2.0 * eps)).abs())
        .fold(0.0, f64::max);
    let den = jv.iter().map(|x| x.abs()).fold(1e-300, f64::max);
    let rel = num / den;
    if rel > 2e-5 {
        return Err(format!("FULL_COUPLED_JVP_MISMATCH_{rel:e}"));
    }
    Ok(
        json!({"max_absolute":num,"scaled_max_relative":rel,"global_nonlocal_rank":3,"dimension":D,"local_derivative":"central_fd_not_formally_exact"}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = env::args().collect::<Vec<_>>();
    if a.len() != 5 {
        return Err("USAGE: rei_original46k_peer_w SOURCE.json SOURCE_POINT.raw OWNER_RHS_ORACLE.raw OUTPUT_DIR".into());
    }
    let m = Model::load(Path::new(&a[1]), Path::new(&a[2]))?;
    let out = Path::new(&a[4]);
    fs::create_dir_all(out)?;
    let checks = json!({"source_rhs":oracle(&m,Path::new(&a[3]))?,"full_JVP":jac_probe(&m)?});
    fs::write(
        out.join("SOURCE_CHECKS.json"),
        serde_json::to_vec_pretty(&checks)?,
    )?;
    println!("SOURCE_CHECKS_PASS {checks}");
    if env::var("REI_PEER_BENCH").as_deref() == Ok("paired") {
        let mut runs = Vec::new();
        for (rep, threads) in [1usize, 6, 6, 1, 1, 6].into_iter().enumerate() {
            let (path, mut rec) = peer_run(&m, 8, threads)?;
            rec["repetition"] = json!(rep);
            rec["output_sha256"] = json!(save(
                &out.join(format!("repeat_{rep}_p{threads}.raw")),
                &path
            )?);
            println!("PAIRED {rep} {rec}");
            runs.push(rec);
            fs::write(
                out.join("BENCHMARK.json"),
                serde_json::to_vec_pretty(&json!({
               "schema":"REI_FULL_POINT_PEER_ABBAAB_V1","runs":runs,
               "source_sha256":STATE_SHA,"dimension":D,"production":"HOLD",
               "official_tick161":"NOT_ACCEPTED","canonical_delta":0}))?,
            )?;
        }
        return Ok(());
    }
    let mut result = json!({"schema":"VIGILODE_CROSS_BRANCH_ORIGINAL46K_FULL_POINT_PEER_V1","source_state_sha256":STATE_SHA,"source_forcing_sha256":SPEC_SHA,"dimension":D,"cells":N,"peer_coefficient_commit":"0caa3ed21fcbd5c3fdb19f873734d86ffeba5853","runs":[],"production":"HOLD","official_tick161":"NOT_ACCEPTED","native_interval_final14":"NOT_RUN","canonical_delta":0});
    let stamp = Instant::now();
    let ref64 = rk4_path(&m, 64)?;
    println!("RK4_64_COMPLETE {}", stamp.elapsed().as_secs_f64());
    let sec64 = stamp.elapsed().as_secs_f64();
    let stamp = Instant::now();
    let reference = rk4_path(&m, 128)?;
    let sec128 = stamp.elapsed().as_secs_f64();
    result["reference_64_vs128"] = errors(&ref64, &reference)?;
    result["reference64_seconds"] = json!(sec64);
    result["reference128_seconds"] = json!(sec128);
    result["reference128_sha256"] = json!(save(&out.join("RK4_128.raw"), &reference)?);
    result["reference64_sha256"] = json!(save(&out.join("RK4_64.raw"), &ref64)?);
    fs::write(out.join("RESULT.json"), serde_json::to_vec_pretty(&result)?)?;
    for (n, threads) in [(4usize, 1usize), (8, 1), (16, 1), (8, 6)] {
        let name = format!("peer_n{n}_p{threads}");
        let t = Instant::now();
        let entry = match peer_run(&m, n, threads) {
            Ok((path, mut rec)) => {
                rec["error_to_ref128"] = errors(&path, &reference)?;
                rec["output_sha256"] = json!(save(&out.join(format!("{name}.raw")), &path)?);
                rec["status"] = json!("SCOPED_POINT_IVP_EXECUTED");
                rec
            }
            Err(e) => {
                json!({"status":"FAILED_NO_PROMOTION","method":name,"error":e,"elapsed_seconds":t.elapsed().as_secs_f64()})
            }
        };
        println!("RUN {name} {entry}");
        result["runs"].as_array_mut().unwrap().push(entry);
        fs::write(out.join("RESULT.json"), serde_json::to_vec_pretty(&result)?)?;
    }
    println!("COMPLETE_NO_PRODUCTION_PROMOTION");
    Ok(())
}
