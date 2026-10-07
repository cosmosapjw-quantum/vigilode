//! The fast RODAS5P driver specialized to a compile-time dimension with
//! statically dispatched problem functions (research node
//! `research/thread_transfer_smalln_cost_20261002`, thread-transfer DAG node
//! P1-SMALLN-COST). Research only.
//!
//! The arithmetic is that of [`crate::integrate_rodas5p_fast_observed`]
//! (v2): the transformed stages, the same coefficient order, the same
//! partial-pivoting LU with tracked row extents and zero-multiplier skipping,
//! the same reuse of `J`, `f(t, y)` and `f_t` after a rejection, the same
//! finite checks, counters, controller, represented-clock and output rules.
//! Only the storage (`[f64; N]`, `[[f64; N]; N]` on the stack) and the
//! dispatch of the problem (a [`SmallProblem`] type parameter instead of
//! `Arc<dyn Fn>` callbacks) differ, so its results equal v2's as values.
//!
//! Opt-in, through [`Rodas5pFastSmallOptions`] (speed research node SPD06,
//! `research/spd06_small_static_stages_20261007`): a fixed stage structure on
//! `[[f64; 8]; 8]` coefficient tables and index-loop triangular solves, both
//! with the same operations in the same order (bitwise the same results).

use rodas5p_core::{CoreError, CoreResult, WorkCounters, rodas5p_coefficients};

use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ObservedIntegrationResult,
    OutputSchedule, adaptive::rodas_next_step_after_attempt_prevalidated, output::OutputCollector,
    rodas_next_step_after_attempt, rodas5p_fast::Rodas5pFastOptions,
};

/// Identifier of this driver in research records.
pub const RODAS5P_FAST_SMALL_DRIVER_ID: &str = "rodas5p-fast-small-static-v1";

pub(crate) const STAGES: usize = 8;

/// An autonomous ODE (time is not passed) of fixed dimension `N` with an in-place Jacobian that
/// writes a fixed sparsity pattern (as `OdeProblem::with_jacobian_into`).
pub trait SmallProblem<const N: usize> {
    fn rhs(&self, y: &[f64; N], out: &mut [f64; N]);
    fn jacobian(&self, y: &[f64; N], out: &mut [[f64; N]; N]);
}

#[derive(Clone, Debug)]
pub struct Rodas5pFastSmallResult {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub jacobian_reuses: usize,
    pub driver: &'static str,
}

pub(crate) struct Workspace<const N: usize> {
    pub(crate) gamma: f64,
    pub(crate) a_nonzero: Vec<Vec<(usize, f64)>>,
    pub(crate) c_nonzero: Vec<Vec<(usize, f64)>>,
    pub(crate) b_nonzero: Vec<(usize, f64)>,
    w: [[f64; N]; N],
    pivots: [usize; N],
    row_end: [usize; N],
    l_start: [usize; N],
    jacobian: [[f64; N]; N],
    f0: [f64; N],
    u: [[f64; N]; STAGES],
    stage_state: [f64; N],
    stage_rhs: [f64; N],
    y_new: [f64; N],
    built: bool,
}

fn rhs_counted<const N: usize, P: SmallProblem<N>>(
    problem: &P,
    y: &[f64; N],
    out: &mut [f64; N],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    problem.rhs(y, out);
    if !out.iter().all(|v| v.is_finite()) {
        return Err(CoreError::NonFinite("RHS produced NaN/Inf".into()));
    }
    counters.rhs_calls += 1;
    counters.rhs_evaluations += 1;
    Ok(())
}

impl<const N: usize> Workspace<N> {
    pub(crate) fn new() -> CoreResult<Self> {
        Self::with_lists(true)
    }

    /// `lists == false` leaves the heap coefficient lists empty: the fixed
    /// stage structure of SPD06 reads [`StaticTables`] instead, so it makes
    /// none of their allocations.
    fn with_lists(lists: bool) -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        if coeffs.stages() != STAGES {
            return Err(CoreError::Coefficients("RODAS5P has eight stages".into()));
        }
        for i in 0..STAGES {
            for j in i..STAGES {
                if coeffs.a[(i, j)] != 0.0 || coeffs.c_matrix[(i, j)] != 0.0 {
                    return Err(CoreError::Coefficients(
                        "RODAS5P transformed coefficients are not strictly lower triangular".into(),
                    ));
                }
            }
        }
        let nonzero_row = |m: &rodas5p_core::DenseMatrix, i: usize| {
            (0..i)
                .filter(|&j| m[(i, j)] != 0.0)
                .map(|j| (j, m[(i, j)]))
                .collect::<Vec<_>>()
        };
        let (a_nonzero, c_nonzero, b_nonzero) = if lists {
            (
                (0..STAGES).map(|i| nonzero_row(&coeffs.a, i)).collect(),
                (0..STAGES)
                    .map(|i| nonzero_row(&coeffs.c_matrix, i))
                    .collect(),
                coeffs
                    .b_code
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| **b != 0.0)
                    .map(|(j, b)| (j, *b))
                    .collect(),
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        Ok(Self {
            gamma: coeffs.gamma,
            a_nonzero,
            c_nonzero,
            b_nonzero,
            w: [[0.0; N]; N],
            pivots: [0; N],
            row_end: [0; N],
            l_start: [N; N],
            jacobian: [[0.0; N]; N],
            f0: [0.0; N],
            u: [[0.0; N]; STAGES],
            stage_state: [0.0; N],
            stage_rhs: [0.0; N],
            y_new: [0.0; N],
            built: false,
        })
    }

    /// `W = I/(h gamma) - J` and its in-place factors (v2's operation order).
    fn factor(&mut self, h: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        let inv = 1.0 / (h * self.gamma);
        for (w_row, j_row) in self.w.iter_mut().zip(&self.jacobian) {
            for (w, j) in w_row.iter_mut().zip(j_row) {
                *w = -j;
            }
        }
        for i in 0..N {
            self.w[i][i] += inv;
        }
        counters.direct_factorizations += 1;
        for i in 0..N {
            let last = self.w[i].iter().rposition(|v| *v != 0.0).unwrap_or(0);
            self.row_end[i] = last.max(i);
            self.l_start[i] = N;
        }
        lu_in_place(
            &mut self.w,
            &mut self.pivots,
            &mut self.row_end,
            &mut self.l_start,
        )
    }

    /// `INDEX_SOLVES` selects the index-loop triangular solves of SPD06
    /// (the same operations in the same order).
    fn solve<const INDEX_SOLVES: bool>(&mut self, counters: &mut WorkCounters) -> CoreResult<()> {
        counters.linear_solves += 1;
        counters.direct_solve_calls += 1;
        if INDEX_SOLVES {
            lu_solve_in_place_indexed(
                &self.w,
                &self.pivots,
                &self.row_end,
                &self.l_start,
                &mut self.stage_rhs,
            );
        } else {
            lu_solve_in_place(
                &self.w,
                &self.pivots,
                &self.row_end,
                &self.l_start,
                &mut self.stage_rhs,
            );
        }
        if self.stage_rhs.iter().all(|v| v.is_finite()) {
            Ok(())
        } else {
            Err(CoreError::LinearSolve(
                "RODAS5P fast stage solve produced NaN/Inf".into(),
            ))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn attempt<P: SmallProblem<N>, const INDEX_SOLVES: bool>(
        &mut self,
        problem: &P,
        y: &[f64; N],
        h: f64,
        fresh: bool,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        if fresh {
            counters.jacobian_builds += 1;
            problem.jacobian(y, &mut self.jacobian);
            self.built = true;
            rhs_counted(problem, y, &mut self.f0, counters)?;
        }
        self.factor(h, counters)?;
        for i in 0..STAGES {
            if i == 0 {
                self.stage_rhs = self.f0;
            } else {
                self.stage_state = *y;
                for &(j, aij) in &self.a_nonzero[i] {
                    for (x, v) in self.stage_state.iter_mut().zip(&self.u[j]) {
                        *x += aij * v;
                    }
                }
                let state = self.stage_state;
                rhs_counted(problem, &state, &mut self.stage_rhs, counters)?;
            }
            for &(j, c) in &self.c_nonzero[i] {
                let cij = c / h;
                if cij != 0.0 {
                    for (x, v) in self.stage_rhs.iter_mut().zip(&self.u[j]) {
                        *x += cij * v;
                    }
                }
            }
            self.solve::<INDEX_SOLVES>(counters)?;
            self.u[i] = self.stage_rhs;
        }
        self.y_new = *y;
        for &(j, bj) in &self.b_nonzero {
            for (x, v) in self.y_new.iter_mut().zip(&self.u[j]) {
                *x += bj * v;
            }
        }
        if !self.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P fast step produced NaN/Inf".into(),
            ));
        }
        let error = &self.u[STAGES - 1];
        let mut sum = 0.0;
        for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
            let z = e / (atol + rtol * a.abs().max(b.abs()));
            sum += z * z;
        }
        let norm = (sum / N as f64).sqrt();
        Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        })
    }

    /// Stage `I` of [`Self::attempt`] on the fixed tables (speed research
    /// node SPD06, `research/spd06_small_static_stages_20261007`). Both
    /// loops have constant trip counts. Each component is accumulated in a
    /// register in the order of the legacy axpy sequence (`j` ascending, the
    /// coefficient times the stage value, one rounding per product and per
    /// sum), so every value is bitwise the legacy one: the tables hold
    /// exactly the entries of the legacy lists (checked at construction,
    /// [`StaticTables::new`]). The `c / h` quotients are computed once per
    /// stage, and their `!= 0.0` guard is kept.
    // Index loops with constant trip counts are the point of SPD06.
    #[allow(clippy::needless_range_loop)]
    #[inline(always)]
    fn static_stage<P: SmallProblem<N>, const I: usize, const INDEX_SOLVES: bool>(
        &mut self,
        problem: &P,
        tables: &StaticTables,
        y: &[f64; N],
        h: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        if I == 0 {
            self.stage_rhs = self.f0;
        } else {
            let a = &tables.a[I];
            let mut state = [0.0; N];
            for (k, x) in state.iter_mut().enumerate() {
                let mut sum = y[k];
                for j in 0..I {
                    sum += a[j] * self.u[j][k];
                }
                *x = sum;
            }
            rhs_counted(problem, &state, &mut self.stage_rhs, counters)?;
            let mut quotients = [0.0; STAGES];
            for j in 0..I {
                quotients[j] = tables.c[I][j] / h;
            }
            for k in 0..N {
                let mut sum = self.stage_rhs[k];
                for j in 0..I {
                    let cij = quotients[j];
                    if cij != 0.0 {
                        sum += cij * self.u[j][k];
                    }
                }
                self.stage_rhs[k] = sum;
            }
        }
        self.solve::<INDEX_SOLVES>(counters)?;
        self.u[I] = self.stage_rhs;
        Ok(())
    }

    /// [`Self::attempt`] with the fixed stage structure of SPD06: the same
    /// operations in the same order, one stage function per stage index.
    #[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
    fn attempt_static<P: SmallProblem<N>, const INDEX_SOLVES: bool>(
        &mut self,
        problem: &P,
        tables: &StaticTables,
        y: &[f64; N],
        h: f64,
        fresh: bool,
        atol: f64,
        rtol: f64,
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        if fresh {
            counters.jacobian_builds += 1;
            problem.jacobian(y, &mut self.jacobian);
            self.built = true;
            rhs_counted(problem, y, &mut self.f0, counters)?;
        }
        self.factor(h, counters)?;
        self.static_stage::<P, 0, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        self.static_stage::<P, 1, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        self.static_stage::<P, 2, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        self.static_stage::<P, 3, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        self.static_stage::<P, 4, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        self.static_stage::<P, 5, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        self.static_stage::<P, 6, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        self.static_stage::<P, 7, INDEX_SOLVES>(problem, tables, y, h, counters)?;
        let b = &tables.b;
        for k in 0..N {
            let mut sum = y[k];
            for j in 0..STAGES {
                sum += b[j] * self.u[j][k];
            }
            self.y_new[k] = sum;
        }
        if !self.y_new.iter().all(|v| v.is_finite()) {
            return Err(CoreError::NonFinite(
                "RODAS5P fast step produced NaN/Inf".into(),
            ));
        }
        let error = &self.u[STAGES - 1];
        let mut sum = 0.0;
        for ((e, a), b) in error.iter().zip(y).zip(&self.y_new) {
            let z = e / (atol + rtol * a.abs().max(b.abs()));
            sum += z * z;
        }
        let norm = (sum / N as f64).sqrt();
        Ok(if norm.is_finite() {
            norm
        } else {
            f64::INFINITY
        })
    }
}

/// The transformed RODAS5P coefficients as fixed tables (speed research
/// node SPD06): `a[i][j]` and `c[i][j]` for `j < i` (zero elsewhere) and
/// `b[j] = b_code[j]`, filled from [`rodas5p_coefficients`] (no literals).
#[derive(Clone, Copy, Debug, PartialEq)]
struct StaticTables {
    a: [[f64; STAGES]; STAGES],
    c: [[f64; STAGES]; STAGES],
    b: [f64; STAGES],
}

impl StaticTables {
    /// Filled from the coefficients and checked by [`Self::check`].
    fn new() -> CoreResult<Self> {
        let coeffs = rodas5p_coefficients()?;
        let tables = Self::fill(coeffs)?;
        tables.check(coeffs)?;
        Ok(tables)
    }

    fn fill(coeffs: &rodas5p_core::Rodas5pCoefficients) -> CoreResult<Self> {
        if coeffs.stages() != STAGES || coeffs.b_code.len() != STAGES {
            return Err(CoreError::Coefficients("RODAS5P has eight stages".into()));
        }
        let mut tables = Self {
            a: [[0.0; STAGES]; STAGES],
            c: [[0.0; STAGES]; STAGES],
            b: [0.0; STAGES],
        };
        for i in 0..STAGES {
            for j in 0..i {
                tables.a[i][j] = coeffs.a[(i, j)];
                tables.c[i][j] = coeffs.c_matrix[(i, j)];
            }
            tables.b[i] = coeffs.b_code[i];
        }
        Ok(tables)
    }

    /// The construction-time check: every table entry equals its
    /// coefficient bit for bit, and the legacy lists hold exactly these
    /// entries, i.e. `A` and `C` are zero on and above the diagonal and
    /// nonzero below it and `b_code` has no zero. Under these conditions the
    /// fixed loops visit the same `(j, value)` sequence as the legacy lists;
    /// otherwise the fixed structure would add operations, so it is refused.
    fn check(&self, coeffs: &rodas5p_core::Rodas5pCoefficients) -> CoreResult<()> {
        let mismatch = |what: &str, i: usize, j: usize| {
            Err(CoreError::Coefficients(format!(
                "SPD06 fixed table {what}[{i}][{j}] does not equal the RODAS5P coefficient bit for bit"
            )))
        };
        if coeffs.stages() != STAGES || coeffs.b_code.len() != STAGES {
            return Err(CoreError::Coefficients("RODAS5P has eight stages".into()));
        }
        for i in 0..STAGES {
            for j in 0..STAGES {
                let (a, c) = (coeffs.a[(i, j)], coeffs.c_matrix[(i, j)]);
                if j < i {
                    if self.a[i][j].to_bits() != a.to_bits() {
                        return mismatch("a", i, j);
                    }
                    if self.c[i][j].to_bits() != c.to_bits() {
                        return mismatch("c", i, j);
                    }
                    if a == 0.0 || c == 0.0 {
                        return Err(CoreError::Coefficients(format!(
                            "SPD06 fixed stages need a full strictly lower A and C; entry ({i}, {j}) is zero"
                        )));
                    }
                } else if a != 0.0
                    || c != 0.0
                    || self.a[i][j].to_bits() != 0
                    || self.c[i][j].to_bits() != 0
                {
                    return Err(CoreError::Coefficients(
                        "RODAS5P transformed coefficients are not strictly lower triangular".into(),
                    ));
                }
            }
            if self.b[i].to_bits() != coeffs.b_code[i].to_bits() {
                return mismatch("b", 0, i);
            }
            if coeffs.b_code[i] == 0.0 {
                return Err(CoreError::Coefficients(format!(
                    "SPD06 fixed stages need a full b_code; entry {i} is zero"
                )));
            }
        }
        Ok(())
    }
}

/// The fixed tables of the SPD06 static stage structure (`a`, `c`, `b`), as
/// the small driver builds them, after its construction-time check. For
/// contract tests (`research/spd06_small_static_stages_20261007`).
#[allow(clippy::type_complexity)]
pub fn rodas5p_fast_small_static_tables() -> CoreResult<(
    [[f64; STAGES]; STAGES],
    [[f64; STAGES]; STAGES],
    [f64; STAGES],
)> {
    let tables = StaticTables::new()?;
    Ok((tables.a, tables.c, tables.b))
}

/// v2's `lu_in_place` on `[[f64; N]; N]`, the same operations in the same
/// order.
pub(crate) fn lu_in_place<const N: usize>(
    a: &mut [[f64; N]; N],
    pivots: &mut [usize; N],
    row_end: &mut [usize; N],
    l_start: &mut [usize; N],
) -> CoreResult<()> {
    for k in 0..N {
        let mut p = k;
        let mut max = a[k][k].abs();
        for (i, row) in a.iter().enumerate().skip(k + 1) {
            let v = row[k].abs();
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
            let (upper, lower) = a.split_at_mut(p);
            upper[k][..span].swap_with_slice(&mut lower[0][..span]);
            row_end.swap(k, p);
            l_start.swap(k, p);
        }
        let pivot_end = row_end[k];
        let (top, bottom) = a.split_at_mut(k + 1);
        let pivot_row = &top[k];
        let pivot = pivot_row[k];
        for (offset, row) in bottom.iter_mut().enumerate() {
            if row[k] == 0.0 {
                continue;
            }
            let i = k + 1 + offset;
            let l = row[k] / pivot;
            row[k] = l;
            l_start[i] = l_start[i].min(k);
            for (x, r) in row[k + 1..=pivot_end]
                .iter_mut()
                .zip(&pivot_row[k + 1..=pivot_end])
            {
                *x -= l * r;
            }
            row_end[i] = row_end[i].max(pivot_end);
        }
    }
    Ok(())
}

/// v2's `lu_solve_in_place` on fixed arrays.
pub(crate) fn lu_solve_in_place<const N: usize>(
    lu: &[[f64; N]; N],
    pivots: &[usize; N],
    row_end: &[usize; N],
    l_start: &[usize; N],
    b: &mut [f64; N],
) {
    for (k, &p) in pivots.iter().enumerate() {
        b.swap(k, p);
    }
    for i in 0..N {
        let start = l_start[i].min(i);
        let mut sum = b[i];
        for (l, x) in lu[i][start..i].iter().zip(&b[start..i]) {
            sum -= l * x;
        }
        b[i] = sum;
    }
    for i in (0..N).rev() {
        let end = row_end[i];
        let row = &lu[i][i..=end];
        let mut sum = b[i];
        for (u, x) in row[1..].iter().zip(&b[i + 1..=end]) {
            sum -= u * x;
        }
        b[i] = sum / row[0];
    }
}

/// [`lu_solve_in_place`] with index loops instead of slice and zip
/// constructions (speed research node SPD06, `static_solves`): the same
/// operations in the same order (`sum -= lu[i][j] * b[j]` with `j`
/// ascending, then the division by the diagonal).
#[allow(clippy::needless_range_loop)]
fn lu_solve_in_place_indexed<const N: usize>(
    lu: &[[f64; N]; N],
    pivots: &[usize; N],
    row_end: &[usize; N],
    l_start: &[usize; N],
    b: &mut [f64; N],
) {
    for k in 0..N {
        b.swap(k, pivots[k]);
    }
    for i in 0..N {
        let start = l_start[i].min(i);
        let mut sum = b[i];
        for j in start..i {
            sum -= lu[i][j] * b[j];
        }
        b[i] = sum;
    }
    for i in (0..N).rev() {
        let end = row_end[i];
        let mut sum = b[i];
        for j in i + 1..=end {
            sum -= lu[i][j] * b[j];
        }
        b[i] = sum / lu[i][i];
    }
}

pub(crate) fn failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        _ => None,
    }
}

/// [`crate::integrate_rodas5p_fast_observed`] for a [`SmallProblem`] of
/// dimension `N`.
pub fn integrate_rodas5p_fast_small_observed<const N: usize, P: SmallProblem<N>>(
    problem: &P,
    t_span: (f64, f64),
    y0: &[f64; N],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<Rodas5pFastSmallResult> {
    integrate_rodas5p_fast_small_observed_with_options(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastOptions::default(),
    )
}

/// [`integrate_rodas5p_fast_small_observed`] with opt-in
/// [`Rodas5pFastOptions`] (speed research node SPD01); the default options
/// give the same driver. `lu_policy` is ignored (the small driver has one LU).
pub fn integrate_rodas5p_fast_small_observed_with_options<const N: usize, P: SmallProblem<N>>(
    problem: &P,
    t_span: (f64, f64),
    y0: &[f64; N],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastOptions,
) -> CoreResult<Rodas5pFastSmallResult> {
    integrate_small::<N, P, false, false>(
        problem,
        t_span,
        y0,
        adaptive,
        output,
        Rodas5pFastSmallOptions {
            fast: options,
            ..Rodas5pFastSmallOptions::default()
        },
    )
}

/// Opt-in variants of the small driver only (speed research node SPD06,
/// `research/spd06_small_static_stages_20261007`). The default is the
/// L-0041 driver with `fast`'s SPD01 options, bit for bit.
///
/// These switches live here and not in [`Rodas5pFastOptions`] because that
/// struct is built with exhaustive literals by recorded test inputs of SPD01;
/// a new field there would not compile them. The dense and banded drivers
/// take [`Rodas5pFastOptions`] only, so they cannot be given these switches
/// (nothing is silently ignored).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rodas5pFastSmallOptions {
    /// SPD01's options (and an ignored `lu_policy`).
    pub fast: Rodas5pFastOptions,
    /// Fixed `[[f64; 8]; 8]` coefficient tables and one stage function per
    /// stage index with constant trip counts, instead of the heap lists.
    pub static_stages: bool,
    /// Index-loop triangular solves instead of slice and zip constructions.
    pub static_solves: bool,
}

impl Rodas5pFastSmallOptions {
    /// The driver identifier under these options: the L-0041 id, then
    /// `-stages` / `-solves` / `-stages-solves` for SPD06, then SPD01's
    /// `-val` / `-land` / `-ovh`.
    pub fn driver_id(&self) -> &'static str {
        const IDS: [[&str; 4]; 4] = [
            [
                RODAS5P_FAST_SMALL_DRIVER_ID,
                "rodas5p-fast-small-static-v1-val",
                "rodas5p-fast-small-static-v1-land",
                "rodas5p-fast-small-static-v1-ovh",
            ],
            [
                "rodas5p-fast-small-static-v1-stages",
                "rodas5p-fast-small-static-v1-stages-val",
                "rodas5p-fast-small-static-v1-stages-land",
                "rodas5p-fast-small-static-v1-stages-ovh",
            ],
            [
                "rodas5p-fast-small-static-v1-solves",
                "rodas5p-fast-small-static-v1-solves-val",
                "rodas5p-fast-small-static-v1-solves-land",
                "rodas5p-fast-small-static-v1-solves-ovh",
            ],
            [
                "rodas5p-fast-small-static-v1-stages-solves",
                "rodas5p-fast-small-static-v1-stages-solves-val",
                "rodas5p-fast-small-static-v1-stages-solves-land",
                "rodas5p-fast-small-static-v1-stages-solves-ovh",
            ],
        ];
        let structure = usize::from(self.static_stages) + 2 * usize::from(self.static_solves);
        let overhead = usize::from(self.fast.prevalidated_controller)
            + 2 * usize::from(self.fast.fused_landing);
        IDS[structure][overhead]
    }
}

/// [`integrate_rodas5p_fast_small_observed_with_options`] with the SPD06
/// switches of [`Rodas5pFastSmallOptions`]; the default options give the
/// L-0041 driver. Each combination is a separate monomorphization, so no
/// variant pays a per-attempt branch on the switches.
pub fn integrate_rodas5p_fast_small_observed_with_small_options<
    const N: usize,
    P: SmallProblem<N>,
>(
    problem: &P,
    t_span: (f64, f64),
    y0: &[f64; N],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    options: Rodas5pFastSmallOptions,
) -> CoreResult<Rodas5pFastSmallResult> {
    match (options.static_stages, options.static_solves) {
        (false, false) => {
            integrate_small::<N, P, false, false>(problem, t_span, y0, adaptive, output, options)
        }
        (true, false) => {
            integrate_small::<N, P, true, false>(problem, t_span, y0, adaptive, output, options)
        }
        (false, true) => {
            integrate_small::<N, P, false, true>(problem, t_span, y0, adaptive, output, options)
        }
        (true, true) => {
            integrate_small::<N, P, true, true>(problem, t_span, y0, adaptive, output, options)
        }
    }
}

fn integrate_small<
    const N: usize,
    P: SmallProblem<N>,
    const STATIC_STAGES: bool,
    const INDEX_SOLVES: bool,
>(
    problem: &P,
    t_span: (f64, f64),
    y0: &[f64; N],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
    small_options: Rodas5pFastSmallOptions,
) -> CoreResult<Rodas5pFastSmallResult> {
    let options = small_options.fast;
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if tf < t || !y0.iter().all(|v| v.is_finite()) {
        return Err(CoreError::InvalidInput(
            "invalid RODAS5P fast integration input".into(),
        ));
    }
    let mut work = if STATIC_STAGES {
        Workspace::<N>::with_lists(false)?
    } else {
        Workspace::<N>::new()?
    };
    // Unused (and removed by the optimizer) unless `STATIC_STAGES`.
    let tables = if STATIC_STAGES {
        StaticTables::new()?
    } else {
        StaticTables {
            a: [[0.0; STAGES]; STAGES],
            c: [[0.0; STAGES]; STAGES],
            b: [0.0; STAGES],
        }
    };
    let mut y = *y0;
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let (mut attempts, mut accepted_steps, mut rejected_steps, mut reuses) = (0, 0, 0, 0);
    let mut internal_steps = 0_usize;
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
        let outcome = if STATIC_STAGES {
            work.attempt_static::<P, INDEX_SOLVES>(
                problem,
                &tables,
                &y,
                trial_h,
                !fresh_state,
                adaptive.atol,
                adaptive.rtol,
                &mut counters,
            )
        } else {
            work.attempt::<P, INDEX_SOLVES>(
                problem,
                &y,
                trial_h,
                !fresh_state,
                adaptive.atol,
                adaptive.rtol,
                &mut counters,
            )
        };
        fresh_state = work.built;
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
                y = work.y_new;
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
    }
    let success = t >= tf;
    let (times, states, output_clipped_steps) = if success {
        collector.finish()?
    } else {
        collector.finish_partial()
    };
    Ok(Rodas5pFastSmallResult {
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
        driver: small_options.driver_id(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_construction_check_refuses_a_table_off_by_one_bit() {
        let coeffs = rodas5p_coefficients().unwrap();
        let tables = StaticTables::new().unwrap();
        assert_eq!(tables, StaticTables::fill(coeffs).unwrap());
        let mut a = tables;
        a.a[5][2] = f64::from_bits(a.a[5][2].to_bits() + 1);
        assert!(matches!(a.check(coeffs), Err(CoreError::Coefficients(_))));
        let mut c = tables;
        c.c[7][6] = f64::from_bits(c.c[7][6].to_bits() ^ 1);
        assert!(matches!(c.check(coeffs), Err(CoreError::Coefficients(_))));
        let mut b = tables;
        b.b[0] = -b.b[0];
        assert!(matches!(b.check(coeffs), Err(CoreError::Coefficients(_))));
        let mut upper = tables;
        upper.a[2][3] = -0.0;
        assert!(matches!(
            upper.check(coeffs),
            Err(CoreError::Coefficients(_))
        ));
    }

    #[test]
    fn the_indexed_solve_equals_the_slice_solve_bitwise() {
        // A pivoted 4 x 4 system through the small LU, solved both ways.
        let mut w = [
            [0.0, 2.0, 1.0, 0.0],
            [3.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 4.0, 1.0],
            [0.0, 0.0, 1.0, 5.0],
        ];
        let mut row_end = [0; 4];
        for (i, row) in w.iter().enumerate() {
            row_end[i] = row.iter().rposition(|v| *v != 0.0).unwrap_or(0).max(i);
        }
        let (mut pivots, mut l_start) = ([0; 4], [4; 4]);
        lu_in_place(&mut w, &mut pivots, &mut row_end, &mut l_start).unwrap();
        for seed in 0..64_u64 {
            let rhs: [f64; 4] =
                std::array::from_fn(|i| ((seed * 7 + i as u64 * 13) as f64).sin() * 1.0e3);
            let (mut x, mut z) = (rhs, rhs);
            lu_solve_in_place(&w, &pivots, &row_end, &l_start, &mut x);
            lu_solve_in_place_indexed(&w, &pivots, &row_end, &l_start, &mut z);
            assert_eq!(x.map(f64::to_bits), z.map(f64::to_bits));
        }
    }

    #[test]
    fn every_small_option_set_has_its_own_driver_id() {
        let mut ids = std::collections::BTreeSet::new();
        for bits in 0..16_u8 {
            let options = Rodas5pFastSmallOptions {
                fast: Rodas5pFastOptions {
                    prevalidated_controller: bits & 1 != 0,
                    fused_landing: bits & 2 != 0,
                    ..Rodas5pFastOptions::default()
                },
                static_stages: bits & 4 != 0,
                static_solves: bits & 8 != 0,
            };
            assert!(ids.insert(options.driver_id()));
        }
        assert_eq!(
            Rodas5pFastSmallOptions::default().driver_id(),
            RODAS5P_FAST_SMALL_DRIVER_ID
        );
    }
}
