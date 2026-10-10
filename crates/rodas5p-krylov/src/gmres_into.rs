//! GMRES writing into caller storage with a declared workspace capacity
//! (research node `research/rnext02_gmres_into_20261003`, remaining-only DAG
//! node R-NEXT-02).
//!
//! Same algorithm, stopping rule, true-residual checks and work counters as
//! [`crate::solve_gmres_with_workspace_and_residual_scale`]. Two things
//! differ. The small least-squares problem is solved once per cycle (the
//! existing loop solves it after every column but uses only the last
//! solution), on the same Hessenberg prefix with the same routine, so the
//! correction is bitwise the same. And the solution is written into `output`
//! only on success. One consequence is stated in the node's
//! preregistration: an intermediate least-squares solution that would have
//! been non-finite is never formed, so it cannot fail the solve; the final
//! one and the final true-residual check are unchanged.

use crate::{
    common::{
        apply_left_with_raw, residual_threshold, selected_residual_norm, true_residual_into,
        validate_residual_scale, validate_system,
    },
    gmres::{GmresConfig, arnoldi_happy_breakdown},
    kernels::{axpy, linear_combination_into, normalize, two_pass_mgs_into},
    small::least_squares,
    workspace::GmresWorkspace,
};
use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, LinearOperator, Preconditioner, WorkCounters,
    apply_preconditioner, safe_l2,
};

/// What a solve may do when the workspace is smaller than it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapacityGrowth {
    /// Fail with `KRYLOV_CAPACITY_EXCEEDED` before any operator application.
    Refuse,
    /// Grow the workspace and report it in [`GmresIntoReport::workspace_grew`].
    Allow,
}

/// The largest system a workspace is declared for: dimension and Arnoldi
/// columns per cycle (`min(restart, max_arnoldi, dimension)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GmresCapacity {
    pub max_dimension: usize,
    pub max_columns: usize,
    pub growth: CapacityGrowth,
}

impl GmresCapacity {
    /// Unbounded: any size, growth allowed and reported.
    pub fn unbounded() -> Self {
        Self {
            max_dimension: usize::MAX,
            max_columns: usize::MAX,
            growth: CapacityGrowth::Allow,
        }
    }
}

/// A solve's report without the solution (which is in the caller's output).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GmresIntoReport {
    pub residual_norm: f64,
    pub relative_residual: f64,
    pub iterations: u64,
    pub matvecs: u64,
    pub preconditioner_apps: u64,
    pub cycles: u64,
    /// Small least-squares solves (one per cycle; the existing loop does one
    /// per Arnoldi column).
    pub least_squares_solves: u64,
    /// The workspace's capacity grew during this solve.
    pub workspace_grew: bool,
    pub method: &'static str,
}

impl GmresWorkspace {
    /// Size the workspace for systems up to `dimension` with up to `columns`
    /// Arnoldi columns per cycle, so later solves within that size do not
    /// grow it.
    pub fn reserve(&mut self, dimension: usize, columns: usize) -> CoreResult<()> {
        self.common.prepare(dimension);
        self.arnoldi.prepare(dimension, columns)?;
        self.arnoldi
            .hessenberg_prefix
            .resize_zeros(columns + 1, columns)?;
        self.arnoldi.hessenberg_prefix.resize_zeros(0, 0)?;
        Ok(())
    }
}

/// Options of [`solve_gmres_into_with_options`]. The default is
/// [`solve_gmres_into`] exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GmresIntoOptions {
    /// Skip the final diagnostic true residual (research node
    /// `research/alg04_coupled_target_v2_20261010`, arm `DupFix`). The loop
    /// leaves only after a true residual of the same iterate met the
    /// threshold (computed in the Krylov category, or `rhs` itself for a
    /// zero iterate), so the final residual repeats it: skipping it changes
    /// no solution bit, only the counters (one diagnostic operator
    /// application per successful solve). The report then carries the
    /// loop's residual norm.
    pub skip_final_residual: bool,
}

/// GMRES into caller storage; see the module documentation. `output` is
/// written only when the solve succeeds. Rust's borrow rules keep `output`
/// from aliasing `rhs`, `x0` or the workspace.
#[allow(clippy::too_many_arguments)]
pub fn solve_gmres_into(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    residual_scale: Option<&[f64]>,
    output: &mut [f64],
    workspace: &mut GmresWorkspace,
    capacity: GmresCapacity,
    counters: &mut WorkCounters,
) -> CoreResult<GmresIntoReport> {
    solve_gmres_into_with_options(
        op,
        pc,
        rhs,
        x0,
        config,
        residual_scale,
        output,
        workspace,
        capacity,
        GmresIntoOptions::default(),
        counters,
    )
}

/// [`solve_gmres_into`] with [`GmresIntoOptions`].
#[allow(clippy::too_many_arguments)]
pub fn solve_gmres_into_with_options(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    residual_scale: Option<&[f64]>,
    output: &mut [f64],
    workspace: &mut GmresWorkspace,
    capacity: GmresCapacity,
    options: GmresIntoOptions,
    counters: &mut WorkCounters,
) -> CoreResult<GmresIntoReport> {
    config.validate()?;
    let n = validate_system(op, pc, rhs, x0)?;
    validate_residual_scale(residual_scale, n)?;
    if output.len() != n {
        return Err(CoreError::Dimension(
            "GMRES output length differs from the system dimension".into(),
        ));
    }
    let columns = config.restart.min(config.max_arnoldi).min(n.max(1));
    if capacity.growth == CapacityGrowth::Refuse
        && (n > capacity.max_dimension || columns > capacity.max_columns)
    {
        return Err(CoreError::InvalidInput(format!(
            "KRYLOV_CAPACITY_EXCEEDED: GMRES needs dimension {n} and {columns} columns, \
             the workspace is declared for {} and {}",
            capacity.max_dimension, capacity.max_columns
        )));
    }
    let capacity_before = workspace.capacity_f64();
    let before = *counters;
    let right_norm = selected_residual_norm(rhs, residual_scale)?;
    let threshold = residual_threshold("GMRES", config.rtol, config.atol, right_norm)?;
    workspace.common.prepare(n);
    if let Some(initial) = x0 {
        workspace.common.x.copy_from_slice(initial);
    }

    let (mut total, mut cycles) = (0usize, 0u64);
    let loop_residual_norm = loop {
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
        let residual_norm = selected_residual_norm(&workspace.common.residual, residual_scale)?;
        if residual_norm <= threshold {
            break residual_norm;
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
        let iterations = cycle(op, pc, beta, steps, workspace, counters)?;
        axpy(
            1.0,
            &workspace.arnoldi.correction,
            &mut workspace.common.x,
            counters,
        )?;
        total += iterations;
        cycles += 1;
        counters.linear_iterations += iterations as u64;
    };

    let residual_norm = if options.skip_final_residual {
        loop_residual_norm
    } else {
        true_residual_into(
            op,
            rhs,
            &workspace.common.x,
            &mut workspace.common.operator_output,
            &mut workspace.common.residual,
            counters,
            ApplyCategory::Diagnostic,
        )?;
        selected_residual_norm(&workspace.common.residual, residual_scale)?
    };
    if !residual_norm.is_finite() || residual_norm > threshold {
        return Err(CoreError::LinearSolve(format!(
            "GMRES true residual {residual_norm:.3e} exceeds {threshold:.3e}"
        )));
    }
    counters.linear_solves += 1;
    let delta = counters.delta(before);
    output.copy_from_slice(&workspace.common.x);
    Ok(GmresIntoReport {
        residual_norm,
        relative_residual: residual_norm / right_norm.max(f64::MIN_POSITIVE),
        iterations: total as u64,
        matvecs: delta.linear_matvecs,
        preconditioner_apps: delta.preconditioner_apps,
        cycles,
        least_squares_solves: cycles,
        workspace_grew: workspace.capacity_f64() > capacity_before,
        method: "gmres-into",
    })
}

/// One Arnoldi cycle from the preconditioned residual in
/// `workspace.common.preconditioned` (norm `beta`); the correction is left in
/// `workspace.arnoldi.correction`. The operations and their order are those
/// of `arnoldi_augmented_with_workspace` without augmentation, except that
/// the small least-squares problem is solved once, after the last column.
fn cycle(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    beta: f64,
    steps: usize,
    workspace: &mut GmresWorkspace,
    counters: &mut WorkCounters,
) -> CoreResult<usize> {
    let arnoldi = &mut workspace.arnoldi;
    let n = workspace.common.preconditioned.len();
    arnoldi.prepare(n, steps)?;
    arnoldi.basis[0].copy_from_slice(&workspace.common.preconditioned);
    if normalize(&mut arnoldi.basis[0])? == 0.0 {
        return Err(CoreError::LinearSolve("zero Arnoldi residual".into()));
    }
    let mut actual = 0usize;
    for j in 0..steps {
        let (previous_basis, remaining_basis) = arnoldi.basis.split_at_mut(j + 1);
        let w = &mut remaining_basis[0];
        apply_left_with_raw(
            op,
            pc,
            &previous_basis[j],
            w,
            &mut arnoldi.raw_operator_output,
            counters,
            ApplyCategory::Krylov,
        )?;
        two_pass_mgs_into(w, previous_basis, &mut arnoldi.h_column, counters)?;
        for i in 0..previous_basis.len() {
            arnoldi.hessenberg[(i, j)] = arnoldi.h_column[i];
        }
        let h_next = safe_l2(w);
        arnoldi.hessenberg[(j + 1, j)] = h_next;
        actual = j + 1;
        let happy_breakdown =
            arnoldi_happy_breakdown(&arnoldi.h_column[..previous_basis.len()], h_next)?;
        if happy_breakdown {
            w.fill(0.0);
            break;
        }
        for value in w {
            *value /= h_next;
        }
    }
    arnoldi.hessenberg_prefix.resize_zeros(actual + 1, actual)?;
    for row in 0..actual + 1 {
        for column in 0..actual {
            arnoldi.hessenberg_prefix[(row, column)] = arnoldi.hessenberg[(row, column)];
        }
    }
    arnoldi.rhs_small[..actual + 1].fill(0.0);
    arnoldi.rhs_small[0] = beta;
    match workspace.least_squares.as_deref_mut() {
        // Research node `research/spd04_ls_workspace_20261007`: the same
        // faer kernels on reused buffers, bitwise the same solution.
        Some(small) => {
            let solution = &mut workspace.least_squares_solution;
            small.solve_into(
                &arnoldi.hessenberg_prefix,
                &arnoldi.rhs_small[..actual + 1],
                solution,
            )?;
            linear_combination_into(&arnoldi.basis[..actual], solution, &mut arnoldi.correction)?;
        }
        None => {
            let solution =
                least_squares(&arnoldi.hessenberg_prefix, &arnoldi.rhs_small[..actual + 1])?;
            linear_combination_into(&arnoldi.basis[..actual], &solution, &mut arnoldi.correction)?;
        }
    }
    Ok(actual)
}
