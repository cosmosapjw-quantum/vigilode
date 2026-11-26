//! Contracts of the latitude-longitude sphere Laplacian of `rodas5p_integrators::mol`
//! (loop node V11, part a; choices in
//! `research/production_loop_20261116/battery_v4/PREDECLARED.json`, C2 to C6).
//!
//! * C2: second-order truncation error on spherical harmonics in the band of cell centres
//!   with colatitude in `[pi/6, 5 pi/6]` (frozen discriminator, every consecutive order
//!   >= 1.9); the two pole rings reported separately, and their stated first-order
//!   behaviour for `Y_1^1` tested (orders in `[0.8, 1.2]`);
//! * C3: conservation of the area-weighted integral, defect <= 1e-12;
//! * C4 to C6: JVP, structure, weights summing to `4 pi R^2`, weighted certificate.

use std::f64::consts::PI;

use rodas5p_integrators::mol::{
    MolOperator, SphereGrid, SphereLaplacian, certify_weighted_self_adjoint,
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

type Harmonic = (&'static str, f64, fn(f64, f64) -> f64);

fn harmonics() -> [Harmonic; 5] {
    [
        ("Y10", 1.0, |t, _| t.cos()),
        ("Y20", 2.0, |t, _| 1.5 * t.cos().powi(2) - 0.5),
        ("Y11", 1.0, |t, p| t.sin() * p.cos()),
        ("Y21", 2.0, |t, p| t.sin() * t.cos() * p.cos()),
        ("Y22", 2.0, |t, p| t.sin().powi(2) * (2.0 * p).cos()),
    ]
}

fn field(grid: &SphereGrid, f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    (0..grid.count())
        .map(|cell| {
            let (t, p) = grid.centre(cell);
            f(t, p)
        })
        .collect()
}

const RADIUS: f64 = 2.0;
const DIFFUSIVITY: f64 = 0.5;
const LADDER: [usize; 4] = [16, 32, 64, 128];

#[test]
fn c2_second_order_away_from_the_poles_and_first_order_pole_rings() {
    let modes = harmonics();
    let mut band_errors: Vec<Vec<f64>> = vec![Vec::new(); modes.len()];
    let mut pole_errors: Vec<Vec<f64>> = vec![Vec::new(); modes.len()];
    let mut weighted: Vec<Vec<f64>> = vec![Vec::new(); modes.len()];
    let mut levels = Vec::new();
    for nt in LADDER {
        let grid = SphereGrid::new(nt, 2 * nt, RADIUS).unwrap();
        let op = SphereLaplacian::new(grid.clone(), DIFFUSIVITY).unwrap();
        let np = grid.longitudes();
        let w = op.structure().weights().to_vec();
        let mut band_cells = 0usize;
        for cell in 0..grid.count() {
            let (t, _) = grid.centre(cell);
            if (PI / 6.0..=5.0 * PI / 6.0).contains(&t) {
                band_cells += 1;
            }
        }
        for (m, (_, l, f)) in modes.iter().enumerate() {
            let y = field(&grid, f);
            let ly = apply(&op, &y);
            let lambda = -DIFFUSIVITY * l * (l + 1.0) / (RADIUS * RADIUS);
            let scale = lambda.abs() * max_abs(&y);
            let mut band = 0.0f64;
            let mut pole = 0.0f64;
            for cell in 0..grid.count() {
                let (t, _) = grid.centre(cell);
                let e = (ly[cell] - lambda * y[cell]).abs() / scale;
                if (PI / 6.0..=5.0 * PI / 6.0).contains(&t) {
                    band = band.max(e);
                }
                let ring = cell / np;
                if ring == 0 || ring + 1 == nt {
                    pole = pole.max(e);
                }
            }
            let num = neumaier(
                ly.iter()
                    .zip(&y)
                    .zip(&w)
                    .map(|((a, b), c)| c * (a - lambda * b).powi(2)),
            );
            let den = neumaier(y.iter().zip(&w).map(|(b, c)| c * (lambda * b).powi(2)));
            band_errors[m].push(band);
            pole_errors[m].push(pole);
            weighted[m].push((num / den).sqrt());
        }
        levels.push(json!({"rings": nt, "longitudes": np, "band_cells": band_cells}));
    }
    let mut rows = Vec::new();
    let mut pass = true;
    let mut pole_pass = true;
    for (m, (name, l, _)) in modes.iter().enumerate() {
        let band_orders = orders(&band_errors[m]);
        let pole_orders = orders(&pole_errors[m]);
        let ok = band_orders.iter().all(|&x| x >= 1.9);
        pass &= ok;
        if *name == "Y11" {
            pole_pass &= pole_orders.iter().all(|&x| (0.8..=1.2).contains(&x));
        }
        rows.push(json!({"mode": name, "degree": l,
                         "band_errors": band_errors[m], "band_orders": band_orders, "band_pass": ok,
                         "pole_ring_errors": pole_errors[m], "pole_ring_orders": pole_orders,
                         "weighted_l2_errors": weighted[m], "weighted_l2_orders": orders(&weighted[m])}));
    }
    qual(
        "c2_sphere_convergence",
        json!({"radius": RADIUS, "diffusivity": DIFFUSIVITY, "levels": levels,
               "band": "cell centres with colatitude in [pi/6, 5 pi/6]",
               "rows": rows, "band_rule": "every consecutive band order >= 1.9",
               "band_pass": pass, "pole_rule": "Y11 pole-ring orders in [0.8, 1.2]",
               "pole_first_order_pass": pole_pass}),
    );
    assert!(pass, "a harmonic converges below order 1.9 away from the poles");
    assert!(
        pole_pass,
        "the stated first-order pole-ring behaviour of Y11 is not reproduced"
    );
}

#[test]
fn c3_sphere_conserves_the_area_weighted_integral() {
    let mut rows = Vec::new();
    let mut worst = 0.0f64;
    for nt in LADDER {
        let grid = SphereGrid::new(nt, 2 * nt, RADIUS).unwrap();
        let op = SphereLaplacian::new(grid.clone(), DIFFUSIVITY).unwrap();
        assert!(op.structure().is_conservative());
        let mut seed = 0x5bea_0000 + nt as u64;
        let random: Vec<f64> = (0..grid.count()).map(|_| 0.5 + lcg(&mut seed)).collect();
        let smooth = field(&grid, |t, p| {
            1.0 + (1.5 * t.cos().powi(2) - 0.5)
                + 0.5 * t.sin() * p.cos()
                + 0.25 * t.sin().powi(2) * (2.0 * p).cos()
        });
        for (name, u) in [("random", &random), ("smooth", &smooth)] {
            let (defect, absolute) = conservation(&op, u);
            worst = worst.max(defect);
            rows.push(json!({"rings": nt, "field": name, "defect": defect, "absolute": absolute}));
        }
    }
    let pass = worst <= 1e-12;
    qual(
        "c3_sphere_conservation",
        json!({"rows": rows, "max_defect": worst, "rule": "defect <= 1e-12", "pass": pass}),
    );
    assert!(pass, "sphere conservation defect {worst:e}");
}

#[test]
fn c4_c5_c6_sphere_jvp_structure_weights_and_certificate() {
    let mut rows = Vec::new();
    let mut ok = true;
    for (nt, np) in [(8usize, 16usize), (5, 7), (4, 2)] {
        let grid = SphereGrid::new(nt, np, RADIUS).unwrap();
        let op = SphereLaplacian::new(grid.clone(), DIFFUSIVITY).unwrap();
        let n = op.dimension();
        let mut seed = 99 + n as u64;
        let u: Vec<f64> = (0..n).map(|_| 0.5 + lcg(&mut seed)).collect();
        let v: Vec<f64> = (0..n).map(|_| lcg(&mut seed) - 0.5).collect();
        let mut jvp = vec![0.0; n];
        op.jvp_into(0.0, &u, &v, &mut jvp).unwrap();
        let bitwise = apply(&op, &v)
            .iter()
            .zip(&jvp)
            .all(|(a, b)| a.to_bits() == b.to_bits());
        let values = assemble(&op, &u);
        let (jv, abs) = csc_products(&op, &values, &v);
        let scale = max_abs(&abs);
        let csc = jv
            .iter()
            .zip(&jvp)
            .fold(0.0f64, |m, (a, b)| m.max((a - b).abs()))
            / scale;
        let delta = 1e-6 * max_abs(&u).max(1.0) / max_abs(&v);
        let up: Vec<f64> = u.iter().zip(&v).map(|(a, b)| a + delta * b).collect();
        let um: Vec<f64> = u.iter().zip(&v).map(|(a, b)| a - delta * b).collect();
        let (fp, fm) = (apply(&op, &up), apply(&op, &um));
        let fd = fp
            .iter()
            .zip(&fm)
            .zip(&jvp)
            .fold(0.0f64, |m, ((a, b), c)| m.max(((a - b) / (2.0 * delta) - c).abs()))
            / scale;
        let s = op.structure();
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
        let weights_positive = s.weights().iter().all(|w| w.is_finite() && *w > 0.0);
        let area = 4.0 * PI * RADIUS * RADIUS;
        let weight_error = (neumaier(s.weights().iter().copied()) - area).abs() / area;
        let certificate = certify_weighted_self_adjoint(&op);
        let w = s.weights();
        let dot = |a: &[f64], b: &[f64]| -> f64 { (0..n).map(|i| w[i] * a[i] * b[i]).sum() };
        let (lu, lv) = (apply(&op, &u), apply(&op, &v));
        let sym = (dot(&u, &lv) - dot(&v, &lu)).abs()
            / (dot(&u, &u).sqrt() * dot(&lv, &lv).sqrt() + dot(&v, &v).sqrt() * dot(&lu, &lu).sqrt());
        let rayleigh = dot(&v, &lv) / dot(&v, &v);
        let constant = max_abs(&apply(&op, &vec![1.0; n]));
        let row_ok = bitwise
            && csc <= 1e-12
            && fd <= 1e-6
            && outside == 0.0
            && inside
            && s.stencil_radius() == 1
            && weights_positive
            && weight_error <= 1e-12
            && certificate.is_ok()
            && sym <= 1e-12
            && rayleigh <= 0.0
            && constant == 0.0;
        ok &= row_ok;
        rows.push(json!({
            "rings": nt, "longitudes": np, "n": n,
            "jvp_equals_apply_bitwise": bitwise, "csc_relative_defect": csc, "fd_relative_defect": fd,
            "largest_entry_outside_pattern": outside, "pattern_inside_bandwidth": inside,
            "bandwidth": [lower, upper], "stencil_radius": s.stencil_radius(),
            "weights_positive": weights_positive, "weight_sum_relative_error": weight_error,
            "certificate": format!("{certificate:?}"), "random_symmetry_defect": sym,
            "random_rayleigh": rayleigh, "constant_mode_max": constant, "pass": row_ok,
        }));
    }
    qual("c4_c5_c6_sphere", json!({"rows": rows, "pass": ok}));
    assert!(ok, "a sphere JVP, structure or certificate check failed");
}

#[test]
fn sphere_constructors_refuse_invalid_input() {
    assert!(SphereGrid::new(1, 8, 1.0).is_err());
    assert!(SphereGrid::new(8, 1, 1.0).is_err());
    assert!(SphereGrid::new(8, 16, 0.0).is_err());
    assert!(SphereGrid::new(8, 16, f64::INFINITY).is_err());
    let grid = SphereGrid::new(8, 16, 1.0).unwrap();
    assert!(SphereLaplacian::new(grid.clone(), -1.0).is_err());
    assert!(SphereLaplacian::new(grid, f64::NAN).is_err());
}
