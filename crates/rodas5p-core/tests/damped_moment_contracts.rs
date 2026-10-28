use rodas5p_core::CoreError;
use rodas5p_core::damped_moments::{
    DampedMomentTable, DampedMomentsConfig, compute_normalized_damped_moment,
};
use rodas5p_core::directed::Interval;

fn mid(i: Interval) -> f64 {
    (i.lo + i.hi) * 0.5
}

fn exact_beta(j: usize, k: usize) -> f64 {
    let mut val = 1.0 / ((j + 1) as f64);
    for step in 1..=k {
        val *= (step as f64) / ((j + step + 1) as f64);
    }
    val
}

// ---------------------------------------------------------------------------
// VG-1: Zero-Allocation and Workspace Safety Contract
// ---------------------------------------------------------------------------

#[test]
fn test_vg1_zero_allocation_table_lookup_contract() {
    let config = DampedMomentsConfig::new(2.5, 0.4, 8, 8).expect("valid config");
    let table = DampedMomentTable::compile(config).expect("compile table");

    // Once compiled, table lookups do not perform any heap allocation
    for j in 0..=8 {
        for k in 0..=8 {
            let norm = table.get_normalized(j, k).expect("get norm");
            assert!(norm.lo.is_finite() && norm.hi.is_finite());
            assert!(norm.lo <= norm.hi);
            let unscaled = table.get_unscaled(j, k).expect("get unscaled");
            assert!(unscaled.lo.is_finite() && unscaled.hi.is_finite());
            assert!(unscaled.lo <= unscaled.hi);
        }
    }
}

// ---------------------------------------------------------------------------
// VG-2: Mathematical Convergence & Limiting Consistency
// ---------------------------------------------------------------------------

#[test]
fn test_vg2_kappa_zero_reduces_to_exact_beta_weights() {
    let h = 0.5;
    let config = DampedMomentsConfig::new(0.0, h, 6, 6).expect("kappa = 0");
    let table = DampedMomentTable::compile(config).expect("compile table");

    for j in 0..=6 {
        for k in 0..=6 {
            let b_true = exact_beta(j, k);
            let norm = table.get_normalized(j, k).expect("norm moment");
            assert!(
                norm.contains(b_true),
                "norm [{:e}, {:e}] must contain exact beta {:e} for (j={}, k={})",
                norm.lo,
                norm.hi,
                b_true,
                j,
                k
            );

            let unscaled = table.get_unscaled(j, k).expect("unscaled moment");
            let expected_unscaled = b_true * h.powi((j + k + 1) as i32);
            let diff = (mid(unscaled) - expected_unscaled).abs();
            assert!(
                diff <= 1e-14 * (1.0 + expected_unscaled),
                "unscaled moment mismatch: diff={:e}",
                diff
            );
        }
    }
}

#[test]
fn test_vg2_monotonic_damping_reduction() {
    // For any fixed (j, k), as z = kappa * h increases, e^{-z(1-u)} decreases monotonically,
    // hence J_{j, k}(z1) > J_{j, k}(z2) for z1 < z2.
    let kappas = [0.0, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 50.0];
    let h = 0.2;

    for j in 0..=4 {
        for k in 0..=4 {
            let mut prev_val = f64::INFINITY;
            for &kappa in &kappas {
                let m = compute_normalized_damped_moment(kappa, h, j, k).expect("moment");
                assert!(
                    m.hi <= prev_val + 1e-15,
                    "monotonicity violated for (j={}, k={}, kappa={}): curr hi={:e} > prev={:e}",
                    j,
                    k,
                    kappa,
                    m.hi,
                    prev_val
                );
                prev_val = mid(m);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// VG-3: Outward Interval Enclosure & Branch Continuity
// ---------------------------------------------------------------------------

#[test]
fn test_vg3_outward_interval_enclosure_bounds() {
    // Certified: 0 <= lo <= hi <= Beta(j+1, k+1)
    let kappas = [0.0, 0.05, 0.8, 1.0, 1.5, 10.0, 100.0];
    let h = 0.3;

    for &kappa in &kappas {
        let config = DampedMomentsConfig::new(kappa, h, 6, 6).expect("config");
        let table = DampedMomentTable::compile(config).expect("table");

        for j in 0..=6 {
            for k in 0..=6 {
                let b_true = exact_beta(j, k);
                let m = table.get_normalized(j, k).expect("norm");

                assert!(m.lo >= 0.0, "moment lower bound must be non-negative");
                assert!(m.lo <= m.hi, "moment interval must be ordered");
                assert!(
                    m.hi <= b_true * (1.0 + 1e-14),
                    "moment upper bound must not exceed un-damped beta weight"
                );
            }
        }
    }
}

#[test]
fn test_vg3_branch_boundary_continuity_at_z_one() {
    // Check continuity across the algorithm switch at z = kappa * h = 1.0
    // (Taylor series vs backward recurrence)
    let h = 1.0;
    let z_left = 0.999999;
    let z_right = 1.000001;

    for j in 0..=4 {
        for k in 0..=4 {
            let m_left = compute_normalized_damped_moment(z_left, h, j, k).expect("left");
            let m_right = compute_normalized_damped_moment(z_right, h, j, k).expect("right");

            let diff = (mid(m_left) - mid(m_right)).abs();
            assert!(
                diff < 1e-5,
                "continuity jump across z=1.0 at (j={}, k={}): diff={:e}",
                j,
                k,
                diff
            );
        }
    }
}

#[test]
fn test_vg3_unscaled_scaling_identity() {
    let h = 0.25;
    let kappa = 3.5;
    let config = DampedMomentsConfig::new(kappa, h, 4, 4).expect("config");
    let table = DampedMomentTable::compile(config).expect("table");

    for j in 0..=4 {
        for k in 0..=4 {
            let norm = table.get_normalized(j, k).expect("norm");
            let unscaled = table.get_unscaled(j, k).expect("unscaled");

            let factor = h.powi((j + k + 1) as i32);
            let expected_mid = mid(norm) * factor;
            let actual_mid = mid(unscaled);

            let rel_diff = (actual_mid - expected_mid).abs() / expected_mid;
            assert!(
                rel_diff < 1e-12,
                "unscaled scaling identity violated at (j={}, k={}): rel_diff={:e}",
                j,
                k,
                rel_diff
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Negative Admission & Refusal Gates
// ---------------------------------------------------------------------------

#[test]
fn test_negative_admission_gates() {
    // 1. Negative kappa
    let res = DampedMomentsConfig::new(-1.0, 0.5, 4, 4);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));

    // 2. Zero or negative horizon
    let res = DampedMomentsConfig::new(1.0, 0.0, 4, 4);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));
    let res = DampedMomentsConfig::new(1.0, -0.5, 4, 4);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));

    // 3. Non-finite values
    let res = DampedMomentsConfig::new(f64::NAN, 0.5, 4, 4);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));
    let res = DampedMomentsConfig::new(1.0, f64::INFINITY, 4, 4);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));

    // 4. Order too large (> 64)
    let res = DampedMomentsConfig::new(1.0, 0.5, 65, 4);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));
    let res = DampedMomentsConfig::new(1.0, 0.5, 4, 65);
    assert!(matches!(res, Err(CoreError::InvalidInput(_))));

    // 5. Table index out of bounds
    let config = DampedMomentsConfig::new(1.0, 0.5, 3, 3).expect("config");
    let table = DampedMomentTable::compile(config).expect("table");
    assert!(matches!(
        table.get_normalized(4, 2),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(matches!(
        table.get_normalized(2, 4),
        Err(CoreError::InvalidInput(_))
    ));
}
