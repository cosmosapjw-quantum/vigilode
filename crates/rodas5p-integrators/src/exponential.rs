use std::sync::Arc;

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearOperator, WorkCounters, dense_phi_action, safe_l2,
};
use serde::{Deserialize, Serialize};

use crate::{OdeProblem, ParallelExecution};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialKrylovConfig {
    pub minimum_dimension: usize,
    pub maximum_dimension: usize,
    pub dimension_increment: usize,
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub reorthogonalize: bool,
}

impl Default for ExponentialKrylovConfig {
    fn default() -> Self {
        Self {
            minimum_dimension: 4,
            maximum_dimension: 24,
            dimension_increment: 2,
            relative_tolerance: 1.0e-11,
            absolute_tolerance: 1.0e-13,
            reorthogonalize: true,
        }
    }
}

impl ExponentialKrylovConfig {
    fn validate(self, n: usize) -> CoreResult<Self> {
        if self.minimum_dimension == 0
            || self.maximum_dimension < self.minimum_dimension
            || self.dimension_increment == 0
            || self.maximum_dimension == 0
        {
            return Err(CoreError::InvalidInput(
                "invalid exponential Krylov dimension contract".into(),
            ));
        }
        if !(self.relative_tolerance >= 0.0 && self.relative_tolerance.is_finite())
            || !(self.absolute_tolerance >= 0.0 && self.absolute_tolerance.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "invalid exponential Krylov tolerance".into(),
            ));
        }
        let maximum_dimension = self.maximum_dimension.min(n.max(1));
        let minimum_dimension = self.minimum_dimension.min(maximum_dimension);
        Ok(Self {
            minimum_dimension,
            maximum_dimension,
            ..self
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhiActionReport {
    pub phi_index: usize,
    pub scale: f64,
    pub krylov_dimension: usize,
    pub converged: bool,
    pub happy_breakdown: bool,
    pub error_estimate: f64,
    pub action_norm: f64,
    pub value: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExponentialStepReport {
    pub method: String,
    pub y_new: Vec<f64>,
    pub y_embedded: Option<Vec<f64>>,
    pub error_estimate: Option<Vec<f64>>,
    pub logical_critical_depth: usize,
    pub phi_reports: Vec<PhiActionReport>,
    pub work: WorkCounters,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pexprb54s4Tableau {
    pub c2: f64,
    pub c3: f64,
    pub c4: f64,
    pub a32_phi3: f64,
    pub a42_phi3: f64,
    pub a43: f64,
    pub b3_phi3: f64,
    pub b3_phi4: f64,
    pub b4_phi3: f64,
    pub b4_phi4: f64,
    pub embedded_b2_phi3: f64,
    pub embedded_b2_phi4: f64,
    pub embedded_b3_phi3: f64,
    pub embedded_b3_phi4: f64,
    pub embedded_b4_phi3: f64,
    pub embedded_b4_phi4: f64,
}

pub fn pexprb54s4_tableau() -> Pexprb54s4Tableau {
    Pexprb54s4Tableau {
        c2: 1.0 / 4.0,
        c3: 1.0 / 2.0,
        c4: 9.0 / 10.0,
        a32_phi3: 4.0,
        a42_phi3: 2916.0 / 125.0,
        a43: 0.0,
        b3_phi3: 18.0,
        b3_phi4: -60.0,
        b4_phi3: -250.0 / 81.0,
        b4_phi4: 500.0 / 27.0,
        embedded_b2_phi3: 64.0,
        embedded_b2_phi4: -60.0,
        embedded_b3_phi3: -8.0,
        embedded_b3_phi4: -285.0 / 8.0,
        embedded_b4_phi3: 0.0,
        embedded_b4_phi4: 125.0 / 8.0,
    }
}

fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(a, b)| a * b).sum()
}

fn axpy(alpha: f64, x: &[f64], y: &mut [f64]) {
    for (value, source) in y.iter_mut().zip(x) {
        *value += alpha * source;
    }
}

fn projected_action(
    basis: &[Vec<f64>],
    hessenberg: &[Vec<f64>],
    beta: f64,
    dimension: usize,
    scale: f64,
    phi_index: usize,
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    let mut h = DenseMatrix::zeros(dimension, dimension);
    for i in 0..dimension {
        for j in 0..dimension {
            h[(i, j)] = hessenberg[i][j];
        }
    }
    let mut reduced_input = vec![0.0; dimension];
    reduced_input[0] = beta;
    let reduced = dense_phi_action(&h, scale, phi_index, &reduced_input)?;
    counters.phi_projected_exponentials += 1;
    counters.phi_dense_oracle_calls += 1;
    let mut out = vec![0.0; basis[0].len()];
    for (coefficient, vector) in reduced.iter().zip(&basis[..dimension]) {
        axpy(*coefficient, vector, &mut out);
    }
    Ok(out)
}

/// Matrix-free Arnoldi approximation of `phi_k(scale*A) v`.
///
/// The current G2 foundation uses full modified Gram--Schmidt with optional reorthogonalization
/// and a nested-dimension convergence diagnostic.  It is a counted correctness reference, not yet
/// KIOPS: incomplete orthogonalization, adaptive substepping and restart remain a later node.
pub fn krylov_phi_action(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    phi_index: usize,
    vector: &[f64],
    config: ExponentialKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<PhiActionReport> {
    let n = operator.dimension();
    if vector.len() != n {
        return Err(CoreError::Dimension(
            "Krylov phi-action vector shape mismatch".into(),
        ));
    }
    if !scale.is_finite() || !vector.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "Krylov phi-action input contains NaN/Inf".into(),
        ));
    }
    let config = config.validate(n)?;
    counters.phi_actions += 1;
    let beta = safe_l2(vector);
    if beta == 0.0 {
        return Ok(PhiActionReport {
            phi_index,
            scale,
            krylov_dimension: 0,
            converged: true,
            happy_breakdown: true,
            error_estimate: 0.0,
            action_norm: 0.0,
            value: vec![0.0; n],
        });
    }

    let maximum = config.maximum_dimension;
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(maximum + 1);
    basis.push(vector.iter().map(|value| value / beta).collect());
    let mut hessenberg = vec![vec![0.0; maximum]; maximum + 1];
    let mut previous: Option<Vec<f64>> = None;
    let mut latest = vec![0.0; n];
    let mut latest_error = f64::INFINITY;
    let mut latest_dimension = 0;
    let mut happy_breakdown = false;
    let breakdown_tolerance = 64.0 * f64::EPSILON.sqrt();

    for column in 0..maximum {
        let mut work = vec![0.0; n];
        operator.apply(&basis[column], &mut work)?;
        counters.jvp_calls += 1;
        counters.jvp_vectors += 1;
        counters.phi_krylov_vectors += 1;

        for (row, basis_vector) in basis.iter().take(column + 1).enumerate() {
            let coefficient = dot(basis_vector, &work);
            hessenberg[row][column] += coefficient;
            axpy(-coefficient, basis_vector, &mut work);
            counters.orthogonalization_inner_products += 1;
            counters.orthogonalization_vector_updates += 1;
        }
        if config.reorthogonalize {
            for (row, basis_vector) in basis.iter().take(column + 1).enumerate() {
                let coefficient = dot(basis_vector, &work);
                hessenberg[row][column] += coefficient;
                axpy(-coefficient, basis_vector, &mut work);
                counters.orthogonalization_inner_products += 1;
                counters.orthogonalization_vector_updates += 1;
            }
        }
        let next_norm = safe_l2(&work);
        hessenberg[column + 1][column] = next_norm;
        let scale_norm = hessenberg
            .iter()
            .take(column + 1)
            .map(|row| row[column].abs())
            .fold(1.0, f64::max);
        happy_breakdown = next_norm <= breakdown_tolerance * scale_norm;
        if !happy_breakdown && column + 1 < maximum {
            basis.push(work.iter().map(|value| value / next_norm).collect());
        }

        let dimension = column + 1;
        let checkpoint = dimension >= config.minimum_dimension
            && ((dimension - config.minimum_dimension) % config.dimension_increment == 0
                || dimension == maximum
                || happy_breakdown);
        if checkpoint {
            let current = projected_action(
                &basis,
                &hessenberg,
                beta,
                dimension,
                scale,
                phi_index,
                counters,
            )?;
            latest_error = previous.as_ref().map_or(f64::INFINITY, |prior| {
                let difference: Vec<f64> = current.iter().zip(prior).map(|(a, b)| a - b).collect();
                safe_l2(&difference)
            });
            latest_dimension = dimension;
            latest = current.clone();
            let threshold =
                config.absolute_tolerance + config.relative_tolerance * safe_l2(&current).max(beta);
            let full_space = dimension == n;
            if happy_breakdown || full_space || latest_error <= threshold {
                return Ok(PhiActionReport {
                    phi_index,
                    scale,
                    krylov_dimension: dimension,
                    converged: true,
                    happy_breakdown,
                    error_estimate: if latest_error.is_finite() {
                        latest_error
                    } else {
                        0.0
                    },
                    action_norm: safe_l2(&current),
                    value: current,
                });
            }
            previous = Some(current);
        }
        if happy_breakdown {
            break;
        }
    }

    Ok(PhiActionReport {
        phi_index,
        scale,
        krylov_dimension: latest_dimension,
        converged: false,
        happy_breakdown,
        error_estimate: latest_error,
        action_norm: safe_l2(&latest),
        value: latest,
    })
}

fn validate_problem(problem: &OdeProblem, y: &[f64]) -> CoreResult<()> {
    if y.len() != problem.dimension {
        return Err(CoreError::Dimension(
            "exponential step state shape mismatch".into(),
        ));
    }
    if !problem.autonomous {
        return Err(CoreError::InvalidInput(
            "G2 foundation currently supports autonomous problems only".into(),
        ));
    }
    if problem.mass_matrix.is_some() {
        return Err(CoreError::InvalidInput(
            "G2 foundation currently supports identity mass only".into(),
        ));
    }
    Ok(())
}

fn require_action(report: PhiActionReport) -> CoreResult<PhiActionReport> {
    if report.converged {
        Ok(report)
    } else {
        Err(CoreError::LinearSolve(format!(
            "phi_{} Krylov action failed to converge at dimension {} (estimate {})",
            report.phi_index, report.krylov_dimension, report.error_estimate
        )))
    }
}

fn phi_combination(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    terms: &[(f64, usize, &[f64])],
    config: ExponentialKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, Vec<PhiActionReport>)> {
    let n = operator.dimension();
    let mut value = vec![0.0; n];
    let mut reports = Vec::with_capacity(terms.len());
    for &(coefficient, phi_index, vector) in terms {
        if coefficient == 0.0 {
            continue;
        }
        let report = require_action(krylov_phi_action(
            operator.clone(),
            scale,
            phi_index,
            vector,
            config,
            counters,
        )?)?;
        axpy(coefficient, &report.value, &mut value);
        reports.push(report);
    }
    Ok((value, reports))
}

fn nonlinear_remainder(
    problem: &OdeProblem,
    operator: &dyn LinearOperator,
    t: f64,
    y: &[f64],
    f0: &[f64],
    stage: &[f64],
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    let f_stage = problem.eval_rhs(t, stage, counters)?;
    let increment: Vec<f64> = stage.iter().zip(y).map(|(a, b)| a - b).collect();
    let mut linear = vec![0.0; increment.len()];
    operator.apply(&increment, &mut linear)?;
    counters.jvp_calls += 1;
    counters.jvp_vectors += 1;
    Ok(f_stage
        .iter()
        .zip(f0)
        .zip(linear)
        .map(|((stage_value, base_value), linear_value)| stage_value - base_value - linear_value)
        .collect())
}

fn base_stage(y: &[f64], h: f64, c: f64, phi1: &[f64]) -> Vec<f64> {
    y.iter()
        .zip(phi1)
        .map(|(state, action)| state + c * h * action)
        .collect()
}

pub fn exprb2_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: ExponentialKrylovConfig,
) -> CoreResult<ExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut counters = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut counters)?;
    let operator = problem.linearize_matrix_free(t, y)?;
    let phi1 = require_action(krylov_phi_action(
        operator,
        h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    let y_new: Vec<f64> = y
        .iter()
        .zip(&phi1.value)
        .map(|(state, action)| state + h * action)
        .collect();
    Ok(ExponentialStepReport {
        method: "exprb2".into(),
        y_new,
        y_embedded: None,
        error_estimate: None,
        logical_critical_depth: 1,
        phi_reports: vec![phi1],
        work: counters,
    })
}

pub fn exprb43_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: ExponentialKrylovConfig,
) -> CoreResult<ExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut counters = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut counters)?;
    let operator = problem.linearize_matrix_free(t, y)?;
    let phi_half = require_action(krylov_phi_action(
        operator.clone(),
        0.5 * h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    let u2 = base_stage(y, h, 0.5, &phi_half.value);
    let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut counters)?;

    let phi_full = require_action(krylov_phi_action(
        operator.clone(),
        h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    let a32 = require_action(krylov_phi_action(
        operator.clone(),
        h,
        1,
        &d2,
        config,
        &mut counters,
    )?)?;
    let mut u3 = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &a32.value, &mut u3);
    let d3 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u3, &mut counters)?;

    let (main_correction, mut main_reports) = phi_combination(
        operator.clone(),
        h,
        &[
            (16.0, 3, &d2),
            (-48.0, 4, &d2),
            (-2.0, 3, &d3),
            (12.0, 4, &d3),
        ],
        config,
        &mut counters,
    )?;
    let (embedded_correction, mut embedded_reports) = phi_combination(
        operator,
        h,
        &[(16.0, 3, &d2), (-2.0, 3, &d3)],
        config,
        &mut counters,
    )?;
    let mut y_new = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &main_correction, &mut y_new);
    let mut y_embedded = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &embedded_correction, &mut y_embedded);
    let error_estimate: Vec<f64> = y_new.iter().zip(&y_embedded).map(|(a, b)| a - b).collect();
    let mut phi_reports = vec![phi_half, phi_full, a32];
    phi_reports.append(&mut main_reports);
    phi_reports.append(&mut embedded_reports);
    Ok(ExponentialStepReport {
        method: "exprb43".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        phi_reports,
        work: counters,
    })
}

pub fn pexprb54s4_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: ExponentialKrylovConfig,
    execution: &ParallelExecution,
) -> CoreResult<ExponentialStepReport> {
    validate_problem(problem, y)?;
    let tableau = pexprb54s4_tableau();
    let mut counters = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut counters)?;
    let operator = problem.linearize_matrix_free(t, y)?;

    let phi_c2 = require_action(krylov_phi_action(
        operator.clone(),
        tableau.c2 * h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    let u2 = base_stage(y, h, tableau.c2, &phi_c2.value);
    let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut counters)?;

    let stage_ids = [3_usize, 4_usize];
    let stage_results = execution.map_ordered(&stage_ids, |stage_id| {
        let mut local = WorkCounters::default();
        let (c, coefficient) = if *stage_id == 3 {
            (tableau.c3, tableau.a32_phi3)
        } else {
            (tableau.c4, tableau.a42_phi3)
        };
        let phi1 = require_action(krylov_phi_action(
            operator.clone(),
            c * h,
            1,
            &f0,
            config,
            &mut local,
        )?)?;
        let phi3 = require_action(krylov_phi_action(
            operator.clone(),
            c * h,
            3,
            &d2,
            config,
            &mut local,
        )?)?;
        let mut stage = base_stage(y, h, c, &phi1.value);
        axpy(h * coefficient, &phi3.value, &mut stage);
        let remainder =
            nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &stage, &mut local)?;
        Ok((stage, remainder, vec![phi1, phi3], local))
    })?;
    let d3 = stage_results[0].1.clone();
    let d4 = stage_results[1].1.clone();
    let mut phi_reports = vec![phi_c2];
    for result in &stage_results {
        counters.accumulate(result.3);
        phi_reports.extend(result.2.clone());
    }

    let endpoint_ids = [0_usize, 1_usize];
    let endpoint_results = execution.map_ordered(&endpoint_ids, |endpoint_id| {
        let mut local = WorkCounters::default();
        let (correction, reports) = if *endpoint_id == 0 {
            phi_combination(
                operator.clone(),
                h,
                &[
                    (tableau.b3_phi3, 3, &d3),
                    (tableau.b3_phi4, 4, &d3),
                    (tableau.b4_phi3, 3, &d4),
                    (tableau.b4_phi4, 4, &d4),
                ],
                config,
                &mut local,
            )?
        } else {
            phi_combination(
                operator.clone(),
                h,
                &[
                    (tableau.embedded_b2_phi3, 3, &d2),
                    (tableau.embedded_b2_phi4, 4, &d2),
                    (tableau.embedded_b3_phi3, 3, &d3),
                    (tableau.embedded_b3_phi4, 4, &d3),
                    (tableau.embedded_b4_phi3, 3, &d4),
                    (tableau.embedded_b4_phi4, 4, &d4),
                ],
                config,
                &mut local,
            )?
        };
        Ok((correction, reports, local))
    })?;
    for result in &endpoint_results {
        counters.accumulate(result.2);
        phi_reports.extend(result.1.clone());
    }
    let phi_full = require_action(krylov_phi_action(
        operator,
        h,
        1,
        &f0,
        config,
        &mut counters,
    )?)?;
    phi_reports.push(phi_full.clone());
    let mut y_new = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &endpoint_results[0].0, &mut y_new);
    let mut y_embedded = base_stage(y, h, 1.0, &phi_full.value);
    axpy(h, &endpoint_results[1].0, &mut y_embedded);
    let error_estimate: Vec<f64> = y_new.iter().zip(&y_embedded).map(|(a, b)| a - b).collect();

    Ok(ExponentialStepReport {
        method: "pexprb54s4".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        phi_reports,
        work: counters,
    })
}
