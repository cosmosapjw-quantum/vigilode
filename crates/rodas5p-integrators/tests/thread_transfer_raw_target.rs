//! Raw-stage (U) against native K target contract (research node
//! `research/thread_transfer_mf_target_20261002`, thread-transfer DAG node
//! P0-MF-TARGET). The symbolic identity and the exact containment of the
//! enclosures are checked by `tools/thread_transfer_raw_target_check.py`.

#[path = "thread_transfer_common/mod.rs"]
mod common;

use common::write_output;
use rodas5p_core::{DenseMatrix, load_rodas5p_coefficients, rodas5p_coefficients};
use rodas5p_integrators::{
    RawStageReceipt, raw_absolute_residual_budget, raw_relative_residual_budget,
    raw_residual_transport_bound, raw_stage_allowance, residual_scale_digest, sequential_gamma,
};
use serde_json::{Value, json};

const N: usize = 3;
const A: [[f64; N]; N] = [[-2.0, 0.5, 0.0], [0.3, -40.0, 1.0], [0.0, 2.0, -500.0]];
const Q: [f64; N] = [0.4, -0.7, 0.2];

/// `f(t, y) = A y + q y^2 + g(t)`, `g(t) = (sin t, cos 2t, t)`.
fn f(t: f64, y: &[f64]) -> Vec<f64> {
    let g = [t.sin(), (2.0 * t).cos(), t];
    (0..N)
        .map(|a| (0..N).map(|b| A[a][b] * y[b]).sum::<f64>() + Q[a] * y[a] * y[a] + g[a])
        .collect()
}

fn f_t(t: f64) -> Vec<f64> {
    vec![t.cos(), -2.0 * (2.0 * t).sin(), 1.0]
}

fn jacobian(y: &[f64]) -> [[f64; N]; N] {
    let mut j = A;
    for a in 0..N {
        j[a][a] += 2.0 * Q[a] * y[a];
    }
    j
}

fn matvec(m: &[[f64; N]; N], x: &[f64]) -> Vec<f64> {
    (0..N)
        .map(|a| (0..N).map(|b| m[a][b] * x[b]).sum())
        .collect()
}

fn pseudo_random_stages(s: usize, salt: u64) -> Vec<Vec<f64>> {
    let mut state = 0x2545_f491_4f6c_dd1d_u64 ^ salt;
    (0..s)
        .map(|_| {
            (0..N)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    (state >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0
                })
                .collect()
        })
        .collect()
}

/// Largest `|r_U(SK) - gamma r_K(K)|` over the bound plus the rounding
/// allowance (<= 1 passes), and the largest transport bound.
fn transport_case(h: f64, salt: u64) -> (f64, f64, f64) {
    let coeffs = rodas5p_coefficients().unwrap();
    let s = coeffs.stages();
    let gamma = coeffs.gamma;
    let sm = sequential_gamma(coeffs);
    let allowance = raw_stage_allowance(coeffs).unwrap();
    let (t, y) = (0.3, vec![0.8, -0.4, 0.1]);
    let k = pseudo_random_stages(s, salt);
    let u = (0..s)
        .map(|i| {
            (0..N)
                .map(|a| (0..=i).map(|j| sm[i][j] * k[j][a]).sum())
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<_>>();
    let jac = jacobian(&y);
    let ft = f_t(t);
    let mut worst = 0.0_f64;
    let mut largest_bound = 0.0_f64;
    let mut magnitude = 0.0_f64;
    // Lipschitz bound of f in the max norm over the stage arguments:
    // ||A||_inf + 2 max|q| max|z|.
    let mut zmax = y.iter().map(|v| v.abs()).fold(0.0, f64::max);
    let mut r_k = Vec::new();
    let mut r_u = Vec::new();
    for i in 0..s {
        let ti = t + coeffs.c[i] * h;
        let state_k = (0..N)
            .map(|a| y[a] + (0..i).map(|j| coeffs.alpha[(i, j)] * k[j][a]).sum::<f64>())
            .collect::<Vec<_>>();
        let state_u = (0..N)
            .map(|a| y[a] + (0..i).map(|j| coeffs.a[(i, j)] * u[j][a]).sum::<f64>())
            .collect::<Vec<_>>();
        zmax = zmax
            .max(state_k.iter().map(|v| v.abs()).fold(0.0, f64::max))
            .max(state_u.iter().map(|v| v.abs()).fold(0.0, f64::max));
        let gmix = (0..N)
            .map(|a| (0..i).map(|j| sm[i][j] * k[j][a]).sum::<f64>())
            .collect::<Vec<_>>();
        let (fk, fu) = (f(ti, &state_k), f(ti, &state_u));
        let (jk, ju, jg) = (
            matvec(&jac, &k[i]),
            matvec(&jac, &u[i]),
            matvec(&jac, &gmix),
        );
        let cmix = (0..N)
            .map(|a| {
                (0..i)
                    .map(|j| coeffs.c_matrix[(i, j)] * u[j][a])
                    .sum::<f64>()
            })
            .collect::<Vec<_>>();
        r_k.push(
            (0..N)
                .map(|a| {
                    k[i][a]
                        - h * gamma * jk[a]
                        - h * fk[a]
                        - h * jg[a]
                        - h * h * coeffs.gamma_rows[i] * ft[a]
                })
                .collect::<Vec<_>>(),
        );
        r_u.push(
            (0..N)
                .map(|a| {
                    u[i][a]
                        - h * gamma * ju[a]
                        - h * gamma * fu[a]
                        - gamma * cmix[a]
                        - h * h * gamma * coeffs.gamma_rows[i] * ft[a]
                })
                .collect::<Vec<_>>(),
        );
        magnitude = magnitude.max(
            k[i].iter()
                .chain(&u[i])
                .map(|v| v.abs())
                .fold(0.0, f64::max),
        );
    }
    let a_norm = A
        .iter()
        .map(|row| row.iter().map(|v| v.abs()).sum::<f64>())
        .fold(0.0, f64::max);
    let lip = 2.0 * (a_norm + 2.0 * Q.iter().map(|v| v.abs()).fold(0.0, f64::max) * zmax);
    let bound = raw_residual_transport_bound(&allowance, &k, h, gamma, lip).unwrap();
    for i in 0..s {
        for a in 0..N {
            let difference = (r_u[i][a] - gamma * r_k[i][a]).abs();
            // Evaluation rounding: terms of size |K|, |U|, |h A y| enter.
            let rounding = 64.0
                * f64::EPSILON
                * (r_u[i][a].abs() + (gamma * r_k[i][a]).abs() + magnitude.max(1.0));
            worst = worst.max(difference / (bound[i][a] + rounding));
            largest_bound = largest_bound.max(bound[i][a]);
        }
    }
    (worst, largest_bound, magnitude)
}

fn bump(value: f64) -> f64 {
    if value == 0.0 {
        1.0e-300
    } else {
        value.next_up()
    }
}

fn first_nonzero(m: &mut DenseMatrix) -> &mut f64 {
    let index = m.as_slice().iter().position(|v| *v != 0.0).unwrap();
    &mut m.as_mut_slice()[index]
}

fn receipt_invalidation() -> (bool, Value) {
    let base = load_rodas5p_coefficients().unwrap();
    let (h, op, scale) = (
        0.01,
        "problem=test|t=0|y=..",
        residual_scale_digest(1e-8, 1e-6, &[1.0, 2.0]),
    );
    let receipt = RawStageReceipt::new(&base, h, op, &scale).unwrap();
    let mut rows = Vec::new();
    let mut all = receipt.validate(&base, h, op, &scale).is_ok();
    rows.push(json!({"case": "unchanged", "validates": all}));
    type Mutation = fn(&mut rodas5p_core::Rodas5pCoefficients);
    let mutations: [(&str, Mutation); 10] = [
        ("a", |c| {
            let v = first_nonzero(&mut c.a);
            *v = bump(*v);
        }),
        ("c_matrix", |c| {
            let v = first_nonzero(&mut c.c_matrix);
            *v = bump(*v);
        }),
        ("gamma", |c| c.gamma = bump(c.gamma)),
        ("b_code", |c| c.b_code[0] = bump(c.b_code[0])),
        ("gamma_matrix", |c| {
            let v = first_nonzero(&mut c.gamma_matrix);
            *v = bump(*v);
        }),
        ("alpha", |c| {
            let v = first_nonzero(&mut c.alpha);
            *v = bump(*v);
        }),
        ("b", |c| c.b[0] = bump(c.b[0])),
        ("btilde", |c| c.btilde[0] = bump(c.btilde[0])),
        ("dense_h", |c| {
            let v = first_nonzero(&mut c.dense_h);
            *v = bump(*v);
        }),
        ("dense_d", |c| {
            let v = first_nonzero(&mut c.dense_d);
            *v = bump(*v);
        }),
    ];
    for (name, mutate) in mutations {
        let mut mutated = base.clone();
        mutate(&mut mutated);
        let rejected = receipt.validate(&mutated, h, op, &scale).is_err();
        all &= rejected;
        rows.push(json!({"case": format!("coefficient {name}"), "rejected": rejected}));
    }
    let others = [
        (
            "h",
            receipt.validate(&base, h.next_up(), op, &scale).is_err(),
        ),
        (
            "operator",
            receipt
                .validate(&base, h, "problem=test|t=1|y=..", &scale)
                .is_err(),
        ),
        (
            "scale",
            receipt
                .validate(
                    &base,
                    h,
                    op,
                    &residual_scale_digest(1e-8, 1e-6, &[1.0, 2.5]),
                )
                .is_err(),
        ),
    ];
    for (name, rejected) in others {
        all &= rejected;
        rows.push(json!({"case": name, "rejected": rejected}));
    }
    (all, Value::Array(rows))
}

fn residual_mapping() -> (bool, Value) {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    let tau_k = 1.0e-7;
    let tau_u = raw_absolute_residual_budget(tau_k, gamma).unwrap();
    // At most the exact product: tau_u <= |gamma| tau_k as reals.
    let product_ok =
        tau_u <= gamma.abs() * tau_k && (gamma.abs() * tau_k - tau_u) <= f64::EPSILON * tau_u;
    let rejects = [0.0, -1.0, f64::NAN, f64::INFINITY]
        .iter()
        .all(|bad| raw_absolute_residual_budget(*bad, gamma).is_err())
        && raw_absolute_residual_budget(tau_k, 0.0).is_err();
    // The relative criterion is recomputed from the U right-hand side. On a
    // stage whose K and U right-hand sides differ, eta ||rhs_U|| is not
    // |gamma| eta ||rhs_K||.
    let (eta, rhs_k_norm, rhs_u_norm) = (1.0e-3, 2.0, 0.9);
    let recomputed = raw_relative_residual_budget(eta, rhs_u_norm).unwrap();
    let transported = gamma.abs() * eta * rhs_k_norm;
    let nearest = eta * rhs_u_norm;
    let relative_ok = recomputed <= nearest
        && nearest - recomputed <= f64::EPSILON * nearest
        && (recomputed - transported).abs() > 1.0e-6 * transported
        && raw_relative_residual_budget(0.0, 1.0).is_err()
        && raw_relative_residual_budget(eta, f64::NAN).is_err();
    let ok = product_ok && rejects && relative_ok;
    (
        ok,
        json!({"gamma": gamma, "tau_k": tau_k, "tau_u": tau_u, "product_ok": product_ok,
        "rejects_invalid": rejects, "relative_recomputed": recomputed, "relative_if_transported": transported,
        "relative_ok": relative_ok}),
    )
}

#[test]
fn raw_stage_target_contract() {
    let coeffs = rodas5p_coefficients().unwrap();
    let allowance = raw_stage_allowance(coeffs).unwrap();
    let mut transport = Vec::new();
    let mut transport_ok = true;
    for h in [1.0e-3, 0.05] {
        for salt in 0..8_u64 {
            let (worst, largest, magnitude) = transport_case(h, salt);
            transport_ok &= worst <= 1.0;
            transport.push(
                json!({"h": h, "salt": salt, "worst_ratio_to_allowed": worst,
                "largest_transport_bound": largest, "stage_magnitude": magnitude}),
            );
        }
    }
    let (receipt_ok, receipt_rows) = receipt_invalidation();
    let (mapping_ok, mapping) = residual_mapping();
    let result = json!({
        "schema": "vigilode-thread-transfer-raw-target-v1",
        "target_id": allowance.target_id,
        "coefficient_sha256": allowance.coefficient_sha256,
        "magnitudes": {
            "d_gamma_max": allowance.d_gamma_max(),
            "d_alpha_max": allowance.d_alpha_max(),
            "d_output_max": allowance.d_output_max(),
            "d_embedded_max": allowance.d_embedded_max(),
            "d_dense_max": allowance.d_dense_max(),
        },
        "gate_native": {
            "native_transport": transport_ok,
            "receipt_invalidation": receipt_ok,
            "residual_mapping": mapping_ok,
        },
        "note": "gate items 1 (symbolic identity) and 2 (exact containment) are in EXACT_CHECK.json",
        "transport_cases": transport,
        "receipt_cases": receipt_rows,
        "residual_mapping": mapping,
    });
    write_output("THREAD_TRANSFER_RAW_TARGET_OUTPUT", &result);
    println!(
        "{}",
        json!({"gate_native": result["gate_native"], "magnitudes": result["magnitudes"]})
    );
    assert!(transport_ok && receipt_ok && mapping_ok);
}

/// Writes the enclosures and the coefficient bits for the exact check.
#[test]
#[ignore = "writes fixtures/thread_transfer_raw_stage_allowance.json"]
fn write_raw_stage_allowance() {
    let coeffs = rodas5p_coefficients().unwrap();
    let allowance = raw_stage_allowance(coeffs).unwrap();
    let hex = |v: f64| format!("{:016x}", v.to_bits());
    let matrix = |m: &DenseMatrix| {
        (0..m.nrows())
            .map(|i| (0..m.ncols()).map(|j| hex(m[(i, j)])).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    let intervals = |rows: &[Vec<rodas5p_core::directed::Interval>]| {
        rows.iter()
            .map(|row| {
                row.iter()
                    .map(|x| [hex(x.lo), hex(x.hi)])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let vector = |v: &[rodas5p_core::directed::Interval]| {
        v.iter().map(|x| [hex(x.lo), hex(x.hi)]).collect::<Vec<_>>()
    };
    let value = json!({
        "schema": "vigilode-thread-transfer-raw-stage-allowance-v1",
        "target_id": allowance.target_id,
        "coefficient_sha256": allowance.coefficient_sha256,
        "coefficients": {
            "gamma": hex(coeffs.gamma),
            "a": matrix(&coeffs.a),
            "c_matrix": matrix(&coeffs.c_matrix),
            "b_code": coeffs.b_code.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
            "gamma_matrix": matrix(&coeffs.gamma_matrix),
            "alpha": matrix(&coeffs.alpha),
            "b": coeffs.b.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
            "btilde": coeffs.btilde.iter().map(|v| hex(*v)).collect::<Vec<_>>(),
            "dense_h": matrix(&coeffs.dense_h),
            "dense_d": matrix(&coeffs.dense_d),
        },
        "enclosures": {
            "d_gamma": intervals(&allowance.d_gamma),
            "d_alpha": intervals(&allowance.d_alpha),
            "d_output": vector(&allowance.d_output),
            "d_embedded": vector(&allowance.d_embedded),
            "d_dense": intervals(&allowance.d_dense),
        },
    });
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/thread_transfer_raw_stage_allowance.json");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
}
