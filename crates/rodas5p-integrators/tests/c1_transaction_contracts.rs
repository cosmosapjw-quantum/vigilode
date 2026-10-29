//! Contracts for the C1 transaction module (plan §6 C1, T8 Proposition 8):
//! commit/rollback/drop accounting, budget reservation under concurrency,
//! the admission rule's bounded-overhead property and the telemetry record.

use std::sync::Arc;
use std::thread;

use rodas5p_core::WorkCounters;
use rodas5p_integrators::transaction::{
    Admission, AdmissionConfig, AdmissionError, AdmissionRule, AttemptRecord, Budget,
    BudgetExhausted, CandidateCosts, RegimeBin, RegimeTelemetry, RitzSummary, Transaction,
    TrialOutcome, callback_work_units,
};

#[derive(Clone, Debug, PartialEq)]
struct Committed {
    y: Vec<f64>,
    h: f64,
    history: Vec<f64>,
}

fn committed() -> Committed {
    Committed {
        y: vec![1.0, 2.0],
        h: 0.1,
        history: vec![0.5],
    }
}

fn work(rhs: u64, jvp: u64) -> WorkCounters {
    WorkCounters {
        rhs_calls: rhs,
        rhs_evaluations: rhs,
        jvp_calls: jvp,
        jvp_vectors: jvp,
        ..WorkCounters::default()
    }
}

fn sum(a: WorkCounters, b: WorkCounters) -> WorkCounters {
    let mut out = a;
    out.accumulate(b);
    out
}

// ---------------------------------------------------------------- Transaction

#[test]
fn commit_returns_the_trial_state_and_counts_its_work_as_committed() {
    let base = committed();
    let mut tx = Transaction::<Committed>::new();
    let mut trial = tx.begin(&base);
    trial.state_mut().y[0] = 3.0;
    trial.state_mut().history.push(0.6);
    assert_eq!(trial.charge(work(8, 3)), 11);
    let next = trial.commit();
    assert_eq!(next.y, vec![3.0, 2.0]);
    assert_eq!(next.history, vec![0.5, 0.6]);
    assert_eq!(
        base,
        committed(),
        "begin clones; the committed state is untouched"
    );

    let ledger = tx.ledger();
    assert_eq!(ledger.trials_begun, 1);
    assert_eq!(ledger.trials_committed, 1);
    assert_eq!(ledger.trials_rolled_back, 0);
    assert_eq!(ledger.attempted_work, work(8, 3));
    assert_eq!(ledger.committed_work, work(8, 3));
    assert_eq!(ledger.rolled_back_work, WorkCounters::default());
    assert_eq!(ledger.attempted_units, 11);
    assert_eq!(ledger.committed_units, 11);
    assert!(ledger.is_balanced());
}

#[test]
fn rollback_discards_the_state_but_keeps_the_work_attempted() {
    let base = committed();
    let mut tx = Transaction::<Committed>::new();
    let mut trial = tx.begin(&base);
    trial.state_mut().h = 99.0;
    trial.charge(work(5, 0));
    trial.charge(work(2, 4));
    trial.rollback();
    assert_eq!(base.h, 0.1);

    let ledger = tx.ledger();
    assert_eq!(ledger.trials_rolled_back, 1);
    assert_eq!(ledger.trials_dropped, 0);
    assert_eq!(ledger.attempted_work, work(7, 4));
    assert_eq!(ledger.rolled_back_work, work(7, 4));
    assert_eq!(ledger.committed_work, WorkCounters::default());
    assert_eq!(ledger.attempted_units, 11);
    assert_eq!(ledger.rolled_back_units, 11);
    assert_eq!(ledger.committed_units, 0);
    assert_eq!(ledger.last_outcome, Some(TrialOutcome::RolledBack));
    assert!(ledger.is_balanced());
}

#[test]
fn dropping_a_trial_is_a_counted_rollback() {
    let base = committed();
    let mut tx = Transaction::<Committed>::new();
    {
        let mut trial = tx.begin(&base);
        trial.charge(work(4, 1));
        // Early return / `?` / panic-free drop without a decision.
    }
    let ledger = tx.ledger();
    assert_eq!(ledger.trials_begun, 1);
    assert_eq!(ledger.trials_rolled_back, 1);
    assert_eq!(ledger.trials_dropped, 1);
    assert_eq!(ledger.trials_committed, 0);
    assert_eq!(ledger.rolled_back_work, work(4, 1));
    assert_eq!(ledger.last_outcome, Some(TrialOutcome::Dropped));
    assert!(ledger.is_balanced());
}

#[test]
fn attempted_equals_committed_plus_rolled_back_over_a_mixed_sequence() {
    let mut state = committed();
    let mut tx = Transaction::<Committed>::new();
    let mut expected_attempted = WorkCounters::default();
    for i in 0..60_u64 {
        let w = work(1 + i % 7, i % 3);
        expected_attempted.accumulate(w);
        let mut trial = tx.begin(&state);
        trial.charge(w);
        trial.state_mut().y[1] += 1.0;
        match i % 3 {
            0 => state = trial.commit(),
            1 => trial.rollback(),
            _ => drop(trial),
        }
    }
    let l = tx.ledger();
    assert_eq!(l.trials_begun, 60);
    assert_eq!(l.trials_committed, 20);
    assert_eq!(l.trials_rolled_back, 40);
    assert_eq!(l.trials_dropped, 20);
    assert_eq!(l.trials_begun, l.trials_committed + l.trials_rolled_back);
    assert_eq!(l.attempted_work, expected_attempted);
    assert_eq!(
        l.attempted_work,
        sum(l.committed_work, l.rolled_back_work),
        "attempted = committed + rolled back"
    );
    assert_eq!(l.attempted_units, l.committed_units + l.rolled_back_units);
    assert!(l.is_balanced());
    assert_eq!(
        state.y[1],
        2.0 + 20.0,
        "only committed trials move the state"
    );
}

#[test]
fn a_custom_work_measure_sets_the_units() {
    fn factorizations_weighted(w: &WorkCounters) -> u64 {
        w.rhs_evaluations + 50 * w.direct_factorizations
    }
    let mut tx = Transaction::<Committed>::with_measure(factorizations_weighted);
    let base = committed();
    let mut trial = tx.begin(&base);
    let w = WorkCounters {
        rhs_evaluations: 3,
        direct_factorizations: 2,
        ..WorkCounters::default()
    };
    assert_eq!(trial.charge(w), 103);
    assert_eq!(trial.charged_units(), 103);
    assert_eq!(trial.charged_work(), w);
    let _ = trial.commit();
    assert_eq!(tx.ledger().committed_units, 103);
    assert_eq!(callback_work_units(&work(3, 4)), 7);
}

#[test]
fn trial_reservations_are_recorded_and_refusals_counted() {
    let budget = Budget::new(10);
    let mut tx = Transaction::<Committed>::new();
    let base = committed();
    let mut trial = tx.begin(&base);
    trial.reserve(&budget, 6).unwrap();
    let refused = trial.reserve(&budget, 5).unwrap_err();
    assert_eq!(
        refused,
        BudgetExhausted {
            cap: 10,
            used: 6,
            requested: 5
        }
    );
    trial.reserve(&budget, 4).unwrap();
    assert_eq!(trial.reserved_units(), 10);
    trial.rollback();
    let l = tx.ledger();
    assert_eq!(l.reserved_units, 10);
    assert_eq!(l.reservation_refusals, 1);
    assert_eq!(budget.used(), 10, "rollback never refunds reserved work");
}

#[test]
fn ledger_serializes() {
    let mut tx = Transaction::<Committed>::new();
    let base = committed();
    let mut t = tx.begin(&base);
    t.charge(work(1, 1));
    t.rollback();
    let json = serde_json::to_string(tx.ledger()).unwrap();
    assert!(json.contains("\"trials_rolled_back\":1"));
}

// --------------------------------------------------------------------- Budget

#[test]
fn budget_reserves_up_to_the_cap_exactly_and_reports_work_used() {
    let budget = Budget::new(100);
    assert_eq!(budget.reserve(60), Ok(60));
    assert_eq!(budget.reserve(40), Ok(100));
    assert_eq!(
        budget.reserve(1),
        Err(BudgetExhausted {
            cap: 100,
            used: 100,
            requested: 1
        })
    );
    assert_eq!(budget.reserve(0), Ok(100));
    assert_eq!(budget.used(), 100);
    assert_eq!(budget.remaining(), 0);
    assert_eq!(budget.refusals(), 1);
    let msg = budget.reserve(7).unwrap_err().to_string();
    assert!(msg.contains("100") && msg.contains("7"), "{msg}");
}

#[test]
fn budget_refuses_overflowing_requests() {
    let budget = Budget::new(u64::MAX);
    budget.reserve(u64::MAX - 1).unwrap();
    assert_eq!(
        budget.reserve(2),
        Err(BudgetExhausted {
            cap: u64::MAX,
            used: u64::MAX - 1,
            requested: 2
        })
    );
    assert_eq!(budget.used(), u64::MAX - 1);
}

#[test]
fn concurrent_reservation_never_exceeds_the_cap() {
    for cap in [1_000_u64, 4_093, 50_000] {
        let budget = Arc::new(Budget::new(cap));
        let threads: Vec<_> = (0..4_u64)
            .map(|t| {
                let budget = Arc::clone(&budget);
                thread::spawn(move || {
                    let (mut granted, mut refused) = (0_u64, 0_u64);
                    // Deterministic per-thread request sizes 1..=17.
                    let mut x = 0x9E37_79B9_7F4A_7C15_u64 ^ (t + 1);
                    for _ in 0..20_000 {
                        x ^= x << 13;
                        x ^= x >> 7;
                        x ^= x << 17;
                        let request = 1 + x % 17;
                        match budget.reserve(request) {
                            Ok(used) => {
                                assert!(used <= cap);
                                granted += request;
                            }
                            Err(e) => {
                                assert!(e.used <= cap);
                                assert!(e.used + e.requested > cap);
                                refused += 1;
                            }
                        }
                    }
                    (granted, refused)
                })
            })
            .collect();
        let (mut granted, mut refused) = (0_u64, 0_u64);
        for handle in threads {
            let (g, r) = handle.join().unwrap();
            granted += g;
            refused += r;
        }
        assert!(budget.used() <= cap);
        assert_eq!(
            budget.used(),
            granted,
            "every granted unit is in the ledger"
        );
        assert_eq!(budget.refusals(), refused);
        assert!(
            cap - budget.used() < 17,
            "budget is filled to within one request"
        );
    }
}

// -------------------------------------------------------------- AdmissionRule

/// One protected step = cost `c_f`. Each opportunity either declines (a
/// protected step) or attempts the candidate (cost `c_a + c_v`); a failure is
/// followed by the protected fallback step. Runs until `n_protected`
/// protected steps have been accepted; returns (charged candidate cost,
/// protected cost, attempts).
fn always_failing_run(rule: &mut AdmissionRule, costs: CandidateCosts, n: u64) -> (f64, f64, u64) {
    let bin = RegimeBin(3);
    let (mut charged, mut protected, mut steps, mut attempts) = (0.0, 0.0, 0_u64, 0_u64);
    while steps < n {
        if rule.decide(bin, costs).unwrap().is_admitted() {
            attempts += 1;
            charged += costs.attempt + costs.verify;
            rule.record_attempt(
                bin,
                AttemptRecord {
                    charged_units: costs.attempt + costs.verify,
                    success: false,
                    replaced_protected_units: costs.fallback,
                },
            )
            .unwrap();
        }
        rule.record_protected_step(bin, costs.fallback).unwrap();
        protected += costs.fallback;
        steps += 1;
    }
    (charged, protected, attempts)
}

#[test]
fn always_failing_candidate_costs_at_most_ten_percent_of_the_protected_path() {
    for &(attempt, verify, fallback) in &[
        (0.5_f64, 0.5_f64, 20.0_f64),
        (3.0, 1.0, 10.0),
        (20.0, 5.0, 50.0),
        (8.0, 1.0, 10.0),
        (14.0, 1.0, 10.0),
        (40.0, 2.0, 7.0),
    ] {
        let costs = CandidateCosts {
            attempt,
            verify,
            fallback,
        };
        for n in [100_u64, 1000] {
            let mut rule = AdmissionRule::new(AdmissionConfig::default());
            let (charged, protected, attempts) = always_failing_run(&mut rule, costs, n);
            assert!(
                charged <= 0.10 * protected,
                "tau={} n={n}: charged {charged} > 0.1 x {protected} ({attempts} attempts)",
                costs.threshold().unwrap()
            );
            // The rule still explores: the throttle allows ~n/m attempts.
            let m = costs.throttle_interval(10.0).unwrap();
            assert!(
                attempts >= 1,
                "tau={}: no exploration at all",
                costs.threshold().unwrap()
            );
            assert!(attempts <= n / m + n / 10, "attempts {attempts}, m {m}");
            let st = rule.bin_state(RegimeBin(3)).unwrap();
            assert_eq!(st.failures, attempts);
            assert_eq!(st.successes, 0);
        }
    }
}

#[test]
fn throttle_interval_is_the_ceiling_of_ten_tau() {
    let c = CandidateCosts {
        attempt: 3.0,
        verify: 1.0,
        fallback: 10.0,
    };
    assert_eq!(c.threshold().unwrap(), 0.4);
    assert_eq!(c.throttle_interval(10.0).unwrap(), 4);
    let c = CandidateCosts {
        attempt: 0.1,
        verify: 0.0,
        fallback: 100.0,
    };
    assert_eq!(c.throttle_interval(10.0).unwrap(), 1, "m >= 1");
    let c = CandidateCosts {
        attempt: 25.0,
        verify: 5.0,
        fallback: 10.0,
    };
    assert_eq!(c.throttle_interval(10.0).unwrap(), 30);
}

#[test]
fn always_succeeding_cheaper_candidate_is_admitted_every_time_within_fifty_attempts() {
    for &(attempt, verify, fallback) in &[
        (1.0_f64, 1.0_f64, 10.0_f64),
        (4.0, 1.0, 10.0),
        (7.0, 1.0, 10.0),
    ] {
        let costs = CandidateCosts {
            attempt,
            verify,
            fallback,
        };
        let bin = RegimeBin(0);
        let mut rule = AdmissionRule::new(AdmissionConfig::default());
        let mut decisions = Vec::new();
        let mut attempts = 0_u64;
        while attempts < 50 {
            let d = rule.decide(bin, costs).unwrap();
            decisions.push(d.is_admitted());
            if d.is_admitted() {
                attempts += 1;
                rule.record_attempt(
                    bin,
                    AttemptRecord {
                        charged_units: attempt + verify,
                        success: true,
                        replaced_protected_units: fallback,
                    },
                )
                .unwrap();
            } else {
                rule.record_protected_step(bin, fallback).unwrap();
            }
        }
        // The protected steps needed before the first attempt are bounded by m.
        let m = costs.throttle_interval(10.0).unwrap() as usize;
        let declines = decisions.iter().filter(|a| !**a).count();
        assert!(
            declines <= m,
            "tau={}: {declines} declines > m={m}",
            costs.threshold().unwrap()
        );
        // Admission rate -> 1: every decision after the first attempt admits,
        // and keeps admitting.
        let first = decisions.iter().position(|a| *a).unwrap();
        assert!(decisions[first..].iter().all(|a| *a));
        for _ in 0..500 {
            assert_eq!(rule.decide(bin, costs).unwrap(), Admission::AdmitPosterior);
            rule.record_attempt(
                bin,
                AttemptRecord {
                    charged_units: attempt + verify,
                    success: true,
                    replaced_protected_units: fallback,
                },
            )
            .unwrap();
        }
    }
}

#[test]
fn posterior_is_updated_only_by_charged_attempts() {
    let bin = RegimeBin(1);
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    for _ in 0..10 {
        rule.record_protected_step(bin, 5.0).unwrap();
    }
    assert_eq!(
        rule.posterior(bin),
        (1.0, 1.0),
        "protected steps carry no evidence on p"
    );
    assert_eq!(
        rule.record_attempt(
            bin,
            AttemptRecord {
                charged_units: 0.0,
                success: true,
                replaced_protected_units: 5.0
            }
        ),
        Err(AdmissionError::UnchargedAttempt)
    );
    assert!(matches!(
        rule.record_attempt(
            bin,
            AttemptRecord {
                charged_units: f64::NAN,
                success: true,
                replaced_protected_units: 5.0
            }
        ),
        Err(AdmissionError::InvalidCost(_))
    ));
    assert_eq!(rule.posterior(bin), (1.0, 1.0));
    rule.record_attempt(
        bin,
        AttemptRecord {
            charged_units: 1.0,
            success: true,
            replaced_protected_units: 5.0,
        },
    )
    .unwrap();
    rule.record_attempt(
        bin,
        AttemptRecord {
            charged_units: 1.0,
            success: false,
            replaced_protected_units: 5.0,
        },
    )
    .unwrap();
    assert_eq!(rule.posterior(bin), (2.0, 2.0));
    assert_eq!(
        rule.posterior(RegimeBin(2)),
        (1.0, 1.0),
        "bins are independent"
    );
}

#[test]
fn invalid_costs_are_typed_errors() {
    let rule = AdmissionRule::new(AdmissionConfig::default());
    for bad in [
        CandidateCosts {
            attempt: 1.0,
            verify: 1.0,
            fallback: 0.0,
        },
        CandidateCosts {
            attempt: -1.0,
            verify: 1.0,
            fallback: 3.0,
        },
        CandidateCosts {
            attempt: f64::INFINITY,
            verify: 1.0,
            fallback: 3.0,
        },
    ] {
        assert!(matches!(
            rule.decide(RegimeBin(0), bad),
            Err(AdmissionError::InvalidCost(_))
        ));
    }
}

#[test]
fn upper_confidence_bound_is_monotone_and_inside_the_unit_interval() {
    let bin = RegimeBin(0);
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    let mut previous = rule.upper_confidence_bound(bin);
    assert!(previous > 0.5 && previous < 1.0);
    for _ in 0..200 {
        rule.record_attempt(
            bin,
            AttemptRecord {
                charged_units: 1.0,
                success: false,
                replaced_protected_units: 1.0,
            },
        )
        .unwrap();
        let ucb = rule.upper_confidence_bound(bin);
        assert!(ucb < previous && ucb > 0.0);
        let (a, b) = rule.posterior(bin);
        assert!(ucb >= a / (a + b), "the bound is above the posterior mean");
        previous = ucb;
    }
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    let mut previous = rule.upper_confidence_bound(bin);
    for _ in 0..200 {
        rule.record_attempt(
            bin,
            AttemptRecord {
                charged_units: 1.0,
                success: true,
                replaced_protected_units: 1.0,
            },
        )
        .unwrap();
        let ucb = rule.upper_confidence_bound(bin);
        assert!(ucb > previous && ucb < 1.0);
        previous = ucb;
    }
}

#[test]
fn admission_is_deterministic() {
    let costs = CandidateCosts {
        attempt: 3.0,
        verify: 1.0,
        fallback: 10.0,
    };
    let run = || {
        let mut rule = AdmissionRule::new(AdmissionConfig::default());
        let mut trace = Vec::new();
        for i in 0..300_u64 {
            let bin = RegimeBin((i % 3) as u32);
            let d = rule.decide(bin, costs).unwrap();
            trace.push(d);
            if d.is_admitted() {
                rule.record_attempt(
                    bin,
                    AttemptRecord {
                        charged_units: 4.0,
                        success: i % 5 < 2,
                        replaced_protected_units: 10.0,
                    },
                )
                .unwrap();
            }
            rule.record_protected_step(bin, 10.0).unwrap();
        }
        (trace, serde_json::to_string(&rule).unwrap())
    };
    assert_eq!(run(), run());
}

// ------------------------------------------------------------------ Telemetry

#[test]
fn telemetry_is_plain_serde_data() {
    let t = RegimeTelemetry {
        ritz: Some(RitzSummary {
            spectral_radius_estimate: 1.5e3,
            max_imag_over_real: 12.25,
            ritz_values: 6,
        }),
        refresh_q_estimate: Some(0.31),
        jacobian_drift_estimate: Some(2.5e-4),
        rejection_streak: 2,
    };
    let json = serde_json::to_string(&t).unwrap();
    let back: RegimeTelemetry = serde_json::from_str(&json).unwrap();
    assert_eq!(back, t);
    let empty = RegimeTelemetry::default();
    assert_eq!(empty.ritz, None);
    assert_eq!(empty.rejection_streak, 0);
    let back: RegimeTelemetry =
        serde_json::from_str(&serde_json::to_string(&empty).unwrap()).unwrap();
    assert_eq!(back, empty);
}

#[test]
fn throttle_and_overhead_guard_decline_with_distinct_statuses() {
    let bin = RegimeBin(7);
    // tau = 0.9 is above the Beta(1, 1) bound (0.879): throttled regime, m = 9.
    let costs = CandidateCosts {
        attempt: 8.0,
        verify: 1.0,
        fallback: 10.0,
    };
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    assert!(rule.upper_confidence_bound(bin) <= 0.9);
    for _ in 0..8 {
        assert_eq!(rule.decide(bin, costs).unwrap(), Admission::DeclineThrottle);
        rule.record_protected_step(bin, 10.0).unwrap();
    }
    assert_eq!(rule.decide(bin, costs).unwrap(), Admission::DeclineThrottle);
    rule.record_protected_step(bin, 10.0).unwrap();
    assert_eq!(rule.decide(bin, costs).unwrap(), Admission::AdmitThrottled);

    // tau = 0.2 is below the prior bound: posterior regime, but a fresh bin
    // has no overhead allowance until 2 protected steps have accrued.
    let costs = CandidateCosts {
        attempt: 1.5,
        verify: 0.5,
        fallback: 10.0,
    };
    let mut rule = AdmissionRule::new(AdmissionConfig::default());
    assert_eq!(rule.decide(bin, costs).unwrap(), Admission::DeclineOverhead);
    rule.record_protected_step(bin, 10.0).unwrap();
    assert_eq!(rule.decide(bin, costs).unwrap(), Admission::DeclineOverhead);
    rule.record_protected_step(bin, 10.0).unwrap();
    assert_eq!(rule.decide(bin, costs).unwrap(), Admission::AdmitPosterior);
    assert!(!Admission::DeclineOverhead.is_admitted());
    assert!(!Admission::DeclineThrottle.is_admitted());
}

#[test]
fn invalid_config_is_rejected() {
    for bad in [
        AdmissionConfig {
            prior_alpha: 0.0,
            ..AdmissionConfig::default()
        },
        AdmissionConfig {
            z: f64::NAN,
            ..AdmissionConfig::default()
        },
        AdmissionConfig {
            overhead_factor: 0.5,
            ..AdmissionConfig::default()
        },
    ] {
        assert!(matches!(
            AdmissionRule::try_new(bad),
            Err(AdmissionError::InvalidConfig(_))
        ));
    }
}
