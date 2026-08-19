use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};
use rodas5p_core::{CoreError, CoreResult, LinearOperator};

/// Bounded execution context for independent scientific jobs.
///
/// The pool is local rather than global, so CLI thread counts are explicit and nested library
/// parallelism cannot silently consume additional cores. `map_ordered` uses an indexed parallel
/// iterator and therefore preserves the input order.
pub struct ParallelExecution {
    threads: usize,
    pool: Option<ThreadPool>,
}

impl ParallelExecution {
    pub fn sequential() -> Self {
        Self {
            threads: 1,
            pool: None,
        }
    }

    pub fn rayon(threads: usize) -> CoreResult<Self> {
        if threads == 0 {
            return Err(CoreError::InvalidInput(
                "Rayon thread count must be positive".into(),
            ));
        }
        if threads == 1 {
            return Ok(Self::sequential());
        }
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("rodas5p-rayon-{index}"))
            .build()
            .map_err(|error| {
                CoreError::InvalidInput(format!("failed to create local Rayon pool: {error}"))
            })?;
        Ok(Self {
            threads,
            pool: Some(pool),
        })
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    pub fn backend(&self) -> &'static str {
        if self.pool.is_some() {
            "rayon-local"
        } else {
            "sequential"
        }
    }

    pub fn map_ordered<T, U, F>(&self, items: &[T], operation: F) -> CoreResult<Vec<U>>
    where
        T: Sync,
        U: Send,
        F: Fn(&T) -> CoreResult<U> + Send + Sync,
    {
        if let Some(pool) = &self.pool {
            pool.install(|| items.par_iter().map(&operation).collect())
        } else {
            items.iter().map(operation).collect()
        }
    }

    /// Apply an operation to matching input/output slots without allocating an intermediate
    /// result vector. The output order is identical to the input order.
    pub fn try_for_each_ordered_mut<T, U, F>(
        &self,
        inputs: &[T],
        outputs: &mut [U],
        operation: F,
    ) -> CoreResult<()>
    where
        T: Sync,
        U: Send,
        F: Fn(&T, &mut U) -> CoreResult<()> + Send + Sync,
    {
        if inputs.len() != outputs.len() {
            return Err(CoreError::Dimension(
                "parallel input/output batch length mismatch".into(),
            ));
        }
        if let Some(pool) = &self.pool {
            pool.install(|| {
                inputs
                    .par_iter()
                    .zip(outputs.par_iter_mut())
                    .try_for_each(|(input, output)| operation(input, output))
            })
        } else {
            inputs
                .iter()
                .zip(outputs.iter_mut())
                .try_for_each(|(input, output)| operation(input, output))
        }
    }

    /// Apply one shared operator to independent method-stage rows.
    ///
    /// A multi-thread local Rayon pool gives genuine parallelism across stages while preserving
    /// deterministic input order.  This is intentionally separate from case-level scheduling.
    pub fn apply_operator_rows(
        &self,
        operator: &dyn LinearOperator,
        rows: &[Vec<f64>],
    ) -> CoreResult<Vec<Vec<f64>>> {
        let n = operator.dimension();
        if rows.iter().any(|row| row.len() != n) {
            return Err(CoreError::Dimension(
                "stage operator row shape mismatch".into(),
            ));
        }
        let indices: Vec<usize> = (0..rows.len()).collect();
        self.map_ordered(&indices, |&index| {
            let mut out = vec![0.0; n];
            operator.apply(&rows[index], &mut out)?;
            if out.iter().all(|value| value.is_finite()) {
                Ok(out)
            } else {
                Err(CoreError::NonFinite(
                    "stage operator batch produced NaN/Inf".into(),
                ))
            }
        })
    }
}
