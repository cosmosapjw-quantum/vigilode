//! Client-owned fixed-grid LRS photon transport adapter (BASS/rei DAG node
//! P02). It ports the conservative first-order finite-volume operator of
//! `research/ncp_followup_20261021/BASS_REI_FVM_REFERENCE.py` to allocation
//! free callbacks of the RODAS5P fast drivers, adding prescribed
//! time-dependent geometry with its exact `f_t`, a comoving energy
//! coordinate, a moving absorption threshold, per-group rank-two Thomson
//! scattering and number ledgers.
//!
//! State layout, `nm` angular cells per energy group `i`:
//! `[L_lo, (N_i0 .. N_i(nm-1), A_i) for i in 0..nq, L_hi]`, where `L_lo` and
//! `L_hi` collect the number leaving through the low and high energy
//! boundaries and `A_i` the number absorbed in group `i`. Every component
//! enters a linear invariant: the sum of all components is conserved by the
//! semi-discrete operator. The operator is linear in the state, so the JVP
//! is the RHS applied to the direction at the same time. The Jacobian lies
//! in the band `lower = upper = nm + 1`.
//!
//! Transport follows the oracle's operations and accumulation order bit for
//! bit (`powf(3.0)` for the oracle's `**3`, the same association), so the
//! transport RHS equals the oracle's. Thomson sums its moments in order
//! (the oracle uses an exactly rounded `fsum`).

#![allow(dead_code)] // shared by several example and test binaries

pub mod block_solver;
pub mod border;
pub mod chem;
pub mod remesh;

use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix};
use rodas5p_integrators::{BandedJacobian, OdeProblem};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coordinate {
    /// `q = ln(E / E_ref)`: `dot q = -H - S`.
    Physical,
    /// `x = ln(a E / E_ref)`: `dot x = -S`; a threshold fixed in physical
    /// energy moves as `x_chi(t) = q_chi + ln a(t)`.
    Comoving,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// Zero inflow; outflow enters the ledgers.
    Outflow,
    /// No flux through the energy boundaries.
    Closed,
}

/// Prescribed LRS Bianchi-I geometry `H(t)`, shear `s(t)` (`sigma =
/// diag(-s, -s, 2s)`) with closed-form `ln a` and `int s`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geometry {
    Constant {
        h: f64,
        s: f64,
    },
    /// `H = hh / (1 + t)`, `s = s0 (1 + t)^(-p)`, `p != 1`.
    PowerLaw {
        hh: f64,
        s0: f64,
        p: f64,
    },
}

impl Geometry {
    fn validate(&self) -> CoreResult<()> {
        let ok = match *self {
            Geometry::Constant { h, s } => h.is_finite() && s.is_finite(),
            Geometry::PowerLaw { hh, s0, p } => {
                hh.is_finite() && s0.is_finite() && p.is_finite() && p != 1.0
            }
        };
        if ok {
            Ok(())
        } else {
            Err(CoreError::InvalidInput(
                "geometry parameters must be finite (p != 1)".into(),
            ))
        }
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Geometry::Constant { .. })
    }

    /// `(H, s, dH/dt, ds/dt)` at `t`.
    #[inline(always)]
    pub fn at(&self, t: f64) -> (f64, f64, f64, f64) {
        match *self {
            Geometry::Constant { h, s } => (h, s, 0.0, 0.0),
            Geometry::PowerLaw { hh, s0, p } => {
                let x = 1.0 + t;
                let s = s0 * x.powf(-p);
                (hh / x, s, -hh / (x * x), -p * s / x)
            }
        }
    }

    /// `ln a(t)` with `a(0) = 1`.
    pub fn ln_a(&self, t: f64) -> f64 {
        match *self {
            Geometry::Constant { h, .. } => h * t,
            Geometry::PowerLaw { hh, .. } => hh * t.ln_1p(),
        }
    }

    /// `int_0^t s`.
    pub fn int_s(&self, t: f64) -> f64 {
        match *self {
            Geometry::Constant { s, .. } => s * t,
            Geometry::PowerLaw { s0, p, .. } => s0 * ((1.0 - p) * t.ln_1p()).exp_m1() / (1.0 - p),
        }
    }
}

/// Absorber `k(E) = k0 (E / chi)^(-3)` above the threshold `chi =
/// E_ref exp(q_chi)`, zero below; cell averages assume a density uniform in
/// the cell's energy coordinate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Absorber {
    pub k0: f64,
    pub q_chi: f64,
}

pub struct Layout {
    pub q_edges: Vec<f64>,
    pub mu_edges: Vec<f64>,
    pub dq: Vec<f64>,
    pub dm: Vec<f64>,
    pub nq: usize,
    pub nm: usize,
    /// `3 <mu^2>_j - 1` with the oracle's exact cell average.
    pub c: Vec<f64>,
    /// Exact cell average of `P2(mu)`, as the oracle's Thomson kernel.
    pub p2: Vec<f64>,
    pub coordinate: Coordinate,
    pub boundary: Boundary,
}

fn widths(edges: &[f64], what: &str) -> CoreResult<Vec<f64>> {
    if edges.len() < 2 || edges.iter().any(|x| !x.is_finite()) {
        return Err(CoreError::InvalidInput(format!(
            "{what}: finite nonempty grid required"
        )));
    }
    let w: Vec<f64> = edges.windows(2).map(|p| p[1] - p[0]).collect();
    if w.iter().any(|d| !d.is_finite() || *d <= 0.0) {
        return Err(CoreError::InvalidInput(format!(
            "{what}: strict finite positive widths required"
        )));
    }
    Ok(w)
}

impl Layout {
    pub fn new(
        q_edges: Vec<f64>,
        mu_edges: Vec<f64>,
        coordinate: Coordinate,
        boundary: Boundary,
    ) -> CoreResult<Self> {
        let dq = widths(&q_edges, "energy edges")?;
        let dm = widths(&mu_edges, "angle edges")?;
        if mu_edges[0] != -1.0 || mu_edges[mu_edges.len() - 1] != 1.0 {
            return Err(CoreError::InvalidInput(
                "full axial sphere -1..1 required".into(),
            ));
        }
        let c = (0..dm.len())
            .map(|j| {
                let mu2 = (mu_edges[j + 1].powf(3.0) - mu_edges[j].powf(3.0)) / (3.0 * dm[j]);
                3.0 * mu2 - 1.0
            })
            .collect();
        let p2 = (0..dm.len())
            .map(|j| {
                let (a, b) = (mu_edges[j], mu_edges[j + 1]);
                ((b.powf(3.0) - b) - (a.powf(3.0) - a)) / (2.0 * dm[j])
            })
            .collect();
        Ok(Self {
            nq: dq.len(),
            nm: dm.len(),
            q_edges,
            mu_edges,
            dq,
            dm,
            c,
            p2,
            coordinate,
            boundary,
        })
    }

    /// State dimension.
    pub fn n(&self) -> usize {
        2 + self.nq * (self.nm + 1)
    }

    pub const LO: usize = 0;

    pub fn hi(&self) -> usize {
        self.n() - 1
    }

    #[inline(always)]
    pub fn cell(&self, i: usize, j: usize) -> usize {
        1 + i * (self.nm + 1) + j
    }

    #[inline(always)]
    pub fn absorbed(&self, i: usize) -> usize {
        1 + i * (self.nm + 1) + self.nm
    }

    /// Half bandwidth of the Jacobian.
    pub fn bandwidth(&self) -> usize {
        self.nm + 1
    }
}

pub struct Model {
    pub layout: Layout,
    pub geometry: Geometry,
    /// Thomson rate `c sigma_T n_e` (constant).
    pub gamma: f64,
    pub absorber: Option<Absorber>,
    /// Negative control: report a zero `f_t` for a non-autonomous problem.
    pub drop_partial_t: bool,
}

impl Model {
    pub fn new(
        layout: Layout,
        geometry: Geometry,
        gamma: f64,
        absorber: Option<Absorber>,
        drop_partial_t: bool,
    ) -> CoreResult<Self> {
        geometry.validate()?;
        if !gamma.is_finite() || gamma < 0.0 {
            return Err(CoreError::InvalidInput(
                "Thomson rate must be finite and >= 0".into(),
            ));
        }
        if let Some(a) = absorber
            && (!a.k0.is_finite() || a.k0 < 0.0 || !a.q_chi.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "absorber must be finite with k0 >= 0".into(),
            ));
        }
        Ok(Self {
            layout,
            geometry,
            gamma,
            absorber,
            drop_partial_t,
        })
    }

    /// Whether the semi-discrete operator is independent of time.
    pub fn autonomous(&self) -> bool {
        self.geometry.is_constant()
            && (self.absorber.is_none() || self.layout.coordinate == Coordinate::Physical)
    }

    /// `(H_eff, s, dH_eff, ds)`: the energy coordinate drift drops `H` in
    /// comoving coordinates.
    #[inline(always)]
    fn drift(&self, t: f64) -> (f64, f64, f64, f64) {
        let (h, s, dh, ds) = self.geometry.at(t);
        match self.layout.coordinate {
            Coordinate::Physical => (h, s, dh, ds),
            Coordinate::Comoving => (0.0, s, 0.0, ds),
        }
    }

    /// Threshold position in the grid's energy coordinate and its rate.
    #[inline(always)]
    fn threshold(&self, a: &Absorber, t: f64) -> (f64, f64) {
        match self.layout.coordinate {
            Coordinate::Physical => (a.q_chi, 0.0),
            Coordinate::Comoving => (a.q_chi + self.geometry.ln_a(t), self.geometry.at(t).0),
        }
    }

    /// Cell average of the absorber over energy cell `i` and its time
    /// derivative (closed forms, `exp_m1` against cancellation).
    ///
    /// The value is continuous in `t`; the derivative has a kink whenever
    /// the threshold crosses an edge. At an exact tie the branch follows the
    /// threshold's motion, so the derivative is the right-sided one (the
    /// branch the cell is in for `t' > t`): with `rate > 0` a cell with
    /// `lo == x_chi` straddles; with `rate < 0` a cell with `hi == x_chi`
    /// straddles (value 0, derivative `-3 scale rate`); with `rate == 0` the
    /// time derivative vanishes and ties count as below/above.
    #[inline(always)]
    pub fn kbar(&self, i: usize, t: f64) -> (f64, f64) {
        let Some(a) = self.absorber else {
            return (0.0, 0.0);
        };
        let (x_chi, rate) = self.threshold(&a, t);
        let (lo, hi) = (self.layout.q_edges[i], self.layout.q_edges[i + 1]);
        let scale = a.k0 / (3.0 * self.layout.dq[i]);
        let below = if rate < 0.0 { hi < x_chi } else { hi <= x_chi };
        let above = if rate > 0.0 { lo > x_chi } else { lo >= x_chi };
        if below {
            (0.0, 0.0)
        } else if above {
            // exp(-3(lo - x)) - exp(-3(hi - x)) = -exp(-3(lo - x)) expm1(-3 dq)
            let k = -scale * (-3.0 * (lo - x_chi)).exp() * (-3.0 * self.layout.dq[i]).exp_m1();
            (k, 3.0 * rate * k)
        } else {
            let tail = -3.0 * (hi - x_chi);
            (-scale * tail.exp_m1(), -3.0 * scale * rate * tail.exp())
        }
    }

    fn check(&self, y: &[f64], out: &[f64]) -> CoreResult<()> {
        let n = self.layout.n();
        if y.len() != n || out.len() != n {
            return Err(CoreError::Dimension(
                "photon transport state shape mismatch".into(),
            ));
        }
        Ok(())
    }

    /// The oracle's transport loops: upwind direction from `(h, s)`, flux
    /// velocity from `(vh, vs)` (equal for the RHS; the time derivatives for
    /// `f_t`). Accumulates into `out` (zeroed by the caller).
    #[inline(always)]
    fn transport(&self, h: f64, s: f64, vh: f64, vs: f64, y: &[f64], out: &mut [f64]) {
        let l = &self.layout;
        let (nq, nm) = (l.nq, l.nm);
        let hi = l.hi();
        let closed = l.boundary == Boundary::Closed;
        for j in 0..nm {
            let select = -h - s * l.c[j];
            let velocity = -vh - vs * l.c[j];
            for k in 0..=nq {
                if closed && (k == 0 || k == nq) {
                    continue;
                }
                let upstream = if select >= 0.0 {
                    k as isize - 1
                } else {
                    k as isize
                };
                let val = if upstream >= 0 && (upstream as usize) < nq {
                    let u = upstream as usize;
                    y[l.cell(u, j)] / (l.dq[u] * l.dm[j])
                } else {
                    0.0
                };
                let flux = velocity * val * l.dm[j];
                if k > 0 {
                    out[l.cell(k - 1, j)] -= flux;
                } else {
                    out[Layout::LO] -= flux;
                }
                if k < nq {
                    out[l.cell(k, j)] += flux;
                } else {
                    out[hi] += flux;
                }
            }
        }
        for i in 0..nq {
            for k in 1..nm {
                let mu = l.mu_edges[k];
                let select = -3.0 * s * mu * (1.0 - mu * mu);
                let velocity = -3.0 * vs * mu * (1.0 - mu * mu);
                let upstream = if select >= 0.0 { k - 1 } else { k };
                let flux =
                    velocity * (y[l.cell(i, upstream)] / (l.dq[i] * l.dm[upstream])) * l.dq[i];
                out[l.cell(i, k - 1)] -= flux;
                out[l.cell(i, k)] += flux;
            }
        }
    }

    /// `f(t, y)`.
    pub fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.check(y, out)?;
        out.fill(0.0);
        let (h, s, _, _) = self.drift(t);
        self.transport(h, s, h, s, y, out);
        let l = &self.layout;
        if self.gamma != 0.0 {
            for i in 0..l.nq {
                let (mut m0, mut m2) = (0.0, 0.0);
                for j in 0..l.nm {
                    let n = y[l.cell(i, j)];
                    m0 += n;
                    m2 += l.p2[j] * n;
                }
                for j in 0..l.nm {
                    let n = y[l.cell(i, j)];
                    out[l.cell(i, j)] +=
                        self.gamma * (l.dm[j] / 2.0 * (m0 + 0.5 * l.p2[j] * m2) - n);
                }
            }
        }
        if self.absorber.is_some() {
            for i in 0..l.nq {
                let (k, _) = self.kbar(i, t);
                let mut absorbed = 0.0;
                for j in 0..l.nm {
                    let rate = k * y[l.cell(i, j)];
                    out[l.cell(i, j)] -= rate;
                    absorbed += rate;
                }
                out[l.absorbed(i)] += absorbed;
            }
        }
        Ok(())
    }

    /// `df/dt` at fixed state (upwind direction of `f(t, .)`).
    pub fn partial_t(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.check(y, out)?;
        out.fill(0.0);
        if self.drop_partial_t {
            return Ok(());
        }
        let (h, s, dh, ds) = self.drift(t);
        self.transport(h, s, dh, ds, y, out);
        let l = &self.layout;
        if self.absorber.is_some() {
            for i in 0..l.nq {
                let (_, dk) = self.kbar(i, t);
                let mut absorbed = 0.0;
                for j in 0..l.nm {
                    let rate = dk * y[l.cell(i, j)];
                    out[l.cell(i, j)] -= rate;
                    absorbed += rate;
                }
                out[l.absorbed(i)] += absorbed;
            }
        }
        Ok(())
    }

    /// Accumulates every Jacobian entry through `add(row, col, value)` in
    /// the RHS's own order, so a column equals the JVP of a unit vector.
    #[inline(always)]
    fn jacobian_entries(&self, t: f64, add: impl FnMut(usize, usize, f64)) {
        let (h, s, _, _) = self.drift(t);
        self.jacobian_entries_with(t, h, s, self.gamma, add);
    }

    /// [`Self::jacobian_entries`] with the energy drift `-h - s c_j`, the
    /// angular drift of `s` and the Thomson rate `gamma` given explicitly
    /// (a bordered model's shear and ionization are states).
    #[inline(always)]
    fn jacobian_entries_with(
        &self,
        t: f64,
        h: f64,
        s: f64,
        gamma: f64,
        mut add: impl FnMut(usize, usize, f64),
    ) {
        let l = &self.layout;
        let (nq, nm) = (l.nq, l.nm);
        let hi = l.hi();
        let closed = l.boundary == Boundary::Closed;
        for j in 0..nm {
            let velocity = -h - s * l.c[j];
            for k in 0..=nq {
                if closed && (k == 0 || k == nq) {
                    continue;
                }
                let upstream = if velocity >= 0.0 {
                    k as isize - 1
                } else {
                    k as isize
                };
                if upstream < 0 || upstream as usize >= nq {
                    continue;
                }
                let u = upstream as usize;
                let coef = velocity * (1.0 / (l.dq[u] * l.dm[j])) * l.dm[j];
                let col = l.cell(u, j);
                add(
                    if k > 0 { l.cell(k - 1, j) } else { Layout::LO },
                    col,
                    -coef,
                );
                add(if k < nq { l.cell(k, j) } else { hi }, col, coef);
            }
        }
        for i in 0..nq {
            for k in 1..nm {
                let mu = l.mu_edges[k];
                let velocity = -3.0 * s * mu * (1.0 - mu * mu);
                let upstream = if velocity >= 0.0 { k - 1 } else { k };
                let coef = velocity * (1.0 / (l.dq[i] * l.dm[upstream])) * l.dq[i];
                let col = l.cell(i, upstream);
                add(l.cell(i, k - 1), col, -coef);
                add(l.cell(i, k), col, coef);
            }
        }
        if gamma != 0.0 {
            for i in 0..nq {
                for j in 0..nm {
                    for jj in 0..nm {
                        let delta = if j == jj { 1.0 } else { 0.0 };
                        let v = gamma * (l.dm[j] / 2.0 * (1.0 + 0.5 * l.p2[j] * l.p2[jj]) - delta);
                        add(l.cell(i, j), l.cell(i, jj), v);
                    }
                }
            }
        }
        if self.absorber.is_some() {
            for i in 0..nq {
                let (k, _) = self.kbar(i, t);
                for j in 0..nm {
                    add(l.cell(i, j), l.cell(i, j), -k);
                    add(l.absorbed(i), l.cell(i, j), k);
                }
            }
        }
    }

    /// Dense Jacobian in place: zeroes the whole band (every entry that can
    /// ever be nonzero), then accumulates.
    pub fn dense_fill(&self, t: f64, m: &mut DenseMatrix) -> CoreResult<()> {
        let n = self.layout.n();
        if m.nrows() != n || m.ncols() != n {
            return Err(CoreError::Dimension(
                "photon transport Jacobian shape mismatch".into(),
            ));
        }
        let bw = self.layout.bandwidth();
        for r in 0..n {
            for c in r.saturating_sub(bw)..(r + bw + 1).min(n) {
                m[(r, c)] = 0.0;
            }
        }
        self.jacobian_entries(t, |r, c, v| m[(r, c)] += v);
        Ok(())
    }

    /// Packed band `i * (2 bw + 1) + (j + bw - i)` (zeroed by the driver).
    pub fn band_fill(&self, t: f64, band: &mut [f64]) -> CoreResult<()> {
        let bw = self.layout.bandwidth();
        let width = 2 * bw + 1;
        if band.len() != self.layout.n() * width {
            return Err(CoreError::Dimension(
                "photon transport band shape mismatch".into(),
            ));
        }
        self.jacobian_entries(t, |r, c, v| band[r * width + (c + bw - r)] += v);
        Ok(())
    }

    /// Sum of all components (the semi-discrete linear invariant).
    pub fn invariant(y: &[f64]) -> f64 {
        y.iter().sum()
    }
}

type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// The RODAS5P problem with RHS, JVP, allocation-free in-place dense
/// Jacobian and (non-autonomous) exact `f_t`.
pub fn problem(model: Arc<Model>) -> CoreResult<OdeProblem> {
    let n = model.layout.n();
    let autonomous = model.autonomous() && !model.drop_partial_t;
    let (m_rhs, m_jac, m_jvp, m_ft, m_into) = (
        model.clone(),
        model.clone(),
        model.clone(),
        model.clone(),
        model,
    );
    let partial_t: Option<PartialT> = if autonomous {
        None
    } else {
        Some(Arc::new(move |t, y, out| m_ft.partial_t(t, y, out)))
    };
    Ok(OdeProblem::new(
        "lrs-photon-fv",
        n,
        Arc::new(move |t, y, out| m_rhs.rhs(t, y, out)),
        None,
        Some(Arc::new(move |t, _| {
            let mut m = DenseMatrix::zeros(n, n);
            m_jac.dense_fill(t, &mut m)?;
            Ok(m)
        })),
        Some(Arc::new(move |t, _, v, out| m_jvp.rhs(t, v, out))),
        partial_t,
        autonomous,
        None,
        None,
    )?
    .with_jacobian_into(Arc::new(move |t, _, m| m_into.dense_fill(t, m))))
}

/// The packed band declaration for the banded fast driver.
pub fn band(model: Arc<Model>) -> BandedJacobian {
    let bw = model.layout.bandwidth();
    BandedJacobian {
        lower: bw,
        upper: bw,
        fill: Arc::new(move |t, _, b| model.band_fill(t, b)),
    }
}
