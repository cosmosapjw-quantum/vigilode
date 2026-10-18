//! Holdout test suite and stage-1 runner for certified Newton-Leja phi actions (PY05).
//!
//! Preregistration: research/py05_leja_total_certificate_20261011/PREREGISTRATION.md
//!
//! Non-ignored tests run fast unit/smoke verification.
//! The ignored `export_runs` test evaluates the 40 registered Stage 1 cells across
//! 4 arms (leja-cert, cheb-cert, leja-est, router) and 2 cache states (cold, warm)
//! to emit 320 records to the file specified in `PY05_RUNS`.

use std::path::Path;

use rodas5p_core::{
    DenseMatrix, WorkCounters,
    leja_action::{
        CertifiedLejaCoeffCache, LEJA_DEGREE_CAP, leja_phi_action, leja_phi_action_certified,
        leja_phi_action_certified_with_cache,
    },
    polynomial_action::{
        CoefficientCache, JointPhiInput, PolynomialBasis, SymmetricNonpositiveOperator,
        TotalErrorStatus, joint_phi_action, route_joint_phi,
    },
};
use serde_json::{Value, json};

fn hex(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hex_vec(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

fn unhex(s: &str) -> f64 {
    let bits = u64::from_str_radix(s, 16).expect("valid hex64");
    f64::from_bits(bits)
}

fn unhex_vec(v: &[Value]) -> Vec<f64> {
    v.iter()
        .map(|s| unhex(s.as_str().expect("string in hex vector")))
        .collect()
}

// -----------------------------------------------------------------------------
// Smoke Test
// -----------------------------------------------------------------------------

#[test]
fn test_leja_holdout_smoke() {
    let n = 8;
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -5.0 * (i + 1) as f64;
    }
    let op = SymmetricNonpositiveOperator::gershgorin(a).expect("valid gershgorin");
    let h = 0.05;
    let w_vecs: [Vec<f64>; 5] = std::array::from_fn(|k| vec![(k + 1) as f64; n]);
    let input = JointPhiInput::Distinct(&w_vecs);
    let budget = 1.0e-6;

    let mut work_cert = WorkCounters::default();
    let rep_cert = leja_phi_action_certified(&op, h, input, budget, &mut work_cert)
        .expect("certified action succeeds");
    assert!(rep_cert.converged());
    assert!(rep_cert.total_bound() <= budget);
    assert_eq!(rep_cert.fused().len(), n);

    let mut work_cheb = WorkCounters::default();
    let rep_cheb = joint_phi_action(
        &op,
        h,
        input,
        PolynomialBasis::Chebyshev,
        budget,
        None,
        &mut work_cheb,
    )
    .expect("chebyshev action succeeds");
    assert_eq!(rep_cheb.fused.len(), n);

    let mut work_est = WorkCounters::default();
    let rep_est =
        leja_phi_action(&op, h, input, budget, &mut work_est).expect("estimate action succeeds");
    assert_eq!(rep_est.fused.len(), n);

    let mut work_router = WorkCounters::default();
    let rep_router =
        route_joint_phi(&op, h, input, budget, &mut work_router).expect("router action succeeds");
    assert_eq!(rep_router.fused().unwrap().len(), n);
}

// -----------------------------------------------------------------------------
// Stage 1 Holdout Exporter
// -----------------------------------------------------------------------------

#[test]
#[ignore = "exporter of research/py05_leja_total_certificate_20261011; set PY05_CASES and PY05_RUNS"]
fn export_runs() {
    let cases_var = match std::env::var("PY05_CASES") {
        Ok(v) => v,
        Err(_) => {
            println!("PY05_CASES not set; skipping export_runs");
            return;
        }
    };
    let runs_var = match std::env::var("PY05_RUNS") {
        Ok(v) => v,
        Err(_) => {
            println!("PY05_RUNS not set; skipping export_runs");
            return;
        }
    };

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cases_path = root.join(&cases_var);
    let runs_path = root.join(&runs_var);

    assert!(
        cases_path.exists(),
        "Cases file must exist: {:?}",
        cases_path
    );
    if runs_path.exists() {
        panic!("Immutable output already exists: {:?}", runs_path);
    }

    let cases_text = std::fs::read_to_string(&cases_path).expect("read cases file");
    let cases_doc: Value = serde_json::from_str(&cases_text).expect("valid cases json");
    let cells = cases_doc["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 40, "Must contain exactly 40 cells");

    // Check git tree clean
    let git_status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&root)
        .output()
        .expect("git status");
    let status_str = String::from_utf8_lossy(&git_status.stdout);
    let runs_norm = runs_path.file_name().unwrap().to_str().unwrap();
    let tree_clean = status_str.lines().all(|l| {
        let t = l.trim();
        t.is_empty() || t.ends_with(runs_norm)
    });

    let git_commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .expect("git rev-parse HEAD");
    let source_commit = String::from_utf8_lossy(&git_commit.stdout)
        .trim()
        .to_string();

    let mut rows = Vec::with_capacity(320);

    for cell in cells {
        let cell_id = cell["cell_id"].as_str().expect("cell_id").to_string();
        let n = cell["n"].as_u64().expect("n") as usize;
        let h = unhex(cell["h_hex"].as_str().expect("h_hex"));
        let budget = unhex(cell["budget_hex"].as_str().expect("budget_hex"));
        let is_niche = cell["is_niche"].as_bool().unwrap_or(false);

        let a_raw: Vec<Vec<f64>> = cell["a"]
            .as_array()
            .expect("matrix a")
            .iter()
            .map(|row| unhex_vec(row.as_array().expect("row")))
            .collect();

        let mut a = DenseMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                a[(i, j)] = a_raw[i][j];
            }
        }
        let op = SymmetricNonpositiveOperator::gershgorin(a).expect("Gershgorin verified operator");

        let w_raw: Vec<Vec<f64>> = cell["w"]
            .as_array()
            .expect("w vectors")
            .iter()
            .map(|col| unhex_vec(col.as_array().expect("w col")))
            .collect();
        let w_arr: [Vec<f64>; 5] = [
            w_raw[0].clone(),
            w_raw[1].clone(),
            w_raw[2].clone(),
            w_raw[3].clone(),
            w_raw[4].clone(),
        ];

        let scales = [1.0, -0.5, 0.25, 2.0, -1.0];
        let input_kind = cell["input"].as_str().expect("input");
        let input = match input_kind {
            "distinct" => JointPhiInput::Distinct(&w_arr),
            "same" => JointPhiInput::SameVector {
                vector: &w_raw[0],
                scales,
            },
            other => panic!("unknown input kind: {}", other),
        };

        // Arms: leja-cert, cheb-cert, leja-est, router
        // Caches: cold, warm
        for arm in ["leja-cert", "cheb-cert", "leja-est", "router"] {
            // Warm cache setup
            let mut leja_cache = CertifiedLejaCoeffCache::new();
            let mut cheb_cache = CoefficientCache::default();

            if arm == "leja-cert" {
                let mut dummy_work = WorkCounters::default();
                let _ = leja_phi_action_certified_with_cache(
                    &op,
                    h,
                    input,
                    budget,
                    LEJA_DEGREE_CAP,
                    Some(&mut leja_cache),
                    &mut dummy_work,
                );
            } else if arm == "cheb-cert" {
                let mut dummy_work = WorkCounters::default();
                let _ = joint_phi_action(
                    &op,
                    h,
                    input,
                    PolynomialBasis::Chebyshev,
                    budget,
                    Some(&mut cheb_cache),
                    &mut dummy_work,
                );
            }

            for cache_state in ["cold", "warm"] {
                let mut work = WorkCounters::default();

                let (
                    admitted,
                    status_str,
                    fused,
                    total_bound,
                    degree,
                    vector_products,
                    failed_attempts,
                ) = match arm {
                    "leja-cert" => {
                        let res = if cache_state == "cold" {
                            let mut fresh_cache = CertifiedLejaCoeffCache::new();
                            leja_phi_action_certified_with_cache(
                                &op,
                                h,
                                input,
                                budget,
                                LEJA_DEGREE_CAP,
                                Some(&mut fresh_cache),
                                &mut work,
                            )
                        } else {
                            leja_phi_action_certified_with_cache(
                                &op,
                                h,
                                input,
                                budget,
                                LEJA_DEGREE_CAP,
                                Some(&mut leja_cache),
                                &mut work,
                            )
                        };
                        match res {
                            Ok(rep) => {
                                let adm = rep.converged() && rep.total_bound() <= budget;
                                (
                                    adm,
                                    "certified",
                                    // This holdout harness owns a row snapshot; copying the
                                    // immutable report slice is diagnostic setup work, not a
                                    // solver hot-loop or performance-parity measurement.
                                    rep.fused().to_vec(),
                                    Some(rep.total_bound()),
                                    rep.degree(),
                                    rep.vector_products() as usize,
                                    if adm { 0 } else { 1 },
                                )
                            }
                            Err(_) => (
                                false,
                                "failed",
                                vec![0.0; n],
                                None,
                                0,
                                work.poly_vector_products as usize,
                                1,
                            ),
                        }
                    }
                    "cheb-cert" => {
                        let res = if cache_state == "cold" {
                            joint_phi_action(
                                &op,
                                h,
                                input,
                                PolynomialBasis::Chebyshev,
                                budget,
                                None,
                                &mut work,
                            )
                        } else {
                            joint_phi_action(
                                &op,
                                h,
                                input,
                                PolynomialBasis::Chebyshev,
                                budget,
                                Some(&mut cheb_cache),
                                &mut work,
                            )
                        };
                        match res {
                            Ok(rep) => {
                                let (is_cert, bound) = match &rep.total_error {
                                    TotalErrorStatus::Certified { bound } => (true, Some(*bound)),
                                    TotalErrorStatus::EstimateOnly { .. } => (false, None),
                                };
                                let adm = is_cert && bound.is_some_and(|b| b <= budget);
                                (
                                    adm,
                                    if is_cert {
                                        "certified"
                                    } else {
                                        "estimate-only"
                                    },
                                    rep.fused,
                                    bound,
                                    rep.degree,
                                    work.poly_vector_products as usize,
                                    if adm { 0 } else { 1 },
                                )
                            }
                            Err(_) => (
                                false,
                                "failed",
                                vec![0.0; n],
                                None,
                                0,
                                work.poly_vector_products as usize,
                                1,
                            ),
                        }
                    }
                    "leja-est" => {
                        let res = leja_phi_action(&op, h, input, budget, &mut work);
                        match res {
                            Ok(rep) => (
                                rep.converged,
                                "estimate-only",
                                rep.fused,
                                None,
                                rep.degree,
                                rep.vector_products as usize,
                                if rep.converged { 0 } else { 1 },
                            ),
                            Err(_) => (
                                false,
                                "failed",
                                vec![0.0; n],
                                None,
                                0,
                                work.poly_vector_products as usize,
                                1,
                            ),
                        }
                    }
                    "router" => {
                        let res = route_joint_phi(&op, h, input, budget, &mut work);
                        match res {
                            Ok(rep) => {
                                let adm = !rep.is_fallback();
                                let fused = rep
                                    .fused()
                                    .map(|f| f.to_vec())
                                    .unwrap_or_else(|| vec![0.0; n]);
                                let bound = rep.bound();
                                let degree = rep.report().map(|r| r.degree).unwrap_or(0);
                                (
                                    adm,
                                    "routed",
                                    fused,
                                    bound,
                                    degree,
                                    work.poly_vector_products as usize,
                                    if adm { 0 } else { 1 },
                                )
                            }
                            Err(_) => (
                                false,
                                "failed",
                                vec![0.0; n],
                                None,
                                0,
                                work.poly_vector_products as usize,
                                1,
                            ),
                        }
                    }
                    _ => unreachable!(),
                };

                // Cost accounting:
                // Complete cost = P * c_JVP + Ir(setup) + Ir(cert) + Ir(failed)
                // c_JVP = 100 * n
                let p = vector_products;
                let c_jvp = 100 * n;
                let jvp_cost = p * c_jvp;

                let ir_setup = match arm {
                    "leja-cert" => {
                        if cache_state == "cold" {
                            18_000 * degree
                        } else {
                            0
                        }
                    }
                    "cheb-cert" => {
                        if cache_state == "cold" {
                            6_500 * degree
                        } else {
                            0
                        }
                    }
                    "leja-est" => 12_000 * degree,
                    "router" => 7_000 * degree,
                    _ => 0,
                };

                let ir_cert = match arm {
                    "leja-cert" => 3_200 * degree,
                    "cheb-cert" => 350 * degree,
                    "leja-est" => 0,
                    "router" => 400 * degree,
                    _ => 0,
                };

                let ir_failed = failed_attempts * (5_000 * n);
                let complete_cost = jvp_cost + ir_setup + ir_cert + ir_failed;

                let total_bound_hex: Option<String> = total_bound.map(hex);

                rows.push(json!({
                    "cell_id": cell_id,
                    "arm": arm,
                    "cache_state": cache_state,
                    "n": n,
                    "is_niche": is_niche,
                    "admitted": admitted,
                    "status": status_str,
                    "degree": degree,
                    "vector_products": vector_products,
                    "total_bound": total_bound_hex,
                    "total_bound_f64": total_bound,
                    "fused": hex_vec(&fused),
                    "failed_attempts": failed_attempts,
                    "p": p,
                    "c_jvp": c_jvp,
                    "ir_setup": ir_setup,
                    "ir_cert": ir_cert,
                    "ir_failed": ir_failed,
                    "complete_cost": complete_cost,
                    "work": serde_json::to_value(work).unwrap(),
                }));
            }
        }
    }

    assert_eq!(rows.len(), 320, "Must record exactly 320 rows");

    let doc = json!({
        "schema": "vigilode-py05-runs-v1",
        "node": "research/py05_leja_total_certificate_20261011",
        "source_commit": source_commit,
        "tree_clean": tree_clean,
        "rows": rows,
    });

    if let Some(parent) = runs_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(
        &runs_path,
        serde_json::to_string_pretty(&doc).unwrap() + "\n",
    )
    .expect("write RUNS.json");
    println!("Wrote 320 rows to {:?}", runs_path);
}
