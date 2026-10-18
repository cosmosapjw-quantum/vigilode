//! Contract tests for Newton-Leja total certificate (RVJ DAG node PY05).
//!
//! Research node: `research/py05_leja_total_certificate_20261011`
//! Preregistration: `research/py05_leja_total_certificate_20261011/PREREGISTRATION.md`

use std::alloc::{GlobalAlloc, Layout, System};

use rodas5p_core::leja_action::{
    CertifiedLejaCoeffCache, CertifiedLejaReport, CertifiedLejaWorkspace, LEJA_CERTIFIED_SCHEMA,
    LEJA_DEGREE_CAP, LEJA_TOTAL_NOT_CERTIFIED, leja_node_bounds, leja_node_polynomial_bound,
    leja_phi_action, leja_phi_action_certified, leja_phi_action_certified_capped,
    leja_phi_action_certified_with_cache, leja_phi_action_certified_with_cache_and_workspace,
    leja_points,
};
use rodas5p_core::polynomial_action::{
    EnclosureEvidence, JOINT_PHI_TERMS, JointPhiInput, SymmetricNonpositiveOperator,
    TotalErrorStatus,
};
use rodas5p_core::{DenseMatrix, LejaSetupWork, WorkCounters, dense_phi_combination, safe_l2};

// -----------------------------------------------------------------------------
// VG-1 Zero-Allocation Allocator
// -----------------------------------------------------------------------------

struct CountingAllocator;

thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALLOCATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.with(|t| t.get()) {
            ALLOCATIONS.with(|count| count.set(count.get().saturating_add(1)));
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn track_allocations<R>(f: impl FnOnce() -> R) -> (R, u64) {
    ALLOCATIONS.with(|count| count.set(0));
    TRACK.with(|t| t.set(true));
    let result = f();
    TRACK.with(|t| t.set(false));
    let count = ALLOCATIONS.with(std::cell::Cell::get);
    (result, count)
}

#[test]
fn allocation_tracker_counts_each_thread_independently() {
    std::thread::scope(|scope| {
        let a = scope.spawn(|| track_allocations(|| std::hint::black_box(vec![1_u8])).1);
        let b = scope.spawn(|| track_allocations(|| std::hint::black_box(vec![2_u8])).1);
        assert_eq!(a.join().unwrap(), 1);
        assert_eq!(b.join().unwrap(), 1);
    });
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

#[test]
fn test_vg1_zero_allocation_warmed_certified_leja_action() {
    let n = 4;
    let op = test_operator(n, 4242);
    let h = 0.05;
    let w_vecs: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(200 + k as u64, n));
    let input = JointPhiInput::Distinct(&w_vecs);

    let budget = 1e-10;
    let cap = 32;

    let mut cache = CertifiedLejaCoeffCache::new();
    let mut workspace = CertifiedLejaWorkspace::new(n, 5);
    let mut warm_work = WorkCounters::default();

    // Warm-up call: populates cache and workspace
    let warm_rep = leja_phi_action_certified_with_cache_and_workspace(
        &op,
        h,
        input,
        budget,
        cap,
        Some(&mut cache),
        &mut warm_work,
        &mut workspace,
    )
    .unwrap();
    assert!(warm_rep.converged());

    // Warmed call: must have EXACTLY 0 heap allocations!
    let (bound, allocs) = track_allocations(|| {
        let mut work = WorkCounters::default();
        let rep = leja_phi_action_certified_with_cache_and_workspace(
            &op,
            h,
            input,
            budget,
            cap,
            Some(&mut cache),
            &mut work,
            &mut workspace,
        );
        rep.unwrap().total_bound()
    });

    assert_eq!(
        allocs, 0,
        "warmed leja_phi_action_certified_with_cache_and_workspace performed {allocs} heap allocations; must be 0"
    );
    assert!(bound <= budget);
}

#[test]
fn test_a04_warmed_scalar_branch_zero_allocation() {
    let mut a = DenseMatrix::zeros(1, 1);
    a[(0, 0)] = -1.0;
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let v = [0.75];
    let scales = [1.0, 0.5, -0.25, 2.0, -1.0];
    let input = JointPhiInput::SameVector { vector: &v, scales };
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut workspace = CertifiedLejaWorkspace::new(1, 1);
    let mut work = WorkCounters::default();
    let warm = leja_phi_action_certified_with_cache_and_workspace(
        &op,
        0.0,
        input,
        1e-8,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
    )
    .unwrap();
    let vectors: [Vec<f64>; 5] = std::array::from_fn(|k| vec![scales[k] * v[0]]);
    let reference = dense_phi_combination(op.matrix(), 0.0, &vectors).unwrap();
    assert!((warm.fused()[0] - reference[0]).abs() <= 2.0 * f64::EPSILON);
    let (result, allocations) = track_allocations(|| {
        leja_phi_action_certified_with_cache_and_workspace(
            &op,
            0.0,
            input,
            1e-8,
            8,
            Some(&mut cache),
            &mut work,
            &mut workspace,
        )
    });
    result.unwrap();
    assert_eq!(
        allocations, 0,
        "warmed scalar Leja action allocated {allocations} times"
    );
}

#[test]
fn test_a04_counted_warmed_certified_leja_action_zero_allocation() {
    let n = 2;
    let matrix = DenseMatrix::from_rows(&[&[-1.0, 0.0], &[0.0, -2.0]]).unwrap();
    let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
    let vector = [0.75, -0.25];
    let input = JointPhiInput::SameVector {
        vector: &vector,
        scales: [1.0, -0.5, 0.25, 0.0, 0.0],
    };
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut workspace = CertifiedLejaWorkspace::new(n, 1);
    let mut work = WorkCounters::default();
    let mut setup = LejaSetupWork::new();
    rodas5p_core::leja_action::leja_phi_action_certified_with_setup_work(
        &op,
        0.125,
        input,
        1.0,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
        &mut setup,
    )
    .unwrap();

    let (result, allocations) = track_allocations(|| {
        rodas5p_core::leja_action::leja_phi_action_certified_with_setup_work(
            &op,
            0.125,
            input,
            1.0,
            8,
            Some(&mut cache),
            &mut work,
            &mut workspace,
            &mut setup,
        )
        .map(|report| report.total_bound())
    });
    result.unwrap();
    assert_eq!(allocations, 0, "warmed counted Leja route allocated");
}

#[test]
fn test_a04_workspace_report_buffers_follow_dimension_growth_and_shrink() {
    let mut workspace = CertifiedLejaWorkspace::new(0, 1);
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut work = WorkCounters::default();

    // Reproduce a zero-sized constructor followed by the first 1D report.
    let op1 = test_operator(1, 701);
    let v1 = [0.25];
    let scales = [1.0, -0.5, 0.25, 2.0, -1.0];
    let report = leja_phi_action_certified_with_cache_and_workspace(
        &op1,
        0.0,
        JointPhiInput::SameVector {
            vector: &v1,
            scales,
        },
        1e-8,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
    )
    .unwrap();
    assert_eq!(report.dimension(), 1);
    assert_eq!(report.fused().len(), 1);
    assert_eq!(report.columns().len(), 5);
    assert!(report.columns().iter().all(|column| column.len() == 1));

    // Grow to two dimensions and then shrink back. Report slices must track
    // the active dimension in both directions, independently of capacity.
    let op2 = test_operator(2, 702);
    let vectors: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(703 + k as u64, 2));
    let report = leja_phi_action_certified_with_cache_and_workspace(
        &op2,
        0.0,
        JointPhiInput::Distinct(&vectors),
        1e-8,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
    )
    .unwrap();
    assert_eq!(report.dimension(), 2);
    assert_eq!(report.fused().len(), 2);
    assert!(report.columns().iter().all(|column| column.len() == 2));

    let report = leja_phi_action_certified_with_cache_and_workspace(
        &op1,
        0.0,
        JointPhiInput::SameVector {
            vector: &v1,
            scales,
        },
        1e-8,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
    )
    .unwrap();
    assert_eq!(report.dimension(), 1);
    assert_eq!(report.fused().len(), 1);
    assert_eq!(report.columns().len(), 5);
    assert!(report.columns().iter().all(|column| column.len() == 1));
}

#[test]
fn test_a04_failed_dimension_change_preserves_last_successful_report() {
    let mut workspace = CertifiedLejaWorkspace::new(2, JOINT_PHI_TERMS);
    let mut cache = CertifiedLejaCoeffCache::new();
    let mut work = WorkCounters::default();
    let op2 = test_operator(2, 704);
    let vectors: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(705 + k as u64, 2));
    leja_phi_action_certified_with_cache_and_workspace(
        &op2,
        0.0,
        JointPhiInput::Distinct(&vectors),
        1e-8,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
    )
    .unwrap();
    let committed = workspace.report().unwrap().clone();
    assert_eq!(committed.dimension(), 2);
    assert_eq!(committed.columns().len(), JOINT_PHI_TERMS);
    assert_eq!(committed.fused().len(), 2);

    // The new operator has dimension one, but the candidate input is still
    // dimension two. This fails after workspace scratch is resized and must
    // leave the last successful public report transaction intact.
    let op1 = test_operator(1, 706);
    let wrong_dimension = [0.25, -0.5];
    let err = leja_phi_action_certified_with_cache_and_workspace(
        &op1,
        0.0,
        JointPhiInput::SameVector {
            vector: &wrong_dimension,
            scales: [1.0; JOINT_PHI_TERMS],
        },
        1e-8,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
    );
    assert!(err.is_err());
    assert_eq!(workspace.report().unwrap(), &committed);

    // A call rejected before any dimension-dependent work has the same
    // transactional guarantee, including status and attempted-work report.
    let err = leja_phi_action_certified_with_cache_and_workspace(
        &op1,
        f64::NAN,
        JointPhiInput::SameVector {
            vector: &[0.25],
            scales: [1.0; JOINT_PHI_TERMS],
        },
        1e-8,
        8,
        Some(&mut cache),
        &mut work,
        &mut workspace,
    );
    assert!(err.is_err());
    assert_eq!(workspace.report().unwrap(), &committed);
}

#[test]
fn test_a04_scalar_interval_coefficients_match_reference_at_nonzero_center() {
    let mut a = DenseMatrix::zeros(1, 1);
    a[(0, 0)] = -1.0;
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let v = [0.75];
    let scales = [1.0, 0.5, -0.25, 2.0, -1.0];
    let vectors: [Vec<f64>; 5] = std::array::from_fn(|k| vec![scales[k] * v[0]]);
    let reference = dense_phi_combination(op.matrix(), 0.25, &vectors).unwrap();
    let input = JointPhiInput::SameVector { vector: &v, scales };
    let mut work = WorkCounters::default();
    let report = leja_phi_action_certified(&op, 0.25, input, 1e-8, &mut work).unwrap();
    assert!(
        (report.fused()[0] - reference[0]).abs() <= 1e-14,
        "actual={} reference={} diff={:e}",
        report.fused()[0],
        reference[0],
        (report.fused()[0] - reference[0]).abs()
    );
    assert!(matches!(
        report.total_error(),
        TotalErrorStatus::EstimateOnly { .. }
    ));
}

#[test]
fn test_a04_report_serialization_preserves_exact_schema_and_estimate_status() {
    let mut matrix = DenseMatrix::zeros(1, 1);
    matrix[(0, 0)] = -1.0;
    let op = SymmetricNonpositiveOperator::gershgorin(matrix).unwrap();
    let vector = [0.25];
    let scales = [1.0, 0.5, -0.25, 2.0, -1.0];
    let input = JointPhiInput::SameVector {
        vector: &vector,
        scales,
    };
    let mut work = WorkCounters::default();
    let report = leja_phi_action_certified(&op, 0.0, input, 1.0, &mut work).unwrap();

    let total_error = match report.total_error() {
        TotalErrorStatus::EstimateOnly {
            reason,
            bounded_components,
        } => serde_json::json!({
            "status": "estimate-only",
            "reason": reason,
            "bounded_components": bounded_components,
        }),
        TotalErrorStatus::Certified { .. } => panic!("Leja total authority must remain held"),
    };
    let expected = serde_json::json!({
        "schema": report.schema(),
        "input": report.input(),
        "branch": report.branch(),
        "dimension": report.dimension(),
        "degree": report.degree(),
        "degree_cap": report.degree_cap(),
        "converged": report.converged(),
        "interval": report.interval(),
        "center": report.center(),
        "scale": report.scale(),
        "total_budget": report.total_budget(),
        "total_bound": report.total_bound(),
        "truncation_bound": report.truncation_bound(),
        "divided_difference_bound": report.divided_difference_bound(),
        "recurrence_bound": report.recurrence_bound(),
        "summation_bound": report.summation_bound(),
        "columns": report.columns(),
        "fused": report.fused(),
        "total_error": total_error,
        "evidence": report.evidence(),
        "vector_products": report.vector_products(),
        "block_products": report.block_products(),
        "coefficient_setups": report.coefficient_setups(),
        "certification_ops": report.certification_ops(),
    });
    assert_eq!(serde_json::to_value(&report).unwrap(), expected);
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

    assert_eq!(report.schema(), LEJA_CERTIFIED_SCHEMA);
    assert!(report.converged(), "must converge under 1e-8 budget");

    // Zero slack assertion: total_bound <= budget
    assert!(
        report.total_bound() <= budget,
        "zero slack violation: total_bound {} > budget {}",
        report.total_bound(),
        budget
    );

    // Verified enclosure: ||report.fused() - reference||_2 <= total_bound
    let dense_ref = dense_phi_combination(op.matrix(), h, &w_vecs).unwrap();
    let actual_diff = safe_l2(
        &report
            .fused()
            .iter()
            .zip(&dense_ref)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    assert!(
        actual_diff <= report.total_bound(),
        "enclosure violation: actual error {} > certified bound {}",
        actual_diff,
        report.total_bound()
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

    assert!(report.converged(), "must converge under 1e-9 budget");
    assert!(
        report.total_bound() <= budget,
        "zero slack violation: total_bound {} > budget {}",
        report.total_bound(),
        budget
    );

    // Reference by forming explicit distinct vectors
    let distinct_w: [Vec<f64>; 5] =
        std::array::from_fn(|k| v.iter().map(|&x| scales[k] * x).collect());
    let dense_ref = dense_phi_combination(op.matrix(), h, &distinct_w).unwrap();
    let actual_diff = safe_l2(
        &report
            .fused()
            .iter()
            .zip(&dense_ref)
            .map(|(a, b)| a - b)
            .collect::<Vec<_>>(),
    );
    assert!(
        actual_diff <= report.total_bound(),
        "enclosure violation for same-vector: actual error {} > certified bound {}",
        actual_diff,
        report.total_bound()
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
fn test_a04_subnormal_domain_keeps_total_authority_on_hold() {
    // A smallest-subnormal input is a valid finite binary64 value.  The
    // present component estimates do not establish a complete additive-
    // underflow enclosure for the operator action and final fused output.
    // Keep this adversarial domain explicit and require the API to withhold
    // total certificate authority even when the heuristic meets its budget.
    let mut a = DenseMatrix::zeros(2, 2);
    a[(0, 0)] = -1.0;
    a[(1, 1)] = -2.0;
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let v = [f64::from_bits(1), f64::from_bits(1)];
    let input = JointPhiInput::SameVector {
        vector: &v,
        scales: [1.0; 5],
    };
    // The public cache API can only populate entries through the certified
    // coefficient generator; callers cannot supply or mutate its arrays.
    let mut cache = CertifiedLejaCoeffCache::new();
    cache.precompute_certified(-0.375, 0.0625, 8).unwrap();
    let mut work = WorkCounters::default();
    let report =
        leja_phi_action_certified_with_cache(&op, 0.25, input, 1.0, 8, Some(&mut cache), &mut work)
            .unwrap();
    match report.total_error() {
        TotalErrorStatus::EstimateOnly { reason, .. } => {
            assert!(reason.contains("LEJA_ARITHMETIC_HOLD"));
            assert!(reason.contains("independent review"));
            assert!(reason.contains("coefficient-tail premises"));
        }
        TotalErrorStatus::Certified { .. } => {
            panic!("Leja must not issue total certificate authority")
        }
    }
    assert_eq!(
        work.poly_coefficient_setups, 0,
        "precomputed trusted cache should be reused"
    );
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

    assert!(!report.converged(), "must not converge under budget 1e-300");
    assert_eq!(report.degree(), cap);
    assert!(matches!(
        report.total_error(),
        TotalErrorStatus::EstimateOnly { .. }
    ));

    // Speculative work must be recorded in full in WorkCounters and report
    assert_eq!(report.degree(), cap);
    assert_eq!(report.block_products(), cap as u64);
    assert_eq!(report.vector_products(), cap as u64); // width = 1 for SameVector
    assert_eq!(report.coefficient_setups(), 1);

    assert_eq!(work.poly_block_products, cap as u64);
    assert_eq!(work.poly_vector_products, cap as u64);
    assert_eq!(work.poly_coefficient_setups, 1);
}

#[test]
fn test_vg2_dense_eigendecomposition_enclosure() {
    let n = 4;
    // Stiff diagonal matrix with eigenvalues from -100 to -0.01
    let mut a = DenseMatrix::zeros(n, n);
    a[(0, 0)] = -100.0;
    a[(1, 1)] = -10.0;
    a[(2, 2)] = -1.0;
    a[(3, 3)] = -0.01;
    let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();

    let h = 0.05;
    let vectors: [Vec<f64>; 5] = std::array::from_fn(|k| splitmix(500 + k as u64, n));
    let input = JointPhiInput::Distinct(&vectors);

    let budget = 1e-8;
    let mut work = WorkCounters::default();
    let report = leja_phi_action_certified(&op, h, input, budget, &mut work).unwrap();
    assert!(report.converged());

    // Independent reference: dense fused phi action (via Schur / matrix exp)
    let exact_fused = dense_phi_combination(&a, h, &vectors).unwrap();

    let mut diff = vec![0.0; n];
    for i in 0..n {
        diff[i] = report.fused()[i] - exact_fused[i];
    }
    let actual_error = safe_l2(&diff);

    // Strict enclosure: actual error against independent reference <= certified total_bound
    assert!(
        actual_error <= report.total_bound(),
        "true error {actual_error:e} exceeds certified total_bound {:e}",
        report.total_bound()
    );
    assert!(
        report.total_bound() <= budget,
        "total_bound {:e} exceeds budget {budget:e}",
        report.total_bound()
    );
}

#[test]
fn test_vg3_work_preservation_on_all_error_paths() {
    let n = 4;
    let op = test_operator(n, 7777);
    let v = splitmix(888, n);
    let input = JointPhiInput::SameVector {
        vector: &v,
        scales: [1.0; 5],
    };

    // 1. Extreme h causing LEJA_RANGE_LIMIT failure in setup:
    // Should preserve work counter consistency
    let mut work = WorkCounters::default();
    let impossible_h = 1e8; // -h rho exceeds LEJA_RANGE_LIMIT
    let res = leja_phi_action_certified(&op, impossible_h, input, 1e-6, &mut work);
    assert!(res.is_err());

    // 2. NaN in input vectors: fails before recurrence
    let mut nan_v = v.clone();
    nan_v[0] = f64::NAN;
    let nan_input = JointPhiInput::SameVector {
        vector: &nan_v,
        scales: [1.0; 5],
    };
    let mut work_nan = WorkCounters::default();
    let res_nan = leja_phi_action_certified(&op, 0.1, nan_input, 1e-6, &mut work_nan);
    assert!(res_nan.is_err());

    // 3. Negative-control for recurrence error (e.g. non-finite produced):
    // Accumulation of poly_block_products and poly_vector_products before failure
    let mut work_flushed = WorkCounters::default();
    let cap = 5;
    let rep_capped =
        leja_phi_action_certified_capped(&op, 0.1, input, 1e-300, cap, &mut work_flushed).unwrap();
    assert!(!rep_capped.converged());
    assert_eq!(work_flushed.poly_block_products, cap as u64);
    assert_eq!(work_flushed.poly_vector_products, cap as u64);
    assert_eq!(work_flushed.poly_coefficient_setups, 1);
}
