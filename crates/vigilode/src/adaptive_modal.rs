//! Opt-in adaptive modal RODAS5P and protected dispatch under the same frontend.
//! Tolerances for `Rates` refer to modal coordinates. Callers holding a
//! physical-space RHS pass it as `General` with a physical-space initial state.

use crate::modal_rodas5p::ModalRodas5pStep;
use crate::{Atol, Jacobian, Options, Problem, Rodas5p, Solution, SolveError, VigilODE};
use rodas5p_core::{error_scale, step_error_wrms};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveOutputSession, AdaptiveStepConfig, OutputSchedule,
    rodas_next_step_after_attempt,
};

/// A source declaration fixes the interpretation of the input amplitudes.
pub enum ModalInput<'a> {
    /// Independent, time-independent diagonal rates with amplitudes already in their basis.
    Rates(&'a [f64]),
    /// An ordinary ODE whose own coordinates and callbacks are retained.
    General(&'a Problem),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Reason the original protected RODAS5P driver was selected.
pub enum ModalFallback {
    /// The input supplies a general right-hand side.
    GeneralProblem,
    /// The requested continuous extension uses protected stage values.
    DenseOutput,
    /// The declared real spectrum includes a positive rate.
    GrowingMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Termination of the modal adaptive loop.
pub enum ModalStop {
    /// The last accepted state is at the requested final time.
    ReachedEnd,
    /// All allowed accepted and rejected attempts have been charged.
    AttemptBudget,
    /// The common represented-time controller could not issue another interval.
    StepSizeTooSmall,
    /// The attempt or clock failed; the accompanying message records the cause.
    Numerical(String),
}

/// Adaptive endpoint, termination reason and charged work counts.
pub struct ModalIntegration {
    /// Reason integration stopped.
    pub stop: ModalStop,
    /// Last accepted time and modal state, including on stop.
    pub time: f64,
    /// Amplitudes at the last accepted time.
    pub state: Vec<f64>,
    /// Number of accepted physical intervals.
    pub accepted: usize,
    /// Number of attempts that did not commit a state.
    pub rejected: usize,
}

/// Executed method with its native run record.
pub enum ModalOutcome {
    /// The specialized modal endpoint controller ran.
    Modal(ModalIntegration),
    /// Protected RODAS5P ran on the supplied or declared ODE.
    Protected {
        /// Capability which selected the protected path.
        reason: ModalFallback,
        /// Protected solution, including its own termination and counters.
        solution: Box<Solution>,
    },
}

fn invalid(e: impl std::fmt::Display) -> SolveError {
    SolveError::InvalidInput(e.to_string())
}

fn diagonal_problem(rates: &[f64]) -> Problem {
    let rhs_rates = rates.to_vec();
    let jac_rates = rhs_rates.clone();
    Problem::new(rates.len(), move |_, y, out| {
        for ((f, y), rate) in out.iter_mut().zip(y).zip(&rhs_rates) {
            *f = rate * y;
        }
        Ok(())
    })
    .jacobian(Jacobian::banded(0, 0, move |_, _, band| {
        band.copy_from_slice(&jac_rates);
        Ok(())
    }))
    .time_derivative(|_, _, out| {
        out.fill(0.0);
        Ok(())
    })
}

impl VigilODE {
    /// Integrate declared rates with the protected pair's adaptive controller.
    /// A general RHS goes directly to protected RODAS5P. Dense output or
    /// growing real modes use protected RODAS5P on the same declared diagonal
    /// ODE, retaining the caller's coordinates and options.
    pub fn solve_modal(
        input: ModalInput<'_>,
        span: (f64, f64),
        y0: &[f64],
        opts: &Options,
    ) -> Result<ModalOutcome, SolveError> {
        let protected = |problem: &Problem, reason| {
            Rodas5p::solve(problem, span, y0, opts).map(|solution| ModalOutcome::Protected {
                reason,
                solution: Box::new(solution),
            })
        };
        let rates = match input {
            ModalInput::General(problem) => {
                return protected(problem, ModalFallback::GeneralProblem);
            }
            ModalInput::Rates(rates) => rates,
        };
        if rates.len() != y0.len()
            || rates.is_empty()
            || !rates.iter().chain(y0).all(|x| x.is_finite())
        {
            return Err(invalid(
                "spectrum and initial state must be nonempty, finite, and of the same length",
            ));
        }
        if opts.dense_output {
            return protected(&diagonal_problem(rates), ModalFallback::DenseOutput);
        }
        if rates.iter().any(|&r| r > 0.0) {
            return protected(&diagonal_problem(rates), ModalFallback::GrowingMode);
        }
        let atol = match &opts.atol {
            Atol::Scalar(a) => std::slice::from_ref(a),
            Atol::Vector(v) if v.len() == y0.len() => v.as_slice(),
            Atol::Vector(_) => return Err(invalid("modal atol must have one entry per rate")),
        };
        if !atol.iter().all(|&a| a.is_finite() && a >= 0.0) {
            return Err(invalid("modal atol must be finite and nonnegative"));
        }
        let cfg = AdaptiveStepConfig {
            atol: atol.iter().copied().fold(0.0, f64::max),
            rtol: opts.rtol,
            initial_step: opts.h0,
            max_attempts: opts.max_steps,
            ..Default::default()
        };
        cfg.validate().map_err(invalid)?;
        let times = if span.0 == span.1 {
            vec![span.0]
        } else {
            vec![span.0, span.1]
        };
        let outputs = OutputSchedule::new(times).map_err(invalid)?;
        let mut clock = AdaptiveOutputSession::new(&cfg, &outputs, span, y0).map_err(invalid)?;
        let mut control = AdaptiveControllerState::default();
        let mut run = ModalIntegration {
            stop: ModalStop::ReachedEnd,
            time: span.0,
            state: y0.to_vec(),
            accepted: 0,
            rejected: 0,
        };
        let mut trial_y = vec![0.; y0.len()];
        let mut error = vec![0.; y0.len()];
        let mut h = cfg.initial_step;
        while run.time < span.1 {
            if run.accepted + run.rejected >= cfg.max_attempts {
                run.stop = ModalStop::AttemptBudget;
                break;
            }
            let trial = match clock.trial(run.time, h, &control) {
                Ok(Some(trial)) => trial,
                Ok(None) => {
                    run.stop = ModalStop::StepSizeTooSmall;
                    break;
                }
                Err(e) => {
                    run.stop = ModalStop::Numerical(e.to_string());
                    break;
                }
            };
            let norm = ModalRodas5pStep::new(trial.h, rates)
                .and_then(|p| p.apply_into(&run.state, &mut trial_y, &mut error))
                .map_err(|e| e.to_string())
                .and_then(|()| {
                    error_scale(&run.state, &trial_y, atol, cfg.rtol)
                        .and_then(|scale| step_error_wrms(&error, &scale))
                        .map_err(|e| e.to_string())
                });
            let norm = match norm {
                Ok(v) => v,
                Err(e) => {
                    run.rejected += 1;
                    run.stop = ModalStop::Numerical(e);
                    break;
                }
            };
            let accepted = norm.is_finite() && norm <= 1.0;
            if accepted {
                let end = run.time + trial.h;
                if let Err(e) = clock.accept(end, &trial_y, trial.clipped) {
                    run.rejected += 1;
                    run.stop = ModalStop::Numerical(e.to_string());
                    break;
                }
                std::mem::swap(&mut run.state, &mut trial_y);
                run.time = end;
                run.accepted += 1;
                if end == span.1 {
                    break;
                }
            } else {
                run.rejected += 1;
            }
            match rodas_next_step_after_attempt(
                &mut control,
                &cfg,
                trial.requested_h,
                trial.h,
                norm,
                accepted,
                trial.clipped,
            ) {
                Ok(next) => h = next,
                Err(e) => {
                    run.stop = ModalStop::Numerical(e.to_string());
                    break;
                }
            }
        }
        Ok(ModalOutcome::Modal(run))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_rejections_rollback_and_match_protected_and_exact() {
        let rates = [0., -1., -300.];
        let y0 = [0., 2., -1.];
        let opts = Options {
            h0: 0.4,
            rtol: 1e-7,
            atol: Atol::Vector(vec![0., 1e-10, 1e-10]),
            ..Default::default()
        };
        let solve = |opts: &Options| match VigilODE::solve_modal(
            ModalInput::Rates(&rates),
            (0., 1.),
            &y0,
            opts,
        )
        .unwrap()
        {
            ModalOutcome::Modal(run) => run,
            _ => panic!("unexpected fallback"),
        };
        let denied = solve(&Options {
            max_steps: 1,
            ..opts.clone()
        });
        assert_eq!(
            (
                denied.stop,
                denied.time,
                denied.state,
                denied.accepted,
                denied.rejected
            ),
            (ModalStop::AttemptBudget, 0., y0.to_vec(), 0, 1)
        );
        let run = solve(&opts);
        let pref = Rodas5p::solve(&diagonal_problem(&rates), (0., 1.), &y0, &opts).unwrap();
        assert!(pref.is_success());
        assert_eq!(run.stop, ModalStop::ReachedEnd);
        assert!(run.accepted > 0 && run.rejected > 0);
        assert_eq!(run.time, 1.);
        println!(
            "accepted={} rejected={} modal={:?} protected={:?}",
            run.accepted,
            run.rejected,
            run.state,
            pref.final_state().unwrap()
        );
        for i in 0..rates.len() {
            let tol = [0., 2e-7, 2e-10][i];
            assert!((run.state[i] - y0[i] * rates[i].exp()).abs() <= tol);
            assert!((run.state[i] - pref.final_state().unwrap()[i]).abs() <= tol);
        }
    }

    #[test]
    fn fallback_keeps_coordinates_and_requested_output() {
        let opts = Options {
            dense_output: true,
            ..Default::default()
        };
        let outcome =
            VigilODE::solve_modal(ModalInput::Rates(&[-2.]), (0., 1.), &[1.], &opts).unwrap();
        match outcome {
            ModalOutcome::Protected {
                reason: ModalFallback::DenseOutput,
                solution,
            } => {
                assert!(solution.is_success());
                assert!((solution.at(0.5).unwrap()[0] - (-1.0f64).exp()).abs() < 1e-6);
            }
            _ => panic!("dense output must use the protected route"),
        }
        let general = diagonal_problem(&[0.5]);
        for (input, expected) in [
            (ModalInput::Rates(&[0.5]), ModalFallback::GrowingMode),
            (ModalInput::General(&general), ModalFallback::GeneralProblem),
        ] {
            match VigilODE::solve_modal(input, (0., 1.), &[1.], &Options::default()).unwrap() {
                ModalOutcome::Protected { reason, solution } => {
                    assert_eq!(reason, expected);
                    assert!((solution.final_state().unwrap()[0] - 0.5f64.exp()).abs() < 1e-6);
                }
                _ => panic!("unexpected modal dispatch"),
            }
        }
    }
}
