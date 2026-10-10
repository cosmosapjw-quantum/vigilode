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

use std::sync::Arc;

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, WorkCounters, rodas5p_coefficients,
};
use serde::Serialize;

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ControllerDecision,
    ControllerTelemetry, ObservedIntegrationResult, OdeProblem, OutputSchedule,
    adaptive::rodas_next_step_after_attempt_prevalidated, output::OutputCollector,
    rodas_next_step_after_attempt,
};

/// Identifier of this driver in benchmark and research records.
pub const RODAS5P_FAST_DRIVER_ID: &str = "rodas5p-fast-transformed-v2";

/// Identifier of the banded pipeline (integrated DAG node INT-03).
pub const RODAS5P_FAST_BANDED_DRIVER_ID: &str = "rodas5p-fast-banded-v1";

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
}

/// Counted linear-algebra work and storage of a banded run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BandedWork {
    /// Multiply-subtracts count 2, divisions 1, `W` assembly 1 per band entry.
    pub factor_operations: u64,
    pub solve_operations: u64,
    /// f64 slots of the Jacobian band and the factors.
    pub stored_slots: usize,
}

#[derive(Clone, Debug)]
pub struct Rodas5pFastBandedResult {
    pub fast: Rodas5pFastResult,
    pub work: BandedWork,
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
}

impl BandState {
    fn width(&self) -> usize {
        2 * self.l + self.u + 1
    }

    fn at(&self, i: usize, j: usize) -> usize {
        i * self.width() + (j + self.l - i)
    }

    /// `W = I/(h gamma) - J` on the band, then the banded LU with partial
    /// pivoting over rows `k ..= k + l`.
    fn factor(&mut self, n: usize, inv: f64) -> CoreResult<()> {
        if self.kernel == BandedKernel::Slices {
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
        if self.kernel == BandedKernel::Slices {
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
struct Workspace {
    n: usize,
    s: usize,
    gamma: f64,
    c: Vec<f64>,
    gamma_rows: Vec<f64>,
    lu_kind: Rodas5pFastLu,
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
    a_nonzero: Vec<Vec<(usize, f64)>>,
    c_nonzero: Vec<Vec<(usize, f64)>>,
    b_nonzero: Vec<(usize, f64)>,
    /// `W` for faer, and its factorization.
    w_dense: DenseMatrix,
    faer_lu: Option<LuFactorization>,
    /// The Jacobian at the state of the current attempt, kept across steps
    /// (the in-place Jacobian callback rewrites its own pattern).
    jacobian: DenseMatrix,
    lu_chosen: bool,
    f0: Vec<f64>,
    ft: Vec<f64>,
    u: Vec<f64>,
    stage_state: Vec<f64>,
    stage_rhs: Vec<f64>,
    y_new: Vec<f64>,
    band: Option<BandState>,
}

impl Workspace {
    /// The banded workspace: no `n x n` storage.
    fn new_banded(n: usize, band: &BandedJacobian, kernel: BandedKernel) -> CoreResult<Self> {
        let (l, u) = (band.lower, band.upper);
        let mut work = Self::with_dense_size(n, 0)?;
        let state = BandState {
            l,
            u,
            fill: band.fill.clone(),
            jacobian: vec![0.0; n * (l + u + 1)],
            factors: vec![0.0; n * (2 * l + u + 1)],
            pivots: vec![0; n],
            work: BandedWork {
                stored_slots: n * (3 * l + 2 * u + 2),
                ..BandedWork::default()
            },
            kernel,
        };
        work.band = Some(state);
        work.lu_kind = Rodas5pFastLu::Banded;
        work.lu_chosen = true;
        Ok(work)
    }

    fn new(n: usize, lu_policy: FastLuPolicy) -> CoreResult<Self> {
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
            jacobian: DenseMatrix::zeros(dense, dense),
            lu_chosen: false,
            f0: vec![0.0; n],
            ft: vec![0.0; n],
            u: vec![0.0; s * n],
            stage_state: vec![0.0; n],
            stage_rhs: vec![0.0; n],
            y_new: vec![0.0; n],
            band: None,
        })
    }

    fn choose_lu(&mut self) {
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
                Rodas5pFastLu::Faer
            };
    }

    /// Form `W = I/(h gamma) - J` and factor it.
    fn factor(&mut self, h: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let n = self.n;
        let inv = 1.0 / (h * self.gamma);
        if let Some(band) = self.band.as_mut() {
            counters.direct_factorizations += 1;
            return band.factor(n, inv);
        }
        let jacobian = &self.jacobian;
        let target = match self.lu_kind {
            Rodas5pFastLu::InPlaceZeroSkipping | Rodas5pFastLu::InPlaceColumnExtents => {
                self.w.as_mut_slice()
            }
            Rodas5pFastLu::Faer => self.w_dense.as_mut_slice(),
            Rodas5pFastLu::Banded => unreachable!("banded factors are formed above"),
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
            Rodas5pFastLu::Banded => unreachable!("banded factors are formed above"),
        }
    }

    /// Solve `W x = stage_rhs` in place.
    fn solve(&mut self, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.linear_solves += 1;
        counters.direct_solve_calls += 1;
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
                let x = self
                    .faer_lu
                    .as_ref()
                    .expect("factored")
                    .solve(&self.stage_rhs)?;
                self.stage_rhs.copy_from_slice(&x);
            }
            Rodas5pFastLu::Banded => {
                let n = self.n;
                self.band
                    .as_mut()
                    .expect("banded workspace")
                    .solve(n, &mut self.stage_rhs);
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
            } else {
                problem.dense_jacobian_into(t, y, &mut self.jacobian, counters)?;
            }
            if !self.lu_chosen {
                self.choose_lu();
                self.lu_chosen = true;
            }
            problem.eval_rhs_into(t, y, &mut self.f0, counters)?;
            if !problem.autonomous {
                let ft = problem.eval_partial_t(t, y, counters)?;
                self.ft.copy_from_slice(&ft);
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
        for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
            let z = e / (atol + rtol * a.abs().max(b.abs()));
            sum += z * z;
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
    if n == 0
        || lower >= n
        || upper >= n
        || jacobian.len() != n * (lower + upper + 1)
        || b.len() != n
    {
        return Err(CoreError::InvalidInput("invalid banded solve input".into()));
    }
    let mut state = BandState {
        l: lower,
        u: upper,
        fill: Arc::new(|_, _, _| Ok(())),
        jacobian: jacobian.to_vec(),
        factors: vec![0.0; n * (2 * lower + upper + 1)],
        pivots: vec![0; n],
        work: BandedWork::default(),
        kernel: BandedKernel::Indexed,
    };
    state.factor(n, inv)?;
    let mut x = b.to_vec();
    state.solve(n, &mut x);
    Ok((x, state.pivots))
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

fn failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
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
    )
    .map(|(result, _)| result)
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
    )
    .map(|(result, _)| (result, telemetry))
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
    integrate_fast(problem, None, t_span, y0, adaptive, output, options, None)
        .map(|(result, _)| result)
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
    )
}

#[allow(clippy::too_many_arguments)]
fn integrate_banded(
    problem: &OdeProblem,
    band: &BandedJacobian,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
    kernel: BandedKernel,
) -> CoreResult<Rodas5pFastBandedResult> {
    let (fast, work) = integrate_fast(
        problem,
        Some((band, kernel)),
        t_span,
        y0,
        adaptive,
        output,
        options,
        None,
    )?;
    Ok(Rodas5pFastBandedResult {
        fast,
        work: work.unwrap_or_default(),
    })
}

#[allow(clippy::too_many_arguments)]
fn integrate_fast(
    problem: &OdeProblem,
    band: Option<(&BandedJacobian, BandedKernel)>,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
    mut telemetry: Option<&mut ControllerTelemetry>,
) -> CoreResult<(Rodas5pFastResult, Option<BandedWork>)> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
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
    let mut work = match band {
        Some((band, kernel)) => Workspace::new_banded(problem.dimension, band, kernel)?,
        None => Workspace::new(problem.dimension, options.lu_policy)?,
    };
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
        // Every path below has the state's Jacobian and f(t, y) in place,
        // unless they failed to build.
        fresh_state = work.lu_chosen;
        let (error, failure) = match outcome {
            Ok(error) if error <= 1.0 => (error, None),
            Ok(error) => (error, Some(AdaptiveFailureKind::LocalError)),
            Err(error) => match failure_kind(&error) {
                Some(kind) => {
                    fresh_state = false;
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
                fresh_state = false;
            }
            Some(kind) => {
                counters.rejected_steps += 1;
                rejected_steps += 1;
                match kind {
                    AdaptiveFailureKind::LocalError => counters.local_error_failures += 1,
                    AdaptiveFailureKind::LinearSolve => counters.linear_solve_failures += 1,
                    AdaptiveFailureKind::NonlinearSolve => counters.nonlinear_solve_failures += 1,
                    AdaptiveFailureKind::NonFinite => counters.nonfinite_step_failures += 1,
                }
            }
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
    }
    let success = t >= tf;
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
        driver: match band {
            Some((_, BandedKernel::Slices)) => RODAS5P_FAST_BANDED_SLICES_DRIVER_ID,
            _ => options.driver_id(banded.is_some()),
        },
    };
    Ok((result, banded))
}

#[cfg(test)]
mod tests {
    use super::*;

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
