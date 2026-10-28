//! H5 certificate for the actual represented all-entry K stage target.
//!
//! The immutable collective source generates f and the frozen represented J.
//! Every stored alpha/beta entry, including upper/diagonal leakage, is retained.
//! A successful result certifies the stage equation and its output functional;
//! it does NOT certify the ODE truncation error of that RODAS step.

use crate::CollectiveModel;
use rayon::prelude::*;
use rodas5p_core::directed::{Interval, add_up, div_up, mul_up, sub_down};
use rodas5p_core::{CoreError, DenseMatrix, Rodas5pCoefficients, inverse, rodas5p_coefficients};
use serde::Serialize;
use thiserror::Error;

type Iv = Interval;
fn pt(x: f64) -> Result<Iv, CoreError> {
    Iv::point(x)
}

#[derive(Clone, Debug, Serialize)]
pub struct ActualKHomotopyConfig {
    pub h: f64,
    pub lambdas: Vec<f64>,
    pub rounds_per_stage: usize,
    /// Component infinity-radius of the COMMON all-lambda stage tube.
    pub tube_radius: f64,
    /// Bound on the represented stage-output functional, not ODE truncation.
    pub stage_output_tolerance: f64,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ActualKHomotopyWork {
    pub candidate_rounds: u64,
    pub candidate_rhs_calls: u64,
    pub frozen_jvp_columns: u64,
    pub inverse_constructions: u64,
    pub certificate_rhs_evaluations: u64,
    pub retained_alpha_beta_entries: usize,
    pub positive_series_terms: usize,
}
#[derive(Debug, Error)]
pub enum ActualKHomotopyError {
    #[error("actual-K input is outside the sealed collective-stage contract")]
    InvalidInput,
    #[error("inverse witness failed: residual norm {0} is not below one")]
    InverseWitness(f64),
    #[error("H5 leakage witness failed: ||SE||={0} is not below one")]
    LeakageWitness(f64),
    #[error("H5 common tube did not close")]
    TubeDidNotClose,
    #[error("actual-K output radius {radius} exceeds {tolerance}")]
    OutputBudget { radius: f64, tolerance: f64 },
    #[error(transparent)]
    Core(#[from] CoreError),
}
#[derive(Clone, Debug)]
pub struct CertifiedActualKHomotopy {
    source: CollectiveModel,
    coefficients: Rodas5pCoefficients,
    frozen_jacobian: DenseMatrix,
    stages: Vec<Vec<f64>>,
    stage_radii: Vec<f64>,
    output: Vec<f64>,
    output_radii: Vec<f64>,
    h: f64,
    inverse_residual: f64,
    leakage_q: f64,
    noncausal_alpha_max: f64,
    noncausal_linear_max: f64,
    common_tube_radius: f64,
    majorant: Vec<Vec<f64>>,
    common_defect: Vec<f64>,
}
impl CertifiedActualKHomotopy {
    pub fn common_tube_radius(&self) -> f64 {
        self.common_tube_radius
    }
    pub fn majorant(&self) -> &[Vec<f64>] {
        &self.majorant
    }
    pub fn common_defect(&self) -> &[f64] {
        &self.common_defect
    }
    pub fn source_model(&self) -> &CollectiveModel {
        &self.source
    }
    pub fn coefficients(&self) -> &Rodas5pCoefficients {
        &self.coefficients
    }
    pub fn frozen_jacobian(&self) -> &DenseMatrix {
        &self.frozen_jacobian
    }
    pub fn stages(&self) -> &[Vec<f64>] {
        &self.stages
    }
    pub fn stage_radii(&self) -> &[f64] {
        &self.stage_radii
    }
    pub fn output(&self) -> &[f64] {
        &self.output
    }
    pub fn output_radii(&self) -> &[f64] {
        &self.output_radii
    }
    pub fn h(&self) -> f64 {
        self.h
    }
    pub fn inverse_residual(&self) -> f64 {
        self.inverse_residual
    }
    pub fn leakage_q(&self) -> f64 {
        self.leakage_q
    }
    pub fn noncausal_alpha_max(&self) -> f64 {
        self.noncausal_alpha_max
    }
    pub fn noncausal_linear_max(&self) -> f64 {
        self.noncausal_linear_max
    }
}
fn iv_rhs(model: &CollectiveModel, y: &[Iv]) -> Result<Vec<Iv>, CoreError> {
    let (mut u, mut v) = (pt(0.0)?, pt(0.0)?);
    for (j, &w) in model.weights().iter().enumerate() {
        u = u.add(y[2 * j].scale(w)?)?;
        v = v.add(y[2 * j + 1].scale(w)?)?;
    }
    let r2 = u.mul(u)?.add(v.mul(v)?)?;
    let nu = r2.mul(v)?.scale(-model.chi())?;
    let nv = r2.mul(u)?.scale(model.chi())?;
    let mut out = Vec::with_capacity(y.len());
    for (j, &xi) in model.xi().iter().enumerate() {
        let omega = pt(model.width())?.mul(pt(xi)?)?;
        out.push(
            y[2 * j]
                .scale(-model.kappa())?
                .sub(omega.mul(y[2 * j + 1])?)?
                .add(nu)?,
        );
        out.push(
            y[2 * j + 1]
                .scale(-model.kappa())?
                .add(omega.mul(y[2 * j])?)?
                .add(nv)?,
        );
    }
    Ok(out)
}
fn remainder_derivative_norm(
    model: &CollectiveModel,
    y: &[Iv],
    jac: &DenseMatrix,
) -> Result<f64, CoreError> {
    let (mut u, mut v) = (pt(0.0)?, pt(0.0)?);
    for (j, &w) in model.weights().iter().enumerate() {
        u = u.add(y[2 * j].scale(w)?)?;
        v = v.add(y[2 * j + 1].scale(w)?)?;
    }
    let uu = u.mul(u)?;
    let vv = v.mul(v)?;
    let uv = u.mul(v)?;
    let d00 = uv.scale(-2.0)?.scale(model.chi())?;
    let d01 = uu.add(vv.scale(3.0)?)?.scale(-model.chi())?;
    let d10 = uu.scale(3.0)?.add(vv)?.scale(model.chi())?;
    let d11 = uv.scale(2.0)?.scale(model.chi())?;
    let mut norm = 0.0_f64;
    for row in 0..y.len() {
        let mut sum = 0.0;
        for col in 0..y.len() {
            let d = match (row % 2, col % 2) {
                (0, 0) => d00,
                (0, 1) => d01,
                (1, 0) => d10,
                _ => d11,
            };
            let mut value = d.scale(model.weights()[col / 2])?;
            if row == col {
                value = value.add(pt(-model.kappa())?)?;
            }
            if row / 2 == col / 2 && row % 2 != col % 2 {
                let omega = pt(model.width())?.mul(pt(model.xi()[row / 2])?)?;
                value = if row % 2 == 0 {
                    value.sub(omega)?
                } else {
                    value.add(omega)?
                };
            }
            sum = add_up(sum, value.sub(pt(jac[(row, col)])?)?.mag())?;
        }
        norm = norm.max(sum);
    }
    Ok(norm)
}
fn iv_matvec(a: &[Vec<Iv>], x: &[Iv]) -> Result<Vec<Iv>, CoreError> {
    a.iter()
        .map(|r| {
            r.iter()
                .zip(x)
                .try_fold(pt(0.0)?, |v, (&a, &x)| v.add(a.mul(x)?))
        })
        .collect()
}
fn positive_matvec(a: &[Vec<f64>], x: &[f64]) -> Result<Vec<f64>, CoreError> {
    a.iter()
        .map(|r| {
            r.iter()
                .zip(x)
                .try_fold(0.0, |v, (&a, &x)| add_up(v, mul_up(a, x)?))
        })
        .collect()
}

/// Real nonlinear stage homotopy, followed by an all-entry H5 witness on one
/// common tube for every lambda in [0,1]. The finite-precision candidate is
/// speculative; interval source polynomials supply all certificate authority.
// Indices intentionally mirror the all-entry tableau and the S/E matrix proof.
#[allow(clippy::needless_range_loop)]
pub fn certify_collective_actual_k_homotopy(
    model: &CollectiveModel,
    config: &ActualKHomotopyConfig,
    work: &mut ActualKHomotopyWork,
) -> Result<CertifiedActualKHomotopy, ActualKHomotopyError> {
    if model.carrier() != 0.0
        || !config.h.is_finite()
        || config.h <= 0.0
        || config.rounds_per_stage == 0
        || !config.tube_radius.is_finite()
        || config.tube_radius <= 0.0
        || !config.stage_output_tolerance.is_finite()
        || config.stage_output_tolerance <= 0.0
        || config.lambdas.is_empty()
        || config.lambdas.last() != Some(&1.0)
        || config
            .lambdas
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0 || *x > 1.0)
        || config.lambdas.windows(2).any(|p| p[1] <= p[0])
    {
        return Err(ActualKHomotopyError::InvalidInput);
    }
    let n = model.dimension();
    let coeff = rodas5p_coefficients()?.clone();
    let stages = coeff.stages();
    let y: Vec<_> = model
        .initial_amplitudes()
        .iter()
        .flat_map(|z| [z.0, z.1])
        .collect();
    let mut jac = DenseMatrix::zeros(n, n);
    for col in 0..n {
        let mut v = vec![0.0; n];
        v[col] = 1.0;
        let mut jv = vec![0.0; n];
        work.frozen_jvp_columns += 1;
        model.jvp(0.0, &y, &v, &mut jv)?;
        for row in 0..n {
            jac[(row, col)] = jv[row];
        }
    }
    let mut w = DenseMatrix::identity(n);
    let mut wi = vec![vec![pt(0.0)?; n]; n];
    let mut ji = vec![vec![pt(0.0)?; n]; n];
    for i in 0..n {
        for j in 0..n {
            ji[i][j] = pt(jac[(i, j)])?;
            wi[i][j] = pt(if i == j { 1.0 } else { 0.0 })?
                .sub(pt(config.h)?.mul(pt(coeff.gamma)?)?.mul(ji[i][j])?)?;
            w[(i, j)] -= config.h * coeff.gamma * jac[(i, j)];
        }
    }
    work.inverse_constructions += 1;
    let inv = inverse(&w)?;
    let (mut rho, mut invnorm, mut jnorm) = (0.0_f64, 0.0_f64, 0.0_f64);
    for i in 0..n {
        let (mut rr, mut vr, mut jr) = (0.0, 0.0, 0.0);
        for j in 0..n {
            let mut sum = pt(0.0)?;
            for l in 0..n {
                sum = sum.add(wi[l][j].scale(inv[(i, l)])?)?;
            }
            rr = add_up(rr, pt(if i == j { 1.0 } else { 0.0 })?.sub(sum)?.mag())?;
            vr = add_up(vr, inv[(i, j)].abs())?;
            jr = add_up(jr, jac[(i, j)].abs())?;
        }
        rho = rho.max(rr);
        invnorm = invnorm.max(vr);
        jnorm = jnorm.max(jr);
    }
    if rho >= 1.0 {
        return Err(ActualKHomotopyError::InverseWitness(rho));
    }
    let winv = div_up(invnorm, sub_down(1.0, rho)?)?;
    let mut initial_rhs = vec![0.0; n];
    model.rhs(0.0, &y, &mut initial_rhs)?;
    let mut k = vec![vec![0.0; n]; stages];
    for &lambda in &config.lambdas {
        for _ in 0..config.rounds_per_stage {
            work.candidate_rounds += 1;
            work.candidate_rhs_calls += stages as u64;
            let next: Vec<_> = (0..stages)
                .into_par_iter()
                .map(|i| -> Result<Vec<f64>, CoreError> {
                    let mut delta = vec![0.0; n];
                    let mut coupling = vec![0.0; n];
                    for j in 0..stages {
                        for a in 0..n {
                            delta[a] += coeff.alpha[(i, j)] * k[j][a];
                            coupling[a] += (coeff.beta[(i, j)]
                                - if i == j { coeff.gamma } else { 0.0 })
                                * k[j][a];
                        }
                    }
                    let state: Vec<_> = y.iter().zip(&delta).map(|(a, b)| a + b).collect();
                    let mut f = vec![0.0; n];
                    model.rhs(0.0, &state, &mut f)?;
                    let jd = jac.matvec(&delta)?;
                    let jc = jac.matvec(&coupling)?;
                    let rhs: Vec<_> = (0..n)
                        .map(|a| {
                            config.h
                                * (initial_rhs[a]
                                    + jc[a]
                                    + lambda * (f[a] - initial_rhs[a] - jd[a]))
                        })
                        .collect();
                    inv.matvec(&rhs)
                })
                .collect();
            k = next.into_iter().collect::<Result<Vec<_>, _>>()?;
        }
    }
    work.retained_alpha_beta_entries = 2 * stages * stages;
    let yiv: Vec<_> = y.iter().map(|&x| pt(x)).collect::<Result<_, _>>()?;
    work.certificate_rhs_evaluations += 1;
    let f0 = iv_rhs(model, &yiv)?;
    let mut d = vec![0.0_f64; stages];
    let mut major = vec![vec![0.0; stages]; stages];
    let (mut leak_alpha, mut leak_linear) = (0.0_f64, 0.0_f64);
    for i in 0..stages {
        let mut delta = vec![pt(0.0)?; n];
        let mut tube_delta = delta.clone();
        let mut coupling = delta.clone();
        for j in 0..stages {
            let lin = pt(coeff.beta[(i, j)])?.sub(pt(if i == j { coeff.gamma } else { 0.0 })?)?;
            if j >= i {
                leak_alpha = leak_alpha.max(coeff.alpha[(i, j)].abs());
                leak_linear = leak_linear.max(lin.mag());
            }
            for a in 0..n {
                delta[a] = delta[a].add(pt(k[j][a])?.scale(coeff.alpha[(i, j)])?)?;
                let tube = Iv::new(
                    sub_down(k[j][a], config.tube_radius)?,
                    add_up(k[j][a], config.tube_radius)?,
                )?;
                tube_delta[a] = tube_delta[a].add(tube.scale(coeff.alpha[(i, j)])?)?;
                coupling[a] = coupling[a].add(lin.mul(pt(k[j][a])?)?)?;
            }
        }
        let state: Vec<_> = yiv
            .iter()
            .zip(&delta)
            .map(|(&a, &b)| a.add(b))
            .collect::<Result<_, _>>()?;
        let tube: Vec<_> = yiv
            .iter()
            .zip(&tube_delta)
            .map(|(&a, &b)| a.add(b))
            .collect::<Result<_, _>>()?;
        work.certificate_rhs_evaluations += 1;
        let f = iv_rhs(model, &state)?;
        let jd = iv_matvec(&ji, &delta)?;
        let jc = iv_matvec(&ji, &coupling)?;
        let ki: Vec<_> = k[i].iter().map(|&x| pt(x)).collect::<Result<_, _>>()?;
        let wk = iv_matvec(&wi, &ki)?;
        for a in 0..n {
            let base = f0[a].add(jc[a])?.scale(config.h)?.sub(wk[a])?;
            let remainder = f[a].sub(f0[a])?.sub(jd[a])?.scale(config.h)?;
            d[i] = d[i].max(mul_up(winv, base.mag().max(base.add(remainder)?.mag()))?);
        }
        let ln = remainder_derivative_norm(model, &tube, &jac)?;
        for j in 0..stages {
            let lin = pt(coeff.beta[(i, j)])?
                .sub(pt(if i == j { coeff.gamma } else { 0.0 })?)?
                .mag();
            major[i][j] = mul_up(
                mul_up(winv, config.h)?,
                add_up(mul_up(jnorm, lin)?, mul_up(ln, coeff.alpha[(i, j)].abs())?)?,
            )?;
        }
    }
    // S=(I-B)^-1 by finite positive forward substitution, retaining E.
    let mut s = vec![vec![0.0; stages]; stages];
    for i in 0..stages {
        for j in 0..=i {
            let mut x = if i == j { 1.0 } else { 0.0 };
            for l in 0..i {
                x = add_up(x, mul_up(major[i][l], s[l][j])?)?;
            }
            s[i][j] = x;
        }
    }
    let mut t = vec![vec![0.0; stages]; stages];
    let mut q = 0.0_f64;
    for i in 0..stages {
        let mut rowsum = 0.0;
        for j in 0..stages {
            for l in 0..=j {
                t[i][j] = add_up(t[i][j], mul_up(s[i][l], major[l][j])?)?;
            }
            rowsum = add_up(rowsum, t[i][j])?;
        }
        q = q.max(rowsum);
    }
    if q >= 1.0 {
        return Err(ActualKHomotopyError::LeakageWitness(q));
    }
    let slack = (config.tube_radius * 1e-8).max(f64::MIN_POSITIVE);
    let dplus: Vec<_> = d
        .iter()
        .map(|&x| add_up(x, slack))
        .collect::<Result<_, _>>()?;
    let mut term = positive_matvec(&s, &dplus)?;
    let sd_norm = term.iter().copied().fold(0.0, f64::max);
    let mut radii = term.clone();
    let mut qpower = 1.0;
    for _ in 0..8 {
        term = positive_matvec(&t, &term)?;
        for (r, &x) in radii.iter_mut().zip(&term) {
            *r = add_up(*r, x)?;
        }
        qpower = mul_up(qpower, q)?;
    }
    work.positive_series_terms = 9;
    let tail = div_up(mul_up(mul_up(qpower, q)?, sd_norm)?, sub_down(1.0, q)?)?;
    for r in &mut radii {
        *r = add_up(*r, tail)?;
    }
    let ar = positive_matvec(&major, &radii)?;
    if radii
        .iter()
        .zip(&d)
        .zip(&ar)
        .any(|((&r, &d), &a)| r > config.tube_radius || add_up(d, a).map_or(true, |v| v > r))
    {
        return Err(ActualKHomotopyError::TubeDidNotClose);
    }
    let mut output = y.clone();
    let mut bounds = vec![0.0; n];
    let mut stage_bound = 0.0;
    for (i, &r) in radii.iter().enumerate() {
        stage_bound = add_up(stage_bound, mul_up(coeff.b[i].abs(), r)?)?;
    }
    for a in 0..n {
        let mut exact = pt(y[a])?;
        for i in 0..stages {
            output[a] += coeff.b[i] * k[i][a];
            exact = exact.add(pt(k[i][a])?.scale(coeff.b[i])?)?;
        }
        bounds[a] = add_up(stage_bound, exact.sub(pt(output[a])?)?.mag())?;
    }
    let max_bound = bounds.iter().copied().fold(0.0, f64::max);
    if max_bound > config.stage_output_tolerance {
        return Err(ActualKHomotopyError::OutputBudget {
            radius: max_bound,
            tolerance: config.stage_output_tolerance,
        });
    }
    Ok(CertifiedActualKHomotopy {
        source: model.clone(),
        coefficients: coeff,
        frozen_jacobian: jac,
        stages: k,
        stage_radii: radii,
        output,
        output_radii: bounds,
        h: config.h,
        inverse_residual: rho,
        leakage_q: q,
        noncausal_alpha_max: leak_alpha,
        noncausal_linear_max: leak_linear,
        common_tube_radius: config.tube_radius,
        majorant: major,
        common_defect: d,
    })
}
