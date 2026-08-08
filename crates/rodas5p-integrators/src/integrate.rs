use crate::{
    KrylovState, OdeProblem, SabrConfig, StageHistory, StepResult, sabr_step, sequential_step,
};
use rodas5p_core::{CoreError, CoreResult, LinearSolverConfig, WorkCounters};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationMethod {
    Sequential,
    Sabr,
}

#[derive(Clone, Debug)]
pub struct IntegrationResult {
    pub t: Vec<f64>,
    pub y: Vec<Vec<f64>>,
    pub success: bool,
    pub message: String,
    pub counters: WorkCounters,
    pub step_methods: Vec<String>,
    pub step_sizes: Vec<f64>,
    pub error_norms: Vec<f64>,
    pub attempts: usize,
}
fn effective(r: &StepResult) -> f64 {
    r.error_norm + r.certificate.as_ref().map_or(0.0, |c| c.fixed_point_error)
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_fixed(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h: f64,
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    atol: f64,
    rtol: f64,
) -> CoreResult<IntegrationResult> {
    let (t0, tf) = t_span;
    if h <= 0.0 || tf < t0 {
        return Err(CoreError::InvalidInput(
            "invalid fixed-step interval".into(),
        ));
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let mut t = t0;
    let mut y = y0.to_vec();
    let mut times = vec![t];
    let mut states = vec![y.clone()];
    let mut methods = Vec::new();
    let mut hs = Vec::new();
    let mut errors = Vec::new();
    let sabr_cfg = sabr_config.unwrap_or_default();
    let mut attempts = 0;
    while t < tf - 10.0 * f64::EPSILON * tf.abs().max(1.0) {
        attempts += 1;
        let step = h.min(tf - t);
        let r = match method {
            IntegrationMethod::Sequential => {
                let r = sequential_step(
                    problem,
                    t,
                    &y,
                    step,
                    &config,
                    recycle.as_mut(),
                    atol,
                    rtol,
                    true,
                    &mut counters,
                )?;
                history.push(step, r.stages.clone());
                r
            }
            IntegrationMethod::Sabr => sabr_step(
                problem,
                t,
                &y,
                step,
                &sabr_cfg,
                Some(&config),
                &mut history,
                recycle.as_mut(),
                atol,
                rtol,
                true,
                &mut counters,
            )?,
        };
        t = r.t_new;
        y = r.y_new.clone();
        times.push(t);
        states.push(y.clone());
        methods.push(r.method.clone());
        hs.push(step);
        errors.push(effective(&r));
    }
    Ok(IntegrationResult {
        t: times,
        y: states,
        success: true,
        message: "success".into(),
        counters,
        step_methods: methods,
        step_sizes: hs,
        error_norms: errors,
        attempts,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn integrate_adaptive(
    problem: &OdeProblem,
    t_span: (f64, f64),
    y0: &[f64],
    h0: f64,
    method: IntegrationMethod,
    linear_config: Option<&LinearSolverConfig>,
    sabr_config: Option<SabrConfig>,
    atol: f64,
    rtol: f64,
    max_steps: usize,
    max_step: f64,
) -> CoreResult<IntegrationResult> {
    let (mut t, tf) = t_span;
    if tf < t || h0 <= 0.0 {
        return Err(CoreError::InvalidInput("invalid adaptive interval".into()));
    }
    let config = linear_config.cloned().unwrap_or_default();
    let mut y = y0.to_vec();
    let mut h = h0.min(max_step).min(tf - t);
    let mut counters = WorkCounters::default();
    let mut history = StageHistory::default();
    let mut recycle = KrylovState::for_method(config.method);
    let sabr_cfg = sabr_config.unwrap_or_default();
    let mut times = vec![t];
    let mut states = vec![y.clone()];
    let mut methods = Vec::new();
    let mut hs = Vec::new();
    let mut errors = Vec::new();
    let mut attempts = 0;
    let safety = 0.9;
    let min_factor = 0.2;
    let max_factor = 5.0;
    while t < tf && attempts < max_steps {
        attempts += 1;
        h = h.min(max_step).min(tf - t);
        if h <= f64::MIN_POSITIVE {
            break;
        }
        let state_snapshot = recycle.clone();
        let history_snapshot = history.clone();
        let trial = match method {
            IntegrationMethod::Sequential => sequential_step(
                problem,
                t,
                &y,
                h,
                &config,
                recycle.as_mut(),
                atol,
                rtol,
                false,
                &mut counters,
            ),
            IntegrationMethod::Sabr => sabr_step(
                problem,
                t,
                &y,
                h,
                &sabr_cfg,
                Some(&config),
                &mut history,
                recycle.as_mut(),
                atol,
                rtol,
                false,
                &mut counters,
            ),
        };
        let r = match trial {
            Ok(v) => v,
            Err(CoreError::NonFinite(_) | CoreError::LinearSolve(_)) => {
                counters.rejected_steps += 1;
                recycle = state_snapshot;
                history = history_snapshot;
                h *= min_factor;
                continue;
            }
            Err(e) => return Err(e),
        };
        let e = effective(&r);
        if r.accepted && e <= 1.0 && r.y_new.iter().all(|v| v.is_finite()) {
            t = r.t_new;
            y = r.y_new.clone();
            times.push(t);
            states.push(y.clone());
            methods.push(r.method.clone());
            hs.push(r.h);
            errors.push(e);
            if method == IntegrationMethod::Sequential {
                history.push(r.h, r.stages.clone());
            }
            let factor = if e == 0.0 {
                max_factor
            } else {
                (safety * e.powf(-0.2)).clamp(min_factor, max_factor)
            };
            h *= factor;
        } else {
            recycle = state_snapshot;
            history = history_snapshot;
            let factor = (safety * e.max(1e-16).powf(-0.2)).clamp(min_factor, 0.9);
            h *= factor;
        }
    }
    let success = t >= tf - 10.0 * f64::EPSILON * tf.abs().max(1.0);
    Ok(IntegrationResult {
        t: times,
        y: states,
        success,
        message: if success {
            "success".into()
        } else {
            "maximum step count or minimum step reached".into()
        },
        counters,
        step_methods: methods,
        step_sizes: hs,
        error_norms: errors,
        attempts,
    })
}
