use vigilode::*;

#[test]
fn default_automatic_is_rhs_only_and_returns_actual_selection_history() {
    let p = Problem::new(1, |_, y, d| {
        d[0] = -y[0];
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("derivative forbidden")))
    .autonomous();
    let r = AutomaticOde::solve(
        &p,
        (0.0, 1.0),
        &[1.0],
        &Options::default(),
        &AutomaticOdePolicy::default(),
    )
    .unwrap();
    assert!(r.success);
    assert_eq!(r.stats.jacobian_builds, 0);
    assert!((r.final_state[0] - (-1.0_f64).exp()).abs() < 1e-5);
}

#[test]
fn authorized_trials_require_replay_safe_callback_contract() {
    let p = Problem::new(1, |_, _, _| panic!("preflight must precede callbacks"))
        .jacobian(Jacobian::dense(|_, _, _| panic!("preflight")))
        .autonomous();
    let policy = AutomaticOdePolicy {
        derivatives: DerivativeFallback::ProvidedOnly,
        ..Default::default()
    };
    assert!(matches!(
        AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &Options::default(), &policy),
        Err(AutomaticOdeError::ReplayRequired)
    ));
}

#[test]
fn matrix_free_permission_and_dense_ceiling_are_not_bypassed() {
    let p = Problem::new(100, |_, _, _| panic!("preflight"))
        .jacobian(Jacobian::dense(|_, _, _| panic!("preflight")))
        .autonomous();
    let policy = AutomaticOdePolicy {
        derivatives: DerivativeFallback::ProvidedOnly,
        replay_safe_callbacks: true,
        ..Default::default()
    };
    assert!(matches!(
        AutomaticOde::solve(&p, (0.0, 1.0), &[0.0; 100], &Options::default(), &policy),
        Err(AutomaticOdeError::Selection(
            SelectionRefusal::DenseDimensionLimit
        ))
    ));
    let p = Problem::new(1, |_, _, _| panic!("preflight"))
        .jacobian(Jacobian::jvp(|_, _, _, _| panic!("preflight")))
        .autonomous();
    assert!(matches!(
        AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &Options::default(), &policy),
        Err(AutomaticOdeError::Selection(
            SelectionRefusal::MatrixFreeNotEnabled
        ))
    ));
}

#[test]
fn permitted_competition_keeps_total_attempt_budget() {
    let p = Problem::new(1, |t, y, d| {
        d[0] = -1000.0 * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, j| {
        j[0] = -1000.0;
        Ok(())
    }))
    .time_derivative(|t, _, f| {
        f[0] = -1000.0 * t.sin() - t.cos();
        Ok(())
    });
    let policy = AutomaticOdePolicy {
        derivatives: DerivativeFallback::ProvidedOnly,
        replay_safe_callbacks: true,
        ..Default::default()
    };
    let o = Options {
        max_steps: 10000,
        ..Default::default()
    };
    let r = AutomaticOde::solve(&p, (0.0, 1.0), &[1.0], &o, &policy).unwrap();
    assert!(r.success, "{}", r.message);
    assert!(r.stats.attempts <= o.max_steps);
    assert!((r.final_state[0] - 1.0_f64.cos()).abs() < 1e-4);
}
