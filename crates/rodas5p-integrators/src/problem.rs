use std::sync::Arc;

use crate::parallel::ParallelExecution;

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

    fn eval_rhs_uncounted(&self, t: f64, y: &[f64]) -> CoreResult<Vec<f64>> {
        if y.len() != self.dimension {
            return Err(CoreError::Dimension("RHS state shape mismatch".into()));
        }
        let mut out = vec![0.0; self.dimension];
        (self.rhs)(t, y, &mut out)?;
        if out.iter().all(|v| v.is_finite()) {
            Ok(out)
        } else {
            Err(CoreError::NonFinite("RHS produced NaN/Inf".into()))
        }
    }

    pub fn eval_rhs(&self, t: f64, y: &[f64], counters: &mut WorkCounters) -> CoreResult<Vec<f64>> {
        let out = self.eval_rhs_uncounted(t, y)?;
        counters.rhs_calls += 1;
        counters.rhs_evaluations += 1;
        Ok(out)
    }

    /// Evaluate independent method-stage RHS rows through a bounded execution context.
    ///
    /// This path calls the scalar physics kernel once per stage.  With a local Rayon pool those
    /// calls run concurrently, so the implementation measures real within-step stage parallelism
    /// rather than case-level throughput or a serial function with a batched signature.
    pub fn eval_rhs_stage_rows(
        &self,
        times: &[f64],
        states: &[Vec<f64>],
        execution: &ParallelExecution,
        counters: &mut WorkCounters,
    ) -> CoreResult<Vec<Vec<f64>>> {
        if times.len() != states.len() || states.iter().any(|s| s.len() != self.dimension) {
            return Err(CoreError::Dimension(
                "stage RHS batch shape mismatch".into(),
            ));
        }
        let indices: Vec<usize> = (0..states.len()).collect();
        // Count the entire scheduled batch before execution so an early error cannot erase work
        // requested from the numerical method. This is the same vector-work convention used by
        // the provided batch callback.
        counters.rhs_batch_calls += 1;
        counters.rhs_evaluations += states.len() as u64;
        execution.map_ordered(&indices, |&index| {
            self.eval_rhs_uncounted(times[index], &states[index])
        })
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

    /// Build a strictly matrix-free linearization operator.
    ///
    /// Unlike [`OdeProblem::linearize`], this entry point never falls back to an explicit
    /// Jacobian callback.  It is the load-bearing contract for the generic vectorized/JF fast
    /// path: callers either supplied a genuine JVP implementation or the request fails closed
    /// before a speculative timestep starts.
    pub fn linearize_matrix_free(&self, t: f64, y: &[f64]) -> CoreResult<Arc<dyn LinearOperator>> {
        if y.len() != self.dimension {
            return Err(CoreError::Dimension(
                "matrix-free linearization state shape mismatch".into(),
            ));
        }
        let jvp = self.jvp.clone().ok_or_else(|| {
            CoreError::InvalidInput(
                "strict matrix-free integration requires a user-supplied JVP".into(),
            )
        })?;
        let state = y.to_vec();
        let n = self.dimension;
        Ok(Arc::new(ClosureOperator::new(n, move |v, out| {
            jvp(t, &state, v, out)
        })))
    }

    pub fn supports_matrix_free_jvp(&self) -> bool {
        self.jvp.is_some()
    }

    /// Return a clone whose linearization is exposed only through the configured JVP.
    ///
    /// This is the strict matrix-free research lane: even when an explicit Jacobian callback is
    /// available for offline certification, the returned problem cannot materialize it through
    /// [`OdeProblem::linearize`].
    pub fn jvp_only_clone(&self) -> CoreResult<Self> {
        if self.jvp.is_none() {
            return Err(CoreError::InvalidInput(
                "strict matrix-free clone requires a JVP callback".into(),
            ));
        }
        let mut cloned = self.clone();
        cloned.jacobian = None;
        Ok(cloned)
    }

    pub fn has_explicit_jacobian(&self) -> bool {
        self.jacobian.is_some()
    }

    pub fn has_jvp(&self) -> bool {
        self.jvp.is_some()
    }

    /// Convert a nonautonomous identity-mass problem into the autonomous augmented system
    /// `(y, tau)' = (F(tau, y), 1)` without forming an explicit Jacobian.
    ///
    /// The augmented JVP is `(J_y v_y + F_t v_tau, 0)`. A declared `partial_t` callback is
    /// mandatory: hidden finite differences would violate the strict work and JVP-quality contract.
    pub fn time_augmented_clone(&self) -> CoreResult<Self> {
        if self.mass_matrix.is_some() {
            return Err(CoreError::InvalidInput(
                "time augmentation currently supports identity mass only".into(),
            ));
        }
        let jvp = self.jvp.clone().ok_or_else(|| {
            CoreError::InvalidInput("time augmentation requires a JVP callback".into())
        })?;
        let partial_t = self.partial_t.clone().ok_or_else(|| {
            CoreError::InvalidInput(
                "time augmentation requires an explicit partial_t callback".into(),
            )
        })?;
        let rhs = self.rhs.clone();
        let n = self.dimension;
        let augmented_rhs: RhsFn = Arc::new(move |_, state, out| {
            if state.len() != n + 1 || out.len() != n + 1 {
                return Err(CoreError::Dimension(
                    "time-augmented RHS shape mismatch".into(),
                ));
            }
            rhs(state[n], &state[..n], &mut out[..n])?;
            out[n] = 1.0;
            Ok(())
        });
        let augmented_jvp: JvpFn = Arc::new(move |_, state, direction, out| {
            if state.len() != n + 1 || direction.len() != n + 1 || out.len() != n + 1 {
                return Err(CoreError::Dimension(
                    "time-augmented JVP shape mismatch".into(),
                ));
            }
            jvp(state[n], &state[..n], &direction[..n], &mut out[..n])?;
            if direction[n] != 0.0 {
                let mut ft = vec![0.0; n];
                partial_t(state[n], &state[..n], &mut ft)?;
                for (value, source) in out[..n].iter_mut().zip(ft) {
                    *value += direction[n] * source;
                }
            }
            out[n] = 0.0;
            Ok(())
        });
        let exact = self.exact_solution.clone().map(|solution| {
            Arc::new(move |time| {
                let mut state = solution(time);
                state.push(time);
                state
            }) as ExactFn
        });
        Self::new(
            format!("{}-time-augmented", self.name),
            n + 1,
            augmented_rhs,
            None,
            None,
            Some(augmented_jvp),
            None,
            true,
            None,
            exact,
        )
    }

    pub fn exact(&self, t: f64) -> Option<Vec<f64>> {
        self.exact_solution.as_ref().map(|f| f(t))
    }
}
