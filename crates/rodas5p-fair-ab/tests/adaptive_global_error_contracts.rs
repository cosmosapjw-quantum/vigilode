use std::collections::{BTreeMap, BTreeSet};

use rodas5p_fair_ab::{
    AdaptiveOutputMode, AdaptiveOutputPolicyPairStatus, GlobalErrorParetoProfile,
    run_adaptive_global_error_screen,
};

#[test]
fn adaptive_screen_covers_every_current_family_and_preserves_every_run_row() {
    let report = run_adaptive_global_error_screen(GlobalErrorParetoProfile::Smoke, 1).unwrap();
    assert_eq!(report.schema, "rodas5p-adaptive-global-error-v2");
    let ids = report
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ids,
        BTreeSet::from([
            "bdf1-adaptive-reference",
            "bdf2-adaptive-reference",
            "homotopy-rodas5p-q7-adaptive",
            "radau-iia1-adaptive-reference",
            "radau-iia3-adaptive-reference",
            "sabr5p-adaptive",
            "sequential-rodas5p-direct-adaptive",
            "sequential-rodas5p-gcrodr-adaptive",
            "sequential-rodas5p-gmres-adaptive",
            "sequential-rodas5p-lgmres-adaptive",
        ])
    );
    assert_eq!(report.problems.len(), 2);
    assert_eq!(report.tolerance_ladder, vec![1.0e-4, 1.0e-6]);
    assert_eq!(
        report.runs.len(),
        2 * report.candidates.len() * report.problems.len() * report.tolerance_ladder.len()
    );
    assert_eq!(
        report.output_policy_pairs.len(),
        report.candidates.len() * report.problems.len() * report.tolerance_ladder.len()
    );
    assert!(report.output_policy.dense_output_used);
    assert_eq!(
        report.output_policy.landing,
        "paired-independent-step-clipping-and-dense-sampling"
    );

    let problem_grids = report
        .problems
        .iter()
        .map(|problem| (problem.problem_id.as_str(), problem.output_grid_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut coverage = BTreeMap::<&str, BTreeSet<u64>>::new();
    for row in &report.runs {
        assert_eq!(
            problem_grids.get(row.problem_id.as_str()).copied(),
            Some(row.output_grid_id.as_str())
        );
        coverage
            .entry(row.candidate_id.as_str())
            .or_default()
            .insert(row.rtol.to_bits());
    }
    let expected_tolerances = report
        .tolerance_ladder
        .iter()
        .map(|value| value.to_bits())
        .collect::<BTreeSet<_>>();
    assert!(
        coverage
            .values()
            .all(|observed| observed == &expected_tolerances)
    );

    let mut pair_modes = BTreeMap::<&str, BTreeSet<AdaptiveOutputMode>>::new();
    for row in &report.runs {
        pair_modes
            .entry(row.pair_id.as_str())
            .or_default()
            .insert(row.output_mode);
        assert_eq!(
            row.same_error_ranking_admissible,
            report
                .output_policy_pairs
                .iter()
                .any(|pair| { pair.ranking_record_id.as_deref() == Some(row.record_id.as_str()) })
        );
    }
    assert!(pair_modes.values().all(|modes| {
        modes == &BTreeSet::from([AdaptiveOutputMode::Clipped, AdaptiveOutputMode::Dense])
    }));
    for pair in &report.output_policy_pairs {
        assert_ne!(pair.status, AdaptiveOutputPolicyPairStatus::Incomplete);
        assert!(pair.evidence.is_some());
        assert_eq!(
            pair.pair_id,
            pair.clipped_record_id.trim_end_matches("|output-clipped")
        );
        assert_eq!(
            pair.pair_id,
            pair.dense_record_id.trim_end_matches("|output-dense")
        );
        if let Some(ranking) = &pair.ranking_record_id {
            assert_eq!(pair.status, AdaptiveOutputPolicyPairStatus::Admissible);
            assert_eq!(ranking, &pair.dense_record_id);
        }
    }
}

#[test]
fn adaptive_screen_is_scientifically_identical_in_one_and_four_threads() {
    let one = run_adaptive_global_error_screen(GlobalErrorParetoProfile::Smoke, 1).unwrap();
    let four = run_adaptive_global_error_screen(GlobalErrorParetoProfile::Smoke, 4).unwrap();
    assert_eq!(one.scientific_checksum, four.scientific_checksum);
    assert_eq!(one.execution.threads, 1);
    assert_eq!(four.execution.threads, 4);
    assert_eq!(one.candidates, four.candidates);
    assert_eq!(one.problems, four.problems);
    assert_eq!(one.tolerance_ladder, four.tolerance_ladder);
    assert_eq!(one.output_policy, four.output_policy);
    assert_eq!(one.output_policy_pairs, four.output_policy_pairs);
    assert_eq!(one.runs.len(), four.runs.len());
    for (left, right) in one.runs.iter().zip(&four.runs) {
        assert_eq!(left.record_id, right.record_id);
        assert_eq!(left.status, right.status);
        assert_eq!(left.errors, right.errors);
        assert_eq!(left.work, right.work);
        assert_eq!(left.diagnostics, right.diagnostics);
    }
}

#[test]
fn g1_adaptive_screen_is_limited_to_the_transactional_decision_set() {
    let report =
        rodas5p_fair_ab::run_g1_adaptive_global_error_screen(GlobalErrorParetoProfile::Smoke, 1)
            .unwrap();
    let ids = report
        .candidates
        .iter()
        .map(|candidate| candidate.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ids,
        BTreeSet::from([
            "protected-sequential-jf-rodas5p-gmres-adaptive",
            "transactional-q1-q2-rodas5p-t1-adaptive",
            "transactional-q1-q2-rodas5p-t4-adaptive",
            "bdf2-adaptive-reference",
            "radau-iia3-adaptive-reference",
        ])
    );
    assert_eq!(report.schema, "generic-q1-q2-adaptive-global-error-v2");
    assert_eq!(report.output_policy_pairs.len(), 5 * 2 * 2);
    assert_eq!(report.runs.len(), 2 * 5 * 2 * 2);
}
