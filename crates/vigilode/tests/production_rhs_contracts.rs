//! A finite previous stage buffer must never conceal an incomplete callback.
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::{Atol, Callback, FailureReason, Jacobian, Options, Problem, Rodas5p, Status};

fn options() -> Options {
    Options {
        h0: 1e-3,
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        max_steps: 100,
        ..Options::default()
    }
}

#[test]
fn incomplete_rhs_stops_once_on_dense_and_finite_difference_routes() {
    for finite_difference in [false, true] {
        for malformed_at in [1, 2] {
            let calls = Arc::new(AtomicU64::new(0));
            let seen = calls.clone();
            let mut p = Problem::new(2, move |_, y, out| {
                let call = seen.fetch_add(1, Ordering::SeqCst) + 1;
                out[0] = -y[0];
                if call != malformed_at {
                    out[1] = -y[1];
                }
                Ok(())
            })
            .autonomous();
            if !finite_difference {
                p = p.jacobian(Jacobian::dense(|_, _, out| {
                    // Sparse writes remain valid: the dense matrix is zero-filled.
                    out[0] = -1.0;
                    out[3] = -1.0;
                    Ok(())
                }));
            }
            let s = Rodas5p::solve(&p, (0.0, 0.1), &[1.0, 2.0], &options()).unwrap();
            assert!(
                matches!(
                    s.status,
                    Status::Failed(FailureReason::CallbackFailed {
                        callback: Callback::Rhs,
                        ..
                    })
                ),
                "{finite_difference} {malformed_at}: {:?}",
                s.status
            );
            assert_eq!(calls.load(Ordering::SeqCst), malformed_at);
            assert_eq!(s.stats.callbacks.fatal_errors, 1);
            assert_eq!(s.stats.accepted_steps, 0);
            assert_eq!(s.t, vec![0.0]);
            assert_eq!(s.y, vec![vec![1.0, 2.0]]);
        }
    }
}

#[test]
fn incomplete_time_derivative_is_terminal() {
    let calls = Arc::new(AtomicU64::new(0));
    let seen = calls.clone();
    let p = Problem::new(2, |t, y, out| {
        out[0] = -y[0] + t;
        out[1] = -y[1] + t;
        Ok(())
    })
    .jacobian(Jacobian::dense(|_, _, out| {
        out[0] = -1.0;
        out[3] = -1.0;
        Ok(())
    }))
    .time_derivative(move |_, _, out| {
        seen.fetch_add(1, Ordering::SeqCst);
        out[0] = 1.0;
        Ok(())
    });
    let s = Rodas5p::solve(&p, (0.0, 0.1), &[1.0, 2.0], &options()).unwrap();
    assert!(matches!(
        s.status,
        Status::Failed(FailureReason::CallbackFailed {
            callback: Callback::TimeDerivative,
            ..
        })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(s.stats.accepted_steps, 0);
}

#[test]
fn sparse_jacobian_writes_remain_supported() {
    let p = Problem::new(2, |_, y, out| {
        out[0] = -y[0];
        out[1] = -2.0 * y[1];
        Ok(())
    })
    .autonomous()
    .jacobian(Jacobian::dense(|_, _, out| {
        out[0] = -1.0;
        out[3] = -2.0;
        Ok(())
    }));
    let s = Rodas5p::solve(&p, (0.0, 0.1), &[1.0, 2.0], &options()).unwrap();
    assert!(matches!(s.status, Status::Success));
    let last = s.y.last().unwrap();
    assert!((last[0] - (-0.1_f64).exp()).abs() < 1e-7);
    assert!((last[1] - 2.0 * (-0.2_f64).exp()).abs() < 1e-7);
}
