use std::{collections::BTreeMap, time::Instant};

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use rodas5p_core::{CoreError, LinearSolverConfig, WorkCounters, safe_l2, sha256_hex};
use rodas5p_integrators::{
    BdfConfig, BdfOrder, IntegrationMethod, NewtonConfig, OdeProblem, OutputSchedule,
    ParallelExecution, RadauConfig, RadauIiaStages, integrate_bdf_fixed_observed,
    integrate_fixed_observed, integrate_radau_fixed_observed, manufactured_mass_nonlinear_problem,
    manufactured_vector_problem, prothero_robinson_problem, scalar_linear_problem,
};
use serde::{Deserialize, Serialize};

use crate::{FairError, FairResult};

const TIMING_SEED: u64 = 20_260_808;
const SMOKE_WARMUPS: usize = 1;
const SMOKE_REPETITIONS: usize = 3;
const CANONICAL_WARMUPS: usize = 1;
const CANONICAL_REPETITIONS: usize = 5;
const MIN_TIMING_SAMPLE_SECONDS: f64 = 2.0e-3;
const MAX_TIMING_BATCH_ITERATIONS: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommonOutputGrid {
    pub times: Vec<f64>,
    pub grid_id: String,
}

impl CommonOutputGrid {
    pub fn new(times: Vec<f64>) -> FairResult<Self> {
        if times.is_empty() || !times.iter().all(|value| value.is_finite()) {
            return Err(FairError::Invalid(
                "common output grid must be finite and nonempty".into(),
            ));
        }
        if times.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(FairError::Invalid(
                "common output grid must be strictly increasing".into(),
            ));
        }
        let grid_id = sha256_hex(&serde_json::to_vec(&times)?);
        Ok(Self { times, grid_id })
    }

    pub fn uniform(start: f64, end: f64, spacing: f64) -> FairResult<Self> {
        if !start.is_finite()
            || !end.is_finite()
            || !spacing.is_finite()
            || end < start
            || spacing <= 0.0
        {
            return Err(FairError::Invalid(
                "invalid uniform common output grid".into(),
            ));
        }
        let span = end - start;
        let intervals = (span / spacing).round() as usize;
        let tolerance = 128.0 * f64::EPSILON * end.abs().max(start.abs()).max(1.0);
        if (start + intervals as f64 * spacing - end).abs() > tolerance {
            return Err(FairError::Invalid(
                "uniform output spacing must divide the interval".into(),
            ));
        }
        let mut times = (0..=intervals)
            .map(|index| start + index as f64 * spacing)
            .collect::<Vec<_>>();
        if let Some(last) = times.last_mut() {
            *last = end;
        }
        Self::new(times)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExternalErrorScale {
    pub absolute: Vec<f64>,
    pub relative: f64,
    pub reference_uncertainty_wrms: f64,
}

impl ExternalErrorScale {
    pub fn new(absolute: Vec<f64>, relative: f64) -> FairResult<Self> {
        Self::with_reference_uncertainty(absolute, relative, 0.0)
    }

    pub fn with_reference_uncertainty(
        absolute: Vec<f64>,
        relative: f64,
        reference_uncertainty_wrms: f64,
    ) -> FairResult<Self> {
        let valid = !absolute.is_empty()
            && absolute
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
            && relative.is_finite()
            && relative >= 0.0
            && reference_uncertainty_wrms.is_finite()
            && reference_uncertainty_wrms >= 0.0;
        if !valid {
            return Err(FairError::Invalid(
                "external error scale must be finite, dimensioned, and positive".into(),
            ));
        }
        Ok(Self {
            absolute,
            relative,
            reference_uncertainty_wrms,
        })
    }

    fn weights(&self, reference: &[f64]) -> FairResult<Vec<f64>> {
        if reference.len() != self.absolute.len() {
            return Err(FairError::Invalid(
                "external error scale/reference dimension mismatch".into(),
            ));
        }
        let weights = reference
            .iter()
            .zip(&self.absolute)
            .map(|(value, absolute)| absolute + self.relative * value.abs())
            .collect::<Vec<_>>();
        if weights
            .iter()
            .all(|value| value.is_finite() && *value > 0.0)
        {
            Ok(weights)
        } else {
            Err(FairError::Invalid(
                "external error weights must be finite and positive".into(),
            ))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GlobalErrorMetric {
    EndpointL2,
    MaxGridL2,
    RmsGridL2,
    EndpointWrms,
    MaxGridWrms,
    RmsGridWrms,
    ConservativeMaxWrms,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalErrorMetrics {
    pub endpoint_l2: f64,
    pub max_grid_l2: f64,
    pub rms_grid_l2: f64,
    pub endpoint_wrms: f64,
    pub max_grid_wrms: f64,
    pub rms_grid_wrms: f64,
    pub reference_uncertainty_wrms: f64,
    pub conservative_max_wrms: f64,
}

pub type GlobalErrorReport = GlobalErrorMetrics;

impl GlobalErrorMetrics {
    pub fn value(&self, metric: GlobalErrorMetric) -> f64 {
        match metric {
            GlobalErrorMetric::EndpointL2 => self.endpoint_l2,
            GlobalErrorMetric::MaxGridL2 => self.max_grid_l2,
            GlobalErrorMetric::RmsGridL2 => self.rms_grid_l2,
            GlobalErrorMetric::EndpointWrms => self.endpoint_wrms,
            GlobalErrorMetric::MaxGridWrms => self.max_grid_wrms,
            GlobalErrorMetric::RmsGridWrms => self.rms_grid_wrms,
            GlobalErrorMetric::ConservativeMaxWrms => self.conservative_max_wrms,
        }
    }
}

fn matching_index(candidate_times: &[f64], target: f64) -> Option<usize> {
    candidate_times.iter().position(|candidate| {
        let tolerance = 64.0 * f64::EPSILON * candidate.abs().max(target.abs()).max(1.0);
        (*candidate - target).abs() <= tolerance
    })
}

pub fn compute_global_error_metrics(
    grid: &CommonOutputGrid,
    candidate_times: &[f64],
    candidate_states: &[Vec<f64>],
    reference_states: &[Vec<f64>],
    scale: &ExternalErrorScale,
) -> FairResult<GlobalErrorMetrics> {
    if candidate_times.len() != candidate_states.len() {
        return Err(FairError::Invalid(
            "candidate time/state length mismatch".into(),
        ));
    }
    if reference_states.len() != grid.times.len() {
        return Err(FairError::Invalid(
            "reference trajectory/output-grid length mismatch".into(),
        ));
    }
    let dimension = scale.absolute.len();
    if reference_states
        .iter()
        .any(|state| state.len() != dimension)
        || candidate_states
            .iter()
            .any(|state| state.len() != dimension)
    {
        return Err(FairError::Invalid(
            "candidate/reference state dimension mismatch".into(),
        ));
    }

    let mut l2_values = Vec::with_capacity(grid.times.len());
    let mut wrms_values = Vec::with_capacity(grid.times.len());
    for (grid_index, &time) in grid.times.iter().enumerate() {
        let candidate_index = matching_index(candidate_times, time)
            .ok_or_else(|| FairError::Invalid(format!("missing common output time {time:.17e}")))?;
        let candidate = &candidate_states[candidate_index];
        let reference = &reference_states[grid_index];
        let difference = candidate
            .iter()
            .zip(reference)
            .map(|(observed, exact)| observed - exact)
            .collect::<Vec<_>>();
        if !difference.iter().all(|value| value.is_finite()) {
            return Err(FairError::Invalid(
                "candidate/reference difference contains NaN/Inf".into(),
            ));
        }
        let weights = scale.weights(reference)?;
        let normalized = difference
            .iter()
            .zip(weights)
            .map(|(value, weight)| value / weight)
            .collect::<Vec<_>>();
        l2_values.push(safe_l2(&difference));
        wrms_values.push(safe_l2(&normalized) / (dimension as f64).sqrt());
    }

    let endpoint_l2 = *l2_values.last().expect("validated nonempty grid");
    let endpoint_wrms = *wrms_values.last().expect("validated nonempty grid");
    let max_grid_l2 = l2_values.iter().copied().fold(0.0, f64::max);
    let max_grid_wrms = wrms_values.iter().copied().fold(0.0, f64::max);
    let rms_grid_l2 =
        (l2_values.iter().map(|value| value * value).sum::<f64>() / l2_values.len() as f64).sqrt();
    let rms_grid_wrms = (wrms_values.iter().map(|value| value * value).sum::<f64>()
        / wrms_values.len() as f64)
        .sqrt();
    Ok(GlobalErrorMetrics {
        endpoint_l2,
        max_grid_l2,
        rms_grid_l2,
        endpoint_wrms,
        max_grid_wrms,
        rms_grid_wrms,
        reference_uncertainty_wrms: scale.reference_uncertainty_wrms,
        conservative_max_wrms: max_grid_wrms + scale.reference_uncertainty_wrms,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ParetoObservationStatus {
    Success,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParetoObservation {
    pub id: String,
    pub error: Option<f64>,
    pub cost: Option<f64>,
    pub status: ParetoObservationStatus,
}

impl ParetoObservation {
    pub fn success(id: impl Into<String>, error: f64, cost: f64) -> FairResult<Self> {
        if !(error.is_finite() && error >= 0.0 && cost.is_finite() && cost >= 0.0) {
            return Err(FairError::Invalid(
                "Pareto success point requires finite nonnegative error and cost".into(),
            ));
        }
        Ok(Self {
            id: id.into(),
            error: Some(error),
            cost: Some(cost),
            status: ParetoObservationStatus::Success,
        })
    }

    pub fn failure(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            error: None,
            cost: None,
            status: ParetoObservationStatus::Failure,
        }
    }
}

pub fn nondominated_observation_ids(points: &[ParetoObservation]) -> Vec<&str> {
    let successful = points
        .iter()
        .filter_map(|point| match (&point.status, point.error, point.cost) {
            (ParetoObservationStatus::Success, Some(error), Some(cost)) => {
                Some((point, error, cost))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut ids = successful
        .iter()
        .filter(|(point, error, cost)| {
            !successful.iter().any(|(other, other_error, other_cost)| {
                other.id != point.id
                    && *other_error <= *error
                    && *other_cost <= *cost
                    && (*other_error < *error || *other_cost < *cost)
            })
        })
        .map(|(point, _, _)| point.id.as_str())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

pub fn select_cheapest_below_target(points: &[ParetoObservation], target: f64) -> Option<&str> {
    if !(target.is_finite() && target >= 0.0) {
        return None;
    }
    points
        .iter()
        .filter_map(|point| match (&point.status, point.error, point.cost) {
            (ParetoObservationStatus::Success, Some(error), Some(cost)) if error <= target => {
                Some((point.id.as_str(), cost, error))
            }
            _ => None,
        })
        .min_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.0.cmp(right.0))
        })
        .map(|(id, _, _)| id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GlobalErrorParetoProfile {
    Smoke,
    Canonical,
}

impl GlobalErrorParetoProfile {
    fn timing_protocol(self) -> TimingProtocol {
        match self {
            Self::Smoke => TimingProtocol {
                warmups: SMOKE_WARMUPS,
                repetitions: SMOKE_REPETITIONS,
                seed: TIMING_SEED,
                minimum_sample_seconds: MIN_TIMING_SAMPLE_SECONDS,
                maximum_batch_iterations: MAX_TIMING_BATCH_ITERATIONS,
            },
            Self::Canonical => TimingProtocol {
                warmups: CANONICAL_WARMUPS,
                repetitions: CANONICAL_REPETITIONS,
                seed: TIMING_SEED,
                minimum_sample_seconds: MIN_TIMING_SAMPLE_SECONDS,
                maximum_batch_iterations: MAX_TIMING_BATCH_ITERATIONS,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParetoCostMetric {
    WallSeconds,
    RhsEvaluations,
    JvpVectors,
    JacobianBuilds,
    DirectFactorizations,
    NonlinearIterations,
    LinearIterations,
    AcceptedSteps,
    InternalSteps,
    OutputClippedSteps,
    StoredStateBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegratorRunStatus {
    Success,
    SolverFailure,
    MissingOutput,
    NonFinite,
    TimingFailure,
}

pub type GlobalRunStatus = IntegratorRunStatus;

impl IntegratorRunStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegratorWorkReport {
    pub counters: WorkCounters,
    /// Accepted internal integration steps, independent of retained output count.
    pub internal_steps: u64,
    /// Accepted steps shortened solely to land on a requested common-output time.
    pub output_clipped_steps: u64,
    /// Deterministic bytes retained in the returned requested-output trajectory; not peak RSS.
    pub stored_state_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegratorTimingReport {
    pub authoritative: bool,
    pub batch_iterations: usize,
    pub wall_samples_seconds: Vec<f64>,
    pub wall_median_seconds: Option<f64>,
    pub wall_q25_seconds: Option<f64>,
    pub wall_q75_seconds: Option<f64>,
}

impl IntegratorTimingReport {
    fn unavailable(authoritative: bool) -> Self {
        Self {
            authoritative,
            batch_iterations: 0,
            wall_samples_seconds: Vec::new(),
            wall_median_seconds: None,
            wall_q25_seconds: None,
            wall_q75_seconds: None,
        }
    }

    fn from_samples(
        mut samples: Vec<f64>,
        authoritative: bool,
        batch_iterations: usize,
    ) -> FairResult<Self> {
        if batch_iterations == 0
            || samples.is_empty()
            || !samples
                .iter()
                .all(|sample| sample.is_finite() && *sample >= 0.0)
        {
            return Err(FairError::Invalid(
                "timing samples must be finite, nonnegative, and nonempty".into(),
            ));
        }
        samples.sort_by(f64::total_cmp);
        let median = quantile_sorted(&samples, 0.5);
        let q25 = quantile_sorted(&samples, 0.25);
        let q75 = quantile_sorted(&samples, 0.75);
        Ok(Self {
            authoritative,
            batch_iterations,
            wall_samples_seconds: samples,
            wall_median_seconds: Some(median),
            wall_q25_seconds: Some(q25),
            wall_q75_seconds: Some(q75),
        })
    }
}

fn quantile_sorted(values: &[f64], probability: f64) -> f64 {
    debug_assert!(!values.is_empty());
    let position = probability * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let fraction = position - lower as f64;
        values[lower] * (1.0 - fraction) + values[upper] * fraction
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReferenceSourceKind {
    AnalyticExact,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceSolutionProvenance {
    pub problem_id: String,
    pub source_kind: ReferenceSourceKind,
    pub output_grid_id: String,
    pub state_checksum: String,
    pub reference_uncertainty_wrms: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReferenceTrajectory {
    pub output_grid: CommonOutputGrid,
    pub states: Vec<Vec<f64>>,
    pub provenance: ReferenceSolutionProvenance,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntegratorRunRecord {
    pub record_id: String,
    pub candidate_id: String,
    pub problem_id: String,
    pub step_size: f64,
    pub status: IntegratorRunStatus,
    pub message: String,
    pub errors: Option<GlobalErrorMetrics>,
    pub work: IntegratorWorkReport,
    pub timing: IntegratorTimingReport,
    pub reference_checksum: String,
    pub output_grid_id: String,
}

pub type GlobalErrorRunRow = IntegratorRunRecord;

impl IntegratorRunRecord {
    fn cost(&self, metric: ParetoCostMetric) -> Option<f64> {
        if !self.status.is_success() {
            return None;
        }
        let counters = self.work.counters;
        match metric {
            ParetoCostMetric::WallSeconds => self
                .timing
                .authoritative
                .then_some(self.timing.wall_median_seconds)
                .flatten(),
            ParetoCostMetric::RhsEvaluations => Some(counters.rhs_evaluations as f64),
            ParetoCostMetric::JvpVectors => Some(counters.jvp_vectors as f64),
            ParetoCostMetric::JacobianBuilds => Some(counters.jacobian_builds as f64),
            ParetoCostMetric::DirectFactorizations => Some(counters.direct_factorizations as f64),
            ParetoCostMetric::NonlinearIterations => Some(counters.nonlinear_iterations as f64),
            ParetoCostMetric::LinearIterations => Some(counters.linear_iterations as f64),
            ParetoCostMetric::AcceptedSteps => Some(counters.accepted_steps as f64),
            ParetoCostMetric::InternalSteps => Some(self.work.internal_steps as f64),
            ParetoCostMetric::OutputClippedSteps => Some(self.work.output_clipped_steps as f64),
            ParetoCostMetric::StoredStateBytes => Some(self.work.stored_state_bytes as f64),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalErrorParetoFront {
    pub problem_id: String,
    pub error_metric: GlobalErrorMetric,
    pub cost_metric: ParetoCostMetric,
    pub record_ids: Vec<String>,
}

pub type ParetoFront = GlobalErrorParetoFront;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalErrorTarget {
    pub target_id: String,
    pub metric: GlobalErrorMetric,
    pub threshold: f64,
}

impl GlobalErrorTarget {
    pub fn new(
        target_id: impl Into<String>,
        metric: GlobalErrorMetric,
        threshold: f64,
    ) -> FairResult<Self> {
        if !(threshold.is_finite() && threshold >= 0.0) {
            return Err(FairError::Invalid(
                "global-error target must be finite and nonnegative".into(),
            ));
        }
        Ok(Self {
            target_id: target_id.into(),
            metric,
            threshold,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TargetAttainment {
    pub problem_id: String,
    pub target_id: String,
    pub cost_metric: ParetoCostMetric,
    pub record_id: Option<String>,
    pub achieved_error: Option<f64>,
    pub cost: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimingProtocol {
    pub warmups: usize,
    pub repetitions: usize,
    pub seed: u64,
    pub minimum_sample_seconds: f64,
    pub maximum_batch_iterations: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalErrorExecution {
    pub threads: usize,
    pub backend: String,
    /// One pass over every scientific case/control task; comparable across thread counts.
    pub scientific_suite_wall_seconds: f64,
    /// Full randomized repeated timing campaign; present only for the authoritative T1 run.
    pub timing_campaign_wall_seconds: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPolicyMetadata {
    pub save_internal_steps: bool,
    pub dense_output_used: bool,
    pub landing: String,
}

impl OutputPolicyMetadata {
    fn requested_step_clipping() -> Self {
        Self {
            save_internal_steps: false,
            dense_output_used: false,
            landing: "step-clipping".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlobalErrorParetoReport {
    pub schema: String,
    pub profile: GlobalErrorParetoProfile,
    pub execution: GlobalErrorExecution,
    pub timing_authoritative: bool,
    pub timing_protocol: TimingProtocol,
    pub output_policy: OutputPolicyMetadata,
    pub references: Vec<ReferenceSolutionProvenance>,
    pub runs: Vec<IntegratorRunRecord>,
    pub fronts: Vec<GlobalErrorParetoFront>,
    pub targets: Vec<GlobalErrorTarget>,
    pub attainments: Vec<TargetAttainment>,
    pub scientific_checksum: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FixedAnchorCandidate {
    SequentialRodas5p,
    Bdf1,
    Bdf2,
    RadauIia1,
    RadauIia3,
}

impl FixedAnchorCandidate {
    const ALL: [Self; 5] = [
        Self::SequentialRodas5p,
        Self::Bdf1,
        Self::Bdf2,
        Self::RadauIia1,
        Self::RadauIia3,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::SequentialRodas5p => "sequential-rodas5p-direct",
            Self::Bdf1 => "bdf1-fixed",
            Self::Bdf2 => "bdf2-fixed",
            Self::RadauIia1 => "radau-iia1-fixed",
            Self::RadauIia3 => "radau-iia3-fixed",
        }
    }
}

#[derive(Clone)]
struct ReferenceProblem {
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    reference: ReferenceTrajectory,
    scale: ExternalErrorScale,
    step_sizes: Vec<f64>,
}

#[derive(Clone)]
struct FixedRunSpec {
    reference: ReferenceProblem,
    candidate: FixedAnchorCandidate,
    step_size: f64,
}

struct Trajectory {
    times: Vec<f64>,
    states: Vec<Vec<f64>>,
    counters: WorkCounters,
    internal_steps: u64,
    output_clipped_steps: u64,
}

fn analytic_reference_problem(
    problem: OdeProblem,
    y0: Vec<f64>,
    t_span: (f64, f64),
    output_spacing: f64,
    step_sizes: Vec<f64>,
) -> FairResult<ReferenceProblem> {
    let output_grid = CommonOutputGrid::uniform(t_span.0, t_span.1, output_spacing)?;
    let states = output_grid
        .times
        .iter()
        .map(|&time| {
            problem.exact(time).ok_or_else(|| {
                FairError::Invalid(format!("problem {} lacks analytic reference", problem.name))
            })
        })
        .collect::<FairResult<Vec<_>>>()?;
    let state_checksum = sha256_hex(&serde_json::to_vec(&(
        &problem.name,
        &output_grid.times,
        &states,
    ))?);
    let scale = ExternalErrorScale::new(vec![1.0e-10; problem.dimension], 1.0e-8)?;
    let provenance = ReferenceSolutionProvenance {
        problem_id: problem.name.clone(),
        source_kind: ReferenceSourceKind::AnalyticExact,
        output_grid_id: output_grid.grid_id.clone(),
        state_checksum: state_checksum.clone(),
        reference_uncertainty_wrms: 0.0,
    };
    Ok(ReferenceProblem {
        problem,
        y0,
        t_span,
        reference: ReferenceTrajectory {
            output_grid,
            states,
            provenance,
        },
        scale,
        step_sizes,
    })
}

fn fixed_anchor_corpus(profile: GlobalErrorParetoProfile) -> FairResult<Vec<ReferenceProblem>> {
    let mut corpus = Vec::new();
    let (linear, linear_y0) = scalar_linear_problem(-2.0, 1.0);
    corpus.push(analytic_reference_problem(
        linear,
        linear_y0,
        (0.0, 0.2),
        0.04,
        vec![0.04, 0.02, 0.01],
    )?);
    let (pr, pr_y0) = prothero_robinson_problem(-20.0, 1.0, 0.0);
    corpus.push(analytic_reference_problem(
        pr,
        pr_y0,
        (0.0, 0.2),
        0.04,
        vec![0.04, 0.02, 0.01],
    )?);
    if profile == GlobalErrorParetoProfile::Canonical {
        let (vector, vector_y0) = manufactured_vector_problem(4, 20.0, 1.0, 0.2, 0.0)?;
        corpus.push(analytic_reference_problem(
            vector,
            vector_y0,
            (0.0, 0.2),
            0.04,
            vec![0.04, 0.02, 0.01, 0.005],
        )?);
        let (mass, mass_y0, _, _) = manufactured_mass_nonlinear_problem(20.0, 1.0, 0.2, 0.0)?;
        corpus.push(analytic_reference_problem(
            mass,
            mass_y0,
            (0.0, 0.08),
            0.02,
            vec![0.02, 0.01, 0.005, 0.0025],
        )?);
    }
    Ok(corpus)
}

fn execute_candidate(spec: &FixedRunSpec) -> Result<Trajectory, CoreError> {
    let problem = &spec.reference.problem;
    let t_span = spec.reference.t_span;
    let y0 = &spec.reference.y0;
    let h = spec.step_size;
    let output = OutputSchedule::new(spec.reference.reference.output_grid.times.clone())?;
    let result = match spec.candidate {
        FixedAnchorCandidate::SequentialRodas5p => {
            let config = LinearSolverConfig::default();
            integrate_fixed_observed(
                problem,
                t_span,
                y0,
                h,
                IntegrationMethod::Sequential,
                Some(&config),
                None,
                1.0e-12,
                1.0e-10,
                &output,
            )?
        }
        FixedAnchorCandidate::Bdf1 | FixedAnchorCandidate::Bdf2 => {
            let order = if matches!(spec.candidate, FixedAnchorCandidate::Bdf1) {
                BdfOrder::One
            } else {
                BdfOrder::Two
            };
            integrate_bdf_fixed_observed(
                problem,
                t_span,
                y0,
                h,
                &BdfConfig {
                    order,
                    newton: NewtonConfig::default(),
                },
                &output,
            )?
        }
        FixedAnchorCandidate::RadauIia1 | FixedAnchorCandidate::RadauIia3 => {
            let stages = if matches!(spec.candidate, FixedAnchorCandidate::RadauIia1) {
                RadauIiaStages::One
            } else {
                RadauIiaStages::Three
            };
            integrate_radau_fixed_observed(
                problem,
                t_span,
                y0,
                h,
                &RadauConfig {
                    stages,
                    ..RadauConfig::default()
                },
                &output,
            )?
        }
    };
    Ok(Trajectory {
        times: result.t,
        states: result.y,
        counters: result.counters,
        internal_steps: result.internal_steps as u64,
        output_clipped_steps: result.output_clipped_steps as u64,
    })
}

fn stored_state_bytes(trajectory: &Trajectory) -> u64 {
    let scalars = trajectory.times.len() + trajectory.states.iter().map(Vec::len).sum::<usize>();
    (scalars * std::mem::size_of::<f64>()) as u64
}

fn record_id(spec: &FixedRunSpec) -> String {
    format!(
        "{}|{}|h{:016x}",
        spec.reference.problem.name,
        spec.candidate.id(),
        spec.step_size.to_bits()
    )
}

fn run_scientific_spec(spec: &FixedRunSpec) -> IntegratorRunRecord {
    let id = record_id(spec);
    match execute_candidate(spec) {
        Ok(trajectory) => match compute_global_error_metrics(
            &spec.reference.reference.output_grid,
            &trajectory.times,
            &trajectory.states,
            &spec.reference.reference.states,
            &spec.reference.scale,
        ) {
            Ok(errors) => IntegratorRunRecord {
                record_id: id,
                candidate_id: spec.candidate.id().into(),
                problem_id: spec.reference.problem.name.clone(),
                step_size: spec.step_size,
                status: IntegratorRunStatus::Success,
                message: "success".into(),
                errors: Some(errors),
                work: IntegratorWorkReport {
                    counters: trajectory.counters,
                    internal_steps: trajectory.internal_steps,
                    output_clipped_steps: trajectory.output_clipped_steps,
                    stored_state_bytes: stored_state_bytes(&trajectory),
                },
                timing: IntegratorTimingReport::unavailable(false),
                reference_checksum: spec.reference.reference.provenance.state_checksum.clone(),
                output_grid_id: spec.reference.reference.output_grid.grid_id.clone(),
            },
            Err(error) => IntegratorRunRecord {
                record_id: id,
                candidate_id: spec.candidate.id().into(),
                problem_id: spec.reference.problem.name.clone(),
                step_size: spec.step_size,
                status: if error.to_string().contains("missing common output time") {
                    IntegratorRunStatus::MissingOutput
                } else {
                    IntegratorRunStatus::NonFinite
                },
                message: error.to_string(),
                errors: None,
                work: IntegratorWorkReport {
                    counters: trajectory.counters,
                    internal_steps: trajectory.internal_steps,
                    output_clipped_steps: trajectory.output_clipped_steps,
                    stored_state_bytes: stored_state_bytes(&trajectory),
                },
                timing: IntegratorTimingReport::unavailable(false),
                reference_checksum: spec.reference.reference.provenance.state_checksum.clone(),
                output_grid_id: spec.reference.reference.output_grid.grid_id.clone(),
            },
        },
        Err(error) => IntegratorRunRecord {
            record_id: id,
            candidate_id: spec.candidate.id().into(),
            problem_id: spec.reference.problem.name.clone(),
            step_size: spec.step_size,
            status: IntegratorRunStatus::SolverFailure,
            message: error.to_string(),
            errors: None,
            work: IntegratorWorkReport {
                counters: WorkCounters::default(),
                internal_steps: 0,
                output_clipped_steps: 0,
                stored_state_bytes: 0,
            },
            timing: IntegratorTimingReport::unavailable(false),
            reference_checksum: spec.reference.reference.provenance.state_checksum.clone(),
            output_grid_id: spec.reference.reference.output_grid.grid_id.clone(),
        },
    }
}

fn error_metrics() -> [GlobalErrorMetric; 7] {
    [
        GlobalErrorMetric::EndpointL2,
        GlobalErrorMetric::MaxGridL2,
        GlobalErrorMetric::RmsGridL2,
        GlobalErrorMetric::EndpointWrms,
        GlobalErrorMetric::MaxGridWrms,
        GlobalErrorMetric::RmsGridWrms,
        GlobalErrorMetric::ConservativeMaxWrms,
    ]
}

fn cost_metrics() -> [ParetoCostMetric; 11] {
    [
        ParetoCostMetric::WallSeconds,
        ParetoCostMetric::RhsEvaluations,
        ParetoCostMetric::JvpVectors,
        ParetoCostMetric::JacobianBuilds,
        ParetoCostMetric::DirectFactorizations,
        ParetoCostMetric::NonlinearIterations,
        ParetoCostMetric::LinearIterations,
        ParetoCostMetric::AcceptedSteps,
        ParetoCostMetric::InternalSteps,
        ParetoCostMetric::OutputClippedSteps,
        ParetoCostMetric::StoredStateBytes,
    ]
}

fn build_fronts(runs: &[IntegratorRunRecord]) -> Vec<GlobalErrorParetoFront> {
    let mut by_problem = BTreeMap::<&str, Vec<&IntegratorRunRecord>>::new();
    for run in runs {
        by_problem.entry(&run.problem_id).or_default().push(run);
    }
    let mut fronts = Vec::new();
    for (problem_id, problem_runs) in by_problem {
        for error_metric in error_metrics() {
            for cost_metric in cost_metrics() {
                let observations = problem_runs
                    .iter()
                    .map(|run| {
                        if let (Some(errors), Some(cost)) = (&run.errors, run.cost(cost_metric)) {
                            ParetoObservation::success(
                                run.record_id.clone(),
                                errors.value(error_metric),
                                cost,
                            )
                            .expect("validated run metrics")
                        } else {
                            ParetoObservation::failure(run.record_id.clone())
                        }
                    })
                    .collect::<Vec<_>>();
                fronts.push(GlobalErrorParetoFront {
                    problem_id: problem_id.to_owned(),
                    error_metric,
                    cost_metric,
                    record_ids: nondominated_observation_ids(&observations)
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                });
            }
        }
    }
    fronts
}

fn default_targets() -> FairResult<Vec<GlobalErrorTarget>> {
    [1.0e-2, 1.0e-4, 1.0e-6, 1.0e-8]
        .into_iter()
        .map(|threshold| {
            GlobalErrorTarget::new(
                format!("max-grid-l2-{threshold:.0e}"),
                GlobalErrorMetric::MaxGridL2,
                threshold,
            )
        })
        .collect()
}

fn build_attainments(
    runs: &[IntegratorRunRecord],
    targets: &[GlobalErrorTarget],
) -> Vec<TargetAttainment> {
    let mut by_problem = BTreeMap::<&str, Vec<&IntegratorRunRecord>>::new();
    for run in runs {
        by_problem.entry(&run.problem_id).or_default().push(run);
    }
    let mut out = Vec::new();
    for (problem_id, problem_runs) in by_problem {
        for target in targets {
            for cost_metric in cost_metrics() {
                let best = problem_runs
                    .iter()
                    .filter_map(|run| {
                        let errors = run.errors.as_ref()?;
                        let error = errors.value(target.metric);
                        let cost = run.cost(cost_metric)?;
                        (error <= target.threshold).then_some((run.record_id.as_str(), error, cost))
                    })
                    .min_by(|left, right| {
                        left.2
                            .total_cmp(&right.2)
                            .then_with(|| left.1.total_cmp(&right.1))
                            .then_with(|| left.0.cmp(right.0))
                    });
                out.push(TargetAttainment {
                    problem_id: problem_id.to_owned(),
                    target_id: target.target_id.clone(),
                    cost_metric,
                    record_id: best.map(|item| item.0.to_owned()),
                    achieved_error: best.map(|item| item.1),
                    cost: best.map(|item| item.2),
                });
            }
        }
    }
    out
}

#[derive(Serialize)]
struct ScientificRun<'a> {
    record_id: &'a str,
    candidate_id: &'a str,
    problem_id: &'a str,
    step_size_bits: u64,
    status: &'a IntegratorRunStatus,
    errors: &'a Option<GlobalErrorMetrics>,
    work: &'a IntegratorWorkReport,
    reference_checksum: &'a str,
    output_grid_id: &'a str,
}

fn scientific_checksum(
    profile: GlobalErrorParetoProfile,
    output_policy: &OutputPolicyMetadata,
    references: &[ReferenceSolutionProvenance],
    runs: &[IntegratorRunRecord],
    fronts: &[GlobalErrorParetoFront],
    targets: &[GlobalErrorTarget],
    attainments: &[TargetAttainment],
) -> FairResult<String> {
    let scientific_runs = runs
        .iter()
        .map(|run| ScientificRun {
            record_id: &run.record_id,
            candidate_id: &run.candidate_id,
            problem_id: &run.problem_id,
            step_size_bits: run.step_size.to_bits(),
            status: &run.status,
            errors: &run.errors,
            work: &run.work,
            reference_checksum: &run.reference_checksum,
            output_grid_id: &run.output_grid_id,
        })
        .collect::<Vec<_>>();
    let scientific_fronts = fronts
        .iter()
        .filter(|front| front.cost_metric != ParetoCostMetric::WallSeconds)
        .collect::<Vec<_>>();
    let scientific_attainments = attainments
        .iter()
        .filter(|row| row.cost_metric != ParetoCostMetric::WallSeconds)
        .collect::<Vec<_>>();
    Ok(sha256_hex(&serde_json::to_vec(&(
        profile,
        output_policy,
        references,
        scientific_runs,
        scientific_fronts,
        targets,
        scientific_attainments,
    ))?))
}

fn shuffled_indices(length: usize, rng: &mut Pcg64Mcg) -> Vec<usize> {
    let mut indices = (0..length).collect::<Vec<_>>();
    for index in (1..indices.len()).rev() {
        let other = rng.random_range(0..=index);
        indices.swap(index, other);
    }
    indices
}

fn authoritative_timing(
    specs: &[FixedRunSpec],
    protocol: &TimingProtocol,
) -> (
    BTreeMap<String, IntegratorTimingReport>,
    BTreeMap<String, String>,
    f64,
) {
    let total_started = Instant::now();
    let mut batch_iterations = BTreeMap::<String, usize>::new();
    let mut failures = BTreeMap::<String, String>::new();

    for spec in specs {
        let id = record_id(spec);
        let mut calibration_seconds = None;
        for _ in 0..protocol.warmups.max(1) {
            let started = Instant::now();
            match execute_candidate(spec) {
                Ok(_) => calibration_seconds = Some(started.elapsed().as_secs_f64()),
                Err(error) => {
                    failures.insert(id.clone(), error.to_string());
                    break;
                }
            }
        }
        if let Some(seconds) = calibration_seconds {
            let seconds = seconds.max(f64::MIN_POSITIVE);
            let iterations = (protocol.minimum_sample_seconds / seconds).ceil().max(1.0) as usize;
            batch_iterations.insert(id, iterations.min(protocol.maximum_batch_iterations));
        }
    }

    let mut samples = BTreeMap::<String, Vec<f64>>::new();
    let mut rng = Pcg64Mcg::seed_from_u64(protocol.seed);
    for _ in 0..protocol.repetitions {
        for index in shuffled_indices(specs.len(), &mut rng) {
            let spec = &specs[index];
            let id = record_id(spec);
            if failures.contains_key(&id) {
                continue;
            }
            let iterations = *batch_iterations.get(&id).unwrap_or(&1);
            let started = Instant::now();
            let mut failure = None;
            for _ in 0..iterations {
                if let Err(error) = execute_candidate(spec) {
                    failure = Some(error.to_string());
                    break;
                }
            }
            if let Some(message) = failure {
                failures.insert(id, message);
            } else {
                samples
                    .entry(id)
                    .or_default()
                    .push(started.elapsed().as_secs_f64() / iterations as f64);
            }
        }
    }
    let reports = samples
        .into_iter()
        .filter_map(|(id, values)| {
            let iterations = *batch_iterations.get(&id).unwrap_or(&1);
            IntegratorTimingReport::from_samples(values, true, iterations)
                .ok()
                .map(|report| (id, report))
        })
        .collect();
    (reports, failures, total_started.elapsed().as_secs_f64())
}

fn parallel_scientific_runs(
    specs: &[FixedRunSpec],
    execution: &ParallelExecution,
) -> Result<(Vec<IntegratorRunRecord>, f64), CoreError> {
    let started = Instant::now();
    let rows = execution.map_ordered(specs, |spec| {
        let row_started = Instant::now();
        let mut row = run_scientific_spec(spec);
        row.timing = IntegratorTimingReport::from_samples(
            vec![row_started.elapsed().as_secs_f64()],
            false,
            1,
        )
        .map_err(|error| CoreError::InvalidInput(error.to_string()))?;
        Ok(row)
    })?;
    Ok((rows, started.elapsed().as_secs_f64()))
}

pub fn run_global_error_pareto_screen(
    profile: GlobalErrorParetoProfile,
    threads: usize,
) -> FairResult<GlobalErrorParetoReport> {
    let execution = ParallelExecution::rayon(threads)?;
    let protocol = profile.timing_protocol();
    let output_policy = OutputPolicyMetadata::requested_step_clipping();
    let corpus = fixed_anchor_corpus(profile)?;
    let references = corpus
        .iter()
        .map(|problem| problem.reference.provenance.clone())
        .collect::<Vec<_>>();
    let mut specs = Vec::new();
    for reference in corpus {
        for &step_size in &reference.step_sizes {
            for candidate in FixedAnchorCandidate::ALL {
                specs.push(FixedRunSpec {
                    reference: reference.clone(),
                    candidate,
                    step_size,
                });
            }
        }
    }

    let (mut runs, scientific_suite_wall_seconds, timing_campaign_wall_seconds) = if threads == 1 {
        let scientific_started = Instant::now();
        let mut rows = specs.iter().map(run_scientific_spec).collect::<Vec<_>>();
        let scientific_elapsed = scientific_started.elapsed().as_secs_f64();
        let (timing, failures, timing_elapsed) = authoritative_timing(&specs, &protocol);
        for row in &mut rows {
            if let Some(message) = failures.get(&row.record_id) {
                row.status = IntegratorRunStatus::TimingFailure;
                row.message = format!("authoritative timing failed: {message}");
                row.timing = IntegratorTimingReport::unavailable(true);
            } else if let Some(report) = timing.get(&row.record_id) {
                row.timing = report.clone();
            } else {
                row.status = IntegratorRunStatus::TimingFailure;
                row.message = "authoritative timing produced no samples".into();
                row.timing = IntegratorTimingReport::unavailable(true);
            }
        }
        (rows, scientific_elapsed, Some(timing_elapsed))
    } else {
        let (rows, scientific_elapsed) = parallel_scientific_runs(&specs, &execution)?;
        (rows, scientific_elapsed, None)
    };

    runs.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    let fronts = build_fronts(&runs);
    let targets = default_targets()?;
    let attainments = build_attainments(&runs, &targets);
    let scientific_checksum = scientific_checksum(
        profile,
        &output_policy,
        &references,
        &runs,
        &fronts,
        &targets,
        &attainments,
    )?;
    Ok(GlobalErrorParetoReport {
        schema: "rodas5p-global-error-pareto-v2".into(),
        profile,
        execution: GlobalErrorExecution {
            threads: execution.threads(),
            backend: execution.backend().into(),
            scientific_suite_wall_seconds,
            timing_campaign_wall_seconds,
        },
        timing_authoritative: threads == 1,
        timing_protocol: protocol,
        output_policy,
        references,
        runs,
        fronts,
        targets,
        attainments,
        scientific_checksum,
    })
}
