//! Shared frozen U11 holdout equations, initial data and analytic Jacobians.
use vigilode::{Jacobian, Problem};

pub(crate) struct Spec {
    pub(crate) name: &'static str,
    pub(crate) n: usize,
    pub(crate) t_end: f64,
    pub(crate) y0: Vec<f64>,
    pub(crate) f: fn(&[f64], &mut [f64]),
    pub(crate) j: fn(&[f64], &mut [f64]),
}

fn hires_f(y: &[f64], d: &mut [f64]) {
    d[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
    d[1] = 1.71 * y[0] - 8.75 * y[1];
    d[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
    d[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
    d[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
    d[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
    d[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
    d[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
}

fn hires_j(y: &[f64], j: &mut [f64]) {
    let n = 8;
    j.fill(0.0);
    let mut s = |r: usize, c: usize, v: f64| j[r * n + c] = v;
    s(0, 0, -1.71);
    s(0, 1, 0.43);
    s(0, 2, 8.32);
    s(1, 0, 1.71);
    s(1, 1, -8.75);
    s(2, 2, -10.03);
    s(2, 3, 0.43);
    s(2, 4, 0.035);
    s(3, 1, 8.32);
    s(3, 2, 1.71);
    s(3, 3, -1.12);
    s(4, 4, -1.745);
    s(4, 5, 0.43);
    s(4, 6, 0.43);
    s(5, 3, 0.69);
    s(5, 4, 1.71);
    s(5, 5, -0.43 - 280.0 * y[7]);
    s(5, 6, 0.69);
    s(5, 7, -280.0 * y[5]);
    s(6, 5, 280.0 * y[7]);
    s(6, 6, -1.81);
    s(6, 7, 280.0 * y[5]);
    s(7, 5, -280.0 * y[7]);
    s(7, 6, 1.81);
    s(7, 7, -280.0 * y[5]);
}

fn oreg_f(y: &[f64], d: &mut [f64]) {
    d[0] = 77.27 * (y[1] + y[0] * (1.0 - 8.375e-6 * y[0] - y[1]));
    d[1] = (y[2] - (1.0 + y[0]) * y[1]) / 77.27;
    d[2] = 0.161 * (y[0] - y[2]);
}

fn oreg_j(y: &[f64], j: &mut [f64]) {
    j.copy_from_slice(&[
        77.27 * (1.0 - 2.0 * 8.375e-6 * y[0] - y[1]),
        77.27 * (1.0 - y[0]),
        0.0,
        -y[1] / 77.27,
        -(1.0 + y[0]) / 77.27,
        1.0 / 77.27,
        0.161,
        0.0,
        -0.161,
    ]);
}

const BN: usize = 32;
const BA: f64 = 1.0;
const BB: f64 = 3.0;
const BALPHA: f64 = 1.0 / 50.0;

fn bruss_f(y: &[f64], d: &mut [f64]) {
    let dx = 1.0 / (BN as f64 + 1.0);
    let c = BALPHA / (dx * dx);
    for i in 0..BN {
        let (u, v) = (y[i], y[BN + i]);
        let ul = if i > 0 { y[i - 1] } else { 1.0 };
        let ur = if i + 1 < BN { y[i + 1] } else { 1.0 };
        let vl = if i > 0 { y[BN + i - 1] } else { 3.0 };
        let vr = if i + 1 < BN { y[BN + i + 1] } else { 3.0 };
        d[i] = BA + u * u * v - (BB + 1.0) * u + c * (ul - 2.0 * u + ur);
        d[BN + i] = BB * u - u * u * v + c * (vl - 2.0 * v + vr);
    }
}

fn bruss_j(y: &[f64], j: &mut [f64]) {
    let n = 2 * BN;
    let dx = 1.0 / (BN as f64 + 1.0);
    let c = BALPHA / (dx * dx);
    j.fill(0.0);
    for i in 0..BN {
        let (u, v) = (y[i], y[BN + i]);
        j[i * n + i] = 2.0 * u * v - (BB + 1.0) - 2.0 * c;
        j[i * n + BN + i] = u * u;
        j[(BN + i) * n + i] = BB - 2.0 * u * v;
        j[(BN + i) * n + BN + i] = -u * u - 2.0 * c;
        if i > 0 {
            j[i * n + i - 1] = c;
            j[(BN + i) * n + BN + i - 1] = c;
        }
        if i + 1 < BN {
            j[i * n + i + 1] = c;
            j[(BN + i) * n + BN + i + 1] = c;
        }
    }
}

pub(crate) fn specs() -> Vec<Spec> {
    let dx = 1.0 / (BN as f64 + 1.0);
    let mut by0: Vec<f64> = (1..=BN)
        .map(|i| 1.0 + (2.0 * std::f64::consts::PI * i as f64 * dx).sin())
        .collect();
    by0.extend(std::iter::repeat_n(3.0, BN));
    vec![
        Spec {
            name: "hires",
            n: 8,
            t_end: 321.8122,
            y0: vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
            f: hires_f,
            j: hires_j,
        },
        Spec {
            name: "oregonator",
            n: 3,
            t_end: 360.0,
            y0: vec![1.0, 2.0, 3.0],
            f: oreg_f,
            j: oreg_j,
        },
        Spec {
            name: "brusselator_mol32",
            n: 2 * BN,
            t_end: 10.0,
            y0: by0,
            f: bruss_f,
            j: bruss_j,
        },
    ]
}

pub(crate) fn problem(s: &Spec) -> Problem {
    let (f, j) = (s.f, s.j);
    Problem::new(s.n, move |_t, y, d| {
        f(y, d);
        Ok(())
    })
    .jacobian(Jacobian::dense(move |_t, y, m| {
        j(y, m);
        Ok(())
    }))
    .autonomous()
}
