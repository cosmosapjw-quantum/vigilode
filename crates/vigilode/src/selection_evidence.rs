//! Common comparison boundary for method and execution selection.
//!
//! Identities are caller-maintained versions of the *physical* comparison. A
//! smaller step, changed chart, tolerance, provider or output window invalidates
//! old evidence unless its declared validity region explicitly contains it.
//! Empirical timing envelopes remain empirical; this helper checks arithmetic
//! and compatibility, and does not turn three samples into future-cost bounds.
use serde::Serialize;

/// Exact comparison scope. Zero is a valid initial epoch, not a wildcard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SelectionIdentity {
    /// Original vector field and parameter version.
    pub model_epoch: u64,
    /// Callback implementation and derivative permission/provider version.
    pub provider_epoch: u64,
    /// Coordinate transformation and physical reconstruction version.
    pub chart_epoch: u64,
    /// Spatial mesh and state layout version.
    pub mesh_epoch: u64,
    /// Original-space error metric version.
    pub physical_norm_epoch: u64,
    /// Absolute/relative tolerance and error budget version.
    pub tolerance_epoch: u64,
    /// Same physical checkpoint, requested horizon, and output contract.
    pub window_epoch: u64,
    /// Hardware, concurrency and external-load policy version.
    pub environment_epoch: u64,
}

/// Units must match: a deterministic work score cannot be compared to seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CostUnit {
    /// A caller-defined common deterministic work score.
    Work,
    /// Elapsed seconds with the same accounting boundary.
    Seconds,
}

/// Provenance of the interval, kept separate from the arithmetic decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CostEvidenceKind {
    /// Actual work on the observed prefix; no future enclosure is implied.
    ObservedWork,
    /// An empirical interval made from observed elapsed times.
    EmpiricalTiming,
    /// A caller-provided bound whose mathematical validity is external.
    DeclaredBound,
}

/// Nonnegative finite cost endpoints in one declared unit.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct CostInterval {
    /// Lower endpoint; a future guarantee additionally needs a valid enclosure.
    pub lower: f64,
    /// Upper endpoint in the same unit and scope.
    pub upper: f64,
}
impl CostInterval {
    /// Check finite, ordered and nonnegative endpoints.
    pub fn valid(self) -> bool {
        self.lower.is_finite()
            && self.upper.is_finite()
            && self.lower >= 0.0
            && self.upper >= self.lower
    }
}

/// Accuracy/output eligibility must be established before comparing costs.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct CostEvidence {
    /// Physical comparison scope shared by both candidates.
    pub identity: SelectionIdentity,
    /// Observed or declared cost interval.
    pub cost: CostInterval,
    /// Common unit of both cost intervals and the transition charge.
    pub unit: CostUnit,
    /// Origin of the interval, without upgrading empirical evidence.
    pub kind: CostEvidenceKind,
    /// Same requested original-space accuracy/output contract was satisfied.
    /// For execution-only alternatives, this is the paired-output equivalence
    /// contract; mathematical accuracy remains the numerical solver's duty.
    pub admissible: bool,
}

/// Why two pieces of evidence cannot justify a cost comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CostIncomparability {
    /// Physical, callback, output or execution versions differ.
    IdentityMismatch,
    /// Work scores and elapsed seconds were mixed.
    UnitMismatch,
    /// At least one candidate lacks the requested output qualification.
    OutputNotQualified,
    /// A cost interval is negative, reversed or nonfinite.
    InvalidInterval,
    /// The horizon, gain or transition charge is invalid.
    InvalidPolicy,
}

/// Arithmetic conclusion for the declared intervals and scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CostComparison {
    /// The candidate upper cost clears the required conservative margin.
    PreferCandidate,
    /// The available intervals do not justify a change.
    RetainIncumbent,
    /// Comparing these inputs is outside the stated contract.
    Incomparable(CostIncomparability),
}

/// Admit a future change only if
/// `candidate.upper + transition / remaining <= (1 - gain) * incumbent.lower`.
///
/// `transition` is *future* transition cost. Already-spent calibration is not
/// charged a second time here. Endpoint arithmetic is conservatively rounded.
/// When intervals really enclose future costs for the declared identical scope,
/// this condition implies the stated gain. Observed-work and empirical-timing
/// intervals support only comparisons of the observations that produced them.
pub fn compare_qualified_costs(
    incumbent: &CostEvidence,
    candidate: &CostEvidence,
    future_transition_cost: f64,
    remaining_calls: usize,
    required_fractional_gain: f64,
) -> CostComparison {
    use CostComparison::*;
    use CostIncomparability::*;
    if incumbent.identity != candidate.identity {
        return Incomparable(IdentityMismatch);
    }
    if incumbent.unit != candidate.unit {
        return Incomparable(UnitMismatch);
    }
    if !incumbent.admissible || !candidate.admissible {
        return Incomparable(OutputNotQualified);
    }
    if !incumbent.cost.valid() || !candidate.cost.valid() {
        return Incomparable(InvalidInterval);
    }
    if !future_transition_cost.is_finite()
        || future_transition_cost < 0.0
        || remaining_calls == 0
        || !required_fractional_gain.is_finite()
        || !(0.0..1.0).contains(&required_fractional_gain)
    {
        return Incomparable(InvalidPolicy);
    }
    // Converting a huge usize can round upward. Use a lower denominator so
    // amortized future cost remains an upper bound, then round division upward.
    let n = (remaining_calls as f64).next_down().max(1.0);
    let per_call_transition = if future_transition_cost == 0.0 {
        0.0
    } else {
        (future_transition_cost / n).next_up()
    };
    let candidate_upper = if per_call_transition == 0.0 {
        candidate.cost.upper
    } else {
        (candidate.cost.upper + per_call_transition).next_up()
    };
    let retained_fraction = (1.0 - required_fractional_gain).next_down();
    let incumbent_threshold = (retained_fraction * incumbent.cost.lower).next_down();
    if candidate_upper.is_finite() && candidate_upper <= incumbent_threshold {
        PreferCandidate
    } else {
        RetainIncumbent
    }
}
