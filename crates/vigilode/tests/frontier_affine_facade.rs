use vigilode::{
    Atol, CallbackError, DampedRotationBlock, DampedRotationFlow, Jacobian, LawsonDopri5, Options,
    Problem,
};
fn flow() -> DampedRotationFlow {
    DampedRotationFlow::repeated(
        DampedRotationBlock {
            damping: 1000.,
            frequency: 3000.,
        },
        1,
    )
    .unwrap()
}
fn options() -> Options {
    Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-12),
        h0: 1.,
        ..Default::default()
    }
}
#[test]
fn declared_affine_facade_uses_no_derivatives_and_reports_split() {
    let residual = Problem::new(2, |_, _, out| {
        out.fill(0.);
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("Jacobian called")))
    .time_derivative(|_, _, _| panic!("dfdt called"));
    let b = [1000., -2000.];
    let y = [1., 2.];
    let flow = flow();
    let mut exact = [0.; 2];
    flow.affine_into(1., &y, &b, &mut exact).unwrap();
    let r = LawsonDopri5::solve_affine_observed(
        &flow,
        &b,
        &residual,
        (0., 1.),
        &y,
        &options(),
        &[0., 1.],
    )
    .unwrap()
    .integration;
    assert!(r.success);
    assert_eq!(r.final_state, exact);
    assert_eq!(r.stats.accepted_steps, 1);
    assert_eq!(r.stats.method, "lawson-dopri5-declared-affine-rotation");
    let c = r.stats.counters;
    assert_eq!(
        c.jacobian_builds + c.jvp_calls + c.ft_calls + c.linear_solves + c.nonlinear_solves,
        0
    );
}
#[test]
fn bad_forcing_is_rejected_before_callback() {
    let residual = Problem::new(2, |_, _, _| panic!("callback ran"));
    let f = flow();
    for b in [vec![1.], vec![f64::NAN, 0.]] {
        assert!(
            LawsonDopri5::solve_affine_observed(
                &f,
                &b,
                &residual,
                (0., 1.),
                &[0., 0.],
                &options(),
                &[0., 1.]
            )
            .is_err()
        );
    }
}
#[test]
fn affine_fatal_callback_preserves_initial_state() {
    let residual = Problem::new(2, |_, _, _| Err(CallbackError::Fatal("stop".into())));
    let r = LawsonDopri5::solve_affine_observed(
        &flow(),
        &[1., 2.],
        &residual,
        (0., 1.),
        &[3., 4.],
        &options(),
        &[0., 1.],
    )
    .unwrap()
    .integration;
    assert!(!r.success);
    assert_eq!(r.final_time, 0.);
    assert_eq!(r.final_state, [3., 4.]);
    assert_eq!(r.stats.callback_work.fatal_errors, 1);
}
