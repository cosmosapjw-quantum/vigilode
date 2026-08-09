use faer::linalg::solvers::SolveLstsq;
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearOperator, Preconditioner, WorkCounters,
    apply_preconditioner, safe_l2,
};
use serde::{Deserialize, Serialize};

use crate::kernels::{axpy, dot};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockGmresConfig {
    /// Maximum number of block-Arnoldi search directions, counted as vectors rather than calls.
    pub max_basis: usize,
    pub rtol: f64,
    pub atol: f64,
    /// Relative numerical-rank threshold used for initial-RHS and Arnoldi deflation.
    pub rank_tolerance: f64,
}

impl Default for BlockGmresConfig {
    fn default() -> Self {
        Self {
            max_basis: 80,
            rtol: 1.0e-10,
            atol: 1.0e-12,
            rank_tolerance: 1.0e-12,
        }
    }
}

impl BlockGmresConfig {
    fn validate(&self) -> CoreResult<()> {
        if self.max_basis == 0 {
            return Err(CoreError::InvalidInput(
                "block GMRES max_basis must be positive".into(),
            ));
        }
        if !self.rtol.is_finite()
            || !self.atol.is_finite()
            || !self.rank_tolerance.is_finite()
            || self.rtol < 0.0
            || self.atol < 0.0
            || self.rank_tolerance <= 0.0
        {
            return Err(CoreError::InvalidInput(
                "block GMRES tolerances must be finite and nonnegative".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockLinearSolveReport {
    pub solutions: Vec<Vec<f64>>,
    pub converged: bool,
    pub residual_norms: Vec<f64>,
    pub relative_residuals: Vec<f64>,
    pub maximum_residual_norm: f64,
    pub maximum_relative_residual: f64,
    pub initial_block_rank: usize,
    pub final_basis_dimension: usize,
    pub search_directions: usize,
    pub block_iterations: usize,
    pub operator_vectors: u64,
    pub block_operator_calls: u64,
    pub method: String,
}

fn validate_rhs(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs_rows: &[Vec<f64>],
) -> CoreResult<(usize, usize)> {
    let n = op.dimension();
    if n == 0 || pc.dimension() != n {
        return Err(CoreError::Dimension(
            "block GMRES operator/preconditioner dimension mismatch".into(),
        ));
    }
    if rhs_rows.is_empty() || rhs_rows.iter().any(|row| row.len() != n) {
        return Err(CoreError::Dimension(
            "block GMRES RHS rows must be nonempty and rectangular".into(),
        ));
    }
    if !rhs_rows.iter().flatten().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "block GMRES RHS contains NaN/Inf".into(),
        ));
    }
    Ok((n, rhs_rows.len()))
}

fn orthogonalize(
    vector: &mut [f64],
    basis: &[Vec<f64>],
    coefficients: &mut Vec<f64>,
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    coefficients.resize(basis.len(), 0.0);
    coefficients.fill(0.0);
    for _ in 0..2 {
        for (index, direction) in basis.iter().enumerate() {
            let coefficient = dot(direction, vector, counters)?;
            coefficients[index] += coefficient;
            axpy(-coefficient, direction, vector, counters)?;
        }
    }
    Ok(())
}

fn initial_basis(
    preconditioned_rhs: &[Vec<f64>],
    rank_tolerance: f64,
    counters: &mut WorkCounters,
) -> CoreResult<Vec<Vec<f64>>> {
    let mut basis = Vec::<Vec<f64>>::new();
    let mut coefficients = Vec::new();
    for rhs in preconditioned_rhs {
        let rhs_norm = safe_l2(rhs);
        if rhs_norm <= f64::MIN_POSITIVE {
            continue;
        }
        let mut vector = rhs.iter().map(|value| value / rhs_norm).collect::<Vec<_>>();
        orthogonalize(&mut vector, &basis, &mut coefficients, counters)?;
        let independent_norm = safe_l2(&vector);
        if independent_norm > rank_tolerance {
            for value in &mut vector {
                *value /= independent_norm;
            }
            basis.push(vector);
        }
    }
    Ok(basis)
}

fn least_squares_matrix(a: &DenseMatrix, b: &DenseMatrix) -> CoreResult<DenseMatrix> {
    if a.nrows() != b.nrows() {
        return Err(CoreError::Dimension(
            "block GMRES least-squares row mismatch".into(),
        ));
    }
    if a.ncols() == 0 {
        return Ok(DenseMatrix::zeros(0, b.ncols()));
    }
    let fa = a.to_faer();
    let fb = b.to_faer();
    let x = fa.col_piv_qr().solve_lstsq(&fb);
    let mut out = DenseMatrix::zeros(a.ncols(), b.ncols());
    for row in 0..out.nrows() {
        for column in 0..out.ncols() {
            out[(row, column)] = x[(row, column)];
        }
    }
    if out.as_slice().iter().all(|value| value.is_finite()) {
        Ok(out)
    } else {
        Err(CoreError::LinearSolve(
            "block GMRES least-squares solve produced NaN/Inf".into(),
        ))
    }
}

fn apply_left_rows(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    inputs: &[Vec<f64>],
    outputs: &mut [Vec<f64>],
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    op.apply_rows(inputs, outputs)?;
    counters.linear_matvecs += inputs.len() as u64;
    counters.block_matvecs += 1;
    for output in outputs.iter_mut() {
        let raw = output.clone();
        apply_preconditioner(pc, &raw, output, counters)?;
    }
    counters.block_preconditioner_apps += 1;
    Ok(())
}

fn true_residuals(
    op: &dyn LinearOperator,
    rhs_rows: &[Vec<f64>],
    solutions: &[Vec<f64>],
    counters: &mut WorkCounters,
) -> CoreResult<(Vec<f64>, Vec<f64>)> {
    let n = op.dimension();
    let mut applied = vec![vec![0.0; n]; solutions.len()];
    op.apply_rows(solutions, &mut applied)?;
    counters.diagnostic_matvecs += solutions.len() as u64;
    counters.block_matvecs += 1;
    let mut norms = Vec::with_capacity(rhs_rows.len());
    let mut relative = Vec::with_capacity(rhs_rows.len());
    for (rhs, ax) in rhs_rows.iter().zip(applied) {
        let residual = rhs
            .iter()
            .zip(ax)
            .map(|(right, value)| right - value)
            .collect::<Vec<_>>();
        let norm = safe_l2(&residual);
        norms.push(norm);
        relative.push(norm / safe_l2(rhs).max(f64::MIN_POSITIVE));
    }
    Ok((norms, relative))
}

fn projection_solution(
    basis: &[Vec<f64>],
    directions: usize,
    h_columns: &[Vec<f64>],
    preconditioned_rhs: &[Vec<f64>],
    counters: &mut WorkCounters,
) -> CoreResult<Vec<Vec<f64>>> {
    let rows = basis.len();
    let rhs_count = preconditioned_rhs.len();
    let mut h = DenseMatrix::zeros(rows, directions);
    for (column, coefficients) in h_columns.iter().take(directions).enumerate() {
        for (row, &value) in coefficients.iter().enumerate() {
            h[(row, column)] = value;
        }
    }
    let mut projected_rhs = DenseMatrix::zeros(rows, rhs_count);
    for row in 0..rows {
        for column in 0..rhs_count {
            projected_rhs[(row, column)] = dot(&basis[row], &preconditioned_rhs[column], counters)?;
        }
    }
    let coefficients = least_squares_matrix(&h, &projected_rhs)?;
    let n = basis[0].len();
    let mut solutions = vec![vec![0.0; n]; rhs_count];
    for rhs_index in 0..rhs_count {
        for direction in 0..directions {
            let coefficient = coefficients[(direction, rhs_index)];
            if coefficient != 0.0 {
                axpy(
                    coefficient,
                    &basis[direction],
                    &mut solutions[rhs_index],
                    counters,
                )?;
            }
        }
    }
    Ok(solutions)
}

pub fn solve_block_gmres(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs_rows: &[Vec<f64>],
    config: &BlockGmresConfig,
    counters: &mut WorkCounters,
) -> CoreResult<BlockLinearSolveReport> {
    config.validate()?;
    let (n, rhs_count) = validate_rhs(op, pc, rhs_rows)?;
    let before = *counters;

    let mut preconditioned_rhs = vec![vec![0.0; n]; rhs_count];
    for (rhs, output) in rhs_rows.iter().zip(&mut preconditioned_rhs) {
        apply_preconditioner(pc, rhs, output, counters)?;
    }
    counters.block_preconditioner_apps += 1;

    let thresholds = rhs_rows
        .iter()
        .map(|rhs| config.atol.max(config.rtol * safe_l2(rhs)))
        .collect::<Vec<_>>();
    let mut basis = initial_basis(&preconditioned_rhs, config.rank_tolerance, counters)?;
    let initial_block_rank = basis.len();
    if initial_block_rank == 0 {
        let solutions = vec![vec![0.0; n]; rhs_count];
        let (residual_norms, relative_residuals) =
            true_residuals(op, rhs_rows, &solutions, counters)?;
        let converged = residual_norms
            .iter()
            .zip(&thresholds)
            .all(|(residual, threshold)| residual.is_finite() && residual <= threshold);
        if !converged {
            return Err(CoreError::LinearSolve(
                "block GMRES initial block lost a required RHS direction".into(),
            ));
        }
        counters.linear_solves += rhs_count as u64;
        counters.block_linear_solves += 1;
        let delta = counters.delta(before);
        return Ok(BlockLinearSolveReport {
            maximum_residual_norm: residual_norms.iter().copied().fold(0.0_f64, f64::max),
            maximum_relative_residual: relative_residuals.iter().copied().fold(0.0_f64, f64::max),
            solutions,
            converged,
            residual_norms,
            relative_residuals,
            initial_block_rank: 0,
            final_basis_dimension: 0,
            search_directions: 0,
            block_iterations: 0,
            operator_vectors: delta.diagnostic_matvecs,
            block_operator_calls: delta.block_matvecs,
            method: "block-gmres".into(),
        });
    }
    let mut frontier_start = 0usize;
    let mut frontier_end = basis.len();
    let mut h_columns = Vec::<Vec<f64>>::new();
    let mut directions = 0usize;
    let mut block_iterations = 0usize;
    let mut coefficient_scratch = Vec::new();

    loop {
        if directions >= config.max_basis || frontier_start >= frontier_end {
            return Err(CoreError::LinearSolve(format!(
                "block GMRES exhausted {} search directions",
                config.max_basis
            )));
        }
        let available = config.max_basis - directions;
        frontier_end = frontier_end.min(frontier_start + available);
        let inputs = basis[frontier_start..frontier_end].to_vec();
        let mut images = vec![vec![0.0; n]; inputs.len()];
        apply_left_rows(op, pc, &inputs, &mut images, counters)?;
        block_iterations += 1;

        let basis_before = basis.len();
        for mut image in images {
            let image_scale = safe_l2(&image).max(1.0);
            orthogonalize(&mut image, &basis, &mut coefficient_scratch, counters)?;
            let norm = safe_l2(&image);
            let mut column = coefficient_scratch.clone();
            if norm > config.rank_tolerance * image_scale {
                for value in &mut image {
                    *value /= norm;
                }
                basis.push(image);
                column.push(norm);
            }
            h_columns.push(column);
            directions += 1;
        }
        counters.linear_iterations += inputs.len() as u64;
        counters.block_linear_iterations += 1;

        let solutions = projection_solution(
            &basis,
            directions,
            &h_columns,
            &preconditioned_rhs,
            counters,
        )?;
        let (residual_norms, relative_residuals) =
            true_residuals(op, rhs_rows, &solutions, counters)?;
        let converged = residual_norms
            .iter()
            .zip(&thresholds)
            .all(|(residual, threshold)| residual.is_finite() && residual <= threshold);
        if converged {
            counters.linear_solves += rhs_count as u64;
            counters.block_linear_solves += 1;
            let delta = counters.delta(before);
            return Ok(BlockLinearSolveReport {
                maximum_residual_norm: residual_norms.iter().copied().fold(0.0_f64, f64::max),
                maximum_relative_residual: relative_residuals
                    .iter()
                    .copied()
                    .fold(0.0_f64, f64::max),
                residual_norms,
                relative_residuals,
                solutions,
                converged: true,
                initial_block_rank,
                final_basis_dimension: basis.len(),
                search_directions: directions,
                block_iterations,
                operator_vectors: delta.linear_matvecs + delta.diagnostic_matvecs,
                block_operator_calls: delta.block_matvecs,
                method: "block-gmres".into(),
            });
        }

        let next_start = frontier_end;
        let next_end = basis.len();
        if next_end <= next_start || basis.len() == basis_before {
            return Err(CoreError::LinearSolve(
                "block GMRES Arnoldi relation stagnated before convergence".into(),
            ));
        }
        frontier_start = next_start;
        frontier_end = next_end;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeededGmresConfig {
    /// Number of scalar Arnoldi directions built once from the largest seed RHS.
    pub shared_basis: usize,
    pub restart: usize,
    pub max_arnoldi: usize,
    pub rtol: f64,
    pub atol: f64,
    pub rank_tolerance: f64,
}

impl Default for SeededGmresConfig {
    fn default() -> Self {
        Self {
            shared_basis: 12,
            restart: 40,
            max_arnoldi: 200,
            rtol: 1.0e-10,
            atol: 1.0e-12,
            rank_tolerance: 1.0e-12,
        }
    }
}

impl SeededGmresConfig {
    fn validate(&self) -> CoreResult<()> {
        if self.shared_basis == 0 || self.restart == 0 || self.max_arnoldi == 0 {
            return Err(CoreError::InvalidInput(
                "seeded GMRES iteration limits must be positive".into(),
            ));
        }
        BlockGmresConfig {
            max_basis: self.shared_basis,
            rtol: self.rtol,
            atol: self.atol,
            rank_tolerance: self.rank_tolerance,
        }
        .validate()
    }
}

/// Build one seed Krylov basis, project every right-hand side into that space, and refine only
/// residual components that remain outside the shared space with ordinary restarted GMRES.
///
/// This is a seed/shared-basis comparator, not a Rosenbrock--Krylov time integrator: it does not
/// change the temporal order conditions and every returned solution is certified by the same
/// unpreconditioned true-residual rule as independent GMRES.
pub fn solve_seeded_gmres(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs_rows: &[Vec<f64>],
    config: &SeededGmresConfig,
    counters: &mut WorkCounters,
) -> CoreResult<BlockLinearSolveReport> {
    use crate::gmres::{GmresConfig, solve_gmres};

    config.validate()?;
    let (n, rhs_count) = validate_rhs(op, pc, rhs_rows)?;
    let before = *counters;
    let mut preconditioned_rhs = vec![vec![0.0; n]; rhs_count];
    for (rhs, output) in rhs_rows.iter().zip(&mut preconditioned_rhs) {
        apply_preconditioner(pc, rhs, output, counters)?;
    }
    counters.block_preconditioner_apps += 1;

    let seed_index = preconditioned_rhs
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| safe_l2(left).total_cmp(&safe_l2(right)))
        .map(|(index, _)| index)
        .expect("validated nonempty RHS");
    let seed_norm = safe_l2(&preconditioned_rhs[seed_index]);
    if seed_norm <= f64::MIN_POSITIVE {
        counters.linear_solves += rhs_count as u64;
        counters.block_linear_solves += 1;
        return Ok(BlockLinearSolveReport {
            solutions: vec![vec![0.0; n]; rhs_count],
            converged: true,
            residual_norms: vec![0.0; rhs_count],
            relative_residuals: vec![0.0; rhs_count],
            maximum_residual_norm: 0.0,
            maximum_relative_residual: 0.0,
            initial_block_rank: 0,
            final_basis_dimension: 0,
            search_directions: 0,
            block_iterations: 0,
            operator_vectors: 0,
            block_operator_calls: 0,
            method: "seeded-shared-gmres".into(),
        });
    }

    let mut seed = preconditioned_rhs[seed_index].clone();
    for value in &mut seed {
        *value /= seed_norm;
    }
    let mut basis = vec![seed];
    let mut h_columns = Vec::<Vec<f64>>::new();
    let mut coefficient_scratch = Vec::new();
    let mut directions = 0usize;
    let mut block_iterations = 0usize;

    while directions < config.shared_basis.min(n) && directions < basis.len() {
        let inputs = vec![basis[directions].clone()];
        let mut images = vec![vec![0.0; n]];
        apply_left_rows(op, pc, &inputs, &mut images, counters)?;
        block_iterations += 1;
        let mut image = images.pop().expect("one image");
        let image_scale = safe_l2(&image).max(1.0);
        orthogonalize(&mut image, &basis, &mut coefficient_scratch, counters)?;
        let norm = safe_l2(&image);
        let mut column = coefficient_scratch.clone();
        if norm > config.rank_tolerance * image_scale {
            for value in &mut image {
                *value /= norm;
            }
            basis.push(image);
            column.push(norm);
        }
        h_columns.push(column);
        directions += 1;
        counters.linear_iterations += 1;
        counters.block_linear_iterations += 1;
        if norm <= config.rank_tolerance * image_scale {
            break;
        }
    }

    let mut solutions = projection_solution(
        &basis,
        directions,
        &h_columns,
        &preconditioned_rhs,
        counters,
    )?;
    let (mut residual_norms, mut relative_residuals) =
        true_residuals(op, rhs_rows, &solutions, counters)?;
    let thresholds = rhs_rows
        .iter()
        .map(|rhs| config.atol.max(config.rtol * safe_l2(rhs)))
        .collect::<Vec<_>>();
    let mut refined = 0usize;
    for rhs_index in 0..rhs_count {
        if residual_norms[rhs_index] <= thresholds[rhs_index] {
            continue;
        }
        let report = solve_gmres(
            op,
            pc,
            &rhs_rows[rhs_index],
            Some(&solutions[rhs_index]),
            &GmresConfig {
                restart: config.restart,
                max_arnoldi: config.max_arnoldi,
                rtol: config.rtol,
                atol: config.atol,
            },
            counters,
        )?;
        solutions[rhs_index] = report.x;
        residual_norms[rhs_index] = report.residual_norm;
        relative_residuals[rhs_index] = report.relative_residual;
        refined += 1;
    }
    counters.linear_solves += (rhs_count - refined) as u64;
    counters.block_linear_solves += 1;

    let converged = residual_norms
        .iter()
        .zip(&thresholds)
        .all(|(residual, threshold)| residual.is_finite() && residual <= threshold);
    if !converged {
        return Err(CoreError::LinearSolve(
            "seeded shared-basis GMRES failed the true-residual certificate".into(),
        ));
    }
    let delta = counters.delta(before);
    Ok(BlockLinearSolveReport {
        maximum_residual_norm: residual_norms.iter().copied().fold(0.0_f64, f64::max),
        maximum_relative_residual: relative_residuals.iter().copied().fold(0.0_f64, f64::max),
        solutions,
        converged,
        residual_norms,
        relative_residuals,
        initial_block_rank: 1,
        final_basis_dimension: basis.len(),
        search_directions: directions,
        block_iterations,
        operator_vectors: delta.linear_matvecs + delta.diagnostic_matvecs,
        block_operator_calls: delta.block_matvecs,
        method: "seeded-shared-gmres".into(),
    })
}
