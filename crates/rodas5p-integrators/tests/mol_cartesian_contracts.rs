//! Contracts of the Cartesian Laplacians of `rodas5p_integrators::mol` (loop node V11,
//! part a; choices in `research/production_loop_20261116/battery_v4/PREDECLARED.json`).
//!
//! * C1: second-order convergence of the discrete Helmholtz solution `(I - L_h) u_h = f_h`
//!   for manufactured solutions, every boundary type, 1D, 2D, 3D and the anisotropic box
//!   (frozen discriminator, rule: every consecutive observed order >= 1.9);
//! * exactness of the discrete eigenvectors and the periodic truncation order (reported);
//! * C3: conservation of the weighted integral, defect <= 1e-12 (frozen discriminator);
//! * C4: JVP against `apply_into`, the assembled Jacobian and central finite differences;
//! * C5: weights, stencil radius, bandwidth and pattern;
//! * C6: the weighted self-adjoint certificate.
//!
//! Every test prints its numbers on a line `V11_QUAL {json}` before it asserts.

use std::f64::consts::PI;

use rodas5p_integrators::mol::{
    Boundary, CartesianGrid, CartesianLaplacian, MolOperator, SpatialOrder,
    certify_weighted_self_adjoint,
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

/// Neumaier-compensated sum.
fn neumaier(values: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut c) = (0.0f64, 0.0f64);
    for v in values {
        let t = sum + v;
        if sum.abs() >= v.abs() {
            c += (sum - t) + v;
        } else {
            c += (v - t) + sum;
        }
        sum = t;
    }
    sum + c
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

/// `(J v, sum_j |J_ij| |v_j|)` from the assembled pattern values.
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

/// C3 defect and the absolute weighted sum.
fn conservation(op: &dyn MolOperator, u: &[f64]) -> (f64, f64) {
    let w = op.structure().weights();
    let f = apply(op, u);
    let values = assemble(op, u);
    let (_, abs) = csc_products(op, &values, u);
    let total = neumaier(f.iter().zip(w).map(|(a, b)| a * b));
    let scale = neumaier(abs.iter().zip(w).map(|(a, b)| a * b));
    (total.abs() / scale, total.abs())
}

/// C4: (bitwise jvp == apply for linear operators, CSC relative defect, FD relative defect).
fn jvp_checks(op: &dyn MolOperator, u: &[f64], v: &[f64]) -> (bool, f64, f64) {
    let n = u.len();
    let mut jvp = vec![0.0; n];
    op.jvp_into(0.0, u, v, &mut jvp).unwrap();
    let bitwise = if op.structure().is_linear() {
        apply(op, v)
            .iter()
            .zip(&jvp)
            .all(|(a, b)| a.to_bits() == b.to_bits())
    } else {
        true
    };
    let values = assemble(op, u);
    let (jv, abs) = csc_products(op, &values, v);
    let scale = max_abs(&abs);
    let csc = jv
        .iter()
        .zip(&jvp)
        .fold(0.0f64, |m, (a, b)| m.max((a - b).abs()))
        / scale;
    let delta = 1e-6 * max_abs(u).max(1.0) / max_abs(v);
    let up: Vec<f64> = u.iter().zip(v).map(|(a, b)| a + delta * b).collect();
    let um: Vec<f64> = u.iter().zip(v).map(|(a, b)| a - delta * b).collect();
    let (fp, fm) = (apply(op, &up), apply(op, &um));
    let fd = fp
        .iter()
        .zip(&fm)
        .zip(&jvp)
        .fold(0.0f64, |m, ((a, b), c)| m.max(((a - b) / (2.0 * delta) - c).abs()))
        / scale;
    (bitwise, csc, fd)
}

/// C5 on a small operator: (largest |J_ij| outside the pattern from JVP columns, every
/// pattern entry inside the bandwidth, diagonal present).
fn pattern_checks(op: &dyn MolOperator, u: &[f64]) -> (f64, bool, bool) {
    let s = op.structure();
    let p = s.pattern();
    let n = p.dimension();
    let (lower, upper) = s.bandwidth();
    let mut outside = 0.0f64;
    let mut e = vec![0.0; n];
    let mut col = vec![0.0; n];
    for j in 0..n {
        e.fill(0.0);
        e[j] = 1.0;
        op.jvp_into(0.0, u, &e, &mut col).unwrap();
        for (i, &value) in col.iter().enumerate() {
            if value != 0.0 && p.position(i, j).is_none() {
                outside = outside.max(value.abs());
            }
        }
    }
    let mut inside = true;
    for c in 0..n {
        for k in p.col_ptr()[c]..p.col_ptr()[c + 1] {
            let r = p.row_idx()[k];
            if r > c + lower || c > r + upper {
                inside = false;
            }
        }
    }
    let diagonal = (0..n).all(|i| p.position(i, i) == Some(p.diagonal_positions()[i]));
    (outside, inside, diagonal)
}

fn bc_name(b: Boundary) -> &'static str {
    match b {
        Boundary::Periodic => "P",
        Boundary::NeumannZero => "N",
        Boundary::DirichletZero => "D",
    }
}

/// Manufactured factor `(g, g'')` of one axis for its boundary.
fn factor(b: Boundary, x: f64, l: f64) -> (f64, f64) {
    match b {
        Boundary::Periodic => {
            let k = 2.0 * PI / l;
            let (s, c) = (k * x).sin_cos();
            let g = s.exp();
            (g, k * k * (c * c - s) * g)
        }
        Boundary::NeumannZero => {
            let k = PI / l;
            let (s, c) = (k * x).sin_cos();
            let g = c.exp();
            (g, k * k * (s * s - c) * g)
        }
        Boundary::DirichletZero => {
            let k = PI / l;
            let (s, c) = (k * x).sin_cos();
            let e = (x / l).exp();
            (s * e, e * (-k * k * s + 2.0 * k * c / l + s / (l * l)))
        }
    }
}

/// `(u, sum_a D_a d_aa u)` at every cell centre.
fn manufactured(grid: &CartesianGrid, diffusivity: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let dims = grid.dims();
    let mut u = vec![0.0; grid.count()];
    let mut lap = vec![0.0; grid.count()];
    for cell in 0..grid.count() {
        let x = grid.centre(cell);
        let parts: Vec<(f64, f64)> = (0..dims)
            .map(|a| factor(grid.boundaries()[a], x[a], grid.lengths()[a]))
            .collect();
        let product: f64 = parts.iter().map(|p| p.0).product();
        u[cell] = product;
        lap[cell] = (0..dims)
            .map(|a| {
                let others: f64 = (0..dims).filter(|&b| b != a).map(|b| parts[b].0).product();
                diffusivity[a] * parts[a].1 * others
            })
            .sum();
    }
    (u, lap)
}

/// Conjugate gradients on `(I - L) x = f`: (x, iterations, true relative residual).
fn helmholtz_cg(op: &dyn MolOperator, f: &[f64]) -> (Vec<f64>, usize, f64) {
    let n = f.len();
    let dot = |a: &[f64], b: &[f64]| -> f64 { a.iter().zip(b).map(|(x, y)| x * y).sum() };
    let a_times = |x: &[f64], out: &mut Vec<f64>| {
        op.apply_into(0.0, x, out).unwrap();
        for (o, xi) in out.iter_mut().zip(x) {
            *o = xi - *o;
        }
    };
    let fnorm = dot(f, f).sqrt();
    let mut x = vec![0.0; n];
    let mut r = f.to_vec();
    let mut p = r.clone();
    let mut ap = vec![0.0; n];
    let mut rr = dot(&r, &r);
    let mut iterations = 0;
    while rr.sqrt() > 1e-13 * fnorm && iterations < 200_000 {
        a_times(&p, &mut ap);
        let alpha = rr / dot(&p, &ap);
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        let next = dot(&r, &r);
        let beta = next / rr;
        rr = next;
        for i in 0..n {
            p[i] = r[i] + beta * p[i];
        }
        iterations += 1;
    }
    a_times(&x, &mut ap);
    let true_residual = ap
        .iter()
        .zip(f)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
        / fnorm;
    (x, iterations, true_residual)
}

struct Case {
    id: &'static str,
    cells: fn(usize) -> Vec<usize>,
    ladder: Vec<usize>,
    lengths: Vec<f64>,
    diffusivity: Vec<f64>,
    boundaries: Vec<Vec<Boundary>>,
}

fn cases() -> Vec<Case> {
    use Boundary::{DirichletZero as D, NeumannZero as N, Periodic as P};
    vec![
        Case {
            id: "1D",
            cells: |m| vec![m],
            ladder: vec![32, 64, 128, 256],
            lengths: vec![1.0],
            diffusivity: vec![1.0],
            boundaries: vec![vec![P], vec![N], vec![D]],
        },
        Case {
            id: "2D",
            cells: |m| vec![m, m],
            ladder: vec![16, 32, 64, 128],
            lengths: vec![1.0, 1.0],
            diffusivity: vec![1.0, 1.0],
            boundaries: vec![vec![P, P], vec![N, N], vec![D, D], vec![P, D]],
        },
        Case {
            id: "3D",
            cells: |m| vec![m, m, m],
            ladder: vec![8, 16, 32],
            lengths: vec![1.0, 1.0, 1.0],
            diffusivity: vec![1.0, 1.0, 1.0],
            boundaries: vec![vec![P, P, P], vec![N, N, N], vec![D, D, D]],
        },
        Case {
            id: "3D-anisotropic-box",
            cells: |m| vec![m, m, m / 2],
            ladder: vec![8, 16, 32],
            lengths: vec![1.0, 2.0, 0.5],
            diffusivity: vec![1.0, 0.5, 0.1],
            boundaries: vec![vec![P, N, D]],
        },
    ]
}

fn laplacian(case: &Case, m: usize, bc: &[Boundary]) -> CartesianLaplacian {
    let grid = CartesianGrid::new(&(case.cells)(m), &case.lengths, bc).unwrap();
    CartesianLaplacian::new(grid, &case.diffusivity).unwrap()
}

#[test]
fn c1_helmholtz_solution_converges_at_second_order_for_every_boundary() {
    let mut rows = Vec::new();
    let mut worst = f64::INFINITY;
    for case in cases() {
        for bc in &case.boundaries {
            let mut errors = Vec::new();
            let mut detail = Vec::new();
            for &m in &case.ladder {
                let op = laplacian(&case, m, bc);
                let (u, lap) = manufactured(op.grid(), &case.diffusivity);
                let f: Vec<f64> = u.iter().zip(&lap).map(|(a, b)| a - b).collect();
                let (uh, iterations, residual) = helmholtz_cg(&op, &f);
                let e = uh
                    .iter()
                    .zip(&u)
                    .fold(0.0f64, |acc, (a, b)| acc.max((a - b).abs()))
                    / max_abs(&u);
                errors.push(e);
                detail.push(json!({"m": m, "cells": op.grid().cells(), "error": e,
                                   "cg_iterations": iterations, "cg_true_relative_residual": residual}));
            }
            let o = orders(&errors);
            worst = o.iter().fold(worst, |acc, &x| acc.min(x));
            let boundary: Vec<&str> = bc.iter().map(|b| bc_name(*b)).collect();
            rows.push(json!({"case": case.id, "boundaries": boundary,
                             "lengths": case.lengths, "diffusivity": case.diffusivity,
                             "levels": detail, "errors": errors, "orders": o,
                             "pass": o.iter().all(|&x| x >= 1.9)}));
        }
    }
    let pass = rows.iter().all(|r| r["pass"].as_bool() == Some(true));
    qual(
        "c1_cartesian_helmholtz_convergence",
        json!({"rows": rows, "min_order": worst, "rule": "every consecutive order >= 1.9", "pass": pass}),
    );
    assert!(pass, "a Cartesian case converges below order 1.9: min {worst}");
}

#[test]
fn c1_secondary_eigenvectors_are_exact_and_periodic_truncation_is_second_order() {
    use Boundary::{DirichletZero as D, NeumannZero as N, Periodic as P};
    let mut eigen = Vec::new();
    for (cells, lengths, diffusivity, bc, modes) in [
        (vec![48], vec![1.0], vec![1.0], vec![P], vec![3usize]),
        (vec![48], vec![1.0], vec![1.0], vec![N], vec![3]),
        (vec![48], vec![1.0], vec![1.0], vec![D], vec![3]),
        (vec![24, 20], vec![1.0, 2.0], vec![1.0, 0.3], vec![P, N], vec![2, 1]),
        (vec![24, 20], vec![1.0, 2.0], vec![1.0, 0.3], vec![D, P], vec![1, 2]),
        (vec![12, 10, 8], vec![1.0, 2.0, 0.5], vec![1.0, 0.5, 0.1], vec![P, N, D], vec![1, 2, 1]),
    ] {
        let grid = CartesianGrid::new(&cells, &lengths, &bc).unwrap();
        let op = CartesianLaplacian::new(grid.clone(), &diffusivity).unwrap();
        let dims = cells.len();
        let mut v = vec![1.0; grid.count()];
        let mut lambda = 0.0;
        for a in 0..dims {
            let n = cells[a] as f64;
            let theta = match bc[a] {
                Boundary::Periodic => 2.0 * PI * modes[a] as f64 / n,
                _ => PI * modes[a] as f64 / n,
            };
            lambda += op.rates()[a] * (-4.0 * (0.5 * theta).sin().powi(2));
            for (cell, value) in v.iter_mut().enumerate() {
                let i = grid.coordinates(cell)[a] as f64 + 0.5;
                *value *= match bc[a] {
                    Boundary::DirichletZero => (theta * i).sin(),
                    _ => (theta * i).cos(),
                };
            }
        }
        let lv = apply(&op, &v);
        let defect = lv
            .iter()
            .zip(&v)
            .fold(0.0f64, |m, (a, b)| m.max((a - lambda * b).abs()))
            / (lambda.abs() * max_abs(&v));
        let boundary: Vec<&str> = bc.iter().map(|b| bc_name(*b)).collect();
        eigen.push(json!({"cells": cells, "boundaries": boundary, "modes": modes,
                          "lambda_h": lambda, "relative_defect": defect}));
    }
    let mut truncation = Vec::new();
    for (id, ladder, dims) in [
        ("2D-PP", vec![16usize, 32, 64, 128], 2usize),
        ("3D-PPP", vec![8, 16, 32], 3),
    ] {
        let mut errors = Vec::new();
        for &m in &ladder {
            let cells = vec![m; dims];
            let lengths = vec![1.0; dims];
            let diffusivity = vec![1.0; dims];
            let grid = CartesianGrid::new(&cells, &lengths, &vec![Boundary::Periodic; dims]).unwrap();
            let op = CartesianLaplacian::new(grid, &diffusivity).unwrap();
            let (u, lap) = manufactured(op.grid(), &diffusivity);
            let lu = apply(&op, &u);
            errors.push(
                lu.iter()
                    .zip(&lap)
                    .fold(0.0f64, |acc, (a, b)| acc.max((a - b).abs()))
                    / max_abs(&lap),
            );
        }
        truncation.push(json!({"case": id, "ladder": ladder, "errors": errors, "orders": orders(&errors)}));
    }
    let pass = eigen
        .iter()
        .all(|r| r["relative_defect"].as_f64().unwrap() <= 1e-10);
    qual(
        "c1_secondary_eigen_and_truncation",
        json!({"eigenvectors": eigen, "periodic_truncation": truncation,
               "eigen_rule": "relative defect <= 1e-10", "eigen_pass": pass}),
    );
    assert!(pass, "a discrete eigenvector is not exact");
}

#[test]
fn c3_conservative_laplacians_keep_the_weighted_integral() {
    let mut rows = Vec::new();
    let mut worst = 0.0f64;
    for case in cases() {
        for bc in &case.boundaries {
            for &m in &case.ladder {
                let op = laplacian(&case, m, bc);
                let closed = bc.iter().all(|b| *b != Boundary::DirichletZero);
                assert_eq!(op.structure().is_conservative(), closed);
                if !closed {
                    continue;
                }
                let mut seed = 0x5eed_0000 + m as u64;
                let random: Vec<f64> = (0..op.dimension()).map(|_| 0.5 + lcg(&mut seed)).collect();
                let (smooth, _) = manufactured(op.grid(), &case.diffusivity);
                for (field, u) in [("random", &random), ("smooth", &smooth)] {
                    let (defect, absolute) = conservation(&op, u);
                    worst = worst.max(defect);
                    let boundary: Vec<&str> = bc.iter().map(|b| bc_name(*b)).collect();
                    rows.push(json!({"case": case.id, "boundaries": boundary, "m": m,
                                     "field": field, "defect": defect, "absolute": absolute}));
                }
            }
        }
    }
    let pass = worst <= 1e-12;
    qual(
        "c3_cartesian_conservation",
        json!({"rows": rows, "max_defect": worst, "rule": "defect <= 1e-12", "pass": pass}),
    );
    assert!(pass, "conservation defect {worst:e}");
}

#[test]
fn c4_c5_c6_jvp_structure_and_certificate() {
    use Boundary::{DirichletZero as D, NeumannZero as N, Periodic as P};
    let mut rows = Vec::new();
    let mut ok = true;
    let configurations: Vec<(Vec<usize>, Vec<f64>, Vec<f64>, Vec<Boundary>)> = vec![
        (vec![20], vec![1.0], vec![0.7], vec![P]),
        (vec![20], vec![1.0], vec![0.7], vec![N]),
        (vec![20], vec![1.0], vec![0.7], vec![D]),
        (vec![2], vec![1.0], vec![0.7], vec![P]),
        (vec![12, 10], vec![1.0, 2.0], vec![1.0, 0.3], vec![P, P]),
        (vec![12, 10], vec![1.0, 2.0], vec![1.0, 0.3], vec![N, D]),
        (vec![12, 10], vec![1.0, 2.0], vec![1.0, 0.3], vec![D, N]),
        (vec![6, 5, 4], vec![1.0, 2.0, 0.5], vec![1.0, 0.5, 0.1], vec![P, N, D]),
        (vec![6, 5, 4], vec![1.0, 1.0, 1.0], vec![1.0, 1.0, 1.0], vec![P, P, P]),
        (vec![6, 5, 4], vec![1.0, 1.0, 1.0], vec![1.0, 1.0, 1.0], vec![N, N, N]),
    ];
    for (cells, lengths, diffusivity, bc) in configurations {
        let grid = CartesianGrid::new(&cells, &lengths, &bc).unwrap();
        let op = CartesianLaplacian::new(grid.clone(), &diffusivity).unwrap();
        let n = op.dimension();
        let mut seed = 17 + n as u64;
        let u: Vec<f64> = (0..n).map(|_| 0.5 + lcg(&mut seed)).collect();
        let v: Vec<f64> = (0..n).map(|_| lcg(&mut seed) - 0.5).collect();
        let (bitwise, csc, fd) = jvp_checks(&op, &u, &v);
        let (outside, inside, diagonal) = pattern_checks(&op, &u);
        let s = op.structure();
        let weights_positive = s.weights().iter().all(|w| w.is_finite() && *w > 0.0);
        let volume: f64 = lengths.iter().product();
        let weight_sum = neumaier(s.weights().iter().copied());
        let weight_error = (weight_sum - volume).abs() / volume;
        let certificate = certify_weighted_self_adjoint(&op);
        // Independent random check of the certificate (as in loop node U09).
        let w = s.weights();
        let dot = |a: &[f64], b: &[f64]| -> f64 { (0..n).map(|i| w[i] * a[i] * b[i]).sum() };
        let (lu, lv) = (apply(&op, &u), apply(&op, &v));
        let sym = (dot(&u, &lv) - dot(&v, &lu)).abs()
            / (dot(&u, &u).sqrt() * dot(&lv, &lv).sqrt() + dot(&v, &v).sqrt() * dot(&lu, &lu).sqrt());
        let rayleigh = dot(&v, &lv) / dot(&v, &v);
        let rate_scale: f64 = op.rates().iter().map(|r| 4.0 * r).sum();
        let row_ok = bitwise
            && csc <= 1e-12
            && fd <= 1e-6
            && outside == 0.0
            && inside
            && diagonal
            && s.stencil_radius() == 1
            && weights_positive
            && weight_error <= 1e-12
            && certificate.is_ok()
            && sym <= 1e-12
            && rayleigh <= 1e-12 * rate_scale
            && s.order() == SpatialOrder::Second
            && s.is_linear();
        ok &= row_ok;
        let boundary: Vec<&str> = bc.iter().map(|b| bc_name(*b)).collect();
        rows.push(json!({
            "cells": cells, "boundaries": boundary, "n": n,
            "jvp_equals_apply_bitwise": bitwise, "csc_relative_defect": csc, "fd_relative_defect": fd,
            "largest_entry_outside_pattern": outside, "pattern_inside_bandwidth": inside,
            "bandwidth": [s.bandwidth().0, s.bandwidth().1], "diagonal_in_pattern": diagonal,
            "stencil_radius": s.stencil_radius(), "weights_positive": weights_positive,
            "weight_sum_relative_error": weight_error,
            "certificate": format!("{certificate:?}"),
            "random_symmetry_defect": sym, "random_rayleigh": rayleigh, "pass": row_ok,
        }));
    }
    qual("c4_c5_c6_cartesian", json!({"rows": rows, "pass": ok}));
    assert!(ok, "a JVP, structure or certificate check failed");
}

#[test]
fn constructors_refuse_invalid_input() {
    use Boundary::Periodic as P;
    assert!(CartesianGrid::new(&[], &[], &[]).is_err());
    assert!(CartesianGrid::new(&[1], &[1.0], &[P]).is_err());
    assert!(CartesianGrid::new(&[4, 4, 4, 4], &[1.0; 4], &[P; 4]).is_err());
    assert!(CartesianGrid::new(&[4], &[-1.0], &[P]).is_err());
    assert!(CartesianGrid::new(&[4], &[f64::NAN], &[P]).is_err());
    let grid = CartesianGrid::new(&[4, 4], &[1.0, 1.0], &[P, P]).unwrap();
    assert!(CartesianLaplacian::new(grid.clone(), &[1.0]).is_err());
    assert!(CartesianLaplacian::new(grid.clone(), &[1.0, -1.0]).is_err());
    let op = CartesianLaplacian::new(grid, &[1.0, 1.0]).unwrap();
    let mut out = vec![0.0; 15];
    assert!(op.apply_into(0.0, &[0.0; 16], &mut out).is_err());
    let mut values = vec![0.0; 3];
    assert!(op.jacobian_values_into(0.0, &[0.0; 16], &mut values).is_err());
}
