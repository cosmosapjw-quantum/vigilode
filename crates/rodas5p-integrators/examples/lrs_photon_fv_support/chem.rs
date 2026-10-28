//! Source-owned H/He photoionization, Case-A recombination, signed stresses
//! and dynamic shear over the P02 photon transport (BASS/rei DAG node P04).
//! The single source of truth is
//! `research/bassrei_p04_source_exchange_stress_20261022/MODEL.md`; the
//! section numbers below refer to it.
//!
//! State (§2): the P02 photon layout (`n_ph = 2 + nq (nm + 1)` slots) and
//! twelve matter slots, `[photons, W_gamma, N_HI, N_HII, N_HeI, N_HeII,
//! N_HeIII, U, D_m, W_m, E_esc, R_e, s]` (offsets in [`slot`]). Densities are
//! comoving, energies comoving eV cm^-3, the shear `s` is a state and the
//! matter-dominated background `H(t) = (2/3)/(t0 + t)` is prescribed, so the
//! problem is non-autonomous with an exact `f_t` (§1, §8).
//!
//! Design:
//!
//! * Transport is P02's `Model::transport` with `(H(t), s)` passed
//!   explicitly; the geometry of the P02 model is not read by it (§2).
//! * P02's `Model::jacobian_entries` takes `(H, s)` from the model's own
//!   geometry. The Jacobian therefore keeps a second P02 model behind a
//!   `Mutex`, whose geometry is set to `Constant { h: H(t), s }` per fill,
//!   together with the scratch column `d out / d s`. Locking an uncontended
//!   `Mutex` allocates nothing; concurrent fills of one model serialize.
//! * The Jacobian is analytic, dense and filled in place (§8). Every entry
//!   that can be nonzero is zeroed first, the photon rows' band
//!   (`nm + 1`) and twelve matter columns and the twelve matter rows, then
//!   the blocks are accumulated. The matter slots come last, so the
//!   zero-skipping LU fills only the band and the border (an arrow head).
//! * The RHS and `f_t` share one kernel. `f_t` takes the flux velocity
//!   `(dH/dt, 0)` with the upwind direction of `(H, s)`, replaces `a^-3` by
//!   `d(a^-3)/dt` and `H` by `dH/dt` in the matter rows, and drops the terms
//!   free of both.
//! * A gas temperature outside `[T_min, T_max]` of the Case-A table, or
//!   non-finite, is the typed refusal `CoreError::NonFinite` with a message
//!   starting `CASE_A_T_DOMAIN:` in every callback (§6), counted by
//!   [`CaseATable::domain_refusals`]. Only a refusal allocates (its message).
//! * Scalar `libm` functions (`exp`, `ln`, `exp_m1`, `ln_1p`, `powf`,
//!   `hypot`, `sqrt`) and left-to-right association reproduce every formula
//!   with the operation order MODEL.md states.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, sha256_hex};
use rodas5p_integrators::OdeProblem;

use super::{Boundary, Coordinate, Geometry, Layout, Model};

// §0 constants (verbatim from the source owner unless marked ours).

/// Speed of light, cm/s (source C9 `C_CGS`).
pub const C: f64 = 29979245800.0;
/// Boltzmann constant, eV/K (source C10 `KB_EV_K`).
pub const KB: f64 = 8.617333262145e-5;
/// erg per eV (source C11 `rate_table.EV_ERG`).
pub const EV_ERG: f64 = 1.602176634e-12;
/// Binding energies of HI, HeI and HeII, eV (source C9 `BIND`).
pub const BIND: [f64; 3] = [13.598434599702, 24.587389011, 54.41776];
/// Thomson cross section, cm^2 (ours, CODATA).
pub const SIGMA_T: f64 = 6.6524587321e-25;
/// Newton constant, cgs (ours, CODATA).
pub const G: f64 = 6.67430e-8;
/// Shear coupling `8 pi G / (3 C C) * EV_ERG`, cm^3 eV^-1 s^-2 (ours).
pub const K_D: f64 = 8.0 * PI * G / (3.0 * C * C) * EV_ERG;
/// Verner fits `(chi, E0, sigma0, ya, p, yw, y0, y1)` of HI, HeI, HeII
/// (source C9 `experiment.py`).
pub const FIT: [[f64; 8]; 3] = [
    [13.60, 0.4298, 5.475e4, 32.88, 2.963, 0.0, 0.0, 0.0],
    [24.59, 13.61, 949.2, 1.469, 3.188, 2.039, 0.4434, 2.136],
    [54.42, 1.720, 1.369e4, 32.88, 2.963, 0.0, 0.0, 0.0],
];
/// Upper end of the energy grid, eV.
pub const E_MAX: f64 = 100.0;
/// Helium mass fraction of the initial data (§9).
pub const Y_HE: f64 = 0.2454;

/// Header of the Case-A table (§0).
pub const CASE_A_HEADER: &str = "T_K,rr_HII_cm3_s,rr_HeII_cm3_s,rr_HeIII_cm3_s,dr_HeII_cm3_s,\
rr_cool_HII_erg_cm3_s,rr_cool_HeII_erg_cm3_s,rr_cool_HeIII_erg_cm3_s,dr_cool_HeII_erg_cm3_s";
/// Data rows of the Case-A table.
pub const CASE_A_ROWS: usize = 129;
/// Rate columns: `rrHII, rrHeII, rrHeIII, drHeII, cHII, cHeII, cHeIII,
/// cdrHeII` (cm^3/s, then erg cm^3/s).
pub const CASE_A_COLUMNS: usize = 8;
/// sha256 of `owner_grackle_rates_129.csv` (§0).
pub const CASE_A_SHA256: &str = "33396b071c840b40ad79d68ee626de59f12bb2cf0b1df2159627efd7fde42aec";
/// Prefix of the typed domain refusal (§6).
pub const CASE_A_T_DOMAIN: &str = "CASE_A_T_DOMAIN:";

/// Offsets of the matter slots after the `n_ph` photon slots (§2).
pub mod slot {
    pub const W_GAMMA: usize = 0;
    pub const HI: usize = 1;
    pub const HII: usize = 2;
    pub const HEI: usize = 3;
    pub const HEII: usize = 4;
    pub const HEIII: usize = 5;
    pub const U: usize = 6;
    pub const DM: usize = 7;
    pub const WM: usize = 8;
    pub const EESC: usize = 9;
    pub const RE: usize = 10;
    pub const S: usize = 11;
    /// Number of matter slots.
    pub const COUNT: usize = 12;
    pub const NAMES: [&str; COUNT] = [
        "W_gamma", "N_HI", "N_HII", "N_HeI", "N_HeII", "N_HeIII", "U", "D_m", "W_m", "E_esc",
        "R_e", "s",
    ];
}

/// Photoionization reactants and products (HI, HeI, HeII; §4).
pub const REACTANT: [usize; 3] = [slot::HI, slot::HEI, slot::HEII];
pub const PRODUCT: [usize; 3] = [slot::HII, slot::HEII, slot::HEIII];

/// Verner cross section of species `s` (0 HI, 1 HeI, 2 HeII) at `e` eV, in
/// cm^2, in the operation order of §0; zero below `chi`.
#[inline]
pub fn verner_sigma(species: usize, e: f64) -> f64 {
    let [chi, e0, sigma0, ya, p, yw, y0, y1] = FIT[species];
    if e < chi {
        return 0.0;
    }
    let x = e / e0 - y0;
    let y = x.hypot(y1);
    let xm1 = x - 1.0;
    sigma0
        * (xm1 * xm1 + yw * yw)
        * y.powf(p / 2.0 - 5.5)
        * (1.0 + (y / ya).sqrt()).powf(-p)
        * 1e-18
}

/// The 16-point Gauss-Legendre rule on `[-1, 1]`, nodes ascending.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaussLegendre {
    pub nodes: [f64; 16],
    pub weights: [f64; 16],
}

/// `(P_n(z), P_n'(z))` by the three-term recurrence.
fn legendre(n: usize, z: f64) -> (f64, f64) {
    let (mut p0, mut p1) = (1.0, z);
    for k in 2..=n {
        let k = k as f64;
        let p2 = ((2.0 * k - 1.0) * z * p1 - (k - 1.0) * p0) / k;
        p0 = p1;
        p1 = p2;
    }
    let n = n as f64;
    (p1, n * (z * p1 - p0) / (z * z - 1.0))
}

/// The rule, computed once per process: Newton on `P_16` from
/// `cos(pi (i + 3/4) / (n + 1/2))` until the update is at most `2 eps` (at
/// most 100 iterations), weights `2 / ((1 - z^2) P'(z)^2)` at the final
/// node, nodes mirrored exactly (`x_{15-i} = -x_i`).
pub fn gauss_legendre_16() -> &'static GaussLegendre {
    static RULE: OnceLock<GaussLegendre> = OnceLock::new();
    RULE.get_or_init(|| {
        const N: usize = 16;
        let mut rule = GaussLegendre {
            nodes: [0.0; N],
            weights: [0.0; N],
        };
        for i in 0..N / 2 {
            let mut z = (PI * (i as f64 + 0.75) / (N as f64 + 0.5)).cos();
            for _ in 0..100 {
                let (p, dp) = legendre(N, z);
                let dz = p / dp;
                z -= dz;
                if dz.abs() <= 2.0 * f64::EPSILON {
                    break;
                }
            }
            let (_, dp) = legendre(N, z);
            let w = 2.0 / ((1.0 - z * z) * dp * dp);
            rule.nodes[i] = -z;
            rule.nodes[N - 1 - i] = z;
            rule.weights[i] = w;
            rule.weights[N - 1 - i] = w;
        }
        rule
    })
}

/// Cell average `(1/dq) int_lo^hi sigma_s(exp q) dq` (§4): with `mid =
/// 0.5 (lo + hi)`, `half = 0.5 (hi - lo)` and `q_k = mid + half x_k`,
/// `0.5 sum_k w_k sigma_s(exp q_k)` (`k` ascending); exactly zero for a
/// cell below the threshold (`hi <= ln chi`).
pub fn cell_average_sigma(species: usize, lo: f64, hi: f64) -> f64 {
    if hi <= FIT[species][0].ln() {
        return 0.0;
    }
    let rule = gauss_legendre_16();
    let (mid, half) = (0.5 * (lo + hi), 0.5 * (hi - lo));
    let mut sum = 0.0;
    for (x, w) in rule.nodes.iter().zip(&rule.weights) {
        sum += w * verner_sigma(species, (mid + half * x).exp());
    }
    0.5 * sum
}

fn table_error(msg: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("CASE_A_TABLE: {msg}"))
}

/// `numpy.sign` on finite values.
fn sign(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// scipy's `PchipInterpolator._edge_case`.
fn pchip_edge(h0: f64, h1: f64, m0: f64, m1: f64) -> f64 {
    let d = ((2.0 * h0 + h1) * m0 - h0 * m1) / (h0 + h1);
    if sign(d) != sign(m0) {
        0.0
    } else if sign(m0) != sign(m1) && d.abs() > 3.0 * m0.abs() {
        3.0 * m0
    } else {
        d
    }
}

/// The Case-A table with its per-column PCHIP in `(ln T, ln v)` (§0.1).
pub struct CaseATable {
    /// `T_k`, K, strictly increasing.
    pub temperatures: Vec<f64>,
    /// `v_k` per row, the [`CASE_A_COLUMNS`] rate columns.
    pub values: Vec<[f64; CASE_A_COLUMNS]>,
    /// `x_k = ln T_k`.
    pub x: Vec<f64>,
    /// `y_k = ln v_k`.
    pub y: Vec<[f64; CASE_A_COLUMNS]>,
    /// PCHIP derivatives `d_k` (in `ln v` per `ln T`).
    pub d: Vec<[f64; CASE_A_COLUMNS]>,
    /// Per interval `k` and column: `[c0, c1, c2, c3]`.
    pub coef: Vec<[[f64; 4]; CASE_A_COLUMNS]>,
    /// sha256 of the parsed bytes.
    pub sha256: String,
    domain_refusals: AtomicU64,
}

impl CaseATable {
    pub fn from_csv_path(path: impl AsRef<Path>) -> CoreResult<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|e| table_error(format!("cannot read {}: {e}", path.display())))?;
        Self::from_csv_str(&text)
    }

    /// Parses the exact header, [`CASE_A_ROWS`] rows of nine float literals
    /// (finite, positive, `T` strictly increasing) and builds the PCHIP.
    pub fn from_csv_str(text: &str) -> CoreResult<Self> {
        let mut lines = text.lines();
        if lines.next() != Some(CASE_A_HEADER) {
            return Err(table_error("header mismatch"));
        }
        let mut temperatures = Vec::with_capacity(CASE_A_ROWS);
        let mut values = Vec::with_capacity(CASE_A_ROWS);
        for (r, line) in lines.enumerate() {
            let mut fields = [0.0; CASE_A_COLUMNS + 1];
            let mut count = 0;
            for field in line.split(',') {
                if count == fields.len() {
                    return Err(table_error(format!("row {r}: more than 9 fields")));
                }
                let v: f64 = field
                    .parse()
                    .map_err(|_| table_error(format!("row {r}: not a float: {field:?}")))?;
                if !v.is_finite() || v <= 0.0 {
                    return Err(table_error(format!("row {r}: not finite positive: {v:e}")));
                }
                fields[count] = v;
                count += 1;
            }
            if count != fields.len() {
                return Err(table_error(format!("row {r}: {count} fields, 9 required")));
            }
            temperatures.push(fields[0]);
            let mut row = [0.0; CASE_A_COLUMNS];
            row.copy_from_slice(&fields[1..]);
            values.push(row);
        }
        if temperatures.len() != CASE_A_ROWS {
            return Err(table_error(format!(
                "{} data rows, {CASE_A_ROWS} required",
                temperatures.len()
            )));
        }
        let x: Vec<f64> = temperatures.iter().map(|t| t.ln()).collect();
        if temperatures.windows(2).any(|p| p[1] <= p[0]) || x.windows(2).any(|p| p[1] <= p[0]) {
            return Err(table_error("T (and ln T) must be strictly increasing"));
        }
        let y: Vec<[f64; CASE_A_COLUMNS]> = values.iter().map(|row| row.map(f64::ln)).collect();
        let m = CASE_A_ROWS;
        let h: Vec<f64> = x.windows(2).map(|p| p[1] - p[0]).collect();
        let mut d = vec![[0.0; CASE_A_COLUMNS]; m];
        let mut coef = vec![[[0.0; 4]; CASE_A_COLUMNS]; m - 1];
        for c in 0..CASE_A_COLUMNS {
            let slopes: Vec<f64> = (0..m - 1).map(|k| (y[k + 1][c] - y[k][c]) / h[k]).collect();
            for k in 1..m - 1 {
                let (m_prev, m_k) = (slopes[k - 1], slopes[k]);
                d[k][c] = if sign(m_k) != sign(m_prev) || m_k == 0.0 || m_prev == 0.0 {
                    0.0
                } else {
                    let w1 = 2.0 * h[k] + h[k - 1];
                    let w2 = h[k] + 2.0 * h[k - 1];
                    let whmean = (w1 / m_prev + w2 / m_k) / (w1 + w2);
                    1.0 / whmean
                };
            }
            d[0][c] = pchip_edge(h[0], h[1], slopes[0], slopes[1]);
            d[m - 1][c] = pchip_edge(h[m - 2], h[m - 3], slopes[m - 2], slopes[m - 3]);
            for k in 0..m - 1 {
                let slope = (y[k + 1][c] - y[k][c]) / h[k];
                let t = (d[k][c] + d[k + 1][c] - 2.0 * slope) / h[k];
                coef[k][c] = [t / h[k], (slope - d[k][c]) / h[k] - t, d[k][c], y[k][c]];
            }
        }
        Ok(Self {
            temperatures,
            values,
            x,
            y,
            d,
            coef,
            sha256: sha256_hex(text.as_bytes()),
            domain_refusals: AtomicU64::new(0),
        })
    }

    pub fn t_min(&self) -> f64 {
        self.temperatures[0]
    }

    pub fn t_max(&self) -> f64 {
        self.temperatures[CASE_A_ROWS - 1]
    }

    /// Typed refusals so far (§6), of this table and every model using it.
    pub fn domain_refusals(&self) -> u64 {
        self.domain_refusals.load(Ordering::Relaxed)
    }

    /// The interval `k` with `x_k <= x < x_{k+1}`; `x == x_{m-1}` uses
    /// `k = m - 2`.
    #[inline]
    pub fn interval(&self, x: f64) -> usize {
        self.x
            .partition_point(|xk| *xk <= x)
            .saturating_sub(1)
            .min(CASE_A_ROWS - 2)
    }

    /// `(P, P')` of every column on interval `k` at `s = X - x_k`
    /// (increasing-power order, §0.1).
    #[inline(always)]
    pub fn segment(&self, k: usize, s: f64) -> ([f64; CASE_A_COLUMNS], [f64; CASE_A_COLUMNS]) {
        let (ss, sss) = (s * s, s * s * s);
        let mut p = [0.0; CASE_A_COLUMNS];
        let mut dp = [0.0; CASE_A_COLUMNS];
        for (c, [c0, c1, c2, c3]) in self.coef[k].iter().enumerate() {
            p[c] = c3 + c2 * s + c1 * ss + c0 * sss;
            dp[c] = c2 + 2.0 * c1 * s + 3.0 * c0 * ss;
        }
        (p, dp)
    }

    /// The domain check of §6 (inclusive; NaN refused). Counts and
    /// describes a refusal.
    #[inline]
    pub fn check_domain(&self, temperature: f64) -> CoreResult<()> {
        let (lo, hi) = (self.t_min(), self.t_max());
        if temperature >= lo && temperature <= hi {
            return Ok(());
        }
        self.domain_refusals.fetch_add(1, Ordering::Relaxed);
        Err(CoreError::NonFinite(format!(
            "{CASE_A_T_DOMAIN} gas temperature {temperature:e} K outside [{lo:e}, {hi:e}]"
        )))
    }

    /// Every coefficient `v = exp(P(ln T))` and `dv/dT = v P' / T` (§0.1),
    /// after the domain check.
    #[inline]
    pub fn eval(
        &self,
        temperature: f64,
    ) -> CoreResult<([f64; CASE_A_COLUMNS], [f64; CASE_A_COLUMNS])> {
        self.check_domain(temperature)?;
        let x = temperature.ln();
        let k = self.interval(x);
        let (p, dp) = self.segment(k, x - self.x[k]);
        let mut v = [0.0; CASE_A_COLUMNS];
        let mut dv = [0.0; CASE_A_COLUMNS];
        for c in 0..CASE_A_COLUMNS {
            v[c] = p[c].exp();
            dv[c] = v[c] * dp[c] / temperature;
        }
        Ok((v, dv))
    }
}

/// Energy edges in `q = ln(E / 1 eV)` (§2): three uniform segments with
/// faces at `ln 13.60`, `ln 24.59`, `ln 54.42` and `ln 100`, each face
/// exactly the logarithm (interior edge `lo + (hi - lo) k / n`).
pub fn energy_edges(n1: usize, n2: usize, n3: usize) -> Vec<f64> {
    let faces = [FIT[0][0].ln(), FIT[1][0].ln(), FIT[2][0].ln(), E_MAX.ln()];
    let mut edges = Vec::with_capacity(n1 + n2 + n3 + 1);
    for (seg, n) in [n1, n2, n3].into_iter().enumerate() {
        let (lo, hi) = (faces[seg], faces[seg + 1]);
        for k in 0..n {
            edges.push(lo + (hi - lo) * k as f64 / n as f64);
        }
    }
    edges.push(faces[3]);
    edges
}

/// Uniform angle edges `mu_k = -1 + 2 k / nm` (§2).
pub fn angle_edges(nm: usize) -> Vec<f64> {
    (0..=nm)
        .map(|k| -1.0 + (2 * k) as f64 / nm as f64)
        .collect()
}

/// Cells per energy segment and angle cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChemGrid {
    pub n1: usize,
    pub n2: usize,
    pub n3: usize,
    pub nm: usize,
}

/// Largest state dimension a grid may declare.
pub const MAX_DIMENSION: usize = 1 << 20;

/// Scenario parameters: `H0` (s^-1), the shear coupling factor `amp` and
/// the stress relaxation time `tau_m` (s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChemParams {
    pub h0: f64,
    pub amp: f64,
    pub tau_m: f64,
}

/// Initial data of §9.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InitialData {
    pub n_h0: f64,
    pub x_hi0: f64,
    pub f_hei: f64,
    pub f_heii: f64,
    pub temperature0: f64,
    pub d_m0: f64,
    pub s0_over_h0: f64,
    pub photons_per_h: f64,
    pub alpha: f64,
    pub a2: f64,
}

/// `H`, `dH/dt`, `a^-3` and `d(a^-3)/dt` at a time (§1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Background {
    pub h: f64,
    pub dh: f64,
    pub a_inv3: f64,
    pub da_inv3: f64,
}

/// The gas slots of a state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gas {
    pub n_hi: f64,
    pub n_hii: f64,
    pub n_hei: f64,
    pub n_heii: f64,
    pub n_heiii: f64,
    pub u: f64,
}

impl Gas {
    #[inline(always)]
    fn of(y: &[f64], base: usize) -> Self {
        Self {
            n_hi: y[base + slot::HI],
            n_hii: y[base + slot::HII],
            n_hei: y[base + slot::HEI],
            n_heii: y[base + slot::HEII],
            n_heiii: y[base + slot::HEIII],
            u: y[base + slot::U],
        }
    }

    #[inline(always)]
    pub fn n_h(&self) -> f64 {
        self.n_hi + self.n_hii
    }

    #[inline(always)]
    pub fn n_he(&self) -> f64 {
        self.n_hei + self.n_heii + self.n_heiii
    }

    #[inline(always)]
    pub fn n_e(&self) -> f64 {
        self.n_hii + self.n_heii + 2.0 * self.n_heiii
    }

    /// `N_H + N_He + N_e`.
    #[inline(always)]
    pub fn particles(&self) -> f64 {
        self.n_h() + self.n_he() + self.n_e()
    }

    /// `T = U / (1.5 KB (N_H + N_He + N_e))` (§5).
    #[inline(always)]
    pub fn temperature(&self) -> f64 {
        self.u / (1.5 * KB * self.particles())
    }

    /// Implicit binding energy `B` (§4).
    #[inline(always)]
    pub fn binding(&self) -> f64 {
        BIND[0] * self.n_hii + BIND[1] * (self.n_heii + self.n_heiii) + BIND[2] * self.n_heiii
    }
}

/// `n_gamma`, `E_gamma`, `D_gamma` (§2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moments {
    pub n_gamma: f64,
    pub e_gamma: f64,
    pub d_gamma: f64,
}

/// The five linear invariants of §7.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledgers {
    pub number: f64,
    pub hydrogen: f64,
    pub helium: f64,
    pub photon: f64,
    pub energy: f64,
}

impl Ledgers {
    pub const NAMES: [&str; 5] = ["number", "hydrogen", "helium", "photon", "energy"];

    pub fn as_array(&self) -> [f64; 5] {
        [
            self.number,
            self.hydrogen,
            self.helium,
            self.photon,
            self.energy,
        ]
    }
}

/// The diagnostics of §10.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Diagnostics {
    pub moments: Moments,
    pub temperature: f64,
    pub x_hii: f64,
    pub x_heii: f64,
    pub x_heiii: f64,
    pub s_over_h: f64,
    pub ledgers: Ledgers,
}

/// The Jacobian's P02 model (geometry set per fill) and `d out / d s`.
struct JacobianWork {
    model: Model,
    ds: Vec<f64>,
}

pub struct ChemModel {
    /// P02 photon model: no Thomson, no absorber, physical coordinate,
    /// outflow boundaries; its geometry is unused (`H`, `s` are passed).
    pub photon: Model,
    pub grid: ChemGrid,
    pub params: ChemParams,
    pub table: CaseATable,
    /// `t0 = 2 / (3 H0)`.
    pub age0: f64,
    /// `K_D * amp`.
    pub k_d_amp: f64,
    /// `Ebar_i` per energy cell (§2).
    pub ebar: Vec<f64>,
    /// Photon energy weight per photon slot: `Ebar_i` on cells, `Ebar_0` on
    /// `L_lo`, `Ebar_{nq-1}` on `L_hi`, 0 on `A_i`.
    pub weights: Vec<f64>,
    /// `sbar[s][i]` (§4).
    pub sbar: [Vec<f64>; 3],
    /// Photon slots.
    pub n_ph: usize,
    jacobian: Mutex<JacobianWork>,
}

fn invalid(msg: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(msg.into())
}

fn photon_model(grid: &ChemGrid) -> CoreResult<Model> {
    let layout = Layout::new(
        energy_edges(grid.n1, grid.n2, grid.n3),
        angle_edges(grid.nm),
        Coordinate::Physical,
        Boundary::Outflow,
    )?;
    Model::new(
        layout,
        Geometry::Constant { h: 0.0, s: 0.0 },
        0.0,
        None,
        false,
    )
}

impl ChemModel {
    pub fn new(table: CaseATable, grid: ChemGrid, params: ChemParams) -> CoreResult<Self> {
        let counts = [grid.n1, grid.n2, grid.n3, grid.nm];
        if counts.contains(&0) {
            return Err(invalid("grid: n1, n2, n3 and nm must be at least 1"));
        }
        let dimension = grid
            .n1
            .checked_add(grid.n2)
            .and_then(|nq| nq.checked_add(grid.n3))
            .and_then(|nq| nq.checked_mul(grid.nm.checked_add(1)?))
            .and_then(|cells| cells.checked_add(2 + slot::COUNT));
        if dimension.is_none_or(|n| n > MAX_DIMENSION) {
            return Err(invalid(format!(
                "grid: state dimension above {MAX_DIMENSION}"
            )));
        }
        let ChemParams { h0, amp, tau_m } = params;
        if !(h0.is_finite() && h0 > 0.0) {
            return Err(invalid("params: H0 must be finite and positive"));
        }
        if !amp.is_finite() {
            return Err(invalid("params: amp must be finite"));
        }
        if !(tau_m.is_finite() && tau_m > 0.0) {
            return Err(invalid("params: tau_m must be finite and positive"));
        }
        let photon = photon_model(&grid)?;
        let transport = photon_model(&grid)?;
        let l = &photon.layout;
        let (nq, n_ph) = (l.nq, l.n());
        let ebar: Vec<f64> = (0..nq)
            .map(|i| l.q_edges[i].exp() * l.dq[i].exp_m1() / l.dq[i])
            .collect();
        let mut weights = vec![0.0; n_ph];
        for i in 0..nq {
            for j in 0..l.nm {
                weights[l.cell(i, j)] = ebar[i];
            }
        }
        weights[Layout::LO] = ebar[0];
        weights[l.hi()] = ebar[nq - 1];
        let sbar: [Vec<f64>; 3] = [0, 1, 2].map(|s| {
            (0..nq)
                .map(|i| cell_average_sigma(s, l.q_edges[i], l.q_edges[i + 1]))
                .collect()
        });
        if ebar
            .iter()
            .chain(sbar.iter().flatten())
            .any(|v| !v.is_finite())
        {
            return Err(invalid("grid: non-finite cell energy or cross section"));
        }
        Ok(Self {
            age0: 2.0 / (3.0 * h0),
            k_d_amp: K_D * amp,
            jacobian: Mutex::new(JacobianWork {
                model: transport,
                ds: vec![0.0; n_ph],
            }),
            photon,
            grid,
            params,
            table,
            ebar,
            weights,
            sbar,
            n_ph,
        })
    }

    /// State dimension `n_ph + 12`.
    pub fn n(&self) -> usize {
        self.n_ph + slot::COUNT
    }

    /// Index of matter slot `offset` ([`slot`]).
    pub fn slot(&self, offset: usize) -> usize {
        self.n_ph + offset
    }

    pub fn layout(&self) -> &Layout {
        &self.photon.layout
    }

    /// Typed refusals so far (§6).
    pub fn domain_refusals(&self) -> u64 {
        self.table.domain_refusals()
    }

    /// `H`, `dH/dt = -1.5 H H`, `a^-3 = (1 + t/t0)^-2` and
    /// `d(a^-3)/dt = -3 H a^-3` (§1).
    #[inline(always)]
    pub fn background(&self, t: f64) -> Background {
        let h = (2.0 / 3.0) / (self.age0 + t);
        let a_inv3 = (1.0 + t / self.age0).powf(-2.0);
        Background {
            h,
            dh: -1.5 * h * h,
            a_inv3,
            da_inv3: -3.0 * h * a_inv3,
        }
    }

    /// `ln a = (2/3) ln1p(t/t0)` (§1).
    pub fn ln_a(&self, t: f64) -> f64 {
        (2.0 / 3.0) * (t / self.age0).ln_1p()
    }

    fn check(&self, y: &[f64], out: usize) -> CoreResult<()> {
        let n = self.n();
        if y.len() != n || out != n {
            return Err(CoreError::Dimension(
                "photon chemistry state shape mismatch".into(),
            ));
        }
        Ok(())
    }

    /// `n_gamma`, `E_gamma`, `D_gamma`, each summed `i` then `j` ascending
    /// (`D_gamma = sum_i Ebar_i (sum_j p_j N_ij)`).
    #[inline(always)]
    pub fn moments(&self, y: &[f64]) -> Moments {
        let l = &self.photon.layout;
        let (mut n_gamma, mut e_gamma, mut d_gamma) = (0.0, 0.0, 0.0);
        for i in 0..l.nq {
            let mut quad = 0.0;
            for j in 0..l.nm {
                let n = y[l.cell(i, j)];
                n_gamma += n;
                e_gamma += self.ebar[i] * n;
                quad += l.p2[j] * n;
            }
            d_gamma += self.ebar[i] * quad;
        }
        Moments {
            n_gamma,
            e_gamma,
            d_gamma,
        }
    }

    /// `T` of a state (no domain check).
    pub fn temperature(&self, y: &[f64]) -> f64 {
        Gas::of(y, self.n_ph).temperature()
    }

    /// The five ledgers of §7, each summed left to right as written
    /// (`sum_i A_i` is summed first and then added).
    pub fn ledgers(&self, y: &[f64]) -> Ledgers {
        let l = &self.photon.layout;
        let b = self.n_ph;
        let gas = Gas::of(y, b);
        let m = self.moments(y);
        let (lo, hi) = (y[Layout::LO], y[l.hi()]);
        let mut absorbed = 0.0;
        for i in 0..l.nq {
            absorbed += y[l.absorbed(i)];
        }
        Ledgers {
            number: m.n_gamma + lo + hi + gas.n_e() + y[b + slot::RE],
            hydrogen: gas.n_h(),
            helium: gas.n_he(),
            photon: m.n_gamma + lo + hi + absorbed,
            energy: m.e_gamma
                + self.ebar[0] * lo
                + self.ebar[l.nq - 1] * hi
                + y[b + slot::W_GAMMA]
                + gas.u
                + gas.binding()
                + y[b + slot::EESC]
                + y[b + slot::WM],
        }
    }

    /// The constant weight vector of each ledger (same order as
    /// [`Ledgers::NAMES`]): every ledger is `w . y`.
    pub fn ledger_weights(&self) -> [Vec<f64>; 5] {
        let l = &self.photon.layout;
        let (n, b) = (self.n(), self.n_ph);
        let mut w = [0, 1, 2, 3, 4].map(|_| vec![0.0; n]);
        for i in 0..l.nq {
            for j in 0..l.nm {
                let c = l.cell(i, j);
                w[0][c] = 1.0;
                w[3][c] = 1.0;
                w[4][c] = self.ebar[i];
            }
            w[3][l.absorbed(i)] = 1.0;
        }
        for k in [Layout::LO, l.hi()] {
            w[0][k] = 1.0;
            w[3][k] = 1.0;
            w[4][k] = self.weights[k];
        }
        w[0][b + slot::HII] = 1.0;
        w[0][b + slot::HEII] = 1.0;
        w[0][b + slot::HEIII] = 2.0;
        w[0][b + slot::RE] = 1.0;
        w[1][b + slot::HI] = 1.0;
        w[1][b + slot::HII] = 1.0;
        for k in [slot::HEI, slot::HEII, slot::HEIII] {
            w[2][b + k] = 1.0;
        }
        for k in [slot::W_GAMMA, slot::U, slot::EESC, slot::WM] {
            w[4][b + k] = 1.0;
        }
        w[4][b + slot::HII] = BIND[0];
        w[4][b + slot::HEII] = BIND[1];
        w[4][b + slot::HEIII] = BIND[1] + BIND[2];
        w
    }

    /// The diagnostics of §10 at `(t, y)` (no domain check).
    pub fn diagnostics(&self, t: f64, y: &[f64]) -> CoreResult<Diagnostics> {
        self.check(y, y.len())?;
        let gas = Gas::of(y, self.n_ph);
        Ok(Diagnostics {
            moments: self.moments(y),
            temperature: gas.temperature(),
            x_hii: gas.n_hii / gas.n_h(),
            x_heii: gas.n_heii / gas.n_he(),
            x_heiii: gas.n_heiii / gas.n_he(),
            s_over_h: y[self.n_ph + slot::S] / self.background(t).h,
            ledgers: self.ledgers(y),
        })
    }

    /// The RHS (`RHS = true`) or `f_t` (`RHS = false`) in the term order of
    /// §2-§6.
    #[inline(always)]
    fn evaluate<const RHS: bool>(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.check(y, out.len())?;
        let b = self.n_ph;
        let gas = Gas::of(y, b);
        let (v, _) = self.table.eval(gas.temperature())?;
        out.fill(0.0);
        let bg = self.background(t);
        let s = y[b + slot::S];
        // Flux velocity, the a^-3 factor and the H of the matter rows.
        let (vh, vs, rate, hw) = if RHS {
            (bg.h, s, bg.a_inv3, bg.h)
        } else {
            (bg.dh, 0.0, bg.da_inv3, bg.dh)
        };
        let l = &self.photon.layout;
        let (nq, nm) = (l.nq, l.nm);

        // §2 transport, then the discrete photon work.
        self.photon.transport(bg.h, s, vh, vs, y, out);
        let mut work = 0.0;
        for i in 0..nq {
            for j in 0..nm {
                work += self.ebar[i] * out[l.cell(i, j)];
            }
        }
        work += self.ebar[0] * out[Layout::LO];
        work += self.ebar[nq - 1] * out[l.hi()];
        out[b + slot::W_GAMMA] = -work;

        // §3 Thomson.
        let ne = gas.n_e();
        let gam = C * SIGMA_T * (ne * rate);
        for i in 0..nq {
            let (mut m0, mut m2) = (0.0, 0.0);
            for j in 0..nm {
                let n = y[l.cell(i, j)];
                m0 += n;
                m2 += l.p2[j] * n;
            }
            for j in 0..nm {
                let n = y[l.cell(i, j)];
                out[l.cell(i, j)] += gam * (l.dm[j] / 2.0 * (m0 + 0.5 * l.p2[j] * m2) - n);
            }
        }

        // §4 photoionization, s outer, then i, then j.
        for sp in 0..3 {
            let (reactant, product) = (b + REACTANT[sp], b + PRODUCT[sp]);
            let ns = y[reactant];
            for i in 0..nq {
                let k = C * self.sbar[sp][i] * (ns * rate);
                let heat = self.ebar[i] - BIND[sp];
                let absorbed = l.absorbed(i);
                for j in 0..nm {
                    let cell = l.cell(i, j);
                    let r = k * y[cell];
                    out[cell] -= r;
                    out[absorbed] += r;
                    out[reactant] -= r;
                    out[product] += r;
                    out[b + slot::U] += heat * r;
                }
            }
        }

        // §5 recombination, cooling, escape and electron ledgers.
        let [
            rr_hii,
            rr_heii,
            rr_heiii,
            dr_heii,
            c_hii,
            c_heii,
            c_heiii,
            cdr_heii,
        ] = v;
        let ne_p = ne * rate;
        let r_h = rr_hii * ne_p * gas.n_hii;
        let r2 = (rr_heii + dr_heii) * ne_p * gas.n_heii;
        let r3 = rr_heiii * ne_p * gas.n_heiii;
        out[b + slot::HII] -= r_h;
        out[b + slot::HI] += r_h;
        out[b + slot::HEII] -= r2;
        out[b + slot::HEI] += r2;
        out[b + slot::HEIII] -= r3;
        out[b + slot::HEII] += r3;
        let lam = ne_p
            * (gas.n_hii * c_hii
                + gas.n_heii * c_heii
                + gas.n_heiii * c_heiii
                + gas.n_heii * cdr_heii)
            / EV_ERG;
        out[b + slot::U] -= lam;
        out[b + slot::EESC] += BIND[0] * r_h + BIND[1] * r2 + BIND[2] * r3 + lam;
        out[b + slot::RE] += r_h + r2 + r3;

        // §6 matter stress, gas energy work and shear.
        let (d_m, u) = (y[b + slot::DM], gas.u);
        out[b + slot::DM] += if RHS {
            -2.0 * hw * d_m - d_m / self.params.tau_m - 4.0 * s * u
        } else {
            -2.0 * hw * d_m
        };
        let a1 = 2.0 * hw * u;
        let a2 = if RHS { 2.0 * s * d_m } else { 0.0 };
        out[b + slot::U] += -a1 - a2;
        out[b + slot::WM] += a1 + a2;
        let d_gamma = self.moments(y).d_gamma;
        out[b + slot::S] = -3.0 * hw * s + self.k_d_amp * (d_gamma + d_m) * rate;
        Ok(())
    }

    /// `f(t, y)`.
    pub fn rhs(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.evaluate::<true>(t, y, out)
    }

    /// `df/dt` at fixed `y` (§8).
    pub fn partial_t(&self, t: f64, y: &[f64], out: &mut [f64]) -> CoreResult<()> {
        self.evaluate::<false>(t, y, out)
    }

    /// The analytic dense Jacobian in place (§8): zeroes every entry that
    /// can be nonzero, then accumulates the blocks.
    pub fn dense_fill(&self, t: f64, y: &[f64], m: &mut DenseMatrix) -> CoreResult<()> {
        let n = self.n();
        if m.nrows() != n || m.ncols() != n {
            return Err(CoreError::Dimension(
                "photon chemistry Jacobian shape mismatch".into(),
            ));
        }
        self.check(y, n)?;
        let b = self.n_ph;
        let gas = Gas::of(y, b);
        let temperature = gas.temperature();
        let (v, dv) = self.table.eval(temperature)?;
        let bg = self.background(t);
        let (h, am3) = (bg.h, bg.a_inv3);
        let s = y[b + slot::S];
        let l = &self.photon.layout;
        let (nq, nm) = (l.nq, l.nm);
        let bw = l.bandwidth();
        let (iw, iu, idm, iwm, is) = (
            b + slot::W_GAMMA,
            b + slot::U,
            b + slot::DM,
            b + slot::WM,
            b + slot::S,
        );

        // The pattern: photon rows on the band and the matter columns,
        // matter rows everywhere.
        for r in 0..b {
            for c in r.saturating_sub(bw)..(r + bw + 1).min(b) {
                m[(r, c)] = 0.0;
            }
            for c in b..n {
                m[(r, c)] = 0.0;
            }
        }
        for r in b..n {
            for c in 0..n {
                m[(r, c)] = 0.0;
            }
        }

        // Transport at (H, s), its W_gamma work row, and d/ds with the
        // upwind direction frozen.
        {
            let mut work = self.jacobian.lock().unwrap_or_else(PoisonError::into_inner);
            let JacobianWork { model, ds } = &mut *work;
            model.geometry = Geometry::Constant { h, s };
            let weights = &self.weights;
            model.jacobian_entries(t, |r, c, value| {
                m[(r, c)] += value;
                m[(iw, c)] -= weights[r] * value;
            });
            ds.fill(0.0);
            model.transport(h, s, 0.0, 1.0, y, ds);
            for (r, d) in ds.iter().enumerate() {
                m[(r, is)] += d;
                m[(iw, is)] -= weights[r] * d;
            }
        }

        // Thomson: the group kernel and Gam(N_e).
        let ne = gas.n_e();
        let gam = C * SIGMA_T * (ne * am3);
        let dgam = C * SIGMA_T * am3;
        for i in 0..nq {
            let (mut m0, mut m2) = (0.0, 0.0);
            for j in 0..nm {
                let n = y[l.cell(i, j)];
                m0 += n;
                m2 += l.p2[j] * n;
            }
            for j in 0..nm {
                let row = l.cell(i, j);
                for jj in 0..nm {
                    let delta = if j == jj { 1.0 } else { 0.0 };
                    m[(row, l.cell(i, jj))] +=
                        gam * (l.dm[j] / 2.0 * (1.0 + 0.5 * l.p2[j] * l.p2[jj]) - delta);
                }
                let bracket = l.dm[j] / 2.0 * (m0 + 0.5 * l.p2[j] * m2) - y[row];
                let g = dgam * bracket;
                m[(row, b + slot::HII)] += g;
                m[(row, b + slot::HEII)] += g;
                m[(row, b + slot::HEIII)] += 2.0 * g;
            }
        }

        // Photoionization: R = C sbar (N_s a^-3) N_ij.
        for sp in 0..3 {
            let (reactant, product) = (b + REACTANT[sp], b + PRODUCT[sp]);
            let ns = y[reactant];
            for i in 0..nq {
                let sb = self.sbar[sp][i];
                if sb == 0.0 {
                    continue;
                }
                let k = C * sb * (ns * am3);
                let kn = C * sb * am3;
                let heat = self.ebar[i] - BIND[sp];
                let absorbed = l.absorbed(i);
                for j in 0..nm {
                    let cell = l.cell(i, j);
                    for (col, d) in [(cell, k), (reactant, kn * y[cell])] {
                        m[(cell, col)] -= d;
                        m[(absorbed, col)] += d;
                        m[(reactant, col)] -= d;
                        m[(product, col)] += d;
                        m[(iu, col)] += heat * d;
                    }
                }
            }
        }

        // Recombination and cooling through N_e, the species and T(U, N).
        let ne_p = ne * am3;
        let ntot = gas.particles();
        let (c_h, dc_h) = (v[0], dv[0]);
        let (c_2, dc_2) = (v[1] + v[3], dv[1] + dv[3]);
        let (c_3, dc_3) = (v[2], dv[2]);
        let cool = gas.n_hii * v[4] + gas.n_heii * v[5] + gas.n_heiii * v[6] + gas.n_heii * v[7];
        let dcool =
            gas.n_hii * dv[4] + gas.n_heii * dv[5] + gas.n_heiii * dv[6] + gas.n_heii * dv[7];
        // Columns HI, HII, HeI, HeII, HeIII, U: dT/dy, dN_e/dy, d(cool)/dy
        // at fixed T, and the species index of the direct term.
        let dntot = [1.0, 2.0, 1.0, 2.0, 3.0];
        let dne = [0.0, 1.0, 0.0, 1.0, 2.0];
        let dcool_direct = [0.0, v[4], 0.0, v[5] + v[7], v[6]];
        for x in 0..6 {
            let (col, dt, dn, dcd) = if x < 5 {
                (
                    b + slot::HI + x,
                    -temperature * dntot[x] / ntot,
                    dne[x],
                    dcool_direct[x],
                )
            } else {
                (iu, 1.0 / (1.5 * KB * ntot), 0.0, 0.0)
            };
            let direct = |species: usize, c: f64| if x == species { c * ne_p } else { 0.0 };
            let d_rh = dc_h * dt * ne_p * gas.n_hii + c_h * am3 * dn * gas.n_hii + direct(1, c_h);
            let d_r2 = dc_2 * dt * ne_p * gas.n_heii + c_2 * am3 * dn * gas.n_heii + direct(3, c_2);
            let d_r3 =
                dc_3 * dt * ne_p * gas.n_heiii + c_3 * am3 * dn * gas.n_heiii + direct(4, c_3);
            let d_lam = (am3 * dn * cool + ne_p * (dcd + dcool * dt)) / EV_ERG;
            m[(b + slot::HII, col)] -= d_rh;
            m[(b + slot::HI, col)] += d_rh;
            m[(b + slot::HEII, col)] += d_r3 - d_r2;
            m[(b + slot::HEI, col)] += d_r2;
            m[(b + slot::HEIII, col)] -= d_r3;
            m[(iu, col)] -= d_lam;
            m[(b + slot::EESC, col)] += BIND[0] * d_rh + BIND[1] * d_r2 + BIND[2] * d_r3 + d_lam;
            m[(b + slot::RE, col)] += d_rh + d_r2 + d_r3;
        }

        // Matter stress, gas energy work and shear.
        let (d_m, u) = (y[idm], gas.u);
        m[(idm, idm)] += -2.0 * h - 1.0 / self.params.tau_m;
        m[(idm, is)] += -4.0 * u;
        m[(idm, iu)] += -4.0 * s;
        m[(iu, iu)] += -2.0 * h;
        m[(iu, is)] += -2.0 * d_m;
        m[(iu, idm)] += -2.0 * s;
        m[(iwm, iu)] += 2.0 * h;
        m[(iwm, is)] += 2.0 * d_m;
        m[(iwm, idm)] += 2.0 * s;
        let shear = self.k_d_amp * am3;
        m[(is, is)] += -3.0 * h;
        m[(is, idm)] += shear;
        for i in 0..nq {
            for j in 0..nm {
                m[(is, l.cell(i, j))] += shear * self.ebar[i] * l.p2[j];
            }
        }
        Ok(())
    }

    /// The initial state of §9.
    pub fn initial_state(&self, init: &InitialData) -> CoreResult<Vec<f64>> {
        let InitialData {
            n_h0,
            x_hi0,
            f_hei,
            f_heii,
            temperature0,
            d_m0,
            s0_over_h0,
            photons_per_h,
            alpha,
            a2,
        } = *init;
        let finite = [
            n_h0,
            x_hi0,
            f_hei,
            f_heii,
            temperature0,
            d_m0,
            s0_over_h0,
            photons_per_h,
            alpha,
            a2,
        ];
        if finite.iter().any(|v| !v.is_finite()) {
            return Err(invalid("initial: every value must be finite"));
        }
        if n_h0 <= 0.0 || temperature0 <= 0.0 {
            return Err(invalid("initial: nH0 and T0 must be positive"));
        }
        if !(0.0..=1.0).contains(&x_hi0) {
            return Err(invalid("initial: xHI0 must lie in [0, 1]"));
        }
        if f_hei < 0.0 || f_heii < 0.0 || f_hei + f_heii > 1.0 {
            return Err(invalid("initial: fHeI, fHeII >= 0 with fHeI + fHeII <= 1"));
        }
        if photons_per_h < 0.0 {
            return Err(invalid("initial: photons_per_H must be >= 0"));
        }
        let l = &self.photon.layout;
        let b = self.n_ph;
        let mut y = vec![0.0; self.n()];
        let weight = |i: usize, j: usize| {
            self.ebar[i].powf(-alpha) * l.dq[i] * (l.dm[j] / 2.0) * (1.0 + a2 * l.p2[j])
        };
        let mut total_weight = 0.0;
        for i in 0..l.nq {
            for j in 0..l.nm {
                let w = weight(i, j);
                if !(w.is_finite() && w > 0.0) {
                    return Err(invalid(
                        "initial: photon weights Ebar^-alpha dq dm/2 (1 + a2 p) must be finite positive",
                    ));
                }
                total_weight += w;
            }
        }
        if !total_weight.is_finite() {
            return Err(invalid("initial: photon weight sum overflows"));
        }
        let total = photons_per_h * n_h0;
        for i in 0..l.nq {
            for j in 0..l.nm {
                y[l.cell(i, j)] = total * weight(i, j) / total_weight;
            }
        }
        let n_he = n_h0 * Y_HE / (4.0 * (1.0 - Y_HE));
        let n_hii = (1.0 - x_hi0) * n_h0;
        let n_hei = f_hei * n_he;
        let n_heii = f_heii * n_he;
        let n_heiii = n_he - n_hei - n_heii;
        let gas = Gas {
            n_hi: x_hi0 * n_h0,
            n_hii,
            n_hei,
            n_heii,
            n_heiii,
            u: 0.0,
        };
        y[b + slot::HI] = gas.n_hi;
        y[b + slot::HII] = n_hii;
        y[b + slot::HEI] = n_hei;
        y[b + slot::HEII] = n_heii;
        y[b + slot::HEIII] = n_heiii;
        y[b + slot::U] = 1.5 * KB * temperature0 * (n_h0 + n_he + gas.n_e());
        y[b + slot::DM] = d_m0;
        y[b + slot::S] = s0_over_h0 * self.params.h0;
        if y.iter().any(|v| !v.is_finite()) {
            return Err(invalid("initial: non-finite initial state"));
        }
        Ok(y)
    }
}

type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

/// The RODAS5P problem: non-autonomous RHS, exact `f_t`, the analytic
/// Jacobian (allocating and in place); no JVP.
pub fn problem(model: Arc<ChemModel>) -> CoreResult<OdeProblem> {
    let n = model.n();
    let (m_rhs, m_jac, m_ft, m_into) = (model.clone(), model.clone(), model.clone(), model);
    let partial_t: PartialT = Arc::new(move |t, y, out| m_ft.partial_t(t, y, out));
    Ok(OdeProblem::new(
        "lrs-photon-chem",
        n,
        Arc::new(move |t, y, out| m_rhs.rhs(t, y, out)),
        None,
        Some(Arc::new(move |t, y| {
            let mut m = DenseMatrix::zeros(n, n);
            m_jac.dense_fill(t, y, &mut m)?;
            Ok(m)
        })),
        None,
        Some(partial_t),
        false,
        None,
        None,
    )?
    .with_jacobian_into(Arc::new(move |t, y, m| m_into.dense_fill(t, y, m))))
}
