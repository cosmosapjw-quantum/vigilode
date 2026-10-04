//! Research node `research/pp12b_chain_symmetrizer_20261004` (PP12b): the
//! log-norm certificate with identity, Osborne and chain-symmetrizer metrics
//! on the REV-02 families (generator verbatim) and a new holdout family,
//! exported for `tools/pp12b_chain_check.py`.

use rodas5p_core::nonnormal_certificate::{
    LognormCertificate, LognormMetric, certify_exp_action_lognorm_auto3, chain_symmetrizer_metric,
    osborne_metric,
};
use serde_json::{Value, json};

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

fn splitmix(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
}

struct Family {
    group: &'static str,
    label: String,
    a: Vec<Vec<f64>>,
    v: Vec<f64>,
    tau: f64,
}

fn families() -> Vec<Family> {
    let mut out = Vec::new();
    for k in [10i32, 46] {
        out.push(Family {
            group: "F1",
            label: format!("vig-a02-k{k}"),
            a: vec![vec![-2.0, 2.0_f64.powi(k)], vec![2.0_f64.powi(-k), -2.0]],
            v: vec![1.0, 0.0],
            tau: 1.0,
        });
    }
    for pe in [10.0_f64, 50.0, 200.0] {
        for tau in [1.0e-2, 1.0e-1] {
            let n = 32;
            let h = 1.0 / (n as f64 + 1.0);
            let a = (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| {
                            if i == j {
                                -2.0 / (h * h)
                            } else if j + 1 == i {
                                1.0 / (h * h) + pe / (2.0 * h)
                            } else if j == i + 1 {
                                1.0 / (h * h) - pe / (2.0 * h)
                            } else {
                                0.0
                            }
                        })
                        .collect()
                })
                .collect();
            out.push(Family {
                group: "F2",
                label: format!("cd-central-32-pe{pe}-tau{tau}"),
                a,
                v: (0..n)
                    .map(|i| (std::f64::consts::PI * (i + 1) as f64 * h).sin())
                    .collect(),
                tau,
            });
        }
    }
    for mu in [1.0_f64, 30.0] {
        for seed in 1..=4u64 {
            let n = 16;
            let mut state = seed;
            let mut a = vec![vec![0.0; n]; n];
            for (i, row) in a.iter_mut().enumerate() {
                row[i] = -(10.0_f64.powf(2.0 * i as f64 / 15.0));
                for value in row.iter_mut().skip(i + 1) {
                    *value = mu * splitmix(&mut state);
                }
            }
            out.push(Family {
                group: "F3",
                label: format!("random-16-mu{mu}-seed{seed}"),
                a,
                v: vec![1.0; n],
                tau: 0.5,
            });
        }
    }
    for mu in [10.0_f64, 100.0] {
        let n = 8;
        let a = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        if i == j {
                            -1.0
                        } else if j == i + 1 {
                            mu
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect();
        out.push(Family {
            group: "F4",
            label: format!("jordan-8-mu{mu}"),
            a,
            v: vec![1.0; n],
            tau: 1.0,
        });
    }
    out
}

/// The holdout family of PP12b: central differences (n 24/48, Pe 100/400,
/// tau 0.05/0.2) and first-order upwind (n 32, Pe 50/200, tau 0.01/0.1).
fn holdout() -> Vec<Family> {
    let mut out = Vec::new();
    for n in [24usize, 48] {
        for pe in [100.0_f64, 400.0] {
            for tau in [0.05, 0.2] {
                let h = 1.0 / (n as f64 + 1.0);
                let a = (0..n)
                    .map(|i| {
                        (0..n)
                            .map(|j| {
                                if i == j {
                                    -2.0 / (h * h)
                                } else if j + 1 == i {
                                    1.0 / (h * h) + pe / (2.0 * h)
                                } else if j == i + 1 {
                                    1.0 / (h * h) - pe / (2.0 * h)
                                } else {
                                    0.0
                                }
                            })
                            .collect()
                    })
                    .collect();
                out.push(Family {
                    group: "H-central",
                    label: format!("cd-central-{n}-pe{pe}-tau{tau}"),
                    a,
                    v: (0..n)
                        .map(|i| (std::f64::consts::PI * (i + 1) as f64 * h).sin())
                        .collect(),
                    tau,
                });
            }
        }
    }
    for pe in [50.0_f64, 200.0] {
        for tau in [0.01, 0.1] {
            let n = 32;
            let h = 1.0 / (n as f64 + 1.0);
            let a = (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| {
                            if i == j {
                                -2.0 / (h * h) - pe / h
                            } else if j + 1 == i {
                                1.0 / (h * h) + pe / h
                            } else if j == i + 1 {
                                1.0 / (h * h)
                            } else {
                                0.0
                            }
                        })
                        .collect()
                })
                .collect();
            out.push(Family {
                group: "H-upwind",
                label: format!("cd-upwind-32-pe{pe}-tau{tau}"),
                a,
                v: (0..n)
                    .map(|i| (std::f64::consts::PI * (i + 1) as f64 * h).sin())
                    .collect(),
                tau,
            });
        }
    }
    out
}

fn cert_json(c: &LognormCertificate, metric: &[f64]) -> Value {
    json!({
        "metric": c.metric, "metric_d": hexes(metric), "mu_up": hex(c.mu_up),
        "mu_source": c.mu_source, "gershgorin_re_hi": hex(c.gershgorin_re_hi), "steps": c.steps,
        "status": c.certificate.status, "error_upper": hex(c.certificate.error_upper),
        "transport": hex(c.certificate.transport),
        "candidate": hexes(&c.certificate.candidate),
    })
}

#[test]
fn chain_symmetrizer_makes_pairs_equal() {
    let f = holdout().into_iter().next().unwrap();
    let d = chain_symmetrizer_metric(&f.a).unwrap();
    for i in 1..d.len() {
        let low = d[i] * f.a[i][i - 1] / d[i - 1];
        let up = d[i - 1] * f.a[i - 1][i] / d[i];
        assert!((low.abs() - up.abs()).abs() <= 1e-12 * low.abs());
    }
}

#[test]
#[ignore = "recorded run of research/pp12b_chain_symmetrizer_20261004; release build"]
fn export_chain_cases() {
    let mut cases = Vec::new();
    for f in families().into_iter().chain(holdout()) {
        let osborne = osborne_metric(&f.a).unwrap();
        let chain = chain_symmetrizer_metric(&f.a).unwrap();
        let ones = vec![1.0; f.v.len()];
        let certs = certify_exp_action_lognorm_auto3(&f.a, &f.v, f.tau).unwrap();
        let entries: Vec<Value> = certs
            .iter()
            .map(|c| {
                let d = match c.metric {
                    LognormMetric::Identity => &ones,
                    LognormMetric::Osborne => &osborne,
                    LognormMetric::ChainSymmetrizer => &chain,
                };
                cert_json(c, d)
            })
            .collect();
        cases.push(json!({
            "group": f.group, "label": f.label,
            "a": f.a.iter().map(|r| hexes(r)).collect::<Vec<_>>(),
            "v": hexes(&f.v), "tau": hex(f.tau), "certificates": entries,
        }));
    }
    let out = json!({"schema": "vigilode-pp12b-chain-v1", "cases": cases});
    if let Ok(path) = std::env::var("PP12B_CASES") {
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
