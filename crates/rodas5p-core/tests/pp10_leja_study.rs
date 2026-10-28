//! Exporter of the preregistered Newton-Leja comparison (RVJ DAG node PP10,
//! `research/pp10_leja_candidate_20261004`). Ignored: run with
//! `PP10_CASES=<path relative to the repository root>`; the 50-digit check is
//! `tools/pp10_leja_check.py`. Refuses to overwrite its output.
//!
//! The fixtures are the PP08 main grid, built by the PP08 exporter's code
//! (`tests/pp08_laguerre_router_study.rs`, copied verbatim below: `splitmix`,
//! `spectral_matrix`, `SCALES`, `Input`, the seeds and the labels): n in {5,
//! 16}, rho in {2, 30, 200}, h in {1e-3, 3e-2, 0.2}, distinct and
//! same-vector inputs, at the estimated tolerances 1e-8 and 1e-12. Each case
//! runs the Leja action and the Chebyshev `joint_phi_action` (certified
//! execution, no cache, truncation budget = the same tolerance) on the same
//! target, each with fresh counters.

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    leja_action::{LEJA_DEGREE_CAP, LEJA_GRID_INTERVALS, LejaReport, leja_phi_action},
    polynomial_action::{
        JointPhiInput, JointPhiReport, PolynomialBasis, SymmetricNonpositiveOperator,
        TotalErrorAdmission, TotalErrorStatus, joint_phi_action, route_admission,
    },
};
use serde_json::{Value, json};

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hex_vec(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

// ---- PP08 fixture code (verbatim) -------------------------------------
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

// ---- end of the PP08 fixture code --------------------------------------

fn status_json(status: &TotalErrorStatus) -> Value {
    match status {
        TotalErrorStatus::Certified { bound } => {
            json!({"status": "certified", "bound": hex(*bound), "bound_f64": bound})
        }
        TotalErrorStatus::EstimateOnly {
            reason,
            bounded_components,
        } => json!({"status": "estimate-only", "reason": reason,
                    "bounded_components": hex(*bounded_components)}),
    }
}

fn admission_json(a: &TotalErrorAdmission) -> Value {
    match a {
        TotalErrorAdmission::Admitted { bound, .. } => {
            json!({"admitted": true, "bound": hex(*bound)})
        }
        TotalErrorAdmission::Rejected { reason } => json!({"admitted": false, "reason": reason}),
    }
}

/// The certified APIs on the most admissible-looking carrier of a Leja
/// result: the case's certified Chebyshev report with the Leja output and
/// status put in (`None` when the Chebyshev action failed).
fn certified_apis(leja: &LejaReport, chebyshev: Option<&JointPhiReport>) -> Value {
    let Some(chebyshev) = chebyshev else {
        return Value::Null;
    };
    let mut carrier = chebyshev.clone();
    carrier.fused = leja.fused.clone();
    carrier.columns = leja.columns.clone();
    carrier.degree = leja.degree;
    carrier.total_error = leja.total_error().clone();
    json!({
        "admit_total_error": admission_json(&carrier.admit_total_error(1.0e300)),
        "admit_laguerre_total": admission_json(&carrier.admit_laguerre_total(1.0e300)),
        "route_admission": admission_json(&route_admission(&carrier, 1.0e300)),
    })
}

fn leja_json(result: &Result<LejaReport, String>, work: &WorkCounters) -> Value {
    match result {
        Err(error) => json!({"error": error, "work": serde_json::to_value(work).unwrap()}),
        Ok(r) => json!({
            "branch": r.branch,
            "degree": r.degree,
            "degree_cap": r.degree_cap,
            "converged": r.converged,
            "interval": hex_vec(&r.interval),
            "center": hex(r.center),
            "scale": hex(r.scale),
            "tolerance": hex(r.tolerance),
            "estimate": hex(r.estimate),
            "estimate_f64": r.estimate,
            "column_estimates": hex_vec(&r.column_estimates),
            "fused": hex_vec(&r.fused),
            "total_error": status_json(r.total_error()),
            "vector_products": r.vector_products,
            "block_products": r.block_products,
            "coefficient_setups": r.coefficient_setups,
            "work": serde_json::to_value(work).unwrap(),
        }),
    }
}

fn chebyshev_json(result: &Result<JointPhiReport, String>, work: &WorkCounters) -> Value {
    match result {
        Err(error) => json!({"error": error, "work": serde_json::to_value(work).unwrap()}),
        Ok(r) => json!({
            "branch": r.branch,
            "degree": r.degree,
            "execution": r.execution,
            "truncation_budget": hex(r.truncation_budget),
            "truncation_bound": hex(r.truncation_bound_exact_arithmetic),
            "fused": hex_vec(&r.fused),
            "total_error": status_json(&r.total_error),
            "vector_products": work.poly_vector_products,
            "block_products": work.poly_block_products,
            "coefficient_setups": work.poly_coefficient_setups,
            "work": serde_json::to_value(work).unwrap(),
        }),
    }
}

fn case(
    label: &str,
    a: &DenseMatrix,
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: &Input,
    tolerance: f64,
) -> Value {
    let n = a.nrows();
    let mut leja_work = WorkCounters::default();
    let leja = leja_phi_action(op, h, input.as_input(), tolerance, &mut leja_work)
        .map_err(|e| e.to_string());
    let mut cheb_work = WorkCounters::default();
    let chebyshev = joint_phi_action(
        op,
        h,
        input.as_input(),
        PolynomialBasis::Chebyshev,
        tolerance,
        None,
        &mut cheb_work,
    )
    .map_err(|e| e.to_string());
    let enclosure = op.enclosure();
    json!({
        "label": label,
        "group": "main",
        "n": n,
        "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "h": hex(h),
        "h_f64": h,
        "input": input.name(),
        "w": input.columns().iter().map(|c| hex_vec(c)).collect::<Vec<_>>(),
        "enclosure": {"lambda": hex(enclosure.lambda), "rho": hex(enclosure.rho),
                      "evidence": format!("{:?}", enclosure.evidence)},
        "tolerance": hex(tolerance),
        "tolerance_f64": tolerance,
        "leja": leja_json(&leja, &leja_work),
        "chebyshev": chebyshev_json(&chebyshev, &cheb_work),
        "certified_apis_on_leja": match &leja {
            Ok(r) => certified_apis(r, chebyshev.as_ref().ok()),
            Err(_) => Value::Null,
        },
    })
}

#[test]
#[ignore = "exporter of research/pp10_leja_candidate_20261004; set PP10_CASES"]
fn pp10_leja_study_export() {
    // Without PP10_CASES (the workspace's ignored-test run) nothing is written.
    let Ok(output) = std::env::var("PP10_CASES") else {
        println!("PP10_CASES not set: cases not written");
        return;
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&output);
    assert!(!path.exists(), "immutable output exists: {output}");

    let mut cases = Vec::new();
    let mut spectra = serde_json::Map::new();
    // The PP08 main grid: same seeds, labels with `b` the tolerance.
    for n in [5, 16] {
        for (rho_index, rho) in [2.0, 30.0, 200.0].into_iter().enumerate() {
            let seed = 1000 + 100 * n as u64 + rho_index as u64;
            let (a, spectrum) = spectral_matrix(n, rho, seed);
            let op = SymmetricNonpositiveOperator::gershgorin(a.clone())
                .expect("fixture outside the verified domain");
            spectra.insert(format!("n{n}-rho{rho}"), json!(spectrum));
            for (h_index, h) in [1.0e-3, 3.0e-2, 0.2].into_iter().enumerate() {
                let seed = 3000 + 100 * n as u64 + 10 * rho_index as u64 + h_index as u64;
                for input in [Input::distinct(n, seed), Input::same(n, seed + 5)] {
                    for tolerance in [1.0e-8, 1.0e-12] {
                        let label = format!("n{n}-rho{rho}-h{h}-{}-b{tolerance:e}", input.name());
                        cases.push(case(&label, &a, &op, h, &input, tolerance));
                    }
                }
            }
        }
    }

    let report = json!({
        "schema": "vigilode-pp10-leja-cases-v1",
        "node": "research/pp10_leja_candidate_20261004",
        "fixture": {"source": "PP08 main grid (tests/pp08_laguerre_router_study.rs)",
                    "lambda_min_fraction": LAMBDA_MIN_FRACTION, "max_givens_angle": MAX_ANGLE,
                    "spectra": spectra},
        "leja": {"degree_cap": LEJA_DEGREE_CAP, "grid_intervals": LEJA_GRID_INTERVALS,
                 "points": hex_vec(rodas5p_core::leja_action::leja_points())},
        "cases": cases,
    });
    std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    let count = report["cases"].as_array().unwrap().len();
    println!("wrote {count} cases to {output}");
}
