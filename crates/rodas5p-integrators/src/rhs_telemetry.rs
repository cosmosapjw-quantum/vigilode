use std::{collections::BTreeMap, sync::Arc};

use faer::Mat;
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearOperator, LuFactorization, WorkCounters, safe_l2,
    sha256_hex,
};
use serde::Serialize;

use crate::homotopy::{HomotopyBatchEvent, HomotopyBatchObserver};
use crate::{
    HomotopyPathConfig, HomotopyPredictor, OdeProblem, PredictorKind, SabrConfig, StageHistory,
    StructuredBlockSystem, build_step_context, constant_affine_mass_problem,
    manufactured_vector_problem, prothero_robinson_problem,
};

const DEFAULT_RANK_TOLERANCE: f64 = 1.0e-8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommonWBackendChoice {
    SeededSharedGmres,
    BlockGmres,
    IndependentRayonGmres,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RhsTelemetryRisk {
    Low,
    Moderate,
    High,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RhsBatchAnalysis {
    pub dimension: usize,
    pub rhs_count: usize,
    pub singular_values: Vec<f64>,
    pub relative_singular_values: Vec<f64>,
    pub numerical_rank: usize,
    pub rank_tolerance: f64,
    pub energy_rank_99: usize,
    pub energy_rank_999: usize,
    pub stable_rank: f64,
    pub condition_proxy: Option<f64>,
    pub row_norm_minimum: f64,
    pub row_norm_maximum: f64,
    pub row_norm_spread: f64,
    pub pairwise_cosines: Vec<Vec<f64>>,
    pub maximum_abs_pairwise_cosine: f64,
    pub median_abs_pairwise_cosine: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RhsSubspaceComparison {
    pub previous_rank: usize,
    pub current_rank: usize,
    pub principal_cosines: Vec<f64>,
    pub minimum_principal_angle_degrees: f64,
    pub maximum_principal_angle_degrees: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RhsBatchTelemetryRow {
    pub case_id: String,
    pub method: String,
    pub phase: String,
    pub q: Option<usize>,
    pub theta: Option<f64>,
    pub path_rounds: Option<usize>,
    pub round: usize,
    pub propagation_level: usize,
    pub iteration: Option<usize>,
    pub dimension: usize,
    pub rhs_count: usize,
    pub stiffness_proxy: f64,
    pub nonnormality_proxy: f64,
    pub risk: RhsTelemetryRisk,
    pub raw: RhsBatchAnalysis,
    pub transformed: RhsBatchAnalysis,
    pub raw_directional: RhsBatchAnalysis,
    pub transformed_directional: RhsBatchAnalysis,
    pub raw_drift: Option<RhsSubspaceComparison>,
    pub transformed_drift: Option<RhsSubspaceComparison>,
    pub suggested_backend: CommonWBackendChoice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HomotopyRhsTelemetryProfile {
    Smoke,
    Canonical,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyRhsTelemetryCase {
    pub case_id: String,
    pub family: String,
    pub dimension: usize,
    pub stiffness_proxy: f64,
    pub nonnormality_proxy: f64,
    pub t: f64,
    pub h: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RhsTelemetryFailure {
    pub case_id: String,
    pub method: String,
    pub q: Option<usize>,
    pub theta: Option<f64>,
    pub predictor: Option<HomotopyPredictor>,
    pub error: String,
    pub rows_preserved: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BackendRecommendationSummary {
    pub seeded_shared: usize,
    pub block_gmres: usize,
    pub independent_rayon: usize,
    pub raw_rank_one: usize,
    pub transformed_rank_one: usize,
    pub transformed_directional_rank_one: usize,
    pub high_risk_rows: usize,
    pub failed_paths: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HomotopyRhsTelemetryReport {
    pub schema: &'static str,
    pub profile: HomotopyRhsTelemetryProfile,
    pub cases: Vec<HomotopyRhsTelemetryCase>,
    pub rows: Vec<RhsBatchTelemetryRow>,
    pub failures: Vec<RhsTelemetryFailure>,
    pub summary: BackendRecommendationSummary,
    pub rank_tolerance: f64,
    pub dispatcher_active: bool,
    pub backend_recommendations_advisory: bool,
    pub explicit_jacobian_builds_in_dispatch: u64,
    pub reference_explicit_jacobian_builds: u64,
    pub reference_factorization_builds: u64,
    pub behavior_comparisons: usize,
    pub behavior_mismatches: usize,
    pub solver_behavior_changed: bool,
    pub scientific_checksum: String,
    pub verdict: String,
}

#[derive(Clone, Debug)]
pub(crate) struct RhsTelemetryContext {
    pub case_id: String,
    pub method: String,
    pub q: Option<usize>,
    pub theta: Option<f64>,
    pub path_rounds: Option<usize>,
    pub stiffness_proxy: f64,
    pub nonnormality_proxy: f64,
    pub risk: RhsTelemetryRisk,
}

#[derive(Clone, Debug)]
pub(crate) struct RhsBatchIdentity {
    pub phase: String,
    pub round: usize,
    pub propagation_level: usize,
    pub iteration: Option<usize>,
}

#[derive(Clone, Debug)]
struct SubspaceBasis {
    rank: usize,
    vectors: Vec<Vec<f64>>,
}

#[derive(Clone, Debug)]
pub(crate) struct RhsTelemetryRecorder {
    context: RhsTelemetryContext,
    rank_tolerance: f64,
    rows: Vec<RhsBatchTelemetryRow>,
    previous_raw: BTreeMap<(String, usize), Vec<Vec<f64>>>,
    previous_transformed: BTreeMap<(String, usize), Vec<Vec<f64>>>,
}

impl RhsTelemetryRecorder {
    pub(crate) fn new(context: RhsTelemetryContext, rank_tolerance: f64) -> CoreResult<Self> {
        validate_rank_tolerance(rank_tolerance)?;
        Ok(Self {
            context,
            rank_tolerance,
            rows: Vec::new(),
            previous_raw: BTreeMap::new(),
            previous_transformed: BTreeMap::new(),
        })
    }

    pub(crate) fn record(
        &mut self,
        identity: RhsBatchIdentity,
        raw: &[Vec<f64>],
        transformed: &[Vec<f64>],
    ) -> CoreResult<()> {
        let raw_analysis = analyze_rhs_batch(raw, self.rank_tolerance)?;
        let transformed_analysis = analyze_rhs_batch(transformed, self.rank_tolerance)?;
        let raw_directional = analyze_rhs_directions(raw, self.rank_tolerance)?;
        let transformed_directional = analyze_rhs_directions(transformed, self.rank_tolerance)?;
        let semantic_key = (identity.phase.clone(), identity.propagation_level);
        let raw_drift = self
            .previous_raw
            .get(&semantic_key)
            .map(|previous| compare_rhs_subspaces(previous, raw, self.rank_tolerance))
            .transpose()?;
        let transformed_drift = self
            .previous_transformed
            .get(&semantic_key)
            .map(|previous| compare_rhs_subspaces(previous, transformed, self.rank_tolerance))
            .transpose()?;
        let suggested_backend =
            recommend_common_w_backend(&transformed_directional, self.context.risk);
        self.rows.push(RhsBatchTelemetryRow {
            case_id: self.context.case_id.clone(),
            method: self.context.method.clone(),
            phase: identity.phase,
            q: self.context.q,
            theta: self.context.theta,
            path_rounds: self.context.path_rounds,
            round: identity.round,
            propagation_level: identity.propagation_level,
            iteration: identity.iteration,
            dimension: raw_analysis.dimension,
            rhs_count: raw_analysis.rhs_count,
            stiffness_proxy: self.context.stiffness_proxy,
            nonnormality_proxy: self.context.nonnormality_proxy,
            risk: self.context.risk,
            raw: raw_analysis,
            transformed: transformed_analysis,
            raw_directional,
            transformed_directional,
            raw_drift,
            transformed_drift,
            suggested_backend,
        });
        self.previous_raw.insert(semantic_key.clone(), raw.to_vec());
        self.previous_transformed
            .insert(semantic_key, transformed.to_vec());
        Ok(())
    }

    pub(crate) fn into_rows(self) -> Vec<RhsBatchTelemetryRow> {
        self.rows
    }
}

impl HomotopyBatchObserver for RhsTelemetryRecorder {
    fn observe(&mut self, event: HomotopyBatchEvent<'_>) -> CoreResult<()> {
        self.record(
            RhsBatchIdentity {
                phase: event.phase.into(),
                round: event.round,
                propagation_level: event.propagation_level,
                iteration: event.iteration,
            },
            event.raw,
            event.transformed,
        )
    }
}

fn validate_rows(rows: &[Vec<f64>]) -> CoreResult<(usize, usize)> {
    if rows.is_empty() {
        return Err(CoreError::InvalidInput(
            "RHS telemetry requires at least one right-hand side".into(),
        ));
    }
    let dimension = rows[0].len();
    if dimension == 0 || rows.iter().any(|row| row.len() != dimension) {
        return Err(CoreError::Dimension(
            "RHS telemetry rows must be nonempty and rectangular".into(),
        ));
    }
    if !rows.iter().flatten().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "RHS telemetry rows contain NaN/Inf".into(),
        ));
    }
    Ok((dimension, rows.len()))
}

fn validate_rank_tolerance(rank_tolerance: f64) -> CoreResult<()> {
    if !(rank_tolerance > 0.0 && rank_tolerance < 1.0 && rank_tolerance.is_finite()) {
        return Err(CoreError::InvalidInput(
            "RHS telemetry rank tolerance must lie strictly between zero and one".into(),
        ));
    }
    Ok(())
}

fn column_matrix(rows: &[Vec<f64>]) -> CoreResult<Mat<f64>> {
    let (dimension, rhs_count) = validate_rows(rows)?;
    Ok(Mat::from_fn(dimension, rhs_count, |component, rhs| {
        rows[rhs][component]
    }))
}

fn singular_spectrum_and_basis(
    rows: &[Vec<f64>],
    rank_tolerance: f64,
) -> CoreResult<(Vec<f64>, SubspaceBasis)> {
    validate_rank_tolerance(rank_tolerance)?;
    let matrix = column_matrix(rows)?;
    let svd = matrix
        .thin_svd()
        .map_err(|error| CoreError::LinearSolve(format!("RHS telemetry SVD failed: {error:?}")))?;
    let singular_values = svd.S().column_vector().iter().copied().collect::<Vec<_>>();
    if !singular_values
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
    {
        return Err(CoreError::NonFinite(
            "RHS telemetry SVD produced invalid singular values".into(),
        ));
    }
    let maximum = singular_values.first().copied().unwrap_or(0.0);
    let threshold = rank_tolerance * maximum;
    let rank = if maximum <= f64::MIN_POSITIVE {
        0
    } else {
        singular_values
            .iter()
            .take_while(|value| **value > threshold)
            .count()
    };
    let u = svd.U();
    let vectors = (0..rank)
        .map(|column| (0..u.nrows()).map(|row| u[(row, column)]).collect())
        .collect();
    Ok((singular_values, SubspaceBasis { rank, vectors }))
}

fn energy_rank(singular_values: &[f64], fraction: f64) -> usize {
    let total = singular_values
        .iter()
        .map(|value| value * value)
        .sum::<f64>();
    if total <= f64::MIN_POSITIVE {
        return 0;
    }
    let mut cumulative = 0.0;
    for (index, value) in singular_values.iter().enumerate() {
        cumulative += value * value;
        if cumulative / total >= fraction {
            return index + 1;
        }
    }
    singular_values.len()
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    if values.len() % 2 == 1 {
        values[values.len() / 2]
    } else {
        let upper = values.len() / 2;
        0.5 * (values[upper - 1] + values[upper])
    }
}

pub fn analyze_rhs_batch(rows: &[Vec<f64>], rank_tolerance: f64) -> CoreResult<RhsBatchAnalysis> {
    let (dimension, rhs_count) = validate_rows(rows)?;
    let (singular_values, basis) = singular_spectrum_and_basis(rows, rank_tolerance)?;
    let maximum = singular_values.first().copied().unwrap_or(0.0);
    let relative_singular_values = if maximum <= f64::MIN_POSITIVE {
        vec![0.0; singular_values.len()]
    } else {
        singular_values
            .iter()
            .map(|value| value / maximum)
            .collect()
    };
    let stable_rank = if maximum <= f64::MIN_POSITIVE {
        0.0
    } else {
        singular_values
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            / (maximum * maximum)
    };
    let condition_proxy = if basis.rank == 0 {
        None
    } else {
        Some(maximum / singular_values[basis.rank - 1].max(f64::MIN_POSITIVE))
    };

    let norms = rows.iter().map(|row| safe_l2(row)).collect::<Vec<_>>();
    let row_norm_minimum = norms.iter().copied().fold(f64::INFINITY, f64::min);
    let row_norm_maximum = norms.iter().copied().fold(0.0_f64, f64::max);
    let positive_minimum = norms
        .iter()
        .copied()
        .filter(|value| *value > f64::MIN_POSITIVE)
        .fold(f64::INFINITY, f64::min);
    let row_norm_spread = if positive_minimum.is_finite() {
        row_norm_maximum / positive_minimum
    } else if row_norm_maximum <= f64::MIN_POSITIVE {
        1.0
    } else {
        f64::INFINITY
    };

    let mut pairwise_cosines = vec![vec![0.0; rhs_count]; rhs_count];
    let mut off_diagonal_abs = Vec::new();
    for i in 0..rhs_count {
        for j in 0..rhs_count {
            let denominator = norms[i] * norms[j];
            let cosine = if denominator <= f64::MIN_POSITIVE {
                0.0
            } else {
                rows[i]
                    .iter()
                    .zip(&rows[j])
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    / denominator
            }
            .clamp(-1.0, 1.0);
            pairwise_cosines[i][j] = cosine;
            if i < j {
                off_diagonal_abs.push(cosine.abs());
            }
        }
    }
    let maximum_abs_pairwise_cosine = off_diagonal_abs.iter().copied().fold(0.0_f64, f64::max);
    let median_abs_pairwise_cosine = median(&mut off_diagonal_abs);

    let analysis = RhsBatchAnalysis {
        dimension,
        rhs_count,
        singular_values: singular_values.clone(),
        relative_singular_values,
        numerical_rank: basis.rank,
        rank_tolerance,
        energy_rank_99: energy_rank(&singular_values, 0.99),
        energy_rank_999: energy_rank(&singular_values, 0.999),
        stable_rank,
        condition_proxy,
        row_norm_minimum,
        row_norm_maximum,
        row_norm_spread,
        pairwise_cosines,
        maximum_abs_pairwise_cosine,
        median_abs_pairwise_cosine,
    };
    if analysis
        .singular_values
        .iter()
        .all(|value| value.is_finite())
        && analysis
            .relative_singular_values
            .iter()
            .all(|value| value.is_finite())
        && [
            analysis.stable_rank,
            analysis.row_norm_minimum,
            analysis.row_norm_maximum,
            analysis.row_norm_spread,
            analysis.maximum_abs_pairwise_cosine,
            analysis.median_abs_pairwise_cosine,
        ]
        .iter()
        .all(|value| value.is_finite())
    {
        Ok(analysis)
    } else {
        Err(CoreError::NonFinite(
            "RHS telemetry analysis contains NaN/Inf".into(),
        ))
    }
}

pub fn analyze_rhs_directions(
    rows: &[Vec<f64>],
    rank_tolerance: f64,
) -> CoreResult<RhsBatchAnalysis> {
    validate_rows(rows)?;
    let normalized = rows
        .iter()
        .map(|row| {
            let norm = safe_l2(row);
            if norm <= f64::MIN_POSITIVE {
                vec![0.0; row.len()]
            } else {
                row.iter().map(|value| value / norm).collect()
            }
        })
        .collect::<Vec<_>>();
    analyze_rhs_batch(&normalized, rank_tolerance)
}

pub fn compare_rhs_subspaces(
    previous: &[Vec<f64>],
    current: &[Vec<f64>],
    rank_tolerance: f64,
) -> CoreResult<RhsSubspaceComparison> {
    let (previous_dimension, _) = validate_rows(previous)?;
    let (current_dimension, _) = validate_rows(current)?;
    if previous_dimension != current_dimension {
        return Err(CoreError::Dimension(
            "RHS telemetry subspace dimensions differ".into(),
        ));
    }
    let (_, previous_basis) = singular_spectrum_and_basis(previous, rank_tolerance)?;
    let (_, current_basis) = singular_spectrum_and_basis(current, rank_tolerance)?;
    if previous_basis.rank == 0 || current_basis.rank == 0 {
        return Ok(RhsSubspaceComparison {
            previous_rank: previous_basis.rank,
            current_rank: current_basis.rank,
            principal_cosines: Vec::new(),
            minimum_principal_angle_degrees: 90.0,
            maximum_principal_angle_degrees: 90.0,
        });
    }
    let cross = Mat::from_fn(previous_basis.rank, current_basis.rank, |i, j| {
        previous_basis.vectors[i]
            .iter()
            .zip(&current_basis.vectors[j])
            .map(|(a, b)| a * b)
            .sum::<f64>()
    });
    let mut principal_cosines = cross
        .singular_values()
        .map_err(|error| CoreError::LinearSolve(format!("principal-angle SVD failed: {error:?}")))?
        .into_iter()
        .map(|value| value.clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    principal_cosines.sort_by(|a, b| b.total_cmp(a));
    let mut angles = principal_cosines
        .iter()
        .map(|value| value.acos().to_degrees())
        .collect::<Vec<_>>();
    if previous_basis.rank != current_basis.rank {
        angles.push(90.0);
    }
    Ok(RhsSubspaceComparison {
        previous_rank: previous_basis.rank,
        current_rank: current_basis.rank,
        principal_cosines,
        minimum_principal_angle_degrees: angles.iter().copied().fold(90.0_f64, f64::min),
        maximum_principal_angle_degrees: angles.iter().copied().fold(0.0_f64, f64::max),
    })
}

pub fn recommend_common_w_backend(
    analysis: &RhsBatchAnalysis,
    risk: RhsTelemetryRisk,
) -> CommonWBackendChoice {
    if risk == RhsTelemetryRisk::High {
        return CommonWBackendChoice::IndependentRayonGmres;
    }
    if analysis.numerical_rank <= 1
        && analysis.energy_rank_99 <= 1
        && analysis.maximum_abs_pairwise_cosine >= 0.98
        && analysis.median_abs_pairwise_cosine >= 0.90
        && analysis.row_norm_spread <= 1.0e6
    {
        return CommonWBackendChoice::SeededSharedGmres;
    }
    if analysis.numerical_rank <= 4
        && analysis.energy_rank_999 <= 4
        && analysis.condition_proxy.is_none_or(|value| value <= 1.0e8)
    {
        CommonWBackendChoice::BlockGmres
    } else {
        CommonWBackendChoice::IndependentRayonGmres
    }
}

fn risk_from_proxies(stiffness: f64, nonnormality: f64) -> RhsTelemetryRisk {
    if stiffness >= 1.0e4 && nonnormality >= 0.8 {
        RhsTelemetryRisk::High
    } else if stiffness >= 1.0e4 || nonnormality >= 0.2 {
        RhsTelemetryRisk::Moderate
    } else {
        RhsTelemetryRisk::Low
    }
}

fn with_context(error: CoreError, context: impl AsRef<str>) -> CoreError {
    let message = format!("{}: {error}", context.as_ref());
    match error {
        CoreError::Dimension(_) => CoreError::Dimension(message),
        CoreError::InvalidInput(_) => CoreError::InvalidInput(message),
        CoreError::NonFinite(_) => CoreError::NonFinite(message),
        CoreError::LinearSolve(_) => CoreError::LinearSolve(message),
        CoreError::NonlinearSolve(_) => CoreError::NonlinearSolve(message),
        CoreError::Coefficients(_) => CoreError::Coefficients(message),
    }
}

struct RuntimeTelemetryCase {
    descriptor: HomotopyRhsTelemetryCase,
    problem: OdeProblem,
    y0: Vec<f64>,
}

fn complex_dahlquist_problem(
    dimension: usize,
    damping: f64,
    frequency: f64,
    t0: f64,
) -> CoreResult<(OdeProblem, Vec<f64>)> {
    if dimension == 0 || !dimension.is_multiple_of(2) {
        return Err(CoreError::InvalidInput(
            "complex Dahlquist telemetry dimension must be positive and even".into(),
        ));
    }
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for pair in 0..dimension / 2 {
            let i = 2 * pair;
            out[i] = -damping * y[i] - frequency * y[i + 1];
            out[i + 1] = frequency * y[i] - damping * y[i + 1];
        }
        Ok(())
    });
    let batch = Arc::new(move |_times: &[f64], states: &[Vec<f64>]| {
        let mut out = vec![vec![0.0; dimension]; states.len()];
        for (stage, state) in states.iter().enumerate() {
            for pair in 0..dimension / 2 {
                let i = 2 * pair;
                out[stage][i] = -damping * state[i] - frequency * state[i + 1];
                out[stage][i + 1] = frequency * state[i] - damping * state[i + 1];
            }
        }
        Ok(out)
    });
    let mut jacobian = DenseMatrix::zeros(dimension, dimension);
    for pair in 0..dimension / 2 {
        let i = 2 * pair;
        jacobian[(i, i)] = -damping;
        jacobian[(i, i + 1)] = -frequency;
        jacobian[(i + 1, i)] = frequency;
        jacobian[(i + 1, i + 1)] = -damping;
    }
    let jacobian_arc = Arc::new(jacobian);
    let jacobian_fn = {
        let matrix = jacobian_arc.clone();
        Arc::new(move |_t: f64, _y: &[f64]| Ok((*matrix).clone()))
    };
    let exact = Arc::new(move |t: f64| {
        let elapsed = t - t0;
        let envelope = (-damping * elapsed).exp();
        (0..dimension / 2)
            .flat_map(|pair| {
                let phase = frequency * elapsed + 0.031 * pair as f64;
                [envelope * phase.cos(), envelope * phase.sin()]
            })
            .collect::<Vec<_>>()
    });
    let y0 = exact(t0);
    Ok((
        OdeProblem::new(
            format!("complex-dahlquist-n{dimension}-d{damping}-w{frequency}"),
            dimension,
            rhs,
            Some(batch),
            Some(jacobian_fn),
            None,
            None,
            true,
            None,
            Some(exact),
        )?,
        y0,
    ))
}

fn build_cases(profile: HomotopyRhsTelemetryProfile) -> CoreResult<Vec<RuntimeTelemetryCase>> {
    let mut cases = Vec::new();
    let (affine, affine_y0, _, _) = constant_affine_mass_problem();
    cases.push(RuntimeTelemetryCase {
        descriptor: HomotopyRhsTelemetryCase {
            case_id: "affine-noncommuting-mass".into(),
            family: "affine-mass".into(),
            dimension: affine.dimension,
            stiffness_proxy: 5.0,
            nonnormality_proxy: 0.2,
            t: 0.0,
            h: 1.0e-2,
        },
        problem: affine,
        y0: affine_y0,
    });

    let (complex, complex_y0) = complex_dahlquist_problem(8, 120.0, 180.0, 0.0)?;
    cases.push(RuntimeTelemetryCase {
        descriptor: HomotopyRhsTelemetryCase {
            case_id: "complex-dahlquist-n8".into(),
            family: "complex-dahlquist".into(),
            dimension: 8,
            stiffness_proxy: 120.0,
            nonnormality_proxy: 0.0,
            t: 0.0,
            h: 1.0e-3,
        },
        problem: complex,
        y0: complex_y0,
    });

    let (pr, pr_y0) = prothero_robinson_problem(-1.0e4, 1.0e3, 0.2);
    cases.push(RuntimeTelemetryCase {
        descriptor: HomotopyRhsTelemetryCase {
            case_id: "pr-l1e4-m1e3".into(),
            family: "prothero-robinson".into(),
            dimension: 1,
            stiffness_proxy: 1.0e4,
            nonnormality_proxy: 0.0,
            t: 0.2,
            h: 1.0e-3,
        },
        problem: pr,
        y0: pr_y0,
    });

    let (mv, mv_y0) = manufactured_vector_problem(8, 1.0e4, 1.0e3, 0.9, 0.1)?;
    cases.push(RuntimeTelemetryCase {
        descriptor: HomotopyRhsTelemetryCase {
            case_id: "mv-n8-s1e4-m1e3-eta0.9".into(),
            family: "manufactured-vector".into(),
            dimension: 8,
            stiffness_proxy: 1.0e4,
            nonnormality_proxy: 0.9,
            t: 0.1,
            h: 1.0e-3,
        },
        problem: mv,
        y0: mv_y0,
    });

    if profile == HomotopyRhsTelemetryProfile::Canonical {
        for &(dimension, stiffness, nonlinearity, nonnormality, h) in &[
            (16, 1.0e2, 10.0, 0.0, 2.0e-3),
            (16, 1.0e4, 1.0e3, 0.2, 1.0e-3),
            (32, 1.0e4, 1.0e3, 0.9, 5.0e-4),
            (32, 1.0e6, 1.0e3, 0.9, 1.0e-5),
        ] {
            let (problem, y0) = manufactured_vector_problem(
                dimension,
                stiffness,
                nonlinearity,
                nonnormality,
                0.125,
            )?;
            cases.push(RuntimeTelemetryCase {
                descriptor: HomotopyRhsTelemetryCase {
                    case_id: format!(
                        "mv-n{dimension}-s{stiffness:.0e}-m{nonlinearity:.0e}-eta{nonnormality:.1}"
                    ),
                    family: "manufactured-vector".into(),
                    dimension,
                    stiffness_proxy: stiffness,
                    nonnormality_proxy: nonnormality,
                    t: 0.125,
                    h,
                },
                problem,
                y0,
            });
        }
        let (complex32, complex32_y0) = complex_dahlquist_problem(32, 1.0e4, 2.0e4, 0.0)?;
        cases.push(RuntimeTelemetryCase {
            descriptor: HomotopyRhsTelemetryCase {
                case_id: "complex-dahlquist-n32-stiff".into(),
                family: "complex-dahlquist".into(),
                dimension: 32,
                stiffness_proxy: 1.0e4,
                nonnormality_proxy: 0.0,
                t: 0.0,
                h: 1.0e-5,
            },
            problem: complex32,
            y0: complex32_y0,
        });
    }
    Ok(cases)
}

fn homotopy_configs(profile: HomotopyRhsTelemetryProfile) -> CoreResult<Vec<HomotopyPathConfig>> {
    let thetas: &[f64] = match profile {
        HomotopyRhsTelemetryProfile::Smoke => &[0.5],
        HomotopyRhsTelemetryProfile::Canonical => &[0.0, 0.5, 1.0],
    };
    let predictors: &[HomotopyPredictor] = match profile {
        HomotopyRhsTelemetryProfile::Smoke => &[HomotopyPredictor::Euler],
        HomotopyRhsTelemetryProfile::Canonical => {
            &[HomotopyPredictor::Euler, HomotopyPredictor::AdamsBashforth2]
        }
    };
    let mut configs = Vec::new();
    for &theta in thetas {
        for q in [0, 1, 2] {
            for &predictor in predictors {
                configs.push(HomotopyPathConfig::new(theta, q, 3, predictor, 0)?);
            }
        }
    }
    Ok(configs)
}

fn run_sabr_telemetry(
    case: &RuntimeTelemetryCase,
) -> CoreResult<(Vec<RhsBatchTelemetryRow>, WorkCounters)> {
    let mut counters = WorkCounters::default();
    let context = build_step_context(
        &case.problem,
        case.descriptor.t,
        &case.y0,
        case.descriptor.h,
        &mut counters,
    )?;
    let block = StructuredBlockSystem::new(&context);
    let shifted = context.shifted.explicit().ok_or_else(|| {
        CoreError::LinearSolve("SABR telemetry reference requires explicit common W".into())
    })?;
    let factor = LuFactorization::new(shifted)?;
    let config = SabrConfig {
        predictor: PredictorKind::Zero,
        max_iterations: 3,
        ..SabrConfig::default()
    };
    let history = StageHistory::default();
    let mut stages = history.predictor(
        case.descriptor.h,
        config.predictor,
        (context.coeffs.stages(), case.problem.dimension),
    );
    let risk = risk_from_proxies(
        case.descriptor.stiffness_proxy,
        case.descriptor.nonnormality_proxy,
    );
    let mut recorder = RhsTelemetryRecorder::new(
        RhsTelemetryContext {
            case_id: case.descriptor.case_id.clone(),
            method: "sabr".into(),
            q: None,
            theta: None,
            path_rounds: None,
            stiffness_proxy: case.descriptor.stiffness_proxy,
            nonnormality_proxy: case.descriptor.nonnormality_proxy,
            risk,
        },
        DEFAULT_RANK_TOLERANCE,
    )?;
    for iteration in 1..=config.max_iterations {
        let (rhs, _, _, _) = block.nonlinear_rhs(&stages, &mut counters)?;
        let transformed = factor.solve_rows(&rhs)?;
        recorder.record(
            RhsBatchIdentity {
                phase: "fixed-point-rhs".into(),
                round: iteration - 1,
                propagation_level: 0,
                iteration: Some(iteration),
            },
            &rhs,
            &transformed,
        )?;
        stages = block.forward_solve(&rhs, &mut counters)?.stages;
    }
    Ok((recorder.into_rows(), counters))
}

fn checksum(
    cases: &[HomotopyRhsTelemetryCase],
    rows: &[RhsBatchTelemetryRow],
    failures: &[RhsTelemetryFailure],
) -> String {
    let mut signature = String::new();
    for case in cases {
        signature.push_str(&format!(
            "case|{}|{}|{}|{:016x}|{:016x}|{:016x}|{:016x}\n",
            case.case_id,
            case.family,
            case.dimension,
            case.stiffness_proxy.to_bits(),
            case.nonnormality_proxy.to_bits(),
            case.t.to_bits(),
            case.h.to_bits(),
        ));
    }
    for row in rows {
        signature.push_str(&format!(
            "row|{}|{}|{}|{:?}|{:?}|{}|{}|{:?}|{}|{}|{:?}|{}|{}|{:016x}|{:016x}\n",
            row.case_id,
            row.method,
            row.phase,
            row.q,
            row.theta.map(f64::to_bits),
            row.round,
            row.propagation_level,
            row.iteration,
            row.raw_directional.numerical_rank,
            row.transformed_directional.numerical_rank,
            row.suggested_backend,
            row.raw_directional.energy_rank_99,
            row.transformed_directional.energy_rank_99,
            row.raw_directional.stable_rank.to_bits(),
            row.transformed_directional.stable_rank.to_bits(),
        ));
    }
    for failure in failures {
        signature.push_str(&format!(
            "failure|{}|{}|{:?}|{:?}|{:?}|{}|{}\n",
            failure.case_id,
            failure.method,
            failure.q,
            failure.theta.map(f64::to_bits),
            failure.predictor,
            failure.rows_preserved,
            failure.error,
        ));
    }
    sha256_hex(signature.as_bytes())
}

fn homotopy_outcomes_match(
    reference: &CoreResult<crate::HomotopyPathReport>,
    telemetry: &CoreResult<crate::HomotopyPathReport>,
) -> bool {
    match (reference, telemetry) {
        (Ok(left), Ok(right)) => left == right,
        (Err(left), Err(right)) => {
            std::mem::discriminant(left) == std::mem::discriminant(right)
                && left.to_string() == right.to_string()
        }
        _ => false,
    }
}

pub fn run_homotopy_rhs_telemetry_screen(
    profile: HomotopyRhsTelemetryProfile,
) -> CoreResult<HomotopyRhsTelemetryReport> {
    let runtime_cases = build_cases(profile)?;
    let configs = homotopy_configs(profile)?;
    let cases = runtime_cases
        .iter()
        .map(|case| case.descriptor.clone())
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    let mut reference_explicit_jacobian_builds = 0_u64;
    let mut reference_factorization_builds = 0_u64;
    let mut behavior_comparisons = 0_usize;
    let mut behavior_mismatches = 0_usize;
    for case in &runtime_cases {
        match run_sabr_telemetry(case) {
            Ok((sabr_rows, sabr_counters)) => {
                reference_explicit_jacobian_builds += sabr_counters.jacobian_builds;
                reference_factorization_builds += sabr_counters.direct_factorizations;
                rows.extend(sabr_rows);
            }
            Err(error) => failures.push(RhsTelemetryFailure {
                case_id: case.descriptor.case_id.clone(),
                method: "sabr".into(),
                q: None,
                theta: None,
                predictor: None,
                error: with_context(
                    error,
                    format!("SABR telemetry case {}", case.descriptor.case_id),
                )
                .to_string(),
                rows_preserved: 0,
            }),
        }
        let risk = risk_from_proxies(
            case.descriptor.stiffness_proxy,
            case.descriptor.nonnormality_proxy,
        );
        for config in &configs {
            let mut reference_counters = WorkCounters::default();
            let reference_context = build_step_context(
                &case.problem,
                case.descriptor.t,
                &case.y0,
                case.descriptor.h,
                &mut reference_counters,
            )?;
            let reference_block = StructuredBlockSystem::new(&reference_context);
            let reference_result = crate::homotopy::run_fixed_homotopy_path(
                &reference_block,
                config,
                &mut reference_counters,
            );

            let mut counters = WorkCounters::default();
            let context = build_step_context(
                &case.problem,
                case.descriptor.t,
                &case.y0,
                case.descriptor.h,
                &mut counters,
            )?;
            let block = StructuredBlockSystem::new(&context);
            let telemetry_context = RhsTelemetryContext {
                case_id: case.descriptor.case_id.clone(),
                method: "homotopy".into(),
                q: Some(config.q()),
                theta: Some(config.theta()),
                path_rounds: Some(config.path_rounds()),
                stiffness_proxy: case.descriptor.stiffness_proxy,
                nonnormality_proxy: case.descriptor.nonnormality_proxy,
                risk,
            };
            let mut recorder =
                RhsTelemetryRecorder::new(telemetry_context, DEFAULT_RANK_TOLERANCE)?;
            let path_result = crate::homotopy::run_fixed_homotopy_path_observed(
                &block,
                config,
                &mut counters,
                &mut recorder,
            );
            let telemetry_rows = recorder.into_rows();
            behavior_comparisons += 1;
            if !homotopy_outcomes_match(&reference_result, &path_result)
                || reference_counters != counters
            {
                behavior_mismatches += 1;
            }
            reference_explicit_jacobian_builds += counters.jacobian_builds;
            reference_factorization_builds += counters.direct_factorizations;
            let rows_preserved = telemetry_rows.len();
            rows.extend(telemetry_rows);
            if let Err(error) = path_result {
                failures.push(RhsTelemetryFailure {
                    case_id: case.descriptor.case_id.clone(),
                    method: "homotopy".into(),
                    q: Some(config.q()),
                    theta: Some(config.theta()),
                    predictor: Some(config.predictor()),
                    error: with_context(
                        error,
                        format!(
                            "homotopy telemetry case={} theta={} q={} predictor={:?}",
                            case.descriptor.case_id,
                            config.theta(),
                            config.q(),
                            config.predictor(),
                        ),
                    )
                    .to_string(),
                    rows_preserved,
                });
            }
        }
    }
    rows.sort_by(|left, right| {
        (
            &left.case_id,
            &left.method,
            left.theta.map(f64::to_bits),
            left.q,
            left.round,
            left.propagation_level,
            left.iteration,
            &left.phase,
        )
            .cmp(&(
                &right.case_id,
                &right.method,
                right.theta.map(f64::to_bits),
                right.q,
                right.round,
                right.propagation_level,
                right.iteration,
                &right.phase,
            ))
    });
    failures.sort_by(|left, right| {
        (
            &left.case_id,
            &left.method,
            left.theta.map(f64::to_bits),
            left.q,
            left.predictor.map(|value| format!("{value:?}")),
        )
            .cmp(&(
                &right.case_id,
                &right.method,
                right.theta.map(f64::to_bits),
                right.q,
                right.predictor.map(|value| format!("{value:?}")),
            ))
    });
    let summary = BackendRecommendationSummary {
        seeded_shared: rows
            .iter()
            .filter(|row| row.suggested_backend == CommonWBackendChoice::SeededSharedGmres)
            .count(),
        block_gmres: rows
            .iter()
            .filter(|row| row.suggested_backend == CommonWBackendChoice::BlockGmres)
            .count(),
        independent_rayon: rows
            .iter()
            .filter(|row| row.suggested_backend == CommonWBackendChoice::IndependentRayonGmres)
            .count(),
        raw_rank_one: rows
            .iter()
            .filter(|row| row.raw.numerical_rank == 1)
            .count(),
        transformed_rank_one: rows
            .iter()
            .filter(|row| row.transformed.numerical_rank == 1)
            .count(),
        transformed_directional_rank_one: rows
            .iter()
            .filter(|row| row.transformed_directional.numerical_rank == 1)
            .count(),
        high_risk_rows: rows
            .iter()
            .filter(|row| row.risk == RhsTelemetryRisk::High)
            .count(),
        failed_paths: failures.len(),
    };
    let scientific_checksum = checksum(&cases, &rows, &failures);
    Ok(HomotopyRhsTelemetryReport {
        schema: "rodas5p-homotopy-rhs-telemetry-v1",
        profile,
        cases,
        rows,
        failures,
        summary,
        rank_tolerance: DEFAULT_RANK_TOLERANCE,
        dispatcher_active: false,
        backend_recommendations_advisory: true,
        explicit_jacobian_builds_in_dispatch: 0,
        reference_explicit_jacobian_builds,
        reference_factorization_builds,
        behavior_comparisons,
        behavior_mismatches,
        solver_behavior_changed: behavior_mismatches > 0,
        scientific_checksum,
        verdict: "telemetry-only; backend selection is not yet active".into(),
    })
}
