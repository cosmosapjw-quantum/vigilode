use std::sync::Arc;

use rodas5p_core::{
    ClosureOperator, CoreError, CoreResult, DenseMatrix, LinearOperator, WorkCounters,
    dense_phi_action, safe_l2,
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
        if !(self.relative_tolerance >= 0.0
            && self.relative_tolerance.is_finite()
            && self.absolute_tolerance >= 0.0
            && self.absolute_tolerance.is_finite())
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


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FusedOrthogonalization {
    FullMgs,
    Incomplete { length: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedPhiKrylovConfig {
    pub minimum_dimension: usize,
    pub maximum_dimension: usize,
    pub dimension_increment: usize,
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub orthogonalization: FusedOrthogonalization,
    pub maximum_substeps: usize,
}

impl Default for FusedPhiKrylovConfig {
    fn default() -> Self {
        Self {
            minimum_dimension: 4,
            maximum_dimension: 24,
            dimension_increment: 2,
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 1.0e-13,
            orthogonalization: FusedOrthogonalization::FullMgs,
            maximum_substeps: 16,
        }
    }
}

impl FusedPhiKrylovConfig {
    fn validate(self, dimension: usize) -> CoreResult<Self> {
        if self.minimum_dimension == 0
            || self.maximum_dimension < self.minimum_dimension
            || self.dimension_increment == 0
            || self.maximum_substeps == 0
        {
            return Err(CoreError::InvalidInput(
                "invalid fused phi Krylov dimension/substep contract".into(),
            ));
        }
        if !(self.relative_tolerance >= 0.0
            && self.relative_tolerance.is_finite()
            && self.absolute_tolerance >= 0.0
            && self.absolute_tolerance.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "invalid fused phi Krylov tolerance".into(),
            ));
        }
        if matches!(self.orthogonalization, FusedOrthogonalization::Incomplete { length: 0 }) {
            return Err(CoreError::InvalidInput(
                "incomplete orthogonalization length must be positive".into(),
            ));
        }
        let maximum_dimension = self.maximum_dimension.min(dimension.max(1));
        let minimum_dimension = self.minimum_dimension.min(maximum_dimension);
        Ok(Self { minimum_dimension, maximum_dimension, ..self })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedPhiSubstepReport {
    pub substep_index: usize,
    pub krylov_dimension: usize,
    pub converged: bool,
    pub happy_breakdown: bool,
    pub error_estimate: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedPhiActionReport {
    pub scale: f64,
    pub highest_phi_index: usize,
    pub substeps: usize,
    pub converged: bool,
    pub maximum_krylov_dimension: usize,
    pub error_estimate: f64,
    pub action_norm: f64,
    pub value: Vec<f64>,
    pub substep_reports: Vec<FusedPhiSubstepReport>,
}

#[derive(Clone, Copy, Debug)]
pub struct FusedPhiTerm<'a> {
    pub coefficient: f64,
    pub phi_index: usize,
    pub vector: &'a [f64],
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


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FusedExponentialStepReport {
    pub method: String,
    pub y_new: Vec<f64>,
    pub y_embedded: Option<Vec<f64>>,
    pub error_estimate: Option<Vec<f64>>,
    pub logical_critical_depth: usize,
    pub fused_phi_reports: Vec<FusedPhiActionReport>,
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


fn projected_exponential_action(
    basis: &[Vec<f64>],
    hessenberg: &[Vec<f64>],
    beta: f64,
    dimension: usize,
    scale: f64,
    counters: &mut WorkCounters,
) -> CoreResult<Vec<f64>> {
    projected_action(basis, hessenberg, beta, dimension, scale, 0, counters)
}

fn fused_orthogonalize(
    basis: &[Vec<f64>],
    column: usize,
    work: &mut [f64],
    hessenberg: &mut [Vec<f64>],
    mode: FusedOrthogonalization,
    counters: &mut WorkCounters,
) {
    let end = column + 1;
    let start = match mode {
        FusedOrthogonalization::FullMgs => 0,
        FusedOrthogonalization::Incomplete { length } => end.saturating_sub(length),
    };
    let passes = usize::from(matches!(mode, FusedOrthogonalization::FullMgs)) + 1;
    for _ in 0..passes {
        for row in start..end {
            let coefficient = dot(&basis[row], work);
            hessenberg[row][column] += coefficient;
            axpy(-coefficient, &basis[row], work);
            counters.orthogonalization_inner_products += 1;
            counters.orthogonalization_vector_updates += 1;
        }
    }
}

fn krylov_exponential_once(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    vector: &[f64],
    config: FusedPhiKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, FusedPhiSubstepReport)> {
    let dimension = operator.dimension();
    if vector.len() != dimension {
        return Err(CoreError::Dimension(
            "fused Krylov exponential vector shape mismatch".into(),
        ));
    }
    let config = config.validate(dimension)?;
    let beta = safe_l2(vector);
    if beta == 0.0 {
        return Ok((
            vec![0.0; dimension],
            FusedPhiSubstepReport {
                substep_index: 0,
                krylov_dimension: 0,
                converged: true,
                happy_breakdown: true,
                error_estimate: 0.0,
            },
        ));
    }

    let maximum = config.maximum_dimension;
    let mut basis = Vec::with_capacity(maximum + 1);
    basis.push(vector.iter().map(|value| value / beta).collect::<Vec<_>>());
    let mut hessenberg = vec![vec![0.0; maximum]; maximum + 1];
    let mut previous: Option<Vec<f64>> = None;
    let mut latest = vector.to_vec();
    let mut latest_error = f64::INFINITY;
    let mut latest_dimension = 0;
    let mut latest_breakdown = false;
    let breakdown_tolerance = 64.0 * f64::EPSILON.sqrt();

    for column in 0..maximum {
        let mut work = vec![0.0; dimension];
        operator.apply(&basis[column], &mut work)?;
        counters.jvp_calls += 1;
        counters.jvp_vectors += 1;
        counters.phi_krylov_vectors += 1;
        fused_orthogonalize(
            &basis,
            column,
            &mut work,
            &mut hessenberg,
            config.orthogonalization,
            counters,
        );
        let next_norm = safe_l2(&work);
        hessenberg[column + 1][column] = next_norm;
        let column_scale = hessenberg
            .iter()
            .take(column + 1)
            .map(|row| row[column].abs())
            .fold(1.0, f64::max);
        let happy_breakdown = next_norm <= breakdown_tolerance * column_scale;
        if !happy_breakdown && column + 1 < maximum {
            basis.push(work.iter().map(|value| value / next_norm).collect());
        }

        let krylov_dimension = column + 1;
        // A true invariant-subspace breakdown is a valid checkpoint even before the requested
        // minimum dimension; otherwise zero or affine combinations can burn the entire budget.
        let checkpoint = happy_breakdown
            || krylov_dimension == maximum
            || (krylov_dimension >= config.minimum_dimension
                && (krylov_dimension - config.minimum_dimension) % config.dimension_increment == 0);
        if checkpoint {
            let current = projected_exponential_action(
                &basis,
                &hessenberg,
                beta,
                krylov_dimension,
                scale,
                counters,
            )?;
            latest_error = previous.as_ref().map_or(f64::INFINITY, |prior| {
                let difference = current
                    .iter()
                    .zip(prior)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>();
                safe_l2(&difference)
            });
            latest_dimension = krylov_dimension;
            latest_breakdown = happy_breakdown;
            latest = current.clone();
            let threshold = config.absolute_tolerance
                + config.relative_tolerance * safe_l2(&current).max(beta);
            if happy_breakdown || krylov_dimension == dimension || latest_error <= threshold {
                return Ok((
                    current,
                    FusedPhiSubstepReport {
                        substep_index: 0,
                        krylov_dimension,
                        converged: true,
                        happy_breakdown,
                        error_estimate: if latest_error.is_finite() { latest_error } else { 0.0 },
                    },
                ));
            }
            previous = Some(current);
        }
        if happy_breakdown {
            break;
        }
    }
    Ok((
        latest,
        FusedPhiSubstepReport {
            substep_index: 0,
            krylov_dimension: latest_dimension,
            converged: false,
            happy_breakdown: latest_breakdown,
            error_estimate: latest_error,
        },
    ))
}

fn augmented_fused_operator(
    operator: Arc<dyn LinearOperator>,
    vectors: &[Vec<f64>],
) -> CoreResult<(Arc<dyn LinearOperator>, Vec<f64>, usize)> {
    if vectors.is_empty() {
        return Err(CoreError::InvalidInput(
            "fused phi action requires at least b0".into(),
        ));
    }
    let n = operator.dimension();
    if vectors.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension(
            "fused phi action vector shape mismatch".into(),
        ));
    }
    let p = vectors.len() - 1;
    if p == 0 {
        return Ok((operator, vectors[0].clone(), n));
    }
    let owned = vectors.to_vec();
    let augmented_dimension = n + p;
    let augmented = Arc::new(ClosureOperator::new(augmented_dimension, move |input, output| {
        if input.len() != augmented_dimension || output.len() != augmented_dimension {
            return Err(CoreError::Dimension(
                "augmented fused operator shape mismatch".into(),
            ));
        }
        operator.apply(&input[..n], &mut output[..n])?;
        for column in 0..p {
            let coefficient = input[n + column];
            if coefficient != 0.0 {
                axpy(coefficient, &owned[p - column], &mut output[..n]);
            }
        }
        for row in 0..p {
            output[n + row] = if row + 1 < p { input[n + row + 1] } else { 0.0 };
        }
        Ok(())
    })) as Arc<dyn LinearOperator>;
    let mut start = vec![0.0; augmented_dimension];
    start[..n].copy_from_slice(&vectors[0]);
    start[augmented_dimension - 1] = 1.0;
    Ok((augmented, start, n))
}

/// One augmented matrix-free Krylov process for a linear combination of phi functions.
///
/// The `vectors` input follows the scaled convention
/// `b0 + scale*phi1(scale*A)b1 + ... + scale^p*phi_p(scale*A)bp`.
pub fn fused_phi_action(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    vectors: &[Vec<f64>],
    config: FusedPhiKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<FusedPhiActionReport> {
    if !scale.is_finite()
        || vectors.is_empty()
        || !vectors.iter().flat_map(|v| v.iter()).all(|value| value.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "invalid fused phi-action input".into(),
        ));
    }
    counters.phi_actions += 1;
    let n = operator.dimension();
    if vectors.iter().any(|vector| vector.len() != n) {
        return Err(CoreError::Dimension(
            "fused phi-action vector shape mismatch".into(),
        ));
    }
    if vectors.iter().all(|vector| safe_l2(vector) == 0.0) {
        return Ok(FusedPhiActionReport {
            scale,
            highest_phi_index: vectors.len() - 1,
            substeps: 0,
            converged: true,
            maximum_krylov_dimension: 0,
            error_estimate: 0.0,
            action_norm: 0.0,
            value: vec![0.0; n],
            substep_reports: Vec::new(),
        });
    }
    let (augmented, initial, physical_dimension) = augmented_fused_operator(operator, vectors)?;
    let config = config.validate(augmented.dimension())?;
    let mut substeps = 1usize;
    loop {
        let delta = scale / substeps as f64;
        let mut state = initial.clone();
        let mut reports = Vec::with_capacity(substeps);
        let mut total_error = 0.0;
        let mut maximum_dimension = 0;
        let mut completed = true;
        for index in 0..substeps {
            let (next, mut report) = krylov_exponential_once(
                augmented.clone(),
                delta,
                &state,
                config,
                counters,
            )?;
            report.substep_index = index;
            maximum_dimension = maximum_dimension.max(report.krylov_dimension);
            if report.error_estimate.is_finite() {
                total_error += report.error_estimate;
            }
            completed &= report.converged;
            reports.push(report);
            state = next;
            if !completed {
                break;
            }
        }
        if completed {
            let value = state[..physical_dimension].to_vec();
            return Ok(FusedPhiActionReport {
                scale,
                highest_phi_index: vectors.len() - 1,
                substeps,
                converged: true,
                maximum_krylov_dimension: maximum_dimension,
                error_estimate: total_error,
                action_norm: safe_l2(&value),
                value,
                substep_reports: reports,
            });
        }
        if substeps >= config.maximum_substeps {
            let value = state[..physical_dimension].to_vec();
            return Ok(FusedPhiActionReport {
                scale,
                highest_phi_index: vectors.len() - 1,
                substeps,
                converged: false,
                maximum_krylov_dimension: maximum_dimension,
                error_estimate: total_error,
                action_norm: safe_l2(&value),
                value,
                substep_reports: reports,
            });
        }
        counters.phi_restarts += 1;
        substeps = (substeps * 2).min(config.maximum_substeps);
    }
}

/// Fused unscaled combination `sum coefficient*phi_k(scale*A)vector`.
pub fn fused_phi_linear_combination(
    operator: Arc<dyn LinearOperator>,
    scale: f64,
    terms: &[FusedPhiTerm<'_>],
    config: FusedPhiKrylovConfig,
    counters: &mut WorkCounters,
) -> CoreResult<FusedPhiActionReport> {
    if scale == 0.0 {
        return Err(CoreError::InvalidInput(
            "fused unscaled phi combination requires nonzero scale".into(),
        ));
    }
    let n = operator.dimension();
    let highest = terms.iter().map(|term| term.phi_index).max().unwrap_or(0);
    let mut vectors = vec![vec![0.0; n]; highest + 1];
    for term in terms {
        if term.vector.len() != n || !term.coefficient.is_finite() {
            return Err(CoreError::Dimension(
                "fused phi combination term shape mismatch".into(),
            ));
        }
        let divisor = scale.powi(term.phi_index as i32);
        if divisor == 0.0 || !divisor.is_finite() {
            return Err(CoreError::NonFinite(
                "fused phi combination scale power is invalid".into(),
            ));
        }
        axpy(term.coefficient / divisor, term.vector, &mut vectors[term.phi_index]);
    }
    fused_phi_action(operator, scale, &vectors, config, counters)
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

    // Dependency level 1: the U2 path and the endpoint base action are independent.
    // Running both inside the same local execution context makes the published three-level
    // dependency graph an actual code-path property rather than metadata only.
    let level_one_ids = [0_usize, 1_usize];
    let level_one = execution.map_ordered(&level_one_ids, |task_id| {
        let mut local = WorkCounters::default();
        if *task_id == 0 {
            let phi_c2 = require_action(krylov_phi_action(
                operator.clone(),
                tableau.c2 * h,
                1,
                &f0,
                config,
                &mut local,
            )?)?;
            let u2 = base_stage(y, h, tableau.c2, &phi_c2.value);
            let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut local)?;
            Ok((Some(d2), Some(phi_c2), None, local))
        } else {
            let phi_full = require_action(krylov_phi_action(
                operator.clone(),
                h,
                1,
                &f0,
                config,
                &mut local,
            )?)?;
            Ok((None, None, Some(phi_full), local))
        }
    })?;
    for result in &level_one {
        counters.accumulate(result.3);
    }
    let d2 = level_one[0]
        .0
        .clone()
        .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 level-one U2 result missing".into()))?;
    let phi_c2 = level_one[0]
        .1
        .clone()
        .ok_or_else(|| CoreError::InvalidInput("pexprb54s4 level-one c2 action missing".into()))?;
    let phi_full = level_one[1].2.clone().ok_or_else(|| {
        CoreError::InvalidInput("pexprb54s4 level-one endpoint action missing".into())
    })?;

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


fn require_fused(report: FusedPhiActionReport) -> CoreResult<FusedPhiActionReport> {
    if report.converged {
        Ok(report)
    } else {
        Err(CoreError::LinearSolve(format!(
            "fused phi action failed after {} substeps at dimension {} (estimate {})",
            report.substeps, report.maximum_krylov_dimension, report.error_estimate
        )))
    }
}

fn add_scaled_state(y: &[f64], h: f64, action: &[f64]) -> Vec<f64> {
    y.iter().zip(action).map(|(state, delta)| state + h * delta).collect()
}

pub fn exprb2_fused_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
) -> CoreResult<FusedExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut work = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut work)?;
    let operator = problem.linearize_matrix_free(t, y)?;
    let action = require_fused(fused_phi_linear_combination(
        operator,
        h,
        &[FusedPhiTerm { coefficient: 1.0, phi_index: 1, vector: &f0 }],
        config,
        &mut work,
    )?)?;
    Ok(FusedExponentialStepReport {
        method: "exprb2-fused".into(),
        y_new: add_scaled_state(y, h, &action.value),
        y_embedded: None,
        error_estimate: None,
        logical_critical_depth: 1,
        fused_phi_reports: vec![action],
        work,
    })
}

pub fn exprb43_fused_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
) -> CoreResult<FusedExponentialStepReport> {
    validate_problem(problem, y)?;
    let mut work = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut work)?;
    let operator = problem.linearize_matrix_free(t, y)?;

    let u2_action = require_fused(fused_phi_linear_combination(
        operator.clone(),
        0.5 * h,
        &[FusedPhiTerm { coefficient: 0.5, phi_index: 1, vector: &f0 }],
        config,
        &mut work,
    )?)?;
    let u2 = add_scaled_state(y, h, &u2_action.value);
    let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut work)?;

    let u3_action = require_fused(fused_phi_linear_combination(
        operator.clone(),
        h,
        &[
            FusedPhiTerm { coefficient: 1.0, phi_index: 1, vector: &f0 },
            FusedPhiTerm { coefficient: 1.0, phi_index: 1, vector: &d2 },
        ],
        config,
        &mut work,
    )?)?;
    let u3 = add_scaled_state(y, h, &u3_action.value);
    let d3 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u3, &mut work)?;

    let endpoint_ids = [0usize, 1usize];
    let endpoints = execution.map_ordered(&endpoint_ids, |id| {
        let mut local = WorkCounters::default();
        let terms = if *id == 0 {
            vec![
                FusedPhiTerm { coefficient: 1.0, phi_index: 1, vector: &f0 },
                FusedPhiTerm { coefficient: 16.0, phi_index: 3, vector: &d2 },
                FusedPhiTerm { coefficient: -48.0, phi_index: 4, vector: &d2 },
                FusedPhiTerm { coefficient: -2.0, phi_index: 3, vector: &d3 },
                FusedPhiTerm { coefficient: 12.0, phi_index: 4, vector: &d3 },
            ]
        } else {
            vec![
                FusedPhiTerm { coefficient: 1.0, phi_index: 1, vector: &f0 },
                FusedPhiTerm { coefficient: 16.0, phi_index: 3, vector: &d2 },
                FusedPhiTerm { coefficient: -2.0, phi_index: 3, vector: &d3 },
            ]
        };
        let action = require_fused(fused_phi_linear_combination(
            operator.clone(), h, &terms, config, &mut local,
        )?)?;
        Ok((action, local))
    })?;
    for (_, local) in &endpoints { work.accumulate(*local); }
    let y_new = add_scaled_state(y, h, &endpoints[0].0.value);
    let y_embedded = add_scaled_state(y, h, &endpoints[1].0.value);
    let error_estimate = y_new.iter().zip(&y_embedded).map(|(a,b)| a-b).collect();
    Ok(FusedExponentialStepReport {
        method: "exprb43-fused".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        fused_phi_reports: vec![u2_action, u3_action, endpoints[0].0.clone(), endpoints[1].0.clone()],
        work,
    })
}

pub fn pexprb54s4_fused_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: FusedPhiKrylovConfig,
    execution: &ParallelExecution,
) -> CoreResult<FusedExponentialStepReport> {
    validate_problem(problem, y)?;
    let tableau = pexprb54s4_tableau();
    let mut work = WorkCounters::default();
    let f0 = problem.eval_rhs(t, y, &mut work)?;
    let operator = problem.linearize_matrix_free(t, y)?;

    // Dependency level 1: U2.
    let u2_action = require_fused(fused_phi_linear_combination(
        operator.clone(),
        tableau.c2 * h,
        &[FusedPhiTerm { coefficient: tableau.c2, phi_index: 1, vector: &f0 }],
        config,
        &mut work,
    )?)?;
    let u2 = add_scaled_state(y, h, &u2_action.value);
    let d2 = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &u2, &mut work)?;

    // Dependency level 2: U3 and U4 are independent once D2 is known.
    let stage_ids = [3usize, 4usize];
    let stages = execution.map_ordered(&stage_ids, |id| {
        let mut local = WorkCounters::default();
        let (c, a) = if *id == 3 {
            (tableau.c3, tableau.a32_phi3)
        } else {
            (tableau.c4, tableau.a42_phi3)
        };
        let action = require_fused(fused_phi_linear_combination(
            operator.clone(),
            c * h,
            &[
                FusedPhiTerm { coefficient: c, phi_index: 1, vector: &f0 },
                FusedPhiTerm { coefficient: a, phi_index: 3, vector: &d2 },
            ],
            config,
            &mut local,
        )?)?;
        let stage = add_scaled_state(y, h, &action.value);
        let remainder = nonlinear_remainder(problem, operator.as_ref(), t, y, &f0, &stage, &mut local)?;
        Ok((action, remainder, local))
    })?;
    for (_, _, local) in &stages { work.accumulate(*local); }
    let d3 = stages[0].1.clone();
    let d4 = stages[1].1.clone();

    // Dependency level 3: main and embedded endpoints are independent.
    let endpoint_ids = [0usize, 1usize];
    let endpoints = execution.map_ordered(&endpoint_ids, |id| {
        let mut local = WorkCounters::default();
        let terms = if *id == 0 {
            vec![
                FusedPhiTerm { coefficient: 1.0, phi_index: 1, vector: &f0 },
                FusedPhiTerm { coefficient: tableau.b3_phi3, phi_index: 3, vector: &d3 },
                FusedPhiTerm { coefficient: tableau.b3_phi4, phi_index: 4, vector: &d3 },
                FusedPhiTerm { coefficient: tableau.b4_phi3, phi_index: 3, vector: &d4 },
                FusedPhiTerm { coefficient: tableau.b4_phi4, phi_index: 4, vector: &d4 },
            ]
        } else {
            vec![
                FusedPhiTerm { coefficient: 1.0, phi_index: 1, vector: &f0 },
                FusedPhiTerm { coefficient: tableau.embedded_b2_phi3, phi_index: 3, vector: &d2 },
                FusedPhiTerm { coefficient: tableau.embedded_b2_phi4, phi_index: 4, vector: &d2 },
                FusedPhiTerm { coefficient: tableau.embedded_b3_phi3, phi_index: 3, vector: &d3 },
                FusedPhiTerm { coefficient: tableau.embedded_b3_phi4, phi_index: 4, vector: &d3 },
                FusedPhiTerm { coefficient: tableau.embedded_b4_phi3, phi_index: 3, vector: &d4 },
                FusedPhiTerm { coefficient: tableau.embedded_b4_phi4, phi_index: 4, vector: &d4 },
            ]
        };
        let action = require_fused(fused_phi_linear_combination(
            operator.clone(), h, &terms, config, &mut local,
        )?)?;
        Ok((action, local))
    })?;
    for (_, local) in &endpoints { work.accumulate(*local); }
    let y_new = add_scaled_state(y, h, &endpoints[0].0.value);
    let y_embedded = add_scaled_state(y, h, &endpoints[1].0.value);
    let error_estimate = y_new.iter().zip(&y_embedded).map(|(a,b)| a-b).collect();
    Ok(FusedExponentialStepReport {
        method: "pexprb54s4-fused".into(),
        y_new,
        y_embedded: Some(y_embedded),
        error_estimate: Some(error_estimate),
        logical_critical_depth: 3,
        fused_phi_reports: vec![
            u2_action,
            stages[0].0.clone(),
            stages[1].0.clone(),
            endpoints[0].0.clone(),
            endpoints[1].0.clone(),
        ],
        work,
    })
}
