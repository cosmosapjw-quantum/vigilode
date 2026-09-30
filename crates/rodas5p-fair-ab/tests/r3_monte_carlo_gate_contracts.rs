//! Monte-Carlo error gate of the paired timing decision (re-audit R3 of
//! 2026-10-01, STAT-DEV-03).

use rodas5p_fair_ab::{
    PAIRED_TIMING_MC_FAILURE_BUDGET, PairedTimingCase, PairedTimingDecision, PairedTimingProtocol,
    case_clustered_bootstrap, monte_carlo_gate,
};

const RATIOS: [f64; 6] = [0.8, 0.9, 1.0, 1.1, 1.4, 1.5];

/// The R3 fixture: six sessions of five identical pairs, one case.
fn fixture(protocol: &PairedTimingProtocol) -> PairedTimingCase {
    let labels = (0..protocol.pairs)
        .map(|pair| (pair * 6 / protocol.pairs) as u32)
        .collect::<Vec<_>>();
    let candidate = vec![1.0e-3; protocol.pairs];
    let reference = labels
        .iter()
        .map(|session| 1.0e-3 * RATIOS[*session as usize])
        .collect();
    PairedTimingCase::from_samples(
        "fixture",
        protocol,
        vec![5.0e-3, 1.0e-3],
        candidate,
        reference,
    )
    .unwrap()
    .with_process_blocks(labels)
}

/// Exact P(T* < ln 1.15) over all 6^6 equally likely session draws: the
/// statistic is the mean of the third and fourth order statistics of the
/// drawn session logs.
fn exact_probability_below(threshold_log: f64) -> (usize, usize) {
    let logs = RATIOS.map(|ratio: f64| (1.0e-3 * ratio / 1.0e-3).ln());
    let mut below = 0;
    let mut total = 0;
    for code in 0..6_usize.pow(6) {
        let mut draw = (0..6)
            .map(|position| logs[(code / 6_usize.pow(position)) % 6])
            .collect::<Vec<_>>();
        draw.sort_by(f64::total_cmp);
        let statistic = 0.5 * (draw[2] + draw[3]);
        total += 1;
        if statistic < threshold_log {
            below += 1;
        }
    }
    (below, total)
}

#[test]
fn the_gate_contains_the_exact_conditional_probability() {
    let protocol = PairedTimingProtocol::authoritative(1);
    let interval = case_clustered_bootstrap(&[fixture(&protocol)], &protocol).unwrap();
    let (below, total) = exact_probability_below(protocol.required_speedup.ln());
    assert_eq!((below, total), (32884, 46656));
    let exact = below as f64 / total as f64;
    let gate = monte_carlo_gate(
        interval.replicates_below_required,
        interval.resamples,
        protocol.confidence_level,
        PAIRED_TIMING_MC_FAILURE_BUDGET,
    );
    assert!(
        gate.lower <= exact && exact <= gate.upper,
        "{gate:?} vs {exact}"
    );
    assert_eq!(gate.decision, PairedTimingDecision::Inconclusive);
}

#[test]
fn atoms_equal_samples_and_the_minimum_resample_count() {
    // An atom at the threshold straddles the 2.5% tail: 100 of 10000
    // replicates strictly below it is not resolved (upper 0.0262762...).
    let atom = monte_carlo_gate(100, 10_000, 0.95, 0.01);
    assert_eq!(atom.decision, PairedTimingDecision::Inconclusive);
    assert!((atom.upper - 0.02627623630718729).abs() < 1.0e-15);
    // All replicates above the threshold, and all below.
    let above = monte_carlo_gate(0, 10_000, 0.95, 0.01);
    assert_eq!(above.decision, PairedTimingDecision::Promote);
    assert!((above.upper - 0.016276236307187292).abs() < 1.0e-15);
    let below = monte_carlo_gate(10_000, 10_000, 0.95, 0.01);
    assert_eq!(below.decision, PairedTimingDecision::Block);
    assert!((below.lower - 0.9837237636928127).abs() < 1.0e-15);
    // Hoeffding needs at least 4239 resamples before anything promotes.
    assert_eq!(
        monte_carlo_gate(0, 4_238, 0.95, 0.01).decision,
        PairedTimingDecision::Inconclusive
    );
    assert_eq!(
        monte_carlo_gate(0, 4_239, 0.95, 0.01).decision,
        PairedTimingDecision::Promote
    );
    // The largest count that still promotes at B = 10000 is 87.
    assert_eq!(
        monte_carlo_gate(87, 10_000, 0.95, 0.01).decision,
        PairedTimingDecision::Promote
    );
    assert_eq!(
        monte_carlo_gate(88, 10_000, 0.95, 0.01).decision,
        PairedTimingDecision::Inconclusive
    );
}
