// Source-only integer tally. No solver/reference numerical evaluation occurs
// here; formulas expand every charged counter on successful delegated paths.
pub fn prepare(n: usize, o: usize, d: usize, sweeps: usize) -> PacketWorkCounters {
    let (n, o, d, s) = (n as u64, o as u64, d as u64, sweeps as u64);
    let b = o + 1;
    let c = d + 1;
    let f = 3 * d + 1;
    let m = o + 3 * d + 1;
    let l = m + 1;
    let r = 3 * m + 1;
    let payload = 4 * n + 5;
    let identity = payload + 1 + 7 * b;
    let products = c * c + (2 * d + 1) * c;
    PacketWorkCounters {
        identity_snapshot_items: payload + 2 * identity + 2 * c + 3 * n + 5 + 4 * b,
        identity_comparison_items: 3 * identity,
        model_copy_items: payload,
        model_bounds_snapshot_items: 2 * n,
        model_bounds_comparison_items: 4 * n,
        moment_terms: 2 * n * b,
        poly_multiply_terms: (s + 1) * products,
        volterra_terms: (s + n) * b * f,
        nonlinear_evaluations: s,
        reconstruction_terms: c + 2 * f + b * (2 * f - 1) + n * (2 + 3 * o + 2 * m),
        certificate_items: 3 * n * l
            + l
            + 3 * (l * l + (2 * m + 1) * l)
            + 2 * r
            + n * (2 * m + l + 2 * r)
            + 5 * l
            + n * (5 * r + 3)
            + 1,
        curve_setup_allocation_requests: 23,
        curve_setup_reserved_items: 2 * c
            + (2 * d + 1)
            + 2 * f
            + b * f
            + n * l
            + 5 * b
            + 4 * l
            + (2 * m + 1)
            + 2 * r
            + n * r
            + 3 * n,
        curve_setup_interval_ops: n + 1,
        curve_model_constructor_payload_items: 8 * n + 8,
        curve_model_constructor_arithmetic_ops: 12 * n + 4,
        curve_frozen_allocation_requests: 1,
        curve_frozen_radius_items: n,
        curve_output_allocation_requests: 3,
        ..PacketWorkCounters::default()
    }
}
pub fn query(n: usize, o: usize, d: usize, count: u64) -> PacketWorkCounters {
    let n = n as u64;
    let l = (o + 3 * d + 2) as u64;
    PacketWorkCounters {
        certificate_items: 3 * l * (n + 1) * count,
        output_horner_terms: n * l * count,
        output_value_items: 2 * n * count,
        curve_time_normalization_ops: 3 * count,
        curve_output_certificate_ops: (32 * n + 24) * count,
        curve_frozen_query_attempts: count,
        ..PacketWorkCounters::default()
    }
}
pub fn assert_delta(
    before: &PacketWorkCounters,
    after: &PacketWorkCounters,
    expected: &PacketWorkCounters,
    operation: &str,
) {
    let before = serde_json::to_value(before).unwrap();
    let after = serde_json::to_value(after).unwrap();
    let expected = serde_json::to_value(expected).unwrap();
    for (field, delta) in expected.as_object().unwrap() {
        let prior = before[field].as_u64().unwrap();
        let next = after[field].as_u64().unwrap();
        assert!(next >= prior, "{operation}: {field} counter regressed");
        assert_eq!(
            next - prior,
            delta.as_u64().unwrap(),
            "{operation}: {field}; source-derived frozen expected vector {expected}; before {before}; after {after}"
        );
    }
}

pub fn admission(n: usize) -> PacketWorkCounters {
    PacketWorkCounters {
        curve_model_constructor_payload_items: (4 * n + 4) as u64,
        curve_model_constructor_arithmetic_ops: (6 * n + 2) as u64,
        ..PacketWorkCounters::default()
    }
}
pub fn initial_adapter(n: usize, g: usize) -> VerifiedTrajectoryWork {
    VerifiedTrajectoryWork {
        validation_items: (12 + g + 4 * n) as u64,
        arithmetic_attempts: (12 * n + 2) as u64,
        copied_items: (7 * n + g) as u64,
        allocation_requests: (8 + 2 * g) as u64,
        ..VerifiedTrajectoryWork::default()
    }
}
pub fn prepare_adapter(
    n: usize,
    o: usize,
    d: usize,
    s: usize,
    g: usize,
    k: usize,
) -> VerifiedTrajectoryWork {
    VerifiedTrajectoryWork {
        packet: prepare(n, o, d, s),
        validation_items: (4 * n) as u64,
        arithmetic_attempts: (13 * n + 14 + (o + 1) * (1 + 6 * d)) as u64,
        copied_items: (7 * n + g) as u64,
        allocation_requests: (o + 15 + 2 * k) as u64,
        prepare_attempts: 1,
        ..VerifiedTrajectoryWork::default()
    }
}
pub fn query_adapter(n: usize, o: usize, d: usize) -> VerifiedTrajectoryWork {
    VerifiedTrajectoryWork {
        packet: query(n, o, d, 1),
        validation_items: 1,
        arithmetic_attempts: (15 + 6 * n) as u64,
        copied_items: (10 + 3 * n) as u64,
        query_attempts: 1,
        ..VerifiedTrajectoryWork::default()
    }
}
// `completed` queries pass tolerance; optional final failed query's mode
// determines the exact source-defined short-circuit validation count.
pub fn rejected_tolerance_adapter(
    n: usize,
    o: usize,
    d: usize,
    g: usize,
    completed: usize,
    failed_mode: Option<usize>,
) -> VerifiedTrajectoryWork {
    let count = completed + 1;
    VerifiedTrajectoryWork {
        packet: query(n, o, d, count as u64),
        validation_items: (completed * (n + 2) + 2 + failed_mode.map(|j| j + 1).unwrap_or(0))
            as u64,
        identity_items: (22 + 5 * n + g) as u64,
        arithmetic_attempts: (count * (15 + 6 * n)) as u64,
        copied_items: (count * (10 + 3 * n) + completed * (11 + 3 * n)) as u64,
        query_attempts: count as u64,
        commit_attempts: 1,
        rejected_attempts: 1,
        ..VerifiedTrajectoryWork::default()
    }
}
pub fn fault_adapter(
    n: usize,
    o: usize,
    d: usize,
    g: usize,
    completed: usize,
) -> VerifiedTrajectoryWork {
    VerifiedTrajectoryWork {
        packet: query(n, o, d, completed as u64),
        validation_items: (completed * (n + 2)) as u64,
        identity_items: (22 + 5 * n + g) as u64,
        arithmetic_attempts: (completed * (15 + 6 * n)) as u64,
        copied_items: (completed * (21 + 6 * n)) as u64,
        query_attempts: completed as u64,
        commit_attempts: 1,
        rejected_attempts: 1,
        ..VerifiedTrajectoryWork::default()
    }
}
pub fn commit_adapter(n: usize, o: usize, d: usize, g: usize, k: usize) -> VerifiedTrajectoryWork {
    let mut packet = query(n, o, d, (k + 1) as u64);
    let extra = admission(n);
    packet.curve_model_constructor_payload_items = extra.curve_model_constructor_payload_items;
    packet.curve_model_constructor_arithmetic_ops = extra.curve_model_constructor_arithmetic_ops;
    VerifiedTrajectoryWork {
        packet,
        validation_items: ((k + 1) * (n + 2)) as u64,
        identity_items: (2 * (22 + 5 * n + g)) as u64,
        arithmetic_attempts: ((k + 1) * (15 + 6 * n)) as u64,
        copied_items: ((k + 1) * (10 + 3 * n) + 2 * k * (11 + 3 * n) + 3 * n + 2) as u64,
        query_attempts: (k + 1) as u64,
        commit_attempts: 1,
        ..VerifiedTrajectoryWork::default()
    }
}
pub fn admission_rejection_adapter(
    n: usize,
    o: usize,
    d: usize,
    g: usize,
    k: usize,
) -> VerifiedTrajectoryWork {
    let mut expected = commit_adapter(n, o, d, g, k);
    expected.identity_items = (22 + 5 * n + g) as u64;
    expected.copied_items = ((k + 1) * (10 + 3 * n) + k * (11 + 3 * n)) as u64;
    expected.rejected_attempts = 1;
    expected
}
pub fn assert_full_delta(
    before: &VerifiedTrajectoryWork,
    after: &VerifiedTrajectoryWork,
    expected: &VerifiedTrajectoryWork,
    operation: &str,
) {
    assert_delta(&before.packet, &after.packet, &expected.packet, operation);
    let before = serde_json::to_value(before).unwrap();
    let after = serde_json::to_value(after).unwrap();
    let expected = serde_json::to_value(expected).unwrap();
    for (field, value) in expected.as_object().unwrap() {
        if field == "packet" {
            continue;
        }
        let old = before[field].as_u64().unwrap();
        let new = after[field].as_u64().unwrap();
        assert!(new >= old, "{operation}: {field} regressed");
        assert_eq!(
            new - old,
            value.as_u64().unwrap(),
            "{operation}: {field}; before {before}; after {after}; frozen expected {expected}"
        );
    }
}
