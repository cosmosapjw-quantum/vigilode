//! Accepted-step conservative remap of the LRS photon transport adapter
//! (BASS/rei DAG node P05), with the operator and controller caches rebound
//! at every remap.
//!
//! In the comoving coordinate the absorption threshold drifts through the
//! fixed energy grid as `x_chi(t) = q_chi + ln a(t)`. The aligned mesh
//! ([`aligned_edges`]) moves the interior base edge nearest `x_chi(t)` onto
//! it, so that no cell straddles the threshold at the remap time.
//!
//! [`Remesher`] maps an accepted state onto a new energy mesh with the
//! library's positive overlap remap ([`ConservativeRemapWorkspace`]). It
//! remaps every angular column of cell integrals `N_ij`, and the absorbed
//! ledgers `A_i` as one more column, by sign: the nonnegative parts and the
//! negated negative parts separately, then their difference. `L_lo` and
//! `L_hi` are copied. The result is staged in the new layout's index order
//! and validated (finite; the compensated total of all components changes
//! by at most [`MAX_RELATIVE_TOTAL_CHANGE`] relative; the energy domain is
//! unchanged; the angular grid, the coordinate and the boundary policy are
//! the old ones by construction). A refusal leaves the committed state,
//! mesh and epoch as they were but keeps its work counted. [`Remesher::commit`]
//! publishes the staged state and advances the mesh epoch.
//!
//! [`integrate_segments`] drives the fast RODAS5P driver segment by
//! segment:
//! - `Static` runs once on the base mesh.
//! - `Hook` lets the L3 accept control stop at the first accepted step where
//!   the threshold has left the aligned face by more than `trigger` base
//!   widths, remaps there and restarts from the stop with the controller's
//!   proposal `h_next`.
//! - `Schedule` and `FixedSchedule` end segments at given remap times.
//!
//! Each remap builds the new mesh's model (a new `kbar` epoch) and the
//! structured client's binding from the staged proposal, then commits:
//! the commit is the last fallible step, after which the model, face and
//! client are swapped in. The rebind makes the client's old factorization
//! stale; a client rebuilt for a new shape carries its predecessor's work.
//! A remap that is refused or cannot be built ends the run with
//! `success = false` and the typed reason, keeping the accepted state, mesh
//! and epoch from before the attempt and every counter (the refused attempt
//! included). The remap itself allocates nothing after construction; a
//! restart allocates a constant number of buffers (layout, model, problem,
//! driver workspace), independent of the step count.

#![allow(clippy::needless_range_loop)] // the kernels index several arrays by cell

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

use rodas5p_core::{CoreError, CoreResult, WorkCounters};
use rodas5p_integrators::conservative_remap::{ConservativeRemapWorkspace, RemapError};
use rodas5p_integrators::{
    AcceptControl, AcceptedStepView, AdaptiveStepConfig, DriverStopReason, OdeProblem,
    OutputSchedule, StageLinearSystem,
    integrate_rodas5p_fast_banded_observed_with_atol_and_control,
    integrate_rodas5p_fast_client_observed_with_atol_and_control,
};

use super::block_solver::{BlockStageSolver, Capacitance, Mode, OpCounts};
use super::{Absorber, Boundary, Coordinate, Geometry, Layout, Model};

/// Largest relative change of the total of all state components that a
/// remap admits. Each kernel call (one angular column, one sign) is held to
/// the same bound on its own compensated totals.
pub const MAX_RELATIVE_TOTAL_CHANGE: f64 = 2.0e-15;

/// Uniform base energy grid on `[lo, hi]` with `n` cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaseGrid {
    pub lo: f64,
    pub hi: f64,
    pub n: usize,
}

impl BaseGrid {
    pub fn validate(&self) -> CoreResult<()> {
        if !(self.lo.is_finite() && self.hi.is_finite() && self.lo < self.hi && self.n >= 2) {
            return Err(CoreError::InvalidInput(
                "base grid needs finite lo < hi and at least two cells".into(),
            ));
        }
        Ok(())
    }

    /// Base cell width `D`.
    #[inline(always)]
    pub fn width(&self) -> f64 {
        (self.hi - self.lo) / self.n as f64
    }

    /// Base edge `k` (`hi` exactly at `k = n`), as the P02 uniform grids.
    #[inline(always)]
    pub fn edge(&self, k: usize) -> f64 {
        if k == self.n {
            self.hi
        } else {
            self.lo + (self.hi - self.lo) * k as f64 / self.n as f64
        }
    }

    /// The base edges into `out` (cleared first; no allocation within its
    /// capacity).
    pub fn edges_into(&self, out: &mut Vec<f64>) {
        out.clear();
        for k in 0..=self.n {
            out.push(self.edge(k));
        }
    }

    /// Whether the aligned mesh moves an edge onto `x`: `x` lies inside the
    /// grid, farther than `D / 4` from both ends (false for NaN).
    #[inline(always)]
    pub fn alignable(&self, x: f64) -> bool {
        let quarter = 0.25 * self.width();
        x - self.lo > quarter && self.hi - x > quarter
    }

    /// The interior base edge (`1 <= k <= n - 1`) nearest `x`, ties to the
    /// lower index; `None` when `x` is not [`Self::alignable`].
    pub fn aligned_index(&self, x: f64) -> Option<usize> {
        if self.n < 2 || !self.alignable(x) {
            return None;
        }
        let mut best = 1;
        let mut distance = (x - self.edge(1)).abs();
        for k in 2..self.n {
            let d = (x - self.edge(k)).abs();
            if d < distance {
                best = k;
                distance = d;
            }
        }
        Some(best)
    }
}

/// The aligned mesh of a uniform base grid on `[lo, hi]` with `n` cells:
/// the interior base edge nearest `x_chi` (`1 <= k <= n - 1`, ties to the
/// lower index) moves to exactly `x_chi`, unless `x_chi` lies within `D / 4`
/// of `lo` or `hi` or outside the grid (then the base grid itself). With
/// `refine`, the two cells adjacent to the aligned face are split into
/// equal halves (`n + 2` cells). Fills `out` (cleared first; no allocation
/// within its capacity) and returns the aligned face's index in `out`
/// (`None` when no edge moved).
///
/// Away from the ends (`x_chi` at least `D / 2` inside) the unrefined
/// widths lie in `[D / 2, 3 D / 2]`; in the end zones (`D / 4 < distance <
/// D / 2`) the nearest interior edge can be up to `3 D / 4` away and the
/// widths lie in `(D / 4, 7 D / 4)`.
pub fn aligned_edges(
    lo: f64,
    hi: f64,
    n: usize,
    x_chi: f64,
    refine: bool,
    out: &mut Vec<f64>,
) -> Option<usize> {
    let base = BaseGrid { lo, hi, n };
    let Some(k) = base.aligned_index(x_chi) else {
        base.edges_into(out);
        return None;
    };
    out.clear();
    for m in 0..=n {
        let e = if m == k { x_chi } else { base.edge(m) };
        if refine && (m == k || m == k + 1) {
            let previous = out[out.len() - 1];
            out.push(0.5 * (previous + e));
        }
        out.push(e);
    }
    Some(if refine { k + 1 } else { k })
}

/// The threshold position `x_chi(t)` in the grid's energy coordinate
/// (`q_chi + ln a(t)` comoving, `q_chi` physical); `None` without an
/// absorber.
pub fn threshold_position(model: &Model, t: f64) -> Option<f64> {
    model.absorber.map(|a| model.threshold(&a, t).0)
}

/// Neumaier-compensated sum.
pub fn compensated_total(values: &[f64]) -> f64 {
    let (mut sum, mut c) = (0.0_f64, 0.0_f64);
    for &v in values {
        let t = sum + v;
        c += if sum.abs() >= v.abs() {
            (sum - t) + v
        } else {
            (v - t) + sum
        };
        sum = t;
    }
    sum + c
}

/// Work of a [`Remesher`], refused attempts included.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RemeshWork {
    pub attempts: u64,
    pub commits: u64,
    pub refusals: u64,
    /// Overlap segments of every completed kernel call.
    pub overlap_segments: u64,
    /// Reconstruction L1 change of every completed kernel call (both sign
    /// parts; a diagnostic, not an error estimate).
    pub l1_change_sum: f64,
}

/// One staged remap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemapReport {
    pub source_cells: usize,
    pub destination_cells: usize,
    /// `nm` angular columns plus the absorbed-ledger column.
    pub columns: usize,
    pub overlap_segments: usize,
    /// Compensated totals of all components before and after.
    pub total_before: f64,
    pub total_after: f64,
    pub relative_total_change: f64,
    /// Sum over columns and sign parts of the kernel's reconstruction L1
    /// change.
    pub l1_change: f64,
}

/// Typed refusals of [`Remesher::propose`] and [`Remesher::commit`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RemapRefusal {
    /// Shapes disagree or exceed the remesher's capacity.
    Shape(&'static str),
    /// The old layout's energy edges are not the committed mesh's.
    StaleMesh,
    /// The new energy grid does not span the old energy domain exactly.
    Domain,
    /// A non-finite old or remapped component.
    NonFinite,
    /// The overlap kernel refused column `column` (`nm` is the absorbed
    /// ledgers), sign part `negative`.
    Kernel {
        column: usize,
        negative: bool,
        error: RemapError,
    },
    /// The total of all components changed by more than
    /// [`MAX_RELATIVE_TOTAL_CHANGE`].
    TotalChange { relative: f64 },
    /// A requested negative control: all work done, then refused.
    Injected,
    /// `commit` without a staged proposal.
    NothingStaged,
}

impl fmt::Display for RemapRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "REMAP_REFUSED: {self:?}")
    }
}

/// Transactional conservative remesher with preallocated staging.
pub struct Remesher {
    kernel: ConservativeRemapWorkspace,
    nm: usize,
    max_cells: usize,
    /// The committed mesh.
    edges: Vec<f64>,
    epoch: u64,
    pos: Vec<f64>,
    neg: Vec<f64>,
    out_pos: Vec<f64>,
    out_neg: Vec<f64>,
    staged_state: Vec<f64>,
    staged_edges: Vec<f64>,
    staged: Option<RemapReport>,
    pub work: RemeshWork,
}

impl Remesher {
    /// A remesher for `nm` angular cells and up to `max_cells` energy
    /// cells, whose committed mesh is `edges` at `epoch`. Every buffer is
    /// allocated here.
    pub fn new(
        edges: &[f64],
        nm: usize,
        max_cells: usize,
        epoch: u64,
    ) -> Result<Self, RemapRefusal> {
        let cells = edges.len().saturating_sub(1);
        if nm == 0 || cells == 0 || cells > max_cells {
            return Err(RemapRefusal::Shape(
                "remesher needs nm >= 1 and 1 <= cells <= max_cells",
            ));
        }
        let kernel =
            ConservativeRemapWorkspace::new(max_cells).map_err(|error| RemapRefusal::Kernel {
                column: 0,
                negative: false,
                error,
            })?;
        let mut committed = Vec::with_capacity(max_cells + 1);
        committed.extend_from_slice(edges);
        Ok(Self {
            kernel,
            nm,
            max_cells,
            edges: committed,
            epoch,
            pos: vec![0.0; max_cells],
            neg: vec![0.0; max_cells],
            out_pos: vec![0.0; max_cells],
            out_neg: vec![0.0; max_cells],
            staged_state: vec![0.0; 2 + max_cells * (nm + 1)],
            staged_edges: Vec::with_capacity(max_cells + 1),
            staged: None,
            work: RemeshWork::default(),
        })
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The committed mesh's energy edges.
    pub fn edges(&self) -> &[f64] {
        &self.edges
    }

    pub fn max_cells(&self) -> usize {
        self.max_cells
    }

    /// State length for `cells` energy cells.
    pub fn state_len(&self, cells: usize) -> usize {
        2 + cells * (self.nm + 1)
    }

    /// Remaps `y_old` (on `old`, the committed mesh) onto `new_q_edges` and
    /// stages it. `inject_failure` does all the work and then refuses. A
    /// refusal discards any staged proposal and changes nothing visible
    /// (the committed mesh and epoch; `y_old` is borrowed immutably); the
    /// work counters keep the attempt.
    pub fn propose(
        &mut self,
        old: &Layout,
        y_old: &[f64],
        new_q_edges: &[f64],
        inject_failure: bool,
    ) -> Result<RemapReport, RemapRefusal> {
        self.staged = None;
        self.work.attempts += 1;
        match self.stage(old, y_old, new_q_edges, inject_failure) {
            Ok(report) => {
                self.staged = Some(report);
                Ok(report)
            }
            Err(refusal) => {
                self.work.refusals += 1;
                Err(refusal)
            }
        }
    }

    fn stage(
        &mut self,
        old: &Layout,
        y_old: &[f64],
        new_q_edges: &[f64],
        inject_failure: bool,
    ) -> Result<RemapReport, RemapRefusal> {
        let nm = self.nm;
        if old.nm != nm {
            return Err(RemapRefusal::Shape(
                "old layout's angular cell count differs from the remesher's",
            ));
        }
        if y_old.len() != old.n() {
            return Err(RemapRefusal::Shape(
                "state length differs from the old layout",
            ));
        }
        let same_mesh = old.q_edges.len() == self.edges.len()
            && old
                .q_edges
                .iter()
                .zip(&self.edges)
                .all(|(a, b)| a.to_bits() == b.to_bits());
        if !same_mesh {
            return Err(RemapRefusal::StaleMesh);
        }
        let nq_old = old.nq;
        let nq_new = new_q_edges.len().saturating_sub(1);
        if nq_new == 0 || nq_new > self.max_cells {
            return Err(RemapRefusal::Shape(
                "new mesh cell count outside 1..=max_cells",
            ));
        }
        if new_q_edges[0].to_bits() != old.q_edges[0].to_bits()
            || new_q_edges[nq_new].to_bits() != old.q_edges[nq_old].to_bits()
        {
            return Err(RemapRefusal::Domain);
        }
        if !y_old.iter().all(|v| v.is_finite()) {
            return Err(RemapRefusal::NonFinite);
        }
        let stride = nm + 1;
        let n_new = 2 + nq_new * stride;
        let mut segments = 0;
        let mut l1 = 0.0;
        for col in 0..=nm {
            for i in 0..nq_old {
                let v = y_old[1 + i * stride + col];
                self.pos[i] = if v > 0.0 { v } else { 0.0 };
                self.neg[i] = if v < 0.0 { -v } else { 0.0 };
            }
            for negative in [false, true] {
                let (src, dst) = if negative {
                    (&self.neg[..nq_old], &mut self.out_neg[..nq_new])
                } else {
                    (&self.pos[..nq_old], &mut self.out_pos[..nq_new])
                };
                let r = self
                    .kernel
                    .remap(
                        &old.q_edges,
                        src,
                        new_q_edges,
                        dst,
                        MAX_RELATIVE_TOTAL_CHANGE,
                    )
                    .map_err(|error| RemapRefusal::Kernel {
                        column: col,
                        negative,
                        error,
                    })?;
                segments += r.overlap_segments;
                l1 += r.reconstruction_change_l1;
                self.work.overlap_segments += r.overlap_segments as u64;
                self.work.l1_change_sum += r.reconstruction_change_l1;
            }
            for i in 0..nq_new {
                self.staged_state[1 + i * stride + col] = self.out_pos[i] - self.out_neg[i];
            }
        }
        self.staged_state[0] = y_old[Layout::LO];
        self.staged_state[n_new - 1] = y_old[old.hi()];
        let staged = &self.staged_state[..n_new];
        if !staged.iter().all(|v| v.is_finite()) {
            return Err(RemapRefusal::NonFinite);
        }
        let before = compensated_total(y_old);
        let after = compensated_total(staged);
        let scale = if before != 0.0 {
            before.abs()
        } else {
            y_old.iter().map(|v| v.abs()).sum::<f64>()
        };
        let relative = if after == before {
            0.0
        } else {
            (after - before).abs() / scale
        };
        if relative.is_nan() || relative > MAX_RELATIVE_TOTAL_CHANGE {
            return Err(RemapRefusal::TotalChange { relative });
        }
        self.staged_edges.clear();
        self.staged_edges.extend_from_slice(new_q_edges);
        if inject_failure {
            return Err(RemapRefusal::Injected);
        }
        Ok(RemapReport {
            source_cells: nq_old,
            destination_cells: nq_new,
            columns: stride,
            overlap_segments: segments,
            total_before: before,
            total_after: after,
            relative_total_change: relative,
            l1_change: l1,
        })
    }

    /// Publishes the staged proposal: copies the staged state into `y_new`
    /// (no allocation within its capacity), makes the staged edges the
    /// committed mesh and advances the epoch, which it returns.
    pub fn commit(&mut self, y_new: &mut Vec<f64>) -> Result<u64, RemapRefusal> {
        let Some(report) = self.staged.take() else {
            return Err(RemapRefusal::NothingStaged);
        };
        let n = self.state_len(report.destination_cells);
        y_new.clear();
        y_new.extend_from_slice(&self.staged_state[..n]);
        self.edges.clear();
        self.edges.extend_from_slice(&self.staged_edges);
        self.epoch += 1;
        self.work.commits += 1;
        Ok(self.epoch)
    }

    /// The staged proposal's energy edges, if one is staged.
    pub fn staged_edges(&self) -> Option<&[f64]> {
        self.staged.as_ref().map(|_| self.staged_edges.as_slice())
    }

    /// Drops a staged proposal (nothing visible changes).
    pub fn discard(&mut self) {
        self.staged = None;
    }

    /// The committed mesh as a layout with `template`'s angular grid,
    /// coordinate and boundary policy.
    pub fn layout(&self, template: &Layout) -> CoreResult<Layout> {
        Layout::new(
            self.edges.clone(),
            template.mu_edges.clone(),
            template.coordinate,
            template.boundary,
        )
    }
}

fn check_shape(edges: &[f64], nm: usize, y: &[f64]) -> Result<usize, RemapRefusal> {
    let nq = edges.len().saturating_sub(1);
    if nq == 0 || nm == 0 || y.len() != 2 + nq * (nm + 1) {
        return Err(RemapRefusal::Shape(
            "state length differs from 2 + nq (nm + 1)",
        ));
    }
    Ok(nq)
}

/// Refinement helper: halves every energy cell. The edges gain the
/// midpoints; every cell integral and absorbed ledger becomes half of its
/// parent's in both halves (exact in binary for normal numbers); `L_lo` and
/// `L_hi` are copied. Fills the caller's buffers (cleared first; no
/// allocation within their capacity).
pub fn split_all(
    edges: &[f64],
    nm: usize,
    y: &[f64],
    edges_out: &mut Vec<f64>,
    y_out: &mut Vec<f64>,
) -> Result<(), RemapRefusal> {
    let nq = check_shape(edges, nm, y)?;
    edges_out.clear();
    for i in 0..nq {
        let (a, b) = (edges[i], edges[i + 1]);
        let mid = 0.5 * (a + b);
        if !(a < mid && mid < b) {
            return Err(RemapRefusal::Shape("cell too narrow to split"));
        }
        edges_out.push(a);
        edges_out.push(mid);
    }
    edges_out.push(edges[nq]);
    let stride = nm + 1;
    y_out.clear();
    y_out.push(y[Layout::LO]);
    for i in 0..nq {
        for _half in 0..2 {
            for col in 0..stride {
                y_out.push(0.5 * y[1 + i * stride + col]);
            }
        }
    }
    y_out.push(y[y.len() - 1]);
    Ok(())
}

/// Coarsening helper, the inverse of [`split_all`]: merges cell pairs
/// `(2g, 2g + 1)` by summing their integrals and ledgers and drops every
/// odd edge.
pub fn merge_pairs(
    edges: &[f64],
    nm: usize,
    y: &[f64],
    edges_out: &mut Vec<f64>,
    y_out: &mut Vec<f64>,
) -> Result<(), RemapRefusal> {
    let nq = check_shape(edges, nm, y)?;
    if nq % 2 != 0 {
        return Err(RemapRefusal::Shape("merge needs an even cell count"));
    }
    edges_out.clear();
    for k in (0..=nq).step_by(2) {
        edges_out.push(edges[k]);
    }
    let stride = nm + 1;
    y_out.clear();
    y_out.push(y[Layout::LO]);
    for g in 0..nq / 2 {
        for col in 0..stride {
            y_out.push(y[1 + 2 * g * stride + col] + y[1 + (2 * g + 1) * stride + col]);
        }
    }
    y_out.push(y[y.len() - 1]);
    Ok(())
}

/// A structured client shared with its owner, who rebinds it between
/// segments (the driver owns the boxed handle during a run).
pub struct SharedClient(pub Arc<Mutex<BlockStageSolver>>);

impl SharedClient {
    fn inner(&self) -> MutexGuard<'_, BlockStageSolver> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl StageLinearSystem for SharedClient {
    fn dimension(&self) -> usize {
        self.inner().dimension()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.inner().linearize(problem, t, y, counters)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner().factor(inv_h_gamma, counters)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        self.inner().solve_in_place(b, counters)
    }
}

/// The P02 model parameters but the energy grid.
#[derive(Clone, Debug)]
pub struct Physics {
    pub mu_edges: Vec<f64>,
    pub coordinate: Coordinate,
    pub boundary: Boundary,
    pub geometry: Geometry,
    pub gamma: f64,
    pub absorber: Option<Absorber>,
}

impl Physics {
    pub fn model(&self, q_edges: Vec<f64>) -> CoreResult<Model> {
        let layout = Layout::new(
            q_edges,
            self.mu_edges.clone(),
            self.coordinate,
            self.boundary,
        )?;
        Model::new(layout, self.geometry, self.gamma, self.absorber, false)
    }
}

/// Vector atol of the cell scale rule: `scale` times the cell measure on
/// cells, `scale` on the ledger slots (as the P02 scenario).
pub fn cell_atol(layout: &Layout, scale: f64) -> Vec<f64> {
    let mut atol = vec![scale; layout.n()];
    for i in 0..layout.nq {
        for j in 0..layout.nm {
            atol[layout.cell(i, j)] = scale * layout.dq[i] * layout.dm[j];
        }
    }
    atol
}

/// When the integration remaps.
#[derive(Clone, Debug, PartialEq)]
pub enum Arm {
    /// No remap: one run on the base mesh.
    Static,
    /// L3 accept control: stop at the first accepted step with
    /// `|x_chi(t) - face| > trigger D`, remap there, restart with `h_next`.
    Hook { trigger: f64 },
    /// Segments end at the remap times; restart with the last accepted
    /// step's proposal.
    Schedule { times: Vec<f64> },
    /// As `Schedule` with `steps` fixed steps per segment.
    FixedSchedule { times: Vec<f64>, steps: usize },
}

/// The stage linear solver.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Linear {
    /// The banded LU of the fast banded driver.
    Banded,
    /// The structured client, rebound at every remap.
    Client(Mode),
}

/// The first step of a segment after a remap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restart {
    /// The controller's proposal at the stop (`h_next`).
    HNext,
    /// The run's `initial_step` (a control).
    Fresh,
}

#[derive(Clone, Debug)]
pub struct RunConfig {
    pub base: BaseGrid,
    pub physics: Physics,
    /// Initial state on the base mesh.
    pub y0: Vec<f64>,
    pub t_span: (f64, f64),
    pub output_times: Vec<f64>,
    pub rtol: f64,
    /// Cell scale of the atol rule ([`cell_atol`]).
    pub atol_scale: f64,
    pub initial_step: f64,
    pub max_attempts: usize,
    pub linear: Linear,
    pub arm: Arm,
    /// Split the two cells adjacent to the aligned face (`n + 2` cells).
    pub refine: bool,
    /// Capacity of the remap and trace records (a remap beyond it is an
    /// error).
    pub max_remaps: usize,
    pub restart: Restart,
    /// Refuse the k-th remap (1-based) after all its work: a negative
    /// control of the run-level rollback.
    pub inject_failure_at_remap: Option<usize>,
    /// Record every remap's states and meshes (allocates per remap).
    pub record_remap_states: bool,
}

/// One requested output, on the mesh in force at its time.
#[derive(Clone, Debug)]
pub struct OutputRecord {
    pub t: f64,
    pub epoch: u64,
    pub q_edges: Vec<f64>,
    pub state: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemapEvent {
    pub t: f64,
    /// The epoch of the committed mesh.
    pub epoch: u64,
    /// The aligned face position (`None`: the base mesh).
    pub face: Option<f64>,
    /// The restart's initial step.
    pub h_restart: f64,
    /// The controller's proposal as the accept control saw it at the
    /// segment's last accepted step.
    pub control_h_next: f64,
    /// Attempts and counters of the run up to the remap.
    pub attempts_before: usize,
    pub counters_before: WorkCounters,
    pub report: RemapReport,
}

/// A remap's states and meshes ([`RunConfig::record_remap_states`]).
#[derive(Clone, Debug, PartialEq)]
pub struct RemapStates {
    pub t: f64,
    pub old_edges: Vec<f64>,
    pub new_edges: Vec<f64>,
    pub y_before: Vec<f64>,
    pub y_after: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceEvent {
    Start,
    Output,
    BeforeRemap,
    AfterRemap,
    End,
}

/// The compensated total of all components at an event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TracePoint {
    pub t: f64,
    pub epoch: u64,
    pub total: f64,
    pub event: TraceEvent,
}

pub struct RunOutcome {
    pub outputs: Vec<OutputRecord>,
    pub remaps: Vec<RemapEvent>,
    pub trace: Vec<TracePoint>,
    /// Summed over segments.
    pub counters: WorkCounters,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub segments: usize,
    pub remesh: RemeshWork,
    pub client_ops: Option<OpCounts>,
    pub client_epoch: Option<u64>,
    pub final_t: f64,
    pub final_state: Vec<f64>,
    pub final_edges: Vec<f64>,
    /// The epoch of the committed mesh at the end.
    pub final_epoch: u64,
    pub remap_states: Vec<RemapStates>,
    pub success: bool,
    pub message: String,
}

#[derive(Debug)]
pub enum RunError {
    Core(CoreError),
    Remap { t: f64, refusal: RemapRefusal },
}

impl From<CoreError> for RunError {
    fn from(e: CoreError) -> Self {
        RunError::Core(e)
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Core(e) => write!(f, "{e}"),
            RunError::Remap { t, refusal } => write!(f, "{refusal} at t = {t:e}"),
        }
    }
}

fn bad(msg: &str) -> RunError {
    RunError::Core(CoreError::InvalidInput(msg.into()))
}

/// Whether the hook remaps at threshold position `x`: the threshold left
/// the aligned face by more than `width`, or entered or left the alignable
/// zone.
#[inline(always)]
pub fn needs_remap(base: &BaseGrid, face: Option<f64>, x: f64, width: f64) -> bool {
    match (face, base.alignable(x)) {
        (Some(f), true) => (x - f).abs() > width,
        (None, false) => false,
        _ => true,
    }
}

/// The mutable state of a segmented run between segments.
struct Segmented<'a> {
    cfg: &'a RunConfig,
    model: Arc<Model>,
    remesher: Remesher,
    y: Vec<f64>,
    new_edges: Vec<f64>,
    face: Option<f64>,
    client: Option<(Arc<Mutex<BlockStageSolver>>, Mode)>,
    remaps: Vec<RemapEvent>,
    remap_states: Vec<RemapStates>,
    trace: Vec<TracePoint>,
}

/// The run's work up to a remap.
#[derive(Clone, Copy)]
struct Before {
    control_h_next: f64,
    attempts: usize,
    counters: WorkCounters,
}

impl Segmented<'_> {
    fn trace(&mut self, t: f64, total: f64, event: TraceEvent) {
        let epoch = self.remesher.epoch();
        self.trace.push(TracePoint {
            t,
            epoch,
            total,
            event,
        });
    }

    /// Remaps the accepted state at `t` onto the aligned mesh of `x_chi(t)`:
    /// propose, build the new epoch's model and client binding from the
    /// staged proposal, then commit and swap them in. On an error nothing
    /// visible changed (state, mesh, epoch, model, client); the attempt is
    /// counted in the remesher's work.
    fn remap(&mut self, t: f64, h_restart: f64, before: Before) -> Result<(), RunError> {
        if self.remaps.len() >= self.cfg.max_remaps {
            return Err(bad("REMAP_BUDGET: more remaps than max_remaps"));
        }
        let base = self.cfg.base;
        let x = threshold_position(&self.model, t).unwrap_or(f64::NAN);
        let face = aligned_edges(
            base.lo,
            base.hi,
            base.n,
            x,
            self.cfg.refine,
            &mut self.new_edges,
        );
        let total_before = compensated_total(&self.y);
        self.trace(t, total_before, TraceEvent::BeforeRemap);
        let inject = self.cfg.inject_failure_at_remap == Some(self.remaps.len() + 1);
        let report = self
            .remesher
            .propose(&self.model.layout, &self.y, &self.new_edges, inject)
            .map_err(|refusal| RunError::Remap { t, refusal })?;
        let (model, rebuilt) = match self.build_staged() {
            Ok(v) => v,
            Err(e) => {
                self.remesher.discard();
                return Err(e);
            }
        };
        let recorded = self.cfg.record_remap_states.then(|| {
            (
                self.remesher.edges().to_vec(),
                self.new_edges.clone(),
                self.y.clone(),
            )
        });
        // the last fallible step; afterwards only infallible swaps
        let epoch = self
            .remesher
            .commit(&mut self.y)
            .map_err(|refusal| RunError::Remap { t, refusal })?;
        self.model = model;
        self.face = face.map(|_| x);
        if let Some((client, _)) = &self.client {
            let mut solver = client.lock().unwrap_or_else(|e| e.into_inner());
            match rebuilt {
                // the shape was checked before the commit
                None => solver.rebind(self.model.clone(), epoch)?,
                Some(mut fresh) => {
                    fresh.ops = solver.ops;
                    *solver = fresh;
                }
            }
        }
        if let Some((old_edges, new_edges, y_before)) = recorded {
            self.remap_states.push(RemapStates {
                t,
                old_edges,
                new_edges,
                y_before,
                y_after: self.y.clone(),
            });
        }
        let after = compensated_total(&self.y);
        self.trace(t, after, TraceEvent::AfterRemap);
        self.remaps.push(RemapEvent {
            t,
            epoch,
            face: self.face,
            h_restart,
            control_h_next: before.control_h_next,
            attempts_before: before.attempts,
            counters_before: before.counters,
            report,
        });
        Ok(())
    }

    /// The staged mesh's model, and a new client when the staged shape
    /// differs from the client's (built at the next epoch); nothing is
    /// mutated.
    fn build_staged(&self) -> Result<(Arc<Model>, Option<BlockStageSolver>), RunError> {
        let edges = self
            .remesher
            .staged_edges()
            .ok_or_else(|| bad("remap: no staged proposal"))?
            .to_vec();
        let template = &self.model.layout;
        let layout = Layout::new(
            edges,
            template.mu_edges.clone(),
            template.coordinate,
            template.boundary,
        )?;
        let model = Arc::new(Model::new(
            layout,
            self.cfg.physics.geometry,
            self.cfg.physics.gamma,
            self.cfg.physics.absorber,
            false,
        )?);
        let rebuilt = match &self.client {
            Some((client, mode)) => {
                let solver = client.lock().unwrap_or_else(|e| e.into_inner());
                if solver.dimension() == model.layout.n() {
                    None
                } else {
                    Some(BlockStageSolver::new(
                        model.clone(),
                        self.remesher.epoch() + 1,
                        *mode,
                        Capacitance::Conservation,
                    )?)
                }
            }
            None => None,
        };
        Ok((model, rebuilt))
    }
}

/// How a segment ended.
enum SegmentEnd {
    /// The accept control stopped at an accepted step; the restart's step.
    Stop(f64),
    /// The driver stopped before the segment's end.
    Failed(String),
    /// The segment's end was reached.
    Reached,
}

fn strictly_increasing(v: &[f64]) -> bool {
    v.iter().all(|x| x.is_finite()) && v.windows(2).all(|p| p[0] < p[1])
}

/// Runs `cfg` segment by segment (see the module documentation).
pub fn integrate_segments(cfg: &RunConfig) -> Result<RunOutcome, RunError> {
    cfg.base.validate()?;
    let (t0, tf) = cfg.t_span;
    if !(t0.is_finite() && tf.is_finite() && t0 < tf) {
        return Err(bad("t_span needs finite t0 < tf"));
    }
    let outs = &cfg.output_times;
    if outs.is_empty() || !strictly_increasing(outs) || outs[0] < t0 || outs[outs.len() - 1] > tf {
        return Err(bad(
            "output_times must be strictly increasing within t_span",
        ));
    }
    let positive = |x: f64| x.is_finite() && x > 0.0;
    if !(positive(cfg.rtol) && positive(cfg.atol_scale) && positive(cfg.initial_step)) {
        return Err(bad(
            "rtol, atol_scale and initial_step must be finite and positive",
        ));
    }
    let (trigger_width, remap_times, fixed_steps) = match &cfg.arm {
        Arm::Static => (None, None, None),
        Arm::Hook { trigger } => {
            if !(trigger.is_finite() && *trigger > 0.0) {
                return Err(bad("hook trigger must be finite and positive"));
            }
            if cfg.physics.absorber.is_none() {
                return Err(bad(
                    "the hook tracks the absorber threshold: absorber required",
                ));
            }
            (Some(trigger * cfg.base.width()), None, None)
        }
        Arm::Schedule { times } => (None, Some(times.as_slice()), None),
        Arm::FixedSchedule { times, steps } => {
            if *steps == 0 {
                return Err(bad("fixed_schedule needs steps > 0"));
            }
            (None, Some(times.as_slice()), Some(*steps))
        }
    };
    if let Some(times) = remap_times
        && (!strictly_increasing(times)
            || times.first().is_some_and(|t| *t <= t0)
            || times.last().is_some_and(|t| *t >= tf))
    {
        return Err(bad(
            "remap times must be strictly increasing inside (t0, tf)",
        ));
    }
    let mut base_edges = Vec::with_capacity(cfg.base.n + 3);
    cfg.base.edges_into(&mut base_edges);
    let model = Arc::new(cfg.physics.model(base_edges.clone())?);
    if cfg.y0.len() != model.layout.n() {
        return Err(RunError::Core(CoreError::Dimension(
            "initial_state must have the base layout's length".into(),
        )));
    }
    let nm = model.layout.nm;
    let max_cells = cfg.base.n + if cfg.refine { 2 } else { 0 };
    let remesher = Remesher::new(&base_edges, nm, max_cells, 0)
        .map_err(|refusal| RunError::Remap { t: t0, refusal })?;
    let mut y = Vec::with_capacity(remesher.state_len(max_cells));
    y.extend_from_slice(&cfg.y0);
    let client = match cfg.linear {
        Linear::Banded => None,
        Linear::Client(mode) => Some((
            Arc::new(Mutex::new(BlockStageSolver::new(
                model.clone(),
                0,
                mode,
                Capacitance::Conservation,
            )?)),
            mode,
        )),
    };
    let mut client_box: Option<Box<dyn StageLinearSystem>> = client
        .as_ref()
        .map(|(c, _)| Box::new(SharedClient(c.clone())) as Box<dyn StageLinearSystem>);
    let face = threshold_position(&model, t0)
        .and_then(|x| cfg.base.aligned_index(x))
        .map(|k| cfg.base.edge(k));
    let mut run = Segmented {
        cfg,
        model,
        remesher,
        y,
        new_edges: Vec::with_capacity(max_cells + 1),
        face,
        client,
        remaps: Vec::with_capacity(cfg.max_remaps),
        remap_states: Vec::new(),
        trace: Vec::with_capacity(2 * cfg.max_remaps + outs.len() + 2),
    };
    let total0 = compensated_total(&run.y);
    run.trace(t0, total0, TraceEvent::Start);
    let mut outputs: Vec<OutputRecord> = Vec::with_capacity(outs.len());
    let mut next_out = 0;
    let mut next_remap = 0;
    let mut t = t0;
    let mut h0 = cfg.initial_step;
    let mut counters = WorkCounters::default();
    let (mut attempts, mut accepted, mut rejected, mut segments) = (0, 0, 0, 0);
    let (success, message) = loop {
        let seg_end = remap_times
            .and_then(|times| times.get(next_remap).copied())
            .unwrap_or(tf);
        let mut times = Vec::with_capacity(outs.len() - next_out + 2);
        times.push(t);
        times.extend(outs[next_out..].iter().filter(|o| **o > t && **o < seg_end));
        times.push(seg_end);
        let schedule = OutputSchedule::new(times)?;
        let atol = cell_atol(&run.model.layout, cfg.atol_scale);
        let atol_min = atol.iter().copied().fold(f64::INFINITY, f64::min);
        let budget = cfg.max_attempts.saturating_sub(attempts);
        if budget == 0 {
            break (false, "attempt budget exhausted before tf".to_string());
        }
        let mut adaptive = AdaptiveStepConfig {
            atol: atol_min,
            rtol: cfg.rtol,
            initial_step: h0,
            max_step: tf - t0,
            max_attempts: budget,
            ..AdaptiveStepConfig::default()
        };
        if let Some(steps) = fixed_steps {
            let h = (seg_end - t) / steps as f64;
            adaptive.rtol = 1.0e10;
            adaptive.initial_step = h;
            adaptive.max_step = h;
        }
        let problem = super::problem(run.model.clone())?;
        let mut last_h = f64::NAN;
        let (fast, partial) = {
            let model = &*run.model;
            let (base, face) = (&cfg.base, run.face);
            let mut control = |view: &AcceptedStepView<'_>| -> AcceptControl {
                last_h = view.h_next;
                if let Some(width) = trigger_width {
                    let x = threshold_position(model, view.t).unwrap_or(f64::NAN);
                    if needs_remap(base, face, x, width) {
                        return AcceptControl::Stop;
                    }
                }
                AcceptControl::Continue
            };
            match client_box.take() {
                None => {
                    let band = super::band(run.model.clone());
                    let r = integrate_rodas5p_fast_banded_observed_with_atol_and_control(
                        &problem,
                        &band,
                        (t, seg_end),
                        &run.y,
                        &adaptive,
                        &schedule,
                        &atol,
                        &mut control,
                    )?;
                    (r.fast, r.partial)
                }
                Some(b) => {
                    let (fast, partial, b) =
                        integrate_rodas5p_fast_client_observed_with_atol_and_control(
                            &problem,
                            b,
                            (t, seg_end),
                            &run.y,
                            &adaptive,
                            &schedule,
                            &atol,
                            &mut control,
                        )?;
                    client_box = Some(b);
                    (fast, partial)
                }
            }
        };
        segments += 1;
        counters.accumulate(fast.observed.counters);
        attempts += fast.attempts;
        accepted += fast.accepted_steps;
        rejected += fast.rejected_steps;
        // the segment's end: a client stop (remap there), a driver failure,
        // or the segment's tf
        let end = match &partial {
            Some(p) => {
                run.y.clear();
                run.y.extend_from_slice(&p.y);
                t = p.t;
                if p.reason == DriverStopReason::ClientStop {
                    SegmentEnd::Stop(p.h_next.unwrap_or(last_h))
                } else {
                    SegmentEnd::Failed(format!(
                        "segment {segments} stopped at t = {:e}: {:?}",
                        p.t, p.reason
                    ))
                }
            }
            None => {
                let last = fast.observed.y.last().ok_or_else(|| bad("empty output"))?;
                run.y.clear();
                run.y.extend_from_slice(last);
                t = seg_end;
                SegmentEnd::Reached
            }
        };
        // the requested outputs this segment recorded (a failed segment's
        // too), on the mesh in force
        for (tt, yy) in fast.observed.t.into_iter().zip(fast.observed.y) {
            if next_out < outs.len() && tt == outs[next_out] {
                let total = compensated_total(&yy);
                run.trace(tt, total, TraceEvent::Output);
                outputs.push(OutputRecord {
                    t: tt,
                    epoch: run.remesher.epoch(),
                    q_edges: run.model.layout.q_edges.clone(),
                    state: yy,
                });
                next_out += 1;
            }
        }
        let before = Before {
            control_h_next: last_h,
            attempts,
            counters,
        };
        let restart = |h: f64| match cfg.restart {
            Restart::HNext => h,
            Restart::Fresh => cfg.initial_step,
        };
        match end {
            SegmentEnd::Failed(message) => break (false, message),
            SegmentEnd::Stop(h_next) => {
                h0 = restart(h_next);
                if let Err(e) = run.remap(t, h0, before) {
                    break (false, e.to_string());
                }
            }
            SegmentEnd::Reached if seg_end >= tf => break (true, "success".to_string()),
            SegmentEnd::Reached => {
                next_remap += 1;
                if fixed_steps.is_none() {
                    if !(last_h.is_finite() && last_h > 0.0) {
                        break (
                            false,
                            "no accepted step proposal to restart from".to_string(),
                        );
                    }
                    h0 = restart(last_h);
                }
                if let Err(e) = run.remap(t, h0, before) {
                    break (false, e.to_string());
                }
            }
        }
    };
    let final_total = compensated_total(&run.y);
    run.trace(t, final_total, TraceEvent::End);
    let (client_ops, client_epoch) = match &run.client {
        Some((c, _)) => {
            let g = c.lock().unwrap_or_else(|e| e.into_inner());
            (Some(g.ops), Some(g.epoch()))
        }
        None => (None, None),
    };
    Ok(RunOutcome {
        outputs,
        remaps: run.remaps,
        trace: run.trace,
        counters,
        attempts,
        accepted_steps: accepted,
        rejected_steps: rejected,
        segments,
        remesh: run.remesher.work,
        client_ops,
        client_epoch,
        final_t: t,
        final_state: run.y,
        final_edges: run.remesher.edges().to_vec(),
        final_epoch: run.remesher.epoch(),
        remap_states: run.remap_states,
        success,
        message,
    })
}
