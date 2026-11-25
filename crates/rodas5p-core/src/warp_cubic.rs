//! Research-only, caller-declared cubic quotient pencil.
//!
//! P(xi)=xi^3+p[2]xi^2+p[1]xi+p[0], W=W0+xi D. The coordinate xi is
//! dimensionless; the caller owns scaling and branch identity. All three RHS
//! coefficients are independent real vectors. No inverse Vandermonde, root
//! extraction, eigenvector computation, or explicit powers of a dense K.
//!
//! With K=-W0^-1 D the elimination of the first two coefficients leaves
//! (I+p2 K+p1 K^2+p0 K^3) X2 = Y2, where Yj is the confluent triangular
//! solve. A bounded folded Neumann recurrence generates candidates. Only an
//! outward residual in the original coefficient equation can accept them.
//! This is not a nonlinear or time-integration error certificate.
use serde::Serialize;

use crate::directed::{Interval, add_up, div_up, mul_up, sub_down};
use crate::shared_shift_jet::CertificateStatus;
use crate::{CoreError, CoreResult, DenseMatrix, LuFactorization, LuSolveWorkspace};

pub type CubicCoefficients = [Vec<f64>; 3];

pub const MAX_DIMENSION: usize = 2048;
pub const MAX_UPDATES: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct CubicConfig {
    /// Absolute error tolerance in sum_j ||X_j||_infinity.
    pub absolute_tolerance: f64,
    /// Maximum number of folded correction terms beyond the triangular seed.
    pub max_updates: usize,
}
impl Default for CubicConfig {
    fn default() -> Self {
        Self {
            absolute_tolerance: 1e-10,
            max_updates: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CubicCertificate {
    status: CertificateStatus,
    residual_upper: f64,
    error_upper: f64,
    gap_lower: f64,
    absolute_tolerance: f64,
}
impl CubicCertificate {
    pub fn status(&self) -> CertificateStatus {
        self.status
    }
    pub fn residual_upper(&self) -> f64 {
        self.residual_upper
    }
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }
    pub fn gap_lower(&self) -> f64 {
        self.gap_lower
    }
    pub fn absolute_tolerance(&self) -> f64 {
        self.absolute_tolerance
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct CubicWork {
    /// Executed solves with the already prepared center LU.
    pub center_solves: usize,
    pub direction_matvecs: usize,
    pub correction_updates: usize,
    /// Products in the original coefficient residual, excluding bound setup.
    pub residual_fmas: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CubicReport {
    coefficients: CubicCoefficients,
    certificate: CubicCertificate,
    work: CubicWork,
}
impl CubicReport {
    /// Approximation even on rejection: inspect certificate().status().
    pub fn coefficients(&self) -> &CubicCoefficients {
        &self.coefficients
    }
    pub fn certificate(&self) -> &CubicCertificate {
        &self.certificate
    }
    pub fn work(&self) -> &CubicWork {
        &self.work
    }
    /// Rounded polynomial evaluation, not a certificate of the target root
    /// or of the additional evaluation roundoff. Coefficients remain available
    /// at a repeated root, where point evaluation alone loses jets.
    pub fn evaluate(&self, re: f64, im: f64) -> CoreResult<(Vec<f64>, Vec<f64>)> {
        if !(re.is_finite() && im.is_finite()) {
            return Err(invalid("nonfinite evaluation coordinate"));
        }
        let n = self.coefficients[0].len();
        let mut a = Vec::with_capacity(n);
        let mut b = Vec::with_capacity(n);
        for i in 0..n {
            let xr = re.mul_add(self.coefficients[2][i], self.coefficients[1][i]);
            let xi = im * self.coefficients[2][i];
            let vr = re.mul_add(xr, self.coefficients[0][i]) - im * xi;
            let vi = im.mul_add(xr, re * xi);
            if !(vr.is_finite() && vi.is_finite()) {
                return Err(CoreError::NonFinite("WARP evaluation".into()));
            }
            a.push(vr);
            b.push(vi);
        }
        Ok((a, b))
    }
}

fn invalid(s: &str) -> CoreError {
    CoreError::InvalidInput(format!("WARP cubic: {s}"))
}
fn inf_norm(x: &[f64]) -> f64 {
    x.iter().fold(0.0_f64, |a, b| a.max(b.abs()))
}
fn validate_vectors(n: usize, x: &CubicCoefficients) -> CoreResult<()> {
    if x.iter().any(|v| v.len() != n) {
        return Err(CoreError::Dimension(
            "WARP coefficient shape mismatch".into(),
        ));
    }
    if !x.iter().flatten().all(|v| v.is_finite()) {
        return Err(CoreError::NonFinite("WARP coefficients".into()));
    }
    Ok(())
}
fn tolerance(t: f64) -> CoreResult<()> {
    if t.is_finite() && t > 0.0 {
        Ok(())
    } else {
        Err(invalid("invalid absolute tolerance"))
    }
}

fn row_bounds(a: &DenseMatrix) -> CoreResult<(f64, f64)> {
    let mut lower = f64::INFINITY;
    let mut upper = 0.0_f64;
    for i in 0..a.nrows() {
        let mut off = 0.0;
        for (j, &v) in a.row(i).iter().enumerate() {
            if i != j {
                off = add_up(off, v.abs())?;
            }
        }
        lower = lower.min(sub_down(a[(i, i)].abs(), off)?);
        upper = upper.max(add_up(a[(i, i)].abs(), off)?);
    }
    Ok((lower, upper))
}

/// Immutable exact-binary-input contract. This holds no factorization and can
/// certify independently generated candidates. Borrowing prevents stale
/// matrix/modulus data while a prepared packet is in use.
#[derive(Clone, Copy, Debug)]
pub struct CubicPencil<'a> {
    w0: &'a DenseMatrix,
    d: &'a DenseMatrix,
    p: [f64; 3],
    alpha: f64,
    beta: f64,
    wnorm: f64,
    gap: f64,
    q: f64,
    gamma: f64,
    underflow_floor: f64,
}

impl<'a> CubicPencil<'a> {
    pub fn new(w0: &'a DenseMatrix, d: &'a DenseMatrix, p: [f64; 3]) -> CoreResult<Self> {
        let n = w0.nrows();
        if n == 0 || n > MAX_DIMENSION || w0.ncols() != n || d.nrows() != n || d.ncols() != n {
            return Err(invalid(
                "expected matching square matrices within dimension cap",
            ));
        }
        if !w0
            .as_slice()
            .iter()
            .chain(d.as_slice())
            .chain(&p)
            .all(|v| v.is_finite())
        {
            return Err(CoreError::NonFinite("WARP pencil".into()));
        }
        let mut ps = 0.0;
        for v in p {
            ps = add_up(ps, v.abs())?;
        }
        if ps > 1.0 {
            return Err(invalid(
                "unit-disk quotient norm not admitted; normalize coordinate",
            ));
        }
        let (alpha, wnorm) = row_bounds(w0)?;
        let (_, beta) = row_bounds(d)?;
        let gap = sub_down(alpha, beta)?;
        if !gap.is_finite() || gap <= 0.0 {
            return Err(invalid(
                "strict row-dominance minus direction norm not positive",
            ));
        }
        let q = div_up(beta, alpha)?;
        // Each row residual is evaluated by exactly 2n sequential FMAs,
        // with exact binary inputs and no intermediate nonfinite arithmetic.
        // Under RN gradual underflow, fl(z)=z(1+delta)+eta with |delta|<=u,
        // |eta|<=MIN_POSITIVE (loose additive floor). Then the absolute
        // recurrence error is <= gamma_(2n)*sum_abs + 2n*eta/(1-2n*u).
        let m = (2 * n) as f64;
        let mu = m * (f64::EPSILON / 2.0);
        let denom = sub_down(1.0, mu)?;
        let gamma = div_up(mu, denom)?;
        let underflow_floor = div_up(mul_up(m, f64::MIN_POSITIVE)?, denom)?;
        Ok(Self {
            w0,
            d,
            p,
            alpha,
            beta,
            wnorm,
            gap,
            q,
            gamma,
            underflow_floor,
        })
    }
    pub fn contraction_upper(&self) -> f64 {
        self.q
    }
    pub fn dominance_lower(&self) -> f64 {
        self.alpha
    }
    pub fn direction_norm_upper(&self) -> f64 {
        self.beta
    }

    /// Full original coefficient residual. For V=(xi X mod P), the three
    /// vectors are V_j=X_(j-1)-p_j X2 (X_-1=0).
    /// Enclose V roundoff with directed arithmetic and row residual roundoff
    /// with the sequential-FMA error bound. No generated K or Y is trusted.
    pub fn certify(
        &self,
        b: &CubicCoefficients,
        x: &CubicCoefficients,
        tol: f64,
    ) -> CoreResult<CubicCertificate> {
        tolerance(tol)?;
        let n = self.w0.nrows();
        validate_vectors(n, b)?;
        validate_vectors(n, x)?;
        let mut residual_upper = 0.0;
        let mut v = vec![0.0; n];
        for j in 0..3 {
            let mut v_error = 0.0_f64;
            for (i, vi) in v.iter_mut().enumerate() {
                let prev = if j == 0 { 0.0 } else { x[j - 1][i] };
                *vi = (-self.p[j]).mul_add(x[2][i], prev);
                let exact =
                    Interval::point(prev)?.sub(Interval::point(self.p[j])?.scale(x[2][i])?)?;
                v_error = v_error.max(exact.sub(Interval::point(*vi)?)?.mag());
            }
            let mut computed = 0.0_f64;
            for (i, &bi) in b[j].iter().enumerate() {
                let mut r = bi;
                for (&a, &xx) in self.w0.row(i).iter().zip(&x[j]) {
                    r = (-a).mul_add(xx, r);
                }
                for (&a, &vv) in self.d.row(i).iter().zip(&v) {
                    r = (-a).mul_add(vv, r);
                }
                if !r.is_finite() {
                    return Err(CoreError::NonFinite("WARP residual".into()));
                }
                computed = computed.max(r.abs());
            }
            let sum_abs = add_up(
                add_up(inf_norm(&b[j]), mul_up(self.wnorm, inf_norm(&x[j]))?)?,
                mul_up(self.beta, inf_norm(&v))?,
            )?;
            let rounding = add_up(mul_up(self.gamma, sum_abs)?, self.underflow_floor)?;
            let upper = add_up(add_up(computed, rounding)?, mul_up(self.beta, v_error)?)?;
            residual_upper = add_up(residual_upper, upper)?;
        }
        let error_upper = div_up(residual_upper, self.gap)?;
        Ok(CubicCertificate {
            status: if error_upper <= tol {
                CertificateStatus::Certified
            } else {
                CertificateStatus::Rejected
            },
            residual_upper,
            error_upper,
            gap_lower: self.gap,
            absolute_tolerance: tol,
        })
    }

    /// One factorization and reusable single-RHS LU scratch.
    pub fn prepare(self) -> CoreResult<CubicPacket<'a>> {
        Ok(CubicPacket {
            lu: LuFactorization::new(self.w0)?,
            scratch: LuSolveWorkspace::new(self.w0.nrows()),
            pencil: self,
        })
    }
}

pub struct CubicPacket<'a> {
    pencil: CubicPencil<'a>,
    lu: LuFactorization,
    scratch: LuSolveWorkspace,
}
impl<'a> CubicPacket<'a> {
    pub fn prepare(w0: &'a DenseMatrix, d: &'a DenseMatrix, p: [f64; 3]) -> CoreResult<Self> {
        CubicPencil::new(w0, d, p)?.prepare()
    }
    pub fn pencil(&self) -> &CubicPencil<'a> {
        &self.pencil
    }

    fn center_solve(&mut self, x: &mut [f64], work: &mut CubicWork) -> CoreResult<()> {
        self.lu
            .solve_in_place_with_workspace(x, &mut self.scratch)?;
        work.center_solves += 1;
        Ok(())
    }

    fn k_action(&mut self, x: &[f64], work: &mut CubicWork) -> CoreResult<Vec<f64>> {
        let mut y = self.pencil.d.matvec(x)?;
        work.direction_matvecs += 1;
        for yi in &mut y {
            *yi = -*yi;
        }
        self.center_solve(&mut y, work)?;
        Ok(y)
    }

    pub fn solve(&mut self, b: &CubicCoefficients, cfg: CubicConfig) -> CoreResult<CubicReport> {
        let n = self.pencil.w0.nrows();
        validate_vectors(n, b)?;
        tolerance(cfg.absolute_tolerance)?;
        if cfg.max_updates > MAX_UPDATES {
            return Err(invalid("correction cap exceeded"));
        }
        let mut work = CubicWork::default();
        let mut y = b.clone();
        self.center_solve(&mut y[0], &mut work)?;
        for j in 1..3 {
            let dy = self.pencil.d.matvec(&y[j - 1])?;
            work.direction_matvecs += 1;
            for (v, dv) in y[j].iter_mut().zip(dy) {
                *v -= dv;
            }
            self.center_solve(&mut y[j], &mut work)?;
        }
        let p = self.pencil.p;
        let q = self.pencil.q.min(1.0);
        // A heuristic tail estimate chooses recurrence length only. The
        // final outward original-equation certificate is the sole authority.
        let cq = p[2].abs() * q + p[1].abs() * q * q + p[0].abs() * q * q * q;
        let rebuild = 1.0 + (p[0].abs() + p[1].abs()) * q + p[0].abs() * q * q;
        let mut tail = if cq < 1.0 {
            inf_norm(&y[2]) * cq / (1.0 - cq) * rebuild
        } else {
            f64::INFINITY
        };
        let degree = if p[0] != 0.0 {
            3
        } else if p[1] != 0.0 {
            2
        } else if p[2] != 0.0 {
            1
        } else {
            0
        };
        let mut term = y[2].clone();
        for _ in 0..cfg.max_updates {
            if degree == 0 || tail <= 0.125 * cfg.absolute_tolerance {
                break;
            }
            let mut power = term;
            let mut next = vec![0.0; n];
            for j in 1..=degree {
                power = self.k_action(&power, &mut work)?;
                if p[3 - j] != 0.0 {
                    for (v, k) in next.iter_mut().zip(&power) {
                        *v = (-p[3 - j]).mul_add(*k, *v);
                    }
                }
            }
            for (v, t) in y[2].iter_mut().zip(&next) {
                *v += t;
            }
            term = next;
            work.correction_updates += 1;
            tail *= cq;
        }
        if p[0] != 0.0 || p[1] != 0.0 {
            let kx = self.k_action(&y[2], &mut work)?;
            for i in 0..n {
                y[0][i] = (-p[0]).mul_add(kx[i], y[0][i]);
                y[1][i] = (-p[1]).mul_add(kx[i], y[1][i]);
            }
            if p[0] != 0.0 {
                let k2x = self.k_action(&kx, &mut work)?;
                for (v, k) in y[1].iter_mut().zip(k2x) {
                    *v = (-p[0]).mul_add(k, *v);
                }
            }
        }
        let certificate = self.pencil.certify(b, &y, cfg.absolute_tolerance)?;
        work.residual_fmas = 6 * n * n;
        Ok(CubicReport {
            coefficients: y,
            certificate,
            work,
        })
    }
}
