//! Exporter of the preregistered router fixtures (RVJ DAG node PP08,
//! `research/pp08_laguerre_router_20261004`). Ignored: run with
//! `PP08_CASES=<path relative to the repository root>`; the 50-digit check is
//! `tools/pp08_laguerre_router_check.py`. Refuses to overwrite its output.

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    laguerre_adjoint::LAGUERRE_ADJOINT_DEGREE_LIMIT,
    polynomial_action::{
        EnclosureEvidence, JointPhiInput, JointPhiReport, PolynomialBasis, RouteAttempt,
        RouteChoice, RoutedPhi, SymmetricNonpositiveOperator, TotalErrorAdmission,
        TotalErrorStatus, joint_phi_action_unbounded, route_admission, route_joint_phi,
    },
};
use serde_json::{Value, json};

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

/// Fraction of `rho` at the upper end of the spectrum.
const LAMBDA_MIN_FRACTION: f64 = 0.1;
/// Largest Givens angle of the sweep that forms `Q`.
const MAX_ANGLE: f64 = 0.3;

/// `A = Q diag(lambda) Q^T` rounded and symmetrized: `lambda` in
/// `[-rho, -0.1 rho]` with both ends present, `Q` one sweep of adjacent
/// Givens rotations with angles in `[-0.3, 0.3]`. The small angles keep
/// the Gershgorin discs nonpositive, so the enclosure verifies.
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

fn minus_three_identity(n: usize) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -3.0;
    }
    a
}

const SCALES: [f64; 5] = [1.0, -0.5, 0.25, 2.0, -1.0];

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

fn admission_json(a: &TotalErrorAdmission) -> Value {
    match a {
        TotalErrorAdmission::Admitted { bound, .. } => {
            json!({"admitted": true, "bound": hex(*bound), "bound_f64": bound})
        }
        TotalErrorAdmission::Rejected { reason } => json!({"admitted": false, "reason": reason}),
    }
}

/// The report's own candidate total: the certified total for Chebyshev, the
/// adjoint total for Laguerre (reported beside the admission).
fn candidate_total(report: &JointPhiReport) -> Value {
    let value = match report.basis {
        PolynomialBasis::Chebyshev => match report.total_error {
            TotalErrorStatus::Certified { bound } => Some(bound),
            TotalErrorStatus::EstimateOnly { .. } => None,
        },
        PolynomialBasis::Laguerre => report.laguerre_adjoint_total,
    };
    value.map_or(Value::Null, |v| json!({"hex": hex(v), "f64": v}))
}

fn attempt_json(a: &RouteAttempt) -> Value {
    let report = a.report().map(|r| {
        json!({
            "branch": r.branch,
            "evidence": format!("{:?}", r.evidence),
            "evidence_verified": matches!(r.evidence, EnclosureEvidence::Gershgorin),
            "execution": r.execution,
            "fused": hex_vec(&r.fused),
            "candidate_total": candidate_total(r),
            "total_error_is_estimate": matches!(r.total_error, TotalErrorStatus::EstimateOnly { .. }),
            "truncation_budget": hex(r.truncation_budget),
            "condition_proxy": r.condition_proxy,
        })
    });
    json!({
        "basis": format!("{:?}", a.basis()),
        "degree": a.degree(),
        "laguerre_scale": a.laguerre_scale(),
        "admission": admission_json(a.admission()),
        "vector_products": a.vector_products(),
        "coefficient_setups": a.coefficient_setups(),
        "work": serde_json::to_value(a.work()).unwrap(),
        "report": report,
    })
}

fn choice_name(routed: &RoutedPhi) -> &'static str {
    match routed.choice() {
        RouteChoice::Chebyshev => "Chebyshev",
        RouteChoice::Laguerre => "Laguerre",
        RouteChoice::Fallback { .. } => "Fallback",
    }
}

fn case(
    label: &str,
    group: &str,
    a: &DenseMatrix,
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    total_budget: f64,
) -> Value {
    let n = a.nrows();
    // The caller's counters start at zero, so after routing they are the
    // routed work itself.
    let mut work = WorkCounters::default();
    let routed = route_joint_phi(op, h, input.as_input(), total_budget, &mut work).unwrap();
    let enclosure = op.enclosure();
    json!({
        "label": label,
        "group": group,
        "n": n,
        "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "h": hex(h),
        "h_f64": h,
        "input": input.name(),
        "w": input.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
        "enclosure": {"lambda": enclosure.lambda, "rho": enclosure.rho,
                      "evidence": format!("{:?}", enclosure.evidence)},
        "total_budget": hex(total_budget),
        "total_budget_f64": total_budget,
        "truncation_budget": hex(routed.truncation_budget()),
        "choice": choice_name(&routed),
        "fallback_reasons": match routed.choice() {
            RouteChoice::Fallback { chebyshev_reason, laguerre_reason } =>
                json!({"chebyshev": chebyshev_reason, "laguerre": laguerre_reason}),
            _ => Value::Null,
        },
        "attempts": routed.attempts().iter().map(attempt_json).collect::<Vec<_>>(),
        "routed_fused": routed.fused().map(hex_vec),
        "routed_bound": routed.bound().map(hex),
        "caller_counters": serde_json::to_value(work).unwrap(),
    })
}

#[test]
#[ignore = "exporter of research/pp08_laguerre_router_20261004; set PP08_CASES"]
fn pp08_router_study_export() {
    let output = std::env::var("PP08_CASES").expect("set PP08_CASES");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&output);
    assert!(!path.exists(), "immutable output exists: {output}");

    let mut cases = Vec::new();
    let mut spectra = serde_json::Map::new();
    let mut fixture = |n: usize, rho_index: usize, rho: f64| {
        let seed = 1000 + 100 * n as u64 + rho_index as u64;
        let (a, spectrum) = spectral_matrix(n, rho, seed);
        let op = SymmetricNonpositiveOperator::gershgorin(a.clone())
            .expect("fixture outside the verified domain");
        spectra.insert(format!("n{n}-rho{rho}"), json!(spectrum));
        (a, op)
    };

    // Main grid.
    for n in [5, 16] {
        for (rho_index, rho) in [2.0, 30.0, 200.0].into_iter().enumerate() {
            let (a, op) = fixture(n, rho_index, rho);
            for (h_index, h) in [1.0e-3, 3.0e-2, 0.2].into_iter().enumerate() {
                let seed = 3000 + 100 * n as u64 + 10 * rho_index as u64 + h_index as u64;
                for input in [Input::distinct(n, seed), Input::same(n, seed + 5)] {
                    for budget in [1.0e-8, 1.0e-12] {
                        let label = format!("n{n}-rho{rho}-h{h}-{}-b{budget:e}", input.name());
                        cases.push(case(&label, "main", &a, &op, h, &input, budget));
                    }
                }
            }
        }
    }

    // Scalar branch A = -3 I.
    let scalar = minus_three_identity(5);
    let scalar_op = SymmetricNonpositiveOperator::gershgorin(scalar.clone()).unwrap();
    let input = Input::distinct(5, 4001);
    for budget in [1.0e-8, 1.0e-12] {
        cases.push(case(
            &format!("scalar-minus3I-b{budget:e}"),
            "scalar",
            &scalar,
            &scalar_op,
            0.2,
            &input,
            budget,
        ));
    }

    // Declared enclosure: the largest diagonal magnitude contains the
    // diagonal but not the Gershgorin discs.
    let (a30, op30) = fixture(5, 1, 30.0);
    let diagonal_max = (0..5).map(|i| a30[(i, i)].abs()).fold(0.0, f64::max);
    let declared =
        SymmetricNonpositiveOperator::new(a30.clone(), 0.0, diagonal_max, "pp08-declared").unwrap();
    assert!(matches!(
        declared.enclosure().evidence,
        EnclosureEvidence::Declared { .. }
    ));
    let input = Input::distinct(5, 4101);
    for h in [3.0e-2, 0.0] {
        cases.push(case(
            &format!("declared-h{h}-b1e-8"),
            "declared",
            &a30,
            &declared,
            h,
            &input,
            1.0e-8,
        ));
    }

    // Zero step on a verified operator.
    let (a16, op16) = fixture(16, 1, 30.0);
    let input16 = Input::distinct(16, 4201);
    cases.push(case(
        "zero-h-n16-rho30-b1e-12",
        "zero-h",
        &a16,
        &op16,
        0.0,
        &input16,
        1.0e-12,
    ));

    // An unmet budget.
    cases.push(case(
        "unmet-budget-1e-300",
        "unmet",
        &a16,
        &op16,
        3.0e-2,
        &input16,
        1.0e-300,
    ));

    // Laguerre degree above the adjoint limit: the first h of a fixed list
    // whose Laguerre attempt exceeds it.
    let (a200, op200) = fixture(5, 2, 200.0);
    let input200 = Input::distinct(5, 4301);
    let mut above = json!({"found": false});
    for h in [0.25, 0.35, 0.5] {
        let mut work = WorkCounters::default();
        let routed = route_joint_phi(&op200, h, input200.as_input(), 1.0e-12, &mut work).unwrap();
        let degree = routed.attempt(PolynomialBasis::Laguerre).degree();
        if degree.is_some_and(|d| d > LAGUERRE_ADJOINT_DEGREE_LIMIT) {
            cases.push(case(
                &format!("laguerre-degree-above-limit-h{h}"),
                "degree-above-limit",
                &a200,
                &op200,
                h,
                &input200,
                1.0e-12,
            ));
            above = json!({"found": true, "h": h, "degree": degree});
            break;
        }
    }

    // Guards outside the router's own path.
    let mut guards = serde_json::Map::new();
    guards.insert("degree_above_limit".into(), above);
    let invalid: Vec<Value> = [f64::NAN, -1.0, f64::INFINITY]
        .iter()
        .map(|b| {
            let mut work = WorkCounters::default();
            let result = route_joint_phi(&op30, 3.0e-2, input.as_input(), *b, &mut work);
            json!({"budget": format!("{b:e}"), "is_error": result.is_err(),
                   "no_work": work == WorkCounters::default()})
        })
        .collect();
    guards.insert("invalid_total_budgets".into(), json!(invalid));
    let mut timing = Vec::new();
    for (label, op, h, w) in [
        ("n5-rho30-h0.03", &op30, 3.0e-2, &input),
        (
            "scalar-minus3I-h0.2",
            &scalar_op,
            0.2,
            &Input::distinct(5, 4001),
        ),
        ("n5-rho30-h0", &op30, 0.0, &input),
    ] {
        for basis in [PolynomialBasis::Chebyshev, PolynomialBasis::Laguerre] {
            let report = joint_phi_action_unbounded(
                op,
                h,
                w.as_input(),
                basis,
                0.25e-8,
                None,
                &mut WorkCounters::default(),
            )
            .unwrap();
            timing.push(json!({
                "label": label,
                "basis": format!("{basis:?}"),
                "execution": report.execution,
                "admitted_at_1e300": matches!(route_admission(&report, 1.0e300), TotalErrorAdmission::Admitted { .. }),
            }));
        }
    }
    guards.insert("unbounded_timing_execution".into(), json!(timing));

    let report = json!({
        "schema": "vigilode-pp08-laguerre-router-cases-v1",
        "node": "research/pp08_laguerre_router_20261004",
        "fixture": {"lambda_min_fraction": LAMBDA_MIN_FRACTION, "max_givens_angle": MAX_ANGLE,
                    "laguerre_adjoint_degree_limit": LAGUERRE_ADJOINT_DEGREE_LIMIT,
                    "spectra": spectra},
        "cases": cases,
        "guards": guards,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    let count = report["cases"].as_array().unwrap().len();
    println!("wrote {count} cases to {output}");
}
