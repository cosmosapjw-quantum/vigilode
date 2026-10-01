//! Statistics contracts of re-audit R4 of 2026-10-01 (R4-STAT-DEV-01..04).

use std::collections::BTreeMap;

use rodas5p_fair_ab::{
    ArmIdentity, PAIRED_TIMING_MONOTONIC_CLOCK, POOLED_PAIR_MEDIAN_ESTIMAND, PairedArm,
    PairedTimingCase, PairedTimingDecision, PairedTimingEvidence, PairedTimingProtocol,
    PairedTimingReceipt, SESSION_CELL_MEDIAN_ESTIMAND, SessionCells, SessionIntervalStatus,
    SessionProvenance, SessionRecord, TimingAuthorityDomain, TimingAuthorityStatus,
    TimingDesignIdentity, detect_timing_host_metadata, exact_session_median_interval,
    select_timing_authority, session_cells_from_records, session_median_design,
    timing_authority_registry, verify_timing_authority,
};
use serde_json::Value;

const EXACT: &str = include_str!(
    "../../../research/adversarial_reaudit_20261001_r4/statistics/EXACT_SESSION_INTERVAL.json"
);
const POLY03: &str =
    include_str!("../../../research/r3_matched_accuracy_poly03_20261001/CAMPAIGN.json");

fn protocol() -> PairedTimingProtocol {
    PairedTimingProtocol::authoritative(20261011)
}

/// The R4 probe's synthetic campaign: six sessions, one case at 1.3x and
/// its A/A control.
fn records(p: &PairedTimingProtocol) -> Vec<SessionRecord> {
    (0..6)
        .map(|s| {
            let case = |id: &str, ratio: f64| {
                PairedTimingCase::from_samples(
                    id,
                    p,
                    vec![0.005, 0.001],
                    vec![0.001; p.pairs],
                    vec![0.001 * ratio; p.pairs],
                )
                .unwrap()
                .with_process_blocks(vec![s; p.pairs])
            };
            SessionRecord {
                campaign_id: "r4-synthetic-contract".into(),
                provenance: SessionProvenance {
                    session: s,
                    process_id: 1000 + s,
                    started_unix_seconds: 1000.0 + f64::from(s),
                    finished_unix_seconds: 1000.5 + f64::from(s),
                    clock: PAIRED_TIMING_MONOTONIC_CLOCK.into(),
                    host: detect_timing_host_metadata(1),
                },
                cases: vec![case("case-0", 1.3)],
                aa_cases: vec![case("case-0#aa", 1.0)],
                failures: vec![],
            }
        })
        .collect()
}

fn arm(id: &str) -> ArmIdentity {
    ArmIdentity {
        arm_id: id.into(),
        executable_sha256: "a".repeat(64),
        workload_id: "synthetic-r4-source-contract-only".into(),
    }
}

fn receipt(records: Vec<SessionRecord>) -> Result<PairedTimingReceipt, String> {
    PairedTimingReceipt::from_sessions(
        "r4-synthetic-contract",
        arm("candidate"),
        arm("reference"),
        protocol(),
        records,
    )
    .map_err(|e| e.to_string())
}

#[test]
fn every_raw_cell_is_validated_before_merging() {
    let original = records(&protocol());
    let genuine = receipt(original.clone()).unwrap();
    let evidence = PairedTimingEvidence::from_receipt(genuine).unwrap();
    assert_eq!(
        evidence.verified_decision().unwrap(),
        PairedTimingDecision::Promote
    );
    type Mutation = fn(&mut Vec<SessionRecord>);
    let malformed: [(&str, Mutation); 7] = [
        ("later_empty_warmups", |r| {
            r[1].cases[0].warmup_seconds.clear()
        }),
        ("later_one_warmup", |r| {
            r[1].cases[0].warmup_seconds.truncate(1)
        }),
        ("shift_candidate_counts", |r| {
            let x = r[1].cases[0].candidate_seconds.pop().unwrap();
            r[2].cases[0].candidate_seconds.push(x);
        }),
        ("shift_reference_counts", |r| {
            let x = r[1].cases[0].reference_seconds.pop().unwrap();
            r[2].cases[0].reference_seconds.push(x);
        }),
        ("end_before_start", |r| {
            r[1].provenance.finished_unix_seconds = 0.0
        }),
        ("wrong_later_batch", |r| r[1].cases[0].batch_iterations += 1),
        ("nan_start", |r| {
            r[3].provenance.started_unix_seconds = f64::NAN
        }),
    ];
    for (name, mutate) in malformed {
        let mut r = original.clone();
        mutate(&mut r);
        let error = receipt(r).unwrap_err();
        assert!(error.contains("TIMING_NOT_EVALUATED"), "{name}: {error}");
    }
    // An empty workload identity is rejected.
    let empty_identity = PairedTimingReceipt::from_sessions(
        "r4-synthetic-contract",
        ArmIdentity {
            workload_id: " ".into(),
            ..arm("candidate")
        },
        arm("reference"),
        protocol(),
        original.clone(),
    );
    assert!(empty_identity.is_err());
    // A missing later cell is a recorded failure: it never gates.
    let mut missing = original.clone();
    missing[1].cases.clear();
    let receipt = receipt(missing).unwrap();
    assert!(
        receipt
            .failures
            .iter()
            .any(|f| f.session == 1 && f.message.contains("missing cell"))
    );
    let evidence = PairedTimingEvidence::from_receipt(receipt).unwrap();
    assert_eq!(
        evidence.verified_decision().unwrap(),
        PairedTimingDecision::Inconclusive
    );
    // The five R3 malformed-case rejections still hold.
    let p = protocol();
    let base = original[0].cases[0].clone();
    type CaseMutation = fn(&mut PairedTimingCase);
    let r3: [CaseMutation; 5] = [
        |c| c.order.clear(),
        |c| c.order.fill([PairedArm::Candidate, PairedArm::Reference]),
        |c| c.batch_iterations += 1,
        |c| c.warmup_seconds[0] = -1.0,
        |c| c.warmup_seconds[0] = f64::NAN,
    ];
    for mutate in r3 {
        let mut c = base.clone();
        mutate(&mut c);
        assert!(c.admit(&p).is_err());
    }
}

#[test]
fn published_campaigns_stay_valid_and_keep_their_estimand() {
    let campaign: Value = serde_json::from_str(POLY03).unwrap();
    for (arm, value) in campaign["evidence"].as_object().unwrap() {
        let evidence: PairedTimingEvidence = serde_json::from_value(value.clone()).unwrap();
        // Written before R4 without an estimand: the pooled-pair one.
        assert_eq!(
            evidence.assessment.estimand, POOLED_PAIR_MEDIAN_ESTIMAND,
            "{arm}"
        );
        let decision = evidence.verified_decision().unwrap();
        assert_eq!(
            format!("{decision:?}"),
            campaign["summary"][arm]["verified_decision"]
                .as_str()
                .unwrap(),
            "{arm}"
        );
        // ... and its authority is the failed studies' hold.
        let admissible = evidence.admissible_decision().unwrap();
        assert_eq!(admissible.authority, TimingAuthorityStatus::Hold, "{arm}");
        assert_eq!(admissible.diagnostic, decision);
        assert!(admissible.admissible.is_none());
    }
}

#[test]
fn authority_comes_from_the_registry_and_forgeries_fail() {
    let evidence =
        PairedTimingEvidence::from_receipt(receipt(records(&protocol())).unwrap()).unwrap();
    let decision = evidence.admissible_decision().unwrap();
    assert_eq!(decision.diagnostic, PairedTimingDecision::Promote);
    assert_eq!(decision.authority, TimingAuthorityStatus::Hold);
    assert!(decision.admissible.is_none());
    assert!(decision.reason.contains("L-0007") && decision.reason.contains("L-0010"));
    for study in timing_authority_registry() {
        verify_timing_authority(&study).unwrap();
        let mut forged = study.clone();
        forged.status = TimingAuthorityStatus::Admissible;
        assert!(verify_timing_authority(&forged).is_err());
        let mut unrelated = study.clone();
        unrelated.study_id = "my-own-study".into();
        assert!(verify_timing_authority(&unrelated).is_err());
    }
    // A future admissible study authorizes only the designs it covers.
    let passed = rodas5p_fair_ab::TimingAuthority {
        study_id: "future-study".into(),
        ledger_row: "L-9999".into(),
        domain: TimingAuthorityDomain {
            estimand: POOLED_PAIR_MEDIAN_ESTIMAND.into(),
            assessment_schema: evidence.assessment.schema.clone(),
            min_sessions: 6,
            max_sessions: 6,
            case_counts: vec![1],
        },
        status: TimingAuthorityStatus::Admissible,
        reason: "test".into(),
    };
    let registry = vec![passed];
    let admitted = evidence.admissible_decision_in(&registry).unwrap();
    assert_eq!(admitted.admissible, Some(PairedTimingDecision::Promote));
    let design = evidence.design_identity();
    for other in [
        TimingDesignIdentity {
            cases: 2,
            ..design.clone()
        },
        TimingDesignIdentity {
            sessions: 8,
            ..design.clone()
        },
        TimingDesignIdentity {
            estimand: SESSION_CELL_MEDIAN_ESTIMAND.into(),
            ..design.clone()
        },
    ] {
        assert_eq!(
            select_timing_authority(&registry, &other).0,
            TimingAuthorityStatus::NotEvaluated
        );
    }
    // The session-cell estimand has no study in the compiled registry.
    assert_eq!(
        select_timing_authority(
            &timing_authority_registry(),
            &TimingDesignIdentity {
                estimand: SESSION_CELL_MEDIAN_ESTIMAND.into(),
                ..design
            }
        )
        .0,
        TimingAuthorityStatus::NotEvaluated
    );
}

/// Population medians of a discrete session law `u` plus `pairs` iid pair
/// errors `e` per cell: the pooled-pair median (of `u + e`) and the median
/// of session-cell medians (of `u + median(e_1..e_pairs)`).
fn medians(u: &[(f64, f64)], e: &[(f64, f64)], pairs: usize) -> (f64, f64) {
    fn median_of(law: &mut [(f64, f64)]) -> f64 {
        law.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut cumulative = 0.0;
        for (value, p) in law.iter() {
            cumulative += p;
            if cumulative >= 0.5 - 1.0e-15 {
                return *value;
            }
        }
        unreachable!()
    }
    let mut pooled = u
        .iter()
        .flat_map(|(a, p)| e.iter().map(move |(b, q)| (a + b, p * q)))
        .collect::<Vec<_>>();
    // e takes two values here: the cell median is the upper one iff more
    // than half the pairs take it.
    assert_eq!(e.len(), 2);
    let (low, high) = if e[0].0 < e[1].0 {
        (e[0], e[1])
    } else {
        (e[1], e[0])
    };
    let mut choose = 1.0_f64;
    let mut upper = 0.0;
    for k in 0..=pairs {
        if k > 0 {
            choose = choose * (pairs + 1 - k) as f64 / k as f64;
        }
        if 2 * k > pairs {
            upper += choose * high.1.powi(k as i32) * low.1.powi((pairs - k) as i32);
        }
    }
    let mut cells = u
        .iter()
        .flat_map(|(a, p)| [(a + low.0, p * (1.0 - upper)), (a + high.0, p * upper)])
        .collect::<Vec<_>>();
    (median_of(&mut pooled), median_of(&mut cells))
}

#[test]
fn the_two_estimands_differ_off_the_symmetric_family() {
    // Session effect 0 (0.6) or 10 (0.4); pair error 0 (0.6) or 3 (0.4);
    // 31 pairs per cell: pooled-pair median 3, median of cell medians 0.
    let (pooled, cells) = medians(&[(0.0, 0.6), (10.0, 0.4)], &[(0.0, 0.6), (3.0, 0.4)], 31);
    assert_eq!((pooled, cells), (3.0, 0.0));
    // Symmetric session effect and pair error about 0: the two laws are
    // the same (median set [-1, 1], lower median -1 for both).
    let (pooled, cells) = medians(
        &[(-1.0, 0.25), (0.0, 0.5), (1.0, 0.25)],
        &[(-2.0, 0.5), (2.0, 0.5)],
        31,
    );
    assert_eq!(pooled, cells);
    // The assessment names its estimand; the interval names the other.
    let evidence =
        PairedTimingEvidence::from_receipt(receipt(records(&protocol())).unwrap()).unwrap();
    assert_eq!(evidence.assessment.estimand, POOLED_PAIR_MEDIAN_ESTIMAND);
    let rows = session_cells_from_records(&evidence.receipt.session_records);
    let interval =
        exact_session_median_interval(&rows, &["case-0".to_string()], 6, 1, 20, 1.15).unwrap();
    assert_eq!(interval.estimand, SESSION_CELL_MEDIAN_ESTIMAND);
    assert_ne!(interval.estimand, evidence.assessment.estimand);
}

#[test]
fn the_exact_design_table_enumerations_controls_and_invalid_inputs_reproduce() {
    let exact: Value = serde_json::from_str(EXACT).unwrap();
    let designs = exact["designs"].as_array().unwrap();
    assert_eq!(designs.len(), 32);
    for row in designs {
        let (s, c) = (
            row["sessions"].as_u64().unwrap() as usize,
            row["cases"].as_u64().unwrap() as usize,
        );
        let design = session_median_design(s, c, 1, 20).unwrap();
        assert_eq!(design.k.map(|k| k as u64), row["k"].as_u64(), "S{s} C{c}");
        assert_eq!(
            design.per_case_failure.display(),
            row["per_case_failure"].as_str().unwrap(),
            "S{s} C{c}"
        );
        assert_eq!(
            design.simultaneous_coverage_lower.display(),
            row["simultaneous_coverage_lower"].as_str().unwrap(),
            "S{s} C{c}"
        );
    }
    // Exhaustive sign enumerations: a per-case interval [Z_(k), Z_(S-k+1)]
    // holds iff k <= #(Z <= m) <= S - k.
    let enumerations = exact["enumerations"].as_array().unwrap();
    assert_eq!(enumerations.len(), 6);
    for row in enumerations {
        let (s, c) = (
            row["sessions"].as_u64().unwrap() as usize,
            row["cases"].as_u64().unwrap() as usize,
        );
        let design = session_median_design(s, c, 1, 20).unwrap();
        let accepted = (0_u32..(1 << s))
            .filter(|signs| {
                let n = signs.count_ones() as usize;
                design.k.is_none_or(|k| k <= n && n <= s - k)
            })
            .count() as u64;
        assert_eq!(
            accepted,
            row["accepted_sign_patterns"].as_u64().unwrap(),
            "S{s} C{c}"
        );
    }
    // Controls: identical sessions at 0.9x, 1.15x and 1.3x.
    let controls = exact["controls"].as_array().unwrap();
    assert_eq!(controls.len(), 12);
    for row in controls {
        let (s, c) = (
            row["sessions"].as_u64().unwrap() as usize,
            row["cases"].as_u64().unwrap() as usize,
        );
        let ratio = row["ratio"].as_f64().unwrap();
        let cases = (0..c).map(|j| j.to_string()).collect::<Vec<_>>();
        let rows = (0..s as u32)
            .map(|session| SessionCells {
                session,
                cells: cases.iter().map(|j| (j.clone(), ratio.ln())).collect(),
                failed: false,
            })
            .collect::<Vec<_>>();
        let interval = exact_session_median_interval(&rows, &cases, s, 1, 20, 1.15).unwrap();
        assert_eq!(
            format!("{:?}", interval.decision),
            row["decision"].as_str().unwrap(),
            "S{s} C{c} {ratio}"
        );
        assert_eq!(
            interval.status == SessionIntervalStatus::Finite,
            row["status"] == "FINITE"
        );
        assert_eq!(interval.lower, row["lower"].as_f64());
        assert_eq!(interval.upper, row["upper"].as_f64());
    }
    // Invalid inputs, and a stopping rule other than the planned one.
    let cases = vec!["a".to_string()];
    let base = (0..6)
        .map(|session| SessionCells {
            session,
            cells: BTreeMap::from([("a".to_string(), 0.3)]),
            failed: false,
        })
        .collect::<Vec<_>>();
    type RowMutation = fn(&mut Vec<SessionCells>);
    let invalid: [RowMutation; 4] = [
        |r| r[1].session = 0,
        |r| r[1].cells.clear(),
        |r| {
            r[1].cells.insert("a".into(), f64::NAN);
        },
        |r| r[1].failed = true,
    ];
    assert_eq!(
        exact["invalid_inputs"].as_array().unwrap().len(),
        invalid.len()
    );
    for mutate in invalid {
        let mut rows = base.clone();
        mutate(&mut rows);
        assert!(exact_session_median_interval(&rows, &cases, 6, 1, 20, 1.15).is_err());
    }
    assert!(exact_session_median_interval(&base, &cases, 8, 1, 20, 1.15).is_err());
    // Named designs: S6/C1 31/32, S6/C5 unbounded, S8/C5 123/128.
    assert_eq!(
        session_median_design(6, 1, 1, 20)
            .unwrap()
            .simultaneous_coverage_lower
            .display(),
        "31/32"
    );
    assert_eq!(session_median_design(6, 5, 1, 20).unwrap().k, None);
    assert_eq!(
        session_median_design(8, 5, 1, 20)
            .unwrap()
            .simultaneous_coverage_lower
            .display(),
        "123/128"
    );
    // Large designs stay exact (no 2^S overflow).
    let big = session_median_design(1000, 5, 1, 20).unwrap();
    assert!(big.k.unwrap() > 400);
}
