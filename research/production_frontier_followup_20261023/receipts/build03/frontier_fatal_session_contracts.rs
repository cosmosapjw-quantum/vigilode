//! FA01: terminal native callbacks preserve the accepted transaction and work.
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use vigilode::{
    Atol, Callback, CallbackError, Direction, FailureReason, InteriorError, Jacobian, Options,
    Problem, Rodas5p, Status, StopReason,
};

#[derive(Clone, Copy, Debug)]
enum Route {
    Dense,
    Band,
    MatrixFree,
    FiniteDifference,
    FastDense,
    Sequential,
    SequentialEnforce,
}
#[derive(Clone, Default)]
struct Calls {
    rhs: Arc<AtomicU64>,
    jac: Arc<AtomicU64>,
    jvp: Arc<AtomicU64>,
    ft: Arc<AtomicU64>,
    selected: Arc<AtomicU64>,
    fatal: Arc<AtomicBool>,
}
impl Calls {
    fn enter(
        &self,
        kind: Callback,
        selected: Callback,
        fatal_at: u64,
        recover_first: bool,
    ) -> Result<(), CallbackError> {
        assert!(
            !self.fatal.load(Ordering::SeqCst),
            "user callback entered after fatal"
        );
        match kind {
            Callback::Rhs => &self.rhs,
            Callback::Jacobian => &self.jac,
            Callback::Jvp => &self.jvp,
            Callback::TimeDerivative => &self.ft,
            _ => unreachable!(),
        }
        .fetch_add(1, Ordering::SeqCst);
        if kind == selected {
            let n = self.selected.fetch_add(1, Ordering::SeqCst) + 1;
            if recover_first && n == 1 {
                return Err(CallbackError::Recoverable("first trial".into()));
            }
            if n == fatal_at {
                self.fatal.store(true, Ordering::SeqCst);
                return Err(CallbackError::Fatal("terminal probe".into()));
            }
        }
        Ok(())
    }
}
fn problem(route: Route, selected: Callback, fatal_at: u64, recover: bool, c: &Calls) -> Problem {
    let seen = c.clone();
    let mut p = Problem::new(1, move |_, y, out| {
        seen.enter(Callback::Rhs, selected, fatal_at, recover)?;
        out[0] = -y[0];
        Ok(())
    });
    let seen = c.clone();
    p = match route {
        Route::MatrixFree => p.jacobian(Jacobian::jvp(move |_, _, v, out| {
            seen.enter(Callback::Jvp, selected, fatal_at, recover)?;
            out[0] = -v[0];
            Ok(())
        })),
        Route::FiniteDifference => p.jacobian(Jacobian::FiniteDifference),
        Route::Band => p.jacobian(Jacobian::banded(0, 0, move |_, _, out| {
            seen.enter(Callback::Jacobian, selected, fatal_at, recover)?;
            out[0] = -1.0;
            Ok(())
        })),
        _ => p.jacobian(Jacobian::dense(move |_, _, out| {
            seen.enter(Callback::Jacobian, selected, fatal_at, recover)?;
            out[0] = -1.0;
            Ok(())
        })),
    };
    if matches!(route, Route::Sequential) {
        p = p.mass(vec![1.0]);
    }
    if selected == Callback::TimeDerivative {
        let seen = c.clone();
        p.time_derivative(move |_, _, out| {
            seen.enter(Callback::TimeDerivative, selected, fatal_at, recover)?;
            out.fill(0.0);
            Ok(())
        })
    } else {
        p.autonomous()
    }
}
fn options(route: Route) -> Options {
    Options {
        h0: 1e-3,
        rtol: 1e-6,
        atol: Atol::Scalar(1e-9),
        max_steps: 2000,
        dense_output: matches!(
            route,
            Route::FastDense | Route::Sequential | Route::SequentialEnforce
        ),
        interior_error: if matches!(route, Route::SequentialEnforce) {
            InteriorError::Enforce
        } else {
            InteriorError::WFilteredEnforce
        },
        ..Options::default()
    }
}
#[test]
fn terminal_callbacks_stop_once_and_preserve_prefix_on_supported_routes() {
    for route in [
        Route::Dense,
        Route::Band,
        Route::MatrixFree,
        Route::FiniteDifference,
        Route::FastDense,
        Route::Sequential,
        Route::SequentialEnforce,
    ] {
        let derivative = if matches!(route, Route::MatrixFree) {
            Callback::Jvp
        } else {
            Callback::Jacobian
        };
        let kinds = if matches!(route, Route::FiniteDifference) {
            vec![Callback::Rhs, Callback::TimeDerivative]
        } else {
            vec![Callback::Rhs, derivative, Callback::TimeDerivative]
        };
        for kind in kinds {
            // Dense derivatives run once per trial; RHS/JVP run at stages.
            let late = if matches!(kind, Callback::Jacobian | Callback::TimeDerivative) {
                4
            } else {
                40
            };
            for (fatal_at, recover) in [(1, false), (2, false), (late, false), (2, true)] {
                let c = Calls::default();
                let p = problem(route, kind, fatal_at, recover, &c);
                let o = options(route);
                let s = Rodas5p::solve(&p, (0.0, 10.0), &[1.0], &o).unwrap();
                assert!(
                    matches!(s.status, Status::Failed(FailureReason::CallbackFailed { callback, .. }) if callback == kind),
                    "{route:?} {kind:?} {fatal_at}: {:?}",
                    s.status
                );
                assert_eq!(s.stats.callbacks.fatal_errors, 1);
                if fatal_at == late {
                    assert!(
                        s.stats.accepted_steps > 0,
                        "late failure must follow acceptance: {route:?} {kind:?}"
                    );
                }
                if recover {
                    assert!(s.stats.rejected_steps >= 2);
                }
                assert_eq!(s.stats.callbacks.short_circuited_calls, 0);
                assert_eq!(s.stats.callbacks.recoverable_errors, u64::from(recover));
                assert_eq!(
                    s.stats.stop.as_ref().unwrap().reason,
                    StopReason::FatalCallback
                );
                assert_eq!(
                    s.stats.attempts,
                    s.stats.accepted_steps + s.stats.rejected_steps
                );
                assert_eq!(
                    s.stats.counters.rhs_calls,
                    c.rhs.load(Ordering::SeqCst),
                    "RHS {route:?} {kind:?} {fatal_at} recover={recover}"
                );
                assert_eq!(
                    s.stats.counters.jvp_calls,
                    c.jvp.load(Ordering::SeqCst),
                    "JVP {route:?} {kind:?} {fatal_at} recover={recover}"
                );
                if kind == Callback::TimeDerivative {
                    assert_eq!(
                        s.stats.counters.ft_calls,
                        c.ft.load(Ordering::SeqCst),
                        "dfdt {route:?} {kind:?} {fatal_at} recover={recover}"
                    );
                } else {
                    // The native evaluator counts autonomous zero-df/dt
                    // evaluations too (eval_partial_t), without user code.
                    assert_eq!(c.ft.load(Ordering::SeqCst), 0);
                }
                if s.stats.attempts > 1 {
                    let reference = Calls::default();
                    let control = Rodas5p::solve(
                        &problem(route, kind, u64::MAX, recover, &reference),
                        (0.0, 10.0),
                        &[1.0],
                        &Options {
                            max_steps: s.stats.attempts - 1,
                            ..o
                        },
                    )
                    .unwrap();
                    assert_eq!(
                        s.t.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                        control.t.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                        "{route:?} {kind:?}"
                    );
                    assert_eq!(s.y, control.y);
                    assert!(s.stats.counters.rhs_calls >= control.stats.counters.rhs_calls);
                    assert!(s.stats.counters.jvp_calls >= control.stats.counters.jvp_calls);
                } else {
                    assert_eq!(s.t, vec![0.0]);
                    assert_eq!(s.y, vec![vec![1.0]]);
                }
            }
        }
    }
}
#[test]
fn fatal_on_last_budget_attempt_overrides_budget_and_skips_events() {
    let c = Calls::default();
    let p = problem(Route::FastDense, Callback::Rhs, 1, false, &c).event(
        |_, _| panic!("event callback after fatal"),
        Direction::Either,
    );
    let s = Rodas5p::solve(
        &p,
        (0.0, 1.0),
        &[1.0],
        &Options {
            max_steps: 1,
            ..options(Route::FastDense)
        },
    )
    .unwrap();
    assert_eq!(s.stats.attempts, 1);
    assert_eq!(s.stats.event_evaluations, 0);
    assert_eq!(s.stats.stop.unwrap().reason, StopReason::FatalCallback);
}

#[test]
fn fatal_interior_sink_rejects_candidate_and_keeps_initial_prefix() {
    for route in [
        Route::FastDense,
        Route::Sequential,
        Route::SequentialEnforce,
    ] {
        let o = options(route);
        let clean = Rodas5p::solve(
            &problem(route, Callback::Rhs, u64::MAX, false, &Calls::default()),
            (0.0, 2.0),
            &[1.0],
            &o,
        )
        .unwrap();
        assert!(clean.is_success());
        let midpoint = 0.5 * clean.t[1];
        let calls = Arc::new(AtomicU64::new(0));
        let seen = calls.clone();
        let fatal = Arc::new(AtomicBool::new(false));
        let latch = fatal.clone();
        let mut p = Problem::new(1, move |t, y, out| {
            assert!(
                !latch.load(Ordering::SeqCst),
                "callback after fatal interior probe"
            );
            seen.fetch_add(1, Ordering::SeqCst);
            if t.to_bits() == midpoint.to_bits() {
                latch.store(true, Ordering::SeqCst);
                return Err(CallbackError::Fatal("interior sink".into()));
            }
            out[0] = -y[0];
            Ok(())
        })
        .jacobian(Jacobian::dense(|_, _, out| {
            out[0] = -1.0;
            Ok(())
        }))
        .autonomous();
        if matches!(route, Route::Sequential) {
            p = p.mass(vec![1.0]);
        }
        let s = Rodas5p::solve(&p, (0.0, 2.0), &[1.0], &o).unwrap();
        assert!(fatal.load(Ordering::SeqCst), "{route:?}");
        assert_eq!(s.stats.attempts, 1);
        assert_eq!(s.stats.accepted_steps, 0);
        assert_eq!(s.stats.rejected_steps, 1);
        assert_eq!(s.t, vec![0.0]);
        assert_eq!(s.y, vec![vec![1.0]]);
        assert_eq!(s.stats.callbacks.fatal_errors, 1);
        assert_eq!(s.stats.callbacks.short_circuited_calls, 0);
        assert_eq!(s.stats.counters.rhs_calls, calls.load(Ordering::SeqCst));
        assert_eq!(
            s.stats.stop.as_ref().unwrap().reason,
            StopReason::FatalCallback
        );
        match &s.stats.driver_work {
            vigilode::DriverWork::SequentialDense {
                estimate_failures, ..
            } => assert_eq!(*estimate_failures, 1),
            vigilode::DriverWork::FastDense { .. } => {}
            other => panic!("unexpected sink route {other:?}"),
        }
    }
}
