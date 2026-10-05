//! Speed research node SPD02 (`research/spd02_lu_column_extents_20261005`):
//! the in-place LU with column extents against the zero-skipping LU of v2 on
//! 2,000 seeded random matrices (factors, pivots, extents, outcome and the
//! solution of a random right-hand side bitwise), plus the allocation and
//! library identity checks. No timing.

use rodas5p_integrators::lu_research::{column_extents, extents_of, solve, zero_skipping};

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 * 2.0_f64.powi(-53)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn value(&mut self) -> f64 {
        // Magnitudes over several decades, both signs.
        let m = 10.0_f64.powf(self.unit() * 6.0 - 3.0);
        if self.unit() < 0.5 { -m } else { m }
    }
}

/// The seeded matrix families of the preregistration.
fn matrix(rng: &mut SplitMix, index: usize) -> (Vec<f64>, usize, &'static str) {
    let n = 1 + rng.below(24);
    let family = index % 10;
    let mut a = vec![0.0; n * n];
    let label = match family {
        0..=2 => {
            // Banded with l, u in 0..4.
            let (l, u) = (rng.below(4), rng.below(4));
            for i in 0..n {
                for j in i.saturating_sub(l)..=(i + u).min(n - 1) {
                    a[i * n + j] = rng.value();
                }
            }
            "banded"
        }
        3 | 4 => {
            // Random sparse pattern with 1-20 % nonzeros.
            let density = 0.01 + 0.19 * rng.unit();
            for v in a.iter_mut() {
                if rng.unit() < density {
                    *v = rng.value();
                }
            }
            "sparse"
        }
        5 => {
            // Arrow: first row, first column and the diagonal.
            for i in 0..n {
                a[i] = rng.value();
                a[i * n] = rng.value();
                a[i * n + i] = rng.value();
            }
            "arrow"
        }
        6 => {
            for v in a.iter_mut() {
                *v = rng.value();
            }
            "dense"
        }
        7 => {
            // Banded with a dominant entry below the diagonal in some
            // columns, so pivoting swaps rows and fill-in leaves the band.
            let l = 1 + rng.below(3);
            for i in 0..n {
                for j in i.saturating_sub(l)..=(i + 1).min(n - 1) {
                    a[i * n + j] = rng.value();
                }
            }
            for k in 0..n {
                if rng.unit() < 0.5 {
                    let i = (k + 1 + rng.below(l)).min(n - 1);
                    a[i * n + k] = 1.0e3 * (1.0 + rng.unit());
                }
            }
            "forced-swaps"
        }
        8 => {
            // Sparse with structural zero rows, zero diagonals and exact
            // cancellation candidates (duplicated values).
            let density = 0.05 + 0.2 * rng.unit();
            for v in a.iter_mut() {
                if rng.unit() < density {
                    *v = rng.value();
                }
            }
            let zero_row = rng.below(n);
            for j in 0..n {
                a[zero_row * n + j] = 0.0;
            }
            let zero_diag = rng.below(n);
            a[zero_diag * n + zero_diag] = 0.0;
            if n > 1 {
                // Two rows that cancel under elimination: row 1 = row 0.
                for j in 0..n {
                    a[n + j] = a[j];
                }
            }
            "degenerate"
        }
        _ => {
            // Banded with -0.0, one NaN and one Inf entry.
            let (l, u) = (rng.below(3), rng.below(3));
            for i in 0..n {
                for j in i.saturating_sub(l)..=(i + u).min(n - 1) {
                    a[i * n + j] = rng.value();
                }
            }
            a[rng.below(n * n)] = -0.0;
            if rng.unit() < 0.5 {
                a[rng.below(n * n)] = f64::NAN;
            } else {
                a[rng.below(n * n)] = f64::INFINITY;
            }
            "nonfinite"
        }
    };
    (a, n, label)
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Factors, pivots, extents, outcome and the solution of both variants are
/// bitwise equal on every matrix of the seeded set.
#[test]
fn column_extents_equal_the_zero_skipping_lu_bitwise() {
    let mut rng = SplitMix(0x5bd2_0105_2026_1005);
    let mut counts = std::collections::BTreeMap::new();
    let mut errors = 0_usize;
    for index in 0..2000 {
        let (a, n, label) = matrix(&mut rng, index);
        let b: Vec<f64> = (0..n).map(|_| rng.value()).collect();
        // v2.
        let mut a1 = a.clone();
        let mut pivots1 = vec![0; n];
        let mut row_end1 = vec![0; n];
        let mut l_start1 = vec![n; n];
        for (i, row) in a1.chunks_exact(n).enumerate() {
            row_end1[i] = row.iter().rposition(|v| *v != 0.0).unwrap_or(0).max(i);
        }
        let initial_row_end = row_end1.clone();
        let r1 = zero_skipping(&mut a1, n, &mut pivots1, &mut row_end1, &mut l_start1);
        // Column extents.
        let mut a2 = a.clone();
        let mut pivots2 = vec![0; n];
        let mut row_end2 = vec![0; n];
        let mut l_start2 = vec![n; n];
        let mut col_end = vec![0; n];
        extents_of(&a2, n, &mut row_end2, &mut l_start2, &mut col_end);
        assert_eq!(
            initial_row_end, row_end2,
            "{label} #{index}: initial row extents"
        );
        let r2 = column_extents(
            &mut a2,
            n,
            &mut pivots2,
            &mut row_end2,
            &mut l_start2,
            &mut col_end,
        );
        assert_eq!(r1.is_ok(), r2.is_ok(), "{label} #{index}: outcome");
        if let (Err(e1), Err(e2)) = (&r1, &r2) {
            assert_eq!(
                e1.to_string(),
                e2.to_string(),
                "{label} #{index}: error text"
            );
            errors += 1;
        }
        assert_eq!(bits(&a1), bits(&a2), "{label} #{index}: factors");
        assert_eq!(pivots1, pivots2, "{label} #{index}: pivots");
        assert_eq!(row_end1, row_end2, "{label} #{index}: row extents");
        assert_eq!(l_start1, l_start2, "{label} #{index}: multiplier starts");
        if r1.is_ok() {
            let mut x1 = b.clone();
            let mut x2 = b.clone();
            solve(&a1, n, &pivots1, &row_end1, &l_start1, &mut x1);
            solve(&a2, n, &pivots2, &row_end2, &l_start2, &mut x2);
            assert_eq!(bits(&x1), bits(&x2), "{label} #{index}: solution");
        }
        *counts.entry(label).or_insert(0_usize) += 1;
    }
    println!("families {counts:?}, singular or non-finite outcomes {errors}");
    assert_eq!(counts.values().sum::<usize>(), 2000);
    assert!(
        errors > 0,
        "the degenerate and nonfinite families must exercise the error path"
    );
}

/// The column extent is an upper bound on the last nonzero row of every
/// column at every stage of the factorization (checked on the final factors
/// of banded matrices with forced swaps: no stored multiplier lies below it).
#[test]
fn column_extents_bound_the_stored_multipliers() {
    let mut rng = SplitMix(0x1234_5678_9abc_def0);
    for index in 0..300 {
        let (a, n, _) = matrix(&mut rng, 7 + 10 * index);
        let mut a2 = a.clone();
        let mut pivots = vec![0; n];
        let mut row_end = vec![0; n];
        let mut l_start = vec![n; n];
        let mut col_end = vec![0; n];
        extents_of(&a2, n, &mut row_end, &mut l_start, &mut col_end);
        if column_extents(
            &mut a2,
            n,
            &mut pivots,
            &mut row_end,
            &mut l_start,
            &mut col_end,
        )
        .is_err()
        {
            continue;
        }
        for i in 0..n {
            for j in 0..n {
                if a2[i * n + j] != 0.0 {
                    assert!(
                        i <= col_end[j],
                        "#{index}: entry ({i}, {j}) below the column extent {}",
                        col_end[j]
                    );
                }
            }
        }
    }
}
