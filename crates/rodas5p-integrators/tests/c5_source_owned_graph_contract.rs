//! Actual C5 opt-in StageLinearSystem native integration: source-bound W and fallback gates.
//! Unrelated historical lib cfg(test) may fail before these tests; this
//! integration test compiles the production library (no cfg(test) on lib).
use rodas5p_core::{CoreError, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, StageLinearSystem,
    integrate_rodas5p_fast_client_observed_with_atol, integrate_rodas5p_fast_observed_with_atol,
    source_owned_graph_schur::{CooperativeCubicOwner, GraphSchurStageClient, GraphSchurTelemetry},
};
use std::sync::{Arc, Mutex};

fn orig() -> CooperativeCubicOwner {
    CooperativeCubicOwner {
        name: "wp3-original-cooperative-cubic".into(),
        epoch: 17,
        k: vec![4., 7.],
        g: vec![20., 40.],
        c: vec![vec![0., 12.], vec![8., 0.]],
        feedback: vec![0],
    }
}
fn original_problem(o: &CooperativeCubicOwner, alter: bool, epoch: u64) -> OdeProblem {
    let rhs_owner = o.clone();
    let jvp_owner = o.clone();
    OdeProblem::new(
        o.name.clone(),
        o.k.len(),
        Arc::new(move |_, y, out| {
            for i in 0..out.len() {
                let mut value = -rhs_owner.k[i] * y[i] - rhs_owner.g[i] * y[i] * y[i] * y[i];
                for j in 0..out.len() {
                    if j != i {
                        value += rhs_owner.c[i][j] * (y[j] - y[i]);
                    }
                }
                out[i] = if alter { value + 1. } else { value };
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, out| {
            for i in 0..out.len() {
                let mut value = -(jvp_owner.k[i] + 3. * jvp_owner.g[i] * y[i] * y[i]) * v[i];
                for j in 0..out.len() {
                    if j != i {
                        value += jvp_owner.c[i][j] * (v[j] - v[i]);
                    }
                }
                out[i] = value;
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
    .with_model_epoch(Arc::new(move || epoch))
}
fn client(o: CooperativeCubicOwner) -> GraphSchurStageClient {
    GraphSchurStageClient::new(o, Arc::new(Mutex::new(GraphSchurTelemetry::default()))).unwrap()
}

#[test]
fn original_unscaled_row_w_stage_residual() {
    let o = orig();
    let problem = original_problem(&o, false, 17);
    let mut candidate = client(o);
    let mut counters = WorkCounters::default();
    candidate
        .linearize(&problem, 0., &[0.2, 0.12], &mut counters)
        .unwrap();
    candidate.factor(300., &mut counters).unwrap();
    let mut stage = vec![1., -2.];
    candidate.solve_in_place(&mut stage, &mut counters).unwrap();
    let d0 = 300. + 4. + 12. + 3. * 20. * 0.2 * 0.2;
    let d1 = 300. + 7. + 8. + 3. * 40. * 0.12 * 0.12;
    assert!((d0 * stage[0] - 12. * stage[1] - 1.).abs() < 1e-12);
    assert!((-8. * stage[0] + d1 * stage[1] + 2.).abs() < 1e-12);
    assert_eq!(candidate.telemetry().certified_stage_solves, 1);
    assert_eq!(candidate.telemetry().fallback_stage_solves, 0);
    assert_eq!(counters.jvp_calls, 0);
}
#[test]
fn true_native_eight_stage_full_ivp_same_clipped_output_as_fast_direct() {
    let o = orig();
    let p = original_problem(&o, false, 17);
    let output = OutputSchedule::new((0..=64).map(|i| 0.1 * i as f64 / 64.).collect()).unwrap();
    let config = AdaptiveStepConfig {
        atol: 1e-8,
        rtol: 1e-6,
        initial_step: 0.1 / 64.,
        max_step: 0.1,
        max_attempts: 100000,
        ..AdaptiveStepConfig::default()
    };
    let (protected, _) = integrate_rodas5p_fast_observed_with_atol(
        &p,
        (0., 0.1),
        &[0.2, 0.12],
        &config,
        &output,
        &[1e-8, 1e-8],
    )
    .unwrap();
    let (got, _, _) = integrate_rodas5p_fast_client_observed_with_atol(
        &p,
        Box::new(client(o)),
        (0., 0.1),
        &[0.2, 0.12],
        &config,
        &output,
        &[1e-8, 1e-8],
    )
    .unwrap();
    assert!(protected.observed.success && got.observed.success);
    assert_eq!(got.observed.t, protected.observed.t);
    assert_eq!(got.observed.y.len(), 65);
    for (a, b) in got.observed.y.iter().zip(protected.observed.y.iter()) {
        for (x, y) in a.iter().zip(b.iter()) {
            assert!((x - y).abs() < 1e-12, "source-owned stage differs");
        }
    }
    assert_eq!(got.observed.counters.jvp_calls, 0);
}
#[test]
fn wrong_epoch_and_wrong_rhs_callback_refused() {
    let o = orig();
    let mut c = client(o.clone());
    let mut work = WorkCounters::default();
    let changed = original_problem(&o, false, 18);
    let err = c
        .linearize(&changed, 0., &[0.2, 0.12], &mut work)
        .unwrap_err();
    assert!(matches!(err, CoreError::InvalidInput(_)));
    let corrupted = original_problem(&o, true, 17);
    let err = c
        .linearize(&corrupted, 0., &[0.2, 0.12], &mut work)
        .unwrap_err();
    assert!(matches!(err, CoreError::InvalidInput(_)));
}
#[test]
fn unsupported_negative_source_and_non_fvs_graph_refused() {
    let mut owner = orig();
    owner.c[0][1] = -1.;
    assert!(
        GraphSchurStageClient::new(owner, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
            .is_err()
    );
    let mut owner = orig();
    owner.k = vec![1.; 6];
    owner.g = vec![1.; 6];
    owner.c = (0..6)
        .map(|i| (0..6).map(|j| if i == j { 0. } else { 1. }).collect())
        .collect();
    owner.feedback = vec![0, 1];
    assert!(
        GraphSchurStageClient::new(owner, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
            .is_err()
    );
}
#[test]
fn malformed_feedback_vertices_refused() {
    let mut o = orig();
    o.feedback = vec![0, 0];
    assert!(
        GraphSchurStageClient::new(o, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
            .is_err()
    );
    let mut o = orig();
    o.feedback = vec![0, 1, 2, 3];
    assert!(
        GraphSchurStageClient::new(o, Arc::new(Mutex::new(GraphSchurTelemetry::default())))
            .is_err()
    );
}

#[test]
fn paired_factory_binds_immutable_source_and_rust_jvp() {
    use rodas5p_integrators::source_owned_graph_schur::source_owned_cooperative_cubic_problem;
    let source = orig();
    let (problem, mut client) = source_owned_cooperative_cubic_problem(
        source.clone(),
        Arc::new(Mutex::new(GraphSchurTelemetry::default())),
    )
    .unwrap();
    assert_eq!(problem.model_epoch(), Some(17));
    let reference = original_problem(&source, false, 17);
    for y in [&[0.2, 0.12][..], &[0.21, 0.1][..], &[0.01, 0.08][..]] {
        let native = problem
            .eval_rhs(0., y, &mut WorkCounters::default())
            .unwrap();
        let verified = reference
            .eval_rhs(0., y, &mut WorkCounters::default())
            .unwrap();
        assert_eq!(native, verified);
    }
    let mut counters = WorkCounters::default();
    client
        .linearize(&problem, 0., &[0.2, 0.12], &mut counters)
        .unwrap();
    client.factor(300., &mut counters).unwrap();
    let mut v = vec![1., -2.];
    client.solve_in_place(&mut v, &mut counters).unwrap();
    assert_eq!(client.telemetry().certified_stage_solves, 1);
    assert_eq!(counters.jvp_calls, 0);
}

#[test]
fn native_ring12_ring24_path_scan_executes_real_compositions() {
    use rodas5p_integrators::source_owned_graph_schur::source_owned_cooperative_cubic_problem;
    for n in [12_usize, 24] {
        let mut c = vec![vec![0.; n]; n];
        for i in 1..n {
            c[i][i - 1] = 24.;
        }
        c[0][n - 1] = 24.;
        let src = CooperativeCubicOwner {
            name: format!("C5_synthetic_ring_{n}"),
            epoch: 17,
            k: (0..n).map(|i| 4. + (i % 4) as f64).collect(),
            g: vec![2.; n],
            c,
            feedback: vec![0],
        };
        let shared = Arc::new(Mutex::new(GraphSchurTelemetry::default()));
        let (owned, client) =
            source_owned_cooperative_cubic_problem(src.clone(), Arc::clone(&shared)).unwrap();
        let ref_ode = original_problem(&src, false, 17);
        let y0 = (0..n)
            .map(|i| 0.01 + 0.005 * (i % 4) as f64)
            .collect::<Vec<_>>();
        let grid = OutputSchedule::new(vec![0., 0.01, 0.02]).unwrap();
        let ctl = AdaptiveStepConfig {
            rtol: 1e-6,
            atol: 1e-8,
            initial_step: 0.01,
            max_step: 0.02,
            ..AdaptiveStepConfig::default()
        };
        let atol = vec![1e-8; n];
        let (gold, _) = integrate_rodas5p_fast_observed_with_atol(
            &ref_ode,
            (0., 0.02),
            &y0,
            &ctl,
            &grid,
            &atol,
        )
        .unwrap();
        let (tryrun, _, _) = integrate_rodas5p_fast_client_observed_with_atol(
            &owned,
            Box::new(client),
            (0., 0.02),
            &y0,
            &ctl,
            &grid,
            &atol,
        )
        .unwrap();
        assert!(gold.observed.success && tryrun.observed.success);
        assert_eq!(tryrun.observed.t, gold.observed.t);
        let discrepancy = tryrun
            .observed
            .y
            .iter()
            .zip(gold.observed.y.iter())
            .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()))
            .fold(0_f64, f64::max);
        assert!(discrepancy < 1e-11, "n={n} discrepancy {discrepancy}");
        let telemetry = shared.lock().unwrap().clone();
        assert!(
            telemetry.affine_pair_compositions > 0,
            "actual scan never ran"
        );
        assert!(telemetry.certified_stage_solves > 0);
        assert_eq!(telemetry.fallback_stage_solves, 0);
    }
}
