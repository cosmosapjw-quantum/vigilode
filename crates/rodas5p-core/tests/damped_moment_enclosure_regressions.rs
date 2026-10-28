//! Adversarial input-arithmetic regressions for the exact real meaning of f64 inputs.

use rodas5p_core::damped_moments::{
    DampedMomentTable, DampedMomentsConfig, compute_normalized_damped_moment,
};

#[test]
fn zero_damping_encloses_nonrepresentable_beta_seed() {
    let result = compute_normalized_damped_moment(0.0, 0.5, 2, 0).unwrap();
    // Exact 1/3 lies strictly between these consecutive binary64 numbers.
    assert!(result.lo <= f64::from_bits(0x3fd5_5555_5555_5555));
    assert!(result.hi >= f64::from_bits(0x3fd5_5555_5555_5556));
}

#[test]
fn input_product_and_both_branches_enclose_independent_decimal_oracles() {
    // 240-decimal-digit integration of the exponential Taylor series using exact
    // Decimal::from_float inputs. Each oracle is bracketed by adjacent binary64
    // endpoints. In particular exact-real (0.1_f64 * 10) is strictly above 1,
    // although the nearest product is 1: the input-product enclosure is material.
    let fixtures = [
        (0.1, 10.0, 0, 0, 0x3fe43a54e4e98863, 0x3fe43a54e4e98864),
        (0.1, 0.3, 2, 1, 0x3fb4f401614cd73b, 0x3fb4f401614cd73c),
        (1.000001, 1.0, 3, 2, 0x3f839363a7bfaf46, 0x3f839363a7bfaf47),
        (10.0, 1.0, 0, 0, 0x3fb9994d6e7a8858, 0x3fb9994d6e7a8859),
        (25.0, 0.4, 4, 3, 0x3f07f84896eb6b33, 0x3f07f84896eb6b34),
        (0.8, 0.3, 6, 2, 0x3f6b7ef37bb314dc, 0x3f6b7ef37bb314dd),
        (0.0, 0.5, 2, 3, 0x3f91111111111111, 0x3f91111111111112),
    ];
    for (kappa, horizon, j, k, lo_bits, hi_bits) in fixtures {
        let result = compute_normalized_damped_moment(kappa, horizon, j, k).unwrap();
        assert!(
            result.lo <= f64::from_bits(lo_bits),
            "lower endpoint: {kappa}, {horizon}, {j}, {k}: {result:?}"
        );
        assert!(
            result.hi >= f64::from_bits(hi_bits),
            "upper endpoint: {kappa}, {horizon}, {j}, {k}: {result:?}"
        );
    }
}

#[test]
fn public_config_mutations_are_revalidated() {
    let mut config = DampedMomentsConfig::new(1.0, 0.5, 2, 2).unwrap();
    config.max_j = usize::MAX;
    assert!(DampedMomentTable::compile(config).is_err());
    config.max_j = 2;
    config.horizon = -0.5;
    assert!(DampedMomentTable::compile(config).is_err());
}

#[test]
fn product_overflow_is_refused_and_underflow_is_enclosed() {
    assert!(compute_normalized_damped_moment(f64::MAX, f64::MAX, 0, 0).is_err());
    let tiny = f64::from_bits(1);
    let result = compute_normalized_damped_moment(tiny, 0.5, 0, 0).unwrap();
    // 0 < exact kappa*h < min_subnormal; exp(-z v) is near 1 throughout.
    assert!(result.lo <= 1.0_f64.next_down());
    assert!(result.hi >= 1.0);
}
