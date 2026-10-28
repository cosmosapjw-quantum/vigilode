//! Paired, dispersion-aware wall-time protocol (audit F-053).
//!
//! The legacy wall gates compare a median of medians against a fixed `1.15x`
//! with one to three samples, below the project's own recorded host noise
//! (v3.6 N=384 pair ratios 0.547-4.43).  This module replaces that rule with
//! a deterministic protocol and decision:
//!
//! * per case, at least two warmups, with the batch size calibrated from the
//!   *minimum* warmup sample, then at least thirty interleaved
//!   candidate/reference pairs in a seeded ABBA order;
//! * per case, the statistic is the median of per-pair
//!   `ln(reference / candidate)`; the corpus statistic is the median of the
//!   case statistics, with a seeded two-way percentile bootstrap: each
//!   resample draws measurement sessions (processes) with replacement, once
//!   for all cases, and draws cases with replacement; a drawn case takes the
//!   pairs its drawn sessions measured. Pairs measured in one process share
//!   its state (caches, frequency, allocator), within a case and across
//!   cases, so a session moves all its cases together and replicating cases
//!   in the same sessions cannot narrow the session part of the interval
//!   (external re-audit 6.1; audit 2026-09-30, B-01). Resampling cases as
//!   well keeps the interval conservative for a case population;
//! * process labels are global session identities shared across cases. A
//!   case without labels is resampled as one session of its own but counts
//!   as no session: a missing identity cannot be told apart from a shared
//!   process, so any unlabelled case makes the decision Inconclusive (re-audit
//!   R2, R2-STAT-01). A decision needs at least
//!   [`PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS`] distinct labelled sessions over
//!   all cases; with fewer, it is Inconclusive;
//! * the decision is three-valued: Promote iff the interval's lower bound is
//!   at least the required speedup, Block iff its upper bound is below it,
//!   otherwise Inconclusive;
//! * an A/A control (reference against reference, same sessions) makes the
//!   whole timing non-authoritative when its interval excludes `1.0`, its
//!   log half-width exceeds `ln(required) / 2`, or its labelled sessions are
//!   not exactly the candidate's (re-audit R2, R2-STAT-01);
//! * only the confirmatory protocol ([`PairedTimingProtocol::is_confirmatory`]:
//!   at least the default resamples, confidence and required speedup) may
//!   gate; a weaker protocol is a preview, recorded but never authoritative
//!   (re-audit R2, R2-STAT-03: one resample gave a zero-width interval that
//!   promoted), and a consumer re-derives the gate from the recorded fields
//!   with [`PairedTimingAssessment::verified_gate_decision`].
//!
//! Everything except [`detect_timing_host_metadata`] and the caller-supplied
//! clock of [`measure_paired_case`] is a pure function of its inputs and seed.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{FairError, FairResult};

pub const PAIRED_TIMING_MIN_WARMUPS: usize = 2;
pub const PAIRED_TIMING_MIN_PAIRS: usize = 30;
pub const PAIRED_TIMING_BOOTSTRAP_RESAMPLES: usize = 10_000;
pub const PAIRED_TIMING_CONFIDENCE_LEVEL: f64 = 0.95;
pub const PAIRED_TIMING_REQUIRED_SPEEDUP: f64 = 1.15;
/// Independent process blocks (over all cases) a decision needs.
pub const PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS: usize = 6;
/// v2: unlabelled cases count as no session, A/A sessions must match, and
/// only a confirmatory protocol gates (re-audit R2).
/// v3: the decision also needs the Monte-Carlo gate (re-audit R3,
/// STAT-DEV-03).
pub const PAIRED_TIMING_SCHEMA: &str = "vigilode-paired-timing-v3";
/// The estimand of [`assess_paired_timing`] (re-audit R4, R4-STAT-DEV-03):
/// per case, the median of the case's per-pair log speedups pooled over all
/// its sessions; over cases, the median of those. Not the median over cases
/// of the population median of session-cell medians
/// ([`SESSION_CELL_MEDIAN_ESTIMAND`]); the two coincide in the symmetric
/// additive family of `docs/TIMING_DESIGN_CONTRACT.md` and can differ
/// otherwise.
pub const POOLED_PAIR_MEDIAN_ESTIMAND: &str = "case-median-of-pooled-pair-log-speedup-median-v1";
/// The estimand of the exact session-median interval (re-audit R4,
/// R4-STAT-DEV-04): the median over a fixed case set of each case's
/// population median of complete session-cell medians.
pub const SESSION_CELL_MEDIAN_ESTIMAND: &str = "case-median-of-session-cell-median-log-speedup-v1";

fn pooled_pair_estimand() -> String {
    POOLED_PAIR_MEDIAN_ESTIMAND.into()
}
/// Predeclared two-sided failure probability of the Monte-Carlo gate.
pub const PAIRED_TIMING_MC_FAILURE_BUDGET: f64 = 0.01;
/// The number of resamples is the protocol's and is never extended after
/// seeing data; an extension would need a predeclared schedule with the
/// failure budget split over its stages.
pub const PAIRED_TIMING_MC_EXTENSION_POLICY: &str = "fixed-resamples-no-data-adaptive-extension";
const UNKNOWN: &str = "unknown";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedTimingProtocol {
    pub warmups: usize,
    pub pairs: usize,
    pub seed: u64,
    pub bootstrap_resamples: usize,
    pub confidence_level: f64,
    pub required_speedup: f64,
    pub minimum_sample_seconds: f64,
    pub maximum_batch_iterations: usize,
}

impl PairedTimingProtocol {
    /// The minimum protocol that may emit an authoritative wall decision.
    pub fn authoritative(seed: u64) -> Self {
        Self {
            warmups: PAIRED_TIMING_MIN_WARMUPS,
            pairs: PAIRED_TIMING_MIN_PAIRS,
            seed,
            bootstrap_resamples: PAIRED_TIMING_BOOTSTRAP_RESAMPLES,
            confidence_level: PAIRED_TIMING_CONFIDENCE_LEVEL,
            required_speedup: PAIRED_TIMING_REQUIRED_SPEEDUP,
            minimum_sample_seconds: 2.0e-3,
            maximum_batch_iterations: 10_000,
        }
    }

    /// True when the protocol is at least the confirmatory one: the
    /// minimum warmups and pairs (which [`Self::validate`] enforces), at
    /// least [`PAIRED_TIMING_BOOTSTRAP_RESAMPLES`] resamples, a confidence
    /// level of at least [`PAIRED_TIMING_CONFIDENCE_LEVEL`] and a required
    /// speedup of at least [`PAIRED_TIMING_REQUIRED_SPEEDUP`]. Anything
    /// weaker is a preview and never gates.
    pub fn is_confirmatory(&self) -> bool {
        self.validate().is_ok()
            && self.bootstrap_resamples >= PAIRED_TIMING_BOOTSTRAP_RESAMPLES
            && self.confidence_level >= PAIRED_TIMING_CONFIDENCE_LEVEL
            && self.required_speedup >= PAIRED_TIMING_REQUIRED_SPEEDUP
    }

    pub fn validate(&self) -> FairResult<()> {
        if self.warmups < PAIRED_TIMING_MIN_WARMUPS {
            return Err(FairError::Invalid(format!(
                "paired timing requires at least {PAIRED_TIMING_MIN_WARMUPS} warmups"
            )));
        }
        if self.pairs < PAIRED_TIMING_MIN_PAIRS {
            return Err(FairError::Invalid(format!(
                "paired timing requires at least {PAIRED_TIMING_MIN_PAIRS} pairs"
            )));
        }
        if self.bootstrap_resamples == 0 {
            return Err(FairError::Invalid(
                "paired timing bootstrap needs at least one resample".into(),
            ));
        }
        if !(self.confidence_level > 0.0 && self.confidence_level < 1.0) {
            return Err(FairError::Invalid(
                "paired timing confidence level must lie in (0,1)".into(),
            ));
        }
        if !(self.required_speedup > 1.0 && self.required_speedup.is_finite()) {
            return Err(FairError::Invalid(
                "paired timing required speedup must be finite and above 1".into(),
            ));
        }
        if !(self.minimum_sample_seconds > 0.0 && self.minimum_sample_seconds.is_finite())
            || self.maximum_batch_iterations == 0
        {
            return Err(FairError::Invalid(
                "paired timing batch calibration bounds must be positive".into(),
            ));
        }
        Ok(())
    }
}

/// SplitMix64 (Steele, Lea & Flood 2014): a small, self-contained, seeded
/// generator so the order and bootstrap bits are reproducible without
/// depending on an external crate's stream stability.
#[derive(Clone, Debug)]
pub(crate) struct SplitMix64(pub(crate) u64);

impl SplitMix64 {
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform index in `0..bound` by multiply-shift (bias below 2^-32 for
    /// the small bounds used here).
    pub(crate) fn below(&mut self, bound: usize) -> usize {
        ((u128::from(self.next_u64()) * bound as u128) >> 64) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PairedArm {
    Candidate,
    Reference,
}

/// Seeded ABBA order.  Pairs are grouped in blocks of two whose arm orders
/// mirror each other, so each arm runs first equally often and linear drift
/// cancels within a block; each block's leading arm is a seeded coin flip.
pub fn abba_pair_order(pairs: usize, seed: u64) -> Vec<[PairedArm; 2]> {
    let mut rng = SplitMix64(seed);
    let mut order = Vec::with_capacity(pairs);
    while order.len() < pairs {
        let first = if rng.next_u64() >> 63 == 0 {
            [PairedArm::Candidate, PairedArm::Reference]
        } else {
            [PairedArm::Reference, PairedArm::Candidate]
        };
        order.push(first);
        if order.len() < pairs {
            order.push([first[1], first[0]]);
        }
    }
    order
}

/// Lengths of the contiguous runs of equal session labels.
fn session_run_lengths(process_blocks: &[u32]) -> Vec<usize> {
    let mut lengths = Vec::new();
    let mut start = 0;
    while start < process_blocks.len() {
        let label = process_blocks[start];
        let end = process_blocks[start..]
            .iter()
            .position(|block| *block != label)
            .map_or(process_blocks.len(), |offset| start + offset);
        lengths.push(end - start);
        start = end;
    }
    lengths
}

/// The seeded ABBA order restarted in each contiguous run of equal session
/// labels: the order of a case whose sessions each measured
/// `protocol.pairs` pairs in their own process and were then concatenated
/// ([`merge_session_cases`]). Empty labels give an empty order.
pub fn session_abba_order(process_blocks: &[u32], seed: u64) -> Vec<[PairedArm; 2]> {
    let mut order = Vec::with_capacity(process_blocks.len());
    let mut start = 0;
    while start < process_blocks.len() {
        let label = process_blocks[start];
        let end = process_blocks[start..]
            .iter()
            .position(|block| *block != label)
            .map_or(process_blocks.len(), |offset| start + offset);
        order.extend(abba_pair_order(end - start, seed));
        start = end;
    }
    order
}

/// Batch iterations such that the fastest warmup sample would reach the
/// minimum sample time.  The minimum, not the last, sample is used so a slow
/// cold warmup cannot shrink the batch below the timer-resolution floor.
pub fn calibrate_batch_iterations(
    warmup_seconds: &[f64],
    protocol: &PairedTimingProtocol,
) -> FairResult<usize> {
    if warmup_seconds.len() < protocol.warmups
        || !warmup_seconds
            .iter()
            .all(|seconds| seconds.is_finite() && *seconds >= 0.0)
    {
        return Err(FairError::Invalid(format!(
            "batch calibration needs at least {} finite nonnegative warmup samples",
            protocol.warmups
        )));
    }
    let fastest = warmup_seconds
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .max(f64::MIN_POSITIVE);
    let iterations = (protocol.minimum_sample_seconds / fastest).ceil().max(1.0);
    Ok((iterations.min(protocol.maximum_batch_iterations as f64)) as usize)
}

/// Per-iteration wall samples of one case, pair `k` being
/// `(candidate_seconds[k], reference_seconds[k])`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedTimingCase {
    pub case_id: String,
    pub batch_iterations: usize,
    pub warmup_seconds: Vec<f64>,
    pub order: Vec<[PairedArm; 2]>,
    pub candidate_seconds: Vec<f64>,
    pub reference_seconds: Vec<f64>,
    /// Global session (process) identity of each pair, shared by every case
    /// measured in that process; empty means the whole case ran in one
    /// session of its own. A session is resampled as one unit across all
    /// cases (audit 2026-09-30, B-01).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub process_blocks: Vec<u32>,
}

/// Identity of a measurement session: a declared global process label, or
/// the implicit single session of a case without labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SessionKey {
    Global(u32),
    OwnCase(usize),
}

impl PairedTimingCase {
    /// Attach the global session (process) of each pair.
    pub fn with_process_blocks(mut self, process_blocks: Vec<u32>) -> Self {
        self.process_blocks = process_blocks;
        self
    }

    /// Log speedups grouped by session, keyed by the session identity.
    fn session_log_speedups(&self, case_index: usize) -> Vec<(SessionKey, Vec<f64>)> {
        let logs = self.log_speedups();
        if self.process_blocks.is_empty() {
            return vec![(SessionKey::OwnCase(case_index), logs)];
        }
        let mut blocks = std::collections::BTreeMap::<u32, Vec<f64>>::new();
        for (block, log) in self.process_blocks.iter().zip(logs) {
            blocks.entry(*block).or_default().push(log);
        }
        blocks
            .into_iter()
            .map(|(block, logs)| (SessionKey::Global(block), logs))
            .collect()
    }

    /// Case from already measured per-iteration samples (replayed or
    /// synthetic evidence).  The warmups must satisfy the protocol and fix the
    /// batch size exactly as [`measure_paired_case`] would; the order is the
    /// protocol's ABBA order.
    pub fn from_samples(
        case_id: impl Into<String>,
        protocol: &PairedTimingProtocol,
        warmup_seconds: Vec<f64>,
        candidate_seconds: Vec<f64>,
        reference_seconds: Vec<f64>,
    ) -> FairResult<Self> {
        let batch_iterations = calibrate_batch_iterations(&warmup_seconds, protocol)?;
        let pairs = candidate_seconds.len();
        Ok(Self {
            case_id: case_id.into(),
            batch_iterations,
            warmup_seconds,
            order: abba_pair_order(pairs, protocol.seed),
            candidate_seconds,
            reference_seconds,
            process_blocks: Vec::new(),
        })
    }

    /// Admission of a raw case against the protocol it claims (re-audit R3,
    /// R3-STAT-01): the producer, the assessment and the raw-receipt check
    /// all call this. Besides complete positive samples it requires finite
    /// nonnegative warmups, the batch size the protocol derives from them,
    /// and the protocol's seeded ABBA order for exactly the recorded pairs.
    /// An empty or wrong order, a changed batch, or a negative or NaN warmup
    /// used to pass and promote.
    pub fn admit(&self, protocol: &PairedTimingProtocol) -> FairResult<()> {
        self.validate(protocol)
            .map_err(|error| FairError::Invalid(format!("INVALID_RAW_TIMING_PROTOCOL: {error}")))
    }

    fn validate(&self, protocol: &PairedTimingProtocol) -> FairResult<()> {
        if self.warmup_seconds.len() < protocol.warmups || self.batch_iterations == 0 {
            return Err(FairError::Invalid(format!(
                "paired timing case {} needs at least {} warmups and a calibrated batch",
                self.case_id, protocol.warmups
            )));
        }
        let calibrated = calibrate_batch_iterations(&self.warmup_seconds, protocol)
            .map_err(|error| FairError::Invalid(format!("case {}: {error}", self.case_id)))?;
        if calibrated != self.batch_iterations {
            return Err(FairError::Invalid(format!(
                "paired timing case {} records batch {} but its warmups calibrate to {calibrated}",
                self.case_id, self.batch_iterations
            )));
        }
        // The whole-case ABBA order, or the per-session order of a merged
        // case in which every session measured exactly `protocol.pairs`
        // pairs (singleton or odd session runs would let every pair restart
        // the same seed; re-audit R3 review).
        let session_runs_complete = !self.process_blocks.is_empty()
            && session_run_lengths(&self.process_blocks)
                .iter()
                .all(|length| *length == protocol.pairs);
        if self.order != abba_pair_order(self.candidate_seconds.len(), protocol.seed)
            && !(session_runs_complete
                && self.order == session_abba_order(&self.process_blocks, protocol.seed))
        {
            return Err(FairError::Invalid(format!(
                "paired timing case {} does not follow the protocol's seeded ABBA order",
                self.case_id
            )));
        }
        if self.candidate_seconds.len() != self.reference_seconds.len()
            || self.candidate_seconds.len() < protocol.pairs
        {
            return Err(FairError::Invalid(format!(
                "paired timing case {} needs at least {} complete pairs",
                self.case_id, protocol.pairs
            )));
        }
        if !self.process_blocks.is_empty()
            && self.process_blocks.len() != self.candidate_seconds.len()
        {
            return Err(FairError::Invalid(format!(
                "paired timing case {} needs one process block per pair",
                self.case_id
            )));
        }
        if !self
            .candidate_seconds
            .iter()
            .chain(&self.reference_seconds)
            .all(|seconds| seconds.is_finite() && *seconds > 0.0)
        {
            return Err(FairError::Invalid(format!(
                "paired timing case {} has a nonpositive or nonfinite sample",
                self.case_id
            )));
        }
        Ok(())
    }

    /// Per-pair `ln(reference / candidate)`; positive favours the candidate.
    pub fn log_speedups(&self) -> Vec<f64> {
        self.candidate_seconds
            .iter()
            .zip(&self.reference_seconds)
            .map(|(candidate, reference)| (reference / candidate).ln())
            .collect()
    }
}

/// [`measure_paired_case`] in a declared global session: every pair is
/// labelled `session`. A case measured without a session is never
/// authoritative (re-audit R2, R2-STAT-01).
pub fn measure_paired_case_in_session<C, R, K>(
    case_id: impl Into<String>,
    session: u32,
    protocol: &PairedTimingProtocol,
    candidate: C,
    reference: R,
    clock: K,
) -> FairResult<PairedTimingCase>
where
    C: FnMut() -> FairResult<()>,
    R: FnMut() -> FairResult<()>,
    K: FnMut() -> f64,
{
    let case = measure_paired_case(case_id, protocol, candidate, reference, clock)?;
    let pairs = case.candidate_seconds.len();
    Ok(case.with_process_blocks(vec![session; pairs]))
}

/// Run one case: `protocol.warmups` alternating warmup rounds of both arms,
/// batch calibration from the minimum warmup sample, then `protocol.pairs`
/// batched pairs in seeded ABBA order.  `clock` returns monotonic seconds;
/// production callers pass an `Instant`-based clock, tests a synthetic one.
pub fn measure_paired_case<C, R, K>(
    case_id: impl Into<String>,
    protocol: &PairedTimingProtocol,
    mut candidate: C,
    mut reference: R,
    mut clock: K,
) -> FairResult<PairedTimingCase>
where
    C: FnMut() -> FairResult<()>,
    R: FnMut() -> FairResult<()>,
    K: FnMut() -> f64,
{
    protocol.validate()?;
    let mut run = |arm: PairedArm, iterations: usize| -> FairResult<f64> {
        let started = clock();
        for _ in 0..iterations {
            match arm {
                PairedArm::Candidate => candidate()?,
                PairedArm::Reference => reference()?,
            }
        }
        Ok(((clock() - started) / iterations as f64).max(0.0))
    };
    let mut warmup_seconds = Vec::with_capacity(2 * protocol.warmups);
    for round in 0..protocol.warmups {
        let arms = if round % 2 == 0 {
            [PairedArm::Candidate, PairedArm::Reference]
        } else {
            [PairedArm::Reference, PairedArm::Candidate]
        };
        for arm in arms {
            warmup_seconds.push(run(arm, 1)?);
        }
    }
    let batch_iterations = calibrate_batch_iterations(&warmup_seconds, protocol)?;
    let order = abba_pair_order(protocol.pairs, protocol.seed);
    let mut candidate_seconds = Vec::with_capacity(protocol.pairs);
    let mut reference_seconds = Vec::with_capacity(protocol.pairs);
    for arms in &order {
        for &arm in arms {
            let seconds = run(arm, batch_iterations)?;
            match arm {
                PairedArm::Candidate => candidate_seconds.push(seconds),
                PairedArm::Reference => reference_seconds.push(seconds),
            }
        }
    }
    let case = PairedTimingCase {
        case_id: case_id.into(),
        batch_iterations,
        warmup_seconds,
        order,
        candidate_seconds,
        reference_seconds,
        process_blocks: Vec::new(),
    };
    // The producer passes the same admission as a replayed receipt.
    case.admit(protocol)?;
    Ok(case)
}

/// The median by selection: the same order statistics, and so the same bits,
/// as sorting, in linear time.
pub(crate) fn median_in_place(values: &mut [f64]) -> f64 {
    debug_assert!(!values.is_empty());
    let even = values.len().is_multiple_of(2);
    let middle = values.len() / 2;
    let (left, upper, _) = values.select_nth_unstable_by(middle, f64::total_cmp);
    if even {
        let lower = left
            .iter()
            .copied()
            .max_by(f64::total_cmp)
            .expect("an even nonempty slice has a lower half");
        0.5 * (lower + *upper)
    } else {
        *upper
    }
}

fn quantile_sorted(values: &[f64], probability: f64) -> f64 {
    let position = probability * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    values[lower] * (1.0 - fraction) + values[upper] * fraction
}

/// Speedup interval (`reference / candidate`), stored in both log and ratio
/// form so identical inputs reproduce identical bits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeedupInterval {
    pub point_log: f64,
    pub lower_log: f64,
    pub upper_log: f64,
    pub point: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
    pub resamples: usize,
    pub seed: u64,
    /// Distinct labelled sessions over all cases (re-audit R2: unlabelled
    /// cases are not counted).
    #[serde(default)]
    pub independent_blocks: usize,
    /// The labelled sessions, sorted.
    #[serde(default)]
    pub sessions: Vec<u32>,
    /// Cases without session labels; any makes the interval non-deciding.
    #[serde(default)]
    pub unlabeled_cases: usize,
    /// Simulation uncertainty of the percentile endpoints: the replicate
    /// order statistics two binomial standard deviations either side of each
    /// endpoint's position, in log form. The endpoints themselves are those
    /// of the finite-resample bootstrap distribution, not of the exact one.
    #[serde(default)]
    pub lower_log_simulation_band: [f64; 2],
    #[serde(default)]
    pub upper_log_simulation_band: [f64; 2],
    /// Replicates strictly below `ln(required_speedup)` (compared in log
    /// space), for the Monte-Carlo gate.
    #[serde(default)]
    pub replicates_below_required: usize,
}

impl SpeedupInterval {
    pub fn contains(&self, speedup: f64) -> bool {
        self.lower <= speedup && speedup <= self.upper
    }

    pub fn half_width_log(&self) -> f64 {
        0.5 * (self.upper_log - self.lower_log)
    }
}

/// Median over cases of the per-case median log speedup, with a seeded
/// two-way percentile bootstrap: each resample draws sessions with
/// replacement (shared by all cases, so a session moves every case it
/// measured together; audit 2026-09-30, B-01) and cases with replacement,
/// and a drawn case takes the pairs of its drawn sessions, with
/// multiplicity. A case with no pairs in a replicate's sessions is left out
/// of that replicate's median.
pub fn case_clustered_bootstrap(
    cases: &[PairedTimingCase],
    protocol: &PairedTimingProtocol,
) -> FairResult<SpeedupInterval> {
    protocol.validate()?;
    if cases.is_empty() {
        return Err(FairError::Invalid(
            "paired timing needs at least one case".into(),
        ));
    }
    let mut ids = BTreeSet::new();
    for case in cases {
        case.admit(protocol)?;
        if !ids.insert(case.case_id.as_str()) {
            return Err(FairError::Invalid(format!(
                "duplicate paired timing case {}",
                case.case_id
            )));
        }
    }
    let per_case = cases
        .iter()
        .enumerate()
        .map(|(index, case)| case.session_log_speedups(index))
        .collect::<Vec<_>>();
    let sessions = per_case
        .iter()
        .flat_map(|case| case.iter().map(|(key, _)| *key))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let labelled = sessions
        .iter()
        .filter_map(|key| match key {
            SessionKey::Global(label) => Some(*label),
            SessionKey::OwnCase(_) => None,
        })
        .collect::<Vec<_>>();
    let unlabeled_cases = cases
        .iter()
        .filter(|case| case.process_blocks.is_empty())
        .count();
    // For each case, its pairs by session index.
    let by_session = per_case
        .iter()
        .map(|case| {
            case.iter()
                .map(|(key, logs)| {
                    let index = sessions.binary_search(key).expect("session collected");
                    (index, logs.clone())
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut case_statistics = per_case
        .iter()
        .map(|case| {
            let mut all = case
                .iter()
                .flat_map(|(_, logs)| logs.iter().copied())
                .collect::<Vec<_>>();
            median_in_place(&mut all)
        })
        .collect::<Vec<_>>();
    let point_log = median_in_place(&mut case_statistics);

    let mut rng = SplitMix64(protocol.seed);
    let mut replicates = Vec::with_capacity(protocol.bootstrap_resamples);
    let mut multiplicity = vec![0usize; sessions.len()];
    let mut resampled_cases = Vec::with_capacity(cases.len());
    let mut resampled_pairs = Vec::new();
    while replicates.len() < protocol.bootstrap_resamples {
        multiplicity.fill(0);
        for _ in 0..sessions.len() {
            multiplicity[rng.below(sessions.len())] += 1;
        }
        resampled_cases.clear();
        for _ in 0..by_session.len() {
            let case = &by_session[rng.below(by_session.len())];
            resampled_pairs.clear();
            for (session, logs) in case {
                for _ in 0..multiplicity[*session] {
                    resampled_pairs.extend_from_slice(logs);
                }
            }
            if !resampled_pairs.is_empty() {
                resampled_cases.push(median_in_place(&mut resampled_pairs));
            }
        }
        if !resampled_cases.is_empty() {
            replicates.push(median_in_place(&mut resampled_cases));
        }
    }
    replicates.sort_by(f64::total_cmp);
    let threshold_log = protocol.required_speedup.ln();
    let replicates_below_required = replicates
        .iter()
        .filter(|value| **value < threshold_log)
        .count();
    let tail = 0.5 * (1.0 - protocol.confidence_level);
    let lower_log = quantile_sorted(&replicates, tail);
    let upper_log = quantile_sorted(&replicates, 1.0 - tail);
    let band = |probability: f64| {
        let resamples = replicates.len() as f64;
        let spread = 2.0 * (resamples * probability * (1.0 - probability)).sqrt() / resamples;
        [
            quantile_sorted(&replicates, (probability - spread).max(0.0)),
            quantile_sorted(&replicates, (probability + spread).min(1.0)),
        ]
    };
    Ok(SpeedupInterval {
        point_log,
        lower_log,
        upper_log,
        point: point_log.exp(),
        lower: lower_log.exp(),
        upper: upper_log.exp(),
        confidence_level: protocol.confidence_level,
        resamples: protocol.bootstrap_resamples,
        seed: protocol.seed,
        independent_blocks: labelled.len(),
        sessions: labelled,
        unlabeled_cases,
        lower_log_simulation_band: band(tail),
        upper_log_simulation_band: band(1.0 - tail),
        replicates_below_required,
    })
}

/// The Monte-Carlo error gate on a finite-resample decision (re-audit R3,
/// STAT-DEV-03).
///
/// With `K` of `B` replicates strictly below the threshold, the conditional
/// probability `p` that a replicate falls below it lies in
/// `[K/B - e, K/B + e]`, `e = sqrt(ln(2/delta) / (2B))` (Hoeffding), except
/// with probability `delta` over the resampling. Promote needs `upper <
/// tail`, Block needs `lower > 1 - tail`; anything else is Inconclusive
/// (MC_UNRESOLVED). It says nothing about coverage of the population
/// speedup. Hoeffding ignores the variance of the indicator, so the gate is
/// conservative beyond the resampling error: at `B = 10000` a Promote needs
/// `K / B < 0.0087` against the 2.5% tail, and some decisions a
/// variance-aware (Clopper-Pearson or Bernstein) bound would resolve are
/// withheld. It can only withhold a decision, never create one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonteCarloGate {
    pub resamples: usize,
    pub count_below: usize,
    pub delta: f64,
    pub tail: f64,
    pub lower: f64,
    pub upper: f64,
    pub method: String,
    pub decision: PairedTimingDecision,
}

pub fn monte_carlo_gate(
    count_below: usize,
    resamples: usize,
    confidence_level: f64,
    delta: f64,
) -> MonteCarloGate {
    let tail = 0.5 * (1.0 - confidence_level);
    let b = resamples.max(1) as f64;
    let epsilon = ((2.0 / delta).ln() / (2.0 * b)).sqrt();
    let fraction = count_below as f64 / b;
    let lower = (fraction - epsilon).max(0.0);
    let upper = (fraction + epsilon).min(1.0);
    let decision = if resamples == 0 {
        PairedTimingDecision::Inconclusive
    } else if upper < tail {
        PairedTimingDecision::Promote
    } else if lower > 1.0 - tail {
        PairedTimingDecision::Block
    } else {
        PairedTimingDecision::Inconclusive
    };
    MonteCarloGate {
        resamples,
        count_below,
        delta,
        tail,
        lower,
        upper,
        method: "hoeffding".into(),
        decision,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PairedTimingDecision {
    Promote,
    Block,
    Inconclusive,
}

/// The percentile-endpoint rule alone, without the session count or the
/// Monte-Carlo gate: the pre-R3 decision, kept as a diagnostic.
pub fn percentile_endpoint_decision(
    interval: &SpeedupInterval,
    required_speedup: f64,
) -> PairedTimingDecision {
    if interval.lower >= required_speedup {
        PairedTimingDecision::Promote
    } else if interval.upper < required_speedup {
        PairedTimingDecision::Block
    } else {
        PairedTimingDecision::Inconclusive
    }
}

pub fn paired_timing_decision(
    interval: &SpeedupInterval,
    required_speedup: f64,
) -> PairedTimingDecision {
    if interval.independent_blocks < PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS
        || interval.unlabeled_cases > 0
    {
        PairedTimingDecision::Inconclusive
    } else {
        let endpoint = percentile_endpoint_decision(interval, required_speedup);
        // The percentile endpoints and the Monte-Carlo gate must agree: an
        // endpoint decided by simulation noise is not decided (re-audit R3,
        // STAT-DEV-03; the old endpoint rule is kept as the diagnostic).
        let gate = monte_carlo_gate(
            interval.replicates_below_required,
            interval.resamples,
            interval.confidence_level,
            PAIRED_TIMING_MC_FAILURE_BUDGET,
        );
        if gate.decision == endpoint {
            endpoint
        } else {
            PairedTimingDecision::Inconclusive
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AaControlAssessment {
    pub interval: SpeedupInterval,
    pub contains_unity: bool,
    pub half_width_log: f64,
    pub maximum_half_width_log: f64,
    /// False when the A/A interval excludes 1.0 or is wider than
    /// `ln(required) / 2`: the session cannot resolve the required effect.
    pub authoritative: bool,
}

pub fn assess_aa_control(
    cases: &[PairedTimingCase],
    protocol: &PairedTimingProtocol,
) -> FairResult<AaControlAssessment> {
    let interval = case_clustered_bootstrap(cases, protocol)?;
    let contains_unity = interval.contains(1.0);
    let half_width_log = interval.half_width_log();
    let maximum_half_width_log = 0.5 * protocol.required_speedup.ln();
    Ok(AaControlAssessment {
        authoritative: contains_unity
            && half_width_log <= maximum_half_width_log
            && interval.independent_blocks >= PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS
            && interval.unlabeled_cases == 0,
        interval,
        contains_unity,
        half_width_log,
        maximum_half_width_log,
    })
}

/// Host facts recorded beside every paired timing; `unknown` when a source
/// is unreadable (non-Linux hosts, restricted containers).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimingHostMetadata {
    pub cpu_model: String,
    pub scaling_governor: String,
    pub load_average: String,
    pub thread_count: usize,
    pub cpus_allowed: String,
    /// `single-cpu` when the affinity mask admits exactly one CPU,
    /// `multi-cpu` otherwise, `unknown` when the mask is unreadable.
    pub pinning: String,
}

fn read_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn proc_field(path: &str, key: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()?
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name.trim() == key).then(|| value.trim().to_string())
        })
}

fn affinity_pinning(cpus_allowed: &str) -> &'static str {
    let mut count = 0_usize;
    for range in cpus_allowed.split(',') {
        let bounds = range
            .split_once('-')
            .map_or((range, range), |(low, high)| (low, high));
        match (
            bounds.0.trim().parse::<usize>(),
            bounds.1.trim().parse::<usize>(),
        ) {
            (Ok(low), Ok(high)) if high >= low => count += high - low + 1,
            _ => return UNKNOWN,
        }
    }
    if count == 1 {
        "single-cpu"
    } else {
        "multi-cpu"
    }
}

/// Read host metadata from `/proc` and `/sys` where available.
pub fn detect_timing_host_metadata(thread_count: usize) -> TimingHostMetadata {
    let cpus_allowed = proc_field("/proc/self/status", "Cpus_allowed_list");
    TimingHostMetadata {
        cpu_model: proc_field("/proc/cpuinfo", "model name").unwrap_or_else(|| UNKNOWN.into()),
        scaling_governor: read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
            .unwrap_or_else(|| UNKNOWN.into()),
        load_average: read_trimmed("/proc/loadavg")
            .map(|text| {
                text.split_whitespace()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| UNKNOWN.into()),
        thread_count,
        pinning: cpus_allowed
            .as_deref()
            .map_or(UNKNOWN, affinity_pinning)
            .into(),
        cpus_allowed: cpus_allowed.unwrap_or_else(|| UNKNOWN.into()),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedCaseSummary {
    pub case_id: String,
    pub pairs: usize,
    pub batch_iterations: usize,
    pub median_log_speedup: f64,
    pub median_speedup: f64,
    pub minimum_pair_speedup: f64,
    pub maximum_pair_speedup: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PairedTimingAssessment {
    pub schema: String,
    /// [`POOLED_PAIR_MEDIAN_ESTIMAND`]; records written before R4 name no
    /// estimand and deserialize to it, the estimand they were computed for.
    #[serde(default = "pooled_pair_estimand")]
    pub estimand: String,
    pub protocol: PairedTimingProtocol,
    pub host: TimingHostMetadata,
    pub cases: Vec<PairedCaseSummary>,
    pub corpus: SpeedupInterval,
    /// Three-valued decision from the candidate interval alone.
    pub decision: PairedTimingDecision,
    pub aa_control: Option<AaControlAssessment>,
    /// True only with an A/A control whose interval contains 1.0, whose
    /// half-width resolves the required effect and whose labelled sessions
    /// are exactly the candidate's.
    pub timing_authoritative: bool,
    /// The A/A control measured in exactly the candidate's sessions.
    #[serde(default)]
    pub aa_sessions_match: bool,
    /// The protocol is confirmatory ([`PairedTimingProtocol::is_confirmatory`]).
    #[serde(default)]
    pub confirmatory: bool,
    /// `decision` when authoritative and confirmatory, otherwise
    /// `Inconclusive`.
    pub gate_decision: PairedTimingDecision,
    /// SHA-256 of the JSON of the raw candidate cases and A/A cases (with
    /// their session labels) this record was computed from, so a record can
    /// be tied to its raw receipt ([`Self::verify_against_raw`]).
    #[serde(default)]
    pub raw_cases_sha256: String,
}

fn raw_cases_digest(
    cases: &[PairedTimingCase],
    aa_cases: Option<&[PairedTimingCase]>,
) -> FairResult<String> {
    let json = serde_json::to_vec(&(cases, aa_cases))
        .map_err(|error| FairError::Invalid(format!("paired timing raw cases: {error}")))?;
    Ok(rodas5p_core::sha256_hex(&json))
}

impl PairedTimingAssessment {
    /// Recompute the whole record from the raw cases it names and require
    /// bit equality. Unlike [`Self::verified_gate_decision`], this also
    /// catches edited interval values, which are self-consistent by
    /// construction once decision and bounds are edited together.
    pub fn verify_against_raw(
        &self,
        cases: &[PairedTimingCase],
        aa_cases: Option<&[PairedTimingCase]>,
    ) -> FairResult<PairedTimingDecision> {
        if raw_cases_digest(cases, aa_cases)? != self.raw_cases_sha256 {
            return Err(FairError::Invalid(
                "paired timing record: raw cases do not match the recorded digest".into(),
            ));
        }
        let recomputed = assess_paired_timing(&self.protocol, cases, aa_cases, self.host.clone())?;
        if &recomputed != self {
            return Err(FairError::Invalid(
                "paired timing record: fields differ from the recomputation".into(),
            ));
        }
        self.verified_gate_decision()
    }

    /// The gate decision re-derived from the recorded fields, for a
    /// consumer that did not run the assessment. It checks internal
    /// consistency only: interval values edited together with the decision
    /// pass it; [`Self::verify_against_raw`] catches those. An assessment with another
    /// schema, a preview protocol, an interval computed under other
    /// settings, or a recorded decision or authority flag that its own
    /// fields do not imply is rejected (re-audit R2, R2-STAT-03).
    pub fn verified_gate_decision(&self) -> FairResult<PairedTimingDecision> {
        let reject = |reason: &str| {
            Err(FairError::Invalid(format!(
                "paired timing record: {reason}"
            )))
        };
        if self.schema != PAIRED_TIMING_SCHEMA {
            return reject("schema is not the current one");
        }
        if !self.protocol.is_confirmatory() || !self.confirmatory {
            return reject("protocol is not confirmatory");
        }
        let settings = |interval: &SpeedupInterval| {
            interval.resamples == self.protocol.bootstrap_resamples
                && interval.confidence_level == self.protocol.confidence_level
                && interval.seed == self.protocol.seed
                && interval.independent_blocks == interval.sessions.len()
                && interval.sessions.windows(2).all(|pair| pair[0] < pair[1])
                && interval.lower <= interval.upper
        };
        if !settings(&self.corpus) || self.corpus.replicates_below_required > self.corpus.resamples
        {
            return reject("corpus interval does not match the protocol");
        }
        let decision = paired_timing_decision(&self.corpus, self.protocol.required_speedup);
        if decision != self.decision {
            return reject("decision does not follow from the corpus interval");
        }
        let Some(control) = &self.aa_control else {
            if self.timing_authoritative
                || self.aa_sessions_match
                || self.gate_decision != PairedTimingDecision::Inconclusive
            {
                return reject("authority recorded without an A/A control");
            }
            return Ok(PairedTimingDecision::Inconclusive);
        };
        if !settings(&control.interval) {
            return reject("A/A interval does not match the protocol");
        }
        let maximum = 0.5 * self.protocol.required_speedup.ln();
        let control_authoritative = control.interval.contains(1.0)
            && control.interval.half_width_log() <= maximum
            && control.interval.independent_blocks >= PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS
            && control.interval.unlabeled_cases == 0;
        if control.authoritative != control_authoritative {
            return reject("A/A authority does not follow from its interval");
        }
        let sessions_match = self.corpus.sessions == control.interval.sessions;
        if sessions_match != self.aa_sessions_match {
            return reject("A/A session match flag does not follow from the sessions");
        }
        let authoritative = control_authoritative && sessions_match;
        if authoritative != self.timing_authoritative {
            return reject("timing authority does not follow from the A/A control");
        }
        let gate = if authoritative {
            decision
        } else {
            PairedTimingDecision::Inconclusive
        };
        if gate != self.gate_decision {
            return reject("gate decision does not follow from its fields");
        }
        Ok(gate)
    }
}

/// Full paired assessment.  Without an A/A control the timing is recorded
/// but never authoritative.
pub fn assess_paired_timing(
    protocol: &PairedTimingProtocol,
    cases: &[PairedTimingCase],
    aa_cases: Option<&[PairedTimingCase]>,
    host: TimingHostMetadata,
) -> FairResult<PairedTimingAssessment> {
    let corpus = case_clustered_bootstrap(cases, protocol)?;
    let decision = paired_timing_decision(&corpus, protocol.required_speedup);
    let aa_control = aa_cases
        .map(|aa_cases| assess_aa_control(aa_cases, protocol))
        .transpose()?;
    let aa_sessions_match = aa_control
        .as_ref()
        .is_some_and(|control| control.interval.sessions == corpus.sessions);
    let timing_authoritative = aa_sessions_match
        && aa_control
            .as_ref()
            .is_some_and(|control| control.authoritative);
    let confirmatory = protocol.is_confirmatory();
    let summaries = cases
        .iter()
        .map(|case| {
            let mut logs = case.log_speedups();
            logs.sort_by(f64::total_cmp);
            let (minimum, maximum) = (logs[0], logs[logs.len() - 1]);
            let median_log_speedup = median_in_place(&mut logs);
            PairedCaseSummary {
                case_id: case.case_id.clone(),
                pairs: logs.len(),
                batch_iterations: case.batch_iterations,
                median_log_speedup,
                median_speedup: median_log_speedup.exp(),
                minimum_pair_speedup: minimum.exp(),
                maximum_pair_speedup: maximum.exp(),
            }
        })
        .collect();
    Ok(PairedTimingAssessment {
        schema: PAIRED_TIMING_SCHEMA.into(),
        estimand: POOLED_PAIR_MEDIAN_ESTIMAND.into(),
        protocol: protocol.clone(),
        host,
        cases: summaries,
        corpus,
        decision,
        aa_control,
        timing_authoritative,
        aa_sessions_match,
        confirmatory,
        raw_cases_sha256: raw_cases_digest(cases, aa_cases)?,
        gate_decision: if timing_authoritative && confirmatory {
            decision
        } else {
            PairedTimingDecision::Inconclusive
        },
    })
}
