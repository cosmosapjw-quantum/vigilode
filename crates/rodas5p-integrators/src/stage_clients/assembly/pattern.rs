//! Declared sparsity patterns and their Curtis-Powell-Reid column groups
//! (loop node V08).

use super::AssemblyError;

/// The entries of an `n x n` Jacobian that may be nonzero, by row.
///
/// The pattern is a statement about what the right-hand side *reads*: entry
/// `(i, j)` must be declared whenever the computation of component `i`
/// uses the value of `y_j` in any way that can change a bit of the result.
/// A term that cancels in exact arithmetic but not in floating point is a
/// dependence. [`super::AssemblyStageClient`] checks the declaration (see
/// its documentation) and refuses a pattern it finds violated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparsityPattern {
    n: usize,
    row_ptr: Vec<usize>,
    col_idx: Vec<usize>,
}

impl SparsityPattern {
    /// The pattern with the columns `rows[i]` in row `i`. Every row must be
    /// strictly increasing with columns below `n`; `rows.len()` must be `n`.
    pub fn from_rows(n: usize, rows: &[Vec<usize>]) -> Result<Self, AssemblyError> {
        if n == 0 || rows.len() != n {
            return Err(AssemblyError::Configuration(format!(
                "a pattern of order {n} needs {n} rows (n > 0), got {}",
                rows.len()
            )));
        }
        let mut row_ptr = Vec::with_capacity(n + 1);
        let mut col_idx = Vec::with_capacity(rows.iter().map(Vec::len).sum());
        row_ptr.push(0);
        for (i, row) in rows.iter().enumerate() {
            if row.windows(2).any(|w| w[0] >= w[1]) {
                return Err(AssemblyError::Configuration(format!(
                    "pattern row {i}: columns must be strictly increasing"
                )));
            }
            if row.last().is_some_and(|&last| last >= n) {
                return Err(AssemblyError::Configuration(format!(
                    "pattern row {i}: column index out of range"
                )));
            }
            col_idx.extend_from_slice(row);
            row_ptr.push(col_idx.len());
        }
        Ok(Self {
            n,
            row_ptr,
            col_idx,
        })
    }

    /// The full band: row `i` holds the columns `i - lower ..= i + upper`
    /// inside the matrix. Requires `n > 0` and `lower, upper < n`.
    pub fn banded(n: usize, lower: usize, upper: usize) -> Result<Self, AssemblyError> {
        if n == 0 || lower >= n || upper >= n {
            return Err(AssemblyError::Configuration(format!(
                "a band ({lower}, {upper}) must lie inside a matrix of positive order, got {n}"
            )));
        }
        let mut row_ptr = Vec::with_capacity(n + 1);
        let mut col_idx = Vec::with_capacity(n * (lower + upper + 1));
        row_ptr.push(0);
        for i in 0..n {
            col_idx.extend(i.saturating_sub(lower)..=(i + upper).min(n - 1));
            row_ptr.push(col_idx.len());
        }
        Ok(Self {
            n,
            row_ptr,
            col_idx,
        })
    }

    /// The order of the matrix.
    pub fn n(&self) -> usize {
        self.n
    }

    /// The number of declared entries.
    pub fn nnz(&self) -> usize {
        self.col_idx.len()
    }

    /// The declared columns of row `i`, strictly increasing.
    pub fn row(&self, i: usize) -> &[usize] {
        &self.col_idx[self.row_ptr[i]..self.row_ptr[i + 1]]
    }

    /// Whether entry `(i, j)` is declared.
    pub fn contains(&self, i: usize, j: usize) -> bool {
        i < self.n && self.row(i).binary_search(&j).is_ok()
    }

    /// The smallest band `(lower, upper)` that holds the pattern.
    pub fn bandwidth(&self) -> (usize, usize) {
        let (mut lower, mut upper) = (0, 0);
        for i in 0..self.n {
            let row = self.row(i);
            if let (Some(&first), Some(&last)) = (row.first(), row.last()) {
                lower = lower.max(i.saturating_sub(first));
                upper = upper.max(last.saturating_sub(i));
            }
        }
        (lower, upper)
    }

    /// The pattern by column: `rows_of(j)` are the rows of column `j`,
    /// increasing.
    pub(super) fn by_column(&self) -> ColumnView {
        let n = self.n;
        let mut col_ptr = vec![0_usize; n + 1];
        for &j in &self.col_idx {
            col_ptr[j + 1] += 1;
        }
        for j in 0..n {
            col_ptr[j + 1] += col_ptr[j];
        }
        let mut next = col_ptr.clone();
        let mut row_idx = vec![0_usize; self.col_idx.len()];
        for i in 0..n {
            for &j in self.row(i) {
                row_idx[next[j]] = i;
                next[j] += 1;
            }
        }
        ColumnView { col_ptr, row_idx }
    }
}

/// A [`SparsityPattern`] by column.
pub(super) struct ColumnView {
    col_ptr: Vec<usize>,
    row_idx: Vec<usize>,
}

impl ColumnView {
    /// The rows of column `j`, increasing.
    pub(super) fn rows_of(&self, j: usize) -> &[usize] {
        &self.row_idx[self.col_ptr[j]..self.col_ptr[j + 1]]
    }
}

/// The Curtis-Powell-Reid column groups of `pattern`: the columns are taken
/// in natural order and each goes into the first group none of whose columns
/// shares a row with it. Deterministic. Returns the group ("colour") of
/// every column and the number of groups. For a full band `(lower, upper)`
/// this is column `j` in group `j mod (lower + upper + 1)`, the grouping of
/// the facade's banded finite differences (`crates/vigilode/src/fd.rs:597-598`).
pub fn colour_columns(pattern: &SparsityPattern) -> (Vec<usize>, usize) {
    let n = pattern.n();
    let columns = pattern.by_column();
    let mut colour = vec![usize::MAX; n];
    // `forbidden[k] == j` while column `j` is being placed: group `k` already
    // holds a column that shares a row with `j`.
    let mut forbidden = vec![usize::MAX; n + 1];
    let mut count = 0;
    for j in 0..n {
        for &i in columns.rows_of(j) {
            for &other in pattern.row(i) {
                if colour[other] != usize::MAX {
                    forbidden[colour[other]] = j;
                }
            }
        }
        let k = (0..=n)
            .find(|&k| forbidden[k] != j)
            .expect("n + 1 groups always suffice");
        colour[j] = k;
        count = count.max(k + 1);
    }
    (colour, count)
}

/// Whether no two columns of one group share a row.
pub fn colouring_is_valid(pattern: &SparsityPattern, colour: &[usize]) -> bool {
    if colour.len() != pattern.n() {
        return false;
    }
    let mut seen = vec![usize::MAX; pattern.n() + 1];
    (0..pattern.n()).all(|i| {
        pattern.row(i).iter().all(|&j| {
            let k = colour[j].min(pattern.n());
            let fresh = seen[k] != i;
            seen[k] = i;
            fresh
        })
    })
}
