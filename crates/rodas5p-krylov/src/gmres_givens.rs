use crate::{
    common::{apply_left_with_raw, true_residual_into, validate_system},
    gmres::GmresConfig,
    kernels::{axpy, linear_combination_into, normalize, two_pass_mgs_into},
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, LinearOperator, LinearSolveReport,
    Preconditioner, WorkCounters, apply_preconditioner, safe_l2,
};

#[derive(Clone, Copy, Debug)]
struct RealGivens {
    cosine: f64,
    sine: f64,
}

impl RealGivens {
    fn from_pair(a: f64, b: f64) -> CoreResult<(Self, f64)> {
        if !a.is_finite() || !b.is_finite() {
            return Err(CoreError::NonFinite(
                "GMRES Givens input contains NaN/Inf".into(),
            ));
        }
        if b == 0.0 {
            return Ok((
                Self {
                    cosine: 1.0,
                    sine: 0.0,
                },
                a,
            ));
        }
        if a == 0.0 {
            let sine = b.signum();
            return Ok((Self { cosine: 0.0, sine }, b.abs()));
        }
        let radius = a.hypot(b);
        if !(radius > 0.0 && radius.is_finite()) {
            return Err(CoreError::NonFinite(
                "GMRES Givens radius is non-finite".into(),
            ));
        }
        Ok((
            Self {
                cosine: a / radius,
                sine: b / radius,
            },
            radius,
        ))
    }

    fn apply(self, first: &mut f64, second: &mut f64) {
        let a = *first;
        let b = *second;
        *first = self.cosine * a + self.sine * b;
        *second = -self.sine * a + self.cosine * b;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GmresGivensStatistics {
    pub restart_cycles: u64,
    pub projected_residual_checks: u64,
    pub rejected_projected_residual_checks: u64,
    pub triangular_solves: u64,
}

#[derive(Clone, Debug, Default)]
pub struct GmresGivensWorkspace {
    x: Vec<f64>,
    residual: Vec<f64>,
    operator_output: Vec<f64>,
    preconditioned: Vec<f64>,
    raw_operator_output: Vec<f64>,
    candidate_x: Vec<f64>,
    basis: Vec<Vec<f64>>,
    directions: Vec<Vec<f64>>,
    r_factor: DenseMatrix,
    h_column: Vec<f64>,
    correction: Vec<f64>,
    coefficients: Vec<f64>,
    rotated_rhs: Vec<f64>,
    givens_cosines: Vec<f64>,
    givens_sines: Vec<f64>,
    statistics: GmresGivensStatistics,
}

impl GmresGivensWorkspace {
    fn ensure_vector(vector: &mut Vec<f64>, length: usize) {
        vector.resize(length, 0.0);
        vector.fill(0.0);
    }

    fn ensure_pool(pool: &mut Vec<Vec<f64>>, count: usize, dimension: usize) {
        while pool.len() < count {
            pool.push(vec![0.0; dimension]);
        }
        for vector in &mut pool[..count] {
            Self::ensure_vector(vector, dimension);
        }
    }

    fn prepare_solve(&mut self, dimension: usize) {
        Self::ensure_vector(&mut self.x, dimension);
        Self::ensure_vector(&mut self.residual, dimension);
        Self::ensure_vector(&mut self.operator_output, dimension);
        Self::ensure_vector(&mut self.preconditioned, dimension);
        Self::ensure_vector(&mut self.raw_operator_output, dimension);
        Self::ensure_vector(&mut self.candidate_x, dimension);
        Self::ensure_vector(&mut self.correction, dimension);
        self.statistics = GmresGivensStatistics::default();
    }

    fn prepare_cycle(&mut self, dimension: usize, columns: usize) -> CoreResult<()> {
        Self::ensure_pool(&mut self.basis, columns + 1, dimension);
        Self::ensure_pool(&mut self.directions, columns, dimension);
        self.r_factor.resize_zeros(columns, columns)?;
        Self::ensure_vector(&mut self.h_column, columns + 1);
        Self::ensure_vector(&mut self.coefficients, columns);
        Self::ensure_vector(&mut self.rotated_rhs, columns + 1);
        Self::ensure_vector(&mut self.givens_cosines, columns);
        Self::ensure_vector(&mut self.givens_sines, columns);
        Ok(())
    }

    pub fn statistics(&self) -> GmresGivensStatistics {
        self.statistics
    }

    pub fn capacity_f64(&self) -> usize {
        self.x.capacity()
            + self.residual.capacity()
            + self.operator_output.capacity()
            + self.preconditioned.capacity()
            + self.raw_operator_output.capacity()
            + self.candidate_x.capacity()
            + self.basis.iter().map(Vec::capacity).sum::<usize>()
            + self.directions.iter().map(Vec::capacity).sum::<usize>()
            + self.r_factor.capacity()
            + self.h_column.capacity()
            + self.correction.capacity()
            + self.coefficients.capacity()
            + self.rotated_rhs.capacity()
            + self.givens_cosines.capacity()
            + self.givens_sines.capacity()
    }
}

fn update_incremental_qr(workspace: &mut GmresGivensWorkspace, column: usize) -> CoreResult<f64> {
    for index in 0..column {
        let rotation = RealGivens {
            cosine: workspace.givens_cosines[index],
            sine: workspace.givens_sines[index],
        };
        let (left, right) = workspace.h_column.split_at_mut(index + 1);
        rotation.apply(&mut left[index], &mut right[0]);
    }

    let (rotation, diagonal) =
        RealGivens::from_pair(workspace.h_column[column], workspace.h_column[column + 1])?;
    workspace.givens_cosines[column] = rotation.cosine;
    workspace.givens_sines[column] = rotation.sine;
    workspace.h_column[column] = diagonal;
    workspace.h_column[column + 1] = 0.0;

    for row in 0..=column {
        workspace.r_factor[(row, column)] = workspace.h_column[row];
    }

    let (left, right) = workspace.rotated_rhs.split_at_mut(column + 1);
    rotation.apply(&mut left[column], &mut right[0]);
    let projected_residual = right[0].abs();
    if projected_residual.is_finite() {
        Ok(projected_residual)
    } else {
        Err(CoreError::NonFinite(
            "GMRES projected residual is NaN/Inf".into(),
        ))
    }
}

fn back_substitute(
    r_factor: &DenseMatrix,
    rotated_rhs: &[f64],
    dimension: usize,
    coefficients: &mut [f64],
) -> CoreResult<()> {
    if r_factor.nrows() < dimension
        || r_factor.ncols() < dimension
        || rotated_rhs.len() < dimension
        || coefficients.len() < dimension
    {
        return Err(CoreError::Dimension(
            "GMRES triangular solve shape mismatch".into(),
        ));
    }
    coefficients[..dimension].fill(0.0);
    let diagonal_scale = (0..dimension)
        .map(|index| r_factor[(index, index)].abs())
        .fold(0.0, f64::max);
    if !diagonal_scale.is_finite() {
        return Err(CoreError::NonFinite(
            "GMRES incremental triangular factor scale is NaN/Inf".into(),
        ));
    }
    if diagonal_scale == 0.0 {
        return Err(CoreError::LinearSolve(
            "GMRES incremental triangular factor is identically zero".into(),
        ));
    }
    let diagonal_tolerance = 100.0 * f64::EPSILON * diagonal_scale;

    for row in (0..dimension).rev() {
        let mut value = rotated_rhs[row];
        for column in row + 1..dimension {
            value -= r_factor[(row, column)] * coefficients[column];
        }
        let diagonal = r_factor[(row, row)];
        if !diagonal.is_finite() || diagonal.abs() <= diagonal_tolerance {
            return Err(CoreError::LinearSolve(format!(
                "GMRES incremental triangular factor is singular at row {row}"
            )));
        }
        coefficients[row] = value / diagonal;
        if !coefficients[row].is_finite() {
            return Err(CoreError::NonFinite(
                "GMRES triangular solve produced NaN/Inf".into(),
            ));
        }
    }
    Ok(())
}

fn build_candidate(
    workspace: &mut GmresGivensWorkspace,
    base_x: &[f64],
    dimension: usize,
    counters: &mut WorkCounters,
) -> CoreResult<()> {
    back_substitute(
        &workspace.r_factor,
        &workspace.rotated_rhs,
        dimension,
        &mut workspace.coefficients,
    )?;
    linear_combination_into(
        &workspace.directions[..dimension],
        &workspace.coefficients[..dimension],
        &mut workspace.correction,
    )?;
    workspace.candidate_x.copy_from_slice(base_x);
    axpy(
        1.0,
        &workspace.correction,
        &mut workspace.candidate_x,
        counters,
    )?;
    workspace.statistics.triangular_solves += 1;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn certify_candidate(
    op: &dyn LinearOperator,
    rhs: &[f64],
    threshold: f64,
    workspace: &mut GmresGivensWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<bool> {
    true_residual_into(
        op,
        rhs,
        &workspace.candidate_x,
        &mut workspace.operator_output,
        &mut workspace.residual,
        counters,
        ApplyCategory::Diagnostic,
    )?;
    let residual_norm = safe_l2(&workspace.residual);
    if !residual_norm.is_finite() {
        return Err(CoreError::NonFinite(
            "GMRES true residual is NaN/Inf".into(),
        ));
    }
    Ok(residual_norm <= threshold)
}

pub fn solve_gmres_givens_with_workspace(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    workspace: &mut GmresGivensWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    config.validate()?;
    if !config.rtol.is_finite() || !config.atol.is_finite() {
        return Err(CoreError::InvalidInput(
            "GMRES tolerances must be finite".into(),
        ));
    }
    let n = validate_system(op, pc, rhs, x0)?;
    let before = *counters;
    let right_norm = safe_l2(rhs);
    let threshold = config.atol.max(config.rtol * right_norm);
    if !threshold.is_finite() {
        return Err(CoreError::InvalidInput(
            "GMRES residual threshold must be finite".into(),
        ));
    }

    workspace.prepare_solve(n);
    if let Some(initial) = x0 {
        workspace.x.copy_from_slice(initial);
    }

    let mut total_iterations = 0usize;
    'solve: loop {
        if workspace.x.iter().all(|value| *value == 0.0) {
            workspace.residual.copy_from_slice(rhs);
        } else {
            true_residual_into(
                op,
                rhs,
                &workspace.x,
                &mut workspace.operator_output,
                &mut workspace.residual,
                counters,
                ApplyCategory::Krylov,
            )?;
        }
        let true_initial_norm = safe_l2(&workspace.residual);
        if !true_initial_norm.is_finite() {
            return Err(CoreError::NonFinite(
                "GMRES cycle residual is NaN/Inf".into(),
            ));
        }
        if true_initial_norm <= threshold {
            break;
        }
        if total_iterations >= config.max_arnoldi {
            return Err(CoreError::LinearSolve(format!(
                "GMRES-Givens exhausted {} Arnoldi vectors",
                config.max_arnoldi
            )));
        }

        apply_preconditioner(
            pc,
            &workspace.residual,
            &mut workspace.preconditioned,
            counters,
        )?;
        let beta = safe_l2(&workspace.preconditioned);
        if !(beta > f64::MIN_POSITIVE && beta.is_finite()) {
            return Err(CoreError::LinearSolve(
                "GMRES-Givens preconditioned residual breakdown".into(),
            ));
        }
        let trigger_scale = beta / true_initial_norm;
        if !(trigger_scale > 0.0 && trigger_scale.is_finite()) {
            return Err(CoreError::LinearSolve(
                "GMRES-Givens projected-residual scale is invalid".into(),
            ));
        }
        let projected_trigger = threshold * trigger_scale;
        if !projected_trigger.is_finite() {
            return Err(CoreError::NonFinite(
                "GMRES-Givens projected-residual trigger is NaN/Inf".into(),
            ));
        }

        let cycle_columns = config
            .restart
            .min(config.max_arnoldi - total_iterations)
            .min(n.max(1));
        workspace.prepare_cycle(n, cycle_columns)?;
        workspace.basis[0].copy_from_slice(&workspace.preconditioned);
        normalize(&mut workspace.basis[0])?;
        workspace.rotated_rhs[0] = beta;
        workspace.statistics.restart_cycles += 1;

        let mut actual = 0usize;
        let mut certified = false;
        for column in 0..cycle_columns {
            workspace.directions[column].copy_from_slice(&workspace.basis[column]);
            let (previous_basis, remaining_basis) = workspace.basis.split_at_mut(column + 1);
            let work = &mut remaining_basis[0];
            apply_left_with_raw(
                op,
                pc,
                &workspace.directions[column],
                work,
                &mut workspace.raw_operator_output,
                counters,
                ApplyCategory::Krylov,
            )?;
            two_pass_mgs_into(work, previous_basis, &mut workspace.h_column, counters)?;
            let h_next = safe_l2(work);
            actual = column + 1;

            let orthogonalization_scale = workspace.h_column[..previous_basis.len()]
                .iter()
                .map(|value| value.abs())
                .fold(0.0, f64::max);
            let breakdown_threshold = 100.0 * f64::EPSILON * (1.0 + orthogonalization_scale);
            let happy_breakdown = h_next <= breakdown_threshold;
            workspace.h_column[column + 1] = h_next;
            if happy_breakdown {
                work.fill(0.0);
            } else {
                for value in work {
                    *value /= h_next;
                }
            }

            let projected_residual = update_incremental_qr(workspace, column)?;
            if projected_residual <= projected_trigger || happy_breakdown {
                workspace.statistics.projected_residual_checks += 1;
                let base_x = workspace.x.clone();
                build_candidate(workspace, &base_x, actual, counters)?;
                if certify_candidate(op, rhs, threshold, workspace, counters)? {
                    workspace.x.copy_from_slice(&workspace.candidate_x);
                    certified = true;
                } else {
                    workspace.statistics.rejected_projected_residual_checks += 1;
                }
            }

            if certified {
                total_iterations += actual;
                counters.linear_iterations += actual as u64;
                break 'solve;
            }
            if happy_breakdown {
                break;
            }
        }

        if actual == 0 {
            return Err(CoreError::LinearSolve(
                "GMRES-Givens produced an empty restart cycle".into(),
            ));
        }
        let base_x = workspace.x.clone();
        build_candidate(workspace, &base_x, actual, counters)?;
        workspace.x.copy_from_slice(&workspace.candidate_x);
        total_iterations += actual;
        counters.linear_iterations += actual as u64;
    }

    true_residual_into(
        op,
        rhs,
        &workspace.x,
        &mut workspace.operator_output,
        &mut workspace.residual,
        counters,
        ApplyCategory::Diagnostic,
    )?;
    let residual_norm = safe_l2(&workspace.residual);
    if !residual_norm.is_finite() || residual_norm > threshold {
        return Err(CoreError::LinearSolve(format!(
            "GMRES-Givens true residual {residual_norm:.3e} exceeds {threshold:.3e}"
        )));
    }
    counters.linear_solves += 1;
    let delta = counters.delta(before);
    Ok(LinearSolveReport {
        x: workspace.x.clone(),
        converged: true,
        info: 0,
        residual_norm,
        relative_residual: residual_norm / right_norm.max(f64::MIN_POSITIVE),
        iterations: total_iterations as u64,
        matvecs: delta.linear_matvecs,
        preconditioner_apps: delta.preconditioner_apps,
        method: "gmres-givens-candidate".into(),
    })
}

pub fn solve_gmres_givens(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_gmres_givens_with_workspace(
        op,
        pc,
        rhs,
        x0,
        config,
        &mut GmresGivensWorkspace::default(),
        counters,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::small::least_squares;
    use rodas5p_core::{DenseOperator, IdentityPreconditioner};

    fn qr_oracle_residual(hessenberg: &DenseMatrix, beta: f64) -> CoreResult<f64> {
        let mut rhs = vec![0.0; hessenberg.nrows()];
        rhs[0] = beta;
        let coefficients = least_squares(hessenberg, &rhs)?;
        for row in 0..hessenberg.nrows() {
            for column in 0..hessenberg.ncols() {
                rhs[row] -= hessenberg[(row, column)] * coefficients[column];
            }
        }
        Ok(safe_l2(&rhs))
    }

    #[test]
    fn stable_real_givens_zeroes_the_second_component() {
        for (a, b) in [(3.0, 4.0), (-3.0, 4.0), (0.0, -2.0), (2.0, 0.0)] {
            let (rotation, radius) = RealGivens::from_pair(a, b).unwrap();
            let mut first = a;
            let mut second = b;
            rotation.apply(&mut first, &mut second);
            assert!((first - radius).abs() <= 8.0 * f64::EPSILON * radius.abs().max(1.0));
            assert!(second.abs() <= 8.0 * f64::EPSILON * radius.abs().max(1.0));
        }
    }

    #[test]
    fn incremental_projected_residual_matches_full_qr_oracle() {
        let columns = [
            [2.0, 0.5, 0.0, 0.0, 0.0],
            [-0.3, 1.7, 0.4, 0.0, 0.0],
            [0.2, -0.6, 1.4, 0.3, 0.0],
            [0.1, 0.4, -0.2, 1.2, 0.25],
        ];
        let beta = 2.3;
        let mut workspace = GmresGivensWorkspace::default();
        workspace.prepare_cycle(1, columns.len()).unwrap();
        workspace.rotated_rhs[0] = beta;
        let mut raw_hessenberg = DenseMatrix::zeros(columns.len() + 1, columns.len());

        for (column, values) in columns.iter().enumerate() {
            for row in 0..=column + 1 {
                raw_hessenberg[(row, column)] = values[row];
                workspace.h_column[row] = values[row];
            }
            let projected = update_incremental_qr(&mut workspace, column).unwrap();
            let mut prefix = DenseMatrix::zeros(column + 2, column + 1);
            for row in 0..column + 2 {
                for inner_column in 0..column + 1 {
                    prefix[(row, inner_column)] = raw_hessenberg[(row, inner_column)];
                }
            }
            let oracle = qr_oracle_residual(&prefix, beta).unwrap();
            assert!((projected - oracle).abs() <= 2.0e-12 * oracle.max(1.0));
        }
    }

    #[test]
    fn triangular_backsolve_reconstructs_the_rotated_system() {
        let mut r = DenseMatrix::zeros(3, 3);
        r[(0, 0)] = 2.0;
        r[(0, 1)] = -0.5;
        r[(0, 2)] = 0.25;
        r[(1, 1)] = 1.5;
        r[(1, 2)] = -0.75;
        r[(2, 2)] = 0.8;
        let rhs = [1.0, -2.0, 0.5];
        let mut coefficients = [0.0; 3];
        back_substitute(&r, &rhs, 3, &mut coefficients).unwrap();
        for row in 0..3 {
            let reconstructed = (row..3)
                .map(|column| r[(row, column)] * coefficients[column])
                .sum::<f64>();
            assert!((reconstructed - rhs[row]).abs() <= 2.0e-14);
        }
    }

    #[test]
    fn triangular_backsolve_rejects_an_identically_zero_factor() {
        let r = DenseMatrix::zeros(1, 1);
        let mut coefficients = [0.0; 1];
        let error = back_substitute(&r, &[1.0], 1, &mut coefficients).unwrap_err();
        assert!(matches!(error, CoreError::LinearSolve(_)));
    }

    #[test]
    fn uniformly_scaled_identity_preserves_solution_and_convergence() {
        let solve = |alpha: f64| {
            let matrix = DenseMatrix::from_rows(&[&[alpha, 0.0], &[0.0, alpha]]).unwrap();
            let operator = DenseOperator::new(matrix).unwrap();
            let preconditioner = IdentityPreconditioner::new(2);
            let rhs = [alpha, -2.0 * alpha];
            let config = GmresConfig {
                restart: 2,
                max_arnoldi: 2,
                rtol: 1.0e-12,
                atol: 0.0,
            };
            let mut counters = WorkCounters::default();
            let report = solve_gmres_givens(
                &operator,
                &preconditioner,
                &rhs,
                None,
                &config,
                &mut counters,
            )
            .expect("scaled identity GMRES-Givens solve");
            (report, counters, rhs, config)
        };

        let (unscaled, unscaled_work, unscaled_rhs, unscaled_config) = solve(1.0);
        let (scaled, scaled_work, scaled_rhs, scaled_config) = solve(1.0e-15);
        let expected = [1.0, -2.0];

        for (report, rhs, config) in [
            (&unscaled, &unscaled_rhs, &unscaled_config),
            (&scaled, &scaled_rhs, &scaled_config),
        ] {
            let error: Vec<f64> = report
                .x
                .iter()
                .zip(expected)
                .map(|(actual, exact)| actual - exact)
                .collect();
            let threshold = config.atol.max(config.rtol * safe_l2(rhs));
            assert!(safe_l2(&error) <= 64.0 * f64::EPSILON * safe_l2(&expected));
            assert!(report.residual_norm <= threshold);
            assert_eq!(report.iterations, 1);
        }

        let parity_error: Vec<f64> = unscaled
            .x
            .iter()
            .zip(&scaled.x)
            .map(|(left, right)| left - right)
            .collect();
        assert!(safe_l2(&parity_error) <= 64.0 * f64::EPSILON * safe_l2(&expected));
        assert_eq!(
            unscaled_work.linear_iterations,
            scaled_work.linear_iterations
        );
        assert_eq!(unscaled_work.linear_matvecs, scaled_work.linear_matvecs);
    }
}
