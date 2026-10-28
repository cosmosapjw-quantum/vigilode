#[path = "../examples/support/production_return_cases.rs"]
mod cases;
use vigilode::{DerivativeFallback, SwitchingConfig, SwitchingMethod};

#[test]
fn exact_return_defaults_off_and_serializes_without_silent_unknown_keys() {
    for c in [
        SwitchingConfig::default(),
        SwitchingConfig::evidence_driven(DerivativeFallback::ProvidedOnly),
        SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly),
    ] {
        assert!(!c.strict_return_work_improvement);
        let mut j = serde_json::to_value(&c).unwrap();
        let restored: SwitchingConfig = serde_json::from_value(j.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), j);
        j["misspelled_strict_return"] = true.into();
        assert!(serde_json::from_value::<SwitchingConfig>(j).is_err());
    }
}

#[test]
fn strict_sentinel_keeps_all_work_and_same_window_exact_wins() {
    let (r, rhs, jvp) = cases::solve(
        "relaxing-nonlinear",
        &cases::config("observed-strict-sentinel"),
        None,
    );
    assert!(r.success, "{}", r.message);
    assert!(cases::max_error("relaxing-nonlinear", &r) <= 1e-5);
    assert_eq!(r.stats.rhs_calls, rhs);
    assert_eq!(r.stats.jvp_vectors, jvp);
    assert!(r.stats.accounting_complete);
    let mut comparisons = 0;
    for pair in r.stats.segments.windows(2) {
        let (i, e) = (&pair[0], &pair[1]);
        if !i.decision.starts_with("sentinel implicit") || e.method != SwitchingMethod::Dopri5 {
            continue;
        }
        comparisons += 1;
        assert!(i.success);
        assert_eq!((i.start_time, i.target_time), (e.start_time, e.target_time));
        if e.committed {
            assert!(e.success && e.work_units < i.work_units);
        } else {
            assert!(i.committed);
            // Native DOPRI checks the exact I−1 cap after a completed attempt.
            // Initial attempt uses seven RHS calls; later attempts use six.
            assert!(e.rhs_calls as f64 <= i.work_units + 6.0);
        }
    }
    assert!(comparisons > 0, "fixture must exercise same-window return");
}

#[test]
fn strict_flags_preserve_forbidden_derivative_policy() {
    let mut c = cases::config("strict-sentinel");
    c.fallback = DerivativeFallback::Forbidden;
    let (r, rhs, jvp) = cases::solve("easy-cubic", &c, None);
    assert!(r.success && r.stats.accounting_complete);
    assert_eq!(jvp, 0);
    assert_eq!(rhs, r.stats.rhs_calls);
    assert!(cases::max_error("easy-cubic", &r) <= 1e-6);
}

#[test]
fn fatal_strict_sentinel_trial_keeps_productive_implicit_window_and_stops() {
    let c = cases::config("observed-strict-sentinel");
    let (base, _, _) = cases::solve("relaxing-nonlinear", &c, None);
    let index = base
        .stats
        .segments
        .windows(2)
        .position(|p| {
            p[0].decision.starts_with("sentinel implicit") && p[1].method == SwitchingMethod::Dopri5
        })
        .expect("sentinel fixture");
    let before_trial: u64 = base.stats.segments[..=index]
        .iter()
        .map(|s| s.rhs_calls)
        .sum();
    let (failed, rhs, jvp) = cases::solve("relaxing-nonlinear", &c, Some(before_trial + 1));
    assert!(!failed.success);
    assert!(failed.message.contains("fatal sentinel explicit return"));
    assert_eq!(failed.final_time, base.stats.segments[index].reached_time);
    assert_eq!(rhs, before_trial + 1);
    assert_eq!(failed.stats.rhs_calls, rhs);
    assert_eq!(failed.stats.jvp_vectors, jvp);
    assert!(failed.stats.accounting_complete);
    let last = failed.stats.segments.last().unwrap();
    assert!(!last.committed && !last.success);
}
