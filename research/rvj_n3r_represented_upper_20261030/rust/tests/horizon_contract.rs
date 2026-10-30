//! N3R causal window cannot pass its source-frozen exact clock horizon.
use rvj_n3_window_source_locked::{Session, Source};

#[test]
fn time_window_refuses_small_positive_overshoot_before_work() {
    let mut s = Session::new(Source::toy(12).unwrap()).unwrap();
    let old = s.state().to_vec();
    let h = 0.5 + 2f64.powi(-48);
    assert!(h > 0.5 && h <= 0.5 + 1e-13);
    assert!(
        s.propose(h, 2).is_err(),
        "positive horizon overshoot accepted"
    );
    assert_eq!(s.state(), &old);
    assert_eq!(s.time(), 0.0);
    assert_eq!(s.work().attempted, 0);
    assert!(s.is_balanced());
}

#[test]
fn late_window_refuses_overshoot_and_exact_endpoint_remains_legal() {
    let mut s = Session::new(Source::toy(12).unwrap()).unwrap();
    let first = s.propose(0.25, 2).unwrap();
    assert_eq!(
        s.decide(first, 1.0).unwrap(),
        "RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION"
    );
    let before = s.state().to_vec();
    let late_h = 0.25 + 2f64.powi(-49);
    assert!(0.25 + late_h > 0.5 && 0.25 + late_h <= 0.5 + 1e-13);
    assert!(
        s.propose(late_h, 2).is_err(),
        "later source-horizon overshoot accepted"
    );
    assert_eq!(s.state(), &before);
    assert_eq!(s.time(), 0.25);
    assert_eq!(s.work().attempted, 1);
    let second = s.propose(0.25, 2).unwrap();
    assert_eq!(
        s.decide(second, 1.0).unwrap(),
        "RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION"
    );
    assert_eq!(s.time(), 0.5);
    assert!(s.is_balanced());
}

#[test]
fn exact_represented_clock_sum_refuses_rounded_to_horizon_overshoot() {
    let mut s = Session::new(Source::toy(12).unwrap()).unwrap();
    let first = s.propose(0.25, 2).unwrap();
    s.decide(first, 1.0).unwrap();
    let before = s.state().to_vec();
    // Here true binary64-as-exact-real addition exceeds T, but nearest rounding gives T.
    let near = 0.25_f64.next_up();
    assert!(near > 0.25);
    assert_eq!(0.25 + near, 0.5);
    assert!(
        s.propose(near, 2).is_err(),
        "rounded sum admitted a true source horizon overshoot"
    );
    assert_eq!(s.state(), &before);
    assert_eq!(s.time(), 0.25);
    assert_eq!(s.work().attempted, 1);
    assert!(s.is_balanced());
}
