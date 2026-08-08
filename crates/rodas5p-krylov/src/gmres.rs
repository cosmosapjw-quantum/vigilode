use crate::{
    common::{apply_left_with_raw, true_residual_into, validate_system},
    kernels::{axpy, linear_combination_into, normalize, two_pass_mgs_into},
    small::least_squares,
    workspace::{ArnoldiWorkspace, GmresWorkspace},
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, LinearOperator, LinearSolveReport, Preconditioner,
    WorkCounters, apply_preconditioner, safe_l2,
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
