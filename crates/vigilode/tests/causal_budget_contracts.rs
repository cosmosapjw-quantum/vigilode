use vigilode::{
    CausalBudgetError, CausalBudgetInput, CausalTargetKey, SessionEpochs, assess_causal_budget,
    causal_residual_allowances,
};

fn key() -> CausalTargetKey {
    CausalTargetKey::new(SessionEpochs::default(), 3, 7, 11, 0.0, 0.125).unwrap()
}
fn input<'a>(b: &'a [f64], d: &'a [f64], w: &'a [f64], budget: f64) -> CausalBudgetInput<'a> {
    CausalBudgetInput {
        expected: key(),
        observed: key(),
        lambda: 1.0,
        propagation: b,
        residual_seeds: d,
        output_weights: w,
        reconstruction_error: 0.0,
        output_budget: budget,
    }
}

#[test]
fn noncontractive_chain_bound_is_finite_and_sharp() {
    let b = [
        0., 0., 0., 0., 2., 0., 0., 0., 0., 2., 0., 0., 0., 0., 2., 0.,
    ];
    let mut e = [0.; 4];
    // Exact dyadic linear recurrence saturates the bound. ||B||_infty=2.
    let r = assess_causal_budget(&input(&b, &[0.125; 4], &[0., 0., 0., 1.], 2.), &mut e).unwrap();
    assert_eq!(e, [0.125, 0.375, 0.875, 1.875]);
    assert_eq!(r.output_error_upper, 1.875);
    assert!(r.admitted);
    // A max-residual-only screen would accept 0.125 < 1; true amplification does not.
    assert!(
        !assess_causal_budget(&input(&b, &[0.125; 4], &[0., 0., 0., 1.], 1.), &mut e)
            .unwrap()
            .admitted
    );
}

#[test]
fn adjoint_weighted_allowances_fit_original_budget() {
    let b = [0., 0., 0., 2., 0., 0., 0., 4., 0.];
    let w = [0., 0., 1.];
    let mut v = [0.; 3];
    let mut d = [0.; 3];
    causal_residual_allowances(&b, &w, 1., &mut v, &mut d).unwrap();
    assert_eq!(v, [8., 4., 1.]);
    let r = assess_causal_budget(&input(&b, &d, &w, 1.), &mut [0.; 3]).unwrap();
    assert!(r.admitted);
    assert!(d[0] < d[1] && d[1] < d[2]);
    // Independent exact recurrence output =8*d0+4*d1+d2.
    assert!(r.output_error_upper >= 8. * d[0] + 4. * d[1] + d[2]);
}

#[test]
fn intermediate_homotopy_root_never_admits_as_original_target() {
    let b = [0.];
    let d = [0.];
    let w = [1.];
    let mut i = input(&b, &d, &w, 1.);
    for lambda in [0., 0.5, 1.0_f64.next_down(), f64::NAN] {
        i.lambda = lambda;
        let mut e = [99.];
        assert!(matches!(
            assess_causal_budget(&i, &mut e),
            Err(CausalBudgetError::NotOriginalTarget)
        ));
        assert_eq!(e, [99.]);
    }
}

#[test]
fn all_context_changes_invalidate_before_workspace_write() {
    let b = [0.];
    let d = [0.];
    let w = [1.];
    let original = key();
    let mut mutations = vec![];
    let mut k = original;
    k.epochs.model += 1;
    mutations.push(k);
    let mut k = original;
    k.epochs.provider += 1;
    mutations.push(k);
    let mut k = original;
    k.epochs.norm += 1;
    mutations.push(k);
    let mut k = original;
    k.epochs.tolerance += 1;
    mutations.push(k);
    let mut k = original;
    k.chart += 1;
    mutations.push(k);
    let mut k = original;
    k.operator += 1;
    mutations.push(k);
    let mut k = original;
    k.state += 1;
    mutations.push(k);
    let mut k = original;
    k.time_bits = 0.125_f64.to_bits();
    mutations.push(k);
    let mut k = original;
    k.step_bits = 0.0625_f64.to_bits();
    mutations.push(k);
    for changed in mutations {
        let mut i = input(&b, &d, &w, 1.);
        i.observed = changed;
        let mut e = [99.];
        assert!(matches!(
            assess_causal_budget(&i, &mut e),
            Err(CausalBudgetError::StaleTarget)
        ));
        assert_eq!(e, [99.]);
    }
}

#[test]
fn upper_diagonal_negative_and_nonfinite_entries_refused() {
    for b in [
        [0., 0.01, 0., 0.],
        [0., 0., -1., 0.],
        [0., 0., f64::NAN, 0.],
        [0., 0., 1., f64::MIN_POSITIVE],
    ] {
        let mut e = [99.; 2];
        assert!(assess_causal_budget(&input(&b, &[0., 0.], &[1., 1.], 1.), &mut e).is_err());
        assert_eq!(e, [99.; 2]);
    }
    assert!(assess_causal_budget(&input(&[0.], &[0.], &[1.], f64::INFINITY), &mut [0.]).is_err());
}

#[test]
fn reconstruction_and_directed_boundary_are_counted() {
    let b = [0.];
    let d = [0.1];
    let w = [1.];
    let mut i = input(&b, &d, &w, 0.3);
    i.reconstruction_error = 0.2;
    let r = assess_causal_budget(&i, &mut [0.]).unwrap();
    assert_eq!(r.output_error_upper, 0.30000000000000004);
    assert!(!r.admitted);
}

#[test]
fn zero_influence_and_tiny_allowances_remain_finite() {
    let mut v = [0.; 2];
    let mut d = [0.; 2];
    causal_residual_allowances(&[0.; 4], &[0., 2.], f64::from_bits(1), &mut v, &mut d).unwrap();
    assert_eq!(d, [0., 0.]);
    assert_eq!(v, [0., 2.]);
    causal_residual_allowances(&[0.; 4], &[0., 0.], 1., &mut v, &mut d).unwrap();
    assert_eq!(d, [0., 0.]);
}

#[test]
fn overflow_fails_closed_and_invalid_clock_is_refused() {
    assert!(
        assess_causal_budget(
            &input(&[0., 0., f64::MAX, 0.], &[2., 0.], &[0., 1.], f64::MAX),
            &mut [0.; 2]
        )
        .is_err()
    );
    assert!(CausalTargetKey::new(SessionEpochs::default(), 0, 0, 0, 1e30, 1.).is_err());
    let b = [0.];
    let d = [0.];
    let w = [1.];
    let mut i = input(&b, &d, &w, 1.);
    i.expected.step_bits = 0;
    i.observed = i.expected;
    assert!(assess_causal_budget(&i, &mut [0.]).is_err());
}
