use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::{
    Atol, CallbackError, DampedRotationBlock, DampedRotationFlow, Jacobian, LawsonDopri5, Options,
    Problem,
};

fn opts() -> Options {
    Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        h0: 0.01,
        ..Default::default()
    }
}
fn flow(d: f64, w: f64) -> DampedRotationFlow {
    DampedRotationFlow::repeated(
        DampedRotationBlock {
            damping: d,
            frequency: w,
        },
        1,
    )
    .unwrap()
}

#[test]
fn declared_linear_flow_removes_fast_scale_without_derivatives() {
    let calls = Arc::new(AtomicU64::new(0));
    let seen = calls.clone();
    let n = Problem::new(2, move |_, _, out| {
        seen.fetch_add(1, Ordering::Relaxed);
        out.fill(0.0);
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, _| panic!("Lawson usedJacobian")))
    .time_derivative(|_, _, _| panic!("Lawson useddfdt"));
    let f = flow(0.1, 10000.0);
    let r = LawsonDopri5::solve(&f, &n, (0.0, 1.0), &[1.0, 0.0], &opts()).unwrap();
    let s = r.integration;
    assert!(s.success, "{}", s.message);
    let exact = [
        (-0.1f64).exp() * 10000.0f64.cos(),
        (-0.1f64).exp() * 10000.0f64.sin(),
    ];
    assert!((s.final_state[0] - exact[0]).hypot(s.final_state[1] - exact[1]) < 5e-12);
    let c = s.stats.counters;
    assert_eq!(
        c.jacobian_builds + c.jvp_calls + c.ft_calls + c.linear_solves + c.nonlinear_solves,
        0
    );
    assert_eq!(c.rhs_calls, calls.load(Ordering::Relaxed));
    assert!(c.rhs_calls < 100);
    assert!(r.quotient_work.applied_block_actions > 0);
}

#[test]
fn residual_is_n_not_full_rhs_and_forcing_matches_analytic_solution() {
    let n = Problem::new(2, |_, _, out| {
        out[0] = 1.0;
        out[1] = 0.0;
        Ok(())
    });
    let d = 0.2;
    let w = 3.0;
    let t: f64 = 2.0;
    let r = LawsonDopri5::solve(&flow(d, w), &n, (0.0, t), &[0.0, 0.0], &opts())
        .unwrap()
        .integration;
    assert!(r.success, "{}", r.message);
    let a = (-d * t).exp() * (w * t).cos() - 1.0;
    let b = (-d * t).exp() * (w * t).sin();
    let exact = [
        (-d * a + w * b) / (d * d + w * w),
        (-w * a - d * b) / (d * d + w * w),
    ];
    assert!((r.final_state[0] - exact[0]).hypot(r.final_state[1] - exact[1]) < 2e-8);
}

#[test]
fn nonlinear_radial_residual_has_independent_exact_decay() {
    let epsilon = 0.3;
    let n = Problem::new(2, move |_, y, out| {
        let r2 = y[0] * y[0] + y[1] * y[1];
        out[0] = -epsilon * r2 * y[0];
        out[1] = -epsilon * r2 * y[1];
        Ok(())
    });
    let d = 0.2;
    let w = 80.0;
    let t: f64 = 2.0;
    let result = LawsonDopri5::solve(&flow(d, w), &n, (0.0, t), &[1.0, 0.0], &opts())
        .unwrap()
        .integration;
    assert!(result.success, "{}", result.message);
    let radius = (-d * t).exp() / (1.0 + (epsilon / d) * (1.0 - (-2.0 * d * t).exp())).sqrt();
    assert!(
        (result.final_state[0] - radius * (w * t).cos())
            .hypot(result.final_state[1] - radius * (w * t).sin())
            < 2e-8
    );
}

#[test]
fn fatal_and_malformed_residuals_preserve_partial_states_and_telemetry() {
    for malformed in [false, true] {
        let n = Problem::new(2, move |t, _, out| {
            if t > 0.03 {
                if malformed {
                    out[0] = 0.0;
                    return Ok(());
                }
                return Err(CallbackError::Fatal("stop residual".into()));
            }
            out.fill(0.0);
            Ok(())
        });
        let r = LawsonDopri5::solve(&flow(0.1, 1.0), &n, (0.0, 1.0), &[1.0, 0.0], &opts())
            .unwrap()
            .integration;
        assert!(!r.success);
        assert!(r.callback_failure.is_some());
        assert!(r.final_time <= 0.03);
        assert_eq!(r.final_time, *r.t.last().unwrap());
        assert_eq!(r.final_state, *r.y.last().unwrap());
        assert_eq!(
            r.stats.callback_work.fatal_errors + r.stats.callback_work.malformed_outputs,
            1
        );
    }
}

#[test]
fn malformed_requests_are_refused_before_residual_evaluation() {
    let n = Problem::new(2, |_, _, _| panic!("badLawsonrequestenteredcallback"));
    assert!(
        LawsonDopri5::solve(
            &flow(0.0, 1.0),
            &n,
            (0.0, 1.0),
            &[1.0, 0.0],
            &Options {
                dense_output: true,
                ..opts()
            }
        )
        .is_err()
    );
    let odd = Problem::new(1, |_, _, _| panic!("dimensionmismatchenteredcallback"));
    assert!(LawsonDopri5::solve(&flow(0.0, 1.0), &odd, (0.0, 1.0), &[1.0], &opts()).is_err());
    assert!(
        LawsonDopri5::solve_observed(
            &flow(0.0, 1.0),
            &n,
            (0.0, 1.0),
            &[1.0, 0.0],
            &opts(),
            &[0.0, 0.8, 0.2]
        )
        .is_err()
    );
}

#[test]
fn facade_records_residual_work_and_method_identity() {
    let n = Problem::new(2, |_, _, out| {
        out.fill(0.0);
        Ok(())
    });
    let r = LawsonDopri5::solve(&flow(0.0, 1.0), &n, (0.0, 1.0), &[1.0, 0.0], &opts()).unwrap();
    let j = serde_json::to_value(r).unwrap();
    assert_eq!(
        j["integration"]["stats"]["schema"],
        "vigilode.lawson-stats.v1"
    );
    assert_eq!(
        j["integration"]["stats"]["method"],
        "lawson-dopri5-declared-damped-rotation"
    );
    assert!(
        j["quotient_work"]["applied_block_actions"]
            .as_u64()
            .unwrap()
            > 0
    );
}
