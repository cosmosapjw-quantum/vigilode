//! RED contracts for PW04/PPSW1-6b: the opt-in singly implicit, common-shift, two-step Peer-W
//! method of Schmitt & Weiner (SINUM 42(1), 2004), variant 6b (s = 6, gamma = 0.6194119750...,
//! knots c = (-1, -1/2, 0, 2/5, 4/5, 1)).
//!
//! Evidence behind every number: `docs/agent/unified_local_agent_20261109/
//! PW04_PPSW1_6BC_VERIFICATION_KO.md` (+ `.json`, two independent routes agree to ~87 digits),
//! and the SINUM paper (eqs. (14)-(16), (20)-(24), Tables 1-2).  The Vandermonde convention is
//! the node-row one, V_ij = c_i^(j-1) (the printed one is inconsistent with B*1 = 1; see the
//! UNRESOLVED list in `PW04_PPSW1_6B_RED_TEST_KO.md`).
//!
//! The test calls an opt-in public module `rodas5p_integrators::ppsw1` that does NOT exist yet.
//! Until it exists this test binary fails to compile: that is the RED state.
//!
//! The oracle is independent of the implementation: exact rational arithmetic (i128 with
//! checked operations) for everything that does not involve gamma, and a double-double plus a
//! tiny big-integer layer for gamma (a root of a degree-6 polynomial).  Every coefficient is
//! affine in gamma: B = B0 + gamma B1, A = beta + gamma Theta, Gamma = -gamma Theta.
//!
//! NOT pinned (UNRESOLVED, owner decision pending): the starter, the error estimator and any
//! step controller, strong-stiff global order, the W-tolerance window 0.8-1.0 and the printed
//! Vandermonde convention.  See `unresolved_items_are_listed_not_tested`.

#![allow(clippy::needless_range_loop, clippy::type_complexity)]

use std::cmp::Ordering;
use std::sync::Arc;

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::ppsw1::{self, Config, DerivativeClass, Session, StageInput, State};
use rodas5p_integrators::{
    CandidateCatalog, CandidateFamily, CandidateStatus, OdeProblem, ParallelExecution, ptsw2b,
};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------------------
// Exact rational oracle (i128, checked: overflow panics instead of wrapping in release)
// ---------------------------------------------------------------------------------------

fn ck(x: Option<i128>) -> i128 {
    x.expect("i128 overflow in the exact oracle")
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rat {
    n: i128,
    d: i128,
}

impl Rat {
    fn new(n: i128, d: i128) -> Rat {
        assert!(d != 0);
        let g = gcd(n, d).max(1);
        let s = if d < 0 { -1 } else { 1 };
        Rat {
            n: s * (n / g),
            d: s * (d / g),
        }
    }
    fn zero() -> Rat {
        Rat { n: 0, d: 1 }
    }
    fn one() -> Rat {
        Rat { n: 1, d: 1 }
    }
    fn is_zero(self) -> bool {
        self.n == 0
    }
    fn inv(self) -> Rat {
        Rat::new(self.d, self.n)
    }
    fn pow(self, k: u32) -> Rat {
        (0..k).fold(Rat::one(), |acc, _| acc * self)
    }
}
impl std::ops::Add for Rat {
    type Output = Rat;
    fn add(self, o: Rat) -> Rat {
        // Least-common-denominator form keeps intermediates small (no needless growth).
        let g = gcd(self.d, o.d).max(1);
        let (ds, do_) = (self.d / g, o.d / g);
        Rat::new(
            ck(ck(self.n.checked_mul(do_)).checked_add(ck(o.n.checked_mul(ds)))),
            ck(ds.checked_mul(o.d)),
        )
    }
}
impl std::ops::Neg for Rat {
    type Output = Rat;
    fn neg(self) -> Rat {
        Rat {
            n: -self.n,
            d: self.d,
        }
    }
}
impl std::ops::Sub for Rat {
    type Output = Rat;
    fn sub(self, o: Rat) -> Rat {
        self + (-o)
    }
}
impl std::ops::Mul for Rat {
    type Output = Rat;
    fn mul(self, o: Rat) -> Rat {
        // Cross-cancel before multiplying.
        let (g1, g2) = (gcd(self.n, o.d).max(1), gcd(o.n, self.d).max(1));
        Rat::new(
            ck((self.n / g1).checked_mul(o.n / g2)),
            ck((self.d / g2).checked_mul(o.d / g1)),
        )
    }
}
impl std::ops::Div for Rat {
    type Output = Rat;
    fn div(self, o: Rat) -> Rat {
        Rat::new(ck(self.n.checked_mul(o.d)), ck(self.d.checked_mul(o.n)))
    }
}

fn r(n: i128, d: i128) -> Rat {
    Rat::new(n, d)
}

type Mat6 = [[Rat; 6]; 6];

fn rzero() -> Mat6 {
    [[Rat::zero(); 6]; 6]
}
fn ridentity() -> Mat6 {
    let mut m = rzero();
    for i in 0..6 {
        m[i][i] = Rat::one();
    }
    m
}
fn rmul(a: &Mat6, b: &Mat6) -> Mat6 {
    let mut m = rzero();
    for i in 0..6 {
        for j in 0..6 {
            for k in 0..6 {
                m[i][j] = m[i][j] + a[i][k] * b[k][j];
            }
        }
    }
    m
}
fn radd(a: &Mat6, b: &Mat6) -> Mat6 {
    let mut m = rzero();
    for i in 0..6 {
        for j in 0..6 {
            m[i][j] = a[i][j] + b[i][j];
        }
    }
    m
}
fn rscale(a: &Mat6, s: Rat) -> Mat6 {
    let mut m = rzero();
    for i in 0..6 {
        for j in 0..6 {
            m[i][j] = a[i][j] * s;
        }
    }
    m
}
fn rtranspose(a: &Mat6) -> Mat6 {
    let mut m = rzero();
    for i in 0..6 {
        for j in 0..6 {
            m[j][i] = a[i][j];
        }
    }
    m
}
fn rinverse(a: &Mat6) -> Mat6 {
    let mut m = *a;
    let mut inv = ridentity();
    for col in 0..6 {
        let piv = (col..6)
            .find(|&row| !m[row][col].is_zero())
            .expect("singular matrix in the exact oracle");
        m.swap(col, piv);
        inv.swap(col, piv);
        let d = m[col][col].inv();
        for j in 0..6 {
            m[col][j] = m[col][j] * d;
            inv[col][j] = inv[col][j] * d;
        }
        for row in 0..6 {
            if row != col && !m[row][col].is_zero() {
                let f = m[row][col];
                for j in 0..6 {
                    m[row][j] = m[row][j] - f * m[col][j];
                    inv[row][j] = inv[row][j] - f * inv[col][j];
                }
            }
        }
    }
    inv
}

/// Table 2 knots c = (-1, -1/2, 0, 2/5, 4/5, 1).
fn c_rat() -> [Rat; 6] {
    [r(-1, 1), r(-1, 2), r(0, 1), r(2, 5), r(4, 5), r(1, 1)]
}

/// Rational data of the reconstruction (PW04 route 2, node-row Vandermonde); gamma enters
/// affinely: B = b0 + gamma b1, A = beta + gamma theta, Gamma = -gamma theta.
struct Exact {
    theta: Mat6,
    beta: Mat6,
    b0: Mat6,
    b1: Mat6,
}

fn exact(sigma: Rat) -> Exact {
    let c = c_rat();
    let mut v = rzero();
    let mut s = rzero();
    let mut p = rzero();
    let mut d = rzero();
    let mut di = rzero();
    let mut f0 = rzero();
    for i in 0..6 {
        for j in 0..6 {
            v[i][j] = c[i].pow(j as u32);
            // P_ij = C(j, i) (0-based)
            let mut binom = Rat::one();
            if j >= i {
                for t in 0..i {
                    binom = binom * r((j - t) as i128, (t + 1) as i128);
                }
            } else {
                binom = Rat::zero();
            }
            p[i][j] = binom;
        }
        s[i][i] = sigma.pow(i as u32);
        d[i][i] = r((i + 1) as i128, 1);
        di[i][i] = r(1, (i + 1) as i128);
        if i >= 1 {
            f0[i][i - 1] = Rat::one();
        }
    }
    let vi = rinverse(&v);
    // Theta = V S P V^-1 (eq. (16));  E = -V D F0^T V^-1;  beta = V F0 S D^-1 V^-1 (eq. (23)).
    let theta = rmul(&rmul(&rmul(&v, &s), &p), &vi);
    let e = rscale(&rmul(&rmul(&rmul(&v, &d), &rtranspose(&f0)), &vi), r(-1, 1));
    let beta = rmul(&rmul(&rmul(&rmul(&v, &f0), &s), &di), &vi);
    // B = (I + gamma E) Theta + sigma beta E (eq. (21)).
    let b0 = radd(&theta, &rscale(&rmul(&beta, &e), sigma));
    let b1 = rmul(&e, &theta);
    Exact {
        theta,
        beta,
        b0,
        b1,
    }
}

// ---------------------------------------------------------------------------------------
// Double-double arithmetic (about 32 digits) for the gamma-affine coefficient evaluation
// ---------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Dd {
    hi: f64,
    lo: f64,
}

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}
fn quick(a: f64, b: f64) -> Dd {
    let s = a + b;
    Dd {
        hi: s,
        lo: b - (s - a),
    }
}
fn dd(x: f64) -> Dd {
    Dd { hi: x, lo: 0.0 }
}
fn dd_add(x: Dd, y: Dd) -> Dd {
    let (s, e) = two_sum(x.hi, y.hi);
    let (t, f) = two_sum(x.lo, y.lo);
    let u = quick(s, e + t);
    quick(u.hi, u.lo + f)
}
fn dd_neg(x: Dd) -> Dd {
    Dd {
        hi: -x.hi,
        lo: -x.lo,
    }
}
fn dd_sub(x: Dd, y: Dd) -> Dd {
    dd_add(x, dd_neg(y))
}
fn dd_mul(x: Dd, y: Dd) -> Dd {
    let p = x.hi * y.hi;
    let e = x.hi.mul_add(y.hi, -p) + (x.hi * y.lo + x.lo * y.hi);
    quick(p, e)
}
fn dd_div(x: Dd, y: Dd) -> Dd {
    let q1 = x.hi / y.hi;
    let rem = dd_sub(x, dd_mul(y, dd(q1)));
    let q2 = rem.hi / y.hi;
    let rem = dd_sub(rem, dd_mul(y, dd(q2)));
    let q3 = rem.hi / y.hi;
    dd_add(quick(q1, q2), dd(q3))
}
fn dd_from_i128(n: i128) -> Dd {
    let hi = n as f64;
    let rem = n - (hi as i128);
    quick(hi, rem as f64)
}
fn dd_from_rat(x: Rat) -> Dd {
    dd_div(dd_from_i128(x.n), dd_from_i128(x.d))
}
fn dd_to_f64(x: Dd) -> f64 {
    x.hi + x.lo
}

// ---------------------------------------------------------------------------------------
// gamma: root of the printed degree-6 polynomial (SINUM Table 1, s = 6 row)
// ---------------------------------------------------------------------------------------

/// Table 1, s = 6: 720 g^6 - 7920 g^5 + 32760 g^4 - 64200 g^3 + 62130 g^2 - 28098 g + 4683
/// (reproduced symbolically in PW04 verification, script s01_vtilde.py), highest degree first.
const POLY_V7: [u32; 7] = [720, 7920, 32760, 64200, 62130, 28098, 4683];
/// Signs of the coefficients in the same order.
const POLY_V7_NEGATIVE: [bool; 7] = [false, true, false, true, false, true, false];

/// gamma_hat to 60 decimals: `0.` followed by these digits.  Digits 1-40 equal the 90-digit root
/// recorded in PW04_PPSW1_6BC_VERIFICATION.json (`gamma_root_90dps`); digits 41-60 come from an
/// independent mpmath re-solve, and the exact bracket test below certifies all 60 digits.
const GAMMA_DIGITS_60: &str = "619411975066250516310763374002905109124746717004509617066045";
const GAMMA_DIGITS_40_PW04_JSON: &str = "6194119750662505163107633740029051091247";
/// The value printed in the paper (12 decimals).
const GAMMA_PRINTED: f64 = 0.619411975066;

type Big = Vec<u32>;

fn big_trim(mut a: Big) -> Big {
    while a.last() == Some(&0) {
        a.pop();
    }
    a
}
fn big_from_u32(x: u32) -> Big {
    big_trim(vec![x])
}
fn big_mul_small(a: &Big, m: u32) -> Big {
    let mut out = Vec::with_capacity(a.len() + 1);
    let mut carry = 0u64;
    for &x in a {
        let v = u64::from(x) * u64::from(m) + carry;
        out.push((v & 0xFFFF_FFFF) as u32);
        carry = v >> 32;
    }
    out.push(carry as u32);
    big_trim(out)
}
fn big_add(a: &Big, b: &Big) -> Big {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n + 1);
    let mut carry = 0u64;
    for i in 0..n {
        let v = u64::from(*a.get(i).unwrap_or(&0)) + u64::from(*b.get(i).unwrap_or(&0)) + carry;
        out.push((v & 0xFFFF_FFFF) as u32);
        carry = v >> 32;
    }
    out.push(carry as u32);
    big_trim(out)
}
fn big_mul(a: &Big, b: &Big) -> Big {
    let mut out = vec![0u32; a.len() + b.len() + 1];
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0u64;
        for (j, &y) in b.iter().enumerate() {
            let v = u64::from(out[i + j]) + u64::from(x) * u64::from(y) + carry;
            out[i + j] = (v & 0xFFFF_FFFF) as u32;
            carry = v >> 32;
        }
        let mut k = i + b.len();
        while carry != 0 {
            let v = u64::from(out[k]) + carry;
            out[k] = (v & 0xFFFF_FFFF) as u32;
            carry = v >> 32;
            k += 1;
        }
    }
    big_trim(out)
}
fn big_cmp(a: &Big, b: &Big) -> Ordering {
    let (a, b) = (big_trim(a.clone()), big_trim(b.clone()));
    a.len().cmp(&b.len()).then_with(|| {
        for i in (0..a.len()).rev() {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }
        Ordering::Equal
    })
}
fn big_from_dec(s: &str) -> Big {
    let mut v: Big = Vec::new();
    for ch in s.chars() {
        let digit = ch.to_digit(10).expect("decimal digit");
        v = big_add(&big_mul_small(&v, 10), &big_from_u32(digit));
    }
    v
}
fn big_pow_small(base: &Big, k: u32) -> Big {
    (0..k).fold(big_from_u32(1), |acc, _| big_mul(&acc, base))
}
fn big_sub_one(a: &Big) -> Big {
    let mut out = a.clone();
    for limb in out.iter_mut() {
        if *limb == 0 {
            *limb = u32::MAX;
        } else {
            *limb -= 1;
            break;
        }
    }
    big_trim(out)
}

/// Sign of P(N / 10^60) as an exact big-integer comparison of the positive and negative parts
/// of 10^(6*60) * P = sum_i a_i N^(6-i) 10^(60 i).
fn poly_sign_at(n: &Big) -> Ordering {
    let ten60 = big_pow_small(&big_from_u32(10), 60);
    let (mut pos, mut neg): (Big, Big) = (Vec::new(), Vec::new());
    for i in 0..7usize {
        let term = big_mul(
            &big_mul_small(&big_pow_small(n, (6 - i) as u32), POLY_V7[i]),
            &big_pow_small(&ten60, i as u32),
        );
        if POLY_V7_NEGATIVE[i] {
            neg = big_add(&neg, &term);
        } else {
            pos = big_add(&pos, &term);
        }
    }
    big_cmp(&pos, &neg)
}

fn gamma_dd() -> Dd {
    // 38 leading digits: error below 1e-38, far below double-double resolution of the use.
    let n38: i128 = GAMMA_DIGITS_60[..38].parse().unwrap();
    let ten38 = 10_i128.pow(38);
    dd_div(dd_from_i128(n38), dd_from_i128(ten38))
}

// ---------------------------------------------------------------------------------------
// High-precision oracle matrices for a rational sigma, rounded to f64 for comparisons
// ---------------------------------------------------------------------------------------

type M6 = [[f64; 6]; 6];

struct OracleF64 {
    b: M6,
    a: M6,
    gam: M6,
    beta: M6,
}

fn oracle_f64(sigma: Rat) -> OracleF64 {
    let ex = exact(sigma);
    let g = gamma_dd();
    let mut o = OracleF64 {
        b: [[0.0; 6]; 6],
        a: [[0.0; 6]; 6],
        gam: [[0.0; 6]; 6],
        beta: [[0.0; 6]; 6],
    };
    for i in 0..6 {
        for j in 0..6 {
            let (b0, b1) = (dd_from_rat(ex.b0[i][j]), dd_from_rat(ex.b1[i][j]));
            let (be, th) = (dd_from_rat(ex.beta[i][j]), dd_from_rat(ex.theta[i][j]));
            o.b[i][j] = dd_to_f64(dd_add(b0, dd_mul(g, b1)));
            o.a[i][j] = dd_to_f64(dd_add(be, dd_mul(g, th)));
            o.gam[i][j] = dd_to_f64(dd_neg(dd_mul(g, th)));
            o.beta[i][j] = dd_to_f64(dd_from_rat(ex.beta[i][j]));
        }
    }
    o
}

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + a.abs().max(b.abs()))
}

/// Sigma values (as exact rationals) and the matching f64 values handed to the module.
fn sigma_cases() -> Vec<(Rat, f64)> {
    vec![
        (r(1, 2), 0.5),
        (r(4, 5), 0.8),
        (r(1, 1), 1.0),
        (r(21, 20), 1.05),
        (r(11, 10), 1.1),
    ]
}

// ---------------------------------------------------------------------------------------
// (a) identity, derivative class, sigma cap constants
// ---------------------------------------------------------------------------------------

#[test]
fn a1_method_identity_is_distinct_and_opt_in() {
    let id = ppsw1::identity();
    let m = id.method_id.to_lowercase();
    assert_eq!(id.method_id, ppsw1::METHOD_ID);
    assert!(m.contains("ppsw1") && m.contains("6b"), "{m}");
    for other in ["ptsw", "mipeer", "ppsw2", "rodas", "6c"] {
        assert!(
            !m.contains(other),
            "method id must not contain {other}: {m}"
        );
    }
    assert_ne!(id.method_id, ptsw2b::identity().method_id);
    assert_eq!((ppsw1::STAGES, id.stages), (6, 6));
    // Variable-step order is exactly 5 = s - 1 (PW04 sec. 2); the constant-step superconvergence
    // is a paper claim that is NOT part of the identity.
    assert_eq!((ppsw1::FORMAL_ORDER, id.order), (5, 5));
    assert!(
        !id.default_routed,
        "PPSW1-6b is opt-in, never default routed"
    );
    assert!(!id.reuses_rodas5p_starter, "own starter, never RODAS");
    assert!(!id.reuses_rodas5p_estimator);
    assert!(!id.reuses_rodas5p_controller);
    assert!(!id.claims_shared_krylov_recurrence);
    assert!(!id.claims_g0_qualified);
}

#[test]
fn a2_derivative_class_is_exact_jacobian_or_jvp_frozen_at_step_start() {
    assert_eq!(
        ppsw1::identity().derivative_class,
        DerivativeClass::ExactJacobianOrJvpFrozenAtStepStart
    );
}

#[test]
fn a3_accepted_step_sigma_cap_is_1_10_and_the_paper_1_5_is_recorded_separately() {
    assert_eq!(ppsw1::ACCEPTED_STEP_SIGMA_MAX, 1.10);
    assert_eq!(ppsw1::PAPER_STEP_RATIO_BOUND, 1.5);
    assert_ne!(
        ppsw1::ACCEPTED_STEP_SIGMA_MAX,
        ppsw1::PAPER_STEP_RATIO_BOUND
    );
    let id = ppsw1::identity();
    assert_eq!(id.accepted_step_sigma_max, 1.10);
    assert_eq!(id.paper_step_ratio_bound, 1.5);
    assert_eq!(Config::default().step_ratio_cap, 1.10);
}

// ---------------------------------------------------------------------------------------
// (b) coefficients versus the independent high-precision oracle
// ---------------------------------------------------------------------------------------

#[test]
fn b1_exact_oracle_reproduces_the_pw04_structure() {
    // Nodes and B*1 = 1 (consistency; this is what selects the node-row Vandermonde convention).
    for (sigma, _) in sigma_cases() {
        let ex = exact(sigma);
        let c = c_rat();
        let w: Vec<Rat> = c.iter().map(|&cj| (cj - Rat::one()) / sigma).collect();
        for i in 0..6 {
            let row0 = (0..6).fold(Rat::zero(), |a, j| a + ex.b0[i][j]);
            let row1 = (0..6).fold(Rat::zero(), |a, j| a + ex.b1[i][j]);
            assert_eq!(row0, Rat::one(), "B0 1 = 1 (sigma {sigma:?}, row {i})");
            assert!(row1.is_zero(), "B1 1 = 0 (sigma {sigma:?}, row {i})");
        }
        // Gamma(6): c_i^k = sum_j Theta_ij w_j^k, k = 0..5  (Gamma = -gamma Theta, eq. (14)).
        // AB(6): c_i^k - sum_j B_ij w_j^k - k sum_j A_ij w_j^(k-1) = 0, k = 0..5 (eq. (15)),
        // for gamma^0 and gamma^1 parts separately (holds for every gamma by construction).
        for k in 0..=6u32 {
            let mut gamma_zero = true;
            let mut ab0_zero = true;
            let mut ab1_zero = true;
            for i in 0..6 {
                let mut th = Rat::zero();
                let mut r0 = c[i].pow(k);
                let mut r1 = Rat::zero();
                for j in 0..6 {
                    th = th + ex.theta[i][j] * w[j].pow(k);
                    r0 = r0 - ex.b0[i][j] * w[j].pow(k);
                    r1 = r1 - ex.b1[i][j] * w[j].pow(k);
                    if k >= 1 {
                        let kk = r(i128::from(k), 1);
                        r0 = r0 - kk * ex.beta[i][j] * w[j].pow(k - 1);
                        r1 = r1 - kk * ex.theta[i][j] * w[j].pow(k - 1);
                    }
                }
                gamma_zero &= (c[i].pow(k) - th).is_zero();
                ab0_zero &= r0.is_zero();
                ab1_zero &= r1.is_zero();
            }
            if k <= 5 {
                assert!(gamma_zero, "Gamma condition k={k} sigma={sigma:?}");
                assert!(ab0_zero && ab1_zero, "AB condition k={k} sigma={sigma:?}");
            } else {
                assert!(
                    !ab0_zero || !ab1_zero,
                    "AB(k=6) must NOT vanish: order is exactly 5 (sigma {sigma:?})"
                );
            }
        }
        // beta is nilpotent of index exactly 6 (Theorem 3.1 / Lemma 3.2).
        let mut pw = ridentity();
        for k in 1..=6 {
            pw = rmul(&pw, &ex.beta);
            let zero = pw.iter().flatten().all(|x| x.is_zero());
            assert_eq!(zero, k == 6, "beta^{k} zero? sigma {sigma:?}");
        }
    }
}

#[test]
fn b2_gamma_is_the_root_of_the_printed_degree6_polynomial_to_60_digits() {
    assert!(
        GAMMA_DIGITS_60.starts_with(GAMMA_DIGITS_40_PW04_JSON),
        "60-digit string must extend the PW04 JSON root"
    );
    assert_eq!(GAMMA_DIGITS_60.len(), 60);
    let n = big_from_dec(GAMMA_DIGITS_60);
    // P is increasing at the root (P' ~ 681), so P(N-1) < 0 < P(N+1) brackets the root within
    // one unit of the 60th decimal; exact big-integer sign evaluation.
    assert_eq!(poly_sign_at(&big_sub_one(&n)), Ordering::Less);
    assert_eq!(
        poly_sign_at(&big_add(&n, &big_from_u32(1))),
        Ordering::Greater
    );
    // The value printed in the paper agrees to its 12 printed decimals (PW04: 2.5e-13).
    assert!((dd_to_f64(gamma_dd()) - GAMMA_PRINTED).abs() < 3.0e-13);
    // The module's f64 gamma is the correctly computed root.
    assert!(approx(ppsw1::gamma(), dd_to_f64(gamma_dd()), 1e-15));
}

#[test]
fn b3_module_tableau_matches_the_high_precision_oracle_within_1e_13() {
    let c = c_rat();
    let nodes = ppsw1::nodes();
    for j in 0..6 {
        assert!(
            approx(nodes[j], dd_to_f64(dd_from_rat(c[j])), 1e-15),
            "node {j}"
        );
    }
    for (sigma, sf) in sigma_cases() {
        let t = ppsw1::tableau(sf).expect("tableau");
        let o = oracle_f64(sigma);
        assert_eq!(t.gamma.to_bits(), ppsw1::gamma().to_bits());
        assert_eq!(t.nodes, ppsw1::nodes());
        for i in 0..6 {
            for j in 0..6 {
                assert!(
                    approx(t.b_matrix[i][j], o.b[i][j], 1e-13),
                    "B[{i}][{j}] sigma {sf}"
                );
                assert!(
                    approx(t.a_matrix[i][j], o.a[i][j], 1e-13),
                    "A[{i}][{j}] sigma {sf}"
                );
                assert!(
                    approx(t.gamma_matrix[i][j], o.gam[i][j], 1e-13),
                    "Gamma[{i}][{j}] sigma {sf}"
                );
                assert!(
                    approx(t.beta[i][j], o.beta[i][j], 1e-13),
                    "beta[{i}][{j}] sigma {sf}"
                );
                // beta = A + Gamma (the stage-form identity A = beta - Gamma).
                assert!(
                    approx(t.a_matrix[i][j] + t.gamma_matrix[i][j], t.beta[i][j], 1e-12),
                    "A + Gamma != beta at [{i}][{j}] sigma {sf}"
                );
            }
        }
    }
}

#[rustfmt::skip]
const PW04_B_SIGMA1: [[&str; 6]; 6] = [
    ["-1.039327744448650826432429", "0.4517648593075787241468815", "3.083970956365938403543836", "-2.048320023367230543355699", "0.8272061632829200271244169", "-0.2752942111405557850270059"],
    ["0.01115721768504341455867813", "-1.090829602149209498793477", "1.411038790293379366089274", "1.890209534762913126721793", "-1.559112841155746533693636", "0.337536900563620125117368"],
    ["0.0442437125047321797364831", "-0.3388236444806840431101612", "1.393676943899063661699218", "-3.072480035050845815033548", "7.444855469546280244119752", "-4.471472446418546227411743"],
    ["0.7520804534964250444117248", "-5.331559599262556403493061", "20.0452264380186090766268", "-34.02725641460951229451227", "37.41858938857166666403732", "-17.85708026621463208707052"],
    ["2.667156152537627963288574", "-18.03055770240428330337959", "60.80502973796819992690156", "-94.17141656225382957137205", "90.14378211490758680362333", "-40.41399374075530181906183"],
    ["4.115016696879332002325004", "-27.27772557153782240114905", "88.92608983778046081026678", "-133.8586394151540054592769", "123.4753037496313128934186", "-54.38004529759927784558438"],
];
#[rustfmt::skip]
const PW04_A_SIGMA1: [[&str; 6]; 6] = [
    ["-0.08664021164021164021164021", "-1.116809116809116809116809", "1.66107864173291718297743", "-1.570767195767195767195767", "1.246438746438746438746439", "-0.5138888888888888888888889"],
    ["0.006547715682673196214683575", "-0.1963683539156801861914619", "-0.4842402256603581455228678", "0.7207659324979595162447163", "0.08971876601440790394622703", "-0.01701185955275176838053382"],
    ["2.783890941581624216269009e-95", "-7.423709177550997910050692e-95", "5.938967342040798328040554e-94", "-8.90845101306119749206083e-94", "1.113556376632649686507604e-93", "0.6194119750662505163107634"],
    ["-0.155248282360698808047404", "1.122926989614642544394301", "-4.031284576119820198232288", "8.084191895812718180147976", "-10.60444635547438558438723", "6.603272303593794382435411"],
    ["-1.122055388619255447366302", "7.864573257786828295090592", "-27.49450516931990660862255", "47.62200803550248667377228", "-53.23489170738158678296359", "27.78428294709768438640032"],
    ["-2.292865725120107787003966", "15.87607054760844660490027", "-54.53874442262921313463537", "91.26500951713384005947206", "-97.84023816944897875350553", "49.15018022752226352708329"],
];

#[test]
fn b4_oracle_and_module_match_the_printed_pw04_sigma1_matrices() {
    // The strings are the R1 (direct linear solve) 24-digit values recorded in the PW04 JSON
    // (variants.6b.sigma1_B_20dig / sigma1_A_20dig); the oracle here is route R2 in exact
    // rational arithmetic, so agreement is a third, independent confirmation.
    let o = oracle_f64(r(1, 1));
    let t = ppsw1::tableau(1.0).unwrap();
    for i in 0..6 {
        for j in 0..6 {
            let pb: f64 = PW04_B_SIGMA1[i][j].parse().unwrap();
            let pa: f64 = PW04_A_SIGMA1[i][j].parse().unwrap();
            assert!(approx(o.b[i][j], pb, 1e-15), "oracle B[{i}][{j}]");
            assert!(approx(o.a[i][j], pa, 1e-15), "oracle A[{i}][{j}]");
            assert!(approx(t.b_matrix[i][j], pb, 1e-13), "module B[{i}][{j}]");
            assert!(approx(t.a_matrix[i][j], pa, 1e-13), "module A[{i}][{j}]");
        }
    }
}

#[test]
fn b5_tableau_is_total_on_positive_finite_sigma_and_typed_error_otherwise() {
    // The paper's 1.5 is a coefficient-domain fact; the 1.10 cap is enforced on steps (e4), not
    // on this pure function of sigma.
    assert!(ppsw1::tableau(1.5).is_ok());
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            matches!(ppsw1::tableau(bad), Err(CoreError::InvalidInput(_))),
            "sigma {bad}"
        );
    }
}

// ---------------------------------------------------------------------------------------
// (c) order: variable-step order-5 conditions vanish, order-6 residual does not
// ---------------------------------------------------------------------------------------

/// AB(k) residual and its magnitude scale for stage row i.
fn ab_residual(t: &ppsw1::Tableau, sigma: f64, k: i32, i: usize) -> (f64, f64) {
    let c = t.nodes;
    let mut res = c[i].powi(k);
    let mut scale = 1.0 + res.abs();
    for j in 0..6 {
        let w = (c[j] - 1.0) / sigma;
        let tb = t.b_matrix[i][j] * w.powi(k);
        res -= tb;
        scale += tb.abs();
        if k >= 1 {
            let ta = f64::from(k) * t.a_matrix[i][j] * w.powi(k - 1);
            res -= ta;
            scale += ta.abs();
        }
    }
    (res, scale)
}

/// Gamma(k) residual: gamma c_i^k + sum_j Gamma_ij w_j^k.
fn gamma_residual(t: &ppsw1::Tableau, sigma: f64, k: i32, i: usize) -> (f64, f64) {
    let c = t.nodes;
    let mut res = t.gamma * c[i].powi(k);
    let mut scale = 1.0 + res.abs();
    for j in 0..6 {
        let w = (c[j] - 1.0) / sigma;
        let term = t.gamma_matrix[i][j] * w.powi(k);
        res += term;
        scale += term.abs();
    }
    (res, scale)
}

#[test]
fn c1_order5_conditions_vanish_for_module_coefficients_at_variable_step() {
    // Scaled residual |res| <= 1e-12 * (1 + sum of |terms|): the unscaled sums reach ~1e5.
    for sf in [0.5_f64, 0.8, 1.0, 1.1] {
        let t = ppsw1::tableau(sf).unwrap();
        for k in 0..=5 {
            for i in 0..6 {
                let (res, scale) = ab_residual(&t, sf, k, i);
                assert!(
                    res.abs() <= 1e-12 * scale,
                    "AB k={k} i={i} sigma={sf}: {res:e}"
                );
                let (res, scale) = gamma_residual(&t, sf, k, i);
                assert!(
                    res.abs() <= 1e-12 * scale,
                    "Gamma k={k} i={i} sigma={sf}: {res:e}"
                );
            }
        }
    }
}

#[test]
fn c2_order6_residual_is_nonzero_so_variable_step_order_is_exactly_5() {
    // PW04 sec. 2: max |AB(k=6)| is 30-153 for 6b over sigma in [0.5, 1.10]; require > 1.
    for sf in [0.5_f64, 0.8, 1.0, 1.1] {
        let t = ppsw1::tableau(sf).unwrap();
        let worst = (0..6)
            .map(|i| ab_residual(&t, sf, 6, i).0.abs())
            .fold(0.0_f64, f64::max);
        assert!(
            worst > 1.0,
            "AB(6) residual must be clearly nonzero at sigma={sf}: {worst}"
        );
    }
}

#[test]
fn c3_one_step_error_with_exact_stage_data_scales_like_h6() {
    // y' = -y (J = -1 exact, so W is exact); stage data are exact samples (no starter involved).
    // Local error O(h^6) <=> observed ratio ~ 2^6 under halving (exact order 5).
    let problem = scalar_problem(-1.0);
    let exec = ParallelExecution::sequential();
    let c = ppsw1::nodes();
    for sigma in [0.8_f64, 1.1] {
        let mut errs = Vec::new();
        for h in [0.2_f64, 0.1, 0.05] {
            let h_prev = h / sigma;
            let y: [Vec<f64>; 6] = std::array::from_fn(|j| vec![(-(c[j] - 1.0) * h_prev).exp()]);
            let f: [Vec<f64>; 6] = std::array::from_fn(|j| vec![-y[j][0]]);
            let st = State {
                t: 0.0,
                y,
                f,
                h_prev,
            };
            let mut w = WorkCounters::default();
            let out = ppsw1::advance(&problem, &st, h, &exec, &mut w).unwrap();
            errs.push((out.next.y[5][0] - (-h).exp()).abs());
        }
        for pair in errs.windows(2) {
            let order = (pair[0] / pair[1]).log2();
            assert!(
                (5.5..6.6).contains(&order),
                "observed local exponent {order} at sigma {sigma}, errors {errs:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// shared nonlinear test problem: y1' = -3 y1 + y2^2 + cos t, y2' = -2 y2 + 0.5 y1 y2
// ---------------------------------------------------------------------------------------

fn rhs2(t: f64, y: &[f64]) -> [f64; 2] {
    [
        -3.0 * y[0] + y[1] * y[1] + t.cos(),
        -2.0 * y[1] + 0.5 * y[0] * y[1],
    ]
}
fn jac2(_t: f64, y: &[f64]) -> [[f64; 2]; 2] {
    [[-3.0, 2.0 * y[1]], [0.5 * y[1], -2.0 + 0.5 * y[0]]]
}

type JacobianFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;

fn problem2(with_jacobian: bool) -> OdeProblem {
    let jacobian: Option<JacobianFn> = if with_jacobian {
        Some(Arc::new(|t: f64, y: &[f64]| {
            let j = jac2(t, y);
            DenseMatrix::from_vec_rows(vec![j[0].to_vec(), j[1].to_vec()])
        }))
    } else {
        None
    };
    let jvp: Option<JvpFn> = if with_jacobian {
        None
    } else {
        Some(Arc::new(|t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
            let j = jac2(t, y);
            out[0] = j[0][0] * v[0] + j[0][1] * v[1];
            out[1] = j[1][0] * v[0] + j[1][1] * v[1];
            Ok(())
        }))
    };
    OdeProblem::new(
        "ppsw1-contract-nonlinear-2d",
        2,
        Arc::new(|t: f64, y: &[f64], out: &mut [f64]| {
            out.copy_from_slice(&rhs2(t, y));
            Ok(())
        }),
        None,
        jacobian,
        jvp,
        None,
        false,
        None,
        None,
    )
    .unwrap()
}

fn scalar_problem(lambda: f64) -> OdeProblem {
    OdeProblem::new(
        "ppsw1-contract-scalar",
        1,
        Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = lambda * y[0];
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, _y: &[f64]| {
            DenseMatrix::from_vec_rows(vec![vec![lambda]])
        })),
        None,
        None,
        false,
        None,
        None,
    )
    .unwrap()
}

/// Six distinct previous-step stages with consistent derivative data `f_j = f(t_prev,j, y_j)`.
fn demo_state(h_prev: f64) -> State {
    let c = ppsw1::nodes();
    let y: [Vec<f64>; 6] =
        std::array::from_fn(|j| vec![1.0 + 0.02 * j as f64, 1.0 - 0.03 * j as f64]);
    let f: [Vec<f64>; 6] = std::array::from_fn(|j| rhs2((c[j] - 1.0) * h_prev, &y[j]).to_vec());
    State {
        t: 0.0,
        y,
        f,
        h_prev,
    }
}

/// Independent f64 coefficients at a rational sigma.
fn oracle_at(sigma: f64) -> OracleF64 {
    // sigma used in tests is always a small decimal; recover its exact rational form.
    let q = 1_000_000_i128;
    let p = (sigma * q as f64).round() as i128;
    oracle_f64(r(p, q))
}

// ---------------------------------------------------------------------------------------
// (d) same-step stage independence, one shared factorization, charging, determinism
// ---------------------------------------------------------------------------------------

#[test]
fn d1_stage_inputs_use_only_previous_step_data_and_oracle_formulas() {
    let (h, h_prev) = (0.0168, 0.016); // sigma = 1.05 <= 1.10
    let st = demo_state(h_prev);
    let inputs = ppsw1::stage_inputs(&st, h).unwrap();
    assert_eq!(inputs.len(), 6);
    let o = oracle_at(h / h_prev);
    let c = ppsw1::nodes();
    for i in 0..6 {
        assert!(approx(inputs[i].t, st.t + c[i] * h, 1e-14));
        for comp in 0..2 {
            let explicit = (0..6)
                .map(|j| o.b[i][j] * st.y[j][comp] + h * o.a[i][j] * st.f[j][comp])
                .sum::<f64>();
            let transfer = (0..6).map(|j| o.gam[i][j] * st.y[j][comp]).sum::<f64>();
            assert!(
                approx(inputs[i].explicit[comp], explicit, 1e-11),
                "explicit[{i}][{comp}]"
            );
            assert!(
                approx(inputs[i].transfer[comp], transfer, 1e-11),
                "transfer[{i}][{comp}]"
            );
        }
    }
}

fn stage_setup() -> (OdeProblem, State, f64, [StageInput; 6]) {
    let (h, h_prev) = (0.02, 0.02);
    let st = demo_state(h_prev);
    let inputs = ppsw1::stage_inputs(&st, h).unwrap();
    (problem2(true), st, h, inputs)
}

fn bits_equal(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

#[test]
fn d2_perturbing_one_current_stage_input_changes_no_other_current_stage_output() {
    let (problem, st, h, inputs) = stage_setup();
    let exec = ParallelExecution::sequential();
    let mut w0 = WorkCounters::default();
    let base = ppsw1::solve_stages(&problem, &st, h, &inputs, &exec, &mut w0).unwrap();
    for victim in 0..6usize {
        for field in 0..2 {
            let mut perturbed = inputs.clone();
            if field == 0 {
                perturbed[victim].explicit[0] += 1e-3;
            } else {
                perturbed[victim].transfer[1] -= 2e-3;
            }
            let mut w = WorkCounters::default();
            let out = ppsw1::solve_stages(&problem, &st, h, &perturbed, &exec, &mut w).unwrap();
            for other in (0..6).filter(|&o| o != victim) {
                assert!(
                    bits_equal(&out.y[other], &base.y[other]),
                    "stage {other} moved when stage {victim} input changed (field {field})"
                );
            }
            assert!(
                !bits_equal(&out.y[victim], &base.y[victim]),
                "perturbation must reach its own stage"
            );
        }
    }
}

#[test]
fn d3_six_stages_share_one_factorization_and_satisfy_the_stage_equations() {
    let (problem, st, h, inputs) = stage_setup();
    let exec = ParallelExecution::sequential();
    let mut w = WorkCounters::default();
    let out = ppsw1::solve_stages(&problem, &st, h, &inputs, &exec, &mut w).unwrap();
    // One W = I - h gamma T per step: one Jacobian provider call, ONE factorization, six solves.
    assert_eq!(w.jacobian_builds, 1);
    assert_eq!(
        w.direct_factorizations, 1,
        "I - h*gamma*T must be factored exactly once for all six stages"
    );
    assert_eq!(w.direct_solve_calls, 6);
    assert_eq!(w.linear_solves, 6);
    assert_eq!(
        w.rhs_calls, 0,
        "stage solves evaluate no f (sequential.rs convention)"
    );
    assert_eq!(
        (w.jvp_calls, w.nonlinear_solves, w.nonlinear_iterations),
        (0, 0, 0)
    );
    // (I - h gamma T) Y_i = explicit_i + h T transfer_i with T = J(t_m, u_m), u_m = y[5].
    let t_m = jac2(st.t, &st.y[5]);
    let g = ppsw1::gamma();
    for i in 0..6 {
        for row in 0..2 {
            let lhs =
                out.y[i][row] - h * g * (0..2).map(|k| t_m[row][k] * out.y[i][k]).sum::<f64>();
            let rhs = inputs[i].explicit[row]
                + h * (0..2)
                    .map(|k| t_m[row][k] * inputs[i].transfer[k])
                    .sum::<f64>();
            assert!(
                approx(lhs, rhs, 1e-11),
                "stage {i} row {row}: {lhs} vs {rhs}"
            );
        }
    }
}

#[test]
fn d4_advance_charges_one_factorization_six_solves_and_six_rhs_calls() {
    let problem = problem2(true);
    let st = demo_state(0.02);
    let mut w = WorkCounters::default();
    let out = ppsw1::advance(
        &problem,
        &st,
        0.02,
        &ParallelExecution::sequential(),
        &mut w,
    )
    .unwrap();
    assert_eq!(w.jacobian_builds, 1);
    assert_eq!(w.direct_factorizations, 1);
    assert_eq!(w.direct_solve_calls, 6);
    assert_eq!(w.linear_solves, 6);
    assert_eq!(
        w.rhs_calls, 6,
        "one f evaluation per new stage, charged once"
    );
    // The new history is the new stage values with their derivatives, at the new time.
    let c = ppsw1::nodes();
    assert!(approx(out.next.t, st.t + 0.02, 1e-15));
    assert_eq!(out.next.h_prev, 0.02);
    for i in 0..6 {
        let f = rhs2(st.t + c[i] * 0.02, &out.next.y[i]);
        assert!(approx(out.next.f[i][0], f[0], 1e-13) && approx(out.next.f[i][1], f[1], 1e-13));
    }
}

#[test]
fn d5_serial_and_two_thread_stage_solves_are_bitwise_identical() {
    let (problem, st, h, inputs) = stage_setup();
    let (mut w_s, mut w_p) = (WorkCounters::default(), WorkCounters::default());
    let s = ppsw1::solve_stages(
        &problem,
        &st,
        h,
        &inputs,
        &ParallelExecution::sequential(),
        &mut w_s,
    )
    .unwrap();
    let exec = ParallelExecution::rayon(2).unwrap();
    let p = ppsw1::solve_stages(&problem, &st, h, &inputs, &exec, &mut w_p).unwrap();
    for i in 0..6 {
        assert!(bits_equal(&s.y[i], &p.y[i]), "stage {i}");
    }
    assert_eq!(
        w_s, w_p,
        "work ledger must not depend on the execution backend"
    );
    let (mut a_s, mut a_p) = (WorkCounters::default(), WorkCounters::default());
    let adv_s =
        ppsw1::advance(&problem, &st, h, &ParallelExecution::sequential(), &mut a_s).unwrap();
    let adv_p = ppsw1::advance(&problem, &st, h, &exec, &mut a_p).unwrap();
    assert_eq!(
        adv_s.next, adv_p.next,
        "full step must be bitwise identical"
    );
    assert_eq!(a_s, a_p);
}

#[test]
fn d6_jvp_only_problem_reproduces_explicit_jacobian_stage_solution() {
    let (problem, st, h, inputs) = stage_setup();
    let jvp_problem = problem2(false);
    let exec = ParallelExecution::sequential();
    let (mut wa, mut wb) = (WorkCounters::default(), WorkCounters::default());
    let a = ppsw1::solve_stages(&problem, &st, h, &inputs, &exec, &mut wa).unwrap();
    let b = ppsw1::solve_stages(&jvp_problem, &st, h, &inputs, &exec, &mut wb).unwrap();
    for i in 0..6 {
        for row in 0..2 {
            assert!(approx(a.y[i][row], b.y[i][row], 1e-12));
        }
    }
    assert_eq!(
        wb.jvp_calls, 2,
        "dense T from an exact JVP costs n JVP calls and must be charged"
    );
    assert_eq!(wb.jacobian_builds, 1);
    assert_eq!(wb.direct_factorizations, 1);
    assert_eq!(wa.jvp_calls, 0);
}

// ---------------------------------------------------------------------------------------
// (e) lifecycle: transactional attempts, cap refusal
// ---------------------------------------------------------------------------------------

fn session_for(problem: &OdeProblem, st: State) -> Session<'_> {
    Session::new(
        problem,
        st,
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap()
}

#[test]
fn e1_rejected_attempt_leaves_history_and_controller_untouched_but_charges_work() {
    let problem = problem2(true);
    let st = demo_state(0.02);
    let mut session = session_for(&problem, st.clone());
    let state_before = session.state().clone();
    let ctrl_before = session.controller().clone();
    let w_before = session.counters();
    let attempt = session.attempt(0.02).unwrap();
    // Trial work is charged immediately; nothing is committed or counted yet.
    assert_eq!(*session.state(), st);
    assert_eq!(*session.controller(), ctrl_before);
    let w_trial = session.counters();
    assert_eq!(w_trial.accepted_steps, w_before.accepted_steps);
    assert_eq!(w_trial.rejected_steps, w_before.rejected_steps);
    assert_eq!(w_trial.rhs_calls, w_before.rhs_calls + 6);
    assert_eq!(
        w_trial.direct_factorizations,
        w_before.direct_factorizations + 1
    );
    assert_eq!(w_trial.linear_solves, w_before.linear_solves + 6);
    assert_eq!(w_trial.jacobian_builds, w_before.jacobian_builds + 1);
    session.reject(attempt).unwrap();
    // Owner-approved semantics (same as ptsw2b): history/controller bit-identical, accepted
    // counter untouched, rejected +1, failed work stays charged.
    assert_eq!(*session.state(), state_before);
    assert_eq!(*session.state(), st);
    assert_eq!(*session.controller(), ctrl_before);
    let w = session.counters();
    assert_eq!(w.accepted_steps, w_before.accepted_steps);
    assert_eq!(w.rejected_steps, w_before.rejected_steps + 1);
    assert_eq!(w.rhs_calls, w_trial.rhs_calls);
    assert_eq!(w.direct_factorizations, w_trial.direct_factorizations);
    assert_eq!(w.linear_solves, w_trial.linear_solves);
    // The session remains usable after a rejection.
    let retry = session.attempt(0.01).unwrap();
    session.accept(retry).unwrap();
    assert_eq!(session.counters().accepted_steps, 1);
}

#[test]
fn e2_accepted_attempt_commits_exactly_the_certified_update() {
    let problem = problem2(true);
    let (h, h_prev) = (1.0e-3, 1.25e-3); // sigma = 0.8
    let st = demo_state(h_prev);
    let mut session = session_for(&problem, st.clone());
    let attempt = session.attempt(h).unwrap();
    let proposed = attempt.next_state().clone();
    session.accept(attempt).unwrap();
    assert_eq!(
        *session.state(),
        proposed,
        "commit must be the proposed state, bit for bit"
    );
    // Independent recomputation: oracle coefficients and a 2x2 Cramer solve.
    let o = oracle_at(h / h_prev);
    let (g, c) = (ppsw1::gamma(), ppsw1::nodes());
    let t_m = jac2(st.t, &st.y[5]);
    let w = [
        [1.0 - h * g * t_m[0][0], -h * g * t_m[0][1]],
        [-h * g * t_m[1][0], 1.0 - h * g * t_m[1][1]],
    ];
    let det = w[0][0] * w[1][1] - w[0][1] * w[1][0];
    let next = session.state();
    assert!(approx(next.t, st.t + h, 1e-15));
    assert_eq!(next.h_prev, h);
    for i in 0..6 {
        let mut rhs = [0.0_f64; 2];
        for comp in 0..2 {
            let explicit = (0..6)
                .map(|j| o.b[i][j] * st.y[j][comp] + h * o.a[i][j] * st.f[j][comp])
                .sum::<f64>();
            let transfer: Vec<f64> = (0..2)
                .map(|k| (0..6).map(|j| o.gam[i][j] * st.y[j][k]).sum::<f64>())
                .collect();
            rhs[comp] = explicit + h * (t_m[comp][0] * transfer[0] + t_m[comp][1] * transfer[1]);
        }
        let y = [
            (rhs[0] * w[1][1] - w[0][1] * rhs[1]) / det,
            (w[0][0] * rhs[1] - w[1][0] * rhs[0]) / det,
        ];
        let f = rhs2(st.t + c[i] * h, &y);
        for comp in 0..2 {
            assert!(approx(next.y[i][comp], y[comp], 1e-10), "y[{i}][{comp}]");
            assert!(approx(next.f[i][comp], f[comp], 1e-10), "f[{i}][{comp}]");
        }
    }
    let cnt = session.counters();
    assert_eq!((cnt.accepted_steps, cnt.rejected_steps), (1, 0));
    assert_eq!(session.controller().accepted_ratios, vec![h / st.h_prev]);
}

#[test]
fn e3_stale_attempt_is_refused_without_side_effects() {
    let problem = problem2(true);
    let mut session = session_for(&problem, demo_state(0.02));
    let first = session.attempt(0.02).unwrap();
    let second = session.attempt(0.02).unwrap(); // built from the same committed state
    session.accept(first).unwrap();
    let state_after = session.state().clone();
    let ctrl_after = session.controller().clone();
    let w_after = session.counters();
    assert!(matches!(
        session.accept(second),
        Err(CoreError::InvalidInput(_))
    ));
    assert_eq!(*session.state(), state_after);
    assert_eq!(*session.controller(), ctrl_after);
    assert_eq!(session.counters(), w_after);
}

#[test]
fn e4_ratio_above_the_1_10_cap_is_a_typed_error_with_no_side_effects() {
    let problem = scalar_problem(-1.0);
    let c = ppsw1::nodes();
    let h_prev = 1.0;
    let y: [Vec<f64>; 6] = std::array::from_fn(|j| vec![(-(c[j] - 1.0) * h_prev).exp()]);
    let f: [Vec<f64>; 6] = std::array::from_fn(|j| vec![-y[j][0]]);
    let st = State {
        t: 0.0,
        y,
        f,
        h_prev,
    };
    let exec = ParallelExecution::sequential();
    // sigma exactly 1.10 is admissible.
    let mut w = WorkCounters::default();
    assert!(ppsw1::advance(&problem, &st, 1.1, &exec, &mut w).is_ok());
    // Anything above is refused with CoreError::InvalidInput; never a panic, never clamped.
    for h in [1.1000001, 1.2, 1.5, 3.0] {
        let mut w = WorkCounters::default();
        assert!(
            matches!(
                ppsw1::advance(&problem, &st, h, &exec, &mut w),
                Err(CoreError::InvalidInput(_))
            ),
            "advance sigma {h}"
        );
        assert_eq!(
            w,
            WorkCounters::default(),
            "refusal must not charge work (sigma {h})"
        );
        assert!(matches!(
            ppsw1::stage_inputs(&st, h),
            Err(CoreError::InvalidInput(_))
        ));
        let mut session = session_for(&problem, st.clone());
        assert!(matches!(
            session.attempt(h),
            Err(CoreError::InvalidInput(_))
        ));
        assert_eq!(*session.state(), st);
        assert_eq!(session.counters(), WorkCounters::default());
    }
    // The configured cap can only tighten the owner rule, never loosen it.
    let loose = Config {
        step_ratio_cap: ppsw1::PAPER_STEP_RATIO_BOUND,
    };
    assert!(matches!(
        Session::new(&problem, st.clone(), loose, ParallelExecution::sequential()),
        Err(CoreError::InvalidInput(_))
    ));
    let tight = Config {
        step_ratio_cap: 1.05,
    };
    let mut session =
        Session::new(&problem, st.clone(), tight, ParallelExecution::sequential()).unwrap();
    assert!(matches!(
        session.attempt(1.08),
        Err(CoreError::InvalidInput(_))
    ));
    assert!(session.attempt(1.04).is_ok());
}

// ---------------------------------------------------------------------------------------
// (f) stiff-limit and stability facts
// ---------------------------------------------------------------------------------------

fn mat_mul6(x: &M6, y: &M6) -> M6 {
    let mut m = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            for k in 0..6 {
                m[i][j] += x[i][k] * y[k][j];
            }
        }
    }
    m
}
fn max_abs6(x: &M6) -> f64 {
    x.iter().flatten().fold(0.0_f64, |a, v| a.max(v.abs()))
}

#[test]
fn f1_beta_is_nilpotent_so_the_stiff_limit_has_spectral_radius_zero() {
    for (_, sf) in sigma_cases() {
        let t = ppsw1::tableau(sf).unwrap();
        let mut p: M6 = t.beta;
        let mut maxes = Vec::new();
        for _ in 1..6 {
            maxes.push(max_abs6(&p));
            p = mat_mul6(&p, &t.beta);
        }
        // beta^5 != 0 (index exactly 6, exact oracle in b1) and beta^6 = 0 up to roundoff.
        // beta^5 = sigma^10/120 * V e6 e1^T V^-1, so its size scales like sigma^10.
        assert!(
            maxes[4] > 1e-4 * sf.powi(10),
            "beta^5 must be nonzero at sigma {sf}: {}",
            maxes[4]
        );
        assert!(
            max_abs6(&p) < 1e-9,
            "beta^6 must vanish at sigma {sf}: {}",
            max_abs6(&p)
        );
    }
}

#[test]
fn f2_stiff_limit_of_the_stability_matrix_is_minus_beta_over_gamma_with_rho_zero() {
    // M(z) = (I - gamma z)^-1 (B + z beta)  ->  -(1/gamma) beta  as |z| -> infinity.
    let t = ppsw1::tableau(1.0).unwrap();
    let g: f64 = t.gamma;
    let z: f64 = -1.0e10;
    for i in 0..6 {
        for j in 0..6 {
            let m = (t.b_matrix[i][j] + z * t.beta[i][j]) / (1.0 - g * z);
            assert!(
                (m + t.beta[i][j] / g).abs() < 1e-7 * (1.0 + t.beta[i][j].abs()),
                "M(z)[{i}][{j}] = {m} vs -beta/gamma = {}",
                -t.beta[i][j] / g
            );
        }
    }
    let mut m_inf: M6 = t.beta;
    for i in 0..6 {
        for j in 0..6 {
            m_inf[i][j] /= -g;
        }
    }
    let mut p = m_inf;
    for _ in 1..6 {
        p = mat_mul6(&p, &m_inf);
    }
    assert!(
        max_abs6(&p) < 1e-9,
        "(-beta/gamma)^6 = 0, so rho(M(inf)) = 0"
    );
}

#[derive(Clone, Copy, Debug)]
struct Cx {
    re: f64,
    im: f64,
}
impl Cx {
    fn new(re: f64, im: f64) -> Cx {
        Cx { re, im }
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
}
impl std::ops::Add for Cx {
    type Output = Cx;
    fn add(self, o: Cx) -> Cx {
        Cx::new(self.re + o.re, self.im + o.im)
    }
}
impl std::ops::Sub for Cx {
    type Output = Cx;
    fn sub(self, o: Cx) -> Cx {
        Cx::new(self.re - o.re, self.im - o.im)
    }
}
impl std::ops::Mul for Cx {
    type Output = Cx;
    fn mul(self, o: Cx) -> Cx {
        Cx::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}
impl std::ops::Div for Cx {
    type Output = Cx;
    fn div(self, o: Cx) -> Cx {
        let d = o.re * o.re + o.im * o.im;
        Cx::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
}

type C6 = [[Cx; 6]; 6];

fn cmat_mul(a: &C6, b: &C6) -> C6 {
    let mut m = [[Cx::new(0.0, 0.0); 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            let mut acc = Cx::new(0.0, 0.0);
            for k in 0..6 {
                acc = acc + a[i][k] * b[k][j];
            }
            m[i][j] = acc;
        }
    }
    m
}

/// Characteristic polynomial coefficients (index = power of lambda) by Faddeev-LeVerrier.
fn char_poly(a: &C6) -> [Cx; 7] {
    let mut c = [Cx::new(0.0, 0.0); 7];
    c[6] = Cx::new(1.0, 0.0);
    let mut m = [[Cx::new(0.0, 0.0); 6]; 6];
    for k in 1..=6usize {
        let mut mk = cmat_mul(a, &m);
        for i in 0..6 {
            mk[i][i] = mk[i][i] + c[7 - k];
        }
        let amk = cmat_mul(a, &mk);
        let tr = (0..6).fold(Cx::new(0.0, 0.0), |acc, i| acc + amk[i][i]);
        c[6 - k] = Cx::new(-tr.re / k as f64, -tr.im / k as f64);
        m = mk;
    }
    c
}

/// Largest root modulus of a monic degree-6 polynomial (Aberth-Ehrlich iteration).
fn max_root_modulus(c: &[Cx; 7]) -> f64 {
    let radius = 2.0
        * (1..=6usize)
            .map(|k| c[6 - k].abs().powf(1.0 / k as f64))
            .fold(1e-3_f64, f64::max);
    let mut z: Vec<Cx> = (0..6)
        .map(|k| {
            let ang = 2.0 * std::f64::consts::PI * k as f64 / 6.0 + 0.4;
            Cx::new(radius * ang.cos(), radius * ang.sin())
        })
        .collect();
    for _ in 0..300 {
        let mut moved = 0.0_f64;
        for i in 0..6 {
            let (mut p, mut dp) = (Cx::new(1.0, 0.0), Cx::new(0.0, 0.0));
            for k in (0..6).rev() {
                dp = dp * z[i] + p;
                p = p * z[i] + c[k];
            }
            // Horner above evaluates p(z) with leading coefficient c[6] = 1 folded in.
            let ratio = p / dp;
            let mut s = Cx::new(0.0, 0.0);
            for j in 0..6 {
                if j != i {
                    s = s + Cx::new(1.0, 0.0) / (z[i] - z[j]);
                }
            }
            let w = ratio / (Cx::new(1.0, 0.0) - ratio * s);
            z[i] = z[i] - w;
            moved = moved.max(w.abs() / (1.0 + z[i].abs()));
        }
        if moved < 1e-15 {
            break;
        }
    }
    z.iter().map(|v| v.abs()).fold(0.0_f64, f64::max)
}

/// Spectral radius of M(z) = (I - gamma z)^-1 (B + z beta) at sigma = 1.
fn rho_m(t: &ppsw1::Tableau, z: Cx) -> f64 {
    let den = Cx::new(1.0, 0.0) - Cx::new(t.gamma, 0.0) * z;
    let mut m = [[Cx::new(0.0, 0.0); 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            m[i][j] = (Cx::new(t.b_matrix[i][j], 0.0) + z * Cx::new(t.beta[i][j], 0.0)) / den;
        }
    }
    max_root_modulus(&char_poly(&m))
}

fn ray_unstable(t: &ppsw1::Tableau, phi_deg: f64, points: usize) -> bool {
    let phi = phi_deg.to_radians();
    let (lo, hi) = (-6.0_f64, 7.0_f64);
    (0..points).any(|k| {
        let r = 10f64.powf(lo + (hi - lo) * k as f64 / (points - 1) as f64);
        let rho = rho_m(t, Cx::new(-r * phi.cos(), -r * phi.sin()));
        assert!(
            rho.is_finite(),
            "non-finite spectral radius at phi={phi_deg}, r={r}"
        );
        rho > 1.0 + 1e-9
    })
}

#[test]
fn f3_l_alpha_angle_is_57_54_degrees_within_0_05() {
    // PW04 sec. 3 / paper: A(alpha) with alpha = 57.5 deg (6b).  Same scan as PW04 s04:
    // first unstable ray at phi = 0.02 k, alpha = last stable ray; coarse then fine for speed.
    let t = ppsw1::tableau(1.0).unwrap();
    let mut coarse = 0.0_f64;
    let first_unstable_coarse = loop {
        assert!(coarse < 90.0, "no unstable ray found below 90 degrees");
        if ray_unstable(&t, coarse, 800) {
            break coarse;
        }
        coarse += 0.5;
    };
    let start = ((first_unstable_coarse - 0.5).max(0.0) / 0.02).floor() as usize;
    let mut alpha = None;
    for k in start..start + 40 {
        let phi = 0.02 * k as f64;
        if ray_unstable(&t, phi, 4000) {
            alpha = Some(phi - 0.02);
            break;
        }
    }
    let alpha = alpha.expect("fine scan must find the first unstable ray");
    assert!(
        (alpha - 57.54).abs() <= 0.05,
        "L(alpha) angle {alpha} deg, expected 57.54 +- 0.05 (printed 57.5)"
    );
    // L(alpha) is a property of the sector: 45 degrees is stable.
    assert!(!ray_unstable(&t, 45.0, 800));
}

// ---------------------------------------------------------------------------------------
// (g) guards: catalog unchanged, no derivative-free label, protected sources
// ---------------------------------------------------------------------------------------

#[test]
fn g1_default_candidate_catalog_is_unchanged_and_peerw_stays_deferred() {
    assert!(!ppsw1::identity().default_routed);
    let catalog = CandidateCatalog::research_default().unwrap();
    for entry in catalog.entries() {
        let id = entry.id().to_lowercase();
        assert!(
            !id.contains("ppsw") && !id.contains("pw04") && !id.contains(ppsw1::METHOD_ID),
            "{id}"
        );
        if entry.family() == CandidateFamily::PeerW {
            assert!(
                matches!(entry.status(), CandidateStatus::Deferred { .. }),
                "PeerW stub must stay Deferred; PPSW1-6b is exposed only through rodas5p_integrators::ppsw1"
            );
        }
    }
    assert!(
        catalog
            .entries()
            .iter()
            .any(|c| c.family() == CandidateFamily::PeerW)
    );
}

#[test]
fn g2_nothing_is_labelled_derivative_free() {
    let id = ppsw1::identity();
    assert!(!id.claims_derivative_free);
    let method_label: String = id.method_id.to_lowercase();
    let class_label: String = format!("{:?}", id.derivative_class).to_lowercase();
    for banned in [
        "derivative-free",
        "derivative_free",
        "derivativefree",
        "jacobian-free",
    ] {
        assert!(
            !method_label.contains(banned),
            "{method_label} contains {banned}"
        );
        assert!(
            !class_label.contains(banned),
            "{class_label} contains {banned}"
        );
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn g3_protected_sources_are_byte_identical() {
    // sha256 of the worktree files at the start of PW04/PPSW1-6b (computed with sha256sum):
    // the ptsw2b h2 list plus ptsw2b.rs and ptsw2b_contracts.rs themselves.
    let pinned: [(&str, &str); 14] = [
        (
            "src/sequential.rs",
            "9256587e04625eecaf21b053f4b94f907b8d8e2f7afdb2b8854f32e240bd5a0d",
        ),
        (
            "src/rodas5p_matrix_free_fast.rs",
            "8425652310c29d8b3f8e649ea477040fa48c35bb391dcaf6f3073b83e1aae185",
        ),
        (
            "src/collective_rodas5p.rs",
            "9e84ec5cd53496c2951a91c418b76069384fbfb48d0e3bc5a94fcb5d0aadd7b3",
        ),
        (
            "src/candidates.rs",
            "d54dea3159cdf8a34d61e920e838152d1ec2074b29fcb5835f829841fbb6562b",
        ),
        (
            "src/problem.rs",
            "f8da89e24e532c208b84711b5bbaa4c0c3a213ff5cdc90c8b9d7c53e6583365d",
        ),
        (
            "src/parallel.rs",
            "b1216fcb5a5352a0f51422d089caa9343cda1d13fefbf94146b1b171b8431d92",
        ),
        (
            "src/adaptive.rs",
            "3a04609dc5a7141d6bdfe6ebccec441d1d2c8a9960305ea58873914e10703c40",
        ),
        (
            "src/integrate.rs",
            "8016764fa7e0f8c4dd8322b07ab49bb6dba5dc4f9f632a2d99ac6a09fd328253",
        ),
        (
            "tests/unified_candidate_contracts.rs",
            "4229c08c483ce777c61772e8af0319c2e56d7b1c7d48db91720ac3c001f0d277",
        ),
        (
            "../rodas5p-core/src/coefficients.rs",
            "fe6417d0e696a836e91d2058dc015d0bcb4e0b6af3f58f648694be1de0d90f73",
        ),
        (
            "../rodas5p-core/src/norms.rs",
            "d9855786d8ab94e1def28e3525fa40e5a44914a3ecf5f1d318e41421f8cfd604",
        ),
        (
            "../rodas5p-core/src/work.rs",
            "8e45d23ac8d07bf2d7fa857a94e57453807c6832711363b627c8376ea0f899f0",
        ),
        (
            "src/ptsw2b.rs",
            "49fcccf7ac9670717640a7479aa96fa1a072b5c79cf92c981f852319c9578202",
        ),
        (
            "tests/ptsw2b_contracts.rs",
            "89ca2fae065d88addf5dab0623e1d96cfed2ccdb727f291a847b58c3e0bf1d1d",
        ),
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (rel, want) in pinned {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(sha256_hex(&bytes), want, "protected file changed: {rel}");
    }
    // lib.rs may gain exactly ONE line `pub mod ppsw1;` and nothing else (owner allowlist):
    // with that single line removed, its bytes must equal the pre-PW04 file.
    let lib = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let marker = "pub mod ppsw1;\n";
    assert!(
        lib.matches(marker).count() <= 1,
        "at most one `pub mod ppsw1;` line"
    );
    assert_eq!(
        sha256_hex(lib.replace(marker, "").as_bytes()),
        "56822aa7a940bd5c2c79bddaa28c0e133c3f22be65e17e4ca378c1bb5a2e31d9",
        "src/lib.rs may differ from the pre-PW04 file only by the one `pub mod ppsw1;` line"
    );
}

// ---------------------------------------------------------------------------------------
// UNRESOLVED: recorded, deliberately not pinned
// ---------------------------------------------------------------------------------------

#[test]
#[ignore = "UNRESOLVED (owner decision pending): starter, error estimator, strong-stiff global order, W-tolerance window 0.8-1.0, Vandermonde convention"]
fn unresolved_items_are_listed_not_tested() {
    // 1. Starter: Y_0 needs stages at t0 + h0 c_i with c_1 = -1 < t0; the paper gives none.
    // 2. Estimator: the paper describes it in prose only (no coefficients).
    // 3. Strong-stiff global order: PR(-1000) and LIN3 were NOT_CONCLUSIVE in PW04.
    // 4. W tolerance: T = c*J converged for c in {1.0, 1.2}, diverged for 0.8; 0.8-1.0 NOT_RUN.
    // 5. Vandermonde convention: printed V is inconsistent with B*1 = 1; node-row V is used.
    unimplemented!("UNRESOLVED items are not testable yet");
}
