//! LGMRES writing into caller storage (research node
//! `research/rev04_lgmres_into_20261003`, review DAG node REV-04).
//!
//! Same algorithm, arithmetic order, stopping rule, counters and state
//! commit rules as [`crate::solve_lgmres_with_workspace_and_residual_scale`].
//! What changes is where vectors live: the rollback snapshot of the carried
//! state is copied into reused buffers instead of cloned, vectors for new
//! images and directions come from a pool that dropped directions and
//! images return to, the warm start reuses its buffer, and the solution is
//! written into the caller's `output` only on success.

use crate::{
    common::{
        apply_left_with_raw, residual_threshold, selected_residual_norm, true_residual_into,
        validate_residual_scale, validate_system, validate_tolerances,
    },
    gmres::arnoldi_augmented_with_workspace,
    kernels::{axpy, normalize},
    lgmres::{LgmresConfig, LgmresState},
    workspace::LgmresWorkspace,
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, KrylovSystemIdentity, LinearOperator, Preconditioner,
    WorkCounters, apply_preconditioner, exact_krylov_system_identity, safe_l2,
};

/// A solve's report without the solution (which is in the caller's output).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LgmresIntoReport {
    pub residual_norm: f64,
    pub relative_residual: f64,
    pub iterations: u64,
    pub matvecs: u64,
    pub preconditioner_apps: u64,
}

/// Reused storage of [`solve_lgmres_into`]: the LGMRES workspace, the
/// rollback snapshot and a pool of spare vectors.
#[derive(Debug, Default)]
pub struct LgmresIntoWorkspace {
    inner: LgmresWorkspace,
    snapshot: Snapshot,
    pool: Vec<Vec<f64>>,
}

#[derive(Debug, Default)]
struct Snapshot {
    directions: Vec<Vec<f64>>,
    direction_count: usize,
    images: Vec<Vec<f64>>,
    image_present: Vec<bool>,
    operator_token: Option<u64>,
    system_identity: Option<KrylovSystemIdentity>,
    previous_solution: Vec<f64>,
    previous_present: bool,
    generation: u64,
}

fn copy_into(target: &mut Vec<f64>, source: &[f64]) {
    target.clear();
    target.extend_from_slice(source);
}

fn take_buffer(pool: &mut Vec<Vec<f64>>, source: &[f64]) -> Vec<f64> {
    let mut buffer = pool.pop().unwrap_or_default();
    copy_into(&mut buffer, source);
    buffer
}

impl Snapshot {
    fn save(&mut self, state: &LgmresState) {
        let count = state.directions.len();
        while self.directions.len() < count {
            self.directions.push(Vec::new());
        }
        while self.images.len() < state.images.len() {
            self.images.push(Vec::new());
            self.image_present.push(false);
        }
        for (index, direction) in state.directions.iter().enumerate() {
            copy_into(&mut self.directions[index], direction);
        }
        for index in 0..state.images.len() {
            match &state.images[index] {
                Some(image) => {
                    copy_into(&mut self.images[index], image);
                    self.image_present[index] = true;
                }
                None => self.image_present[index] = false,
            }
        }
        self.direction_count = count;
        self.operator_token = state.operator_token;
        self.system_identity = state.system_identity.clone();
        self.previous_present = state.previous_solution.is_some();
        if let Some(previous) = &state.previous_solution {
            copy_into(&mut self.previous_solution, previous);
        }
        self.generation = state.generation;
    }

    fn restore(&self, state: &mut LgmresState, pool: &mut Vec<Vec<f64>>, images_len: usize) {
        while state.directions.len() > self.direction_count {
            pool.push(state.directions.pop().unwrap());
        }
        while state.directions.len() < self.direction_count {
            state.directions.push(pool.pop().unwrap_or_default());
        }
        for (index, direction) in state.directions.iter_mut().enumerate() {
            copy_into(direction, &self.directions[index]);
        }
        while state.images.len() > images_len {
            if let Some(Some(image)) = state.images.pop() {
                pool.push(image);
            }
        }
        while state.images.len() < images_len {
            state.images.push(None);
        }
        for index in 0..images_len {
            let present = index < self.image_present.len() && self.image_present[index];
            match (&mut state.images[index], present) {
                (Some(image), true) => copy_into(image, &self.images[index]),
                (slot @ None, true) => {
                    *slot = Some(take_buffer(pool, &self.images[index]));
                }
                (slot @ Some(_), false) => {
                    if let Some(image) = slot.take() {
                        pool.push(image);
                    }
                }
                (None, false) => {}
            }
        }
        state.operator_token = self.operator_token;
        state.system_identity = self.system_identity.clone();
        match (&mut state.previous_solution, self.previous_present) {
            (Some(previous), true) => copy_into(previous, &self.previous_solution),
            (slot @ None, true) => *slot = Some(take_buffer(pool, &self.previous_solution)),
            (slot @ Some(_), false) => {
                if let Some(previous) = slot.take() {
                    pool.push(previous);
                }
            }
            (None, false) => {}
        }
        state.generation = self.generation;
    }
}

/// [`crate::solve_lgmres_with_workspace_and_residual_scale`] writing the
/// solution into `output` (length `n`), and only on success. On failure the
/// carried state is restored to its value before the call.
#[allow(clippy::too_many_arguments)]
pub fn solve_lgmres_into(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &LgmresConfig,
    state: &mut LgmresState,
    residual_scale: Option<&[f64]>,
    output: &mut [f64],
    workspace: &mut LgmresIntoWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<LgmresIntoReport> {
    if config.inner_m == 0 || config.max_outer == 0 || config.outer_k == 0 {
        return Err(CoreError::InvalidInput(
            "LGMRES iteration limits must be positive".into(),
        ));
    }
    validate_tolerances("LGMRES", config.rtol, config.atol)?;
    let n = validate_system(op, pc, rhs, x0)?;
    if output.len() != n {
        return Err(CoreError::Dimension("LGMRES into: output length".into()));
    }
    validate_residual_scale(residual_scale, n)?;
    let right_norm = selected_residual_norm(rhs, residual_scale)?;
    let threshold = residual_threshold("LGMRES", config.rtol, config.atol, right_norm)?;
    let before = *counters;
    let images_len_before = state.images.len();
    workspace.snapshot.save(state);
    let system_identity = exact_krylov_system_identity(op, pc);
    let LgmresIntoWorkspace { inner, pool, .. } = workspace;
    let result = (|| {
        let same_system =
            system_identity.is_some() && state.system_identity.as_ref() == system_identity.as_ref();
        if !same_system {
            for image in &mut state.images {
                if let Some(old) = image.take() {
                    pool.push(old);
                }
            }
            if let Some(old) = state.previous_solution.take() {
                pool.push(old);
            }
            state.operator_token = Some(op.token());
            state.system_identity = system_identity.clone();
        }
        // As the existing solve: retain the directions of length n, then
        // truncate the images to their count (the removed vectors return to
        // the pool instead of being freed).
        let mut index = 0;
        while index < state.directions.len() {
            if state.directions[index].len() == n {
                index += 1;
            } else {
                pool.push(state.directions.remove(index));
            }
        }
        while state.images.len() > state.directions.len() {
            if let Some(Some(old)) = state.images.pop() {
                pool.push(old);
            }
        }
        while state.images.len() < state.directions.len() {
            state.images.push(None);
        }

        inner.common.prepare(n);
        if let Some(initial) = x0.or(state.previous_solution.as_deref()) {
            if initial.len() != n {
                return Err(CoreError::Dimension(format!(
                    "LGMRES warm start has length {} but the system has dimension {n}",
                    initial.len()
                )));
            }
            inner.common.x.copy_from_slice(initial);
        }
        let mut total = 0usize;
        for _ in 0..config.max_outer {
            if inner.common.x.iter().all(|value| *value == 0.0) {
                inner.common.residual.copy_from_slice(rhs);
            } else {
                true_residual_into(
                    op,
                    rhs,
                    &inner.common.x,
                    &mut inner.common.operator_output,
                    &mut inner.common.residual,
                    counters,
                    ApplyCategory::Krylov,
                )?;
            }
            if selected_residual_norm(&inner.common.residual, residual_scale)? <= threshold {
                break;
            }
            apply_preconditioner(
                pc,
                &inner.common.residual,
                &mut inner.common.preconditioned,
                counters,
            )?;
            let beta = safe_l2(&inner.common.preconditioned);
            if beta <= f64::MIN_POSITIVE {
                return Err(CoreError::LinearSolve("LGMRES residual breakdown".into()));
            }
            for index in 0..state.directions.len() {
                if state.images[index].is_none() {
                    let mut image = pool.pop().unwrap_or_default();
                    image.clear();
                    image.resize(n, 0.0);
                    apply_left_with_raw(
                        op,
                        pc,
                        &state.directions[index],
                        &mut image,
                        &mut inner.common.scratch_b,
                        counters,
                        ApplyCategory::Refresh,
                    )?;
                    state.images[index] = Some(image);
                }
            }
            let arnoldi = arnoldi_augmented_with_workspace(
                op,
                pc,
                &inner.common.preconditioned,
                beta,
                config.inner_m.min(n.max(1)),
                &state.directions,
                &state.images,
                counters,
                &mut inner.arnoldi,
            )?;
            let norm = normalize(&mut inner.arnoldi.correction)?;
            if norm > 0.0 {
                apply_left_with_raw(
                    op,
                    pc,
                    &inner.arnoldi.correction,
                    &mut inner.common.scratch_a,
                    &mut inner.common.scratch_b,
                    counters,
                    ApplyCategory::Refresh,
                )?;
                let direction = take_buffer(pool, &inner.arnoldi.correction);
                let image = take_buffer(pool, &inner.common.scratch_a);
                state.directions.push(direction);
                state.images.push(Some(image));
                while state.directions.len() > config.outer_k {
                    pool.push(state.directions.remove(0));
                    if let Some(old) = state.images.remove(0) {
                        pool.push(old);
                    }
                }
                counters.recycle_updates += 1;
            }
            for value in &mut inner.arnoldi.correction {
                *value *= norm;
            }
            axpy(
                1.0,
                &inner.arnoldi.correction,
                &mut inner.common.x,
                counters,
            )?;
            total += arnoldi.iterations;
            counters.linear_iterations += arnoldi.iterations as u64;
        }
        true_residual_into(
            op,
            rhs,
            &inner.common.x,
            &mut inner.common.operator_output,
            &mut inner.common.residual,
            counters,
            ApplyCategory::Diagnostic,
        )?;
        let residual_norm = selected_residual_norm(&inner.common.residual, residual_scale)?;
        if !residual_norm.is_finite() || residual_norm > threshold {
            return Err(CoreError::LinearSolve(format!(
                "LGMRES true residual {residual_norm:.3e} exceeds {threshold:.3e}"
            )));
        }
        match &mut state.previous_solution {
            Some(previous) => copy_into(previous, &inner.common.x),
            slot @ None => *slot = Some(take_buffer(pool, &inner.common.x)),
        }
        state.operator_token = Some(op.token());
        state.system_identity = system_identity.clone();
        state.generation += 1;
        counters.linear_solves += 1;
        let delta = counters.delta(before);
        output.copy_from_slice(&inner.common.x);
        Ok(LgmresIntoReport {
            residual_norm,
            relative_residual: residual_norm / right_norm.max(f64::MIN_POSITIVE),
            iterations: total as u64,
            matvecs: delta.linear_matvecs,
            preconditioner_apps: delta.preconditioner_apps,
        })
    })();
    if result.is_err() {
        workspace
            .snapshot
            .restore(state, &mut workspace.pool, images_len_before);
    }
    result
}
