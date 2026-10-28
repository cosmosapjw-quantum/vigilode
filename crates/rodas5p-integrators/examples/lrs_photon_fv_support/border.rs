//! Bordered LRS photon model and its Schur-complement stage solver (BASS/rei
//! DAG node P03). The P02 photon operator in the physical coordinate, with
//! three border unknowns appended after `L_hi`:
//!
//! - the shear `s` as a state: `ds/dt = -3 H s + k (D_gamma + D_m)`, with
//!   the photon anisotropic stress `D_gamma = sum_i w_i sum_j p_j N_ij`
//!   (`w_i` the cell average of `exp(q)`); the transport drifts with this
//!   `s`;
//! - the free-electron fraction `x_e` of a Riccati law `dx_e/dt = beta (1 -
//!   x_e) - alpha x_e^2` (closed-form solution), setting the Thomson rate
//!   `Gamma = gamma0 x_e`;
//! - a matter stress relaxing towards `-kappa s`: `dD_m/dt = -(D_m + kappa
//!   s) / tau`.
//!
//! `H` is the prescribed P02 geometry's (its shear is unused). The
//! dimensionless constants are scenario parameters, not physical ones.
//!
//! [`BorderedStageSolver`] solves `W = inv I - J` through the structured
//! photon solver and a three-by-three Schur complement: with `A` the photon
//! block, `B` its two nonzero border columns (`-df/ds`, `-df/dx_e`) and `c`
//! the one nonzero border row (`-k w_i p_j`), `x_ph = y - Z x_b` with `y =
//! A^-1 b_ph`, `Z = A^-1 B` and `(D - c Z) x_b = b_b - c y`.

#![allow(clippy::needless_range_loop)] // the kernels index several arrays by cell

use std::sync::{Arc, Mutex};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::{OdeProblem, StageLinearSystem};

use super::block_solver::{BlockStageSolver, Capacitance, Mode};
use super::{Coordinate, Model};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderParams {
    /// Stress coupling of the shear equation.
    pub k: f64,
    /// Riccati recombination and ionization coefficients.
    pub alpha: f64,
    pub beta: f64,
    /// Matter stress: equilibrium slope and relaxation time.
    pub kappa: f64,
    pub tau: f64,
    /// Thomson rate per unit `x_e`.
    pub gamma0: f64,
}

pub struct Bordered {
    pub photon: Arc<Model>,
    pub params: BorderParams,
    /// Cell average of `exp(q)` per energy group.
    pub w: Vec<f64>,
    /// Scratch of the photon size for the `s` column of the Jacobian.
    scratch: Mutex<Vec<f64>>,
}

impl Bordered {
    pub fn new(photon: Arc<Model>, params: BorderParams) -> CoreResult<Self> {
        let p = params;
        let finite = [p.k, p.alpha, p.beta, p.kappa, p.tau, p.gamma0]
            .iter()
            .all(|v| v.is_finite());
        if !finite || p.alpha < 0.0 || p.beta < 0.0 || p.tau <= 0.0 || p.gamma0 < 0.0 {
            return Err(CoreError::InvalidInput(
                "bordered model: finite parameters, alpha, beta, gamma0 >= 0, tau > 0".into(),
            ));
        }
        if photon.layout.coordinate != Coordinate::Physical {
            return Err(CoreError::InvalidInput(
                "bordered model: physical energy coordinate only".into(),
            ));
        }
        if photon.gamma != 0.0 || photon.drop_partial_t {
            return Err(CoreError::InvalidInput(
                "bordered model: the photon model's own Thomson rate must be 0".into(),
            ));
        }
        let l = &photon.layout;
        let w = (0..l.nq)
            .map(|i| l.q_edges[i].exp() * l.dq[i].exp_m1() / l.dq[i])
            .collect();
        let n_ph = l.n();
        Ok(Self {
            params,
            w,
            scratch: Mutex::new(vec![0.0; n_ph]),
            photon,
        })
    }

    pub fn n_photon(&self) -> usize {
        self.photon.layout.n()
    }

    pub fn n(&self) -> usize {
        self.n_photon() + 3
    }

    /// Indices of `s`, `x_e` and `D_m`.
    pub fn border(&self) -> (usize, usize, usize) {
        let n = self.n_photon();
        (n, n + 1, n + 2)
    }

    /// `D_gamma = sum_i w_i sum_j p_j N_ij`.
    pub fn d_gamma(&self, y: &[f64]) -> f64 {
        let l = &self.photon.layout;
        let mut total = 0.0;
        for i in 0..l.nq {
            let mut m2 = 0.0;
            for j in 0..l.nm {
                m2 += l.p2[j] * y[l.cell(i, j)];
            }
            total += self.w[i] * m2;
        }
        total
    }

    fn check(&self, y: &[f64], out: &[f64]) -> CoreResult<()> {
        if y.len() != self.n() || out.len() != self.n() {
            return Err(CoreError::Dimension(
                "bordered photon state shape mismatch".into(),
            ));
        }
        Ok(())
    }

    /// Accumulates `rate` times the Thomson operator `d_j/2 (M0 + p_j M2 /
    /// 2) - N_ij` of `y` into `out` (the P02 formula).
    #[inline(always)]
    fn thomson(&self, rate: f64, y: &[f64], out: &mut [f64]) {
        let l = &self.photon.layout;
        for i in 0..l.nq {
            let (mut m0, mut m2) = (0.0, 0.0);
            for j in 0..l.nm {
                let n = y[l.cell(i, j)];
                m0 += n;
                m2 += l.p2[j] * n;
            }
            for j in 0..l.nm {
                let n = y[l.cell(i, j)];
                out[l.cell(i, j)] += rate * (l.dm[j] / 2.0 * (m0 + 0.5 * l.p2[j] * m2) - n);
            }
        }
    }

    /// `f(t, y)`.
    pub fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.check(y, out)?;
        out.fill(0.0);
        let p = &self.params;
        let (is, ix, idm) = self.border();
        let (h, _, _, _) = self.photon.geometry.at(t);
        let (s, xe, dm) = (y[is], y[ix], y[idm]);
        self.photon.transport(h, s, h, s, y, out);
        let gamma = p.gamma0 * xe;
        if gamma != 0.0 {
            self.thomson(gamma, y, out);
        }
        let l = &self.photon.layout;
        if self.photon.absorber.is_some() {
            for i in 0..l.nq {
                let (k, _) = self.photon.kbar(i, t);
                let mut absorbed = 0.0;
                for j in 0..l.nm {
                    let rate = k * y[l.cell(i, j)];
                    out[l.cell(i, j)] -= rate;
                    absorbed += rate;
                }
                out[l.absorbed(i)] += absorbed;
            }
        }
        out[is] = -3.0 * h * s + p.k * (self.d_gamma(y) + dm);
        out[ix] = p.beta * (1.0 - xe) - p.alpha * xe * xe;
        out[idm] = -(dm + p.kappa * s) / p.tau;
        Ok(())
    }

    /// `df/dt` at fixed state: only `H(t)` is explicit.
    pub fn partial_t(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.check(y, out)?;
        out.fill(0.0);
        let (is, _, _) = self.border();
        let (h, _, dh, _) = self.photon.geometry.at(t);
        let s = y[is];
        self.photon.transport(h, s, dh, 0.0, y, out);
        out[is] = -3.0 * dh * s;
        Ok(())
    }

    /// Dense Jacobian in place: zeroes the photon band and the border rows
    /// and columns, then accumulates.
    pub fn dense_fill(&self, t: f64, y: &[f64], m: &mut DenseMatrix) -> CoreResult<()> {
        let n = self.n();
        let n_ph = self.n_photon();
        if m.nrows() != n || m.ncols() != n || y.len() != n {
            return Err(CoreError::Dimension(
                "bordered Jacobian shape mismatch".into(),
            ));
        }
        let p = &self.params;
        let l = &self.photon.layout;
        let bw = l.bandwidth();
        let (is, ix, idm) = self.border();
        for r in 0..n_ph {
            for c in r.saturating_sub(bw)..(r + bw + 1).min(n_ph) {
                m[(r, c)] = 0.0;
            }
            for c in n_ph..n {
                m[(r, c)] = 0.0;
            }
        }
        for r in n_ph..n {
            for c in 0..n {
                m[(r, c)] = 0.0;
            }
        }
        let (h, _, _, _) = self.photon.geometry.at(t);
        let (s, xe) = (y[is], y[ix]);
        self.photon
            .jacobian_entries_with(t, h, s, p.gamma0 * xe, |r, c, v| m[(r, c)] += v);
        // the s column: the transport's derivative with its upwind direction frozen
        {
            let mut col = self.scratch.lock().unwrap_or_else(|e| e.into_inner());
            col.fill(0.0);
            self.photon.transport(h, s, 0.0, 1.0, &y[..n_ph], &mut col);
            for (r, v) in col.iter().enumerate() {
                m[(r, is)] += *v;
            }
        }
        // the x_e column: gamma0 times the Thomson operator applied to N
        for i in 0..l.nq {
            let (mut m0, mut m2) = (0.0, 0.0);
            for j in 0..l.nm {
                let v = y[l.cell(i, j)];
                m0 += v;
                m2 += l.p2[j] * v;
            }
            for j in 0..l.nm {
                let v = y[l.cell(i, j)];
                m[(l.cell(i, j), ix)] += p.gamma0 * (l.dm[j] / 2.0 * (m0 + 0.5 * l.p2[j] * m2) - v);
            }
        }
        for i in 0..l.nq {
            for j in 0..l.nm {
                m[(is, l.cell(i, j))] += p.k * (self.w[i] * l.p2[j]);
            }
        }
        m[(is, is)] += -3.0 * h;
        m[(is, idm)] += p.k;
        m[(ix, ix)] += -p.beta - 2.0 * p.alpha * xe;
        m[(idm, is)] += -p.kappa / p.tau;
        m[(idm, idm)] += -1.0 / p.tau;
        Ok(())
    }

    /// The closed-form Riccati solution `x_e(t)` from `x_e(0) = x0`, with
    /// the cancellation-free positive root.
    pub fn riccati(&self, x0: f64, t: f64) -> f64 {
        let (a, b) = (self.params.alpha, self.params.beta);
        if a == 0.0 {
            return 1.0 + (x0 - 1.0) * (-b * t).exp();
        }
        let root = (b * b + 4.0 * a * b).sqrt();
        let r1 = 2.0 * b / (b + root);
        let r2 = -(b + root) / (2.0 * a);
        let c = (x0 - r1) / (x0 - r2);
        let e = c * (-a * (r1 - r2) * t).exp();
        r1 + (r1 - r2) * e / (1.0 - e)
    }
}

type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// The RODAS5P problem of the bordered model (dense Jacobian in place).
pub fn problem(model: Arc<Bordered>) -> CoreResult<OdeProblem> {
    let n = model.n();
    let autonomous = model.photon.geometry.is_constant();
    let (m_rhs, m_jac, m_ft, m_into) = (model.clone(), model.clone(), model.clone(), model);
    let partial_t: Option<PartialT> = if autonomous {
        None
    } else {
        Some(Arc::new(move |t, y, out| m_ft.partial_t(t, y, out)))
    };
    Ok(OdeProblem::new(
        "lrs-photon-fv-bordered",
        n,
        Arc::new(move |t, y, out| m_rhs.rhs(t, y, out)),
        None,
        Some(Arc::new(move |t, y| {
            let mut m = DenseMatrix::zeros(n, n);
            m_jac.dense_fill(t, y, &mut m)?;
            Ok(m)
        })),
        None,
        partial_t,
        autonomous,
        None,
        None,
    )?
    .with_jacobian_into(Arc::new(move |t, y, m| m_into.dense_fill(t, y, m))))
}

/// The bordered client stage solver.
pub struct BorderedStageSolver {
    model: Arc<Bordered>,
    pub inner: BlockStageSolver,
    /// `B` columns (`-df_ph/ds`, `-df_ph/dx_e`) and `Z = A^-1 B`.
    bs: Vec<f64>,
    bx: Vec<f64>,
    zs: Vec<f64>,
    zx: Vec<f64>,
    /// Border block of `-J`, row-major.
    jd: [[f64; 3]; 3],
    /// LU of the Schur complement and its row interchanges.
    lu: [[f64; 3]; 3],
    perm: [usize; 3],
    linearized: bool,
    factored: bool,
}

impl BorderedStageSolver {
    pub fn new(model: Arc<Bordered>, epoch: u64) -> CoreResult<Self> {
        let inner = BlockStageSolver::new(
            model.photon.clone(),
            epoch,
            Mode::Exact,
            Capacitance::Conservation,
        )?;
        let n_ph = model.n_photon();
        Ok(Self {
            model,
            inner,
            bs: vec![0.0; n_ph],
            bx: vec![0.0; n_ph],
            zs: vec![0.0; n_ph],
            zx: vec![0.0; n_ph],
            jd: [[0.0; 3]; 3],
            lu: [[0.0; 3]; 3],
            perm: [0, 1, 2],
            linearized: false,
            factored: false,
        })
    }

    /// `c . v` over the photon cells, `-k D_gamma(v)`.
    fn c_dot(&self, v: &[f64]) -> f64 {
        -self.model.params.k * self.model.d_gamma(v)
    }
}

impl StageLinearSystem for BorderedStageSolver {
    fn dimension(&self) -> usize {
        self.model.n()
    }

    fn linearize(
        &mut self,
        problem: &OdeProblem,
        t: f64,
        y: &[f64],
        _counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.linearized = false;
        self.factored = false;
        let m = &*self.model;
        if problem.dimension != m.n() || y.len() != m.n() {
            return Err(CoreError::Dimension(
                "bordered solver: problem dimension mismatch".into(),
            ));
        }
        let p = m.params;
        let l = &m.photon.layout;
        let (is, ix, _) = m.border();
        let n_ph = m.n_photon();
        let (h, _, _, _) = m.photon.geometry.at(t);
        let (s, xe) = (y[is], y[ix]);
        self.inner.linearize_with(t, h, s, p.gamma0 * xe)?;
        self.bs.fill(0.0);
        m.photon.transport(h, s, 0.0, 1.0, &y[..n_ph], &mut self.bs);
        for v in self.bs.iter_mut() {
            *v = -*v;
        }
        self.bx.fill(0.0);
        for i in 0..l.nq {
            let (mut m0, mut m2) = (0.0, 0.0);
            for j in 0..l.nm {
                let v = y[l.cell(i, j)];
                m0 += v;
                m2 += l.p2[j] * v;
            }
            for j in 0..l.nm {
                let v = y[l.cell(i, j)];
                self.bx[l.cell(i, j)] =
                    -(p.gamma0 * (l.dm[j] / 2.0 * (m0 + 0.5 * l.p2[j] * m2) - v));
            }
        }
        self.jd = [
            [3.0 * h, 0.0, -p.k],
            [0.0, p.beta + 2.0 * p.alpha * xe, 0.0],
            [p.kappa / p.tau, 0.0, 1.0 / p.tau],
        ];
        self.linearized = true;
        Ok(())
    }

    fn factor(&mut self, inv_h_gamma: f64, counters: &mut WorkCounters) -> CoreResult<()> {
        if !self.linearized {
            return Err(CoreError::InvalidInput(
                "BORDERED_NOT_LINEARIZED: factor without a current linearization".into(),
            ));
        }
        self.factored = false;
        counters.direct_factorizations += 1;
        self.inner.factor_with(inv_h_gamma)?;
        self.zs.copy_from_slice(&self.bs);
        self.inner.solve_with(&mut self.zs, counters)?;
        self.zx.copy_from_slice(&self.bx);
        self.inner.solve_with(&mut self.zx, counters)?;
        // S = inv I + jd, its first row less [c Z_s, c Z_x, 0]
        let mut s = self.jd;
        for (k, row) in s.iter_mut().enumerate() {
            row[k] += inv_h_gamma;
        }
        s[0][0] -= self.c_dot(&self.zs);
        s[0][1] -= self.c_dot(&self.zx);
        let mut perm = [0usize, 1, 2];
        for k in 0..3 {
            let mut p = k;
            for i in k + 1..3 {
                if s[i][k].abs() > s[p][k].abs() {
                    p = i;
                }
            }
            if !(s[p][k].is_finite() && s[p][k] != 0.0) {
                return Err(CoreError::LinearSolve(
                    "BORDERED_SINGULAR: singular Schur complement".into(),
                ));
            }
            s.swap(k, p);
            perm.swap(k, p);
            for i in k + 1..3 {
                let f = s[i][k] / s[k][k];
                s[i][k] = f;
                for j in k + 1..3 {
                    s[i][j] -= f * s[k][j];
                }
            }
        }
        self.lu = s;
        self.perm = perm;
        self.factored = true;
        Ok(())
    }

    fn solve_in_place(&mut self, b: &mut [f64], counters: &mut WorkCounters) -> CoreResult<()> {
        if !self.factored {
            return Err(CoreError::InvalidInput(
                "BORDERED_STALE: solve without a factorization of the current linearization".into(),
            ));
        }
        let n_ph = self.zs.len();
        if b.len() != n_ph + 3 {
            return Err(CoreError::Dimension(
                "bordered solver: right-hand side length mismatch".into(),
            ));
        }
        let (ph, bd) = b.split_at_mut(n_ph);
        self.inner.solve_with(ph, counters)?;
        let r = [bd[0] - self.c_dot(ph), bd[1], bd[2]];
        let mut z = [r[self.perm[0]], r[self.perm[1]], r[self.perm[2]]];
        for i in 1..3 {
            for j in 0..i {
                z[i] -= self.lu[i][j] * z[j];
            }
        }
        for i in (0..3).rev() {
            for j in i + 1..3 {
                z[i] -= self.lu[i][j] * z[j];
            }
            z[i] /= self.lu[i][i];
        }
        for k in 0..n_ph {
            ph[k] -= self.zs[k] * z[0] + self.zx[k] * z[1];
        }
        bd.copy_from_slice(&z);
        Ok(())
    }
}
