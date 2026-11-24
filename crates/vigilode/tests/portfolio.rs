use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use vigilode::{
    CallbackError, CapabilitySet, GeneralOdePolicy, Jacobian, MethodAutoPolicy, MethodChoice,
    MethodFallback, MethodRegistry, Options, Problem,
};

#[test]
fn fatal_callback_ends_the_registry_fallback_chain() {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    let problem = Problem::new(1, move |_, y, out| {
        if counter.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(CallbackError::Fatal("stop at first evaluation".into()));
        }
        out[0] = -y[0];
        Ok(())
    })
    .autonomous()
    .jacobian(Jacobian::dense(|_, _, jac| {
        jac[0] = -1.0;
        Ok(())
    }));
    let policy = MethodAutoPolicy {
        allowed: CapabilitySet {
            explicit_jacobian: true,
            dense_matrix: true,
            ..CapabilitySet::NONE
        },
        fallback: MethodFallback::NextAdmitted { max_attempts: 2 },
        ..MethodAutoPolicy::from_general(&problem, &GeneralOdePolicy::default())
    };
    let opts = Options::default();
    let plan = MethodRegistry::route(&problem, &opts, &MethodChoice::Auto(policy)).unwrap();
    assert_eq!(plan.fallbacks.len(), 1);
    let run = MethodRegistry::solve(&problem, (0., 0.1), &[1.], &opts, &plan).unwrap();
    assert_eq!(run.attempts().len(), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert!(!run.is_success());
}

#[test]
fn rhs_only_bdf_and_radau_expose_their_fd_and_newton_work() {
    use vigilode::{MethodParams, MethodResult, VigilODE};
    for id in ["bdf1-bdf2-variable", "radau-iia3"] {
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        let problem = Problem::new(2, move |_, y, f| {
            counter.fetch_add(1, Ordering::Relaxed);
            f[0] = -y[0];
            f[1] = -300.0 * y[1];
            Ok(())
        })
        .autonomous();
        let sol = VigilODE::solve_method(
            &problem,
            (0.0, 0.04),
            &[1.0, 1.0],
            &Options::default(),
            MethodChoice::Explicit(id),
            &MethodParams::default(),
        )
        .unwrap();
        let view = sol.view();
        assert!(view.is_success(), "{id}: {:?}", view.failure());
        assert_eq!(view.route().variant, "finite-difference-dense-newton");
        assert!(
            view.route().needs.finite_difference
                && view.route().needs.dense_matrix
                && view.route().needs.newton
        );
        assert!(!view.route().needs.explicit_jacobian && !view.route().needs.provided_jvp);
        let MethodResult::Observed(native) = view.native() else {
            panic!("expected observed solver");
        };
        assert!(native.finite_difference.jacobian_builds > 0);
        assert_eq!(
            native.finite_difference.rhs_calls,
            3 * native.finite_difference.jacobian_builds
        );
        assert_eq!(
            native.callbacks_entered.rhs,
            hits.load(Ordering::Relaxed) as u64
        );
        assert_eq!(native.counters.rhs_calls, native.callbacks_entered.rhs);
        assert_eq!(native.counters.rhs_evaluations, native.counters.rhs_calls);
        assert_eq!(native.callbacks_entered.jacobian, 0);
        let end = view.final_state().unwrap();
        let error = (end[0] - (-0.04_f64).exp())
            .abs()
            .max((end[1] - (-12.0_f64).exp()).abs());
        assert!(error < 5e-5, "{id} endpoint error = {error:e}");
        println!(
            "{id} max_abs={error:e} rhs={} fd_rhs={} jac={} newton={}",
            native.counters.rhs_calls,
            native.finite_difference.rhs_calls,
            native.finite_difference.jacobian_builds,
            native.counters.nonlinear_iterations
        );
    }
}

#[test]
fn portfolio_preserves_typed_preflight_refusal() {
    use vigilode::{MethodParams, MethodRouteRefusal, PortfolioError, VigilODE};
    let problem = Problem::new(1, |_, _, _| panic!("routing invoked user code"));
    let error = VigilODE::solve_method(
        &problem,
        (0.0, 1.0),
        &[1.0],
        &Options::default(),
        MethodChoice::Explicit("unregistered"),
        &MethodParams::default(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        PortfolioError::Route(MethodRouteRefusal::UnknownMethod { .. })
    ));
}

#[test]
fn a_declared_lawson_residual_cannot_be_integrated_as_the_full_rhs() {
    use vigilode::{
        DampedRotationBlock, DampedRotationFlow, MethodParams, PortfolioError, VigilODE,
    };
    let flow = DampedRotationFlow::repeated(
        DampedRotationBlock {
            damping: 1.0,
            frequency: 3.0,
        },
        1,
    )
    .unwrap();
    let residual = Problem::new(2, |_, _, _| panic!("preflight invoked the residual"));
    let params = MethodParams {
        lawson_flow: Some(flow),
        ..MethodParams::default()
    };
    let error = VigilODE::solve_method(
        &residual,
        (0., 1.),
        &[1., 0.],
        &Options::default(),
        MethodChoice::Explicit("rodas5p"),
        &params,
    )
    .unwrap_err();
    assert!(matches!(error, PortfolioError::SplitRequiresLawson));
}
