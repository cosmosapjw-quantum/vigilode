use crate::{
    common::{apply_left_with_raw, true_residual_into, validate_system},
    kernels::{axpy, linear_combination_into, normalize, two_pass_mgs_into},
    small::least_squares,
    workspace::{ArnoldiWorkspace, GmresWorkspace},
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, DenseMatrix, LinearOperator, LinearSolveReport,
    Preconditioner, WorkCounters, apply_preconditioner, safe_l2,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GmresConfig {
    pub restart: usize,
    pub max_arnoldi: usize,
    pub rtol: f64,
    pub atol: f64,
}
impl Default for GmresConfig {
    fn default() -> Self {
        Self {
            restart: 40,
            max_arnoldi: 200,
            rtol: 1e-11,
            atol: 1e-13,
        }
    }
}
impl GmresConfig {
    pub fn validate(&self) -> CoreResult<()> {
        if self.restart == 0 || self.max_arnoldi == 0 {
            return Err(CoreError::InvalidInput(
                "GMRES iteration limits must be positive".into(),
            ));
        }
        if self.rtol < 0.0 || self.atol < 0.0 {
            return Err(CoreError::InvalidInput(
                "GMRES tolerances must be nonnegative".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) struct ArnoldiResult {
    pub iterations: usize,
}

// Keep augmentation vectors, cached images, accounting, and reusable scratch explicit;
// hiding them in a transient context would obscure the fair-work contract.
#[allow(clippy::too_many_arguments)]
pub(crate) fn arnoldi_augmented_with_workspace(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    r_pre: &[f64],
    beta: f64,
    krylov_steps: usize,
    augment_directions: &[Vec<f64>],
    augment_images: &[Option<Vec<f64>>],
    counters: &mut WorkCounters,
    workspace: &mut ArnoldiWorkspace,
) -> CoreResult<ArnoldiResult> {
    if augment_directions.len() != augment_images.len() {
        return Err(CoreError::Dimension(
            "Arnoldi augmentation direction/image mismatch".into(),
        ));
    }
    let n = r_pre.len();
    let total = krylov_steps + augment_directions.len();
    workspace.prepare(n, total)?;
    workspace.basis[0].copy_from_slice(r_pre);
    if normalize(&mut workspace.basis[0])? == 0.0 {
        return Err(CoreError::LinearSolve("zero Arnoldi residual".into()));
    }

    let mut actual = 0usize;
    let mut last_solution = Vec::new();
    for j in 0..total {
        if j < krylov_steps {
            workspace.directions[j].copy_from_slice(&workspace.basis[j]);
        } else {
            workspace.directions[j].copy_from_slice(&augment_directions[j - krylov_steps]);
        }

        let (previous_basis, remaining_basis) = workspace.basis.split_at_mut(j + 1);
        let w = &mut remaining_basis[0];
        if j >= krylov_steps {
            if let Some(image) = &augment_images[j - krylov_steps] {
                w.copy_from_slice(image);
            } else {
                apply_left_with_raw(
                    op,
                    pc,
                    &workspace.directions[j],
                    w,
                    &mut workspace.raw_operator_output,
                    counters,
                    ApplyCategory::Krylov,
                )?;
            }
        } else {
            apply_left_with_raw(
                op,
                pc,
                &workspace.directions[j],
                w,
                &mut workspace.raw_operator_output,
                counters,
                ApplyCategory::Krylov,
            )?;
        }

        two_pass_mgs_into(w, previous_basis, &mut workspace.h_column, counters)?;
        for i in 0..previous_basis.len() {
            workspace.hessenberg[(i, j)] = workspace.h_column[i];
        }
        let h_next = safe_l2(w);
        workspace.hessenberg[(j + 1, j)] = h_next;
        actual = j + 1;
        let orthogonalization_scale = workspace.h_column[..previous_basis.len()]
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max);
        let breakdown_threshold = 100.0 * f64::EPSILON * (1.0 + orthogonalization_scale);
        if h_next > breakdown_threshold {
            for value in w {
                *value /= h_next;
            }
        } else {
            w.fill(0.0);
        }

        workspace
            .hessenberg_prefix
            .resize_zeros(actual + 1, actual)?;
        for row in 0..actual + 1 {
            for column in 0..actual {
                workspace.hessenberg_prefix[(row, column)] = workspace.hessenberg[(row, column)];
            }
        }
        workspace.rhs_small[..actual + 1].fill(0.0);
        workspace.rhs_small[0] = beta;
        last_solution = least_squares(
            &workspace.hessenberg_prefix,
            &workspace.rhs_small[..actual + 1],
        )?;
        if h_next <= breakdown_threshold {
            break;
        }
    }

    linear_combination_into(
        &workspace.directions[..actual],
        &last_solution,
        &mut workspace.correction,
    )?;
    Ok(ArnoldiResult { iterations: actual })
}

pub fn solve_gmres_with_workspace(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    workspace: &mut GmresWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    config.validate()?;
    let n = validate_system(op, pc, rhs, x0)?;
    let before = *counters;
    let right_norm = safe_l2(rhs);
    let threshold = config.atol.max(config.rtol * right_norm);
    workspace.common.prepare(n);
    if let Some(initial) = x0 {
        workspace.common.x.copy_from_slice(initial);
    }

    let mut total = 0usize;
    loop {
        if workspace.common.x.iter().all(|value| *value == 0.0) {
            workspace.common.residual.copy_from_slice(rhs);
        } else {
            true_residual_into(
                op,
                rhs,
                &workspace.common.x,
                &mut workspace.common.operator_output,
                &mut workspace.common.residual,
                counters,
                ApplyCategory::Krylov,
            )?;
        }
        let residual_norm = safe_l2(&workspace.common.residual);
        if residual_norm <= threshold {
            break;
        }
        if total >= config.max_arnoldi {
            return Err(CoreError::LinearSolve(format!(
                "GMRES exhausted {} Arnoldi vectors",
                config.max_arnoldi
            )));
        }
        apply_preconditioner(
            pc,
            &workspace.common.residual,
            &mut workspace.common.preconditioned,
            counters,
        )?;
        let beta = safe_l2(&workspace.common.preconditioned);
        if !(beta > f64::MIN_POSITIVE && beta.is_finite()) {
            return Err(CoreError::LinearSolve(
                "GMRES preconditioned residual breakdown".into(),
            ));
        }
        let steps = config.restart.min(config.max_arnoldi - total).min(n.max(1));
        let arnoldi = arnoldi_augmented_with_workspace(
            op,
            pc,
            &workspace.common.preconditioned,
            beta,
            steps,
            &[],
            &[],
            counters,
            &mut workspace.arnoldi,
        )?;
        axpy(
            1.0,
            &workspace.arnoldi.correction,
            &mut workspace.common.x,
            counters,
        )?;
        total += arnoldi.iterations;
        counters.linear_iterations += arnoldi.iterations as u64;
    }

    true_residual_into(
        op,
        rhs,
        &workspace.common.x,
        &mut workspace.common.operator_output,
        &mut workspace.common.residual,
        counters,
        ApplyCategory::Diagnostic,
    )?;
    let residual_norm = safe_l2(&workspace.common.residual);
    if !residual_norm.is_finite() || residual_norm > threshold {
        return Err(CoreError::LinearSolve(format!(
            "GMRES true residual {residual_norm:.3e} exceeds {threshold:.3e}"
        )));
    }
    counters.linear_solves += 1;
    let delta = counters.delta(before);
    Ok(LinearSolveReport {
        x: workspace.common.x.clone(),
        converged: true,
        info: 0,
        residual_norm,
        relative_residual: residual_norm / right_norm.max(f64::MIN_POSITIVE),
        iterations: total as u64,
        matvecs: delta.linear_matvecs,
        preconditioner_apps: delta.preconditioner_apps,
        method: "gmres".into(),
    })
}

pub fn solve_gmres(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_gmres_with_workspace(
        op,
        pc,
        rhs,
        x0,
        config,
        &mut GmresWorkspace::default(),
        counters,
    )
}

/// A cheap prediction extracted from an actually computed GMRES prefix.
///
/// The prediction is deliberately advisory: arbitrary nonnormal operators can exhibit delayed
/// GMRES convergence, so the final solve always retains the original true-residual certificate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GmresPrefixPrediction {
    pub prefix_iterations: usize,
    pub predicted_total_iterations: usize,
    pub residual_norm: f64,
    pub target_residual_norm: f64,
    pub observed_contraction: f64,
}

fn geometric_iteration_prediction(
    history: &[f64],
    target: f64,
    current: usize,
    maximum: usize,
) -> (usize, f64) {
    let last = history.last().copied().unwrap_or(f64::INFINITY);
    if last <= target {
        return (current, 0.0);
    }
    let mut ratios = history
        .windows(2)
        .rev()
        .take(2)
        .filter_map(|pair| {
            let denominator = pair[0];
            let ratio = pair[1] / denominator;
            (denominator > 0.0 && ratio.is_finite() && ratio > 0.0).then_some(ratio)
        })
        .collect::<Vec<_>>();
    if ratios.is_empty() {
        return (maximum, 1.0);
    }
    // A conservative short-prefix estimate uses the worst of the most recent ratios.
    let observed = ratios
        .drain(..)
        .fold(0.0_f64, f64::max)
        .clamp(1.0e-3, 0.999);
    if observed >= 0.999 || !last.is_finite() || !target.is_finite() || target <= 0.0 {
        return (maximum, observed);
    }
    let remaining = (target / last).ln() / observed.ln();
    let remaining = remaining.ceil().max(0.0) as usize;
    (current.saturating_add(remaining).min(maximum), observed)
}

/// Restart-free GMRES state whose first Krylov vectors can be used as a bounded method-entry probe
/// and then reused without rebuilding the basis if this method is selected.
///
/// This G4-S4 research kernel is intentionally one-cycle.  Production restart/recycle policy stays
/// in the existing protected GMRES implementation until the prefix gate is passed.
pub struct GmresPrefixSession {
    config: GmresConfig,
    rhs: Vec<f64>,
    x_base: Vec<f64>,
    right_norm: f64,
    threshold: f64,
    beta: f64,
    basis: Vec<Vec<f64>>,
    directions: Vec<Vec<f64>>,
    hessenberg: DenseMatrix,
    current_iterations: usize,
    residual_history: Vec<f64>,
    approximate: Vec<f64>,
    converged: bool,
    happy_breakdown: bool,
    start_counters: WorkCounters,
}

impl GmresPrefixSession {
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        op: &dyn LinearOperator,
        pc: &dyn Preconditioner,
        rhs: &[f64],
        x0: Option<&[f64]>,
        config: &GmresConfig,
        prefix_iterations: usize,
        counters: &mut WorkCounters,
    ) -> CoreResult<Self> {
        config.validate()?;
        let n = validate_system(op, pc, rhs, x0)?;
        let start_counters = *counters;
        let right_norm = safe_l2(rhs);
        let threshold = config.atol.max(config.rtol * right_norm);
        let x_base = x0.map_or_else(|| vec![0.0; n], ToOwned::to_owned);
        let mut residual = vec![0.0; n];
        if x_base.iter().all(|value| *value == 0.0) {
            residual.copy_from_slice(rhs);
        } else {
            let mut operator_output = vec![0.0; n];
            true_residual_into(
                op,
                rhs,
                &x_base,
                &mut operator_output,
                &mut residual,
                counters,
                ApplyCategory::Krylov,
            )?;
        }
        let initial_norm = safe_l2(&residual);
        if initial_norm <= threshold {
            return Ok(Self {
                config: config.clone(),
                rhs: rhs.to_vec(),
                x_base: x_base.clone(),
                right_norm,
                threshold,
                beta: initial_norm,
                basis: Vec::new(),
                directions: Vec::new(),
                hessenberg: DenseMatrix::zeros(1, 0),
                current_iterations: 0,
                residual_history: vec![initial_norm],
                approximate: x_base,
                converged: true,
                happy_breakdown: true,
                start_counters,
            });
        }
        let mut preconditioned = vec![0.0; n];
        apply_preconditioner(pc, &residual, &mut preconditioned, counters)?;
        let beta = safe_l2(&preconditioned);
        if !(beta > f64::MIN_POSITIVE && beta.is_finite()) {
            return Err(CoreError::LinearSolve(
                "GMRES prefix preconditioned residual breakdown".into(),
            ));
        }
        let maximum = config.max_arnoldi.min(n.max(1));
        let mut session = Self {
            config: config.clone(),
            rhs: rhs.to_vec(),
            x_base: x_base.clone(),
            right_norm,
            threshold,
            beta,
            basis: vec![preconditioned.iter().map(|value| value / beta).collect()],
            directions: Vec::with_capacity(maximum),
            hessenberg: DenseMatrix::zeros(maximum + 1, maximum),
            current_iterations: 0,
            residual_history: vec![beta],
            approximate: x_base,
            converged: false,
            happy_breakdown: false,
            start_counters,
        };
        let requested = prefix_iterations.min(maximum);
        for _ in 0..requested {
            if session.converged || session.happy_breakdown {
                break;
            }
            session.extend_one(op, pc, counters)?;
        }
        Ok(session)
    }

    fn update_approximation(&mut self) -> CoreResult<f64> {
        let m = self.current_iterations;
        if m == 0 {
            return Ok(self.beta);
        }
        let mut prefix = DenseMatrix::zeros(m + 1, m);
        for row in 0..=m {
            for column in 0..m {
                prefix[(row, column)] = self.hessenberg[(row, column)];
            }
        }
        let mut rhs_small = vec![0.0; m + 1];
        rhs_small[0] = self.beta;
        let coefficients = least_squares(&prefix, &rhs_small)?;
        let mut correction = vec![0.0; self.x_base.len()];
        linear_combination_into(&self.directions[..m], &coefficients, &mut correction)?;
        self.approximate.copy_from_slice(&self.x_base);
        for (value, delta) in self.approximate.iter_mut().zip(correction) {
            *value += delta;
        }
        let mut residual_small = rhs_small;
        for row in 0..=m {
            for column in 0..m {
                residual_small[row] -= prefix[(row, column)] * coefficients[column];
            }
        }
        Ok(safe_l2(&residual_small))
    }

    fn extend_one(
        &mut self,
        op: &dyn LinearOperator,
        pc: &dyn Preconditioner,
        counters: &mut WorkCounters,
    ) -> CoreResult<()> {
        let n = self.rhs.len();
        let column = self.current_iterations;
        let maximum = self.config.max_arnoldi.min(n.max(1));
        if column >= maximum {
            return Ok(());
        }
        if column >= self.basis.len() {
            return Err(CoreError::LinearSolve(
                "GMRES prefix basis exhausted before continuation".into(),
            ));
        }
        let direction = self.basis[column].clone();
        self.directions.push(direction.clone());
        let mut work = vec![0.0; n];
        let mut raw = vec![0.0; n];
        apply_left_with_raw(
            op,
            pc,
            &direction,
            &mut work,
            &mut raw,
            counters,
            ApplyCategory::Krylov,
        )?;
        let mut h_column = vec![0.0; column + 1];
        two_pass_mgs_into(&mut work, &self.basis[..=column], &mut h_column, counters)?;
        for (row, &value) in h_column.iter().enumerate() {
            self.hessenberg[(row, column)] = value;
        }
        let next_norm = safe_l2(&work);
        self.hessenberg[(column + 1, column)] = next_norm;
        let orthogonalization_scale = h_column.iter().map(|v| v.abs()).fold(0.0, f64::max);
        let breakdown_threshold = 100.0 * f64::EPSILON * (1.0 + orthogonalization_scale);
        self.happy_breakdown = next_norm <= breakdown_threshold;
        if !self.happy_breakdown && column + 1 < maximum {
            self.basis
                .push(work.iter().map(|value| value / next_norm).collect());
        }
        self.current_iterations += 1;
        counters.linear_iterations += 1;
        let small_residual = self.update_approximation()?;
        self.residual_history.push(small_residual);
        if small_residual <= self.threshold || self.happy_breakdown {
            let mut op_output = vec![0.0; n];
            let mut true_residual = vec![0.0; n];
            true_residual_into(
                op,
                &self.rhs,
                &self.approximate,
                &mut op_output,
                &mut true_residual,
                counters,
                ApplyCategory::Diagnostic,
            )?;
            self.converged = safe_l2(&true_residual) <= self.threshold;
        }
        Ok(())
    }

    pub fn prediction(&self) -> GmresPrefixPrediction {
        let maximum = self.config.max_arnoldi.min(self.rhs.len().max(1));
        let (predicted, contraction) = geometric_iteration_prediction(
            &self.residual_history,
            self.threshold,
            self.current_iterations,
            maximum,
        );
        GmresPrefixPrediction {
            prefix_iterations: self.current_iterations,
            predicted_total_iterations: predicted,
            residual_norm: self.residual_history.last().copied().unwrap_or(self.beta),
            target_residual_norm: self.threshold,
            observed_contraction: contraction,
        }
    }

    pub fn finish(
        mut self,
        op: &dyn LinearOperator,
        pc: &dyn Preconditioner,
        counters: &mut WorkCounters,
    ) -> CoreResult<LinearSolveReport> {
        let maximum = self.config.max_arnoldi.min(self.rhs.len().max(1));
        while !self.converged && self.current_iterations < maximum {
            self.extend_one(op, pc, counters)?;
            if self.happy_breakdown && !self.converged {
                break;
            }
        }
        let mut op_output = vec![0.0; self.rhs.len()];
        let mut residual = vec![0.0; self.rhs.len()];
        true_residual_into(
            op,
            &self.rhs,
            &self.approximate,
            &mut op_output,
            &mut residual,
            counters,
            ApplyCategory::Diagnostic,
        )?;
        let residual_norm = safe_l2(&residual);
        if !residual_norm.is_finite() || residual_norm > self.threshold {
            return Err(CoreError::LinearSolve(format!(
                "incremental GMRES exhausted {} Arnoldi vectors; true residual {residual_norm:.3e} exceeds {:.3e}",
                self.current_iterations, self.threshold
            )));
        }
        counters.linear_solves += 1;
        let delta = counters.delta(self.start_counters);
        Ok(LinearSolveReport {
            x: self.approximate,
            converged: true,
            info: 0,
            residual_norm,
            relative_residual: residual_norm / self.right_norm.max(f64::MIN_POSITIVE),
            iterations: self.current_iterations as u64,
            matvecs: delta.linear_matvecs,
            preconditioner_apps: delta.preconditioner_apps,
            method: "gmres-incremental-prefix".into(),
        })
    }
}

pub fn solve_gmres_incremental(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    GmresPrefixSession::begin(op, pc, rhs, x0, config, 0, counters)?.finish(op, pc, counters)
}
