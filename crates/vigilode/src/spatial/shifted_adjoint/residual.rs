//! Source-bound residual enclosures for the exact represented diffusion rates.
use super::*;
use rodas5p_core::directed::{sub_up, sum_up};
const U: f64 = f64::EPSILON * 0.5;
const ETA: f64 = f64::from_bits(1);
const MAX_N: usize = 1 << 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ResidualPlan {
    Legacy,
    PackedInterval,
    FusedEdges,
    FusedDiagonal,
    Hybrid,
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ResidualReceipt {
    pub residual_l1: f64,
    pub error_l1: f64,
    pub rhs_l1: f64,
    pub rounding_upper: f64,
    pub admitted: bool,
    pub fast_checks: u64,
    pub interval_checks: u64,
    pub fallbacks: u64,
    pub fmas: u64,
}
#[derive(Clone, Copy)]
struct Row {
    out: [(usize, f64); 6],
    incoming: [(usize, f64); 6],
    count: usize,
    exit: Iv,
    diagonal: f64,
}
pub struct ResidualWorkspace {
    grid: DiffusionGrid,
    plan: ResidualPlan,
    z: Option<f64>,
    rows: Vec<Row>,
    q: f64,
    diag_defect: f64,
    scratch: Vec<Iv>,
}
fn finite(v: f64) -> CoreResult<f64> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(err("nonfinite residual arithmetic"))
    }
}
fn allowed(rhs: f64) -> CoreResult<f64> {
    finite(add_up(mul_up(1e-10, rhs)?, 1e-13)?)
}
fn norm_bound(sum: f64, n: usize) -> CoreResult<f64> {
    // All terms nonnegative represented binary64; addition with a
    // subnormal exact result is exact. Bernoulli: fl(sum)>=(1-nu)*sum.
    // n<=2^20 makes denominator exactly representable and positive.
    finite(div_up(finite(sum)?, 1. - (n as f64) * U)?)
}
impl ResidualWorkspace {
    pub fn new(g: &DiffusionGrid, plan: ResidualPlan) -> CoreResult<Self> {
        rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
        let n = g.dimension();
        if n == 0 || n > MAX_N {
            return Err(err("residual dimension cap"));
        }
        let mut rows = if plan == ResidualPlan::Legacy {
            vec![]
        } else {
            vec![
                Row {
                    out: [(0, 0.); 6],
                    incoming: [(0, 0.); 6],
                    count: 0,
                    exit: Z,
                    diagonal: 0.
                };
                n
            ]
        };
        let mut q = 0_f64;
        if !rows.is_empty() {
            for i in 0..n {
                let out = g.represented_row(i).ok_or_else(|| err("represented row"))?;
                let mut exit = Z;
                for (j, a) in out {
                    if j >= n || !a.is_finite() || a < 0. {
                        return Err(err("source rate"));
                    }
                    if i == j || a == 0. {
                        continue;
                    }
                    exit = exit.add(Iv::point(a)?)?;
                    let row = &mut rows[j];
                    if row.count == 6 {
                        return Err(err("incoming degree cap"));
                    }
                    row.incoming[row.count] = (i, a);
                    row.count += 1;
                }
                rows[i].out = out;
                rows[i].exit = exit;
                q = q.max(exit.hi);
            }
        }
        Ok(Self {
            grid: g.clone(),
            plan,
            z: None,
            rows,
            q,
            diag_defect: 0.,
            scratch: vec![Z; n],
        })
    }
    pub fn prepare(&mut self, z: f64) -> CoreResult<()> {
        self.z = None;
        self.diag_defect = 0.;
        if !z.is_finite() || z <= 0. {
            return Err(err("shift"));
        }
        if matches!(
            self.plan,
            ResidualPlan::FusedDiagonal | ResidualPlan::Hybrid
        ) {
            for r in &mut self.rows {
                // Lower endpoint of exact positive diagonal; its full interval
                // width covers the error in the chosen represented coefficient.
                let d = r.exit.add(Iv::point(z)?)?;
                r.diagonal = d.lo;
                self.diag_defect = self.diag_defect.max(sub_up(d.hi, d.lo)?);
            }
        }
        self.z = Some(z);
        Ok(())
    }
    fn packed(&mut self, z: f64, b: &[f64], x: &[f64], rhs: f64) -> CoreResult<f64> {
        for i in 0..x.len() {
            self.scratch[i] = Iv::point(b[i])?.sub(Iv::point(x[i])?.scale(z)?)?;
        }
        for (i, &v) in x.iter().enumerate() {
            for (j, a) in self.rows[i].out {
                if j != i && a != 0. {
                    let p = Iv::point(v)?.scale(a)?;
                    self.scratch[i] = self.scratch[i].sub(p)?;
                    self.scratch[j] = self.scratch[j].add(p)?;
                }
            }
        }
        let _ = rhs;
        finite(sum_up(self.scratch.iter().map(|v| v.mag()))?)
    }
    fn fused(
        &self,
        z: f64,
        b: &[f64],
        x: &[f64],
        rhs: f64,
        diag: bool,
    ) -> CoreResult<ResidualReceipt> {
        let n = x.len();
        let mut rsum = 0.;
        let mut xsum = 0.;
        let mut fmas = 0_u64;
        for (i, row) in self.rows.iter().enumerate() {
            let c = x[i];
            xsum += c.abs();
            let mut v = if diag {
                (-row.diagonal).mul_add(c, b[i])
            } else {
                (-z).mul_add(c, b[i])
            };
            fmas += 1;
            if !diag {
                for (j, a) in row.out {
                    if j != i && a != 0. {
                        v = (-a).mul_add(c, v);
                        fmas += 1;
                    }
                }
            }
            for &(j, a) in &row.incoming[..row.count] {
                v = a.mul_add(x[j], v);
                fmas += 1;
            }
            rsum += v.abs();
        }
        let xnorm = norm_bound(xsum, n)?;
        let rnorm = norm_bound(rsum, n)?;
        // Sum of exact term magnitudes across all rows. Each original
        // non-self edge contributes once at its tail and once at its head.
        let mass = finite(add_up(
            add_up(rhs, mul_up(z, xnorm)?)?,
            mul_up(2., mul_up(self.q, xnorm)?)?,
        )?)?;
        let m = if diag { 7. } else { 13. };
        // FMA local error <= u*|exact|+eta. Repeated recurrence gives
        // ((1+u)^m-1)*S_i + eta*sum_{k<m}(1+u)^k <= 2*m*u*S_i+2*m*eta.
        let round = finite(add_up(
            mul_up(2. * m * U, mass)?,
            mul_up(2. * m * n as f64, ETA)?,
        )?)?;
        let round = if diag {
            finite(add_up(round, mul_up(self.diag_defect, xnorm)?)?)?
        } else {
            round
        };
        let rho = finite(add_up(rnorm, round)?)?;
        Ok(ResidualReceipt {
            residual_l1: rho,
            error_l1: finite(div_up(rho, z)?)?,
            rhs_l1: rhs,
            rounding_upper: round,
            admitted: rho <= allowed(rhs)?,
            fast_checks: 1,
            fmas,
            ..Default::default()
        })
    }
    /// Return a bound even if it exceeds the stage gate. No proposal is
    /// admitted from a rounded residual without the derived outward bound.
    pub fn evaluate(&mut self, b: &[f64], x: &[f64]) -> CoreResult<ResidualReceipt> {
        rodas5p_integrators::affine_event_certificate::check_affine_event_arithmetic()?;
        let z = self.z.ok_or_else(|| err("not prepared"))?;
        let n = self.grid.dimension();
        if b.len() != n || x.len() != n || b.iter().chain(x).any(|v| !v.is_finite()) {
            return Err(err("residual shape/nonfinite"));
        }
        if self.plan == ResidualPlan::Legacy {
            let (r, e, s) = directed_residual(&self.grid, z, b, x, &mut self.scratch)?;
            return Ok(ResidualReceipt {
                residual_l1: finite(r)?,
                error_l1: finite(e)?,
                rhs_l1: s,
                admitted: r <= allowed(s)?,
                interval_checks: 1,
                ..Default::default()
            });
        }
        // Keep exactly the legacy directed RHS norm and acceptance threshold.
        let rhs = finite(sum_up(b.iter().map(|v| v.abs()))?)?;
        if matches!(
            self.plan,
            ResidualPlan::FusedEdges | ResidualPlan::FusedDiagonal
        ) {
            return self.fused(z, b, x, rhs, self.plan == ResidualPlan::FusedDiagonal);
        }
        let mut attempt = ResidualReceipt::default();
        if self.plan == ResidualPlan::Hybrid {
            match self.fused(z, b, x, rhs, true) {
                Ok(r) if r.admitted => return Ok(r),
                Ok(r) => attempt = r,
                Err(_) => attempt.fast_checks = 1,
            }
            attempt.fallbacks = 1;
        }
        let rho = self.packed(z, b, x, rhs)?;
        Ok(ResidualReceipt {
            residual_l1: rho,
            error_l1: finite(div_up(rho, z)?)?,
            rhs_l1: rhs,
            admitted: rho <= allowed(rhs)?,
            interval_checks: 1,
            fast_checks: attempt.fast_checks,
            fallbacks: attempt.fallbacks,
            fmas: attempt.fmas,
            rounding_upper: attempt.rounding_upper,
        })
    }
}
