//! Client-owned fixed-grid full-sphere photon transport adapter (BASS/rei
//! DAG node P06). It ports the conservative first-order finite-volume
//! operator of the formula oracle
//! `research/bassrei_p06_full_sphere_scoped_20261022/FULL_SPHERE_FVM_REFERENCE.py`
//! (normative: `FULL_SPHERE_THEORY.md` of the same node) on the
//! `(q, mu, phi)` grid of a non-tilted triaxial Bianchi-I geometry to
//! allocation-free callbacks of the RODAS5P fast drivers: prescribed
//! time-dependent `(H_1, H_2, H_3)` with its exact `f_t`, a comoving energy
//! coordinate (the shears `sigma_k = H_k - H`), the rank-six unpolarized
//! Thomson kernel per energy group, a moving absorption threshold and the
//! number ledgers.
//!
//! State layout, `M = nm * nphi` angular cells per energy group `i`:
//! `[L_lo, (N_i,0 .. N_i,M-1, A_i) for i in 0..nq, L_hi]` with
//! `cell(i, j, k) = 1 + i (M + 1) + j nphi + k` and
//! `absorbed(i) = 1 + i (M + 1) + M`. For `nphi = 1` this is the P02
//! layout. The sum of all components is the semi-discrete linear
//! invariant. The operator is linear in the state, so the JVP is the RHS of
//! the direction at the same time. The Jacobian lies in the band
//! `lower = upper = M + 1`.
//!
//! Bit parity (theory section 9): every expression is evaluated as the
//! oracle parenthesizes it, without FMA and without `powf` (squares as
//! products); `sin` and `cos` are the platform libm's. The transport stages
//! run in the contract order (stage Q: columns `j`, `k`, energy faces
//! `0..=nq`; stage MU: `i`, `k`, interior faces `1..nm-1`; stage PHI: `i`,
//! `j`, periodic faces `0..nphi-1`; each face `lower -= flux` then
//! `upper += flux`), then the Thomson gain per group (moments in cell order,
//! then `s = m0 + sum_m Aw_m,c mom_m`), then the absorber. The geometry
//! quantities `hp_k`, `g_k` and `vq_jk` are recomputed inline from the same
//! expressions instead of being cached, so the callbacks allocate nothing.

#![allow(dead_code)] // shared by several example and test binaries

use std::f64::consts::TAU;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix};
use rodas5p_integrators::{BandedJacobian, OdeProblem};

/// Weights `w_m` of the unnormalized real quadrupole basis
/// `b = (P2, xz, yz, x^2 - y^2, xy)`:
/// `P2(n . n') / 2 = sum_m w_m b_m(n) b_m(n')`.
pub const QUAD_WEIGHTS: [f64; 5] = [0.5, 1.5, 1.5, 0.375, 1.5];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coordinate {
    /// `q = ln(E / E_ref)`, drift from `(H_1, H_2, H_3)`.
    Physical,
    /// `x = ln(a E / E_ref)` with `a = (a_1 a_2 a_3)^(1/3)`: the same
    /// operator evaluated with the shears `sigma_k = H_k - H`,
    /// `H = (H_1 + H_2 + H_3) / 3`; a threshold fixed in physical energy
    /// moves as `x_chi(t) = q_chi + ln a(t)`.
    Comoving,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// Zero inflow; outflow enters the ledgers.
    Outflow,
    /// No flux through the energy boundaries.
    Closed,
}

/// A typed refusal: `CoreError::InvalidInput` (values) or
/// `CoreError::Dimension` (shapes) whose message starts with a stable code
/// (the oracle's codes where they exist) followed by `": "`.
fn refuse(code: &str, message: &str) -> CoreError {
    CoreError::InvalidInput(format!("{code}: {message}"))
}

fn refuse_shape(code: &str, message: &str) -> CoreError {
    CoreError::Dimension(format!("{code}: {message}"))
}

/// The stable code of a refusal made by this module.
pub fn refusal_code(e: &CoreError) -> Option<&str> {
    match e {
        CoreError::InvalidInput(s) | CoreError::Dimension(s) | CoreError::NonFinite(s) => {
            s.split_once(": ").map(|(code, _)| code)
        }
        _ => None,
    }
}

/// Python `float.hex` of `x` (the golden fixture's format).
pub fn float_hex(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let bits = x.to_bits();
    let sign = if bits >> 63 == 1 { "-" } else { "" };
    let exponent = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & ((1_u64 << 52) - 1);
    if exponent == 0 && mantissa == 0 {
        format!("{sign}0x0.0p+0")
    } else if exponent == 0 {
        format!("{sign}0x0.{mantissa:013x}p-1022")
    } else {
        format!("{sign}0x1.{mantissa:013x}p{:+}", exponent - 1023)
    }
}

/// Validated widths of one axis (the oracle's `_edges`, in its order).
fn widths(edges: &[f64], axis: &str) -> CoreResult<Vec<f64>> {
    if edges.len() < 2 {
        return Err(refuse(
            &format!("{axis}.too_few_edges"),
            "at least two edges are required",
        ));
    }
    if edges.iter().any(|x| !x.is_finite()) {
        return Err(refuse(
            &format!("{axis}.nonfinite_edge"),
            "edges must be finite",
        ));
    }
    let w: Vec<f64> = edges.windows(2).map(|p| p[1] - p[0]).collect();
    for d in &w {
        if !d.is_finite() || *d <= 0.0 {
            return Err(refuse(
                &format!("{axis}.width"),
                "strictly increasing edges with finite widths required",
            ));
        }
        if *d < f64::MIN_POSITIVE {
            return Err(refuse(
                &format!("{axis}.subnormal_width"),
                "every width must be a normal float",
            ));
        }
    }
    Ok(w)
}

/// `[lo + ((hi - lo) j) / n for j in 0..n] + [hi]` (the oracle's helper).
pub fn uniform_edges(lo: f64, hi: f64, n: usize) -> CoreResult<Vec<f64>> {
    if n == 0 {
        return Err(refuse("grid.cell_count", "n must be a positive integer"));
    }
    let mut e: Vec<f64> = (0..n)
        .map(|j| lo + (hi - lo) * j as f64 / n as f64)
        .collect();
    e.push(hi);
    Ok(e)
}

pub fn uniform_mu_edges(n: usize) -> CoreResult<Vec<f64>> {
    uniform_edges(-1.0, 1.0, n)
}

/// `[offset + (TAU l) / n for l in 0..n] + [offset + TAU]`.
pub fn uniform_phi_edges(n: usize, offset: f64) -> CoreResult<Vec<f64>> {
    if n == 0 {
        return Err(refuse("grid.cell_count", "n must be a positive integer"));
    }
    let mut e: Vec<f64> = (0..n).map(|l| offset + TAU * l as f64 / n as f64).collect();
    e.push(offset + TAU);
    Ok(e)
}

/// `(m2, t2, p2, r)` of the mu cell `[a, b]`: the exact cell averages of
/// `mu^2`, `1 - mu^2`, `P2(mu)` and `mu sqrt(1 - mu^2)` in the oracle's
/// cancellation-free closed forms.
fn mu_cell(a: f64, b: f64) -> (f64, f64, f64, f64) {
    let d = b - a;
    let m2 = ((a * a + a * b) + b * b) / 3.0;
    let ea = (1.0 - a) * (1.0 + a);
    let eb = (1.0 - b) * (1.0 + b);
    let t2 = (ea + eb) / 2.0 + (d * d) / 6.0;
    let p2 = m2 - t2 / 2.0;
    let sa = ea.sqrt();
    let sb = eb.sqrt();
    let den = sa + sb;
    // den == 0 only for the single cell [-1, 1], where the average is 0.
    let r = if den > 0.0 {
        ((a + b) * ((ea + sa * sb) + eb)) / (3.0 * den)
    } else {
        0.0
    };
    (m2, t2, p2, r)
}

/// `(g, cphi, sphi, c2, s2h)` of the phi cell `[al, be]`: `g = D / TAU` and
/// the exact cell averages of `cos phi`, `sin phi`, `cos 2 phi` and
/// `sin phi cos phi` in half-angle product form.
fn phi_cell(al: f64, be: f64) -> (f64, f64, f64, f64, f64) {
    let delta = be - al;
    let g = delta / TAU;
    let half = 0.5 * delta;
    let sinc_half = half.sin() / half;
    let mid = 0.5 * (al + be);
    let cphi = mid.cos() * sinc_half;
    let sphi = mid.sin() * sinc_half;
    let total = al + be;
    let sinc = delta.sin() / delta;
    let c2 = total.cos() * sinc;
    let s2h = 0.5 * (total.sin() * sinc);
    (g, cphi, sphi, c2, s2h)
}

pub struct Layout3 {
    pub q_edges: Vec<f64>,
    pub mu_edges: Vec<f64>,
    pub phi_edges: Vec<f64>,
    pub dq: Vec<f64>,
    pub dm: Vec<f64>,
    pub dphi: Vec<f64>,
    pub nq: usize,
    pub nm: usize,
    pub nphi: usize,
    /// Angular cells per energy group, `nm * nphi`.
    pub m: usize,
    /// `<mu^2>_j`, `<1 - mu^2>_j`, `<P2>_j`, `<mu sqrt(1 - mu^2)>_j`, `dm_j / 2`.
    pub m2: Vec<f64>,
    pub t2: Vec<f64>,
    pub p2: Vec<f64>,
    pub r: Vec<f64>,
    pub e: Vec<f64>,
    /// `dphi_k / TAU` and the phi cell averages of `cos`, `sin`, `cos 2`,
    /// `sin cos`.
    pub g: Vec<f64>,
    pub cphi: Vec<f64>,
    pub sphi: Vec<f64>,
    pub c2: Vec<f64>,
    pub s2h: Vec<f64>,
    /// `w_l = mu_l (1 - mu_l)(1 + mu_l)` of the interior mu faces
    /// `l = 1..nm-1` (`wface[0] = 0` is unused: the pole faces carry no
    /// flux).
    pub wface: Vec<f64>,
    /// `sin(2 phi_l)` of the periodic phi faces `l = 0..nphi-1`.
    pub sn2: Vec<f64>,
    /// `B_m,c` and `Aw_m,c = w_m B_m,c` per angular cell `c = j nphi + k`.
    pub b: Vec<[f64; 5]>,
    pub aw: Vec<[f64; 5]>,
    /// Gain weight `om_c = (dm_j / 2)(dphi_k / TAU)`.
    pub om: Vec<f64>,
    pub coordinate: Coordinate,
    pub boundary: Boundary,
}

impl Layout3 {
    /// Validates the grid in the oracle's order (`q`, `mu`, `phi` edges,
    /// then the full-sphere and periodic-cover conditions) and computes
    /// every grid constant of the parity contract.
    pub fn new(
        q_edges: Vec<f64>,
        mu_edges: Vec<f64>,
        phi_edges: Vec<f64>,
        coordinate: Coordinate,
        boundary: Boundary,
    ) -> CoreResult<Self> {
        let dq = widths(&q_edges, "q")?;
        let dm = widths(&mu_edges, "mu")?;
        let dphi = widths(&phi_edges, "phi")?;
        if mu_edges[0] != -1.0 || mu_edges[mu_edges.len() - 1] != 1.0 {
            return Err(refuse(
                "mu.not_full_sphere",
                "mu edges must start at -1.0 and end at 1.0 exactly",
            ));
        }
        let phi0 = phi_edges[0];
        if !(0.0..TAU).contains(&phi0) || phi_edges[phi_edges.len() - 1] != phi0 + TAU {
            return Err(refuse(
                "phi.not_periodic_cover",
                "phi edges need 0 <= phi0 < TAU and phi[-1] == phi0 + TAU exactly",
            ));
        }
        let (nq, nm, nphi) = (dq.len(), dm.len(), dphi.len());
        let m = nm.checked_mul(nphi);
        let n = m
            .and_then(|m| m.checked_add(1))
            .and_then(|w| w.checked_mul(nq))
            .and_then(|s| s.checked_add(2));
        let (Some(m), Some(_)) = (m, n) else {
            return Err(refuse("grid.cell_count", "the state size overflows"));
        };
        let mut m2 = Vec::with_capacity(nm);
        let mut t2 = Vec::with_capacity(nm);
        let mut p2 = Vec::with_capacity(nm);
        let mut r = Vec::with_capacity(nm);
        for j in 0..nm {
            let (a, b, c, d) = mu_cell(mu_edges[j], mu_edges[j + 1]);
            m2.push(a);
            t2.push(b);
            p2.push(c);
            r.push(d);
        }
        let e: Vec<f64> = dm.iter().map(|d| d / 2.0).collect();
        let mut g = Vec::with_capacity(nphi);
        let mut cphi = Vec::with_capacity(nphi);
        let mut sphi = Vec::with_capacity(nphi);
        let mut c2 = Vec::with_capacity(nphi);
        let mut s2h = Vec::with_capacity(nphi);
        for k in 0..nphi {
            let (a, b, c, d, f) = phi_cell(phi_edges[k], phi_edges[k + 1]);
            g.push(a);
            cphi.push(b);
            sphi.push(c);
            c2.push(d);
            s2h.push(f);
        }
        let mut wface = vec![0.0; nm];
        for (l, w) in wface.iter_mut().enumerate().skip(1) {
            let mu = mu_edges[l];
            *w = mu * ((1.0 - mu) * (1.0 + mu));
        }
        let sn2: Vec<f64> = (0..nphi).map(|l| (2.0 * phi_edges[l]).sin()).collect();
        let mut b = Vec::with_capacity(m);
        let mut aw = Vec::with_capacity(m);
        let mut om = Vec::with_capacity(m);
        for j in 0..nm {
            for k in 0..nphi {
                let bc = [
                    p2[j],
                    r[j] * cphi[k],
                    r[j] * sphi[k],
                    t2[j] * c2[k],
                    t2[j] * s2h[k],
                ];
                let mut a = [0.0; 5];
                for (am, (w, bm)) in a.iter_mut().zip(QUAD_WEIGHTS.iter().zip(&bc)) {
                    *am = w * bm;
                }
                b.push(bc);
                aw.push(a);
                om.push(e[j] * g[k]);
            }
        }
        Ok(Self {
            q_edges,
            mu_edges,
            phi_edges,
            dq,
            dm,
            dphi,
            nq,
            nm,
            nphi,
            m,
            m2,
            t2,
            p2,
            r,
            e,
            g,
            cphi,
            sphi,
            c2,
            s2h,
            wface,
            sn2,
            b,
            aw,
            om,
            coordinate,
            boundary,
        })
    }

    /// State dimension `2 + nq (M + 1)`.
    pub fn n(&self) -> usize {
        2 + self.nq * (self.m + 1)
    }

    pub const LO: usize = 0;

    pub fn hi(&self) -> usize {
        self.n() - 1
    }

    /// Slot of the angular cell `c = j nphi + k` of group `i`.
    #[inline(always)]
    pub fn slot(&self, i: usize, c: usize) -> usize {
        1 + i * (self.m + 1) + c
    }

    #[inline(always)]
    pub fn cell(&self, i: usize, j: usize, k: usize) -> usize {
        1 + i * (self.m + 1) + j * self.nphi + k
    }

    #[inline(always)]
    pub fn absorbed(&self, i: usize) -> usize {
        1 + i * (self.m + 1) + self.m
    }

    /// Half bandwidth of the Jacobian.
    pub fn bandwidth(&self) -> usize {
        self.m + 1
    }

    /// The exact face-averaged normal velocities (diagnostic, allocating)
    /// for the drift `v = (v_1, v_2, v_3)`: `vq[j nphi + k]`,
    /// `vmu[k (nm - 1) + l - 1]` for the interior mu faces `l = 1..nm-1`, and
    /// `vphi[l]` for the periodic phi faces.
    pub fn face_velocities(&self, v: [f64; 3]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let (hbar, hd, h3) = (0.5 * (v[0] + v[1]), 0.5 * (v[0] - v[1]), v[2]);
        let mut vq = Vec::with_capacity(self.m);
        for j in 0..self.nm {
            for k in 0..self.nphi {
                let hp = hbar + hd * self.c2[k];
                vq.push(-(h3 * self.m2[j] + self.t2[j] * hp));
            }
        }
        let mut vmu = Vec::with_capacity(self.nphi * self.nm.saturating_sub(1));
        for k in 0..self.nphi {
            let gk = h3 - (hbar + hd * self.c2[k]);
            for l in 1..self.nm {
                vmu.push(-(gk * self.wface[l]));
            }
        }
        let vphi = (0..self.nphi).map(|l| hd * self.sn2[l]).collect();
        (vq, vmu, vphi)
    }
}

/// Prescribed triaxial Bianchi-I geometry with closed-form `ln a_k`
/// (`a_k(0) = 1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geometry3 {
    Constant {
        h1: f64,
        h2: f64,
        h3: f64,
    },
    /// `H_k = hh_k / (1 + t)`, `ln a_k = hh_k ln(1 + t)`; defined for
    /// `t > -1`.
    PowerLaw {
        hh1: f64,
        hh2: f64,
        hh3: f64,
    },
}

impl Geometry3 {
    fn parameters(&self) -> [f64; 3] {
        match *self {
            Geometry3::Constant { h1, h2, h3 } => [h1, h2, h3],
            Geometry3::PowerLaw { hh1, hh2, hh3 } => [hh1, hh2, hh3],
        }
    }

    /// The oracle's geometry refusals on the parameters: finite, and
    /// `(p_1 +- p_2) / 2` finite.
    pub fn validate(&self) -> CoreResult<()> {
        let p = self.parameters();
        if p.iter().any(|x| !x.is_finite()) {
            return Err(refuse(
                "geometry.nonfinite",
                "H1, H2, H3 (or hh1, hh2, hh3) must be finite",
            ));
        }
        if !(0.5 * (p[0] + p[1])).is_finite() || !(0.5 * (p[0] - p[1])).is_finite() {
            return Err(refuse("geometry.overflow", "(H1 +- H2)/2 overflows"));
        }
        Ok(())
    }

    pub fn is_constant(&self) -> bool {
        matches!(self, Geometry3::Constant { .. })
    }

    /// `(H, dH/dt)` at `t`, unchecked (`NaN` or infinite outside the domain).
    #[inline(always)]
    pub fn rates(&self, t: f64) -> ([f64; 3], [f64; 3]) {
        match *self {
            Geometry3::Constant { h1, h2, h3 } => ([h1, h2, h3], [0.0; 3]),
            Geometry3::PowerLaw { hh1, hh2, hh3 } => {
                let x = 1.0 + t;
                let xx = x * x;
                (
                    [hh1 / x, hh2 / x, hh3 / x],
                    [-hh1 / xx, -hh2 / xx, -hh3 / xx],
                )
            }
        }
    }

    /// [`Self::rates`] with the time domain checked.
    #[inline(always)]
    pub fn at(&self, t: f64) -> CoreResult<([f64; 3], [f64; 3])> {
        let x = 1.0 + t;
        if let Geometry3::PowerLaw { .. } = self
            && (x.is_nan() || x <= 0.0)
        {
            return Err(refuse(
                "geometry.domain",
                "the power-law geometry needs 1 + t > 0",
            ));
        }
        Ok(self.rates(t))
    }

    /// `ln a_k(t)`.
    pub fn ln_a(&self, t: f64) -> [f64; 3] {
        match *self {
            Geometry3::Constant { h1, h2, h3 } => [h1 * t, h2 * t, h3 * t],
            Geometry3::PowerLaw { hh1, hh2, hh3 } => {
                let l = t.ln_1p();
                [hh1 * l, hh2 * l, hh3 * l]
            }
        }
    }

    /// `ln a(t) = (ln a_1 + ln a_2 + ln a_3) / 3`.
    pub fn mean_ln_a(&self, t: f64) -> f64 {
        let l = self.ln_a(t);
        ((l[0] + l[1]) + l[2]) / 3.0
    }
}

/// `(v_1 + v_2 + v_3) / 3`.
#[inline(always)]
fn mean3(v: [f64; 3]) -> f64 {
    ((v[0] + v[1]) + v[2]) / 3.0
}

/// Absorber `k(E) = k0 (E / chi)^(-3)` above the threshold
/// `chi = E_ref exp(q_chi)`, zero below (P02's); the cell averages assume a
/// density uniform in the cell's energy coordinate and do not depend on
/// the angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Absorber {
    pub k0: f64,
    pub q_chi: f64,
}

/// The grid drift at `t`: `v` (the flux velocities and the upwind
/// pattern) and `dv/dt` (the velocities of `f_t`).
#[derive(Clone, Copy, Debug)]
struct Drift {
    v: [f64; 3],
    dv: [f64; 3],
}

/// The geometry quantities of one velocity set `(v_1, v_2, v_3)`:
/// `hbar = (v_1 + v_2) / 2`, `hd = (v_1 - v_2) / 2`, `v_3`.
#[derive(Clone, Copy, Debug)]
struct Faces {
    hbar: f64,
    hd: f64,
    h3: f64,
}

impl Faces {
    #[inline(always)]
    fn of(v: [f64; 3]) -> Self {
        Self {
            hbar: 0.5 * (v[0] + v[1]),
            hd: 0.5 * (v[0] - v[1]),
            h3: v[2],
        }
    }

    #[inline(always)]
    fn finite(&self) -> bool {
        self.hbar.is_finite() && self.hd.is_finite() && self.h3.is_finite()
    }

    /// `hp_k = hbar + hd <cos 2 phi>_k`.
    #[inline(always)]
    fn hp(&self, c2: f64) -> f64 {
        self.hbar + self.hd * c2
    }
}

pub struct Model3 {
    pub layout: Layout3,
    pub geometry: Geometry3,
    /// Thomson rate `c sigma_T n_e` (constant).
    pub gamma: f64,
    pub absorber: Option<Absorber>,
    /// Negative control: report a zero `f_t` for a non-autonomous problem.
    pub drop_partial_t: bool,
}

impl Model3 {
    pub fn new(
        layout: Layout3,
        geometry: Geometry3,
        gamma: f64,
        absorber: Option<Absorber>,
        drop_partial_t: bool,
    ) -> CoreResult<Self> {
        geometry.validate()?;
        if !gamma.is_finite() || gamma < 0.0 {
            return Err(refuse(
                "rate.invalid",
                "Thomson rate must be finite and >= 0",
            ));
        }
        if let Some(a) = absorber
            && (!a.k0.is_finite() || a.k0 < 0.0 || !a.q_chi.is_finite())
        {
            return Err(refuse(
                "absorber.invalid",
                "absorber must be finite with k0 >= 0",
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

    /// The drift of the grid's energy coordinate: `(H, dH/dt)`, or in
    /// comoving coordinates the shears `sigma_k = H_k - H` and their rates.
    #[inline(always)]
    fn drift(&self, t: f64) -> CoreResult<Drift> {
        let (h, dh) = self.geometry.at(t)?;
        let d = match self.layout.coordinate {
            Coordinate::Physical => Drift { v: h, dv: dh },
            Coordinate::Comoving => {
                let (hm, dhm) = (mean3(h), mean3(dh));
                Drift {
                    v: [h[0] - hm, h[1] - hm, h[2] - hm],
                    dv: [dh[0] - dhm, dh[1] - dhm, dh[2] - dhm],
                }
            }
        };
        if !Faces::of(d.v).finite() || !Faces::of(d.dv).finite() {
            return Err(CoreError::NonFinite(format!(
                "geometry.nonfinite: the drift overflows at t = {t}"
            )));
        }
        Ok(d)
    }

    /// `(v, dv/dt)` of the grid's energy coordinate at `t` (diagnostic).
    pub fn drift_at(&self, t: f64) -> CoreResult<([f64; 3], [f64; 3])> {
        self.drift(t).map(|d| (d.v, d.dv))
    }

    /// Threshold position in the grid's energy coordinate and its rate.
    #[inline(always)]
    fn threshold(&self, a: &Absorber, t: f64) -> (f64, f64) {
        match self.layout.coordinate {
            Coordinate::Physical => (a.q_chi, 0.0),
            Coordinate::Comoving => (
                a.q_chi + self.geometry.mean_ln_a(t),
                mean3(self.geometry.rates(t).0),
            ),
        }
    }

    /// Cell average of the absorber over energy cell `i` and its time
    /// derivative (P02's closed forms and tie rule: at an exact tie the
    /// branch follows the threshold's motion, so the derivative is the
    /// right-sided one).
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
            return Err(refuse_shape(
                "state.length",
                "photon transport state shape mismatch",
            ));
        }
        Ok(())
    }

    /// The oracle's three transport stages: upwind pattern from `sel`, flux
    /// velocities from `vel` (equal for the RHS; `dv/dt` for `f_t`).
    /// Accumulates into `out` (zeroed by the caller).
    #[inline(always)]
    fn transport(&self, sel: Faces, vel: Faces, y: &[f64], out: &mut [f64]) {
        let l = &self.layout;
        let (nq, nm, nphi) = (l.nq, l.nm, l.nphi);
        let hi = l.hi();
        let closed = l.boundary == Boundary::Closed;
        // Stage Q: energy faces 0..=nq of every angular column (j, k).
        for j in 0..nm {
            for k in 0..nphi {
                let jk = j * nphi + k;
                let v = -(vel.h3 * l.m2[j] + l.t2[j] * vel.hp(l.c2[k]));
                let s = -(sel.h3 * l.m2[j] + l.t2[j] * sel.hp(l.c2[k]));
                for f in 0..=nq {
                    if closed && (f == 0 || f == nq) {
                        continue;
                    }
                    let val = if s >= 0.0 {
                        if f > 0 {
                            y[l.slot(f - 1, jk)] / l.dq[f - 1]
                        } else {
                            0.0
                        }
                    } else if f < nq {
                        y[l.slot(f, jk)] / l.dq[f]
                    } else {
                        0.0
                    };
                    let flux = v * val;
                    if f > 0 {
                        out[l.slot(f - 1, jk)] -= flux;
                    } else {
                        out[Layout3::LO] -= flux;
                    }
                    if f < nq {
                        out[l.slot(f, jk)] += flux;
                    } else {
                        out[hi] += flux;
                    }
                }
            }
        }
        // Stage MU: interior mu faces 1..nm-1 for every (i, k).
        for i in 0..nq {
            let base = l.slot(i, 0);
            for k in 0..nphi {
                let gv = vel.h3 - vel.hp(l.c2[k]);
                let gs = sel.h3 - sel.hp(l.c2[k]);
                for f in 1..nm {
                    let v = -(gv * l.wface[f]);
                    let s = -(gs * l.wface[f]);
                    let up = if s >= 0.0 { f - 1 } else { f };
                    let flux = v * (y[base + up * nphi + k] / l.dm[up]);
                    out[base + (f - 1) * nphi + k] -= flux;
                    out[base + f * nphi + k] += flux;
                }
            }
        }
        // Stage PHI: periodic phi faces 0..nphi-1 for every (i, j).
        for i in 0..nq {
            for j in 0..nm {
                let base = l.cell(i, j, 0);
                for f in 0..nphi {
                    let v = vel.hd * l.sn2[f];
                    let s = sel.hd * l.sn2[f];
                    let lo = if f > 0 { f - 1 } else { nphi - 1 };
                    let up = if s >= 0.0 { lo } else { f };
                    let flux = v * (y[base + up] / l.dphi[up]);
                    out[base + lo] -= flux;
                    out[base + f] += flux;
                }
            }
        }
    }

    /// The rank-six Thomson gain of one group in the contract order:
    /// moments in cell order, then `s = m0 + sum_m Aw_m,c mom_m`, then
    /// `put(c, rate (om_c s - N_c))`.
    #[inline(always)]
    fn thomson_group(&self, group: &[f64], rate: f64, mut put: impl FnMut(usize, f64)) {
        let l = &self.layout;
        let mut m0 = 0.0;
        let mut mom = [0.0; 5];
        for (n, b) in group.iter().zip(&l.b) {
            m0 += n;
            for (mm, bm) in mom.iter_mut().zip(b) {
                *mm += bm * n;
            }
        }
        for (c, (n, a)) in group.iter().zip(&l.aw).enumerate() {
            let mut s = m0;
            for (am, mm) in a.iter().zip(&mom) {
                s += am * mm;
            }
            put(c, rate * (l.om[c] * s - n));
        }
    }

    /// The state RHS with the absorber rate of group `i` from `rate_of(i)`.
    #[inline(always)]
    fn rhs_core(
        &self,
        v: [f64; 3],
        y: &[f64],
        out: &mut [f64],
        absorb: bool,
        rate_of: impl Fn(usize) -> f64,
    ) {
        out.fill(0.0);
        let faces = Faces::of(v);
        self.transport(faces, faces, y, out);
        let l = &self.layout;
        if self.gamma != 0.0 {
            for i in 0..l.nq {
                let base = l.slot(i, 0);
                let group = &y[base..base + l.m];
                self.thomson_group(group, self.gamma, |c, gain| out[base + c] += gain);
            }
        }
        if absorb {
            for i in 0..l.nq {
                let k = rate_of(i);
                let base = l.slot(i, 0);
                let mut absorbed = 0.0;
                for c in 0..l.m {
                    let rr = k * y[base + c];
                    out[base + c] -= rr;
                    absorbed += rr;
                }
                out[l.absorbed(i)] += absorbed;
            }
        }
    }

    /// `f(t, y)`.
    pub fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.check(y, out)?;
        let d = self.drift(t)?;
        self.rhs_core(d.v, y, out, self.absorber.is_some(), |i| self.kbar(i, t).0);
        Ok(())
    }

    /// `f(t, y)` with explicit per-group absorption rates `kbar_i` in place
    /// of the absorber model (the oracle's `grid_state_rhs(..., kbar)`).
    pub fn rhs_with_group_rates(
        &self,
        t: f64,
        y: &[f64],
        kbar: &[f64],
        out: &mut [f64],
    ) -> CoreResult<()> {
        self.check(y, out)?;
        if kbar.len() != self.layout.nq || kbar.iter().any(|k| !k.is_finite() || *k < 0.0) {
            return Err(refuse(
                "absorber.invalid",
                "one finite nonnegative absorption rate per energy group required",
            ));
        }
        let d = self.drift(t)?;
        self.rhs_core(d.v, y, out, true, |i| kbar[i]);
        Ok(())
    }

    /// Transport alone with the flux velocities from `velocity` and the
    /// upwind pattern from `select` (the oracle's `select=`): with
    /// `select = v(t)` and `velocity = dv/dt` this is the transport part of
    /// `f_t`. Writes the full state vector (Thomson and absorber off).
    pub fn transport_with_select(
        &self,
        velocity: [f64; 3],
        select: [f64; 3],
        y: &[f64],
        out: &mut [f64],
    ) -> CoreResult<()> {
        self.check(y, out)?;
        let as_geometry = |v: [f64; 3]| Geometry3::Constant {
            h1: v[0],
            h2: v[1],
            h3: v[2],
        };
        as_geometry(velocity).validate()?;
        as_geometry(select).validate().map_err(|e| {
            refuse(
                "geometry.select",
                &format!("select must be a valid (H1, H2, H3): {e}"),
            )
        })?;
        out.fill(0.0);
        self.transport(Faces::of(select), Faces::of(velocity), y, out);
        Ok(())
    }

    /// One energy group's Thomson RHS `rate (om_c s_c - N_c)` (the oracle's
    /// `thomson6_rhs` on the grid's angular cells).
    pub fn thomson_group_rhs(&self, group: &[f64], rate: f64, out: &mut [f64]) -> CoreResult<()> {
        if !rate.is_finite() || rate < 0.0 {
            return Err(refuse(
                "rate.invalid",
                "Thomson rate must be finite and >= 0",
            ));
        }
        let m = self.layout.m;
        if group.len() != m || out.len() != m {
            return Err(refuse_shape(
                "state.length",
                "a group must have nm*nphi entries",
            ));
        }
        self.thomson_group(group, rate, |c, gain| out[c] = gain);
        Ok(())
    }

    /// `df/dt` at fixed state: transport velocities from `dv/dt` with the
    /// upwind pattern of `v(t)`, and the absorber rate derivatives (the
    /// Thomson rate is constant).
    pub fn partial_t(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.check(y, out)?;
        out.fill(0.0);
        if self.drop_partial_t {
            return Ok(());
        }
        let d = self.drift(t)?;
        self.transport(Faces::of(d.v), Faces::of(d.dv), y, out);
        let l = &self.layout;
        if self.absorber.is_some() {
            for i in 0..l.nq {
                let (_, dk) = self.kbar(i, t);
                let base = l.slot(i, 0);
                let mut absorbed = 0.0;
                for c in 0..l.m {
                    let rr = dk * y[base + c];
                    out[base + c] -= rr;
                    absorbed += rr;
                }
                out[l.absorbed(i)] += absorbed;
            }
        }
        Ok(())
    }

    /// Accumulates every Jacobian entry through `add(row, col, value)` in
    /// the RHS's own order (the oracle's `grid_state_jacobian_entries`), so
    /// a dense accumulation from `+0.0` reproduces `rhs(t, e_c)` bit for
    /// bit: transport `coef = v (1 / h_up)` with `add(lower, col, -coef)`
    /// then `add(upper, col, coef)`; Thomson
    /// `rate (om_r (1 + sum_m Aw_m,r B_m,c) - delta_rc)`; absorber `-kbar_i`
    /// on the diagonal, then `+kbar_i` into `A_i`.
    #[inline(always)]
    fn jacobian_entries(&self, t: f64, mut add: impl FnMut(usize, usize, f64)) -> CoreResult<()> {
        let d = self.drift(t)?;
        let vel = Faces::of(d.v);
        let l = &self.layout;
        let (nq, nm, nphi, m) = (l.nq, l.nm, l.nphi, l.m);
        let hi = l.hi();
        let closed = l.boundary == Boundary::Closed;
        for j in 0..nm {
            for k in 0..nphi {
                let jk = j * nphi + k;
                let v = -(vel.h3 * l.m2[j] + l.t2[j] * vel.hp(l.c2[k]));
                for f in 0..=nq {
                    if closed && (f == 0 || f == nq) {
                        continue;
                    }
                    let up = if v >= 0.0 {
                        if f == 0 {
                            continue;
                        }
                        f - 1
                    } else {
                        if f == nq {
                            continue;
                        }
                        f
                    };
                    let coef = v * (1.0 / l.dq[up]);
                    let col = l.slot(up, jk);
                    add(
                        if f > 0 {
                            l.slot(f - 1, jk)
                        } else {
                            Layout3::LO
                        },
                        col,
                        -coef,
                    );
                    add(if f < nq { l.slot(f, jk) } else { hi }, col, coef);
                }
            }
        }
        for i in 0..nq {
            for k in 0..nphi {
                let gv = vel.h3 - vel.hp(l.c2[k]);
                for f in 1..nm {
                    let v = -(gv * l.wface[f]);
                    let up = if v >= 0.0 { f - 1 } else { f };
                    let coef = v * (1.0 / l.dm[up]);
                    let col = l.cell(i, up, k);
                    add(l.cell(i, f - 1, k), col, -coef);
                    add(l.cell(i, f, k), col, coef);
                }
            }
        }
        for i in 0..nq {
            for j in 0..nm {
                for f in 0..nphi {
                    let v = vel.hd * l.sn2[f];
                    let lo = if f > 0 { f - 1 } else { nphi - 1 };
                    let up = if v >= 0.0 { lo } else { f };
                    let coef = v * (1.0 / l.dphi[up]);
                    let col = l.cell(i, j, up);
                    add(l.cell(i, j, lo), col, -coef);
                    add(l.cell(i, j, f), col, coef);
                }
            }
        }
        if self.gamma != 0.0 {
            for i in 0..nq {
                let base = l.slot(i, 0);
                for r in 0..m {
                    let ar = &l.aw[r];
                    for c in 0..m {
                        let bc = &l.b[c];
                        let mut s = 1.0;
                        for (am, bm) in ar.iter().zip(bc) {
                            s += am * bm;
                        }
                        let delta = if r == c { 1.0 } else { 0.0 };
                        add(base + r, base + c, self.gamma * (l.om[r] * s - delta));
                    }
                }
            }
        }
        if self.absorber.is_some() {
            for i in 0..nq {
                let (k, _) = self.kbar(i, t);
                let base = l.slot(i, 0);
                for c in 0..m {
                    add(base + c, base + c, -k);
                    add(l.absorbed(i), base + c, k);
                }
            }
        }
        Ok(())
    }

    /// Dense Jacobian in place: zeroes the whole band (every entry that can
    /// ever be nonzero), then accumulates.
    pub fn dense_fill(&self, t: f64, m: &mut DenseMatrix) -> CoreResult<()> {
        let n = self.layout.n();
        if m.nrows() != n || m.ncols() != n {
            return Err(refuse_shape(
                "jacobian.shape",
                "photon transport Jacobian shape mismatch",
            ));
        }
        let bw = self.layout.bandwidth();
        for r in 0..n {
            for c in r.saturating_sub(bw)..(r + bw + 1).min(n) {
                m[(r, c)] = 0.0;
            }
        }
        self.jacobian_entries(t, |r, c, v| m[(r, c)] += v)
    }

    /// Packed band `i * (2 bw + 1) + (j + bw - i)` (zeroed by the driver).
    pub fn band_fill(&self, t: f64, band: &mut [f64]) -> CoreResult<()> {
        let bw = self.layout.bandwidth();
        let width = 2 * bw + 1;
        if band.len() != self.layout.n() * width {
            return Err(refuse_shape(
                "band.shape",
                "photon transport band shape mismatch",
            ));
        }
        self.jacobian_entries(t, |r, c, v| band[r * width + (c + bw - r)] += v)
    }

    /// Sum of all components (the semi-discrete linear invariant).
    pub fn invariant(y: &[f64]) -> f64 {
        y.iter().sum()
    }
}

type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// The RODAS5P problem with RHS, JVP, allocation-free in-place dense
/// Jacobian and (non-autonomous) exact `f_t`.
pub fn problem(model: Arc<Model3>) -> CoreResult<OdeProblem> {
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
        "sphere-photon-fv",
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
pub fn band(model: Arc<Model3>) -> BandedJacobian {
    let bw = model.layout.bandwidth();
    BandedJacobian {
        lower: bw,
        upper: bw,
        fill: Arc::new(move |t, _, b| model.band_fill(t, b)),
    }
}
