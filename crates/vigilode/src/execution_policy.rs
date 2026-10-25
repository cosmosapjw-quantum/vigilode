//! Opt-in, measured selection of serial versus one bounded parallel executor.
//!
//! The calibration key is exact: callers must change its epochs when the method,
//! callback, affinity, pool topology, or cost regime changes. Timing observations
//! are empirical, not confidence bounds. No callbacks are replayed by `map_ordered`.
use rodas5p_core::{CoreError, CoreResult};
use rodas5p_integrators::ParallelExecution;
use serde::Serialize;
use std::{cell::Cell, time::Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// Independent work split being calibrated.
pub enum ExecutionLayer {
    /// Independent complete initial-value problems.
    Ensemble,
    /// Independent output components of one callback.
    Data,
    /// Independent rows of a coupled-stage iteration.
    Stage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// Exact identity of the cost regime; epochs are supplied by the caller.
pub struct WorkloadKey {
    /// Change when the model, parameters, or callback cost regime changes.
    pub model_epoch: u64,
    /// Caller-defined identity of the active numerical method.
    pub method_tag: u64,
    /// Number of state components in one problem.
    pub dimension: usize,
    /// Number of independent jobs in this call.
    pub batch_len: usize,
    /// Parallelism layer; evidence is never shared across layers.
    pub layer: ExecutionLayer,
    /// Change for pool, affinity, hardware, or external-load policy changes.
    pub environment_epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// Selected execution backend.
pub enum ExecutionMode {
    /// Ordered execution on the calling worker.
    Serial,
    /// Ordered dispatch through the bounded local pool.
    Parallel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
/// Recorded reason for a selection; suitable for regret diagnosis.
pub enum ExecutionReason {
    /// Observed future cost improves by the configured margin.
    MeasuredGain,
    /// Retain the incumbent while within the exit margin.
    HysteresisRetained,
    /// The candidate does not show sufficient measured improvement.
    NoMeasuredGain,
    /// Three valid pairs for this exact key are unavailable.
    UncalibratedWorkload,
    /// The allowed number of decisions since calibration was reached.
    ExpiredCalibration,
    /// An outer execution layer is already parallel.
    NestedParallelism,
    /// One worker or fewer than two jobs cannot use this split.
    SingleWorker,
    /// Keep the current path for the configured minimum residence.
    Residency,
}
#[derive(Clone, Copy, Debug, Serialize)]
/// Mode, explanation, and empirical cost envelope for one dispatch.
pub struct ExecutionDecision {
    /// Selected backend for this call.
    pub mode: ExecutionMode,
    /// Rule responsible for this selection.
    pub reason: ExecutionReason,
    /// A caller may schedule a replay-safe paired probe at a safe boundary.
    pub probe_due: bool,
    /// Minimum observed serial time; not a confidence bound.
    pub serial_lower_seconds: Option<f64>,
    /// Maximum observed parallel time; not a confidence bound.
    pub parallel_upper_seconds: Option<f64>,
    /// Calls needed to recover the already incurred calibration cost; diagnostic only.
    pub estimated_cost_recovery_calls: Option<f64>,
    /// Whether the declared horizon would recover the calibration cost at observed costs.
    pub horizon_recovers_calibration: bool,
}
#[derive(Clone, Copy, Debug)]
/// Conservative entry, hysteresis, and evidence-expiry settings.
pub struct ExecutionPolicyConfig {
    /// Require this fractional gain before entering the parallel path.
    pub enter_margin: f64,
    /// Permit this fractional slowdown before leaving an incumbent parallel path.
    pub exit_margin: f64,
    /// Minimum residence before re-entering the parallel path.
    pub min_resident_calls: usize,
    /// Maximum non-nested decisions using one calibration.
    pub max_calibration_age: usize,
}
impl Default for ExecutionPolicyConfig {
    fn default() -> Self {
        Self {
            enter_margin: 0.15,
            exit_margin: 0.02,
            min_resident_calls: 8,
            max_calibration_age: 128,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
/// Cumulative policy work counters; independent of solver step counters.
pub struct ExecutionTelemetry {
    /// Number of policy decisions, including nested calls.
    pub decisions: u64,
    /// Number of calls dispatched to the parallel backend.
    pub parallel_calls: u64,
    /// Number of calls dispatched serially.
    pub serial_calls: u64,
    /// Number of calls made serial because an outer layer is parallel.
    pub nested_fallbacks: u64,
    /// Number of valid paired observations recorded.
    pub calibration_pairs: u64,
    /// Number of rejected calibration observations.
    pub invalid_calibrations: u64,
    /// Number of non-nested incumbent changes.
    pub switches: u64,
}
/// Small controller with three paired observations per exact workload.
/// Calibration must include dispatch, callback, and ordered-result costs.
/// `overhead_seconds` includes all calibration work plus pool startup if not
/// already amortized externally. The recovery horizon is reported separately:
/// after calibration, sunk cost must not prevent choosing the cheaper future path.
pub struct ExecutionSelector {
    config: ExecutionPolicyConfig,
    workers: usize,
    key: Option<WorkloadKey>,
    serial: [f64; 3],
    parallel: [f64; 3],
    pairs: usize,
    age: usize,
    overhead_seconds: f64,
    incumbent: ExecutionMode,
    resident_calls: usize,
    telemetry: ExecutionTelemetry,
}
impl ExecutionSelector {
    /// Validate policy settings and start with no cost evidence.
    pub fn new(workers: usize, config: ExecutionPolicyConfig) -> Result<Self, &'static str> {
        if workers == 0
            || workers > 128
            || !config.enter_margin.is_finite()
            || !config.exit_margin.is_finite()
            || config.enter_margin < 0.0
            || config.exit_margin < 0.0
            || config.enter_margin <= config.exit_margin
            || config.max_calibration_age == 0
        {
            return Err("invalid execution policy");
        }
        Ok(Self {
            config,
            workers,
            key: None,
            serial: [0.0; 3],
            parallel: [0.0; 3],
            pairs: 0,
            age: 0,
            overhead_seconds: 0.0,
            incumbent: ExecutionMode::Serial,
            resident_calls: config.min_resident_calls,
            telemetry: ExecutionTelemetry::default(),
        })
    }
    /// Return a copy of the cumulative counters.
    pub fn telemetry(&self) -> ExecutionTelemetry {
        self.telemetry
    }
    /// A new calibration transaction invalidates previous evidence immediately.
    pub fn begin_calibration(
        &mut self,
        key: WorkloadKey,
        overhead_seconds: f64,
    ) -> Result<(), &'static str> {
        self.key = None;
        self.pairs = 0;
        self.incumbent = ExecutionMode::Serial;
        self.age = 0;
        self.resident_calls = self.config.min_resident_calls;
        if key.dimension == 0
            || key.batch_len == 0
            || !overhead_seconds.is_finite()
            || overhead_seconds < 0.0
        {
            return Err("invalid workload or calibration overhead");
        }
        self.key = Some(key);
        self.overhead_seconds = overhead_seconds;
        Ok(())
    }
    /// `equivalent` must include successful computation and a predeclared output
    /// comparison. False or invalid timing clears all calibration evidence.
    pub fn record_pair(
        &mut self,
        key: WorkloadKey,
        serial_seconds: f64,
        parallel_seconds: f64,
        equivalent: bool,
    ) -> Result<(), &'static str> {
        if self.key != Some(key)
            || self.pairs == 3
            || !equivalent
            || !serial_seconds.is_finite()
            || !parallel_seconds.is_finite()
            || serial_seconds <= 0.0
            || parallel_seconds <= 0.0
        {
            self.key = None;
            self.pairs = 0;
            self.incumbent = ExecutionMode::Serial;
            self.telemetry.invalid_calibrations += 1;
            return Err("invalid or mismatched paired calibration");
        }
        self.serial[self.pairs] = serial_seconds;
        self.parallel[self.pairs] = parallel_seconds;
        self.pairs += 1;
        self.telemetry.calibration_pairs += 1;
        Ok(())
    }
    /// Select a backend without executing or replaying any callback.
    pub fn decide(
        &mut self,
        key: WorkloadKey,
        remaining_calls: usize,
        nested: bool,
    ) -> ExecutionDecision {
        self.telemetry.decisions += 1;
        let calibrated = self.key == Some(key) && self.pairs == 3;
        let s = calibrated.then(|| self.serial.iter().copied().fold(f64::INFINITY, f64::min));
        let p = calibrated.then(|| self.parallel.iter().copied().fold(0.0, f64::max));
        let mut decision = ExecutionDecision {
            mode: ExecutionMode::Serial,
            reason: ExecutionReason::UncalibratedWorkload,
            probe_due: !calibrated,
            serial_lower_seconds: s,
            parallel_upper_seconds: p,
            estimated_cost_recovery_calls: None,
            horizon_recovers_calibration: false,
        };
        if nested {
            decision.reason = ExecutionReason::NestedParallelism;
            decision.probe_due = false;
            self.telemetry.nested_fallbacks += 1;
        } else if self.workers == 1 || key.batch_len < 2 {
            decision.reason = ExecutionReason::SingleWorker;
            decision.probe_due = false;
        } else if calibrated && self.age >= self.config.max_calibration_age {
            decision.reason = ExecutionReason::ExpiredCalibration;
            decision.probe_due = true;
        } else if let (Some(s), Some(p)) = (s, p) {
            let saving = s - p;
            if saving > 0.0 {
                let recovery = self.overhead_seconds / saving;
                decision.estimated_cost_recovery_calls = recovery.is_finite().then_some(recovery);
                decision.horizon_recovers_calibration = remaining_calls > 0
                    && self.overhead_seconds / (remaining_calls as f64) < saving;
            }
            // Calibration has already happened: its cost is sunk. Choose using
            // future costs, while retaining the full recovery diagnostic above.
            if self.incumbent == ExecutionMode::Parallel && p <= s * (1.0 + self.config.exit_margin)
            {
                decision.mode = ExecutionMode::Parallel;
                decision.reason = ExecutionReason::HysteresisRetained;
            } else if p < s / (1.0 + self.config.enter_margin) {
                if self.resident_calls >= self.config.min_resident_calls {
                    decision.mode = ExecutionMode::Parallel;
                    decision.reason = ExecutionReason::MeasuredGain;
                } else {
                    decision.reason = ExecutionReason::Residency;
                }
            } else {
                decision.reason = ExecutionReason::NoMeasuredGain;
            }
        }
        // Nested calls do not alter the outer workload's incumbent or age.
        if !nested {
            self.age = self.age.saturating_add(1);
            if decision.mode != self.incumbent {
                self.telemetry.switches += 1;
                self.incumbent = decision.mode;
                self.resident_calls = 0;
            } else {
                self.resident_calls = self.resident_calls.saturating_add(1);
            }
        }
        if decision.mode == ExecutionMode::Parallel {
            self.telemetry.parallel_calls += 1;
        } else {
            self.telemetry.serial_calls += 1;
        }
        decision
    }
}
thread_local! { static ADAPTIVE_DEPTH: Cell<usize> = const { Cell::new(0) }; }
struct DepthGuard;
impl DepthGuard {
    fn enter() -> Self {
        ADAPTIVE_DEPTH.with(|d| d.set(d.get() + 1));
        Self
    }
}
impl Drop for DepthGuard {
    fn drop(&mut self) {
        ADAPTIVE_DEPTH.with(|d| d.set(d.get() - 1));
    }
}
/// Actual ordered dispatch. One pool is reused; nested uses of this wrapper
/// execute serially inside each worker. For other libraries' outer parallelism,
/// explicitly set `external_parallel_context` on each call.
pub struct AdaptiveBatchExecutor {
    serial: ParallelExecution,
    parallel: ParallelExecution,
    /// Cost policy, exposed for external paired calibration and diagnostics.
    pub selector: ExecutionSelector,
    /// Measured construction time of the reusable pool; zero until first required.
    pub startup_seconds: f64,
    pool_ready: bool,
    /// Three raw measured pairs, retained for selection-error diagnosis.
    pub calibration_samples: Vec<CalibrationSample>,
    /// Charged cold warmup pair; excluded only from steady-state cost envelopes.
    pub warmup_seconds: f64,
}
/// One paired observation in seconds, with actual arm order.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct CalibrationSample {
    /// Whether the serial arm ran first.
    pub serial_first: bool,
    /// Time including dispatch and ordered output allocation.
    pub serial_seconds: f64,
    /// Time including dispatch and ordered output allocation.
    pub parallel_seconds: f64,
}
impl AdaptiveBatchExecutor {
    /// Build a lazy bounded executor; no pool is created until a useful probe or dispatch.
    pub fn new(workers: usize, config: ExecutionPolicyConfig) -> CoreResult<Self> {
        let mut selector = ExecutionSelector::new(workers, config)
            .map_err(|e| CoreError::InvalidInput(e.into()))?;
        selector.workers = if ADAPTIVE_DEPTH.with(|d| d.get() > 0) {
            1
        } else {
            workers
        };
        Ok(Self {
            serial: ParallelExecution::sequential(),
            parallel: ParallelExecution::sequential(),
            selector,
            startup_seconds: 0.0,
            pool_ready: false,
            calibration_samples: Vec::new(),
            warmup_seconds: 0.0,
        })
    }
    fn ensure_pool(&mut self) -> CoreResult<()> {
        if !self.pool_ready {
            let started = Instant::now();
            self.parallel = ParallelExecution::rayon(self.selector.workers)?;
            self.startup_seconds += started.elapsed().as_secs_f64();
            self.pool_ready = true;
        }
        Ok(())
    }
    /// Explicit opt-in replay of pure/restartable independent jobs only.
    /// Alternates arm order; includes result allocation and dispatch. Callback
    /// effects must be absent or restored by the caller before every replay.
    pub fn calibrate_replay_safe<T, U, F, V>(
        &mut self,
        key: WorkloadKey,
        items: &[T],
        operation: F,
        equivalent: V,
    ) -> CoreResult<f64>
    where
        T: Sync,
        U: Send,
        F: Fn(&T) -> CoreResult<U> + Send + Sync,
        V: Fn(&[U], &[U]) -> bool,
    {
        if key.batch_len != items.len() || ADAPTIVE_DEPTH.with(|d| d.get() > 0) {
            return Err(CoreError::InvalidInput(
                "calibration shape mismatch or nested context".into(),
            ));
        }
        self.selector
            .begin_calibration(key, 0.0)
            .map_err(|e| CoreError::InvalidInput(e.into()))?;
        self.calibration_samples.clear();
        self.warmup_seconds = 0.0;
        // A one-item/one-worker workload cannot exploit this split. Do not
        // create a pool and do not replay even one callback for calibration.
        if self.selector.workers == 1 || items.len() < 2 {
            return Ok(0.0);
        }
        self.ensure_pool()?;
        let started = Instant::now();
        // First pair is a charged warmup of the reused pool and code paths.
        // Preserve its cost separately; compare three subsequent steady pairs.
        for pair in 0..4 {
            let pair_start = Instant::now();
            let run = |backend: &ParallelExecution| {
                let t = Instant::now();
                let output = backend.map_ordered(items, |x| {
                    let _guard = DepthGuard::enter();
                    operation(x)
                })?;
                Ok::<_, CoreError>((output, t.elapsed().as_secs_f64()))
            };
            let (serial, parallel) = if pair % 2 == 0 {
                (run(&self.serial)?, run(&self.parallel)?)
            } else {
                let p = run(&self.parallel)?;
                (run(&self.serial)?, p)
            };
            let equal = equivalent(&serial.0, &parallel.0);
            if pair == 0 {
                if !equal {
                    self.selector.key = None;
                    return Err(CoreError::InvalidInput("warmup output mismatch".into()));
                }
                self.warmup_seconds = pair_start.elapsed().as_secs_f64();
            } else {
                self.selector
                    .record_pair(key, serial.1, parallel.1, equal)
                    .map_err(|e| CoreError::InvalidInput(e.into()))?;
                self.calibration_samples.push(CalibrationSample {
                    serial_first: pair % 2 == 0,
                    serial_seconds: serial.1,
                    parallel_seconds: parallel.1,
                });
            }
        }
        let calibration_seconds = started.elapsed().as_secs_f64();
        self.selector.overhead_seconds = self.startup_seconds + calibration_seconds;
        Ok(calibration_seconds)
    }
    /// Execute each job once, in the selected backend, returning ordered outputs.
    pub fn map_ordered<T, U, F>(
        &mut self,
        key: WorkloadKey,
        remaining_calls: usize,
        external_parallel_context: bool,
        items: &[T],
        operation: F,
    ) -> CoreResult<(Vec<U>, ExecutionDecision)>
    where
        T: Sync,
        U: Send,
        F: Fn(&T) -> CoreResult<U> + Send + Sync,
    {
        if key.dimension == 0 || key.batch_len != items.len() {
            return Err(CoreError::Dimension(
                "execution workload batch length mismatch".into(),
            ));
        }
        let nested = external_parallel_context || ADAPTIVE_DEPTH.with(|d| d.get() > 0);
        let decision = self.selector.decide(key, remaining_calls, nested);
        if decision.mode == ExecutionMode::Parallel {
            self.ensure_pool()?;
        }
        let backend = if decision.mode == ExecutionMode::Parallel {
            &self.parallel
        } else {
            &self.serial
        };
        let output = backend.map_ordered(items, |x| {
            let _guard = DepthGuard::enter();
            operation(x)
        })?;
        Ok((output, decision))
    }
}
