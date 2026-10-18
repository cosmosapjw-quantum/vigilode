use rodas5p_core::{DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::collective_direct::{CollectiveDirectWork, CollectiveStageFactors};
use rodas5p_integrators::collective_model::CollectiveModel;
use rodas5p_integrators::collective_rodas5p::{
    CollectiveRodas5pWorkspace, integrate_collective_rodas5p_observed,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
    sequential_step,
};
use std::sync::Arc;

fn model(n: usize, chi: f64, kappa: f64) -> CollectiveModel {
    CollectiveModel::new(
        (0..n).map(|j| -0.8 + 1.6 * j as f64 / n as f64).collect(),
        vec![1.0 / n as f64; n],
        (0..n).map(|j| (0.25 + 0.002 * j as f64, -0.1)).collect(),
        1.2,
        chi,
        0.0,
        kappa,
    )
    .unwrap()
}
fn dense_problem(m: &CollectiveModel) -> OdeProblem {
    let mm = m.clone();
    let rhs = m.clone();
    OdeProblem::new(
        "collective-dense-reference",
        m.dimension(),
        Arc::new(move |t, y, out| rhs.rhs(t, y, out)),
        None,
        Some(Arc::new(move |t, y| {
            let n = mm.dimension();
            let mut j = DenseMatrix::zeros(n, n);
            let mut e = vec![0.0; n];
            let mut v = vec![0.0; n];
            for col in 0..n {
                e.fill(0.0);
                e[col] = 1.0;
                mm.jvp(t, y, &e, &mut v)?;
                for row in 0..n {
                    j[(row, col)] = v[row];
                }
            }
            Ok(j)
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn all_eight_native_stages_match_dense_direct_reference() {
    let config = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..Default::default()
    };
    for n in [1, 2, 4, 8, 16] {
        for chi in [-0.7, 0.0, 0.9] {
            for kappa in [0.0, 0.6] {
                for h in [1e-3, 0.07] {
                    let m = model(n, chi, kappa);
                    let problem = dense_problem(&m);
                    let mut y = vec![0.0; 2 * n];
                    m.initial_state(&mut y).unwrap();
                    let mut workspace = CollectiveRodas5pWorkspace::new(m).unwrap();
                    let mut out = vec![0.0; 2 * n];
                    let mut work = WorkCounters::default();
                    let error = workspace
                        .step_into(0.0, &y, h, 1e-10, 1e-7, &mut out, &mut work)
                        .unwrap();
                    let reference = sequential_step(
                        &problem,
                        0.0,
                        &y,
                        h,
                        &config,
                        None,
                        1e-10,
                        1e-7,
                        false,
                        &mut WorkCounters::default(),
                    )
                    .unwrap();
                    for i in 0..8 {
                        for (a, b) in workspace.stage(i).unwrap().iter().zip(&reference.stages[i]) {
                            assert!(
                                (a - b).abs() < 2e-12,
                                "N={n}, chi={chi}, kappa={kappa}, h={h}, stage={i}: {a} vs {b}"
                            );
                        }
                    }
                    for (a, b) in out.iter().zip(&reference.y_new) {
                        assert!((a - b).abs() < 2e-12);
                    }
                    assert!(
                        (error - reference.error_norm).abs() < 2e-5 * (1.0 + reference.error_norm)
                    );
                    assert_eq!(work.direct_factorizations, 1);
                    assert_eq!(work.direct_solve_calls, 8);
                    assert_eq!(work.rhs_evaluations, 8);
                }
            }
        }
    }
}

#[test]
fn refactorization_reuses_storage_and_failure_invalidates_old_factors() {
    let m = model(4, 0.7, 0.1);
    let mut y = vec![0.0; 8];
    m.initial_state(&mut y).unwrap();
    let mut work = CollectiveDirectWork::default();
    let mut factors = CollectiveStageFactors::factorize(&m, &y, 0.1, &mut work).unwrap();
    let rhs = [0.2; 8];
    let mut out = [0.0; 8];
    for k in 0..8 {
        factors
            .refactor(&m, &y, 0.05 + 0.01 * k as f64, &mut work)
            .unwrap();
        factors.solve_into(&rhs, &mut out, &mut work).unwrap();
    }
    assert_eq!(work.setup_allocations, 3);
    let high = model(4, 1e6, 0.1);
    assert!(factors.refactor(&high, &y, 1.0, &mut work).is_err());
    assert!(!factors.is_valid());
    assert!(factors.schur_determinant_enclosure().is_err());
    assert!(factors.solve_into(&rhs, &mut out, &mut work).is_err());
    factors.refactor(&m, &y, 0.1, &mut work).unwrap();
    factors.solve_into(&rhs, &mut out, &mut work).unwrap();
    assert!(factors.is_valid());
}

#[test]
fn invalid_attempt_is_transactional_and_hides_stale_stages() {
    let m = model(2, 0.3, 0.0);
    let mut y = vec![0.0; 4];
    m.initial_state(&mut y).unwrap();
    let mut workspace = CollectiveRodas5pWorkspace::new(m).unwrap();
    let mut out = [0.0; 4];
    let mut counters = WorkCounters::default();
    workspace
        .step_into(0.0, &y, 0.01, 1e-10, 1e-7, &mut out, &mut counters)
        .unwrap();
    let saved = out;
    assert!(
        workspace
            .step_into(0.0, &y, -0.01, 1e-10, 1e-7, &mut out, &mut counters)
            .is_err()
    );
    assert_eq!(saved, out);
    assert!(workspace.stage(0).is_none());
    assert!(workspace.embedded_error().is_none());
    assert!(
        workspace
            .step_into(0.0, &y, 0.01, 0.0, 0.0, &mut out, &mut counters)
            .is_err()
    );
    assert_eq!(saved, out);
}

#[test]
fn adaptive_landed_outputs_match_nonlinear_one_mode_exact_flow_and_fast_driver() {
    for kappa in [0.0, 0.6] {
        let m = CollectiveModel::new(
            vec![0.3],
            vec![1.0],
            vec![(0.4, -0.2)],
            1.2,
            0.8,
            0.0,
            kappa,
        )
        .unwrap();
        let y = [0.4, -0.2];
        let config = AdaptiveStepConfig {
            atol: 1e-12,
            rtol: 1e-9,
            initial_step: 0.03,
            max_step: 0.2,
            ..Default::default()
        };
        let output = OutputSchedule::uniform(0.0, 1.0, 0.1).unwrap();
        let result =
            integrate_collective_rodas5p_observed(&m, (0.0, 1.0), &y, &config, &output).unwrap();
        let reference =
            integrate_rodas5p_fast_observed(&dense_problem(&m), (0.0, 1.0), &y, &config, &output)
                .unwrap();
        assert!(result.observed.success && reference.observed.success);
        assert_eq!(result.observed.t, output.times());
        for ((&t, actual), other) in result
            .observed
            .t
            .iter()
            .zip(&result.observed.y)
            .zip(&reference.observed.y)
        {
            let theta = if kappa == 0.0 {
                t
            } else {
                -(-2.0 * kappa * t).exp_m1() / (2.0 * kappa)
            };
            let phase = 0.36 * t + 0.8 * 0.2 * theta;
            let amp = (-kappa * t).exp();
            let expected = [
                amp * (0.4 * phase.cos() + 0.2 * phase.sin()),
                amp * (0.4 * phase.sin() - 0.2 * phase.cos()),
            ];
            for q in 0..2 {
                assert!(
                    (actual[q] - expected[q]).abs() < 2e-9,
                    "kappa={kappa},t={t}: {actual:?} vs {expected:?}"
                );
                assert!((actual[q] - other[q]).abs() < 2e-9);
            }
        }
    }
}

#[test]
fn model_admission_and_zero_span_are_checked() {
    let bad =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.5, 0.0)], 0.0, 0.0, 1.0, 0.0).unwrap();
    assert!(CollectiveRodas5pWorkspace::new(bad).is_err());
    let m = model(1, 0.0, 0.0);
    let y = [0.25, -0.1];
    let schedule = OutputSchedule::new(vec![0.0]).unwrap();
    let ok = integrate_collective_rodas5p_observed(
        &m,
        (0.0, 0.0),
        &y,
        &AdaptiveStepConfig::default(),
        &schedule,
    )
    .unwrap();
    assert!(ok.observed.success);
    assert_eq!(ok.attempts, 0);
    assert_eq!(ok.observed.y[0], y);
    let badcfg = AdaptiveStepConfig {
        atol: f64::NAN,
        ..Default::default()
    };
    assert!(integrate_collective_rodas5p_observed(&m, (0.0, 0.0), &y, &badcfg, &schedule).is_err());
}
