//! Same-window return sentinel and the incomplete-comparator lower bound
//! (NEXT_STEPS `return-evidence-renewal`). The fixture relaxes from stiff
//! decay to a nonstiff, accuracy-limited oscillation; its exact solution is
//! y = g(t) for every damping, so a cheap implicit history coexists with a
//! current explicit win.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingEvidence, SwitchingMethod, SwitchingSegment, SwitchingSolution, SwitchingSolver,
};

fn lam(t: f64) -> (f64, f64) {
    let q = (200.0 * (t - 0.5)).tanh();
    (1.0 + 4999.5 * (1.0 - q), -4999.5 * 200.0 * (1.0 - q * q))
}
fn g(t: f64) -> (f64, f64, f64) {
    let (a, w) = (1e-2, 150.0);
    let q = (200.0 * (t - 0.5)).tanh();
    let (s, s1, s2) = (
        0.5 * (1.0 + q),
        100.0 * (1.0 - q * q),
        -40000.0 * q * (1.0 - q * q),
    );
    let (sn, cs) = (w * t).sin_cos();
    (
        t.cos() + a * s * sn,
        -t.sin() + a * (s1 * sn + s * w * cs),
        -t.cos() + a * (s2 * sn + 2.0 * s1 * w * cs - s * w * w * sn),
    )
}
// `fail_rhs_call` injects a fatal RHS error at that one-based call.
fn stale_return(fail_rhs_call: Option<usize>, jac_calls: Arc<AtomicUsize>) -> Problem {
    let calls = AtomicUsize::new(0);
    Problem::new(1, move |t, y, o| {
        let k = calls.fetch_add(1, Ordering::Relaxed) + 1;
        if fail_rhs_call == Some(k) {
            return Err(CallbackError::Fatal(
                "injected sentinel return error".into(),
            ));
        }
        let (g0, g1, _) = g(t);
        o[0] = -lam(t).0 * (y[0] - g0) + g1;
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, _, j| {
        jac_calls.fetch_add(1, Ordering::Relaxed);
        j[0] = -lam(t).0;
        Ok(())
    }))
    .time_derivative(|t, y, o| {
        let (l, lp) = lam(t);
        let (g0, g1, g2) = g(t);
        o[0] = -lp * (y[0] - g0) + l * g1 + g2;
        Ok(())
    })
}
fn options() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 100_000,
        ..Default::default()
    }
}
// The previous-window rate screens the return: ratio 0.5 of the cheap
// stiff history is below the current explicit window work. Lifting the
// fixed probe attempt budget isolates that screen.
fn screened(sentinel: usize) -> SwitchingConfig {
    SwitchingConfig {
        return_cost_ratio: 0.5,
        probe_attempt_budget: usize::MAX,
        return_sentinel_interval: sentinel,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    }
}
fn solve(c: &SwitchingConfig, o: &Options, fail: Option<usize>) -> SwitchingSolution {
    SwitchingSolver::solve(
        &stale_return(fail, Arc::new(AtomicUsize::new(0))),
        (0.0, 1.0),
        &[g(0.0).0],
        o,
        c,
    )
    .unwrap()
}
fn total_work(r: &SwitchingSolution) -> f64 {
    r.stats.segments.iter().map(|s| s.work_units).sum()
}
fn accounted(r: &SwitchingSolution) {
    assert!(r.stats.accounting_complete);
    assert_eq!(
        r.stats.attempts,
        r.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
    assert_eq!(
        r.stats.rhs_calls,
        r.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
    );
    assert_eq!(
        r.stats.linear_solves,
        r.stats
            .segments
            .iter()
            .map(|s| s.linear_solves)
            .sum::<u64>()
    );
}
fn sentinel_pairs(r: &SwitchingSolution) -> Vec<(&SwitchingSegment, &SwitchingSegment)> {
    r.stats
        .segments
        .windows(2)
        .filter(|w| {
            w[0].decision.starts_with("sentinel implicit") && w[1].method == SwitchingMethod::Dopri5
        })
        .map(|w| (&w[0], &w[1]))
        .collect()
}

#[test]
fn sentinel_is_opt_in_and_off_in_every_factory() {
    assert_eq!(SwitchingConfig::default().return_sentinel_interval, 0);
    for f in [
        DerivativeFallback::Forbidden,
        DerivativeFallback::ProvidedOnly,
    ] {
        assert_eq!(
            SwitchingConfig::evidence_driven(f).return_sentinel_interval,
            0
        );
        assert_eq!(
            SwitchingConfig::adaptive_evidence(f).return_sentinel_interval,
            0
        );
    }
}

#[test]
fn stale_rate_screen_misses_the_current_explicit_win_that_the_sentinel_recovers() {
    let o = options();
    let exact = g(1.0).0;
    let base = solve(&screened(0), &o, None);
    assert!(base.success, "{}", base.message);
    assert!(base
        .stats
        .segments
        .iter()
        .any(|s| s.decision == "bounded explicit probe discarded: exceeded return work ceiling"));
    assert!(
        !base
            .stats
            .segments
            .iter()
            .any(|s| s.committed && s.method == SwitchingMethod::Dopri5 && s.start_time >= 0.5)
    );

    let r = solve(&screened(1), &o, None);
    assert!(r.success, "{}", r.message);
    assert!(
        (r.final_state[0] - exact).abs() <= 1e-5,
        "{}",
        (r.final_state[0] - exact).abs()
    );
    let pairs = sentinel_pairs(&r);
    let (implicit, explicit) = pairs
        .iter()
        .copied()
        .find(|(_, e)| e.committed)
        .expect("a sentinel explicit return");
    assert_eq!(
        explicit.evidence,
        SwitchingEvidence::ExplicitCheaperOnSameWindow
    );
    assert!(implicit.is_probe && !implicit.committed && implicit.success);
    assert_eq!(implicit.method, SwitchingMethod::Rodas5p);
    assert_eq!(
        (implicit.start_time, implicit.target_time),
        (explicit.start_time, explicit.target_time)
    );
    assert!(explicit.work_units <= 0.5 * implicit.work_units);
    assert_eq!(
        (
            explicit.jacobian_builds,
            explicit.jvp_vectors,
            explicit.ft_calls
        ),
        (0, 0, 0)
    );
    // The discarded implicit window is charged, and the whole run still costs less.
    accounted(&r);
    assert!(
        total_work(&r) < total_work(&base),
        "{} {}",
        total_work(&r),
        total_work(&base)
    );
}

#[test]
fn losing_sentinel_return_is_bounded_by_current_implicit_work() {
    // Ratio 0.5 makes the explicit return lose on the stiff relaxation windows.
    let c = SwitchingConfig {
        window_count: 16,
        implicit_residence: 1,
        probe_interval: 1,
        ..screened(1)
    };
    let r = solve(&c, &options(), None);
    assert!(r.success, "{}", r.message);
    assert!((r.final_state[0] - g(1.0).0).abs() <= 1e-5);
    let pairs = sentinel_pairs(&r);
    let losing: Vec<_> = pairs.iter().filter(|(_, e)| !e.committed).collect();
    assert!(!losing.is_empty());
    for (implicit, explicit) in losing {
        assert!(implicit.committed && implicit.success);
        assert_eq!(
            implicit.evidence,
            SwitchingEvidence::ImplicitPreferredOnSameWindow
        );
        assert_eq!(
            explicit.evidence,
            SwitchingEvidence::ExplicitReturnNotSupported
        );
        // Ceiling floor(0.5 W) + 1, checked after each attempt: at most one
        // further DOPRI attempt (seven RHS calls) beyond it.
        let ceiling = (0.5 * implicit.work_units).floor() as u64 + 1;
        assert!(
            explicit.rhs_calls <= ceiling + 7,
            "{} {}",
            explicit.rhs_calls,
            ceiling
        );
    }
    accounted(&r);
}

#[test]
fn incomplete_implicit_comparator_above_explicit_work_does_not_retain_implicit() {
    // Without the sentinel, the probe passes the rate screen while the
    // replay exhausts its own attempt budget. Its charged work is a lower
    // bound on completing the window.
    let c = SwitchingConfig {
        return_cost_ratio: 0.8,
        ..screened(0)
    };
    let r = solve(&c, &options(), None);
    assert!(r.success, "{}", r.message);
    let i = r
        .stats
        .segments
        .iter()
        .position(|s| {
            s.decision
                == "same-window implicit comparator incomplete above explicit work; discarded"
        })
        .expect("incomplete comparator exercised");
    let (explicit, replay) = (&r.stats.segments[i - 1], &r.stats.segments[i]);
    assert!(!replay.success && !replay.committed);
    assert!(explicit.committed && explicit.method == SwitchingMethod::Dopri5);
    assert_eq!(
        explicit.decision,
        "explicit return passed same-window lower-bound comparison"
    );
    assert_eq!(
        explicit.evidence,
        SwitchingEvidence::ExplicitCheaperOnSameWindow
    );
    assert!(explicit.work_units <= 0.8 * replay.work_units);
    assert!((r.final_state[0] - g(1.0).0).abs() <= 1e-5);
    accounted(&r);
}

#[test]
fn fatal_sentinel_explicit_trial_keeps_only_the_completed_implicit_window() {
    let o = options();
    let c = screened(1);
    let full = solve(&c, &o, None);
    let k = full
        .stats
        .segments
        .iter()
        .position(|s| s.decision.starts_with("sentinel implicit"))
        .unwrap();
    // Fail the third RHS call of the explicit trial that follows the window.
    let before: u64 = full.stats.segments[..=k].iter().map(|s| s.rhs_calls).sum();
    let window = &full.stats.segments[k];
    let r = solve(&c, &o, Some(before as usize + 3));
    assert!(!r.success);
    assert!(
        r.message.contains("fatal sentinel explicit"),
        "{}",
        r.message
    );
    // The implicit window completed before the failing trial: it is published,
    // the fatal explicit trial is not, and nothing runs afterwards.
    assert_eq!(r.final_time, window.target_time);
    let tail = &r.stats.segments[r.stats.segments.len() - 2..];
    assert!(tail[0].committed && tail[0].method == SwitchingMethod::Rodas5p);
    assert!(!tail[1].committed && tail[1].method == SwitchingMethod::Dopri5);
    assert_eq!(*r.t.last().unwrap(), window.target_time);
    accounted(&r);
}

#[test]
fn exhausted_budget_inside_the_sentinel_window_keeps_accepted_state() {
    let c = screened(1);
    let mut o = options();
    let full = solve(&c, &o, None);
    let k = full
        .stats
        .segments
        .iter()
        .position(|s| s.decision.starts_with("sentinel implicit"))
        .unwrap();
    let before: usize = full.stats.segments[..k].iter().map(|s| s.attempts).sum();
    o.max_steps = before + 3;
    let r = solve(&c, &o, None);
    assert!(!r.success);
    assert_eq!(r.stats.attempts, o.max_steps);
    let last = r.stats.segments.last().unwrap();
    assert_eq!(
        last.decision,
        "sentinel implicit window stopped; preserve partial accepted state"
    );
    assert!(last.committed);
    assert_eq!(r.final_time, last.reached_time);
    assert!(r.final_time >= full.stats.segments[k].start_time);
    accounted(&r);
}

#[test]
fn every_second_recheck_is_a_sentinel_and_the_count_restarts_each_tenure() {
    // Ratio 0.1 keeps the stiff relaxation implicit through several rechecks.
    let c = SwitchingConfig {
        window_count: 16,
        implicit_residence: 1,
        probe_interval: 1,
        probe_backoff_factor: 1,
        return_cost_ratio: 0.1,
        ..screened(2)
    };
    let r = solve(&c, &options(), None);
    assert!(r.success, "{}", r.message);
    // Classify every due recheck of each implicit tenure in order.
    let mut tenures: Vec<Vec<bool>> = vec![Vec::new()];
    for (i, s) in r.stats.segments.iter().enumerate() {
        if s.decision.starts_with("sentinel implicit") {
            tenures.last_mut().unwrap().push(true);
        } else if s.is_probe
            && s.method == SwitchingMethod::Dopri5
            && !(i > 0
                && r.stats.segments[i - 1]
                    .decision
                    .starts_with("sentinel implicit"))
        {
            tenures.last_mut().unwrap().push(false);
        }
        if s.committed && s.method == SwitchingMethod::Dopri5 && s.is_probe {
            tenures.push(Vec::new());
        }
    }
    let checked: Vec<_> = tenures.iter().filter(|t| t.len() >= 2).collect();
    assert!(!checked.is_empty(), "{tenures:?}");
    for t in checked {
        for (j, sentinel) in t.iter().enumerate() {
            assert_eq!(*sentinel, j % 2 == 0, "{tenures:?}");
        }
    }
    accounted(&r);
}
