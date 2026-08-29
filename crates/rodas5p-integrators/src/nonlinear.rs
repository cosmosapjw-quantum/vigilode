use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, WorkCounters, error_scale, safe_l2, wrms,
};

#[derive(Clone, Debug)]
pub struct NewtonConfig {
    pub atol: f64,
    pub rtol: f64,
    pub max_iterations: usize,
    pub max_backtracks: usize,
    pub minimum_damping: f64,
    /// Maximum number of bounded modified-Newton Jacobian refreshes after the
    /// initial factorization.
    pub max_jacobian_refreshes: usize,
    /// Refresh when one accepted Newton update contracts the residual by no
    /// more than this ratio. Must lie in `(0, 1)`.
    pub stagnation_ratio: f64,
}

impl Default for NewtonConfig {
    fn default() -> Self {
        Self {
            atol: 1.0e-12,
            rtol: 1.0e-10,
            max_iterations: 12,
            max_backtracks: 8,
            minimum_damping: 1.0 / 256.0,
            max_jacobian_refreshes: 1,
            stagnation_ratio: 0.9,
        }
    }
}

impl NewtonConfig {
    pub fn validate(&self) -> CoreResult<()> {
        if !(self.atol > 0.0 && self.atol.is_finite()) {
            return Err(CoreError::InvalidInput(
                "Newton atol must be finite and positive".into(),
            ));
        }
        if !(self.rtol >= 0.0 && self.rtol.is_finite()) {
            return Err(CoreError::InvalidInput(
                "Newton rtol must be finite and nonnegative".into(),
            ));
        }
        if self.max_iterations == 0 || self.max_backtracks == 0 {
            return Err(CoreError::InvalidInput(
                "Newton iteration limits must be positive".into(),
            ));
        }
        if !(self.minimum_damping > 0.0 && self.minimum_damping <= 1.0) {
            return Err(CoreError::InvalidInput(
                "Newton minimum damping must lie in (0,1]".into(),
            ));
        }
        if !(self.stagnation_ratio > 0.0
            && self.stagnation_ratio < 1.0
            && self.stagnation_ratio.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "Newton stagnation ratio must lie in (0,1)".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct NewtonReport {
    pub x: Vec<f64>,
    pub converged: bool,
    pub iterations: usize,
    pub residual_wrms: f64,
    pub correction_wrms: f64,
    pub damping: f64,
    /// Total Jacobian builds, including the initial build.
    pub jacobian_evaluations: usize,
    /// Number of bounded rebuilds after the initial frozen-Jacobian build.
    pub jacobian_refreshes: usize,
    pub line_search_refreshes: usize,
    pub stagnation_refreshes: usize,
}

fn scaled_wrms(
    values: &[f64],
    reference: &[f64],
    x: &[f64],
    config: &NewtonConfig,
) -> CoreResult<f64> {
    let scale = error_scale(reference, x, &[config.atol], config.rtol)?;
    wrms(values, &scale)
}

pub fn solve_dense_newton<R, J>(
    initial: &[f64],
    reference: &[f64],
    config: &NewtonConfig,
    counters: &mut WorkCounters,
    mut residual: R,
    mut jacobian: J,
) -> CoreResult<NewtonReport>
where
    R: FnMut(&[f64], &mut WorkCounters) -> CoreResult<Vec<f64>>,
    J: FnMut(&[f64], &mut WorkCounters) -> CoreResult<DenseMatrix>,
{
    config.validate()?;
    if initial.is_empty() || initial.len() != reference.len() {
        return Err(CoreError::Dimension(
            "Newton initial/reference shape mismatch".into(),
        ));
    }
    if !initial
        .iter()
        .chain(reference)
        .all(|value| value.is_finite())
    {
        return Err(CoreError::NonFinite(
            "Newton initial/reference contains NaN/Inf".into(),
        ));
    }

    counters.nonlinear_solves += 1;
    let mut x = initial.to_vec();
    counters.nonlinear_residual_evaluations += 1;
    let mut r = match residual(&x, counters) {
        Ok(residual) => residual,
        Err(error) => {
            counters.nonlinear_failures += 1;
            return Err(error);
        }
    };
    if r.len() != x.len() || !r.iter().all(|value| value.is_finite()) {
        counters.nonlinear_failures += 1;
        return Err(CoreError::NonFinite(
            "Newton residual is non-finite or has wrong shape".into(),
        ));
    }
    let mut residual_wrms = scaled_wrms(&r, reference, &x, config)?;
    if residual_wrms <= 1.0 {
        return Ok(NewtonReport {
            x,
            converged: true,
            iterations: 0,
            residual_wrms,
            correction_wrms: 0.0,
            damping: 1.0,
            jacobian_evaluations: 0,
            jacobian_refreshes: 0,
            line_search_refreshes: 0,
            stagnation_refreshes: 0,
        });
    }

    let mut factorize = |at: &[f64], counters: &mut WorkCounters| {
        counters.nonlinear_jacobian_evaluations += 1;
        let matrix = jacobian(at, counters)?;
        if matrix.nrows() != at.len() || matrix.ncols() != at.len() {
            return Err(CoreError::Dimension(
                "Newton Jacobian shape mismatch".into(),
            ));
        }
        counters.direct_factorizations += 1;
        LuFactorization::new(&matrix)
    };
    let mut factor = match factorize(&x, counters) {
        Ok(factor) => factor,
        Err(error) => {
            counters.nonlinear_failures += 1;
            return Err(error);
        }
    };
    let mut jacobian_evaluations = 1_usize;
    let mut jacobian_refreshes = 0_usize;
    let mut line_search_refreshes = 0_usize;
    let mut stagnation_refreshes = 0_usize;

    let mut correction_wrms = f64::INFINITY;
    let mut last_damping = 1.0;
    for iteration in 1..=config.max_iterations {
        let rhs: Vec<f64> = r.iter().map(|value| -value).collect();
        counters.direct_solve_calls += 1;
        counters.linear_solves += 1;
        let delta = match factor.solve(&rhs) {
            Ok(delta) => delta,
            Err(error) => {
                counters.nonlinear_failures += 1;
                return Err(error);
            }
        };
        if !delta.iter().all(|value| value.is_finite()) {
            counters.nonlinear_failures += 1;
            return Err(CoreError::NonFinite(
                "Newton correction contains NaN/Inf".into(),
            ));
        }

        let previous_residual = residual_wrms;
        let mut damping = 1.0;
        let mut accepted = None;
        for _ in 0..config.max_backtracks {
            let trial: Vec<f64> = x
                .iter()
                .zip(&delta)
                .map(|(value, correction)| value + damping * correction)
                .collect();
            counters.nonlinear_residual_evaluations += 1;
            let trial_r = match residual(&trial, counters) {
                Ok(residual) => residual,
                Err(error) => {
                    counters.nonlinear_failures += 1;
                    return Err(error);
                }
            };
            if trial_r.len() != x.len() || !trial_r.iter().all(|value| value.is_finite()) {
                damping *= 0.5;
                if damping < config.minimum_damping {
                    break;
                }
                continue;
            }
            let trial_norm = scaled_wrms(&trial_r, reference, &trial, config)?;
            if trial_norm < previous_residual || trial_norm <= 1.0 {
                accepted = Some((trial, trial_r, trial_norm));
                break;
            }
            damping *= 0.5;
            if damping < config.minimum_damping {
                break;
            }
        }
        let Some((trial, trial_r, trial_norm)) = accepted else {
            counters.nonlinear_iterations += 1;
            if jacobian_refreshes < config.max_jacobian_refreshes {
                factor = match factorize(&x, counters) {
                    Ok(factor) => factor,
                    Err(error) => {
                        counters.nonlinear_failures += 1;
                        return Err(error);
                    }
                };
                jacobian_evaluations += 1;
                jacobian_refreshes += 1;
                line_search_refreshes += 1;
                continue;
            }
            counters.nonlinear_failures += 1;
            return Err(CoreError::NonlinearSolve(
                "Newton line search failed to reduce the residual".into(),
            ));
        };
        let scaled_delta: Vec<f64> = delta.iter().map(|value| damping * value).collect();
        correction_wrms = scaled_wrms(&scaled_delta, reference, &trial, config)?;
        x = trial;
        r = trial_r;
        residual_wrms = trial_norm;
        last_damping = damping;
        counters.nonlinear_iterations += 1;
        let roundoff_residual =
            safe_l2(&r) <= 64.0 * f64::EPSILON * (1.0 + safe_l2(reference) + safe_l2(&x));
        if residual_wrms <= 1.0 && (correction_wrms <= 1.0 || roundoff_residual) {
            return Ok(NewtonReport {
                x,
                converged: true,
                iterations: iteration,
                residual_wrms,
                correction_wrms,
                damping: last_damping,
                jacobian_evaluations,
                jacobian_refreshes,
                line_search_refreshes,
                stagnation_refreshes,
            });
        }
        let contraction_ratio = residual_wrms / previous_residual;
        if contraction_ratio >= config.stagnation_ratio
            && iteration < config.max_iterations
            && jacobian_refreshes < config.max_jacobian_refreshes
        {
            factor = match factorize(&x, counters) {
                Ok(factor) => factor,
                Err(error) => {
                    counters.nonlinear_failures += 1;
                    return Err(error);
                }
            };
            jacobian_evaluations += 1;
            jacobian_refreshes += 1;
            stagnation_refreshes += 1;
        }
    }

    counters.nonlinear_failures += 1;
    Err(CoreError::NonlinearSolve(format!(
        "Newton failed after {} iterations (residual WRMS={residual_wrms:e}, correction WRMS={correction_wrms:e}, damping={last_damping:e})",
        config.max_iterations
    )))
}
