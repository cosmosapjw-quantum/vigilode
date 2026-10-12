//! Research node PY02 (`research/py02_laguerre_adjoint_bound_20261011`),
//! stage 1: the plan-only export of admission item A3.
//!
//! - The contract tests (run always) check the local plan-only helper
//!   [`plan_only_comparison`] on fresh fixtures that are not stage-2 cases:
//!   its degree, Laguerre scale, truncation bound and vector products equal
//!   the action's, it agrees with PY03's `plan_joint_phi`, and it refuses
//!   what the action refuses. The generator reproduces R-NEXT-04's cases.
//! - `plan_only` (ignored; registered command 1) plans the 32 registered
//!   stage-2 cases in both bases at total_budget / 4 and writes `PY02_PLANS`
//!   (compact JSON) for `tools/py02_laguerre_admission.py`. It executes no
//!   recurrence, coefficient table, envelope or admission; `||F||` (for the
//!   registered budget `factor max(||F||, 1)`) comes from the dense
//!   augmented exponential and is checked at 50 digits by the checker.
//! - `fixture_format_plans` (ignored; not registered) writes the same record
//!   format for fresh fixtures to `PY02_FIXTURE_PLANS`, to check the checker's
//!   schema before the recorded run.
//!
//! Without their path variables both ignored tests write nothing.

use rodas5p_core::{
    DenseMatrix, WorkCounters, dense_phi_combination,
    polynomial_action::{
        EnclosureEvidence, JointPhiInput, LAGUERRE_SCALES, PlanBranch, PlanOnlyBasis,
        PlanOnlyComparison, PolynomialBasis, PolynomialCaches, RouteCost,
        SymmetricNonpositiveOperator, joint_phi_action, plan_joint_phi, plan_only_comparison,
    },
};
use serde_json::{Value, json};

const FIRST_SEED: u64 = 9001;
const GRID_N: [usize; 2] = [7, 14];
const GRID_RHO: [f64; 2] = [100.0, 300.0];
const GRID_H: [f64; 2] = [0.08, 0.16];
const BUDGET_FACTORS: [f64; 2] = [1.0e-6, 1.0e-8];
const INPUT_SCALES: [f64; 5] = [1.0, -0.5, 0.25, 2.0, -1.0];
const TRUNCATION_FRACTION: f64 = 0.25;
const A3_THRESHOLD: u64 = 8;

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hex_vec(values: &[f64]) -> Vec<String> {
    values.iter().map(|x| hex(*x)).collect()
}

// ---------------------------------------------------------------------------
// The R-NEXT-04 generator (crates/rodas5p-core/tests/rnext04_laguerre_admission.rs)
// ---------------------------------------------------------------------------

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

/// Diagonally dominant symmetric nonpositive matrix: `a_ii = -d_i` with
/// `d_i in [0.3 rho, rho]`, symmetric off-diagonal entries whose row sums
/// stay below `0.2 d_i`, so the Gershgorin discs lie in `[-1.2 rho, 0)`.
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
            let cap = 0.2 * d[i].min(d[j]) / n as f64;
            let v = cap * raw[i * n + j];
            a[(i, j)] = v;
            a[(j, i)] = v;
        }
    }
    a
}

fn operator(a: DenseMatrix, rho: f64) -> SymmetricNonpositiveOperator {
    SymmetricNonpositiveOperator::new(a, 0.0, 1.2 * rho * (1.0 + 1.0e-12), "test").unwrap()
}

fn distinct(n: usize, seed: u64, amplitude: f64) -> [Vec<f64>; 5] {
    std::array::from_fn(|k| {
        splitmix(seed + 77 * k as u64, n)
            .iter()
            .map(|v| amplitude * v)
            .collect()
    })
}

fn same_vector(n: usize, seed: u64) -> ([Vec<f64>; 5], Vec<f64>, [f64; 5]) {
    let v = splitmix(seed, n);
    let w = std::array::from_fn(|k| v.iter().map(|x| INPUT_SCALES[k] * x).collect());
    (w, v, INPUT_SCALES)
}

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

struct Spec {
    n: usize,
    rho: f64,
    h: f64,
    same: bool,
    factor: f64,
    seed: u64,
}

impl Spec {
    fn label(&self) -> String {
        format!(
            "n{}-rho{}-h{}-{}-b{:e}",
            self.n,
            self.rho,
            self.h,
            if self.same { "same" } else { "distinct" },
            self.factor
        )
    }

    fn input_seed(&self) -> u64 {
        if self.same {
            self.seed + 5
        } else {
            self.seed + 9
        }
    }
}

/// The grid in export order: one operator seed per (n, rho, h), from
/// `first_seed` up; both inputs and both budget factors share it.
fn grid(first_seed: u64, ns: &[usize], rhos: &[f64], hs: &[f64]) -> Vec<Spec> {
    let mut out = Vec::new();
    let mut seed = first_seed;
    for &n in ns {
        for &rho in rhos {
            for &h in hs {
                for same in [false, true] {
                    for factor in BUDGET_FACTORS {
                        out.push(Spec {
                            n,
                            rho,
                            h,
                            same,
                            factor,
                            seed,
                        });
                    }
                }
                seed += 1;
            }
        }
    }
    out
}

fn basis_json(plan: &Option<Result<PlanOnlyBasis, String>>) -> Value {
    match plan {
        Some(Ok(p)) => json!({
            "degree": p.degree,
            "laguerre_scale": p.laguerre_scale.map(hex),
            "truncation_bound": hex(p.truncation_bound),
            "vector_products": p.vector_products,
            "tail_evaluations": p.tail_evaluations,
            "error": null,
        }),
        Some(Err(error)) => json!({
            "degree": null,
            "laguerre_scale": null,
            "truncation_bound": null,
            "vector_products": null,
            "tail_evaluations": null,
            "error": error,
        }),
        None => Value::Null,
    }
}

fn comparison_json(c: &PlanOnlyComparison) -> Value {
    json!({
        "branch": c.branch,
        "normalization_shift": c.normalization_shift,
        "width": c.width,
        "weight_factor": hex(c.weight_factor),
        "core_budget": hex(c.core_budget),
        "chebyshev": basis_json(&c.chebyshev),
        "laguerre": basis_json(&c.laguerre),
    })
}

/// PY03's planner on the same inputs (reported parity only).
fn py03_plan(
    op: &SymmetricNonpositiveOperator,
    h: f64,
    input: JointPhiInput<'_>,
    basis: PolynomialBasis,
    truncation_budget: f64,
) -> Value {
    let caches = PolynomialCaches::default();
    let mut cost = RouteCost::default();
    match plan_joint_phi(op, h, input, basis, truncation_budget, &caches, &mut cost) {
        Ok(plan) => json!({
            "degree": plan.degree(),
            "laguerre_scale": plan.laguerre_scale().map(hex),
            "vector_products": plan.predicted_vector_products(),
            "tail_evaluations": plan.tail_evaluations(),
            "error": null,
        }),
        Err(error) => json!({
            "degree": null,
            "laguerre_scale": null,
            "vector_products": null,
            "tail_evaluations": null,
            "error": error.to_string(),
        }),
    }
}

fn parity_equal(local: &Value, py03: &Value) -> bool {
    if local.is_null() {
        return false;
    }
    [
        "degree",
        "laguerre_scale",
        "vector_products",
        "tail_evaluations",
    ]
    .iter()
    .all(|k| local[k] == py03[k])
        && local["error"].is_null() == py03["error"].is_null()
}

/// One plan-only record: the inputs, `||F||` from the dense augmented
/// exponential, the budgets, the local helper's plans of both bases and the
/// PY03 parity.
fn plan_record(spec: &Spec) -> Value {
    let n = spec.n;
    let a = matrix(n, spec.rho, spec.seed);
    let op = operator(a.clone(), spec.rho);
    let verified = matches!(op.enclosure().evidence, EnclosureEvidence::Gershgorin);
    let (w, vector, scales) = if spec.same {
        let (w, v, scales) = same_vector(n, spec.input_seed());
        (w, Some(v), Some(scales))
    } else {
        (distinct(n, spec.input_seed(), 1.0), None, None)
    };
    let fused = dense_phi_combination(&a, spec.h, &w).expect("dense reference of ||F||");
    let norm_f = fused.iter().map(|x| x * x).sum::<f64>().sqrt();
    let total_budget = spec.factor * norm_f.max(1.0);
    let truncation_budget = total_budget * TRUNCATION_FRACTION;
    let input = match (&vector, scales) {
        (Some(v), Some(scales)) => JointPhiInput::SameVector { vector: v, scales },
        _ => JointPhiInput::Distinct(&w),
    };
    let comparison =
        plan_only_comparison(&op, spec.h, input, truncation_budget).expect("plan-only comparison");
    let plan = comparison_json(&comparison);
    let cheb = py03_plan(
        &op,
        spec.h,
        input,
        PolynomialBasis::Chebyshev,
        truncation_budget,
    );
    let lag = py03_plan(
        &op,
        spec.h,
        input,
        PolynomialBasis::Laguerre,
        truncation_budget,
    );
    let equal = parity_equal(&plan["chebyshev"], &cheb) && parity_equal(&plan["laguerre"], &lag);
    let enclosure = op.enclosure();
    json!({
        "label": spec.label(),
        "n": n,
        "rho_nominal": hex(spec.rho),
        "h": hex(spec.h),
        "input": if spec.same { "same" } else { "distinct" },
        "budget_factor": hex(spec.factor),
        "seed": spec.seed,
        "input_seed": spec.input_seed(),
        "a": (0..n).map(|i| (0..n).map(|j| hex(a[(i, j)])).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "w": w.iter().map(|v| hex_vec(v)).collect::<Vec<_>>(),
        "vector": vector.as_ref().map(|v| hex_vec(v)),
        "scales": scales.map(|s| hex_vec(&s)),
        "enclosure": {
            "lambda": hex(enclosure.lambda),
            "rho": hex(enclosure.rho),
            "verified": verified,
        },
        "norm_f": hex(norm_f),
        "total_budget": hex(total_budget),
        "truncation_budget": hex(truncation_budget),
        "plan": plan,
        "parity_reported": {"chebyshev": cheb, "laguerre": lag, "equal": equal},
    })
}

fn git(args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8(out.stdout).unwrap()
}

fn write_plans(output: &str, specs: &[Spec], first_seed: u64) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out_path = root.join(output);
    assert!(!out_path.exists(), "immutable output exists: {output}");
    let source_commit = git(&["rev-parse", "HEAD"]).trim().to_string();
    let tree_status = git(&["status", "--porcelain"]);
    let cases = specs.iter().map(plan_record).collect::<Vec<_>>();
    let document = json!({
        "schema": "vigilode-py02-laguerre-plans-v1",
        "node": "research/py02_laguerre_adjoint_bound_20261011",
        "source_commit": source_commit,
        "tree_status": tree_status,
        "constants": {
            "first_seed": first_seed,
            "truncation_fraction": TRUNCATION_FRACTION,
            "laguerre_scales": hex_vec(&LAGUERRE_SCALES),
            "budget_factors": hex_vec(&BUDGET_FACTORS),
            "input_scales": hex_vec(&INPUT_SCALES),
            "a3_threshold": A3_THRESHOLD,
        },
        "cases": cases,
    });
    std::fs::write(&out_path, serde_json::to_string(&document).unwrap() + "\n").unwrap();
    println!("wrote {output} ({} cases)", specs.len());
}

/// Registered command 1: the plan-only export of the 32 stage-2 cases.
#[test]
#[ignore = "recorded run of research node PY02 (stage 1); writes PY02_PLANS when set"]
fn plan_only() {
    let Ok(output) = std::env::var("PY02_PLANS") else {
        println!("PY02_PLANS not set: plans not written");
        return;
    };
    let specs = grid(FIRST_SEED, &GRID_N, &GRID_RHO, &GRID_H);
    assert_eq!(specs.len(), 32);
    write_plans(&output, &specs, FIRST_SEED);
}

/// Not registered: the record format on fresh fixtures (n = 5, seeds from
/// 70001, no stage-2 case), for the checker's schema before the recorded run.
#[test]
#[ignore = "format check on fresh fixtures; writes PY02_FIXTURE_PLANS when set"]
fn fixture_format_plans() {
    let Ok(output) = std::env::var("PY02_FIXTURE_PLANS") else {
        println!("PY02_FIXTURE_PLANS not set: nothing written");
        return;
    };
    let specs = grid(70_001, &[5], &[100.0], &[0.08]);
    write_plans(&output, &specs, 70_001);
}

// ---------------------------------------------------------------------------
// Contract tests on fresh fixtures (no stage-2 case)
// ---------------------------------------------------------------------------

/// Fresh fixtures: n = 5, rho in {50, 400}, h in {0.01, 0.05, 0.2}, both
/// inputs, seeds from 60001; and the two normalization amplitudes.
fn fixtures() -> Vec<(SymmetricNonpositiveOperator, f64, [Vec<f64>; 5], bool)> {
    let mut out = Vec::new();
    let mut seed = 60_001;
    for rho in [50.0, 400.0] {
        for h in [0.01, 0.05, 0.2] {
            let op = operator(matrix(5, rho, seed), rho);
            out.push((op.clone(), h, distinct(5, seed + 9, 1.0), false));
            out.push((op, h, same_vector(5, seed + 5).0, true));
            seed += 1;
        }
    }
    for amplitude in [1.0e-310, 1.0e300] {
        let op = operator(matrix(5, 50.0, seed), 50.0);
        out.push((op, 0.02, distinct(5, seed + 9, amplitude), false));
        seed += 1;
    }
    out
}

fn input_of<'a>(w: &'a [Vec<f64>; 5], same: bool) -> JointPhiInput<'a> {
    if same {
        // w_0 = v (scale 1).
        JointPhiInput::SameVector {
            vector: &w[0],
            scales: INPUT_SCALES,
        }
    } else {
        JointPhiInput::Distinct(w)
    }
}

fn budget_for(w: &[Vec<f64>; 5]) -> f64 {
    let max = w.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
    1.0e-8 * max
}

#[test]
fn helper_matches_action_degrees() {
    for (op, h, w, same) in fixtures() {
        let budget = budget_for(&w);
        let comparison = plan_only_comparison(&op, h, input_of(&w, same), budget).unwrap();
        assert_eq!(comparison.branch, PlanBranch::Recurrence);
        assert_eq!(comparison.width, if same { 1 } else { 5 });
        for (basis, plan) in [
            (PolynomialBasis::Chebyshev, &comparison.chebyshev),
            (PolynomialBasis::Laguerre, &comparison.laguerre),
        ] {
            let plan = plan.clone().unwrap().unwrap();
            assert_eq!(plan.basis, basis);
            let mut work = WorkCounters::default();
            let report =
                joint_phi_action(&op, h, input_of(&w, same), basis, budget, None, &mut work)
                    .unwrap();
            assert_eq!(report.degree, plan.degree, "{basis:?} h {h}");
            assert_eq!(
                report.laguerre_scale.map(f64::to_bits),
                plan.laguerre_scale.map(f64::to_bits)
            );
            assert_eq!(report.normalization_shift, comparison.normalization_shift);
            assert_eq!(work.poly_vector_products, plan.vector_products);
            assert!(plan.truncation_bound <= comparison.core_budget);
            if comparison.normalization_shift == 0 {
                assert_eq!(
                    report.truncation_bound_exact_arithmetic.to_bits(),
                    plan.truncation_bound.to_bits()
                );
                assert_eq!(comparison.core_budget.to_bits(), budget.to_bits());
            } else {
                assert!(comparison.core_budget != budget);
            }
        }
    }
}

#[test]
fn helper_agrees_with_py03_planner() {
    for (op, h, w, same) in fixtures() {
        let budget = budget_for(&w);
        let comparison = plan_only_comparison(&op, h, input_of(&w, same), budget).unwrap();
        let plan = comparison_json(&comparison);
        for (key, basis) in [
            ("chebyshev", PolynomialBasis::Chebyshev),
            ("laguerre", PolynomialBasis::Laguerre),
        ] {
            let py03 = py03_plan(&op, h, input_of(&w, same), basis, budget);
            assert!(parity_equal(&plan[key], &py03), "{key}: {plan} vs {py03}");
        }
    }
}

#[test]
fn helper_refuses_what_the_action_refuses() {
    let op = operator(matrix(5, 50.0, 60_101), 50.0);
    let w = distinct(5, 60_102, 1.0);
    for budget in [f64::NAN, -1.0, 0.0, f64::INFINITY] {
        assert!(plan_only_comparison(&op, 0.1, JointPhiInput::Distinct(&w), budget).is_err());
    }
    for h in [f64::NAN, -0.1, f64::INFINITY] {
        assert!(plan_only_comparison(&op, h, JointPhiInput::Distinct(&w), 1e-8).is_err());
    }
    let short = [
        vec![1.0; 4],
        vec![1.0; 5],
        vec![1.0; 5],
        vec![1.0; 5],
        vec![1.0; 5],
    ];
    assert!(plan_only_comparison(&op, 0.1, JointPhiInput::Distinct(&short), 1e-8).is_err());
    let mut bad = w.clone();
    bad[2][1] = f64::NAN;
    assert!(plan_only_comparison(&op, 0.1, JointPhiInput::Distinct(&bad), 1e-8).is_err());
    // Zero input and the scalar branch plan no recurrence.
    let zero: [Vec<f64>; 5] = std::array::from_fn(|_| vec![0.0; 5]);
    let c = plan_only_comparison(&op, 0.1, JointPhiInput::Distinct(&zero), 1e-8).unwrap();
    assert_eq!(c.branch, PlanBranch::ZeroInput);
    assert!(c.chebyshev.is_none() && c.laguerre.is_none());
    let c = plan_only_comparison(&op, 0.0, JointPhiInput::Distinct(&w), 1e-8).unwrap();
    assert_eq!(c.branch, PlanBranch::Scalar);
    let mut scalar = DenseMatrix::zeros(5, 5);
    for i in 0..5 {
        scalar[(i, i)] = -2.0;
    }
    let scalar_op = SymmetricNonpositiveOperator::new(scalar, 2.0, 2.0, "test").unwrap();
    let c = plan_only_comparison(&scalar_op, 0.1, JointPhiInput::Distinct(&w), 1e-8).unwrap();
    assert_eq!(c.branch, PlanBranch::Scalar);
    assert!(c.chebyshev.is_none() && c.laguerre.is_none());
}

/// The generator is R-NEXT-04's: its first case (n = 6, rho = 1, seed 2)
/// is reproduced bit for bit from the committed cases.json.
#[test]
fn generator_reproduces_rnext04() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/rnext04_laguerre_admission_20261003/cases.json"
    );
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let case = &doc["cases"][0];
    assert_eq!(case["label"], "n6-rho1-h0.001-distinct");
    let a = matrix(6, 1.0, 2);
    for i in 0..6 {
        for j in 0..6 {
            assert_eq!(case["a"][i][j], hex(a[(i, j)]));
        }
    }
    let w = distinct(6, 2 + 9, 1.0);
    for (k, v) in w.iter().enumerate() {
        assert_eq!(case["w"][k], json!(hex_vec(v)));
    }
    let same = &doc["cases"][1];
    assert_eq!(same["label"], "n6-rho1-h0.001-same");
    let (w, _, _) = same_vector(6, 2 + 5);
    for (k, v) in w.iter().enumerate() {
        assert_eq!(same["w"][k], json!(hex_vec(v)));
    }
}

/// The stage-2 grid: 32 labels, unique, seeds from 9001, disjoint (n, rho, h)
/// from R-NEXT-04, PP08 and PY03, and every operator verified.
#[test]
fn stage_two_grid_is_registered() {
    let specs = grid(FIRST_SEED, &GRID_N, &GRID_RHO, &GRID_H);
    assert_eq!(specs.len(), 32);
    let labels: std::collections::BTreeSet<_> = specs.iter().map(Spec::label).collect();
    assert_eq!(labels.len(), 32);
    assert!(labels.contains("n7-rho100-h0.08-distinct-b1e-6"));
    assert!(labels.contains("n14-rho300-h0.16-same-b1e-8"));
    assert_eq!(specs[0].seed, 9001);
    assert_eq!(specs[31].seed, 9008);
    for s in &specs {
        assert!(![6, 12, 20, 5, 16, 8, 24].contains(&s.n));
        // Verified enclosure, without planning anything.
        let op = operator(matrix(s.n, s.rho, s.seed), s.rho);
        assert!(matches!(
            op.enclosure().evidence,
            EnclosureEvidence::Gershgorin
        ));
    }
}
