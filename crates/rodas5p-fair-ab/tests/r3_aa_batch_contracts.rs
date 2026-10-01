//! The A/A control calibrates its own batch (re-audit R3 campaign run of
//! 2026-10-01: a fast candidate against a slow reference calibrated a larger
//! batch than the reference alone, and the A/A case failed its admission).

use std::{collections::BTreeMap, time::Instant};

use rodas5p_fair_ab::{
    ArmIdentity, PairedTimingProtocol, PairedTimingReceipt, PairedWorkload, measure_paired_session,
    session_batches,
};

fn spin(seconds: f64) {
    let start = Instant::now();
    while start.elapsed().as_secs_f64() < seconds {
        std::hint::spin_loop();
    }
}

fn workloads() -> Vec<PairedWorkload<'static>> {
    vec![PairedWorkload {
        case_id: "fast-against-slow".into(),
        candidate: Box::new(|| {
            spin(1.0e-4);
            Ok(())
        }),
        reference: Box::new(|| {
            spin(1.0e-3);
            Ok(())
        }),
    }]
}

#[test]
fn the_aa_control_keeps_its_own_batch_across_sessions() {
    let protocol = PairedTimingProtocol {
        pairs: 30,
        ..PairedTimingProtocol::authoritative(7)
    };
    let first =
        measure_paired_session("aa-batch", 0, &protocol, &mut workloads(), &BTreeMap::new())
            .unwrap();
    assert!(first.failures.is_empty(), "{:?}", first.failures);
    let (case, aa) = (&first.cases[0], &first.aa_cases[0]);
    // The candidate case calibrates on the fast arm, the A/A on the slow one.
    assert!(
        case.batch_iterations > aa.batch_iterations,
        "{case:?} {aa:?}"
    );
    case.admit(&protocol).unwrap();
    aa.admit(&protocol).unwrap();
    let batches = session_batches(&first);
    assert_eq!(batches["fast-against-slow"], case.batch_iterations);
    assert_eq!(batches["fast-against-slow#aa"], aa.batch_iterations);
    let mut second =
        measure_paired_session("aa-batch", 1, &protocol, &mut workloads(), &batches).unwrap();
    // A campaign runs each session in its own process; stand in for that.
    second.provenance.process_id = first.provenance.process_id.wrapping_add(1);
    assert_eq!(second.cases[0].batch_iterations, case.batch_iterations);
    assert_eq!(second.aa_cases[0].batch_iterations, aa.batch_iterations);
    // The merged receipt admits every case and A/A control.
    let arm = |arm_id: &str| ArmIdentity {
        arm_id: arm_id.into(),
        executable_sha256: "0".repeat(64),
        workload_id: "aa-batch".into(),
    };
    let receipt = PairedTimingReceipt::from_sessions(
        "aa-batch",
        arm("candidate"),
        arm("reference"),
        protocol,
        vec![first, second],
    )
    .unwrap();
    receipt.validate().unwrap();
    for case in receipt.cases.iter().chain(&receipt.aa_cases) {
        case.admit(&receipt.protocol).unwrap();
    }
}
