//! Contracts of the transport operators of `rodas5p_integrators::mol`: first-order upwind
//! advection and viscous Burgers with the Engquist-Osher flux (loop node V11, part a;
//! choices in `research/production_loop_20261116/battery_v4/PREDECLARED.json`).
//!
//! * C7: declared first order, every consecutive truncation order >= 0.9;
//! * C3: conservation with periodic boundaries, defect <= 1e-12;
//! * C4, C5: JVP and structure; C6: the certificate refuses both (advection as asymmetric,
//!   Burgers as nonlinear).

use std::f64::consts::PI;

use rodas5p_integrators::mol::{
    Boundary, Burgers, CartesianGrid, CertificateRefusal, MolOperator, SpatialOrder,
    UpwindAdvection, certify_weighted_self_adjoint,
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

fn conservation(op: &dyn MolOperator, u: &[f64]) -> (f64, f64) {
    let w = op.structure().weights();
    let f = apply(op, u);
    let values = assemble(op, u);
    let (_, abs) = csc_products(op, &values, u);
    let total = neumaier(f.iter().zip(w).map(|(a, b)| a * b));
    let scale = neumaier(abs.iter().zip(w).map(|(a, b)| a * b));
    (total.abs() / scale, total.abs())
}

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

fn pattern_checks(op: &dyn MolOperator, u: &[f64]) -> (f64, bool) {
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
    let inside = (0..n).all(|c| {
        (p.col_ptr()[c]..p.col_ptr()[c + 1]).all(|k| {
            let r = p.row_idx()[k];
            r <= c + lower && c <= r + upper
        })
    });
    (outside, inside)
}

/// Smooth periodic field `sum_a s_a(x_a)` with `s_0 = sin(2 pi x / L)`,
/// `s_1 = 0.5 cos(2 pi y / L)`, `s_2 = 0.25 sin(2 pi z / L)`: values, gradient, Laplacian.
fn periodic_field(grid: &CartesianGrid) -> (Vec<f64>, Vec<[f64; 3]>, Vec<f64>) {
    let n = grid.count();
    let (mut u, mut grad, mut lap) = (vec![0.0; n], vec![[0.0; 3]; n], vec![0.0; n]);
    for cell in 0..n {
        let x = grid.centre(cell);
        for a in 0..grid.dims() {
            let k = 2.0 * PI / grid.lengths()[a];
            let (s, c) = (k * x[a]).sin_cos();
            let (value, d1, d2) = match a {
                0 => (s, k * c, -k * k * s),
                1 => (0.5 * c, -0.5 * k * s, -0.5 * k * k * c),
                _ => (0.25 * s, 0.25 * k * c, -0.25 * k * k * s),
            };
            u[cell] += value;
            grad[cell][a] = d1;
            lap[cell] += d2;
        }
    }
    (u, grad, lap)
}

#[test]
fn c7_transport_operators_converge_at_first_order() {
    let mut rows = Vec::new();
    let mut pass = true;
    let velocity3 = [1.0, -0.6, 0.3];
    for (id, dims, ladder) in [
        ("advection-1D", 1usize, vec![32usize, 64, 128, 256]),
        ("advection-2D", 2, vec![16, 32, 64, 128]),
        ("burgers-2D", 2, vec![16, 32, 64, 128]),
    ] {
        let mut errors = Vec::new();
        for &m in &ladder {
            let cells = vec![m; dims];
            let lengths = vec![1.0; dims];
            let grid = CartesianGrid::new(&cells, &lengths, &[Boundary::Periodic; 3][..dims]).unwrap();
            let (u, grad, lap) = periodic_field(&grid);
            let (fh, exact): (Vec<f64>, Vec<f64>) = if id.starts_with("advection") {
                let op = UpwindAdvection::new(grid.clone(), &velocity3[..dims]).unwrap();
                assert_eq!(op.structure().order(), SpatialOrder::First);
                let exact = grad
                    .iter()
                    .map(|g| -(0..dims).map(|a| velocity3[a] * g[a]).sum::<f64>())
                    .collect();
                (apply(&op, &u), exact)
            } else {
                let nu = 0.01;
                let op = Burgers::new(grid.clone(), nu).unwrap();
                assert_eq!(op.structure().order(), SpatialOrder::First);
                let exact = (0..grid.count())
                    .map(|i| -u[i] * (0..dims).map(|a| grad[i][a]).sum::<f64>() + nu * lap[i])
                    .collect();
                (apply(&op, &u), exact)
            };
            errors.push(
                fh.iter()
                    .zip(&exact)
                    .fold(0.0f64, |acc, (a, b)| acc.max((a - b).abs()))
                    / max_abs(&exact),
            );
        }
        let o = orders(&errors);
        let ok = o.iter().all(|&x| x >= 0.9);
        pass &= ok;
        rows.push(json!({"case": id, "ladder": ladder, "errors": errors, "orders": o, "pass": ok}));
    }
    qual(
        "c7_transport_truncation",
        json!({"rows": rows, "rule": "declared first order: every consecutive order >= 0.9", "pass": pass}),
    );
    assert!(pass, "a transport operator converges below order 0.9");
}

#[test]
fn c3_periodic_transport_conserves_the_weighted_integral() {
    let mut rows = Vec::new();
    let mut worst = 0.0f64;
    let velocity3 = [1.0, -0.6, 0.3];
    for (id, dims, ladder) in [
        ("advection-1D", 1usize, vec![32usize, 64, 128, 256]),
        ("advection-2D", 2, vec![16, 32, 64, 128]),
        ("advection-3D", 3, vec![8, 16, 32]),
        ("burgers-1D", 1, vec![32, 64, 128, 256]),
        ("burgers-2D", 2, vec![16, 32, 64, 128]),
    ] {
        for &m in &ladder {
            let cells = vec![m; dims];
            let lengths = vec![1.0; dims];
            let grid = CartesianGrid::new(&cells, &lengths, &[Boundary::Periodic; 3][..dims]).unwrap();
            let op: Box<dyn MolOperator> = if id.starts_with("advection") {
                Box::new(UpwindAdvection::new(grid.clone(), &velocity3[..dims]).unwrap())
            } else {
                Box::new(Burgers::new(grid.clone(), 0.01).unwrap())
            };
            assert!(op.structure().is_conservative());
            let mut seed = 0x7a11 + m as u64;
            let random: Vec<f64> = (0..grid.count()).map(|_| 2.0 * lcg(&mut seed) - 1.0).collect();
            let (smooth, _, _) = periodic_field(&grid);
            for (field, u) in [("random", &random), ("smooth", &smooth)] {
                let (defect, absolute) = conservation(op.as_ref(), u);
                worst = worst.max(defect);
                rows.push(json!({"case": id, "m": m, "field": field, "defect": defect, "absolute": absolute}));
            }
        }
    }
    let pass = worst <= 1e-12;
    qual(
        "c3_transport_conservation",
        json!({"rows": rows, "max_defect": worst, "rule": "defect <= 1e-12", "pass": pass}),
    );
    assert!(pass, "transport conservation defect {worst:e}");
}

#[test]
fn c4_c5_c6_transport_jvp_structure_and_certificate_refusal() {
    use Boundary::{DirichletZero as D, Periodic as P};
    let mut rows = Vec::new();
    let mut ok = true;
    for (cells, bc) in [
        (vec![20usize], vec![P]),
        (vec![20], vec![D]),
        (vec![12, 10], vec![P, D]),
        (vec![12, 10], vec![D, P]),
        (vec![6, 5, 4], vec![P, P, D]),
    ] {
        let dims = cells.len();
        let lengths = vec![1.0; dims];
        let grid = CartesianGrid::new(&cells, &lengths, &bc).unwrap();
        let velocity = [0.8, -1.1, 0.4];
        let operators: Vec<(&str, Box<dyn MolOperator>)> = vec![
            (
                "advection",
                Box::new(UpwindAdvection::new(grid.clone(), &velocity[..dims]).unwrap()),
            ),
            ("burgers", Box::new(Burgers::new(grid.clone(), 0.02).unwrap())),
        ];
        for (name, op) in operators {
            let n = op.dimension();
            let mut seed = 5 + n as u64;
            let u: Vec<f64> = (0..n).map(|_| 2.0 * lcg(&mut seed) - 1.0).collect();
            let v: Vec<f64> = (0..n).map(|_| lcg(&mut seed) - 0.5).collect();
            let (bitwise, csc, fd) = jvp_checks(op.as_ref(), &u, &v);
            let (outside, inside) = pattern_checks(op.as_ref(), &u);
            let certificate = certify_weighted_self_adjoint(op.as_ref());
            let refused_as_expected = match name {
                "advection" => matches!(certificate, Err(CertificateRefusal::Asymmetric { .. })),
                _ => certificate == Err(CertificateRefusal::Nonlinear),
            };
            let w = op.structure().weights();
            let dot = |a: &[f64], b: &[f64]| -> f64 { (0..n).map(|i| w[i] * a[i] * b[i]).sum() };
            let (lu, lv) = (apply(op.as_ref(), &u), apply(op.as_ref(), &v));
            let sym = (dot(&u, &lv) - dot(&v, &lu)).abs()
                / (dot(&u, &u).sqrt() * dot(&lv, &lv).sqrt() + dot(&v, &v).sqrt() * dot(&lu, &lu).sqrt());
            let closed = bc.iter().all(|b| *b == P);
            let row_ok = bitwise
                && csc <= 1e-12
                && fd <= 1e-6
                && outside == 0.0
                && inside
                && op.structure().stencil_radius() == 1
                && refused_as_expected
                && op.structure().is_conservative() == closed;
            ok &= row_ok;
            let boundary: Vec<String> = bc.iter().map(|b| format!("{b:?}")).collect();
            rows.push(json!({
                "operator": name, "cells": cells, "boundaries": boundary, "n": n,
                "jvp_equals_apply_bitwise": bitwise, "csc_relative_defect": csc,
                "fd_relative_defect": fd, "largest_entry_outside_pattern": outside,
                "pattern_inside_bandwidth": inside, "certificate": format!("{certificate:?}"),
                "random_symmetry_defect": sym, "conservative_declared": op.structure().is_conservative(),
                "pass": row_ok,
            }));
        }
    }
    qual("c4_c5_c6_transport", json!({"rows": rows, "pass": ok}));
    assert!(ok, "a transport JVP, structure or certificate check failed");
}

#[test]
fn transport_operators_refuse_neumann_boundaries() {
    let grid = CartesianGrid::new(&[8, 8], &[1.0, 1.0], &[Boundary::Periodic, Boundary::NeumannZero])
        .unwrap();
    assert!(UpwindAdvection::new(grid.clone(), &[1.0, 1.0]).is_err());
    assert!(Burgers::new(grid, 0.1).is_err());
    let grid = CartesianGrid::new(&[8], &[1.0], &[Boundary::Periodic]).unwrap();
    assert!(UpwindAdvection::new(grid.clone(), &[f64::NAN]).is_err());
    assert!(UpwindAdvection::new(grid, &[1.0, 1.0]).is_err());
}
