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
pub enum BdfOrder {
    One,
    Two,
}

impl BdfOrder {
    pub fn value(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BdfConfig {
    pub order: BdfOrder,
    pub newton: NewtonConfig,
}

impl Default for BdfConfig {
    fn default() -> Self {
        Self {
            order: BdfOrder::Two,
            newton: NewtonConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BdfHistory {
    previous_state: Option<Vec<f64>>,
    previous_step: Option<f64>,
}

impl BdfHistory {
    pub fn previous_state(&self) -> Option<&[f64]> {
        self.previous_state.as_deref()
    }

    pub fn previous_step(&self) -> Option<f64> {
        self.previous_step
    }

    pub fn clear(&mut self) {
        self.previous_state = None;
        self.previous_step = None;
    }

    pub fn with_previous(previous_state: Vec<f64>, previous_step: f64) -> CoreResult<Self> {
        if previous_state.is_empty() || !previous_state.iter().all(|value| value.is_finite()) {
            return Err(CoreError::InvalidInput(
                "BDF previous state must be finite and nonempty".into(),
            ));
        }
        if !(previous_step > 0.0 && previous_step.is_finite()) {
            return Err(CoreError::InvalidInput(
                "BDF previous step must be finite and positive".into(),
            ));
        }
        Ok(Self {
            previous_state: Some(previous_state),
            previous_step: Some(previous_step),
        })
    }
}

#[derive(Clone, Debug)]
pub struct BdfStepReport {
    pub t_new: f64,
    pub y_new: Vec<f64>,
    pub requested_order: BdfOrder,
    pub applied_order: BdfOrder,
    pub used_startup: bool,
    pub step_ratio: Option<f64>,
    pub newton: NewtonReport,
}

#[derive(Clone, Debug)]
pub struct BdfIntegrationResult {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
    pub applied_orders: Vec<BdfOrder>,
    pub startup_steps: usize,
    pub counters: WorkCounters,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariableBdf2Coefficients {
    pub step_ratio: f64,
    pub a0: f64,
    pub a1: f64,
    pub a2: f64,
}

pub fn variable_bdf2_coefficients(
    current_step: f64,
    previous_step: f64,
) -> CoreResult<VariableBdf2Coefficients> {
    if !(current_step > 0.0
        && previous_step > 0.0
        && current_step.is_finite()
        && previous_step.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "BDF2 step sizes must be finite and positive".into(),
        ));
    }
    let step_ratio = current_step / previous_step;
    if !step_ratio.is_finite() || step_ratio <= 0.0 {
        return Err(CoreError::NonFinite("BDF2 step ratio is non-finite".into()));
    }
    let denominator = 1.0 + step_ratio;
    Ok(VariableBdf2Coefficients {
        step_ratio,
        a0: (1.0 + 2.0 * step_ratio) / denominator,
        a1: -(1.0 + step_ratio),
        a2: step_ratio * step_ratio / denominator,
    })
}

pub fn variable_bdf2_predictor(
    current: &[f64],
    previous: &[f64],
    step_ratio: f64,
) -> CoreResult<Vec<f64>> {
    if current.len() != previous.len() || current.is_empty() {
        return Err(CoreError::Dimension(
            "BDF2 predictor state shape mismatch".into(),
        ));
    }
    if !(step_ratio > 0.0 && step_ratio.is_finite()) {
        return Err(CoreError::InvalidInput(
            "BDF2 predictor step ratio must be finite and positive".into(),
        ));
    }
    if !current
        .iter()
        .chain(previous)
        .all(|value| value.is_finite())
    {
        return Err(CoreError::NonFinite(
            "BDF2 predictor state contains NaN/Inf".into(),
        ));
    }
    Ok(current
        .iter()
        .zip(previous)
        .map(|(now, old)| (1.0 + step_ratio) * now - step_ratio * old)
        .collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BdfStepMode {
    Fixed,
    Variable,
}

fn same_step(previous: f64, current: f64) -> bool {
    let scale = previous.abs().max(current.abs()).max(1.0);
    (previous - current).abs() <= 32.0 * f64::EPSILON * scale
}

fn scaled_mass_jacobian(
    mass: &DenseMatrix,
    jacobian: &DenseMatrix,
    mass_scale: f64,
    jacobian_scale: f64,
) -> CoreResult<DenseMatrix> {
    mass.scale(mass_scale).combine(jacobian, jacobian_scale)
}

pub fn bdf_step(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &BdfConfig,
    history: &mut BdfHistory,
    counters: &mut WorkCounters,
) -> CoreResult<BdfStepReport> {
    bdf_step_impl(
        problem,
        t,
        y,
        h,
        config,
        history,
        counters,
        BdfStepMode::Fixed,
    )
}

pub fn bdf_step_variable(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &BdfConfig,
    history: &mut BdfHistory,
    counters: &mut WorkCounters,
) -> CoreResult<BdfStepReport> {
    bdf_step_impl(
        problem,
        t,
        y,
        h,
        config,
        history,
        counters,
        BdfStepMode::Variable,
    )
}

#[allow(clippy::too_many_arguments)]
fn bdf_step_impl(
    problem: &OdeProblem,
    t: f64,
    y: &[f64],
    h: f64,
    config: &BdfConfig,
    history: &mut BdfHistory,
    counters: &mut WorkCounters,
    mode: BdfStepMode,
) -> CoreResult<BdfStepReport> {
    if y.len() != problem.dimension {
        return Err(CoreError::Dimension("BDF state shape mismatch".into()));
    }
    if !(h > 0.0 && h.is_finite() && t.is_finite()) {
        return Err(CoreError::InvalidInput(
            "BDF time and step must be finite with positive step".into(),
        ));
    }
    let previous = history.previous_state.as_ref().filter(|state| {
        state.len() == y.len()
            && history.previous_step.is_some_and(|previous_h| match mode {
                BdfStepMode::Fixed => same_step(previous_h, h),
                BdfStepMode::Variable => previous_h > 0.0 && previous_h.is_finite(),
            })
    });
    let (applied_order, used_startup) = match config.order {
        BdfOrder::One => (BdfOrder::One, false),
        BdfOrder::Two if previous.is_some() => (BdfOrder::Two, false),
        BdfOrder::Two => (BdfOrder::One, true),
    };
    let variable_coefficients = if applied_order == BdfOrder::Two && mode == BdfStepMode::Variable {
        Some(variable_bdf2_coefficients(
            h,
            history.previous_step.expect("BDF2 history validated"),
        )?)
    } else {
        None
    };
    let predictor = match (applied_order, previous) {
        (BdfOrder::Two, Some(previous_state)) => {
            let ratio = variable_coefficients.map_or(1.0, |coefficients| coefficients.step_ratio);
            variable_bdf2_predictor(y, previous_state, ratio)?
        }
        _ => y.to_vec(),
    };
    let mass = problem.mass_or_identity();
    let previous_state = previous.cloned();
    let t_new = t + h;
    let reference = predictor.clone();
    let newton = solve_dense_newton(
        &predictor,
        &reference,
        &config.newton,
        counters,
        |candidate, local_counters| {
            let rhs = problem.eval_rhs(t_new, candidate, local_counters)?;
            let (mass_next, mass_current, mass_previous, rhs_scale) = match applied_order {
                BdfOrder::One => (1.0, -1.0, 0.0, h),
                BdfOrder::Two if mode == BdfStepMode::Fixed => (3.0, -4.0, 1.0, 2.0 * h),
                BdfOrder::Two => {
                    let coefficients = variable_coefficients.expect("BDF2 coefficients validated");
                    (coefficients.a0, coefficients.a1, coefficients.a2, h)
                }
            };
            let combination: Vec<f64> = candidate
                .iter()
                .zip(y.iter())
                .enumerate()
                .map(|(index, (next, current))| {
                    let old = previous_state.as_ref().map_or(0.0, |state| state[index]);
                    mass_next * next + mass_current * current + mass_previous * old
                })
                .collect();
            let mut residual = mass.matvec(&combination)?;
            local_counters.mass_matvecs += 1;
            for (value, forcing) in residual.iter_mut().zip(rhs) {
                *value -= rhs_scale * forcing;
            }
            Ok(residual)
        },
        |candidate, local_counters| {
            let jacobian = problem.dense_jacobian(t_new, candidate, local_counters)?;
            match applied_order {
                BdfOrder::One => scaled_mass_jacobian(&mass, &jacobian, 1.0, -h),
                BdfOrder::Two if mode == BdfStepMode::Fixed => {
                    scaled_mass_jacobian(&mass, &jacobian, 3.0, -2.0 * h)
                }
                BdfOrder::Two => {
                    let coefficients = variable_coefficients.expect("BDF2 coefficients validated");
                    scaled_mass_jacobian(&mass, &jacobian, coefficients.a0, -h)
                }
            }
        },
    )?;

    history.previous_state = Some(y.to_vec());
    history.previous_step = Some(h);
    Ok(BdfStepReport {
        t_new,
        y_new: newton.x.clone(),
        requested_order: config.order,
        applied_order,
        used_startup,
        step_ratio: variable_coefficients.map(|coefficients| coefficients.step_ratio),
        newton,
    })
}

pub fn integrate_bdf_fixed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    config: &BdfConfig,
) -> CoreResult<BdfIntegrationResult> {
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || !(h > 0.0 && h.is_finite()) || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid fixed-step BDF integration input".into(),
        ));
    }
    let mut state = y0.to_vec();
    let mut history = BdfHistory::default();
    let mut counters = WorkCounters::default();
    let mut times = vec![t];
    let mut states = vec![state.clone()];
    let mut applied_orders = Vec::new();
    let mut startup_steps = 0;
    while t < tf - 10.0 * f64::EPSILON * tf.abs().max(1.0) {
        let step = h.min(tf - t);
        let report = bdf_step(
            problem,
            t,
            &state,
            step,
            config,
            &mut history,
            &mut counters,
        )?;
        if report.used_startup {
            startup_steps += 1;
        }
        applied_orders.push(report.applied_order);
        t = report.t_new;
        state = report.y_new;
        times.push(t);
        states.push(state.clone());
        counters.accepted_steps += 1;
    }
    Ok(BdfIntegrationResult {
        t: times,
        y: states,
        applied_orders,
        startup_steps,
        counters,
    })
}

pub fn integrate_bdf_fixed_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    config: &BdfConfig,
    output: &OutputSchedule,
) -> CoreResult<ObservedIntegrationResult> {
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || !(h > 0.0 && h.is_finite()) || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid fixed-step BDF integration input".into(),
        ));
    }
    let mut state = y0.to_vec();
    let mut history = BdfHistory::default();
    let mut counters = WorkCounters::default();
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut internal_steps = 0_usize;
    while t < tf - 10.0 * f64::EPSILON * tf.abs().max(1.0) {
        let (step, clipped) = collector.limit_step(t, h, tf)?;
        let report = bdf_step(
            problem,
            t,
            &state,
            step,
            config,
            &mut history,
            &mut counters,
        )?;
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

pub fn integrate_bdf_adaptive_observed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    config: &BdfConfig,
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<AdaptiveObservedIntegrationResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if y0.len() != problem.dimension || tf < t {
        return Err(CoreError::InvalidInput(
            "invalid adaptive BDF integration input".into(),
        ));
    }
    let mut state = y0.to_vec();
    let mut history = BdfHistory::default();
    let mut counters = WorkCounters::default();
    let mut controller = AdaptiveControllerState::default();
    let mut collector = OutputCollector::new(output, t_span, y0)?;
    let mut diagnostics = AdaptiveRunDiagnostics::default();
    let mut h = adaptive.initial_step.min(tf - t);
    let mut internal_steps = 0_usize;

    while t < tf && diagnostics.attempts < adaptive.max_attempts {
        h = h.min(adaptive.max_step).min(tf - t);
        if h < adaptive.min_step || 0.5 * h <= f64::MIN_POSITIVE {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        let mut coarse_history = history.clone();
        let mut fine_history = history.clone();
        let trial = (|| {
            let coarse = bdf_step_variable(
                problem,
                t,
                &state,
                trial_h,
                config,
                &mut coarse_history,
                &mut counters,
            )?;
            let half = 0.5 * trial_h;
            let fine_first = bdf_step_variable(
                problem,
                t,
                &state,
                half,
                config,
                &mut fine_history,
                &mut counters,
            )?;
            let fine_second = bdf_step_variable(
                problem,
                t + half,
                &fine_first.y_new,
                half,
                config,
                &mut fine_history,
                &mut counters,
            )?;
            Ok::<_, CoreError>((coarse, fine_first, fine_second))
        })();

        let (coarse, fine_first, fine_second) = match trial {
            Ok(value) => value,
            Err(
                CoreError::NonFinite(_) | CoreError::LinearSolve(_) | CoreError::NonlinearSolve(_),
            ) => {
                counters.rejected_steps += 1;
                diagnostics.record(trial_h, f64::INFINITY, 2, "bdf-step-doubling-failed", false);
                h = trial_h * adaptive.min_factor;
                continue;
            }
            Err(error) => return Err(error),
        };
        let method_order = if config.order == BdfOrder::One
            || coarse.applied_order == BdfOrder::One
            || fine_first.applied_order == BdfOrder::One
            || fine_second.applied_order == BdfOrder::One
        {
            1
        } else {
            2
        };
        let estimate = step_doubling_wrms_error(
            &state,
            &coarse.y_new,
            &fine_second.y_new,
            adaptive.atol,
            adaptive.rtol,
            method_order,
        )?;
        let estimator_id = if method_order == 1 {
            "bdf1-step-doubling"
        } else {
            "bdf2-step-doubling"
        };
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
            history = fine_history;
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
