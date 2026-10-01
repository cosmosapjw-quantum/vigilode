//! Transform-error bound contracts of re-audit R4 of 2026-10-01
//! (ARITH-DEV-01..03), against the exact-rational oracle exported by
//! `tools/r4_export_transform_oracle_fixtures.py`.

use rodas5p_core::{
    DenseMatrix,
    transform_bound::{
        ExpBound, MAX_TRANSFORM_ORDER, TransformBound, TransformOperatorClass,
        bound_transform_error, reciprocal_factorials_upper,
    },
    weight_phi_vectors,
};
use serde_json::Value;
use std::cmp::Ordering;

const FIXTURES: &str = include_str!("../../../fixtures/r4_transform_oracle_fixtures.json");

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).unwrap()
}

fn bits(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}

fn vector(value: &Value) -> Vec<f64> {
    value.as_array().unwrap().iter().map(bits).collect()
}

fn vectors(value: &Value) -> Vec<Vec<f64>> {
    value.as_array().unwrap().iter().map(vector).collect()
}

fn exp_bound(value: &Value) -> ExpBound {
    ExpBound {
        mantissa: value["mantissa"].as_f64().unwrap(),
        exponent: value["exponent"].as_i64().unwrap(),
    }
}

fn matrix(rows: &[Vec<f64>]) -> DenseMatrix {
    DenseMatrix::from_rows(&rows.iter().map(Vec::as_slice).collect::<Vec<_>>()).unwrap()
}

/// `upper >= error` exactly: `error_upper` is the exact error rounded up on
/// its binade, so a valid bound is at least it.
fn assert_encloses(id: &str, result: &TransformBound, error_upper: ExpBound) {
    let TransformBound::Bounded { upper, .. } = result else {
        panic!("{id}: {result:?}");
    };
    assert_ne!(
        upper.total_cmp(&error_upper),
        Ordering::Less,
        "{id}: bound {upper:?} below the exact error {error_upper:?}"
    );
}

#[test]
fn every_oracle_case_is_enclosed_including_permutations_and_transposes() {
    let data = fixtures();
    for case in data["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let rows = vectors(&case["matrix"]);
        let h = bits(&case["h"]);
        let input = vectors(&case["vectors"]);
        let stored = vectors(&case["stored"]);
        if case["stored_is_source"].as_bool().unwrap() {
            // The oracle's emulation of the weighting is the production one.
            let (source, _) = weight_phi_vectors(h, &input).unwrap();
            for (left, right) in source.iter().flatten().zip(stored.iter().flatten()) {
                assert_eq!(left.to_bits(), right.to_bits(), "{id}");
            }
        }
        let result = bound_transform_error(&matrix(&rows), h, &input, &stored).unwrap();
        assert_encloses(id, &result, exp_bound(&case["error_upper"]));
        if id.starts_with("subunit") || id.starts_with("chain3") {
            assert!(matches!(
                result,
                TransformBound::Bounded {
                    class: TransformOperatorClass::Nilpotent { .. },
                    ..
                }
            ));
        }
    }
}

#[test]
fn the_subunit_nilpotent_witness_keeps_its_norm() {
    // R4-ARITH-01: ZERO (exponent 0) used to win the row maximum over 0.25
    // (exponent -1), erasing h A; the claimed bound 5.8e-11 sat below the
    // exact error 1.507e-9.
    let a = matrix(&[vec![0.0, 0.25], vec![0.0, 0.0]]);
    let input = vec![vec![0.0, 0.0], vec![0.0, 0.0], vec![0.0, 1.0]];
    let (stored, _) = weight_phi_vectors(1000.1, &input).unwrap();
    let result = bound_transform_error(&a, 1000.1, &input, &stored).unwrap();
    let TransformBound::Bounded {
        upper, operator, ..
    } = &result
    else {
        panic!("{result:?}");
    };
    // C_2 = 1/2 + |h| N / 3! with N = 0.25: about 41.8, not 1/2.
    assert!(operator[2].to_f64_up() > 41.0, "{operator:?}");
    assert!(upper.to_f64_up() >= 1.507e-9);
    // Within a small factor of the exact error (the stored-weight interval
    // is wider than the actual rounding).
    assert!(upper.to_f64_up() <= 1.0e-8, "{upper:?}");
}

#[test]
fn the_order_is_total_and_puts_zero_first() {
    let values = [
        ExpBound::ZERO,
        ExpBound::exact(f64::from_bits(1)).unwrap(),
        ExpBound {
            mantissa: 0.5,
            exponent: -5000,
        },
        ExpBound::exact(0.25).unwrap(),
        ExpBound::exact(1.0).unwrap(),
        ExpBound::exact(1.5).unwrap(),
        ExpBound {
            mantissa: 0.75,
            exponent: 5000,
        },
        // A hand-built unnormalized bound compares by value: 4 * 2^0 = 4.
        ExpBound {
            mantissa: 4.0,
            exponent: 0,
        },
        // Malformed bounds sort above every valid one.
        ExpBound {
            mantissa: -1.0,
            exponent: 0,
        },
        ExpBound {
            mantissa: f64::NAN,
            exponent: 0,
        },
    ];
    let value_of = |bound: &ExpBound| -> Option<f64> {
        (bound.mantissa.is_finite() && bound.mantissa >= 0.0).then(|| {
            if bound.mantissa == 0.0 {
                0.0
            } else {
                bound.mantissa.log2() + bound.exponent as f64
            }
        })
    };
    for a in &values {
        assert_eq!(a.total_cmp(a), Ordering::Equal);
        for b in &values {
            // Antisymmetry.
            assert_eq!(a.total_cmp(b), b.total_cmp(a).reverse(), "{a:?} {b:?}");
            if let (Some(x), Some(y)) = (value_of(a), value_of(b)) {
                if a.is_zero() || b.is_zero() {
                    assert_eq!(
                        a.total_cmp(b),
                        (!a.is_zero()).cmp(&!b.is_zero()),
                        "{a:?} {b:?}"
                    );
                } else {
                    assert_eq!(a.total_cmp(b), x.total_cmp(&y), "{a:?} {b:?}");
                }
            } else if value_of(a).is_some() {
                assert_eq!(a.total_cmp(b), Ordering::Less, "{a:?} {b:?}");
            }
            // Transitivity.
            for c in &values {
                if a.total_cmp(b) != Ordering::Greater && b.total_cmp(c) != Ordering::Greater {
                    assert_ne!(a.total_cmp(c), Ordering::Greater, "{a:?} {b:?} {c:?}");
                }
            }
        }
    }
    assert_eq!(ExpBound::ZERO.max_bound(values[2]), values[2]);
    assert_eq!(values[2].max_bound(ExpBound::ZERO), values[2]);
}

/// `n` as little-endian base-2^32 limbs times small factors.
fn big_mul(limbs: &mut Vec<u64>, factor: u64) {
    let mut carry = 0_u128;
    for limb in limbs.iter_mut() {
        let product = u128::from(*limb) * u128::from(factor) + carry;
        *limb = (product & 0xffff_ffff) as u64;
        carry = product >> 32;
    }
    while carry > 0 {
        limbs.push((carry & 0xffff_ffff) as u64);
        carry >>= 32;
    }
}

fn bit_length(limbs: &[u64]) -> i64 {
    let top = limbs.iter().rposition(|limb| *limb != 0).unwrap();
    32 * top as i64 + (64 - limbs[top].leading_zeros() as i64)
}

#[test]
fn reciprocal_factorials_enclose_exactly_and_tightly() {
    // R4-ARITH-02: 1/k! from the outward recurrence, checked in exact
    // integers: r = M 2^(e - 53) with M an integer, so r >= 1/k! iff
    // M k! >= 2^(53 - e), i.e. bit_length(M k!) > 53 - e (M k! is not a
    // power of two for k >= 3 unless equality, which is also accepted).
    let table = reciprocal_factorials_upper(MAX_TRANSFORM_ORDER).unwrap();
    assert_eq!(table.len(), MAX_TRANSFORM_ORDER + 1);
    let mut factorial = vec![1_u64];
    for (k, bound) in table.iter().enumerate() {
        if k > 0 {
            big_mul(&mut factorial, k as u64);
        }
        let integer = (bound.mantissa * 2.0_f64.powi(53)) as u64;
        assert_eq!(integer as f64, bound.mantissa * 2.0_f64.powi(53));
        let mut product = factorial.clone();
        big_mul(&mut product, integer);
        let target = 53 - bound.exponent;
        let length = bit_length(&product);
        // product >= 2^target  <=>  length >= target + 1.
        assert!(length > target, "k = {k}: {bound:?}");
        // Tight: product < 2^(target + 1), i.e. r < 2/k!.
        assert!(length <= target + 1, "k = {k}: {bound:?}");
    }
    for k in [4, 30, 50, 170, 171, 172] {
        assert!(!table[k].is_zero(), "k = {k}");
    }
}

#[test]
fn every_supported_order_of_the_zero_operator_is_enclosed() {
    let data = fixtures();
    let h = bits(&data["zero_operator_h"]);
    assert_eq!(
        data["supported_order"].as_u64().unwrap() as usize,
        MAX_TRANSFORM_ORDER
    );
    let zero = matrix(&[vec![0.0]]);
    let orders = data["zero_operator_orders"].as_array().unwrap();
    assert_eq!(orders.len(), MAX_TRANSFORM_ORDER + 1);
    // Every order p in one call per p would be quadratic; the bound is a
    // sum of per-order terms, so check each p on its own input b_p = [1].
    for row in orders {
        let p = row["order"].as_u64().unwrap() as usize;
        let mut input = vec![vec![0.0]; p + 1];
        input[p][0] = 1.0;
        let (stored, _) = weight_phi_vectors(h, &input).unwrap();
        assert_eq!(
            stored[p][0].to_bits(),
            bits(&row["stored"]).to_bits(),
            "p = {p}"
        );
        let result = bound_transform_error(&zero, h, &input, &stored).unwrap();
        assert_encloses(
            &format!("order {p}"),
            &result,
            exp_bound(&row["error_upper"]),
        );
    }
    // Above the declared domain: a typed rejection, not a bound.
    let input = vec![vec![0.0]; MAX_TRANSFORM_ORDER + 2];
    assert!(bound_transform_error(&zero, h, &input, &input).is_err());
}

#[test]
fn signed_stored_weights_and_the_tolerance_domain_fail_closed() {
    let zero = matrix(&[vec![0.0]]);
    // R4-ARITH-03: b0 = 1 stored as -1 is an error of 2.
    let result = bound_transform_error(&zero, 1.0, &[vec![1.0]], &[vec![-1.0]]).unwrap();
    let TransformBound::Bounded { upper, .. } = &result else {
        panic!("{result:?}");
    };
    assert!(upper.to_f64_up() >= 2.0);
    assert!(!result.admits(0.0));
    assert!(!result.admits(1.0));
    assert!(result.admits(2.0));
    // An error bound of 1 against negative, NaN and infinite tolerances.
    let one = bound_transform_error(&zero, 1.0, &[vec![1.0]], &[vec![0.0]]).unwrap();
    for tolerance in [
        -10.0,
        -0.0_f64.next_down(),
        f64::NAN,
        f64::INFINITY,
        -f64::INFINITY,
    ] {
        assert!(!one.admits(tolerance), "{tolerance}");
    }
    assert!(one.admits(1.0));
    assert!(!one.admits(1.0_f64.next_down()));
    // A zero tolerance admits exactly the zero error.
    let exact = bound_transform_error(&zero, 1.0, &[vec![1.0]], &[vec![1.0]]).unwrap();
    assert!(exact.admits(0.0));
    assert!(exact.admits(-0.0));
    assert!(!exact.admits(-1.0e-300));
    // Non-finite stored weights are rejected, not bounded.
    assert!(bound_transform_error(&zero, 1.0, &[vec![1.0]], &[vec![f64::NAN]]).is_err());
}
