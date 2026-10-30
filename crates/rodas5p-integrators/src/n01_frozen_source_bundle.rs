//! C9 opt-in trusted N01 one-step source/callback bundle.
//!
//! Source creation and native OdeProblem callback creation are ONE operation.
//! No caller-provided independent owner, problem, or parameters can replace
//! the callback source inside a frozen step. Temporary stage evaluations
//! check the original source box but do not advance the model generation,
//! clock or accepted state. This is a small original RODAS8 DIRECT REFERENCE
//! only, NOT production derivative-free RODAS and NOT global-IVP acceptance.
//!
//! The legacy n01_mass_action_owned_problem factory is intentionally unchanged.
use crate::{OdeProblem, StepResult, sequential_step};
use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearSolverConfig, N01MassActionOwner,
    N01MassActionTrial, WorkCounters,
};
use std::sync::Arc;

fn state3(y: &[f64]) -> CoreResult<[f64; 3]> {
    y.try_into()
        .map_err(|_| CoreError::Dimension("N01_FROZEN_SOURCE_STATE_DIMENSION".into()))
}

/// Privately owns both the exact source and the callbacks bound to that source.
/// Unlike a public (OdeProblem, Handle) pair, the caller cannot supply an
/// unrelated OdeProblem to methods of this bundle.
pub struct N01FrozenSourceProblem {
    source: Arc<N01MassActionOwner>,
    problem: OdeProblem,
}

/// A borrowing single-step session. Its source and native OdeProblem are
/// permanently tied together by N01FrozenSourceProblem::new.
pub struct N01FrozenSourceStep<'a> {
    bundle: &'a N01FrozenSourceProblem,
    trial: N01MassActionTrial,
    time: f64,
    state: [f64; 3],
    h: f64,
}

impl N01FrozenSourceProblem {
    /// The source is moved into the bundle; no mutable owner handle escapes.
    pub fn new(owner: N01MassActionOwner) -> CoreResult<Self> {
        let source = Arc::new(owner);
        let rhs_source = Arc::clone(&source);
        let rhs = Arc::new(
            move |t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
                if !t.is_finite() || out.len() != 3 {
                    return Err(CoreError::InvalidInput(
                        "N01_FROZEN_RHS_BAD_CLOCK_OR_SHAPE".into(),
                    ));
                }
                let state = state3(y)?;
                rhs_source.check_read_only_stage_probe(state)?;
                let mut temp = [0.0; 3];
                rhs_source.rhs_at(&state, &mut temp)?;
                out.copy_from_slice(&temp);
                Ok(())
            },
        );
        let jac_source = Arc::clone(&source);
        let jac = Arc::new(move |t: f64, y: &[f64]| -> CoreResult<DenseMatrix> {
            if !t.is_finite() {
                return Err(CoreError::InvalidInput(
                    "N01_FROZEN_JAC_BAD_CLOCK".into(),
                ));
            }
            let state = state3(y)?;
            jac_source.check_read_only_stage_probe(state)?;
            let j = jac_source.jacobian_at(&state)?;
            DenseMatrix::from_rows(&[&j[0], &j[1], &j[2]])
        });
        // Supplied JVP exists solely for an explicitly requested fallback.
        // It is never called by the direct small reference step below.
        let jvp_source = Arc::clone(&source);
        let jvp = Arc::new(
            move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| -> CoreResult<()> {
                if !t.is_finite() || out.len() != 3 {
                    return Err(CoreError::InvalidInput(
                        "N01_FROZEN_JVP_BAD_CLOCK_OR_SHAPE".into(),
                    ));
                }
                let state = state3(y)?;
                let vector = state3(v)?;
                jvp_source.check_read_only_stage_probe(state)?;
                let mut temp = [0.0; 3];
                jvp_source.jvp_at(&state, &vector, &mut temp)?;
                out.copy_from_slice(&temp);
                Ok(())
            },
        );
        let epoch_source = Arc::clone(&source);
        let epoch = Arc::new(move || epoch_source.model_epoch());
        let problem = OdeProblem::new(
            "n01-c9-immutable-bundled-single-step-reference",
            3,
            rhs,
            None,
            Some(jac),
            Some(jvp),
            None,
            true,
            None,
            None,
        )?
        .with_model_epoch(epoch);
        Ok(Self { source, problem })
    }

    pub fn begin_step(
        &self,
        time: f64,
        state: [f64; 3],
        h: f64,
    ) -> CoreResult<N01FrozenSourceStep<'_>> {
        if !time.is_finite()
            || !h.is_finite()
            || h <= 0.0
            || !(time + h).is_finite()
        {
            return Err(CoreError::InvalidInput(
                "N01_FROZEN_STEP_BAD_TIME_OR_SIZE".into(),
            ));
        }
        let trial = self.source.freeze();
        self.source.check_raw_u_stage_trial(&trial, time, state)?;
        self.source.check_read_only_stage_probe(state)?;
        Ok(N01FrozenSourceStep {
            bundle: self,
            trial,
            time,
            state,
            h,
        })
    }

    /// Information-only: model generation cannot be changed through this bundle.
    pub fn model_epoch(&self) -> u64 {
        self.source.model_epoch()
    }
}

impl N01FrozenSourceStep<'_> {
    fn live(&self) -> CoreResult<()> {
        self.bundle
            .source
            .check_raw_u_stage_trial(&self.trial, self.time, self.state)
    }

    /// Exact represented rates and accepted carrier of the SAME source
    /// whose Arc is captured by all problem callbacks.
    pub fn represented_source(&self) -> CoreResult<([f64; 3], [f64; 3])> {
        self.live()?;
        let (rates, carrier, _, _) = self.bundle.source.value_only_stage_inputs(&self.trial)?;
        self.live()?;
        Ok((rates, carrier))
    }

    /// One temporary source callback through the bundled native OdeProblem.
    /// The output is touched only after all original callback guards pass.
    pub fn probe_rhs(
        &self,
        time: f64,
        state: [f64; 3],
        out: &mut [f64; 3],
        work: &mut WorkCounters,
    ) -> CoreResult<()> {
        self.live()?;
        let mut temp = [0.0; 3];
        self.bundle.problem.eval_rhs_into(time, &state, &mut temp, work)?;
        self.live()?;
        out.copy_from_slice(&temp);
        Ok(())
    }

    /// Execute the ORIGINAL eight-stage native sequential RODAS5P once,
    /// through this bundle's unreplaceable, read-only callback source.
    /// This uses a small DIRECT linear solver as a scientific oracle only.
    /// force_accept=true is DIAGNOSTIC, not physical/adaptive admission.
    pub fn original_rodas8_direct_reference(
        &self,
        work: &mut WorkCounters,
    ) -> CoreResult<StepResult> {
        self.live()?;
        let result = sequential_step(
            &self.bundle.problem,
            self.time,
            &self.state,
            self.h,
            &LinearSolverConfig::default(),
            None,
            1.0e-10,
            1.0e-7,
            true,
            work,
        )?;
        self.live()?;
        Ok(result)
    }
}
