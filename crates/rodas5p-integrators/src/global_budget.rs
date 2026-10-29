//! Sealed full-horizon replay of model-bound continuous residuals, plus a
//! separately typed algebraic work allocator. No public numeric residual or
//! arbitrary callback can construct a trajectory certificate.
//!
//! The running comparison uses one fixed Euclidean norm. If a segment has
//! residual rho, one-sided logarithmic norm mu, duration h, and a jump J from
//! the previous represented curve, then G=exp(max(mu,0)h) gives the whole-segment
//! bound G*(R+J+h*rho). All arithmetic is outward. The original sealed
//! provider receipt already proves that the true solution and represented
//! curve stay in its tube; mu/rho are used on that tube even if the replay
//! radius is looser than the original provider's bound. This may be more conservative
//! than a model-specific signed-mu verifier; a refusal never invalidates that
//! existing certificate. The full returned curve and stored query centres have
//! separate bounds. This is a post-run replay, not an online gain forecast.
use std::sync::Arc;

use rodas5p_core::CoreError;
use rodas5p_core::directed::{Interval, add_up, div_up, exp_interval, mul_up, sqrt_up, sub_up};
use serde::Serialize;
use thiserror::Error;

use crate::generic_verified::{
    VerifiedContinuousProblem, VerifiedResidualError, VerifiedResidualRun,
    integrate_verified_residual,
};
use crate::{AdaptiveStepConfig, DenseOutputError, DenseSegmentWork};

#[derive(Debug, Error)]
pub enum GlobalBudgetError {
    #[error(transparent)]
    Arithmetic(#[from] CoreError),
    #[error(transparent)]
    Curve(#[from] DenseOutputError),
    #[error("invalid global budget: {0}")]
    Invalid(&'static str),
    #[error("verified trajectory is incomplete at {time:e}")]
    Incomplete { time: f64 },
    #[error("global radius {radius:e} exceeds original L2 absolute budget {budget:e}")]
    Exceeded { radius: f64, budget: f64 },
}

/// Attempted replay work, including a refused segment. Scalar entries count
/// calls to directed bound primitives, not their hidden floating operations.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct GlobalBudgetWork {
    pub segment_attempts: u64,
    pub segment_commits: u64,
    pub refusals: u64,
    pub scalar_bound_operations: u64,
    pub exponential_enclosures: u64,
    pub query_checks: u64,
    pub workspace_items: u64,
    pub curve_work: DenseSegmentWork,
}

/// Diagnostic values; constructing or deserializing numbers creates no proof.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct GlobalBudgetSnapshot {
    pub time: f64,
    pub segments: usize,
    pub endpoint_radius: f64,
    pub maximum_curve_radius: f64,
    pub maximum_stored_query_radius: f64,
    pub cumulative_jump_upper: f64,
    pub cumulative_residual_integral_upper: f64,
}

/// Immutable proof result, minted only by a completed sealed replay. The norm
/// is Euclidean L2, with no change to rtol/atol or to the model's coordinates.
#[derive(Clone, Debug)]
pub struct GlobalBudgetCertificate {
    model_identity: Arc<()>,
    run_identity: Arc<()>,
    model_name: String,
    span: (f64, f64),
    dimension: usize,
    budget: f64,
    snapshot: GlobalBudgetSnapshot,
    work: GlobalBudgetWork,
}
impl GlobalBudgetCertificate {
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
    pub fn span(&self) -> (f64, f64) {
        self.span
    }
    pub fn dimension(&self) -> usize {
        self.dimension
    }
    pub fn absolute_l2_budget(&self) -> f64 {
        self.budget
    }
    pub fn snapshot(&self) -> GlobalBudgetSnapshot {
        self.snapshot
    }
    pub fn work(&self) -> GlobalBudgetWork {
        self.work
    }
    /// Capability identity, not equality of names or floating model parameters.
    pub fn same_model_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.model_identity, &other.model_identity)
    }
    /// Exact originating trajectory, including its initial ball and queries.
    /// Equal model names, parameters or spans do not establish this binding.
    pub fn binds_run(&self, source: &VerifiedResidualRun) -> bool {
        Arc::ptr_eq(&self.run_identity, &source.run_identity)
            && Arc::ptr_eq(&self.model_identity, &source.model_identity)
    }
}

#[derive(Debug)]
pub struct GlobalBudgetFailure {
    pub error: GlobalBudgetError,
    pub snapshot: GlobalBudgetSnapshot,
    pub work: GlobalBudgetWork,
}

/// A budget borrows the complete private scientific binding of one run. Its
/// next receipt is read internally; receipts from other trajectories cannot be
/// passed in. Scientific state changes atomically after the entire segment and
/// its stored outputs pass. Attempted work is retained on refusal.
pub struct VerifiedGlobalBudget<'a> {
    source: &'a VerifiedResidualRun,
    budget: f64,
    snapshot: GlobalBudgetSnapshot,
    previous_endpoint: Vec<Interval>,
    start_buffer: Vec<Interval>,
    end_buffer: Vec<Interval>,
    query_buffer: Vec<Interval>,
    output_cursor: usize,
    work: GlobalBudgetWork,
}

fn charged<T>(
    work: &mut GlobalBudgetWork,
    op: impl FnOnce() -> Result<T, CoreError>,
) -> Result<T, GlobalBudgetError> {
    work.scalar_bound_operations = work.scalar_bound_operations.saturating_add(1);
    Ok(op()?)
}

fn distance_upper(
    a: &[Interval],
    b: &[Interval],
    work: &mut GlobalBudgetWork,
) -> Result<f64, GlobalBudgetError> {
    if a.len() != b.len() {
        return Err(GlobalBudgetError::Invalid("curve dimension changed"));
    }
    let mut square = 0.0;
    for (&x, &y) in a.iter().zip(b) {
        let d = charged(work, || x.sub(y))?.mag();
        let term = charged(work, || mul_up(d, d))?;
        square = charged(work, || add_up(square, term))?;
    }
    charged(work, || sqrt_up(square))
}

impl<'a> VerifiedGlobalBudget<'a> {
    pub fn new(source: &'a VerifiedResidualRun, budget: f64) -> Result<Self, GlobalBudgetError> {
        if !(budget.is_finite() && budget > 0.0) {
            return Err(GlobalBudgetError::Invalid(
                "positive finite absolute L2 budget required",
            ));
        }
        if budget > source.source_absolute_l2_budget {
            return Err(GlobalBudgetError::Invalid(
                "replay cannot relax the source run's original absolute L2 budget",
            ));
        }
        let radius = source.source_initial_radius;
        if radius > budget {
            return Err(GlobalBudgetError::Exceeded { radius, budget });
        }
        let n = source.source_initial_center.len();
        let zero = Interval::point(0.0)?;
        let allocate = || -> Result<Vec<Interval>, GlobalBudgetError> {
            let mut values = Vec::new();
            values
                .try_reserve_exact(n)
                .map_err(|_| GlobalBudgetError::Invalid("global replay allocation refused"))?;
            values.resize(n, zero);
            Ok(values)
        };
        let mut previous_endpoint = allocate()?;
        for (out, &value) in previous_endpoint
            .iter_mut()
            .zip(&source.source_initial_center)
        {
            *out = Interval::point(value)?;
        }
        let start_buffer = allocate()?;
        let end_buffer = allocate()?;
        let query_buffer = allocate()?;
        let mut output_cursor = 0;
        while output_cursor < source.outputs().len()
            && source.outputs()[output_cursor].time() == source.source_span.0
        {
            // Initial output is the exact represented initial centre. This
            // equality is established by the sealed driver's constructor.
            output_cursor += 1;
        }
        Ok(Self {
            source,
            budget,
            previous_endpoint,
            start_buffer,
            end_buffer,
            query_buffer,
            output_cursor,
            snapshot: GlobalBudgetSnapshot {
                time: source.source_span.0,
                segments: 0,
                endpoint_radius: radius,
                maximum_curve_radius: radius,
                maximum_stored_query_radius: radius,
                cumulative_jump_upper: 0.0,
                cumulative_residual_integral_upper: 0.0,
            },
            work: GlobalBudgetWork {
                workspace_items: (n as u64).saturating_mul(4),
                ..GlobalBudgetWork::default()
            },
        })
    }
    pub fn snapshot(&self) -> GlobalBudgetSnapshot {
        self.snapshot
    }
    pub fn work(&self) -> GlobalBudgetWork {
        self.work
    }

    /// Consume one internally selected immutable full-tube receipt. `false`
    /// means no further segment exists; `finish` checks full horizon completion.
    pub fn advance(&mut self) -> Result<bool, GlobalBudgetError> {
        let Some(segment) = self.source.segments().get(self.snapshot.segments) else {
            return Ok(false);
        };
        self.work.segment_attempts = self.work.segment_attempts.saturating_add(1);
        let trial = (|| {
            let d = segment.diagnostics();
            if d.span.0 != self.snapshot.time
                || d.span.1 <= d.span.0
                || d.span.1 > self.source.source_span.1
            {
                return Err(GlobalBudgetError::Invalid(
                    "receipt sequence is not contiguous in the bound horizon",
                ));
            }

            segment.curve.evaluate_value_at_into(
                d.span.0,
                &mut self.start_buffer,
                &mut self.work.curve_work,
            )?;
            segment.curve.evaluate_value_at_into(
                d.span.1,
                &mut self.end_buffer,
                &mut self.work.curve_work,
            )?;
            let jump = distance_upper(&self.start_buffer, &self.previous_endpoint, &mut self.work)?;
            let h = charged(&mut self.work, || sub_up(d.span.1, d.span.0))?;
            let exponent = charged(&mut self.work, || {
                mul_up(h, d.logarithmic_norm_upper.max(0.0))
            })?;
            let gain = if exponent == 0.0 {
                1.0
            } else {
                self.work.exponential_enclosures =
                    self.work.exponential_enclosures.saturating_add(1);
                // Existing interval Taylor enclosure, not a libm one-ULP guess.
                exp_interval(exponent)?.hi
            };
            let residual = charged(&mut self.work, || mul_up(h, d.residual_upper))?;
            let initial = charged(&mut self.work, || {
                add_up(self.snapshot.endpoint_radius, jump)
            })?;
            let local = charged(&mut self.work, || add_up(initial, residual))?;
            let radius = charged(&mut self.work, || mul_up(gain, local))?;
            if radius > self.budget {
                return Err(GlobalBudgetError::Exceeded {
                    radius,
                    budget: self.budget,
                });
            }
            let mut output_cursor = self.output_cursor;
            let mut max_query = self.snapshot.maximum_stored_query_radius;
            while let Some(output) = self.source.outputs().get(output_cursor) {
                if output.time() > d.span.1 {
                    break;
                }
                if output.time() < d.span.0 {
                    return Err(GlobalBudgetError::Invalid(
                        "stored query precedes its receipt",
                    ));
                }
                self.work.query_checks = self.work.query_checks.saturating_add(1);
                segment.curve.evaluate_value_at_into(
                    output.time(),
                    &mut self.query_buffer,
                    &mut self.work.curve_work,
                )?;
                let mut square = 0.0;
                for (&value, &center) in self.query_buffer.iter().zip(output.center()) {
                    let displacement =
                        charged(&mut self.work, || value.sub(Interval::point(center)?))?.mag();
                    let term = charged(&mut self.work, || mul_up(displacement, displacement))?;
                    square = charged(&mut self.work, || add_up(square, term))?;
                }
                let rounding = charged(&mut self.work, || sqrt_up(square))?;
                let query = charged(&mut self.work, || add_up(radius, rounding))?;
                if query > self.budget {
                    return Err(GlobalBudgetError::Exceeded {
                        radius: query,
                        budget: self.budget,
                    });
                }
                max_query = max_query.max(query);
                output_cursor += 1;
            }
            let cumulative_jump_upper = charged(&mut self.work, || {
                add_up(self.snapshot.cumulative_jump_upper, jump)
            })?;
            let cumulative_residual_integral_upper = charged(&mut self.work, || {
                add_up(self.snapshot.cumulative_residual_integral_upper, residual)
            })?;
            Ok((
                GlobalBudgetSnapshot {
                    time: d.span.1,
                    segments: self.snapshot.segments + 1,
                    endpoint_radius: radius,
                    maximum_curve_radius: self.snapshot.maximum_curve_radius.max(radius),
                    maximum_stored_query_radius: max_query,
                    cumulative_jump_upper,
                    cumulative_residual_integral_upper,
                },
                output_cursor,
            ))
        })();
        match trial {
            Ok((snapshot, cursor)) => {
                self.snapshot = snapshot;
                std::mem::swap(&mut self.previous_endpoint, &mut self.end_buffer);
                self.output_cursor = cursor;
                self.work.segment_commits = self.work.segment_commits.saturating_add(1);
                Ok(true)
            }
            Err(error) => {
                self.work.refusals = self.work.refusals.saturating_add(1);
                Err(error)
            }
        }
    }
    #[expect(
        clippy::result_large_err,
        reason = "failure retains the full inline snapshot and work ledger"
    )]
    pub fn finish(self) -> Result<GlobalBudgetCertificate, GlobalBudgetFailure> {
        if !self.source.source_complete
            || self.snapshot.time != self.source.source_span.1
            || self.snapshot.segments != self.source.segments().len()
            || self.output_cursor != self.source.outputs().len()
        {
            return Err(GlobalBudgetFailure {
                error: GlobalBudgetError::Incomplete {
                    time: self.snapshot.time,
                },
                snapshot: self.snapshot,
                work: self.work,
            });
        }
        Ok(GlobalBudgetCertificate {
            model_identity: self.source.model_identity.clone(),
            run_identity: self.source.run_identity.clone(),
            model_name: self.source.model_name.clone(),
            span: self.source.source_span,
            dimension: self.source.source_initial_center.len(),
            budget: self.budget,
            snapshot: self.snapshot,
            work: self.work,
        })
    }
}

/// Certify the immutable source at its original absolute L2 budget or a smaller
/// requested budget. Relaxing the original budget is explicitly refused.
#[expect(
    clippy::result_large_err,
    reason = "failure retains the full inline snapshot and work ledger"
)]
pub fn certify_global_run(
    source: &VerifiedResidualRun,
    budget: f64,
) -> Result<GlobalBudgetCertificate, GlobalBudgetFailure> {
    let mut replay =
        VerifiedGlobalBudget::new(source, budget).map_err(|error| GlobalBudgetFailure {
            error,
            snapshot: GlobalBudgetSnapshot {
                time: source.source_span.0,
                segments: 0,
                endpoint_radius: source.source_initial_radius,
                maximum_curve_radius: source.source_initial_radius,
                maximum_stored_query_radius: source.source_initial_radius,
                cumulative_jump_upper: 0.0,
                cumulative_residual_integral_upper: 0.0,
            },
            work: GlobalBudgetWork::default(),
        })?;
    loop {
        match replay.advance() {
            Ok(true) => (),
            Ok(false) => return replay.finish(),
            Err(error) => {
                return Err(GlobalBudgetFailure {
                    error,
                    snapshot: replay.snapshot(),
                    work: replay.work(),
                });
            }
        }
    }
}

/// Retains the entire native run and failed-attempt ledger even if the extra
/// whole-horizon replay refuses. The ordinary provider certificate stays valid.
#[derive(Debug)]
pub struct GlobalVerifiedResidualRun {
    run: VerifiedResidualRun,
    global: Result<GlobalBudgetCertificate, GlobalBudgetFailure>,
}
impl GlobalVerifiedResidualRun {
    pub fn run(&self) -> &VerifiedResidualRun {
        &self.run
    }
    pub fn global(&self) -> &Result<GlobalBudgetCertificate, GlobalBudgetFailure> {
        &self.global
    }
    pub fn into_parts(
        self,
    ) -> (
        VerifiedResidualRun,
        Result<GlobalBudgetCertificate, GlobalBudgetFailure>,
    ) {
        (self.run, self.global)
    }
}

pub fn integrate_verified_residual_global(
    owner: &VerifiedContinuousProblem,
    span: (f64, f64),
    initial_center: &[f64],
    initial_radius: f64,
    output_times: &[f64],
    config: &AdaptiveStepConfig,
) -> Result<GlobalVerifiedResidualRun, VerifiedResidualError> {
    let run = integrate_verified_residual(
        owner,
        span,
        initial_center,
        initial_radius,
        output_times,
        config,
    )?;
    let global = certify_global_run(&run, config.atol);
    Ok(GlobalVerifiedResidualRun { run, global })
}

// ---------------------- Algebraic allocation: no ODE authority ----------------

/// A_j in sum A_j/n_j^p and per-step work ceiling c_j. These are caller-supplied
/// algebraic quantities. A validated allocation proves only this inequality;
/// binding A_j to an actual ODE local error requires a separate model proof.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct AllocationBlock {
    pub error_coefficient: f64,
    pub step_cost_upper: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct AllocationLimits {
    pub max_steps_per_block: u64,
    pub max_total_steps: u64,
    pub max_refinements: u32,
}
impl Default for AllocationLimits {
    fn default() -> Self {
        Self {
            max_steps_per_block: 1 << 40,
            max_total_steps: 1 << 50,
            max_refinements: 32,
        }
    }
}
#[derive(Debug, Error)]
pub enum AllocationError {
    #[error(transparent)]
    Arithmetic(#[from] CoreError),
    #[error("invalid allocation input: {0}")]
    Invalid(&'static str),
    #[error("allocation exceeds its declared resource limit")]
    ResourceLimit,
    #[error("outward allocation error {upper:e} exceeds budget {budget:e}")]
    Budget { upper: f64, budget: f64 },
}
#[derive(Clone, Debug, Serialize)]
pub struct ValidatedWorkAllocation {
    blocks: Vec<AllocationBlock>,
    counts: Vec<u64>,
    order: u32,
    budget: f64,
    error_upper: f64,
    work_upper: f64,
    refinements: u32,
}
impl ValidatedWorkAllocation {
    pub fn blocks(&self) -> &[AllocationBlock] {
        &self.blocks
    }
    pub fn order(&self) -> u32 {
        self.order
    }
    pub fn budget(&self) -> f64 {
        self.budget
    }
    pub fn total_steps(&self) -> u64 {
        self.counts.iter().sum()
    }
    pub fn counts(&self) -> &[u64] {
        &self.counts
    }
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }
    pub fn work_upper(&self) -> f64 {
        self.work_upper
    }
    pub fn refinements(&self) -> u32 {
        self.refinements
    }
}

fn validate_allocation_input(
    blocks: &[AllocationBlock],
    order: u32,
    budget: f64,
    limits: AllocationLimits,
) -> Result<(), AllocationError> {
    if blocks.is_empty()
        || blocks.len() > 4096
        || !(1..=32).contains(&order)
        || !(budget.is_finite() && budget > 0.0)
    {
        return Err(AllocationError::Invalid(
            "1..4096 blocks, order1..32, positive finite budget required",
        ));
    }
    if limits.max_steps_per_block == 0
        || limits.max_steps_per_block > (1u64 << 53)
        || limits.max_total_steps == 0
        || limits.max_refinements > 64
    {
        return Err(AllocationError::Invalid(
            "bounded exactly representable counts/refinements required",
        ));
    }
    if blocks.iter().any(|b| {
        !(b.error_coefficient.is_finite()
            && b.error_coefficient >= 0.0
            && b.step_cost_upper.is_finite()
            && b.step_cost_upper > 0.0)
    }) {
        return Err(AllocationError::Invalid(
            "finite nonnegative A and positive c required",
        ));
    }
    Ok(())
}

fn allocation_bounds(
    blocks: &[AllocationBlock],
    order: u32,
    counts: &[u64],
    limits: AllocationLimits,
) -> Result<(f64, f64), AllocationError> {
    if blocks.len() != counts.len() {
        return Err(AllocationError::Invalid("count dimension"));
    }
    let (mut work, mut total) = (0.0, 0u64);
    let mut pointwise_error = Ok(0.0);
    let mut grouped = Vec::with_capacity(blocks.len());
    for (b, &n) in blocks.iter().zip(counts) {
        if n == 0 || n > limits.max_steps_per_block {
            return Err(AllocationError::ResourceLimit);
        }
        total = total.checked_add(n).ok_or(AllocationError::ResourceLimit)?;
        if total > limits.max_total_steps {
            return Err(AllocationError::ResourceLimit);
        }
        let mut term = b.error_coefficient;
        for _ in 0..order {
            term = div_up(term, n as f64)?;
        }
        pointwise_error = pointwise_error.and_then(|sum| add_up(sum, term));
        grouped.push((n, b.error_coefficient));
        work = add_up(work, mul_up(b.step_cost_upper, n as f64)?)?;
    }
    // Equal exact integer denominators permit sum(A_j)/n^p. Summing before
    // division can remove rounding inflation (e.g. (1+9)/20 = 1/2).
    // Both calculations are outward bounds on the same nonnegative sum; take
    // their minimum. Retain the original pointwise route if a grouped
    // numerator overflows, or regrouping gives a wider enclosure.
    grouped.sort_unstable_by_key(|&(n, _)| n);
    let grouped_error = (|| -> Result<f64, CoreError> {
        let (mut cursor, mut error) = (0, 0.0);
        while cursor < grouped.len() {
            let n = grouped[cursor].0;
            let mut numerator = 0.0;
            while cursor < grouped.len() && grouped[cursor].0 == n {
                numerator = add_up(numerator, grouped[cursor].1)?;
                cursor += 1;
            }
            for _ in 0..order {
                numerator = div_up(numerator, n as f64)?;
            }
            error = add_up(error, numerator)?;
        }
        Ok(error)
    })();
    let error = match (pointwise_error, grouped_error) {
        (Ok(a), Ok(b)) => a.min(b),
        (Ok(a), Err(_)) | (Err(_), Ok(a)) => a,
        (Err(error), Err(_)) => return Err(error.into()),
    };
    if !(error.is_finite() && work.is_finite()) {
        return Err(AllocationError::Invalid("nonfinite bound"));
    }
    Ok((error, work))
}

/// Validate supplied exact integer counts using directed division and addition.
/// Equal denominators are also grouped, retaining the smaller outward bound;
/// work remains the independently charged sum over every original block.
pub fn validate_work_allocation(
    blocks: &[AllocationBlock],
    order: u32,
    budget: f64,
    counts: &[u64],
    limits: AllocationLimits,
) -> Result<ValidatedWorkAllocation, AllocationError> {
    validate_allocation_input(blocks, order, budget, limits)?;
    let (error_upper, work_upper) = allocation_bounds(blocks, order, counts, limits)?;
    if error_upper > budget {
        return Err(AllocationError::Budget {
            upper: error_upper,
            budget,
        });
    }
    Ok(ValidatedWorkAllocation {
        blocks: blocks.to_vec(),
        counts: counts.to_vec(),
        order,
        budget,
        error_upper,
        work_upper,
        refinements: 0,
    })
}

/// G2's real optimum seeds a bounded integer search. Log/exp are heuristics
/// only: every returned count vector is independently validated outwardly.
/// No exact-optimality or W*+sum(c) claim is attached to the floating seed.
pub fn allocate_global_work(
    blocks: &[AllocationBlock],
    order: u32,
    budget: f64,
    limits: AllocationLimits,
) -> Result<ValidatedWorkAllocation, AllocationError> {
    validate_allocation_input(blocks, order, budget, limits)?;
    let p = order as f64;
    let logs: Vec<_> = blocks
        .iter()
        .map(|b| {
            if b.error_coefficient == 0.0 {
                f64::NEG_INFINITY
            } else {
                (b.error_coefficient.ln() + p * b.step_cost_upper.ln()) / (p + 1.0)
            }
        })
        .collect();
    let max = logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let log_s = if max.is_finite() {
        max + logs.iter().map(|x| (*x - max).exp()).sum::<f64>().ln()
    } else {
        f64::NEG_INFINITY
    };
    let mut counts = Vec::with_capacity(blocks.len());
    for b in blocks {
        let n = if b.error_coefficient == 0.0 {
            1.0
        } else {
            ((b.error_coefficient.ln() - b.step_cost_upper.ln()) / (p + 1.0)
                + (log_s - budget.ln()) / p)
                .exp()
                .ceil()
                .max(1.0)
        };
        if !n.is_finite() || n > limits.max_steps_per_block as f64 {
            return Err(AllocationError::ResourceLimit);
        }
        counts.push(n as u64);
    }
    for refinement in 0..=limits.max_refinements {
        let (error_upper, work_upper) = allocation_bounds(blocks, order, &counts, limits)?;
        if error_upper <= budget {
            return Ok(ValidatedWorkAllocation {
                blocks: blocks.to_vec(),
                counts,
                order,
                budget,
                error_upper,
                work_upper,
                refinements: refinement,
            });
        }
        if refinement == limits.max_refinements {
            return Err(AllocationError::Budget {
                upper: error_upper,
                budget,
            });
        }
        for (b, n) in blocks.iter().zip(&mut counts) {
            if b.error_coefficient > 0.0 {
                *n = if refinement == 0 {
                    n.checked_add(1)
                } else {
                    n.checked_mul(2)
                }
                .ok_or(AllocationError::ResourceLimit)?;
            }
        }
    }
    unreachable!("bounded inclusive loop returns")
}
