//! The opt-in threaded factorization of [`super::DenseLuReuseStageClient`].
//!
//! faer's recursion (`lu_in_place_recursion` in
//! `linalg/lu/partial_pivoting/factor.rs` of faer 0.24.4) on a pool the
//! client owns. For the columns `[0, w)` of a panel with `m >= w` rows:
//!
//! * `w <= `[`DENSE_LU_REUSE_SEQUENTIAL_PANEL_COLUMNS`]: faer's sequential
//!   `lu_in_place` on the panel, the call of the sequential path;
//! * otherwise, with `w1` faer's block size (half of `w`, rounded up to a
//!   multiple of 16): factor the left `w1` columns, apply their row swaps to
//!   the right columns, solve `L11 X = A12`, subtract `A21 X` from `A22`,
//!   factor `A22`, apply its row swaps to `A21`.
//!
//! The triangular solve, the product and the swaps are split into at most
//! `workers` parts with [`rayon::join`]; every part calls faer's sequential
//! kernel on a disjoint block. A part is at least
//! [`DENSE_LU_REUSE_CHUNK_FLOPS`] floating-point operations (solve and
//! product) or [`DENSE_LU_REUSE_CHUNK_SWAPS`] element swaps, so small levels
//! stay on one thread. The partition depends on the sizes and on `workers`
//! only: the factors are the same on every run with the same worker count.

use faer::dyn_stack::{MemBuffer, MemStack};
use faer::linalg::lu::partial_pivoting::factor as faer_lu_factor;
use faer::linalg::matmul::matmul;
use faer::linalg::triangular_solve::solve_unit_lower_triangular_in_place;
use faer::prelude::{Reborrow, ReborrowMut};
use faer::{Accum, MatMut, MatRef, Par};
use rayon::{ThreadPool, ThreadPoolBuilder};
use rodas5p_core::{CoreError, CoreResult};

/// Panels of at most this many columns are factored by faer's sequential
/// `lu_in_place`.
pub const DENSE_LU_REUSE_SEQUENTIAL_PANEL_COLUMNS: usize = 64;

/// The least work of one part of a split triangular solve or product, in
/// floating-point operations.
pub const DENSE_LU_REUSE_CHUNK_FLOPS: usize = 2_000_000;

/// The least work of one part of a split row swap, in swapped elements.
pub const DENSE_LU_REUSE_CHUNK_SWAPS: usize = 65_536;

/// Split points are multiples of this many rows or columns: with faer's
/// column stride (a multiple of 8) and block sizes (multiples of 16 above
/// the sequential panel) two parts never write to one cache line.
const SPLIT_ALIGNMENT: usize = 16;

/// The pool and the scratch of the threaded factorization.
pub(super) struct FactorPool {
    pool: ThreadPool,
    workers: usize,
    /// Row `k` is swapped with row `k + transpositions[k]`, in order.
    transpositions: Vec<usize>,
    scratch: PanelScratch,
}

/// Scratch of the sequential panel factorizations, each of the order of
/// the matrix.
struct PanelScratch {
    perm_forward: Vec<usize>,
    perm_inverse: Vec<usize>,
    /// The row at each position and the position of each row while a
    /// panel's permutation is turned into transpositions.
    row_at: Vec<usize>,
    position_of: Vec<usize>,
    stack: MemBuffer,
}

impl FactorPool {
    pub(super) fn new(n: usize, workers: usize) -> CoreResult<Self> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(workers)
            .thread_name(|index| format!("dense-lu-reuse-{index}"))
            .build()
            .map_err(|error| {
                CoreError::InvalidInput(format!(
                    "the dense stage client could not start {workers} factorization workers: \
                     {error}"
                ))
            })?;
        let stack =
            faer_lu_factor::lu_in_place_scratch::<usize, f64>(n, n, Par::Seq, Default::default());
        Ok(Self {
            pool,
            workers,
            transpositions: vec![0; n],
            scratch: PanelScratch {
                perm_forward: vec![0; n],
                perm_inverse: vec![0; n],
                row_at: vec![0; n],
                position_of: vec![0; n],
                stack: MemBuffer::new(stack),
            },
        })
    }

    pub(super) fn workers(&self) -> usize {
        self.workers
    }

    /// Factor the square `matrix` in place (`L` strictly below the
    /// diagonal, `U` on and above it) and write the row permutation as
    /// faer's `lu_in_place` does: `perm_forward[i]` is the row of the input
    /// at position `i`, `perm_inverse` its inverse.
    pub(super) fn factor(
        &mut self,
        matrix: MatMut<'_, f64>,
        perm_forward: &mut [usize],
        perm_inverse: &mut [usize],
    ) {
        let workers = self.workers;
        let transpositions = &mut self.transpositions;
        let scratch = &mut self.scratch;
        self.pool
            .install(|| factor_columns(matrix, transpositions, scratch, workers));
        for (position, row) in perm_forward.iter_mut().enumerate() {
            *row = position;
        }
        for (k, t) in self.transpositions.iter().enumerate() {
            perm_forward.swap(k, k + t);
        }
        for (position, row) in perm_forward.iter().enumerate() {
            perm_inverse[*row] = position;
        }
    }
}

/// faer's block size of a level with `width` columns.
fn left_width(width: usize) -> usize {
    let half = width / 2;
    half.next_multiple_of(half.next_power_of_two().min(16))
}

/// Factor all columns of `panel` (at least as many rows as columns) with
/// row pivoting inside the panel; `transpositions[k]` is the offset of the
/// row swapped with row `k`.
fn factor_columns(
    panel: MatMut<'_, f64>,
    transpositions: &mut [usize],
    scratch: &mut PanelScratch,
    workers: usize,
) {
    let width = panel.ncols();
    if width <= DENSE_LU_REUSE_SEQUENTIAL_PANEL_COLUMNS {
        factor_panel(panel, transpositions, scratch);
        return;
    }
    let left_width = left_width(width);
    let (mut left, mut right) = panel.split_at_col_mut(left_width);
    let (left_transpositions, right_transpositions) = transpositions.split_at_mut(left_width);
    factor_columns(left.rb_mut(), left_transpositions, scratch, workers);
    swap_rows(right.rb_mut(), left_transpositions, workers);
    {
        let (left_top, left_bottom) = left.rb().split_at_row(left_width);
        let (mut right_top, mut right_bottom) = right.rb_mut().split_at_row_mut(left_width);
        solve_unit_lower(left_top, right_top.rb_mut(), workers);
        subtract_product(right_bottom.rb_mut(), left_bottom, right_top.rb(), workers);
        factor_columns(right_bottom, right_transpositions, scratch, workers);
    }
    let (_, left_bottom) = left.split_at_row_mut(left_width);
    swap_rows(left_bottom, right_transpositions, workers);
}

/// faer's sequential factorization of a panel, and its permutation as
/// transpositions. The permutation is the result of swapping position `k`
/// with a position at or below it for `k = 0, 1, ...`; replaying it from the
/// identity recovers those swaps.
fn factor_panel(panel: MatMut<'_, f64>, transpositions: &mut [usize], scratch: &mut PanelScratch) {
    let rows = panel.nrows();
    let perm_forward = &mut scratch.perm_forward[..rows];
    faer_lu_factor::lu_in_place(
        panel,
        perm_forward,
        &mut scratch.perm_inverse[..rows],
        Par::Seq,
        MemStack::new(&mut scratch.stack),
        Default::default(),
    );
    let row_at = &mut scratch.row_at[..rows];
    let position_of = &mut scratch.position_of[..rows];
    for (position, (row, place)) in row_at.iter_mut().zip(position_of.iter_mut()).enumerate() {
        *row = position;
        *place = position;
    }
    for (k, t) in transpositions.iter_mut().enumerate() {
        let wanted = perm_forward[k];
        let from = position_of[wanted];
        let displaced = row_at[k];
        *t = from - k;
        row_at[k] = wanted;
        row_at[from] = displaced;
        position_of[wanted] = k;
        position_of[displaced] = from;
    }
}

/// The number of parts for `work` units with at least `grain` units per
/// part, at most `workers` and at most `extent / SPLIT_ALIGNMENT`.
fn parts(work: usize, grain: usize, extent: usize, workers: usize) -> usize {
    (work / grain)
        .min(workers)
        .min(extent / SPLIT_ALIGNMENT)
        .max(1)
}

/// The split point of `extent` for `left_parts` of `parts`, on the
/// alignment and strictly inside the extent.
fn split_point(extent: usize, left_parts: usize, parts: usize) -> usize {
    let exact = extent * left_parts / parts;
    (exact / SPLIT_ALIGNMENT * SPLIT_ALIGNMENT).clamp(SPLIT_ALIGNMENT.min(extent - 1), extent - 1)
}

/// `op(block, first_column)` on `parts` column blocks of `columns`.
fn for_each_column_block<F>(columns: MatMut<'_, f64>, parts: usize, first: usize, op: &F)
where
    F: Fn(MatMut<'_, f64>, usize) + Sync,
{
    if parts <= 1 || columns.ncols() < 2 {
        op(columns, first);
        return;
    }
    let left_parts = parts / 2;
    let split = split_point(columns.ncols(), left_parts, parts);
    let (left, right) = columns.split_at_col_mut(split);
    rayon::join(
        || for_each_column_block(left, left_parts, first, op),
        || for_each_column_block(right, parts - left_parts, first + split, op),
    );
}

/// `op(block, first_row)` on `parts` row blocks of `rows`.
fn for_each_row_block<F>(rows: MatMut<'_, f64>, parts: usize, first: usize, op: &F)
where
    F: Fn(MatMut<'_, f64>, usize) + Sync,
{
    if parts <= 1 || rows.nrows() < 2 {
        op(rows, first);
        return;
    }
    let left_parts = parts / 2;
    let split = split_point(rows.nrows(), left_parts, parts);
    let (top, bottom) = rows.split_at_row_mut(split);
    rayon::join(
        || for_each_row_block(top, left_parts, first, op),
        || for_each_row_block(bottom, parts - left_parts, first + split, op),
    );
}

/// Swap row `k` with row `k + transpositions[k]` in every column, for
/// `k = 0, 1, ...`.
fn swap_rows(columns: MatMut<'_, f64>, transpositions: &[usize], workers: usize) {
    let work = transpositions.len() * columns.ncols();
    let parts = parts(work, DENSE_LU_REUSE_CHUNK_SWAPS, columns.ncols(), workers);
    for_each_column_block(columns, parts, 0, &|mut block: MatMut<'_, f64>, _| {
        for j in 0..block.ncols() {
            let mut column = block.rb_mut().col_mut(j);
            if let Some(contiguous) = column.rb_mut().try_as_col_major_mut() {
                let values = contiguous.as_slice_mut();
                for (k, t) in transpositions.iter().enumerate() {
                    values.swap(k, k + t);
                }
            } else {
                for (k, t) in transpositions.iter().enumerate() {
                    let upper = column[k];
                    column[k] = column[k + t];
                    column[k + t] = upper;
                }
            }
        }
    });
}

/// `rhs <- L^-1 rhs` with the unit lower triangle of `triangle`.
fn solve_unit_lower(triangle: MatRef<'_, f64>, rhs: MatMut<'_, f64>, workers: usize) {
    let order = triangle.nrows();
    let work = order * order * rhs.ncols();
    let parts = parts(work, DENSE_LU_REUSE_CHUNK_FLOPS, rhs.ncols(), workers);
    for_each_column_block(rhs, parts, 0, &|block: MatMut<'_, f64>, _| {
        solve_unit_lower_triangular_in_place(triangle, block, Par::Seq);
    });
}

/// `dst <- dst - lhs rhs`, split over the longer side of `dst`.
fn subtract_product(
    dst: MatMut<'_, f64>,
    lhs: MatRef<'_, f64>,
    rhs: MatRef<'_, f64>,
    workers: usize,
) {
    let (rows, columns, inner) = (dst.nrows(), dst.ncols(), lhs.ncols());
    let work = 2 * rows * columns * inner;
    if columns >= rows {
        let parts = parts(work, DENSE_LU_REUSE_CHUNK_FLOPS, columns, workers);
        for_each_column_block(dst, parts, 0, &|block: MatMut<'_, f64>, first| {
            let width = block.ncols();
            matmul(
                block,
                Accum::Add,
                lhs,
                rhs.subcols(first, width),
                -1.0,
                Par::Seq,
            );
        });
    } else {
        let parts = parts(work, DENSE_LU_REUSE_CHUNK_FLOPS, rows, workers);
        for_each_row_block(dst, parts, 0, &|block: MatMut<'_, f64>, first| {
            let height = block.nrows();
            matmul(
                block,
                Accum::Add,
                lhs.subrows(first, height),
                rhs,
                -1.0,
                Par::Seq,
            );
        });
    }
}
