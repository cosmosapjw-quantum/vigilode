use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters, error_scale, wrms};
use serde::Serialize;

use crate::adaptive::record_adaptive_work_failure;
use crate::output::OutputCollector;
use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveObservedIntegrationResult,
    AdaptiveRunDiagnostics, AdaptiveStepConfig, NewtonConfig, NewtonReport,
    ObservedIntegrationResult, OdeProblem, OutputSchedule, adaptive_next_step_after_attempt,
    solve_dense_newton, step_doubling_wrms_error,
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
    older_state: Option<Vec<f64>>,
    older_step: Option<f64>,
}

impl BdfHistory {
    pub fn previous_state(&self) -> Option<&[f64]> {
        self.previous_state.as_deref()
    }

    pub fn previous_step(&self) -> Option<f64> {
        self.previous_step
    }

    pub fn older_state(&self) -> Option<&[f64]> {
        self.older_state.as_deref()
    }

    pub fn older_step(&self) -> Option<f64> {
        self.older_step
    }

    pub fn clear(&mut self) {
        self.previous_state = None;
        self.previous_step = None;
        self.older_state = None;
        self.older_step = None;
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
            older_state: None,
            older_step: None,
        })
    }

    /// Construct a complete two-step history for numerical-oracle and restart
    /// boundaries.  `previous_state` is one step behind the current state and
    /// `older_state` is the state preceding it; each step is the interval from
    /// that state to its successor.
    pub fn with_two_previous(
        previous_state: Vec<f64>,
        previous_step: f64,
        older_state: Vec<f64>,
        older_step: f64,
    ) -> CoreResult<Self> {
        if previous_state.is_empty()
            || previous_state.len() != older_state.len()
            || !previous_state
                .iter()
                .chain(&older_state)
                .all(|value| value.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "BDF two-step history states must be finite, nonempty, and shape-compatible".into(),
            ));
        }
        if ![previous_step, older_step]
            .iter()
            .all(|step| *step > 0.0 && step.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "BDF two-step history steps must be finite and positive".into(),
            ));
        }
        Ok(Self {
            previous_state: Some(previous_state),
            previous_step: Some(previous_step),
            older_state: Some(older_state),
            older_step: Some(older_step),
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
    /// Polynomial extrapolation used as the Newton initial value and by the
    /// step-geometry-derived pure-BDF predictor/corrector LTE estimator.
    pub predictor: Vec<f64>,
    /// State immediately before `y` in the accepted BDF history, retained for
    /// dense interpolation of this interval.
    pub interpolation_previous_state: Option<Vec<f64>>,
    pub interpolation_previous_step: Option<f64>,
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

/// Largest adjacent-step growth ratio for zero-stable variable-step BDF2.
///
/// The parasitic root is `r^2 / (1 + 2r)`, so root stability requires
/// `r <= 1 + sqrt(2)`.  The variable-step kernel takes one BDF1 restart when
/// a requested step exceeds this boundary instead of applying unstable BDF2
/// coefficients.
pub const BDF2_ZERO_STABILITY_RATIO_MAX: f64 = 2.414_213_562_373_095;

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

/// Leading-order local-error magnitude per BDF1 predictor/corrector
/// difference for the actual two-interval geometry.
///
/// The linear history predictor has quadratic defect `h(h+a)y''/2`, while
/// backward Euler has one-step local error `-h^2 y''/2`.  Their computed
/// difference therefore yields the magnitude factor `h / (a + 2h)`.
pub fn bdf1_predictor_correction_lte_factor(
    current_step: f64,
    previous_step: f64,
) -> CoreResult<f64> {
    if ![current_step, previous_step]
        .iter()
        .all(|step| *step > 0.0 && step.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "BDF1 estimator step geometry must be finite and positive".into(),
        ));
    }
    let ratio = current_step / previous_step;
    if !(ratio > 0.0 && ratio.is_finite()) {
        return Err(CoreError::NonFinite(
            "BDF1 estimator step ratio is non-finite".into(),
        ));
    }
    Ok(ratio / (1.0 + 2.0 * ratio))
}

/// Leading-order local-error magnitude per BDF2 predictor/corrector
/// difference for the actual three-interval geometry.
///
/// Let `h`, `a`, and `b` denote the current, previous, and older step sizes.
/// The quadratic history predictor has cubic defect
/// `h(h+a)(h+a+b)y'''/6`.  Variable-step BDF2 has one-step local error
/// `-h^2(h+a)y'''/(6*a0)`, where `a0=(a+2h)/(a+h)`.  Eliminating `y'''`
/// between that error and the computed corrector-minus-predictor difference
/// gives the positive magnitude factor returned here:
///
/// `h / (a0*(h+a+b) + h)`.
pub fn bdf2_predictor_correction_lte_factor(
    current_step: f64,
    previous_step: f64,
    older_step: f64,
) -> CoreResult<f64> {
    if ![current_step, previous_step, older_step]
        .iter()
        .all(|step| *step > 0.0 && step.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "BDF2 estimator step geometry must be finite and positive".into(),
        ));
    }
    let ratio = current_step / previous_step;
    let older_ratio = older_step / previous_step;
    if !(ratio > 0.0 && ratio.is_finite() && older_ratio > 0.0 && older_ratio.is_finite()) {
        return Err(CoreError::NonFinite(
            "BDF2 estimator step geometry ratio is non-finite".into(),
        ));
    }
    let a0 = (1.0 + 2.0 * ratio) / (1.0 + ratio);
    let denominator = a0 * (ratio + 1.0 + older_ratio) + ratio;
    let factor = ratio / denominator;
    if !(factor > 0.0 && factor.is_finite()) {
        return Err(CoreError::NonFinite(
            "BDF2 estimator factor is non-finite".into(),
        ));
    }
    Ok(factor)
}

fn variable_quadratic_predictor(
    current: &[f64],
    previous: &[f64],
    older: &[f64],
    current_step: f64,
    previous_step: f64,
    older_step: f64,
) -> CoreResult<Vec<f64>> {
    if current.len() != previous.len() || current.len() != older.len() || current.is_empty() {
        return Err(CoreError::Dimension(
            "BDF quadratic predictor state shape mismatch".into(),
        ));
    }
    if !current
        .iter()
        .chain(previous)
        .chain(older)
        .all(|value| value.is_finite())
        || ![current_step, previous_step, older_step]
            .iter()
            .all(|step| *step > 0.0 && step.is_finite())
    {
        return Err(CoreError::NonFinite(
            "BDF quadratic predictor inputs must be finite".into(),
        ));
    }

    // Lagrange extrapolation through y_n, y_{n-1}, y_{n-2}.  For equal
    // steps this is the standard order-two backward-difference predictor
    // y_{n+1}^{(0)} = y_n + Delta y_n + Delta^2 y_n.
    let h = current_step;
    let a = previous_step;
    let b = older_step;
    let l0 = (h + a) * (h + a + b) / (a * (a + b));
    let l1 = -h * (h + a + b) / (a * b);
    let l2 = h * (h + a) / (b * (a + b));
    Ok(current
        .iter()
        .zip(previous)
        .zip(older)
        .map(|((now, old), oldest)| l0 * now + l1 * old + l2 * oldest)
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

fn requires_bdf2_stability_restart(current_step: f64, previous_step: f64) -> bool {
    let ratio = current_step / previous_step;
    !ratio.is_finite() || ratio > BDF2_ZERO_STABILITY_RATIO_MAX
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
    let stability_restart = config.order == BdfOrder::Two
        && mode == BdfStepMode::Variable
        && previous.is_some()
        && requires_bdf2_stability_restart(
            h,
            history.previous_step.expect("BDF2 history validated"),
        );
    let (applied_order, used_startup) = match config.order {
        BdfOrder::One => (BdfOrder::One, false),
        BdfOrder::Two if previous.is_some() && !stability_restart => (BdfOrder::Two, false),
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
    let predictor = match (
        applied_order,
        previous,
        history.older_state.as_ref(),
        history.older_step,
    ) {
        (BdfOrder::Two, Some(previous_state), Some(older_state), Some(older_h)) => {
            let previous_h = history.previous_step.expect("BDF2 history validated");
            variable_quadratic_predictor(y, previous_state, older_state, h, previous_h, older_h)?
        }
        (BdfOrder::Two, Some(previous_state), _, _) => {
            let ratio = variable_coefficients.map_or(1.0, |coefficients| coefficients.step_ratio);
            variable_bdf2_predictor(y, previous_state, ratio)?
        }
        (BdfOrder::One, Some(previous_state), _, _) => {
            let ratio = h / history.previous_step.expect("BDF1 history validated");
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

    let interpolation_previous_state = history.previous_state.clone();
    let interpolation_previous_step = history.previous_step;
    if stability_restart {
        // A BDF1 restart intentionally discards the pre-jump polynomial.  The
        // accepted interval becomes the sole new-history interval, and the
        // following step may rebuild a zero-stable BDF2 history from it.
        history.older_state = None;
        history.older_step = None;
    } else {
        history.older_state = history.previous_state.take();
        history.older_step = history.previous_step.take();
    }
    history.previous_state = Some(y.to_vec());
    history.previous_step = Some(h);
    Ok(BdfStepReport {
        t_new,
        y_new: newton.x.clone(),
        requested_order: config.order,
        applied_order,
        used_startup,
        step_ratio: variable_coefficients.map(|coefficients| coefficients.step_ratio),
        predictor,
        interpolation_previous_state,
        interpolation_previous_step,
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

const BDF_STARTUP_ESTIMATOR_ID: &str = "bdf-explicit-startup-step-doubling";
const BDF1_PREDICTOR_ESTIMATOR_ID: &str = "bdf1-pure-bdf-backward-difference-lte";
const BDF2_PREDICTOR_ESTIMATOR_ID: &str = "bdf2-pure-bdf-backward-difference-lte";

#[derive(Clone, Debug)]
pub(crate) struct AdaptiveBdfTrial {
    pub(crate) accepted_reports: Vec<BdfStepReport>,
    pub(crate) accepted_history: BdfHistory,
    pub(crate) error_norm: f64,
    pub(crate) estimator_order: usize,
    pub(crate) estimator_id: &'static str,
}

impl AdaptiveBdfTrial {
    pub(crate) fn y_new(&self) -> &[f64] {
        self.accepted_reports
            .last()
            .expect("adaptive BDF trial has at least one accepted report")
            .y_new
            .as_slice()
    }

    pub(crate) fn accepted_internal_steps(&self) -> usize {
        self.accepted_reports.len()
    }
}

fn bdf_predictor_estimator_ready(history: &BdfHistory, order: BdfOrder) -> bool {
    match order {
        BdfOrder::One => history.previous_state.is_some() && history.previous_step.is_some(),
        BdfOrder::Two => {
            history.previous_state.is_some()
                && history.previous_step.is_some()
                && history.older_state.is_some()
                && history.older_step.is_some()
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn adaptive_bdf_trial(
    problem: &OdeProblem,
    t: f64,
    state: &[f64],
    h: f64,
    config: &BdfConfig,
    adaptive: &AdaptiveStepConfig,
    history: &BdfHistory,
    counters: &mut WorkCounters,
) -> CoreResult<AdaptiveBdfTrial> {
    if !bdf_predictor_estimator_ready(history, config.order) {
        // Startup is explicit and finite: two advancing half steps are kept,
        // while the one coarse solve exists solely to certify their error.
        // Once those half steps are accepted, both BDF1 and BDF2 have enough
        // backward differences for the single-solve steady-state estimator.
        let mut coarse_history = history.clone();
        let mut fine_history = history.clone();
        let coarse =
            bdf_step_variable(problem, t, state, h, config, &mut coarse_history, counters)?;
        let half = 0.5 * h;
        let fine_first =
            bdf_step_variable(problem, t, state, half, config, &mut fine_history, counters)?;
        let fine_second = bdf_step_variable(
            problem,
            t + half,
            &fine_first.y_new,
            half,
            config,
            &mut fine_history,
            counters,
        )?;
        let method_order = coarse
            .applied_order
            .value()
            .min(fine_first.applied_order.value())
            .min(fine_second.applied_order.value());
        let estimate = step_doubling_wrms_error(
            state,
            &coarse.y_new,
            &fine_second.y_new,
            adaptive.atol,
            adaptive.rtol,
            method_order,
        )?;
        return Ok(AdaptiveBdfTrial {
            accepted_reports: vec![fine_first, fine_second],
            accepted_history: fine_history,
            error_norm: estimate.error_norm,
            estimator_order: estimate.estimator_order,
            estimator_id: BDF_STARTUP_ESTIMATOR_ID,
        });
    }

    let mut accepted_history = history.clone();
    let report = bdf_step_variable(
        problem,
        t,
        state,
        h,
        config,
        &mut accepted_history,
        counters,
    )?;
    let lte_factor = match report.applied_order {
        BdfOrder::One => bdf1_predictor_correction_lte_factor(
            h,
            history
                .previous_step
                .expect("steady BDF1 estimator history validated"),
        )?,
        BdfOrder::Two => bdf2_predictor_correction_lte_factor(
            h,
            history
                .previous_step
                .expect("steady BDF2 estimator history validated"),
            history
                .older_step
                .expect("steady BDF2 estimator geometry validated"),
        )?,
    };
    // These factors belong to this implementation's explicit history
    // extrapolants.  They are derived by eliminating the leading derivative
    // between the BDF local truncation error and predictor defect; importing
    // a constant-step Nordsieck/NDF coefficient would use a different state
    // representation and is not valid here.
    let error_vector = report
        .y_new
        .iter()
        .zip(&report.predictor)
        .map(|(corrected, predicted)| lte_factor * (corrected - predicted))
        .collect::<Vec<_>>();
    let scale = error_scale(state, &report.y_new, &[adaptive.atol], adaptive.rtol)?;
    let error_norm = wrms(&error_vector, &scale)?;
    let estimator_id = match report.applied_order {
        BdfOrder::One => BDF1_PREDICTOR_ESTIMATOR_ID,
        BdfOrder::Two => BDF2_PREDICTOR_ESTIMATOR_ID,
    };
    Ok(AdaptiveBdfTrial {
        accepted_reports: vec![report.clone()],
        accepted_history,
        error_norm,
        estimator_order: report.applied_order.value() + 1,
        estimator_id,
    })
}

fn adaptive_failure_kind(error: &CoreError) -> Option<AdaptiveFailureKind> {
    match error {
        CoreError::NonFinite(_) => Some(AdaptiveFailureKind::NonFinite),
        CoreError::LinearSolve(_) => Some(AdaptiveFailureKind::LinearSolve),
        CoreError::NonlinearSolve(_) => Some(AdaptiveFailureKind::NonlinearSolve),
        _ => None,
    }
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
        let requested_h = h;
        let (trial_h, clipped) = collector.limit_step(t, requested_h, tf)?;
        let trial = match adaptive_bdf_trial(
            problem,
            t,
            &state,
            trial_h,
            config,
            adaptive,
            &history,
            &mut counters,
        ) {
            Ok(value) => value,
            Err(error) if adaptive_failure_kind(&error).is_some() => {
                let failure = adaptive_failure_kind(&error).expect("failure kind checked");
                counters.rejected_steps += 1;
                record_adaptive_work_failure(&mut counters, failure);
                diagnostics.record_with_failure(
                    trial_h,
                    f64::INFINITY,
                    config.order.value() + 1,
                    "bdf-trial-failed",
                    false,
                    Some(failure),
                );
                h = adaptive_next_step_after_attempt(
                    &mut controller,
                    adaptive,
                    requested_h,
                    trial_h,
                    f64::INFINITY,
                    config.order.value() + 1,
                    false,
                    clipped,
                )?;
                continue;
            }
            Err(error) => return Err(error),
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
            history = trial.accepted_history;
            collector.accept(t, &state, clipped)?;
            counters.accepted_steps += accepted_internal_steps as u64;
            internal_steps += accepted_internal_steps;
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
