use rodas5p_core::{
    ApplyCategory, CoreError, CoreResult, LinearOperator, Preconditioner, WorkCounters,
    apply_counted, apply_preconditioner,
};

pub fn validate_system(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    rhs: &[f64],
    x0: Option<&[f64]>,
) -> CoreResult<usize> {
    let n = op.dimension();
    if pc.dimension() != n || rhs.len() != n || x0.is_some_and(|x| x.len() != n) {
        return Err(CoreError::Dimension("Krylov system shape mismatch".into()));
    }
    if !rhs.iter().all(|v| v.is_finite()) || x0.is_some_and(|x| !x.iter().all(|v| v.is_finite())) {
        return Err(CoreError::NonFinite("Krylov input contains NaN/Inf".into()));
    }
    Ok(n)
}

pub fn apply_left_with_raw(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    x: &[f64],
    out: &mut [f64],
    raw: &mut [f64],
    counters: &mut WorkCounters,
    category: ApplyCategory,
) -> CoreResult<()> {
    if raw.len() != out.len() {
        return Err(CoreError::Dimension(
            "left-preconditioned operator scratch mismatch".into(),
        ));
    }
    apply_counted(op, x, raw, counters, category)?;
    apply_preconditioner(pc, raw, out, counters)
}

pub fn apply_left(
    op: &dyn LinearOperator,
    pc: &dyn Preconditioner,
    x: &[f64],
    out: &mut [f64],
    counters: &mut WorkCounters,
    category: ApplyCategory,
) -> CoreResult<()> {
    let mut raw = vec![0.0; out.len()];
    apply_left_with_raw(op, pc, x, out, &mut raw, counters, category)
}

pub fn true_residual_into(
    op: &dyn LinearOperator,
    rhs: &[f64],
    x: &[f64],
    operator_output: &mut [f64],
    residual: &mut [f64],
    counters: &mut WorkCounters,
    category: ApplyCategory,
) -> CoreResult<()> {
    if operator_output.len() != rhs.len() || residual.len() != rhs.len() {
        return Err(CoreError::Dimension("residual scratch mismatch".into()));
    }
    apply_counted(op, x, operator_output, counters, category)?;
    for i in 0..rhs.len() {
        residual[i] = rhs[i] - operator_output[i];
    }
    Ok(())
}
