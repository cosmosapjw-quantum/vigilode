//! Problems and run helpers of loop node V08 (finite-difference Jacobian
//! assembly), included by the `stage_clients_assembly_*` test targets with
//! `#[path]`.
//!
//! The declared instances are those of
//! `research/production_loop_20261116/jacobian_assembly/PREDECLARED.json`
//! ([`declared`]); the generators also build the small instances of the
//! contract tests. Every problem is autonomous and has no usable Jacobian
//! callback: the one it carries fails, so a client that called it would be
//! noticed.
//!
//! The reference finite differences ([`reference_dense_fd`],
//! [`reference_banded_fd`]) are a second transcription of
//! `crates/vigilode/src/fd.rs:443-455`, `:476-509` and `:581-622`, used as
//! the Jacobian callbacks of the protected drivers.
#![allow(dead_code)]

use std::f64::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::stage_clients::assembly::{
    AssemblyOptions, AssemblyReportHandle, AssemblyStageClient, AssemblyStats, CheckColumns,
    ColumnRoute, FINGERPRINT_SEED, LinearBackend, RhsPurity, SparsityPattern, fingerprint,
};
use rodas5p_integrators::stage_clients::{PhaseTimes, TimedStageClient};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, DriverPartial, OdeProblem, OutputSchedule,
    Rodas5pFastResult, StageLinearSystem, integrate_rodas5p_fast_banded_observed_with_atol,
    integrate_rodas5p_fast_client_observed_with_atol, integrate_rodas5p_fast_observed_with_atol,
};

/// A right-hand side callback.
pub type Rhs = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// A problem whose Jacobian is built by finite differences.
pub struct FdProblem {
    pub id: String,
    pub n: usize,
    /// Calls of the right-hand side, when the problem was built to count.
    pub calls: Arc<AtomicU64>,
    pub rhs: Rhs,
    /// The right-hand side with a Jacobian callback that fails.
    pub problem: OdeProblem,
    pub y0: Vec<f64>,
    pub span: (f64, f64),
    /// The declared full band `(lower, upper)`, for banded problems.
    pub band: Option<(usize, usize)>,
}

fn finish(
    id: String,
    raw: impl Fn(&[f64], &mut [f64]) + Send + Sync + 'static,
    y0: Vec<f64>,
    span: (f64, f64),
    band: Option<(usize, usize)>,
    counted: bool,
) -> FdProblem {
    let calls = Arc::new(AtomicU64::new(0));
    let counter = Arc::clone(&calls);
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            if counted {
                counter.fetch_add(1, Ordering::Relaxed);
            }
            raw(y, out);
            Ok(())
        },
    );
    FdProblem {
        calls,
        ..from_rhs(id, rhs, y0, span, band)
    }
}

/// A problem from any right-hand side callback (one that fails or is not a
/// function of its input, for the negative controls).
pub fn from_rhs(
    id: String,
    rhs: Rhs,
    y0: Vec<f64>,
    span: (f64, f64),
    band: Option<(usize, usize)>,
) -> FdProblem {
    let n = y0.len();
    let problem = problem_of(&id, n, &rhs);
    FdProblem {
        id,
        n,
        calls: Arc::new(AtomicU64::new(0)),
        rhs,
        problem,
        y0,
        span,
        band,
    }
}

/// `rhs` as an autonomous problem whose Jacobian callback fails.
pub fn problem_of(id: &str, n: usize, rhs: &Rhs) -> OdeProblem {
    OdeProblem::new(
        id.to_string(),
        n,
        Arc::clone(rhs),
        None,
        Some(Arc::new(|_t: f64, _y: &[f64]| -> CoreResult<DenseMatrix> {
            Err(CoreError::InvalidInput(
                "this problem has no Jacobian callback: finite differences only".into(),
            ))
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

const NLKPP_H: f64 = 0.1;

fn nlkpp_y0(n: usize, ripple: bool) -> Vec<f64> {
    let length = n as f64 * NLKPP_H;
    (0..n)
        .map(|i| {
            let x = (i as f64 + 0.5) * NLKPP_H;
            let bump = 0.1 + 0.8 * (-(x - 0.5 * length).powi(2) / 20.0).exp();
            if ripple {
                bump + 0.05 * (2.0 * PI * x / 12.8).sin()
            } else {
                bump
            }
        })
        .collect()
}

/// Nonlocal Fisher-KPP on `n` cells of width 0.1 with zero-flux boundaries
/// and the Laplace competition kernel `exp(-x / 5) / 10` over the whole
/// domain: `du_i/dt = (u_{i-1} - 2 u_i + u_{i+1}) / h^2 + u_i (1 - h sum_j phi(|i - j| h) u_j)`.
/// Every Jacobian entry is nonzero; a right-hand side costs `n^2`
/// multiply-adds.
pub fn nlkpp_dense(n: usize, counted: bool) -> FdProblem {
    let h = NLKPP_H;
    let sigma = 5.0;
    let phi: Vec<f64> = (0..n)
        .map(|d| (-(d as f64 * h) / sigma).exp() / (2.0 * sigma))
        .collect();
    let raw = move |u: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let mut conv = 0.0;
            for (j, uj) in u[..i].iter().enumerate() {
                conv += phi[i - j] * uj;
            }
            for (d, uj) in u[i..].iter().enumerate() {
                conv += phi[d] * uj;
            }
            let left = if i == 0 { u[0] } else { u[i - 1] };
            let right = if i + 1 == n { u[n - 1] } else { u[i + 1] };
            out[i] = (left - 2.0 * u[i] + right) / (h * h) + u[i] * (1.0 - h * conv);
        }
    };
    finish(
        format!("nlkpp-dense-n{n}"),
        raw,
        nlkpp_y0(n, false),
        (0.0, 10.0),
        None,
        counted,
    )
}

/// The same equation with the compact kernel `c (1 - (x / rho)^2)`,
/// `rho = (radius + 1/2) h`, normalised to unit discrete mass: the sum runs
/// over `|i - j| <= radius` inside the domain. Band `(radius, radius)`.
pub fn nlkpp_band(n: usize, radius: usize, counted: bool) -> FdProblem {
    let h = NLKPP_H;
    let rho = (radius as f64 + 0.5) * h;
    let shape: Vec<f64> = (0..=radius)
        .map(|d| 1.0 - (d as f64 * h / rho).powi(2))
        .collect();
    let mass = h * (shape[0] + 2.0 * shape[1..].iter().sum::<f64>());
    let phi: Vec<f64> = shape.iter().map(|w| w / mass).collect();
    let raw = move |u: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let lo = i.saturating_sub(radius);
            let hi = (i + radius).min(n - 1);
            let mut conv = 0.0;
            for (j, uj) in u[lo..i].iter().enumerate() {
                conv += phi[i - lo - j] * uj;
            }
            for (d, uj) in u[i..=hi].iter().enumerate() {
                conv += phi[d] * uj;
            }
            let left = if i == 0 { u[0] } else { u[i - 1] };
            let right = if i + 1 == n { u[n - 1] } else { u[i + 1] };
            out[i] = (left - 2.0 * u[i] + right) / (h * h) + u[i] * (1.0 - h * conv);
        }
    };
    finish(
        format!("nlkpp-band-n{n}-r{radius}"),
        raw,
        nlkpp_y0(n, true),
        (0.0, 10.0),
        Some((radius, radius)),
        counted,
    )
}

/// The 1D Brusselator of `b1_problems/mod.rs` (same right-hand side and
/// start), without its Jacobian. Band `(2, 2)`.
pub fn brusselator(cells: usize, counted: bool) -> FdProblem {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    let raw = move |y: &[f64], out: &mut [f64]| {
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
            out[2 * i] = 1.0 + u * u * v - 4.0 * u + c * (ul - 2.0 * u + ur);
            out[2 * i + 1] = 3.0 * u - u * u * v + c * (vl - 2.0 * v + vr);
        }
    };
    let y0 = (0..cells)
        .flat_map(|i| {
            let x = (i as f64 + 1.0) / (cells as f64 + 1.0);
            [1.0 + (2.0 * PI * x).sin(), 3.0]
        })
        .collect();
    finish(
        format!("bruss1d-n{}", 2 * cells),
        raw,
        y0,
        (0.0, 10.0),
        Some((2, 2)),
        counted,
    )
}

/// A ring of `n` cells: `dy_i/dt = -y_i^3 + sin(y_{i+1}) - y_{i-1} y_i / 2`
/// with cyclic neighbours. Its pattern is tridiagonal plus the two corner
/// entries: sparse and not banded. With `reach2` component `i` also reads
/// `y_{i-2}` (cyclic), through `cos(y_{i-2}) / 4`.
pub fn ring(n: usize, reach2: bool, counted: bool) -> FdProblem {
    let raw = move |y: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let prev = y[(i + n - 1) % n];
            let next = y[(i + 1) % n];
            out[i] = -y[i].powi(3) + next.sin() - 0.5 * prev * y[i];
            if reach2 {
                out[i] += 0.25 * y[(i + n - 2) % n].cos();
            }
        }
    };
    let y0 = (0..n).map(|i| 0.5 + 0.3 * (0.7 * i as f64).sin()).collect();
    finish(
        format!("ring-n{n}{}", if reach2 { "-reach2" } else { "" }),
        raw,
        y0,
        (0.0, 5.0),
        None,
        counted,
    )
}

/// The true pattern of [`ring`] without `reach2`.
pub fn ring_pattern(n: usize) -> SparsityPattern {
    let rows: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let mut row = vec![(i + n - 1) % n, i, (i + 1) % n];
            row.sort_unstable();
            row.dedup();
            row
        })
        .collect();
    SparsityPattern::from_rows(n, &rows).unwrap()
}

/// A chain whose component `i` reads `y_{i-1}`, `y_i` and `y_{i+1}`; rows
/// `1 ..= n - 2` only read `y_{i-1}`, so that a pattern without the lower
/// diagonal leaves no row uncovered.
pub fn lower_coupled_chain(n: usize) -> FdProblem {
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            for i in 0..n {
                let next = if i + 1 < n { y[i + 1] } else { 0.0 };
                out[i] = -y[i] * y[i] + 0.5 * next.sin();
                if i >= 1 && i + 1 < n {
                    out[i] += 0.3 * y[i - 1] * y[i];
                }
            }
            Ok(())
        },
    );
    let y0 = (0..n).map(|i| 0.4 + 0.2 * (0.6 * i as f64).cos()).collect();
    from_rhs(
        format!("lower-coupled-chain-n{n}"),
        rhs,
        y0,
        (0.0, 1.0),
        None,
    )
}

/// A pattern with the diagonal and the first upper diagonal only.
pub fn upper_bidiagonal(n: usize) -> SparsityPattern {
    SparsityPattern::banded(n, 0, 1).unwrap()
}

/// A right-hand side that is not a function of its input: the result
/// carries the number of the call.
pub fn impure(n: usize) -> FdProblem {
    let calls = AtomicU64::new(0);
    let rhs: Rhs = Arc::new(
        move |_t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            let call = calls.fetch_add(1, Ordering::Relaxed);
            for i in 0..n {
                out[i] = -y[i] + 1.0e-6 * call as f64;
            }
            Ok(())
        },
    );
    from_rhs(format!("impure-n{n}"), rhs, vec![1.0; n], (0.0, 1.0), None)
}

/// The declared problems of `PREDECLARED.json`, by id.
pub fn declared(id: &str, counted: bool) -> Option<FdProblem> {
    let problem = match id {
        "nlkpp-dense-n400" => nlkpp_dense(400, counted),
        "nlkpp-band-n4096-r24" => nlkpp_band(4096, 24, counted),
        "bruss1d-n131072" => brusselator(65536, counted),
        "bruss1d-n128-dense" | "bruss1d-n128-coloured" => brusselator(64, counted),
        _ => return None,
    };
    Some(FdProblem {
        id: id.to_string(),
        ..problem
    })
}

/// The forward-difference increment, from `fd.rs:443-455`.
fn reference_increment(yj: f64) -> (f64, f64) {
    let raw = f64::EPSILON.sqrt() * yj.abs().max(1.0);
    let forward = yj + raw;
    let shifted = if forward.is_finite() {
        forward
    } else {
        yj - raw
    };
    (shifted, shifted - yj)
}

fn eval(problem: &OdeProblem, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
    problem.eval_rhs_into(t, y, out, &mut WorkCounters::default())
}

/// Dense forward differences, from `fd.rs:476-509`.
pub fn reference_dense_fd(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    matrix: &mut DenseMatrix,
) -> CoreResult<()> {
    let n = y.len();
    matrix.as_mut_slice().fill(0.0);
    let mut f0 = vec![0.0; n];
    eval(problem, t, y, &mut f0)?;
    let mut shifted = y.to_vec();
    let mut f1 = vec![0.0; n];
    for j in 0..n {
        let (yj_shifted, delta) = reference_increment(y[j]);
        shifted[j] = yj_shifted;
        f1.fill(0.0);
        let outcome = eval(problem, t, &shifted, &mut f1);
        shifted[j] = y[j];
        outcome?;
        for i in 0..n {
            let entry = (f1[i] - f0[i]) / delta;
            if !entry.is_finite() {
                return Err(CoreError::NonFinite(
                    "finite-difference Jacobian entry overflowed".into(),
                ));
            }
            matrix[(i, j)] = entry;
        }
    }
    Ok(())
}

/// Grouped forward differences of a band, from `fd.rs:581-622`; `band` is
/// zero on entry (the banded driver zeroes it).
pub fn reference_banded_fd(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    band: &mut [f64],
    lower: usize,
    upper: usize,
) -> CoreResult<()> {
    let n = y.len();
    let width = lower + upper + 1;
    let groups = width.min(n);
    let mut f0 = vec![0.0; n];
    eval(problem, t, y, &mut f0)?;
    let mut shifted = y.to_vec();
    let mut f1 = vec![0.0; n];
    let mut deltas = vec![0.0; n];
    for group in 0..groups {
        for j in (group..n).step_by(width) {
            let (yj_shifted, delta) = reference_increment(y[j]);
            shifted[j] = yj_shifted;
            deltas[j] = delta;
        }
        f1.fill(0.0);
        let outcome = eval(problem, t, &shifted, &mut f1);
        for j in (group..n).step_by(width) {
            shifted[j] = y[j];
        }
        outcome?;
        for j in (group..n).step_by(width) {
            for i in j.saturating_sub(upper)..=(j + lower).min(n - 1) {
                let entry = (f1[i] - f0[i]) / deltas[j];
                if !entry.is_finite() {
                    return Err(CoreError::NonFinite(
                        "finite-difference band entry overflowed".into(),
                    ));
                }
                band[i * width + (j + lower - i)] = entry;
            }
        }
    }
    Ok(())
}

/// What the Jacobian callback of a protected run saw: the fingerprints an
/// [`AssemblyStageClient`] with fingerprints keeps, built the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trace {
    pub builds: u64,
    pub state_fingerprint: u64,
    pub jacobian_fingerprint: u64,
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            builds: 0,
            state_fingerprint: FINGERPRINT_SEED,
            jacobian_fingerprint: FINGERPRINT_SEED,
        }
    }
}

impl Trace {
    fn record(&mut self, t: f64, y: &[f64], jacobian: &[f64]) {
        self.builds += 1;
        self.state_fingerprint = fingerprint(fingerprint(self.state_fingerprint, &[t]), y);
        self.jacobian_fingerprint = fingerprint(self.jacobian_fingerprint, jacobian);
    }
}

/// `fd` with the reference dense finite differences as its Jacobian
/// callbacks; the in-place callback (the one the fast driver uses) records
/// every completed build in `trace`.
pub fn protected_dense_problem(fd: &FdProblem, trace: &Arc<Mutex<Trace>>) -> OdeProblem {
    let n = fd.n;
    let for_matrix = fd.problem.clone();
    let for_into = fd.problem.clone();
    let trace = Arc::clone(trace);
    OdeProblem::new(
        format!("{} (reference finite differences)", fd.id),
        n,
        Arc::clone(&fd.rhs),
        None,
        Some(Arc::new(
            move |t: f64, y: &[f64]| -> CoreResult<DenseMatrix> {
                let mut matrix = DenseMatrix::zeros(n, n);
                reference_dense_fd(&for_matrix, t, y, &mut matrix)?;
                Ok(matrix)
            },
        )),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
    .with_jacobian_into(Arc::new(
        move |t: f64, y: &[f64], out: &mut DenseMatrix| -> CoreResult<()> {
            reference_dense_fd(&for_into, t, y, out)?;
            trace.lock().unwrap().record(t, y, out.as_slice());
            Ok(())
        },
    ))
}

/// The reference grouped finite differences of `fd` as the band provider of
/// the protected banded driver, recording every completed build.
pub fn protected_band(
    fd: &FdProblem,
    lower: usize,
    upper: usize,
    trace: &Arc<Mutex<Trace>>,
) -> BandedJacobian {
    let problem = fd.problem.clone();
    let trace = Arc::clone(trace);
    BandedJacobian {
        lower,
        upper,
        fill: Arc::new(
            move |t: f64, y: &[f64], band: &mut [f64]| -> CoreResult<()> {
                reference_banded_fd(&problem, t, y, band, lower, upper)?;
                trace.lock().unwrap().record(t, y, band);
                Ok(())
            },
        ),
    }
}

/// Tolerances and length of a run.
#[derive(Clone, Copy, Debug)]
pub struct RunConfig {
    pub rtol: f64,
    pub atol: f64,
    pub max_attempts: usize,
}

impl RunConfig {
    /// The timed configuration of `PREDECLARED.json`.
    pub fn declared() -> Self {
        Self {
            rtol: 1.0e-6,
            atol: 1.0e-8,
            max_attempts: 48,
        }
    }

    pub fn adaptive(&self, span: (f64, f64)) -> AdaptiveStepConfig {
        AdaptiveStepConfig {
            atol: self.atol,
            rtol: self.rtol,
            initial_step: 1.0e-6,
            min_step: 1.0e-14,
            max_step: span.1 - span.0,
            max_attempts: self.max_attempts,
            ..AdaptiveStepConfig::default()
        }
    }
}

/// Sequential, or threaded with this many workers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arm {
    Seq,
    Threaded(usize),
}

impl Arm {
    pub fn name(self) -> String {
        match self {
            Self::Seq => "seq".into(),
            Self::Threaded(workers) => format!("t{workers}"),
        }
    }
}

/// One client configuration.
#[derive(Clone, Debug)]
pub struct ClientSpec {
    pub route: ColumnRoute,
    pub arm: Arm,
    pub backend: LinearBackend,
    pub first_use_check: CheckColumns,
    pub fingerprints: bool,
}

impl ClientSpec {
    pub fn options(&self) -> AssemblyOptions {
        let options = match &self.route {
            ColumnRoute::Dense => AssemblyOptions::dense(),
            ColumnRoute::Coloured(pattern) => AssemblyOptions::coloured(pattern.clone()),
        };
        let options = match self.arm {
            Arm::Seq => options,
            Arm::Threaded(workers) => options.threaded(workers, RhsPurity::pure_and_reentrant()),
        };
        let options = options.with_first_use_check(self.first_use_check);
        if self.fingerprints {
            options.with_fingerprints()
        } else {
            options
        }
    }

    pub fn build(&self, n: usize) -> AssemblyStageClient {
        AssemblyStageClient::new(n, self.backend, self.options()).unwrap()
    }
}

/// The result of a client run.
pub struct ClientRun {
    pub result: Rodas5pFastResult,
    pub partial: Option<DriverPartial>,
    pub stats: AssemblyStats,
    pub phases: PhaseTimes,
    pub colours: Option<usize>,
    pub rhs_per_build: usize,
    /// Construction of the client and the driver call.
    pub wall_ns: u64,
    /// Construction of the client alone.
    pub construct_ns: u64,
}

fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// One run of the client driver with the assembly client of `spec` inside
/// a [`TimedStageClient`]. The handle is returned whether the run ends
/// with a result or with an error.
pub fn run_client(
    fd: &FdProblem,
    spec: &ClientSpec,
    config: &RunConfig,
) -> (AssemblyReportHandle, CoreResult<ClientRun>) {
    let adaptive = config.adaptive(fd.span);
    let atol = vec![config.atol; fd.n];
    let output = OutputSchedule::new(vec![fd.span.0, fd.span.1]).unwrap();
    let start = Instant::now();
    let client = spec.build(fd.n);
    let handle = client.handle();
    let (colours, rhs_per_build) = (client.colours(), client.rhs_per_build());
    let timed = TimedStageClient::new(client);
    let phases = timed.handle();
    let boxed: Box<dyn StageLinearSystem> = Box::new(timed);
    let construct_ns = nanos(start);
    let outcome = integrate_rodas5p_fast_client_observed_with_atol(
        &fd.problem,
        boxed,
        fd.span,
        &fd.y0,
        &adaptive,
        &output,
        &atol,
    );
    let wall_ns = nanos(start);
    let run = outcome.map(|(result, partial, _client)| ClientRun {
        result,
        partial,
        stats: handle.stats(),
        phases: phases.snapshot(),
        colours,
        rhs_per_build,
        wall_ns,
        construct_ns,
    });
    (handle, run)
}

/// The result of a protected run with reference finite differences.
pub struct ProtectedRun {
    pub result: Rodas5pFastResult,
    pub partial: Option<DriverPartial>,
    pub trace: Trace,
    pub wall_ns: u64,
}

/// The protected dense driver on `fd` with [`reference_dense_fd`].
pub fn run_protected_dense(fd: &FdProblem, config: &RunConfig) -> CoreResult<ProtectedRun> {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let problem = protected_dense_problem(fd, &trace);
    let adaptive = config.adaptive(fd.span);
    let atol = vec![config.atol; fd.n];
    let output = OutputSchedule::new(vec![fd.span.0, fd.span.1]).unwrap();
    let start = Instant::now();
    let (result, partial) = integrate_rodas5p_fast_observed_with_atol(
        &problem, fd.span, &fd.y0, &adaptive, &output, &atol,
    )?;
    let wall_ns = nanos(start);
    let trace = *trace.lock().unwrap();
    Ok(ProtectedRun {
        result,
        partial,
        trace,
        wall_ns,
    })
}

/// The protected banded driver (kernel `Indexed`) on `fd` with
/// [`reference_banded_fd`].
pub fn run_protected_banded(
    fd: &FdProblem,
    lower: usize,
    upper: usize,
    config: &RunConfig,
) -> CoreResult<ProtectedRun> {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let band = protected_band(fd, lower, upper, &trace);
    let adaptive = config.adaptive(fd.span);
    let atol = vec![config.atol; fd.n];
    let output = OutputSchedule::new(vec![fd.span.0, fd.span.1]).unwrap();
    let start = Instant::now();
    let run = integrate_rodas5p_fast_banded_observed_with_atol(
        &fd.problem,
        &band,
        fd.span,
        &fd.y0,
        &adaptive,
        &output,
        &atol,
    )?;
    let wall_ns = nanos(start);
    let trace = *trace.lock().unwrap();
    Ok(ProtectedRun {
        result: run.fast,
        partial: run.partial,
        trace,
        wall_ns,
    })
}

/// The last accepted time and state of a run: the partial record if the run
/// stopped early, otherwise the last output.
pub fn final_state<'a>(
    result: &'a Rodas5pFastResult,
    partial: &'a Option<DriverPartial>,
) -> (f64, &'a [f64]) {
    match partial {
        Some(partial) => (partial.t, &partial.y),
        None => (
            *result.observed.t.last().unwrap(),
            result.observed.y.last().unwrap(),
        ),
    }
}

/// [`fingerprint`] of the last accepted time and state.
pub fn final_fingerprint(result: &Rodas5pFastResult, partial: &Option<DriverPartial>) -> u64 {
    let (t, y) = final_state(result, partial);
    fingerprint(fingerprint(FINGERPRINT_SEED, &[t]), y)
}

pub fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}
