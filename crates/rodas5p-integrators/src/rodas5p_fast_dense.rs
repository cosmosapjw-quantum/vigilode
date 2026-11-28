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
        AcceptControl, AcceptedStepView, BandedJacobian, BandedKernel, BandedWork, DriverPartial,
        Rodas5pFastOptions, Rodas5pFastResult, StepSink, Workspace, integrate_fast,
        require_atol_vector,
    },
};

/// Identifier of the dense-storage fast driver with a continuous extension.
pub const RODAS5P_FAST_DENSE_DRIVER_ID: &str = "rodas5p-fast-dense-v1";

/// Identifier of the banded fast driver with a continuous extension.
pub const RODAS5P_FAST_BANDED_DENSE_DRIVER_ID: &str = "rodas5p-fast-banded-dense-v1";

/// Identifier of the banded fast driver with a continuous extension on the
/// [`BandedKernel::Slices`] kernel (loop L1 node V22).
pub const RODAS5P_FAST_BANDED_DENSE_SLICES_DRIVER_ID: &str = "rodas5p-fast-banded-dense-v1-slices";

/// Identifier of the banded fast driver with a continuous extension on the
/// [`BandedKernel::FixedWidth2`] kernel (`lower = upper = 2`; stage-lattice task SL-N12).
pub const RODAS5P_FAST_BANDED_DENSE_FIXED2_DRIVER_ID: &str = "rodas5p-fast-banded-dense-v1-fixed2";

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

    /// One component of the original accepted dense packet, retaining exact
    /// represented input bits and native provenance. Not a physical IVP bound.
    #[cfg(feature = "polynomial-events")]
    pub fn represented_component(
        &self,
        k: usize,
        c: usize,
    ) -> CoreResult<crate::affine_event_certificate::RepresentedAffineSegment> {
        if k >= self.len() || c >= self.n {
            return Err(CoreError::InvalidInput("dense component index".into()));
        }
        let mut stages = [0.; STAGES];
        for (j, u) in stages.iter_mut().enumerate() {
            *u = self.stage_rows(k)[j * self.n + c];
        }
        crate::affine_event_certificate::RepresentedAffineSegment::from_native(
            [self.t_old(k), self.t_new(k)],
            [self.state(k)[c], self.state(k + 1)[c]],
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

/// The RODAS5P third-degree continuous extension at `theta` in `(0, 1)`:
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
type DenseAcceptFn<'a> = dyn FnMut(&FastDenseSegments) -> AcceptControl + 'a;

/// Bounded native dense packet capture; allocation occurs only at construction.
/// Stops after the last capacity-owned accepted step if the run is unfinished.
pub struct FastDenseCapture {
    segments: FastDenseSegments,
    capacity: usize,
}
impl FastDenseCapture {
    /// Allocate bounded packet storage for one native IVP.
    pub fn new(n: usize, capacity: usize) -> CoreResult<Self> {
        if n == 0
            || capacity == 0
            || capacity > 4096
            || n.checked_mul(capacity)
                .and_then(|v| v.checked_mul(8))
                .is_none_or(|v| v > isize::MAX as usize / 8)
        {
            return Err(CoreError::InvalidInput("dense capture shape".into()));
        }
        let mut segments = FastDenseSegments::new(0., &vec![0.; n], capacity)?;
        segments
            .t
            .reserve_exact((capacity + 1).saturating_sub(segments.t.len()));
        Ok(Self { segments, capacity })
    }
    pub(crate) fn reset(&mut self, t: f64, y: &[f64]) -> CoreResult<()> {
        if y.len() != self.segments.n {
            return Err(CoreError::InvalidInput("dense capture dimension".into()));
        }
        self.segments.t.clear();
        self.segments.t.push(t);
        self.segments.states.clear();
        self.segments.states.extend_from_slice(y);
        self.segments.stages.clear();
        Ok(())
    }
    /// Retained accepted prefix of the most recent run.
    pub fn segments(&self) -> &FastDenseSegments {
        &self.segments
    }
}
impl StepSink for FastDenseCapture {
    fn after_attempt(
        &mut self,
        _problem: &OdeProblem,
        _work: &mut Workspace,
        _t: f64,
        _y: &[f64],
        _h: f64,
        error: f64,
        _atol: f64,
        _rtol: f64,
        _counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        Ok(error)
    }
    fn accepted(&mut self, work: &Workspace, t_new: f64) {
        debug_assert!(self.segments.len() < self.capacity);
        self.segments.push(t_new, &work.y_new, &work.u);
    }
    fn accept_control(&mut self, _view: &AcceptedStepView<'_>) -> AcceptControl {
        if self.segments.len() >= self.capacity {
            AcceptControl::Stop
        } else {
            AcceptControl::Continue
        }
    }
}
/// Accepted-state retention policy; no interpolation or pilot restarts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum FastCheckpointDensity {
    /// Fixed accepted-index stride.
    Stride(usize),
    /// Bounded online rows; minimize local squared-time-gap increase.
    OnlineTime(usize),
    /// Bounded online rows; combine time and normalized state motion gaps.
    OnlineMotion(usize),
}
/// Bounded state-only capture of actual native accepted checkpoints.
pub struct FastCheckpointCapture {
    n: usize,
    capacity: usize,
    density: FastCheckpointDensity,
    row_cap: usize,
    indices: Vec<usize>,
    motion: Vec<f64>,
    times: Vec<f64>,
    states: Vec<f64>,
}
impl FastCheckpointCapture {
    /// Allocate once, without retaining the eight stage rows per step.
    pub fn new(n: usize, capacity: usize) -> CoreResult<Self> {
        Self::with_stride(n, capacity, 1)
    }
    /// Retain all accepted times but only initial, stride-multiple and latest states.
    /// Capacity still limits actual accepted steps, not retained state rows.
    pub fn with_stride(n: usize, capacity: usize, stride: usize) -> CoreResult<Self> {
        Self::with_density(n, capacity, FastCheckpointDensity::Stride(stride))
    }
    /// Allocate an online or fixed-stride capture without hot allocation.
    pub fn with_density(
        n: usize,
        capacity: usize,
        density: FastCheckpointDensity,
    ) -> CoreResult<Self> {
        let row_cap = match density {
            FastCheckpointDensity::Stride(s) if s > 0 && s <= 4096 => capacity.div_ceil(s) + 1,
            FastCheckpointDensity::OnlineTime(r) | FastCheckpointDensity::OnlineMotion(r)
                if r >= 3 && r <= 4097 =>
            {
                r.min(capacity + 1)
            }
            _ => return Err(CoreError::InvalidInput("checkpoint density".into())),
        };
        if n == 0
            || capacity == 0
            || capacity > 4096
            || n.checked_mul(row_cap)
                .is_none_or(|v| v > isize::MAX as usize / 8)
        {
            return Err(CoreError::InvalidInput("checkpoint capacity".into()));
        }
        let online = !matches!(density, FastCheckpointDensity::Stride(_));
        Ok(Self {
            n,
            capacity,
            density,
            row_cap,
            times: Vec::with_capacity(capacity + 1),
            states: Vec::with_capacity(n * row_cap),
            indices: Vec::with_capacity(if online { row_cap } else { 0 }),
            motion: Vec::with_capacity(
                if matches!(density, FastCheckpointDensity::OnlineMotion(_)) {
                    capacity + 1
                } else {
                    0
                },
            ),
        })
    }
    pub(crate) fn reset(&mut self, t: f64, y: &[f64]) -> CoreResult<()> {
        if y.len() != self.n {
            return Err(CoreError::InvalidInput("checkpoint dimension".into()));
        }
        self.times.clear();
        self.states.clear();
        self.indices.clear();
        self.motion.clear();
        if !matches!(self.density, FastCheckpointDensity::Stride(_)) {
            self.indices.push(0);
        }
        if matches!(self.density, FastCheckpointDensity::OnlineMotion(_)) {
            self.motion.push(0.);
        }
        self.times.push(t);
        self.states.extend_from_slice(y);
        Ok(())
    }
    /// Accepted represented time sequence including initial time.
    pub fn times(&self) -> &[f64] {
        &self.times
    }
    /// Number of recorded accepted steps.
    pub fn len(&self) -> usize {
        self.times.len().saturating_sub(1)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Number of retained physical state rows, including initial/latest.
    pub fn retained_count(&self) -> usize {
        self.states.len() / self.n
    }
    /// Actual accepted index of a retained physical state row.
    pub fn retained_index(&self, row: usize) -> Option<usize> {
        if row >= self.retained_count() {
            return None;
        }
        Some(match self.density {
            FastCheckpointDensity::Stride(s) => (row * s).min(self.len()),
            _ => self.indices[row],
        })
    }
    /// Physical state only if its accepted index was retained.
    pub fn state(&self, k: usize) -> Option<&[f64]> {
        if k >= self.times.len() {
            return None;
        }
        let row = match self.density {
            FastCheckpointDensity::Stride(s) => {
                if k % s != 0 && k != self.len() {
                    return None;
                }
                k.div_ceil(s)
            }
            _ => self.indices.binary_search(&k).ok()?,
        };
        Some(&self.states[row * self.n..(row + 1) * self.n])
    }
    // Selection metric is not a certified error bound. Normalize both monitors
    // on the current prefix and remove the cheapest interior row. All states
    // copied are actual accepted native states, never interpolants.
    fn append_online(&mut self, k: usize, y: &[f64]) {
        if matches!(self.density, FastCheckpointDensity::OnlineMotion(_)) {
            let prev = &self.states[self.states.len() - self.n..];
            let delta = prev
                .iter()
                .zip(y)
                .map(|(&a, &b)| ((0.5 * a - 0.5 * b).abs()) / a.abs().max(b.abs()).max(1.))
                .fold(0., f64::max);
            self.motion
                .push(self.motion.last().copied().unwrap() + delta);
        }
        if self.indices.len() == self.row_cap {
            let span = (0.5 * self.times[k] - 0.5 * self.times[0])
                .abs()
                .max(f64::MIN_POSITIVE);
            let total = self.motion.last().copied().unwrap_or(0.);
            let gap = |a: usize, b: usize| {
                let dt = (0.5 * self.times[b] - 0.5 * self.times[a]).abs() / span;
                let dm = if total > 0. {
                    (self.motion[b] - self.motion[a]) / total
                } else {
                    0.
                };
                dt + dm
            };
            // The previous latest becomes an eligible interior row when
            // appending k. Endpoints 0 and k are never eviction candidates.
            let i = (1..self.indices.len())
                .min_by(|&a, &b| {
                    let score = |i: usize| {
                        gap(self.indices[i - 1], self.indices[i])
                            * gap(
                                self.indices[i],
                                self.indices.get(i + 1).copied().unwrap_or(k),
                            )
                    };
                    score(a).total_cmp(&score(b))
                })
                .unwrap();
            self.indices.remove(i);
            let end = self.states.len();
            self.states.copy_within((i + 1) * self.n..end, i * self.n);
            self.states.truncate(end - self.n);
        }
        self.indices.push(k);
        self.states.extend_from_slice(y);
    }
}
impl StepSink for FastCheckpointCapture {
    fn after_attempt(
        &mut self,
        _problem: &OdeProblem,
        _work: &mut Workspace,
        _t: f64,
        _y: &[f64],
        _h: f64,
        error: f64,
        _atol: f64,
        _rtol: f64,
        _counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        Ok(error)
    }
    fn accepted(&mut self, work: &Workspace, t_new: f64) {
        debug_assert!(self.len() < self.capacity);
        let old = self.len();
        self.times.push(t_new);
        match self.density {
            FastCheckpointDensity::Stride(s) => {
                if old % s == 0 {
                    self.states.extend_from_slice(&work.y_new);
                } else {
                    let k = self.states.len() - self.n;
                    self.states[k..].copy_from_slice(&work.y_new);
                }
            }
            _ => self.append_online(old + 1, &work.y_new),
        }
    }
    fn accept_control(&mut self, _view: &AcceptedStepView<'_>) -> AcceptControl {
        if self.len() >= self.capacity {
            AcceptControl::Stop
        } else {
            AcceptControl::Continue
        }
    }
}
impl FastDenseSegments {
    /// Exact represented quartic power coefficient enclosure, normalized
    /// time on the stored accepted interval. No physical error claim.
    pub fn power_component(
        &self,
        k: usize,
        c: usize,
    ) -> CoreResult<[rodas5p_core::directed::Interval; 5]> {
        use rodas5p_core::directed::Interval as I;
        if k >= self.len() || c >= self.n {
            return Err(CoreError::InvalidInput("dense power index".into()));
        }
        let mut d = [I::point(0.)?; 3];
        let stages = self.stage_rows(k);
        for (i, h) in self.dense_h.iter().enumerate() {
            for j in 0..STAGES {
                d[i] = d[i].add(I::point(h[j])?.scale(stages[j * self.n + c])?)?;
            }
        }
        let y0 = I::point(self.state(k)[c])?;
        let y1 = I::point(self.state(k + 1)[c])?;
        Ok([
            y0,
            y1.sub(y0)?.add(d[0])?,
            d[1].sub(d[0])?,
            d[2].sub(d[1])?,
            -d[2],
        ])
    }
}
struct DenseSink<'a> {
    control: FastInteriorControl,
    segments: FastDenseSegments,
    interior: FastInteriorWork,
    /// The midpoint state and then `(h/2) d`, overwritten by the estimate
    /// (`2 n`, one allocation; empty under `Off`).
    scratch: Vec<f64>,
    accept: Option<&'a mut DenseAcceptFn<'a>>,
}

impl DenseSink<'_> {
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
            accept: None,
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

impl StepSink for DenseSink<'_> {
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

    fn accept_control(&mut self, _view: &AcceptedStepView<'_>) -> AcceptControl {
        match self.accept.as_mut() {
            Some(f) => f(&self.segments),
            None => AcceptControl::Continue,
        }
    }
}

/// The run of [`integrate_rodas5p_fast_dense_observed`] and its banded twin.
#[derive(Clone, Debug)]
pub struct Rodas5pFastDenseResult {
    /// The fast driver's result on the output `[t0, t1]` (its `driver` is
    /// [`RODAS5P_FAST_DENSE_DRIVER_ID`],
    /// [`RODAS5P_FAST_BANDED_DENSE_DRIVER_ID`] or
    /// [`RODAS5P_FAST_BANDED_DENSE_SLICES_DRIVER_ID`]).
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

/// Native dense output with an explicit stage client. The original
/// controller, eight-stage attempt and accepted dense sink are unchanged.
/// The W-filtered interior path is deliberately not supported here because
/// its ninth backsolve has a different client contract.
pub fn integrate_rodas5p_fast_client_dense_observed(
    problem: &OdeProblem,
    client: Box<dyn crate::StageLinearSystem>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    atol_vector: &[f64],
) -> CoreResult<Rodas5pFastDenseResult> {
    let output = span_schedule(t_span)?;
    let mut sink = DenseSink::new(FastInteriorControl::Off, t_span.0, y0)?;
    let scalar;
    let atol = if atol_vector.is_empty() {
        scalar = vec![adaptive.atol; problem.dimension];
        &scalar[..]
    } else {
        atol_vector
    };
    let (mut fast, partial, _client) = crate::rodas5p_fast::integrate_client(
        problem, client, t_span, y0, adaptive, &output, atol, &mut sink,
    )?;
    fast.driver = "rodas5p-fast-client-dense-v1";
    Ok(Rodas5pFastDenseResult {
        fast,
        segments: sink.segments,
        interior: sink.interior,
        partial,
        banded: None,
    })
}
fn span_schedule((t0, t1): (f64, f64)) -> CoreResult<OutputSchedule> {
    OutputSchedule::new(if t1 > t0 { vec![t0, t1] } else { vec![t0] })
}

fn run_dense(
    problem: &OdeProblem,
    band: Option<(&BandedJacobian, BandedKernel)>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol_vector: &[f64],
) -> CoreResult<Rodas5pFastDenseResult> {
    run_dense_controlled(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        interior,
        atol_vector,
        None,
    )
}

/// Opt-in accepted-step control for the original fast dense and banded
/// dense drivers. The full accepted numerical proposal and its work are
/// retained; a client may stop before the next proposal and clip its own
/// physical output using the last dense polynomial. No stage is re-run.
/// Empty atol means scalar; otherwise the original vector contract applies.
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_fast_dense_with_control<'a>(
    problem: &OdeProblem,
    band: Option<(&BandedJacobian, BandedKernel)>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol_vector: &[f64],
    control: &'a mut DenseAcceptFn<'a>,
) -> CoreResult<Rodas5pFastDenseResult> {
    if !atol_vector.is_empty() {
        require_atol_vector(problem.dimension, atol_vector)?;
    }
    if let Some((band, kernel)) = band
        && (band.lower >= problem.dimension
            || band.upper >= problem.dimension
            || !matches!(
                kernel,
                BandedKernel::Indexed | BandedKernel::Slices | BandedKernel::FixedWidth2
            ))
    {
        return Err(CoreError::InvalidInput(
            "dense control band contract".into(),
        ));
    }
    run_dense_controlled(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        interior,
        atol_vector,
        Some(control),
    )
}
#[allow(clippy::too_many_arguments)]
fn run_dense_controlled<'a>(
    problem: &OdeProblem,
    band: Option<(&BandedJacobian, BandedKernel)>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol_vector: &[f64],
    accept: Option<&'a mut DenseAcceptFn<'a>>,
) -> CoreResult<Rodas5pFastDenseResult> {
    if y0.len() != problem.dimension {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast integration input".into(),
        ));
    }
    let output = span_schedule(t_span)?;
    let mut sink = DenseSink::new(interior, t_span.0, y0)?;
    sink.accept = accept;
    let (mut fast, banded, partial) = integrate_fast(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        &output,
        Rodas5pFastOptions::default(),
        None,
        atol_vector,
        &mut sink,
    )?;
    fast.driver = match band {
        Some((_, BandedKernel::Slices)) => RODAS5P_FAST_BANDED_DENSE_SLICES_DRIVER_ID,
        Some((_, BandedKernel::FixedWidth2)) => RODAS5P_FAST_BANDED_DENSE_FIXED2_DRIVER_ID,
        Some(_) => RODAS5P_FAST_BANDED_DENSE_DRIVER_ID,
        None => RODAS5P_FAST_DENSE_DRIVER_ID,
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
    run_banded_dense(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        interior,
        BandedKernel::Indexed,
        &[],
    )
}

/// [`integrate_rodas5p_fast_banded_dense_observed`] with an explicit
/// [`BandedKernel`] (loop L1 node V22). `Indexed` is that function;
/// `Slices` returns the same bits under
/// [`RODAS5P_FAST_BANDED_DENSE_SLICES_DRIVER_ID`] and `FixedWidth2` under
/// [`RODAS5P_FAST_BANDED_DENSE_FIXED2_DRIVER_ID`] (SL-N12). `FixedWidth2ExactCache` is
/// refused.
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_fast_banded_dense_observed_with_kernel(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    kernel: BandedKernel,
) -> CoreResult<Rodas5pFastDenseResult> {
    run_banded_dense(problem, band, t_span, y0, adaptive, interior, kernel, &[])
}

#[allow(clippy::too_many_arguments)]
fn run_banded_dense(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    kernel: BandedKernel,
    atol_vector: &[f64],
) -> CoreResult<Rodas5pFastDenseResult> {
    if !matches!(
        kernel,
        BandedKernel::Indexed | BandedKernel::Slices | BandedKernel::FixedWidth2
    ) {
        return Err(CoreError::InvalidInput(
            "the RODAS5P fast banded dense driver runs the Indexed, the Slices and the FixedWidth2 kernel only"
                .into(),
        ));
    }
    let n = problem.dimension;
    if band.lower >= n.max(1) || band.upper >= n.max(1) {
        return Err(CoreError::InvalidInput(
            "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
        ));
    }
    run_dense(
        problem,
        Some((band, kernel)),
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
    run_banded_dense(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        interior,
        BandedKernel::Indexed,
        atol,
    )
}

/// [`integrate_rodas5p_fast_banded_dense_observed_with_atol`] with an
/// explicit [`BandedKernel`]; see
/// [`integrate_rodas5p_fast_banded_dense_observed_with_kernel`].
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_fast_banded_dense_observed_with_atol_and_kernel(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    interior: FastInteriorControl,
    atol: &[f64],
    kernel: BandedKernel,
) -> CoreResult<Rodas5pFastDenseResult> {
    require_atol_vector(problem.dimension, atol)?;
    run_banded_dense(problem, band, t_span, y0, adaptive, interior, kernel, atol)
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
