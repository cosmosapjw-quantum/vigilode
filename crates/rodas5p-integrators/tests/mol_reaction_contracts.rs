//! Contracts of the reaction-diffusion right-hand sides of `rodas5p_integrators::mol`:
//! Gray-Scott, Brusselator, Allen-Cahn and sine-Gordon (loop node V11, part a; choices in
//! `research/production_loop_20261116/battery_v4/PREDECLARED.json`).
//!
//! * C7: second-order truncation on smooth periodic manufactured fields (every consecutive
//!   order >= 1.9);
//! * C4, C5: analytic JVP against the assembled Jacobian and central finite differences,
//!   pattern, bandwidth, weights, declared flags;
//! * C6: the weighted certificate refuses every nonlinear operator.

use std::f64::consts::PI;

use rodas5p_integrators::mol::{
    AllenCahn, Boundary, Brusselator, CartesianGrid, CertificateRefusal, GrayScott, MolOperator,
    SineGordon, SpatialOrder, certify_weighted_self_adjoint,
};
use serde_json::{Value, json};

fn qual(test: &str, data: Value) {
    println!("V11_QUAL {}", json!({"test": test, "data": data}));
}

fn lcg(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn orders(e: &[f64]) -> Vec<f64> {
    e.windows(2).map(|w| (w[0] / w[1]).log2()).collect()
}

fn max_abs(v: &[f64]) -> f64 {
    v.iter().fold(0.0f64, |m, x| m.max(x.abs()))
}

fn apply(op: &dyn MolOperator, u: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; u.len()];
    op.apply_into(0.0, u, &mut out).unwrap();
    out
}

fn assemble(op: &dyn MolOperator, u: &[f64]) -> Vec<f64> {
    let mut values = vec![0.0; op.structure().pattern().nnz()];
    op.jacobian_values_into(0.0, u, &mut values).unwrap();
    values
}

fn csc_products(op: &dyn MolOperator, values: &[f64], v: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let p = op.structure().pattern();
    let n = p.dimension();
    let (mut jv, mut abs) = (vec![0.0; n], vec![0.0; n]);
    for col in 0..n {
        for k in p.col_ptr()[col]..p.col_ptr()[col + 1] {
            let row = p.row_idx()[k];
            jv[row] += values[k] * v[col];
            abs[row] += (values[k] * v[col]).abs();
        }
    }
    (jv, abs)
}

/// Smooth periodic product field `prod_a exp(sin(2 pi x_a / L_a + phase))`: values and its
/// isotropic Laplacian.
fn product_field(grid: &CartesianGrid, phase: f64) -> (Vec<f64>, Vec<f64>) {
    let n = grid.count();
    let dims = grid.dims();
    let (mut u, mut lap) = (vec![0.0; n], vec![0.0; n]);
    for cell in 0..n {
        let x = grid.centre(cell);
        let parts: Vec<(f64, f64)> = (0..dims)
            .map(|a| {
                let k = 2.0 * PI / grid.lengths()[a];
                let (s, c) = (k * x[a] + phase).sin_cos();
                let g = s.exp();
                (g, k * k * (c * c - s) * g)
            })
            .collect();
        u[cell] = parts.iter().map(|p| p.0).product();
        lap[cell] = (0..dims)
            .map(|a| {
                parts[a].1
                    * (0..dims)
                        .filter(|&b| b != a)
                        .map(|b| parts[b].0)
                        .product::<f64>()
            })
            .sum();
    }
    (u, lap)
}

#[derive(Clone, Copy, Debug)]
enum Model {
    GrayScott,
    Brusselator,
    AllenCahn,
    SineGordon,
}

const GS: (f64, f64, f64, f64) = (2.0e-3, 1.0e-3, 0.04, 0.06);
const BR: (f64, f64, f64, f64) = (1.0, 3.4, 2.0e-3, 4.0e-3);
const AC: f64 = 1.0e-2;
const SG: f64 = 0.5;

fn build(model: Model, grid: CartesianGrid) -> Box<dyn MolOperator> {
    match model {
        Model::GrayScott => Box::new(GrayScott::new(grid, GS.0, GS.1, GS.2, GS.3).unwrap()),
        Model::Brusselator => Box::new(Brusselator::new(grid, BR.0, BR.1, BR.2, BR.3).unwrap()),
        Model::AllenCahn => Box::new(AllenCahn::new(grid, AC).unwrap()),
        Model::SineGordon => Box::new(SineGordon::new(grid, SG).unwrap()),
    }
}

/// Interleaved manufactured state and the exact right-hand side at the cell centres.
fn manufactured(model: Model, grid: &CartesianGrid) -> (Vec<f64>, Vec<f64>) {
    let (a, lap_a) = product_field(grid, 0.0);
    let (b, lap_b) = product_field(grid, 1.0);
    let n = grid.count();
    match model {
        Model::GrayScott | Model::Brusselator | Model::SineGordon => {
            let mut y = vec![0.0; 2 * n];
            let mut f = vec![0.0; 2 * n];
            for i in 0..n {
                let (u, v) = (0.3 * a[i], 0.2 * b[i]);
                let (lu, lv) = (0.3 * lap_a[i], 0.2 * lap_b[i]);
                y[2 * i] = u;
                y[2 * i + 1] = v;
                let (fu, fv) = match model {
                    Model::GrayScott => (
                        GS.0 * lu - u * v * v + GS.2 * (1.0 - u),
                        GS.1 * lv + u * v * v - (GS.2 + GS.3) * v,
                    ),
                    Model::Brusselator => (
                        BR.0 + u * u * v - (BR.1 + 1.0) * u + BR.2 * lu,
                        BR.1 * u - u * u * v + BR.3 * lv,
                    ),
                    _ => (v, SG * lu - u.sin()),
                };
                f[2 * i] = fu;
                f[2 * i + 1] = fv;
            }
            (y, f)
        }
        Model::AllenCahn => {
            let y: Vec<f64> = a.iter().map(|x| 0.5 * x - 0.6).collect();
            let f = (0..n)
                .map(|i| AC * 0.5 * lap_a[i] + y[i] - y[i] * y[i] * y[i])
                .collect();
            (y, f)
        }
    }
}

#[test]
fn c7_reaction_diffusion_right_hand_sides_converge_at_second_order() {
    let mut rows = Vec::new();
    let mut pass = true;
    let mut cases: Vec<(Model, usize, Vec<usize>)> = [
        Model::GrayScott,
        Model::Brusselator,
        Model::AllenCahn,
        Model::SineGordon,
    ]
    .into_iter()
    .map(|m| (m, 2usize, vec![16usize, 32, 64, 128]))
    .collect();
    cases.push((Model::GrayScott, 3, vec![8, 16, 32]));
    for (model, dims, ladder) in cases {
        let mut errors = Vec::new();
        for &m in &ladder {
            let cells = vec![m; dims];
            let lengths: Vec<f64> = [1.0, 2.0, 0.5][..dims].to_vec();
            let grid = CartesianGrid::new(&cells, &lengths, &[Boundary::Periodic; 3][..dims]).unwrap();
            let op = build(model, grid.clone());
            assert_eq!(op.structure().order(), SpatialOrder::Second);
            let (y, exact) = manufactured(model, &grid);
            let fh = apply(op.as_ref(), &y);
            errors.push(
                fh.iter()
                    .zip(&exact)
                    .fold(0.0f64, |acc, (a, b)| acc.max((a - b).abs()))
                    / max_abs(&exact),
            );
        }
        let o = orders(&errors);
        let ok = o.iter().all(|&x| x >= 1.9);
        pass &= ok;
        rows.push(json!({"model": format!("{model:?}"), "dims": dims, "ladder": ladder,
                         "errors": errors, "orders": o, "pass": ok}));
    }
    qual(
        "c7_reaction_truncation",
        json!({"rows": rows, "rule": "every consecutive order >= 1.9", "pass": pass}),
    );
    assert!(pass, "a reaction-diffusion right-hand side converges below order 1.9");
}

#[test]
fn c4_c5_c6_reaction_jvp_structure_and_certificate_refusal() {
    use Boundary::{DirichletZero as D, NeumannZero as N, Periodic as P};
    let mut rows = Vec::new();
    let mut ok = true;
    for (cells, bc) in [
        (vec![12usize, 10], vec![P, P]),
        (vec![12, 10], vec![N, D]),
        (vec![20], vec![N]),
        (vec![6, 5, 4], vec![P, N, D]),
    ] {
        let dims = cells.len();
        let lengths = vec![1.0; dims];
        let grid = CartesianGrid::new(&cells, &lengths, &bc).unwrap();
        for model in [
            Model::GrayScott,
            Model::Brusselator,
            Model::AllenCahn,
            Model::SineGordon,
        ] {
            let op = build(model, grid.clone());
            let s = op.structure();
            let n = op.dimension();
            let mut seed = 31 + n as u64;
            let u: Vec<f64> = (0..n).map(|_| 0.2 + lcg(&mut seed)).collect();
            let v: Vec<f64> = (0..n).map(|_| lcg(&mut seed) - 0.5).collect();
            let mut jvp = vec![0.0; n];
            op.jvp_into(0.0, &u, &v, &mut jvp).unwrap();
            let values = assemble(op.as_ref(), &u);
            let (jv, abs) = csc_products(op.as_ref(), &values, &v);
            let scale = max_abs(&abs);
            let csc = jv
                .iter()
                .zip(&jvp)
                .fold(0.0f64, |m, (a, b)| m.max((a - b).abs()))
                / scale;
            let delta = 1e-6 * max_abs(&u).max(1.0) / max_abs(&v);
            let up: Vec<f64> = u.iter().zip(&v).map(|(a, b)| a + delta * b).collect();
            let um: Vec<f64> = u.iter().zip(&v).map(|(a, b)| a - delta * b).collect();
            let (fp, fm) = (apply(op.as_ref(), &up), apply(op.as_ref(), &um));
            let fd = fp
                .iter()
                .zip(&fm)
                .zip(&jvp)
                .fold(0.0f64, |m, ((a, b), c)| m.max(((a - b) / (2.0 * delta) - c).abs()))
                / scale;
            let p = s.pattern();
            let (lower, upper) = s.bandwidth();
            let mut outside = 0.0f64;
            let mut e = vec![0.0; n];
            let mut col = vec![0.0; n];
            for j in 0..n {
                e.fill(0.0);
                e[j] = 1.0;
                op.jvp_into(0.0, &u, &e, &mut col).unwrap();
                for (i, &value) in col.iter().enumerate() {
                    if value != 0.0 && p.position(i, j).is_none() {
                        outside = outside.max(value.abs());
                    }
                }
            }
            let inside = (0..n).all(|c| {
                (p.col_ptr()[c]..p.col_ptr()[c + 1]).all(|k| {
                    let r = p.row_idx()[k];
                    r <= c + lower && c <= r + upper
                })
            });
            let species = s.species();
            let weights_ok = s.weights().len() == n
                && s.weights()
                    .iter()
                    .all(|w| *w == grid.cell_volume() && *w > 0.0);
            let certificate = certify_weighted_self_adjoint(op.as_ref());
            let row_ok = csc <= 1e-12
                && fd <= 1e-6
                && outside == 0.0
                && inside
                && weights_ok
                && s.stencil_radius() == 1
                && !s.is_linear()
                && !s.is_conservative()
                && certificate == Err(CertificateRefusal::Nonlinear)
                && s.lines().len() == dims;
            ok &= row_ok;
            let boundary: Vec<String> = bc.iter().map(|b| format!("{b:?}")).collect();
            rows.push(json!({
                "model": format!("{model:?}"), "cells": cells, "boundaries": boundary,
                "n": n, "species": species, "csc_relative_defect": csc, "fd_relative_defect": fd,
                "largest_entry_outside_pattern": outside, "pattern_inside_bandwidth": inside,
                "bandwidth": [lower, upper], "weights_ok": weights_ok,
                "certificate": format!("{certificate:?}"), "pass": row_ok,
            }));
        }
    }
    qual("c4_c5_c6_reaction", json!({"rows": rows, "pass": ok}));
    assert!(ok, "a reaction-diffusion JVP, structure or certificate check failed");
}

#[test]
fn reaction_constructors_refuse_invalid_parameters() {
    let grid = CartesianGrid::new(&[8, 8], &[1.0, 1.0], &[Boundary::Periodic; 2]).unwrap();
    assert!(GrayScott::new(grid.clone(), -1.0, 1.0, 0.04, 0.06).is_err());
    assert!(GrayScott::new(grid.clone(), 1.0, 1.0, f64::NAN, 0.06).is_err());
    assert!(Brusselator::new(grid.clone(), f64::INFINITY, 3.4, 1.0, 1.0).is_err());
    assert!(AllenCahn::new(grid.clone(), -0.1).is_err());
    assert!(SineGordon::new(grid, f64::NAN).is_err());
}
