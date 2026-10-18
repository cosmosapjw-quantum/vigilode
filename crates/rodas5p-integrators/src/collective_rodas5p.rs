//! Explicit O(N) collective backend for all native RODAS5P stages.
//!
//! Uses the same K coordinates and tableau as `sequential_step`:
//! `(I-h*gamma*J0) K_i = h*f(y+sum alpha_ij K_j) + h*J0*sum Gamma_ij K_j`.
//! One block/rank-two factorization serves all eight stages. The generative
//! model is owned by the workspace, so callbacks cannot silently substitute
//! another equation. Output times are reached by landing, without interpolation.

use rodas5p_core::{
    CoreError, CoreResult, Rodas5pCoefficients, WorkCounters, rodas5p_coefficients, wrms,
};

use crate::collective_direct::{CollectiveDirectWork, CollectiveStageFactors};
use crate::collective_model::CollectiveModel;
use crate::output::OutputCollector;
use crate::{
    AdaptiveControllerState, AdaptiveFailureKind, AdaptiveStepConfig, ObservedIntegrationResult,
    OutputSchedule, rodas_next_step_after_attempt,
};

pub const COLLECTIVE_RODAS5P_DRIVER_ID: &str = "collective-rodas5p-native-woodbury-v1";

/// Reusable state for eight native K-coordinate stage solves. Successful
/// attempts reuse every allocation after construction. Failure diagnostics
/// may allocate. `linear_work` is structural accounting, not instrumentation.
pub struct CollectiveRodas5pWorkspace {
    model: CollectiveModel,
    coeffs: &'static Rodas5pCoefficients,
    factors: CollectiveStageFactors,
    stages: Vec<f64>,
    stage_state: Vec<f64>,
    rhs: Vec<f64>,
    gmix: Vec<f64>,
    jg: Vec<f64>,
    candidate: Vec<f64>,
    embedded: Vec<f64>,
    scale: Vec<f64>,
    has_result: bool,
    linear_work: CollectiveDirectWork,
}

impl CollectiveRodas5pWorkspace {
    /// Admit the carrier=0 rotating-frame model, with exact represented
    /// positive weight sum one. The model and tableau remain immutable.
    pub fn new(model: CollectiveModel) -> CoreResult<Self> {
        let n = model.dimension();
        let coeffs = rodas5p_coefficients()?;
        let mut initial = vec![0.0; n];
        model.initial_state(&mut initial)?;
        let mut linear_work = CollectiveDirectWork::default();
        let factors = CollectiveStageFactors::factorize(&model, &initial, 0.0, &mut linear_work)?;
        Ok(Self {
            model,
            coeffs,
            factors,
            stages: vec![0.0; n * coeffs.stages()],
            stage_state: vec![0.0; n],
            rhs: vec![0.0; n],
            gmix: vec![0.0; n],
            jg: vec![0.0; n],
            candidate: initial,
            embedded: vec![0.0; n],
            scale: vec![0.0; n],
            has_result: false,
            linear_work,
        })
    }

    pub fn model(&self) -> &CollectiveModel {
        &self.model
    }
    pub fn linear_work(&self) -> CollectiveDirectWork {
        self.linear_work
    }
    /// Last successful attempt's K_i; invalidated before every new attempt.
    pub fn stage(&self, i: usize) -> Option<&[f64]> {
        if !self.has_result || i >= self.coeffs.stages() {
            return None;
        }
        let n = self.model.dimension();
        Some(&self.stages[i * n..(i + 1) * n])
    }
    pub fn embedded_error(&self) -> Option<&[f64]> {
        self.has_result.then_some(self.embedded.as_slice())
    }

    /// A complete attempt; writes `out` only on success and returns the native
    /// embedded WRMS estimate. A successful attempt is not automatically an
    /// accepted adaptive step. The caller accepts iff this estimate is <=1.
    #[allow(clippy::too_many_arguments)]
    pub fn step_into(
        &mut self,
        t: f64,
        y: &[f64],
        h: f64,
        atol: f64,
        rtol: f64,
        out: &mut [f64],
        counters: &mut WorkCounters,
    ) -> CoreResult<f64> {
        self.has_result = false;
        let n = self.model.dimension();
        if y.len() != n || out.len() != n {
            return Err(CoreError::Dimension(
                "collective RODAS5P state/output mismatch".into(),
            ));
        }
        if !(t.is_finite() && h.is_finite() && h > 0.0 && (t + h).is_finite() && t + h > t)
            || y.iter().any(|v| !v.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "collective RODAS5P requires finite forward step/state".into(),
            ));
        }
        if !atol.is_finite()
            || !rtol.is_finite()
            || atol < 0.0
            || rtol < 0.0
            || (atol == 0.0 && rtol == 0.0)
        {
            return Err(CoreError::InvalidInput(
                "collective RODAS5P invalid tolerances".into(),
            ));
        }
        counters.jacobian_builds += 1;
        counters.direct_factorizations += 1;
        self.factors
            .refactor(&self.model, y, h * self.coeffs.gamma, &mut self.linear_work)
            .map_err(|e| {
                CoreError::LinearSolve(format!("collective factorization refusal: {e}"))
            })?;
        let s = self.coeffs.stages();
        for i in 0..s {
            self.stage_state.fill(0.0);
            self.gmix.fill(0.0);
            for j in 0..i {
                let alpha = self.coeffs.alpha[(i, j)];
                let gamma = self.coeffs.gamma_matrix[(i, j)];
                for q in 0..n {
                    let k = self.stages[j * n + q];
                    self.stage_state[q] += alpha * k;
                    self.gmix[q] += gamma * k;
                }
            }
            for (q, value) in self.stage_state.iter_mut().enumerate() {
                *value += y[q];
            }
            counters.rhs_calls += 1;
            counters.rhs_evaluations += 1;
            self.model
                .rhs(t + self.coeffs.c[i] * h, &self.stage_state, &mut self.rhs)?;
            self.jg.fill(0.0);
            if self.gmix.iter().any(|v| *v != 0.0) {
                counters.jvp_calls += 1;
                counters.jvp_vectors += 1;
                self.model.jvp(t, y, &self.gmix, &mut self.jg)?;
            }
            for q in 0..n {
                self.rhs[q] = h * self.rhs[q] + h * self.jg[q];
            }
            counters.linear_solves += 1;
            counters.direct_solve_calls += 1;
            self.factors.solve_into(
                &self.rhs,
                &mut self.stages[i * n..(i + 1) * n],
                &mut self.linear_work,
            )?;
        }
        self.candidate.fill(0.0);
        self.embedded.fill(0.0);
        for i in 0..s {
            for q in 0..n {
                self.candidate[q] += self.coeffs.b[i] * self.stages[i * n + q];
                self.embedded[q] += self.coeffs.btilde[i] * self.stages[i * n + q];
            }
        }
        for (q, &yq) in y.iter().enumerate() {
            self.candidate[q] += yq;
            self.scale[q] = atol + rtol * yq.abs().max(self.candidate[q].abs());
        }
        if self.candidate.iter().any(|v| !v.is_finite()) {
            return Err(CoreError::NonFinite(
                "collective RODAS5P endpoint nonfinite".into(),
            ));
        }
        let error = wrms(&self.embedded, &self.scale)?;
        out.copy_from_slice(&self.candidate);
        self.has_result = true;
        Ok(error)
    }
}

#[derive(Clone, Debug)]
pub struct CollectiveRodas5pResult {
    pub observed: ObservedIntegrationResult,
    pub attempts: usize,
    pub accepted_steps: usize,
    pub rejected_steps: usize,
    pub linear_work: CollectiveDirectWork,
    pub driver: &'static str,
}

/// Adaptive, all-stage collective integration using the existing controller,
/// represented-clock policy, and landing output contract. This numerical
/// baseline reports embedded estimates; no global-error certificate is issued.
pub fn integrate_collective_rodas5p_observed(
    model: &CollectiveModel,
    t_span: (f64, f64),
    y0: &[f64],
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> CoreResult<CollectiveRodas5pResult> {
    adaptive.validate()?;
    let (mut t, tf) = t_span;
    if !t.is_finite()
        || !tf.is_finite()
        || tf < t
        || y0.len() != model.dimension()
        || y0.iter().any(|v| !v.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "invalid collective integration input".into(),
        ));
    }
    let mut collector =
        OutputCollector::new(output, t_span, y0)?.with_max_step(adaptive.step_cap());
    let mut workspace = CollectiveRodas5pWorkspace::new(model.clone())?;
    let mut y = y0.to_vec();
    let mut out = vec![0.0; y.len()];
    let mut h = adaptive.initial_step.min(crate::output::step_to(t, tf)?);
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let (mut attempts, mut accepted_steps, mut rejected_steps) = (0, 0, 0);
    while t < tf && attempts < adaptive.max_attempts {
        let Some(next_h) =
            crate::output::adaptive_end_step(t, h, tf, adaptive.step_cap(), &controller)?
        else {
            break;
        };
        h = next_h;
        if (crate::output::below_min_step(t, h, adaptive.min_step) && t + h < tf) || t + h == t {
            break;
        }
        let (trial_h, clipped) = collector.limit_step(t, h, tf)?;
        attempts += 1;
        let outcome = workspace.step_into(
            t,
            &y,
            trial_h,
            adaptive.atol,
            adaptive.rtol,
            &mut out,
            &mut counters,
        );
        let (error, failure) = match outcome {
            Ok(e) if e <= 1.0 => (e, None),
            Ok(e) => (e, Some(AdaptiveFailureKind::LocalError)),
            Err(CoreError::LinearSolve(_)) => {
                (f64::INFINITY, Some(AdaptiveFailureKind::LinearSolve))
            }
            Err(CoreError::NonFinite(_)) => (f64::INFINITY, Some(AdaptiveFailureKind::NonFinite)),
            Err(e) => return Err(e),
        };
        if let Some(kind) = failure {
            counters.rejected_steps += 1;
            rejected_steps += 1;
            crate::adaptive::record_adaptive_work_failure(&mut counters, kind);
        } else {
            counters.accepted_steps += 1;
            accepted_steps += 1;
            t += trial_h;
            y.copy_from_slice(&out);
            collector.accept(t, &y, clipped)?;
        }
        h = rodas_next_step_after_attempt(
            &mut controller,
            adaptive,
            h,
            trial_h,
            error,
            failure.is_none(),
            clipped,
        )?;
    }
    let success = t >= tf;
    let (times, states, output_clipped_steps) = if success {
        collector.finish()?
    } else {
        collector.finish_partial()
    };
    Ok(CollectiveRodas5pResult {
        observed: ObservedIntegrationResult {
            t: times,
            y: states,
            success,
            message: if success {
                "success".into()
            } else {
                "maximum attempt count or minimum step reached".into()
            },
            counters,
            internal_steps: accepted_steps,
            output_clipped_steps,
        },
        attempts,
        accepted_steps,
        rejected_steps,
        linear_work: workspace.linear_work(),
        driver: COLLECTIVE_RODAS5P_DRIVER_ID,
    })
}
