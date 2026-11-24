//! Internal factor/solve parity and cache invalidation tests.
use super::*;

fn band_state(n: usize, l: usize, u: usize, kernel: BandedKernel) -> BandState {
    let band = BandedJacobian {
        lower: l,
        upper: u,
        fill: Arc::new(|_, _, _| Ok(())),
    };
    Workspace::new_banded(n, &band, kernel)
        .unwrap()
        .band
        .unwrap()
}
fn bits(x: &[f64]) -> Vec<u64> {
    x.iter().map(|v| v.to_bits()).collect()
}
fn fixture(n: usize, l: usize, u: usize, seed: u64, pivot: bool) -> Vec<f64> {
    let width = l + u + 1;
    let mut j = vec![0.; n * width];
    let mut z = seed + 1;
    for i in 0..n {
        for col in i.saturating_sub(l)..=(i + u).min(n - 1) {
            z = z
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let v = ((z >> 32) as i32) as f64 / (i32::MAX as f64);
            j[i * width + (col + l - i)] = if i == col { -10.0 } else { v };
        }
    }
    if pivot && n > 2 && l > 0 && u > 0 {
        j[l] = 1.999; // inv=2 gives a small first pivot
        j[l + 1] = -2.;
        j[width + l - 1] = -4.;
    }
    j
}
fn check_one(n: usize, l: usize, u: usize, seed: u64, pivot: bool) {
    let j = fixture(n, l, u, seed, pivot);
    let inv = 2.;
    let rhs: Vec<f64> = (0..n).map(|i| ((i * 7 + 3) as f64).sin()).collect();
    let mut old = band_state(n, l, u, BandedKernel::Slices);
    old.jacobian.copy_from_slice(&j);
    old.factor(n, inv).unwrap();
    let mut expected = rhs.clone();
    old.solve(n, &mut expected);
    for kernel in [
        BandedKernel::FixedWidth2,
        BandedKernel::FixedWidth2ExactCache,
    ] {
        let mut new = band_state(n, l, u, kernel);
        new.jacobian.copy_from_slice(&j);
        new.factor(n, inv).unwrap();
        assert_eq!(old.pivots, new.pivots);
        assert_eq!(bits(&old.factors), bits(&new.factors));
        let mut got = rhs.clone();
        new.solve(n, &mut got);
        assert_eq!(bits(&expected), bits(&got));
        assert_eq!(old.work.factor_operations, new.work.factor_operations);
        assert_eq!(old.work.solve_operations, new.work.solve_operations);
        let (mut nr, mut na, mut nx, mut nb) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        for i in 0..n {
            let mut ax = 0.;
            let mut ar = 0.;
            for col in i.saturating_sub(l)..=(i + u).min(n - 1) {
                let a = if col == i {
                    inv - j[i * (l + u + 1) + l]
                } else {
                    -j[i * (l + u + 1) + (col + l - i)]
                };
                ax += a * got[col];
                ar += a.abs();
            }
            nr = nr.max((ax - rhs[i]).abs());
            na = na.max(ar);
            nx = nx.max(got[i].abs());
            nb = nb.max(rhs[i].abs());
        }
        assert!(
            nr / (na * nx + nb) <= 2e-14,
            "large independent linear residual n={n}"
        );
    }
}
#[test]
fn exact_fixed_width_parity_including_pivot_swaps() {
    for n in [3, 4, 7, 16, 31, 100] {
        for seed in 0..16 {
            for pivot in [false, true] {
                check_one(n, 2, 2, seed, pivot);
            }
        }
    }
}
#[test]
fn other_bandwidths_fall_back_without_changing_bits() {
    for (n, l, u) in [(1, 0, 0), (7, 0, 0), (9, 1, 1), (12, 3, 1), (11, 1, 3)] {
        check_one(n, l, u, 19, false);
    }
}
#[test]
fn cache_hits_only_after_success_and_invalidates_on_changed_matrix() {
    let n = 7;
    let mut b = band_state(n, 2, 2, BandedKernel::FixedWidth2ExactCache);
    b.jacobian = fixture(n, 2, 2, 12, false);
    let mut count = WorkCounters::default();
    b.factor_with_policy(n, 2., &mut count).unwrap();
    assert_eq!(count.direct_factorizations, 1);
    let factors = bits(&b.factors);
    for k in 0..5 {
        let mut rhs = vec![k as f64; n];
        b.solve(n, &mut rhs);
        b.factor_with_policy(n, 2., &mut count).unwrap();
    }
    assert_eq!(count.direct_factorizations, 1);
    assert_eq!(bits(&b.factors), factors);
    let shifted = f64::from_bits(2f64.to_bits() + 1);
    b.factor_with_policy(n, shifted, &mut count).unwrap();
    assert_eq!(count.direct_factorizations, 2);
    b.jacobian[2] = f64::from_bits(b.jacobian[2].to_bits() + 1);
    b.factor_with_policy(n, shifted, &mut count).unwrap();
    assert_eq!(count.direct_factorizations, 3);
}
#[test]
fn failed_factorization_cannot_leave_a_live_key() {
    let n = 7;
    let mut b = band_state(n, 2, 2, BandedKernel::FixedWidth2ExactCache);
    let good = fixture(n, 2, 2, 4, false);
    b.jacobian = good.clone();
    let mut count = WorkCounters::default();
    b.factor_with_policy(n, 2., &mut count).unwrap();
    b.jacobian.fill(0.);
    for i in 0..n {
        b.jacobian[i * 5 + 2] = 2.;
    }
    assert!(b.factor_with_policy(n, 2., &mut count).is_err());
    assert!(!b.exact_cache.as_ref().unwrap().matches(2., &good));
    b.jacobian = good;
    b.factor_with_policy(n, 2., &mut count).unwrap();
    assert_eq!(count.direct_factorizations, 3);
}
#[test]
fn cache_storage_is_only_allocated_for_opt_in_kernel() {
    let n = 17;
    let a = band_state(n, 2, 2, BandedKernel::Slices);
    let b = band_state(n, 2, 2, BandedKernel::FixedWidth2);
    let c = band_state(n, 2, 2, BandedKernel::FixedWidth2ExactCache);
    assert!(a.exact_cache.is_none() && b.exact_cache.is_none());
    assert!(c.exact_cache.is_some());
    assert_eq!(a.work.stored_slots, b.work.stored_slots);
    assert_eq!(c.work.stored_slots - a.work.stored_slots, n * 5);
}

/// A (2, 2) band whose diagonal does not dominate, so partial pivoting picks rows `k + 1`
/// and `k + 2` throughout; `singular` empties column `singular` of `W = inv I - J`.
fn weak_fixture(n: usize, seed: u64, inv: f64, singular: Option<usize>) -> Vec<f64> {
    let mut j = vec![0.0; n * 5];
    let mut z = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x2545_F491_4F6C_DD1D;
    for i in 0..n {
        for col in i.saturating_sub(2)..=(i + 2).min(n - 1) {
            z ^= z << 13;
            z ^= z >> 7;
            z ^= z << 17;
            let v = (z >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
            j[i * 5 + (col + 2 - i)] = if col == i { inv + 0.05 * v } else { v };
        }
    }
    if let Some(c) = singular {
        for i in c.saturating_sub(2)..=(c + 2).min(n - 1) {
            j[i * 5 + (c + 2 - i)] = if i == c { inv } else { 0.0 };
        }
    }
    j
}

/// SL-N12: the fixed-offset factorization and substitutions of `FixedWidth2` return the
/// pivots, factors, solutions, errors and counted operations of the Slices kernel under
/// pivoting in every interior column and under singular columns at the start, in the
/// interior and in the general tail.
#[test]
fn fixed_offsets_match_slices_under_pivoting_and_singular_columns() {
    let inv = 0.5;
    let mut swaps = [0usize; 3];
    for n in [5, 6, 7, 8, 9, 13, 64] {
        for seed in 0..24 {
            let singulars = [None, Some(0), Some(n / 2), Some(n - 3), Some(n - 1)];
            for singular in singulars {
                let j = weak_fixture(n, seed, inv, singular);
                let rhs: Vec<f64> = (0..n).map(|i| ((i * 5 + 1) as f64).cos()).collect();
                let mut old = band_state(n, 2, 2, BandedKernel::Slices);
                let mut new = band_state(n, 2, 2, BandedKernel::FixedWidth2);
                old.jacobian.copy_from_slice(&j);
                new.jacobian.copy_from_slice(&j);
                let a = old.factor(n, inv);
                let b = new.factor(n, inv);
                assert_eq!(format!("{a:?}"), format!("{b:?}"), "n={n} seed={seed}");
                assert_eq!(old.work, new.work, "n={n} seed={seed} {singular:?}");
                if a.is_err() {
                    assert!(singular.is_some(), "n={n} seed={seed}: unexpected singular");
                    continue;
                }
                assert_eq!(old.pivots, new.pivots);
                assert_eq!(bits(&old.factors), bits(&new.factors));
                for (k, p) in old.pivots.iter().enumerate() {
                    swaps[p - k] += 1;
                }
                let (mut x, mut y) = (rhs.clone(), rhs.clone());
                old.solve(n, &mut x);
                new.solve(n, &mut y);
                assert_eq!(bits(&x), bits(&y), "n={n} seed={seed}");
                assert_eq!(old.work, new.work);
            }
        }
    }
    // Every pivot choice of the fixed-offset branch is exercised.
    assert!(swaps.iter().all(|&count| count > 100), "{swaps:?}");
}
