//! New integration-boundary regressions; no replay of an archived campaign.
#![cfg(feature = "audit2-research")]

use rodas5p_core::{CoreResult, rodas5p_coefficients};
use rodas5p_integrators::{
    ChartStatus, QuadraticStageProblem, StageChart, StageTarget,
    chart_transport::{ChartControllerConfig, ChartIdentity, chart_integrate},
    stage_chart_candidate,
};

#[derive(Clone, Copy)]
enum Chart {
    Identity,
    ShortForward,
    ShortJvp,
    NanJvp,
}
impl StageChart for Chart {
    fn name(&self) -> &str {
        "boundary-test"
    }
    fn contains(&self, _: &[f64]) -> bool {
        true
    }
    fn forward(&self, z: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(if matches!(self, Self::ShortForward) {
            vec![]
        } else {
            z.to_vec()
        })
    }
    fn inverse(&self, k: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(k.to_vec())
    }
    fn jvp(&self, _: &[f64], dz: &[f64]) -> CoreResult<Vec<f64>> {
        Ok(match self {
            Self::ShortJvp => vec![],
            Self::NanJvp => vec![f64::NAN; dz.len()],
            _ => dz.to_vec(),
        })
    }
}
fn stage_fixture() -> (StageTarget, QuadraticStageProblem, Vec<f64>) {
    let target = StageTarget::sequential(rodas5p_coefficients().unwrap()).unwrap();
    let z = vec![0.0; target.stages()];
    let problem = QuadraticStageProblem {
        jacobian: vec![vec![-1.0]],
        y: vec![1.0],
        h: 0.1,
        q: vec![0.0],
    };
    (target, problem, z)
}

#[test]
fn nan_problem_cannot_report_converged() {
    let (target, mut problem, z) = stage_fixture();
    problem.q[0] = f64::NAN;
    let result = stage_chart_candidate(&target, &problem, &Chart::Identity, &z, 2, 1e-13);
    assert!(matches!(
        result,
        Err(_)
            | Ok(rodas5p_integrators::ChartCandidate {
                status: ChartStatus::NonFinite,
                ..
            })
    ));
}

#[test]
fn finite_coefficients_with_nan_arithmetic_cannot_report_converged() {
    let (target, mut problem, z) = stage_fixture();
    // Both J*y and q*y^2 overflow: their difference is NaN although every
    // supplied coefficient is finite. Input validation alone is insufficient.
    problem.jacobian[0][0] = f64::MAX;
    problem.y[0] = 2.0;
    problem.q[0] = f64::MAX;
    let result = stage_chart_candidate(&target, &problem, &Chart::Identity, &z, 2, 1e-13);
    assert!(matches!(
        result,
        Ok(rodas5p_integrators::ChartCandidate {
            status: ChartStatus::NonFinite,
            ..
        })
    ));
}

#[test]
fn malformed_target_is_an_error_not_a_panic() {
    let (mut target, problem, z) = stage_fixture();
    target.coupling_rows.clear();
    assert!(stage_chart_candidate(&target, &problem, &Chart::Identity, &z, 2, 1e-13).is_err());
}

#[test]
fn wrong_forward_length_is_a_dimension_error() {
    let (target, problem, z) = stage_fixture();
    assert!(matches!(
        stage_chart_candidate(&target, &problem, &Chart::ShortForward, &z, 2, 1e-13),
        Err(rodas5p_core::CoreError::Dimension(_))
    ));
}

#[test]
fn wrong_jvp_length_is_a_dimension_error_not_a_panic() {
    let (target, problem, z) = stage_fixture();
    assert!(matches!(
        stage_chart_candidate(&target, &problem, &Chart::ShortJvp, &z, 2, 1e-13),
        Err(rodas5p_core::CoreError::Dimension(_))
    ));
}

#[test]
fn nonfinite_jvp_is_explicitly_nonfinite() {
    let (target, problem, z) = stage_fixture();
    let result = stage_chart_candidate(&target, &problem, &Chart::NanJvp, &z, 2, 1e-13);
    assert!(matches!(
        result,
        Err(rodas5p_core::CoreError::NonFinite(_))
            | Ok(rodas5p_integrators::ChartCandidate {
                status: ChartStatus::NonFinite,
                ..
            })
    ));
}

fn chart_fixture() -> (ChartIdentity, ChartControllerConfig) {
    (
        ChartIdentity {
            kappa: 40.0,
            eps: 0.0,
            branch: -1,
        },
        ChartControllerConfig {
            atol: 1.0,
            rtol: 0.0,
            initial_step: 1.0,
            min_step: 0.25,
            max_step: 1.0,
            denominator_min: 0.01,
        },
    )
}

#[test]
fn chart_refuses_before_advancing_at_an_unchanged_time() {
    let (identity, config) = chart_fixture();
    let t0 = 2.0_f64.powi(53);
    let run = chart_integrate(&identity, -1.0, 0.025, t0, t0 + 2.0, &[], &config).unwrap();
    assert!(run.refused.is_some());
    assert!(
        run.steps.is_empty(),
        "must refuse before advancing state at t + h == t"
    );
}

#[test]
fn chart_rejects_nonfinite_configuration_and_output_times() {
    let (identity, config) = chart_fixture();
    let invalid = ChartControllerConfig {
        atol: f64::INFINITY,
        ..config
    };
    assert!(chart_integrate(&identity, -1.0, 0.025, 0.0, 0.25, &[], &invalid).is_err());
    assert!(chart_integrate(&identity, -1.0, 0.025, 0.0, 0.25, &[f64::NAN], &config).is_err());
    assert!(chart_integrate(&identity, -1.0, 0.025, 0.0, f64::INFINITY, &[], &config).is_err());
}

#[test]
fn regular_chart_steps_still_reach_the_requested_output() {
    let (identity, config) = chart_fixture();
    let run = chart_integrate(&identity, -1.0, 0.025, 0.0, 0.5, &[0.25], &config).unwrap();
    assert!(run.refused.is_none());
    assert_eq!(run.outputs.len(), 2);
    assert_eq!(run.outputs[0].t, 0.25);
    assert_eq!(run.outputs[1].t, 0.5);
    assert!(run.steps.iter().all(|(start, end)| end.t > start.t));
}
