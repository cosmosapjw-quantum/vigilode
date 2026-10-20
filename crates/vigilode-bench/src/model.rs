//! The one physical model per problem, shared by every arm.
//!
//! Every arm evaluates the right-hand side through [`Model::rhs`] and the
//! Jacobian through [`Model::jac_entries`] (directly, or via one of the
//! layout writers below). The callbacks increment process-wide counters so
//! that the evaluation work of every arm is measured by the same authority,
//! independently of what the solver itself reports.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// The problem kinds of corpus v1 plus the linear test problem with an exact
/// solution used by the adapter-equivalence tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Robertson chemical kinetics, n = 3.
    Robertson,
    /// HIRES (Schäfer 1975), n = 8.
    Hires,
    /// Van der Pol with mu = 1000 in the unscaled form y1' = y2,
    /// y2' = mu (1 - y1^2) y2 - y1.
    VanDerPolMu1000,
    /// Brusselator 1D reaction-diffusion, interleaved (u_k, v_k), Dirichlet
    /// u = 1, v = 3 at both ends, alpha = 1/50; n = 2m.
    Brusselator1d,
    /// y1' = -y1, y2' = 1000 (y1 - y2), y(0) = (1, 0): exact solution
    /// y1 = e^-t, y2 = 1000/999 (e^-t - e^-1000t).
    LinearExact,
}

/// A problem instance: kind and dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    /// Problem kind.
    pub kind: Kind,
    /// Dimension.
    pub n: usize,
}

static RHS_CALLS: AtomicU64 = AtomicU64::new(0);
static JAC_CALLS: AtomicU64 = AtomicU64::new(0);
static JVP_CALLS: AtomicU64 = AtomicU64::new(0);

/// Callback counts measured by the shared model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallbackCounts {
    /// Right-hand-side evaluations.
    pub rhs: u64,
    /// Full Jacobian evaluations (any layout).
    pub jac: u64,
    /// Jacobian-vector products (Diffsol assembles its Jacobian from these).
    pub jvp: u64,
}

/// Reset the callback counters (call before a run).
pub fn reset_counts() {
    RHS_CALLS.store(0, Ordering::SeqCst);
    JAC_CALLS.store(0, Ordering::SeqCst);
    JVP_CALLS.store(0, Ordering::SeqCst);
}

/// Read the callback counters.
pub fn counts() -> CallbackCounts {
    CallbackCounts {
        rhs: RHS_CALLS.load(Ordering::SeqCst),
        jac: JAC_CALLS.load(Ordering::SeqCst),
        jvp: JVP_CALLS.load(Ordering::SeqCst),
    }
}

impl Model {
    /// Build and check a model.
    pub fn new(kind: Kind, n: usize) -> Result<Self, String> {
        let ok = match kind {
            Kind::Robertson => n == 3,
            Kind::Hires => n == 8,
            Kind::VanDerPolMu1000 | Kind::LinearExact => n == 2,
            Kind::Brusselator1d => n >= 4 && n.is_multiple_of(2),
        };
        if ok {
            Ok(Self { kind, n })
        } else {
            Err(format!("dimension {n} is not valid for {kind:?}"))
        }
    }

    /// The initial state.
    pub fn y0(&self) -> Vec<f64> {
        match self.kind {
            Kind::Robertson => vec![1.0, 0.0, 0.0],
            Kind::Hires => vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
            Kind::VanDerPolMu1000 => vec![2.0, 0.0],
            Kind::Brusselator1d => {
                let m = self.n / 2;
                (0..m)
                    .flat_map(|i| {
                        let x = (i + 1) as f64 / (m + 1) as f64;
                        [1.0 + (2.0 * std::f64::consts::PI * x).sin(), 3.0]
                    })
                    .collect()
            }
            Kind::LinearExact => vec![1.0, 0.0],
        }
    }

    /// Lower and upper half-bandwidths of the Jacobian.
    pub fn band(&self) -> (usize, usize) {
        match self.kind {
            Kind::Brusselator1d => (2, 2),
            Kind::LinearExact => (1, 0),
            _ => (self.n - 1, self.n - 1),
        }
    }

    /// The exact solution, where known.
    pub fn exact(&self, t: f64) -> Option<Vec<f64>> {
        match self.kind {
            Kind::LinearExact => {
                let a = (-t).exp();
                Some(vec![a, 1000.0 / 999.0 * (a - (-1000.0 * t).exp())])
            }
            _ => None,
        }
    }

    /// The right-hand side f(t, y) into `dy`.
    pub fn rhs(&self, _t: f64, y: &[f64], dy: &mut [f64]) {
        RHS_CALLS.fetch_add(1, Ordering::Relaxed);
        match self.kind {
            Kind::Robertson => {
                dy[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
                dy[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
                dy[2] = 3e7 * y[1] * y[1];
            }
            Kind::Hires => {
                dy[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
                dy[1] = 1.71 * y[0] - 8.75 * y[1];
                dy[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
                dy[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
                dy[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
                dy[5] =
                    -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
                dy[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
                dy[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
            }
            Kind::VanDerPolMu1000 => {
                dy[0] = y[1];
                dy[1] = 1000.0 * (1.0 - y[0] * y[0]) * y[1] - y[0];
            }
            Kind::Brusselator1d => {
                let m = self.n / 2;
                let d = ((m + 1) as f64).powi(2) / 50.0;
                for k in 0..m {
                    let (a, b) = (2 * k, 2 * k + 1);
                    let (u, v) = (y[a], y[b]);
                    let (ul, vl) = if k == 0 {
                        (1.0, 3.0)
                    } else {
                        (y[a - 2], y[b - 2])
                    };
                    let (ur, vr) = if k + 1 == m {
                        (1.0, 3.0)
                    } else {
                        (y[a + 2], y[b + 2])
                    };
                    dy[a] = 1.0 + u * u * v - 4.0 * u + d * (ul - 2.0 * u + ur);
                    dy[b] = 3.0 * u - u * u * v + d * (vl - 2.0 * v + vr);
                }
            }
            Kind::LinearExact => {
                dy[0] = -y[0];
                dy[1] = 1000.0 * y[0] - 1000.0 * y[1];
            }
        }
    }

    /// Emit every structurally nonzero Jacobian entry (i, j, df_i/dy_j).
    /// Counts as one Jacobian evaluation; the layout writers below call it.
    fn entries(&self, y: &[f64], mut emit: impl FnMut(usize, usize, f64)) {
        match self.kind {
            Kind::Robertson => {
                emit(0, 0, -0.04);
                emit(0, 1, 1e4 * y[2]);
                emit(0, 2, 1e4 * y[1]);
                emit(1, 0, 0.04);
                emit(1, 1, -1e4 * y[2] - 6e7 * y[1]);
                emit(1, 2, -1e4 * y[1]);
                emit(2, 1, 6e7 * y[1]);
            }
            Kind::Hires => {
                for (i, j, z) in [
                    (0, 0, -1.71),
                    (0, 1, 0.43),
                    (0, 2, 8.32),
                    (1, 0, 1.71),
                    (1, 1, -8.75),
                    (2, 2, -10.03),
                    (2, 3, 0.43),
                    (2, 4, 0.035),
                    (3, 1, 8.32),
                    (3, 2, 1.71),
                    (3, 3, -1.12),
                    (4, 4, -1.745),
                    (4, 5, 0.43),
                    (4, 6, 0.43),
                    (5, 3, 0.69),
                    (5, 4, 1.71),
                    (5, 6, 0.69),
                    (6, 6, -1.81),
                    (7, 6, 1.81),
                ] {
                    emit(i, j, z);
                }
                emit(5, 5, -280.0 * y[7] - 0.43);
                emit(5, 7, -280.0 * y[5]);
                emit(6, 5, 280.0 * y[7]);
                emit(6, 7, 280.0 * y[5]);
                emit(7, 5, -280.0 * y[7]);
                emit(7, 7, -280.0 * y[5]);
            }
            Kind::VanDerPolMu1000 => {
                emit(0, 1, 1.0);
                emit(1, 0, -2000.0 * y[0] * y[1] - 1.0);
                emit(1, 1, 1000.0 * (1.0 - y[0] * y[0]));
            }
            Kind::Brusselator1d => {
                let m = self.n / 2;
                let d = ((m + 1) as f64).powi(2) / 50.0;
                for k in 0..m {
                    let (a, b) = (2 * k, 2 * k + 1);
                    let (u, v) = (y[a], y[b]);
                    emit(a, a, 2.0 * u * v - 4.0 - 2.0 * d);
                    emit(a, b, u * u);
                    emit(b, a, 3.0 - 2.0 * u * v);
                    emit(b, b, -u * u - 2.0 * d);
                    if k > 0 {
                        emit(a, a - 2, d);
                        emit(b, b - 2, d);
                    }
                    if k + 1 < m {
                        emit(a, a + 2, d);
                        emit(b, b + 2, d);
                    }
                }
            }
            Kind::LinearExact => {
                emit(0, 0, -1.0);
                emit(1, 0, 1000.0);
                emit(1, 1, -1000.0);
            }
        }
    }

    /// Dense row-major Jacobian (`out[i * n + j]`), zero-filled first.
    pub fn jac_dense_row_major(&self, _t: f64, y: &[f64], out: &mut [f64]) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let n = self.n;
        out[..n * n].fill(0.0);
        self.entries(y, |i, j, v| out[i * n + j] = v);
    }

    /// Dense column-major Jacobian with leading dimension `ld`
    /// (`out[i + ld * j]`), zero-filled first.
    pub fn jac_dense_col_major(&self, _t: f64, y: &[f64], ld: usize, out: &mut [f64]) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let n = self.n;
        for j in 0..n {
            out[j * ld..j * ld + n].fill(0.0);
        }
        self.entries(y, |i, j, v| out[i + ld * j] = v);
    }

    /// LINPACK/Hairer band storage: `out[(mu + i - j) + ld * j]` for the
    /// entries inside the band, rows `0..=ml+mu` of each column zero-filled.
    /// `out` must start at the row of the first super-diagonal.
    pub fn jac_band_col_major(
        &self,
        _t: f64,
        y: &[f64],
        ml: usize,
        mu: usize,
        ld: usize,
        out: &mut [f64],
    ) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let n = self.n;
        let rows = ml + mu + 1;
        for j in 0..n {
            out[j * ld..j * ld + rows].fill(0.0);
        }
        self.entries(y, |i, j, v| {
            let k = mu as isize + i as isize - j as isize;
            if (0..rows as isize).contains(&k) {
                out[k as usize + ld * j] = v;
            }
        });
    }

    /// Row-major band storage of `rodas5p-integrators` (`out[i * w + (j + ml - i)]`
    /// with `w = ml + mu + 1`), zero-filled first.
    pub fn jac_band_row_major(&self, _t: f64, y: &[f64], ml: usize, mu: usize, out: &mut [f64]) {
        JAC_CALLS.fetch_add(1, Ordering::Relaxed);
        let w = ml + mu + 1;
        out[..self.n * w].fill(0.0);
        self.entries(y, |i, j, v| {
            let k = j as isize + ml as isize - i as isize;
            if (0..w as isize).contains(&k) {
                out[i * w + k as usize] = v;
            }
        });
    }

    /// Jacobian-vector product J(y) v from the same entries.
    pub fn jvp(&self, _t: f64, y: &[f64], v: &[f64], out: &mut [f64]) {
        JVP_CALLS.fetch_add(1, Ordering::Relaxed);
        out[..self.n].fill(0.0);
        self.entries(y, |i, j, a| out[i] += a * v[j]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fd_check(m: Model, y: &[f64]) {
        let n = m.n;
        let mut j = vec![0.0; n * n];
        m.jac_dense_row_major(0.0, y, &mut j);
        let mut f0 = vec![0.0; n];
        let mut f1 = vec![0.0; n];
        m.rhs(0.0, y, &mut f0);
        for c in 0..n {
            let h = 1e-7 * y[c].abs().max(1.0);
            let mut yp = y.to_vec();
            yp[c] += h;
            m.rhs(0.0, &yp, &mut f1);
            for r in 0..n {
                let fd = (f1[r] - f0[r]) / h;
                let scale = j[r * n + c].abs().max(1.0);
                assert!(
                    (fd - j[r * n + c]).abs() <= 1e-4 * scale,
                    "{:?} J[{r},{c}] = {} vs fd {fd}",
                    m.kind,
                    j[r * n + c]
                );
            }
        }
    }

    #[test]
    fn jacobians_match_finite_differences() {
        for (k, n) in [
            (Kind::Robertson, 3),
            (Kind::Hires, 8),
            (Kind::VanDerPolMu1000, 2),
            (Kind::Brusselator1d, 20),
            (Kind::LinearExact, 2),
        ] {
            let m = Model::new(k, n).unwrap();
            let y: Vec<f64> = m
                .y0()
                .iter()
                .enumerate()
                .map(|(i, v)| v + 0.01 * (i as f64 + 1.0))
                .collect();
            fd_check(m, &y);
        }
    }

    #[test]
    fn layouts_agree() {
        let m = Model::new(Kind::Brusselator1d, 12).unwrap();
        let n = m.n;
        let y: Vec<f64> = (0..n).map(|i| 0.3 + 0.1 * i as f64).collect();
        let (ml, mu) = m.band();
        let mut d = vec![0.0; n * n];
        m.jac_dense_row_major(0.0, &y, &mut d);
        let mut c = vec![0.0; n * n];
        m.jac_dense_col_major(0.0, &y, n, &mut c);
        let ld = ml + mu + 1;
        let mut b = vec![0.0; ld * n];
        m.jac_band_col_major(0.0, &y, ml, mu, ld, &mut b);
        let mut r = vec![0.0; ld * n];
        m.jac_band_row_major(0.0, &y, ml, mu, &mut r);
        let v: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
        let mut jv = vec![0.0; n];
        m.jvp(0.0, &y, &v, &mut jv);
        for i in 0..n {
            let mut s = 0.0;
            for j in 0..n {
                assert_eq!(d[i * n + j], c[i + n * j]);
                if i.abs_diff(j) <= 2 {
                    assert_eq!(d[i * n + j], b[(mu + i - j) + ld * j]);
                    assert_eq!(d[i * n + j], r[i * ld + (j + ml - i)]);
                } else {
                    assert_eq!(d[i * n + j], 0.0);
                }
                s += d[i * n + j] * v[j];
            }
            assert!((s - jv[i]).abs() <= 1e-12 * s.abs().max(1.0));
        }
    }
}
