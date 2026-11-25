use std::sync::{Arc, Mutex};
use vigilode::{Atol, Direction, EventTransactionPolicy, Options, Problem, Rodas5p, Status};
fn opts() -> Options {
    Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        dense_output: true,
        ..Options::default()
    }
}
#[test]
fn terminal_stops_before_later_rhs_callback() {
    let times = Arc::new(Mutex::new(Vec::<f64>::new()));
    let copy = times.clone();
    let p = Problem::new(2, move |t, y, f| {
        copy.lock().unwrap().push(t);
        f[0] = y[1];
        f[1] = -y[0];
        Ok(())
    })
    .autonomous()
    .terminal_event(|_, y| y[0], Direction::Down);
    let s = Rodas5p::solve_event_transaction(
        &p,
        (0., 8.5),
        &[1., 0.],
        &opts(),
        EventTransactionPolicy::default(),
    )
    .unwrap();
    assert_eq!(s.status, Status::Success);
    assert_eq!(s.events.len(), 1);
    assert!((s.events[0].t - std::f64::consts::FRAC_PI_2).abs() < 2e-5);
    let observed = times.lock().unwrap();
    assert_eq!(observed.len() as u64, s.stats.counters.rhs_calls);
    assert!(
        observed.iter().copied().fold(0., f64::max) < 2.,
        "RHS continued after terminal trial"
    );
    assert_eq!(s.t.last().copied(), Some(s.events[0].t));
}

use std::sync::atomic::{AtomicU64, Ordering};
use vigilode::{CallbackError, EventRootMethod, FailureReason, InteriorError, Jacobian};
fn counted_oscillator(count: Arc<AtomicU64>) -> Problem {
    Problem::new(2, move |_, y, f| {
        count.fetch_add(1, Ordering::Relaxed);
        f[0] = y[1];
        f[1] = -y[0];
        Ok(())
    })
    .autonomous()
}
fn clock() -> Problem {
    Problem::new(1, |_, _, f| {
        f[0] = 0.;
        Ok(())
    })
    .autonomous()
}
fn checked_record(s: &vigilode::Solution) {
    let tx = s.stats.event_transaction.as_ref().unwrap();
    assert_eq!(tx.committed_end, *s.t.last().unwrap());
    assert_eq!(tx.inspected_steps, s.stats.accepted_steps);
    assert!(tx.numerical_end >= tx.committed_end);
    assert_eq!(s.t.len(), s.y.len());
    assert!(s.t.windows(2).all(|w| w[0] < w[1]));
    assert!(s.at(tx.committed_end).is_some());
    assert!(s.at(tx.committed_end.next_up()).is_none());
    assert!(s.events.windows(2).all(|w| w[0].t <= w[1].t));
    assert!(s.events.iter().all(|e| e.t <= tx.committed_end));
    if let Some(i) = s.stats.terminal_event {
        assert_eq!(s.events[i].t, tx.committed_end);
        assert_eq!(&s.events[i].y, s.y.last().unwrap());
        assert_eq!(s.at(tx.committed_end).unwrap(), s.events[i].y);
        assert_eq!(s.status, Status::Success);
    }
}
#[test]
fn exact_ties_direction_and_event_order() {
    let o = Options { h0: 0.75, ..opts() };
    for root_method in [EventRootMethod::Illinois, EventRootMethod::Bisection] {
        let p = clock()
            .terminal_event(|t, _| t - 0.6, Direction::Up)
            .terminal_event(|t, _| t - 0.4, Direction::Up)
            .event(|t, _| t - 0.3, Direction::Up)
            .event(|t, _| t - 0.4, Direction::Up)
            .event(|t, _| t - 0.4, Direction::Down)
            .event(|t, _| t - 0.5, Direction::Up);
        let s = Rodas5p::solve_event_transaction(
            &p,
            (0., 1.),
            &[2.],
            &o,
            EventTransactionPolicy {
                subdivisions: 4,
                root_method,
                max_step: None,
            },
        )
        .unwrap();
        checked_record(&s);
        assert_eq!(
            s.events.iter().map(|e| e.index).collect::<Vec<_>>(),
            vec![2, 1, 3]
        );
        assert_eq!(s.stats.terminal_event, Some(1));
        assert!((s.events[1].t - 0.4).abs() < 2e-15);
        assert_eq!(s.stats.accepted_steps, 1);
        assert_eq!(
            s.stats.event_transaction.as_ref().unwrap().numerical_end,
            0.75
        );
    }
}
#[test]
fn invalid_initial_event_and_invalid_policy_do_not_call_rhs() {
    let n = Arc::new(AtomicU64::new(0));
    let p = counted_oscillator(n.clone()).event(|_, _| f64::NAN, Direction::Either);
    let s = Rodas5p::solve_event_transaction(
        &p,
        (0., 8.5),
        &[1., 0.],
        &opts(),
        EventTransactionPolicy::default(),
    )
    .unwrap();
    assert_eq!(
        s.status,
        Status::Failed(FailureReason::EventFunctionNonFinite)
    );
    assert_eq!(n.load(Ordering::Relaxed), 0);
    assert_eq!(s.stats.event_evaluations, 1);
    checked_record(&s);
    for bad in [
        EventTransactionPolicy {
            subdivisions: 0,
            ..Default::default()
        },
        EventTransactionPolicy {
            max_step: Some(f64::NAN),
            ..Default::default()
        },
    ] {
        assert!(Rodas5p::solve_event_transaction(&p, (0., 8.5), &[1., 0.], &opts(), bad).is_err());
    }
    let no_dense = Options {
        dense_output: false,
        ..opts()
    };
    assert!(
        Rodas5p::solve_event_transaction(&p, (0., 8.5), &[1., 0.], &no_dense, Default::default())
            .is_err()
    );
    let enforce = Options {
        interior_error: InteriorError::Enforce,
        ..opts()
    };
    assert!(
        Rodas5p::solve_event_transaction(&p, (0., 8.5), &[1., 0.], &enforce, Default::default())
            .is_err()
    );
    assert_eq!(n.load(Ordering::Relaxed), 0);
}
#[test]
fn failure_rolls_back_entire_containing_proposal() {
    let p = clock().event(|t, _| t - 0.1, Direction::Up).event(
        |t, _| if t > 0.6 { f64::NAN } else { 1. },
        Direction::Either,
    );
    let o = Options { h0: 0.5, ..opts() };
    let s = Rodas5p::solve_event_transaction(&p, (0., 1.), &[0.], &o, Default::default()).unwrap();
    assert_eq!(
        s.status,
        Status::Failed(FailureReason::EventFunctionNonFinite)
    );
    assert_eq!(s.events.len(), 1);
    assert_eq!(s.t.last().copied(), Some(0.5));
    assert_eq!(
        s.stats.event_transaction.as_ref().unwrap().numerical_end,
        1.
    );
    checked_record(&s);
    let p = clock().event(|t, _| t - 0.1, Direction::Up).event(
        |t, _| if t > 0.4 { f64::NAN } else { 1. },
        Direction::Either,
    );
    let s = Rodas5p::solve_event_transaction(&p, (0., 1.), &[0.], &o, Default::default()).unwrap();
    assert!(s.events.is_empty());
    assert_eq!(s.t, vec![0.]);
    checked_record(&s);
}
#[test]
fn terminal_prevents_future_fatal_callback() {
    let p = Problem::new(1, |t, y, f| {
        if t > 2. {
            return Err(CallbackError::Fatal("unreachable after event".into()));
        }
        f[0] = -y[0];
        Ok(())
    })
    .autonomous()
    .terminal_event(|_, y| y[0] - 0.5, Direction::Down);
    let old = Rodas5p::solve(&p, (0., 3.), &[1.], &opts()).unwrap();
    assert!(matches!(
        old.status,
        Status::Failed(FailureReason::CallbackFailed { .. })
    ));
    let new =
        Rodas5p::solve_event_transaction(&p, (0., 3.), &[1.], &opts(), Default::default()).unwrap();
    checked_record(&new);
    assert!(new.is_success());
    assert_eq!(new.stats.callbacks.fatal_errors, 0);
}
#[test]
fn nonterminal_bitwise_and_zero_at_initial_convention() {
    let n = Arc::new(AtomicU64::new(0));
    let p = counted_oscillator(n)
        .event(|_, y| y[0], Direction::Either)
        .event(|t, _| t, Direction::Either);
    let a = Rodas5p::solve(&p, (0., 8.5), &[1., 0.], &opts()).unwrap();
    let b = Rodas5p::solve_event_transaction(&p, (0., 8.5), &[1., 0.], &opts(), Default::default())
        .unwrap();
    checked_record(&b);
    assert_eq!(a.t, b.t);
    assert_eq!(a.y, b.y);
    assert_eq!(a.stats.counters, b.stats.counters);
    assert_eq!(a.stats.accepted_steps, b.stats.accepted_steps);
    assert_eq!(b.events.len(), 3);
    for (x, y) in a.events.iter().zip(&b.events) {
        assert_eq!(x.index, y.index);
        assert!((x.t - y.t).abs() < 1e-13);
    }
    let z = Rodas5p::solve_event_transaction(&p, (0., 0.), &[1., 0.], &opts(), Default::default())
        .unwrap();
    checked_record(&z);
    assert!(z.is_success());
    assert!(z.events.is_empty());
}
#[test]
fn explicit_banded_and_vector_atol_transaction() {
    let p = clock()
        .jacobian(Jacobian::banded(0, 0, |_, _, a| {
            a[0] = 0.;
            Ok(())
        }))
        .terminal_event(|t, _| t - 0.35, Direction::Up);
    let o = Options {
        atol: Atol::Vector(vec![1e-10]),
        h0: 0.5,
        ..opts()
    };
    let s = Rodas5p::solve_event_transaction(&p, (0., 1.), &[1.], &o, Default::default()).unwrap();
    checked_record(&s);
    assert!(s.is_success());
    assert!((s.events[0].t - 0.35).abs() < 2e-15);
}
#[test]
fn hidden_tangent_is_not_certified() {
    let p = clock().terminal_event(|t, _| (t - 0.331) * (t - 0.331), Direction::Either);
    let s = Rodas5p::solve_event_transaction(
        &p,
        (0., 1.),
        &[0.],
        &Options { h0: 0.75, ..opts() },
        Default::default(),
    )
    .unwrap();
    checked_record(&s);
    assert!(s.events.is_empty());
    assert_eq!(s.t.last().copied(), Some(1.));
}
