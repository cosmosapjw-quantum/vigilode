use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use serde::Serialize;

use crate::output::OutputCollector;
use crate::{
    AdaptiveControllerState, AdaptiveObservedIntegrationResult, AdaptiveRunDiagnostics,
    AdaptiveStepConfig, NewtonConfig, NewtonReport, ObservedIntegrationResult, OdeProblem,
    OutputSchedule, solve_dense_newton, step_doubling_wrms_error,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RadauIiaStages {
    One,
    Three,
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
    pub newton: NewtonReport,
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

pub fn radau_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &RadauConfig,
    counters: &mut WorkCounters,
) -> CoreResult<RadauStepReport> {
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
            let increments = unflatten_stages(flat, stage_count, n);
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
            let mut stage_jacobians = Vec::with_capacity(stage_count);
            for i in 0..stage_count {
                stage_jacobians.push(problem.dense_jacobian(
                    t + c[i] * h,
                    &states[i],
                    local_counters,
                )?);
            }
            let mut block = DenseMatrix::zeros(unknowns, unknowns);
            for i in 0..stage_count {
                for j in 0..stage_count {
                    for row in 0..n {
                        for col in 0..n {
                            let diagonal_mass = if i == j { mass[(row, col)] } else { 0.0 };
                            block[(i * n + row, j * n + col)] =
                                diagonal_mass - h * a[(i, j)] * stage_jacobians[i][(row, col)];
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
    Ok(RadauStepReport {
        t_new: t + h,
        y_new,
        stage_increments,
        stages: config.stages,
        newton,
    })
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
    let method_order = config.stages.order();
    let estimator_id = match config.stages {
        RadauIiaStages::One => "radau-iia1-step-doubling",
        RadauIiaStages::Three => "radau-iia3-step-doubling",
    };

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step || 0.5 * h <= f64::MIN_POSITIVE {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        let trial = (|| {
            let coarse = radau_step(problem, t, &state, trial_h, config, &mut counters)?;
            let half = 0.5 * trial_h;
            let fine_first = radau_step(problem, t, &state, half, config, &mut counters)?;
            let fine_second = radau_step(
                problem,
                t + half,
                &fine_first.y_new,
                half,
                config,
                &mut counters,
            )?;
            Ok::<_, CoreError>((coarse, fine_second))
        })();

        let (coarse, fine_second) = match trial {
            Ok(value) => value,
            Err(
                CoreError::NonFinite(_) | CoreError::LinearSolve(_) | CoreError::NonlinearSolve(_),
            ) => {
                counters.rejected_steps += 1;
                diagnostics.record(
                    trial_h,
                    f64::INFINITY,
                    method_order + 1,
                    "radau-step-doubling-failed",
                    false,
                );
                h = trial_h * adaptive.min_factor;
                continue;
            }
            Err(error) => return Err(error),
        };
        let estimate = step_doubling_wrms_error(
            &state,
            &coarse.y_new,
            &fine_second.y_new,
            adaptive.atol,
            adaptive.rtol,
            method_order,
        )?;
        let accepted =
            estimate.error_norm <= 1.0 && fine_second.y_new.iter().all(|value| value.is_finite());
        diagnostics.record(
            trial_h,
            estimate.error_norm,
            estimate.estimator_order,
            estimator_id,
            accepted,
        );
        if accepted {
            t += trial_h;
            state = fine_second.y_new;
            collector.accept(t, &state, clipped)?;
            counters.accepted_steps += 2;
            internal_steps += 2;
            controller.record_acceptance(estimate.error_norm)?;
            h = trial_h
                * controller.propose_factor(
                    adaptive,
                    estimate.error_norm,
                    estimate.estimator_order,
                    true,
                )?;
        } else {
            counters.rejected_steps += 1;
            controller.record_rejection(estimate.error_norm)?;
            h = trial_h
                * controller.propose_factor(
                    adaptive,
                    estimate.error_norm.max(1.0e-16),
                    estimate.estimator_order,
                    false,
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
        ObservedIntegrationResult {
            t: Vec::new(),
            y: Vec::new(),
            success: false,
            message: "maximum step count or minimum step reached".into(),
            counters,
            internal_steps,
            output_clipped_steps: 0,
        }
    };
    Ok(AdaptiveObservedIntegrationResult {
        observed,
        diagnostics,
    })
}
