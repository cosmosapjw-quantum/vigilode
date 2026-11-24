#![cfg(feature = "structured")]
use std::f64::consts::TAU;
use vigilode::{
    VigilODE,
    graph_schur::{AdaptiveStepConfig, CooperativeCubicOwner, GraphPathScan, OutputSchedule},
};

fn ring(n: usize) -> CooperativeCubicOwner {
    let mut coupling = vec![vec![0.; n]; n];
    for (i, row) in coupling.iter_mut().enumerate() {
        row[(i + n - 1) % n] = 24.;
    }
    CooperativeCubicOwner {
        name: "facade-ring".into(),
        epoch: 3,
        k: vec![1.25; n],
        g: vec![0.; n],
        c: coupling,
        feedback: vec![0],
    }
}

#[test]
fn wave_on_directed_ring_checks_scan_direction_and_the_whole_output_grid() {
    let n = 33;
    let wave_number = TAU * 2. / n as f64;
    let y0 = (0..n)
        .map(|i| (wave_number * i as f64).cos())
        .collect::<Vec<_>>();
    let times = vec![0., 0.01, 0.03, 0.05];
    let output = OutputSchedule::new(times.clone()).unwrap();
    let controller = AdaptiveStepConfig {
        initial_step: 0.001,
        max_step: 0.01,
        rtol: 1e-8,
        atol: 1e-10,
        ..AdaptiveStepConfig::default()
    };
    let mut total = Vec::new();
    for scan in [
        GraphPathScan::Sequential,
        GraphPathScan::HillisSteele,
        GraphPathScan::Blelloch,
    ] {
        let result = VigilODE::solve_graph_cubic(
            ring(n),
            scan,
            (0., 0.05),
            &y0,
            &controller,
            &output,
            &vec![1e-10; n],
        )
        .unwrap();
        let trajectory = &result.integration.observed;
        assert!(trajectory.success && result.partial.is_none());
        assert_eq!(trajectory.t, times);
        let mut worst_error = 0.0_f64;
        for (&t, state) in trajectory.t.iter().zip(&trajectory.y) {
            let decay = ((-1.25 + 24. * (wave_number.cos() - 1.)) * t).exp();
            for (i, computed) in state.iter().enumerate() {
                let exact = decay * (wave_number * i as f64 - 24. * wave_number.sin() * t).cos();
                worst_error = worst_error.max((computed - exact).abs());
            }
        }
        assert!(worst_error < 1e-8);
        println!(
            "C5_FACADE_WAVE mode={scan:?} error={worst_error:e} stage_checks={} fallback={}",
            result.stages.original_residual_checks, result.stages.fallback_stage_solves
        );
        assert!(trajectory.counters.rhs_calls > 0);
        assert_eq!(trajectory.counters.jvp_calls, 0);
        assert!(result.stages.certified_stage_solves > 0);
        assert_eq!(
            result.stages.certified_stage_solves,
            result.stages.original_residual_checks
        );
        assert_eq!(result.stages.fallback_stage_solves, 0);
        total.push(result.stages.affine_pair_compositions);
    }
    assert!(total[0] < total[2] && total[2] < total[1], "{total:?}");
}

#[test]
fn unsupported_graph_is_refused_and_short_controller_budget_returns_partial_state() {
    let n = 8;
    let times = OutputSchedule::new(vec![0., 0.05]).unwrap();
    let controller = AdaptiveStepConfig {
        initial_step: 1e-4,
        max_attempts: 1,
        rtol: 1e-6,
        atol: 1e-8,
        ..AdaptiveStepConfig::default()
    };
    let atol = vec![1e-8; n];
    let initial = vec![1.; n];
    let mut unbroken = ring(n);
    unbroken.feedback.clear();
    let error = VigilODE::solve_graph_cubic(
        unbroken,
        GraphPathScan::Blelloch,
        (0., 0.05),
        &initial,
        &controller,
        &times,
        &atol,
    )
    .unwrap_err();
    assert!(error.to_string().contains("cycle not broken"));
    let run = VigilODE::solve_graph_cubic(
        ring(n),
        GraphPathScan::Blelloch,
        (0., 0.05),
        &initial,
        &controller,
        &times,
        &atol,
    )
    .unwrap();
    assert!(!run.integration.observed.success);
    let partial = run.partial.unwrap();
    assert_eq!(
        partial.reason,
        vigilode::graph_schur::DriverStopReason::AttemptBudget
    );
    assert!(partial.t > 0. && partial.t < 0.05);
    assert_eq!(partial.y.len(), n);
    assert!(run.stages.original_residual_checks > 0);
    assert_eq!(run.integration.attempts, 1);
}

#[test]
fn rounded_path_margin_switches_to_full_band_and_reports_the_work() {
    // For the downstream row, (shift+1+coupling) rounds to coupling.
    // The optional q<1 test must decline, although triangular W is nonsingular.
    let source = CooperativeCubicOwner {
        name: "rounded-margin".into(),
        epoch: 1,
        k: vec![1.; 2],
        g: vec![0.; 2],
        c: vec![vec![0., 0.], vec![1e24, 0.]],
        feedback: vec![],
    };
    let output = OutputSchedule::new(vec![0., 0.01]).unwrap();
    let config = AdaptiveStepConfig {
        initial_step: 0.01,
        max_step: 0.01,
        ..AdaptiveStepConfig::default()
    };
    let result = VigilODE::solve_graph_cubic(
        source,
        GraphPathScan::Blelloch,
        (0., 0.01),
        &[0., 0.],
        &config,
        &output,
        &[1e-8; 2],
    )
    .unwrap();
    assert!(result.integration.observed.success);
    assert_eq!(result.integration.observed.y.last().unwrap(), &[0., 0.]);
    assert!(result.stages.fallback_factorizations > 0);
    assert!(result.stages.fallback_stage_solves >= 8);
    assert_eq!(result.stages.certified_stage_solves, 0);
    assert!(result.integration.observed.counters.direct_solve_calls >= 8);
    println!(
        "C5_FACADE_FALLBACK factors={} stages={}",
        result.stages.fallback_factorizations, result.stages.fallback_stage_solves
    );
}
