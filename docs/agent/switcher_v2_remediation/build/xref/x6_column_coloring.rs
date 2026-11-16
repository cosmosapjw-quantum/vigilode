//! Column colouring of a declared sparsity pattern for exact
//! Jacobian assembly from compressed JVP probes (Curtis-Powell-Reid).

/// Row-compressed sparsity pattern of an n x n Jacobian.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparsityPattern {
    n: usize,
    row_ptr: Vec<usize>,
    col_idx: Vec<usize>,
}

impl SparsityPattern {
    /// Build from per-row column lists. Columns must be < n, strictly increasing per row.
    pub fn from_rows(n: usize, rows: &[Vec<usize>]) -> Result<Self, String> {
        if rows.len() != n {
            return Err(format!("expected {n} rows, got {}", rows.len()));
        }
        let mut row_ptr = Vec::with_capacity(n + 1);
        let mut col_idx = Vec::new();
        row_ptr.push(0);
        for (i, r) in rows.iter().enumerate() {
            for w in r.windows(2) {
                if w[0] >= w[1] {
                    return Err(format!("row {i}: columns must be strictly increasing"));
                }
            }
            if r.iter().any(|&c| c >= n) {
                return Err(format!("row {i}: column index out of range"));
            }
            col_idx.extend_from_slice(r);
            row_ptr.push(col_idx.len());
        }
        Ok(Self {
            n,
            row_ptr,
            col_idx,
        })
    }
    /// Dimension n of the n x n pattern.
    pub fn n(&self) -> usize {
        self.n
    }
    /// Number of structural nonzeros.
    pub fn nnz(&self) -> usize {
        self.col_idx.len()
    }
    /// Column indices of row `i`, strictly increasing.
    pub fn row(&self, i: usize) -> &[usize] {
        &self.col_idx[self.row_ptr[i]..self.row_ptr[i + 1]]
    }
}

/// Greedy colouring in natural column order: two columns that share a row get
/// different colours. Deterministic. Returns (colour per column, colour count).
pub fn column_coloring(p: &SparsityPattern) -> (Vec<usize>, usize) {
    let n = p.n();
    let mut rows_of_col: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for &c in p.row(i) {
            rows_of_col[c].push(i);
        }
    }
    let mut color = vec![usize::MAX; n];
    let mut forbidden = vec![usize::MAX; n + 1];
    let mut count = 0;
    for c in 0..n {
        for &i in &rows_of_col[c] {
            for &other in p.row(i) {
                if color[other] != usize::MAX {
                    forbidden[color[other]] = c;
                }
            }
        }
        let k = (0..=n)
            .find(|&k| forbidden[k] != c)
            .expect("n + 1 colours always suffice");
        color[c] = k;
        count = count.max(k + 1);
    }
    (color, count)
}

/// True when no two columns sharing a row have the same colour.
pub fn coloring_is_valid(p: &SparsityPattern, color: &[usize]) -> bool {
    (0..p.n()).all(|i| {
        let r = p.row(i);
        r.iter()
            .enumerate()
            .all(|(a, &x)| r[a + 1..].iter().all(|&y| color[x] != color[y]))
    })
}

/// Probe vector for colour `k`: 1 at columns of that colour, 0 elsewhere.
pub fn seed_vector(color: &[usize], k: usize, out: &mut [f64]) {
    for (o, &c) in out.iter_mut().zip(color) {
        *o = if c == k { 1.0 } else { 0.0 };
    }
}

/// Recover the nonzeros (row-major, pattern order) from compressed products
/// `compressed[k] = J * seed_vector(k)`.
pub fn recover_values(
    p: &SparsityPattern,
    color: &[usize],
    compressed: &[Vec<f64>],
    values: &mut [f64],
) {
    let mut slots = values.iter_mut();
    for (i, w) in p.row_ptr.windows(2).enumerate() {
        for &c in &p.col_idx[w[0]..w[1]] {
            *slots.next().expect("values has nnz slots") = compressed[color[c]][i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tridiagonal(n: usize) -> SparsityPattern {
        let rows: Vec<Vec<usize>> = (0..n)
            .map(|i| (i.saturating_sub(1)..=(i + 1).min(n - 1)).collect())
            .collect();
        SparsityPattern::from_rows(n, &rows).expect("valid test input")
    }

    fn five_point(m: usize) -> SparsityPattern {
        let n = m * m;
        let rows: Vec<Vec<usize>> = (0..n)
            .map(|k| {
                let (i, j) = (k / m, k % m);
                let mut r = vec![k];
                if i > 0 {
                    r.push(k - m);
                }
                if i + 1 < m {
                    r.push(k + m);
                }
                if j > 0 {
                    r.push(k - 1);
                }
                if j + 1 < m {
                    r.push(k + 1);
                }
                r.sort_unstable();
                r
            })
            .collect();
        SparsityPattern::from_rows(n, &rows).expect("valid test input")
    }

    #[test]
    fn tridiagonal_needs_three_colours() {
        let p = tridiagonal(10);
        let (c, k) = column_coloring(&p);
        assert_eq!(k, 3);
        assert!(coloring_is_valid(&p, &c));
    }

    #[test]
    fn dense_row_forces_n_colours() {
        let n = 6;
        let mut rows: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
        rows[0] = (0..n).collect();
        let p = SparsityPattern::from_rows(n, &rows).expect("valid test input");
        let (c, k) = column_coloring(&p);
        assert_eq!(k, n);
        assert!(coloring_is_valid(&p, &c));
    }

    #[test]
    fn five_point_colouring_is_valid_and_bounded() {
        let p = five_point(8);
        let (c, k) = column_coloring(&p);
        assert!(coloring_is_valid(&p, &c));
        // Delta(G) + 1 = 5 <= chi(G^2) <= greedy <= Delta(G^2) + 1 = 13.
        assert!((5..=13).contains(&k), "colours {k}");
    }

    #[test]
    fn recovery_reproduces_dense_jacobian() {
        let p = five_point(5);
        let n = p.n();
        let value = |i: usize, j: usize| 1.0 + (3 * i + 7 * j) as f64 / 11.0;
        let (c, k) = column_coloring(&p);
        let mut compressed = vec![vec![0.0; n]; k];
        let mut seed = vec![0.0; n];
        for (kk, out) in compressed.iter_mut().enumerate() {
            seed_vector(&c, kk, &mut seed);
            for (i, o) in out.iter_mut().enumerate() {
                *o = p.row(i).iter().map(|&j| value(i, j) * seed[j]).sum();
            }
        }
        let mut values = vec![0.0; p.nnz()];
        recover_values(&p, &c, &compressed, &mut values);
        let mut v = 0;
        for i in 0..n {
            for &j in p.row(i) {
                assert_eq!(values[v].to_bits(), value(i, j).to_bits());
                v += 1;
            }
        }
    }

    #[test]
    fn malformed_patterns_are_refused() {
        assert!(SparsityPattern::from_rows(2, &[vec![1, 0], vec![1]]).is_err());
        assert!(SparsityPattern::from_rows(2, &[vec![0, 2], vec![1]]).is_err());
        assert!(SparsityPattern::from_rows(2, &[vec![0]]).is_err());
    }
}
