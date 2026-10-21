//! Causal transaction, budget, admission rule and regime telemetry (plan §6
//! C1, §4 idea 2, T8).
//!
//! This module is the single place where a speculative candidate step meets
//! the protected path. It holds no driver logic: a driver begins a
//! [`Trial`] on its committed state, charges every unit of work the
//! candidate performs, and either commits the trial (the candidate's state
//! passed the protected acceptance test) or rolls it back. Work charged to a
//! trial stays in the ledger whatever the outcome, so the ledger always
//! satisfies `attempted = committed + rolled back`.
//!
//! [`Budget`] is a hard cap in work units that is *reserved before
//! scheduling* (the `vigilode-coupled` `Work::reserve` pattern) with an
//! atomic compare-and-swap, so concurrent reservations never exceed the cap
//! (the `Pexprb54s4JvpBudget` pattern in `exponential.rs`).
//!
//! [`AdmissionRule`] is the T8 decision rule: a per-regime-bin
//! Beta(α, β) posterior on the candidate's success probability, updated only
//! from charged attempts, an upper-confidence admission test against the
//! break-even probability (C_a + C_v)/C_f, and the Proposition 8 throttle of
//! one attempt per m = ⌈10 (C_a + C_v)/C_f⌉ accepted protected steps while
//! the bound is below threshold. It is pure and deterministic: no wall time,
//! no randomness, costs are measured work units supplied by the caller.
//!
//! [`RegimeTelemetry`] is the plain-data record of the regime signals the
//! admission is allowed to see (Ritz summary, refresh contraction estimate,
//! Jacobian drift, rejection streak). Mapping telemetry to a [`RegimeBin`]
//! is the caller's choice and is not made here.

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

use rodas5p_core::WorkCounters;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------- work units

/// Maps a work ledger to scalar work units for budgets and ledgers.
pub type WorkMeasure = fn(&WorkCounters) -> u64;

/// Default [`WorkMeasure`]: user-callback evaluations, `rhs_evaluations +
/// jvp_vectors` (saturating), the two counters the `vigilode-coupled` budget
/// caps. Callers whose cost is dominated by factorizations or Krylov work
/// pass their own measure to [`Transaction::with_measure`].
pub fn callback_work_units(work: &WorkCounters) -> u64 {
    work.rhs_evaluations.saturating_add(work.jvp_vectors)
}

// --------------------------------------------------------------- transaction

/// How a trial ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrialOutcome {
    /// [`Trial::commit`]: the trial state became the committed state.
    Committed,
    /// [`Trial::rollback`]: explicit rollback.
    RolledBack,
    /// The trial was dropped without a decision; counted as a rollback.
    Dropped,
}

/// Cumulative accounting of every trial of one [`Transaction`].
///
/// Invariants (checked by [`Self::is_balanced`]): `trials_begun =
/// trials_committed + trials_rolled_back`, `trials_dropped ≤
/// trials_rolled_back`, `attempted_work = committed_work + rolled_back_work`
/// and the same for units. A [`Trial`] borrows the ledger mutably, so the
/// ledger can only be observed between trials, where the identities hold.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionLedger {
    pub trials_begun: u64,
    pub trials_committed: u64,
    /// Explicit rollbacks plus dropped trials.
    pub trials_rolled_back: u64,
    /// Trials dropped without commit or rollback (a subset of
    /// `trials_rolled_back`).
    pub trials_dropped: u64,
    /// All work charged to any trial, at the time it was charged.
    pub attempted_work: WorkCounters,
    pub committed_work: WorkCounters,
    pub rolled_back_work: WorkCounters,
    pub attempted_units: u64,
    pub committed_units: u64,
    pub rolled_back_units: u64,
    /// Budget units successfully reserved through [`Trial::reserve`]. Never
    /// refunded on rollback.
    pub reserved_units: u64,
    /// Reservations refused with [`BudgetExhausted`].
    pub reservation_refusals: u64,
    pub last_outcome: Option<TrialOutcome>,
}

impl TransactionLedger {
    /// Whether the accounting identities hold.
    pub fn is_balanced(&self) -> bool {
        let mut work = self.committed_work;
        work.accumulate(self.rolled_back_work);
        self.trials_committed.checked_add(self.trials_rolled_back) == Some(self.trials_begun)
            && self.trials_dropped <= self.trials_rolled_back
            && work == self.attempted_work
            && self.committed_units.checked_add(self.rolled_back_units)
                == Some(self.attempted_units)
    }
}

/// A transaction over a committed state `S` (state vector, controller state,
/// history, Krylov recycle space: whatever the caller must restore on
/// rejection).
///
/// The transaction does not own the committed state; it owns the ledger.
/// [`Self::begin`] clones the committed state into a [`Trial`].
#[derive(Debug)]
pub struct Transaction<S> {
    ledger: TransactionLedger,
    measure: WorkMeasure,
    _state: PhantomData<fn() -> S>,
}

impl<S: Clone> Default for Transaction<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Clone> Transaction<S> {
    /// A transaction measuring work with [`callback_work_units`].
    pub fn new() -> Self {
        Self::with_measure(callback_work_units)
    }

    /// A transaction measuring work with `measure`.
    pub fn with_measure(measure: WorkMeasure) -> Self {
        Self {
            ledger: TransactionLedger::default(),
            measure,
            _state: PhantomData,
        }
    }

    /// Start a trial on a copy of `committed`.
    pub fn begin(&mut self, committed: &S) -> Trial<'_, S> {
        self.ledger.trials_begun = self.ledger.trials_begun.saturating_add(1);
        Trial {
            state: Some(committed.clone()),
            ledger: &mut self.ledger,
            measure: self.measure,
            work: WorkCounters::default(),
            units: 0,
            reserved: 0,
        }
    }

    pub fn ledger(&self) -> &TransactionLedger {
        &self.ledger
    }

    pub fn measure(&self) -> WorkMeasure {
        self.measure
    }
}

/// One speculative attempt on a copy of the committed state.
///
/// Ends in exactly one of [`Self::commit`], [`Self::rollback`] or drop; a
/// drop is a rollback and is counted in
/// [`TransactionLedger::trials_dropped`].
#[derive(Debug)]
#[must_use = "a dropped trial is counted as a rollback; commit or roll back explicitly"]
pub struct Trial<'t, S> {
    state: Option<S>,
    ledger: &'t mut TransactionLedger,
    measure: WorkMeasure,
    work: WorkCounters,
    units: u64,
    reserved: u64,
}

impl<S> Trial<'_, S> {
    pub fn state(&self) -> &S {
        self.state
            .as_ref()
            .expect("trial state present until finished")
    }

    pub fn state_mut(&mut self) -> &mut S {
        self.state
            .as_mut()
            .expect("trial state present until finished")
    }

    /// Charge work the candidate has performed. It enters the ledger's
    /// attempted counters immediately and stays there on rollback. Returns
    /// the units of this charge under the transaction's measure.
    pub fn charge(&mut self, work: WorkCounters) -> u64 {
        let units = (self.measure)(&work);
        self.work.accumulate(work);
        self.units = self.units.saturating_add(units);
        self.ledger.attempted_work.accumulate(work);
        self.ledger.attempted_units = self.ledger.attempted_units.saturating_add(units);
        units
    }

    /// Reserve `units` on `budget` before scheduling work. A refusal is
    /// counted in the ledger and returned as the typed [`BudgetExhausted`].
    pub fn reserve(&mut self, budget: &Budget, units: u64) -> Result<(), BudgetExhausted> {
        match budget.reserve(units) {
            Ok(_) => {
                self.reserved = self.reserved.saturating_add(units);
                self.ledger.reserved_units = self.ledger.reserved_units.saturating_add(units);
                Ok(())
            }
            Err(exhausted) => {
                self.ledger.reservation_refusals =
                    self.ledger.reservation_refusals.saturating_add(1);
                Err(exhausted)
            }
        }
    }

    pub fn charged_work(&self) -> WorkCounters {
        self.work
    }

    pub fn charged_units(&self) -> u64 {
        self.units
    }

    pub fn reserved_units(&self) -> u64 {
        self.reserved
    }

    /// Accept the trial: its state is the new committed state.
    pub fn commit(mut self) -> S {
        let state = self
            .state
            .take()
            .expect("trial state present until finished");
        self.finish(TrialOutcome::Committed);
        state
    }

    /// Reject the trial: its state is discarded, its work stays charged.
    pub fn rollback(mut self) {
        self.state = None;
        self.finish(TrialOutcome::RolledBack);
    }

    fn finish(&mut self, outcome: TrialOutcome) {
        let l = &mut *self.ledger;
        match outcome {
            TrialOutcome::Committed => {
                l.trials_committed = l.trials_committed.saturating_add(1);
                l.committed_work.accumulate(self.work);
                l.committed_units = l.committed_units.saturating_add(self.units);
            }
            TrialOutcome::RolledBack | TrialOutcome::Dropped => {
                l.trials_rolled_back = l.trials_rolled_back.saturating_add(1);
                if outcome == TrialOutcome::Dropped {
                    l.trials_dropped = l.trials_dropped.saturating_add(1);
                }
                l.rolled_back_work.accumulate(self.work);
                l.rolled_back_units = l.rolled_back_units.saturating_add(self.units);
            }
        }
        l.last_outcome = Some(outcome);
    }
}

impl<S> Drop for Trial<'_, S> {
    fn drop(&mut self) {
        if self.state.take().is_some() {
            self.finish(TrialOutcome::Dropped);
        }
    }
}

// -------------------------------------------------------------------- budget

/// A reservation that would exceed the budget's cap. `used` is the work
/// already reserved when the request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error(
    "work budget exhausted before scheduling: {used} of {cap} units used, {requested} requested"
)]
pub struct BudgetExhausted {
    pub cap: u64,
    pub used: u64,
    pub requested: u64,
}

/// A hard cap in work units, reserved before the work is scheduled.
///
/// `reserve` is a compare-and-swap loop on an [`AtomicU64`], so any number
/// of threads sharing one budget (`&Budget` or `Arc<Budget>`) never reserve
/// more than `cap` in total. Reservations are never refunded: work that was
/// scheduled is charged whether or not it was used.
#[derive(Debug)]
pub struct Budget {
    cap: u64,
    used: AtomicU64,
    refusals: AtomicU64,
}

impl Budget {
    pub fn new(cap: u64) -> Self {
        Self {
            cap,
            used: AtomicU64::new(0),
            refusals: AtomicU64::new(0),
        }
    }

    /// Reserve `units`; returns the work used after the reservation. A zero
    /// request always succeeds. A request that would exceed the cap (or
    /// overflow `u64`) reserves nothing and returns [`BudgetExhausted`].
    pub fn reserve(&self, units: u64) -> Result<u64, BudgetExhausted> {
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            let next = match current.checked_add(units) {
                Some(next) if next <= self.cap => next,
                _ => {
                    self.refusals.fetch_add(1, Ordering::Relaxed);
                    return Err(BudgetExhausted {
                        cap: self.cap,
                        used: current,
                        requested: units,
                    });
                }
            };
            if units == 0 {
                return Ok(current);
            }
            match self.used.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(next),
                Err(observed) => current = observed,
            }
        }
    }

    pub fn cap(&self) -> u64 {
        self.cap
    }

    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    pub fn remaining(&self) -> u64 {
        self.cap.saturating_sub(self.used())
    }

    /// Number of refused reservations.
    pub fn refusals(&self) -> u64 {
        self.refusals.load(Ordering::Relaxed)
    }
}

// ------------------------------------------------------------- admission (T8)

/// A regime bin, assigned by the caller from [`RegimeTelemetry`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegimeBin(pub u32);

/// Measured costs in work units: candidate attempt C_a, its acceptance check
/// C_v, and the protected step C_f it would replace (T8).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CandidateCosts {
    pub attempt: f64,
    pub verify: f64,
    pub fallback: f64,
}

impl CandidateCosts {
    fn validate(self) -> Result<(), AdmissionError> {
        if !(self.attempt.is_finite() && self.attempt >= 0.0) {
            return Err(AdmissionError::InvalidCost(
                "attempt cost must be finite and nonnegative",
            ));
        }
        if !(self.verify.is_finite() && self.verify >= 0.0) {
            return Err(AdmissionError::InvalidCost(
                "verification cost must be finite and nonnegative",
            ));
        }
        if !(self.fallback.is_finite() && self.fallback > 0.0) {
            return Err(AdmissionError::InvalidCost(
                "fallback cost must be finite and positive",
            ));
        }
        if !(self.attempt + self.verify).is_finite() {
            return Err(AdmissionError::InvalidCost(
                "attempt plus verification cost overflows",
            ));
        }
        Ok(())
    }

    /// Break-even success probability τ = (C_a + C_v)/C_f: an attempt lowers
    /// expected cost iff p > τ (T8).
    pub fn threshold(self) -> Result<f64, AdmissionError> {
        self.validate()?;
        Ok((self.attempt + self.verify) / self.fallback)
    }

    /// Proposition 8 throttle m = ⌈factor · τ⌉, at least 1; with the default
    /// factor 10, m = ⌈10 (C_a + C_v)/C_f⌉ caps the worst-case overhead at
    /// τ/m ≤ 1/10.
    pub fn throttle_interval(self, overhead_factor: f64) -> Result<u64, AdmissionError> {
        let tau = self.threshold()?;
        if !(overhead_factor.is_finite() && overhead_factor >= 1.0) {
            return Err(AdmissionError::InvalidConfig(
                "overhead factor must be finite and at least 1",
            ));
        }
        let m = (overhead_factor * tau).ceil();
        if m.is_nan() || m >= u64::MAX as f64 {
            return Err(AdmissionError::InvalidCost(
                "throttle interval overflows u64",
            ));
        }
        Ok((m as u64).max(1))
    }
}

/// Configuration of [`AdmissionRule`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdmissionConfig {
    /// Beta prior pseudo-successes α₀ (> 0).
    pub prior_alpha: f64,
    /// Beta prior pseudo-failures β₀ (> 0).
    pub prior_beta: f64,
    /// One-sided normal quantile z of the upper bound; the default
    /// 1.6448536269514722 is z₀.₉₅.
    pub z: f64,
    /// 1 / (admissible overhead fraction): m = ⌈factor · τ⌉ and the overhead
    /// guard allows a net candidate overhead of (1/factor) × protected cost.
    /// The default 10 is the G-C gate (10%).
    pub overhead_factor: f64,
}

impl Default for AdmissionConfig {
    fn default() -> Self {
        Self {
            prior_alpha: 1.0,
            prior_beta: 1.0,
            z: 1.644_853_626_951_472_2,
            overhead_factor: 10.0,
        }
    }
}

impl AdmissionConfig {
    pub fn validate(&self) -> Result<(), AdmissionError> {
        if !(self.prior_alpha.is_finite() && self.prior_alpha > 0.0) {
            return Err(AdmissionError::InvalidConfig(
                "prior alpha must be finite and positive",
            ));
        }
        if !(self.prior_beta.is_finite() && self.prior_beta > 0.0) {
            return Err(AdmissionError::InvalidConfig(
                "prior beta must be finite and positive",
            ));
        }
        if !(self.z.is_finite() && self.z > 0.0) {
            return Err(AdmissionError::InvalidConfig(
                "z must be finite and positive",
            ));
        }
        if !(self.overhead_factor.is_finite() && self.overhead_factor >= 1.0) {
            return Err(AdmissionError::InvalidConfig(
                "overhead factor must be finite and at least 1",
            ));
        }
        if !(self.prior_alpha + self.prior_beta).is_finite()
            || !(self.z * self.z).is_finite()
            || !beta_upper_confidence_bound(self.prior_alpha, self.prior_beta, self.z).is_finite()
        {
            return Err(AdmissionError::InvalidConfig(
                "posterior confidence arithmetic is not representable",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum AdmissionError {
    #[error("invalid cost: {0}")]
    InvalidCost(&'static str),
    #[error("invalid admission config: {0}")]
    InvalidConfig(&'static str),
    /// The posterior is updated only from charged attempts (T8 causality).
    #[error("an attempt with zero charged work cannot update the posterior")]
    UnchargedAttempt,
}

/// Outcome of [`AdmissionRule::decide`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Admission {
    /// Upper confidence bound of p above τ, overhead guard satisfied.
    AdmitPosterior,
    /// Bound at or below τ; the Proposition 8 throttle allows this attempt.
    AdmitThrottled,
    /// Bound at or below τ and fewer than m protected steps since the last
    /// attempt in this bin.
    DeclineThrottle,
    /// The attempt could push the bin's net overhead above the allowance.
    DeclineOverhead,
}

impl Admission {
    pub fn is_admitted(self) -> bool {
        matches!(self, Self::AdmitPosterior | Self::AdmitThrottled)
    }
}

/// A charged candidate attempt and its outcome.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// Measured C_a + C_v actually charged (> 0).
    pub charged_units: f64,
    /// The candidate's state passed the protected acceptance test and was
    /// committed.
    pub success: bool,
    /// Measured cost of the protected step the attempt replaced (used only
    /// on success, as the counterfactual protected cost it saved).
    pub replaced_protected_units: f64,
}

/// Per-bin posterior and overhead account.
///
/// The overhead account compares the actual cost of the bin with the cost
/// the protected path alone would have had: `protected_base` is the
/// counterfactual protected cost (protected steps plus the protected steps
/// replaced by successful candidates), and `net_overhead` is actual minus
/// counterfactual (charged candidate work minus the protected work saved by
/// successes).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BinState {
    pub alpha: f64,
    pub beta: f64,
    pub attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub protected_steps: u64,
    pub protected_steps_since_attempt: u64,
    pub charged_units: f64,
    pub protected_base: f64,
    pub net_overhead: f64,
}

impl BinState {
    fn fresh(config: &AdmissionConfig) -> Self {
        Self {
            alpha: config.prior_alpha,
            beta: config.prior_beta,
            attempts: 0,
            successes: 0,
            failures: 0,
            protected_steps: 0,
            protected_steps_since_attempt: 0,
            charged_units: 0.0,
            protected_base: 0.0,
            net_overhead: 0.0,
        }
    }
}

/// Upper confidence bound on p for a Beta(α, β) posterior.
///
/// We use the Wilson score upper bound evaluated at the posterior
/// pseudo-counts, n = α + β and p̂ = α/n:
///
/// ```text
/// U = (p̂ + z²/(2n) + z √(p̂(1 − p̂)/n + z²/(4n²))) / (1 + z²/n).
/// ```
///
/// Why this bound: it is closed form and therefore bit-deterministic (no
/// iterative incomplete-beta inversion); it lies strictly inside (0, 1) for
/// α, β > 0, is at least the posterior mean p̂, increases with α and
/// decreases with β, so successes raise and failures lower it monotonically;
/// and it is the score-test inversion that remains well behaved at the small
/// n where admission decisions are made. It *approximates* the z-level Beta
/// quantile rather than bounding it: against the exact 0.95 quantile it is
/// lower when successes dominate at small n (Beta(1, 1): 0.879 vs 0.950;
/// Beta(5, 1): 0.962 vs 0.990) and higher when failures dominate
/// (Beta(1, 20): 0.188 vs 0.139). The bounded-overhead guarantee of
/// [`AdmissionRule`] does not depend on the bound's accuracy: it is enforced
/// by the throttle and the overhead guard.
pub fn beta_upper_confidence_bound(alpha: f64, beta: f64, z: f64) -> f64 {
    let n = alpha + beta;
    let p = alpha / n;
    let z2 = z * z;
    let centre = p + z2 / (2.0 * n);
    let spread = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    (centre + spread) / (1.0 + z2 / n)
}

/// T8 admission rule with per-regime Beta posteriors and the Proposition 8
/// bounded-overhead throttle.
///
/// For a bin with posterior Beta(α, β), costs C = (C_a, C_v, C_f),
/// τ = (C_a + C_v)/C_f and m = ⌈10 τ⌉, [`Self::decide`] returns
///
/// 1. [`Admission::AdmitPosterior`] when U(α, β) > τ (see
///    [`beta_upper_confidence_bound`]);
/// 2. otherwise [`Admission::AdmitThrottled`] when at least m accepted
///    protected steps have passed in this bin since its last attempt, and
///    [`Admission::DeclineThrottle`] before that;
///
/// and either admission is replaced by [`Admission::DeclineOverhead`] when
/// `net_overhead + C_a + C_v > protected_base / 10` (overhead guard).
///
/// The guard is what makes the 10% bound hold for *every* candidate. The
/// throttle alone bounds the overhead of attempts made below threshold by
/// τ/m ≤ 1/10 (Proposition 8), but the attempts admitted while an
/// optimistic prior keeps U above τ are not throttled, and a candidate that
/// always fails spends several of them before U falls below τ. The guard
/// charges both kinds against one allowance, so the bin's actual cost never
/// exceeds 1.1 × its counterfactual protected cost by more than the
/// difference between one estimated and one measured attempt cost. Because
/// successes reduce `net_overhead` by the protected step they replace, the
/// guard never binds for a candidate that pays for itself. A fresh bin
/// therefore attempts only after about m protected steps.
///
/// Causality: the posterior changes only in [`Self::record_attempt`], which
/// requires positive charged work; protected steps update only the overhead
/// account and the throttle counter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdmissionRule {
    config: AdmissionConfig,
    bins: BTreeMap<RegimeBin, BinState>,
}

impl AdmissionRule {
    /// Panics if `config` is invalid; see [`Self::try_new`].
    pub fn new(config: AdmissionConfig) -> Self {
        Self::try_new(config).expect("valid admission config")
    }

    pub fn try_new(config: AdmissionConfig) -> Result<Self, AdmissionError> {
        config.validate()?;
        Ok(Self {
            config,
            bins: BTreeMap::new(),
        })
    }

    pub fn config(&self) -> &AdmissionConfig {
        &self.config
    }

    /// The bin's state, `None` before any record in it.
    pub fn bin_state(&self, bin: RegimeBin) -> Option<&BinState> {
        self.bins.get(&bin)
    }

    fn state(&self, bin: RegimeBin) -> BinState {
        self.bins
            .get(&bin)
            .copied()
            .unwrap_or_else(|| BinState::fresh(&self.config))
    }

    fn state_mut(&mut self, bin: RegimeBin) -> &mut BinState {
        let config = self.config;
        self.bins
            .entry(bin)
            .or_insert_with(|| BinState::fresh(&config))
    }

    /// Posterior (α, β) of the bin.
    pub fn posterior(&self, bin: RegimeBin) -> (f64, f64) {
        let s = self.state(bin);
        (s.alpha, s.beta)
    }

    pub fn upper_confidence_bound(&self, bin: RegimeBin) -> f64 {
        let s = self.state(bin);
        beta_upper_confidence_bound(s.alpha, s.beta, self.config.z)
    }

    /// Decide whether to attempt the candidate at the next step of `bin`.
    pub fn decide(
        &self,
        bin: RegimeBin,
        costs: CandidateCosts,
    ) -> Result<Admission, AdmissionError> {
        let tau = costs.threshold()?;
        let m = costs.throttle_interval(self.config.overhead_factor)?;
        let s = self.state(bin);
        let guard_ok = s.net_overhead + (costs.attempt + costs.verify)
            <= s.protected_base / self.config.overhead_factor;
        let ucb = beta_upper_confidence_bound(s.alpha, s.beta, self.config.z);
        let admission = if ucb > tau {
            Admission::AdmitPosterior
        } else if s.protected_steps_since_attempt >= m {
            Admission::AdmitThrottled
        } else {
            return Ok(Admission::DeclineThrottle);
        };
        Ok(if guard_ok {
            admission
        } else {
            Admission::DeclineOverhead
        })
    }

    /// Record a charged attempt. This is the only update of the posterior.
    /// A cumulative arithmetic overflow is refused without changing the bin.
    /// Callers must preserve the actual work in their transaction ledger even
    /// if this separate admission account can no longer represent the sum.
    pub fn record_attempt(
        &mut self,
        bin: RegimeBin,
        record: AttemptRecord,
    ) -> Result<(), AdmissionError> {
        if record.charged_units == 0.0 {
            return Err(AdmissionError::UnchargedAttempt);
        }
        if !(record.charged_units.is_finite() && record.charged_units > 0.0) {
            return Err(AdmissionError::InvalidCost(
                "charged units must be finite and positive",
            ));
        }
        if !(record.replaced_protected_units.is_finite() && record.replaced_protected_units >= 0.0)
        {
            return Err(AdmissionError::InvalidCost(
                "replaced protected units must be finite and nonnegative",
            ));
        }
        let mut next = self.state(bin);
        let s = &mut next;
        s.attempts = s.attempts.saturating_add(1);
        s.protected_steps_since_attempt = 0;
        s.charged_units += record.charged_units;
        s.net_overhead += record.charged_units;
        if record.success {
            s.alpha += 1.0;
            s.successes = s.successes.saturating_add(1);
            s.net_overhead -= record.replaced_protected_units;
            s.protected_base += record.replaced_protected_units;
        } else {
            s.beta += 1.0;
            s.failures = s.failures.saturating_add(1);
        }
        if ![
            s.alpha + s.beta,
            s.charged_units,
            s.net_overhead,
            s.protected_base,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(AdmissionError::InvalidCost(
                "cumulative admission account overflows",
            ));
        }
        *self.state_mut(bin) = next;
        Ok(())
    }

    /// Record an accepted protected step (including the fallback after a
    /// failed attempt) of measured cost `units` (> 0).
    pub fn record_protected_step(
        &mut self,
        bin: RegimeBin,
        units: f64,
    ) -> Result<(), AdmissionError> {
        if !(units.is_finite() && units > 0.0) {
            return Err(AdmissionError::InvalidCost(
                "protected step units must be finite and positive",
            ));
        }
        let mut next = self.state(bin);
        let s = &mut next;
        let protected_base = s.protected_base + units;
        if !protected_base.is_finite() {
            return Err(AdmissionError::InvalidCost(
                "cumulative protected cost overflows",
            ));
        }
        s.protected_steps = s.protected_steps.saturating_add(1);
        s.protected_steps_since_attempt = s.protected_steps_since_attempt.saturating_add(1);
        s.protected_base = protected_base;
        *self.state_mut(bin) = next;
        Ok(())
    }
}

// ----------------------------------------------------------------- telemetry

/// Summary of the Ritz values of the current Jacobian (or W) Krylov space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RitzSummary {
    /// max |θ| over the Ritz values θ; an estimate, not a bound.
    pub spectral_radius_estimate: f64,
    /// max |Im θ| / |Re θ|, the oscillation signal. A Ritz value with
    /// Re θ = 0 makes this infinite, which JSON cannot carry; callers cap it
    /// or omit the summary.
    pub max_imag_over_real: f64,
    /// Number of Ritz values summarized.
    pub ritz_values: u32,
}

/// Regime signals the admission may observe (plan §4: telemetry is the only
/// admission input). Plain data; `None` means "not estimated at this step".
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RegimeTelemetry {
    pub ritz: Option<RitzSummary>,
    /// Estimated contraction q of the refresh/stage-root iteration.
    pub refresh_q_estimate: Option<f64>,
    /// Estimated Jacobian drift δ_J since the last factorization.
    pub jacobian_drift_estimate: Option<f64>,
    /// Consecutive rejected steps on the protected path.
    pub rejection_streak: u32,
}
