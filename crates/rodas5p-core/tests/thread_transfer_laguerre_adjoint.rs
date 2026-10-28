//! Signed output-adjoint Laguerre recurrence bound (research node
//! `research/thread_transfer_laguerre_adjoint_20261002`, thread-transfer DAG
//! node P1-LAGUERRE-CERT). The exact identity, the exact recurrence errors
//! and the exact Bernstein bounds are checked by
//! `tools/thread_transfer_laguerre_check.py` on the cases this file writes.

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    directed::{add_up, div_up, mul_up, sub_up},
    laguerre_adjoint::{
        LAGUERRE_ADJOINT_DEGREE_LIMIT, LAGUERRE_ADJOINT_DEPTH, laguerre_adjoint_envelopes,
    },
    polynomial_action::{
        JointPhiInput, PolynomialBasis, SymmetricNonpositiveOperator, TotalErrorStatus,
        joint_phi_action, laguerre_recurrence_with_residuals,
    },
};
use serde_json::{Value, json};

fn hex(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

/// The review's operators: diagonal `X = diag(L/8, L/2, L)` or the
/// dyadic rotation `Q diag(L/8, L/4, L/2, L) Q^T`, `Q` the 4 x 4 Hadamard
/// matrix over 2 (exact in binary64). Returned as `A = -X` with `beta = 1`.
fn operator(extent: f64, rotated: bool) -> (DenseMatrix, Vec<f64>) {
    if !rotated {
        let xs = [extent / 8.0, extent / 2.0, extent];
        let mut a = DenseMatrix::zeros(3, 3);
        for (i, x) in xs.iter().enumerate() {
            a[(i, i)] = -x;
        }
        (a, vec![1.0, -0.75, 0.125])
    } else {
        let xs = [extent / 8.0, extent / 4.0, extent / 2.0, extent];
        let signs = [
            [1.0, 1.0, 1.0, 1.0],
            [1.0, -1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0, 1.0],
        ];
        let mut a = DenseMatrix::zeros(4, 4);
        for i in 0..4 {
            for j in 0..4 {
                let x: f64 = (0..4)
                    .map(|k| signs[i][k] * signs[j][k] * xs[k] / 4.0)
                    .sum();
                a[(i, j)] = -x;
            }
        }
        (a, vec![1.0, -0.75, 0.125, 0.375])
    }
}

/// The R4 scalar majorant `sum_j |c_j| E_j`,
/// `E_(n+1) = d_n E_n + n/(n+1) E_(n-1) + eps_n`, rounded upward.
fn majorant(stored: &[f64], local: &[f64], extent: f64) -> f64 {
    let mut e = vec![0.0; local.len() + 1];
    for (n, eps) in local.iter().enumerate() {
        let k = n as f64;
        let center = 2.0 * k + 1.0;
        let d = div_up(center.max(sub_up(extent, center).unwrap()), k + 1.0).unwrap();
        let mut next = add_up(mul_up(d, e[n]).unwrap(), *eps).unwrap();
        if n > 0 {
            next = add_up(next, mul_up(div_up(k, k + 1.0).unwrap(), e[n - 1]).unwrap()).unwrap();
        }
        e[n + 1] = next;
    }
    stored.iter().zip(&e).skip(1).fold(0.0, |acc, (c, b)| {
        add_up(acc, mul_up(c.abs(), *b).unwrap()).unwrap()
    })
}

struct Case {
    label: String,
    degree: usize,
    extent: f64,
    rotated: bool,
    stored: Vec<f64>,
}

fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    let geometric = |m: usize, q: f64| (0..=m).map(|n| (1.0 - q) * q.powi(n as i32)).collect();
    for (m, extent, q) in [
        (16, 1.0, 0.5),
        (32, 1.0, 0.75),
        (64, 1.0, 0.75),
        (32, 4.0, 0.75),
        (64, 4.0, 0.75),
        (32, 16.0, 0.75),
    ] {
        out.push(Case {
            label: format!("geometric-m{m}-L{extent}"),
            degree: m,
            extent,
            rotated: false,
            stored: geometric(m, q),
        });
    }
    out.push(Case {
        label: "geometric-rotated-m32-L4".into(),
        degree: 32,
        extent: 4.0,
        rotated: true,
        stored: geometric(32, 0.75),
    });
    // Signed pseudo-random stored coefficients for the identity.
    let mut state = 0x853c_49e6_748f_ea9b_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
    };
    for (m, extent, rotated) in [(20, 2.0, false), (24, 4.0, true)] {
        out.push(Case {
            label: format!(
                "signed-m{m}-L{extent}{}",
                if rotated { "-rotated" } else { "" }
            ),
            degree: m,
            extent,
            rotated,
            stored: (0..=m).map(|_| next()).collect(),
        });
    }
    out
}

fn run_case(case: &Case) -> Value {
    let (a, w) = operator(case.extent, case.rotated);
    let (vectors, local) = laguerre_recurrence_with_residuals(&a, 1.0, &w, case.degree).unwrap();
    let beta =
        laguerre_adjoint_envelopes(&case.stored, case.extent, LAGUERRE_ADJOINT_DEPTH).unwrap();
    let adjoint = beta[1..].iter().zip(&local).fold(0.0, |acc, (b, e)| {
        add_up(acc, mul_up(*b, *e).unwrap()).unwrap()
    });
    let old = majorant(&case.stored, &local, case.extent);
    json!({
        "label": case.label,
        "degree": case.degree,
        "extent": hex(case.extent),
        "depth": LAGUERRE_ADJOINT_DEPTH,
        "matrix": (0..a.nrows()).map(|i| (0..a.ncols()).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "beta_scale": hex(1.0),
        "source": w.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
        "stored": case.stored.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
        "vectors": vectors.iter().map(|v| v.iter().map(|x| hex(*x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "local": local.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
        "envelopes": beta.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
        "adjoint_bound": hex(adjoint),
        "majorant_bound": hex(old),
        "summary": {"adjoint_bound": adjoint, "majorant_bound": old,
            "majorant_over_adjoint": old / adjoint},
    })
}

/// Writes the cases for the exact checker.
#[test]
#[ignore = "writes fixtures/thread_transfer_laguerre_cases.json"]
fn write_laguerre_cases() {
    let value = json!({
        "schema": "vigilode-thread-transfer-laguerre-cases-v1",
        "cases": cases().iter().map(run_case).collect::<Vec<_>>(),
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/thread_transfer_laguerre_cases.json");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
}

fn symmetric(n: usize) -> DenseMatrix {
    // A diagonally dominant symmetric negative definite tridiagonal matrix:
    // Gershgorin verifies its enclosure.
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -2.0 - 0.1 * i as f64;
        if i + 1 < n {
            a[(i, i + 1)] = 0.5;
            a[(i + 1, i)] = 0.5;
        }
    }
    a
}

#[test]
fn laguerre_adjoint_contract() {
    // 1, 3, 5: native side of the cases (the exact checks are in Python).
    let rows = cases().iter().map(run_case).collect::<Vec<_>>();
    let ratio = |label: &str| {
        rows.iter()
            .find(|r| r["label"] == json!(label))
            .unwrap()["summary"]["majorant_over_adjoint"]
            .as_f64()
            .unwrap()
    };
    let tighter = ratio("geometric-m16-L1") >= 5.0
        && [
            "geometric-m32-L1",
            "geometric-m64-L1",
            "geometric-m32-L4",
            "geometric-m64-L4",
        ]
        .iter()
        .all(|label| ratio(label) >= 1.0e5);
    // 4. Integration into the phi action.
    let n = 6;
    let w = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect::<Vec<_>>();
    let vectors: [Vec<f64>; 5] =
        std::array::from_fn(|k| w.iter().map(|x| x / (k + 1) as f64).collect());
    let verified = SymmetricNonpositiveOperator::gershgorin(symmetric(n)).unwrap();
    let mut work = WorkCounters::default();
    let report = joint_phi_action(
        &verified,
        0.5,
        JointPhiInput::Distinct(&vectors),
        PolynomialBasis::Laguerre,
        1.0e-10,
        None,
        &mut work,
    )
    .unwrap();
    let bounded = match &report.total_error {
        TotalErrorStatus::EstimateOnly {
            bounded_components, ..
        } => Some(*bounded_components),
        TotalErrorStatus::Certified { .. } => None,
    };
    let integrated = report.degree <= LAGUERRE_ADJOINT_DEGREE_LIMIT
        && report
            .column_errors
            .iter()
            .all(|c| c.recurrence_adjoint.is_some())
        && bounded.is_some()
        && report
            .laguerre_adjoint_total
            .is_some_and(|total| total >= bounded.unwrap());
    let declared = SymmetricNonpositiveOperator::new(symmetric(n), 0.0, 2.6, "test").unwrap();
    let declared_report = joint_phi_action(
        &declared,
        0.5,
        JointPhiInput::Distinct(&vectors),
        PolynomialBasis::Laguerre,
        1.0e-10,
        None,
        &mut work,
    )
    .unwrap();
    let declared_ok = matches!(
        declared_report.total_error,
        TotalErrorStatus::EstimateOnly { .. }
    ) && declared_report.laguerre_adjoint_total.is_none();
    // Above the degree limit (a large step needs a high degree) the adjoint is absent.
    let high = joint_phi_action(
        &verified,
        20.0,
        JointPhiInput::Distinct(&vectors),
        PolynomialBasis::Laguerre,
        1.0e-12,
        None,
        &mut work,
    );
    let high_ok = match &high {
        Ok(r) if r.degree > LAGUERRE_ADJOINT_DEGREE_LIMIT => {
            r.column_errors
                .iter()
                .all(|c| c.recurrence_adjoint.is_none())
                && r.laguerre_adjoint_total.is_none()
                && matches!(r.total_error, TotalErrorStatus::EstimateOnly { .. })
        }
        Ok(_) => false,
        Err(_) => false,
    };
    let result = json!({
        "schema": "vigilode-thread-transfer-laguerre-adjoint-v1",
        "gate_native": {
            "integration": integrated && declared_ok && high_ok,
            "tighter": tighter,
        },
        "note": "gate items 1-3 (exact identity, Bernstein validity, enclosure) are in EXACT_CHECK.json",
        "integration": {
            "degree": report.degree,
            "laguerre_scale": report.laguerre_scale,
            "recurrence_adjoint": report.column_errors.iter().map(|c| c.recurrence_adjoint).collect::<Vec<_>>(),
            "recurrence_majorant": report.column_errors.iter().map(|c| c.recurrence_majorant).collect::<Vec<_>>(),
            "bounded_components": bounded,
            "laguerre_adjoint_total": report.laguerre_adjoint_total,
            "laguerre_majorant_total": report.laguerre_majorant_total,
            "total_error": report.total_error,
            "declared_has_no_candidate_total": declared_ok,
            "above_limit": high.as_ref().map(|r| json!({"degree": r.degree, "ok": high_ok})).unwrap_or_else(|e| json!({"error": e.to_string()})),
        },
        "cases": rows.iter().map(|r| json!({"label": r["label"], "degree": r["degree"], "summary": r["summary"]})).collect::<Vec<_>>(),
    });
    if let Ok(path) = std::env::var("THREAD_TRANSFER_LAGUERRE_OUTPUT") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        std::fs::write(&path, serde_json::to_string_pretty(&result).unwrap() + "\n").unwrap();
    }
    println!(
        "{}",
        json!({"gate_native": result["gate_native"], "integration": result["integration"]["degree"]})
    );
    for r in &rows {
        println!(
            "{} majorant/adjoint {:.3e}",
            r["label"],
            r["summary"]["majorant_over_adjoint"].as_f64().unwrap()
        );
    }
    assert!(declared_ok && high_ok);
}
