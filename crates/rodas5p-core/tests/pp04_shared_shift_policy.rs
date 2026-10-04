//! Research node `research/pp04_shared_shift_policy_20261004` (RVJ DAG node
//! PP04): contract tests and the recorded export of the four methods on the
//! 90 preregistered configurations. `tools/pp04_shared_shift_check.py`
//! checks the n = 8 candidates against an exact rational oracle and
//! evaluates the gate.

#![allow(clippy::needless_range_loop, clippy::should_implement_trait)]

use rodas5p_core::DenseMatrix;
use rodas5p_core::shared_shift_policy::{
    JetAbstention, SHIFT_METHODS, ShiftMethod, hessenberg_reduce, plan_shift_family,
    solve_shift_family,
};
use serde_json::{Value, json};

pub struct SplitMix(pub u64);

impl SplitMix {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// Uniform in [-1, 1).
    pub fn symmetric(&mut self) -> f64 {
        2.0 * ((self.next() >> 11) as f64 / (1u64 << 53) as f64) - 1.0
    }
}

/// `J = S - diag(sum_k |(S + S^T)/2|_ik + 0.1)`: the symmetric-part row
/// bound is at most -0.1, so the row witness proves dissipativity.
pub fn dissipative(n: usize, seed: u64) -> DenseMatrix {
    let mut rng = SplitMix(seed);
    let s: Vec<f64> = (0..n * n).map(|_| rng.symmetric()).collect();
    let mut j = DenseMatrix::zeros(n, n);
    for i in 0..n {
        let mut row = 0.0;
        for k in 0..n {
            row += (0.5 * (s[i * n + k] + s[k * n + i])).abs();
        }
        for k in 0..n {
            j[(i, k)] = s[i * n + k];
        }
        j[(i, i)] -= row + 0.1;
    }
    j
}

pub fn shift_set(name: &str, m: usize) -> Vec<f64> {
    let t = |i: usize| {
        if m == 1 {
            0.0
        } else {
            -1.0 + 2.0 * i as f64 / (m - 1) as f64
        }
    };
    match name {
        "narrow" => (0..m).map(|i| 0.25 * (1.0 + 0.02 * t(i))).collect(),
        "medium" => (0..m).map(|i| 0.25 * (1.0 + 0.2 * t(i))).collect(),
        "wide" => (0..m)
            .map(|i| 0.05 * 40.0_f64.powf((t(i) + 1.0) / 2.0))
            .collect(),
        "all-equal" => vec![0.25; m],
        "two-cluster" => {
            let half = m / 2;
            let lo = (0..half).map(|i| {
                let ti = if half > 1 {
                    -1.0 + 2.0 * i as f64 / (half - 1) as f64
                } else {
                    0.0
                };
                0.1 * (1.0 + 0.05 * ti)
            });
            let rest = m - half;
            let hi = (0..rest).map(|i| {
                let ti = if rest > 1 {
                    -1.0 + 2.0 * i as f64 / (rest - 1) as f64
                } else {
                    0.0
                };
                1.0 * (1.0 + 0.05 * ti)
            });
            lo.chain(hi).collect()
        }
        _ => unreachable!(),
    }
}

pub const SETS: [&str; 5] = ["narrow", "medium", "wide", "all-equal", "two-cluster"];
pub const H: f64 = 0.1;
pub const TOL: f64 = 1.0e-10;

fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hx(*x)).collect()
}

fn method_name(m: ShiftMethod) -> &'static str {
    match m {
        ShiftMethod::IndividualLu => "individual-lu",
        ShiftMethod::CommonShiftLu => "common-shift-lu",
        ShiftMethod::HessenbergReuse => "hessenberg-reuse",
        ShiftMethod::SharedJet => "shared-jet",
    }
}

#[test]
fn hessenberg_reduction_reconstructs_j() {
    let j = dissipative(12, 7);
    let (q, h) = hessenberg_reduce(&j);
    let n: usize = 12;
    for i in 0..n {
        for k in 0..i.saturating_sub(1) {
            assert_eq!(h[i][k], 0.0);
        }
    }
    for i in 0..n {
        for k in 0..n {
            let mut s = 0.0;
            for a in 0..n {
                for b in 0..n {
                    s += q[i][a] * h[a][b] * q[k][b];
                }
            }
            assert!((s - j[(i, k)]).abs() < 1e-12, "{i} {k}");
        }
    }
}

#[test]
fn every_method_certifies_a_small_family() {
    let j = dissipative(8, 11);
    let mut rng = SplitMix(5);
    let rhs: Vec<Vec<f64>> = (0..2)
        .map(|_| (0..8).map(|_| rng.symmetric()).collect())
        .collect();
    let gammas = shift_set("medium", 5);
    for method in SHIFT_METHODS {
        let result = solve_shift_family(method, &j, H, &rhs, &gammas, TOL).unwrap();
        assert_eq!(result.targets.len(), 5);
        assert!(result.targets.iter().all(|t| t.certified), "{method:?}");
    }
}

#[test]
fn planner_abstains_typed() {
    let plan = plan_shift_family(8, 1, &[0.25; 4], 1.0, TOL).unwrap();
    assert_eq!(plan.jet_abstention, Some(JetAbstention::CommonShift));
    let wide: Vec<f64> = (0..6).map(|i| 0.01 * 10.0_f64.powi(i)).collect();
    let plan = plan_shift_family(8, 1, &wide, 1.0, TOL).unwrap();
    assert_eq!(plan.jet_abstention, Some(JetAbstention::WideCluster));
    assert!(plan_shift_family(8, 1, &[0.25, -1.0], 1.0, TOL).is_err());
    assert!(plan_shift_family(8, 1, &[0.25], 1.0, f64::NAN).is_err());
}

#[test]
#[ignore = "recorded run of research/pp04_shared_shift_policy_20261004; release build"]
fn export_policy_study() {
    let mut configs: Vec<Value> = Vec::new();
    for (ni, n) in [8usize, 32, 96].into_iter().enumerate() {
        let j = dissipative(n, 1000 + ni as u64);
        for r in [1usize, 4] {
            let mut rng = SplitMix(2000 + 10 * n as u64 + r as u64);
            let rhs: Vec<Vec<f64>> = (0..r)
                .map(|_| (0..n).map(|_| rng.symmetric()).collect())
                .collect();
            let max_norm = rhs
                .iter()
                .map(|b| rodas5p_core::safe_l2(b))
                .fold(0.0_f64, f64::max);
            for m in [3usize, 17, 65] {
                for set in SETS {
                    let gammas = shift_set(set, m);
                    let plan = plan_shift_family(n, r, &gammas, max_norm, TOL).unwrap();
                    let mut methods = serde_json::Map::new();
                    for method in SHIFT_METHODS {
                        let result = solve_shift_family(method, &j, H, &rhs, &gammas, TOL);
                        let value = match result {
                            Ok(result) => {
                                let certified =
                                    result.targets.iter().filter(|t| t.certified).count();
                                let mut entry = json!({
                                    "ok": true,
                                    "work": result.work,
                                    "certified_targets": certified,
                                    "targets": result.targets.len(),
                                    "produced_by": result.targets.iter().map(|t| method_name(t.produced_by)).collect::<Vec<_>>(),
                                });
                                if n == 8 {
                                    entry["candidates"] = result
                                        .targets
                                        .iter()
                                        .map(|t| {
                                            json!({
                                                "gamma": hx(t.gamma),
                                                "certified": t.certified,
                                                "columns": t.columns.iter().map(|c| hexes(c)).collect::<Vec<_>>(),
                                                "error_upper": hexes(&t.error_upper),
                                            })
                                        })
                                        .collect();
                                }
                                entry
                            }
                            Err(e) => json!({"ok": false, "error": e.to_string()}),
                        };
                        methods.insert(method_name(method).into(), value);
                    }
                    let mut config = json!({
                        "n": n, "r": r, "m": m, "set": set,
                        "plan": {
                            "predicted_flops": plan.predicted_flops.iter().map(|(k, v)| json!([method_name(*k), v])).collect::<Vec<_>>(),
                            "chosen": method_name(plan.chosen),
                            "jet_abstention": plan.jet_abstention,
                            "high_rank_screen": plan.high_rank_screen,
                            "clusters": plan.clusters,
                        },
                        "methods": methods,
                    });
                    if n == 8 {
                        config["j"] = json!(
                            (0..n)
                                .map(|i| (0..n).map(|k| hx(j[(i, k)])).collect::<Vec<_>>())
                                .collect::<Vec<_>>()
                        );
                        config["h"] = json!(hx(H));
                        config["rhs"] = json!(rhs.iter().map(|b| hexes(b)).collect::<Vec<_>>());
                    }
                    config["gammas"] = json!(hexes(&gammas));
                    configs.push(config);
                }
            }
        }
    }
    let out = json!({"schema": "vigilode-pp04-shared-shift-policy-v1", "tolerance": hx(TOL), "configs": configs});
    if let Ok(path) = std::env::var("PP04_CASES") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}
