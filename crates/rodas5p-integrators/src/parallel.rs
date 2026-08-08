use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};
use rodas5p_core::{CoreError, CoreResult};

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
}
