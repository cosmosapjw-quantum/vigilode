//! Case generator of research node
//! `research/int05_nonnormal_metric_bound_20261003` (integrated DAG node
//! INT-05): certified bounds for `exp(tau A) v` on the VIG-A02,
//! Jordan-like and central convection-diffusion families, with and without
//! a diagonal metric. `tools/int05_nonnormal_check.py` checks them against
//! 50-digit references. Run with `--ignored --release`.

use rodas5p_core::nonnormal_certificate::certify_exp_action;
use serde_json::{Value, json};

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn hexes(v: &[f64]) -> Vec<String> {
    v.iter().map(|x| hex(*x)).collect()
}

struct Family {
    label: String,
    a: Vec<Vec<f64>>,
    v: Vec<f64>,
    tau: f64,
    metric: Vec<f64>,
    arnoldi_candidate: Option<Vec<f64>>,
}

fn families() -> Vec<Family> {
    let mut out = Vec::new();
    for k in [0i32, 10, 20, 46] {
        out.push(Family {
            label: format!("vig-a02-k{k}"),
            a: vec![vec![-2.0, 2.0_f64.powi(k)], vec![2.0_f64.powi(-k), -2.0]],
            v: vec![1.0, 0.0],
            tau: 1.0,
            metric: vec![1.0, 2.0_f64.powi(k)],
            arnoldi_candidate: Some(vec![(-2.0_f64).exp(), 0.0]),
        });
    }
    for mu in [1.0_f64, 10.0, 100.0] {
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
        let mut metric = vec![1.0];
        for i in 1..n {
            metric.push(metric[i - 1] * mu / 0.5);
        }
        out.push(Family {
            label: format!("jordan-8-mu{mu}"),
            a,
            v: vec![1.0; n],
            tau: 1.0,
            metric,
            arnoldi_candidate: None,
        });
    }
    for pe in [10.0_f64, 50.0] {
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
        let x: Vec<f64> = (0..n).map(|i| (i + 1) as f64 * h).collect();
        out.push(Family {
            label: format!("convection-diffusion-32-pe{pe}"),
            a,
            v: x.iter()
                .map(|xi| (std::f64::consts::PI * xi).sin())
                .collect(),
            tau: 1.0e-3,
            metric: x.iter().map(|xi| (-pe * xi / 2.0).exp()).collect(),
            arnoldi_candidate: None,
        });
    }
    out
}

#[test]
#[ignore = "case generator of research/int05_nonnormal_metric_bound_20261003; release build"]
fn nonnormal_metric_bound_cases() {
    let mut cases = Vec::new();
    for family in families() {
        for (metric_label, metric) in [
            ("identity", None),
            ("diagonal", Some(family.metric.as_slice())),
        ] {
            for degree in [10usize, 20, 30] {
                let mut candidates: Vec<(&str, Option<&[f64]>)> = vec![("own", None)];
                if let Some(x) = &family.arnoldi_candidate {
                    candidates.push(("arnoldi-near-breakdown", Some(x.as_slice())));
                }
                for (kind, candidate) in candidates {
                    let cert = certify_exp_action(
                        &family.a, &family.v, family.tau, metric, degree, candidate,
                    )
                    .unwrap();
                    let record: Value = json!({
                        "family": family.label, "metric": metric_label, "degree": degree, "candidate_kind": kind,
                        "a": family.a.iter().map(|row| hexes(row)).collect::<Vec<_>>(),
                        "v": hexes(&family.v), "tau": hex(family.tau),
                        "metric_values": metric.map(hexes),
                        "candidate": hexes(&cert.candidate),
                        "status": cert.status, "error_upper": cert.error_upper, "error_lower": cert.error_lower,
                        "truncation_upper": cert.truncation_upper, "distance_upper": cert.distance_upper,
                        "distance_lower": cert.distance_lower, "transport": cert.transport,
                        "numerical_range": cert.numerical_range,
                    });
                    println!(
                        "{} {} m={} {}: {:?} upper {:e} lower {:e}",
                        family.label,
                        metric_label,
                        degree,
                        kind,
                        cert.status,
                        cert.error_upper,
                        cert.error_lower
                    );
                    cases.push(record);
                }
            }
        }
    }
    let path = std::env::var("INT05_CASES").expect("INT05_CASES");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = root.join(path);
    std::fs::write(
        &target,
        serde_json::to_string_pretty(&json!({"schema": "vigilode-int05-cases-v1", "cases": cases}))
            .unwrap()
            + "\n",
    )
    .unwrap();
    println!("wrote {}", target.display());
}
