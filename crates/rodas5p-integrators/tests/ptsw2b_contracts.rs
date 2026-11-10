//! RED contracts for PW02: the opt-in PTSW2B order-2 L-stable two-step Peer-W CONTROL.
//!
//! Source of every certified number below:
//! `docs/agent/solver_consolidation_20261108/PTSW2B_EXACT_TABLEAU_CERTIFICATE_20261109.md`
//! (called "CERT" in comments; section numbers refer to that file) and the
//! companion `verify_ptsw2b.py`.  Nothing here is taken from the RODAS5P tableau.
//!
//! These tests call an opt-in public module `rodas5p_integrators::ptsw2b` that does NOT exist
//! yet.  Until it exists this test binary fails to compile: that is the RED state.  The exact
//! arithmetic oracle (`Rat`, `Q3` = Q(sqrt 3)) lives in this file and is independent of the
//! implementation; the implementation is only compared against it.
//!
//! Owner decisions encoded: accepted-step ceiling 1.10x protected RODAS5P (not 1.20x); PW02 is
//! an opt-in CONTROL with its own identity, startup data, estimator and controller; stage
//! parallelism inside one step only; no shared-Krylov claim.

use std::sync::Arc;

use rodas5p_core::{CoreResult, DenseMatrix, WorkCounters};
use rodas5p_integrators::ptsw2b::{self, Attempt, Config, DerivativeClass, StageInput, State};
use rodas5p_integrators::{
    CandidateCatalog, CandidateFamily, CandidateStatus, OdeProblem, ParallelExecution,
};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------------------
// Exact arithmetic oracle over Q(sqrt 3): value = a + b*sqrt(3), a,b rational (i128).
// ---------------------------------------------------------------------------------------

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
            n: s * n / g,
            d: s * d / g,
        }
    }
    fn zero() -> Rat {
        Rat { n: 0, d: 1 }
    }
}
impl std::ops::Add for Rat {
    type Output = Rat;
    fn add(self, o: Rat) -> Rat {
        Rat::new(self.n * o.d + o.n * self.d, self.d * o.d)
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
        Rat::new(self.n * o.n, self.d * o.d)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Q3 {
    a: Rat,
    b: Rat,
}

impl Q3 {
    fn is_zero(&self) -> bool {
        self.a == Rat::zero() && self.b == Rat::zero()
    }
    fn to_f64(self) -> f64 {
        self.a.n as f64 / self.a.d as f64 + (self.b.n as f64 / self.b.d as f64) * 3.0_f64.sqrt()
    }
}
impl std::ops::Add for Q3 {
    type Output = Q3;
    fn add(self, o: Q3) -> Q3 {
        Q3 {
            a: self.a + o.a,
            b: self.b + o.b,
        }
    }
}
impl std::ops::Neg for Q3 {
    type Output = Q3;
    fn neg(self) -> Q3 {
        Q3 {
            a: -self.a,
            b: -self.b,
        }
    }
}
impl std::ops::Sub for Q3 {
    type Output = Q3;
    fn sub(self, o: Q3) -> Q3 {
        self + (-o)
    }
}
impl std::ops::Mul for Q3 {
    type Output = Q3;
    fn mul(self, o: Q3) -> Q3 {
        // (a1 + b1 r)(a2 + b2 r) = a1a2 + 3 b1b2 + (a1b2 + a2b1) r,  r = sqrt(3)
        Q3 {
            a: self.a * o.a + Rat::new(3, 1) * self.b * o.b,
            b: self.a * o.b + o.a * self.b,
        }
    }
}

fn q(n: i128, d: i128) -> Q3 {
    Q3 {
        a: Rat::new(n, d),
        b: Rat::zero(),
    }
}
fn int(n: i128) -> Q3 {
    q(n, 1)
}
fn s3() -> Q3 {
    Q3 {
        a: Rat::zero(),
        b: Rat::new(1, 1),
    }
}
fn pw(x: Q3, k: u32) -> Q3 {
    (0..k).fold(int(1), |acc, _| acc * x)
}

// ---------------------------------------------------------------------------------------
// Certified PTSW2B data, transcribed from CERT sec. 2 (eq. 2.1-2.3) and sec. 5 (eq. 5.1).
// ---------------------------------------------------------------------------------------

/// CERT sec. 2: gamma = (3 - sqrt 3)/2.
fn cert_gamma() -> Q3 {
    (int(3) - s3()) * q(1, 2)
}
/// CERT sec. 2: c1 = 3 - 2 sqrt 3, c2 = 1.
fn cert_c() -> [Q3; 2] {
    [int(3) - int(2) * s3(), int(1)]
}
/// CERT eq. (2.3): b^T = (0, (3 - sqrt 3)/2).
fn cert_b() -> [Q3; 2] {
    [int(0), cert_gamma()]
}
/// CERT eq. (2.1).
fn cert_a(s: Q3) -> [[Q3; 2]; 2] {
    let e = q(1, 8);
    [
        [
            e * int(3) * (int(5) - int(3) * s3()) * s,
            e * (int(24) - int(16) * s3() + int(3) * (int(-5) + int(3) * s3()) * s),
        ],
        [
            e * (-(int(1) + s3())) * s,
            e * (int(8) + (int(1) + s3()) * s),
        ],
    ]
}
/// CERT eq. (2.2).
fn cert_gamma_matrix(s: Q3) -> [[Q3; 2]; 2] {
    let e = q(1, 4);
    [
        [
            e * int(3) * (s3() - int(2)) * s,
            e * (int(-6) + int(2) * s3() + int(6) * s - int(3) * s3() * s),
        ],
        [e * s3() * s, e * (int(-6) + int(2) * s3() - s3() * s)],
    ]
}
/// CERT eq. (2.3): v^T(sigma) = (sqrt 3 - 1)/8 * (sigma, 4 - sigma).
fn cert_v(s: Q3) -> [Q3; 2] {
    let e = (s3() - int(1)) * q(1, 8);
    [e * s, e * (int(4) - s)]
}
/// CERT sec. 5: b_e^T = 0.95 b^T = (0, 19/20 gamma).
fn cert_b_emb() -> [Q3; 2] {
    [int(0), q(19, 20) * cert_gamma()]
}
/// CERT eq. (5.1).
fn cert_v_emb(s: Q3) -> [Q3; 2] {
    [
        (int(8) * s3() - int(11)) * s * q(1, 80),
        (int(11) - int(8) * s3()) * s * q(1, 80) + (int(19) * s3() - int(17)) * q(1, 40),
    ]
}

fn sigma_samples() -> Vec<Q3> {
    vec![q(1, 2), q(3, 4), q(1, 1), q(3, 2), q(2, 1), q(7, 3)]
}

fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + a.abs().max(b.abs()))
}

// ---------------------------------------------------------------------------------------
// (a) tableau coefficients exactly as certified
// ---------------------------------------------------------------------------------------

#[test]
fn a1_certified_data_is_internally_consistent_in_exact_arithmetic() {
    let g = cert_gamma();
    let c = cert_c();
    // CERT sec. 2 (Theorem 3.1 reduction): pi(gamma) = -gamma^2 + 3 gamma - 3/2 = 0.
    assert!((-(g * g) + int(3) * g - q(3, 2)).is_zero());
    // phi*(c, gamma) = c^2 + (2 - 4 gamma) c - 3 + 4 gamma = 0 for both nodes.
    for ci in c {
        assert!((ci * ci + (int(2) - int(4) * g) * ci - int(3) + int(4) * g).is_zero());
    }
    // Stiff accuracy: c2 = 1, b = gamma e2, v^T(sigma) = e2^T (A + Gamma).
    assert_eq!(c[1], int(1));
    assert_eq!(cert_b(), [int(0), g]);
    for s in sigma_samples() {
        let (a, gm, v) = (cert_a(s), cert_gamma_matrix(s), cert_v(s));
        for j in 0..2 {
            assert!(
                (a[1][j] + gm[1][j] - v[j]).is_zero(),
                "v != e2^T(A+Gamma) at column {j}"
            );
        }
    }
    // CERT sec. 3.3: C3 = 2 gamma^2 - 7/2 gamma + 7/6 = 23/12 - 5 sqrt3 / 4.
    let c3 = int(2) * g * g - q(7, 2) * g + q(7, 6);
    assert_eq!(c3, q(23, 12) + (q(-5, 4)) * s3());
}

#[test]
fn a2_implementation_coefficients_equal_certified_values() {
    let tol = 1e-14;
    assert!(approx(ptsw2b::gamma(), cert_gamma().to_f64(), tol));
    for i in 0..2 {
        assert!(approx(ptsw2b::nodes()[i], cert_c()[i].to_f64(), tol));
        assert!(approx(ptsw2b::b()[i], cert_b()[i].to_f64(), tol));
        assert!(approx(
            ptsw2b::b_embedded()[i],
            cert_b_emb()[i].to_f64(),
            tol
        ));
    }
    for s in sigma_samples() {
        let sf = s.to_f64();
        for i in 0..2 {
            assert!(
                approx(ptsw2b::v(sf)[i], cert_v(s)[i].to_f64(), tol),
                "v sigma={sf}"
            );
            assert!(
                approx(ptsw2b::v_embedded(sf)[i], cert_v_emb(s)[i].to_f64(), tol),
                "v_e sigma={sf}"
            );
            for j in 0..2 {
                assert!(
                    approx(ptsw2b::a_matrix(sf)[i][j], cert_a(s)[i][j].to_f64(), tol),
                    "A sigma={sf}"
                );
                assert!(
                    approx(
                        ptsw2b::gamma_matrix(sf)[i][j],
                        cert_gamma_matrix(s)[i][j].to_f64(),
                        tol
                    ),
                    "Gamma sigma={sf}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// (b) exact order-2 conditions hold; order-3 conditions fail
// ---------------------------------------------------------------------------------------

/// CERT sec. 3.1.  C(l): sum_j a_ij (c_j-1)^(l-1) - sigma^(l-1) c_i^l / l.
fn exact_c_res(s: Q3, l: u32) -> [Q3; 2] {
    let (a, c) = (cert_a(s), cert_c());
    let mut r = [int(0); 2];
    for i in 0..2 {
        let mut acc = int(0);
        for j in 0..2 {
            acc = acc + a[i][j] * pw(c[j] - int(1), l - 1);
        }
        r[i] = acc - pw(s, l - 1) * pw(c[i], l) * q(1, l as i128);
    }
    r
}
/// Gamma(l): sum_j Gamma_ij (c_j-1)^(l-1) + gamma sigma^(l-1) c_i^(l-1).
fn exact_g_res(s: Q3, l: u32) -> [Q3; 2] {
    let (gm, c, g) = (cert_gamma_matrix(s), cert_c(), cert_gamma());
    let mut r = [int(0); 2];
    for i in 0..2 {
        let mut acc = int(0);
        for j in 0..2 {
            acc = acc + gm[i][j] * pw(c[j] - int(1), l - 1);
        }
        r[i] = acc + g * pw(s, l - 1) * pw(c[i], l - 1);
    }
    r
}
/// B(p): sum_j b_j sigma^(l-1) c_j^(l-1) + sum_j v_j (c_j-1)^(l-1) - sigma^(l-1)/l.
fn exact_b_res(s: Q3, l: u32) -> Q3 {
    let (b, v, c) = (cert_b(), cert_v(s), cert_c());
    let mut acc = int(0);
    for j in 0..2 {
        acc = acc + b[j] * pw(s, l - 1) * pw(c[j], l - 1) + v[j] * pw(c[j] - int(1), l - 1);
    }
    acc - pw(s, l - 1) * q(1, l as i128)
}

#[test]
fn b1_exact_order2_conditions_vanish_identically_in_sigma() {
    // Every residual is a polynomial of degree <= 1 in sigma, so vanishing at 6 distinct
    // rational points is identical vanishing (CERT sec. 3.1; verify_ptsw2b.py asserts the same).
    for s in sigma_samples() {
        for l in 1..=2u32 {
            for r in exact_c_res(s, l) {
                assert!(r.is_zero(), "C({l}) sigma={s:?}");
            }
            for r in exact_g_res(s, l) {
                assert!(r.is_zero(), "Gamma({l}) sigma={s:?}");
            }
            assert!(exact_b_res(s, l).is_zero(), "B({l}) sigma={s:?}");
        }
    }
}

#[test]
fn b2_exact_order3_condition_fails_so_order_is_exactly_two() {
    for s in sigma_samples() {
        // B(3) residual = sigma (-3 sqrt3 sigma + 7 sigma - 30 + 18 sqrt3) / 6  (symbolic, SymPy).
        let expected = s * ((int(7) - int(3) * s3()) * s - int(30) + int(18) * s3()) * q(1, 6);
        let r = exact_b_res(s, 3);
        assert_eq!(r, expected, "B(3) closed form at sigma={s:?}");
        assert!(
            !r.is_zero(),
            "B(3) must NOT vanish: PTSW2B is order 2, not 3"
        );
    }
    // Stage order is exactly 2: C(3) and Gamma(3) do not vanish either.
    let one = q(1, 1);
    assert!(exact_c_res(one, 3).iter().any(|r| !r.is_zero()));
    assert!(exact_g_res(one, 3).iter().any(|r| !r.is_zero()));
}

#[test]
fn b3_implementation_residuals_vanish_for_order2_and_not_for_order3() {
    let (g, c, b) = (ptsw2b::gamma(), ptsw2b::nodes(), ptsw2b::b());
    for sf in [0.5_f64, 0.75, 1.0, 1.5, 2.0] {
        let (a, gm, v) = (
            ptsw2b::a_matrix(sf),
            ptsw2b::gamma_matrix(sf),
            ptsw2b::v(sf),
        );
        for l in 1..=2i32 {
            let p = l - 1;
            for i in 0..2 {
                let ca: f64 = (0..2).map(|j| a[i][j] * (c[j] - 1.0).powi(p)).sum();
                assert!(
                    (ca - sf.powi(p) * c[i].powi(l) / f64::from(l)).abs() < 1e-14,
                    "C({l}) i={i}"
                );
                let ga: f64 = (0..2).map(|j| gm[i][j] * (c[j] - 1.0).powi(p)).sum();
                assert!(
                    (ga + g * sf.powi(p) * c[i].powi(p)).abs() < 1e-14,
                    "Gamma({l}) i={i}"
                );
            }
            let bb: f64 = (0..2)
                .map(|j| b[j] * sf.powi(p) * c[j].powi(p) + v[j] * (c[j] - 1.0).powi(p))
                .sum();
            assert!((bb - sf.powi(p) / f64::from(l)).abs() < 1e-14, "B({l})");
        }
        let b3: f64 = (0..2)
            .map(|j| b[j] * sf.powi(2) * c[j].powi(2) + v[j] * (c[j] - 1.0).powi(2))
            .sum::<f64>()
            - sf.powi(2) / 3.0;
        assert!(
            b3.abs() > 1e-2,
            "implementation must not satisfy B(3) at sigma={sf}: {b3}"
        );
    }
}

// ---------------------------------------------------------------------------------------
// (c) nilpotent stiff-limit matrix and rho = 0
// ---------------------------------------------------------------------------------------

type M3 = [[Q3; 3]; 3];

fn mat_mul(x: &M3, y: &M3) -> M3 {
    let mut r = [[int(0); 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                r[i][j] = r[i][j] + x[i][k] * y[k][j];
            }
        }
    }
    r
}

/// CERT sec. 3.2: M(z) = [[w beta, w 1],[b^T w beta + v^T, 1 + b^T w 1]], w -> -1/gamma, sigma = 1.
fn exact_m_inf() -> M3 {
    let s = int(1);
    let (a, gm, b, v) = (cert_a(s), cert_gamma_matrix(s), cert_b(), cert_v(s));
    let w = -(int(1)) * inv_gamma();
    let mut beta = [[int(0); 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            beta[i][j] = a[i][j] + gm[i][j];
        }
    }
    let mut m = [[int(0); 3]; 3];
    for i in 0..2 {
        for j in 0..2 {
            m[i][j] = w * beta[i][j];
        }
        m[i][2] = w;
    }
    for j in 0..2 {
        m[2][j] = b[0] * w * beta[0][j] + b[1] * w * beta[1][j] + v[j];
    }
    m[2][2] = int(1) + (b[0] + b[1]) * w;
    m
}
/// 1/gamma = 2/(3 - sqrt3) = (3 + sqrt3)/3 (rationalised).
fn inv_gamma() -> Q3 {
    (int(3) + s3()) * q(1, 3)
}

#[test]
fn c1_exact_stiff_limit_matrix_is_nilpotent_of_index_three_with_rho_zero() {
    assert!((inv_gamma() * cert_gamma() - int(1)).is_zero());
    let m = exact_m_inf();
    // CERT eq. (3.1).
    let r3 = s3();
    let third = q(1, 3);
    let expected: M3 = [
        [r3 * q(1, 4), q(3, 4) * r3, -(int(1)) - r3 * third],
        [-(r3 * q(1, 12)), -(r3 * q(1, 4)), -(int(1)) - r3 * third],
        [int(0), int(0), int(0)],
    ];
    assert_eq!(m, expected, "M(infinity) != CERT (3.1)");
    // CERT sec. 3.2: M^2 = [[0,0,-1-sqrt3],[0,0,(1+sqrt3)/3],[0,0,0]] != 0, M^3 = 0.
    let m2 = mat_mul(&m, &m);
    let expected_m2: M3 = [
        [int(0), int(0), -(int(1) + r3)],
        [int(0), int(0), (int(1) + r3) * third],
        [int(0), int(0), int(0)],
    ];
    assert_eq!(m2, expected_m2);
    assert!(
        m2.iter().flatten().any(|e| !e.is_zero()),
        "index must be exactly 3, M^2 != 0"
    );
    let m3 = mat_mul(&m2, &m);
    assert!(
        m3.iter().flatten().all(|e| e.is_zero()),
        "M^3 must be exactly 0 (rho = 0)"
    );
    // Characteristic polynomial lambda^3: trace = 0, sum of principal 2x2 minors = 0, det = 0.
    let trace = m[0][0] + m[1][1] + m[2][2];
    let minor = |i: usize, j: usize| m[i][i] * m[j][j] - m[i][j] * m[j][i];
    assert!(trace.is_zero());
    assert!((minor(0, 1) + minor(0, 2) + minor(1, 2)).is_zero());
}

#[test]
fn c2_implementation_stiff_limit_matrix_matches_exact_and_is_nilpotent() {
    let exact = exact_m_inf();
    let got = ptsw2b::stiff_limit_matrix();
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                approx(got[i][j], exact[i][j].to_f64(), 1e-13),
                "M_inf[{i}][{j}]"
            );
        }
    }
    let mut cube = [[0.0_f64; 3]; 3];
    let mut sq = [[0.0_f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                sq[i][j] += got[i][k] * got[k][j];
            }
        }
    }
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                cube[i][j] += sq[i][k] * got[k][j];
            }
        }
    }
    assert!(
        cube.iter().flatten().all(|e| e.abs() < 1e-13),
        "M^3 = 0 up to roundoff"
    );
    assert!(sq.iter().flatten().any(|e| e.abs() > 0.5), "M^2 != 0");
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

fn problem2(with_jacobian: bool) -> OdeProblem {
    let jacobian: Option<Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>> =
        if with_jacobian {
            Some(Arc::new(|t: f64, y: &[f64]| {
                let j = jac2(t, y);
                DenseMatrix::from_vec_rows(vec![j[0].to_vec(), j[1].to_vec()])
            }))
        } else {
            None
        };
    let jvp: Option<Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>> =
        if with_jacobian {
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
        "ptsw2b-contract-nonlinear-2d",
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

fn scalar_problem(lambda: f64, forcing: bool) -> OdeProblem {
    // forcing = false: y' = lambda y.   forcing = true: y' = lambda (y - cos t) - sin t.
    OdeProblem::new(
        "ptsw2b-contract-scalar",
        1,
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = if forcing {
                lambda * (y[0] - t.cos()) - t.sin()
            } else {
                lambda * y[0]
            };
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

fn demo_state(h_prev: f64) -> State {
    let u = vec![1.0, 1.0];
    let c = ptsw2b::nodes();
    State {
        t: 0.0,
        u: u.clone(),
        k_prev: [
            rhs2((c[0] - 1.0) * h_prev, &u).to_vec(),
            rhs2((c[1] - 1.0) * h_prev, &u).to_vec(),
        ],
        h_prev,
    }
}

fn exact_f64_a(sigma: f64) -> [[f64; 2]; 2] {
    // Certified A at an arbitrary f64 sigma, evaluated through the exact-form closed expression.
    let r3 = 3.0_f64.sqrt();
    [
        [
            3.0 * (5.0 - 3.0 * r3) * sigma / 8.0,
            (24.0 - 16.0 * r3 + 3.0 * (-5.0 + 3.0 * r3) * sigma) / 8.0,
        ],
        [-(1.0 + r3) * sigma / 8.0, (8.0 + (1.0 + r3) * sigma) / 8.0],
    ]
}
fn exact_f64_gamma_matrix(sigma: f64) -> [[f64; 2]; 2] {
    let r3 = 3.0_f64.sqrt();
    [
        [
            3.0 * (r3 - 2.0) * sigma / 4.0,
            (-6.0 + 2.0 * r3 + 6.0 * sigma - 3.0 * r3 * sigma) / 4.0,
        ],
        [r3 * sigma / 4.0, (-6.0 + 2.0 * r3 - r3 * sigma) / 4.0],
    ]
}

// ---------------------------------------------------------------------------------------
// (d) same-step stage independence, common W factor
// ---------------------------------------------------------------------------------------

#[test]
fn d1_stage_inputs_use_only_previous_step_data_and_certified_formulas() {
    let (h, h_prev) = (0.02, 0.016);
    let sigma = h / h_prev;
    let st = demo_state(h_prev);
    let inputs = ptsw2b::stage_inputs(st.t, &st.u, &st.k_prev, h, sigma);
    let (a, gm, g, c) = (
        exact_f64_a(sigma),
        exact_f64_gamma_matrix(sigma),
        cert_gamma().to_f64(),
        cert_c(),
    );
    for i in 0..2 {
        assert!(approx(inputs[i].t, st.t + c[i].to_f64() * h, 1e-14));
        for comp in 0..2 {
            let y = st.u[comp] + h * (0..2).map(|j| a[i][j] * st.k_prev[j][comp]).sum::<f64>();
            // CERT eq. (2.5): p_i = sum_j Gamma_ij(sigma)/gamma * k_{m-1,j}.
            let p = (0..2)
                .map(|j| gm[i][j] / g * st.k_prev[j][comp])
                .sum::<f64>();
            assert!(approx(inputs[i].y[comp], y, 1e-13), "Y[{i}][{comp}]");
            assert!(approx(inputs[i].p[comp], p, 1e-13), "p[{i}][{comp}]");
        }
    }
}

fn stage_setup() -> (OdeProblem, State, f64, [StageInput; 2]) {
    let (h, h_prev) = (0.02, 0.02);
    let st = demo_state(h_prev);
    let inputs = ptsw2b::stage_inputs(st.t, &st.u, &st.k_prev, h, h / h_prev);
    (problem2(true), st, h, inputs)
}

#[test]
fn d2_perturbing_one_current_stage_input_changes_no_other_current_stage_output() {
    let (problem, st, h, inputs) = stage_setup();
    let exec = ParallelExecution::sequential();
    let mut w0 = WorkCounters::default();
    let base = ptsw2b::solve_stages(&problem, st.t, &st.u, h, &inputs, &exec, &mut w0).unwrap();
    for victim in 0..2usize {
        let other = 1 - victim;
        for field in 0..2 {
            let mut perturbed = inputs.clone();
            if field == 0 {
                perturbed[victim].y[0] += 1e-3;
            } else {
                perturbed[victim].p[1] -= 2e-3;
            }
            let mut w = WorkCounters::default();
            let out =
                ptsw2b::solve_stages(&problem, st.t, &st.u, h, &perturbed, &exec, &mut w).unwrap();
            let same = |a: &Vec<f64>, b: &Vec<f64>| {
                a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
            };
            assert!(
                same(&out.k[other], &base.k[other]),
                "k of stage {other} moved when stage {victim} input changed"
            );
            assert!(
                same(&out.x[other], &base.x[other]),
                "x of stage {other} moved"
            );
            assert!(
                !same(&out.k[victim], &base.k[victim]),
                "perturbation must reach its own stage"
            );
        }
    }
}

#[test]
fn d3_both_stages_share_one_w_factor_and_satisfy_the_certified_stage_equations() {
    let (problem, st, h, inputs) = stage_setup();
    let exec = ParallelExecution::sequential();
    let mut w = WorkCounters::default();
    let out = ptsw2b::solve_stages(&problem, st.t, &st.u, h, &inputs, &exec, &mut w).unwrap();
    // One W per step: one Jacobian provider call, one factorization, two solves, two RHS calls.
    assert_eq!(w.jacobian_builds, 1);
    assert_eq!(
        w.direct_factorizations, 1,
        "common W must be factored once for both stages"
    );
    assert_eq!(w.direct_solve_calls, 2);
    assert_eq!(w.linear_solves, 2);
    assert_eq!(w.rhs_calls, 2);
    assert_eq!(
        (w.jvp_calls, w.nonlinear_solves, w.nonlinear_iterations),
        (0, 0, 0)
    );
    // Independent algebraic residuals with T_m = J(t_m, u_m) frozen at the step start.
    let t_m = jac2(st.t, &st.u);
    let g = cert_gamma().to_f64();
    let gm = exact_f64_gamma_matrix(h / st.h_prev);
    for i in 0..2 {
        let f = rhs2(inputs[i].t, &inputs[i].y);
        for r in 0..2 {
            // CERT (2.5): W x = f + p with W = I - h gamma T_m.
            let wx = out.x[i][r] - h * g * (0..2).map(|c| t_m[r][c] * out.x[i][c]).sum::<f64>();
            assert!(
                approx(wx, f[r] + inputs[i].p[r], 1e-12),
                "W x residual stage {i} row {r}"
            );
            // k = x - p.
            assert!(approx(out.k[i][r], out.x[i][r] - inputs[i].p[r], 1e-13));
            // CERT (2.4): W k = f + h T_m sum_j Gamma_ij k_{m-1,j}.
            let wk = out.k[i][r] - h * g * (0..2).map(|c| t_m[r][c] * out.k[i][c]).sum::<f64>();
            let tg: f64 = (0..2)
                .map(|c| t_m[r][c] * (0..2).map(|j| gm[i][j] * st.k_prev[j][c]).sum::<f64>())
                .sum();
            assert!(
                approx(wk, f[r] + h * tg, 1e-12),
                "original-form residual stage {i} row {r}"
            );
        }
    }
}

#[test]
fn d4_serial_and_two_thread_stage_solves_are_bitwise_identical() {
    let (problem, st, h, inputs) = stage_setup();
    let mut w_s = WorkCounters::default();
    let mut w_p = WorkCounters::default();
    let s = ptsw2b::solve_stages(
        &problem,
        st.t,
        &st.u,
        h,
        &inputs,
        &ParallelExecution::sequential(),
        &mut w_s,
    )
    .unwrap();
    let exec = ParallelExecution::rayon(2).unwrap();
    let p = ptsw2b::solve_stages(&problem, st.t, &st.u, h, &inputs, &exec, &mut w_p).unwrap();
    for i in 0..2 {
        for r in 0..2 {
            assert_eq!(s.k[i][r].to_bits(), p.k[i][r].to_bits());
            assert_eq!(s.x[i][r].to_bits(), p.x[i][r].to_bits());
        }
    }
    assert_eq!(
        w_s, w_p,
        "work ledger must not depend on the execution backend"
    );
}

#[test]
fn d5_jvp_only_problem_reproduces_explicit_jacobian_stage_solution() {
    // Derivative class (proposal): T_m = J(t_m, u_m) from either an analytic Jacobian or an exact
    // JVP callback, both already supported by OdeProblem; no new capability.
    let (problem, st, h, inputs) = stage_setup();
    let jvp_problem = problem2(false);
    let exec = ParallelExecution::sequential();
    let (mut wa, mut wb) = (WorkCounters::default(), WorkCounters::default());
    let a = ptsw2b::solve_stages(&problem, st.t, &st.u, h, &inputs, &exec, &mut wa).unwrap();
    let b = ptsw2b::solve_stages(&jvp_problem, st.t, &st.u, h, &inputs, &exec, &mut wb).unwrap();
    for i in 0..2 {
        for r in 0..2 {
            assert!(approx(a.k[i][r], b.k[i][r], 1e-12));
        }
    }
    assert_eq!(
        wb.jvp_calls, 2,
        "dense T_m from a JVP costs n JVP calls and must be charged"
    );
    assert_eq!(wb.jacobian_builds, 1);
    assert_eq!(wa.jvp_calls, 0);
}

// ---------------------------------------------------------------------------------------
// (e) own estimator and controller, never RODAS
// ---------------------------------------------------------------------------------------

#[test]
fn e1_embedded_estimator_uses_its_own_exact_weights_and_is_order_one() {
    // CERT sec. 5 / eq. (5.1), exact: sum b_e + v_e = 1 (order 1) and B_e(2) residual = sigma/20 != 0.
    for s in sigma_samples() {
        let (be, ve, c) = (cert_b_emb(), cert_v_emb(s), cert_c());
        assert_eq!(be[0] + be[1] + ve[0] + ve[1], int(1), "embedded B_e(1)");
        let b2 =
            be[0] * s * c[0] + be[1] * s * c[1] + ve[0] * (c[0] - int(1)) + ve[1] * (c[1] - int(1))
                - s * q(1, 2);
        assert_eq!(
            b2,
            s * q(1, 20),
            "embedded B_e(2) residual is sigma/20, so order is exactly 1"
        );
    }
    // Estimator vector u_{m+1} - u~_{m+1} = h sum_j [(b_j - be_j) k_j + (v_j - ve_j) k'_j].
    let (h, sigma) = (0.1, 1.5);
    let k = [vec![0.3, -1.2], vec![2.0, 0.7]];
    let kp = [vec![-0.5, 0.25], vec![1.5, -2.0]];
    let got = ptsw2b::embedded_error_vector(h, sigma, &k, &kp);
    let s = q(3, 2);
    let (b, be, v, ve) = (cert_b(), cert_b_emb(), cert_v(s), cert_v_emb(s));
    for comp in 0..2 {
        let want: f64 = (0..2)
            .map(|j| (b[j] - be[j]).to_f64() * k[j][comp] + (v[j] - ve[j]).to_f64() * kp[j][comp])
            .sum::<f64>()
            * h;
        assert!(approx(got[comp], want, 1e-13), "estimator component {comp}");
    }
    // Not the RODAS5P estimator: different stage count and no shared weights/controller flags.
    let rodas = rodas5p_core::load_rodas5p_coefficients().unwrap();
    assert_eq!(rodas.btilde.len(), 8);
    assert_ne!(rodas.btilde.len(), ptsw2b::STAGES);
    let id = ptsw2b::identity();
    assert!(!id.reuses_rodas5p_estimator);
    assert!(!id.reuses_rodas5p_controller);
    assert!(!id.estimator_id.to_lowercase().contains("rodas"));
    assert!(!id.controller_id.to_lowercase().contains("rodas"));
    assert_ne!(id.estimator_id, id.controller_id);
}

#[test]
fn e2_error_norm_is_componentwise_scaled_rms_of_the_embedded_difference() {
    // CERT sec. 5: Err = sqrt( mean( (e_i / (ATOL + RTOL |u_i|))^2 ) ).
    let u = [1.0, -2.0];
    let e = [1.0e-6, 4.0e-6];
    let (atol, rtol) = (1.0e-6, 1.0e-6);
    let want = (((1.0e-6_f64 / 2.0e-6).powi(2) + (4.0e-6_f64 / 3.0e-6).powi(2)) / 2.0).sqrt();
    assert!(approx(ptsw2b::error_norm(&u, &e, atol, rtol), want, 1e-12));
}

#[test]
fn e3_controller_uses_certified_constants_clip_and_hysteresis() {
    // CERT sec. 5: sigma* = min{2, max{0.5, 0.85 Err^(-1/2)}}; keep h when sigma* > 1.05 and any of
    // the last s = 2 applied ratios > 1; keep h when sigma* in [0.95, 1.05]; accept iff Err <= 1.
    assert_eq!(ptsw2b::SAFETY, 0.85);
    assert_eq!(ptsw2b::RATIO_CLIP, (0.5, 2.0));
    assert_eq!(ptsw2b::HYSTERESIS_BAND, (0.95, 1.05));
    assert_eq!(ptsw2b::HYSTERESIS_HISTORY, 2);
    let p = ptsw2b::propose_ratio;
    assert!(approx(p(0.25, &[1.0, 1.0]), 1.7, 1e-12));
    assert!(
        approx(p(0.25, &[1.3, 0.9, 1.0]), 1.7, 1e-12),
        "only the last two ratios count"
    );
    assert_eq!(p(0.25, &[1.0, 1.3]), 1.0);
    assert_eq!(p(0.25, &[1.3, 1.0]), 1.0);
    assert_eq!(p(1.0e-6, &[]), 2.0);
    assert_eq!(p(0.0, &[]), 2.0);
    assert_eq!(p(100.0, &[]), 0.5);
    assert_eq!(p(1.0e6, &[1.5, 1.5]), 0.5);
    assert_eq!(
        p(0.7, &[]),
        1.0,
        "0.85/sqrt(0.7) = 1.0159 lies in the keep band"
    );
    assert!(approx(p(0.9, &[]), 0.85 / 0.9_f64.sqrt(), 1e-12));
    assert!(approx(p(1.0, &[]), 0.85, 1e-12));
    assert!(ptsw2b::accepts(1.0));
    assert!(ptsw2b::accepts(0.3));
    assert!(!ptsw2b::accepts(1.0 + 1e-12));
    assert!(!ptsw2b::accepts(f64::NAN));
}

// ---------------------------------------------------------------------------------------
// (f) rejection / acceptance transactions
// ---------------------------------------------------------------------------------------

#[test]
fn f1_rejected_attempt_does_not_mutate_history_or_controller_but_charges_work() {
    let problem = problem2(true);
    let h = 5.0;
    let st = demo_state(h); // sigma = 1 so the attempt stays inside any admissible ratio domain
    let cfg = Config {
        atol: 1e-12,
        rtol: 1e-12,
    };
    let mut session =
        ptsw2b::Session::new(&problem, st.clone(), cfg, ParallelExecution::sequential()).unwrap();
    let state_before = session.state().clone();
    let ctrl_before = session.controller().clone();
    let w_before = session.counters();
    let outcome = session.attempt(h).unwrap();
    let (err, next_ratio) = match outcome {
        Attempt::Rejected {
            error_norm,
            next_ratio,
        } => (error_norm, next_ratio),
        Attempt::Accepted { .. } => panic!("h = 5 with tol 1e-12 must be rejected"),
    };
    assert!(err > 1.0);
    assert!(
        (0.5..=0.85).contains(&next_ratio),
        "rejected proposal must shrink: {next_ratio}"
    );
    // Committed history: bit-identical (State derives PartialEq over f64 vectors).
    assert_eq!(*session.state(), state_before);
    assert_eq!(*session.state(), st);
    assert_eq!(*session.controller(), ctrl_before);
    // Work: accepted-step counter untouched, rejected counter +1, all failed work charged.
    let w = session.counters();
    assert_eq!(w.accepted_steps, w_before.accepted_steps);
    assert_eq!(w.rejected_steps, w_before.rejected_steps + 1);
    assert_eq!(w.rhs_calls, w_before.rhs_calls + 2);
    assert_eq!(w.direct_factorizations, w_before.direct_factorizations + 1);
    assert_eq!(w.linear_solves, w_before.linear_solves + 2);
    assert_eq!(w.jacobian_builds, w_before.jacobian_builds + 1);
}

#[test]
fn f2_accepted_attempt_commits_exactly_the_certified_update() {
    let problem = problem2(true);
    let h = 1.0e-3;
    let st = demo_state(h);
    let cfg = Config {
        atol: 1e-3,
        rtol: 1e-3,
    };
    let mut session =
        ptsw2b::Session::new(&problem, st.clone(), cfg, ParallelExecution::sequential()).unwrap();
    let outcome = session.attempt(h).unwrap();
    assert!(
        matches!(outcome, Attempt::Accepted { .. }),
        "tiny step with loose tolerance must be accepted"
    );
    // Independent recomputation from the certified weights.
    let exec = ParallelExecution::sequential();
    let mut w = WorkCounters::default();
    let inputs = ptsw2b::stage_inputs(st.t, &st.u, &st.k_prev, h, 1.0);
    let stages = ptsw2b::solve_stages(&problem, st.t, &st.u, h, &inputs, &exec, &mut w).unwrap();
    let (b, v) = (cert_b(), cert_v(q(1, 1)));
    let next = session.state();
    assert!(approx(next.t, st.t + h, 1e-15));
    assert_eq!(next.h_prev, h);
    for comp in 0..2 {
        let want = st.u[comp]
            + h * (0..2)
                .map(|j| b[j].to_f64() * stages.k[j][comp] + v[j].to_f64() * st.k_prev[j][comp])
                .sum::<f64>();
        assert!(approx(next.u[comp], want, 1e-13), "u_next[{comp}]");
        for j in 0..2 {
            assert!(
                approx(next.k_prev[j][comp], stages.k[j][comp], 1e-13),
                "history k[{j}][{comp}]"
            );
        }
    }
    let c = session.counters();
    assert_eq!((c.accepted_steps, c.rejected_steps), (1, 0));
    assert_eq!(session.controller().last_ratios, vec![1.0]);
}

// ---------------------------------------------------------------------------------------
// (g) accepted-step ceiling constant; method identity
// ---------------------------------------------------------------------------------------

#[test]
fn g1_accepted_step_ceiling_is_the_owner_decided_1_10() {
    assert_eq!(ptsw2b::ACCEPTED_STEP_CEILING_VS_RODAS5P, 1.10);
    assert_ne!(ptsw2b::ACCEPTED_STEP_CEILING_VS_RODAS5P, 1.20);
    assert_eq!(ptsw2b::identity().accepted_step_ceiling_vs_rodas5p, 1.10);
}

#[test]
fn g2_method_identity_is_distinct_opt_in_control_with_the_declared_derivative_class() {
    let id = ptsw2b::identity();
    assert!(id.method_id.contains("ptsw2b"));
    assert!(!id.method_id.contains("ptsw2c"));
    assert_eq!((id.order, id.stages), (2, 2));
    assert_eq!((ptsw2b::FORMAL_ORDER, ptsw2b::STAGES), (2, 2));
    assert!(
        !id.claims_shared_krylov_recurrence,
        "common W does not imply a shared Krylov recurrence"
    );
    assert!(
        !id.claims_g0_qualified,
        "order-2 control is not a G0 candidate"
    );
    assert!(!id.claims_derivative_free);
    // Owner decision pending: PROPOSED single class (see PW02_SCAN_AND_RED_TEST_KO.md sec. 3).
    assert_eq!(
        id.derivative_class,
        DerivativeClass::ExactJacobianOrJvpFrozenAtStepStart
    );
}

// ---------------------------------------------------------------------------------------
// (h) opt-in only: default routing / protected sources unchanged
// ---------------------------------------------------------------------------------------

#[test]
fn h1_default_candidate_catalog_is_unchanged_and_peerw_stays_deferred() {
    assert!(!ptsw2b::identity().default_routed);
    let catalog = CandidateCatalog::research_default().unwrap();
    for entry in catalog.entries() {
        let id = entry.id().to_lowercase();
        assert!(
            !id.contains("ptsw")
                && !id.contains("pw02")
                && !id.contains(ptsw2b::identity().method_id),
            "{id}"
        );
        if entry.family() == CandidateFamily::PeerW {
            assert!(
                matches!(entry.status(), CandidateStatus::Deferred { .. }),
                "PeerW stub must stay Deferred; PW02 is exposed only through rodas5p_integrators::ptsw2b"
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
fn h2_protected_sources_are_byte_identical() {
    // sha256 of the worktree files at the start of T02/PW02 (computed with sha256sum).
    // lib.rs is intentionally absent: it may gain exactly one `pub mod ptsw2b;` line (owner decision).
    let pinned: [(&str, &str); 12] = [
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
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (rel, want) in pinned {
        let bytes = std::fs::read(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let got: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(got, want, "protected file changed: {rel}");
    }
}

// ---------------------------------------------------------------------------------------
// startup, convergence, stiff behaviour
// ---------------------------------------------------------------------------------------

#[test]
fn i1_startup_plan_is_forward_only_with_certified_times() {
    // CERT sec. 4: T = {2 - 2 sqrt3, 0}, t1 = t0 + (2 sqrt3 - 2) h0, k_{0,1} = f(t0, u0),
    // k_{0,2} = f(t1, u1), first ratio 1.
    let (t0, h0) = (0.25, 0.01);
    let plan = ptsw2b::startup_plan(t0, h0);
    let dt = 2.0 * 3.0_f64.sqrt() - 2.0;
    assert!(approx(plan.t1, t0 + dt * h0, 1e-15));
    assert!(approx(plan.derivative_times[0], t0, 1e-15));
    assert!(approx(plan.derivative_times[1], plan.t1, 1e-15));
    assert!(
        plan.derivative_times.iter().all(|&t| t >= t0),
        "no backward integration"
    );
    assert_eq!(plan.first_ratio, 1.0);
}

/// Exact startup data for a manufactured problem: peer derivatives at t_m + (c_i - 1) h.
fn exact_start(f: impl Fn(f64, f64) -> f64, y: impl Fn(f64) -> f64, h: f64) -> State {
    let c = ptsw2b::nodes();
    State {
        t: 0.0,
        u: vec![y(0.0)],
        k_prev: [0, 1].map(|i| {
            let t = (c[i] - 1.0) * h;
            vec![f(t, y(t))]
        }),
        h_prev: h,
    }
}

fn march(problem: &OdeProblem, mut st: State, h: f64, n: usize) -> State {
    let exec = ParallelExecution::sequential();
    let mut w = WorkCounters::default();
    for _ in 0..n {
        st = ptsw2b::advance(problem, &st, h, &exec, &mut w)
            .unwrap()
            .next;
    }
    st
}

#[test]
fn j1_observed_global_order_is_two_with_exact_startup() {
    let problem = scalar_problem(-1.0, false);
    let mut errors = Vec::new();
    for n in [20usize, 40, 80] {
        let h = 1.0 / n as f64;
        let st = exact_start(|_, y| -y, |t| (-t).exp(), h);
        let end = march(&problem, st, h, n);
        errors.push((end.u[0] - (-end.t).exp()).abs());
    }
    for w in errors.windows(2) {
        let order = (w[0] / w[1]).log2();
        assert!(
            (1.8..2.3).contains(&order),
            "observed order {order}, errors {errors:?}"
        );
    }
    assert!(errors[2] < 3.0e-5);
}

#[test]
fn j2_l_stable_damping_on_stiff_prothero_robinson() {
    // y' = lambda (y - cos t) - sin t, lambda = -1e6, y = cos t; h*lambda = -5e4.
    let lambda = -1.0e6;
    let problem = scalar_problem(lambda, true);
    let n = 20usize;
    let h = 1.0 / n as f64;
    let st = exact_start(move |t, y| lambda * (y - t.cos()) - t.sin(), f64::cos, h);
    let end = march(&problem, st, h, n);
    assert!(
        (end.u[0] - end.t.cos()).abs() < 1.0e-6,
        "error {}",
        (end.u[0] - end.t.cos()).abs()
    );
    assert!(end.u[0].is_finite());
}
