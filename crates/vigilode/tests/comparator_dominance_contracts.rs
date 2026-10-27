use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingEvidence, SwitchingMethod, SwitchingSolver,
};

fn problem(fatal_at: Option<f64>) -> Problem {
    Problem::new(1, |t, y, o| {
        let l = 1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh());
        o[0] = -l * (y[0] - t.cos()) - t.sin();
        Ok(())
    })
    .jacobian(Jacobian::dense(move |t, _, j| {
        if fatal_at.is_some_and(|at| at.to_bits() == t.to_bits()) {
            return Err(CallbackError::Fatal(
                "injected dominance comparator failure".into(),
            ));
        }
        j[0] = -(1.0 + 4999.5 * (1.0 - (40.0 * (t - 0.4)).tanh()));
        Ok(())
    }))
    .time_derivative(|t, y, o| {
        let q = (40.0 * (t - 0.4)).tanh();
        let l = 1.0 + 4999.5 * (1.0 - q);
        let lp = -199980.0 * (1.0 - q * q);
        o[0] = -lp * (y[0] - t.cos()) - l * t.sin() - t.cos();
        Ok(())
    })
}
fn opts() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 50000,
        ..Default::default()
    }
}
fn config(stopping: bool, sentinel: usize) -> SwitchingConfig {
    SwitchingConfig {
        stop_dominated_return_comparator: stopping,
        return_sentinel_interval: sentinel,
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    }
}
#[test]
fn stopped_comparator_preserves_the_entire_accepted_trajectory_and_reduces_work() {
    let p = problem(None);
    let o = opts();
    let old = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &o, &config(false, 0)).unwrap();
    let new = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &o, &config(true, 0)).unwrap();
    assert!(old.success && new.success);
    assert_eq!(old.t, new.t);
    assert_eq!(old.y, new.y);
    assert!(new.stats.rhs_calls < old.stats.rhs_calls);
    assert!(new.stats.accounting_complete);
    assert_eq!(
        new.stats.attempts,
        new.stats.segments.iter().map(|s| s.attempts).sum::<usize>()
    );
    assert_eq!(
        new.stats.rhs_calls,
        new.stats.segments.iter().map(|s| s.rhs_calls).sum::<u64>()
    );
    assert!(
        new.t
            .iter()
            .zip(&new.y)
            .all(|(t, y)| (y[0] - t.cos()).abs() < 1e-5)
    );
    let mut found = false;
    for pair in new.stats.segments.windows(2) {
        let e = &pair[0];
        let i = &pair[1];
        if e.evidence != SwitchingEvidence::ExplicitCheaperThanComparatorLowerBound {
            continue;
        }
        found = true;
        assert!(e.success && e.committed);
        assert_eq!(e.method, SwitchingMethod::Dopri5);
        assert!(!i.success && !i.committed);
        assert_eq!(i.method, SwitchingMethod::Rodas5p);
        assert_eq!(e.start_time, i.start_time);
        assert_eq!(e.target_time, i.target_time);
        assert!(e.work_units < config(true, 0).return_cost_ratio * i.work_units);
    }
    assert!(found, "fixture must exercise early dominance stop");
}
#[test]
fn an_observed_fatal_comparator_precedes_dominance_and_retains_checkpoint() {
    let o = opts();
    let c = config(true, 0);
    let full = SwitchingSolver::solve(&problem(None), (0.0, 1.0), &[1.0], &o, &c).unwrap();
    let e = full
        .stats
        .segments
        .iter()
        .find(|s| s.evidence == SwitchingEvidence::ExplicitCheaperThanComparatorLowerBound)
        .unwrap();
    let r =
        SwitchingSolver::solve(&problem(Some(e.start_time)), (0.0, 1.0), &[1.0], &o, &c).unwrap();
    assert!(!r.success);
    assert_eq!(r.final_time, e.start_time);
    assert!(r.message.contains("fatal same-window"));
    assert!(r.stats.accounting_complete);
    assert!(!r.stats.segments.last().unwrap().committed);
}
#[test]
fn implicit_first_sentinel_is_unchanged_by_ordinary_comparator_optimization() {
    let p = problem(None);
    let o = opts();
    let old = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &o, &config(false, 1)).unwrap();
    let new = SwitchingSolver::solve(&p, (0.0, 1.0), &[1.0], &o, &config(true, 1)).unwrap();
    assert!(old.success && new.success);
    assert_eq!(old.t, new.t);
    assert_eq!(old.y, new.y);
    assert_eq!(old.stats.rhs_calls, new.stats.rhs_calls);
    assert_eq!(old.stats.attempts, new.stats.attempts);
    assert!(
        new.stats
            .segments
            .iter()
            .all(|s| s.evidence != SwitchingEvidence::ExplicitCheaperThanComparatorLowerBound)
    );
}
