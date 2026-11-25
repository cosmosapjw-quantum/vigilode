//! Structure-exact stage solves for declared diffusion and periodic upwind problems.
//!
//! The longitude rotations of a sphere grid and the translations of a periodic box commute
//! with the heat operator `L`. So the discrete Fourier transform block-diagonalizes every
//! RODAS5P stage matrix `W = z I - (L + c I)`, `z = 1/(h gamma)`: one real tridiagonal
//! system per longitude mode on the sphere, one scalar per wave vector on the box.
//! Reflection makes the blocks of modes `m` and `np - m` equal. Complex conjugation makes the
//! spectra of real data Hermitian. So the sphere needs `floor(np/2) + 1` factorizations, and
//! one complex FFT per pair of rings in each direction. By default the box applies the same
//! pairing to its x-rows and transforms only the Hermitian half along y and z
//! ([`BoxTransform::PairedReal`], tasks KS-N7 and KS-N7b); [`BoxTransform::Complex`] transforms
//! its real field as complex data. A run records the box transform that ran in
//! [`SeparableReport::box_transform`].
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

/// Identifier of the separable stage client in [`crate::DriverWork::SeparableClient`]. Both box
/// transforms run under it; [`SeparableReport::box_transform`] records which one ran.
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

/// How the periodic box transforms its real fields (task KS-N7). Both variants are the same
/// linear map in exact arithmetic. A sphere grid ignores the choice: its kernel always packs
/// two real rings into one complex FFT (THEORY.md §2.3 (ii)). A run records the transform in
/// [`SeparableReport::box_transform`]; it serializes as its [`Self::name`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BoxTransform {
    /// Full complex FFTs of the real field (imaginary part zero) along x, y and z, forward
    /// and inverse; the real part of the result is kept. The default until task KS-N7b; still
    /// selectable with [`SeparableStages::with_box_transform`].
    Complex,
    /// Two-for-one real FFT, the default (task KS-N7b): rows `2q` and `2q + 1` are packed as
    /// `u + i v` along x and separated by the involution `R Z(m) = conj Z(nx - m)`; only the
    /// Hermitian half `k <= nx/2` of the x-spectra is transformed along y and z. About half the
    /// FFT lines of [`Self::Complex`]. Its buffer holds `(nx/2 + 1) ny nz` complex values plus
    /// one batch of packed lines (at most 4096 values, or one line when `nx > 4096`), against
    /// `nx ny nz` for [`Self::Complex`] (0.57 of it at 256x256).
    #[default]
    PairedReal,
}

impl BoxTransform {
    /// The name read by [`std::str::FromStr`] and written by [`std::fmt::Display`]:
    /// `complex` or `paired-real`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Complex => "complex",
            Self::PairedReal => "paired-real",
        }
    }
}

impl std::fmt::Display for BoxTransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::str::FromStr for BoxTransform {
    type Err = SolveError;

    /// `complex`, or `paired-real` (alias `paired`).
    fn from_str(name: &str) -> Result<Self, SolveError> {
        match name {
            "complex" => Ok(Self::Complex),
            "paired-real" | "paired" => Ok(Self::PairedReal),
            other => Err(SolveError::InvalidInput(format!(
                "unknown box transform `{other}`: expected `complex` or `paired-real`"
            ))),
        }
    }
}

/// Spectral facts of the declared Jacobian `J = L + c I` (THEORY.md §5).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SpectralBounds {
    /// c: the spectral abscissa, attained by the constant mode.
    pub abscissa: f64,
    /// A bound `rho_bar` on the spectral radius of `L`:
    /// The diffusion-only spectrum lies in [c - rho_bar, c]; for an upwind
    /// line the disk centered at c also encloses the complex spectrum.
    pub radius_bound: f64,
    /// The DOPRI5 step bound `DOPRI5_REAL_STABILITY_BOUND / (rho_bar - c)` on the negative
    /// real axis for c <= 0 (infinite for a zero operator). NaN for c > 0
    /// or nonzero upwind advection, for which the real-axis bound is invalid.
    pub explicit_step_bound: f64,
}

/// What the separable stage client did in one run, and the box transform it ran.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct SeparableReport {
    /// Preparations of `W(z)` (one per attempt).
    pub preparations: usize,
    /// Refused preparations (including failed Hermitian-part coercivity or nonfinite shifts).
    pub refused_preparations: usize,
    /// Eigenvalues of the declared Hermitian part at or above z (with multiplicity).
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
    /// The transform of the periodic box's stage solves ([`SeparableStages::box_transform`]);
    /// `None` on a sphere grid, whose kernel always pairs rings (task KS-N7b, finding F5 of the
    /// KS-N7 review).
    pub box_transform: Option<BoxTransform>,
    /// Signed advection rate velocity/dx for a declared periodic upwind line.
    /// Absent for the original diffusion-only routes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periodic_upwind_rate: Option<f64>,
}

/// The separable stage solver for a declared constant diffusion or periodic upwind Jacobian.
#[derive(Clone, Debug)]
pub struct SeparableStages {
    grid: DiffusionGrid,
    linear_rate: f64,
    mode: SeparableMode,
    box_transform: BoxTransform,
    periodic_upwind_rate: Option<f64>,
}

impl DiffusionGrid {
    /// The separable (Fourier) stage solver for problems whose Jacobian is
    /// `L + linear_rate I`, with `L` this grid's heat operator ([`Self::apply`]). The mode is
    /// [`SeparableMode::default`] (`Verified`); see [`SeparableStages::with_mode`]. A periodic
    /// box uses [`BoxTransform::default`] (`PairedReal`); see
    /// [`SeparableStages::with_box_transform`]. Run it with
    /// [`crate::VigilODE::solve_separable`].
    pub fn separable_stages(&self, linear_rate: f64) -> Result<SeparableStages, SolveError> {
        if !linear_rate.is_finite() {
            return Err(invalid("the declared linear rate must be finite"));
        }
        Ok(SeparableStages {
            grid: self.clone(),
            linear_rate,
            mode: SeparableMode::default(),
            box_transform: BoxTransform::default(),
            periodic_upwind_rate: None,
        })
    }
}

impl SeparableStages {
    /// Declare the stage Jacobian of a uniform periodic line with central diffusion,
    /// first-order upwind advection and a constant reaction rate.
    ///
    /// The caller's original problem is not modified. The default Verified mode
    /// checks every solve against that problem's JVP; this declaration can also
    /// act as a defect-correction preconditioner for a different Jacobian.
    /// Length, diffusivity and velocity have units L, L^2/T and L/T.
    pub fn periodic_upwind_line(
        n: usize,
        length: f64,
        diffusivity: f64,
        velocity: f64,
        linear_rate: f64,
    ) -> Result<Self, SolveError> {
        let dimension = super::dimension(&[n])?;
        let dx = length / n as f64;
        if !(dx.is_finite()
            && dx > 0.0
            && diffusivity.is_finite()
            && diffusivity >= 0.0
            && velocity.is_finite()
            && linear_rate.is_finite())
        {
            return Err(invalid("invalid periodic upwind line coefficients"));
        }
        let d = diffusivity / dx / dx;
        let v = velocity / dx;
        let rate = d + 0.5 * v.abs();
        if !(rate.is_finite() && v.is_finite() && (4.0 * rate + v.abs()).is_finite())
            || (diffusivity > 0.0 && d == 0.0)
            || (velocity != 0.0 && v == 0.0)
        {
            return Err(invalid(
                "periodic upwind line rate is outside binary64 range",
            ));
        }
        let grid = DiffusionGrid {
            dimension,
            width: n,
            geometry: Arc::new(Geometry::Cartesian {
                ny: 1,
                nz: 1,
                cell_volume: dx,
                rates: [rate, 0.0, 0.0],
            }),
        };
        let mut stages = grid.separable_stages(linear_rate)?;
        stages.periodic_upwind_rate = Some(v);
        Ok(stages)
    }

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

    /// The same solver with the periodic box's transform `transform` (default
    /// [`BoxTransform::PairedReal`]; [`BoxTransform::Complex`] stays selectable). A sphere grid
    /// ignores it.
    pub fn with_box_transform(mut self, transform: BoxTransform) -> Self {
        self.box_transform = transform;
        self
    }

    /// The transform of the periodic box's stage solves.
    pub fn box_transform(&self) -> BoxTransform {
        self.box_transform
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
        let drift = self.periodic_upwind_rate.unwrap_or(0.0).abs();
        let radius_bound = radius_bound + drift;
        let explicit_step_bound = if self.linear_rate > 0.0 || drift != 0.0 {
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

    /// Solve the declared shifted system in place once (no integration, no JVP). The inertia
    /// certificate refuses a `z` with an eigenvalue of `J` at or above it; the error message
    /// gives their number.
    pub fn solve_stage(&self, z: f64, b: &mut [f64]) -> Result<(), SolveError> {
        if b.len() != self.dimension() {
            return Err(invalid(
                "separable stage solve: right-hand side length mismatch",
            ));
        }
        let mut kernel = Kernel::new(&self.grid, self.box_transform, self.periodic_upwind_rate);
        kernel
            .prepare(z, self.linear_rate)
            .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
        solve_checked(&mut kernel, b, self.periodic_upwind_rate.is_some())
    }

    /// A stage solver prepared once for `z`: the FFT plans, the buffers and (on the sphere)
    /// the Thomas factors are built here and reused by every call, as the protected driver
    /// reuses one preparation for the eight stage solves of an attempt. Each call solves
    /// the declared shifted system in place with the same arithmetic as [`Self::solve_stage`].
    /// The preparation is refused like that of [`Self::solve_stage`].
    pub fn stage_solver(
        &self,
        z: f64,
    ) -> Result<impl FnMut(&mut [f64]) -> Result<(), SolveError> + Send + use<>, SolveError> {
        let mut kernel = Kernel::new(&self.grid, self.box_transform, self.periodic_upwind_rate);
        kernel
            .prepare(z, self.linear_rate)
            .map_err(|e| SolveError::InvalidInput(e.to_string()))?;
        let n = self.dimension();
        let checked = self.periodic_upwind_rate.is_some();
        Ok(move |b: &mut [f64]| {
            if b.len() != n {
                return Err(invalid(
                    "separable stage solve: right-hand side length mismatch",
                ));
            }
            solve_checked(&mut kernel, b, checked)
        })
    }

    /// Number of eigenvalues of the declared Hermitian part at or above z,
    /// with multiplicity (area-weighted on a sphere). On a normal periodic
    /// upwind line these equal the number of eigenvalues with real part >= z.
    /// This is a sufficient coercivity screen, not a general singularity test.
    pub fn inertia(&self, z: f64) -> usize {
        Kernel::new(&self.grid, self.box_transform, self.periodic_upwind_rate)
            .crossings(z, self.linear_rate)
    }

    /// The stage client for the protected client driver; `report` collects its work and
    /// records the box transform of the kernel the client runs.
    pub(crate) fn client(&self, report: Arc<Mutex<SeparableReport>>) -> Box<dyn StageLinearSystem> {
        let n = self.dimension();
        let weights = match &*self.grid.geometry {
            Geometry::Sphere(rings) => rings
                .iter()
                .flat_map(|r| std::iter::repeat_n(r.area, self.grid.width))
                .collect(),
            Geometry::Cartesian { .. } => vec![1.0; n],
        };
        let kernel = Kernel::new(&self.grid, self.box_transform, self.periodic_upwind_rate);
        // Read from the kernel that runs, so the record cannot drift from the arithmetic.
        {
            let mut r = report.lock().unwrap_or_else(PoisonError::into_inner);
            r.box_transform = kernel.box_transform();
            r.periodic_upwind_rate = self.periodic_upwind_rate;
        }
        Box::new(SeparableClient {
            kernel,
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

// Standalone declared line solves must not report successful nonfinite output.
fn solve_checked(kernel: &mut Kernel, b: &mut [f64], checked: bool) -> Result<(), SolveError> {
    if checked && b.iter().any(|v| !v.is_finite()) {
        return Err(invalid("nonfinite periodic upwind stage input"));
    }
    kernel.solve(b);
    if checked && b.iter().any(|v| !v.is_finite()) {
        return Err(invalid("nonfinite periodic upwind stage output"));
    }
    Ok(())
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
    fn new(
        grid: &DiffusionGrid,
        transform: BoxTransform,
        periodic_upwind_rate: Option<f64>,
    ) -> Self {
        match &*grid.geometry {
            Geometry::Sphere(rings) => Self::Sphere(SphereKernel::new(rings, grid.width)),
            Geometry::Cartesian { ny, nz, rates, .. } => Self::Box(BoxKernel::new(
                [grid.width, *ny, *nz],
                *rates,
                transform,
                periodic_upwind_rate,
            )),
        }
    }

    /// The box transform this kernel runs; `None` on a sphere grid.
    fn box_transform(&self) -> Option<BoxTransform> {
        match self {
            Self::Sphere(_) => None,
            Self::Box(k) => Some(k.transform),
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
            let shift = z - c;
            if k.cells[1..] == [1, 1]
                && !(shift.is_finite()
                    && shift > 0.0
                    && (shift + k.symbols[0].iter().copied().fold(0.0, f64::max)).is_finite())
            {
                return Err(CoreError::LinearSolve(
                    "periodic upwind shift outside binary64 range".into(),
                ));
            }
            k.shift = shift;
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

// Keep diffusion-only arithmetic unchanged. Scaling before complex division
// avoids squaring a large or subnormal stage shift.
#[inline]
fn divide_mode(value: &mut Complex64, re: f64, im: f64) {
    if im == 0.0 {
        *value /= re;
    } else {
        let scale = re.abs().max(im.abs());
        let (a, b) = (re / scale, im / scale);
        let w = *value / scale;
        let denom = a * a + b * b;
        *value = Complex64::new((w.re * a + w.im * b) / denom, (w.im * a - w.re * b) / denom);
    }
}

/// Complex values per batch of packed x-lines of [`BoxTransform::PairedReal`] (64 KiB).
const PAIRED_LINE_BATCH: usize = 4096;

/// Diagonal of the box's stage matrix in Fourier space: `z - c + e_x + e_y + e_z`.
/// Rows are the x-lines, `r = y + ny z`.
struct BoxKernel {
    cells: [usize; 3],
    symbols: [Vec<f64>; 3],
    // Imaginary part of the original W symbol, v*sin(theta), for the declared line.
    imag_x: Option<Vec<f64>>,
    shift: f64,
    plans: Vec<FftPair>,
    transform: BoxTransform,
    /// `Complex`: the whole field, x fastest. `PairedReal`: the Hermitian half spectra of the
    /// rows, x-mode-major: `buffer[k * rows + r]` is x-mode `k <= nx/2` of row `r`, so the
    /// y-lines are contiguous and the two rows of a pair are adjacent.
    buffer: Vec<Complex64>,
    /// `PairedReal`: one batch of packed lines, two real rows per complex line of length `nx`
    /// (empty for `Complex`).
    lines: Vec<Complex64>,
    line: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

impl BoxKernel {
    fn new(
        cells: [usize; 3],
        rates: [f64; 3],
        transform: BoxTransform,
        periodic_upwind_rate: Option<f64>,
    ) -> Self {
        let symbols = std::array::from_fn(|axis| {
            (0..cells[axis])
                .map(|k| rates[axis] * symbol(k, cells[axis]))
                .collect()
        });
        let imag_x = periodic_upwind_rate.filter(|v| *v != 0.0).map(|v| {
            let n = cells[0];
            (0..n)
                .map(|k| {
                    // Exact real DC/Nyquist and exact conjugate symmetry of partners.
                    if k == 0 || 2 * k == n {
                        0.0
                    } else if k <= n / 2 {
                        v * (2.0 * PI * k as f64 / n as f64).sin()
                    } else {
                        -v * (2.0 * PI * (n - k) as f64 / n as f64).sin()
                    }
                })
                .collect()
        });
        let plans: Vec<_> = cells.iter().map(|&n| plans(n)).collect();
        let scratch = plans
            .iter()
            .flat_map(|(f, i)| [f.get_inplace_scratch_len(), i.get_inplace_scratch_len()])
            .max()
            .unwrap_or(0);
        let [nx, ny, nz] = cells;
        let (buffer, lines) = match transform {
            BoxTransform::Complex => (cells.iter().product(), 0),
            BoxTransform::PairedReal => {
                let rows = ny * nz;
                let packed = rows.div_ceil(2);
                let batch = (PAIRED_LINE_BATCH / nx).max(1).min(packed);
                ((nx / 2 + 1) * rows, batch * nx)
            }
        };
        Self {
            cells,
            symbols,
            imag_x,
            shift: f64::NAN,
            plans,
            transform,
            buffer: vec![Complex64::default(); buffer],
            lines: vec![Complex64::default(); lines],
            line: vec![Complex64::default(); ny.max(nz)],
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
        match self.transform {
            BoxTransform::Complex => self.solve_complex(b),
            BoxTransform::PairedReal => self.solve_paired(b),
        }
    }

    /// [`BoxTransform::PairedReal`] (THEORY.md §2.3 (ii) applied to the box's rows; the
    /// algebra is in `research/kernel_spectral_foundation_20261124/ks_n7/NOTES.txt`).
    fn solve_paired(&mut self, b: &mut [f64]) {
        let [nx, ny, nz] = self.cells;
        let rows = ny * nz;
        let half = nx / 2 + 1;
        let packed = rows.div_ceil(2);
        let batch = self.lines.len() / nx;
        let forward_x = Arc::clone(&self.plans[0].0);
        let inverse_x = Arc::clone(&self.plans[0].1);
        // x: pack rows 2q and 2q + 1 as u + i v, transform, separate the Hermitian halves by
        // R Z(m) = conj Z(nx - m): U = (Z + R Z) / 2, V = (Z - R Z) / (2i). A last odd row is
        // packed alone (v = 0) and projected the same way.
        for first in (0..packed).step_by(batch) {
            let count = batch.min(packed - first);
            let (start, end) = (2 * first, (2 * (first + count)).min(rows));
            let lines = &mut self.lines[..count * nx];
            for (row, line) in (start..end).step_by(2).zip(lines.chunks_exact_mut(nx)) {
                let u = &b[row * nx..(row + 1) * nx];
                if row + 1 < rows {
                    let v = &b[(row + 1) * nx..(row + 2) * nx];
                    for ((dst, &re), &im) in line.iter_mut().zip(u).zip(v) {
                        *dst = Complex64::new(re, im);
                    }
                } else {
                    for (dst, &re) in line.iter_mut().zip(u) {
                        *dst = Complex64::new(re, 0.0);
                    }
                }
            }
            forward_x.process_with_scratch(lines, &mut self.scratch);
            for k in 0..half {
                let partner = if k == 0 { 0 } else { nx - k };
                let column = &mut self.buffer[k * rows + start..k * rows + end];
                for (pair, line) in column.chunks_mut(2).zip(lines.chunks_exact(nx)) {
                    let zm = line[k];
                    let zr = line[partner].conj();
                    pair[0] = (zm + zr) * 0.5;
                    if let Some(v) = pair.get_mut(1) {
                        // (Z - R Z) / (2i) = (d.im - i d.re) / 2, two exact halvings.
                        let d = zm - zr;
                        *v = Complex64::new(0.5 * d.im, -0.5 * d.re);
                    }
                }
            }
        }
        // y: contiguous lines of the x-mode-major half, one batched call; z: gathered inside
        // each x-mode block.
        let forward_y = Arc::clone(&self.plans[1].0);
        forward_y.process_with_scratch(&mut self.buffer, &mut self.scratch);
        self.transform_half_z(false);
        // Same association as the complex transform: ((z - c + e_x) + e_y) + e_z.
        let shift = self.shift;
        let [sx, sy, sz] = &self.symbols;
        if let Some(imag) = &self.imag_x {
            // The upwind declaration is one-dimensional; y,z each have length one.
            debug_assert_eq!(rows, 1);
            for ((value, &ex), &im) in self.buffer.iter_mut().zip(sx).zip(imag) {
                divide_mode(value, shift + ex, im);
            }
        } else {
            for (block, &ex) in self.buffer.chunks_exact_mut(rows).zip(sx) {
                let base = shift + ex;
                for (plane, &ez) in block.chunks_exact_mut(ny).zip(sz) {
                    for (value, &ey) in plane.iter_mut().zip(sy) {
                        *value /= base + ey + ez;
                    }
                }
            }
        }
        self.transform_half_z(true);
        let inverse_y = Arc::clone(&self.plans[1].1);
        inverse_y.process_with_scratch(&mut self.buffer, &mut self.scratch);
        // Rebuild Z = X_{2q} + i X_{2q+1} from the half spectra (the DC and Nyquist values
        // projected to their real parts), invert along x, split real and imaginary parts.
        let scale = 1.0 / (nx * rows) as f64;
        for first in (0..packed).step_by(batch) {
            let count = batch.min(packed - first);
            let (start, end) = (2 * first, (2 * (first + count)).min(rows));
            let lines = &mut self.lines[..count * nx];
            for k in 0..half {
                let column = &self.buffer[k * rows + start..k * rows + end];
                for (pair, line) in column.chunks(2).zip(lines.chunks_exact_mut(nx)) {
                    let u = pair[0];
                    let v = pair.get(1).copied().unwrap_or_default();
                    if k == 0 || 2 * k == nx {
                        line[k] = Complex64::new(u.re, v.re);
                    } else {
                        line[k] = Complex64::new(u.re - v.im, u.im + v.re);
                        line[nx - k] = Complex64::new(u.re + v.im, v.re - u.im);
                    }
                }
            }
            inverse_x.process_with_scratch(lines, &mut self.scratch);
            for (row, line) in (start..end).step_by(2).zip(lines.chunks_exact(nx)) {
                for (dst, z) in b[row * nx..(row + 1) * nx].iter_mut().zip(line) {
                    *dst = z.re * scale;
                }
                if row + 1 < rows {
                    for (dst, z) in b[(row + 1) * nx..(row + 2) * nx].iter_mut().zip(line) {
                        *dst = z.im * scale;
                    }
                }
            }
        }
    }

    /// Transform the x-mode-major half spectra along z (3-D boxes): inside each x-mode block
    /// `[k * rows, (k + 1) * rows)` the z-lines have stride `ny`.
    fn transform_half_z(&mut self, inverse: bool) {
        let [_, ny, nz] = self.cells;
        if nz == 1 {
            return;
        }
        let fft = Arc::clone(if inverse {
            &self.plans[2].1
        } else {
            &self.plans[2].0
        });
        let line = &mut self.line[..nz];
        for block in self.buffer.chunks_exact_mut(ny * nz) {
            for y in 0..ny {
                for (z, dst) in line.iter_mut().enumerate() {
                    *dst = block[z * ny + y];
                }
                fft.process_with_scratch(line, &mut self.scratch);
                for (z, src) in line.iter().enumerate() {
                    block[z * ny + y] = *src;
                }
            }
        }
    }

    fn solve_complex(&mut self, b: &mut [f64]) {
        let [nx, ny, _] = self.cells;
        for (dst, &v) in self.buffer.iter_mut().zip(b.iter()) {
            *dst = Complex64::new(v, 0.0);
        }
        let forward_x = Arc::clone(&self.plans[0].0);
        forward_x.process_with_scratch(&mut self.buffer, &mut self.scratch);
        self.transform_strided(1, false);
        self.transform_strided(2, false);
        let [sx, sy, sz] = &self.symbols;
        if let Some(imag) = &self.imag_x {
            debug_assert_eq!(self.cells[1..], [1, 1]);
            for ((value, &ex), &im) in self.buffer.iter_mut().zip(sx).zip(imag) {
                divide_mode(value, self.shift + ex, im);
            }
        } else {
            for (index, value) in self.buffer.iter_mut().enumerate() {
                let x = index % nx;
                let y = index / nx % ny;
                let z = index / (nx * ny);
                *value /= self.shift + sx[x] + sy[y] + sz[z];
            }
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
        let (mut x_max, mut j_max, mut b_max, mut residual) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        for (((r, &b), &xi), &jx) in self.work.iter_mut().zip(&self.rhs).zip(x).zip(&self.jx) {
            *r = b - (self.z * xi - jx);
            if !r.is_finite() || !b.is_finite() || !xi.is_finite() || !jx.is_finite() {
                return Err(CoreError::LinearSolve(
                    "nonfinite separable residual".into(),
                ));
            }
            x_max = x_max.max(xi.abs());
            j_max = j_max.max(jx.abs());
            b_max = b_max.max(b.abs());
            residual = residual.max(r.abs());
        }
        let scale = self.z * x_max + j_max + b_max;
        if !scale.is_finite() {
            return Err(CoreError::LinearSolve(
                "nonfinite separable residual scale".into(),
            ));
        }
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
