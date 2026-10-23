//! Pure, allocation-free physical error accounting at a fixed output horizon.
//!
//! Clients supply contributions already integrated over a continuous segment or
//! bounded at a jump, and a future amplification bound in the SAME physical norm.
//! A sampled residual is only an estimate. Neither numerical step stability nor
//! mass conservation proves the required continuous growth/consistency bounds.
//! For an UNWEIGHTED segment integral q, amplification must bound
//! `sup_{s in segment} Gamma(T,s)`, not merely Gamma at the endpoint. Positive
//! growth can make endpoint amplification underestimate an earlier residual.
//! An already Gamma-weighted integral instead uses amplification 1.
//! This module checks accounting, not the truth of a client's analytic premises.

use rodas5p_core::directed::Interval;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetError {
    InvalidMagnitude,
    MissingBoundProvenance,
    InvalidAmplification,
    IdentityMismatch,
    DuplicatePerturbation,
    CapacityExceeded,
    ComponentOverspent,
    TotalOverspent,
    ArithmeticOverflow,
}

/// IDs bind every charge to one client's norm, model epoch and output horizon.
/// A changed mesh may retain this identity only if its norm equivalence and
/// remap amplification have been included by the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetIdentity {
    pub physical_norm: u64,
    pub model_epoch: u64,
    pub output_horizon: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ErrorComponent {
    Initial = 0,
    TimeResidual = 1,
    SpatialDefect = 2,
    FlowOrRoot = 3,
    RemapConsistency = 4,
    Roundoff = 5,
}

/// `Bounded` records a caller assertion with justified continuous/integral
/// premises; it is not an independently verified certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceStatus {
    Bounded,
    Estimated,
}

/// Nonzero client receipt/reference identity for analytic or interval premises.
/// The referenced evidence remains the client's responsibility to supply/review.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundProvenance(u64);
impl BoundProvenance {
    pub fn new(reference: u64) -> Result<Self, BudgetError> {
        if reference == 0 {
            return Err(BudgetError::MissingBoundProvenance);
        }
        Ok(Self(reference))
    }
    pub fn reference(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorMagnitude {
    value: f64,
    status: EvidenceStatus,
    provenance: Option<BoundProvenance>,
}

impl ErrorMagnitude {
    pub fn bounded(value: f64, provenance: BoundProvenance) -> Result<Self, BudgetError> {
        let mut result = Self::new(value, EvidenceStatus::Bounded)?;
        result.provenance = Some(provenance);
        Ok(result)
    }
    pub fn estimated(value: f64) -> Result<Self, BudgetError> {
        Self::new(value, EvidenceStatus::Estimated)
    }
    /// A quadrature/sampling result alone never acquires bounded status.
    pub fn sampled_residual(value: f64) -> Result<Self, BudgetError> {
        Self::estimated(value)
    }
    pub fn provenance(self) -> Option<BoundProvenance> {
        self.provenance
    }
    fn new(value: f64, status: EvidenceStatus) -> Result<Self, BudgetError> {
        if !value.is_finite() || value < 0.0 {
            return Err(BudgetError::InvalidMagnitude);
        }
        Ok(Self {
            value,
            status,
            provenance: None,
        })
    }
}

/// Future amplification, including intervening remap norms. Every G used as a
/// divisor or multiplier must be finite and strictly positive. A one-step
/// numerical stability estimate must use `estimated`, absent separate proof of
/// the continuous physical growth bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Amplification {
    value: f64,
    status: EvidenceStatus,
    provenance: Option<BoundProvenance>,
}

impl Amplification {
    pub fn bounded(value: f64, provenance: BoundProvenance) -> Result<Self, BudgetError> {
        let mut result = Self::new(value, EvidenceStatus::Bounded)?;
        result.provenance = Some(provenance);
        Ok(result)
    }
    pub fn estimated(value: f64) -> Result<Self, BudgetError> {
        Self::new(value, EvidenceStatus::Estimated)
    }
    pub fn provenance(self) -> Option<BoundProvenance> {
        self.provenance
    }
    fn new(value: f64, status: EvidenceStatus) -> Result<Self, BudgetError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(BudgetError::InvalidAmplification);
        }
        Ok(Self {
            value,
            status,
            provenance: None,
        })
    }
    /// Conservative local allowance for a final-output allowance.
    /// Derived arithmetic returns Estimated: this finite record does not retain
    /// the complete proof lineage of both inputs. A client may attach a reviewed
    /// derived receipt using ErrorMagnitude::bounded before bounded admission.
    pub fn local_allowance(
        self,
        final_allowance: ErrorMagnitude,
    ) -> Result<ErrorMagnitude, BudgetError> {
        if final_allowance.value / self.value >= f64::MAX {
            return Err(BudgetError::ArithmeticOverflow);
        }
        let value = point(final_allowance.value)?
            .div(point(self.value)?)
            .map_err(|_| BudgetError::ArithmeticOverflow)?
            .lo
            .max(0.0);
        ErrorMagnitude::estimated(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contribution {
    pub perturbation_id: u64,
    pub component: ErrorComponent,
    pub magnitude: ErrorMagnitude,
    pub amplification: Amplification,
}

/// Order is part of the physical contract, never inferred from equal timestamps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JumpOrder {
    /// The flow jump is measured in the new norm AFTER remapping.
    RemapThenFlow,
    /// The flow jump occurs in the old space and must pass through this remap.
    FlowThenRemap { remap_norm: ErrorMagnitude },
}

/// Construct separate remap and flow charges. `future` starts after BOTH jumps;
/// remap consistency is in the new norm, and is never multiplied by remap_norm.
/// Roundoff/correction has its own contribution rather than being hidden here.
/// FlowThenRemap derives a new magnitude and deliberately downgrades that charge
/// to Estimated because this helper does not retain a complete proof lineage.
pub fn remap_flow_contributions(
    remap_id: u64,
    remap_defect: ErrorMagnitude,
    flow_id: u64,
    flow_jump: ErrorMagnitude,
    future: Amplification,
    order: JumpOrder,
) -> Result<[Contribution; 2], BudgetError> {
    let flow_jump = match order {
        JumpOrder::RemapThenFlow => flow_jump,
        JumpOrder::FlowThenRemap { remap_norm } => ErrorMagnitude::new(
            product(flow_jump.value, remap_norm.value)?,
            EvidenceStatus::Estimated,
        )?,
    };
    Ok([
        Contribution {
            perturbation_id: remap_id,
            component: ErrorComponent::RemapConsistency,
            magnitude: remap_defect,
            amplification: future,
        },
        Contribution {
            perturbation_id: flow_id,
            component: ErrorComponent::FlowOrRoot,
            magnitude: flow_jump,
            amplification: future,
        },
    ])
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalErrorBudget<const N: usize> {
    identity: BudgetIdentity,
    total_cap: f64,
    component_caps: [f64; 6],
    spent: [f64; 6],
    ids: [u64; N],
    count: usize,
    status: EvidenceStatus,
}

impl<const N: usize> PhysicalErrorBudget<N> {
    pub fn new(
        identity: BudgetIdentity,
        total_cap: f64,
        component_caps: [f64; 6],
    ) -> Result<Self, BudgetError> {
        ErrorMagnitude::new(total_cap, EvidenceStatus::Estimated)?;
        for cap in component_caps {
            ErrorMagnitude::new(cap, EvidenceStatus::Estimated)?;
        }
        Ok(Self {
            identity,
            total_cap,
            component_caps,
            spent: [0.0; 6],
            ids: [0; N],
            count: 0,
            status: EvidenceStatus::Bounded,
        })
    }

    /// Validate the complete candidate before changing the accepted ledger.
    /// Reuse ONE ID for the same physical perturbation wherever represented;
    /// this rejects counting a flow defect both in residual and endpoint jump.
    /// Omission and dishonest IDs cannot be detected by an accounting layer.
    pub fn commit(
        &mut self,
        identity: BudgetIdentity,
        candidate: &[Contribution],
    ) -> Result<(), BudgetError> {
        if identity != self.identity {
            return Err(BudgetError::IdentityMismatch);
        }
        if candidate.len() > N - self.count {
            return Err(BudgetError::CapacityExceeded);
        }
        let mut spent = self.spent;
        let mut status = self.status;
        for (index, charge) in candidate.iter().enumerate() {
            if self.ids[..self.count].contains(&charge.perturbation_id)
                || candidate[..index]
                    .iter()
                    .any(|prior| prior.perturbation_id == charge.perturbation_id)
            {
                return Err(BudgetError::DuplicatePerturbation);
            }
            let amount = product(charge.magnitude.value, charge.amplification.value)?;
            let slot = charge.component as usize;
            spent[slot] = sum(spent[slot], amount)?;
            if spent[slot] > self.component_caps[slot] {
                return Err(BudgetError::ComponentOverspent);
            }
            status = combine(
                status,
                combine(charge.magnitude.status, charge.amplification.status),
            );
        }
        if total(&spent)? > self.total_cap {
            return Err(BudgetError::TotalOverspent);
        }
        self.spent = spent;
        self.status = status;
        for charge in candidate {
            self.ids[self.count] = charge.perturbation_id;
            self.count += 1;
        }
        Ok(())
    }

    pub fn status(&self) -> EvidenceStatus {
        self.status
    }
    pub fn component_spent(&self) -> [f64; 6] {
        self.spent
    }
    pub fn total_spent(&self) -> Result<f64, BudgetError> {
        total(&self.spent)
    }
    pub fn charged_perturbations(&self) -> usize {
        self.count
    }
}

fn combine(a: EvidenceStatus, b: EvidenceStatus) -> EvidenceStatus {
    if a == EvidenceStatus::Bounded && b == EvidenceStatus::Bounded {
        EvidenceStatus::Bounded
    } else {
        EvidenceStatus::Estimated
    }
}
fn point(value: f64) -> Result<Interval, BudgetError> {
    Interval::point(value).map_err(|_| BudgetError::ArithmeticOverflow)
}
// Precheck range before fallible Interval operations: core's error path owns
// a diagnostic String. Refuse the boundary MAX conservatively so even arithmetic
// refusal in this layer never enters that allocating path.
fn product(a: f64, b: f64) -> Result<f64, BudgetError> {
    if a * b >= f64::MAX {
        return Err(BudgetError::ArithmeticOverflow);
    }
    Ok(point(a)?
        .mul(point(b)?)
        .map_err(|_| BudgetError::ArithmeticOverflow)?
        .hi)
}
fn sum(a: f64, b: f64) -> Result<f64, BudgetError> {
    if a + b >= f64::MAX {
        return Err(BudgetError::ArithmeticOverflow);
    }
    Ok(point(a)?
        .add(point(b)?)
        .map_err(|_| BudgetError::ArithmeticOverflow)?
        .hi)
}
fn total(values: &[f64; 6]) -> Result<f64, BudgetError> {
    let mut result = 0.0;
    for &value in values {
        result = sum(result, value)?;
    }
    Ok(result)
}

/// Bounded is a client assertion backed by named premises, never an automatic
/// certificate. Operational mode admits estimates while retaining their status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalAdmissionMode {
    BoundedRequired,
    Operational,
}

/// Fixed physical and reconstruction identity supplied by the client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalCandidateBinding {
    pub identity: BudgetIdentity,
    pub trial_id: u64,
    pub reconstruction_id: u64,
}

/// Borrowed immutable candidate data. Clients must bind any reconstruction or
/// integral proof to this exact trial, physical identity, and endpoint pair.
/// An endpoint pair alone does not establish a continuous residual bound.
#[derive(Clone, Copy, Debug)]
pub struct PhysicalCandidateContext<'a> {
    pub identity: BudgetIdentity,
    pub trial_id: u64,
    pub reconstruction_id: u64,
    pub old_time: f64,
    pub candidate_time: f64,
    pub old_state: &'a [f64],
    pub candidate_state: &'a [f64],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicalBoundsError {
    MissingBound,
    CallbackFailure,
}

/// These are terminal physical admission refusals. They do not authorize a
/// retry with widened tolerances, a new physical model, or a reset budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopOnPhysicalRefusal {
    InvalidCandidateContext,
    Budget(BudgetError),
    Provider(PhysicalBoundsError),
    MissingContributions,
    BoundedEvidenceRequired,
}

/// Actual callback attempts and outcomes, accumulated even on physical refusal.
/// The solver's RHS/JVP/etc counters remain separately charged by its driver.
/// A dropped prepared token spends provider work but no accepted error budget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhysicalAdmissionWork {
    pub locally_rejected_trials: u64,
    pub provider_attempts: u64,
    pub provider_completed: u64,
    pub physical_refusals: u64,
    pub prepared_candidates: u64,
    pub committed_candidates: u64,
}

/// Exclusive prepared transaction. The accepted ledger is untouched until the
/// infallible commit; accepted time/state are exclusively borrowed for the
/// token lifetime and cannot drift to a different checkpoint. Dropping the token
/// preserves accepted time, state and ledger. No heap storage is created.
///
/// Safe Rust rejects mutation of the held checkpoint before token disposal:
/// ```compile_fail
/// use rodas5p_integrators::physical_error_budget::*;
/// let id = BudgetIdentity { physical_norm: 1, model_epoch: 1, output_horizon: 1 };
/// let key = PhysicalCandidateBinding { identity: id, trial_id: 1, reconstruction_id: 1 };
/// let mut budget = PhysicalErrorBudget::<4>::new(id, 1.0, [1.0; 6]).unwrap();
/// let mut work = PhysicalAdmissionWork::default();
/// let mut time = 0.0;
/// let mut state = [0.0];
/// let candidate = [0.125];
/// let mut scratch = [Contribution { perturbation_id: 1,
///     component: ErrorComponent::TimeResidual,
///     magnitude: ErrorMagnitude::estimated(0.125).unwrap(),
///     amplification: Amplification::estimated(1.0).unwrap() }];
/// let mut provider = |_: &PhysicalCandidateContext<'_>, _: &mut [Contribution; 1]| Ok(1);
/// let prepared = prepare_physical_candidate(&mut budget, &mut work, key,
///     &mut time, &mut state, 0.5, &candidate, true,
///     PhysicalAdmissionMode::Operational, &mut scratch, &mut provider).unwrap();
/// time = 0.75;       // cannot mutate the exclusively borrowed checkpoint
/// state[0] = 99.0;   // cannot mutate its accepted state either
/// drop(prepared);
/// ```
///
/// Driver contract: prepare AFTER all local/dense checks; compute the next
/// controller on a shadow, and perform every fallible output validation BEFORE
/// accepted state/time/cache/controller mutation. Then commit those changes and
/// this token without another fallible operation. This consumer does not itself
/// install a hook into any native integration loop.
#[derive(Debug)]
pub struct PreparedPhysicalCandidate<'a, const N: usize> {
    ledger: &'a mut PhysicalErrorBudget<N>,
    staged: PhysicalErrorBudget<N>,
    work: &'a mut PhysicalAdmissionWork,
    accepted_time: &'a mut f64,
    accepted_state: &'a mut [f64],
    candidate_time: f64,
    candidate_state: &'a [f64],
}
impl<const N: usize> PreparedPhysicalCandidate<'_, N> {
    pub fn status(&self) -> EvidenceStatus {
        self.staged.status()
    }
    pub fn total_spent(&self) -> Result<f64, BudgetError> {
        self.staged.total_spent()
    }
    pub fn commit(self) {
        self.accepted_state.copy_from_slice(self.candidate_state);
        *self.accepted_time = self.candidate_time;
        *self.ledger = self.staged;
        self.work.committed_candidates = self.work.committed_candidates.saturating_add(1);
    }
}

#[derive(Debug)]
pub enum PhysicalCandidatePreparation<'a, const N: usize> {
    LocalRejected,
    Prepared(PreparedPhysicalCandidate<'a, N>),
}

/// Consume a justified client-supplied candidate bound into a prepared budget.
/// The callback fills caller-owned scratch and returns its initialized prefix.
/// It must give each physical perturbation a stable, unique ID, including a
/// zero-valued charge when a justified candidate defect is exactly zero.
///
/// Local rejection invokes NO evidence callback. Identity/context refusal also
/// precedes callback invocation. Provider attempts are charged before invoking
/// it; work is never rolled back with a candidate. Fixed-capacity staging and
/// all arithmetic/identity/evidence checks precede accepted-ledger mutation.
#[allow(clippy::too_many_arguments)]
pub fn prepare_physical_candidate<'a, const N: usize, const K: usize, F>(
    ledger: &'a mut PhysicalErrorBudget<N>,
    work: &'a mut PhysicalAdmissionWork,
    binding: PhysicalCandidateBinding,
    accepted_time: &'a mut f64,
    accepted_state: &'a mut [f64],
    candidate_time: f64,
    candidate_state: &'a [f64],
    locally_accepted: bool,
    mode: PhysicalAdmissionMode,
    scratch: &mut [Contribution; K],
    provider: &mut F,
) -> Result<PhysicalCandidatePreparation<'a, N>, StopOnPhysicalRefusal>
where
    F: FnMut(
        &PhysicalCandidateContext<'_>,
        &mut [Contribution; K],
    ) -> Result<usize, PhysicalBoundsError>,
{
    if !locally_accepted {
        work.locally_rejected_trials = work.locally_rejected_trials.saturating_add(1);
        return Ok(PhysicalCandidatePreparation::LocalRejected);
    }
    let context = PhysicalCandidateContext {
        identity: binding.identity,
        trial_id: binding.trial_id,
        reconstruction_id: binding.reconstruction_id,
        old_time: *accepted_time,
        candidate_time,
        old_state: &*accepted_state,
        candidate_state,
    };
    let mut prepare = || -> Result<PhysicalErrorBudget<N>, StopOnPhysicalRefusal> {
        if ledger.identity != context.identity {
            return Err(StopOnPhysicalRefusal::Budget(BudgetError::IdentityMismatch));
        }
        if !context.old_time.is_finite()
            || !context.candidate_time.is_finite()
            || context.candidate_time <= context.old_time
            || context.old_state.is_empty()
            || context.old_state.len() != context.candidate_state.len()
            || context
                .old_state
                .iter()
                .chain(context.candidate_state)
                .any(|v| !v.is_finite())
        {
            return Err(StopOnPhysicalRefusal::InvalidCandidateContext);
        }
        work.provider_attempts = work.provider_attempts.saturating_add(1);
        let used = provider(&context, scratch).map_err(StopOnPhysicalRefusal::Provider)?;
        work.provider_completed = work.provider_completed.saturating_add(1);
        if used == 0 || used > K {
            return Err(StopOnPhysicalRefusal::MissingContributions);
        }
        let mut staged = ledger.clone();
        staged
            .commit(context.identity, &scratch[..used])
            .map_err(StopOnPhysicalRefusal::Budget)?;
        if mode == PhysicalAdmissionMode::BoundedRequired
            && (staged.status != EvidenceStatus::Bounded
                || scratch[..used].iter().any(|c| {
                    c.magnitude.provenance.is_none() || c.amplification.provenance.is_none()
                }))
        {
            return Err(StopOnPhysicalRefusal::BoundedEvidenceRequired);
        }
        Ok(staged)
    };
    let staged = match prepare() {
        Ok(staged) => staged,
        Err(error) => {
            work.physical_refusals = work.physical_refusals.saturating_add(1);
            return Err(error);
        }
    };
    work.prepared_candidates = work.prepared_candidates.saturating_add(1);
    Ok(PhysicalCandidatePreparation::Prepared(
        PreparedPhysicalCandidate {
            ledger,
            staged,
            work,
            accepted_time,
            accepted_state,
            candidate_time,
            candidate_state,
        },
    ))
}
