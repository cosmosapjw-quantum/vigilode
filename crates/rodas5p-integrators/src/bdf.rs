use rodas5p_core::{CoreError, CoreResult, DenseMatrix, WorkCounters};
use serde::Serialize;

use crate::output::OutputCollector;
use crate::{
    NewtonConfig, NewtonReport, ObservedIntegrationResult, OdeProblem, OutputSchedule,
    solve_dense_newton,
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
            && history
                .previous_step
                .is_some_and(|previous_h| same_step(previous_h, h))
    });
    let (applied_order, used_startup) = match config.order {
        BdfOrder::One => (BdfOrder::One, false),
        BdfOrder::Two if previous.is_some() => (BdfOrder::Two, false),
        BdfOrder::Two => (BdfOrder::One, true),
    };
    let predictor = match (applied_order, previous) {
        (BdfOrder::Two, Some(previous_state)) => y
            .iter()
            .zip(previous_state.iter())
            .map(|(current, old)| 2.0 * current - old)
            .collect(),
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
            let combination: Vec<f64> = match applied_order {
                BdfOrder::One => candidate
                    .iter()
                    .zip(y.iter())
                    .map(|(next, current)| next - current)
                    .collect(),
                BdfOrder::Two => {
                    let old = previous_state.as_ref().expect("BDF2 history validated");
                    candidate
                        .iter()
                        .zip(y.iter())
                        .zip(old.iter())
                        .map(|((next, current), previous)| 3.0 * next - 4.0 * current + previous)
                        .collect()
                }
            };
            let mut residual = mass.matvec(&combination)?;
            local_counters.mass_matvecs += 1;
            let rhs_scale = match applied_order {
                BdfOrder::One => h,
                BdfOrder::Two => 2.0 * h,
            };
            for (value, forcing) in residual.iter_mut().zip(rhs) {
                *value -= rhs_scale * forcing;
            }
            Ok(residual)
        },
        |candidate, local_counters| {
            let jacobian = problem.dense_jacobian(t_new, candidate, local_counters)?;
            match applied_order {
                BdfOrder::One => scaled_mass_jacobian(&mass, &jacobian, 1.0, -h),
                BdfOrder::Two => scaled_mass_jacobian(&mass, &jacobian, 3.0, -2.0 * h),
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
