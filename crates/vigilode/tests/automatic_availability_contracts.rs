use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::*;

fn supplied_only() -> AutomaticOdePolicy {
    AutomaticOdePolicy {
        derivatives: DerivativeFallback::ProvidedOnly,
        ..Default::default()
    }
}

fn assert_explicit_only(run: &SwitchingSolution) {
    assert!(run.success, "{}", run.message);
    assert_eq!(run.config.fallback, DerivativeFallback::Forbidden);
    assert_eq!(run.stats.switches, 0);
    assert_eq!(run.stats.jacobian_builds, 0);
    assert_eq!(run.stats.jvp_vectors, 0);
    assert_eq!(run.stats.ft_calls, 0);
    assert_eq!(run.stats.linear_solves, 0);
    assert!(run.stats.accounting_complete);
    assert_eq!(run.stats.segments.len(), 1);
    assert_eq!(run.stats.segments[0].method, SwitchingMethod::Dopri5);
    assert!(!run.stats.segments[0].is_probe);
}

#[test]
fn missing_optional_provider_runs_rhs_once_and_preserves_window_observations() {
    for adaptive in [false, true] {
        let calls = Arc::new(AtomicU64::new(0));
        let callback_calls = calls.clone();
        let p = Problem::new(1, move |_, y, dy| {
            callback_calls.fetch_add(1, Ordering::Relaxed);
            dy[0] = -y[0];
            Ok(())
        });
        let solve = if adaptive {
            AutomaticOde::solve_adaptive
        } else {
            AutomaticOde::solve
        };
        let run = solve(
            &p,
            (0.0, 1.0),
            &[1.0],
            &Options::default(),
            &supplied_only(),
        )
        .unwrap();
        assert_explicit_only(&run);
        assert_eq!(calls.load(Ordering::Relaxed), run.stats.rhs_calls);
        assert_eq!(run.t.len(), 9);
        assert_eq!(run.y.len(), 9);
        for (i, (&t, y)) in run.t.iter().zip(&run.y).enumerate() {
            assert_eq!(t, i as f64 / 8.0);
            assert!((y[0] - (-t).exp()).abs() < 1e-6);
        }
        assert!(
            run.message
                .contains("ProvidedOnly requires a supplied Jacobian or JVP")
        );
        assert!(
            run.stats.segments[0]
                .decision
                .contains("no authorized Jacobian/JVP")
        );
    }
}

#[test]
fn missing_optional_time_derivative_does_not_force_replay_or_call_jacobian() {
    let p = Problem::new(1, |t, _, dy| {
        dy[0] = t;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("ineligible challenger")));
    let run = AutomaticOde::solve(
        &p,
        (0.0, 1.0),
        &[0.0],
        &Options::default(),
        &supplied_only(),
    )
    .unwrap();
    assert_explicit_only(&run);
    assert!((run.final_state[0] - 0.5).abs() < 1e-10);
    assert!(run.message.contains("time derivative"));
}

#[test]
fn unsupported_optional_vector_atol_route_keeps_the_original_component_tolerances() {
    let p = Problem::new(2, |_, y, dy| {
        dy[0] = -y[0];
        dy[1] = -2.0 * y[1];
        Ok(())
    })
    .jacobian(Jacobian::jvp(|_, _, _, _| {
        panic!("vector atol not supported by challenger")
    }))
    .autonomous();
    let opts = Options {
        atol: Atol::Vector(vec![1e-9, 1e-12]),
        ..Default::default()
    };
    let policy = AutomaticOdePolicy {
        allow_matrix_free: true,
        ..supplied_only()
    };
    let run = AutomaticOde::solve(&p, (0.0, 1.0), &[1.0, 1.0], &opts, &policy).unwrap();
    assert_explicit_only(&run);
    assert_eq!(run.options.atol, opts.atol);
    assert!(
        run.stats.segments[0]
            .decision
            .contains("cannot preserve vector atol")
    );
    assert!((run.final_state[1] - (-2.0_f64).exp()).abs() < 1e-6);
}

#[test]
fn default_rhs_only_route_matches_one_native_observed_run_exactly() {
    let p = Problem::new(1, |_, y, dy| {
        dy[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("derivatives forbidden")))
    .time_derivative(|_, _, _| panic!("derivatives forbidden"));
    let opts = Options::default();
    let times: Vec<_> = (0..=8).map(|i| i as f64 / 8.0).collect();
    let native = Dopri5::solve_observed(&p, (0.0, 1.0), &[1.0], &opts, &times).unwrap();
    let run = AutomaticOde::solve(
        &p,
        (0.0, 1.0),
        &[1.0],
        &opts,
        &AutomaticOdePolicy::default(),
    )
    .unwrap();
    assert_explicit_only(&run);
    assert_eq!(run.t, native.t);
    assert_eq!(run.y, native.y);
    assert_eq!(run.final_state, native.final_state);
    assert_eq!(run.stats.attempts, native.stats.attempts);
    assert_eq!(run.stats.rhs_calls, native.stats.counters.rhs_calls);
}

#[test]
fn global_feature_requests_are_refused_before_any_callback() {
    let rhs =
        |_: f64, _: &[f64], _: &mut [f64]| -> Result<(), CallbackError> { panic!("preflight") };
    let policy = supplied_only();
    for (problem, options, expected) in [
        (
            Problem::new(1, rhs).mass(vec![1.0]),
            Options::default(),
            SelectionRefusal::MassMatrix,
        ),
        (
            Problem::new(1, rhs).event(|_, _| panic!("event preflight"), Direction::Either),
            Options::default(),
            SelectionRefusal::Events,
        ),
        (
            Problem::new(1, rhs),
            Options {
                dense_output: true,
                ..Default::default()
            },
            SelectionRefusal::DenseOutput,
        ),
    ] {
        assert!(matches!(
            AutomaticOde::solve(&problem, (0.0, 1.0), &[1.0], &options, &policy),
            Err(AutomaticOdeError::Selection(reason)) if reason == expected
        ));
    }
}

#[test]
fn ineligible_challenger_does_not_hide_invalid_numerical_input() {
    let p = Problem::new(1, |_, _, _| panic!("numerical preflight"));
    for opts in [
        Options {
            rtol: f64::NAN,
            ..Default::default()
        },
        Options {
            atol: Atol::Vector(vec![1e-9, 1e-9]),
            ..Default::default()
        },
        Options {
            h0: -1.0,
            ..Default::default()
        },
        Options {
            max_steps: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &opts, &supplied_only()),
            Err(AutomaticOdeError::Switching(_))
        ));
    }
    assert!(matches!(
        AutomaticOde::solve(
            &p,
            (1.0, 0.0),
            &[1.0],
            &Options::default(),
            &supplied_only()
        ),
        Err(AutomaticOdeError::Switching(_))
    ));
    assert!(matches!(
        AutomaticOde::solve(
            &p,
            (0.0, 1.0),
            &[f64::NAN],
            &Options::default(),
            &supplied_only()
        ),
        Err(AutomaticOdeError::Switching(_))
    ));
}

#[test]
fn unsupported_provider_declaration_is_not_silently_downgraded() {
    let p = Problem::new(1, |_, _, _| panic!("declaration preflight"))
        .jacobian(Jacobian::jvp(|_, _, _, _| panic!("declaration preflight")))
        .autonomous()
        .banded(0, 0);
    let policy = AutomaticOdePolicy {
        allow_matrix_free: true,
        ..supplied_only()
    };
    assert!(matches!(
        AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &Options::default(), &policy),
        Err(AutomaticOdeError::Selection(SelectionRefusal::Implicit(
            Unsupported::BandDeclarationOnJvpProblem
        )))
    ));
}

#[test]
fn eligible_challenger_still_requires_replay_contract_and_explicit_resource_permission() {
    let p = Problem::new(1, |_, _, _| panic!("replay preflight"))
        .jacobian(Jacobian::jvp(|_, _, _, _| panic!("replay preflight")))
        .autonomous();
    let policy = AutomaticOdePolicy {
        allow_matrix_free: true,
        ..supplied_only()
    };
    assert!(matches!(
        AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &Options::default(), &policy),
        Err(AutomaticOdeError::ReplayRequired)
    ));
}

#[test]
fn missing_provider_does_not_hide_malformed_derivative_declarations() {
    let p = Problem::new(1, |_, _, _| panic!("band preflight")).banded(1, 0);
    assert!(matches!(
        AutomaticOde::solve(
            &p,
            (0.0, 1.0),
            &[1.0],
            &Options::default(),
            &supplied_only()
        ),
        Err(AutomaticOdeError::Switching(_))
    ));
    let p = Problem::new(1, |_, _, _| panic!("autonomy preflight"))
        .autonomous()
        .time_derivative(|_, _, _| panic!("autonomy preflight"));
    assert!(matches!(
        AutomaticOde::solve(
            &p,
            (0.0, 1.0),
            &[1.0],
            &Options::default(),
            &supplied_only()
        ),
        Err(AutomaticOdeError::Switching(_))
    ));
}

#[test]
fn fallback_keeps_failed_partial_run_and_charges_the_attempt_budget() {
    let p = Problem::new(1, |_, y, dy| {
        dy[0] = -y[0];
        Ok(())
    });
    let opts = Options {
        max_steps: 1,
        ..Default::default()
    };
    let run = AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &opts, &supplied_only()).unwrap();
    assert!(!run.success);
    assert!(run.final_time < 1.0);
    assert_eq!(run.stats.attempts, 1);
    assert_eq!(run.stats.jacobian_builds, 0);
    assert!(run.stats.accounting_complete);
    assert!(run.message.contains("implicit challenger unavailable"));
}
