use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    DampedRotationBlock as Block, DampedRotationFlow as Flow, Dopri5Config,
    LawsonDopri5Workspace as Workspace, OutputSchedule, QuotientWork,
    integrate_lawson_dopri5_observed,
};
use std::cell::Cell;
fn flow(d: f64, w: f64) -> Flow {
    Flow::repeated(
        Block {
            damping: d,
            frequency: w,
        },
        1,
    )
    .unwrap()
}
fn close(a: &[f64], b: &[f64], tol: f64) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= tol, "{a:?} versus {b:?}");
    }
}
#[test]
fn quotient_inverse_satisfies_independent_equation() {
    for d in [0.0, 0.1, 100.0] {
        for w in [-200.0, 0.0, 3.0] {
            for s in [0.0, 1e-12, 0.2, 100.0] {
                let f = flow(d, w);
                let b = [2.0, -3.0];
                let mut x = [0.0; 2];
                f.shifted_inverse_into(s, &b, &mut x).unwrap();
                close(
                    &[
                        (1.0 + s * d) * x[0] + s * w * x[1],
                        -s * w * x[0] + (1.0 + s * d) * x[1],
                    ],
                    &b,
                    2e-14,
                );
            }
        }
    }
}
#[test]
fn quotient_inverse_handles_product_and_square_overflow() {
    let f = flow(1e308, 1e308);
    let mut x = [0.0; 2];
    f.shifted_inverse_into(1e308, &[1e308, 1e308], &mut x)
        .unwrap();
    assert!(x[0].abs() < 1e-320);
    assert!((x[1] / 1e-308 - 1.0).abs() < 1e-14);
    let f = flow(1e200, 0.0);
    f.shifted_inverse_into(1.0, &[1e200, 2e200], &mut x)
        .unwrap();
    close(&x, &[1.0, 2.0], 1e-15);
}
#[test]
fn coalescence_and_group_sharing_are_regular() {
    let f = Flow::new(&[
        Block {
            damping: 2.0,
            frequency: 0.0,
        },
        Block {
            damping: 2.0,
            frequency: -0.0,
        },
    ])
    .unwrap();
    assert_eq!(f.distinct_blocks(), 1);
    let mut y = [0.0; 4];
    f.apply_into(1.0, &[1.0, 2.0, 3.0, 4.0], &mut y).unwrap();
    close(
        &y,
        &[
            (-2.0f64).exp(),
            2.0 * (-2.0f64).exp(),
            3.0 * (-2.0f64).exp(),
            4.0 * (-2.0f64).exp(),
        ],
        1e-15,
    );
}
#[test]
fn pure_carrier_exact_step_and_zero_derivative_work() {
    let f = flow(0.0, 12345.0);
    let mut ws = Workspace::new(f.clone()).unwrap();
    let mut y = [0.0; 2];
    let mut e = [0.0; 2];
    let mut c = WorkCounters::default();
    let mut work = QuotientWork::default();
    ws.step_into(
        &|_, _, out: &mut [f64]| {
            out.fill(0.0);
            Ok(())
        },
        0.0,
        &[1.0, 0.0],
        1.0,
        &mut y,
        &mut e,
        &mut c,
        &mut work,
    )
    .unwrap();
    let mut exact = [0.0; 2];
    f.apply_into(1.0, &[1.0, 0.0], &mut exact).unwrap();
    close(&y, &exact, 0.0);
    close(&e, &[0.0; 2], 0.0);
    assert_eq!(c.rhs_calls, 7);
    assert_eq!(c.jvp_calls, 0);
    assert_eq!(c.jacobian_builds, 0);
}
fn fixed(h: f64) -> f64 {
    let f = flow(0.3, 2.0);
    let mut ws = Workspace::new(f).unwrap();
    let mut state = [1.0, 0.2];
    let mut out = [0.0; 2];
    let mut err = [0.0; 2];
    let mut c = WorkCounters::default();
    let mut w = QuotientWork::default();
    for k in 0..(1.0 / h).round() as usize {
        ws.step_into(
            &|_, y, out: &mut [f64]| {
                out[0] = 0.2 * y[0];
                out[1] = -0.1 * y[1];
                Ok(())
            },
            k as f64 * h,
            &state,
            h,
            &mut out,
            &mut err,
            &mut c,
            &mut w,
        )
        .unwrap();
        state = out;
    }
    let nu = (4.0f64 - 0.0225).sqrt();
    let factor = (-0.25f64).exp();
    let (s, c) = nu.sin_cos();
    let exact = [
        factor * (c + s / nu * (0.15 - 2.0 * 0.2)),
        factor * (0.2 * c + s / nu * (2.0 - 0.15 * 0.2)),
    ];
    state
        .iter()
        .zip(exact)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}
#[test]
fn fifth_order_on_noncommuting_residual() {
    let e1 = fixed(0.2);
    let e2 = fixed(0.1);
    let e3 = fixed(0.05);
    assert!(e1 / e2 > 25.0, "{e1} {e2} {e3}");
    assert!(e2 / e3 > 25.0, "{e1} {e2} {e3}");
}
#[test]
fn nonlinear_radial_equivariant_solution() {
    let f = flow(0.0, 2000.0);
    let mut config = Dopri5Config::default();
    config.adaptive.rtol = 1e-8;
    config.adaptive.atol = 1e-11;
    config.adaptive.initial_step = 0.01;
    let schedule = OutputSchedule::new(vec![0.0, 0.5, 1.0, 2.0]).unwrap();
    let result = integrate_lawson_dopri5_observed(
        &f,
        &|_, y, out: &mut [f64]| {
            let a = 0.2 * (1.0 - y[0] * y[0] - y[1] * y[1]);
            out[0] = a * y[0];
            out[1] = a * y[1];
            Ok(())
        },
        (0.0, 2.0),
        &[0.2, 0.0],
        &config,
        &schedule,
    )
    .unwrap();
    assert!(result.integration.observed.success);
    for (&t, y) in result
        .integration
        .observed
        .t
        .iter()
        .zip(&result.integration.observed.y)
    {
        let r = (1.0 / (1.0 + 24.0 * (-0.4 * t).exp())).sqrt();
        let (s, c) = (2000.0 * t).sin_cos();
        close(y, &[r * c, r * s], 2e-8);
    }
    assert_eq!(
        result.integration.observed.counters.rhs_calls,
        1 + 6 * result.integration.attempts as u64
    );
    assert!(result.integration.accepted_steps < 40);
}
#[test]
fn callback_failure_preserves_accepted_state_and_work() {
    let f = flow(0.0, 0.0);
    let mut config = Dopri5Config::default();
    config.adaptive.initial_step = 0.1;
    config.adaptive.max_step = 0.1;
    let result = integrate_lawson_dopri5_observed(
        &f,
        &|t, _, out: &mut [f64]| {
            if t > 0.11 {
                return Err(CoreError::InvalidInput("stop".into()));
            }
            out.fill(1.0);
            Ok(())
        },
        (0.0, 1.0),
        &[0.0, 0.0],
        &config,
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
    )
    .unwrap();
    assert!(!result.integration.observed.success);
    assert_eq!(result.integration.final_time, 0.1);
    close(&result.integration.final_state, &[0.1; 2], 1e-15);
    assert_eq!(result.integration.accepted_steps, 1);
    assert_eq!(result.integration.observed.counters.rhs_calls, 8);
}
#[test]
fn malformed_callback_is_fatal_even_when_recoverable_protocol_enabled() {
    let config = Dopri5Config {
        recover_nonfinite_rhs: true,
        ..Dopri5Config::default()
    };
    let result = integrate_lawson_dopri5_observed(
        &flow(0.0, 1.0),
        &|_, _, out: &mut [f64]| {
            out[0] = 0.0;
            Ok(())
        },
        (0.0, 1.0),
        &[1.0, 0.0],
        &config,
        &OutputSchedule::new(vec![0.0, 1.0]).unwrap(),
    )
    .unwrap();
    assert!(!result.integration.observed.success);
    assert_eq!(result.integration.final_time, 0.0);
    assert_eq!(result.integration.observed.counters.rhs_calls, 1);
}
#[test]
fn invalid_inputs_refused_before_callback_and_large_damping_stays_finite() {
    assert!(
        Flow::new(&[Block {
            damping: -1.0,
            frequency: 0.0
        }])
        .is_err()
    );
    let calls = Cell::new(0);
    let config = Dopri5Config {
        atol: Some(vec![1e-8]),
        ..Dopri5Config::default()
    };
    assert!(
        integrate_lawson_dopri5_observed(
            &flow(0.0, 1.0),
            &|_, _, out: &mut [f64]| {
                calls.set(calls.get() + 1);
                out.fill(0.0);
                Ok(())
            },
            (0.0, 1.0),
            &[1.0, 0.0],
            &config,
            &OutputSchedule::new(vec![0.0, 1.0]).unwrap()
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut out = [0.0; 2];
    flow(1e308, 1e308)
        .apply_into(1e308, &[1.0, 2.0], &mut out)
        .unwrap();
    close(&out, &[0.0; 2], 0.0);
}

#[test]
fn underflowing_exponential_coefficient_does_not_erase_large_state() {
    let f = flow(1000.0, 0.0);
    let mut y = [0.0; 2];
    f.apply_into(1.0, &[1e308, 0.0], &mut y).unwrap();
    let reference = 5.075_958_897_549_457e-127;
    assert!((y[0] / reference - 1.0).abs() < 2e-15, "{:?}", y);
    let mut ws = Workspace::new(f).unwrap();
    let mut e = [0.0; 2];
    ws.step_into(
        &|_, _, out: &mut [f64]| {
            out.fill(0.0);
            Ok(())
        },
        0.0,
        &[1e308, 0.0],
        1.0,
        &mut y,
        &mut e,
        &mut WorkCounters::default(),
        &mut QuotientWork::default(),
    )
    .unwrap();
    assert!((y[0] / reference - 1.0).abs() < 2e-15);
}
#[test]
fn quotient_inverse_divides_before_large_sum() {
    let mut y = [0.0; 2];
    flow(0.0, 1.0)
        .shifted_inverse_into(1.0, &[f64::MAX, f64::MAX], &mut y)
        .unwrap();
    assert_eq!(y[0], 0.0);
    assert_eq!(y[1], f64::MAX);
}

#[test]
fn subnormal_coefficient_is_scaled_before_large_state() {
    let mut y = [0.0; 2];
    flow(744.0, 0.0)
        .apply_into(1.0, &[1e308, 0.0], &mut y)
        .unwrap();
    let reference = 7.671_944_704_179_979e-16;
    assert!((y[0] / reference - 1.0).abs() < 2e-15, "{:?}", y);
}
