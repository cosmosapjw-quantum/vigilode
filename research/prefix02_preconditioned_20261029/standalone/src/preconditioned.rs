//! Source-guarded left/right diagonally preconditioned GMRES prefix prototype.
//! Only original-equation outward residual and stage error admit a candidate.
use crate::{Admission, BoundModel, TriggerMode, check_arithmetic};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingSnapshot {
    pub model_id: String,
    pub source_id: String,
    pub clock_id: String,
    pub norm_id: String,
    pub epoch: u64,
}
impl BindingSnapshot {
    pub fn from_model(model: &BoundModel) -> Self {
        Self {
            model_id: model.model_id.clone(),
            source_id: model.source_id.clone(),
            clock_id: model.clock_id.clone(),
            norm_id: model.norm_id.clone(),
            epoch: model.epoch,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}
#[derive(Clone, Copy, Debug)]
pub struct PrefixConfig {
    pub max_columns: usize,
    pub restart: usize,
    pub tau_orig: f64,
    pub stage_budget: f64,
    pub mode: TriggerMode,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Work {
    pub columns: usize,
    pub arnoldi_actions: usize,
    pub original_actions: usize,
    pub preconditioner_actions: usize,
    pub mgs_dots: usize,
    pub checks: usize,
    pub refused: usize,
    pub backsolves: usize,
    pub restarts: usize,
    pub identity_checks: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct Outcome {
    pub accepted: bool,
    pub certificate: Option<Admission>,
    pub work: Work,
    pub projected_residual: f64,
}
fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(a, b)| a * b).sum()
}
fn norm(x: &[f64]) -> f64 {
    dot(x, x).sqrt()
}
fn validate_identity(
    model: &BoundModel,
    expected: &BindingSnapshot,
    live: &mut impl FnMut() -> BindingSnapshot,
    w: &mut Work,
) -> Result<(), String> {
    w.identity_checks += 1;
    if &BindingSnapshot::from_model(model) != expected || &live() != expected {
        return Err("source/clock/norm/epoch identity changed".into());
    }
    Ok(())
}
fn apply_inv(diag: &[f64], x: &[f64], w: &mut Work) -> Result<Vec<f64>, String> {
    w.preconditioner_actions += 1;
    let y: Vec<f64> = diag.iter().zip(x).map(|(d, x)| d * x).collect();
    if y.iter().all(|v| v.is_finite()) {
        Ok(y)
    } else {
        Err("preconditioner overflow/nonfinite".into())
    }
}
fn original_residual(
    model: &BoundModel,
    rhs: &[f64],
    x: &[f64],
    w: &mut Work,
    live: &mut impl FnMut() -> BindingSnapshot,
    expected: &BindingSnapshot,
) -> Result<Vec<f64>, String> {
    validate_identity(model, expected, live, w)?;
    let mut ax = vec![0.0; model.n];
    model.apply(x, &mut ax)?;
    w.original_actions += 1;
    let r: Vec<f64> = rhs.iter().zip(ax).map(|(b, a)| b - a).collect();
    if r.iter().all(|v| v.is_finite()) {
        Ok(r)
    } else {
        Err("original residual nonfinite".into())
    }
}
fn backsolve(h: &[Vec<f64>], g: &[f64], k: usize) -> Result<Vec<f64>, String> {
    let mut y = vec![0.0; k];
    for i in (0..k).rev() {
        let mut s = g[i];
        for j in (i + 1)..k {
            s -= h[i][j] * y[j];
        }
        let d = h[i][i];
        if !d.is_finite() || d.abs() <= f64::MIN_POSITIVE {
            return Err("singular/nonfinite prefix triangular solve".into());
        }
        y[i] = s / d;
        if !y[i].is_finite() {
            return Err("nonfinite prefix triangular solve".into());
        }
    }
    Ok(y)
}

/// A=I-sigma*J is the original represented operator. For right side Arnoldi
/// uses A*M^-1 and x=x0+M^-1*V*y; for left side M^-1*A and x=x0+V*y.
/// `live` must return the actual binding at each action/commit; no mutation of
/// caller output occurs until a valid original residual and forward bound pass.
#[allow(clippy::too_many_arguments)]
pub fn solve_preconditioned(
    model: &BoundModel,
    rhs: &[f64],
    x0: &[f64],
    output: &mut [f64],
    inv_diag: &[f64],
    side: Side,
    cfg: PrefixConfig,
    expected: &BindingSnapshot,
    live: &mut impl FnMut() -> BindingSnapshot,
) -> Result<Outcome, String> {
    model.validate()?;
    check_arithmetic()?;
    let n = model.n;
    if rhs.len() != n
        || x0.len() != n
        || output.len() != n
        || inv_diag.len() != n
        || cfg.max_columns == 0
        || cfg.restart == 0
        || cfg.restart > n
        || !cfg.tau_orig.is_finite()
        || cfg.tau_orig < 0.0
        || !cfg.stage_budget.is_finite()
        || cfg.stage_budget < 0.0
        || rhs.iter().chain(x0).any(|v| !v.is_finite())
        || inv_diag.iter().any(|v| !v.is_finite() || *v == 0.0)
    {
        return Err("invalid preconditioned system, bounds or budget".into());
    }
    if let TriggerMode::Prefix {
        projected_tau,
        ratio,
    } = cfg.mode
        && (!projected_tau.is_finite() || projected_tau < 0.0 || !ratio.is_finite() || ratio <= 0.0)
    {
        return Err("invalid projected trigger".into());
    }
    let mut work = Work::default();
    validate_identity(model, expected, live, &mut work)?;
    let mut x_cycle = x0.to_vec();
    let initial = model.certify(rhs, &x_cycle, cfg.tau_orig, cfg.stage_budget)?;
    work.checks += 1;
    if initial.accepted {
        validate_identity(model, expected, live, &mut work)?;
        output.copy_from_slice(&x_cycle);
        return Ok(Outcome {
            accepted: true,
            certificate: Some(initial),
            work,
            projected_residual: 0.0,
        });
    }
    let mut last_cert = Some(initial);
    let mut last_projected = f64::INFINITY;
    while work.columns < cfg.max_columns {
        let residual = original_residual(model, rhs, &x_cycle, &mut work, live, expected)?;
        let r0 = match side {
            Side::Right => residual,
            Side::Left => apply_inv(inv_diag, &residual, &mut work)?,
        };
        let beta = norm(&r0);
        if !beta.is_finite() {
            return Err("nonfinite preconditioned initial residual".into());
        }
        if beta <= f64::MIN_POSITIVE {
            // Preconditioned norm can vanish even if original residual does not.
            return Ok(Outcome {
                accepted: false,
                certificate: last_cert,
                work,
                projected_residual: beta,
            });
        }
        let steps = cfg.restart.min(cfg.max_columns - work.columns).min(n);
        let mut v = vec![vec![0.0; n]; steps + 1];
        for i in 0..n {
            v[0][i] = r0[i] / beta;
        }
        let mut h = vec![vec![0.0; steps]; steps + 1];
        let mut cs = vec![0.0; steps];
        let mut sn = vec![0.0; steps];
        let mut g = vec![0.0; steps + 1];
        g[0] = beta;
        let mut last_candidate: Option<Vec<f64>> = None;
        let mut breakdown = false;
        for j in 0..steps {
            validate_identity(model, expected, live, &mut work)?;
            let op_input = match side {
                Side::Right => apply_inv(inv_diag, &v[j], &mut work)?,
                Side::Left => v[j].clone(),
            };
            let mut w = vec![0.0; n];
            model.apply(&op_input, &mut w)?;
            work.arnoldi_actions += 1;
            if side == Side::Left {
                w = apply_inv(inv_diag, &w, &mut work)?;
            }
            let raw_norm = norm(&w);
            for _ in 0..2 {
                for i in 0..=j {
                    let hij = dot(&v[i], &w);
                    work.mgs_dots += 1;
                    h[i][j] += hij;
                    for t in 0..n {
                        w[t] -= hij * v[i][t];
                    }
                }
            }
            let hn = norm(&w);
            h[j + 1][j] = hn;
            breakdown = hn <= 64.0 * f64::EPSILON * raw_norm.max(f64::MIN_POSITIVE);
            if !breakdown {
                for i in 0..n {
                    v[j + 1][i] = w[i] / hn;
                }
            }
            for i in 0..j {
                let t = cs[i] * h[i][j] + sn[i] * h[i + 1][j];
                h[i + 1][j] = -sn[i] * h[i][j] + cs[i] * h[i + 1][j];
                h[i][j] = t;
            }
            let d = h[j][j].hypot(h[j + 1][j]);
            if d == 0.0 || !d.is_finite() {
                return Err("singular/nonfinite Givens diagonal".into());
            }
            cs[j] = h[j][j] / d;
            sn[j] = h[j + 1][j] / d;
            h[j][j] = d;
            h[j + 1][j] = 0.0;
            g[j + 1] = -sn[j] * g[j];
            g[j] *= cs[j];
            last_projected = g[j + 1].abs();
            work.columns += 1;
            let cycle_end = j + 1 == steps || breakdown;
            let trigger = match cfg.mode {
                TriggerMode::Prefix {
                    projected_tau,
                    ratio,
                } => last_projected <= projected_tau * ratio,
                TriggerMode::Always => true,
                TriggerMode::EndOnly => false,
            };
            if trigger || cycle_end {
                work.backsolves += 1;
                let y = backsolve(&h, &g, j + 1)?;
                let mut delta = vec![0.0; n];
                for k in 0..=j {
                    for i in 0..n {
                        delta[i] += y[k] * v[k][i];
                    }
                }
                let delta = match side {
                    Side::Right => apply_inv(inv_diag, &delta, &mut work)?,
                    Side::Left => delta,
                };
                let candidate: Vec<f64> = x_cycle.iter().zip(delta).map(|(a, b)| a + b).collect();
                if candidate.iter().any(|v| !v.is_finite()) {
                    return Err("nonfinite stored candidate".into());
                }
                validate_identity(model, expected, live, &mut work)?;
                let cert = model.certify(rhs, &candidate, cfg.tau_orig, cfg.stage_budget)?;
                work.checks += 1;
                last_cert = Some(cert);
                if cert.accepted {
                    validate_identity(model, expected, live, &mut work)?;
                    output.copy_from_slice(&candidate);
                    return Ok(Outcome {
                        accepted: true,
                        certificate: Some(cert),
                        work,
                        projected_residual: last_projected,
                    });
                }
                work.refused += 1;
                last_candidate = Some(candidate);
            }
            if breakdown {
                break;
            }
        }
        if breakdown {
            break;
        }
        if let Some(candidate) = last_candidate {
            x_cycle = candidate;
            work.restarts += 1;
        } else {
            return Err("cycle ended without a candidate".into());
        }
    }
    Ok(Outcome {
        accepted: false,
        certificate: last_cert,
        work,
        projected_residual: last_projected,
    })
}
