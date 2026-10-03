//! A static stage-coordinate candidate adapter (integrated DAG node INT-04,
//! external review M1, findings TF-06/TF-07). Research only.
//!
//! For the declared stage residual `R(K) = 0` of the quadratic test family
//! (see [`crate::outward_certificate`]) and a regular chart `K = Psi(Z)`,
//! [`stage_chart_candidate`] runs Newton on `F(Z) = R(Psi(Z))` with the
//! static chain rule `D F = D_K R D_Z Psi` and restores `K = Psi(Z)`. The
//! root of the original target is unchanged by the chart; nothing here is a
//! coordinate change of the ODE (no push-forward or connection term).
//!
//! The result is a candidate and nothing more: [`ChartCandidate`] has no
//! acceptance field, and its status and chart residual are diagnostics. A
//! caller admits the restored stages only through the original
//! [`crate::certify_stage_target`] (or the native q2 path) on `K`.

use rodas5p_core::{CoreError, CoreResult, DenseMatrix, LuFactorization};
use serde::Serialize;

use crate::{QuadraticStageProblem, StageTarget};

/// A chart `K = Psi(Z)` on stage vectors flattened stage-major (`s n`).
pub trait StageChart {
    fn name(&self) -> &str;
    /// Whether `z` lies in the chart's domain.
    fn contains(&self, z: &[f64]) -> bool;
    fn forward(&self, z: &[f64]) -> CoreResult<Vec<f64>>;
    fn inverse(&self, k: &[f64]) -> CoreResult<Vec<f64>>;
    /// `D Psi(z) dz`.
    fn jvp(&self, z: &[f64], dz: &[f64]) -> CoreResult<Vec<f64>>;
}

/// Why the Newton iteration stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChartStatus {
    /// `||R(Psi(Z))||_inf` reached the tolerance.
    Converged,
    /// The iteration limit was reached first.
    NotConverged,
    /// The chained Jacobian `D_K R D_Z Psi` could not be factored.
    SingularChart,
    /// A Newton step left the chart's domain; the last iterate inside is
    /// kept.
    DomainExit,
    /// A residual, chart value or step was not finite.
    NonFinite,
}

/// The work of one adapter call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ChartWork {
    pub residual_evaluations: u64,
    pub chart_forward: u64,
    pub chart_jvps: u64,
    pub residual_jacobian_actions: u64,
    pub factorizations: u64,
    /// Order of each factored matrix (`s n`).
    pub factorization_order: u64,
}

/// A candidate from a chart: the restored stages, why the iteration
/// stopped, and diagnostics. It carries no acceptance.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChartCandidate {
    pub chart: String,
    pub stages: Vec<Vec<f64>>,
    pub status: ChartStatus,
    pub iterations: usize,
    /// `||R(Psi(Z))||_inf` at the returned iterate (binary64, midpoint
    /// coupling): a diagnostic, not a bound.
    pub chart_residual_inf: f64,
    pub work: ChartWork,
}

/// The declared stage residual in binary64:
/// `R_i = W K_i - h (J y - q y^2) - h J sum_{j<i} L*_ij K_j - h q (sum_{j<i} alpha_ij K_j)^2`
/// with the midpoint of each `L*` interval.
pub fn stage_residual(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    k: &[f64],
) -> CoreResult<Vec<f64>> {
    let (s, n) = check_shapes(target, problem, k)?;
    let (h, gamma) = (problem.h, target.gamma);
    let base: Vec<f64> = (0..n)
        .map(|a| {
            let jy: f64 = (0..n).map(|b| problem.jacobian[a][b] * problem.y[b]).sum();
            h * (jy - problem.q[a] * problem.y[a] * problem.y[a])
        })
        .collect();
    let mut out = vec![0.0; s * n];
    for i in 0..s {
        let (coupled, delta) = coupled_and_delta(target, k, i, n);
        for a in 0..n {
            let mut wk = 0.0;
            let mut jc = 0.0;
            for b in 0..n {
                let identity = if a == b { 1.0 } else { 0.0 };
                wk += (identity - h * gamma * problem.jacobian[a][b]) * k[i * n + b];
                jc += problem.jacobian[a][b] * coupled[b];
            }
            out[i * n + a] = wk - h * jc - base[a] - h * problem.q[a] * delta[a] * delta[a];
        }
    }
    Ok(out)
}

/// `D_K R(K) v`.
fn residual_jacobian_action(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    k: &[f64],
    v: &[f64],
) -> Vec<f64> {
    let (s, n) = (target.stages(), problem.dimension());
    let (h, gamma) = (problem.h, target.gamma);
    let mut out = vec![0.0; s * n];
    for i in 0..s {
        let (coupled_v, delta_v) = coupled_and_delta(target, v, i, n);
        let (_, delta_k) = coupled_and_delta(target, k, i, n);
        for a in 0..n {
            let mut wv = 0.0;
            let mut jc = 0.0;
            for b in 0..n {
                let identity = if a == b { 1.0 } else { 0.0 };
                wv += (identity - h * gamma * problem.jacobian[a][b]) * v[i * n + b];
                jc += problem.jacobian[a][b] * coupled_v[b];
            }
            out[i * n + a] = wv - h * jc - 2.0 * h * problem.q[a] * delta_k[a] * delta_v[a];
        }
    }
    out
}

/// `sum_{j<i} mid(L*_ij) x_j` and `sum_{j<i} alpha_ij x_j` for stage `i`.
fn coupled_and_delta(target: &StageTarget, x: &[f64], i: usize, n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut coupled = vec![0.0; n];
    let mut delta = vec![0.0; n];
    for j in 0..i {
        let l = 0.5 * (target.coupling_rows[i][j].lo + target.coupling_rows[i][j].hi);
        let alpha = target.alpha_rows[i][j];
        for a in 0..n {
            coupled[a] += l * x[j * n + a];
            delta[a] += alpha * x[j * n + a];
        }
    }
    (coupled, delta)
}

fn check_shapes(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    k: &[f64],
) -> CoreResult<(usize, usize)> {
    let (s, n) = (target.stages(), problem.dimension());
    if n == 0
        || problem.q.len() != n
        || problem.jacobian.len() != n
        || problem.jacobian.iter().any(|row| row.len() != n)
        || k.len() != s * n
    {
        return Err(CoreError::Dimension(format!(
            "stage chart: expected {s} stages of dimension {n} ({} values), got {}",
            s * n,
            k.len()
        )));
    }
    Ok((s, n))
}

fn inf_norm(v: &[f64]) -> f64 {
    v.iter().fold(0.0_f64, |m, x| m.max(x.abs()))
}

/// Newton on `R(Psi(Z)) = 0` from `z0`; see the module documentation. At
/// most `max_iterations` Newton steps; `tolerance` is relative to
/// `1 + ||R(Psi(z0))||_inf`.
pub fn stage_chart_candidate(
    target: &StageTarget,
    problem: &QuadraticStageProblem,
    chart: &dyn StageChart,
    z0: &[f64],
    max_iterations: usize,
    tolerance: f64,
) -> CoreResult<ChartCandidate> {
    let (s, n) = check_shapes(target, problem, z0)?;
    if !(tolerance.is_finite() && tolerance > 0.0) {
        return Err(CoreError::InvalidInput(
            "stage chart: the tolerance must be finite and positive".into(),
        ));
    }
    if !z0.iter().all(|v| v.is_finite()) {
        return Err(CoreError::InvalidInput(
            "stage chart: non-finite start".into(),
        ));
    }
    if !chart.contains(z0) {
        return Err(CoreError::InvalidInput(format!(
            "stage chart {}: the start lies outside the chart's domain",
            chart.name()
        )));
    }
    let m = s * n;
    let mut work = ChartWork::default();
    let mut z = z0.to_vec();
    let mut k = chart.forward(&z)?;
    work.chart_forward += 1;
    let mut r = stage_residual(target, problem, &k)?;
    work.residual_evaluations += 1;
    let limit = tolerance * (1.0 + inf_norm(&r));
    let finish = |z_stages: &[f64], status, iterations, residual: f64, work| ChartCandidate {
        chart: chart.name().into(),
        stages: z_stages.chunks(n).map(<[f64]>::to_vec).collect(),
        status,
        iterations,
        chart_residual_inf: residual,
        work,
    };
    let mut iterations = 0;
    loop {
        let norm = inf_norm(&r);
        if !(norm.is_finite() && k.iter().all(|v| v.is_finite())) {
            return Ok(finish(&k, ChartStatus::NonFinite, iterations, norm, work));
        }
        if norm <= limit {
            return Ok(finish(&k, ChartStatus::Converged, iterations, norm, work));
        }
        if iterations >= max_iterations {
            return Ok(finish(
                &k,
                ChartStatus::NotConverged,
                iterations,
                norm,
                work,
            ));
        }
        // D F = D_K R D_Z Psi, column by column.
        let mut jacobian = DenseMatrix::zeros(m, m);
        let mut unit = vec![0.0; m];
        for column in 0..m {
            unit[column] = 1.0;
            let dk = chart.jvp(&z, &unit)?;
            let dr = residual_jacobian_action(target, problem, &k, &dk);
            unit[column] = 0.0;
            work.chart_jvps += 1;
            work.residual_jacobian_actions += 1;
            for (row, value) in dr.iter().enumerate() {
                jacobian[(row, column)] = *value;
            }
        }
        work.factorizations += 1;
        work.factorization_order = m as u64;
        let step = match LuFactorization::new(&jacobian).and_then(|lu| {
            let rhs: Vec<f64> = r.iter().map(|v| -v).collect();
            lu.solve(&rhs)
        }) {
            Ok(step) if step.iter().all(|v| v.is_finite()) => step,
            Ok(_) => return Ok(finish(&k, ChartStatus::NonFinite, iterations, norm, work)),
            Err(_) => {
                return Ok(finish(
                    &k,
                    ChartStatus::SingularChart,
                    iterations,
                    norm,
                    work,
                ));
            }
        };
        let next: Vec<f64> = z.iter().zip(&step).map(|(a, b)| a + b).collect();
        iterations += 1;
        if !chart.contains(&next) {
            return Ok(finish(&k, ChartStatus::DomainExit, iterations, norm, work));
        }
        z = next;
        k = chart.forward(&z)?;
        work.chart_forward += 1;
        r = stage_residual(target, problem, &k)?;
        work.residual_evaluations += 1;
    }
}
