//! Contract tests and case exporter for PY03 (RVJ DAG node PY03,
//! `research/py03_single_backend_router_20261011`).

use std::{
    alloc::{GlobalAlloc, Layout, System},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT,
    polynomial_action::{
        JointPhiInput, PolynomialBasis, RouteChoice, RouteCost, RoutePolicy, RouterCaches,
        SymmetricNonpositiveOperator, TotalErrorAdmission, count_chebyshev_series_terms,
        execute_joint_phi, joint_phi_action, joint_phi_action_unbounded, plan_joint_phi,
        route_admission, route_joint_phi, route_joint_phi_single,
    },
};
use serde_json::{Value, json};

// -----------------------------------------------------------------------------
// VG-1 Zero-Allocation Allocator
// -----------------------------------------------------------------------------

struct CountingAllocator;

thread_local! {
    static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() && TRACK.with(|t| t.get()) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
    }
}

fn count_allocations<T>(operation: impl FnOnce() -> T) -> (T, u64, u64) {
    ALLOCATIONS.store(0, Ordering::SeqCst);
    ALLOCATED_BYTES.store(0, Ordering::SeqCst);
    TRACK.with(|t| t.set(true));
    let result = operation();
    TRACK.with(|t| t.set(false));
    (
        result,
        ALLOCATIONS.load(Ordering::SeqCst),
        ALLOCATED_BYTES.load(Ordering::SeqCst),
    )
}

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hex_vec(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
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

const LAMBDA_MIN_FRACTION: f64 = 0.1;
const MAX_ANGLE: f64 = 0.3;

fn spectral_matrix(n: usize, rho: f64, seed: u64) -> (DenseMatrix, Vec<f64>) {
    let lambda_min = LAMBDA_MIN_FRACTION * rho;
    let mut u: Vec<f64> = splitmix(seed, n - 2)
        .iter()
        .map(|x| (x + 1.0) / 2.0)
        .collect();
    u.sort_by(f64::total_cmp);
    let mut spectrum = vec![-lambda_min];
    spectrum.extend(u.iter().map(|x| -(lambda_min + (rho - lambda_min) * x)));
    spectrum.push(-rho);
    let angles: Vec<f64> = splitmix(seed + 500, n - 1)
        .iter()
        .map(|x| MAX_ANGLE * x)
        .collect();
    let mut q = vec![vec![0.0; n]; n];
    for (i, row) in q.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for (i, t) in angles.iter().enumerate() {
        let (s, c) = t.sin_cos();
        for row in q.iter_mut() {
            let (a, b) = (row[i], row[i + 1]);
            row[i] = c * a - s * b;
            row[i + 1] = s * a + c * b;
        }
    }
    let mut raw = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            raw[i][j] = (0..n).map(|k| q[i][k] * spectrum[k] * q[j][k]).sum();
        }
    }
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        for j in 0..n {
            a[(i, j)] = (raw[i][j] + raw[j][i]) * 0.5;
        }
    }
    (a, spectrum)
}

fn diagonal_matrix(n: usize, rho: f64) -> (DenseMatrix, Vec<f64>) {
    let mut a = DenseMatrix::zeros(n, n);
    let mut spectrum = Vec::with_capacity(n);
    for i in 0..n {
        let val = -rho * (i as f64) / ((n - 1) as f64);
        a[(i, i)] = val;
        spectrum.push(val);
    }
    (a, spectrum)
}

const SCALES: [f64; 5] = [1.0, -0.5, 0.25, 2.0, -1.0];

enum TestCaseInput {
    Distinct([Vec<f64>; 5]),
    Same(Vec<f64>),
}

impl TestCaseInput {
    fn distinct(n: usize, seed: u64) -> Self {
        TestCaseInput::Distinct(std::array::from_fn(|k| splitmix(seed + 77 * k as u64, n)))
    }

    fn same(n: usize, seed: u64) -> Self {
        TestCaseInput::Same(splitmix(seed, n))
    }

    fn as_input(&self) -> JointPhiInput<'_> {
        match self {
            TestCaseInput::Distinct(w) => JointPhiInput::Distinct(w),
            TestCaseInput::Same(v) => JointPhiInput::SameVector {
                vector: v,
                scales: SCALES,
            },
        }
    }

    #[allow(dead_code)]
    fn name(&self) -> &'static str {
        match self {
            TestCaseInput::Distinct(_) => "distinct",
            TestCaseInput::Same(_) => "same",
        }
    }

    fn columns(&self) -> Vec<Vec<f64>> {
        match self {
            TestCaseInput::Distinct(w) => w.to_vec(),
            TestCaseInput::Same(v) => SCALES
                .iter()
                .map(|s| v.iter().map(|x| s * x).collect())
                .collect(),
        }
    }
}

// -----------------------------------------------------------------------------
// Contract Tests
// -----------------------------------------------------------------------------

#[test]
fn test_plan_joint_phi_zero_allocations() {
    let (a, _) = diagonal_matrix(8, 4.0);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let distinct_input = TestCaseInput::distinct(8, 42);
    let same_input = TestCaseInput::same(8, 42);
    let mut cost = RouteCost::default();

    // 1. Chebyshev Distinct
    let (plan, allocs, _) = count_allocations(|| {
        plan_joint_phi(
            &op,
            0.05,
            distinct_input.as_input(),
            PolynomialBasis::Chebyshev,
            1e-7,
            None,
            &mut cost,
        )
    });
    assert!(plan.is_ok());
    assert_eq!(allocs, 0, "Chebyshev distinct allocated heap memory");

    // 2. Laguerre Distinct
    let (plan, allocs, _) = count_allocations(|| {
        plan_joint_phi(
            &op,
            0.05,
            distinct_input.as_input(),
            PolynomialBasis::Laguerre,
            1e-7,
            None,
            &mut cost,
        )
    });
    assert!(plan.is_ok());
    assert_eq!(allocs, 0, "Laguerre distinct allocated heap memory");

    // 3. Chebyshev SameVector
    let (plan, allocs, _) = count_allocations(|| {
        plan_joint_phi(
            &op,
            0.05,
            same_input.as_input(),
            PolynomialBasis::Chebyshev,
            1e-7,
            None,
            &mut cost,
        )
    });
    assert!(plan.is_ok());
    assert_eq!(allocs, 0, "Chebyshev same-vector allocated heap memory");

    // 4. Laguerre SameVector
    let (plan, allocs, _) = count_allocations(|| {
        plan_joint_phi(
            &op,
            0.05,
            same_input.as_input(),
            PolynomialBasis::Laguerre,
            1e-7,
            None,
            &mut cost,
        )
    });
    assert!(plan.is_ok());
    assert_eq!(allocs, 0, "Laguerre same-vector allocated heap memory");

    // 5. Scalar branch (h = 0)
    let (plan, allocs, _) = count_allocations(|| {
        plan_joint_phi(
            &op,
            0.0,
            same_input.as_input(),
            PolynomialBasis::Chebyshev,
            1e-7,
            None,
            &mut cost,
        )
    });
    assert!(plan.is_ok());
    assert_eq!(allocs, 0, "Scalar branch allocated heap memory");
}

#[test]
fn test_monotone_degree_and_zero_slack_bounds() {
    let (a, _) = spectral_matrix(8, 16.0, 7100);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = TestCaseInput::distinct(8, 123);
    let mut cost = RouteCost::default();

    let budgets = [1e-4, 1e-6, 1e-8, 1e-10, 1e-12];
    let mut prev_cheb_degree = 0;
    let mut prev_lag_degree = 0;

    for &b in &budgets {
        let cheb_plan = plan_joint_phi(
            &op,
            0.05,
            input.as_input(),
            PolynomialBasis::Chebyshev,
            b * 0.25,
            None,
            &mut cost,
        )
        .unwrap();
        let lag_plan = plan_joint_phi(
            &op,
            0.05,
            input.as_input(),
            PolynomialBasis::Laguerre,
            b * 0.25,
            None,
            &mut cost,
        )
        .unwrap();

        assert!(
            cheb_plan.degree() >= prev_cheb_degree,
            "Chebyshev degree not monotone"
        );
        assert!(
            lag_plan.degree() >= prev_lag_degree,
            "Laguerre degree not monotone"
        );
        assert!(
            cheb_plan.truncation_bound() <= b * 0.25,
            "Chebyshev truncation bound exceeds budget"
        );
        assert!(
            lag_plan.truncation_bound() <= b * 0.25,
            "Laguerre truncation bound exceeds budget"
        );

        prev_cheb_degree = cheb_plan.degree();
        prev_lag_degree = lag_plan.degree();

        let mut work = WorkCounters::default();
        let mut caches = RouterCaches::default();
        let routed = route_joint_phi_single(
            &op,
            0.05,
            input.as_input(),
            b,
            RoutePolicy::ChebyshevFirstPreflight,
            Some(&mut caches),
            &mut work,
        )
        .unwrap();

        if let Some(bound) = routed.bound() {
            assert!(
                bound <= b,
                "Admitted bound {bound:e} strictly violates total budget {b:e}"
            );
        }
    }
}

#[test]
fn test_bitwise_parity_execute_joint_phi() {
    let (a, _) = spectral_matrix(8, 8.0, 7200);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = TestCaseInput::same(8, 456);
    let mut cost = RouteCost::default();

    let plan = plan_joint_phi(
        &op,
        0.05,
        input.as_input(),
        PolynomialBasis::Chebyshev,
        1e-8,
        None,
        &mut cost,
    )
    .unwrap();

    let mut work1 = WorkCounters::default();
    let mut cache1 = rodas5p_core::polynomial_action::CoefficientCache::default();
    let report_exec = execute_joint_phi(
        &plan,
        &op,
        0.05,
        input.as_input(),
        Some(&mut cache1),
        &mut work1,
    )
    .unwrap();

    let mut work2 = WorkCounters::default();
    let mut cache2 = rodas5p_core::polynomial_action::CoefficientCache::default();
    let report_direct = joint_phi_action(
        &op,
        0.05,
        input.as_input(),
        PolynomialBasis::Chebyshev,
        1e-8,
        Some(&mut cache2),
        &mut work2,
    )
    .unwrap();

    assert_eq!(report_exec.degree, report_direct.degree);
    assert_eq!(report_exec.degree, plan.degree());
    assert_eq!(
        report_exec
            .fused
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>(),
        report_direct
            .fused
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        report_exec.truncation_bound_exact_arithmetic.to_bits(),
        report_direct.truncation_bound_exact_arithmetic.to_bits()
    );
    assert_eq!(work1, work2);
    assert_eq!(work1.poly_vector_products, plan.predicted_vector_products());

    // Verify execute_joint_phi_with_cost incurs 0 tail evaluations
    let mut exec_cost = RouteCost::default();
    let mut work_cost = WorkCounters::default();
    let mut cache_cost = rodas5p_core::polynomial_action::CoefficientCache::default();
    let report_with_cost = rodas5p_core::polynomial_action::execute_joint_phi_with_cost(
        &plan,
        &op,
        0.05,
        input.as_input(),
        None,
        Some(&mut cache_cost),
        &mut work_cost,
        &mut exec_cost,
    )
    .unwrap();
    assert_eq!(exec_cost.plan_tail_evaluations, 0);
    assert_eq!(report_with_cost.degree, plan.degree());

    // Fail-fast checks on input mismatches
    let (a_other, _) = spectral_matrix(8, 9.0, 7201);
    let op_other = SymmetricNonpositiveOperator::gershgorin(a_other).unwrap();
    let err_op = execute_joint_phi(&plan, &op_other, 0.05, input.as_input(), None, &mut work1);
    assert!(matches!(
        err_op,
        Err(rodas5p_core::CoreError::InvalidInput(_))
    ));

    let err_h = execute_joint_phi(&plan, &op, 0.10, input.as_input(), None, &mut work1);
    assert!(matches!(
        err_h,
        Err(rodas5p_core::CoreError::InvalidInput(_))
    ));

    let input_mismatch = TestCaseInput::same(8, 9999);
    let err_in = execute_joint_phi(
        &plan,
        &op,
        0.05,
        input_mismatch.as_input(),
        None,
        &mut work1,
    );
    assert!(matches!(
        err_in,
        Err(rodas5p_core::CoreError::InvalidInput(_))
    ));
}

#[test]
fn test_accounting_route_cost_vs_work_counters() {
    let (a, _) = diagonal_matrix(8, 4.0);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = TestCaseInput::distinct(8, 789);

    let mut work = WorkCounters::default();
    let mut caches = RouterCaches::default();
    let routed = route_joint_phi_single(
        &op,
        0.05,
        input.as_input(),
        1e-6,
        RoutePolicy::ChebyshevFirstPreflight,
        Some(&mut caches),
        &mut work,
    )
    .unwrap();

    let cost = routed.cost();
    assert_eq!(cost.vector_products, work.poly_vector_products);
    assert_eq!(cost.block_products, work.poly_block_products);
    assert_eq!(cost.coefficient_setups, work.poly_coefficient_setups);
    assert_eq!(cost.coefficient_cache_hits, work.poly_coefficient_reuses);
    assert_eq!(cost.executed_backends, 1);
    assert_eq!(cost.failed_executions, 0);
    assert!(cost.plan_tail_evaluations > 0);
    assert!(cost.coefficient_series_terms > 0);
    assert!(cost.certification_operations > 0);
}

#[test]
fn test_force_laguerre_first_failed_preflight_charged() {
    let (a, _) = spectral_matrix(8, 64.0, 7300);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = TestCaseInput::distinct(8, 999);

    // Budget so small that Laguerre cannot admit
    let mut work = WorkCounters::default();
    let mut caches = RouterCaches::default();
    let routed = route_joint_phi_single(
        &op,
        0.12,
        input.as_input(),
        2.5e-11,
        RoutePolicy::ForceLaguerreFirst,
        Some(&mut caches),
        &mut work,
    )
    .unwrap();

    let cost = routed.cost();
    assert!(!routed.first_attempt().unwrap().is_admitted());
    assert!(routed.second_attempt().unwrap().is_admitted());
    assert_eq!(cost.executed_backends, 2);
    assert_eq!(cost.failed_executions, 1);
    assert_eq!(cost.vector_products, work.poly_vector_products);
    assert_eq!(cost.block_products, work.poly_block_products);
    assert_eq!(cost.coefficient_setups, work.poly_coefficient_setups);
    assert_eq!(cost.coefficient_cache_hits, work.poly_coefficient_reuses);
}

#[test]
fn test_cache_state_cold_vs_warm() {
    let (a, _) = diagonal_matrix(8, 4.0);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = TestCaseInput::same(8, 111);

    let mut caches = RouterCaches::default();
    let mut work_cold = WorkCounters::default();
    let cold_routed = route_joint_phi_single(
        &op,
        0.05,
        input.as_input(),
        1e-6,
        RoutePolicy::ChebyshevFirstPreflight,
        Some(&mut caches),
        &mut work_cold,
    )
    .unwrap();

    assert_eq!(cold_routed.cost().coefficient_setups, 1);
    assert_eq!(cold_routed.cost().coefficient_cache_hits, 0);
    assert!(cold_routed.cost().coefficient_series_terms > 0);

    let mut work_warm = WorkCounters::default();
    let warm_routed = route_joint_phi_single(
        &op,
        0.05,
        input.as_input(),
        1e-6,
        RoutePolicy::ChebyshevFirstPreflight,
        Some(&mut caches),
        &mut work_warm,
    )
    .unwrap();

    assert_eq!(warm_routed.cost().coefficient_setups, 0);
    assert_eq!(warm_routed.cost().coefficient_cache_hits, 1);
    assert_eq!(warm_routed.cost().coefficient_series_terms, 0);
}

#[test]
fn test_boundary_refusals() {
    let (a, _) = diagonal_matrix(8, 4.0);
    let op = SymmetricNonpositiveOperator::gershgorin(a).unwrap();
    let input = TestCaseInput::same(8, 222);
    let mut work = WorkCounters::default();
    let mut caches = RouterCaches::default();

    // 1. Negative budget
    assert!(
        route_joint_phi_single(
            &op,
            0.05,
            input.as_input(),
            -1e-6,
            RoutePolicy::ChebyshevFirstPreflight,
            Some(&mut caches),
            &mut work,
        )
        .is_err()
    );

    // 2. Negative h
    assert!(
        route_joint_phi_single(
            &op,
            -0.05,
            input.as_input(),
            1e-6,
            RoutePolicy::ChebyshevFirstPreflight,
            Some(&mut caches),
            &mut work,
        )
        .is_err()
    );

    // 3. Zero budget -> Fallback
    let zero_routed = route_joint_phi_single(
        &op,
        0.05,
        input.as_input(),
        0.0,
        RoutePolicy::ChebyshevFirstPreflight,
        Some(&mut caches),
        &mut work,
    )
    .unwrap();
    assert!(zero_routed.is_fallback());

    // 4. Mismatched dimension
    let bad_input = TestCaseInput::same(6, 222);
    assert!(
        route_joint_phi_single(
            &op,
            0.05,
            bad_input.as_input(),
            1e-6,
            RoutePolicy::ChebyshevFirstPreflight,
            Some(&mut caches),
            &mut work,
        )
        .is_err()
    );
}

// -----------------------------------------------------------------------------
// Exporter
// -----------------------------------------------------------------------------

fn run_arm_cheb(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    caches: &mut RouterCaches,
) -> Value {
    let truncation_budget = total_budget * 0.25;
    let mut plan_cost = RouteCost::default();
    let plan_res = plan_joint_phi(
        op,
        h,
        input,
        PolynomialBasis::Chebyshev,
        truncation_budget,
        Some(caches),
        &mut plan_cost,
    );
    let mut work = WorkCounters::default();
    let before_work = work;
    let report_res = match &plan_res {
        Ok(plan) => execute_joint_phi(plan, op, h, input, Some(&mut caches.coefficient), &mut work),
        Err(_) => joint_phi_action(
            op,
            h,
            input,
            PolynomialBasis::Chebyshev,
            truncation_budget,
            Some(&mut caches.coefficient),
            &mut work,
        ),
    };
    let delta = work.checked_delta(before_work).unwrap();
    let (degree, admission, report) = match report_res {
        Ok(rep) => {
            let adm = route_admission(&rep, total_budget);
            (Some(rep.degree), adm, Some(rep))
        }
        Err(err) => (
            None,
            TotalErrorAdmission::Rejected {
                reason: format!("action failed: {err}"),
            },
            None,
        ),
    };
    let mut cost = RouteCost {
        executed_backends: 1,
        vector_products: delta.poly_vector_products,
        block_products: delta.poly_block_products,
        coefficient_setups: delta.poly_coefficient_setups,
        coefficient_cache_hits: delta.poly_coefficient_reuses,
        ..RouteCost::default()
    };
    if let Some(ref rep) = report {
        let deg = rep.degree;
        if delta.poly_coefficient_setups > 0 {
            let shift = 0.5 * (op.enclosure().rho + op.enclosure().lambda);
            let half_width = 0.5 * (op.enclosure().rho - op.enclosure().lambda);
            let a = rodas5p_core::directed::Interval::new(
                -rodas5p_core::directed::mul_up(h, shift).unwrap(),
                -rodas5p_core::directed::mul_down(h, shift).unwrap(),
            )
            .unwrap();
            let b = rodas5p_core::directed::Interval::new(
                rodas5p_core::directed::mul_down(h, half_width).unwrap(),
                rodas5p_core::directed::mul_up(h, half_width).unwrap(),
            )
            .unwrap();
            cost.coefficient_series_terms = count_chebyshev_series_terms(a, b, deg).unwrap();
        }
        cost.certification_operations = 5 * ((deg * (deg + 1)) / 2) as u64;
    }
    let bound = match &admission {
        TotalErrorAdmission::Admitted { bound, .. } => Some(*bound),
        _ => None,
    };
    let fused = report.as_ref().map(|r| r.fused.clone());

    let is_adm = matches!(admission, TotalErrorAdmission::Admitted { .. });
    json!({
        "arm": "cheb",
        "choice": if is_adm { "Chebyshev" } else { "Fallback" },
        "admitted": is_adm,
        "bound": bound.map(hex),
        "bound_f64": bound,
        "fused": fused.as_ref().map(|f| hex_vec(f)),
        "cost": cost,
        "plan_cost": RouteCost::default(),
        "execution_costs": [cost],
        "caller_counters": delta,
        "degree": degree,
    })
}

fn run_arm_pp08(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
) -> Value {
    let mut work = WorkCounters::default();
    let routed = route_joint_phi(op, h, input, total_budget, &mut work).unwrap();
    let choice_str = match routed.choice() {
        RouteChoice::Chebyshev => "Chebyshev",
        RouteChoice::Laguerre => "Laguerre",
        RouteChoice::Fallback { .. } => "Fallback",
    };
    let cheb_cost = RouteCost {
        executed_backends: 1,
        vector_products: routed.attempt(PolynomialBasis::Chebyshev).vector_products(),
        coefficient_setups: routed
            .attempt(PolynomialBasis::Chebyshev)
            .coefficient_setups(),
        ..RouteCost::default()
    };
    let lag_cost = RouteCost {
        executed_backends: 1,
        vector_products: routed.attempt(PolynomialBasis::Laguerre).vector_products(),
        coefficient_setups: routed
            .attempt(PolynomialBasis::Laguerre)
            .coefficient_setups(),
        ..RouteCost::default()
    };

    let mut total_cost = RouteCost::default();
    total_cost.accumulate(cheb_cost);
    total_cost.accumulate(lag_cost);

    json!({
        "arm": "pp08",
        "choice": choice_str,
        "admitted": !routed.is_fallback(),
        "bound": routed.bound().map(hex),
        "bound_f64": routed.bound(),
        "fused": routed.fused().map(hex_vec),
        "cost": total_cost,
        "plan_cost": RouteCost::default(),
        "execution_costs": [cheb_cost, lag_cost],
        "caller_counters": work,
    })
}

fn run_arm_single(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    total_budget: f64,
    policy: RoutePolicy,
    caches: &mut RouterCaches,
) -> Value {
    let mut work = WorkCounters::default();
    let routed =
        route_joint_phi_single(op, h, input, total_budget, policy, Some(caches), &mut work)
            .unwrap();
    let choice_str = match routed.choice() {
        RouteChoice::Chebyshev => "Chebyshev",
        RouteChoice::Laguerre => "Laguerre",
        RouteChoice::Fallback { .. } => "Fallback",
    };
    let preselected_laguerre = match routed.policy() {
        RoutePolicy::ForceLaguerreFirst => true,
        RoutePolicy::ChebyshevFirstPreflight => {
            if let (Some(cheb), Some(lag)) = (routed.chebyshev_plan(), routed.laguerre_plan()) {
                lag.predicted_vector_products() < cheb.predicted_vector_products()
                    && lag.degree() <= LAGUERRE_ADJOINT_DEGREE_LIMIT
                    && lag.truncation_bound() <= routed.total_budget()
            } else {
                false
            }
        }
    };

    json!({
        "arm": "single",
        "choice": choice_str,
        "admitted": !routed.is_fallback(),
        "bound": routed.bound().map(hex),
        "bound_f64": routed.bound(),
        "fused": routed.fused().map(hex_vec),
        "cost": routed.cost(),
        "plan_cost": routed.plan_cost(),
        "execution_costs": routed.execution_costs(),
        "caller_counters": work,
        "preselected_laguerre": preselected_laguerre,
        "chebyshev_plan": routed.chebyshev_plan().map(|p| json!({
            "degree": p.degree(),
            "predicted_vector_products": p.predicted_vector_products(),
            "truncation_bound": hex(p.truncation_bound()),
        })),
        "laguerre_plan": routed.laguerre_plan().map(|p| json!({
            "degree": p.degree(),
            "predicted_vector_products": p.predicted_vector_products(),
            "truncation_bound": hex(p.truncation_bound()),
        })),
    })
}

#[test]
#[ignore = "export PY03 cases; set PY03_CASES"]
fn export_cases() {
    let Ok(output) = std::env::var("PY03_CASES") else {
        println!("PY03_CASES not set: export skipped");
        return;
    };
    let raw_path = Path::new(&output);
    let path = if raw_path.is_absolute() {
        raw_path.to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(raw_path)
    };
    assert!(!path.exists(), "immutable output already exists: {output}");

    let mut cases = Vec::new();
    let mut case_counter = 0;

    // 1. Grid Cases (96 cases)
    let n_values = [8, 24];
    let families = ["diagonal", "rotated"];
    let rho_values = [4.0, 64.0];
    let h_values = [0.005, 0.05, 0.12];
    let inputs = ["distinct", "same"];
    let budgets = [1e-6, 1e-10];

    for &n in &n_values {
        for &family in &families {
            for &rho in &rho_values {
                let seed = 7001 + case_counter as u64;
                let (a, spectrum) = if family == "diagonal" {
                    diagonal_matrix(n, rho)
                } else {
                    spectral_matrix(n, rho, seed)
                };
                let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();

                for &h in &h_values {
                    for &inp in &inputs {
                        let test_input = if inp == "distinct" {
                            TestCaseInput::distinct(n, seed + 10)
                        } else {
                            TestCaseInput::same(n, seed + 10)
                        };

                        for &budget in &budgets {
                            case_counter += 1;
                            let label = format!(
                                "grid-{case_counter:02}-n{n}-{family}-rho{rho}-h{h}-{inp}-b{budget:e}"
                            );

                            let mut caches = RouterCaches::default();

                            // Cold run
                            let single_cold = run_arm_single(
                                &op,
                                h,
                                test_input.as_input(),
                                budget,
                                RoutePolicy::ChebyshevFirstPreflight,
                                &mut caches,
                            );
                            let cheb_cold =
                                run_arm_cheb(&op, h, test_input.as_input(), budget, &mut caches);
                            let pp08_cold = run_arm_pp08(&op, h, test_input.as_input(), budget);

                            // Warm run (caches retained)
                            let single_warm = run_arm_single(
                                &op,
                                h,
                                test_input.as_input(),
                                budget,
                                RoutePolicy::ChebyshevFirstPreflight,
                                &mut caches,
                            );
                            let cheb_warm =
                                run_arm_cheb(&op, h, test_input.as_input(), budget, &mut caches);
                            let pp08_warm = run_arm_pp08(&op, h, test_input.as_input(), budget);

                            cases.push(json!({
                                "label": label,
                                "group": "grid",
                                "n": n,
                                "family": family,
                                "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
                                "spectrum": spectrum,
                                "h": hex(h),
                                "h_f64": h,
                                "input": inp,
                                "w": test_input.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
                                "total_budget": hex(budget),
                                "total_budget_f64": budget,
                                "enclosure": {
                                    "lambda": op.enclosure().lambda,
                                    "rho": op.enclosure().rho,
                                    "evidence": "Gershgorin",
                                },
                                "runs": {
                                    "cold": {
                                        "single": single_cold,
                                        "cheb": cheb_cold,
                                        "pp08": pp08_cold,
                                    },
                                    "warm": {
                                        "single": single_warm,
                                        "cheb": cheb_warm,
                                        "pp08": pp08_warm,
                                    }
                                }
                            }));
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases.len(), 96, "Expected exactly 96 grid cases");

    // 2. Boundary Cases (7 cases)
    // 2.1 Declared enclosure
    {
        let n = 8;
        let rho = 4.0;
        let (a, spectrum) = diagonal_matrix(n, rho);
        let op =
            SymmetricNonpositiveOperator::with_declared_enclosure(a.clone(), 0.0, rho, "declared")
                .unwrap();
        let inp = TestCaseInput::same(n, 8001);
        let h = 0.05;
        let budget = 1e-6;
        let mut caches = RouterCaches::default();
        let single_cold = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_cold = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_cold = run_arm_pp08(&op, h, inp.as_input(), budget);

        let single_warm = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_warm = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_warm = run_arm_pp08(&op, h, inp.as_input(), budget);
        cases.push(json!({
            "label": "boundary-01-declared-enclosure",
            "group": "boundary",
            "n": n,
            "family": "diagonal",
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "spectrum": spectrum,
            "h": hex(h),
            "h_f64": h,
            "input": "same",
            "w": inp.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
            "total_budget": hex(budget),
            "total_budget_f64": budget,
            "enclosure": {
                "lambda": op.enclosure().lambda,
                "rho": op.enclosure().rho,
                "evidence": "Declared",
            },
            "runs": {
                "cold": { "single": single_cold, "cheb": cheb_cold, "pp08": pp08_cold },
                "warm": { "single": single_warm, "cheb": cheb_warm, "pp08": pp08_warm }
            }
        }));
    }

    // 2.2 Unbounded timing execution
    {
        let n = 8;
        let rho = 4.0;
        let (a, spectrum) = diagonal_matrix(n, rho);
        let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
        let inp = TestCaseInput::same(n, 8002);
        let h = 0.05;
        let budget = 1e-6;
        let mut work = WorkCounters::default();
        let report = joint_phi_action_unbounded(
            &op,
            h,
            inp.as_input(),
            PolynomialBasis::Chebyshev,
            budget * 0.25,
            None,
            &mut work,
        )
        .unwrap();
        let adm = route_admission(&report, budget);
        assert!(
            !matches!(adm, TotalErrorAdmission::Admitted { .. }),
            "Timing execution must never admit"
        );
        cases.push(json!({
            "label": "boundary-02-unbounded-timing",
            "group": "boundary",
            "n": n,
            "family": "diagonal",
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "spectrum": spectrum,
            "h": hex(h),
            "h_f64": h,
            "input": "same",
            "w": inp.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
            "total_budget": hex(budget),
            "total_budget_f64": budget,
            "enclosure": {
                "lambda": op.enclosure().lambda,
                "rho": op.enclosure().rho,
                "evidence": "Gershgorin",
            },
            "runs": {
                "cold": {
                    "single": { "arm": "single", "choice": "Fallback", "admitted": false, "cost": RouteCost::default(), "plan_cost": RouteCost::default(), "execution_costs": [] },
                    "cheb": { "arm": "cheb", "choice": "Fallback", "admitted": false, "cost": RouteCost::default(), "plan_cost": RouteCost::default(), "execution_costs": [] },
                    "pp08": { "arm": "pp08", "choice": "Fallback", "admitted": false, "cost": RouteCost::default(), "plan_cost": RouteCost::default(), "execution_costs": [] },
                },
                "warm": {
                    "single": { "arm": "single", "choice": "Fallback", "admitted": false, "cost": RouteCost::default(), "plan_cost": RouteCost::default(), "execution_costs": [] },
                    "cheb": { "arm": "cheb", "choice": "Fallback", "admitted": false, "cost": RouteCost::default(), "plan_cost": RouteCost::default(), "execution_costs": [] },
                    "pp08": { "arm": "pp08", "choice": "Fallback", "admitted": false, "cost": RouteCost::default(), "plan_cost": RouteCost::default(), "execution_costs": [] },
                }
            }
        }));
    }

    // 2.3 Laguerre degree > 128
    {
        let n = 8;
        let rho = 64.0;
        let (a, spectrum) = diagonal_matrix(n, rho);
        let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
        let inp = TestCaseInput::distinct(n, 8003);
        let h = 2.0; // Large step forces high degree
        let budget = 1e-12;
        let mut caches = RouterCaches::default();
        let single_cold = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_cold = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_cold = run_arm_pp08(&op, h, inp.as_input(), budget);

        let single_warm = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_warm = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_warm = run_arm_pp08(&op, h, inp.as_input(), budget);
        cases.push(json!({
            "label": "boundary-03-laguerre-degree-gt-128",
            "group": "boundary",
            "n": n,
            "family": "diagonal",
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "spectrum": spectrum,
            "h": hex(h),
            "h_f64": h,
            "input": "distinct",
            "w": inp.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
            "total_budget": hex(budget),
            "total_budget_f64": budget,
            "enclosure": {
                "lambda": op.enclosure().lambda,
                "rho": op.enclosure().rho,
                "evidence": "Gershgorin",
            },
            "runs": {
                "cold": { "single": single_cold, "cheb": cheb_cold, "pp08": pp08_cold },
                "warm": { "single": single_warm, "cheb": cheb_warm, "pp08": pp08_warm }
            }
        }));
    }

    // 2.4 Total budget 0
    {
        let n = 8;
        let rho = 4.0;
        let (a, spectrum) = diagonal_matrix(n, rho);
        let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
        let inp = TestCaseInput::same(n, 8004);
        let h = 0.05;
        let budget = 0.0;
        let mut caches = RouterCaches::default();
        let single_cold = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_cold = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_cold = run_arm_pp08(&op, h, inp.as_input(), budget);

        let single_warm = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_warm = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_warm = run_arm_pp08(&op, h, inp.as_input(), budget);
        cases.push(json!({
            "label": "boundary-04-total-budget-zero",
            "group": "boundary",
            "n": n,
            "family": "diagonal",
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "spectrum": spectrum,
            "h": hex(h),
            "h_f64": h,
            "input": "same",
            "w": inp.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
            "total_budget": hex(budget),
            "total_budget_f64": budget,
            "enclosure": {
                "lambda": op.enclosure().lambda,
                "rho": op.enclosure().rho,
                "evidence": "Gershgorin",
            },
            "runs": {
                "cold": { "single": single_cold, "cheb": cheb_cold, "pp08": pp08_cold },
                "warm": { "single": single_warm, "cheb": cheb_warm, "pp08": pp08_warm }
            }
        }));
    }

    // 2.5 Scalar branch (-3I)
    {
        let n = 8;
        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            a[(i, i)] = -3.0;
        }
        let spectrum = vec![-3.0; n];
        let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
        let inp = TestCaseInput::same(n, 8005);
        let h = 0.05;
        let budget = 1e-6;
        let mut caches = RouterCaches::default();
        let single_cold = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_cold = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_cold = run_arm_pp08(&op, h, inp.as_input(), budget);

        let single_warm = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        let cheb_warm = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_warm = run_arm_pp08(&op, h, inp.as_input(), budget);
        cases.push(json!({
            "label": "boundary-05-scalar-branch",
            "group": "boundary",
            "n": n,
            "family": "diagonal",
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "spectrum": spectrum,
            "h": hex(h),
            "h_f64": h,
            "input": "same",
            "w": inp.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
            "total_budget": hex(budget),
            "total_budget_f64": budget,
            "enclosure": {
                "lambda": op.enclosure().lambda,
                "rho": op.enclosure().rho,
                "evidence": "Gershgorin",
            },
            "runs": {
                "cold": { "single": single_cold, "cheb": cheb_cold, "pp08": pp08_cold },
                "warm": { "single": single_warm, "cheb": cheb_warm, "pp08": pp08_warm }
            }
        }));
    }

    // 2.6 ForceLaguerreFirst Diagonal
    {
        let n = 8;
        let rho = 64.0;
        let (a, spectrum) = diagonal_matrix(n, rho);
        let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
        let inp = TestCaseInput::distinct(n, 8006);
        let h = 0.12;
        let budget = 2.0e-11;
        let mut caches = RouterCaches::default();
        let single_cold = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ForceLaguerreFirst,
            &mut caches,
        );
        let cheb_cold = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_cold = run_arm_pp08(&op, h, inp.as_input(), budget);

        let single_warm = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ForceLaguerreFirst,
            &mut caches,
        );
        let cheb_warm = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_warm = run_arm_pp08(&op, h, inp.as_input(), budget);
        cases.push(json!({
            "label": "boundary-06-force-laguerre-diagonal",
            "group": "boundary",
            "n": n,
            "family": "diagonal",
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "spectrum": spectrum,
            "h": hex(h),
            "h_f64": h,
            "input": "distinct",
            "w": inp.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
            "total_budget": hex(budget),
            "total_budget_f64": budget,
            "enclosure": {
                "lambda": op.enclosure().lambda,
                "rho": op.enclosure().rho,
                "evidence": "Gershgorin",
            },
            "runs": {
                "cold": { "single": single_cold, "cheb": cheb_cold, "pp08": pp08_cold },
                "warm": { "single": single_warm, "cheb": cheb_warm, "pp08": pp08_warm }
            }
        }));
    }

    // 2.7 ForceLaguerreFirst Rotated
    {
        let n = 8;
        let rho = 64.0;
        let (a, spectrum) = spectral_matrix(n, rho, 8007);
        let op = SymmetricNonpositiveOperator::gershgorin(a.clone()).unwrap();
        let inp = TestCaseInput::distinct(n, 8007);
        let h = 0.12;
        let budget = 2.5e-11;
        let mut caches = RouterCaches::default();
        let single_cold = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ForceLaguerreFirst,
            &mut caches,
        );
        let cheb_cold = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_cold = run_arm_pp08(&op, h, inp.as_input(), budget);

        let single_warm = run_arm_single(
            &op,
            h,
            inp.as_input(),
            budget,
            RoutePolicy::ForceLaguerreFirst,
            &mut caches,
        );
        let cheb_warm = run_arm_cheb(&op, h, inp.as_input(), budget, &mut caches);
        let pp08_warm = run_arm_pp08(&op, h, inp.as_input(), budget);
        cases.push(json!({
            "label": "boundary-07-force-laguerre-rotated",
            "group": "boundary",
            "n": n,
            "family": "rotated",
            "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "spectrum": spectrum,
            "h": hex(h),
            "h_f64": h,
            "input": "distinct",
            "w": inp.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
            "total_budget": hex(budget),
            "total_budget_f64": budget,
            "enclosure": {
                "lambda": op.enclosure().lambda,
                "rho": op.enclosure().rho,
                "evidence": "Gershgorin",
            },
            "runs": {
                "cold": { "single": single_cold, "cheb": cheb_cold, "pp08": pp08_cold },
                "warm": { "single": single_warm, "cheb": cheb_warm, "pp08": pp08_warm }
            }
        }));
    }

    assert_eq!(cases.len(), 103, "Expected 103 total cases");

    let doc = json!({
        "schema": "vigilode-py03-single-backend-router-cases-v1",
        "cases": cases,
    });

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    println!("Wrote 103 cases to {output}");
}
