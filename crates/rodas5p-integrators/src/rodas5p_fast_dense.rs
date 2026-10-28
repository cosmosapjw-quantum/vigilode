//! The continuous extension and the W-filtered interior error control of
//! the fast dense and banded RODAS5P drivers (wave-3 node `p0-fastdense`,
//! plan §3 P0 and §5 T3).
//!
//! The run is [`crate::integrate_rodas5p_fast_observed`] (or its banded
//! twin) on the output `[t0, t1]`, with a step sink that
//!
//! * records every accepted step: its end time, its state and its eight
//!   transformed stage rows `u_i = sum_j Gamma_ij k_j` in flat storage
//!   ([`FastDenseSegments`]);
//! * under [`FastInteriorControl::WFilteredEnforce`], on every attempt whose
//!   embedded error is at most one (the [`crate::InteriorSampling::EveryStep`]
//!   rule), forms the W-filtered interior estimate
//!   `E = (I - h gamma J0)^{-1} (h/2) d(1/2) = W^{-1}((h/2) d) / (h gamma)`
//!   with the workspace's own factors of `W = I/(h gamma) - J0`: one
//!   right-hand side at the midpoint and one back-solve, no Jacobian and no
//!   factorization. The controller sees `max(embedded error, estimate)`
//!   for acceptance and for the step proposal. An error in the midpoint
//!   right-hand side or the back-solve rejects the attempt like a stage
//!   error, as does the sequential route under an enforcing mode; the retry
//!   reuses the state's Jacobian.
//!
//! The continuous extension in the `u` coordinates uses the snapshot's
//! `H` rows directly: the sequential path's `D = H Gamma` applied to its
//! `k` rows is `H` applied to `u = Gamma k`. The polynomial, θ and the
//! midpoint derivative `(y_new - y_old + (d1 + d2)/4)/h` are those of
//! `Rodas5pDenseSegment::evaluate` and `scaled_midpoint_defect`, evaluated
//! into caller buffers.
//!
//! Allocations: setup allocates the segment store (three vectors), the
//! output schedule and, under `WFilteredEnforce`, one `2 n` scratch buffer.
//! Per step, on the in-place LUs (dense storage up to
//! [`crate::RODAS5P_FAST_SMALL_LU_MAX`] rows or sparse, and the banded
//! pipeline), nothing is allocated except the store's amortized doubling
//! growth. On the faer LU (dense storage above that size) the back-solve
//! allocates its solution once per sample, as the fast driver's own stage
//! solves do there.

use rodas5p_core::{
    CoreError, CoreResult, WorkCounters, rodas5p_coefficients, scaled_error_component,
};
use serde::Serialize;

use crate::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule,
    rodas5p_fast::{
        BandedJacobian, BandedKernel, BandedWork, DriverPartial, Rodas5pFastOptions,
        Rodas5pFastResult, StepSink, Workspace, integrate_fast, require_atol_vector,
    },
};

/// Identifier of the dense-storage fast driver with a continuous extension.
pub const RODAS5P_FAST_DENSE_DRIVER_ID: &str = "rodas5p-fast-dense-v1";

/// Identifier of the banded fast driver with a continuous extension.
pub const RODAS5P_FAST_BANDED_DENSE_DRIVER_ID: &str = "rodas5p-fast-banded-dense-v1";

/// The number of RODAS5P stages (columns of the dense-output rows).
const STAGES: usize = 8;

/// Interior (continuous-extension) error control of the fast dense drivers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FastInteriorControl {
    /// No interior estimate: the run is the fast driver's, bit for bit.
    Off,
    /// The W-filtered estimate on every attempt the embedded estimate
    /// accepts; the controller sees `max(embedded error, estimate)`.
    #[default]
    WFilteredEnforce,
}

/// Work and outcome of the interior estimates of one run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct FastInteriorWork {
    /// Estimates formed (one per attempt the embedded estimate accepted
    /// whose midpoint right-hand side and back-solve succeeded).
    pub samples: u64,
    /// Attempts the embedded estimate accepted and the interior estimate
    /// rejected (included in the run's `rejected_steps`).
    pub interior_rejections: u64,
    /// Right-hand-side calls of the estimates (attempted calls; also in the
    /// run counters, since the estimates decide steps).
    pub rhs_calls: u64,
    /// Back-solves with the step's factors (attempted; also in the run
    /// counters as linear and direct solves, and on the banded route in
    /// [`BandedWork::solve_operations`]).
    pub back_solves: u64,
    /// The largest estimate of an accepted step, in tolerance units (0 when
    /// none was formed). An estimate, not a bound.
    pub max_estimate: f64,
}

/// The continuous extension of a fast run in flat storage: `m` accepted
/// steps, the times `t_0 < .. < t_m`, the states at those times
/// (`(m + 1) n` values) and every step's eight transformed stage rows
/// (`m · 8 · n` values, step-major, then stage, then component).
#[derive(Clone, Debug, PartialEq)]
pub struct FastDenseSegments {
    n: usize,
    t: Vec<f64>,
    states: Vec<f64>,
    stages: Vec<f64>,
    /// The snapshot's `H` rows (3 x 8), applied to the `u` rows.
    dense_h: [[f64; STAGES]; 3],
}

impl FastDenseSegments {
    /// The empty extension at `(t0, y0)`, with room for `steps` steps.
    fn new(t0: f64, y0: &[f64], steps: usize) -> CoreResult<Self> {
        let coefficients = rodas5p_coefficients()?;
        let h = &coefficients.dense_h;
        if h.nrows() != 3 || h.ncols() != STAGES || coefficients.stages() != STAGES {
            return Err(CoreError::Coefficients(
                "RODAS5P dense-output coefficient shape mismatch".into(),
            ));
        }
        let mut dense_h = [[0.0; STAGES]; 3];
        for (r, row) in dense_h.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = h[(r, j)];
            }
        }
        let n = y0.len();
        let mut t = Vec::with_capacity(16);
        t.push(t0);
        let mut states = Vec::with_capacity((steps + 1) * n);
        states.extend_from_slice(y0);
        Ok(Self {
            n,
            t,
            states,
            stages: Vec::with_capacity(steps * STAGES * n),
            dense_h,
        })
    }

    fn push(&mut self, t_new: f64, y_new: &[f64], u: &[f64]) {
        self.t.push(t_new);
        self.states.extend_from_slice(y_new);
        self.stages.extend_from_slice(u);
    }

    /// The number of accepted steps `m`.
    pub fn len(&self) -> usize {
        self.t.len() - 1
    }

    /// Whether no step was accepted.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The state dimension `n`.
    pub fn dimension(&self) -> usize {
        self.n
    }

    /// The times `t_0, .., t_m` (the initial time and every accepted
    /// step's end).
    pub fn times(&self) -> &[f64] {
        &self.t
    }

    /// The stored state at `times()[k]`, `k <= len()`.
    pub fn state(&self, k: usize) -> &[f64] {
        &self.states[k * self.n..(k + 1) * self.n]
    }

    /// The start of step `k < len()`.
    pub fn t_old(&self, k: usize) -> f64 {
        self.t[k]
    }

    /// The end of step `k < len()`.
    pub fn t_new(&self, k: usize) -> f64 {
        self.t[k + 1]
    }

    /// The eight transformed stage rows of step `k < len()` (`8 n`).
    pub fn stage_rows(&self, k: usize) -> &[f64] {
        let width = STAGES * self.n;
        &self.stages[k * width..(k + 1) * width]
    }

    fn affine_scalar_packet(
        &self,
        k: usize,
    ) -> CoreResult<crate::affine_event_certificate::RepresentedAffineSegment> {
        if self.n != 1 || k >= self.len() {
            return Err(CoreError::InvalidInput(
                "affine event certificate needs one scalar accepted segment".into(),
            ));
        }
        let mut stages = [0.0; STAGES];
        stages.copy_from_slice(self.stage_rows(k));
        crate::affine_event_certificate::RepresentedAffineSegment::from_native(
            [self.t_old(k), self.t_new(k)],
            [self.state(k)[0], self.state(k + 1)[0]],
            self.dense_h,
            stages,
            k,
        )
    }

    /// Opt-in whole-slab scalar-affine event certificate from the stored
    /// accepted native segment. The caller supplies the source/equation
    /// binding and a justified incoming exact-IVP error; the adaptive local
    /// error estimate supplies neither of these premises.
    pub fn certify_affine_scalar_segment(
        &self,
        k: usize,
        spec: &crate::affine_event_certificate::AffineEventSpec,
        initial_error: f64,
    ) -> CoreResult<crate::affine_event_certificate::AffineEventCertificate> {
        crate::affine_event_certificate::AffineEventCertificate::certify(
            self.affine_scalar_packet(k)?,
            spec,
            initial_error,
        )
    }

    /// Carry a prior scalar certificate's endpoint error to an adjacent
    /// accepted segment. Source, clock, model, event and epoch must agree.
    pub fn certify_affine_scalar_segment_with_carry(
        &self,
        k: usize,
        spec: &crate::affine_event_certificate::AffineEventSpec,
        carry: &crate::affine_event_certificate::AffineEventCarry,
    ) -> CoreResult<crate::affine_event_certificate::AffineEventCertificate> {
        carry.certify_next(self.affine_scalar_packet(k)?, spec)
    }

    /// Certify the actual native returned center, after verifying this
    /// segment's complete represented input bits against its certificate.
    pub fn query_affine_scalar_certificate(
        &self,
        certificate: &crate::affine_event_certificate::AffineEventCertificate,
        spec: &crate::affine_event_certificate::AffineEventSpec,
        t: f64,
    ) -> CoreResult<crate::affine_event_certificate::AffineEventQuery> {
        crate::affine_event_certificate::check_affine_event_arithmetic()?;
        certificate.check_segment(&self.affine_scalar_packet(certificate.segment_index())?)?;
        let [t0, t1] = certificate.times();
        if !(t.is_finite() && t0 <= t && t <= t1) {
            return Err(CoreError::InvalidInput(
                "affine event query is outside its certified segment".into(),
            ));
        }
        let mut center = [0.0];
        self.evaluate_into(t, &mut center)?;
        certificate.query(spec, t, center[0])
    }

    /// Keep the first `steps` steps and drop the rest (no-op when
    /// `steps >= len()`).
    pub fn truncate(&mut self, steps: usize) {
        if steps < self.len() {
            self.t.truncate(steps + 1);
            self.states.truncate((steps + 1) * self.n);
            self.stages.truncate(steps * STAGES * self.n);
        }
    }

    /// The continuous extension at `t` into `out` (length `n`), for `t` in
    /// `[t_0, t_m]`. At a step endpoint it is the stored state bit for bit;
    /// otherwise θ is formed as `Rodas5pDenseSegment::evaluate` forms it on
    /// the step that contains `t` (the first whose end is at least `t`).
    pub fn evaluate_into(&self, t: f64, out: &mut [f64]) -> CoreResult<()> {
        if out.len() != self.n {
            return Err(CoreError::Dimension(
                "fast dense-output buffer shape mismatch".into(),
            ));
        }
        let first = self.t[0];
        let last = self.t[self.t.len() - 1];
        if !(t >= first && t <= last) {
            return Err(CoreError::InvalidInput(
                "fast dense output evaluated outside the integrated span".into(),
            ));
        }
        if t == first {
            out.copy_from_slice(self.state(0));
            return Ok(());
        }
        let k = self.t[1..].partition_point(|&end| end < t);
        let (t_old, t_new) = (self.t_old(k), self.t_new(k));
        if t == t_new {
            out.copy_from_slice(self.state(k + 1));
            return Ok(());
        }
        let theta = (t - t_old) / (t_new - t_old);
        if theta == 0.0 {
            out.copy_from_slice(self.state(k));
            return Ok(());
        }
        polynomial_into(
            &self.dense_h,
            self.state(k),
            self.state(k + 1),
            self.stage_rows(k),
            theta,
            out,
        );
        if out.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::NonFinite(
                "RODAS5P fast dense output contains NaN/Inf".into(),
            ))
        }
    }
}

/// `d_r[c] = sum_j H[r][j] u_j[c]` for the three rows, component `c`.
#[inline]
fn dense_rows(dense_h: &[[f64; STAGES]; 3], u: &[f64], n: usize, c: usize) -> [f64; 3] {
    let mut d = [0.0; 3];
    for (r, row) in dense_h.iter().enumerate() {
        let mut sum = 0.0;
        for (j, coefficient) in row.iter().enumerate() {
            sum += coefficient * u[j * n + c];
        }
        d[r] = sum;
    }
    d
}

/// The RODAS5P quartic continuous extension at `theta` in `(0, 1)`:
/// `(1 - θ) y_old + θ (y_new + (1 - θ)(d0 + θ (d1 + θ d2)))`.
fn polynomial_into(
    dense_h: &[[f64; STAGES]; 3],
    y_old: &[f64],
    y_new: &[f64],
    u: &[f64],
    theta: f64,
    out: &mut [f64],
) {
    let n = y_old.len();
    for (c, value) in out.iter_mut().enumerate() {
        *value = polynomial_component(y_old[c], y_new[c], dense_rows(dense_h, u, n, c), theta);
    }
}

/// One component of the continuous extension at `theta`; the single form
/// used by [`FastDenseSegments::evaluate_into`] and the estimate's midpoint.
#[inline]
fn polynomial_component(y_old: f64, y_new: f64, [d0, d1, d2]: [f64; 3], theta: f64) -> f64 {
    let complement = 1.0 - theta;
    complement * y_old + theta * (y_new + complement * (d0 + theta * (d1 + theta * d2)))
}

/// The step sink of the fast dense drivers.
struct DenseSink {
    control: FastInteriorControl,
    segments: FastDenseSegments,
    interior: FastInteriorWork,
    /// The midpoint state and then `(h/2) d`, overwritten by the estimate
    /// (`2 n`, one allocation; empty under `Off`).
    scratch: Vec<f64>,
}

impl DenseSink {
    fn new(control: FastInteriorControl, t0: f64, y0: &[f64]) -> CoreResult<Self> {
        let n = y0.len();
        let scratch = match control {
            FastInteriorControl::Off => Vec::new(),
            FastInteriorControl::WFilteredEnforce => vec![0.0; 2 * n],
        };
        Ok(Self {
            control,
            // Two steps of room for the states and stage rows (the times
            // start at 16): each vector then doubles, so it reallocates about
            // log2(m) times. These initial allocations are setup, charged
            // with the growth against test (g)'s 3 ceil(log2(m + 1)) budget.
            segments: FastDenseSegments::new(t0, y0, 2)?,
            interior: FastInteriorWork::default(),
            scratch,
        })
    }

    /// The WRMS of the W-filtered estimate of the attempt in `work`.
    #[allow(clippy::too_many_arguments)]
    fn estimate(
        &mut self,
        problem: &OdeProblem,
        work: &mut Workspace,
        t: f64,
        y: &[f64],
        h: f64,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        let n = work.n;
        let dense_h = &self.segments.dense_h;
        let (midpoint, defect) = self.scratch.split_at_mut(n);
        // The midpoint state and u'(t_m) (M = I).
        for c in 0..n {
            let d = dense_rows(dense_h, &work.u, n, c);
            midpoint[c] = polynomial_component(y[c], work.y_new[c], d, 0.5);
            defect[c] = (work.y_new[c] - y[c] + 0.25 * (d[1] + d[2])) / h;
        }
        self.interior.rhs_calls += 1;
        // The flow at the midpoint, into the workspace's stage buffer (free
        // between attempts).
        problem.eval_rhs_into(t + 0.5 * h, midpoint, &mut work.stage_rhs, counters)?;
        for (d, f) in defect.iter_mut().zip(&work.stage_rhs) {
            *d = 0.5 * h * (*d - f);
        }
        self.interior.back_solves += 1;
        counters.linear_solves += 1;
        counters.direct_solve_calls += 1;
        work.solve_into(defect)?;
        // The step's scale atol_i + rtol max(|y|, |y_new|), as the
        // embedded norm forms it (p0-atol: a per-component atol when set).
        let shift = h * work.gamma;
        let mut sum = 0.0;
        for (c, ((e, a), b)) in defect.iter().zip(y).zip(&work.y_new).enumerate() {
            let atol_c = if work.atol_vector.is_empty() {
                atol
            } else {
                work.atol_vector[c]
            };
            let z = scaled_error_component(*e / shift, atol_c + rtol * a.abs().max(b.abs()));
            sum += z * z;
        }
        let norm = (sum / n as f64).sqrt();
        self.interior.samples += 1;
        Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        })
    }
}

impl StepSink for DenseSink {
    fn after_attempt(
        &mut self,
        problem: &OdeProblem,
        work: &mut Workspace,
        t: f64,
        y: &[f64],
        h: f64,
        error: f64,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        if self.control == FastInteriorControl::Off {
            return Ok(error);
        }
        let estimate = self.estimate(problem, work, t, y, h, atol, rtol, counters)?;
        let seen = error.max(estimate);
        if seen <= 1.0 {
            self.interior.max_estimate = self.interior.max_estimate.max(estimate);
        } else {
            self.interior.interior_rejections += 1;
        }
        Ok(seen)
    }

    fn accepted(&mut self, work: &Workspace, t_new: f64) {
        self.segments.push(t_new, &work.y_new, &work.u);
    }
}

/// The run of [`integrate_rodas5p_fast_dense_observed`] and its banded twin.
#[derive(Clone, Debug)]
pub struct Rodas5pFastDenseResult {
    /// The fast driver's result on the output `[t0, t1]` (its `driver` is
    /// [`RODAS5P_FAST_DENSE_DRIVER_ID`] or
    /// [`RODAS5P_FAST_BANDED_DENSE_DRIVER_ID`]).
    pub fast: Rodas5pFastResult,
    /// Every accepted step's continuous extension.
    pub segments: FastDenseSegments,
    /// The interior estimates' work.
    pub interior: FastInteriorWork,
    /// Where and why the run stopped, when it stopped before `t1`.
    pub partial: Option<DriverPartial>,
    /// The band work of the banded twin (`None` on dense storage).
    pub banded: Option<BandedWork>,
}

fn span_schedule((t0, t1): (f64, f64)) -> CoreResult<OutputSchedule> {
    OutputSchedule::new(if t1 > t0 { vec![t0, t1] } else { vec![t0] })
}

fn run_dense(
    problem: &OdeProblem,
    band: Option<&BandedJacobian>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol_vector: &[f64],
) -> CoreResult<Rodas5pFastDenseResult> {
    if y0.len() != problem.dimension {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast integration input".into(),
        ));
    }
    let output = span_schedule(t_span)?;
    let mut sink = DenseSink::new(interior, t_span.0, y0)?;
    let (mut fast, banded, partial) = integrate_fast(
        problem,
        band.map(|band| (band, BandedKernel::Indexed)),
        t_span,
        y0,
        adaptive,
        &output,
        Rodas5pFastOptions::default(),
        None,
        atol_vector,
        &mut sink,
    )?;
    fast.driver = if band.is_some() {
        RODAS5P_FAST_BANDED_DENSE_DRIVER_ID
    } else {
        RODAS5P_FAST_DENSE_DRIVER_ID
    };
    Ok(Rodas5pFastDenseResult {
        fast,
        segments: sink.segments,
        interior: sink.interior,
        partial,
        banded,
    })
}

/// [`crate::integrate_rodas5p_fast_observed`] on the output `[t0, t1]` with
/// the continuous extension of every accepted step and the interior
/// control `interior`. Under [`FastInteriorControl::Off`] the run is the
/// fast driver's bit for bit. Requires the identity mass matrix.
pub fn integrate_rodas5p_fast_dense_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
) -> CoreResult<Rodas5pFastDenseResult> {
    run_dense(problem, None, t_span, y0, adaptive, interior, &[])
}

/// [`integrate_rodas5p_fast_dense_observed`] on the banded pipeline of
/// [`crate::integrate_rodas5p_fast_banded_observed`] (the `Indexed` kernel).
/// Requires `lower, upper < n`.
pub fn integrate_rodas5p_fast_banded_dense_observed(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
) -> CoreResult<Rodas5pFastDenseResult> {
    run_banded_dense(problem, band, t_span, y0, adaptive, interior, &[])
}

fn run_banded_dense(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol_vector: &[f64],
) -> CoreResult<Rodas5pFastDenseResult> {
    let n = problem.dimension;
    if band.lower >= n.max(1) || band.upper >= n.max(1) {
        return Err(CoreError::InvalidInput(
            "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
        ));
    }
    run_dense(
        problem,
        Some(band),
        t_span,
        y0,
        adaptive,
        interior,
        atol_vector,
    )
}

/// [`integrate_rodas5p_fast_dense_observed`] with a per-component absolute
/// tolerance (wave-3 node `p0-atol`, ruling R-C1): component `i` of the
/// embedded norm and of the W-filtered estimate is scaled by
/// `atol[i] + rtol max(|y_i|, |y_new,i|)`; `adaptive.atol` is validated but
/// unused. `atol` must have `n` finite nonnegative entries. With every entry
/// equal to `adaptive.atol` the run is bit for bit the scalar run.
pub fn integrate_rodas5p_fast_dense_observed_with_atol(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol: &[f64],
) -> CoreResult<Rodas5pFastDenseResult> {
    require_atol_vector(problem.dimension, atol)?;
    run_dense(problem, None, t_span, y0, adaptive, interior, atol)
}

/// [`integrate_rodas5p_fast_banded_dense_observed`] with a per-component
/// absolute tolerance; see [`integrate_rodas5p_fast_dense_observed_with_atol`].
pub fn integrate_rodas5p_fast_banded_dense_observed_with_atol(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol: &[f64],
) -> CoreResult<Rodas5pFastDenseResult> {
    require_atol_vector(problem.dimension, atol)?;
    run_banded_dense(problem, band, t_span, y0, adaptive, interior, atol)
}

#[cfg(test)]
mod tests {
    use rodas5p_core::{LinearMethod, LinearSolverConfig, WorkCounters, rodas5p_coefficients};

    use super::*;
    use crate::{
        FastLuPolicy, dense_output_v2::rodas5p_dense_output_parts, robertson_problem,
        sequential_step,
    };

    /// Test (c): one step from the same `(t, y, h)` on Robertson. The fast
    /// `u` rows are `u = Gamma k` of the sequential stage rows `k` that
    /// `Rodas5pDenseSegment` stores (the coordinates differ; the transform is
    /// applied here), within 1e-12 relative to the magnitude of the summed
    /// terms `sum_j |Gamma_ij k_j|` (cancellation makes a per-entry relative
    /// test ill-conditioned: entries of 3.4e-9 are sums of 4e-5 terms). The
    /// fast extension equals the sequential one at interior θ within 1e-12
    /// of the step's state magnitude. Gamma is lower triangular; its numeric
    /// inverse holds roundoff (1e-17) above the diagonal, which is skipped.
    #[test]
    fn stage_rows_match_sequential_stages() {
        let (problem, y0) = robertson_problem().unwrap();
        let (t, h, atol, rtol) = (0.0, 1.0e-3, 1.0e-10, 1.0e-6);
        let n = problem.dimension;
        let direct = LinearSolverConfig {
            method: LinearMethod::Direct,
            ..LinearSolverConfig::default()
        };
        let mut counters = WorkCounters::default();
        let step = sequential_step(
            &problem,
            t,
            &y0,
            h,
            &direct,
            None,
            atol,
            rtol,
            false,
            &mut counters,
        )
        .unwrap();
        let mut work = Workspace::new(n, FastLuPolicy::Legacy).unwrap();
        let mut fast_counters = WorkCounters::default();
        work.attempt_for_tests(&problem, t, &y0, h, atol, rtol, &mut fast_counters)
            .unwrap();
        let gamma = &rodas5p_coefficients().unwrap().gamma_matrix;
        let mut worst = 0.0_f64;
        for i in 0..STAGES {
            for c in 0..n {
                let expected: f64 = (0..=i).map(|j| gamma[(i, j)] * step.stages[j][c]).sum();
                let got = work.u[i * n + c];
                let terms: f64 = (0..=i)
                    .map(|j| (gamma[(i, j)] * step.stages[j][c]).abs())
                    .sum();
                let scale = terms.max(got.abs()).max(f64::MIN_POSITIVE);
                worst = worst.max((got - expected).abs() / scale);
            }
        }
        assert!(worst <= 1.0e-12, "u vs Gamma k: {worst:e}");
        for (a, b) in work.y_new.iter().zip(&step.y_new) {
            assert!((a - b).abs() <= 1.0e-12 * a.abs().max(b.abs()), "{a} {b}");
        }
        // The extension in u coordinates (H) against D = H Gamma on k.
        let mut segments = FastDenseSegments::new(t, &y0, 1).unwrap();
        segments.push(t + h, &work.y_new, &work.u);
        let mut out = vec![0.0; n];
        for theta in [0.1, 0.25, 0.5, 0.75, 0.9] {
            segments.evaluate_into(t + theta * h, &mut out).unwrap();
            let th = ((t + theta * h) - t) / ((t + h) - t);
            let expected =
                rodas5p_dense_output_parts(&step.y_old, &work.y_new, &step.stages, th).unwrap();
            for (c, (a, b)) in out.iter().zip(&expected).enumerate() {
                let scale = step.y_old[c].abs().max(work.y_new[c].abs()).max(a.abs());
                assert!((a - b).abs() <= 1.0e-12 * scale, "θ {theta} c {c}: {a} {b}");
            }
        }
    }
}
