//! A lean adaptive RODAS5P driver (research node
//! `research/stiff_rodas5p_fast_20261002`).
//!
//! The profile of `research/stiff_rodas5p_profile_20261002` found the
//! sequential path spending most of its instructions outside the method:
//! 37-52% inside malloc/free on small problems (per-stage vectors, step
//! copies), 12-17% in a heap-allocating small-matrix solve, and, at n = 400,
//! 27% in unvectorized dense matrix-vector products (a diagnostic residual
//! after every direct stage solve and a Jacobian product for the gamma terms
//! of every stage). This driver integrates the same method with:
//!
//! * the transformed stage variables `u_i = sum_j Gamma_ij k_j` of the
//!   snapshot's own coefficients (`a`, `c_matrix`, `b_code`): with
//!   `W = I/(h gamma) - J`,
//!   `W u_i = f(t + c_i h, y + sum_j a_ij u_j) + sum_j (C_ij / h) u_j + gamma_i h f_t`,
//!   `y_new = y + sum_j b_code_j u_j`, and the embedded error is `u_s`
//!   (`btilde` is the last row of Gamma), so no Jacobian product is formed;
//! * no residual product after a direct solve (it never decided acceptance);
//! * one workspace for the whole integration; with an in-place Jacobian
//!   ([`OdeProblem::with_jacobian_into`]) a step allocates nothing at all;
//! * an in-place partial-pivoting LU that skips zero multipliers and, since
//!   v2 (`research/stiff_rodas5p_fast_v2_20261002`), stops every row update,
//!   row swap and triangular solve at the row's tracked nonzero extent, so a
//!   banded W costs O(n b^2); or faer's blocked LU for dense matrices above
//!   64 rows. The skipped operations are exact zeros: v2 reproduces v1;
//! * the Jacobian, `f(t, y)` and `f_t` reused after a rejected attempt from
//!   the same state.
//!
//! The step-size controller, the represented-clock rules and the output
//! collection are the sequential driver's own functions, unchanged. The
//! sequential path itself is untouched; this driver has its own identifier
//! and records.

use std::sync::{Arc, Mutex, PoisonError};

#[path = "exact_band_cache.rs"]
mod exact_band_cache;
use exact_band_cache::ExactBandCache;

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, LuSolveWorkspace, WorkCounters,
    rodas5p_coefficients, scaled_error_component,
};
use serde::Serialize;

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ControllerDecision,
    ControllerTelemetry, ObservedIntegrationResult, OdeProblem, OutputSchedule, StageLinearSystem,
    adaptive::rodas_next_step_after_attempt_prevalidated, output::OutputCollector,
    rodas_next_step_after_attempt,
};

/// Identifier of this driver in benchmark and research records.
pub const RODAS5P_FAST_DRIVER_ID: &str = "rodas5p-fast-transformed-v2";

/// Identifier of the banded pipeline (integrated DAG node INT-03).
pub const RODAS5P_FAST_BANDED_DRIVER_ID: &str = "rodas5p-fast-banded-v1";

/// Identifier of the pipeline with a client-owned stage solver
/// ([`integrate_rodas5p_fast_client_observed_with_atol`], library step L2).
pub const RODAS5P_FAST_CLIENT_DRIVER_ID: &str = "rodas5p-fast-client-v1";

/// Opt-in variants of the fast drivers (speed research node SPD01,
/// `research/spd01_fast_driver_overhead_20261005`). The default is the
/// recorded v2, banded and small-n behaviour, bit for bit; every existing
/// entry point uses the default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rodas5pFastOptions {
    /// Skip the configuration check inside every controller update; the
    /// driver validates the configuration once at entry.
    pub prevalidated_controller: bool,
    /// Land each step toward the span end once (in `adaptive_end_step`)
    /// instead of re-landing it in the output collector, and skip the
    /// collector's finiteness rescan of a state the driver has checked.
    pub fused_landing: bool,
    /// The LU of the dense-storage path (speed research node SPD02,
    /// `research/spd02_lu_column_extents_20261005`, and SPD09,
    /// `research/spd09_colext_threshold_20261007`); the banded and small
    /// drivers ignore it.
    pub lu_policy: FastLuPolicy,
}

/// The in-place LU variant of the dense-storage fast driver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FastLuPolicy {
    /// v2: row extents and zero-multiplier skipping, full column scans.
    #[default]
    Legacy,
    /// v2 plus exact column extents: the pivot search and the elimination
    /// run over the rows that can hold a nonzero in the column.
    ColumnExtents,
    /// [`Self::ColumnExtents`] when the dimension exceeds
    /// [`RODAS5P_FAST_SMALL_LU_MAX`], [`Self::Legacy`] otherwise (speed
    /// research node SPD09, `research/spd09_colext_threshold_20261007`; the
    /// threshold is the existing constant, fixed before any measurement).
    ColumnExtentsAbove64,
}

impl FastLuPolicy {
    /// Whether a dense-storage `W` of dimension `n` is factored with column
    /// extents under this policy.
    pub fn column_extents_at(self, n: usize) -> bool {
        match self {
            Self::Legacy => false,
            Self::ColumnExtents => true,
            Self::ColumnExtentsAbove64 => n > RODAS5P_FAST_SMALL_LU_MAX,
        }
    }
}

impl Rodas5pFastOptions {
    /// The driver identifier of the dense (`banded == false`) or banded
    /// pipeline under these options.
    pub fn driver_id(&self, banded: bool) -> &'static str {
        if !banded && self.lu_policy == FastLuPolicy::ColumnExtentsAbove64 {
            // The policy names the driver whatever the dimension, so a run
            // below the threshold (the legacy LU) is still recorded as SPD09's.
            return match (self.prevalidated_controller, self.fused_landing) {
                (false, false) => RODAS5P_FAST_COLEXT64_DRIVER_ID,
                (true, false) => "rodas5p-fast-transformed-v3-colext64-val",
                (false, true) => "rodas5p-fast-transformed-v3-colext64-land",
                (true, true) => "rodas5p-fast-transformed-v3-colext64-ovh",
            };
        }
        let colext = !banded && self.lu_policy == FastLuPolicy::ColumnExtents;
        match (
            banded,
            colext,
            self.prevalidated_controller,
            self.fused_landing,
        ) {
            (false, false, false, false) => RODAS5P_FAST_DRIVER_ID,
            (false, false, true, false) => "rodas5p-fast-transformed-v2-val",
            (false, false, false, true) => "rodas5p-fast-transformed-v2-land",
            (false, false, true, true) => "rodas5p-fast-transformed-v2-ovh",
            (false, true, false, false) => RODAS5P_FAST_COLEXT_DRIVER_ID,
            (false, true, true, false) => "rodas5p-fast-transformed-v3-colext-val",
            (false, true, false, true) => "rodas5p-fast-transformed-v3-colext-land",
            (false, true, true, true) => "rodas5p-fast-transformed-v3-colext-ovh",
            (true, _, false, false) => RODAS5P_FAST_BANDED_DRIVER_ID,
            (true, _, true, false) => "rodas5p-fast-banded-v1-val",
            (true, _, false, true) => "rodas5p-fast-banded-v1-land",
            (true, _, true, true) => "rodas5p-fast-banded-v1-ovh",
        }
    }
}

/// Identifier of the banded pipeline with the slices kernel (SPD03).
pub const RODAS5P_FAST_BANDED_SLICES_DRIVER_ID: &str = "rodas5p-fast-banded-v1-slices";

/// Identifier of the banded pipeline with the fixed-width (`lower = upper =
/// 2`) kernel ([`BandedKernel::FixedWidth2`]).
pub const RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID: &str = "rodas5p-fast-banded-fixed2-v1";

/// Identifier of the banded pipeline with the fixed-width kernel and the
/// exact factorization cache ([`BandedKernel::FixedWidth2ExactCache`]).
pub const RODAS5P_FAST_BANDED_FIXED2_EXACT_CACHE_DRIVER_ID: &str =
    "rodas5p-fast-banded-fixed2-exact-cache-v1";

/// Identifier of the dense-storage driver with column extents (SPD02).
pub const RODAS5P_FAST_COLEXT_DRIVER_ID: &str = "rodas5p-fast-transformed-v3-colext";

/// Identifier of the dense-storage driver with column extents above
/// [`RODAS5P_FAST_SMALL_LU_MAX`] only (SPD09).
pub const RODAS5P_FAST_COLEXT64_DRIVER_ID: &str = "rodas5p-fast-transformed-v3-colext64";

/// Writes the Jacobian's band at `(t, y)`: row `i`, column `j`
/// (`i - lower <= j <= i + upper`) at `i (lower + upper + 1) + (j + lower - i)`.
/// The buffer is zeroed before each call; entries outside the matrix are
/// never read.
pub type BandedJacobianFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// An explicit band structure and its Jacobian provider for
/// [`integrate_rodas5p_fast_banded_observed`] (integrated DAG node INT-03,
/// external review TF-03). The provider must agree with the problem's
/// Jacobian; nothing here checks that.
#[derive(Clone)]
pub struct BandedJacobian {
    pub lower: usize,
    pub upper: usize,
    pub fill: BandedJacobianFn,
}

/// The inner-loop form of the banded kernel (speed research node SPD03,
/// `research/spd03_banded_arm_instructions_20261005`). Both perform the
/// same floating-point operations in the same order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BandedKernel {
    /// Every operand through `at(i, j)` (a width multiply and a bounds
    /// check); the INT-03 kernel.
    #[default]
    Indexed,
    /// The row update and the back substitution over row slices.
    Slices,
    /// Opt-in Slices specialization for lower=upper=2. Arithmetic order
    /// and pivot ties are unchanged; all other widths fall back to Slices.
    FixedWidth2,
    /// FixedWidth2 plus exact finite shifted-matrix LU reuse. This still
    /// evaluates J and f at every new state, and never freezes J approximately.
    FixedWidth2ExactCache,
}

/// Counted linear-algebra work and storage of a banded run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BandedWork {
    /// Multiply-subtracts count 2, divisions 1, `W` assembly 1 per band entry.
    pub factor_operations: u64,
    pub solve_operations: u64,
    /// Eight-byte slots of the Jacobian band, factors and optional exact key.
    pub stored_slots: usize,
}

#[derive(Clone, Debug)]
pub struct Rodas5pFastBandedResult {
    pub fast: Rodas5pFastResult,
    pub work: BandedWork,
    /// Where and why the run stopped, when it stopped before `tf`
    /// (`None` on success).
    pub partial: Option<DriverPartial>,
}

/// Why a protected adaptive driver stopped before `tf`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DriverStopReason {
    /// A callback requested immediate termination; no retry follows.
    FatalCallback,
    /// The attempt budget `AdaptiveStepConfig::max_attempts` was used up.
    AttemptBudget,
    /// The next step fell below `AdaptiveStepConfig::min_step` (or no
    /// longer advanced `t`) before `tf`.
    MinimumStep,
    /// The accept control of an `_and_control` entry point returned
    /// [`AcceptControl::Stop`] after an accepted step before `tf` (library
    /// step L3).
    ClientStop,
}

/// The partial result of a protected adaptive run that stopped before `tf`
/// (plan §3, P0 tier: a failure returns the partial solution and a typed
/// reason). Recording it performs no arithmetic on the run: the last
/// accepted state is copied once, after the loop.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DriverPartial {
    /// The last accepted time (`t0` when no step was accepted).
    pub t: f64,
    /// The state at [`Self::t`].
    pub y: Vec<f64>,
    /// Why the loop ended.
    pub reason: DriverStopReason,
    /// The kind of the last rejected attempt after the last accepted step;
    /// `None` when no attempt was rejected since then.
    pub last_failure: Option<AdaptiveFailureKind>,
    /// The controller's proposal for the next step
    /// ([`AcceptedStepView::h_next`]) on a [`DriverStopReason::ClientStop`];
    /// `None` for the other reasons (and then not serialized).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h_next: Option<f64>,
}

impl DriverPartial {
    /// The partial record of a run that ended at `(t, y)` after `attempts`
    /// attempts, or `None` when it reached `tf`.
    pub(crate) fn after_loop(
        t: f64,
        tf: f64,
        y: &[f64],
        attempts: usize,
        max_attempts: usize,
        last_failure: Option<AdaptiveFailureKind>,
    ) -> Option<Self> {
        (t < tf).then(|| Self {
            t,
            y: y.to_vec(),
            // The loop tests the budget first, so an exit with the budget
            // spent is a budget exit; every in-loop break leaves it unspent.
            reason: if attempts >= max_attempts {
                DriverStopReason::AttemptBudget
            } else {
                DriverStopReason::MinimumStep
            },
            last_failure,
            h_next: None,
        })
    }
}

/// What the accept control of an `_and_control` entry point sees after
/// every accepted step (library step L3), the last one included. It
/// borrows the driver's own buffers, so building it allocates nothing.
#[derive(Clone, Copy, Debug)]
pub struct AcceptedStepView<'a> {
    /// The accepted time.
    pub t: f64,
    /// The accepted state at [`Self::t`].
    pub y: &'a [f64],
    /// The step just accepted (after any clipping to an output or `tf`).
    pub h_used: f64,
    /// The controller's proposal for the next step, after the controller
    /// update of this step, capped at `max_step` (library step L4), before
    /// the next step is landed toward `tf` or clipped to an output time.
    pub h_next: f64,
    /// Accepted steps so far, this one included (1 on the first).
    pub accepted_steps: usize,
    /// Attempts so far, this one included.
    pub attempts: usize,
    /// The run's counters so far.
    pub counters: &'a WorkCounters,
}

/// The answer of an accept control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcceptControl {
    /// Go on stepping.
    Continue,
    /// End the run at the accepted state: before `tf` with a
    /// [`DriverStopReason::ClientStop`] partial; at `tf` the run is complete
    /// and reported as a success.
    Stop,
}

/// The band storage and factors of the banded pipeline.
struct BandState {
    l: usize,
    u: usize,
    fill: BandedJacobianFn,
    /// Jacobian band, `n (l + u + 1)`.
    jacobian: Vec<f64>,
    /// `W` and then its factors, row `i` holding columns `i - l ..= i + u + l`
    /// (`n (2l + u + 1)`): `U` in columns `>= i`, the multiplier of column
    /// `k` in column `k` (not moved by later interchanges).
    factors: Vec<f64>,
    pivots: Vec<usize>,
    work: BandedWork,
    kernel: BandedKernel,
    exact_cache: Option<ExactBandCache>,
}

impl BandState {
    /// Zeroed band storage of order `n` for `band` under `kernel`.
    fn new(n: usize, band: &BandedJacobian, kernel: BandedKernel) -> Self {
        let (l, u) = (band.lower, band.upper);
        Self {
            l,
            u,
            fill: band.fill.clone(),
            jacobian: vec![0.0; n * (l + u + 1)],
            factors: vec![0.0; n * (2 * l + u + 1)],
            pivots: vec![0; n],
            work: BandedWork {
                stored_slots: n * (3 * l + 2 * u + 2)
                    + if kernel == BandedKernel::FixedWidth2ExactCache {
                        n * (l + u + 1)
                    } else {
                        0
                    },
                ..BandedWork::default()
            },
            kernel,
            exact_cache: (kernel == BandedKernel::FixedWidth2ExactCache)
                .then(|| ExactBandCache::new(n * (l + u + 1))),
        }
    }

    fn width(&self) -> usize {
        2 * self.l + self.u + 1
    }

    fn at(&self, i: usize, j: usize) -> usize {
        i * self.width() + (j + self.l - i)
    }

    /// `W = I/(h gamma) - J` on the band, then the banded LU with partial
    /// pivoting over rows `k ..= k + l`.
    fn factor(&mut self, n: usize, inv: f64) -> CoreResult<()> {
        if matches!(
            self.kernel,
            BandedKernel::FixedWidth2 | BandedKernel::FixedWidth2ExactCache
        ) && self.l == 2
            && self.u == 2
        {
            return self.factor_fixed2(n, inv);
        }
        if self.kernel != BandedKernel::Indexed {
            return self.factor_slices(n, inv);
        }
        let (l, u, width) = (self.l, self.u, self.width());
        let jw = l + u + 1;
        self.factors.fill(0.0);
        for i in 0..n {
            for d in 0..jw {
                self.factors[i * width + d] = -self.jacobian[i * jw + d];
            }
            self.factors[i * width + l] += inv;
        }
        self.work.factor_operations += (n * jw) as u64;
        for k in 0..n {
            let last_row = (k + l).min(n - 1);
            let mut p = k;
            let mut max = self.factors[self.at(k, k)].abs();
            for i in k + 1..=last_row {
                let v = self.factors[self.at(i, k)].abs();
                if v > max {
                    max = v;
                    p = i;
                }
            }
            if !(max > 0.0 && max.is_finite()) {
                return Err(CoreError::LinearSolve(format!(
                    "RODAS5P fast banded LU: singular or non-finite pivot at column {k}"
                )));
            }
            self.pivots[k] = p;
            let last_col = (k + u + l).min(n - 1);
            if p != k {
                for j in k..=last_col {
                    let (a, b) = (self.at(k, j), self.at(p, j));
                    self.factors.swap(a, b);
                }
            }
            let pivot = self.factors[self.at(k, k)];
            for i in k + 1..=last_row {
                let ik = self.at(i, k);
                if self.factors[ik] == 0.0 {
                    continue;
                }
                let m = self.factors[ik] / pivot;
                self.factors[ik] = m;
                for j in k + 1..=last_col {
                    let (x, r) = (self.at(i, j), self.at(k, j));
                    self.factors[x] -= m * self.factors[r];
                }
                self.work.factor_operations += 1 + 2 * (last_col - k) as u64;
            }
        }
        Ok(())
    }

    /// [`Self::factor`] with the row update over row slices (SPD03): the
    /// same assembly, pivot search and interchanges; the elimination zips
    /// the slice of row `i` over columns `k..=last_col` with the pivot
    /// row's, so every multiply-subtract happens in the same order.
    fn factor_slices(&mut self, n: usize, inv: f64) -> CoreResult<()> {
        let (l, u, width) = (self.l, self.u, self.width());
        let jw = l + u + 1;
        self.factors.fill(0.0);
        for i in 0..n {
            for d in 0..jw {
                self.factors[i * width + d] = -self.jacobian[i * jw + d];
            }
            self.factors[i * width + l] += inv;
        }
        self.work.factor_operations += (n * jw) as u64;
        for k in 0..n {
            let last_row = (k + l).min(n - 1);
            let mut p = k;
            let mut max = self.factors[self.at(k, k)].abs();
            for i in k + 1..=last_row {
                let v = self.factors[self.at(i, k)].abs();
                if v > max {
                    max = v;
                    p = i;
                }
            }
            if !(max > 0.0 && max.is_finite()) {
                return Err(CoreError::LinearSolve(format!(
                    "RODAS5P fast banded LU: singular or non-finite pivot at column {k}"
                )));
            }
            self.pivots[k] = p;
            let last_col = (k + u + l).min(n - 1);
            if p != k {
                for j in k..=last_col {
                    let (a, b) = (self.at(k, j), self.at(p, j));
                    self.factors.swap(a, b);
                }
            }
            // Row k holds columns k..=last_col at offsets l..=l + span - 1;
            // row i (below) holds them at k + l - i..
            let span = last_col - k + 1;
            let (top, bottom) = self.factors.split_at_mut((k + 1) * width);
            let pivot_row = &top[k * width + l..k * width + l + span];
            let pivot = pivot_row[0];
            for i in k + 1..=last_row {
                let start = (i - k - 1) * width + (k + l - i);
                let row = &mut bottom[start..start + span];
                if row[0] == 0.0 {
                    continue;
                }
                let m = row[0] / pivot;
                row[0] = m;
                for (x, r) in row[1..].iter_mut().zip(&pivot_row[1..]) {
                    *x -= m * r;
                }
                self.work.factor_operations += 1 + 2 * (last_col - k) as u64;
            }
        }
        Ok(())
    }

    /// [`Self::solve`] with the back substitution over row slices (SPD03).
    #[allow(clippy::needless_range_loop)] // the band offsets index both arrays
    fn solve_slices(&mut self, n: usize, b: &mut [f64]) {
        let (l, u, width) = (self.l, self.u, self.width());
        for k in 0..n {
            b.swap(k, self.pivots[k]);
            let bk = b[k];
            let last_row = (k + l).min(n - 1);
            for i in k + 1..=last_row {
                b[i] -= self.factors[i * width + (k + l - i)] * bk;
            }
            self.work.solve_operations += 2 * (last_row - k) as u64;
        }
        for i in (0..n).rev() {
            let last = (i + u + l).min(n - 1);
            let row = &self.factors[i * width + l..i * width + l + (last - i) + 1];
            let mut sum = b[i];
            for (f, x) in row[1..].iter().zip(&b[i + 1..=last]) {
                sum -= f * x;
            }
            b[i] = sum / row[0];
            self.work.solve_operations += 1 + 2 * (last - i) as u64;
        }
    }

    /// Solve with the factors, applying the interchanges as it goes.
    #[allow(clippy::needless_range_loop)] // the band offsets index both arrays
    fn solve(&mut self, n: usize, b: &mut [f64]) {
        if matches!(
            self.kernel,
            BandedKernel::FixedWidth2 | BandedKernel::FixedWidth2ExactCache
        ) && self.l == 2
            && self.u == 2
        {
            return self.solve_fixed2(n, b);
        }
        if self.kernel != BandedKernel::Indexed {
            return self.solve_slices(n, b);
        }
        let (l, u) = (self.l, self.u);
        for k in 0..n {
            b.swap(k, self.pivots[k]);
            let bk = b[k];
            for i in k + 1..=(k + l).min(n - 1) {
                b[i] -= self.factors[self.at(i, k)] * bk;
            }
            self.work.solve_operations += 2 * ((k + l).min(n - 1) - k) as u64;
        }
        for i in (0..n).rev() {
            let last = (i + u + l).min(n - 1);
            let mut sum = b[i];
            for j in i + 1..=last {
                sum -= self.factors[self.at(i, j)] * b[j];
            }
            b[i] = sum / self.factors[self.at(i, i)];
            self.work.solve_operations += 1 + 2 * (last - i) as u64;
        }
    }

    fn factor_fixed2(&mut self, n: usize, inv: f64) -> CoreResult<()> {
        let (l, u, width) = (2usize, 2usize, 7usize);
        let at = |i: usize, j: usize| i * 7 + (j + 2 - i);
        let jw = l + u + 1;
        self.factors.fill(0.0);
        for i in 0..n {
            for d in 0..jw {
                self.factors[i * width + d] = -self.jacobian[i * jw + d];
            }
            self.factors[i * width + l] += inv;
        }
        self.work.factor_operations += (n * jw) as u64;
        for k in 0..n {
            let last_row = (k + l).min(n - 1);
            let mut p = k;
            let mut max = self.factors[at(k, k)].abs();
            for i in k + 1..=last_row {
                let v = self.factors[at(i, k)].abs();
                if v > max {
                    max = v;
                    p = i;
                }
            }
            if !(max > 0.0 && max.is_finite()) {
                return Err(CoreError::LinearSolve(format!(
                    "RODAS5P fast banded LU: singular or non-finite pivot at column {k}"
                )));
            }
            self.pivots[k] = p;
            let last_col = (k + u + l).min(n - 1);
            if p != k {
                for j in k..=last_col {
                    let (a, b) = (at(k, j), at(p, j));
                    self.factors.swap(a, b);
                }
            }
            // Row k holds columns k..=last_col at offsets l..=l + span - 1;
            // row i (below) holds them at k + l - i..
            let span = last_col - k + 1;
            let (top, bottom) = self.factors.split_at_mut((k + 1) * width);
            let pivot_row = &top[k * width + l..k * width + l + span];
            let pivot = pivot_row[0];
            for i in k + 1..=last_row {
                let start = (i - k - 1) * width + (k + l - i);
                let row = &mut bottom[start..start + span];
                if row[0] == 0.0 {
                    continue;
                }
                let m = row[0] / pivot;
                row[0] = m;
                for (x, r) in row[1..].iter_mut().zip(&pivot_row[1..]) {
                    *x -= m * r;
                }
                self.work.factor_operations += 1 + 2 * (last_col - k) as u64;
            }
        }
        Ok(())
    }

    #[allow(clippy::needless_range_loop)] // the band offsets index both arrays
    fn solve_fixed2(&mut self, n: usize, b: &mut [f64]) {
        let (l, u, width) = (2usize, 2usize, 7usize);
        for k in 0..n {
            b.swap(k, self.pivots[k]);
            let bk = b[k];
            let last_row = (k + l).min(n - 1);
            for i in k + 1..=last_row {
                b[i] -= self.factors[i * width + (k + l - i)] * bk;
            }
            self.work.solve_operations += 2 * (last_row - k) as u64;
        }
        for i in (0..n).rev() {
            let last = (i + u + l).min(n - 1);
            let row = &self.factors[i * width + l..i * width + l + (last - i) + 1];
            let mut sum = b[i];
            for (f, x) in row[1..].iter().zip(&b[i + 1..=last]) {
                sum -= f * x;
            }
            b[i] = sum / row[0];
            self.work.solve_operations += 1 + 2 * (last - i) as u64;
        }
    }

    /// Reuse only successful finite factors of an exactly identical matrix.
    /// Solves do not mutate factors. Failed factorization invalidates the key.
    fn factor_with_policy(
        &mut self,
        n: usize,
        inv: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if self
            .exact_cache
            .as_ref()
            .is_some_and(|key| key.matches(inv, &self.jacobian))
        {
            return Ok(());
        }
        if let Some(key) = self.exact_cache.as_mut() {
            key.invalidate();
        }
        counters.direct_factorizations += 1;
        self.factor(n, inv)?;
        if let Some(key) = self.exact_cache.as_mut()
            && self.factors.iter().all(|x| x.is_finite())
        {
            key.record(inv, &self.jacobian);
        }
        Ok(())
    }
}

/// Matrices with at most this many rows always use the in-place LU.
pub const RODAS5P_FAST_SMALL_LU_MAX: usize = 64;

/// Above [`RODAS5P_FAST_SMALL_LU_MAX`], the in-place (zero-skipping) LU is
/// used when at most this fraction of the first Jacobian's entries is
/// nonzero; denser matrices use faer.
pub const RODAS5P_FAST_SPARSE_DENSITY_MAX: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rodas5pFastLu {
    /// Row-major partial-pivoting LU in the workspace; zero multipliers skip
    /// their row update.
    InPlaceZeroSkipping,
    /// [`Self::InPlaceZeroSkipping`] with exact column extents (speed
    /// research node SPD02): the same factors, pivots and row extents.
    InPlaceColumnExtents,
    /// faer's blocked partial-pivoting LU (one allocation per factorization).
    Faer,
    /// The banded pipeline of [`integrate_rodas5p_fast_banded_observed`].
    Banded,
    /// A client-owned [`StageLinearSystem`]
    /// ([`integrate_rodas5p_fast_client_observed_with_atol`]).
    Client,
}

#[derive(Clone, Debug)]
pub struct Rodas5pFastResult {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    /// Attempts that reused the Jacobian, `f(t, y)` and `f_t` of a rejected
    /// attempt from the same state.
    pub jacobian_reuses: usize,
    pub lu: Rodas5pFastLu,
    pub driver: &'static str,
}

/// The stage arithmetic of one integration, allocated once.
pub(crate) struct Workspace {
    pub(crate) n: usize,
    pub(crate) s: usize,
    pub(crate) gamma: f64,
    pub(crate) c: Vec<f64>,
    pub(crate) gamma_rows: Vec<f64>,
    pub(crate) lu_kind: Rodas5pFastLu,
    /// Row-major `W` and its in-place factors (in-place LU).
    w: Vec<f64>,
    pivots: Vec<usize>,
    /// Per row of the in-place factors: the last column that can be nonzero
    /// (U part) and the first stored multiplier (L part; `n` for none).
    row_end: Vec<usize>,
    l_start: Vec<usize>,
    /// Per column of the in-place factors: the last row that can hold a
    /// nonzero (SPD02; empty under the legacy policy).
    col_end: Vec<usize>,
    lu_policy: FastLuPolicy,
    /// Nonzero entries of the strictly lower `a` and `C` rows and of
    /// `b_code`, in ascending column order.
    pub(crate) a_nonzero: Vec<Vec<(usize, f64)>>,
    pub(crate) c_nonzero: Vec<Vec<(usize, f64)>>,
    pub(crate) b_nonzero: Vec<(usize, f64)>,
    /// `W` for faer, and its factorization.
    w_dense: DenseMatrix,
    faer_lu: Option<LuFactorization>,
    /// Aligned RHS and scratch, retained across same-dimension refactorizations.
    faer_solve_workspace: Option<LuSolveWorkspace>,
    /// The Jacobian at the state of the current attempt, kept across steps
    /// (the in-place Jacobian callback rewrites its own pattern).
    pub(crate) jacobian: DenseMatrix,
    pub(crate) lu_chosen: bool,
    pub(crate) f0: Vec<f64>,
    pub(crate) ft: Vec<f64>,
    /// Prepared RHS buffers for the non-autonomous finite-difference f_t.
    ft_fp: Vec<f64>,
    ft_fm: Vec<f64>,
    pub(crate) u: Vec<f64>,
    pub(crate) stage_state: Vec<f64>,
    pub(crate) stage_rhs: Vec<f64>,
    pub(crate) y_new: Vec<f64>,
    /// Per-component absolute tolerance of the error norm. Empty means the
    /// scalar `atol` passed to [`Workspace::attempt`], bit for bit; with `n`
    /// entries component `i` is scaled by
    /// `atol_vector[i] + rtol max(|y_i|, |y_new,i|)`.
    pub(crate) atol_vector: Vec<f64>,
    band: Option<BandState>,
    /// The client-owned stage solver of [`Rodas5pFastLu::Client`]. The
    /// mutex only keeps the workspace `Sync` (the lagged driver shares
    /// `&Workspace` with its preconditioner) for a `Send` client: every
    /// access is exclusive ([`client_of`]), so it is never locked.
    client: Option<Mutex<Box<dyn StageLinearSystem>>>,
}

/// The client behind its never-locked mutex (see [`Workspace`]).
fn client_of(client: &mut Mutex<Box<dyn StageLinearSystem>>) -> &mut dyn StageLinearSystem {
    client
        .get_mut()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
}

impl Workspace {
    /// The banded workspace: no `n x n` storage.
    fn new_banded(n: usize, band: &BandedJacobian, kernel: BandedKernel) -> CoreResult<Self> {
        let mut work = Self::with_dense_size(n, 0)?;
        work.band = Some(BandState::new(n, band, kernel));
        work.lu_kind = Rodas5pFastLu::Banded;
        work.lu_chosen = true;
        Ok(work)
    }

    /// The client workspace: no `n x n` storage; `client` owns `J` and `W`.
    fn new_client(n: usize, client: Box<dyn StageLinearSystem>) -> CoreResult<Self> {
        let mut work = Self::with_dense_size(n, 0)?;
        work.client = Some(Mutex::new(client));
        work.lu_kind = Rodas5pFastLu::Client;
        work.lu_chosen = true;
        Ok(work)
    }

    pub(crate) fn new(n: usize, lu_policy: FastLuPolicy) -> CoreResult<Self> {
        let mut work = Self::with_dense_size(n, n)?;
        work.lu_policy = lu_policy;
        // Only a policy that uses the extents at this size allocates them, so
        // SPD09 below its threshold keeps the legacy allocation count.
        if lu_policy.column_extents_at(n) {
            work.col_end = vec![0; n];
        }
        Ok(work)
    }

    /// `dense` is the order of the dense `W` and `J` buffers (0 for none).
    fn with_dense_size(n: usize, dense: usize) -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        let s = coeffs.stages();
        for i in 0..s {
            for j in i..s {
                if coeffs.a[(i, j)] != 0.0 || coeffs.c_matrix[(i, j)] != 0.0 {
                    return Err(CoreError::Coefficients(
                        "RODAS5P transformed coefficients are not strictly lower triangular".into(),
                    ));
                }
            }
        }
        let nonzero_row = |m: &DenseMatrix, i: usize| {
            (0..i)
                .filter(|&j| m[(i, j)] != 0.0)
                .map(|j| (j, m[(i, j)]))
                .collect::<Vec<_>>()
        };
        Ok(Self {
            row_end: vec![0; n],
            l_start: vec![n; n],
            col_end: Vec::new(),
            lu_policy: FastLuPolicy::Legacy,
            a_nonzero: (0..s).map(|i| nonzero_row(&coeffs.a, i)).collect(),
            c_nonzero: (0..s).map(|i| nonzero_row(&coeffs.c_matrix, i)).collect(),
            b_nonzero: coeffs
                .b_code
                .iter()
                .enumerate()
                .filter(|(_, b)| **b != 0.0)
                .map(|(j, b)| (j, *b))
                .collect(),
            n,
            s,
            gamma: coeffs.gamma,
            c: coeffs.c.clone(),
            gamma_rows: coeffs.gamma_rows.clone(),
            lu_kind: Rodas5pFastLu::InPlaceZeroSkipping,
            w: vec![0.0; dense * dense],
            pivots: vec![0; n],
            w_dense: DenseMatrix::zeros(0, 0),
            faer_lu: None,
            faer_solve_workspace: None,
            jacobian: DenseMatrix::zeros(dense, dense),
            lu_chosen: false,
            f0: vec![0.0; n],
            ft: vec![0.0; n],
            ft_fp: vec![0.0; n],
            ft_fm: vec![0.0; n],
            u: vec![0.0; s * n],
            stage_state: vec![0.0; n],
            stage_rhs: vec![0.0; n],
            y_new: vec![0.0; n],
            atol_vector: Vec::new(),
            band: None,
            client: None,
        })
    }

    pub(crate) fn choose_lu(&mut self) {
        let n = self.n;
        let nonzero = self
            .jacobian
            .as_slice()
            .iter()
            .filter(|v| **v != 0.0)
            .count();
        let density = nonzero as f64 / (n * n) as f64;
        self.lu_kind =
            if n <= RODAS5P_FAST_SMALL_LU_MAX || density <= RODAS5P_FAST_SPARSE_DENSITY_MAX {
                match self.lu_policy {
                    FastLuPolicy::Legacy => Rodas5pFastLu::InPlaceZeroSkipping,
                    FastLuPolicy::ColumnExtents => Rodas5pFastLu::InPlaceColumnExtents,
                    FastLuPolicy::ColumnExtentsAbove64 if n > RODAS5P_FAST_SMALL_LU_MAX => {
                        Rodas5pFastLu::InPlaceColumnExtents
                    }
                    FastLuPolicy::ColumnExtentsAbove64 => Rodas5pFastLu::InPlaceZeroSkipping,
                }
            } else {
                self.w_dense = DenseMatrix::zeros(n, n);
                self.faer_solve_workspace = Some(LuSolveWorkspace::new(n));
                Rodas5pFastLu::Faer
            };
    }

    /// Form `W = I/(h gamma) - J` and factor it.
    pub(crate) fn factor(&mut self, h: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let n = self.n;
        let inv = 1.0 / (h * self.gamma);
        if let Some(band) = self.band.as_mut() {
            return band.factor_with_policy(n, inv, counters);
        }
        if let Some(client) = self.client.as_mut() {
            // The client charges its own factorization work.
            return client_of(client).factor(inv, counters);
        }
        let jacobian = &self.jacobian;
        let target = match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping | Rodas5pFastLu::InPlaceColumnExtents => {
                self.w.as_mut_slice()
            }
            Rodas5pFastLu::Faer => self.w_dense.as_mut_slice(),
            Rodas5pFastLu::Banded | Rodas5pFastLu::Client => {
                unreachable!("banded and client factors are formed above")
            }
        };
        for (w, j) in target.iter_mut().zip(jacobian.as_slice()) {
            *w = -j;
        }
        for i in 0..n {
            target[i * n + i] += inv;
        }
        counters.direct_factorizations += 1;
        match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping => {
                for (i, row) in self.w.chunks_exact(n).enumerate() {
                    let last = row.iter().rposition(|v| *v != 0.0).unwrap_or(0);
                    self.row_end[i] = last.max(i);
                    self.l_start[i] = n;
                }
                lu_in_place(
                    &mut self.w,
                    n,
                    &mut self.pivots,
                    &mut self.row_end,
                    &mut self.l_start,
                )
            }
            Rodas5pFastLu::InPlaceColumnExtents => {
                extents_of(
                    &self.w,
                    n,
                    &mut self.row_end,
                    &mut self.l_start,
                    &mut self.col_end,
                );
                lu_in_place_col_extents(
                    &mut self.w,
                    n,
                    &mut self.pivots,
                    &mut self.row_end,
                    &mut self.l_start,
                    &mut self.col_end,
                )
            }
            Rodas5pFastLu::Faer => {
                self.faer_lu = Some(LuFactorization::new(&self.w_dense)?);
                Ok(())
            }
            Rodas5pFastLu::Banded | Rodas5pFastLu::Client => {
                unreachable!("banded and client factors are formed above")
            }
        }
    }

    /// Solve `W x = stage_rhs` in place.
    pub(crate) fn solve(&mut self, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.linear_solves += 1;
        // A client charges its own solve work (direct or iterative).
        if self.lu_kind != Rodas5pFastLu::Client {
            counters.direct_solve_calls += 1;
        }
        match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping | Rodas5pFastLu::InPlaceColumnExtents => {
                lu_solve_in_place(
                    &self.w,
                    self.n,
                    &self.pivots,
                    &self.row_end,
                    &self.l_start,
                    &mut self.stage_rhs,
                );
            }
            Rodas5pFastLu::Faer => {
                self.faer_lu
                    .as_ref()
                    .expect("factored")
                    .solve_in_place_with_workspace(
                        &mut self.stage_rhs,
                        self.faer_solve_workspace.as_mut().expect("faer scratch"),
                    )?;
            }
            Rodas5pFastLu::Banded => {
                let n = self.n;
                self.band
                    .as_mut()
                    .expect("banded workspace")
                    .solve(n, &mut self.stage_rhs);
            }
            Rodas5pFastLu::Client => {
                client_of(self.client.as_mut().expect("client workspace"))
                    .solve_in_place(&mut self.stage_rhs, counters)?;
            }
        }
        if self.stage_rhs.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve(
                "RODAS5P fast stage solve produced NaN/Inf".into(),
            ))
        }
    }

    /// Solve `W x = b` in place in a caller buffer with the stored dense
    /// factors (no counters, `&self`): the arithmetic of [`Self::solve`] on
    /// `b` (wave-2 node `b1b-gmres`, the lagged LU as a GMRES right
    /// preconditioner). Banded storage and a client solver are refused.
    pub(crate) fn solve_dense_into(&self, b: &mut [f64]) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "RODAS5P fast stored-LU solve shape mismatch".into(),
            ));
        }
        match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping | Rodas5pFastLu::InPlaceColumnExtents => {
                lu_solve_in_place(
                    &self.w,
                    self.n,
                    &self.pivots,
                    &self.row_end,
                    &self.l_start,
                    b,
                );
            }
            Rodas5pFastLu::Faer => {
                self.faer_lu
                    .as_ref()
                    .ok_or_else(|| CoreError::LinearSolve("RODAS5P fast LU not factored".into()))?
                    .solve_in_place(b)?;
            }
            Rodas5pFastLu::Banded | Rodas5pFastLu::Client => {
                return Err(CoreError::InvalidInput(
                    "the stored-LU preconditioner supports dense storage only".into(),
                ));
            }
        }
        if b.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve(
                "RODAS5P fast stage solve produced NaN/Inf".into(),
            ))
        }
    }

    /// Solve `W x = b` in place in a caller buffer with the current factors
    /// of any storage, dense or banded (wave-3 node `p0-fastdense`, the
    /// W-filtered interior estimate). It charges no [`WorkCounters`]; the
    /// caller does. A banded solve takes `&mut self` because the band
    /// kernel counts its operations: they are added to [`BandedWork`]
    /// `solve_operations`. The faer path reuses its permutation scratch. A
    /// client solver, which charges its own work, is refused.
    pub(crate) fn solve_into(&mut self, b: &mut [f64]) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "RODAS5P fast stored-LU solve shape mismatch".into(),
            ));
        }
        match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping | Rodas5pFastLu::InPlaceColumnExtents => {
                lu_solve_in_place(
                    &self.w,
                    self.n,
                    &self.pivots,
                    &self.row_end,
                    &self.l_start,
                    b,
                );
            }
            Rodas5pFastLu::Faer => {
                self.faer_lu
                    .as_ref()
                    .ok_or_else(|| CoreError::LinearSolve("RODAS5P fast LU not factored".into()))?
                    .solve_in_place_with_workspace(
                        b,
                        self.faer_solve_workspace.as_mut().expect("faer scratch"),
                    )?;
            }
            Rodas5pFastLu::Banded => {
                let n = self.n;
                self.band.as_mut().expect("banded workspace").solve(n, b);
            }
            Rodas5pFastLu::Client => {
                return Err(CoreError::InvalidInput(
                    "the stored-LU solve does not support a client stage solver".into(),
                ));
            }
        }
        if b.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve(
                "RODAS5P fast stored-LU solve produced NaN/Inf".into(),
            ))
        }
    }

    /// A fresh [`Self::attempt`] for the unit tests of sibling modules.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn attempt_for_tests(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        self.attempt(problem, t, y, h, true, atol, rtol, counters)
    }

    /// One attempt from `(t, y)` with step `h`: the new state in `y_new`
    /// and the WRMS norm of the embedded error. `fresh` builds the Jacobian,
    /// `f(t, y)` and `f_t` at this state; otherwise those of the previous
    /// attempt from the same state are reused.
    #[allow(clippy::too_many_arguments)]
    fn attempt(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        h: f64,
        fresh: bool,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        let (n, s) = (self.n, self.s);
        if fresh {
            if let Some(band) = self.band.as_mut() {
                band.jacobian.fill(0.0);
                counters.jacobian_builds += 1;
                (band.fill)(t, y, &mut band.jacobian)?;
            } else if let Some(client) = self.client.as_mut() {
                counters.jacobian_builds += 1;
                client_of(client).linearize(problem, t, y, counters)?;
            } else {
                problem.dense_jacobian_into(t, y, &mut self.jacobian, counters)?;
            }
            if !self.lu_chosen {
                self.choose_lu();
                self.lu_chosen = true;
            }
            problem.eval_rhs_into(t, y, &mut self.f0, counters)?;
            if !problem.autonomous {
                // A failed evaluation leaves `ft` unspecified; the driver then
                // rebuilds the state (`fresh_state = false`) before reuse.
                problem.eval_partial_t_into_with_scratch(
                    t,
                    y,
                    &mut self.ft,
                    &mut self.ft_fp,
                    &mut self.ft_fm,
                    counters,
                )?;
            }
        }
        self.factor(h, counters)?;
        for i in 0..s {
            if i == 0 {
                self.stage_rhs.copy_from_slice(&self.f0);
            } else {
                self.stage_state.copy_from_slice(y);
                for &(j, aij) in &self.a_nonzero[i] {
                    let uj = &self.u[j * n..(j + 1) * n];
                    for (x, v) in self.stage_state.iter_mut().zip(uj) {
                        *x += aij * v;
                    }
                }
                problem.eval_rhs_into(
                    t + self.c[i] * h,
                    &self.stage_state,
                    &mut self.stage_rhs,
                    counters,
                )?;
            }
            for &(j, c) in &self.c_nonzero[i] {
                let cij = c / h;
                if cij != 0.0 {
                    let uj = &self.u[j * n..(j + 1) * n];
                    for (x, v) in self.stage_rhs.iter_mut().zip(uj) {
                        *x += cij * v;
                    }
                }
            }
            if !problem.autonomous {
                let g = self.gamma_rows[i] * h;
                for (x, v) in self.stage_rhs.iter_mut().zip(&self.ft) {
                    *x += g * v;
                }
            }
            self.solve(counters)?;
            self.u[i * n..(i + 1) * n].copy_from_slice(&self.stage_rhs);
        }
        self.y_new.copy_from_slice(y);
        for &(j, bj) in &self.b_nonzero {
            let uj = &self.u[j * n..(j + 1) * n];
            for (x, v) in self.y_new.iter_mut().zip(uj) {
                *x += bj * v;
            }
        }
        if !self.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P fast step produced NaN/Inf".into(),
            ));
        }
        // WRMS of the embedded error u_s in the sequential path's scale
        // atol + rtol max(|y|, |y_new|).
        let error = &self.u[(s - 1) * n..s * n];
        let mut sum = 0.0;
        if self.atol_vector.is_empty() {
            for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
                let scale = atol + rtol * a.abs().max(b.abs());
                if !(scale >= 0.0 && scale.is_finite()) {
                    return Err(CoreError::InvalidInput(
                        "RODAS5P error scale overflow".into(),
                    ));
                }
                let z = scaled_error_component(*e, scale);
                sum += z * z;
            }
        } else {
            for (((e, a), b), atol_i) in error.iter().zip(y).zip(&self.y_new).zip(&self.atol_vector)
            {
                let scale = atol_i + rtol * a.abs().max(b.abs());
                if !(scale >= 0.0 && scale.is_finite()) {
                    return Err(CoreError::InvalidInput(
                        "RODAS5P error scale overflow".into(),
                    ));
                }
                let z = scaled_error_component(*e, scale);
                sum += z * z;
            }
        }
        let norm = (sum / n as f64).sqrt();
        Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        })
    }
}

/// Row-major partial-pivoting LU of `a` in place; `pivots[k]` is the row
/// swapped with row `k`. `row_end[i]` must bound the nonzero columns of row
/// `i` on entry; it is kept up to date under fill-in and row swaps, and every
/// update stops there, so a banded matrix costs O(n b^2) instead of O(n^3).
/// `l_start[i]` (`n` on entry) becomes the first stored multiplier of row
/// `i`. Zero multipliers skip their row update. Every skipped operation is
/// an exact zero, so the factors equal those of the full loops.
fn lu_in_place(
    a: &mut [f64],
    n: usize,
    pivots: &mut [usize],
    row_end: &mut [usize],
    l_start: &mut [usize],
) -> CoreResult<()> {
    for k in 0..n {
        let mut p = k;
        let mut max = a[k * n + k].abs();
        for i in k + 1..n {
            let v = a[i * n + k].abs();
            if v > max {
                max = v;
                p = i;
            }
        }
        if !(max > 0.0 && max.is_finite()) {
            return Err(CoreError::LinearSolve(format!(
                "RODAS5P fast LU: singular or non-finite pivot at column {k}"
            )));
        }
        pivots[k] = p;
        if p != k {
            // Both rows are zero beyond their ends; the pivot row reaches
            // column k, so this span covers their multipliers as well.
            let span = row_end[k].max(row_end[p]) + 1;
            for j in 0..span {
                a.swap(k * n + j, p * n + j);
            }
            row_end.swap(k, p);
            l_start.swap(k, p);
        }
        let pivot_end = row_end[k];
        let (top, bottom) = a.split_at_mut((k + 1) * n);
        let pivot_row = &top[k * n + k..k * n + pivot_end + 1];
        let pivot = pivot_row[0];
        for (offset, row) in bottom.chunks_exact_mut(n).enumerate() {
            if row[k] == 0.0 {
                continue;
            }
            let i = k + 1 + offset;
            let l = row[k] / pivot;
            row[k] = l;
            l_start[i] = l_start[i].min(k);
            for (x, r) in row[k + 1..=pivot_end].iter_mut().zip(&pivot_row[1..]) {
                *x -= l * r;
            }
            row_end[i] = row_end[i].max(pivot_end);
        }
    }
    Ok(())
}

/// Solve with the factors of [`lu_in_place`], overwriting `b`; the loops
/// stop at the stored extents of each row.
fn lu_solve_in_place(
    lu: &[f64],
    n: usize,
    pivots: &[usize],
    row_end: &[usize],
    l_start: &[usize],
    b: &mut [f64],
) {
    for (k, &p) in pivots.iter().enumerate().take(n) {
        b.swap(k, p);
    }
    for i in 0..n {
        let start = l_start[i].min(i);
        let row = &lu[i * n + start..i * n + i];
        let mut sum = b[i];
        for (l, x) in row.iter().zip(&b[start..i]) {
            sum -= l * x;
        }
        b[i] = sum;
    }
    for i in (0..n).rev() {
        let end = row_end[i];
        let row = &lu[i * n + i..i * n + end + 1];
        let mut sum = b[i];
        for (u, x) in row[1..].iter().zip(&b[i + 1..=end]) {
            sum -= u * x;
        }
        b[i] = sum / row[0];
    }
}

/// First index of a nonzero in `row`, tested eight entries at a time so
/// the chunk test can vectorize; the index equals `row.iter().position(..)`.
fn first_nonzero(row: &[f64]) -> Option<usize> {
    let mut start = 0;
    for chunk in row.chunks(8) {
        let any = chunk.iter().fold(false, |a, v| a | (*v != 0.0));
        if any {
            return chunk.iter().position(|v| *v != 0.0).map(|p| start + p);
        }
        start += chunk.len();
    }
    None
}

/// Last index of a nonzero in `row` (as `row.iter().rposition(..)`), tested
/// eight entries at a time from the end.
fn last_nonzero(row: &[f64]) -> Option<usize> {
    let mut end = row.len();
    for chunk in row.rchunks(8) {
        let any = chunk.iter().fold(false, |a, v| a | (*v != 0.0));
        if any {
            return chunk
                .iter()
                .rposition(|v| *v != 0.0)
                .map(|p| end - chunk.len() + p);
        }
        end -= chunk.len();
    }
    None
}

/// The extents of an assembled `W` for [`lu_in_place_col_extents`]:
/// `row_end[i]` as for [`lu_in_place`], `l_start[i] = n`, and `col_end[j]`
/// the last row whose span `[first nonzero, row_end]` contains column `j`.
fn extents_of(
    a: &[f64],
    n: usize,
    row_end: &mut [usize],
    l_start: &mut [usize],
    col_end: &mut [usize],
) {
    col_end[..n].fill(0);
    for (i, row) in a.chunks_exact(n).enumerate() {
        let last = last_nonzero(row).unwrap_or(0).max(i);
        row_end[i] = last;
        l_start[i] = n;
        let first = first_nonzero(row).unwrap_or(i).min(i);
        for c in &mut col_end[first..=last] {
            *c = (*c).max(i);
        }
    }
}

/// [`lu_in_place`] with exact column extents (speed research node SPD02):
/// `col_end[j]` bounds the rows that can hold a nonzero in column `j`
/// (from [`extents_of`] on entry; kept up to date under row swaps and
/// fill-in), so the pivot search and the elimination visit rows
/// `k + 1..=col_end[k]` only. Every skipped row holds an exact zero in
/// column `k`: the strict `>` never selects it and the elimination skips
/// it, so pivots, multipliers, extents and factors equal [`lu_in_place`]'s.
fn lu_in_place_col_extents(
    a: &mut [f64],
    n: usize,
    pivots: &mut [usize],
    row_end: &mut [usize],
    l_start: &mut [usize],
    col_end: &mut [usize],
) -> CoreResult<()> {
    for k in 0..n {
        let limit = col_end[k].min(n - 1);
        let mut p = k;
        let mut max = a[k * n + k].abs();
        for i in k + 1..=limit {
            let v = a[i * n + k].abs();
            if v > max {
                max = v;
                p = i;
            }
        }
        if !(max > 0.0 && max.is_finite()) {
            return Err(CoreError::LinearSolve(format!(
                "RODAS5P fast LU: singular or non-finite pivot at column {k}"
            )));
        }
        pivots[k] = p;
        if p != k {
            let span = row_end[k].max(row_end[p]) + 1;
            for j in 0..span {
                a.swap(k * n + j, p * n + j);
            }
            row_end.swap(k, p);
            l_start.swap(k, p);
            // Row k's entries now sit in row p.
            for c in &mut col_end[..span] {
                *c = (*c).max(p);
            }
        }
        if limit <= k {
            continue;
        }
        let pivot_end = row_end[k];
        let (top, bottom) = a.split_at_mut((k + 1) * n);
        let pivot_row = &top[k * n + k..k * n + pivot_end + 1];
        let pivot = pivot_row[0];
        let mut touched = k;
        for (offset, row) in bottom[..(limit - k) * n].chunks_exact_mut(n).enumerate() {
            if row[k] == 0.0 {
                continue;
            }
            let i = k + 1 + offset;
            let l = row[k] / pivot;
            row[k] = l;
            l_start[i] = l_start[i].min(k);
            for (x, r) in row[k + 1..=pivot_end].iter_mut().zip(&pivot_row[1..]) {
                *x -= l * r;
            }
            row_end[i] = row_end[i].max(pivot_end);
            touched = i;
        }
        if touched > k {
            // Fill-in of the updated rows reaches column pivot_end.
            for c in &mut col_end[k + 1..=pivot_end] {
                *c = (*c).max(touched);
            }
        }
    }
    Ok(())
}

/// The in-place LU variants for the contract tests of speed research node
/// SPD02. Research only.
#[doc(hidden)]
pub mod lu_research {
    use rodas5p_core::CoreResult;

    /// v2's LU ([`super::lu_in_place`]).
    pub fn zero_skipping(
        a: &mut [f64],
        n: usize,
        pivots: &mut [usize],
        row_end: &mut [usize],
        l_start: &mut [usize],
    ) -> CoreResult<()> {
        super::lu_in_place(a, n, pivots, row_end, l_start)
    }

    /// The column-extent LU ([`super::lu_in_place_col_extents`]).
    pub fn column_extents(
        a: &mut [f64],
        n: usize,
        pivots: &mut [usize],
        row_end: &mut [usize],
        l_start: &mut [usize],
        col_end: &mut [usize],
    ) -> CoreResult<()> {
        super::lu_in_place_col_extents(a, n, pivots, row_end, l_start, col_end)
    }

    /// The extents of an assembled matrix ([`super::extents_of`]).
    pub fn extents_of(
        a: &[f64],
        n: usize,
        row_end: &mut [usize],
        l_start: &mut [usize],
        col_end: &mut [usize],
    ) {
        super::extents_of(a, n, row_end, l_start, col_end)
    }

    /// The solve with either variant's factors ([`super::lu_solve_in_place`]).
    pub fn solve(
        lu: &[f64],
        n: usize,
        pivots: &[usize],
        row_end: &[usize],
        l_start: &[usize],
        b: &mut [f64],
    ) {
        super::lu_solve_in_place(lu, n, pivots, row_end, l_start, b)
    }
}

/// Factor `W = I/(h gamma) - J` for a Jacobian band `jacobian` (layout of
/// [`BandedJacobianFn`]) with the banded pipeline's LU and solve `W x = b`:
/// the solution and the pivot row of each column. For contract tests of
/// [`integrate_rodas5p_fast_banded_observed`]; `inv = 1/(h gamma)`.
pub fn rodas5p_fast_banded_solve(
    n: usize,
    lower: usize,
    upper: usize,
    jacobian: &[f64],
    inv: f64,
    b: &[f64],
) -> CoreResult<(Vec<f64>, Vec<usize>)> {
    let invalid = || CoreError::InvalidInput("invalid banded solve input or storage size".into());
    if n == 0 || lower >= n || upper >= n || b.len() != n {
        return Err(invalid());
    }
    let width = lower
        .checked_mul(2)
        .and_then(|v| v.checked_add(upper))
        .and_then(|v| v.checked_add(1))
        .ok_or_else(invalid)?;
    let jwidth = lower
        .checked_add(upper)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(invalid)?;
    let jlen = n.checked_mul(jwidth).ok_or_else(invalid)?;
    let flen = n
        .checked_mul(width)
        .filter(|v| *v <= isize::MAX as usize / std::mem::size_of::<f64>())
        .ok_or_else(invalid)?;
    if jacobian.len() != jlen {
        return Err(invalid());
    }
    if !inv.is_finite() || jacobian.iter().chain(b).any(|x| !x.is_finite()) {
        return Err(CoreError::NonFinite("nonfinite banded solve input".into()));
    }
    let mut state = BandState {
        l: lower,
        u: upper,
        fill: Arc::new(|_, _, _| Ok(())),
        jacobian: jacobian.to_vec(),
        factors: vec![0.0; flen],
        pivots: vec![0; n],
        work: BandedWork::default(),
        kernel: BandedKernel::Indexed,
        exact_cache: None,
    };
    state.factor(n, inv)?;
    let mut x = b.to_vec();
    state.solve(n, &mut x);
    if x.iter().any(|v| !v.is_finite()) {
        return Err(CoreError::LinearSolve(
            "banded solve produced NaN/Inf".into(),
        ));
    }
    Ok((x, state.pivots))
}

/// The banded pipeline's Jacobian band, LU and solve as a client
/// [`StageLinearSystem`] (library step L2): the [`BandedKernel::Indexed`]
/// kernel without the exact cache, charging what the banded driver charges
/// (one `direct_factorizations` per factorization, one `direct_solve_calls`
/// per solve). Under [`integrate_rodas5p_fast_client_observed_with_atol`]
/// it reproduces [`integrate_rodas5p_fast_banded_observed_with_atol`] bit
/// for bit (states and counters); it is the reference client of that entry
/// point.
pub struct BandedStageLu {
    n: usize,
    state: BandState,
}

impl BandedStageLu {
    /// Band storage of order `n` for `band`. Requires `lower, upper < n`.
    pub fn new(n: usize, band: &BandedJacobian) -> CoreResult<Self> {
        if band.lower >= n.max(1) || band.upper >= n.max(1) {
            return Err(CoreError::InvalidInput(
                "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
            ));
        }
        Ok(Self {
            n,
            state: BandState::new(n, band, BandedKernel::Indexed),
        })
    }
}

impl StageLinearSystem for BandedStageLu {
    fn dimension(&self) -> usize {
        self.n
    }

    /// The band of `J` at `(t, y)` from the band callback; the driver has
    /// charged `jacobian_builds`.
    fn linearize(
        &mut self,
        _problem: &OdeProblem,
        t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.state.jacobian.fill(0.0);
        (self.state.fill)(t, y, &mut self.state.jacobian)
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        self.state.factor_with_policy(self.n, inv_h_gamma, counters)
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if b.len() != self.n {
            return Err(CoreError::Dimension(
                "banded stage solve shape mismatch".into(),
            ));
        }
        counters.direct_solve_calls += 1;
        self.state.solve(self.n, b);
        Ok(())
    }
}

/// One step of this driver from `(t, y)` with step `h`: the new state, the
/// WRMS norm of the embedded error and the LU used. For verification
/// against [`crate::sequential_step`]; the adaptive driver keeps its
/// workspace across steps instead.
pub fn rodas5p_fast_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    atol: f64,
    rtol: f64,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, f64, Rodas5pFastLu)> {
    if y.len() != problem.dimension || problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast step input".into(),
        ));
    }
    let mut work = Workspace::new(problem.dimension, FastLuPolicy::Legacy)?;
    let error = work.attempt(problem, t, y, h, true, atol, rtol, counters)?;
    Ok((work.y_new, error, work.lu_kind))
}

pub(crate) fn failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

/// A per-step observer of [`integrate_fast`] (wave-3 node `p0-fastdense`):
/// the continuous extension and the W-filtered interior estimate of
/// [`crate::integrate_rodas5p_fast_dense_observed`]. The driver is generic
/// over it, so [`NoSink`] compiles to the unchanged loop.
pub(crate) trait StepSink {
    /// Called after an attempt from `(t, y)` with step `h` whose embedded
    /// error `error` is at most one, with the attempt's stages, new state
    /// and factored `W` still in `work`. Returns the error the controller
    /// sees for this attempt; an `Err` is classified as an attempt error.
    #[allow(clippy::too_many_arguments)]
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
    ) -> CoreResult<f64>;

    /// Called once per accepted step, after the state advanced to `t_new`
    /// (`work.y_new` and `work.u` still hold the step).
    fn accepted(&mut self, work: &Workspace, t_new: f64);

    /// Called once per accepted step after the controller update (library
    /// step L3); [`AcceptControl::Stop`] ends the run at `view.t`. Only
    /// [`ControlSink`] overrides it, so the other sinks keep their loop.
    #[inline(always)]
    fn accept_control(&mut self, _view: &AcceptedStepView<'_>) -> AcceptControl {
        AcceptControl::Continue
    }
}

/// The sink of the `_and_control` entry points: observes no attempt and
/// hands every accepted step to the client's accept control.
struct ControlSink<'c> {
    control: &'c mut dyn FnMut(&AcceptedStepView<'_>) -> AcceptControl,
}

impl StepSink for ControlSink<'_> {
    #[inline(always)]
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

    #[inline(always)]
    fn accepted(&mut self, _work: &Workspace, _t_new: f64) {}

    fn accept_control(&mut self, view: &AcceptedStepView<'_>) -> AcceptControl {
        (self.control)(view)
    }
}

/// The sink of every existing entry point: observes nothing.
pub(crate) struct NoSink;

impl StepSink for NoSink {
    #[inline(always)]
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

    #[inline(always)]
    fn accepted(&mut self, _work: &Workspace, _t_new: f64) {}
}

/// Adaptive RODAS5P with the sequential driver's controller and output
/// rules, on the transformed stages and one workspace. Requires the
/// identity mass matrix.
pub fn integrate_rodas5p_fast_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pFastResult> {
    integrate_fast(
        problem,
        None,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        None,
        &[],
        &mut NoSink,
    )
    .map(|(result, ..)| result)
}

/// [`integrate_rodas5p_fast_observed`] with the partial record of a run that
/// stopped before `tf` (`None` on success): the same run, bit for bit.
/// `Rodas5pFastResult` is also the lagged driver's result type, so the
/// record travels beside it.
pub fn integrate_rodas5p_fast_observed_with_partial(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<(Rodas5pFastResult, Option<DriverPartial>)> {
    integrate_fast(
        problem,
        None,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        None,
        &[],
        &mut NoSink,
    )
    .map(|(result, _, partial)| (result, partial))
}

/// [`integrate_rodas5p_fast_observed`] with opt-in controller telemetry
/// (research node CT01): the same run, bit for bit, and the counts of the
/// registered controller events ([`ControllerTelemetry`]). Pass
/// [`ControllerTelemetry::with_trace`] to also record every controller
/// update.
pub fn integrate_rodas5p_fast_observed_with_telemetry(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    mut telemetry: ControllerTelemetry,
) -> CoreResult<(Rodas5pFastResult, ControllerTelemetry)> {
    integrate_fast(
        problem,
        None,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        Some(&mut telemetry),
        &[],
        &mut NoSink,
    )
    .map(|(result, ..)| (result, telemetry))
}

/// [`integrate_rodas5p_fast_observed`] with opt-in [`Rodas5pFastOptions`]
/// (speed research node SPD01); the default options give the same driver.
pub fn integrate_rodas5p_fast_observed_with_options(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
) -> CoreResult<Rodas5pFastResult> {
    integrate_fast(
        problem,
        None,
        t_span,
        y0,
        adaptive,
        output,
        options,
        None,
        &[],
        &mut NoSink,
    )
    .map(|(result, ..)| result)
}

/// [`integrate_rodas5p_fast_observed`] with an explicit band structure
/// (integrated DAG node INT-03): the Jacobian comes from `band`, `W` is
/// assembled and factored on the band (partial pivoting over the `lower`
/// rows below the diagonal, `U` with bandwidth `upper + lower`), and
/// nothing is stored as an `n x n` matrix. The controller, clock, output
/// and rejection rules are v2's. Requires `lower, upper < n`.
pub fn integrate_rodas5p_fast_banded_observed(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pFastBandedResult> {
    integrate_rodas5p_fast_banded_observed_with_options(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
    )
}

/// [`integrate_rodas5p_fast_banded_observed`] with opt-in
/// [`Rodas5pFastOptions`] (speed research node SPD01).
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_fast_banded_observed_with_options(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
) -> CoreResult<Rodas5pFastBandedResult> {
    let n = problem.dimension;
    if band.lower >= n.max(1) || band.upper >= n.max(1) {
        return Err(CoreError::InvalidInput(
            "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
        ));
    }
    integrate_banded(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        output,
        options,
        BandedKernel::Indexed,
        &[],
        &mut NoSink,
    )
}

/// [`integrate_rodas5p_fast_banded_observed`] with an explicit
/// [`BandedKernel`] (speed research node SPD03); `Indexed` is the existing
/// driver.
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_fast_banded_observed_with_kernel(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    kernel: BandedKernel,
) -> CoreResult<Rodas5pFastBandedResult> {
    let n = problem.dimension;
    if band.lower >= n.max(1) || band.upper >= n.max(1) {
        return Err(CoreError::InvalidInput(
            "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
        ));
    }
    integrate_banded(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        kernel,
        &[],
        &mut NoSink,
    )
}

/// The per-component absolute tolerance of the fast drivers: length `n`,
/// every entry finite and nonnegative. An empty slice (the scalar path) is
/// accepted by the private driver only; the public entry points require `n`
/// entries.
fn validate_atol_vector(n: usize, atol: &[f64]) -> CoreResult<()> {
    if atol.is_empty() {
        return Ok(());
    }
    if atol.len() != n {
        return Err(CoreError::InvalidInput(format!(
            "the absolute tolerance vector has {} entries, the dimension is {n}",
            atol.len()
        )));
    }
    if !atol.iter().all(|a| a.is_finite() && *a >= 0.0) {
        return Err(CoreError::InvalidInput(
            "every absolute tolerance must be finite and nonnegative".into(),
        ));
    }
    Ok(())
}

/// [`integrate_rodas5p_fast_observed_with_partial`] with a per-component
/// absolute tolerance: component `i` of the error norm is scaled by
/// `atol[i] + rtol max(|y_i|, |y_new,i|)` and `adaptive.atol` is validated
/// but unused. `atol` must have `n` finite nonnegative entries (otherwise
/// [`CoreError::InvalidInput`]); an entry of zero follows the zero-scale rule
/// of [`rodas5p_core::scaled_error_component`]. With every entry equal to
/// `adaptive.atol` the run is bit for bit the scalar run.
pub fn integrate_rodas5p_fast_observed_with_atol(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    atol: &[f64],
) -> CoreResult<(Rodas5pFastResult, Option<DriverPartial>)> {
    require_atol_vector(problem.dimension, atol)?;
    integrate_fast(
        problem,
        None,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        None,
        atol,
        &mut NoSink,
    )
    .map(|(result, _, partial)| (result, partial))
}

/// [`integrate_rodas5p_fast_banded_observed`] with a per-component absolute
/// tolerance; see [`integrate_rodas5p_fast_observed_with_atol`].
pub fn integrate_rodas5p_fast_banded_observed_with_atol(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    atol: &[f64],
) -> CoreResult<Rodas5pFastBandedResult> {
    let n = problem.dimension;
    require_atol_vector(n, atol)?;
    if band.lower >= n.max(1) || band.upper >= n.max(1) {
        return Err(CoreError::InvalidInput(
            "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
        ));
    }
    integrate_banded(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        BandedKernel::Indexed,
        atol,
        &mut NoSink,
    )
}

/// [`integrate_rodas5p_fast_banded_observed_with_atol`] with an accept
/// control (library step L3); see
/// [`integrate_rodas5p_fast_client_observed_with_atol_and_control`] for the
/// control's view, the stop and the partial record.
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_fast_banded_observed_with_atol_and_control(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    atol: &[f64],
    control: &mut dyn FnMut(&AcceptedStepView<'_>) -> AcceptControl,
) -> CoreResult<Rodas5pFastBandedResult> {
    let n = problem.dimension;
    require_atol_vector(n, atol)?;
    if band.lower >= n.max(1) || band.upper >= n.max(1) {
        return Err(CoreError::InvalidInput(
            "the band of the RODAS5P fast banded driver must lie inside the matrix".into(),
        ));
    }
    integrate_banded(
        problem,
        band,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        BandedKernel::Indexed,
        atol,
        &mut ControlSink { control },
    )
}

/// [`integrate_rodas5p_fast_observed_with_atol`] with the stage linear
/// algebra owned by `client` (library step L2): the client linearizes at
/// every fresh state, factors `W = I/(h gamma) - J` once per attempt and
/// solves every stage, while the stages, the controller, the clock, the
/// output and the rejection rules are the driver's own (see
/// [`StageLinearSystem`] for the counters each side charges and the errors
/// that reject an attempt). Nothing is stored as an `n x n` matrix. The
/// result's `lu` is [`Rodas5pFastLu::Client`] and its `driver`
/// [`RODAS5P_FAST_CLIENT_DRIVER_ID`].
///
/// Every input is checked before the client is used: its dimension must be
/// the problem's, `atol` as for [`integrate_rodas5p_fast_observed_with_atol`],
/// the identity mass matrix only (otherwise [`CoreError::InvalidInput`]).
/// The client is handed back with the result; on any `Err` (a refusal or a
/// fatal error of the run) it is dropped.
pub fn integrate_rodas5p_fast_client_observed_with_atol(
    problem: &OdeProblem,
    client: Box<dyn StageLinearSystem>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    atol: &[f64],
) -> CoreResult<(
    Rodas5pFastResult,
    Option<DriverPartial>,
    Box<dyn StageLinearSystem>,
)> {
    integrate_client(
        problem,
        client,
        t_span,
        y0,
        adaptive,
        output,
        atol,
        &mut NoSink,
    )
}

/// [`integrate_rodas5p_fast_client_observed_with_atol`] with an accept
/// control (library step L3): after every accepted step, the last one
/// included, `control` sees the accepted state and the controller's
/// proposal for the next step ([`AcceptedStepView`]) and may end the run
/// there, for example to remap the state onto a new mesh outside the driver
/// and restart from it with `initial_step = h_next`. The proposal is capped
/// at `max_step`, so that restart is admitted. Under the Integral controller
/// it continues the uninterrupted run bit for bit; the other controllers
/// restart without their error history. A proposal below `min_step` is
/// refused as an initial step (`InvalidInput`).
///
/// [`AcceptControl::Stop`] before `tf` ends the run at the accepted `(t, y)`:
/// the output holds every output time at or before `t`, `observed.success`
/// is false and the partial record is
/// `DriverPartial { t, y, reason: ClientStop, last_failure: None, h_next: Some(h_next) }`.
/// A stop on the step that reaches `tf` changes nothing: the run is complete
/// and reported as a success without a partial record. A control that always
/// continues gives the run without one, bit for bit (states, counters and
/// step statistics). The control is called once per accepted step through
/// `dyn`; the driver allocates nothing for it.
#[allow(clippy::too_many_arguments)]
pub fn integrate_rodas5p_fast_client_observed_with_atol_and_control(
    problem: &OdeProblem,
    client: Box<dyn StageLinearSystem>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    atol: &[f64],
    control: &mut dyn FnMut(&AcceptedStepView<'_>) -> AcceptControl,
) -> CoreResult<(
    Rodas5pFastResult,
    Option<DriverPartial>,
    Box<dyn StageLinearSystem>,
)> {
    integrate_client(
        problem,
        client,
        t_span,
        y0,
        adaptive,
        output,
        atol,
        &mut ControlSink { control },
    )
}

/// The client run of [`integrate_rodas5p_fast_client_observed_with_atol`]
/// under `sink`.
#[allow(clippy::too_many_arguments)]
fn integrate_client<S: StepSink>(
    problem: &OdeProblem,
    client: Box<dyn StageLinearSystem>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    atol: &[f64],
    sink: &mut S,
) -> CoreResult<(
    Rodas5pFastResult,
    Option<DriverPartial>,
    Box<dyn StageLinearSystem>,
)> {
    let n = problem.dimension;
    require_atol_vector(n, atol)?;
    if client.dimension() != n {
        return Err(CoreError::InvalidInput(format!(
            "the stage linear system has dimension {}, the problem {n}",
            client.dimension()
        )));
    }
    validate_fast_run(problem, t_span, y0, adaptive, atol)?;
    let mut work = Workspace::new_client(n, client)?;
    let (result, _, partial) = integrate_fast_in(
        &mut work,
        problem,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
        None,
        atol,
        sink,
    )?;
    let client = work
        .client
        .take()
        .expect("client workspace")
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    Ok((result, partial, client))
}

/// The public entry points take a vector, never the empty "scalar" marker.
pub(crate) fn require_atol_vector(n: usize, atol: &[f64]) -> CoreResult<()> {
    if atol.is_empty() && n > 0 {
        return Err(CoreError::InvalidInput(
            "the absolute tolerance vector must have one entry per component".into(),
        ));
    }
    validate_atol_vector(n, atol)
}

#[allow(clippy::too_many_arguments)]
fn integrate_banded<S: StepSink>(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
    kernel: BandedKernel,
    atol_vector: &[f64],
    sink: &mut S,
) -> CoreResult<Rodas5pFastBandedResult> {
    let (fast, work, partial) = integrate_fast(
        problem,
        Some((band, kernel)),
        t_span,
        y0,
        adaptive,
        output,
        options,
        None,
        atol_vector,
        sink,
    )?;
    Ok(Rodas5pFastBandedResult {
        fast,
        work: work.unwrap_or_default(),
        partial,
    })
}

/// The input checks of every fast-driver run, made before its workspace is
/// built.
fn validate_fast_run(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    atol_vector: &[f64],
) -> CoreResult<()> {
    adaptive.validate()?;
    validate_atol_vector(problem.dimension, atol_vector)?;
    let (t, tf) = t_span;
    if tf < t || y0.len() != problem.dimension {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast integration input".into(),
        ));
    }
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "the RODAS5P fast driver supports the identity mass matrix only".into(),
        ));
    }
    Ok(())
}

/// The fast driver loop. `sink` observes every attempt the embedded
/// estimate accepts and every accepted step; [`NoSink`] observes nothing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn integrate_fast<S: StepSink>(
    problem: &OdeProblem,
    band: Option<(&BandedJacobian, BandedKernel)>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
    telemetry: Option<&mut ControllerTelemetry>,
    atol_vector: &[f64],
    sink: &mut S,
) -> CoreResult<(Rodas5pFastResult, Option<BandedWork>, Option<DriverPartial>)> {
    validate_fast_run(problem, t_span, y0, adaptive, atol_vector)?;
    let mut work = match band {
        Some((band, kernel)) => Workspace::new_banded(problem.dimension, band, kernel)?,
        None => Workspace::new(problem.dimension, options.lu_policy)?,
    };
    integrate_fast_in(
        &mut work,
        problem,
        t_span,
        y0,
        adaptive,
        output,
        options,
        telemetry,
        atol_vector,
        sink,
    )
}

/// [`integrate_fast`] on a built workspace (dense, banded or client) for
/// inputs checked by [`validate_fast_run`]; the driver identifier follows
/// the workspace's storage.
#[allow(clippy::too_many_arguments)]
fn integrate_fast_in<S: StepSink>(
    work: &mut Workspace,
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
    mut telemetry: Option<&mut ControllerTelemetry>,
    atol_vector: &[f64],
    sink: &mut S,
) -> CoreResult<(Rodas5pFastResult, Option<BandedWork>, Option<DriverPartial>)> {
    let (mut t, tf) = t_span;
    work.atol_vector = atol_vector.to_vec();
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let (mut attempts, mut accepted_steps, mut rejected_steps, mut reuses) = (0, 0, 0, 0);
    let mut internal_steps = 0_usize;
    // Whether the Jacobian, f(t, y) and f_t in the workspace belong to (t, y).
    let mut fresh_state = false;
    // The kind of the last rejection since the last accepted step (the
    // partial record only; never read by the step arithmetic).
    let mut last_failure = None;
    // The controller's proposal when the accept control stopped the run.
    let mut client_stop = None;
    let mut fatal_callback = false;
    while t < tf && attempts < adaptive.max_attempts {
        let Some(next_h) =
            crate::output::adaptive_end_step(t, h, tf, adaptive.step_cap(), &controller)?
        else {
            break;
        };
        h = next_h;
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let (trial_h, clipped) = if options.fused_landing {
            // `h` was landed toward `tf` above with the collector's own cap.
            debug_assert_eq!(collector.max_step(), adaptive.step_cap());
            collector.limit_landed_step(t, h)?
        } else {
            collector.limit_step(t, h, tf)?
        };
        attempts += 1;
        if fresh_state {
            reuses += 1;
        }
        let outcome = work.attempt(
            problem,
            t,
            &y,
            trial_h,
            !fresh_state,
            adaptive.atol,
            adaptive.rtol,
            &mut counters,
        );
        // The sink sees an attempt the embedded estimate accepts and may
        // raise its error (NoSink returns it unchanged). An error of the sink
        // comes after a successful attempt, so the state's Jacobian, f(t, y)
        // and f_t stay valid for the retry (`sink_failed` below).
        let mut sink_failed = false;
        let outcome = match outcome {
            Ok(error) if error <= 1.0 => sink
                .after_attempt(
                    problem,
                    work,
                    t,
                    &y,
                    trial_h,
                    error,
                    adaptive.atol,
                    adaptive.rtol,
                    &mut counters,
                )
                .inspect_err(|_| sink_failed = true),
            other => other,
        };
        // Every path below has the state's Jacobian and f(t, y) in place,
        // unless they failed to build.
        fresh_state = work.lu_chosen;
        let (error, failure) = match outcome {
            Ok(error) if error <= 1.0 => (error, None),
            Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
            Err(CoreError::FatalCallback(_)) => {
                fatal_callback = true;
                (f64::INFINITY, Some(AdaptiveFailureKind::NonFinite))
            }
            Err(error) => match failure_kind(&error) {
                Some(kind) => {
                    // A failed attempt may have left a partial Jacobian or
                    // f(t, y); a failed sink did not (always false for NoSink).
                    if !sink_failed {
                        fresh_state = false;
                    }
                    (f64::INFINITY, Some(kind))
                }
                None => return Err(error),
            },
        };
        match failure {
            None => {
                counters.accepted_steps += 1;
                accepted_steps += 1;
                t += trial_h;
                y.copy_from_slice(&work.y_new);
                if options.fused_landing {
                    // `y_new` was checked finite by the attempt.
                    collector.accept_prechecked(t, &y, clipped)?;
                } else {
                    collector.accept(t, &y, clipped)?;
                }
                internal_steps += 1;
                sink.accepted(work, t);
                fresh_state = false;
                last_failure = None;
            }
            Some(kind) => {
                counters.rejected_steps += 1;
                rejected_steps += 1;
                last_failure = Some(kind);
                match kind {
                    AdaptiveFailureKind::LocalError => counters.local_error_failures += 1,
                    AdaptiveFailureKind::LinearSolve => counters.linear_solve_failures += 1,
                    AdaptiveFailureKind::NonlinearSolve => counters.nonlinear_solve_failures += 1,
                    AdaptiveFailureKind::NonFinite => counters.nonfinite_step_failures += 1,
                }
            }
        }
        if fatal_callback {
            break;
        }
        let requested_h = h;
        h = if options.prevalidated_controller {
            rodas_next_step_after_attempt_prevalidated(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                failure.is_none(),
                clipped,
            )?
        } else {
            rodas_next_step_after_attempt(
                &mut controller,
                adaptive,
                h,
                trial_h,
                error,
                failure.is_none(),
                clipped,
            )?
        };
        // CT01 telemetry: an observer of the update only (nothing is fed back).
        if let Some(telemetry) = telemetry.as_deref_mut() {
            telemetry.observe(ControllerDecision {
                requested_h,
                trial_h,
                error,
                accepted: failure.is_none(),
                clipped,
                next_h: h,
            });
        }
        // L3 accept control: after the output collector took the step and
        // the controller proposed the next one (NoSink always continues).
        if failure.is_none() {
            // The proposal capped at `max_step`, as the next pass caps it
            // (`adaptive_end_step`), so that a restart from it is admitted.
            let h_next = h.min(adaptive.max_step);
            let view = AcceptedStepView {
                t,
                y: &y,
                h_used: trial_h,
                h_next,
                accepted_steps,
                attempts,
                counters: &counters,
            };
            if sink.accept_control(&view) == AcceptControl::Stop {
                client_stop = Some(h_next);
                break;
            }
        }
    }
    let success = t >= tf;
    let mut partial =
        DriverPartial::after_loop(t, tf, &y, attempts, adaptive.max_attempts, last_failure);
    if fatal_callback {
        partial
            .as_mut()
            .expect("fatal trial leaves an accepted prefix")
            .reason = DriverStopReason::FatalCallback;
    }
    // A stop before `tf` is the client's, even on the budget's last attempt;
    // a stop at `tf` leaves the completed run a success.
    if let (Some(partial), Some(h_next)) = (partial.as_mut(), client_stop) {
        partial.reason = DriverStopReason::ClientStop;
        partial.h_next = Some(h_next);
    }
    let (times, states, output_clipped_steps) = if success {
        collector.finish()?
    } else {
        collector.finish_partial()
    };
    let banded = work.band.as_ref().map(|band| band.work);
    let result = Rodas5pFastResult {
        observed: ObservedIntegrationResult {
            t: times,
            y: states,
            success,
            message: if success {
                "success".into()
            } else if fatal_callback {
                "fatal callback; accepted prefix retained".into()
            } else if client_stop.is_some() {
                "stopped by the client's accept control".into()
            } else {
                "maximum step count or minimum step reached".into()
            },
            counters,
            internal_steps,
            output_clipped_steps,
        },
        attempts,
        accepted_steps,
        rejected_steps,
        jacobian_reuses: reuses,
        lu: work.lu_kind,
        driver: match (work.lu_kind, work.band.as_ref().map(|band| band.kernel)) {
            (Rodas5pFastLu::Client, _) => RODAS5P_FAST_CLIENT_DRIVER_ID,
            (_, Some(BandedKernel::Slices)) => RODAS5P_FAST_BANDED_SLICES_DRIVER_ID,
            (_, Some(BandedKernel::FixedWidth2)) => RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID,
            (_, Some(BandedKernel::FixedWidth2ExactCache)) => {
                RODAS5P_FAST_BANDED_FIXED2_EXACT_CACHE_DRIVER_ID
            }
            _ => options.driver_id(banded.is_some()),
        },
    };
    Ok((result, banded, partial))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_error_norm_refuses_overflowed_scalar_and_vector_scales() {
        let problem = OdeProblem::new(
            "finite constant boundary state",
            1,
            Arc::new(|_, _, out| {
                out.fill(0.0);
                Ok(())
            }),
            None,
            Some(Arc::new(|_, _| DenseMatrix::new(1, 1, vec![0.0]))),
            None,
            None,
            true,
            None,
            None,
        )
        .unwrap();
        for vector in [false, true] {
            let mut workspace = Workspace::new(1, FastLuPolicy::Legacy).unwrap();
            if vector {
                workspace.atol_vector = vec![1e308];
            }
            let mut counters = WorkCounters::default();
            // States, errors and configured tolerances are finite, but their
            // scale sum overflows. It must not silently become a zero norm.
            assert!(matches!(
                workspace.attempt_for_tests(
                    &problem,
                    0.0,
                    &[1e308],
                    1e-3,
                    1e308,
                    1.0,
                    &mut counters,
                ),
                Err(CoreError::InvalidInput(_))
            ));
            // Removing the relative contribution leaves a valid finite scale.
            assert_eq!(
                workspace
                    .attempt_for_tests(&problem, 0.0, &[1e308], 1e-3, 1e308, 0.0, &mut counters,)
                    .unwrap(),
                0.0
            );
        }
    }

    #[test]
    fn in_place_lu_solves_pivoted_and_banded_systems() {
        // Needs a row swap at the first column.
        let n = 4;
        let a = [
            0.0, 2.0, 1.0, 0.0, //
            3.0, 1.0, 0.0, 0.0, //
            1.0, 0.0, 4.0, 1.0, //
            0.0, 0.0, 1.0, 5.0,
        ];
        let x = [1.0, -2.0, 0.5, 3.0];
        let b: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|j| a[i * n + j] * x[j]).sum())
            .collect();
        let mut lu = a.to_vec();
        let mut piv = vec![0; n];
        let mut row_end: Vec<usize> = (0..n)
            .map(|i| (0..n).rev().find(|&j| a[i * n + j] != 0.0).unwrap().max(i))
            .collect();
        let mut l_start = vec![n; n];
        lu_in_place(&mut lu, n, &mut piv, &mut row_end, &mut l_start).unwrap();
        let mut sol = b.clone();
        lu_solve_in_place(&lu, n, &piv, &row_end, &l_start, &mut sol);
        for (s, e) in sol.iter().zip(x) {
            assert!((s - e).abs() <= 1e-14, "{sol:?}");
        }
        // A singular matrix is a typed linear-solve error.
        let mut singular = vec![1.0, 2.0, 2.0, 4.0];
        assert!(matches!(
            lu_in_place(&mut singular, 2, &mut [0, 0], &mut [1, 1], &mut [2, 2]),
            Err(CoreError::LinearSolve(_))
        ));
    }
}

#[cfg(test)]
#[path = "banded_exact_tests.rs"]
mod banded_exact_tests;

/// Stage at which an unrecoverable native session error was observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rodas5pSessionFailureKind {
    CallbackError,
    Clock,
    Output,
    Controller,
}
#[derive(Debug)]
pub struct Rodas5pSessionFailure {
    pub kind: Rodas5pSessionFailureKind,
    pub error: CoreError,
}
#[derive(Debug)]
pub enum Rodas5pSessionTermination {
    Complete,
    Paused,
    DriverStop(DriverStopReason),
    Failure(Rodas5pSessionFailure),
}
/// A session always retains its accepted physical prefix, cumulative work,
/// and original terminal error, including errors outside retry categories.
#[derive(Debug)]
pub struct Rodas5pFastSessionResult {
    pub run: Rodas5pFastResult,
    pub final_time: f64,
    pub final_state: Vec<f64>,
    pub termination: Rodas5pSessionTermination,
}

/// Stage-1 persistent native dense-Jacobian RODAS5P endpoint session.
/// One NoSink workspace and one immutable landing schedule are retained.
/// Banded, client, dense-output, mass, matrix-free and sequential session
/// routes are not supplied by this API. Provider/epoch replacement requires
/// a new session; no cache is carried across an identity change.
pub struct Rodas5pFastSession<'a> {
    problem: &'a OdeProblem,
    identity: crate::NativeSessionIdentity,
    adaptive: AdaptiveStepConfig,
    schedule: OutputSchedule,
    work: Workspace,
    collector: OutputCollector,
    y: Vec<f64>,
    t: f64,
    tf: f64,
    h: f64,
    controller: AdaptiveControllerState,
    counters: WorkCounters,
    attempts: usize,
    accepted_steps: usize,
    rejected_steps: usize,
    reuses: usize,
    internal_steps: usize,
    fresh_state: bool,
    last_failure: Option<AdaptiveFailureKind>,
    terminal: Option<DriverStopReason>,
    failure: Option<Rodas5pSessionFailure>,
}
impl<'a> Rodas5pFastSession<'a> {
    pub fn new(
        problem: &'a OdeProblem,
        t_span: (f64, f64),
        y0: &[f64],
        adaptive: &AdaptiveStepConfig,
        output: &OutputSchedule,
        identity: crate::NativeSessionIdentity,
    ) -> CoreResult<Self> {
        crate::NativeSessionIdentity {
            method: crate::NativeSessionMethod::Rodas5pFast,
            dimension: problem.dimension,
            ..identity
        }
        .check(identity)?;
        if problem.declared_band().is_some() {
            return Err(CoreError::InvalidInput(
                "banded native sessions are not supported by this stage-1 API".into(),
            ));
        }
        if !problem.has_explicit_jacobian() {
            return Err(CoreError::InvalidInput(
                "native RODAS session requires an explicit dense Jacobian".into(),
            ));
        }
        validate_fast_run(problem, t_span, y0, adaptive, &[])?;
        let collector =
            OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
        Ok(Self {
            problem,
            identity,
            adaptive: adaptive.clone(),
            schedule: output.clone(),
            work: Workspace::new(problem.dimension, Rodas5pFastOptions::default().lu_policy)?,
            collector,
            y: y0.to_vec(),
            t: t_span.0,
            tf: t_span.1,
            h: adaptive
                .initial_step
                .min(crate::output::step_to(t_span.0, t_span.1)?),
            controller: AdaptiveControllerState::default(),
            counters: WorkCounters::default(),
            attempts: 0,
            accepted_steps: 0,
            rejected_steps: 0,
            reuses: 0,
            internal_steps: 0,
            fresh_state: false,
            last_failure: None,
            terminal: None,
            failure: None,
        })
    }
    fn latch_failure(
        &mut self,
        kind: Rodas5pSessionFailureKind,
        error: CoreError,
        reject_entered_trial: bool,
    ) {
        if reject_entered_trial {
            self.counters.rejected_steps += 1;
            self.rejected_steps += 1;
            self.last_failure = None;
        }
        self.fresh_state = false;
        if self.failure.is_none() {
            self.failure = Some(Rodas5pSessionFailure { kind, error });
        }
    }
    pub fn failure(&self) -> Option<&Rodas5pSessionFailure> {
        self.failure.as_ref()
    }
    pub fn checkpoint(&self) -> crate::NativeSessionCheckpoint<'_> {
        crate::NativeSessionCheckpoint {
            t: self.t,
            y: &self.y,
            proposed_h: self.h,
            attempts: self.attempts,
            accepted_steps: self.accepted_steps,
            rejected_steps: self.rejected_steps,
            counters: &self.counters,
        }
    }
    pub fn advance_to(
        &mut self,
        target: f64,
        attempt_quota: usize,
        identity: crate::NativeSessionIdentity,
    ) -> CoreResult<crate::NativeSessionAdvance> {
        self.identity.check(identity)?;
        if self.terminal.is_some() || self.failure.is_some() {
            return Ok(crate::NativeSessionAdvance::Stopped);
        }
        if self.t == self.tf {
            return Ok(crate::NativeSessionAdvance::Complete);
        }
        if !target.is_finite()
            || target < self.t
            || target > self.tf
            || (target != self.t
                && self
                    .schedule
                    .times()
                    .binary_search_by(|time| {
                        // Both operands are finite; partial_cmp preserves
                        // the previous +0 == -0 membership semantics.
                        time.partial_cmp(&target)
                            .expect("validated finite schedule and target")
                    })
                    .is_err())
        {
            return Err(CoreError::InvalidInput(
                "native session target must be current or a registered landing time".into(),
            ));
        }
        let call_start = self.attempts;
        while self.t < target && self.attempts < self.adaptive.max_attempts {
            if self.attempts - call_start >= attempt_quota {
                return Ok(crate::NativeSessionAdvance::Yield);
            }
            let next_h = match crate::output::adaptive_end_step(
                self.t,
                self.h,
                self.tf,
                self.adaptive.step_cap(),
                &self.controller,
            ) {
                Ok(Some(h)) => h,
                Ok(None) => {
                    self.terminal = Some(DriverStopReason::MinimumStep);
                    break;
                }
                Err(error) => {
                    self.latch_failure(Rodas5pSessionFailureKind::Clock, error, false);
                    break;
                }
            };
            self.h = next_h;
            if (crate::output::below_min_step(self.t, self.h, self.adaptive.min_step)
                && self.t + self.h < self.tf)
                || self.t + self.h == self.t
            {
                self.terminal = Some(DriverStopReason::MinimumStep);
                break;
            }
            let (trial_h, clipped) = match self.collector.limit_step(self.t, self.h, self.tf) {
                Ok(trial) => trial,
                Err(error) => {
                    self.latch_failure(Rodas5pSessionFailureKind::Clock, error, false);
                    break;
                }
            };
            self.attempts += 1;
            if self.fresh_state {
                self.reuses += 1;
            }
            let outcome = self.work.attempt(
                self.problem,
                self.t,
                &self.y,
                trial_h,
                !self.fresh_state,
                self.adaptive.atol,
                self.adaptive.rtol,
                &mut self.counters,
            );
            self.fresh_state = self.work.lu_chosen;
            let (error, failure) = match outcome {
                Ok(error) if error <= 1.0 => (error, None),
                Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
                Err(error @ CoreError::FatalCallback(_)) => {
                    self.latch_failure(Rodas5pSessionFailureKind::CallbackError, error, false);
                    self.terminal = Some(DriverStopReason::FatalCallback);
                    (f64::INFINITY, Some(AdaptiveFailureKind::NonFinite))
                }
                Err(error) => match failure_kind(&error) {
                    Some(kind) => {
                        // A failed attempt may have left a partial Jacobian or
                        // f(t, y); rebuild them on the next trial.
                        self.fresh_state = false;
                        (f64::INFINITY, Some(kind))
                    }
                    None => {
                        self.latch_failure(Rodas5pSessionFailureKind::CallbackError, error, true);
                        break;
                    }
                },
            };
            match failure {
                None => {
                    // Stage every fallible gate before publishing the accepted
                    // physical state or observation. Controller arithmetic uses
                    // the same inputs/order as the legacy path.
                    let mut next_controller = self.controller.clone();
                    let next_h = match rodas_next_step_after_attempt(
                        &mut next_controller,
                        &self.adaptive,
                        self.h,
                        trial_h,
                        error,
                        true,
                        clipped,
                    ) {
                        Ok(h) => h,
                        Err(error) => {
                            self.latch_failure(Rodas5pSessionFailureKind::Controller, error, true);
                            break;
                        }
                    };
                    let next_t = self.t + trial_h;
                    if let Err(error) = self.collector.accept(next_t, &self.work.y_new, clipped) {
                        self.latch_failure(Rodas5pSessionFailureKind::Output, error, true);
                        break;
                    }
                    self.controller = next_controller;
                    self.counters.accepted_steps += 1;
                    self.accepted_steps += 1;
                    self.t = next_t;
                    self.y.copy_from_slice(&self.work.y_new);
                    self.internal_steps += 1;
                    self.fresh_state = false;
                    self.last_failure = None;
                    self.h = next_h;
                }
                Some(kind) => {
                    self.counters.rejected_steps += 1;
                    self.rejected_steps += 1;
                    self.last_failure = Some(kind);
                    match kind {
                        AdaptiveFailureKind::LocalError => self.counters.local_error_failures += 1,
                        AdaptiveFailureKind::LinearSolve => {
                            self.counters.linear_solve_failures += 1
                        }
                        AdaptiveFailureKind::NonlinearSolve => {
                            self.counters.nonlinear_solve_failures += 1
                        }
                        AdaptiveFailureKind::NonFinite => {
                            self.counters.nonfinite_step_failures += 1
                        }
                    }
                }
            }
            if self.terminal == Some(DriverStopReason::FatalCallback) {
                break;
            }
            if failure.is_some() {
                self.h = match rodas_next_step_after_attempt(
                    &mut self.controller,
                    &self.adaptive,
                    self.h,
                    trial_h,
                    error,
                    false,
                    clipped,
                ) {
                    Ok(h) => h,
                    Err(error) => {
                        self.latch_failure(Rodas5pSessionFailureKind::Controller, error, false);
                        break;
                    }
                };
            }
        }

        if self.failure.is_some() {
            Ok(crate::NativeSessionAdvance::Stopped)
        } else if self.t == self.tf {
            Ok(crate::NativeSessionAdvance::Complete)
        } else {
            if self.terminal.is_none() && self.attempts == self.adaptive.max_attempts {
                self.terminal = Some(DriverStopReason::AttemptBudget);
            }
            if self.terminal.is_some() {
                Ok(crate::NativeSessionAdvance::Stopped)
            } else {
                Ok(crate::NativeSessionAdvance::Checkpoint)
            }
        }
    }
    pub fn finish(mut self) -> CoreResult<Rodas5pFastSessionResult> {
        if self.t >= self.tf && self.failure.is_none() && !self.collector.is_complete() {
            self.latch_failure(
                Rodas5pSessionFailureKind::Output,
                CoreError::InvalidInput(
                    "integration ended before all requested outputs were recorded".into(),
                ),
                false,
            );
        }
        let success = self.t >= self.tf && self.failure.is_none();
        let (times, states, output_clipped_steps) = if success {
            self.collector.finish()?
        } else {
            self.collector.finish_partial()
        };
        let message = if success {
            "success".into()
        } else if let Some(failure) = &self.failure {
            format!("native session {:?}: {}", failure.kind, failure.error)
        } else if self.terminal.is_some() {
            "maximum step count or minimum step reached".into()
        } else {
            "paused at accepted checkpoint".into()
        };
        let termination = if let Some(failure) = self.failure {
            Rodas5pSessionTermination::Failure(failure)
        } else if success {
            Rodas5pSessionTermination::Complete
        } else if let Some(reason) = self.terminal {
            Rodas5pSessionTermination::DriverStop(reason)
        } else {
            Rodas5pSessionTermination::Paused
        };
        Ok(Rodas5pFastSessionResult {
            run: Rodas5pFastResult {
                observed: ObservedIntegrationResult {
                    t: times,
                    y: states,
                    success,
                    message,
                    counters: self.counters,
                    internal_steps: self.internal_steps,
                    output_clipped_steps,
                },
                attempts: self.attempts,
                accepted_steps: self.accepted_steps,
                rejected_steps: self.rejected_steps,
                jacobian_reuses: self.reuses,
                lu: self.work.lu_kind,
                driver: RODAS5P_FAST_DRIVER_ID,
            },
            final_time: self.t,
            final_state: self.y,
            termination,
        })
    }
}

#[cfg(test)]
mod session_terminal_gate_tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    #[test]
    fn injected_clock_output_and_controller_refusals_are_sticky_at_the_accepted_prefix() {
        // Fault injection targets private failure branches without exposing a
        // production API for mutating the frozen configuration or collector.
        for kind in [
            Rodas5pSessionFailureKind::Clock,
            Rodas5pSessionFailureKind::Output,
            Rodas5pSessionFailureKind::Controller,
        ] {
            let calls = Arc::new(AtomicU64::new(0));
            let seen = calls.clone();
            let p = OdeProblem::new(
                "gate refusal",
                1,
                Arc::new(move |_, y, out| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    out[0] = -y[0];
                    Ok(())
                }),
                None,
                Some(Arc::new(|_, _| DenseMatrix::new(1, 1, vec![-1.0]))),
                None,
                None,
                true,
                None,
                None,
            )
            .unwrap()
            .with_jacobian_into(Arc::new(|_, _, j| {
                j[(0, 0)] = -1.0;
                Ok(())
            }));
            let plan = OutputSchedule::new(vec![0.0, 0.001, 0.002, 0.01]).unwrap();
            let cfg = AdaptiveStepConfig {
                initial_step: 0.001,
                max_step: 0.001,
                ..Default::default()
            };
            let identity =
                crate::NativeSessionIdentity::new(crate::NativeSessionMethod::Rodas5pFast, 1);
            let mut s =
                Rodas5pFastSession::new(&p, (0.0, 0.01), &[1.0], &cfg, &plan, identity).unwrap();
            assert_eq!(
                s.advance_to(0.001, usize::MAX, identity).unwrap(),
                crate::NativeSessionAdvance::Checkpoint
            );
            let prefix = (s.t, s.y[0], s.attempts, s.accepted_steps, s.rejected_steps);
            let controller_before = s.controller.clone();
            match kind {
                Rodas5pSessionFailureKind::Clock => {
                    s.adaptive.max_step_policy = crate::MaxStepPolicy::StrictRepresentedCap;
                    s.adaptive.max_step = (s.t.next_up() - s.t) * 0.25;
                }
                Rodas5pSessionFailureKind::Controller => s.adaptive.safety = f64::NAN,
                Rodas5pSessionFailureKind::Output => {
                    let remaining = OutputSchedule::new(vec![s.t, 0.002, 0.01]).unwrap();
                    s.collector =
                        OutputCollector::new(&remaining, (s.t, 0.01), &[1.0, 2.0]).unwrap();
                }
                _ => unreachable!(),
            }
            assert_eq!(
                s.advance_to(0.002, usize::MAX, identity).unwrap(),
                crate::NativeSessionAdvance::Stopped
            );
            assert_eq!(s.t.to_bits(), prefix.0.to_bits());
            assert_eq!(s.y[0].to_bits(), prefix.1.to_bits());
            let entered = usize::from(kind != Rodas5pSessionFailureKind::Clock);
            assert_eq!(s.attempts, prefix.2 + entered);
            assert_eq!(s.accepted_steps, prefix.3);
            assert_eq!(s.rejected_steps, prefix.4 + entered);
            assert!(!s.fresh_state);
            assert_eq!(
                s.controller, controller_before,
                "failed gate changed accepted controller history: {kind:?}"
            );
            assert_eq!(s.failure().unwrap().kind, kind);
            let calls_before = calls.load(Ordering::SeqCst);
            assert_eq!(
                s.advance_to(0.01, usize::MAX, identity).unwrap(),
                crate::NativeSessionAdvance::Stopped
            );
            assert_eq!(calls.load(Ordering::SeqCst), calls_before);
            let result = s.finish().unwrap();
            assert_eq!(result.final_time.to_bits(), prefix.0.to_bits());
            assert_eq!(result.final_state[0].to_bits(), prefix.1.to_bits());
            assert!(!result.run.observed.success);
            assert!(
                matches!(result.termination,Rodas5pSessionTermination::Failure(Rodas5pSessionFailure{kind:k,..})if k==kind)
            );
        }
    }
}
