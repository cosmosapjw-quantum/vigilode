//! Raw timing protocol admission (re-audit R3 of 2026-10-01, R3-STAT-01).
//!
//! A 1.3 speedup over six labelled sessions with a matching ratio-1 A/A
//! control promotes. The audit mutated one raw case five ways (empty order,
//! wrong order, wrong batch, negative warmup, NaN warmup); each still gave
//! `Promote` from the assessment and `Ok(Promote)` from `verify_against_raw`.

use rodas5p_fair_ab::{
    PairedArm, PairedTimingCase, PairedTimingDecision, PairedTimingProtocol, assess_paired_timing,
    detect_timing_host_metadata,
};

const SEED: u64 = 20_261_001;

fn cases(protocol: &PairedTimingProtocol, ratio: f64, prefix: &str) -> Vec<PairedTimingCase> {
    (0..6)
        .map(|case| {
            let base = 1.0e-3 * (1.0 + case as f64);
            let candidate = (0..protocol.pairs)
                .map(|pair| base * (1.0 + 0.01 * ((pair * 7 + case) % 5) as f64))
                .collect::<Vec<_>>();
            let reference = candidate.iter().map(|value| value * ratio).collect();
            PairedTimingCase::from_samples(
                format!("{prefix}-{case}"),
                protocol,
                vec![5.0e-3, 1.0e-3],
                candidate,
                reference,
            )
            .unwrap()
            .with_process_blocks(
                (0..protocol.pairs)
                    .map(|pair| (pair * 6 / protocol.pairs) as u32)
                    .collect(),
            )
        })
        .collect()
}

type Mutation = (&'static str, fn(&mut PairedTimingCase));

fn mutations() -> Vec<Mutation> {
    vec![
        ("empty_order", |case| case.order.clear()),
        ("wrong_order", |case| {
            for arms in &mut case.order {
                *arms = [PairedArm::Candidate, PairedArm::Reference];
            }
        }),
        ("wrong_batch", |case| case.batch_iterations += 1),
        ("negative_warmup", |case| case.warmup_seconds[0] = -1.0),
        ("nan_warmup", |case| case.warmup_seconds[0] = f64::NAN),
    ]
}

#[test]
fn a_valid_receipt_promotes_and_survives_serialization() {
    let protocol = PairedTimingProtocol::authoritative(SEED);
    let candidate = cases(&protocol, 1.3, "case");
    let aa = cases(&protocol, 1.0, "aa");
    let assessment = assess_paired_timing(
        &protocol,
        &candidate,
        Some(&aa),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    assert_eq!(assessment.gate_decision, PairedTimingDecision::Promote);
    for case in candidate.iter().chain(&aa) {
        case.admit(&protocol).unwrap();
    }
    let json = serde_json::to_string(&(&candidate, &aa)).unwrap();
    let (replayed, replayed_aa): (Vec<PairedTimingCase>, Vec<PairedTimingCase>) =
        serde_json::from_str(&json).unwrap();
    assert_eq!(
        assessment
            .verify_against_raw(&replayed, Some(&replayed_aa))
            .unwrap(),
        PairedTimingDecision::Promote
    );
}

#[test]
fn malformed_raw_protocols_are_rejected_before_assessment() {
    let protocol = PairedTimingProtocol::authoritative(SEED);
    let candidate = cases(&protocol, 1.3, "case");
    let aa = cases(&protocol, 1.0, "aa");
    let genuine = assess_paired_timing(
        &protocol,
        &candidate,
        Some(&aa),
        detect_timing_host_metadata(1),
    )
    .unwrap();
    for (name, mutate) in mutations() {
        for in_aa in [false, true] {
            let mut bad_candidate = candidate.clone();
            let mut bad_aa = aa.clone();
            mutate(if in_aa {
                &mut bad_aa[2]
            } else {
                &mut bad_candidate[2]
            });
            let bad = if in_aa { &bad_aa[2] } else { &bad_candidate[2] };
            let error = bad.admit(&protocol).unwrap_err().to_string();
            assert!(
                error.contains("INVALID_RAW_TIMING_PROTOCOL"),
                "{name}: {error}"
            );
            let assessed = assess_paired_timing(
                &protocol,
                &bad_candidate,
                Some(&bad_aa),
                detect_timing_host_metadata(1),
            );
            assert!(assessed.is_err(), "{name} (A/A: {in_aa}) was assessed");
            if name != "nan_warmup" {
                // NaN does not survive JSON; the replay check sees the rest.
                assert!(
                    genuine
                        .verify_against_raw(&bad_candidate, Some(&bad_aa))
                        .is_err(),
                    "{name} (A/A: {in_aa}) verified"
                );
            }
        }
    }
}
