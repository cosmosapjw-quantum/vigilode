//! Adversarial public-API probe, baseline bee5563c51264a7da19c519295bdda2fb317caef.
//! Exit zero means the audit probe completed, NOT that the solver passed.
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{ParallelExecution, scalar_linear_problem};
use rodas5p_integrators::ppsw1::{self, Config, Session, State, Tolerances, Decision};

fn state(t: f64, value: f64, h: f64) -> State {
    State { t, y: std::array::from_fn(|_| vec![value]),
        f: std::array::from_fn(|_| vec![0.0]), h_prev: h }
}
fn main() {
    let (p, _) = scalar_linear_problem(0.0, 0.0);
    let mut a = Session::new(&p, state(0.0, 1.0, 0.1), Config::default(), ParallelExecution::sequential()).unwrap();
    let mut b = Session::new(&p, state(0.0, 2.0, 0.1), Config::default(), ParallelExecution::sequential()).unwrap();
    let attempt = b.attempt(0.1).unwrap();
    let foreign_accepted = a.accept(attempt).is_ok();
    println!("foreign_attempt_accepted={} resulting_y={} accepting_session_rhs={} accepting_session_linear_solves={}",
        foreign_accepted, a.state().y[5][0], a.counters().rhs_calls, a.counters().linear_solves);
    assert!(foreign_accepted && a.state().y[5][0] > 1.5);
    let invalid = Session::new(&p, state(f64::NAN, 0.0, 0.1), Config::default(), ParallelExecution::sequential());
    println!("nan_state_constructor_accepted={}", invalid.is_ok());
    let mut nan_session = invalid.unwrap();
    let tol = Tolerances { atol: 1e-6, rtol: 1e-6 };
    let nan_decision = nan_session.try_step(0.1, &tol).unwrap();
    println!("nan_time_step_accepted={} final_time_is_nan={}", matches!(nan_decision, Decision::Accepted { .. }), nan_session.state().t.is_nan());
    assert!(matches!(nan_decision, Decision::Accepted { .. }) && nan_session.state().t.is_nan());
    let mut stalled = Session::new(&p, state(1e16, 0.0, 0.01), Config::default(), ParallelExecution::sequential()).unwrap();
    let decision = stalled.try_step(0.01, &tol).unwrap();
    println!("zero_time_progress_accepted={} before=1e16 after={:.17e}", matches!(decision, Decision::Accepted { .. }), stalled.state().t);
    assert!(matches!(decision, Decision::Accepted { .. }) && stalled.state().t == 1e16);
    let mut counters = WorkCounters::default();
    let starter = ppsw1::start(&p, 1e16, &[0.0], 0.01, &mut counters).unwrap();
    println!("starter_zero_time_progress_accepted={} startup_rhs={}", starter.t == 1e16, counters.rhs_calls);
    assert!(starter.t == 1e16);
    println!("audit_outcome=BASELINE_DEFECTS_REPRODUCED");
}
