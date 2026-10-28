use vigilode::{Atol, CallbackError, Dopri5, ExponentialCofactor, Options, Problem};

#[test]
fn carrier_has_zero_primitive_and_preserves_initial_value() {
    let carrier = ExponentialCofactor::new(0.0, -1.0, 1e6, 0.2).unwrap();
    assert_eq!(carrier.value(0.0).unwrap(), -1.0);
    assert_eq!(carrier.primitive(0.0).unwrap(), 0.0);
    assert_eq!(carrier.physical_slow(0.0, 1.0).unwrap(), 1.0);
    assert!((carrier.primitive(1e-16).unwrap() / -2e-17 - 1.0).abs() < 1e-9);
    assert_eq!(carrier.value(1.0).unwrap(), 0.0);
    assert!((carrier.primitive(1.0).unwrap() + 2e-7).abs() < 1e-22);
}

#[test]
fn invalid_carrier_and_reconstruction_are_refused() {
    for rate in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(ExponentialCofactor::new(0.0, 1.0, rate, 0.2).is_err());
    }
    let c = ExponentialCofactor::new(0.0, 1.0, 1.0, 0.2).unwrap();
    assert!(c.value(-1.0).is_err());
    assert!(c.value(f64::NAN).is_err());
    assert!(c.physical_slow(1.0, f64::INFINITY).is_err());
    let c = ExponentialCofactor::new(0.0, 1e-300, 1e100, 1e100).unwrap();
    assert!((c.primitive(1.0).unwrap() / 1e-300 - 1.0).abs() < 1e-12);
    let c = ExponentialCofactor::new(0.0, 1e300, 1.0, 0.0).unwrap();
    let expected = (1e300_f64.ln() - 1000.0).exp();
    assert!(c.value(1000.0).unwrap() > 0.0);
    assert!((c.value(1000.0).unwrap() / expected - 1.0).abs() < 1e-12);
    let c = ExponentialCofactor::new(-1e308, 1.0, 1e-308, 0.1).unwrap();
    assert!((c.value(1e308).unwrap() - (-2.0_f64).exp()).abs() < 1e-15);
    let c = ExponentialCofactor::new(0.0, 1.0, 1e-300, 1e8).unwrap();
    let primitive = c.primitive(1e-100).unwrap();
    assert!(primitive > 0.0);
    assert!((primitive / 1e-92 - 1.0).abs() < 1e-12);
}

#[test]
fn reconstructed_velocity_satisfies_the_original_nonlinear_equations() {
    for eps in [1e-2, 1e-6, 1e-12] {
        let c = ExponentialCofactor::new(0.0, -1.0_f64.cos(), 1.0 / eps, 0.2).unwrap();
        for tau in [0.0, 0.1, 1.0, 10.0, 1000.0] {
            let t = tau * eps;
            let y = c.physical_slow(t, 0.9).unwrap();
            let z = c.value(t).unwrap();
            let x = y.cos() + z;
            let vdot = -y / (1.0 + t) + 0.2 * y.cos();
            let ydot = vdot + 0.2 * z;
            let xdot = -y.sin() * ydot - z / eps;
            assert!((ydot - (-y / (1.0 + t) + 0.2 * x)).abs() < 1e-14);
            // Evaluate the cofactor directly; x-cos(y) loses digits after
            // relaxation, which is exactly the cancellation the chart avoids.
            assert!((xdot + y.sin() * ydot + z / eps).abs() <= 1e-12 * (1.0 + (z / eps).abs()));
        }
    }
}

#[test]
fn reduced_dopri_solves_the_published_stiff_case_without_derivatives() {
    let c = ExponentialCofactor::new(0.0, -1.0_f64.cos(), 1e6, 0.2).unwrap();
    let p = Problem::new(1, move |t, v, out| {
        let y = c
            .physical_slow(t, v[0])
            .map_err(|s| CallbackError::Fatal(s.into()))?;
        out[0] = -y / (1.0 + t) + 0.2 * y.cos();
        Ok(())
    });
    let opts = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 1000,
        ..Default::default()
    };
    let run = Dopri5::solve(&p, (0.0, 2.0), &[1.0], &opts).unwrap();
    assert!(run.success);
    assert!(run.stats.attempts < 100);
    assert_eq!(run.stats.counters.jvp_calls, 0);
    assert_eq!(run.stats.counters.jacobian_builds, 0);
    assert_eq!(run.stats.counters.nonlinear_iterations, 0);
    let y = c.physical_slow(2.0, run.final_state[0]).unwrap();
    assert!(y > 0.4 && y < 0.7);
}
