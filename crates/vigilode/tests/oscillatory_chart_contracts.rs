use vigilode::{ExponentialChirpChart, paired_carrier_rotation};

fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() < 2e-12 * (1.0 + a.abs() + b.abs()),
        "{a} vs {b}"
    );
}

#[test]
fn constant_frequency_and_small_beta_have_correct_phase_limit() {
    for beta in [0.0, 1e-15, -1e-15] {
        let c = ExponentialChirpChart::new(2.0, beta, 0.1, (0.0, 3.0)).unwrap();
        close(c.phase(3.0).unwrap(), 6.0);
    }
}

#[test]
fn physical_chart_roundtrip_preserves_arbitrary_initial_data() {
    for beta in [-1.2, 0.0, 1.2] {
        let c = ExponentialChirpChart::new(1.0, beta, 0.8, (0.0, 8.0)).unwrap();
        for t in [0.0, 0.17, 2.0, 8.0] {
            let y = [0.3, -0.9];
            let back = c.to_physical(t, c.to_rotating(t, y).unwrap()).unwrap();
            close(back[0], y[0]);
            close(back[1], y[1]);
        }
    }
}

#[test]
fn time_dependent_chart_chain_rule_matches_original_chirp() {
    let gamma = 0.8;
    let beta = 1.2;
    let c = ExponentialChirpChart::new(1.0, beta, gamma, (0.0, 8.0)).unwrap();
    for t in [0.0, 0.2, 1.7, 7.0] {
        let y = [0.7, -0.2];
        let omega = c.frequency(t).unwrap();
        let f = [
            y[1],
            -(2.0 * gamma - beta) * y[1] - (omega * omega + gamma * gamma - gamma * beta) * y[0],
        ];
        let q = y[0];
        let p = (y[1] + gamma * y[0]) / omega;
        let dp = (f[1] + gamma * f[0]) / omega - beta * p;
        close(f[0], -gamma * q + omega * p);
        close(dp, -omega * q - gamma * p);
        let w = c.to_rotating(t, y).unwrap();
        let dw = c.rotating_rhs(t, w, [0.0; 2]).unwrap();
        close(dw[0], -gamma * w[0]);
        close(dw[1], -gamma * w[1]);
    }
}

#[test]
fn physical_error_transport_accounts_for_velocity_amplification() {
    let c = ExponentialChirpChart::new(1.0, 1.2, 0.8, (0.0, 8.0)).unwrap();
    let w = [0.3, -0.7];
    let delta = [1e-7, -2e-7];
    let a = c.to_physical(8.0, w).unwrap();
    let b = c
        .to_physical(8.0, [w[0] + delta[0], w[1] + delta[1]])
        .unwrap();
    let bound = c
        .physical_error_bound(8.0, delta[0].hypot(delta[1]))
        .unwrap();
    assert!((a[0] - b[0]).abs().max((a[1] - b[1]).abs()) <= bound);
    assert!(bound > 1000.0 * delta[0].hypot(delta[1]));
    let tol = c.conservative_local_atol(1e-8).unwrap();
    assert!(c.physical_error_bound(8.0, 2.0_f64.sqrt() * tol).unwrap() <= 1.000000000000001e-8);
}

#[test]
fn nonlinear_equivariance_removes_carrier_from_residual() {
    let c = ExponentialChirpChart::new(1.0, 1.2, 0.8, (0.0, 8.0)).unwrap();
    let w = [0.4, -0.3];
    let epsilon = 0.3;
    for t in [0.0, 1.0, 8.0] {
        let y = c.to_physical(t, w).unwrap();
        let factor = -epsilon * (w[0] * w[0] + w[1] * w[1]);
        let d = c
            .rotating_rhs(t, w, [factor * y[0], factor * y[1]])
            .unwrap();
        close(d[0], (-0.8 + factor) * w[0]);
        close(d[1], (-0.8 + factor) * w[1]);
    }
}

#[test]
fn forcing_negative_control_retains_fast_phase() {
    let c = ExponentialChirpChart::new(100.0, 0.0, 0.0, (0.0, 1.0)).unwrap();
    let a = c.rotating_rhs(0.0, [0.0; 2], [0.0, 100.0]).unwrap();
    let b = c
        .rotating_rhs(std::f64::consts::FRAC_PI_2 / 100.0, [0.0; 2], [0.0, 100.0])
        .unwrap();
    close(a[0], 0.0);
    close(a[1], 1.0);
    close(b[0], -1.0);
    close(b[1], 0.0);
}

#[test]
fn involution_shares_coefficients_but_preserves_independent_amplitudes() {
    let out = paired_carrier_rotation(0.37, [1.0, 2.0], [-3.0, 0.4]).unwrap();
    let (s, c) = 0.37_f64.sin_cos();
    close(out[0][0], c - 2.0 * s);
    close(out[0][1], s + 2.0 * c);
    close(out[1][0], -3.0 * c + 0.4 * s);
    close(out[1][1], 3.0 * s + 0.4 * c);
}

#[test]
fn malformed_and_nonfinite_inputs_are_rejected() {
    for (w, b, g) in [
        (0.0, 1.0, 0.0),
        (1.0, f64::NAN, 0.0),
        (1.0, 1.0, -1.0),
        (1.0, 1000.0, 0.0),
    ] {
        assert!(ExponentialChirpChart::new(w, b, g, (0.0, 8.0)).is_err());
    }
    let c = ExponentialChirpChart::new(1.0, 1.2, 0.8, (0.0, 8.0)).unwrap();
    assert!(c.to_physical(9.0, [1.0, 0.0]).is_err());
    assert!(c.to_physical(1.0, [f64::NAN, 0.0]).is_err());
    assert!(c.rotating_rhs(1.0, [0.0; 2], [0.0, f64::INFINITY]).is_err());
    assert!(c.physical_error_bound(1.0, -1.0).is_err());
    assert!(c.conservative_local_atol(f64::from_bits(1)).is_err());
    assert!(paired_carrier_rotation(f64::NAN, [0.0; 2], [0.0; 2]).is_err());
}

#[test]
fn phase_does_not_lose_large_frequency_when_beta_dt_underflows() {
    let c = ExponentialChirpChart::new(1e300, 1e-300, 0.0, (0.0, 1e-100)).unwrap();
    close(c.phase(1e-100).unwrap() / 1e200, 1.0);
    assert!(ExponentialChirpChart::new(1e-300, 0.0, 0.0, (-1e308, 1e308)).is_err());
}

#[test]
fn phase_handles_large_relative_growth_without_intermediate_exponential_overflow() {
    let c = ExponentialChirpChart::new(1e-300, 1.0, 0.0, (0.0, 1000.0));
    // The constructor's explicit frequency formula overflows and refuses this range.
    assert!(c.is_err());
    let c = ExponentialChirpChart::new(1.0, 1.0, 0.0, (-400.0, 400.0)).unwrap();
    close(c.phase(400.0).unwrap() / 400.0_f64.exp(), 1.0);
}
