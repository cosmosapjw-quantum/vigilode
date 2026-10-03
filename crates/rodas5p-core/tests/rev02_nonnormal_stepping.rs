//! Case generator of research node `research/rev02_nonnormal_stepping_20261003`
//! (review DAG node REV-02): automatic stepped certificates (identity and
//! Osborne metrics) and INT-05 single-step certificates on the F1-F4
//! families, plus directed-exponential grid points. `tools/rev02_nonnormal_check.py`
//! checks them against 50-digit references. Run with `--ignored --release`.

use rodas5p_core::directed::exp_interval;
use rodas5p_core::nonnormal_certificate::{
    NonnormalExpCertificate, certify_exp_action, certify_exp_action_auto, osborne_metric,
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

fn record(cert: &NonnormalExpCertificate) -> Value {
    json!({
        "status": cert.status, "candidate": hexes(&cert.candidate),
        "error_upper": cert.error_upper, "transport": cert.transport,
        "numerical_range": cert.numerical_range,
    })
}

#[test]
#[ignore = "case generator of research/rev02_nonnormal_stepping_20261003; release build"]
fn nonnormal_stepping_cases() {
    let Ok(path) = std::env::var("REV02_CASES") else {
        println!("REV02_CASES not set: cases not written");
        return;
    };
    let mut cases = Vec::new();
    for family in families() {
        let (chosen, best, other, steps) =
            certify_exp_action_auto(&family.a, &family.v, family.tau).unwrap();
        let (identity, osborne) = match chosen {
            rodas5p_core::nonnormal_certificate::AutoMetric::Identity => (&best, &other),
            rodas5p_core::nonnormal_certificate::AutoMetric::Osborne => (&other, &best),
        };
        let (identity_steps, osborne_steps) = match chosen {
            rodas5p_core::nonnormal_certificate::AutoMetric::Identity => (steps[0], steps[1]),
            rodas5p_core::nonnormal_certificate::AutoMetric::Osborne => (steps[1], steps[0]),
        };
        let metric = osborne_metric(&family.a).unwrap();
        let single_identity =
            certify_exp_action(&family.a, &family.v, family.tau, None, 30, None).unwrap();
        let single_osborne =
            certify_exp_action(&family.a, &family.v, family.tau, Some(&metric), 30, None).unwrap();
        println!(
            "{} {}: chose {:?}; identity {:?} {:e} ({} steps); osborne {:?} {:e} ({} steps)",
            family.group,
            family.label,
            chosen,
            identity.status,
            identity.error_upper,
            identity_steps,
            osborne.status,
            osborne.error_upper,
            osborne_steps
        );
        cases.push(json!({
            "group": family.group, "label": family.label,
            "a": family.a.iter().map(|row| hexes(row)).collect::<Vec<_>>(),
            "v": hexes(&family.v), "tau": hex(family.tau),
            "chosen": chosen,
            "stepped_identity": {"steps": identity_steps, "certificate": record(identity)},
            "stepped_osborne": {"steps": osborne_steps, "metric": hexes(&metric), "certificate": record(osborne)},
            "single_step_identity_m30": record(&single_identity),
            "single_step_osborne_m30": record(&single_osborne),
        }));
    }
    let mut grid = Vec::new();
    let points: Vec<f64> = (0..=200)
        .map(|k| -700.0 + 7.0 * k as f64)
        .chain((0..50).map(|k| -1.0 + 2.0 * k as f64 / 49.0))
        .collect();
    for x in points {
        let e = exp_interval(x).unwrap();
        grid.push(json!({"x": hex(x), "lo": hex(e.lo), "hi": hex(e.hi)}));
    }
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    assert!(
        !target.exists(),
        "immutable output exists: {}",
        target.display()
    );
    std::fs::write(
        &target,
        serde_json::to_string_pretty(
            &json!({"schema": "vigilode-rev02-cases-v1", "cases": cases, "exp_grid": grid}),
        )
        .unwrap()
            + "\n",
    )
    .unwrap();
    println!("wrote {}", target.display());
}
