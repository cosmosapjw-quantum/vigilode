//! Contract tests for Newton-Leja total certificate (RVJ DAG node PY05).
//!
//! Research node: `research/py05_leja_total_certificate_20261011`
//! Preregistration: `research/py05_leja_total_certificate_20261011/PREREGISTRATION.md`

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::leja_action::{
    CertifiedLejaReport, LEJA_CERTIFIED_SCHEMA, LEJA_DEGREE_CAP, LEJA_TOTAL_NOT_CERTIFIED,
    leja_node_bounds, leja_node_polynomial_bound, leja_phi_action, leja_phi_action_certified,
    leja_phi_action_certified_capped, leja_points,
};
use rodas5p_core::polynomial_action::{
    EnclosureEvidence, JointPhiInput, SymmetricNonpositiveOperator, TotalErrorStatus,
};
use rodas5p_core::{DenseMatrix, WorkCounters, dense_phi_combination, safe_l2};

// -----------------------------------------------------------------------------
// VG-1 Zero-Allocation Allocator
// -----------------------------------------------------------------------------

struct CountingAllocator;

thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.with(|t| t.get()) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn track_allocations<R>(f: impl FnOnce() -> R) -> (R, u64) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    TRACK.with(|t| t.set(true));
    let result = f();
    TRACK.with(|t| t.set(false));
    let count = ALLOCATIONS.load(Ordering::SeqCst);
    (result, count)
}

fn splitmix(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^= z >> 31;
            2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
        })
        .collect()
}

fn test_operator(n: usize, seed: u64) -> SymmetricNonpositiveOperator {
    let mut a = DenseMatrix::zeros(n, n);
    let mut rng = seed;
    for i in 0..n {
        for j in 0..=i {
            rng = rng.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let val = ((rng >> 11) as f64 / (1u64 << 53) as f64) * 0.1;
            a[(i, j)] = val;
            a[(j, i)] = val;
        }
    }
    for i in 0..n {
        let off_diag: f64 = (0..n).filter(|&j| j != i).map(|j| a[(i, j)].abs()).sum();
        a[(i, i)] = -off_diag - 2.0 - (i as f64) * 0.5;
    }
    SymmetricNonpositiveOperator::gershgorin(a).unwrap()
}

// -----------------------------------------------------------------------------
// VG-1 Zero-Allocation Tests
// -----------------------------------------------------------------------------

#[test]
fn test_vg1_zero_allocation_node_polynomial_and_bounds() {
    let xi = leja_points();
    assert_eq!(xi.len(), LEJA_DEGREE_CAP + 1);

    // Warm-up static bounds
    let _ = leja_node_bounds();

    // Verify zero heap allocation in node polynomial evaluation for various degrees
    for &m in &[0, 1, 2, 5, 10, 20, 50] {
        let (bound, allocs) = track_allocations(|| leja_node_polynomial_bound(m, xi));
        assert_eq!(allocs, 0, "leja_node_polynomial_bound allocated for m={m}");
        assert!(bound.is_finite() && bound > 0.0);
    }

    // Verify zero allocation querying precomputed static bounds
    let (_, allocs_query) = track_allocations(|| {
        let bounds = leja_node_bounds();
        let mut sum = 0.0;
        for &b in bounds {
            sum += b;
        }
        sum
    });
    assert_eq!(allocs_query, 0, "leja_node_bounds slice query allocated");
}

// -----------------------------------------------------------------------------
// VG-2 Zero-Slack, Monotone Truncation, and Enclosure Tests
// -----------------------------------------------------------------------------

#[test]
fn test_vg2_zero_slack_budget_and_enclosure_distinct() {
    let n = 6;
    let op = test_operator(n, 12345);
    let h = 0.1;
    let w_vecs: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(100 + k as u64, n));
    let input = JointPhiInput::Distinct(&w_vecs);

    let budget = 1e-8;
    let mut work = WorkCounters::default();
    let report: CertifiedLejaReport =
        leja_phi_action_certified(&op, h, input, budget, &mut work).unwrap();

    assert_eq!(report.schema, LEJA_CERTIFIED_SCHEMA);
    assert!(report.converged, "must converge under 1e-8 budget");

    // Zero slack assertion: total_bound <= budget
    assert!(
        report.total_bound <= budget,
        "zero slack violation: total_bound {} > budget {}",
        report.total_bound,
        budget
    );

    // Verified enclosure: ||report.fused - reference||_2 <= total_bound
    let dense_ref = dense_phi_combination(op.matrix(), h, &w_vecs).unwrap();
    let actual_diff = safe_l2(
        &report
            .fused
            .iter()
            .zip(&dense_ref)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    assert!(
        actual_diff <= report.total_bound,
        "enclosure violation: actual error {} > certified bound {}",
        actual_diff,
        report.total_bound
    );
}

#[test]
fn test_vg2_zero_slack_budget_and_enclosure_same_vector() {
    let n = 6;
    let op = test_operator(n, 54321);
    let h = 0.1;
    let v = splitmix(200, n);
    let scales = [1.0, -0.5, 0.25, 2.0, -1.0];
    let input = JointPhiInput::SameVector { vector: &v, scales };

    let budget = 1e-9;
    let mut work = WorkCounters::default();
    let report: CertifiedLejaReport =
        leja_phi_action_certified(&op, h, input, budget, &mut work).unwrap();

    assert!(report.converged, "must converge under 1e-9 budget");
    assert!(
        report.total_bound <= budget,
        "zero slack violation: total_bound {} > budget {}",
        report.total_bound,
        budget
    );

    // Reference by forming explicit distinct vectors
    let distinct_w: [Vec<f64>; 5] =
        std::array::from_fn(|k| v.iter().map(|&x| scales[k] * x).collect());
    let dense_ref = dense_phi_combination(op.matrix(), h, &distinct_w).unwrap();
    let actual_diff = safe_l2(
        &report
            .fused
            .iter()
            .zip(&dense_ref)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    assert!(
        actual_diff <= report.total_bound,
        "enclosure violation for same-vector: actual error {} > certified bound {}",
        actual_diff,
        report.total_bound
    );
}

#[test]
fn test_vg2_monotone_truncation_bound() {
    let n = 4;
    let op = test_operator(n, 999);
    let h = 0.05; // Small h keeps gamma small
    let v = splitmix(300, n);
    let scales = [1.0; 5];
    let input = JointPhiInput::SameVector { vector: &v, scales };

    // Run capped at increasing degrees
    let mut prev_trunc: Option<f64> = None;
    let gamma = 0.25 * (report_gamma(&op, h));

    for cap in 2..=12 {
        let mut work = WorkCounters::default();
        let report =
            leja_phi_action_certified_capped(&op, h, input, 1e-15, cap, &mut work).unwrap();
        let trunc = report.truncation_bound();
        if let (true, Some(prev)) = ((cap as f64) > gamma, prev_trunc) {
            assert!(
                trunc < prev,
                "truncation bound must strictly decrease for cap={} > gamma={}: prev={}, current={}",
                cap,
                gamma,
                prev,
                trunc
            );
        }
        prev_trunc = Some(trunc);
    }
}

fn report_gamma(op: &SymmetricNonpositiveOperator, h: f64) -> f64 {
    let enc = op.enclosure();
    h * (enc.rho - enc.lambda)
}

// -----------------------------------------------------------------------------
// VG-3 & Gate 2: No False Authority & Work Preservation Tests
// -----------------------------------------------------------------------------

#[test]
fn test_vg3_no_false_authority_unaltered_leja_report() {
    let n = 4;
    let op = test_operator(n, 777);
    let h = 0.1;
    let v = splitmix(400, n);
    let input = JointPhiInput::SameVector {
        vector: &v,
        scales: [1.0; 5],
    };

    let mut work = WorkCounters::default();
    let uncertified_report = leja_phi_action(&op, h, input, 1e-4, &mut work).unwrap();

    // Verify uncertified LejaReport continues to return TotalErrorStatus::EstimateOnly
    match uncertified_report.total_error() {
        TotalErrorStatus::EstimateOnly { reason, .. } => {
            assert!(
                reason.starts_with(LEJA_TOTAL_NOT_CERTIFIED),
                "reason must begin with LEJA_TOTAL_NOT_CERTIFIED, got: {reason}"
            );
        }
        TotalErrorStatus::Certified { .. } => {
            panic!("uncertified LejaReport must never claim TotalErrorStatus::Certified");
        }
    }
}

#[test]
fn test_vg3_refuses_declared_enclosure_and_invalid_inputs() {
    let n = 4;
    let op = test_operator(n, 888);
    let mat = op.matrix().clone();
    let enc = op.enclosure();

    // Declared enclosure operator
    let declared_op =
        SymmetricNonpositiveOperator::with_declared_enclosure(mat, enc.lambda, enc.rho, "manual")
            .unwrap();
    assert!(matches!(
        declared_op.enclosure().evidence,
        EnclosureEvidence::Declared { .. }
    ));

    let v = splitmix(500, n);
    let input = JointPhiInput::SameVector {
        vector: &v,
        scales: [1.0; 5],
    };
    let mut work = WorkCounters::default();

    // Refusal on Declared enclosure
    let res = leja_phi_action_certified(&declared_op, 0.1, input, 1e-6, &mut work);
    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(err_str.contains("Gershgorin-verified"));

    // Refusal on invalid h
    let res_h = leja_phi_action_certified(&op, -0.1, input, 1e-6, &mut work);
    assert!(res_h.is_err());

    // Refusal on invalid budget
    let res_b = leja_phi_action_certified(&op, 0.1, input, 0.0, &mut work);
    assert!(res_b.is_err());
    let res_neg_b = leja_phi_action_certified(&op, 0.1, input, -1e-6, &mut work);
    assert!(res_neg_b.is_err());
}

#[test]
fn test_vg3_speculative_work_preservation_on_budget_failure() {
    let n = 4;
    let op = test_operator(n, 9999);
    let h = 0.5;
    let v = splitmix(600, n);
    let input = JointPhiInput::SameVector {
        vector: &v,
        scales: [1.0; 5],
    };

    let cap = 8;
    // Impossible budget forces non-convergence
    let impossible_budget = 1e-300;
    let mut work = WorkCounters::default();

    let report =
        leja_phi_action_certified_capped(&op, h, input, impossible_budget, cap, &mut work).unwrap();

    assert!(!report.converged, "must not converge under budget 1e-300");
    assert_eq!(report.degree, cap);
    assert!(matches!(
        report.total_error(),
        TotalErrorStatus::EstimateOnly { .. }
    ));

    // Speculative work must be recorded in full in WorkCounters and report
    assert_eq!(report.degree, cap);
    assert_eq!(report.block_products, cap as u64);
    assert_eq!(report.vector_products, cap as u64); // width = 1 for SameVector
    assert_eq!(report.coefficient_setups, 1);

    assert_eq!(work.poly_block_products, cap as u64);
    assert_eq!(work.poly_vector_products, cap as u64);
    assert_eq!(work.poly_coefficient_setups, 1);
}
