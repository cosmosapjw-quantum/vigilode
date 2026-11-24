//! Structure-exact stage solves for [`DiffusionGrid`] problems (stage-lattice task SL-N5).
//!
//! The longitude rotations of a sphere grid and the translations of a periodic box commute
//! with the heat operator `L`. So the discrete Fourier transform block-diagonalizes every
//! RODAS5P stage matrix `W = z I - (L + c I)`, `z = 1/(h gamma)`: one real tridiagonal
//! system per longitude mode on the sphere, one scalar per wave vector on the box.
//! Reflection makes the blocks of modes `m` and `np - m` equal. Complex conjugation makes the
//! spectra of real data Hermitian. So the sphere needs `floor(np/2) + 1` factorizations, and
//! one complex FFT per pair of rings in each direction.
//!
//! The derivations, with their claim status, are in
//! `research/kernel_spectral_foundation_20261124/THEORY.md`:
//! - §2: the symmetry and the two involutions.
//! - §3.4: the stage mass identity `(z - c) M(x) = M(b)` checked by every solve.
//! - §4.2: the inertia certificate that refuses a preparation beyond the stage pencil.
//! - §5: the spectral bounds of [`SeparableStages::spectral_bounds`].

use std::f64::consts::PI;
use std::sync::{Arc, Mutex, PoisonError};

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::{OdeProblem, StageLinearSystem};
use rustfft::{Fft, FftPlanner, num_complex::Complex64};
use serde::Serialize;

use super::{DiffusionGrid, Geometry, invalid};
use crate::SolveError;

type Jvp = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// Identifier of the separable stage client in [`crate::DriverWork::SeparableClient`].
pub const SEPARABLE_STAGE_CLIENT_ID: &str = "vigilode-separable-fourier-stages-v1";

/// How the separable solver treats the problem's own Jacobian.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum SeparableMode {
    /// The problem's Jacobian is the declared `L + c I`. Every solve is checked only by the
    /// O(n) stage mass identity `(z - c) M(x) = M(b)`, recorded in
    /// [`SeparableReport::max_mass_defect`].
    Trusted,
    /// After every solve the residual `b - (z x - J x)` is formed with the problem's own JVP.
    /// While its backward error `|r| / (z |x| + |J x| + |b|)` (maximum norms) exceeds
    /// `tolerance`, defect correction `x += W_sep^{-1} r` runs, at most `max_corrections`
    /// times. A solve that still misses is refused with a typed linear-solve error, so the
    /// driver rejects the attempt and shrinks the step.
    Verified {
        /// Corrections allowed per stage solve.
        max_corrections: usize,
        /// Backward-error bound of an accepted stage solve.
        tolerance: f64,
    },
}

impl Default for SeparableMode {
    fn default() -> Self {
        Self::Verified {
            max_corrections: 4,
            tolerance: 1.0e-10,
        }
    }
}

/// Spectral facts of the declared Jacobian `J = L + c I` (THEORY.md §5).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SpectralBounds {
    /// `c`: the largest eigenvalue of `J`, since the kernel of `L` is the constants.
    pub abscissa: f64,
    /// A Gershgorin bound `rho_bar` on the spectral radius of `L`:
    /// `spec J ⊂ [c - rho_bar, c]`.
    pub radius_bound: f64,
    /// The DOPRI5 step bound `DOPRI5_REAL_STABILITY_BOUND / (rho_bar - c)` on the negative
    /// real axis for `c <= 0` (infinite for a zero operator, NaN for `c > 0`).
    pub explicit_step_bound: f64,
}

/// What the separable stage client did in one run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct SeparableReport {
    /// Preparations of `W(z)` (one per attempt).
    pub preparations: usize,
    /// Preparations refused by the inertia certificate: some eigenvalue of `J` at or above `z`.
    pub refused_preparations: usize,
    /// Eigenvalues of the declared `J` at or above `z` (with multiplicity) at the last refusal.
    pub crossings_at_last_refusal: usize,
    /// Stage solves requested by the driver.
    pub solves: usize,
    /// Residuals formed with the problem's JVP (`Verified` mode).
    pub residual_checks: usize,
    /// Defect corrections (`Verified` mode).
    pub corrections: usize,
    /// Stage solves refused after `max_corrections` corrections (`Verified` mode).
    pub refused_solves: usize,
    /// Largest backward error of an accepted solve (`Verified` mode; 0 in `Trusted`).
    pub max_backward_error: f64,
    /// Largest `|(z - c) M(x) - M(b)| / sum_i A_i |b_i|` over the solves.
    pub max_mass_defect: f64,
}

/// The separable stage solver of one grid for a declared Jacobian `L + c I`.
#[derive(Clone, Debug)]
pub struct SeparableStages {
    grid: DiffusionGrid,
    linear_rate: f64,
    mode: SeparableMode,
}

impl DiffusionGrid {
    /// The separable (Fourier) stage solver for problems whose Jacobian is
    /// `L + linear_rate I`, with `L` this grid's heat operator ([`Self::apply`]). The mode is
    /// [`SeparableMode::default`] (`Verified`); see [`SeparableStages::with_mode`]. Run it
    /// with [`crate::VigilODE::solve_separable`].
    pub fn separable_stages(&self, linear_rate: f64) -> Result<SeparableStages, SolveError> {
        if !linear_rate.is_finite() {
            return Err(invalid("the declared linear rate must be finite"));
        }
        Ok(SeparableStages {
            grid: self.clone(),
            linear_rate,
            mode: SeparableMode::default(),
        })
    }
}

impl SeparableStages {
    /// The same solver in `mode`. `Verified` needs a positive finite tolerance.
    pub fn with_mode(mut self, mode: SeparableMode) -> Result<Self, SolveError> {
        if let SeparableMode::Verified { tolerance, .. } = mode
            && !(tolerance.is_finite() && tolerance > 0.0)
        {
            return Err(invalid(
                "the separable backward-error tolerance must be positive and finite",
            ));
        }
        self.mode = mode;
        Ok(self)
    }

    /// The mode of every stage solve.
    pub fn mode(&self) -> SeparableMode {
        self.mode
    }

    /// The declared linear rate `c` of `J = L + c I`.
    pub fn linear_rate(&self) -> f64 {
        self.linear_rate
    }

    /// The number of unknowns (cells of the grid).
    pub fn dimension(&self) -> usize {
        self.grid.dimension
    }

    /// `spec J ⊂ [c - rho_bar, c]` and the DOPRI5 real-axis step bound (THEORY.md §5.2-5.3).
    pub fn spectral_bounds(&self) -> SpectralBounds {
        let radius_bound = match &*self.grid.geometry {
            Geometry::Sphere(rings) => rings
                .iter()
                .map(|r| 2.0 * (r.north + r.south) + 4.0 * r.azimuth)
                .fold(0.0, f64::max),
            Geometry::Cartesian { rates, .. } => 4.0 * rates.iter().sum::<f64>(),
        };
        let explicit_step_bound = if self.linear_rate > 0.0 {
            f64::NAN
        } else {
            crate::DOPRI5_REAL_STABILITY_BOUND / (radius_bound - self.linear_rate)
        };
        SpectralBounds {
            abscissa: self.linear_rate,
            radius_bound,
            explicit_step_bound,
        }
    }

    /// Solve `(z I - L - c I) x = b` in place once (no integration, no JVP). The inertia
    /// certificate refuses a `z` with an eigenvalue of `J` at or above it; the error message
    /// gives their number.
    pub fn solve_stage(&self, z: f64, b: &mut [f64]) -> Result<(), SolveError> {
        if b.len() != self.dimension() {
            return Err(invalid(
                "separable stage solve: right-hand side length mismatch",
            ));
        }
        let mut kernel = Kernel::new(&self.grid);
        kernel
            .prepare(z, self.linear_rate)
            .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
        kernel.solve(b);
        Ok(())
    }

    /// The number of eigenvalues of the declared `J = L + c I` at or above `z`, with
    /// multiplicity, read from the signs of the Thomas pivots of the mode systems
    /// (Sylvester's law of inertia, THEORY.md §4.2).
    pub fn inertia(&self, z: f64) -> usize {
        Kernel::new(&self.grid).crossings(z, self.linear_rate)
    }

    /// The stage client for the protected client driver; `report` collects its work.
    pub(crate) fn client(&self, report: Arc<Mutex<SeparableReport>>) -> Box<dyn StageLinearSystem> {
        let n = self.dimension();
        let weights = match &*self.grid.geometry {
            Geometry::Sphere(rings) => rings
                .iter()
                .flat_map(|r| std::iter::repeat_n(r.area, self.grid.width))
                .collect(),
            Geometry::Cartesian { .. } => vec![1.0; n],
        };
        Box::new(SeparableClient {
            kernel: Kernel::new(&self.grid),
            linear_rate: self.linear_rate,
            mode: self.mode,
            weights,
            jvp: None,
            t: 0.0,
            y: vec![0.0; n],
            z: f64::NAN,
            prepared: false,
            rhs: vec![0.0; n],
            work: vec![0.0; n],
            jx: vec![0.0; n],
            report,
        })
    }
}

/// `2 - 2 cos(2 pi k / n) = 4 sin^2(pi k / n)`, without cancellation near `k = 0`.
fn symbol(k: usize, n: usize) -> f64 {
    let s = (PI * k as f64 / n as f64).sin();
    4.0 * s * s
}

/// Forward and inverse plans of one transform length.
type FftPair = (Arc<dyn Fft<f64>>, Arc<dyn Fft<f64>>);

fn plans(n: usize) -> FftPair {
    let mut planner = FftPlanner::new();
    (planner.plan_fft_forward(n), planner.plan_fft_inverse(n))
}

enum Kernel {
    Sphere(SphereKernel),
    Box(BoxKernel),
}

impl Kernel {
    fn new(grid: &DiffusionGrid) -> Self {
        match &*grid.geometry {
            Geometry::Sphere(rings) => Self::Sphere(SphereKernel::new(rings, grid.width)),
            Geometry::Cartesian { ny, nz, rates, .. } => {
                Self::Box(BoxKernel::new([grid.width, *ny, *nz], *rates))
            }
        }
    }

    /// Eigenvalues of `L + c I` at or above `z`, with multiplicity.
    fn crossings(&mut self, z: f64, c: f64) -> usize {
        match self {
            Self::Sphere(k) => k.factor(z, c),
            Self::Box(k) => k.crossings(z - c),
        }
    }

    /// Prepare `W(z)`; refuse when the inertia certificate finds an eigenvalue at or above `z`.
    fn prepare(&mut self, z: f64, c: f64) -> CoreResult<()> {
        if !(z.is_finite() && z > 0.0) {
            return Err(CoreError::LinearSolve(format!(
                "separable stage preparation: shift {z} is not positive and finite"
            )));
        }
        let crossings = self.crossings(z, c);
        if crossings > 0 {
            return Err(CoreError::LinearSolve(format!(
                "separable stage preparation: {crossings} eigenvalues of the declared Jacobian \
                 at or above 1/(h gamma) = {z} (inertia certificate)"
            )));
        }
        if let Self::Box(k) = self {
            k.shift = z - c;
        }
        Ok(())
    }

    fn solve(&mut self, b: &mut [f64]) {
        match self {
            Self::Sphere(k) => k.solve(b),
            Self::Box(k) => k.solve(b),
        }
    }
}

/// Mode-wise Thomas factors of the sphere's stage matrices. Spectra are mode-major:
/// `spectra[m * nt + r]` is mode `m` of ring `r`, so every Thomas sweep is contiguous.
struct SphereKernel {
    nt: usize,
    np: usize,
    modes: usize,
    north: Vec<f64>,
    south: Vec<f64>,
    azimuth: Vec<f64>,
    symbol: Vec<f64>,
    /// `l_r = W[r][r-1] / p_{r-1}`, mode-major.
    mult: Vec<f64>,
    /// Thomas pivots `p_r`, mode-major.
    pivot: Vec<f64>,
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
    /// One length-`np` line per pair of rings (and one for an odd last ring).
    lines: Vec<Complex64>,
    spectra: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

impl SphereKernel {
    fn new(rings: &[super::Ring], np: usize) -> Self {
        let nt = rings.len();
        let modes = np / 2 + 1;
        let (forward, inverse) = plans(np);
        let scratch = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        Self {
            nt,
            np,
            modes,
            north: rings.iter().map(|r| r.north).collect(),
            south: rings.iter().map(|r| r.south).collect(),
            azimuth: rings.iter().map(|r| r.azimuth).collect(),
            symbol: (0..modes).map(|m| symbol(m, np)).collect(),
            mult: vec![0.0; modes * nt],
            pivot: vec![0.0; modes * nt],
            forward,
            inverse,
            lines: vec![Complex64::default(); nt.div_ceil(2) * np],
            spectra: vec![Complex64::default(); modes * nt],
            scratch: vec![Complex64::default(); scratch],
        }
    }

    /// Thomas factors of `W_m = tridiag(-n_r, z - c + n_r + s_r + a_r sym_m, -s_r)` for every
    /// distinct mode; returns the number of eigenvalues at or above `z`. A mode `0 < m < np/2`
    /// stands for `m` and `np - m`, so its count is doubled.
    fn factor(&mut self, z: f64, c: f64) -> usize {
        let (nt, np) = (self.nt, self.np);
        let shift = z - c;
        let mut crossings = 0;
        for m in 0..self.modes {
            let base = m * nt;
            let sym = self.symbol[m];
            let mut count = 0;
            let mut previous = 1.0;
            for r in 0..nt {
                let d = shift + self.north[r] + self.south[r] + self.azimuth[r] * sym;
                let p = if r == 0 {
                    d
                } else {
                    let l = -self.north[r] / previous;
                    self.mult[base + r] = l;
                    d + l * self.south[r - 1]
                };
                self.pivot[base + r] = p;
                if p.is_nan() || p <= 0.0 {
                    count += 1;
                }
                previous = p;
            }
            let multiplicity = if m == 0 || 2 * m == np { 1 } else { 2 };
            crossings += multiplicity * count;
        }
        crossings
    }

    fn solve(&mut self, b: &mut [f64]) {
        let (nt, np, modes) = (self.nt, self.np, self.modes);
        let pairs = nt / 2;
        // Two real rings per complex line: z = u + i v.
        for (q, line) in self.lines.chunks_exact_mut(np).enumerate() {
            let r0 = 2 * q;
            if q < pairs {
                let (u, v) = (&b[r0 * np..(r0 + 1) * np], &b[(r0 + 1) * np..(r0 + 2) * np]);
                for ((dst, &re), &im) in line.iter_mut().zip(u).zip(v) {
                    *dst = Complex64::new(re, im);
                }
            } else {
                for (dst, &re) in line.iter_mut().zip(&b[r0 * np..(r0 + 1) * np]) {
                    *dst = Complex64::new(re, 0.0);
                }
            }
        }
        self.forward
            .process_with_scratch(&mut self.lines, &mut self.scratch);
        // Separate the pairs by the conjugation involution R Z(m) = conj Z(np - m).
        let minus_half_i = Complex64::new(0.0, -0.5);
        for (q, line) in self.lines.chunks_exact(np).enumerate() {
            let r0 = 2 * q;
            if q < pairs {
                for m in 0..modes {
                    let zm = line[m];
                    let zr = line[(np - m) % np].conj();
                    self.spectra[m * nt + r0] = (zm + zr) * 0.5;
                    self.spectra[m * nt + r0 + 1] = (zm - zr) * minus_half_i;
                }
            } else {
                for (m, &zm) in line.iter().enumerate().take(modes) {
                    self.spectra[m * nt + r0] = zm;
                }
            }
        }
        // One real tridiagonal solve per distinct mode, two real right-hand sides each.
        for m in 0..modes {
            let base = m * nt;
            let col = &mut self.spectra[base..base + nt];
            let mult = &self.mult[base..base + nt];
            let pivot = &self.pivot[base..base + nt];
            for r in 1..nt {
                let prev = col[r - 1];
                col[r] -= prev * mult[r];
            }
            col[nt - 1] /= pivot[nt - 1];
            for r in (0..nt - 1).rev() {
                let next = col[r + 1];
                col[r] = (col[r] + next * self.south[r]) / pivot[r];
            }
        }
        // Rebuild Z = U + i V from the Hermitian half spectra, invert, split.
        let i = Complex64::new(0.0, 1.0);
        for (q, line) in self.lines.chunks_exact_mut(np).enumerate() {
            let r0 = 2 * q;
            let paired = q < pairs;
            for (m, dst) in line.iter_mut().enumerate() {
                let (k, conjugate) = if m < modes {
                    (m, false)
                } else {
                    (np - m, true)
                };
                let mut u = self.spectra[k * nt + r0];
                let mut v = if paired {
                    self.spectra[k * nt + r0 + 1]
                } else {
                    Complex64::default()
                };
                if conjugate {
                    u = u.conj();
                    v = v.conj();
                }
                *dst = u + v * i;
            }
        }
        self.inverse
            .process_with_scratch(&mut self.lines, &mut self.scratch);
        let scale = 1.0 / np as f64;
        for (q, line) in self.lines.chunks_exact(np).enumerate() {
            let r0 = 2 * q;
            for (x, z) in line.iter().enumerate() {
                b[r0 * np + x] = z.re * scale;
            }
            if q < pairs {
                for (x, z) in line.iter().enumerate() {
                    b[(r0 + 1) * np + x] = z.im * scale;
                }
            }
        }
    }
}

/// Diagonal of the box's stage matrix in Fourier space: `z - c + e_x + e_y + e_z`.
struct BoxKernel {
    cells: [usize; 3],
    symbols: [Vec<f64>; 3],
    shift: f64,
    plans: Vec<FftPair>,
    buffer: Vec<Complex64>,
    line: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

impl BoxKernel {
    fn new(cells: [usize; 3], rates: [f64; 3]) -> Self {
        let symbols = std::array::from_fn(|axis| {
            (0..cells[axis])
                .map(|k| rates[axis] * symbol(k, cells[axis]))
                .collect()
        });
        let plans: Vec<_> = cells.iter().map(|&n| plans(n)).collect();
        let scratch = plans
            .iter()
            .flat_map(|(f, i)| [f.get_inplace_scratch_len(), i.get_inplace_scratch_len()])
            .max()
            .unwrap_or(0);
        let n = cells.iter().product();
        Self {
            cells,
            symbols,
            shift: f64::NAN,
            plans,
            buffer: vec![Complex64::default(); n],
            line: vec![Complex64::default(); cells[1].max(cells[2])],
            scratch: vec![Complex64::default(); scratch],
        }
    }

    /// Wave vectors with `shift + e_x + e_y + e_z <= 0`.
    fn crossings(&self, shift: f64) -> usize {
        if shift > 0.0 {
            return 0;
        }
        let [sx, sy, sz] = &self.symbols;
        let mut count = 0;
        for &ez in sz {
            for &ey in sy {
                count += sx
                    .iter()
                    .filter(|&&ex| {
                        let d = shift + ex + ey + ez;
                        d.is_nan() || d <= 0.0
                    })
                    .count();
            }
        }
        count
    }

    /// Transform along `axis` (1: y, 2: z) by gathering the strided lines.
    fn transform_strided(&mut self, axis: usize, inverse: bool) {
        let [nx, ny, nz] = self.cells;
        // (line length, element stride, groups, group stride, lines per group)
        let (len, stride, groups, group_stride, per_group) = if axis == 1 {
            (ny, nx, nz, nx * ny, nx)
        } else {
            (nz, nx * ny, 1, 0, nx * ny)
        };
        if len == 1 {
            return;
        }
        let fft = Arc::clone(if inverse {
            &self.plans[axis].1
        } else {
            &self.plans[axis].0
        });
        let line = &mut self.line[..len];
        for group in 0..groups {
            for j in 0..per_group {
                let start = group * group_stride + j;
                for (k, dst) in line.iter_mut().enumerate() {
                    *dst = self.buffer[start + k * stride];
                }
                fft.process_with_scratch(line, &mut self.scratch);
                for (k, src) in line.iter().enumerate() {
                    self.buffer[start + k * stride] = *src;
                }
            }
        }
    }

    fn solve(&mut self, b: &mut [f64]) {
        let [nx, ny, _] = self.cells;
        for (dst, &v) in self.buffer.iter_mut().zip(b.iter()) {
            *dst = Complex64::new(v, 0.0);
        }
        let forward_x = Arc::clone(&self.plans[0].0);
        forward_x.process_with_scratch(&mut self.buffer, &mut self.scratch);
        self.transform_strided(1, false);
        self.transform_strided(2, false);
        let [sx, sy, sz] = &self.symbols;
        for (index, value) in self.buffer.iter_mut().enumerate() {
            let x = index % nx;
            let y = index / nx % ny;
            let z = index / (nx * ny);
            *value /= self.shift + sx[x] + sy[y] + sz[z];
        }
        self.transform_strided(2, true);
        self.transform_strided(1, true);
        let inverse_x = Arc::clone(&self.plans[0].1);
        inverse_x.process_with_scratch(&mut self.buffer, &mut self.scratch);
        let scale = 1.0 / self.buffer.len() as f64;
        for (dst, v) in b.iter_mut().zip(&self.buffer) {
            *dst = v.re * scale;
        }
    }
}

/// The [`StageLinearSystem`] the protected client driver runs.
struct SeparableClient {
    kernel: Kernel,
    linear_rate: f64,
    mode: SeparableMode,
    /// Cell measures of the mass `M(u) = sum_i A_i u_i` (ones on the box).
    weights: Vec<f64>,
    jvp: Option<Jvp>,
    t: f64,
    y: Vec<f64>,
    z: f64,
    prepared: bool,
    rhs: Vec<f64>,
    work: Vec<f64>,
    jx: Vec<f64>,
    report: Arc<Mutex<SeparableReport>>,
}

fn max_abs(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |m, x| m.max(x.abs()))
}

impl SeparableClient {
    fn record(&self, update: impl FnOnce(&mut SeparableReport)) {
        update(&mut self.report.lock().unwrap_or_else(PoisonError::into_inner));
    }

    /// Backward error of `x` for the problem's own `W`, against the right-hand side kept in
    /// `self.rhs`; leaves the residual in `self.work`.
    fn backward_error(&mut self, x: &[f64], counters: &mut WorkCounters) -> CoreResult<f64> {
        let jvp = self.jvp.as_ref().ok_or_else(|| {
            CoreError::InvalidInput("separable Verified mode needs the problem's JVP".into())
        })?;
        jvp(self.t, &self.y, x, &mut self.jx)?;
        counters.jvp_calls = counters.jvp_calls.saturating_add(1);
        counters.jvp_vectors = counters.jvp_vectors.saturating_add(1);
        for (((r, &b), &xi), &jx) in self.work.iter_mut().zip(&self.rhs).zip(x).zip(&self.jx) {
            *r = b - (self.z * xi - jx);
        }
        let scale = self.z * max_abs(x) + max_abs(&self.jx) + max_abs(&self.rhs);
        let residual = max_abs(&self.work);
        Ok(if scale > 0.0 {
            residual / scale
        } else {
            residual
        })
    }
}

impl StageLinearSystem for SeparableClient {
    fn dimension(&self) -> usize {
        self.weights.len()
    }

    /// Keep `(t, y)` and the problem's JVP for the residual of `Verified` mode. The declared
    /// `J` is constant, so there is nothing to assemble.
    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if matches!(self.mode, SeparableMode::Verified { .. }) {
            self.jvp = problem.jvp_callback().cloned();
            self.t = t;
            self.y.copy_from_slice(y);
        }
        self.prepared = false;
        Ok(())
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.direct_factorizations = counters.direct_factorizations.saturating_add(1);
        self.prepared = false;
        let outcome = self.kernel.prepare(inv_h_gamma, self.linear_rate);
        match &outcome {
            Ok(()) => {
                self.z = inv_h_gamma;
                self.prepared = true;
                self.record(|r| r.preparations += 1);
            }
            Err(_) => {
                let crossings = self.kernel.crossings(inv_h_gamma, self.linear_rate);
                self.record(|r| {
                    r.preparations += 1;
                    r.refused_preparations += 1;
                    r.crossings_at_last_refusal = crossings;
                });
            }
        }
        outcome
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if !self.prepared {
            return Err(CoreError::LinearSolve(
                "separable stage solve before a successful preparation".into(),
            ));
        }
        self.rhs.copy_from_slice(b);
        self.kernel.solve(b);
        counters.direct_solve_calls = counters.direct_solve_calls.saturating_add(1);
        let (mut mass_b, mut mass_x, mut scale) = (0.0, 0.0, 0.0);
        for ((&w, &bi), &xi) in self.weights.iter().zip(&self.rhs).zip(b.iter()) {
            mass_b += w * bi;
            mass_x += w * xi;
            scale += w * bi.abs();
        }
        let defect = if scale > 0.0 {
            ((self.z - self.linear_rate) * mass_x - mass_b).abs() / scale
        } else {
            0.0
        };
        let SeparableMode::Verified {
            max_corrections,
            tolerance,
        } = self.mode
        else {
            self.record(|r| {
                r.solves += 1;
                r.max_mass_defect = r.max_mass_defect.max(defect);
            });
            return Ok(());
        };
        let mut checks = 0;
        let mut corrections = 0;
        let outcome = loop {
            let error = self.backward_error(b, counters)?;
            checks += 1;
            if error <= tolerance {
                break Ok(error);
            }
            if corrections == max_corrections {
                break Err(error);
            }
            // x += W_sep^{-1} r
            let mut d = std::mem::take(&mut self.work);
            self.kernel.solve(&mut d);
            counters.direct_solve_calls = counters.direct_solve_calls.saturating_add(1);
            counters.linear_iterations = counters.linear_iterations.saturating_add(1);
            for (x, dx) in b.iter_mut().zip(&d) {
                *x += dx;
            }
            self.work = d;
            corrections += 1;
        };
        self.record(|r| {
            r.solves += 1;
            r.residual_checks += checks;
            r.corrections += corrections;
            r.max_mass_defect = r.max_mass_defect.max(defect);
            match outcome {
                Ok(error) => r.max_backward_error = r.max_backward_error.max(error),
                Err(_) => r.refused_solves += 1,
            }
        });
        outcome.map(|_| ()).map_err(|error| {
            CoreError::LinearSolve(format!(
                "separable stage solve: backward error {error:.3e} above the tolerance \
                 {tolerance:.3e} after {max_corrections} corrections"
            ))
        })
    }
}
