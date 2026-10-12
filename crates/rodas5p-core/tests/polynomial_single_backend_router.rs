//! Plan first, execute one polynomial backend (research node PY03,
//! `research/py03_single_backend_router_20261011`).
//!
//! Contract tests (the DAG command, `cargo test -p rodas5p-core --test
//! polynomial_single_backend_router`): plan/execute parity with
//! `joint_phi_action`, the charged failed preflight, the cache keys and the
//! boundary refusals, on fresh fixtures (other sizes and seeds than the
//! registered grid).
//!
//! The ignored `export_cases` runs the registered grid and boundary cases
//! once and writes `PY03_CASES` (compact JSON) for
//! `tools/py03_single_backend_router_check.py`.

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT,
    polynomial_action::{
        ActionPlan, EXECUTION_UNBOUNDED_TIMING, EnclosureEvidence, JointPhiInput, JointPhiReport,
        PLAN_MISMATCH, PREFLIGHT_PRODUCTS_NOT_FEWER, PlanBranch, PolynomialBasis, PolynomialCaches,
        ROUTER_ENCLOSURE_NOT_VERIFIED, ROUTER_INPUT_UNSUPPORTED, RouteChoice, RouteCost,
        RoutePolicy, RoutedPhi, SingleRoutedPhi, SymmetricNonpositiveOperator, TotalErrorAdmission,
        TotalErrorStatus, execute_joint_phi, execute_joint_phi_counted,
        execute_joint_phi_unbounded, joint_phi_action, joint_phi_action_unbounded,
        laguerre_preflight_terms, plan_joint_phi, route_admission, route_joint_phi,
        route_joint_phi_single,
    },
};
use serde_json::{Value, json};

const SCALES: [f64; 5] = [1.0, -0.5, 0.25, 2.0, -1.0];
const BASES: [PolynomialBasis; 2] = [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre];

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

// ---------------------------------------------------------------------------
// Fixtures (registered constructions)
// ---------------------------------------------------------------------------

/// Fraction of `rho` at the upper end of the rotated spectrum.
const LAMBDA_MIN_FRACTION: f64 = 0.1;
/// Largest Givens angle of the sweep that forms `Q`.
const MAX_ANGLE: f64 = 0.3;

/// PP08's construction: `A = Q diag(lambda) Q^T` rounded and symmetrized,
/// `lambda` in `[-rho, -0.1 rho]` with both ends present, `Q` one sweep of
/// adjacent Givens rotations with angles in `[-0.3, 0.3]`.
fn rotated_matrix(n: usize, rho: f64, seed: u64) -> (DenseMatrix, Vec<f64>) {
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

/// The next seed from `*next` upward whose rotated matrix has a
/// Gershgorin-verified enclosure, and its operator.
fn rotated_fixture(
    n: usize,
    rho: f64,
    next: &mut u64,
) -> (DenseMatrix, SymmetricNonpositiveOperator, u64) {
    for _ in 0..1000 {
        let seed = *next;
        *next += 1;
        let (a, _) = rotated_matrix(n, rho, seed);
        if let Ok(op) = SymmetricNonpositiveOperator::gershgorin(a.clone()) {
            assert_eq!(op.enclosure().evidence, EnclosureEvidence::Gershgorin);
            return (a, op, seed);
        }
    }
    panic!("no verified rotated fixture in 1000 seeds")
}

/// Diagonal with the spectrum evenly spaced in `[-rho, 0]`, both ends
/// present; the enclosure `[-rho, 0]` is Gershgorin-verified exactly.
fn diagonal_fixture(n: usize, rho: f64) -> (DenseMatrix, SymmetricNonpositiveOperator) {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -(rho * i as f64) / (n - 1) as f64;
    }
    assert_eq!(a[(n - 1, n - 1)], -rho);
    let op = SymmetricNonpositiveOperator::new(a.clone(), 0.0, rho, "py03-diagonal").unwrap();
    assert_eq!(op.enclosure().evidence, EnclosureEvidence::Gershgorin);
    (a, op)
}

/// The largest diagonal magnitude encloses the diagonal but not the discs.
fn declared_operator(a: &DenseMatrix, source: &str) -> SymmetricNonpositiveOperator {
    let n = a.nrows();
    let diagonal_max = (0..n).map(|i| a[(i, i)].abs()).fold(0.0, f64::max);
    let op = SymmetricNonpositiveOperator::new(a.clone(), 0.0, diagonal_max, source).unwrap();
    assert!(matches!(
        op.enclosure().evidence,
        EnclosureEvidence::Declared { .. }
    ));
    op
}

fn minus_three_identity(n: usize) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -3.0;
    }
    a
}

#[derive(Clone)]
enum Input {
    Distinct([Vec<f64>; 5]),
    Same(Vec<f64>),
}

impl Input {
    fn distinct(n: usize, seed: u64) -> Self {
        Input::Distinct(std::array::from_fn(|k| splitmix(seed + 77 * k as u64, n)))
    }

    fn same(n: usize, seed: u64) -> Self {
        Input::Same(splitmix(seed, n))
    }

    fn scaled(&self, factor: f64) -> Self {
        match self {
            Input::Distinct(w) => Input::Distinct(std::array::from_fn(|k| {
                w[k].iter().map(|x| x * factor).collect()
            })),
            Input::Same(v) => Input::Same(v.iter().map(|x| x * factor).collect()),
        }
    }

    fn as_input(&self) -> JointPhiInput<'_> {
        match self {
            Input::Distinct(w) => JointPhiInput::Distinct(w),
            Input::Same(v) => JointPhiInput::SameVector {
                vector: v,
                scales: SCALES,
            },
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Input::Distinct(_) => "distinct",
            Input::Same(_) => "same",
        }
    }

    /// The five `w_k` (the scales are powers of two, so exact).
    fn columns(&self) -> Vec<Vec<f64>> {
        match self {
            Input::Distinct(w) => w.to_vec(),
            Input::Same(v) => SCALES
                .iter()
                .map(|s| v.iter().map(|x| s * x).collect())
                .collect(),
        }
    }
}

/// Bitwise report equality: serde_json writes the shortest round-trip form
/// of every finite binary64 (with the sign of zero), so equal text is equal
/// bits; the fused and column bits are compared as well.
fn same_report(a: &JointPhiReport, b: &JointPhiReport) -> bool {
    serde_json::to_string(a).unwrap() == serde_json::to_string(b).unwrap()
        && a.fused
            .iter()
            .map(|x| x.to_bits())
            .eq(b.fused.iter().map(|x| x.to_bits()))
        && a.columns.iter().flatten().map(|x| x.to_bits()).eq(b
            .columns
            .iter()
            .flatten()
            .map(|x| x.to_bits()))
}

fn admitted(a: &TotalErrorAdmission) -> bool {
    matches!(a, TotalErrorAdmission::Admitted { .. })
}

// ---------------------------------------------------------------------------
// Contract helpers
// ---------------------------------------------------------------------------

/// Plan and execute against `joint_phi_action` with the same cache state:
/// the report, the counters and the cache contents agree, and the
/// execution's products are the plan's.
fn assert_plan_execute_parity(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    basis: PolynomialBasis,
    budget: f64,
    caches: &mut PolynomialCaches,
) -> JointPhiReport {
    let mut reference_cache = caches.coefficients.clone();
    let mut plan_cost = RouteCost::default();
    let tables_before = caches.coefficients.len();
    let plan = plan_joint_phi(
        op,
        h,
        input.as_input(),
        basis,
        budget,
        caches,
        &mut plan_cost,
    )
    .unwrap();
    assert_eq!(
        caches.coefficients.len(),
        tables_before,
        "a plan builds no table"
    );
    assert_eq!(plan_cost.plan_tail_evaluations, plan.tail_evaluations());
    assert_eq!(plan_cost.vector_products, 0);
    let mut work = WorkCounters::default();
    let mut cost = RouteCost::default();
    let report =
        execute_joint_phi_counted(&plan, op, h, input.as_input(), caches, &mut work, &mut cost)
            .unwrap();
    let mut reference_work = WorkCounters::default();
    let reference = joint_phi_action(
        op,
        h,
        input.as_input(),
        basis,
        budget,
        Some(&mut reference_cache),
        &mut reference_work,
    )
    .unwrap();
    assert!(same_report(&report, &reference), "report differs");
    assert_eq!(work, reference_work, "counters differ");
    assert_eq!(caches.coefficients.len(), reference_cache.len());
    assert_eq!(report.degree, plan.degree());
    assert_eq!(cost.vector_products, plan.predicted_vector_products());
    assert_eq!(cost.vector_products, work.poly_vector_products);
    assert_eq!(cost.block_products, work.poly_block_products);
    assert_eq!(cost.coefficient_setups, work.poly_coefficient_setups);
    assert_eq!(cost.coefficient_cache_hits, work.poly_coefficient_reuses);
    assert_eq!(cost.executed_backends, 1);
    assert_eq!(cost.plan_tail_evaluations, 0);
    if plan.branch() == PlanBranch::Recurrence {
        assert_eq!(
            plan.coefficients_cached(),
            report.coefficient_cache_hit,
            "the plan's cache lookup predicts the hit"
        );
        if basis == PolynomialBasis::Chebyshev {
            let d = report.degree as u64;
            assert_eq!(cost.chebyshev_propagation_terms, 5 * d * (d + 1) / 2);
            assert_eq!(cost.adjoint_sum_terms, 0);
        } else if report.degree <= LAGUERRE_ADJOINT_DEGREE_LIMIT {
            assert_eq!(cost.adjoint_sum_terms, 5 * report.degree as u64);
            assert_eq!(cost.envelope_cache_hits + cost.envelope_cache_misses, 5);
            assert_eq!(cost.chebyshev_propagation_terms, 0);
        }
    }
    // The plain execution entry point agrees as well.
    let mut again = caches.clone();
    let mut again_reference = caches.coefficients.clone();
    let mut w1 = WorkCounters::default();
    let mut w2 = WorkCounters::default();
    let r1 = execute_joint_phi(&plan, op, h, input.as_input(), &mut again, &mut w1).unwrap();
    let r2 = joint_phi_action(
        op,
        h,
        input.as_input(),
        basis,
        budget,
        Some(&mut again_reference),
        &mut w2,
    )
    .unwrap();
    assert!(same_report(&r1, &r2));
    assert_eq!(w1, w2);
    // The timing execution agrees with the timing action.
    let mut timing = caches.clone();
    let mut timing_reference = caches.coefficients.clone();
    let mut w3 = WorkCounters::default();
    let mut w4 = WorkCounters::default();
    let t1 =
        execute_joint_phi_unbounded(&plan, op, h, input.as_input(), &mut timing, &mut w3).unwrap();
    let t2 = joint_phi_action_unbounded(
        op,
        h,
        input.as_input(),
        basis,
        budget,
        Some(&mut timing_reference),
        &mut w4,
    )
    .unwrap();
    assert!(same_report(&t1, &t2));
    assert_eq!(w3, w4);
    report
}

/// The route's accounting identities.
fn assert_accounting(routed: &SingleRoutedPhi, caller: &WorkCounters) {
    let mut sum = routed.plan_cost();
    let mut work = routed.preflight().work();
    for execution in routed.executions() {
        sum.accumulate(execution.cost());
        work.accumulate(execution.work());
        let c = execution.cost();
        let w = execution.work();
        assert_eq!(c.vector_products, w.poly_vector_products);
        assert_eq!(c.block_products, w.poly_block_products);
        assert_eq!(c.coefficient_setups, w.poly_coefficient_setups);
        assert_eq!(c.coefficient_cache_hits, w.poly_coefficient_reuses);
        assert_eq!(c.executed_backends, u64::from(execution.executed()));
        assert_eq!(c.failed_executions, u64::from(!execution.is_admitted()));
        if let (Some(report), Some(plan)) = (
            execution.report(),
            routed.plan(execution.basis()).and_then(|p| p.plan()),
        ) {
            assert_eq!(c.vector_products, plan.predicted_vector_products());
            assert_eq!(report.degree, plan.degree());
            assert_eq!(
                *execution.admission(),
                route_admission(report, routed.total_budget())
            );
        }
    }
    assert_eq!(
        sum,
        routed.cost(),
        "route cost = plans + preflight + executions"
    );
    assert_eq!(work, routed.work(), "caller work = preflight + executions");
    assert_eq!(
        *caller,
        routed.work(),
        "the caller's counters started at zero"
    );
    let c = routed.cost();
    assert_eq!(c.vector_products, caller.poly_vector_products);
    assert_eq!(c.block_products, caller.poly_block_products);
    assert_eq!(c.coefficient_setups, caller.poly_coefficient_setups);
    assert_eq!(c.coefficient_cache_hits, caller.poly_coefficient_reuses);
    // The choice is the admitted execution, if any.
    match routed.choice() {
        RouteChoice::Fallback {
            chebyshev_reason,
            laguerre_reason,
        } => {
            assert!(routed.executions().iter().all(|e| !e.is_admitted()));
            assert!(!chebyshev_reason.is_empty() && !laguerre_reason.is_empty());
            assert!(routed.fused().is_none() && routed.bound().is_none());
        }
        _ => {
            let last = routed.executions().last().unwrap();
            assert!(last.is_admitted());
            assert_eq!(Some(last.basis()), routed.basis());
            assert!(routed.bound().unwrap() <= routed.total_budget());
            assert_eq!(
                routed.fused(),
                Some(last.report().unwrap().fused.as_slice())
            );
            assert!(
                routed.executions()[..routed.executions().len() - 1]
                    .iter()
                    .all(|e| !e.is_admitted())
            );
        }
    }
}

fn route_single(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    total_budget: f64,
    policy: RoutePolicy,
    caches: &mut PolynomialCaches,
) -> SingleRoutedPhi {
    let mut work = WorkCounters::default();
    let routed = route_joint_phi_single(
        op,
        h,
        input.as_input(),
        total_budget,
        policy,
        caches,
        &mut work,
    )
    .unwrap();
    assert_accounting(&routed, &work);
    routed
}

// ---------------------------------------------------------------------------
// Contract tests (fresh fixtures: n in {5, 6, 10}, seeds 9001..)
// ---------------------------------------------------------------------------

#[test]
fn plan_and_execute_equal_the_action_bitwise() {
    let (_, diagonal) = diagonal_fixture(6, 10.0);
    let mut next = 9001;
    let (_, rotated, _) = rotated_fixture(6, 10.0, &mut next);
    let scalar = SymmetricNonpositiveOperator::gershgorin(minus_three_identity(5)).unwrap();
    for (op, n) in [(&diagonal, 6), (&rotated, 6), (&scalar, 5)] {
        for input in [Input::distinct(n, 9100), Input::same(n, 9200)] {
            for h in [0.0, 0.03, 0.4] {
                for basis in BASES {
                    // Cold, then warm with the caches the cold call left.
                    let mut caches = PolynomialCaches::default();
                    let cold =
                        assert_plan_execute_parity(op, h, &input, basis, 2.5e-9, &mut caches);
                    let warm =
                        assert_plan_execute_parity(op, h, &input, basis, 2.5e-9, &mut caches);
                    assert_eq!(cold.fused, warm.fused);
                    if cold.branch == "recurrence" {
                        assert!(!cold.coefficient_cache_hit && warm.coefficient_cache_hit);
                    }
                }
            }
        }
    }
    // Zero input, and inputs out of the normalization window.
    let zero = Input::Distinct(std::array::from_fn(|_| vec![0.0; 6]));
    let mut caches = PolynomialCaches::default();
    for basis in BASES {
        let report = assert_plan_execute_parity(&diagonal, 0.1, &zero, basis, 1e-9, &mut caches);
        assert_eq!(report.degree, 0);
    }
    for amplitude in [1.0e-310, 1.0e300] {
        let input = Input::distinct(6, 9300).scaled(amplitude);
        for basis in BASES {
            let mut caches = PolynomialCaches::default();
            let report = assert_plan_execute_parity(
                &rotated,
                0.05,
                &input,
                basis,
                1.0e-9 * amplitude,
                &mut caches,
            );
            assert_ne!(report.normalization_shift, 0);
        }
    }
}

#[test]
fn a_plan_is_bound_to_its_call() {
    let (_, op) = diagonal_fixture(6, 10.0);
    let (_, other) = diagonal_fixture(6, 12.0);
    let input = Input::distinct(6, 9400);
    let changed = Input::distinct(6, 9401);
    let caches = PolynomialCaches::default();
    let plan = plan_joint_phi(
        &op,
        0.05,
        input.as_input(),
        PolynomialBasis::Chebyshev,
        1e-9,
        &caches,
        &mut RouteCost::default(),
    )
    .unwrap();
    assert_eq!(plan.branch(), PlanBranch::Recurrence);
    assert!(plan.degree() > 0);
    assert_eq!(plan.predicted_vector_products(), 5 * plan.degree() as u64);
    for (op, h, input) in [
        (&other, 0.05, &input),
        (&op, 0.06, &input),
        (&op, 0.05, &changed),
    ] {
        let mut caches = PolynomialCaches::default();
        let mut work = WorkCounters::default();
        let error = execute_joint_phi(&plan, op, h, input.as_input(), &mut caches, &mut work)
            .unwrap_err()
            .to_string();
        assert!(error.contains(PLAN_MISMATCH), "{error}");
        assert_eq!(work, WorkCounters::default(), "refused before any work");
        assert!(caches.coefficients.is_empty());
    }
    // A Chebyshev plan is not a Laguerre preflight.
    let mut caches = PolynomialCaches::default();
    let error = laguerre_preflight_terms(
        &plan,
        &op,
        &mut caches,
        &mut WorkCounters::default(),
        &mut RouteCost::default(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains(PLAN_MISMATCH), "{error}");
    // Plan errors are the action's errors.
    for (h, budget) in [(f64::NAN, 1e-9), (-1.0, 1e-9), (0.05, 0.0), (0.05, -1.0)] {
        let planned = plan_joint_phi(
            &op,
            h,
            input.as_input(),
            PolynomialBasis::Laguerre,
            budget,
            &caches,
            &mut RouteCost::default(),
        );
        let action = joint_phi_action(
            &op,
            h,
            input.as_input(),
            PolynomialBasis::Laguerre,
            budget,
            None,
            &mut WorkCounters::default(),
        );
        assert!(planned.is_err() && action.is_err());
    }
}

/// `ForceLaguerreFirst` on a case whose Laguerre total is rejected: the
/// failed Laguerre execution is charged, then Chebyshev executes.
#[test]
fn a_failed_laguerre_execution_is_charged() {
    let mut next = 9011;
    let (_, op, _) = rotated_fixture(10, 40.0, &mut next);
    let input = Input::distinct(10, 9500);
    let mut found = false;
    for total_budget in [1e-8, 1e-11, 1e-13, 1e-15, 1e-17] {
        let mut caches = PolynomialCaches::default();
        let routed = route_single(
            &op,
            0.1,
            &input,
            total_budget,
            RoutePolicy::ForceLaguerreFirst,
            &mut caches,
        );
        let first = &routed.executions()[0];
        assert_eq!(first.basis(), PolynomialBasis::Laguerre);
        assert!(first.executed());
        if first.is_admitted() {
            assert_eq!(routed.executions().len(), 1);
            assert_eq!(routed.choice(), &RouteChoice::Laguerre);
            assert_eq!(routed.cost().executed_backends, 1);
            continue;
        }
        found = true;
        assert!(first.degree().unwrap() <= LAGUERRE_ADJOINT_DEGREE_LIMIT);
        assert!(
            first.cost().vector_products > 0,
            "the failed execution is charged"
        );
        assert_eq!(first.cost().failed_executions, 1);
        assert_eq!(routed.executions().len(), 2);
        let second = &routed.executions()[1];
        assert_eq!(second.basis(), PolynomialBasis::Chebyshev);
        assert!(second.executed());
        assert_eq!(routed.cost().executed_backends, 2);
        assert_eq!(
            routed.cost().vector_products,
            first.cost().vector_products + second.cost().vector_products
        );
        // The default policy never preselects this Laguerre (more products).
        let mut caches = PolynomialCaches::default();
        let default = route_single(
            &op,
            0.1,
            &input,
            total_budget,
            RoutePolicy::ChebyshevFirstPreflight,
            &mut caches,
        );
        assert_eq!(
            default.preflight().preselected(),
            Some(PolynomialBasis::Chebyshev)
        );
        break;
    }
    assert!(found, "no rejected Laguerre total on the fixture");
}

#[test]
fn the_default_route_executes_one_backend_and_matches_chebyshev_alone() {
    let mut next = 9021;
    let (_, rotated, _) = rotated_fixture(6, 30.0, &mut next);
    let (_, diagonal) = diagonal_fixture(5, 30.0);
    for op in [&rotated, &diagonal] {
        let n = op.dimension();
        for input in [Input::distinct(n, 9600), Input::same(n, 9700)] {
            for h in [0.01, 0.1] {
                let mut single_caches = PolynomialCaches::default();
                let mut cheb_caches = PolynomialCaches::default();
                for state in ["cold", "warm"] {
                    let single = route_single(
                        op,
                        h,
                        &input,
                        1e-8,
                        RoutePolicy::ChebyshevFirstPreflight,
                        &mut single_caches,
                    );
                    let cheb = route_single(
                        op,
                        h,
                        &input,
                        1e-8,
                        RoutePolicy::ChebyshevOnly,
                        &mut cheb_caches,
                    );
                    let mut pp08_work = WorkCounters::default();
                    let pp08: RoutedPhi =
                        route_joint_phi(op, h, input.as_input(), 1e-8, &mut pp08_work).unwrap();
                    assert_eq!(pp08.choice(), &RouteChoice::Chebyshev);
                    assert_eq!(single.executions().len(), 1, "{state}");
                    assert_eq!(single.choice(), &RouteChoice::Chebyshev);
                    assert_eq!(cheb.choice(), &RouteChoice::Chebyshev);
                    assert_eq!(
                        single
                            .fused()
                            .unwrap()
                            .iter()
                            .map(|x| x.to_bits())
                            .collect::<Vec<_>>(),
                        cheb.fused()
                            .unwrap()
                            .iter()
                            .map(|x| x.to_bits())
                            .collect::<Vec<_>>()
                    );
                    assert_eq!(
                        single.bound().unwrap().to_bits(),
                        cheb.bound().unwrap().to_bits()
                    );
                    assert_eq!(single.cost().vector_products, cheb.cost().vector_products);
                    assert!(single.cost().vector_products < pp08_work.poly_vector_products);
                    assert!(
                        single
                            .preflight()
                            .abstain_reason()
                            .unwrap()
                            .starts_with(PREFLIGHT_PRODUCTS_NOT_FEWER)
                    );
                    assert!(!single.preflight().laguerre_table_used());
                    let laguerre_plan = single.plan(PolynomialBasis::Laguerre).unwrap();
                    assert!(laguerre_plan.cost().plan_tail_evaluations > 0);
                    if state == "warm" {
                        assert_eq!(single.cost().coefficient_setups, 0);
                        assert_eq!(single.cost().coefficient_cache_hits, 1);
                    }
                }
            }
        }
    }
}

#[test]
fn cache_keys_separate_what_the_entries_depend_on() {
    let (_, op) = diagonal_fixture(6, 10.0);
    let (_, other) = diagonal_fixture(6, 11.0);
    let input = Input::distinct(6, 9800);
    let mut caches = PolynomialCaches::default();
    let run = |op: &SymmetricNonpositiveOperator,
               h: f64,
               basis: PolynomialBasis,
               budget: f64,
               caches: &mut PolynomialCaches| {
        let plan = plan_joint_phi(
            op,
            h,
            input.as_input(),
            basis,
            budget,
            caches,
            &mut RouteCost::default(),
        )
        .unwrap();
        let mut cost = RouteCost::default();
        let report = execute_joint_phi_counted(
            &plan,
            op,
            h,
            input.as_input(),
            caches,
            &mut WorkCounters::default(),
            &mut cost,
        )
        .unwrap();
        (report.coefficient_cache_hit, cost)
    };
    let laguerre = PolynomialBasis::Laguerre;
    let (hit, cold) = run(&op, 0.05, laguerre, 1e-9, &mut caches);
    assert!(!hit && cold.coefficient_setups == 1 && cold.envelope_cache_misses == 5);
    assert!(cold.envelope_interval_operations > 0);
    let (hit, warm) = run(&op, 0.05, laguerre, 1e-9, &mut caches);
    assert!(hit && warm.coefficient_cache_hits == 1 && warm.coefficient_series_terms == 0);
    assert_eq!(warm.envelope_cache_hits, 5);
    assert_eq!(warm.envelope_interval_operations, 0);
    assert_eq!(warm.adjoint_sum_terms, cold.adjoint_sum_terms);
    // Another step, operator, basis or degree misses both caches.
    for (op, h, basis, budget) in [
        (&op, 0.06, laguerre, 1e-9),
        (&other, 0.05, laguerre, 1e-9),
        (&op, 0.05, laguerre, 1e-13),
    ] {
        let (hit, cost) = run(op, h, basis, budget, &mut caches);
        assert!(!hit && cost.coefficient_setups == 1, "{h} {budget}");
        assert!(cost.envelope_cache_misses > 0);
    }
    let (hit, cost) = run(&op, 0.05, PolynomialBasis::Chebyshev, 1e-9, &mut caches);
    assert!(!hit && cost.coefficient_setups == 1 && cost.envelope_interval_operations == 0);
    // A different input with the same operator, step and degree reuses the
    // table (coefficients do not depend on the vectors).
    let tables = caches.coefficients.len();
    let (hit, _) = run(&op, 0.05, PolynomialBasis::Chebyshev, 1e-9, &mut caches);
    assert!(hit);
    assert_eq!(caches.coefficients.len(), tables);
}

#[test]
fn boundary_refusals() {
    let mut next = 9031;
    let (a, verified, _) = rotated_fixture(6, 30.0, &mut next);
    let input = Input::distinct(6, 9900);
    // Invalid budgets, steps and inputs: errors before any work.
    for (h, budget) in [
        (0.05, f64::NAN),
        (0.05, -1.0),
        (0.05, f64::INFINITY),
        (f64::NAN, 1e-8),
        (-0.1, 1e-8),
    ] {
        let mut caches = PolynomialCaches::default();
        let mut work = WorkCounters::default();
        let error = route_joint_phi_single(
            &verified,
            h,
            input.as_input(),
            budget,
            RoutePolicy::default(),
            &mut caches,
            &mut work,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(ROUTER_INPUT_UNSUPPORTED), "{error}");
        assert_eq!(work, WorkCounters::default());
    }
    let short = Input::Same(vec![1.0; 5]);
    assert!(
        route_joint_phi_single(
            &verified,
            0.05,
            short.as_input(),
            1e-8,
            RoutePolicy::default(),
            &mut PolynomialCaches::default(),
            &mut WorkCounters::default(),
        )
        .is_err()
    );

    // A declared enclosure: nothing executes, at h > 0 and at h = 0.
    let declared = declared_operator(&a, "py03-contract-declared");
    for h in [0.05, 0.0] {
        for policy in [
            RoutePolicy::ChebyshevFirstPreflight,
            RoutePolicy::ForceLaguerreFirst,
            RoutePolicy::ChebyshevOnly,
        ] {
            let mut caches = PolynomialCaches::default();
            let routed = route_single(&declared, h, &input, 1e-8, policy, &mut caches);
            assert!(routed.is_fallback());
            assert!(routed.executions().is_empty());
            assert_eq!(routed.cost().vector_products, 0);
            assert!(
                routed
                    .preflight()
                    .abstain_reason()
                    .unwrap()
                    .starts_with(ROUTER_ENCLOSURE_NOT_VERIFIED)
            );
        }
    }

    // The timing execution is never admitted.
    for basis in BASES {
        let mut caches = PolynomialCaches::default();
        let plan = plan_joint_phi(
            &verified,
            0.05,
            input.as_input(),
            basis,
            2.5e-9,
            &caches,
            &mut RouteCost::default(),
        )
        .unwrap();
        let report = execute_joint_phi_unbounded(
            &plan,
            &verified,
            0.05,
            input.as_input(),
            &mut caches,
            &mut WorkCounters::default(),
        )
        .unwrap();
        assert_eq!(report.execution, EXECUTION_UNBOUNDED_TIMING);
        assert!(!admitted(&route_admission(&report, 1e300)));
    }

    // A Laguerre degree above the adjoint limit is never admitted.
    let (_, wide) = diagonal_fixture(5, 64.0);
    let wide_input = Input::distinct(5, 9910);
    let mut above = None;
    for h in [0.5, 1.0, 2.0, 4.0] {
        let plan = plan_joint_phi(
            &wide,
            h,
            wide_input.as_input(),
            PolynomialBasis::Laguerre,
            2.5e-11,
            &PolynomialCaches::default(),
            &mut RouteCost::default(),
        )
        .unwrap();
        if plan.degree() > LAGUERRE_ADJOINT_DEGREE_LIMIT {
            above = Some(h);
            break;
        }
    }
    let h = above.expect("a Laguerre degree above the limit");
    let mut caches = PolynomialCaches::default();
    let forced = route_single(
        &wide,
        h,
        &wide_input,
        1e-10,
        RoutePolicy::ForceLaguerreFirst,
        &mut caches,
    );
    let first = &forced.executions()[0];
    assert_eq!(first.basis(), PolynomialBasis::Laguerre);
    assert!(first.degree().unwrap() > LAGUERRE_ADJOINT_DEGREE_LIMIT && !first.is_admitted());
    assert_ne!(forced.basis(), Some(PolynomialBasis::Laguerre));
    let mut caches = PolynomialCaches::default();
    let default = route_single(
        &wide,
        h,
        &wide_input,
        1e-10,
        RoutePolicy::default(),
        &mut caches,
    );
    assert_eq!(
        default.preflight().preselected(),
        Some(PolynomialBasis::Chebyshev)
    );

    // Budget 0: no plan, no execution of a recurrence, a fallback.
    let mut caches = PolynomialCaches::default();
    let zero = route_single(
        &verified,
        0.05,
        &input,
        0.0,
        RoutePolicy::default(),
        &mut caches,
    );
    assert!(zero.is_fallback());
    assert!(zero.plans().iter().all(|p| p.error().is_some()));
    assert_eq!(zero.cost().vector_products, 0);
    assert_eq!(zero.cost().executed_backends, 0);
    assert_eq!(zero.cost().failed_executions, 2);

    // The scalar branch: Chebyshev (a tie), no products, admitted.
    let scalar = SymmetricNonpositiveOperator::gershgorin(minus_three_identity(5)).unwrap();
    let scalar_input = Input::distinct(5, 9920);
    let mut caches = PolynomialCaches::default();
    let routed = route_single(
        &scalar,
        0.12,
        &scalar_input,
        1e-10,
        RoutePolicy::default(),
        &mut caches,
    );
    assert_eq!(routed.choice(), &RouteChoice::Chebyshev);
    assert_eq!(routed.executions().len(), 1);
    assert_eq!(routed.cost().vector_products, 0);
    assert!(routed.cost().coefficient_series_terms > 0);

    // Estimate-only totals are never admitted: the Laguerre total without
    // an adjoint (degree above the limit) and every declared report.
    let report = joint_phi_action(
        &declared,
        0.05,
        input.as_input(),
        PolynomialBasis::Chebyshev,
        2.5e-9,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(matches!(
        report.total_error,
        TotalErrorStatus::EstimateOnly { .. }
    ));
    assert!(!admitted(&route_admission(&report, 1e300)));
}

/// The preflight's vector-independent terms are components of the
/// Laguerre total, so they never exceed it.
#[test]
fn preflight_terms_are_part_of_the_laguerre_total() {
    let mut next = 9041;
    let (_, op, _) = rotated_fixture(6, 20.0, &mut next);
    for input in [Input::distinct(6, 9950), Input::same(6, 9960)] {
        for h in [0.02, 0.2] {
            let mut caches = PolynomialCaches::default();
            let mut cost = RouteCost::default();
            let mut work = WorkCounters::default();
            let plan: ActionPlan = plan_joint_phi(
                &op,
                h,
                input.as_input(),
                PolynomialBasis::Laguerre,
                2.5e-9,
                &caches,
                &mut cost,
            )
            .unwrap();
            let terms =
                laguerre_preflight_terms(&plan, &op, &mut caches, &mut work, &mut cost).unwrap();
            assert_eq!(cost.coefficient_setups, 1);
            assert_eq!(work.poly_coefficient_setups, 1);
            assert!(cost.coefficient_series_terms > 0);
            let mut exec_work = WorkCounters::default();
            let report =
                execute_joint_phi(&plan, &op, h, input.as_input(), &mut caches, &mut exec_work)
                    .unwrap();
            assert!(
                report.coefficient_cache_hit,
                "the execution reads the preflight's table"
            );
            assert_eq!(exec_work.poly_coefficient_setups, 0);
            let parts: f64 = report
                .column_errors
                .iter()
                .map(|c| c.truncation + c.coefficient)
                .sum();
            assert!(terms >= parts * (1.0 - 1e-12) && terms <= parts * (1.0 + 1e-12));
            assert!(terms <= report.laguerre_adjoint_total.unwrap());
        }
    }
}

// ---------------------------------------------------------------------------
// The recorded export (registered command 2)
// ---------------------------------------------------------------------------

fn git(args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8(out.stdout).unwrap()
}

fn cost_json(c: &RouteCost) -> Value {
    serde_json::to_value(c).unwrap()
}

/// Every `WorkCounters` field (the serialization skips some zero fields;
/// the export writes them all, so every counter map has one key set).
const WORK_FIELDS: [&str; 61] = [
    "accepted_steps",
    "block_linear_iterations",
    "block_linear_solves",
    "block_matvecs",
    "block_preconditioner_apps",
    "diagnostic_matvecs",
    "direct_factorizations",
    "direct_solve_calls",
    "fallback_steps",
    "fast_accepts",
    "fast_attempts",
    "forced_stage_solves",
    "ft_calls",
    "harmonic_ritz_solves",
    "jacobian_builds",
    "jacobian_matvecs",
    "jvp_calls",
    "jvp_vectors",
    "linear_iterations",
    "linear_matvec_vectors",
    "linear_matvecs",
    "linear_solve_failures",
    "linear_solves",
    "local_error_failures",
    "mass_matvecs",
    "merged_unknown_vector_calls",
    "nonfinite_step_failures",
    "nonlinear_failures",
    "nonlinear_iterations",
    "nonlinear_jacobian_evaluations",
    "nonlinear_residual_evaluations",
    "nonlinear_solve_failures",
    "nonlinear_solves",
    "orthogonalization_inner_products",
    "orthogonalization_vector_updates",
    "phi_actions",
    "phi_dense_oracle_calls",
    "phi_krylov_vectors",
    "phi_projected_exponentials",
    "phi_restarts",
    "phi_weight_underflows",
    "poly_block_allocations",
    "poly_block_products",
    "poly_coefficient_reuses",
    "poly_coefficient_setups",
    "poly_fallbacks",
    "poly_vector_products",
    "preconditioner_apps",
    "preconditioner_vectors",
    "recycle_cross_operator_refreshes",
    "recycle_dropped_vectors",
    "recycle_projection_calls",
    "recycle_refresh_matvecs",
    "recycle_same_operator_uses",
    "recycle_update_refreshes",
    "recycle_updates",
    "recycle_vectors_selected",
    "rejected_steps",
    "rhs_batch_calls",
    "rhs_calls",
    "rhs_evaluations",
];

fn work_json(w: &WorkCounters) -> Value {
    let mut value = serde_json::to_value(w).unwrap();
    let map = value.as_object_mut().unwrap();
    for field in WORK_FIELDS {
        map.entry(field).or_insert(json!(0));
    }
    assert_eq!(
        map.len(),
        WORK_FIELDS.len(),
        "a WorkCounters field is not registered"
    );
    value
}

fn admission_json(a: &TotalErrorAdmission) -> Value {
    match a {
        TotalErrorAdmission::Admitted { bound, .. } => {
            json!({"admitted": true, "bound": hex(*bound), "reason": null})
        }
        TotalErrorAdmission::Rejected { reason } => {
            json!({"admitted": false, "bound": null, "reason": reason})
        }
    }
}

/// The report's own candidate total: the certified total for Chebyshev, the
/// adjoint total for Laguerre (the scalar branch: the certified total).
fn candidate_total(report: &JointPhiReport) -> Option<f64> {
    let certified = match report.total_error {
        TotalErrorStatus::Certified { bound } => Some(bound),
        TotalErrorStatus::EstimateOnly { .. } => None,
    };
    match report.basis {
        PolynomialBasis::Laguerre if report.branch != "scalar" => report.laguerre_adjoint_total,
        _ => certified,
    }
}

fn report_json(report: &JointPhiReport) -> Value {
    json!({
        "basis": format!("{:?}", report.basis),
        "branch": report.branch,
        "execution": report.execution,
        "evidence_verified": matches!(report.evidence, EnclosureEvidence::Gershgorin),
        "degree": report.degree,
        "fused": hex_vec(&report.fused),
        "total_error_is_estimate": matches!(report.total_error, TotalErrorStatus::EstimateOnly { .. }),
        "candidate_total": candidate_total(report).map(hex),
        "coefficient_cache_hit": report.coefficient_cache_hit,
    })
}

/// `joint_phi_action` with a cache in the same state for this key: a fresh
/// cache, primed by one discarded call when the execution read the table
/// from its cache.
fn replay(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    report: &JointPhiReport,
    unbounded: bool,
) -> (JointPhiReport, WorkCounters) {
    let mut cache = rodas5p_core::polynomial_action::CoefficientCache::default();
    let action = if unbounded {
        joint_phi_action_unbounded
    } else {
        joint_phi_action
    };
    if report.coefficient_cache_hit {
        action(
            op,
            h,
            input.as_input(),
            report.basis,
            report.truncation_budget,
            Some(&mut cache),
            &mut WorkCounters::default(),
        )
        .unwrap();
    }
    let mut work = WorkCounters::default();
    let again = action(
        op,
        h,
        input.as_input(),
        report.basis,
        report.truncation_budget,
        Some(&mut cache),
        &mut work,
    )
    .unwrap();
    (again, work)
}

fn parity_json(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    report: &JointPhiReport,
    work: &WorkCounters,
    unbounded: bool,
) -> Value {
    let (again, again_work) = replay(op, h, input, report, unbounded);
    json!({
        "report_bitwise_equal": same_report(report, &again),
        "counters_equal": *work == again_work,
        "replay_fused": hex_vec(&again.fused),
        "replay_candidate_total": candidate_total(&again).map(hex),
    })
}

fn caches_json(caches: &PolynomialCaches) -> Value {
    json!({"coefficient_tables": caches.coefficients.len(), "envelopes": caches.envelopes.len()})
}

#[allow(clippy::too_many_arguments)]
fn single_record(
    arm: &str,
    state: &str,
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    total_budget: f64,
    policy: RoutePolicy,
    caches: &mut PolynomialCaches,
) -> Value {
    let before = caches_json(caches);
    let mut work = WorkCounters::default();
    let routed = route_joint_phi_single(
        op,
        h,
        input.as_input(),
        total_budget,
        policy,
        caches,
        &mut work,
    )
    .unwrap();
    let plans: Vec<Value> = routed
        .plans()
        .iter()
        .map(|p| {
            json!({
                "basis": format!("{:?}", p.basis()),
                "plan": p.record().map(|r| json!({
                    "branch": serde_json::to_value(r.branch).unwrap(),
                    "degree": r.degree,
                    "width": r.width,
                    "laguerre_scale": r.laguerre_scale.map(hex),
                    "beta": r.beta.map(hex),
                    "laguerre_extent": r.laguerre_extent.map(hex),
                    "predicted_vector_products": r.predicted_vector_products,
                    "predicted_block_products": r.predicted_block_products,
                    "tail_evaluations": r.tail_evaluations,
                    "normalization_shift": r.normalization_shift,
                    "coefficients_cached": r.coefficients_cached,
                })),
                "error": p.error(),
                "cost": cost_json(&p.cost()),
            })
        })
        .collect();
    let pf = routed.preflight();
    let preflight = json!({
        "preselected": pf.preselected().map(|b| format!("{b:?}")),
        "abstain_reason": pf.abstain_reason(),
        "laguerre_table_used": pf.laguerre_table_used(),
        "vector_independent_terms": pf.vector_independent_terms().map(hex),
        "cost": cost_json(&pf.cost()),
        "work": work_json(&pf.work()),
    });
    let executions: Vec<Value> = routed
        .executions()
        .iter()
        .map(|e| {
            json!({
                "basis": format!("{:?}", e.basis()),
                "executed": e.executed(),
                "degree": e.degree(),
                "admission": admission_json(e.admission()),
                "cost": cost_json(&e.cost()),
                "work": work_json(&e.work()),
                "report": e.report().map(report_json),
                "parity": e.report().map(|r| parity_json(op, h, input, r, &e.work(), false)),
            })
        })
        .collect();
    json!({
        "arm": arm,
        "cache_state": state,
        "policy": serde_json::to_value(policy).unwrap(),
        "choice": choice_name(routed.choice()),
        "fallback_reasons": fallback_json(routed.choice()),
        "fused": routed.fused().map(hex_vec),
        "bound": routed.bound().map(hex),
        "plans": plans,
        "preflight": preflight,
        "executions": executions,
        "cost": cost_json(&routed.cost()),
        "caller_counters": work_json(&work),
        "caches_before": before,
        "caches_after": caches_json(caches),
    })
}

fn choice_name(choice: &RouteChoice) -> &'static str {
    match choice {
        RouteChoice::Chebyshev => "Chebyshev",
        RouteChoice::Laguerre => "Laguerre",
        RouteChoice::Fallback { .. } => "Fallback",
    }
}

fn fallback_json(choice: &RouteChoice) -> Value {
    match choice {
        RouteChoice::Fallback {
            chebyshev_reason,
            laguerre_reason,
        } => json!({"chebyshev": chebyshev_reason, "laguerre": laguerre_reason}),
        _ => Value::Null,
    }
}

fn pp08_record(
    state: &str,
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    total_budget: f64,
) -> (Value, RoutedPhi) {
    let mut work = WorkCounters::default();
    let routed = route_joint_phi(op, h, input.as_input(), total_budget, &mut work).unwrap();
    let attempts: Vec<Value> = routed
        .attempts()
        .iter()
        .map(|a| {
            json!({
                "basis": format!("{:?}", a.basis()),
                "degree": a.degree(),
                "admission": admission_json(a.admission()),
                "vector_products": a.vector_products(),
                "work": work_json(&a.work()),
                "report": a.report().map(report_json),
            })
        })
        .collect();
    let record = json!({
        "arm": "pp08",
        "cache_state": state,
        "choice": choice_name(routed.choice()),
        "fallback_reasons": fallback_json(routed.choice()),
        "fused": routed.fused().map(hex_vec),
        "bound": routed.bound().map(hex),
        "attempts": attempts,
        "caller_counters": work_json(&work),
    });
    (record, routed)
}

/// The timing executions of one case: each basis planned and executed in
/// plain binary64, its admission and its parity with the timing action.
fn timing_records(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    total_budget: f64,
) -> Vec<Value> {
    BASES
        .iter()
        .map(|&basis| {
            let mut caches = PolynomialCaches::default();
            let plan = plan_joint_phi(
                op,
                h,
                input.as_input(),
                basis,
                total_budget * 0.25,
                &caches,
                &mut RouteCost::default(),
            )
            .unwrap();
            let mut work = WorkCounters::default();
            let report = execute_joint_phi_unbounded(
                &plan,
                op,
                h,
                input.as_input(),
                &mut caches,
                &mut work,
            )
            .unwrap();
            json!({
                "basis": format!("{basis:?}"),
                "execution": report.execution,
                "admission_at_total_budget": admission_json(&route_admission(&report, total_budget)),
                "admission_at_1e300": admission_json(&route_admission(&report, 1e300)),
                "report": report_json(&report),
                "parity": parity_json(op, h, input, &report, &work, true),
            })
        })
        .collect()
}

struct Case<'a> {
    label: String,
    group: &'static str,
    family: &'static str,
    n: usize,
    rho: f64,
    seed: Option<u64>,
    a: &'a DenseMatrix,
    op: &'a SymmetricNonpositiveOperator,
    h: f64,
    input: Input,
    input_seed: u64,
    total_budget: f64,
}

/// Every arm in both cache states (and the timing executions) of one case.
fn run_case(case: &Case<'_>, force: bool, timing: bool) -> (Value, [RoutedPhi; 2]) {
    let mut records = Vec::new();
    let mut arms: Vec<(&str, RoutePolicy)> = vec![
        ("single", RoutePolicy::ChebyshevFirstPreflight),
        ("cheb", RoutePolicy::ChebyshevOnly),
    ];
    if force {
        arms.push(("force_laguerre_first", RoutePolicy::ForceLaguerreFirst));
    }
    for (arm, policy) in arms {
        let mut caches = PolynomialCaches::default();
        for state in ["cold", "warm"] {
            records.push(single_record(
                arm,
                state,
                case.op,
                case.h,
                &case.input,
                case.total_budget,
                policy,
                &mut caches,
            ));
        }
    }
    let (cold, pp08_cold) = pp08_record("cold", case.op, case.h, &case.input, case.total_budget);
    let (warm, pp08_warm) = pp08_record("warm", case.op, case.h, &case.input, case.total_budget);
    records.push(cold);
    records.push(warm);
    let n = case.n;
    let enclosure = case.op.enclosure();
    let value = json!({
        "label": case.label,
        "group": case.group,
        "family": case.family,
        "n": n,
        "rho_nominal": case.rho,
        "seed": case.seed,
        "input_seed": case.input_seed,
        "a": (0..n).map(|i| (0..n).map(|j| hex(case.a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "h": hex(case.h),
        "input": case.input.name(),
        "w": case.input.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
        "enclosure": {
            "lambda": hex(enclosure.lambda),
            "rho": hex(enclosure.rho),
            "verified": matches!(enclosure.evidence, EnclosureEvidence::Gershgorin),
        },
        "total_budget": hex(case.total_budget),
        "records": records,
        "timing": if timing { Value::Array(timing_records(case.op, case.h, &case.input, case.total_budget)) } else { Value::Null },
    });
    println!("{} done", case.label);
    (value, [pp08_cold, pp08_warm])
}

/// The registered grid's step sizes, total budgets and nominal radii.
const GRID_N: [usize; 2] = [8, 24];
const GRID_RHO: [f64; 2] = [4.0, 64.0];
const GRID_H: [f64; 3] = [0.005, 0.05, 0.12];
const GRID_BUDGET: [f64; 2] = [1e-6, 1e-10];
const FIRST_ROTATED_SEED: u64 = 7001;
/// The force-Laguerre-first fallback budgets (used only when no grid case of
/// a family has a rejected Laguerre total of degree <= 128).
const FORCE_FALLBACK_BUDGETS: [f64; 3] = [1e-13, 1e-15, 1e-17];
const DEGREE_ABOVE_LIMIT_STEPS: [f64; 4] = [0.5, 1.0, 2.0, 4.0];

/// A grid case as (fixture index, input seed, h, input kind, total budget).
type ForceSource = (usize, u64, f64, &'static str, f64);

fn input_seed(index: u64) -> u64 {
    100_000 + 1000 * index
}

/// Whether PP08's Laguerre attempt executed (degree <= 128) and its total
/// was rejected.
fn laguerre_total_rejected(routed: &RoutedPhi) -> bool {
    let attempt = routed.attempt(PolynomialBasis::Laguerre);
    attempt.report().is_some()
        && attempt
            .degree()
            .is_some_and(|d| d <= LAGUERRE_ADJOINT_DEGREE_LIMIT)
        && !attempt.is_admitted()
}

#[test]
#[ignore = "recorded run of research node PY03; writes PY03_CASES when set"]
fn export_cases() {
    // Without PY03_CASES (the workspace's ignored-test run) nothing runs.
    let Ok(output) = std::env::var("PY03_CASES") else {
        println!("PY03_CASES not set: cases not written");
        return;
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out_path = root.join(&output);
    assert!(!out_path.exists(), "immutable output exists: {output}");
    let source_commit = git(&["rev-parse", "HEAD"]).trim().to_string();
    let tree_status = git(&["status", "--porcelain"]);

    // Fixtures: diagonal and rotated, (n, rho) in grid order.
    let mut next_seed = FIRST_ROTATED_SEED;
    let mut fixtures = Vec::new();
    for family in ["diagonal", "rotated"] {
        for n in GRID_N {
            for rho in GRID_RHO {
                let (a, op, seed) = if family == "diagonal" {
                    let (a, op) = diagonal_fixture(n, rho);
                    (a, op, None)
                } else {
                    let (a, op, seed) = rotated_fixture(n, rho, &mut next_seed);
                    (a, op, Some(seed))
                };
                fixtures.push((family, n, rho, a, op, seed));
            }
        }
    }

    let mut cases = Vec::new();
    // Per family, the first grid case whose Laguerre total is rejected.
    let mut force_source: [Option<ForceSource>; 2] = [None, None];
    let mut index = 0_u64;
    for (fixture_index, (family, n, rho, a, op, seed)) in fixtures.iter().enumerate() {
        for h in GRID_H {
            for kind in ["distinct", "same"] {
                let s = input_seed(index);
                index += 1;
                let input = if kind == "distinct" {
                    Input::distinct(*n, s)
                } else {
                    Input::same(*n, s)
                };
                for budget in GRID_BUDGET {
                    let case = Case {
                        label: format!("{family}-n{n}-rho{rho}-h{h}-{kind}-b{budget:e}"),
                        group: "grid",
                        family,
                        n: *n,
                        rho: *rho,
                        seed: *seed,
                        a,
                        op,
                        h,
                        input: input.clone(),
                        input_seed: s,
                        total_budget: budget,
                    };
                    let (value, [pp08_cold, _]) = run_case(&case, false, false);
                    cases.push(value);
                    let slot = usize::from(*family == "rotated");
                    if force_source[slot].is_none() && laguerre_total_rejected(&pp08_cold) {
                        force_source[slot] = Some((fixture_index, s, h, kind, budget));
                    }
                }
            }
        }
    }

    // Boundary cases.
    let boundary_seed = |k: u64| 200_000 + 1000 * k;
    let fixture = |family: &str, n: usize, rho: f64| {
        fixtures
            .iter()
            .position(|f| f.0 == family && f.1 == n && f.2 == rho)
            .unwrap()
    };
    // Declared enclosure (rotated n = 8, rho = 64).
    let rotated = &fixtures[fixture("rotated", 8, 64.0)];
    let declared = declared_operator(&rotated.3, "py03-declared");
    let s = boundary_seed(1);
    cases.push(
        run_case(
            &Case {
                label: "declared-enclosure".into(),
                group: "declared",
                family: "rotated",
                n: 8,
                rho: 64.0,
                seed: rotated.5,
                a: &rotated.3,
                op: &declared,
                h: 0.05,
                input: Input::distinct(8, s),
                input_seed: s,
                total_budget: 1e-10,
            },
            false,
            false,
        )
        .0,
    );
    // The unbounded-timing execution (diagonal n = 8, rho = 64).
    let diagonal = &fixtures[fixture("diagonal", 8, 64.0)];
    let s = boundary_seed(2);
    cases.push(
        run_case(
            &Case {
                label: "unbounded-timing".into(),
                group: "timing",
                family: "diagonal",
                n: 8,
                rho: 64.0,
                seed: None,
                a: &diagonal.3,
                op: &diagonal.4,
                h: 0.05,
                input: Input::distinct(8, s),
                input_seed: s,
                total_budget: 1e-10,
            },
            false,
            true,
        )
        .0,
    );
    // A Laguerre degree above 128: the first registered step whose Laguerre
    // plan exceeds the limit (plan only).
    let s = boundary_seed(3);
    let above_input = Input::distinct(8, s);
    let above_h = DEGREE_ABOVE_LIMIT_STEPS
        .into_iter()
        .find(|&h| {
            plan_joint_phi(
                &diagonal.4,
                h,
                above_input.as_input(),
                PolynomialBasis::Laguerre,
                1e-10 * 0.25,
                &PolynomialCaches::default(),
                &mut RouteCost::default(),
            )
            .is_ok_and(|p| p.degree() > LAGUERRE_ADJOINT_DEGREE_LIMIT)
        })
        .expect("a registered step with a Laguerre degree above 128");
    cases.push(
        run_case(
            &Case {
                label: "laguerre-degree-above-limit".into(),
                group: "degree-above-limit",
                family: "diagonal",
                n: 8,
                rho: 64.0,
                seed: None,
                a: &diagonal.3,
                op: &diagonal.4,
                h: above_h,
                input: above_input,
                input_seed: s,
                total_budget: 1e-10,
            },
            true,
            false,
        )
        .0,
    );
    // Total budget 0 (rotated n = 8, rho = 4).
    let rotated_small = &fixtures[fixture("rotated", 8, 4.0)];
    let s = boundary_seed(4);
    cases.push(
        run_case(
            &Case {
                label: "total-budget-zero".into(),
                group: "budget-zero",
                family: "rotated",
                n: 8,
                rho: 4.0,
                seed: rotated_small.5,
                a: &rotated_small.3,
                op: &rotated_small.4,
                h: 0.05,
                input: Input::distinct(8, s),
                input_seed: s,
                total_budget: 0.0,
            },
            false,
            false,
        )
        .0,
    );
    // The scalar branch -3 I.
    let scalar_a = minus_three_identity(8);
    let scalar_op = SymmetricNonpositiveOperator::gershgorin(scalar_a.clone()).unwrap();
    let s = boundary_seed(5);
    cases.push(
        run_case(
            &Case {
                label: "scalar-minus3I".into(),
                group: "scalar",
                family: "scalar",
                n: 8,
                rho: 3.0,
                seed: None,
                a: &scalar_a,
                op: &scalar_op,
                h: 0.12,
                input: Input::distinct(8, s),
                input_seed: s,
                total_budget: 1e-10,
            },
            false,
            false,
        )
        .0,
    );
    // One ForceLaguerreFirst case per family with a rejected Laguerre total.
    let mut selection = Vec::new();
    for (slot, family) in ["diagonal", "rotated"].into_iter().enumerate() {
        let (fixture_index, s, h, kind, budget, rule) = match force_source[slot] {
            Some((fixture_index, s, h, kind, budget)) => {
                (fixture_index, s, h, kind, budget, "first-grid-case")
            }
            None => {
                let fixture_index = fixture(family, 8, 64.0);
                let s = boundary_seed(6 + slot as u64);
                let input = Input::distinct(8, s);
                let f = &fixtures[fixture_index];
                let budget = FORCE_FALLBACK_BUDGETS
                    .into_iter()
                    .find(|&b| {
                        let mut work = WorkCounters::default();
                        let routed =
                            route_joint_phi(&f.4, 0.12, input.as_input(), b, &mut work).unwrap();
                        laguerre_total_rejected(&routed)
                    })
                    .expect("a rejected Laguerre total among the fallback budgets");
                (
                    fixture_index,
                    s,
                    0.12,
                    "distinct",
                    budget,
                    "fallback-budget",
                )
            }
        };
        let (_, n, rho, a, op, seed) = &fixtures[fixture_index];
        let input = if kind == "distinct" {
            Input::distinct(*n, s)
        } else {
            Input::same(*n, s)
        };
        let label = format!("force-{family}-n{n}-rho{rho}-h{h}-{kind}-b{budget:e}");
        selection.push(json!({"family": family, "label": label, "rule": rule}));
        cases.push(
            run_case(
                &Case {
                    label,
                    group: "force-laguerre-first",
                    family,
                    n: *n,
                    rho: *rho,
                    seed: *seed,
                    a,
                    op,
                    h,
                    input,
                    input_seed: s,
                    total_budget: budget,
                },
                true,
                false,
            )
            .0,
        );
    }

    let document = json!({
        "schema": "vigilode-py03-single-backend-router-cases-v1",
        "node": "research/py03_single_backend_router_20261011",
        "source_commit": source_commit,
        "tree_status": tree_status,
        "constants": {
            "truncation_fraction": 0.25,
            "degree_limit": LAGUERRE_ADJOINT_DEGREE_LIMIT,
            "scales": hex_vec(&SCALES),
            "first_rotated_seed": FIRST_ROTATED_SEED,
            "lambda_min_fraction": LAMBDA_MIN_FRACTION,
            "max_givens_angle": MAX_ANGLE,
        },
        "force_selection": selection,
        "cases": cases,
    });
    std::fs::write(&out_path, serde_json::to_string(&document).unwrap() + "\n").unwrap();
    println!("wrote {output}");
}

/// Not a registered command: the record format of `export_cases` on fresh
/// fixtures (n = 5, other seeds, no registered case), written to
/// `PY03_FIXTURE_CASES` to check the checker's schema before the recorded
/// run.
#[test]
#[ignore = "format check on fresh fixtures; writes PY03_FIXTURE_CASES when set"]
fn export_fixture_format() {
    let Ok(output) = std::env::var("PY03_FIXTURE_CASES") else {
        println!("PY03_FIXTURE_CASES not set: nothing written");
        return;
    };
    let (a, op) = diagonal_fixture(5, 20.0);
    let mut next = 9601;
    let (ra, rop, seed) = rotated_fixture(5, 20.0, &mut next);
    let declared = declared_operator(&ra, "py03-fixture-declared");
    let scalar_a = minus_three_identity(5);
    let scalar_op = SymmetricNonpositiveOperator::gershgorin(scalar_a.clone()).unwrap();
    let mk = |label: &str,
              group: &'static str,
              family: &'static str,
              a: &DenseMatrix,
              op: &SymmetricNonpositiveOperator,
              seed: Option<u64>,
              h: f64,
              input_seed: u64,
              budget: f64,
              force: bool,
              timing: bool| {
        let input = Input::distinct(5, input_seed);
        run_case(
            &Case {
                label: label.into(),
                group,
                family,
                n: 5,
                rho: 20.0,
                seed,
                a,
                op,
                h,
                input,
                input_seed,
                total_budget: budget,
            },
            force,
            timing,
        )
        .0
    };
    let cases = vec![
        mk(
            "fixture-grid",
            "grid",
            "diagonal",
            &a,
            &op,
            None,
            0.05,
            9700,
            1e-8,
            false,
            false,
        ),
        mk(
            "fixture-declared",
            "declared",
            "rotated",
            &ra,
            &declared,
            Some(seed),
            0.05,
            9701,
            1e-8,
            false,
            false,
        ),
        mk(
            "fixture-timing",
            "timing",
            "diagonal",
            &a,
            &op,
            None,
            0.05,
            9702,
            1e-8,
            false,
            true,
        ),
        mk(
            "fixture-above",
            "degree-above-limit",
            "diagonal",
            &a,
            &op,
            None,
            4.0,
            9703,
            1e-10,
            true,
            false,
        ),
        mk(
            "fixture-zero",
            "budget-zero",
            "rotated",
            &ra,
            &rop,
            Some(seed),
            0.05,
            9704,
            0.0,
            false,
            false,
        ),
        mk(
            "fixture-scalar",
            "scalar",
            "scalar",
            &scalar_a,
            &scalar_op,
            None,
            0.12,
            9705,
            1e-10,
            false,
            false,
        ),
        mk(
            "fixture-force",
            "force-laguerre-first",
            "rotated",
            &ra,
            &rop,
            Some(seed),
            0.1,
            9706,
            1e-15,
            true,
            false,
        ),
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::write(
        root.join(&output),
        serde_json::to_string(&json!({"cases": cases})).unwrap() + "\n",
    )
    .unwrap();
    println!("wrote {output}");
}
