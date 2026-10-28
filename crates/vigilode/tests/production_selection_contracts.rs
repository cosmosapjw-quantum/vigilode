use vigilode::*;

fn stiff_policy() -> GeneralOdePolicy {
    GeneralOdePolicy {
        regime: ProblemRegime::Stiff,
        derivatives: DerivativeFallback::ProvidedOnly,
        ..GeneralOdePolicy::default()
    }
}

fn decay() -> Problem {
    Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -1.0;
        Ok(())
    }))
    .autonomous()
}

#[test]
fn default_is_rhs_only_even_with_provided_derivatives() {
    let p = Problem::new(1, |_, y, out| {
        out[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("unexpected Jacobian")))
    .time_derivative(|_, _, _| panic!("unexpected dfdt"));
    let s = GeneralOde::solve(
        &p,
        (0.0, 1.0),
        &[1.0],
        &Options::default(),
        &GeneralOdePolicy::default(),
    )
    .unwrap()
    .ensure_success()
    .unwrap();
    assert_eq!(s.selection.method, ProductionMethod::Dopri5);
    assert!((s.final_state().unwrap()[0] - (-1.0_f64).exp()).abs() < 2e-6);
    let GeneralResult::Explicit(s) = s.result else {
        panic!("wrong result")
    };
    assert_eq!(
        s.stats.counters.jacobian_builds + s.stats.counters.linear_solves,
        0
    );
}

#[test]
fn unsupported_requests_and_permissions_fail_without_callbacks() {
    let p = Problem::new(1, |_, _, _| panic!("selection entered RHS"));
    let opts = Options::default();
    let mut policy = stiff_policy();
    policy.derivatives = DerivativeFallback::Forbidden;
    assert!(matches!(
        GeneralOde::select(&p, &opts, &policy),
        Err(SelectionRefusal::DerivativesForbidden)
    ));
    policy.derivatives = DerivativeFallback::ProvidedOnly;
    assert!(matches!(
        GeneralOde::select(&p, &opts, &policy),
        Err(SelectionRefusal::MissingJacobian)
    ));
    let p = p.jacobian(Jacobian::dense(|_, _, _| panic!("selection entered J")));
    assert!(matches!(
        GeneralOde::select(&p, &opts, &policy),
        Err(SelectionRefusal::MissingTimeDerivative)
    ));
    let p = p.autonomous();
    assert_eq!(
        GeneralOde::select(&p, &opts, &policy).unwrap().method,
        ProductionMethod::Rodas5p
    );
    assert!(matches!(
        GeneralOde::select(&p.clone().mass(vec![1.0]), &opts, &policy),
        Err(SelectionRefusal::MassMatrix)
    ));
    assert!(matches!(
        GeneralOde::select(
            &p.clone().event(|_, _| 1.0, Direction::Either),
            &opts,
            &policy
        ),
        Err(SelectionRefusal::Events)
    ));
    assert!(matches!(
        GeneralOde::select(
            &p,
            &Options {
                dense_output: true,
                ..opts
            },
            &policy
        ),
        Err(SelectionRefusal::DenseOutput)
    ));
}

#[test]
fn resource_ceiling_includes_dense_callbacks_with_declared_band() {
    let p = Problem::new(65, |_, _, _| panic!("RHS"))
        .jacobian(Jacobian::dense(|_, _, _| panic!("J")))
        .autonomous();
    for p in [p.clone(), p.banded(0, 0)] {
        assert!(matches!(
            GeneralOde::select(&p, &Options::default(), &stiff_policy()),
            Err(SelectionRefusal::DenseDimensionLimit)
        ));
    }
    let p = Problem::new(65, |_, _, _| panic!("RHS"))
        .banded(0, 0)
        .autonomous();
    let policy = GeneralOdePolicy {
        derivatives: DerivativeFallback::AllowFiniteDifference,
        ..stiff_policy()
    };
    assert_eq!(
        GeneralOde::select(&p, &Options::default(), &policy)
            .unwrap()
            .implicit_route,
        Some(FacadeRoute::BandedFast)
    );
}

#[test]
fn matrix_free_is_an_explicit_permission_and_inherits_atol_refusal() {
    let p = Problem::new(2, |_, _, _| panic!("RHS"))
        .jacobian(Jacobian::jvp(|_, _, _, _| panic!("JVP")))
        .autonomous();
    let mut policy = stiff_policy();
    assert!(matches!(
        GeneralOde::select(&p, &Options::default(), &policy),
        Err(SelectionRefusal::MatrixFreeNotEnabled)
    ));
    policy.allow_matrix_free = true;
    assert_eq!(
        GeneralOde::select(&p, &Options::default(), &policy)
            .unwrap()
            .implicit_route,
        Some(FacadeRoute::MatrixFree)
    );
    let opts = Options {
        atol: Atol::Vector(vec![1e-9; 2]),
        ..Options::default()
    };
    assert!(matches!(
        GeneralOde::select(&p, &opts, &policy),
        Err(SelectionRefusal::Implicit(
            Unsupported::VectorAtolOnMatrixFreeRoute
        ))
    ));
}

#[test]
fn selected_implicit_method_matches_direct_method_and_fd_is_opt_in() {
    let opts = Options::default();
    let p = decay();
    let selected = GeneralOde::solve(&p, (0.0, 1.0), &[1.0], &opts, &stiff_policy())
        .unwrap()
        .ensure_success()
        .unwrap();
    let direct = Rodas5p::solve(&p, (0.0, 1.0), &[1.0], &opts).unwrap();
    assert_eq!(selected.final_state(), direct.final_state());
    assert!((selected.final_state().unwrap()[0] - (-1.0_f64).exp()).abs() < 2e-6);
    let p = Problem::new(1, |t, y, out| {
        out[0] = -1000.0 * (y[0] - t.cos()) - t.sin();
        Ok(())
    });
    let policy = GeneralOdePolicy {
        derivatives: DerivativeFallback::AllowFiniteDifference,
        ..stiff_policy()
    };
    let s = GeneralOde::solve(&p, (0.0, 1.0), &[1.0], &opts, &policy)
        .unwrap()
        .ensure_success()
        .unwrap();
    assert!((s.final_state().unwrap()[0] - 1.0_f64.cos()).abs() < 1e-5);
}

#[test]
fn failure_preserves_partial_state_and_typed_error() {
    let opts = Options {
        max_steps: 1,
        h0: 1e-6,
        ..Options::default()
    };
    for policy in [GeneralOdePolicy::default(), stiff_policy()] {
        let s = GeneralOde::solve(&decay(), (0.0, 1.0), &[1.0], &opts, &policy).unwrap();
        assert!(!s.is_success());
        assert!(s.final_state().unwrap()[0].is_finite());
        match s.ensure_success().unwrap_err() {
            GeneralSolveError::Explicit(ExplicitSolveError::Failed { partial, .. }) => {
                assert!(!partial.success)
            }
            GeneralSolveError::Implicit(SolveError::Failed { partial, .. }) => {
                assert!(!partial.is_success())
            }
            e => panic!("lost partial result: {e}"),
        }
    }
}
