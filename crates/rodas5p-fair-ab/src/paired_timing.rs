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
//!   case statistics, with a seeded case-clustered (two-stage) percentile
//!   bootstrap interval;
//! * the decision is three-valued: Promote iff the interval's lower bound is
//!   at least the required speedup, Block iff its upper bound is below it,
//!   otherwise Inconclusive;
//! * an A/A control (reference against reference, same session) makes the
//!   whole timing non-authoritative when its interval excludes `1.0` or its
//!   log half-width exceeds `ln(required) / 2`.
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
const PAIRED_TIMING_SCHEMA: &str = "vigilode-paired-timing-v1";
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
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform index in `0..bound` by multiply-shift (bias below 2^-32 for
    /// the small bounds used here).
    fn below(&mut self, bound: usize) -> usize {
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
}

impl PairedTimingCase {
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
        })
    }

    fn validate(&self, protocol: &PairedTimingProtocol) -> FairResult<()> {
        if self.warmup_seconds.len() < protocol.warmups || self.batch_iterations == 0 {
            return Err(FairError::Invalid(format!(
                "paired timing case {} needs at least {} warmups and a calibrated batch",
                self.case_id, protocol.warmups
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
    Ok(PairedTimingCase {
        case_id: case_id.into(),
        batch_iterations,
        warmup_seconds,
        order,
        candidate_seconds,
        reference_seconds,
    })
}

fn median_in_place(values: &mut [f64]) -> f64 {
    debug_assert!(!values.is_empty());
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        0.5 * (values[middle - 1] + values[middle])
    } else {
        values[middle]
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
/// case-clustered percentile bootstrap: each resample draws cases with
/// replacement and then pairs with replacement inside each drawn case.
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
        case.validate(protocol)?;
        if !ids.insert(case.case_id.as_str()) {
            return Err(FairError::Invalid(format!(
                "duplicate paired timing case {}",
                case.case_id
            )));
        }
    }
    let log_speedups = cases
        .iter()
        .map(PairedTimingCase::log_speedups)
        .collect::<Vec<_>>();
    let mut case_statistics = log_speedups
        .iter()
        .map(|values| median_in_place(&mut values.clone()))
        .collect::<Vec<_>>();
    let point_log = median_in_place(&mut case_statistics);

    let mut rng = SplitMix64(protocol.seed);
    let mut replicates = Vec::with_capacity(protocol.bootstrap_resamples);
    let mut resampled_cases = vec![0.0; cases.len()];
    let mut resampled_pairs = Vec::new();
    for _ in 0..protocol.bootstrap_resamples {
        for slot in &mut resampled_cases {
            let values = &log_speedups[rng.below(cases.len())];
            resampled_pairs.clear();
            resampled_pairs.extend((0..values.len()).map(|_| values[rng.below(values.len())]));
            *slot = median_in_place(&mut resampled_pairs);
        }
        replicates.push(median_in_place(&mut resampled_cases));
    }
    replicates.sort_by(f64::total_cmp);
    let tail = 0.5 * (1.0 - protocol.confidence_level);
    let lower_log = quantile_sorted(&replicates, tail);
    let upper_log = quantile_sorted(&replicates, 1.0 - tail);
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
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PairedTimingDecision {
    Promote,
    Block,
    Inconclusive,
}

pub fn paired_timing_decision(
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
        authoritative: contains_unity && half_width_log <= maximum_half_width_log,
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
    pub protocol: PairedTimingProtocol,
    pub host: TimingHostMetadata,
    pub cases: Vec<PairedCaseSummary>,
    pub corpus: SpeedupInterval,
    /// Three-valued decision from the candidate interval alone.
    pub decision: PairedTimingDecision,
    pub aa_control: Option<AaControlAssessment>,
    /// True only with an A/A control whose interval contains 1.0 and whose
    /// half-width resolves the required effect.
    pub timing_authoritative: bool,
    /// `decision` when authoritative, otherwise `Inconclusive`.
    pub gate_decision: PairedTimingDecision,
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
    let timing_authoritative = aa_control
        .as_ref()
        .is_some_and(|control| control.authoritative);
    let summaries = cases
        .iter()
        .map(|case| {
            let mut logs = case.log_speedups();
            let median_log_speedup = median_in_place(&mut logs);
            PairedCaseSummary {
                case_id: case.case_id.clone(),
                pairs: logs.len(),
                batch_iterations: case.batch_iterations,
                median_log_speedup,
                median_speedup: median_log_speedup.exp(),
                minimum_pair_speedup: logs[0].exp(),
                maximum_pair_speedup: logs[logs.len() - 1].exp(),
            }
        })
        .collect();
    Ok(PairedTimingAssessment {
        schema: PAIRED_TIMING_SCHEMA.into(),
        protocol: protocol.clone(),
        host,
        cases: summaries,
        corpus,
        decision,
        aa_control,
        timing_authoritative,
        gate_decision: if timing_authoritative {
            decision
        } else {
            PairedTimingDecision::Inconclusive
        },
    })
}
