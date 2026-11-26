//! Problems of PREDECLARED.json. New problems (step ratio, regimes): d4, kaps, burgers64,
//! ac1d1000. Cost-only problems (the consumed U11 holdout, definitions copied from
//! research/production_unification_20261115/production_holdout/runner/src/main.rs): oregonator,
//! hires, brusselator_mol32. All are autonomous with analytic Jacobians written row major into a
//! zero-filled `n x n` slice.

use std::hint::black_box;
use std::sync::Arc;

use rodas5p_core::{CoreResult, DenseMatrix};
use vigilode::{CallbackError, Jacobian, Problem};

use crate::OdeProblem;

// Signatures of `rodas5p_integrators::problem::{RhsFn, JacobianFn}` (not re-exported), as in
// the PeerOde adapter.
type CoreRhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type CoreJacobianFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pid {
    D4,
    Kaps,
    Burgers64,
    Ac1d1000,
    Oregonator,
    Hires,
    BrussMol32,
}

pub const NEW_PROBLEMS: [Pid; 4] = [Pid::D4, Pid::Kaps, Pid::Burgers64, Pid::Ac1d1000];

const KAPS_EPS: f64 = 1e-4;
const BURGERS_N: usize = 64;
const BURGERS_NU: f64 = 0.05;
const AC_N: usize = 1000;
const AC_D: f64 = 1e-3;
const BN: usize = 32;
const BA: f64 = 1.0;
const BB: f64 = 3.0;
const BALPHA: f64 = 1.0 / 50.0;

impl Pid {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "d4" => Pid::D4,
            "kaps" => Pid::Kaps,
            "burgers64" => Pid::Burgers64,
            "ac1d1000" => Pid::Ac1d1000,
            "oregonator" => Pid::Oregonator,
            "hires" => Pid::Hires,
            "brusselator_mol32" => Pid::BrussMol32,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Pid::D4 => "d4",
            Pid::Kaps => "kaps",
            Pid::Burgers64 => "burgers64",
            Pid::Ac1d1000 => "ac1d1000",
            Pid::Oregonator => "oregonator",
            Pid::Hires => "hires",
            Pid::BrussMol32 => "brusselator_mol32",
        }
    }

    pub fn n(self) -> usize {
        match self {
            Pid::D4 => 3,
            Pid::Kaps => 2,
            Pid::Burgers64 => BURGERS_N,
            Pid::Ac1d1000 => AC_N,
            Pid::Oregonator => 3,
            Pid::Hires => 8,
            Pid::BrussMol32 => 2 * BN,
        }
    }

    pub fn span(self) -> (f64, f64) {
        match self {
            Pid::D4 => (0.0, 50.0),
            Pid::Kaps => (0.0, 1.0),
            Pid::Burgers64 => (0.0, 1.0),
            Pid::Ac1d1000 => (0.0, 5.0),
            Pid::Oregonator => (0.0, 360.0),
            Pid::Hires => (0.0, 321.8122),
            Pid::BrussMol32 => (0.0, 10.0),
        }
    }

    pub fn y0(self) -> Vec<f64> {
        match self {
            Pid::D4 => vec![1.0, 1.0, 0.0],
            Pid::Kaps => vec![1.0, 1.0],
            Pid::Burgers64 => {
                let dx = 1.0 / (BURGERS_N as f64 + 1.0);
                (1..=BURGERS_N)
                    .map(|i| (2.0 * std::f64::consts::PI * i as f64 * dx).sin())
                    .collect()
            }
            Pid::Ac1d1000 => {
                let dx = 1.0 / (AC_N as f64 + 1.0);
                (1..=AC_N)
                    .map(|i| {
                        let x = i as f64 * dx;
                        -(std::f64::consts::PI * x).cos()
                            + 0.4 * (6.0 * std::f64::consts::PI * x).sin()
                    })
                    .collect()
            }
            Pid::Oregonator => vec![1.0, 2.0, 3.0],
            Pid::Hires => vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
            Pid::BrussMol32 => {
                let dx = 1.0 / (BN as f64 + 1.0);
                let mut y: Vec<f64> = (1..=BN)
                    .map(|i| 1.0 + (2.0 * std::f64::consts::PI * i as f64 * dx).sin())
                    .collect();
                y.extend(std::iter::repeat_n(3.0, BN));
                y
            }
        }
    }

    /// The right-hand side `f(y)` written into `d`.
    pub fn rhs(self, y: &[f64], d: &mut [f64]) {
        match self {
            Pid::D4 => {
                d[0] = -0.013 * y[0] - 1000.0 * y[0] * y[2];
                d[1] = -2500.0 * y[1] * y[2];
                d[2] = -0.013 * y[0] - 1000.0 * y[0] * y[2] - 2500.0 * y[1] * y[2];
            }
            Pid::Kaps => {
                d[0] = -(2.0 + 1.0 / KAPS_EPS) * y[0] + y[1] * y[1] / KAPS_EPS;
                d[1] = y[0] - y[1] - y[1] * y[1];
            }
            Pid::Burgers64 => {
                let n = BURGERS_N;
                let dx = 1.0 / (n as f64 + 1.0);
                let a = 1.0 / (4.0 * dx);
                let c = BURGERS_NU / (dx * dx);
                for i in 0..n {
                    let ul = if i > 0 { y[i - 1] } else { 0.0 };
                    let ur = if i + 1 < n { y[i + 1] } else { 0.0 };
                    d[i] = -(ur * ur - ul * ul) * a + c * (ur - 2.0 * y[i] + ul);
                }
            }
            Pid::Ac1d1000 => {
                let n = AC_N;
                let dx = 1.0 / (n as f64 + 1.0);
                let c = AC_D / (dx * dx);
                for i in 0..n {
                    let ul = if i > 0 { y[i - 1] } else { -1.0 };
                    let ur = if i + 1 < n { y[i + 1] } else { 1.0 };
                    let u = y[i];
                    d[i] = c * (ul - 2.0 * u + ur) + u - u * u * u;
                }
            }
            Pid::Oregonator => {
                d[0] = 77.27 * (y[1] + y[0] * (1.0 - 8.375e-6 * y[0] - y[1]));
                d[1] = (y[2] - (1.0 + y[0]) * y[1]) / 77.27;
                d[2] = 0.161 * (y[0] - y[2]);
            }
            Pid::Hires => {
                d[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
                d[1] = 1.71 * y[0] - 8.75 * y[1];
                d[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
                d[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
                d[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
                d[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5]
                    + 0.69 * y[6];
                d[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
                d[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
            }
            Pid::BrussMol32 => {
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
        }
    }

    /// The Jacobian, row major, into `j` (`n * n`); every entry is written.
    pub fn jac(self, y: &[f64], j: &mut [f64]) {
        j.fill(0.0);
        let n = self.n();
        let mut s = |r: usize, c: usize, v: f64| j[r * n + c] = v;
        match self {
            Pid::D4 => {
                s(0, 0, -0.013 - 1000.0 * y[2]);
                s(0, 2, -1000.0 * y[0]);
                s(1, 1, -2500.0 * y[2]);
                s(1, 2, -2500.0 * y[1]);
                s(2, 0, -0.013 - 1000.0 * y[2]);
                s(2, 1, -2500.0 * y[2]);
                s(2, 2, -1000.0 * y[0] - 2500.0 * y[1]);
            }
            Pid::Kaps => {
                s(0, 0, -(2.0 + 1.0 / KAPS_EPS));
                s(0, 1, 2.0 * y[1] / KAPS_EPS);
                s(1, 0, 1.0);
                s(1, 1, -1.0 - 2.0 * y[1]);
            }
            Pid::Burgers64 => {
                let dx = 1.0 / (n as f64 + 1.0);
                let c = BURGERS_NU / (dx * dx);
                let a = 1.0 / (2.0 * dx);
                for i in 0..n {
                    s(i, i, -2.0 * c);
                    if i > 0 {
                        s(i, i - 1, y[i - 1] * a + c);
                    }
                    if i + 1 < n {
                        s(i, i + 1, -y[i + 1] * a + c);
                    }
                }
            }
            Pid::Ac1d1000 => {
                let dx = 1.0 / (n as f64 + 1.0);
                let c = AC_D / (dx * dx);
                for i in 0..n {
                    s(i, i, -2.0 * c + 1.0 - 3.0 * y[i] * y[i]);
                    if i > 0 {
                        s(i, i - 1, c);
                    }
                    if i + 1 < n {
                        s(i, i + 1, c);
                    }
                }
            }
            Pid::Oregonator => {
                let v = [
                    77.27 * (1.0 - 2.0 * 8.375e-6 * y[0] - y[1]),
                    77.27 * (1.0 - y[0]),
                    0.0,
                    -y[1] / 77.27,
                    -(1.0 + y[0]) / 77.27,
                    1.0 / 77.27,
                    0.161,
                    0.0,
                    -0.161,
                ];
                for (k, x) in v.into_iter().enumerate() {
                    s(k / 3, k % 3, x);
                }
            }
            Pid::Hires => {
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
            Pid::BrussMol32 => {
                let dx = 1.0 / (BN as f64 + 1.0);
                let c = BALPHA / (dx * dx);
                for i in 0..BN {
                    let (u, v) = (y[i], y[BN + i]);
                    s(i, i, 2.0 * u * v - (BB + 1.0) - 2.0 * c);
                    s(i, BN + i, u * u);
                    s(BN + i, i, BB - 2.0 * u * v);
                    s(BN + i, BN + i, -u * u - 2.0 * c);
                    if i > 0 {
                        s(i, i - 1, c);
                        s(BN + i, BN + i - 1, c);
                    }
                    if i + 1 < BN {
                        s(i, i + 1, c);
                        s(BN + i, BN + i + 1, c);
                    }
                }
            }
        }
    }

    /// The right-hand side evaluated `mult` times; the bits of one evaluation are returned
    /// (the regimes W1 and W2 of PREDECLARED.json).
    pub fn rhs_mult(self, mult: u32, y: &[f64], d: &mut [f64]) {
        for _ in 0..mult.max(1) {
            self.rhs(black_box(y), d);
            black_box(&mut *d);
        }
    }
}

/// The facade problem (dense Jacobian, autonomous) for `Rodas5p::solve` and `PeerOde::solve`.
pub fn facade_problem(pid: Pid, mult: u32) -> Problem {
    Problem::new(pid.n(), move |_t, y, d| {
        pid.rhs_mult(mult, y, d);
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(move |_t, y, m| {
        pid.jac(y, m);
        Ok(())
    }))
    .autonomous()
}

/// The core problem the research driver integrates, built as `PeerOde::lower` builds its own
/// (a zero-filled dense matrix handed to the Jacobian callback; autonomous).
pub fn core_problem(pid: Pid, mult: u32) -> CoreResult<OdeProblem> {
    let n = pid.n();
    let rhs: CoreRhsFn = Arc::new(move |_t, y: &[f64], d: &mut [f64]| {
        pid.rhs_mult(mult, y, d);
        Ok(())
    });
    let jac: CoreJacobianFn = Arc::new(move |_t, y: &[f64]| {
        let mut m = DenseMatrix::zeros(n, n);
        pid.jac(y, m.as_mut_slice());
        Ok(m)
    });
    OdeProblem::new(
        format!("v10-{}", pid.name()),
        n,
        rhs,
        None,
        Some(jac),
        None,
        None,
        true,
        None,
        None,
    )
}

/// Scalar test problems of the executed stability runs: `y' = lambda y` (`pr = false`) and the
/// Prothero-Robinson problem `y' = lambda (y - cos t) - sin t` (`pr = true`).
pub fn scalar_problem(lambda: f64, pr: bool) -> CoreResult<OdeProblem> {
    let rhs: CoreRhsFn = Arc::new(move |t, y: &[f64], d: &mut [f64]| {
        d[0] = if pr {
            lambda * (y[0] - t.cos()) - t.sin()
        } else {
            lambda * y[0]
        };
        Ok(())
    });
    let jac: CoreJacobianFn = Arc::new(move |_t, _y: &[f64]| {
        let mut m = DenseMatrix::zeros(1, 1);
        m.as_mut_slice()[0] = lambda;
        Ok(m)
    });
    OdeProblem::new(
        if pr { "v10-pr" } else { "v10-lin" },
        1,
        rhs,
        None,
        Some(jac),
        None,
        None,
        !pr,
        None,
        None,
    )
}
