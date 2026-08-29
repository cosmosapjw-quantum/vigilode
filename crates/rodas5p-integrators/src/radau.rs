use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LuFactorization, WorkCounters, error_scale, wrms,
};
use serde::Serialize;

use crate::adaptive::record_adaptive_work_failure;
use crate::output::OutputCollector;
use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveObservedIntegrationResult,
    AdaptiveRunDiagnostics, AdaptiveStepConfig, NewtonConfig, NewtonReport,
    ObservedIntegrationResult, OdeProblem, OutputSchedule, adaptive_next_step_after_attempt,
    solve_dense_newton, step_doubling_wrms_error,
};

const RADAU1_ESTIMATOR_ID: &str = "radau-iia1-step-doubling";
const RADAU3_ESTIMATOR_ID: &str = "radau-iia3-scipy-1.17.0-embedded-order3";
const RADAU3_ESTIMATOR_ORDER: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RadauIiaStages {
    One,
    Three,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RadauStageSolveArchitecture {
    FullRealStageSystem,
    FullRealStageSystemTransformDeferred(RadauTransformLimitation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RadauTransformLimitation {
    /// The owned core exposes only real `DenseMatrix`/`LuFactorization`; a
    /// standard Radau real-plus-complex n-block transform therefore cannot be
    /// implemented without either a complex factorization API or a rigorously
    /// tested 2n real embedding.
    RealOnlyDenseLu,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadauIia3TransformOracle {
    pub mu_real: f64,
    pub mu_complex: (f64, f64),
    pub transform: [[f64; 3]; 3],
    pub inverse_transform: [[f64; 3]; 3],
}

/// Source-bound Radau IIA3 eigen-transform constants. They are retained as an
/// oracle for a future complex/2n backend, but are deliberately not advertised
/// as an active optimization while the core linear algebra remains real-only.
#[allow(clippy::excessive_precision)]
pub fn radau_iia3_transform_oracle() -> RadauIia3TransformOracle {
    // SciPy 1.17.0 scipy/integrate/_ivp/radau.py (BSD-3-Clause), commit
    // 8c75ae75176236f233824e9a0483c26a69e6dfec.
    RadauIia3TransformOracle {
        mu_real: 3.637_834_252_744_496,
        mu_complex: (2.681_082_873_627_752_3, -3.050_430_199_247_411),
        transform: [
            [
                0.094_438_762_488_975_24,
                -0.141_255_295_020_954_21,
                0.030_029_194_105_147_42,
            ],
            [
                0.250_213_122_965_333_3,
                0.204_129_352_293_799_94,
                -0.382_942_112_757_261_9,
            ],
            [1.0, 1.0, 0.0],
        ],
        inverse_transform: [
            [
                4.178_718_591_551_904,
                0.327_682_820_761_062_37,
                0.523_376_445_499_449_5,
            ],
            [
                -4.178_718_591_551_904,
                -0.327_682_820_761_062_37,
                0.476_623_554_500_550_44,
            ],
            [
                0.502_872_634_945_786_8,
                -2.571_926_949_855_605,
                0.596_039_204_828_224_9,
            ],
        ],
    }
}

impl RadauIiaStages {
    pub fn count(self) -> usize {
        match self {
            Self::One => 1,
            Self::Three => 3,
        }
    }

    pub fn order(self) -> usize {
        match self {
            Self::One => 1,
            Self::Three => 5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RadauConfig {
    pub stages: RadauIiaStages,
    pub newton: NewtonConfig,
}

impl Default for RadauConfig {
    fn default() -> Self {
        Self {
            stages: RadauIiaStages::Three,
            newton: NewtonConfig {
                atol: 1.0e-14,
                rtol: 1.0e-12,
                ..NewtonConfig::default()
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct RadauStepReport {
    pub t_new: f64,
    pub y_new: Vec<f64>,
    pub stage_increments: Vec<Vec<f64>>,
    pub stages: RadauIiaStages,
    pub stage_solve_architecture: RadauStageSolveArchitecture,
    pub newton: NewtonReport,
}

#[derive(Clone, Debug)]
struct RadauStepKernel {
    report: RadauStepReport,
    frozen_jacobian: Option<DenseMatrix>,
}

#[derive(Clone, Debug)]
pub(crate) struct AdaptiveRadauTrial {
    pub(crate) accepted_reports: Vec<RadauStepReport>,
    pub(crate) error_norm: f64,
    pub(crate) estimator_order: usize,
    pub(crate) estimator_id: &'static str,
}

impl AdaptiveRadauTrial {
    pub(crate) fn y_new(&self) -> &[f64] {
        self.accepted_reports
            .last()
            .expect("adaptive Radau trial has at least one accepted report")
            .y_new
            .as_slice()
    }

    pub(crate) fn accepted_internal_steps(&self) -> usize {
        self.accepted_reports.len()
    }
}

#[derive(Clone, Debug)]
pub struct RadauIntegrationResult {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
    pub counters: WorkCounters,
    pub steps: usize,
}

pub fn radau_iia3_tableau() -> (DenseMatrix, Vec<f64>, Vec<f64>) {
    let sqrt6 = 6.0_f64.sqrt();
    let a = DenseMatrix::from_rows(&[
        &[
            (88.0 - 7.0 * sqrt6) / 360.0,
            (296.0 - 169.0 * sqrt6) / 1800.0,
            (-2.0 + 3.0 * sqrt6) / 225.0,
        ],
        &[
            (296.0 + 169.0 * sqrt6) / 1800.0,
            (88.0 + 7.0 * sqrt6) / 360.0,
            (-2.0 - 3.0 * sqrt6) / 225.0,
        ],
        &[(16.0 - sqrt6) / 36.0, (16.0 + sqrt6) / 36.0, 1.0 / 9.0],
    ])
    .expect("exact Radau IIA3 tableau has valid dimensions");
    let b = vec![(16.0 - sqrt6) / 36.0, (16.0 + sqrt6) / 36.0, 1.0 / 9.0];
    let c = vec![(4.0 - sqrt6) / 10.0, (4.0 + sqrt6) / 10.0, 1.0];
    (a, b, c)
}

fn tableau(stages: RadauIiaStages) -> (DenseMatrix, Vec<f64>, Vec<f64>) {
    match stages {
        RadauIiaStages::One => (
            DenseMatrix::from_rows(&[&[1.0]]).expect("1x1 tableau"),
            vec![1.0],
            vec![1.0],
        ),
        RadauIiaStages::Three => radau_iia3_tableau(),
    }
}

fn unflatten_stages(flat: &[f64], stages: usize, dimension: usize) -> Vec<Vec<f64>> {
    (0..stages)
        .map(|stage| flat[stage * dimension..(stage + 1) * dimension].to_vec())
        .collect()
}

fn radau_step_kernel(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &RadauConfig,
    counters: &mut WorkCounters,
) -> CoreResult<RadauStepKernel> {
    if y.len() != problem.dimension {
        return Err(CoreError::Dimension("Radau state shape mismatch".into()));
    }
    if !(h > 0.0 && h.is_finite() && t.is_finite()) {
        return Err(CoreError::InvalidInput(
            "Radau time and step must be finite with positive step".into(),
        ));
    }
    let (a, b, c) = tableau(config.stages);
    let stage_count = config.stages.count();
    let n = problem.dimension;
    let unknowns = stage_count * n;
    let initial = vec![0.0; unknowns];
    let reference = initial.clone();
    let mass = problem.mass_or_identity();
    let mut frozen_jacobian = None;

    let newton = solve_dense_newton(
        &initial,
        &reference,
        &config.newton,
        counters,
        |flat, local_counters| {
            let increments = unflatten_stages(flat, stage_count, n);
            let times: Vec<f64> = c.iter().map(|node| t + node * h).collect();
            let states: Vec<Vec<f64>> = (0..stage_count)
                .map(|i| {
                    let mut state = y.to_vec();
                    for j in 0..stage_count {
                        let coefficient = a[(i, j)];
                        for component in 0..n {
                            state[component] += coefficient * increments[j][component];
                        }
                    }
                    state
                })
                .collect();
            let rhs = problem.eval_rhs_batch(&times, &states, local_counters)?;
            let mut residual = vec![0.0; unknowns];
            for i in 0..stage_count {
                let mass_increment = mass.matvec(&increments[i])?;
                local_counters.mass_matvecs += 1;
                for component in 0..n {
                    residual[i * n + component] = mass_increment[component] - h * rhs[i][component];
                }
            }
            Ok(residual)
        },
        |flat, local_counters| {
            // Modified Newton freezes one Jacobian between refreshes.  When
            // the nonlinear driver requests a refresh, rebuild at the latest
            // stiffly-accurate stage state instead of returning the original
            // matrix again.
            let increments = unflatten_stages(flat, stage_count, n);
            let representative_stage = stage_count - 1;
            let mut representative_state = y.to_vec();
            for j in 0..stage_count {
                let coefficient = a[(representative_stage, j)];
                for component in 0..n {
                    representative_state[component] += coefficient * increments[j][component];
                }
            }
            frozen_jacobian = Some(problem.dense_jacobian(
                t + c[representative_stage] * h,
                &representative_state,
                local_counters,
            )?);
            let jacobian = frozen_jacobian
                .as_ref()
                .expect("Radau frozen Jacobian initialized");
            let mut block = DenseMatrix::zeros(unknowns, unknowns);
            for i in 0..stage_count {
                for j in 0..stage_count {
                    for row in 0..n {
                        for col in 0..n {
                            let diagonal_mass = if i == j { mass[(row, col)] } else { 0.0 };
                            block[(i * n + row, j * n + col)] =
                                diagonal_mass - h * a[(i, j)] * jacobian[(row, col)];
                        }
                    }
                }
            }
            Ok(block)
        },
    )?;

    let stage_increments = unflatten_stages(&newton.x, stage_count, n);
    let mut y_new = y.to_vec();
    for i in 0..stage_count {
        for component in 0..n {
            y_new[component] += b[i] * stage_increments[i][component];
        }
    }
    if !y_new.iter().all(|value| value.is_finite()) {
        return Err(CoreError::NonFinite(
            "Radau endpoint contains NaN/Inf".into(),
        ));
    }
    Ok(RadauStepKernel {
        report: RadauStepReport {
            t_new: t + h,
            y_new,
            stage_increments,
            stages: config.stages,
            stage_solve_architecture: match config.stages {
                RadauIiaStages::One => RadauStageSolveArchitecture::FullRealStageSystem,
                RadauIiaStages::Three => {
                    RadauStageSolveArchitecture::FullRealStageSystemTransformDeferred(
                        RadauTransformLimitation::RealOnlyDenseLu,
                    )
                }
            },
            newton,
        },
        frozen_jacobian,
    })
}

pub fn radau_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &RadauConfig,
    counters: &mut WorkCounters,
) -> CoreResult<RadauStepReport> {
    Ok(radau_step_kernel(problem, t, y, h, config, counters)?.report)
}

#[allow(clippy::too_many_arguments)]
fn radau_iia3_embedded_error(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    kernel: &RadauStepKernel,
    adaptive: &AdaptiveStepConfig,
    previous_local_rejection: bool,
    counters: &mut WorkCounters,
) -> CoreResult<f64> {
    if kernel.report.stages != RadauIiaStages::Three
        || kernel.report.stage_increments.len() != 3
        || kernel
            .report
            .stage_increments
            .iter()
            .any(|increment| increment.len() != problem.dimension)
    {
        return Err(CoreError::Dimension(
            "Radau IIA3 embedded estimator stage shape mismatch".into(),
        ));
    }

    // Source-bound to SciPy 1.17.0, scipy/integrate/_ivp/radau.py at
    // 8c75ae75176236f233824e9a0483c26a69e6dfec (BSD-3-Clause).  VigilODE's
    // stage unknowns are K, so first form SciPy's stage displacements Z=A*K.
    let (a, _, _) = radau_iia3_tableau();
    let n = problem.dimension;
    let mut stage_displacements = vec![vec![0.0; n]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for (displacement, increment) in stage_displacements[i]
                .iter_mut()
                .zip(&kernel.report.stage_increments[j])
            {
                *displacement += a[(i, j)] * increment;
            }
        }
    }
    let sqrt6 = 6.0_f64.sqrt();
    let e = [
        (-13.0 - 7.0 * sqrt6) / 3.0,
        (-13.0 + 7.0 * sqrt6) / 3.0,
        -1.0 / 3.0,
    ];
    let mut v = vec![0.0; n];
    for (weight, displacement) in e.iter().zip(&stage_displacements) {
        for (value, stage_value) in v.iter_mut().zip(displacement) {
            *value += weight * stage_value / h;
        }
    }

    let jacobian = match &kernel.frozen_jacobian {
        Some(jacobian) => jacobian.clone(),
        None => problem.dense_jacobian(t, y, counters)?,
    };
    let mass = problem.mass_or_identity();
    counters.mass_matvecs += 1;
    let mass_v = mass.matvec(&v)?;
    let f_n = problem.eval_rhs(t, y, counters)?;
    let mu = 3.0 + 3.0_f64.powf(2.0 / 3.0) - 3.0_f64.powf(1.0 / 3.0);
    let error_operator = mass.scale(mu / h).combine(&jacobian, -1.0)?;
    counters.direct_factorizations += 1;
    let factor = LuFactorization::new(&error_operator)?;

    let first_rhs = f_n
        .iter()
        .zip(&mass_v)
        .map(|(forcing, extension)| forcing + extension)
        .collect::<Vec<_>>();
    counters.direct_solve_calls += 1;
    counters.linear_solves += 1;
    let mut error = factor.solve(&first_rhs)?;
    let scale = error_scale(y, &kernel.report.y_new, &[adaptive.atol], adaptive.rtol)?;
    let mut error_norm = wrms(&error, &scale)?;

    if previous_local_rejection && error_norm > 1.0 {
        let perturbed = y
            .iter()
            .zip(&error)
            .map(|(state, correction)| state + correction)
            .collect::<Vec<_>>();
        let perturbed_rhs = problem.eval_rhs(t, &perturbed, counters)?;
        let corrected_rhs = perturbed_rhs
            .iter()
            .zip(&mass_v)
            .map(|(forcing, extension)| forcing + extension)
            .collect::<Vec<_>>();
        counters.direct_solve_calls += 1;
        counters.linear_solves += 1;
        error = factor.solve(&corrected_rhs)?;
        error_norm = wrms(&error, &scale)?;
    }
    Ok(error_norm)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn adaptive_radau_trial(
    problem: &OdeProblem,
    t: f64,
    state: &[f64],
    h: f64,
    config: &RadauConfig,
    adaptive: &AdaptiveStepConfig,
    previous_local_rejection: bool,
    counters: &mut WorkCounters,
) -> CoreResult<AdaptiveRadauTrial> {
    match config.stages {
        RadauIiaStages::One => {
            let coarse = radau_step(problem, t, state, h, config, counters)?;
            let half = 0.5 * h;
            let fine_first = radau_step(problem, t, state, half, config, counters)?;
            let fine_second =
                radau_step(problem, t + half, &fine_first.y_new, half, config, counters)?;
            let estimate = step_doubling_wrms_error(
                state,
                &coarse.y_new,
                &fine_second.y_new,
                adaptive.atol,
                adaptive.rtol,
                1,
            )?;
            Ok(AdaptiveRadauTrial {
                accepted_reports: vec![fine_first, fine_second],
                error_norm: estimate.error_norm,
                estimator_order: estimate.estimator_order,
                estimator_id: RADAU1_ESTIMATOR_ID,
            })
        }
        RadauIiaStages::Three => {
            let kernel = radau_step_kernel(problem, t, state, h, config, counters)?;
            let error_norm = radau_iia3_embedded_error(
                problem,
                t,
                state,
                h,
                &kernel,
                adaptive,
                previous_local_rejection,
                counters,
            )?;
            Ok(AdaptiveRadauTrial {
                accepted_reports: vec![kernel.report],
                error_norm,
                estimator_order: RADAU3_ESTIMATOR_ORDER,
                estimator_id: RADAU3_ESTIMATOR_ID,
            })
        }
    }
}

fn adaptive_failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        _ => None,
    }
}

pub fn integrate_radau_fixed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    config: &RadauConfig,
) -> CoreResult<RadauIntegrationResult> {
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || !(h > 0.0 && h.is_finite()) || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid fixed-step Radau integration input".into(),
        ));
    }
    let mut state = y0.to_vec();
    let mut counters = WorkCounters::default();
    let mut times = vec![t];
    let mut states = vec![state.clone()];
    let mut steps = 0;
    while t < tf - 10.0 * f64::EPSILON * tf.abs().max(1.0) {
        let step = h.min(tf - t);
        let report = radau_step(problem, t, &state, step, config, &mut counters)?;
        t = report.t_new;
        state = report.y_new;
        times.push(t);
        states.push(state.clone());
        counters.accepted_steps += 1;
        steps += 1;
    }
    Ok(RadauIntegrationResult {
        t: times,
        y: states,
        counters,
        steps,
    })
}

pub fn integrate_radau_fixed_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    config: &RadauConfig,
    output: &OutputSchedule,
) -> CoreResult<ObservedIntegrationResult> {
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || !(h > 0.0 && h.is_finite()) || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid fixed-step Radau integration input".into(),
        ));
    }
    let mut state = y0.to_vec();
    let mut counters = WorkCounters::default();
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut internal_steps = 0_usize;
    while t < tf - 10.0 * f64::EPSILON * tf.abs().max(1.0) {
        let (step, clipped) = collector.limit_step(t, h, tf)?;
        let report = radau_step(problem, t, &state, step, config, &mut counters)?;
        t = report.t_new;
        state = report.y_new;
        collector.accept(t, &state, clipped)?;
        counters.accepted_steps += 1;
        internal_steps += 1;
    }
    let (times, states, output_clipped_steps) = collector.finish()?;
    Ok(ObservedIntegrationResult {
        t: times,
        y: states,
        success: true,
        message: "success".into(),
        counters,
        internal_steps,
        output_clipped_steps,
    })
}

pub fn integrate_radau_adaptive_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    config: &RadauConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid adaptive Radau integration input".into(),
        ));
    }
    let mut state = y0.to_vec();
    let mut counters = WorkCounters::default();
    let mut controller = AdaptiveControllerState::default();
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut internal_steps = 0_usize;
    let (estimator_order, estimator_id) = match config.stages {
        RadauIiaStages::One => (2, RADAU1_ESTIMATOR_ID),
        RadauIiaStages::Three => (RADAU3_ESTIMATOR_ORDER, RADAU3_ESTIMATOR_ID),
    };
    let mut previous_local_rejection = false;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step || 0.5 * h <= f64::MIN_POSITIVE {
            break;
        }
        let requested_h = h;
        let (trial_h, clipped) = collector.limit_step(t, requested_h, tf)?;
        let trial = adaptive_radau_trial(
            problem,
            t,
            &state,
            trial_h,
            config,
            adaptive,
            previous_local_rejection,
            &mut counters,
        );
        let trial = match trial {
            Ok(value) => value,
            Err(error) => {
                if let Some(failure) = adaptive_failure_kind(&error) {
                    counters.rejected_steps += 1;
                    record_adaptive_work_failure(&mut counters, failure);
                    diagnostics.record_with_failure(
                        trial_h,
                        f64::INFINITY,
                        estimator_order,
                        estimator_id,
                        false,
                        Some(failure),
                    );
                    h = adaptive_next_step_after_attempt(
                        &mut controller,
                        adaptive,
                        requested_h,
                        trial_h,
                        f64::INFINITY,
                        estimator_order,
                        false,
                        clipped,
                    )?;
                    continue;
                }
                return Err(error);
            }
        };
        let accepted =
            trial.error_norm <= 1.0 && trial.y_new().iter().all(|value| value.is_finite());
        diagnostics.record(
            trial_h,
            trial.error_norm,
            trial.estimator_order,
            trial.estimator_id,
            accepted,
        );
        if accepted {
            let accepted_internal_steps = trial.accepted_internal_steps();
            t += trial_h;
            state = trial.y_new().to_vec();
            collector.accept(t, &state, clipped)?;
            counters.accepted_steps += accepted_internal_steps as u64;
            internal_steps += accepted_internal_steps;
            previous_local_rejection = false;
            h = adaptive_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                trial.error_norm,
                trial.estimator_order,
                true,
                clipped,
            )?;
        } else {
            counters.rejected_steps += 1;
            record_adaptive_work_failure(&mut counters, AdaptiveFailureKind::LocalError);
            previous_local_rejection = true;
            h = adaptive_next_step_after_attempt(
                &mut controller,
                adaptive,
                requested_h,
                trial_h,
                trial.error_norm,
                trial.estimator_order,
                false,
                clipped,
            )?;
        }
    }

    let success = t >= tf - 10.0 * f64::EPSILON * tf.abs().max(1.0);
    let observed = if success {
        let (times, states, output_clipped_steps) = collector.finish()?;
        ObservedIntegrationResult {
            t: times,
            y: states,
            success: true,
            message: "success".into(),
            counters,
            internal_steps,
            output_clipped_steps,
        }
    } else {
        let (times, states, output_clipped_steps) = collector.finish_partial();
        ObservedIntegrationResult {
            t: times,
            y: states,
            success: false,
            message: "maximum step count or minimum step reached".into(),
            counters,
            internal_steps,
            output_clipped_steps,
        }
    };
    Ok(AdaptiveObservedIntegrationResult {
        observed,
        diagnostics,
    })
}
