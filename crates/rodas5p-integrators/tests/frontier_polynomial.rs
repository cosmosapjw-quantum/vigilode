use rodas5p_integrators::polynomial_resolvent::{
    DeclaredPolynomialResolvent as Plan, PolynomialResolventError as Error,
    PolynomialResolventWorkspace as Workspace,
};

#[test]
fn repeated_jordan_relation_uses_one_candidate_action_and_one_residual() {
    let plan = Plan::new(&[4.0, 4.0, 1.0], 0.0, 6.0, 1e-12).unwrap();
    let mut calls = 0;
    let mut out = [0.0; 2];
    let report = plan
        .apply_checked(
            &[1.0, 2.0],
            &mut out,
            &mut Workspace::new(2),
            1e-12,
            0.0,
            |x, y| {
                calls += 1;
                y[0] = -2.0 * x[0] + 4.0 * x[1];
                y[1] = -2.0 * x[1];
                Ok(())
            },
        )
        .unwrap();
    assert!(report.residual_accepted);
    assert!((out[0] - 11.0 / 9.0).abs() < 1e-14 && (out[1] - 2.0 / 3.0).abs() < 1e-14);
    assert_eq!(
        (
            report.candidate_operator_actions,
            report.residual_operator_actions,
            calls
        ),
        (1, 1, 2)
    );
}

#[test]
fn nilpotent_jordan_degree_four_is_exact_without_root_separation() {
    let plan = Plan::new(&[0.0, 0.0, 0.0, 0.0, 1.0], 0.0, 100.0, 1e-12).unwrap();
    let mut out = [0.0; 4];
    let report = plan
        .apply_checked(
            &[0.0, 0.0, 0.0, 1.0],
            &mut out,
            &mut Workspace::new(4),
            0.0,
            0.0,
            |x, y| {
                y[0] = 100.0 * x[1];
                y[1] = 100.0 * x[2];
                y[2] = 100.0 * x[3];
                y[3] = 0.0;
                Ok(())
            },
        )
        .unwrap();
    assert!(report.residual_accepted);
    assert_eq!(out, [1e6, 1e4, 1e2, 1.0]);
    assert_eq!(report.candidate_operator_actions, 3);
}

#[test]
fn wrong_probe_inferred_relation_is_caught_on_original_rhs() {
    // q(B)=B+2I vanishes on e1 but not on the whole Jordan block.
    assert_eq!(
        Plan::new(&[2.0, 1.0], 4.0, 6.0, 1e-12).unwrap_err(),
        Error::RelationDefect
    );
    let plan = Plan::new(&[2.0, 1.0], 0.0, 6.0, 1e-12).unwrap(); // dishonest witness
    let mut out = [77.0, 88.0];
    let r = plan
        .apply_checked(
            &[0.0, 1.0],
            &mut out,
            &mut Workspace::new(2),
            1e-12,
            0.0,
            |x, y| {
                y[0] = -2.0 * x[0] + 4.0 * x[1];
                y[1] = -2.0 * x[1];
                Ok(())
            },
        )
        .unwrap();
    assert!(!r.residual_accepted);
    assert!(r.original_residual_norm > 1.0);
    assert_eq!(out, [77.0, 88.0]);
}

#[test]
fn near_relation_bounds_state_error_when_norm_witness_is_true() {
    // q(z)=z+2; B=diag(-2,-1.9), ||q(B)||inf=0.1.
    let plan = Plan::new(&[2.0, 1.0], 0.1, 2.0, 1e-12).unwrap();
    let mut out = [0.0; 2];
    let r = plan
        .apply_checked(
            &[1.0, 1.0],
            &mut out,
            &mut Workspace::new(2),
            0.04,
            1e-15,
            |x, y| {
                y[0] = -2.0 * x[0];
                y[1] = -1.9 * x[1];
                Ok(())
            },
        )
        .unwrap();
    assert!(r.residual_accepted);
    let error = (out[1] - 1.0 / 2.9).abs();
    assert!(error <= r.conditional_error_estimate * (1.0 + 1e-13));
}

#[test]
fn poles_and_cancellation_are_rejected_separately_from_repeated_roots() {
    assert_eq!(
        Plan::new(&[-1.0, 1.0], 0.0, 1.0, 1e-12).unwrap_err(),
        Error::DenominatorMargin
    );
    assert_eq!(
        Plan::new(&[-1.0 + 1e-14, 1.0], 0.0, 1.0, 1e-12).unwrap_err(),
        Error::DenominatorMargin
    );
    assert!(Plan::new(&[1.0, 2.0, 1.0], 0.0, 2.0, 1e-12).is_ok());
}

#[test]
fn callback_failure_partial_write_and_nonfinite_are_transactional() {
    let plan = Plan::new(&[0.0, 0.0, 1.0], 0.0, 1.0, 1e-12).unwrap();
    let mut out = [7.0, 8.0];
    let mut ws = Workspace::new(2);
    assert_eq!(
        plan.apply_checked(&[1.0, 2.0], &mut out, &mut ws, 1e-12, 0.0, |_, _| Err(
            Error::OperatorFailure
        ))
        .unwrap_err(),
        Error::OperatorFailure
    );
    assert_eq!(out, [7.0, 8.0]);
    assert_eq!(
        plan.apply_checked(&[1.0, 2.0], &mut out, &mut ws, 1e-12, 0.0, |_, y| {
            y[0] = 0.0;
            Ok(())
        })
        .unwrap_err(),
        Error::NonFinite
    );
    assert_eq!(out, [7.0, 8.0]);
}

#[test]
fn invalid_degree_and_underflow_do_not_silently_change_relation() {
    assert!(Plan::new(&[1.0, 0.0], 0.0, 1.0, 1e-12).is_err());
    assert!(Plan::new(&[1e308, 1e-308], 0.0, 1.0, 1e-12).is_err());
    assert!(Plan::new(&[1.0, 1.0], f64::NAN, 1.0, 1e-12).is_err());
    assert_eq!(
        Plan::new(&[1.0, 1.0], 0.0, 1.0, 1.0).unwrap_err(),
        Error::InvalidInput
    );
}

#[test]
fn scale_invariance_and_workspace_reuse() {
    let a = Plan::new(&[4.0, 4.0, 1.0], 0.0, 6.0, 1e-12).unwrap();
    let b = Plan::new(&[4e200, 4e200, 1e200], 0.0, 6.0, 1e-12).unwrap();
    let mut ws = Workspace::new(2);
    let mut x = [0.0; 2];
    let mut y = [0.0; 2];
    for rhs in [[1.0, 2.0], [5.0, -3.0]] {
        for (p, out) in [(&a, &mut x), (&b, &mut y)] {
            assert!(
                p.apply_checked(&rhs, out, &mut ws, 1e-12, 0.0, |v, w| {
                    w[0] = -2.0 * v[0] + 4.0 * v[1];
                    w[1] = -2.0 * v[1];
                    Ok(())
                })
                .unwrap()
                .residual_accepted
            );
        }
        assert_eq!(x, y);
    }
}
