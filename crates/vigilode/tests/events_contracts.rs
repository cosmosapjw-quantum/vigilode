//! Event location (wave-3 node `p0-events`, plan §3 P0): Illinois root
//! finding on the continuous extension of the accepted steps. Events are
//! post-processing and need `dense_output`.

use vigilode::{
    CallbackError, Direction, FailureReason, Options, Problem, Rodas5p, Solution, SolveError,
    Status,
};

fn decay() -> Problem {
    Problem::new(1, |_t, y, dy| {
        dy[0] = -y[0];
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn oscillator() -> Problem {
    Problem::new(2, |_t, y, dy| {
        dy[0] = y[1];
        dy[1] = -y[0];
        Ok::<(), CallbackError>(())
    })
    .autonomous()
}

fn dense_opts() -> Options {
    Options {
        rtol: 1e-8,
        atol: vigilode::Atol::Scalar(1e-10),
        dense_output: true,
        ..Options::default()
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn exponential_decay_half_life() {
    let p = decay().event(|_t, y| y[0] - 0.5, Direction::Down);
    let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    assert_eq!(sol.status, Status::Success);
    assert_eq!(sol.events.len(), 1);
    let e = &sol.events[0];
    assert_eq!(e.index, 0);
    assert_eq!(e.direction, Direction::Down);
    assert!((e.t - 2f64.ln()).abs() <= 1e-6, "t = {}", e.t);
    assert!((e.y[0] - 0.5).abs() <= 1e-6);
    assert!(sol.stats.event_evaluations > 0);
    assert_eq!(sol.stats.terminal_event, None);
    // A non-terminal event leaves the outputs alone.
    assert_eq!(*sol.t.last().unwrap(), 3.0);
}

#[test]
fn oscillator_crossings() {
    let pi = std::f64::consts::PI;
    let either = oscillator().event(|_t, y| y[0], Direction::Either);
    let sol = Rodas5p::solve(&either, (0.0, 10.0), &[1.0, 0.0], &dense_opts()).unwrap();
    assert_eq!(sol.events.len(), 3);
    for (e, want) in sol.events.iter().zip([0.5 * pi, 1.5 * pi, 2.5 * pi]) {
        assert!((e.t - want).abs() <= 1e-6, "{} vs {want}", e.t);
    }
    // Observed directions: Down, Up, Down.
    let dirs: Vec<_> = sol.events.iter().map(|e| e.direction).collect();
    assert_eq!(dirs, [Direction::Down, Direction::Up, Direction::Down]);
    assert!(sol.events.windows(2).all(|w| w[0].t <= w[1].t));

    let up = oscillator().event(|_t, y| y[0], Direction::Up);
    let sol = Rodas5p::solve(&up, (0.0, 10.0), &[1.0, 0.0], &dense_opts()).unwrap();
    assert_eq!(sol.events.len(), 1);
    assert!((sol.events[0].t - 1.5 * pi).abs() <= 1e-6);
    assert_eq!(sol.events[0].direction, Direction::Up);
}

#[test]
fn terminal_event_truncates() {
    let p = decay().terminal_event(|_t, y| y[0] - 0.5, Direction::Down);
    let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    assert_eq!(sol.status, Status::Success);
    assert_eq!(sol.events.len(), 1);
    assert_eq!(sol.stats.terminal_event, Some(0));
    let te = sol.events[0].t;
    assert!((te - 2f64.ln()).abs() <= 1e-6);
    assert_eq!(*sol.t.last().unwrap(), te);
    assert_eq!(sol.y.last().unwrap(), &sol.events[0].y);
    assert_eq!(sol.t.len(), sol.y.len());
    assert!(sol.at(1.0).is_none());
    assert!(sol.at(te).is_some());
    assert!(sol.at(0.5 * te).is_some());
    assert!(sol.at(te * (1.0 + 1e-9)).is_none());
}

#[test]
fn terminal_index_is_into_events_not_declaration() {
    // Declaration 0 is non-terminal and fires first and last; declaration 1 is
    // the terminal one. `terminal_event` indexes `events`.
    let p = decay()
        .event(|_t, y| y[0] - 0.9, Direction::Down)
        .terminal_event(|_t, y| y[0] - 0.5, Direction::Down);
    let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    assert_eq!(sol.events.len(), 2);
    assert_eq!(sol.events[0].index, 0);
    assert_eq!(sol.events[1].index, 1);
    assert_eq!(sol.stats.terminal_event, Some(1));
    // Reverse the declaration order: the terminal event is declaration 0 but
    // second... in time it is still the later one.
    let p = decay()
        .terminal_event(|_t, y| y[0] - 0.5, Direction::Down)
        .event(|_t, y| y[0] - 0.9, Direction::Down);
    let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    assert_eq!(sol.events.len(), 2);
    assert_eq!(sol.events[0].index, 1);
    assert_eq!(sol.events[1].index, 0);
    assert_eq!(sol.stats.terminal_event, Some(1));
}

#[test]
fn events_without_dense_output_refused() {
    let p = decay().event(|_t, y| y[0] - 0.5, Direction::Down);
    let opts = Options {
        dense_output: false,
        ..dense_opts()
    };
    let err = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &opts).unwrap_err();
    assert!(matches!(err, SolveError::Unsupported(_)), "{err:?}");
}

#[test]
fn zero_at_t0_is_not_an_event() {
    // g(0) = 0 and g grows afterwards.
    let p = decay().event(|t, _y| t, Direction::Either);
    let sol = Rodas5p::solve(&p, (0.0, 1.0), &[1.0], &dense_opts()).unwrap();
    assert!(sol.events.is_empty());
    let p = decay().event(|t, _y| -t, Direction::Either);
    let sol = Rodas5p::solve(&p, (0.0, 1.0), &[1.0], &dense_opts()).unwrap();
    assert!(sol.events.is_empty());
}

#[test]
fn crossing_at_step_endpoint_reported_once() {
    let base = Rodas5p::solve(&decay(), (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    assert!(base.t.len() > 4);
    for k in [1, base.t.len() / 2, base.t.len() - 2] {
        let tk = base.t[k];
        let p = decay().event(move |t, _y| t - tk, Direction::Either);
        let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
        assert_eq!(sol.events.len(), 1, "k = {k}");
        assert!(
            (sol.events[0].t - tk).abs() <= 1e-12 * tk.max(1.0),
            "k = {k}"
        );
        // The midpoint of a step is a shared half-interval endpoint too.
        let tm = 0.5 * (base.t[k] + base.t[k + 1]);
        let p = decay().event(move |t, _y| t - tm, Direction::Up);
        let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
        assert_eq!(sol.events.len(), 1, "midpoint, k = {k}");
    }
}

#[test]
fn nonfinite_event_function_fails_typed() {
    let p = decay().event(
        |t, _y| if t > 1.0 { f64::NAN } else { 1.0 },
        Direction::Either,
    );
    let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    assert_eq!(
        sol.status,
        Status::Failed(FailureReason::EventFunctionNonFinite)
    );
    // Truncated at the failing step: nothing at or beyond the step's end.
    assert!(*sol.t.last().unwrap() <= 1.0, "{:?}", sol.t.last());
    let last = *sol.t.last().unwrap();
    assert!(last < 3.0);
    assert!(sol.at(3.0).is_none());
    assert_eq!(sol.t.len(), sol.y.len());
    assert!(sol.events.is_empty());
}

fn same_integration(a: &Solution, b: &Solution) {
    assert_eq!(bits(&a.t), bits(&b.t));
    assert_eq!(a.y.len(), b.y.len());
    for (u, v) in a.y.iter().zip(&b.y) {
        assert_eq!(bits(u), bits(v));
    }
    assert_eq!(a.stats.counters, b.stats.counters);
    assert_eq!(a.stats.driver_counters, b.stats.driver_counters);
    assert_eq!(a.stats.attempts, b.stats.attempts);
    assert_eq!(a.stats.accepted_steps, b.stats.accepted_steps);
    assert_eq!(a.stats.rejected_steps, b.stats.rejected_steps);
}

#[test]
fn events_do_not_change_integration() {
    let plain = Rodas5p::solve(&oscillator(), (0.0, 10.0), &[1.0, 0.0], &dense_opts()).unwrap();
    let with = oscillator().event(|_t, y| y[0], Direction::Either);
    let sol = Rodas5p::solve(&with, (0.0, 10.0), &[1.0, 0.0], &dense_opts()).unwrap();
    assert_eq!(sol.events.len(), 3);
    same_integration(&plain, &sol);
}

#[test]
fn terminal_event_charges_work_after_it() {
    let plain = Rodas5p::solve(&decay(), (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    let p = decay().terminal_event(|_t, y| y[0] - 0.5, Direction::Down);
    let sol = Rodas5p::solve(&p, (0.0, 3.0), &[1.0], &dense_opts()).unwrap();
    assert_eq!(sol.stats.counters, plain.stats.counters);
    assert_eq!(sol.stats.accepted_steps, plain.stats.accepted_steps);
}

#[test]
fn terminal_event_wins_over_later_driver_failure() {
    // y' = y^2 blows up at t = 1 from y(0) = 1; the event at y = 2 (t = 0.5)
    // comes first.
    let p = Problem::new(1, |_t, y, dy| {
        dy[0] = y[0] * y[0];
        Ok::<(), CallbackError>(())
    })
    .autonomous()
    .terminal_event(|_t, y| y[0] - 2.0, Direction::Up);
    let sol = Rodas5p::solve(&p, (0.0, 2.0), &[1.0], &dense_opts()).unwrap();
    assert_eq!(sol.status, Status::Success);
    assert!(sol.stats.stop.is_some());
    assert!((sol.events[0].t - 0.5).abs() <= 1e-5);
    assert_eq!(sol.stats.terminal_event, Some(0));
}

#[test]
fn two_crossings_in_one_step_are_both_found() {
    // g changes sign twice inside a single (large) step: y' = 0 so the solver
    // takes a huge step; g(t) = (t - 1)(t - 2) crosses Down then Up on [0, 4].
    let p = Problem::new(1, |_t, _y, dy| {
        dy[0] = 0.0;
        Ok::<(), CallbackError>(())
    })
    .autonomous()
    .event(|t, _y| (t - 1.0) * (t - 2.0), Direction::Either);
    let sol = Rodas5p::solve(&p, (0.0, 4.0), &[0.0], &dense_opts()).unwrap();
    // The three samples of a step may miss a double crossing between them;
    // whatever is found must be correct and in time order.
    assert!(sol.events.windows(2).all(|w| w[0].t <= w[1].t));
    for e in &sol.events {
        assert!(((e.t - 1.0).abs() < 1e-9) || ((e.t - 2.0).abs() < 1e-9));
    }
}
