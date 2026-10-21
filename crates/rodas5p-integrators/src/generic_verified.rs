//! Capability-bound continuous-residual RODAS5P driver.
//!
//! Candidate generation is private to an immutable model owner. Every attempted
//! step is staged, then a whole-interval tube and every requested output are
//! checked before committing state, inherited radius, output cursor or history.
//! This stateless route has no speculative factor/recycle cache to roll
//! back. Its cost includes all failed candidate and verification work.
use rodas5p_core::directed::{Interval, add_up, mul_up, sqrt_up};
use rodas5p_core::{CoreError, LinearMethod, LinearSolverConfig, WorkCounters};
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;

use crate::{
    AdaptiveStepConfig, BoundCollectiveProblem, ContinuousResidualVerificationError,
    ContinuousResidualWorkspace, DenseOutputError, DenseSegmentWork, OdeProblem, Pr101Model,
    Pr101VerifiedError, Rodas5pQuarticSegment, sequential_step,
};

#[derive(Debug, Error)]
pub enum VerifiedResidualError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Dense(#[from] DenseOutputError),
    #[error(transparent)]
    Tube(#[from] ContinuousResidualVerificationError),
    #[error(transparent)]
    Pr101(#[from] Pr101VerifiedError),
    #[error("invalid verified-residual input: {0}")]
    Invalid(&'static str),
    #[error("requested output radius {radius:e} exceeds unchanged absolute budget {budget:e}")]
    OutputBudget { radius: f64, budget: f64 },
}

/// Only crate-owned immutable models can implement this capability. Neither a
/// public point callback nor a caller-made scalar bound can mint a certificate.
pub(crate) trait VerifiedModelProvider {
    fn problem(&self) -> &OdeProblem;
    fn validate_span(&self, span: (f64, f64)) -> Result<(), VerifiedResidualError>;
    fn verify(
        &self,
        curve: &Rodas5pQuarticSegment,
        initial_radius: f64,
        tube_radius: f64,
        workspace: &mut ContinuousResidualWorkspace,
        work: &mut DenseSegmentWork,
    ) -> Result<VerifiedTube, VerifiedResidualError>;
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct VerifiedTube {
    pub(crate) endpoint_radius: f64,
    pub(crate) uniform_radius: f64,
    pub(crate) residual_upper: f64,
    pub(crate) logarithmic_norm_upper: f64,
}

/// Opaque generated problem/provider binding. Factories validate and own every
/// model parameter; the generated callback cannot be replaced by the caller.
pub struct VerifiedContinuousProblem {
    identity: Arc<()>,
    provider: Box<dyn VerifiedModelProvider>,
}

impl VerifiedContinuousProblem {
    pub(crate) fn from_provider(provider: impl VerifiedModelProvider + 'static) -> Self {
        Self {
            identity: Arc::new(()),
            provider: Box::new(provider),
        }
    }

    pub fn from_collective(model: BoundCollectiveProblem) -> Self {
        Self::from_provider(model)
    }

    pub fn from_pr101(model: Pr101Model) -> Result<Self, VerifiedResidualError> {
        let problem = model.candidate_problem()?;
        Ok(Self::from_provider(Pr101Provider { model, problem }))
    }

    pub fn dimension(&self) -> usize {
        self.provider.problem().dimension
    }
}

impl VerifiedModelProvider for BoundCollectiveProblem {
    fn problem(&self) -> &OdeProblem {
        self.candidate_problem()
    }

    fn validate_span(&self, _span: (f64, f64)) -> Result<(), VerifiedResidualError> {
        Ok(())
    }

    fn verify(
        &self,
        curve: &Rodas5pQuarticSegment,
        initial_radius: f64,
        tube_radius: f64,
        workspace: &mut ContinuousResidualWorkspace,
        work: &mut DenseSegmentWork,
    ) -> Result<VerifiedTube, VerifiedResidualError> {
        let bound = self.verify_continuous_residual_tube(
            curve,
            initial_radius,
            tube_radius,
            workspace,
            work,
        )?;
        work.gain_operations = work.gain_operations.saturating_add(1);
        Ok(VerifiedTube {
            endpoint_radius: add_up(
                bound.transported_initial_radius_upper,
                bound.residual_transport_upper,
            )?,
            uniform_radius: bound.error_radius_upper,
            residual_upper: bound.residual_l2_upper,
            logarithmic_norm_upper: bound.one_sided_log_norm_upper,
        })
    }
}

struct Pr101Provider {
    model: Pr101Model,
    problem: OdeProblem,
}
impl VerifiedModelProvider for Pr101Provider {
    fn problem(&self) -> &OdeProblem {
        &self.problem
    }
    fn validate_span(&self, span: (f64, f64)) -> Result<(), VerifiedResidualError> {
        if span.0 < 0.0 || span.1 > 1.0 {
            return Err(VerifiedResidualError::Invalid("PR101 domain is [0,1]"));
        }
        Ok(())
    }
    fn verify(
        &self,
        curve: &Rodas5pQuarticSegment,
        initial_radius: f64,
        tube_radius: f64,
        _workspace: &mut ContinuousResidualWorkspace,
        work: &mut DenseSegmentWork,
    ) -> Result<VerifiedTube, VerifiedResidualError> {
        // Five represented value vectors plus five immutable identity vectors.
        work.workspace_items = work
            .workspace_items
            .saturating_add(10 * self.problem.dimension as u64);
        let verified =
            self.model
                .verify_segment(curve.clone(), initial_radius, tube_radius, work)?;
        let bound = verified.bounds();
        Ok(VerifiedTube {
            endpoint_radius: bound.endpoint_radius,
            uniform_radius: bound.uniform_radius,
            residual_upper: bound.residual_upper,
            logarithmic_norm_upper: bound.logarithmic_norm_upper,
        })
    }
}

/// Diagnostics are copied values, not certificate constructors.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct VerifiedResidualDiagnostics {
    pub span: (f64, f64),
    pub initial_radius: f64,
    pub endpoint_radius: f64,
    pub uniform_radius: f64,
    pub residual_upper: f64,
    pub logarithmic_norm_upper: f64,
}

/// An immutable verified continuous curve. Uniform radius bounds the exact
/// solution of the factory-owned model from the declared initial L2 ball.
#[derive(Clone, Debug)]
pub struct VerifiedResidualSegment {
    pub(crate) curve: Rodas5pQuarticSegment,
    diagnostics: VerifiedResidualDiagnostics,
    dimension: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerifiedResidualOutput {
    time: f64,
    center: Vec<f64>,
    radius_l2: f64,
}
impl VerifiedResidualOutput {
    pub fn time(&self) -> f64 {
        self.time
    }
    pub fn center(&self) -> &[f64] {
        &self.center
    }
    pub fn radius_l2(&self) -> f64 {
        self.radius_l2
    }
}

impl VerifiedResidualSegment {
    pub fn diagnostics(&self) -> VerifiedResidualDiagnostics {
        self.diagnostics
    }

    /// Exact requested physical time; includes the displacement of the returned
    /// binary64 center from the real polynomial, with an outward L2 norm.
    pub fn evaluate(
        &self,
        time: f64,
        work: &mut DenseSegmentWork,
    ) -> Result<VerifiedResidualOutput, VerifiedResidualError> {
        let mut center = vec![0.0; self.dimension];
        let mut box_value = vec![Interval::point(0.0)?; self.dimension];
        work.workspace_items = work
            .workspace_items
            .saturating_add(2 * self.dimension as u64);
        self.curve
            .evaluate_center_at_into(time, &mut center, work)?;
        self.curve
            .evaluate_value_at_into(time, &mut box_value, work)?;
        let mut squared = 0.0;
        for (&value, &point) in box_value.iter().zip(&center) {
            work.residual_operations = work.residual_operations.saturating_add(1);
            let displacement = value.sub(Interval::point(point)?)?;
            work.residual_operations = work.residual_operations.saturating_add(1);
            let displacement = displacement.mag();
            work.residual_operations = work.residual_operations.saturating_add(1);
            let square = mul_up(displacement, displacement)?;
            work.residual_operations = work.residual_operations.saturating_add(1);
            squared = add_up(squared, square)?;
        }
        work.residual_operations = work.residual_operations.saturating_add(1);
        let displacement = sqrt_up(squared)?;
        work.residual_operations = work.residual_operations.saturating_add(1);
        let radius_l2 = add_up(self.diagnostics.uniform_radius, displacement)?;
        Ok(VerifiedResidualOutput {
            time,
            center,
            radius_l2,
        })
    }
}

#[derive(Debug)]
pub struct VerifiedResidualRun {
    // Private scientific binding: mutable public telemetry is never used to
    // mint a global certificate. Only this model-owned driver writes it.
    pub(crate) model_identity: Arc<()>,
    pub(crate) run_identity: Arc<()>,
    pub(crate) model_name: String,
    pub(crate) source_span: (f64, f64),
    pub(crate) source_initial_center: Vec<f64>,
    pub(crate) source_initial_radius: f64,
    pub(crate) source_absolute_l2_budget: f64,
    pub(crate) source_complete: bool,
    segments: Vec<VerifiedResidualSegment>,
    outputs: Vec<VerifiedResidualOutput>,
    pub success: bool,
    pub failure: Option<String>,
    pub attempts: usize,
    pub rejected_attempts: usize,
    pub solver_work: WorkCounters,
    pub verification_work: DenseSegmentWork,
}
impl VerifiedResidualRun {
    pub fn segments(&self) -> &[VerifiedResidualSegment] {
        &self.segments
    }
    pub fn outputs(&self) -> &[VerifiedResidualOutput] {
        &self.outputs
    }
}

/// Generic transaction over admitted model capabilities. The absolute tolerance
/// is enforced throughout each continuous segment and at every returned output;
/// candidate solves retain the exact supplied atol/rtol. Initial radius is L2.
/// Positive atol is required; a verified relative scaling is a separate feature.
/// The model remains immutable throughout the run. Rejection retains its entire
/// attempted work ledger and leaves the committed scientific prefix unchanged.
pub fn integrate_verified_residual(
    owner: &VerifiedContinuousProblem,
    span: (f64, f64),
    initial_center: &[f64],
    initial_radius: f64,
    output_times: &[f64],
    config: &AdaptiveStepConfig,
) -> Result<VerifiedResidualRun, VerifiedResidualError> {
    config.validate()?;
    let n = owner.dimension();
    if !(span.0.is_finite()
        && span.1.is_finite()
        && span.0 <= span.1
        && initial_center.len() == n
        && n > 0
        && initial_center.iter().all(|x| x.is_finite())
        && initial_radius.is_finite()
        && initial_radius >= 0.0
        && config.atol > 0.0
        && initial_radius <= config.atol * 0.5)
    {
        return Err(VerifiedResidualError::Invalid(
            "span, state, L2 radius or positive absolute budget",
        ));
    }
    if output_times
        .iter()
        .any(|&t| !t.is_finite() || t < span.0 || t > span.1)
        || output_times.windows(2).any(|p| p[0] >= p[1])
    {
        return Err(VerifiedResidualError::Invalid(
            "strictly increasing in-span output times required",
        ));
    }
    owner.provider.validate_span(span)?;
    let problem = owner.provider.problem();
    let linear = LinearSolverConfig {
        method: if problem.has_explicit_jacobian() {
            LinearMethod::Direct
        } else {
            LinearMethod::Gmres
        },
        ..LinearSolverConfig::default()
    };
    crate::sequential::validate_problem_linear_config(problem, &linear)?;
    let mut run = VerifiedResidualRun {
        model_identity: owner.identity.clone(),
        run_identity: Arc::new(()),
        model_name: problem.name.clone(),
        source_span: span,
        source_initial_center: initial_center.to_vec(),
        source_initial_radius: initial_radius,
        source_absolute_l2_budget: config.atol,
        source_complete: false,
        segments: Vec::new(),
        outputs: Vec::new(),
        success: false,
        failure: None,
        attempts: 0,
        rejected_attempts: 0,
        solver_work: WorkCounters::default(),
        verification_work: DenseSegmentWork::default(),
    };
    let mut workspace = ContinuousResidualWorkspace::new(n, &mut run.verification_work)?;
    let mut cursor = 0;
    if output_times.first().copied() == Some(span.0) {
        run.outputs.push(VerifiedResidualOutput {
            time: span.0,
            center: initial_center.to_vec(),
            radius_l2: initial_radius,
        });
        cursor = 1;
    }
    let (mut t, mut state, mut radius) = (span.0, initial_center.to_vec(), initial_radius);
    let mut proposed = config.initial_step.min(config.max_step);
    let tube = config.atol * 0.5;
    while t < span.1 && run.attempts < config.max_attempts {
        let h = match crate::output::end_step_capped(t, proposed, span.1, config.step_cap()) {
            Ok(h) => h,
            Err(error) => {
                run.failure = Some(error.to_string());
                break;
            }
        };
        if crate::output::below_min_step(t, h, config.min_step) && t + h < span.1 {
            run.failure = Some("verified driver minimum step reached".into());
            break;
        }
        run.attempts += 1;
        let accepted_before = run.solver_work.accepted_steps;
        let rejected_before = run.solver_work.rejected_steps;
        let candidate = sequential_step(
            problem,
            t,
            &state,
            h,
            &linear,
            None,
            config.atol,
            config.rtol,
            true,
            &mut run.solver_work,
        );
        run.solver_work.accepted_steps = accepted_before;
        run.solver_work.rejected_steps = rejected_before;
        let trial = (|| -> Result<_, VerifiedResidualError> {
            let step = candidate?;
            let curve = Rodas5pQuarticSegment::from_step(&step, &mut run.verification_work)?;
            let progress = ((step.t_new - span.0) / (span.1 - span.0)).clamp(0.0, 1.0);
            // This schedule chooses a tube; it is not a numerical error bound.
            // Every selected radius is subsequently verified outwardly.
            let trial_tube = (initial_radius + (tube - initial_radius) * progress)
                .max(initial_radius)
                .min(tube);
            let bound = owner.provider.verify(
                &curve,
                radius,
                trial_tube,
                &mut workspace,
                &mut run.verification_work,
            )?;
            if !(bound.endpoint_radius.is_finite()
                && bound.endpoint_radius >= 0.0
                && bound.uniform_radius.is_finite()
                && bound.uniform_radius >= bound.endpoint_radius
                && bound.uniform_radius >= radius
                && bound.uniform_radius <= trial_tube
                && bound.residual_upper.is_finite()
                && bound.residual_upper >= 0.0
                && bound.logarithmic_norm_upper.is_finite())
            {
                return Err(VerifiedResidualError::Invalid(
                    "provider violated verified tube contract",
                ));
            }
            let verified = VerifiedResidualSegment {
                curve,
                dimension: n,
                diagnostics: VerifiedResidualDiagnostics {
                    span: (t, step.t_new),
                    initial_radius: radius,
                    endpoint_radius: bound.endpoint_radius,
                    uniform_radius: bound.uniform_radius,
                    residual_upper: bound.residual_upper,
                    logarithmic_norm_upper: bound.logarithmic_norm_upper,
                },
            };
            let mut next = cursor;
            let mut pending = Vec::new();
            while next < output_times.len() && output_times[next] <= step.t_new {
                let output = verified.evaluate(output_times[next], &mut run.verification_work)?;
                if output.radius_l2 > config.atol {
                    return Err(VerifiedResidualError::OutputBudget {
                        radius: output.radius_l2,
                        budget: config.atol,
                    });
                }
                pending.push(output);
                next += 1;
            }
            // A no-fail endpoint copy, state and outputs are all prepared before
            // any visible transaction state changes.
            Ok((verified, step.y_new, pending, next))
        })();
        match trial {
            Ok((verified, endpoint, pending, next)) => {
                run.solver_work.accepted_steps = accepted_before.saturating_add(1);
                t = verified.diagnostics.span.1;
                state = endpoint;
                radius = verified.diagnostics.endpoint_radius;
                run.outputs.extend(pending);
                cursor = next;
                run.segments.push(verified);
                proposed = (h * 1.5).min(config.max_step);
            }
            Err(error) => {
                run.solver_work.rejected_steps = rejected_before.saturating_add(1);
                run.rejected_attempts += 1;
                proposed = h * 0.5;
                run.failure = Some(error.to_string());
            }
        }
    }
    run.success = t == span.1 && cursor == output_times.len();
    run.source_complete = run.success;
    if run.success {
        run.failure = None;
    } else if run.failure.is_none() {
        run.failure = Some("verified driver attempt budget exhausted".into());
    }
    Ok(run)
}
