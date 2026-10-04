//! Case exporter of research node `research/pp11_taylor_fused_total_20261004`
//! (RVJ DAG node PP11): total-target certificates of the scaled-Taylor fused
//! phi action on the preregistered fixtures. Symmetric and nonsymmetric `A`,
//! n in {4, 8, 16}, `||hA||_1` in {0.5, 4, 20}; for each, seeded `w_k` with
//! `h = 0.1` (not a power of two), the same `w_k` scaled by 1e-310
//! (subnormal) and by 1e100, and a control with `h = 0.125` (a power of
//! two). The candidate is `taylor_phi_action`'s fused output; where that
//! backend rejects the domain, the rejection is recorded and the stepped
//! certificate's own top block is certified instead. Everything is written
//! to `PP11_CASES` (repo-relative, never overwritten) as IEEE-754 hex bits
//! and checked by `tools/pp11_taylor_fused_check.py`. Run with
//! `--ignored --release`. No timings.

use rodas5p_core::DenseMatrix;
use rodas5p_core::taylor_phi::{TaylorPhiStatus, taylor_phi_action};
use rodas5p_core::taylor_phi_total::{FusedPhiCertificate, certify_fused_phi_total};
use serde_json::{Value, json};

/// Truncation budget passed to `taylor_phi_action` (absolute, 2-norm).
const TAYLOR_BUDGET: f64 = 1e-13;

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Exact binary value in [-1, 1).
    fn symmetric(&mut self) -> f64 {
        2.0 * ((self.next() >> 11) as f64 * 2f64.powi(-53)) - 1.0
    }
}

/// Off-diagonal entries uniform in [-1, 1) (mirrored when symmetric), the
/// diagonal `-(r_i + c_i) / 2 - 1` with `r_i`, `c_i` the off-diagonal
/// absolute row and column sums.
#[allow(clippy::needless_range_loop)] // (i, j) and (j, i) are both read
fn base_matrix(n: usize, symmetric: bool, rng: &mut SplitMix) -> Vec<Vec<f64>> {
    let mut a = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            if i == j || (symmetric && j < i) {
                continue;
            }
            a[i][j] = rng.symmetric();
            if symmetric {
                a[j][i] = a[i][j];
            }
        }
    }
    for i in 0..n {
        let row: f64 = (0..n).filter(|&j| j != i).map(|j| a[i][j].abs()).sum();
        let col: f64 = (0..n).filter(|&j| j != i).map(|j| a[j][i].abs()).sum();
        a[i][i] = -(row + col) / 2.0 - 1.0;
    }
    a
}

fn one_norm(a: &[Vec<f64>]) -> f64 {
    let n = a.len();
    (0..n)
        .map(|j| (0..n).map(|i| a[i][j].abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn record(cert: &FusedPhiCertificate) -> Value {
    json!({
        "schema": cert.schema(),
        "status": cert.status(),
        "unbounded_reason": cert.unbounded_reason(),
        "bound": hex(cert.bound()),
        "candidate_distance": hex(cert.candidate_distance()),
        "stepped_error": hex(cert.stepped_error()),
        "perturbation": hex(cert.perturbation()),
        "dominant": cert.dominant(),
        "delta_one_norm": hex(cert.delta_one_norm()),
        "delta_inf_norm": hex(cert.delta_inf_norm()),
        "delta_two_norm": hex(cert.delta_two_norm()),
        "omega": hex(cert.omega()),
        "v_two_norm": hex(cert.v_two_norm()),
        "chosen_metric": cert.chosen_metric(),
        "stepped_candidate": hexes(cert.stepped_candidate()),
        "attempts": cert.attempts().iter().map(|a| json!({
            "metric": a.metric(), "steps": a.steps(), "bounded": a.bounded(),
            "error_upper": hex(a.error_upper()), "note": a.note(),
        })).collect::<Vec<_>>(),
    })
}

#[test]
#[ignore = "case exporter of research/pp11_taylor_fused_total_20261004; release build"]
fn pp11_taylor_fused_total_cases() {
    let Ok(path) = std::env::var("PP11_CASES") else {
        println!("PP11_CASES not set: cases not written");
        return;
    };
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&path);
    assert!(
        !target.exists(),
        "immutable output exists: {}",
        target.display()
    );
    let mut cases = Vec::new();
    for (s, symmetric) in [true, false].into_iter().enumerate() {
        for (ni, n) in [4_usize, 8, 16].into_iter().enumerate() {
            for (ti, norm) in [0.5_f64, 4.0, 20.0].into_iter().enumerate() {
                let seed = 1 + (100 * s + 10 * ni + ti) as u64;
                let mut rng = SplitMix(seed);
                let base = base_matrix(n, symmetric, &mut rng);
                let seeded: [Vec<f64>; 5] =
                    std::array::from_fn(|_| (0..n).map(|_| rng.symmetric()).collect());
                let scaled = |factor: f64| -> [Vec<f64>; 5] {
                    std::array::from_fn(|k| seeded[k].iter().map(|x| x * factor).collect())
                };
                let variants = [
                    ("seeded", 0.1, scaled(1.0)),
                    ("subnormal-w", 0.1, scaled(1e-310)),
                    ("large-w", 0.1, scaled(1e100)),
                    ("pow2-control", 0.125, scaled(1.0)),
                ];
                for (variant, h, w) in variants {
                    let factor = norm / (h * one_norm(&base));
                    let a: Vec<Vec<f64>> = base
                        .iter()
                        .map(|row| row.iter().map(|x| x * factor).collect())
                        .collect();
                    let label = format!(
                        "{}-n{n}-norm{norm}-{variant}",
                        if symmetric { "sym" } else { "nonsym" }
                    );
                    let rows: Vec<&[f64]> = a.iter().map(Vec::as_slice).collect();
                    let dense = DenseMatrix::from_rows(&rows).unwrap();
                    let taylor = taylor_phi_action(&dense, h, &w, TAYLOR_BUDGET);
                    let (taylor_record, candidate, source) = match &taylor {
                        Ok(report) => {
                            let TaylorPhiStatus::EstimateOnly {
                                truncation_bound, ..
                            } = report.status
                            else {
                                panic!("the Taylor backend never certifies");
                            };
                            (
                                json!({
                                    "accepted": true,
                                    "fused": hexes(&report.fused),
                                    "truncation_bound": hex(truncation_bound),
                                    "augmented_one_norm": hex(report.augmented_one_norm),
                                    "degree": report.work.degree,
                                    "scaling_steps": report.work.scaling_steps,
                                }),
                                report.fused.clone(),
                                "taylor_phi_action",
                            )
                        }
                        Err(error) => {
                            // The stepped certificate's own top block.
                            let probe = certify_fused_phi_total(&a, h, &w, &vec![0.0; n]).unwrap();
                            let top = probe.stepped_top().to_vec();
                            let (candidate, source) = if top.iter().all(|x| x.is_finite()) {
                                (top, "stepped_top")
                            } else {
                                (vec![0.0; n], "zeros (stepped top not finite)")
                            };
                            (
                                json!({"accepted": false, "rejection": error.to_string()}),
                                candidate,
                                source,
                            )
                        }
                    };
                    let cert = certify_fused_phi_total(&a, h, &w, &candidate).unwrap();
                    println!(
                        "{label}: taylor {} source {source}: {:?} bound {:e} (distance {:e}, E1 {:e}, perturbation {:e}; {:?}, metric {:?})",
                        taylor.is_ok(),
                        cert.status(),
                        cert.bound(),
                        cert.candidate_distance(),
                        cert.stepped_error(),
                        cert.perturbation(),
                        cert.dominant(),
                        cert.chosen_metric(),
                    );
                    cases.push(json!({
                        "label": label,
                        "symmetric": symmetric,
                        "n": n,
                        "target_hA_one_norm": norm,
                        "variant": variant,
                        "seed": seed,
                        "h": hex(h),
                        "h_power_of_two": h == 0.125,
                        "a": a.iter().map(|row| hexes(row)).collect::<Vec<_>>(),
                        "w": w.iter().map(|wk| hexes(wk)).collect::<Vec<_>>(),
                        "taylor": taylor_record,
                        "candidate_source": source,
                        "candidate": hexes(&candidate),
                        "certificate": record(&cert),
                    }));
                }
            }
        }
    }
    let count = cases.len();
    std::fs::write(
        &target,
        serde_json::to_string_pretty(&json!({
            "schema": "vigilode-pp11-cases-v1",
            "taylor_budget": hex(TAYLOR_BUDGET),
            "cases": cases,
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    println!("wrote {} ({count} cases)", target.display());
}
