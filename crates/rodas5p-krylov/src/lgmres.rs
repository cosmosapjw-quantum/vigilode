use crate::{
    common::{
        apply_left_with_raw, selected_residual_norm, true_residual_into, validate_residual_scale,
        validate_system,
    },
    gmres::arnoldi_augmented_with_workspace,
    kernels::{axpy, normalize},
    workspace::LgmresWorkspace,
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, KrylovSystemIdentity, LinearOperator, LinearSolveReport,
    Preconditioner, WorkCounters, apply_preconditioner, exact_krylov_system_identity, safe_l2,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LgmresConfig {
    pub inner_m: usize,
    pub max_outer: usize,
    pub outer_k: usize,
    pub rtol: f64,
    pub atol: f64,
}
impl Default for LgmresConfig {
    fn default() -> Self {
        Self {
            inner_m: 30,
            max_outer: 20,
            outer_k: 8,
            rtol: 1e-11,
            atol: 1e-13,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LgmresState {
    pub directions: Vec<Vec<f64>>,
    pub images: Vec<Option<Vec<f64>>>,
    pub operator_token: Option<u64>,
    pub system_identity: Option<KrylovSystemIdentity>,
    pub previous_solution: Option<Vec<f64>>,
    pub generation: u64,
}

// State, workspace, and work ledger have deliberately distinct lifetimes and commit rules.
#[allow(clippy::too_many_arguments)]
pub fn solve_lgmres_with_workspace_and_residual_scale(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &LgmresConfig,
    state: &mut LgmresState,
    residual_scale: Option<&[f64]>,
    workspace: &mut LgmresWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    if config.inner_m == 0 || config.max_outer == 0 || config.outer_k == 0 {
        return Err(CoreError::InvalidInput(
            "LGMRES iteration limits must be positive".into(),
        ));
    }
    let n = validate_system(op, pc, rhs, x0)?;
    validate_residual_scale(residual_scale, n)?;
    let before = *counters;
    let snapshot = state.clone();
    let system_identity = exact_krylov_system_identity(op, pc);
    let result = (|| {
        let same_system =
            system_identity.is_some() && state.system_identity.as_ref() == system_identity.as_ref();
        if !same_system {
            for image in &mut state.images {
                *image = None;
            }
            state.previous_solution = None;
            state.operator_token = Some(op.token());
            state.system_identity = system_identity.clone();
        }
        state.directions.retain(|vector| vector.len() == n);
        state.images.truncate(state.directions.len());
        while state.images.len() < state.directions.len() {
            state.images.push(None);
        }

        workspace.common.prepare(n);
        if let Some(initial) = x0.or(state.previous_solution.as_deref()) {
            workspace.common.x.copy_from_slice(initial);
        }
        let right_norm = selected_residual_norm(rhs, residual_scale)?;
        let threshold = config.atol.max(config.rtol * right_norm);
        let mut total = 0usize;
        for _ in 0..config.max_outer {
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
            if selected_residual_norm(&workspace.common.residual, residual_scale)? <= threshold {
                break;
            }
            apply_preconditioner(
                pc,
                &workspace.common.residual,
                &mut workspace.common.preconditioned,
                counters,
            )?;
            let beta = safe_l2(&workspace.common.preconditioned);
            if beta <= f64::MIN_POSITIVE {
                return Err(CoreError::LinearSolve("LGMRES residual breakdown".into()));
            }

            for index in 0..state.directions.len() {
                if state.images[index].is_none() {
                    let mut image = vec![0.0; n];
                    apply_left_with_raw(
                        op,
                        pc,
                        &state.directions[index],
                        &mut image,
                        &mut workspace.common.scratch_b,
                        counters,
                        ApplyCategory::Refresh,
                    )?;
                    state.images[index] = Some(image);
                }
            }

            let arnoldi = arnoldi_augmented_with_workspace(
                op,
                pc,
                &workspace.common.preconditioned,
                beta,
                config.inner_m.min(n.max(1)),
                &state.directions,
                &state.images,
                counters,
                &mut workspace.arnoldi,
            )?;
            let norm = normalize(&mut workspace.arnoldi.correction)?;
            if norm > 0.0 {
                apply_left_with_raw(
                    op,
                    pc,
                    &workspace.arnoldi.correction,
                    &mut workspace.common.scratch_a,
                    &mut workspace.common.scratch_b,
                    counters,
                    ApplyCategory::Refresh,
                )?;
                state.directions.push(workspace.arnoldi.correction.clone());
                state.images.push(Some(workspace.common.scratch_a.clone()));
                while state.directions.len() > config.outer_k {
                    state.directions.remove(0);
                    state.images.remove(0);
                }
                counters.recycle_updates += 1;
            }
            for value in &mut workspace.arnoldi.correction {
                *value *= norm;
            }
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
        let residual_norm = selected_residual_norm(&workspace.common.residual, residual_scale)?;
        if !residual_norm.is_finite() || residual_norm > threshold {
            return Err(CoreError::LinearSolve(format!(
                "LGMRES true residual {residual_norm:.3e} exceeds {threshold:.3e}"
            )));
        }
        state.previous_solution = Some(workspace.common.x.clone());
        state.operator_token = Some(op.token());
        state.system_identity = system_identity;
        state.generation += 1;
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
            method: "lgmres".into(),
        })
    })();
    if result.is_err() {
        *state = snapshot;
    }
    result
}

#[allow(clippy::too_many_arguments)]
pub fn solve_lgmres_with_workspace(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &LgmresConfig,
    state: &mut LgmresState,
    workspace: &mut LgmresWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_lgmres_with_workspace_and_residual_scale(
        op, pc, rhs, x0, config, state, None, workspace, counters,
    )
}

pub fn solve_lgmres(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &LgmresConfig,
    state: &mut LgmresState,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_lgmres_with_workspace(
        op,
        pc,
        rhs,
        x0,
        config,
        state,
        &mut LgmresWorkspace::default(),
        counters,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn solve_lgmres_with_residual_scale(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &LgmresConfig,
    state: &mut LgmresState,
    residual_scale: Option<&[f64]>,
    counters: &mut WorkCounters,
) -> CoreResult<LinearSolveReport> {
    solve_lgmres_with_workspace_and_residual_scale(
        op,
        pc,
        rhs,
        x0,
        config,
        state,
        residual_scale,
        &mut LgmresWorkspace::default(),
        counters,
    )
}
