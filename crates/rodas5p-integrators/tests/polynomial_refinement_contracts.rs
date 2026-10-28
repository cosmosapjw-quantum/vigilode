use rodas5p_integrators::polynomial_resolvent::{
    DeclaredPolynomialResolvent as Plan, PolynomialRefinementPolicy as Policy,
    PolynomialRefinementStop as Stop, PolynomialRefinementWork as Work,
    PolynomialRefinementWorkspace as Workspace, PolynomialResolventError as Error,
};

#[test]
fn approximate_annihilator_now_reaches_tight_accuracy() {
    // q(z)=z+100, B=diag(-100,-99): ||q(B)||=1, eta=1/101.
    // The old one-shot polynomial leaves a 1/101 residual; refinement rescues it.
    let plan = Plan::new(&[100.0, 1.0], 1.0, 100.0, 1e-12).unwrap();
    let mut ws = Workspace::new(2);
    let mut out = [17.0, 18.0];
    let mut work = Work::default();
    let report = plan
        .apply_refined_checked(
            &[1.0, 1.0],
            &mut out,
            &mut ws,
            Policy::default(),
            1e-12,
            0.0,
            |x, y| {
                y[0] = -100.0 * x[0];
                y[1] = -99.0 * x[1];
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert!(report.accepted);
    assert_eq!(report.stop, Stop::Accepted);
    assert!(report.corrections > 0 && report.corrections <= 6);
    assert!((out[0] - 1.0 / 101.0).abs() < 1e-15);
    assert!((out[1] - 0.01).abs() < 1e-14);
    assert_eq!(
        work.operator_actions_attempted(),
        1 + report.corrections as u64
    );
    assert_eq!(
        work.operator_actions_completed,
        work.operator_actions_attempted()
    );
}

#[test]
fn nonnormal_defect_correction_is_not_unpreconditioned_neumann() {
    // B=-100I+40N, N²=0. Even though ||B||=140, eta=40/101<1.
    let plan = Plan::new(&[100.0, 1.0], 40.0, 140.0, 1e-12).unwrap();
    let mut out = [0.0; 2];
    let mut work = Work::default();
    let r = plan
        .apply_refined_checked(
            &[0.0, 1.0],
            &mut out,
            &mut Workspace::new(2),
            Policy::default(),
            1e-13,
            0.0,
            |x, y| {
                y[0] = -100.0 * x[0] + 40.0 * x[1];
                y[1] = -100.0 * x[1];
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert!(r.accepted);
    assert_eq!(r.corrections, 1);
    assert!((out[0] - 40.0 / 10201.0).abs() < 1e-16);
    assert_eq!(work.operator_actions_attempted(), 2);
}

#[test]
fn exact_repeated_root_keeps_single_polynomial_candidate() {
    let plan = Plan::new(&[4.0, 4.0, 1.0], 0.0, 6.0, 1e-12).unwrap();
    let mut out = [0.0; 2];
    let mut work = Work::default();
    let r = plan
        .apply_refined_checked(
            &[1.0, 2.0],
            &mut out,
            &mut Workspace::new(2),
            Policy::default(),
            1e-13,
            0.0,
            |x, y| {
                y[0] = -2.0 * x[0] + 4.0 * x[1];
                y[1] = -2.0 * x[1];
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert!(r.accepted);
    assert_eq!(r.corrections, 0);
    assert_eq!(work.operator_actions_attempted(), 2);
}

#[test]
fn hard_budget_and_fallback_use_same_tolerance_and_record_all_calls() {
    let plan = Plan::new(&[100.0, 1.0], 1.0, 100.0, 1e-12).unwrap();
    let mut out = [7.0; 2];
    let mut work = Work::default();
    let r = plan
        .apply_refined_or_fallback(
            &[1.0, 1.0],
            &mut out,
            &mut Workspace::new(2),
            Policy {
                max_operator_actions: 1,
                ..Policy::default()
            },
            1e-12,
            0.0,
            |x, y| {
                y[0] = -100.0 * x[0];
                y[1] = -99.0 * x[1];
                Ok(())
            },
            |b, x| {
                x[0] = b[0] / 101.0;
                x[1] = b[1] / 100.0;
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert!(r.accepted);
    assert_eq!(r.stop, Stop::FallbackAccepted);
    assert_eq!(r.fallback_trigger, Some(Stop::ActionBudget));
    assert_eq!(work.operator_actions_attempted(), 2);
    assert_eq!(work.fallback_calls, 1);
}

#[test]
fn relation_decline_costs_no_polynomial_actions_and_fallback_can_fail_safely() {
    let plan = Plan::new(&[2.0, 1.0], 2.0, 2.0, 1e-12).unwrap();
    let mut out = [7.0, 8.0];
    let mut work = Work::default();
    let r = plan
        .apply_refined_or_fallback(
            &[1.0, 1.0],
            &mut out,
            &mut Workspace::new(2),
            Policy::default(),
            1e-12,
            0.0,
            |x, y| {
                y[0] = -2.0 * x[0];
                y[1] = 0.0;
                Ok(())
            },
            |_, x| {
                x.fill(0.0);
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert!(!r.accepted);
    assert_eq!(r.stop, Stop::FallbackResidualRejected);
    assert_eq!(r.fallback_trigger, Some(Stop::RelationTooWeak));
    assert_eq!(out, [7.0, 8.0]);
    assert_eq!(work.operator_actions_attempted(), 1);
}

#[test]
fn dishonest_witness_stagnation_refuses_without_committing() {
    let plan = Plan::new(&[2.0, 1.0], 0.0, 2.0, 1e-12).unwrap();
    // Actual B=4I contradicts the declaration; r grows by a factor 2.
    let mut out = [7.0, 8.0];
    let mut work = Work::default();
    let r = plan
        .apply_refined_checked(
            &[1.0, 1.0],
            &mut out,
            &mut Workspace::new(2),
            Policy::default(),
            1e-12,
            0.0,
            |x, y| {
                y[0] = 4.0 * x[0];
                y[1] = 4.0 * x[1];
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert!(!r.accepted);
    assert_eq!(r.stop, Stop::Stagnation);
    assert_eq!(r.corrections, 1);
    assert_eq!(out, [7.0, 8.0]);
}

#[test]
fn callback_failures_are_paid_and_never_hidden_by_fallback() {
    let plan = Plan::new(&[2.0, 1.0], 0.0, 2.0, 1e-12).unwrap();
    for partial in [false, true] {
        let mut out = [7.0, 8.0];
        let mut work = Work::default();
        let e = plan
            .apply_refined_or_fallback(
                &[1.0, 1.0],
                &mut out,
                &mut Workspace::new(2),
                Policy::default(),
                1e-12,
                0.0,
                |_, y| {
                    if partial {
                        y[0] = 0.0;
                        Ok(())
                    } else {
                        Err(Error::OperatorFailure)
                    }
                },
                |_, _| panic!("fallback must not hide callback failures"),
                &mut work,
            )
            .unwrap_err();
        assert_eq!(
            e,
            if partial {
                Error::NonFinite
            } else {
                Error::OperatorFailure
            }
        );
        assert_eq!(out, [7.0, 8.0]);
        assert_eq!(work.operator_actions_attempted(), 1);
        assert_eq!(work.operator_actions_completed, 0);
        assert_eq!(work.fallback_calls, 0);
    }
}

#[test]
fn zero_budget_invalid_policy_and_roundoff_allowance_are_not_silent_success() {
    let plan = Plan::new(&[1.0, 1.0], 0.0, 1.0, 1e-12).unwrap();
    let mut out = [9.0];
    let mut work = Work::default();
    let mut ws = Workspace::new(1);
    let r = plan
        .apply_refined_checked(
            &[1.0],
            &mut out,
            &mut ws,
            Policy {
                max_operator_actions: 0,
                ..Policy::default()
            },
            0.0,
            0.0,
            |_, _| panic!("budget preflight must not evaluate"),
            &mut work,
        )
        .unwrap();
    assert_eq!(r.stop, Stop::ActionBudget);
    assert_eq!(out, [9.0]);
    assert!(
        plan.apply_refined_checked(
            &[1.0],
            &mut out,
            &mut ws,
            Policy {
                stagnation_ratio: f64::NAN,
                ..Policy::default()
            },
            0.0,
            0.0,
            |_, _| panic!("invalid preflight must not evaluate"),
            &mut work
        )
        .is_err()
    );
    let r = plan
        .apply_refined_checked(
            &[1.0],
            &mut out,
            &mut ws,
            Policy {
                max_corrections: 0,
                ..Policy::default()
            },
            1e-14,
            1e-12,
            |x, y| {
                y[0] = -x[0];
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert!(!r.accepted);
    assert_eq!(out, [9.0]);
}

#[test]
fn near_pole_stays_outside_quotient_chart() {
    assert_eq!(
        Plan::new(&[-1.0 + 1e-14, 1.0], 0.0, 1.0, 1e-12).unwrap_err(),
        Error::DenominatorMargin
    );
}

#[test]
fn horner_failure_and_fallback_partial_write_preserve_caller_output() {
    let plan = Plan::new(&[4.0, 4.0, 1.0], 0.0, 6.0, 1e-12).unwrap();
    for partial in [false, true] {
        let mut out = [7.0, 8.0];
        let mut work = Work::default();
        let e = plan
            .apply_refined_checked(
                &[1.0, 1.0],
                &mut out,
                &mut Workspace::new(2),
                Policy::default(),
                1e-12,
                0.0,
                |_, y| {
                    if partial {
                        y[0] = 0.0;
                        Ok(())
                    } else {
                        Err(Error::OperatorFailure)
                    }
                },
                &mut work,
            )
            .unwrap_err();
        assert_eq!(
            e,
            if partial {
                Error::NonFinite
            } else {
                Error::OperatorFailure
            }
        );
        assert_eq!(out, [7.0, 8.0]);
        assert_eq!(work.polynomial_actions_attempted, 1);
        assert_eq!(work.residual_actions_attempted, 0);
        assert_eq!(work.operator_actions_completed, 0);
        let e = plan
            .apply_refined_or_fallback(
                &[1.0, 1.0],
                &mut out,
                &mut Workspace::new(2),
                Policy {
                    max_operator_actions: 0,
                    ..Policy::default()
                },
                1e-12,
                0.0,
                |_, _| panic!("fallback fails before its residual check"),
                |_, y| {
                    if partial {
                        y[0] = 0.0;
                        Ok(())
                    } else {
                        Err(Error::OperatorFailure)
                    }
                },
                &mut work,
            )
            .unwrap_err();
        assert_eq!(
            e,
            if partial {
                Error::NonFinite
            } else {
                Error::OperatorFailure
            }
        );
        assert_eq!(out, [7.0, 8.0]);
        assert_eq!(work.fallback_calls, 1);
        assert_eq!(work.operator_actions_attempted(), 1);
    }
}
