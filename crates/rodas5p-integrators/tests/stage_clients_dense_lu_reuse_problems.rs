//! Problems and arms of loop node V18 (dense LU client with reused factor
//! storage), included with `#[path]` by the `stage_clients_dense_lu_reuse_*`
//! test targets and by the measurement program
//! `research/production_loop_20261116/dense_lu_client/runner`.
//!
//! Two dense families on the faer path of the protected policy (order above
//! 64, more than 10 % nonzeros in the Jacobian):
//!
//! * [`meanfield`]: the 1D Brusselator with a mean-field coupling of the
//!   client-kit identity set (`stage_clients_dense_identity.rs`), at any
//!   number of cells. Its right-hand side costs O(n).
//! * [`skewnet`]: a relaxation network with a full skew-symmetric coupling
//!   and a manufactured solution. Its right-hand side costs one dense
//!   matrix-vector product and its `W` is far from diagonally dominant, so
//!   the row pivoting of the factorization does permute rows.
//!
//! Both take an optional hook that is called at the start of every in-place
//! Jacobian evaluation. Every driver evaluates the Jacobian exactly once per
//! accepted state (and once more after an attempt that failed with an
//! error), so the hook sees the accepted trajectory of the protected driver
//! and of the client driver alike.
#![allow(dead_code)]

use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreResult, DenseMatrix};
use rodas5p_integrators::stage_clients::dense_lu_reuse::DenseLuReuseStageClient;
use rodas5p_integrators::stage_clients::{DenseLuMode, DenseLuStageClient};
use rodas5p_integrators::{
    AdaptiveStepConfig, DriverPartial, OdeProblem, OutputSchedule, Rodas5pFastResult,
    StageLinearSystem, integrate_rodas5p_fast_client_observed_with_atol,
    integrate_rodas5p_fast_observed_with_atol,
};

/// Called with `(t, y)` at the start of every in-place Jacobian evaluation.
pub type StepHook = Arc<dyn Fn(f64, &[f64]) + Send + Sync>;

/// One problem instance with its run parameters.
pub struct Cell {
    pub id: String,
    pub family: &'static str,
    pub n: usize,
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub span: (f64, f64),
    /// `atol = rtol * atol_scale`.
    pub atol_scale: f64,
    pub initial_step: f64,
    /// The exact solution at `span.1`, where the family has one.
    pub exact_end: Option<Vec<f64>>,
}

/// The step-size control of a run of `cell` at `rtol` with at most
/// `max_attempts` attempts (the configuration of the client-kit tests,
/// `b1_problems::config`, with the attempt budget explicit).
pub fn adaptive(cell: &Cell, rtol: f64, max_attempts: usize) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * cell.atol_scale,
        rtol,
        initial_step: cell.initial_step,
        min_step: 1.0e-14,
        max_step: cell.span.1 - cell.span.0,
        max_attempts,
        ..AdaptiveStepConfig::default()
    }
}

// ---------------------------------------------------------------------------
// Mean-field Brusselator (client-kit identity set, faer row).
// ---------------------------------------------------------------------------

/// The 1D Brusselator with `cells` cells (order `2 cells`) and a mean-field
/// coupling `kappa (mean(u) - u_i)` on `u` and the same on `v`: the problem
/// of `stage_clients_dense_identity.rs`, with the hook added to the in-place
/// Jacobian. The `u-u` and `v-v` blocks of the Jacobian are full.
pub fn meanfield_problem(
    cells: usize,
    kappa: f64,
    hook: Option<StepHook>,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let mean = kappa / cells as f64;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        let (mut ubar, mut vbar) = (0.0, 0.0);
        for i in 0..cells {
            ubar += y[2 * i];
            vbar += y[2 * i + 1];
        }
        ubar /= cells as f64;
        vbar /= cells as f64;
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (ul, vl) = if i == 0 {
                (1.0, 3.0)
            } else {
                (y[2 * i - 2], y[2 * i - 1])
            };
            let (ur, vr) = if i + 1 == cells {
                (1.0, 3.0)
            } else {
                (y[2 * i + 2], y[2 * i + 3])
            };
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur) + kappa * (ubar - u);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr) + kappa * (vbar - v);
        }
        Ok(())
    });
    let n = 2 * cells;
    let fill = move |y: &[f64], j: &mut DenseMatrix| {
        for i in 0..cells {
            for k in 0..cells {
                j[(2 * i, 2 * k)] = mean;
                j[(2 * i + 1, 2 * k + 1)] = mean;
            }
        }
        for i in 0..cells {
            let (u, v) = (y[2 * i], y[2 * i + 1]);
            let (a, b) = (2 * i, 2 * i + 1);
            j[(a, a)] = mean + 2.0 * u * v - 4.0 - 2.0 * c - kappa;
            j[(a, b)] = u * u;
            j[(b, a)] = 3.0 - 2.0 * u * v;
            j[(b, b)] = mean - u * u - 2.0 * c - kappa;
            if i > 0 {
                j[(a, a - 2)] = mean + c;
                j[(b, b - 2)] = mean + c;
            }
            if i + 1 < cells {
                j[(a, a + 2)] = mean + c;
                j[(b, b + 2)] = mean + c;
            }
        }
    };
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        fill(y, &mut j);
        Ok(j)
    });
    let jacobian_into = Arc::new(move |t: f64, y: &[f64], j: &mut DenseMatrix| {
        if let Some(hook) = &hook {
            hook(t, y);
        }
        fill(y, j);
        Ok(())
    });
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * PI * x).sin(), 3.0]
        })
        .collect();
    Ok((
        OdeProblem::new(
            format!("brusselator-1d-{cells}-meanfield"),
            n,
            rhs,
            None,
            Some(jacobian),
            None,
            None,
            true,
            None,
            None,
        )?
        .with_jacobian_into(jacobian_into),
        y0,
    ))
}

/// The mean-field Brusselator of order `n` (even) on `[0, t_end]` with the
/// coupling 0.1 of the client-kit case.
pub fn meanfield(n: usize, t_end: f64, hook: Option<StepHook>) -> Cell {
    assert!(n >= 4 && n.is_multiple_of(2), "an even order of at least 4");
    let (problem, y0) = meanfield_problem(n / 2, 0.1, hook).unwrap();
    Cell {
        id: format!("meanfield.n{n}"),
        family: "meanfield",
        n,
        problem,
        y0,
        span: (0.0, t_end),
        atol_scale: 1.0,
        initial_step: 1.0e-6,
        exact_end: None,
    }
}

// ---------------------------------------------------------------------------
// skewnet: skew-coupled relaxation network with a manufactured solution.
//
//   y_i' = c_i [ s (S tanh(y))_i - y_i ] - c_i [ s (alpha(t) (S p)_i + beta(t) (S q)_i) - phi_i(t) ]
//          + phi_i'(t),                                   s = 1 / sqrt(n),
//
// S skew-symmetric with entries in [-1, 1) from a 64-bit mixer, rates
// c_i = 10^(4 frac(i g)) over four decades (g the golden ratio conjugate).
// With phi_j(t) = atanh(alpha(t) p_j + beta(t) q_j) the second bracket is
// F(phi(t)), so y = phi is the solution from y(0) = phi(0).
//
// The Jacobian c_i [ s S_ij sech^2(y_j) - delta_ij ] is a positive diagonal
// times a skew matrix times a positive diagonal, minus a positive diagonal:
// similar to (skew - positive diagonal), so every eigenvalue has a real part
// in [-max c, -min c] at every state. Its off-diagonal part is not small
// against the diagonal of W = I/(h gamma) - J.
// ---------------------------------------------------------------------------

pub const SKEWNET_ALPHA_AMPLITUDE: f64 = 0.6;
pub const SKEWNET_ALPHA_FREQUENCY: f64 = 3.0;
pub const SKEWNET_ALPHA_PHASE: f64 = 0.5;
pub const SKEWNET_BETA_AMPLITUDE: f64 = 0.3;
pub const SKEWNET_BETA_FREQUENCY: f64 = 5.0;
const GOLDEN: f64 = 0.618_033_988_749_894_9;
const P_STRIDE: f64 = 0.754_877_666_246_692_7;
const Q_STRIDE: f64 = 0.569_840_290_998_053_2;

/// The fixed data of a skewnet instance.
pub struct SkewnetData {
    pub n: usize,
    /// Row-major skew-symmetric coupling.
    s: Vec<f64>,
    /// Rates.
    c: Vec<f64>,
    /// The spatial profiles of the manufactured solution.
    pub p: Vec<f64>,
    pub q: Vec<f64>,
    /// `S p` and `S q`.
    sp: Vec<f64>,
    sq: Vec<f64>,
    scale: f64,
}

/// A value in `[-1, 1)` from the pair `(i, j)` (splitmix64 finalizer).
fn mixed(i: usize, j: usize) -> f64 {
    let mut z = (((i as u64) << 32) | j as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z >> 11) as f64 * (2.0 / 9_007_199_254_740_992.0) - 1.0
}

/// A dot product with four accumulators in a fixed order (deterministic).
fn dot4(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = [0.0_f64; 4];
    let (ca, cb) = (a.chunks_exact(4), b.chunks_exact(4));
    let (ra, rb) = (ca.remainder(), cb.remainder());
    for (x, y) in ca.zip(cb) {
        acc[0] += x[0] * y[0];
        acc[1] += x[1] * y[1];
        acc[2] += x[2] * y[2];
        acc[3] += x[3] * y[3];
    }
    let mut tail = 0.0;
    for (x, y) in ra.iter().zip(rb) {
        tail += x * y;
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3]) + tail
}

/// `alpha`, `beta` and their first and second derivatives at `t`.
fn modes(t: f64) -> ([f64; 3], [f64; 3]) {
    let (a, w, ph) = (
        SKEWNET_ALPHA_AMPLITUDE,
        SKEWNET_ALPHA_FREQUENCY,
        SKEWNET_ALPHA_PHASE,
    );
    let (b, v) = (SKEWNET_BETA_AMPLITUDE, SKEWNET_BETA_FREQUENCY);
    let (sa, ca) = (w * t + ph).sin_cos();
    let (sb, cb) = (v * t).sin_cos();
    (
        [a * sa, a * w * ca, -a * w * w * sa],
        [b * cb, -b * v * sb, -b * v * v * cb],
    )
}

impl SkewnetData {
    pub fn new(n: usize) -> Self {
        assert!(n >= 2, "skewnet needs at least two unknowns");
        let mut s = vec![0.0; n * n];
        for i in 0..n {
            for j in i + 1..n {
                let value = mixed(i, j);
                s[i * n + j] = value;
                s[j * n + i] = -value;
            }
        }
        let c = (0..n)
            .map(|i| 10.0_f64.powf(4.0 * (i as f64 * GOLDEN).fract()))
            .collect();
        let profile = |stride: f64| -> Vec<f64> {
            (0..n)
                .map(|i| (2.0 * PI * (i as f64 * stride).fract()).cos())
                .collect()
        };
        let (p, q) = (profile(P_STRIDE), profile(Q_STRIDE));
        let times =
            |x: &[f64]| -> Vec<f64> { (0..n).map(|i| dot4(&s[i * n..(i + 1) * n], x)).collect() };
        let (sp, sq) = (times(&p), times(&q));
        Self {
            n,
            s,
            c,
            p,
            q,
            sp,
            sq,
            scale: 1.0 / (n as f64).sqrt(),
        }
    }

    /// The manufactured solution `phi(t)`.
    pub fn exact(&self, t: f64) -> Vec<f64> {
        let (alpha, beta) = modes(t);
        self.p
            .iter()
            .zip(&self.q)
            .map(|(p, q)| (alpha[0] * p + beta[0] * q).atanh())
            .collect()
    }
}

/// The skewnet problem of `data` with the optional Jacobian hook.
pub fn skewnet_problem(data: &Arc<SkewnetData>, hook: Option<StepHook>) -> CoreResult<OdeProblem> {
    let n = data.n;
    let scratch_rhs = Mutex::new(vec![0.0; n]);
    let net = Arc::clone(data);
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let mut th = scratch_rhs.lock().unwrap_or_else(|e| e.into_inner());
        for (h, v) in th.iter_mut().zip(y) {
            *h = v.tanh();
        }
        let (alpha, beta) = modes(t);
        for i in 0..n {
            let z = alpha[0] * net.p[i] + beta[0] * net.q[i];
            let dz = alpha[1] * net.p[i] + beta[1] * net.q[i];
            let phi = z.atanh();
            let dphi = dz / (1.0 - z * z);
            let coupling =
                dot4(&net.s[i * n..(i + 1) * n], &th) - alpha[0] * net.sp[i] - beta[0] * net.sq[i];
            out[i] = net.c[i] * (net.scale * coupling - (y[i] - phi)) + dphi;
        }
        Ok(())
    });
    let net = Arc::clone(data);
    let fill = move |y: &[f64], j: &mut DenseMatrix, weights: &mut [f64]| {
        for (w, v) in weights.iter_mut().zip(y) {
            let th = v.tanh();
            *w = net.scale * (1.0 - th * th);
        }
        let entries = j.as_mut_slice();
        for i in 0..n {
            let ci = net.c[i];
            let row = &mut entries[i * n..(i + 1) * n];
            for ((entry, sij), w) in row
                .iter_mut()
                .zip(&net.s[i * n..(i + 1) * n])
                .zip(&*weights)
            {
                *entry = ci * sij * w;
            }
            row[i] = -ci;
        }
    };
    let fill = Arc::new(fill);
    let fill_owned = Arc::clone(&fill);
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        fill_owned(y, &mut j, &mut vec![0.0; n]);
        Ok(j)
    });
    let scratch_jac = Mutex::new(vec![0.0; n]);
    let jacobian_into = Arc::new(move |t: f64, y: &[f64], j: &mut DenseMatrix| {
        if let Some(hook) = &hook {
            hook(t, y);
        }
        let mut weights = scratch_jac.lock().unwrap_or_else(|e| e.into_inner());
        fill(y, j, &mut weights);
        Ok(())
    });
    let net = Arc::clone(data);
    let partial_t = Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
        let (alpha, beta) = modes(t);
        for i in 0..n {
            let z = alpha[0] * net.p[i] + beta[0] * net.q[i];
            let dz = alpha[1] * net.p[i] + beta[1] * net.q[i];
            let ddz = alpha[2] * net.p[i] + beta[2] * net.q[i];
            let g = 1.0 / (1.0 - z * z);
            let dphi = dz * g;
            let ddphi = ddz * g + 2.0 * z * dz * dz * g * g;
            let forcing = net.scale * (alpha[1] * net.sp[i] + beta[1] * net.sq[i]);
            out[i] = -net.c[i] * (forcing - dphi) + ddphi;
        }
        Ok(())
    });
    Ok(OdeProblem::new(
        format!("skewnet-{n}"),
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        Some(partial_t),
        false,
        None,
        None,
    )?
    .with_jacobian_into(jacobian_into))
}

/// The skewnet cell of `data` on `[0, t_end]`.
pub fn skewnet(data: &Arc<SkewnetData>, t_end: f64, hook: Option<StepHook>) -> Cell {
    let n = data.n;
    Cell {
        id: format!("skewnet.n{n}"),
        family: "skewnet",
        n,
        problem: skewnet_problem(data, hook).unwrap(),
        y0: data.exact(0.0),
        span: (0.0, t_end),
        atol_scale: 1.0,
        initial_step: 1.0e-6,
        exact_end: Some(data.exact(t_end)),
    }
}

// ---------------------------------------------------------------------------
// Arms.
// ---------------------------------------------------------------------------

/// One way to run a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    /// The protected dense driver.
    Protected,
    /// The client-kit client in the protected policy's mode.
    Kit,
    /// The reuse client in the protected policy's mode, sequential.
    Reuse,
    /// The reuse client with the factorization on this many workers.
    Threaded(usize),
}

impl Arm {
    pub fn name(self) -> String {
        match self {
            Self::Protected => "protected".into(),
            Self::Kit => "kit".into(),
            Self::Reuse => "reuse".into(),
            Self::Threaded(workers) => format!("threaded{workers}"),
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "protected" => Some(Self::Protected),
            "kit" => Some(Self::Kit),
            "reuse" => Some(Self::Reuse),
            _ => name
                .strip_prefix("threaded")?
                .parse()
                .ok()
                .map(Self::Threaded),
        }
    }
}

/// The result of one run of an arm.
pub struct ArmRun {
    pub result: Rodas5pFastResult,
    pub partial: Option<DriverPartial>,
    /// Rows moved by the last factorization of a reuse client in the faer
    /// mode (`None` for the other arms).
    pub permuted_rows: Option<usize>,
}

impl ArmRun {
    /// The state the run ended at: the last output of a complete run, the
    /// last accepted state of a run that stopped early.
    pub fn end(&self) -> (f64, &[f64]) {
        match &self.partial {
            Some(partial) => (partial.t, &partial.y),
            None => (
                *self.result.observed.t.last().expect("an output time"),
                self.result.observed.y.last().expect("an output state"),
            ),
        }
    }
}

/// One integration of `problem` (a problem of `cell`, with or without a
/// hook) by `arm` in the LU `mode`, with the output schedule `[t0, tf]`.
/// The client of a client arm is built and dropped inside this call, as the
/// protected driver builds and drops its workspace inside its call.
pub fn run_arm(
    cell: &Cell,
    problem: &OdeProblem,
    arm: Arm,
    mode: DenseLuMode,
    config: &AdaptiveStepConfig,
) -> CoreResult<ArmRun> {
    let n = cell.n;
    let output = OutputSchedule::new(vec![cell.span.0, cell.span.1])?;
    let atol = vec![config.atol; n];
    let client: Box<dyn StageLinearSystem> = match arm {
        Arm::Protected => {
            let (result, partial) = integrate_rodas5p_fast_observed_with_atol(
                problem, cell.span, &cell.y0, config, &output, &atol,
            )?;
            return Ok(ArmRun {
                result,
                partial,
                permuted_rows: None,
            });
        }
        Arm::Kit => Box::new(DenseLuStageClient::new(n, mode)?),
        Arm::Reuse => Box::new(ReuseProbe::new(DenseLuReuseStageClient::new(n, mode)?)),
        Arm::Threaded(workers) => Box::new(ReuseProbe::new(
            DenseLuReuseStageClient::new(n, mode)?.with_factor_workers(workers)?,
        )),
    };
    let (result, partial, client) = integrate_rodas5p_fast_client_observed_with_atol(
        problem, client, cell.span, &cell.y0, config, &output, &atol,
    )?;
    // The probe of a reuse client reports when it is dropped.
    PERMUTED_ROWS.with(|rows| rows.set(None));
    drop(client);
    let permuted_rows = PERMUTED_ROWS.with(|rows| rows.take());
    Ok(ArmRun {
        result,
        partial,
        permuted_rows,
    })
}

thread_local! {
    /// The rows moved by the last factorization of the last [`ReuseProbe`]
    /// dropped on this thread.
    static PERMUTED_ROWS: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// A [`DenseLuReuseStageClient`] that reports, when dropped, how many rows
/// its last factorization moved. It forwards every call unchanged.
struct ReuseProbe {
    inner: DenseLuReuseStageClient,
}

impl ReuseProbe {
    fn new(inner: DenseLuReuseStageClient) -> Self {
        Self { inner }
    }
}

impl Drop for ReuseProbe {
    fn drop(&mut self) {
        let moved = self.inner.row_permutation().map(|rows| {
            rows.iter()
                .enumerate()
                .filter(|(position, row)| position != *row)
                .count()
        });
        PERMUTED_ROWS.with(|rows| rows.set(moved));
    }
}

impl StageLinearSystem for ReuseProbe {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut rodas5p_core::WorkCounters,
    ) -> CoreResult<()> {
        self.inner.linearize(problem, t, y, counters)
    }

    fn factor(
        &mut self,
        inv_h_gamma: f64,
        counters: &mut rodas5p_core::WorkCounters,
    ) -> CoreResult<()> {
        self.inner.factor(inv_h_gamma, counters)
    }

    fn solve_in_place(
        &mut self,
        b: &mut [f64],
        counters: &mut rodas5p_core::WorkCounters,
    ) -> CoreResult<()> {
        self.inner.solve_in_place(b, counters)
    }
}

// ---------------------------------------------------------------------------
// Fingerprints.
// ---------------------------------------------------------------------------

/// FNV-1a (64 bit) over 64-bit words.
#[derive(Clone, Copy)]
pub struct Fnv(pub u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    pub fn eat(&mut self, word: u64) {
        for byte in word.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn eat_point(&mut self, t: f64, y: &[f64]) {
        self.eat(t.to_bits());
        for value in y {
            self.eat(value.to_bits());
        }
    }

    pub fn hex(self) -> String {
        format!("{:016x}", self.0)
    }
}

/// The fingerprint of one point.
pub fn fingerprint(t: f64, y: &[f64]) -> String {
    let mut hash = Fnv::default();
    hash.eat_point(t, y);
    hash.hex()
}

/// The accepted states a hook has seen: their count and the fingerprint of
/// the sequence.
#[derive(Default)]
pub struct Trace {
    pub states: usize,
    pub hash: Fnv,
}

/// A hook that folds every accepted state into the returned trace.
pub fn tracing_hook() -> (StepHook, Arc<Mutex<Trace>>) {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let sink = Arc::clone(&trace);
    let hook: StepHook = Arc::new(move |t: f64, y: &[f64]| {
        let mut trace = sink.lock().unwrap_or_else(|e| e.into_inner());
        trace.states += 1;
        trace.hash.eat_point(t, y);
    });
    (hook, trace)
}

/// The largest absolute difference of two states.
pub fn max_abs_difference(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}
