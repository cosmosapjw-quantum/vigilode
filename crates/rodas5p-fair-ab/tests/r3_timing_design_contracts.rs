//! Design contract of the paired timing statistic (re-audit R3 of
//! 2026-10-01, STAT-DEV-04): duplicated sessions and multiplied pairs cannot
//! manufacture authority, and the coverage simulator is deterministic.

use rodas5p_fair_ab::{
    CoverageScenario, DependenceModel, Missingness, PairedTimingCase, PairedTimingDecision,
    PairedTimingProtocol, TimingDesign, case_clustered_bootstrap, coverage_study,
    paired_timing_decision, preregistered_coverage_grid, simulate_corpus,
};

fn design(sessions: usize, cases: usize, missingness: Missingness) -> TimingDesign {
    TimingDesign {
        sessions,
        cases,
        pairs_per_cell: 30,
        theta_log: 1.3_f64.ln(),
        dependence: DependenceModel::PREREGISTERED,
        missingness,
    }
}

fn protocol(seed: u64) -> PairedTimingProtocol {
    PairedTimingProtocol {
        bootstrap_resamples: 2_000,
        ..PairedTimingProtocol::authoritative(seed)
    }
}

/// Rebuilds a case from `(label, candidate, reference)` triples.
fn rebuild(
    case: &PairedTimingCase,
    protocol: &PairedTimingProtocol,
    pairs: Vec<(u32, f64, f64)>,
) -> PairedTimingCase {
    PairedTimingCase::from_samples(
        case.case_id.clone(),
        protocol,
        case.warmup_seconds.clone(),
        pairs.iter().map(|pair| pair.1).collect(),
        pairs.iter().map(|pair| pair.2).collect(),
    )
    .unwrap()
    .with_process_blocks(pairs.iter().map(|pair| pair.0).collect())
}

fn triples(case: &PairedTimingCase) -> Vec<(u32, f64, f64)> {
    case.process_blocks
        .iter()
        .zip(&case.candidate_seconds)
        .zip(&case.reference_seconds)
        .map(|((label, candidate), reference)| (*label, *candidate, *reference))
        .collect()
}

#[test]
fn multiplying_pairs_within_a_session_changes_no_bit() {
    let protocol = protocol(11);
    let corpus = simulate_corpus(&design(6, 5, Missingness::None), &protocol, 3).unwrap();
    let genuine = case_clustered_bootstrap(&corpus.cases, &protocol).unwrap();
    for factor in [2, 3] {
        let multiplied = corpus
            .cases
            .iter()
            .map(|case| {
                let pairs = triples(case)
                    .into_iter()
                    .flat_map(|pair| std::iter::repeat_n(pair, factor))
                    .collect();
                rebuild(case, &protocol, pairs)
            })
            .collect::<Vec<_>>();
        let interval = case_clustered_bootstrap(&multiplied, &protocol).unwrap();
        assert_eq!(interval, genuine, "factor {factor}");
        assert_eq!(
            paired_timing_decision(&interval, protocol.required_speedup),
            paired_timing_decision(&genuine, protocol.required_speedup)
        );
    }
}

#[test]
fn duplicating_a_session_does_not_add_an_independent_session() {
    let protocol = protocol(12);
    // Five sessions: below the six-session floor.
    let corpus = simulate_corpus(&design(5, 5, Missingness::None), &protocol, 4).unwrap();
    let genuine = case_clustered_bootstrap(&corpus.cases, &protocol).unwrap();
    assert_eq!(genuine.independent_blocks, 5);
    assert_eq!(
        paired_timing_decision(&genuine, protocol.required_speedup),
        PairedTimingDecision::Inconclusive
    );
    // Re-running a session under its own label adds pairs, not sessions.
    let duplicated = corpus
        .cases
        .iter()
        .map(|case| {
            let mut pairs = triples(case);
            let first = pairs
                .iter()
                .filter(|pair| pair.0 == 0)
                .copied()
                .collect::<Vec<_>>();
            pairs.extend(first);
            rebuild(case, &protocol, pairs)
        })
        .collect::<Vec<_>>();
    let interval = case_clustered_bootstrap(&duplicated, &protocol).unwrap();
    assert_eq!(interval.independent_blocks, 5);
    assert_eq!(
        paired_timing_decision(&interval, protocol.required_speedup),
        PairedTimingDecision::Inconclusive
    );
}

#[test]
fn the_simulator_follows_the_declared_design() {
    let protocol = protocol(13);
    let balanced = simulate_corpus(&design(6, 5, Missingness::None), &protocol, 5).unwrap();
    assert_eq!(balanced.cases.len(), 5);
    assert_eq!(balanced.missing_cells, 0);
    assert_eq!(balanced.target_log, 1.3_f64.ln());
    for case in &balanced.cases {
        case.admit(&protocol).unwrap();
        assert_eq!(case.candidate_seconds.len(), 6 * 30);
        for session in 0..6 {
            assert_eq!(
                case.process_blocks
                    .iter()
                    .filter(|label| **label == session)
                    .count(),
                30
            );
        }
    }
    // Deterministic in the seed.
    let again = simulate_corpus(&design(6, 5, Missingness::None), &protocol, 5).unwrap();
    assert_eq!(again.cases, balanced.cases);

    let missing = simulate_corpus(
        &design(12, 5, Missingness::Mcar { probability: 0.3 }),
        &protocol,
        6,
    )
    .unwrap();
    let present = missing
        .cases
        .iter()
        .map(|case| case.candidate_seconds.len() / 30)
        .sum::<usize>();
    assert_eq!(present + missing.missing_cells, 12 * 5);
    assert!(missing.missing_cells > 0);

    // A case with no admissible cell is excluded, not imputed.
    let lone = simulate_corpus(
        &design(1, 3, Missingness::Mcar { probability: 0.99 }),
        &protocol,
        7,
    )
    .unwrap();
    assert_eq!(lone.cases.len() + lone.excluded_cases, 3);
    assert!(lone.excluded_cases > 0);

    // A cell shorter than the protocol is not a design.
    let short = TimingDesign {
        pairs_per_cell: 29,
        ..design(6, 1, Missingness::None)
    };
    assert!(simulate_corpus(&short, &protocol, 8).is_err());
}

#[test]
fn the_study_is_deterministic_and_thread_count_free() {
    let base = PairedTimingProtocol {
        bootstrap_resamples: 300,
        ..PairedTimingProtocol::authoritative(0)
    };
    let grid = preregistered_coverage_grid(&PairedTimingProtocol::authoritative(0));
    assert_eq!(grid.len(), 48);
    assert_eq!(grid.iter().filter(|scenario| scenario.primary).count(), 36);
    let scenarios = grid
        .into_iter()
        .filter(|scenario| {
            scenario.id == "s6-c5-mcar0.1-theta1.15" || scenario.id == "s12-c1-none-theta1.3"
        })
        .collect::<Vec<CoverageScenario>>();
    assert_eq!(scenarios.len(), 2);
    let one = coverage_study(&scenarios, 6, 99, 1, &base).unwrap();
    let three = coverage_study(&scenarios, 6, 99, 3, &base).unwrap();
    assert_eq!(
        serde_json::to_string(&one).unwrap(),
        serde_json::to_string(&three).unwrap()
    );
    assert_eq!(one.scenarios[0].replications, 6);
    assert!(one.scenarios[0].false_promote_rate.is_some());
    assert!(one.scenarios[1].false_promote_rate.is_none());
    // Fewer than 4239 resamples can never promote (STAT-DEV-03).
    assert!(one.scenarios.iter().all(|scenario| scenario.promote == 0));
}
