//! Regression tests for avoidable floating-point loss in stage coefficients.
//! These concern finite endpoint increments, not a general guarantee of
//! high-order nonautonomous accuracy below the stage-time resolution.
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    Dopri5Config, Dopri5Workspace, OutputSchedule, integrate_dopri5_observed,
};

#[test]
fn subnormal_step_preserves_representable_constant_rhs_increment() {
    let h = f64::from_bits(2);
    for derivative in [1.0e308, -1.0e308] {
        let mut workspace = Dopri5Workspace::new(1).unwrap();
        let mut endpoint = [0.0];
        let mut error = [0.0];
        let mut work = WorkCounters::default();
        workspace
            .step_into(
                &|_, _, out| {
                    out[0] = derivative;
                    Ok(())
                },
                0.0,
                &[0.0],
                h,
                &mut endpoint,
                &mut error,
                &mut work,
            )
            .unwrap();
        let exact = h * derivative;
        let rounding_allowance = 32.0 * f64::EPSILON * exact.abs();
        assert!(
            (endpoint[0] - exact).abs() <= rounding_allowance,
            "endpoint={}, exact={exact}, embedded={}",
            endpoint[0],
            error[0]
        );
        assert!(error[0].abs() <= rounding_allowance);
        assert_eq!(work.rhs_calls, 7);
    }
}

#[test]
fn tiny_final_landing_does_not_silently_accept_half_the_increment() {
    // The final landing can be shorter than min_step, so the regression is
    // reachable with an ordinary initial step, not only custom tiny h0.
    let end = f64::from_bits(2);
    let derivative = 1.0e308;
    let rhs = |_: f64, _: &[f64], out: &mut [f64]| {
        out[0] = derivative;
        Ok(())
    };
    let mut config = Dopri5Config::default();
    config.adaptive.atol = 1.0e-30;
    config.adaptive.rtol = 1.0e-12;
    let output = OutputSchedule::new(vec![0.0, end]).unwrap();
    let result = integrate_dopri5_observed(&rhs, (0.0, end), &[0.0], &config, &output).unwrap();
    assert!(result.observed.success, "{}", result.observed.message);
    assert_eq!(result.final_time, end);
    let exact = end * derivative;
    assert!((result.final_state[0] - exact).abs() <= 32.0 * f64::EPSILON * exact.abs());
    assert_eq!(result.observed.t, vec![0.0, end]);
    assert_eq!(result.accepted_steps, 1);
    assert_eq!(result.observed.counters.rhs_calls, 7);
}
