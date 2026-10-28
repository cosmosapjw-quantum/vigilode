//! Contracts of the opt-in Chebyshev/Laguerre total-budget router (RVJ DAG
//! node PP08, `research/pp08_laguerre_router_20261004`): input errors,
//! guards that must never admit, the choice rule and the work accounting.

use rodas5p_core::{
    CoreError, DenseMatrix, WorkCounters,
    laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT,
    polynomial_action::{
        EXECUTION_UNBOUNDED_TIMING, EnclosureEvidence, JointPhiInput, PolynomialBasis,
        ROUTER_INPUT_UNSUPPORTED, RouteChoice, RoutedPhi, SymmetricNonpositiveOperator,
        TotalErrorAdmission, TotalErrorStatus, joint_phi_action, joint_phi_action_unbounded,
        route_admission, route_joint_phi,
    },
};

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

/// Symmetric, diagonally dominant, Gershgorin discs inside `[-1.2 rho, 0)`.
fn matrix(n: usize, rho: f64, seed: u64) -> DenseMatrix {
    let d: Vec<f64> = splitmix(seed, n)
        .iter()
        .map(|u| rho * (0.65 + 0.35 * u))
        .collect();
    let raw = splitmix(seed + 1000, n * n);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -d[i];
        for j in 0..i {
            let v = 0.2 * d[i].min(d[j]) / n as f64 * raw[i * n + j];
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

fn verified(n: usize, rho: f64, seed: u64) -> SymmetricNonpositiveOperator {
    let op = SymmetricNonpositiveOperator::gershgorin(matrix(n, rho, seed)).unwrap();
    assert_eq!(op.enclosure().evidence, EnclosureEvidence::Gershgorin);
    op
}

/// The largest diagonal magnitude contains the diagonal but not the discs,
/// so the enclosure stays declared.
fn declared(n: usize, rho: f64, seed: u64) -> SymmetricNonpositiveOperator {
    let a = matrix(n, rho, seed);
    let diagonal_max = (0..n).map(|i| a[(i, i)].abs()).fold(0.0, f64::max);
    let op = SymmetricNonpositiveOperator::new(a, 0.0, diagonal_max, "pp08-declared").unwrap();
    assert!(matches!(
        op.enclosure().evidence,
        EnclosureEvidence::Declared { .. }
    ));
    op
}

fn distinct(n: usize, seed: u64) -> [Vec<f64>; 5] {
    std::array::from_fn(|k| splitmix(seed + 77 * k as u64, n))
}

fn admitted(a: &TotalErrorAdmission) -> bool {
    matches!(a, TotalErrorAdmission::Admitted { .. })
}

fn route(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    w: &[Vec<f64>; 5],
    total_budget: f64,
) -> RoutedPhi {
    route_joint_phi(
        op,
        h,
        JointPhiInput::Distinct(w),
        total_budget,
        &mut WorkCounters::default(),
    )
    .unwrap()
}

/// Every routed outcome obeys the preregistered rule.
fn assert_rule(routed: &RoutedPhi) {
    let [cheb, lag] = routed.attempts() else {
        panic!("two attempts")
    };
    assert_eq!(cheb.basis(), PolynomialBasis::Chebyshev);
    assert_eq!(lag.basis(), PolynomialBasis::Laguerre);
    assert_eq!(routed.truncation_budget(), routed.total_budget() / 4.0);
    for a in routed.attempts() {
        if let Some(report) = a.report() {
            assert_eq!(report.truncation_budget, routed.truncation_budget());
            assert_eq!(
                *a.admission(),
                route_admission(report, routed.total_budget())
            );
        }
        if let Some(bound) = a.bound() {
            assert!(bound <= routed.total_budget(), "relaxed tolerance");
        }
    }
    match routed.choice() {
        RouteChoice::Chebyshev => {
            assert!(cheb.is_admitted());
            assert!(!lag.is_admitted() || lag.vector_products() >= cheb.vector_products());
        }
        RouteChoice::Laguerre => {
            assert!(lag.is_admitted());
            assert!(!cheb.is_admitted() || lag.vector_products() < cheb.vector_products());
        }
        RouteChoice::Fallback {
            chebyshev_reason,
            laguerre_reason,
        } => {
            assert!(!cheb.is_admitted() && !lag.is_admitted());
            assert_eq!(Some(chebyshev_reason.as_str()), cheb.rejection_reason());
            assert_eq!(Some(laguerre_reason.as_str()), lag.rejection_reason());
            assert!(!chebyshev_reason.is_empty() && !laguerre_reason.is_empty());
            assert!(routed.fused().is_none() && routed.bound().is_none());
            assert!(routed.report().is_none());
        }
    }
    if let Some(basis) = routed.basis() {
        let chosen = routed.attempt(basis);
        assert_eq!(routed.bound(), chosen.bound());
        assert_eq!(
            routed.fused(),
            Some(chosen.report().unwrap().fused.as_slice())
        );
    }
}

#[test]
fn invalid_total_budget_step_and_input_are_errors_without_work() {
    let op = verified(6, 10.0, 11);
    let w = distinct(6, 12);
    for budget in [f64::NAN, -1.0, -1.0e-300, f64::INFINITY, f64::NEG_INFINITY] {
        let mut work = WorkCounters::default();
        let result = route_joint_phi(&op, 0.1, JointPhiInput::Distinct(&w), budget, &mut work);
        assert!(
            matches!(result, Err(CoreError::InvalidInput(ref m)) if m.starts_with(ROUTER_INPUT_UNSUPPORTED)),
            "budget {budget:e}"
        );
        assert_eq!(work, WorkCounters::default());
    }
    for h in [f64::NAN, -0.1, f64::INFINITY] {
        let mut work = WorkCounters::default();
        assert!(route_joint_phi(&op, h, JointPhiInput::Distinct(&w), 1.0e-8, &mut work).is_err());
        assert_eq!(work, WorkCounters::default());
    }
    let mut bad = w.clone();
    bad[3][1] = f64::NAN;
    assert!(
        route_joint_phi(
            &op,
            0.1,
            JointPhiInput::Distinct(&bad),
            1.0e-8,
            &mut WorkCounters::default()
        )
        .is_err()
    );
    let short = vec![1.0; 5];
    assert!(
        route_joint_phi(
            &op,
            0.1,
            JointPhiInput::SameVector {
                vector: &short,
                scales: [1.0; 5]
            },
            1.0e-8,
            &mut WorkCounters::default()
        )
        .is_err()
    );
}

#[test]
fn admitted_route_meets_the_rule_and_its_budget() {
    let op = verified(8, 20.0, 21);
    let w = distinct(8, 22);
    let routed = route(&op, 0.05, &w, 1.0e-8);
    assert_rule(&routed);
    assert!(!routed.is_fallback(), "{:?}", routed.choice());
    let lag = routed.attempt(PolynomialBasis::Laguerre);
    assert!(lag.laguerre_scale().is_some());
    assert!(lag.degree().unwrap() <= LAGUERRE_ADJOINT_DEGREE_LIMIT);
    assert_eq!(lag.coefficient_setups(), 1);
    assert_eq!(
        routed
            .attempt(PolynomialBasis::Chebyshev)
            .coefficient_setups(),
        1
    );
    let json = serde_json::to_value(&routed).unwrap();
    assert!(json["choice"]["kind"].is_string());
    assert_eq!(json["attempts"].as_array().unwrap().len(), 2);
    assert!(json["attempts"][1]["laguerre_scale"].is_number());
}

#[test]
fn declared_enclosure_is_never_admitted_even_at_zero_step() {
    let op = declared(6, 50.0, 31);
    let w = distinct(6, 32);
    for h in [0.03, 0.0] {
        for budget in [1.0e-8, 1.0] {
            let routed = route(&op, h, &w, budget);
            assert_rule(&routed);
            assert!(routed.is_fallback(), "h {h}: {:?}", routed.choice());
        }
    }
    // At h = 0 the Chebyshev report itself is certified (phi_k(0) does not
    // depend on A), so only the router guard keeps it out.
    let report = joint_phi_action(
        &op,
        0.0,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Chebyshev,
        1.0e-9,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(admitted(&report.admit_total_error(1.0)));
    assert!(!admitted(&route_admission(&report, 1.0)));
}

#[test]
fn unbounded_timing_execution_is_never_admitted() {
    let op = verified(6, 30.0, 41);
    let w = distinct(6, 42);
    let mut scalar = DenseMatrix::zeros(5, 5);
    for i in 0..5 {
        scalar[(i, i)] = -3.0;
    }
    let scalar_op = SymmetricNonpositiveOperator::gershgorin(scalar).unwrap();
    let w5 = distinct(5, 43);
    for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
        for (op, w, h) in [(&op, &w, 0.03), (&scalar_op, &w5, 0.2), (&op, &w, 0.0)] {
            let report = joint_phi_action_unbounded(
                op,
                h,
                JointPhiInput::Distinct(w),
                basis,
                1.0e-10,
                None,
                &mut WorkCounters::default(),
            )
            .unwrap();
            assert_eq!(report.execution, EXECUTION_UNBOUNDED_TIMING);
            assert!(
                !admitted(&route_admission(&report, 1.0e300)),
                "{basis:?} h {h}"
            );
        }
    }
    // The scalar branch computes its enclosures in both executions, so the
    // Chebyshev admission alone would accept the timing report.
    let timing_scalar = joint_phi_action_unbounded(
        &scalar_op,
        0.2,
        JointPhiInput::Distinct(&w5),
        PolynomialBasis::Chebyshev,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(admitted(&timing_scalar.admit_total_error(1.0)));
}

#[test]
fn laguerre_degree_above_the_adjoint_limit_is_never_admitted() {
    let op = verified(6, 400.0, 51);
    let w = distinct(6, 52);
    let routed = route(&op, 0.25, &w, 1.0e-12);
    assert_rule(&routed);
    let lag = routed.attempt(PolynomialBasis::Laguerre);
    let degree = lag.degree().unwrap();
    assert!(degree > LAGUERRE_ADJOINT_DEGREE_LIMIT, "degree {degree}");
    assert!(!lag.is_admitted());
    assert!(lag.rejection_reason().unwrap().contains("adjoint limit"));
    assert_ne!(routed.basis(), Some(PolynomialBasis::Laguerre));
    let report = lag.report().unwrap();
    assert!(!admitted(&route_admission(report, 1.0e300)));
    assert!(report.laguerre_adjoint_total.is_none());
    // Larger h: the Laguerre action itself fails (its majorant leaves the
    // binary64 range), which is a rejected attempt, not a router error.
    let routed = route(&op, 1.0, &w, 1.0e-12);
    assert_rule(&routed);
    let lag = routed.attempt(PolynomialBasis::Laguerre);
    assert!(lag.report().is_none() && lag.degree().is_none());
    assert!(lag.rejection_reason().unwrap().contains("action failed"));
    assert_ne!(routed.basis(), Some(PolynomialBasis::Laguerre));
}

#[test]
fn estimate_only_totals_are_never_admitted() {
    let op = verified(6, 30.0, 61);
    let w = distinct(6, 62);
    let lag = joint_phi_action(
        &op,
        0.03,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Laguerre,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(matches!(
        lag.total_error,
        TotalErrorStatus::EstimateOnly { .. }
    ));
    assert!(lag.laguerre_majorant_total.is_some());
    assert!(admitted(&route_admission(&lag, 1.0)));
    // Without the proved adjoint total only estimates remain.
    let mut stripped = lag.clone();
    stripped.laguerre_adjoint_total = None;
    assert!(!admitted(&route_admission(&stripped, 1.0e300)));
    let mut stripped = lag.clone();
    stripped.column_errors[2].recurrence_adjoint = None;
    assert!(!admitted(&route_admission(&stripped, 1.0e300)));
    // A Chebyshev report whose total is an estimate.
    let mut cheb = joint_phi_action(
        &op,
        0.03,
        JointPhiInput::Distinct(&w),
        PolynomialBasis::Chebyshev,
        1.0e-10,
        None,
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(admitted(&route_admission(&cheb, 1.0)));
    cheb.total_error = TotalErrorStatus::EstimateOnly {
        reason: "test".into(),
        bounded_components: 0.0,
    };
    assert!(!admitted(&route_admission(&cheb, 1.0e300)));
}

#[test]
fn unmet_budget_ends_in_fallback() {
    let op = verified(6, 30.0, 71);
    let w = distinct(6, 72);
    for budget in [1.0e-300, 0.0, -0.0] {
        let routed = route(&op, 0.03, &w, budget);
        assert_rule(&routed);
        assert!(routed.is_fallback(), "budget {budget:e}");
    }
    // The bound of an admitted result as the budget's upper neighbour: a
    // smaller budget re-chooses degrees, and whatever it admits stays within
    // it.
    let generous = route(&op, 0.03, &w, 1.0e-8);
    assert_rule(&generous);
    let bound = generous.bound().unwrap();
    let tight = route(&op, 0.03, &w, bound * 0.5);
    assert_rule(&tight);
}

#[test]
fn ties_go_to_chebyshev_and_scalar_branch_routes() {
    // h = 0 and A = -3 I: no vector products in either basis.
    let op = verified(6, 30.0, 81);
    let w = distinct(6, 82);
    let mut scalar = DenseMatrix::zeros(5, 5);
    for i in 0..5 {
        scalar[(i, i)] = -3.0;
    }
    let scalar_op = SymmetricNonpositiveOperator::gershgorin(scalar).unwrap();
    let w5 = distinct(5, 83);
    for routed in [
        route(&op, 0.0, &w, 1.0e-12),
        route(&scalar_op, 0.2, &w5, 1.0e-12),
    ] {
        assert_rule(&routed);
        for a in routed.attempts() {
            assert_eq!(a.vector_products(), 0);
            assert_eq!(a.degree(), Some(0));
            assert!(a.is_admitted());
        }
        assert_eq!(*routed.choice(), RouteChoice::Chebyshev);
    }
}

macro_rules! assert_fields_add_up {
    ($after:expr, $before:expr, $attempts:expr, $($f:ident),* $(,)?) => {
        $(
            let sum: u64 = $attempts.iter().map(|a| a.work().$f).sum();
            assert_eq!($after.$f, $before.$f + sum, stringify!($f));
        )*
    };
}

#[test]
fn caller_counters_equal_the_sum_of_the_attempts() {
    let op = verified(7, 25.0, 91);
    let w = distinct(7, 92);
    let v = splitmix(93, 7);
    let before = WorkCounters {
        poly_vector_products: 7,
        poly_block_products: 2,
        poly_coefficient_setups: 1,
        poly_block_allocations: 3,
        linear_matvecs: 4,
        linear_matvec_vectors: 4,
        linear_solves: 1,
        ..WorkCounters::default()
    };
    let inputs = [
        JointPhiInput::Distinct(&w),
        JointPhiInput::SameVector {
            vector: &v,
            scales: [1.0, -0.5, 0.25, 2.0, -1.0],
        },
    ];
    for (h, budget) in [
        (0.05, 1.0e-8),
        (0.5, 1.0e-12),
        (0.0, 1.0e-8),
        (0.05, 1.0e-300),
    ] {
        for input in inputs {
            let mut work = before;
            let routed = route_joint_phi(&op, h, input, budget, &mut work).unwrap();
            assert_rule(&routed);
            let attempts = routed.attempts();
            assert_fields_add_up!(
                work,
                before,
                attempts,
                poly_block_products,
                poly_vector_products,
                poly_coefficient_setups,
                poly_coefficient_reuses,
                poly_block_allocations,
                poly_fallbacks,
                linear_solves,
                linear_iterations,
                linear_matvecs,
                linear_matvec_vectors,
                preconditioner_apps,
                diagnostic_matvecs,
                phi_actions,
                phi_dense_oracle_calls,
            );
            for a in attempts {
                assert_eq!(a.vector_products(), a.work().poly_vector_products);
                assert_eq!(a.coefficient_setups(), a.work().poly_coefficient_setups);
            }
            // The whole ledger: nothing else moved.
            assert_eq!(work.checked_delta(before), Some(routed.total_work()));
            if h > 0.0 && budget > 1.0e-100 {
                let width = match input {
                    JointPhiInput::Distinct(_) => 5,
                    JointPhiInput::SameVector { .. } => 1,
                };
                for a in attempts {
                    let degree = a.degree().unwrap() as u64;
                    assert_eq!(a.vector_products(), width * degree);
                }
            }
        }
    }
}
