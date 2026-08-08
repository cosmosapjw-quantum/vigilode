use std::sync::Arc;

use rodas5p_core::{
    ClosureOperator, CoreError, CoreResult, DenseMatrix, DenseOperator, LinearOperator,
    WorkCounters,
};

pub type RhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type BatchRhsFn = Arc<dyn Fn(&[f64], &[Vec<f64>]) -> CoreResult<Vec<Vec<f64>>> + Send + Sync>;
pub type JacobianFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
pub type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type PartialTFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
pub type ExactFn = Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>;

#[derive(Clone)]
pub struct OdeProblem {
    pub name: String,
    pub dimension: usize,
    rhs: RhsFn,
    rhs_batch: Option<BatchRhsFn>,
    jacobian: Option<JacobianFn>,
    jvp: Option<JvpFn>,
    partial_t: Option<PartialTFn>,
    pub autonomous: bool,
    pub mass_matrix: Option<DenseMatrix>,
    exact_solution: Option<ExactFn>,
}

impl OdeProblem {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        dimension: usize,
        rhs: RhsFn,
        rhs_batch: Option<BatchRhsFn>,
        jacobian: Option<JacobianFn>,
        jvp: Option<JvpFn>,
        partial_t: Option<PartialTFn>,
        autonomous: bool,
        mass_matrix: Option<DenseMatrix>,
        exact_solution: Option<ExactFn>,
    ) -> CoreResult<Self> {
        if dimension == 0 {
            return Err(CoreError::InvalidInput(
                "ODE dimension must be positive".into(),
            ));
        }
        if jacobian.is_none() && jvp.is_none() {
            return Err(CoreError::InvalidInput(
                "provide an explicit Jacobian or JVP".into(),
            ));
        }
        if mass_matrix
            .as_ref()
            .is_some_and(|m| m.nrows() != dimension || m.ncols() != dimension)
        {
            return Err(CoreError::Dimension("mass matrix shape mismatch".into()));
        }
        Ok(Self {
            name: name.into(),
            dimension,
            rhs,
            rhs_batch,
            jacobian,
            jvp,
            partial_t,
            autonomous,
            mass_matrix,
            exact_solution,
        })
    }

    pub fn eval_rhs(&self, t: f64, y: &[f64], counters: &mut WorkCounters) -> CoreResult<Vec<f64>> {
        if y.len() != self.dimension {
            return Err(CoreError::Dimension("RHS state shape mismatch".into()));
        }
        let mut out = vec![0.0; self.dimension];
        (self.rhs)(t, y, &mut out)?;
        counters.rhs_calls += 1;
        counters.rhs_evaluations += 1;
        if out.iter().all(|v| v.is_finite()) {
            Ok(out)
        } else {
            Err(CoreError::NonFinite("RHS produced NaN/Inf".into()))
        }
    }

    pub fn eval_rhs_batch(
        &self,
        times: &[f64],
        states: &[Vec<f64>],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        if times.len() != states.len() || states.iter().any(|s| s.len() != self.dimension) {
            return Err(CoreError::Dimension("batched RHS shape mismatch".into()));
        }
        if let Some(batch) = &self.rhs_batch {
            let out = batch(times, states)?;
            if out.len() != states.len() || out.iter().any(|r| r.len() != self.dimension) {
                return Err(CoreError::Dimension(
                    "batched RHS output shape mismatch".into(),
                ));
            }
            if !out.iter().flatten().all(|v| v.is_finite()) {
                return Err(CoreError::NonFinite("batched RHS produced NaN/Inf".into()));
            }
            counters.rhs_batch_calls += 1;
            counters.rhs_evaluations += times.len() as u64;
            Ok(out)
        } else {
            times
                .iter()
                .zip(states)
                .map(|(&t, y)| self.eval_rhs(t, y, counters))
                .collect()
        }
    }

    pub fn eval_partial_t(
        &self,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<f64>> {
        counters.ft_calls += 1;
        if self.autonomous {
            return Ok(vec![0.0; self.dimension]);
        }
        if let Some(f) = &self.partial_t {
            let mut out = vec![0.0; self.dimension];
            f(t, y, &mut out)?;
            if out.iter().all(|v| v.is_finite()) {
                return Ok(out);
            }
            return Err(CoreError::NonFinite("partial_t produced NaN/Inf".into()));
        }
        let eps = f64::EPSILON.sqrt() * t.abs().max(1.0);
        let fp = self.eval_rhs(t + eps, y, counters)?;
        let fm = self.eval_rhs(t - eps, y, counters)?;
        Ok(fp
            .iter()
            .zip(fm)
            .map(|(a, b)| (a - b) / (2.0 * eps))
            .collect())
    }

    pub fn mass_or_identity(&self) -> DenseMatrix {
        self.mass_matrix
            .clone()
            .unwrap_or_else(|| DenseMatrix::identity(self.dimension))
    }

    pub fn dense_jacobian(
        &self,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<DenseMatrix> {
        if y.len() != self.dimension {
            return Err(CoreError::Dimension("Jacobian state shape mismatch".into()));
        }
        if let Some(jacobian) = &self.jacobian {
            counters.jacobian_builds += 1;
            let matrix = jacobian(t, y)?;
            if matrix.nrows() != self.dimension || matrix.ncols() != self.dimension {
                return Err(CoreError::Dimension(
                    "Jacobian output shape mismatch".into(),
                ));
            }
            return Ok(matrix);
        }
        let jvp = self.jvp.as_ref().expect("validated JVP");
        let mut matrix = DenseMatrix::zeros(self.dimension, self.dimension);
        let mut basis = vec![0.0; self.dimension];
        let mut column = vec![0.0; self.dimension];
        for j in 0..self.dimension {
            basis.fill(0.0);
            basis[j] = 1.0;
            column.fill(0.0);
            jvp(t, y, &basis, &mut column)?;
            counters.jvp_calls += 1;
            counters.jvp_vectors += 1;
            if !column.iter().all(|value| value.is_finite()) {
                return Err(CoreError::NonFinite("JVP produced NaN/Inf".into()));
            }
            for i in 0..self.dimension {
                matrix[(i, j)] = column[i];
            }
        }
        counters.jacobian_builds += 1;
        Ok(matrix)
    }

    pub fn linearize(
        &self,
        t: f64,
        y: &[f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<Arc<dyn LinearOperator>> {
        if let Some(j) = &self.jacobian {
            counters.jacobian_builds += 1;
            return Ok(Arc::new(DenseOperator::new(j(t, y)?)?));
        }
        let jvp = self.jvp.clone().expect("validated JVP");
        let state = y.to_vec();
        let n = self.dimension;
        Ok(Arc::new(ClosureOperator::new(n, move |v, out| {
            jvp(t, &state, v, out)
        })))
    }

    pub fn exact(&self, t: f64) -> Option<Vec<f64>> {
        self.exact_solution.as_ref().map(|f| f(t))
    }
}
