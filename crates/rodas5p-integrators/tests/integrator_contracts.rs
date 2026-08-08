use rodas5p_core::{LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    IntegrationMethod, SabrConfig, StageHistory, integrate_adaptive, integrate_fixed,
    manufactured_vector_problem, prothero_robinson_problem, scalar_linear_problem, sequential_step,
};

fn max_abs_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

#[test]
fn scalar_linear_step_matches_exact_solution() {
    let (p, y0) = scalar_linear_problem(-100.0, 1.0);
    let mut counters = WorkCounters::default();
    let r = sequential_step(
        &p,
        0.0,
        &y0,
        1e-3,
        &LinearSolverConfig::default(),
        None,
        1e-9,
        1e-7,
        true,
        &mut counters,
    )
    .unwrap();
    let exact = p.exact(1e-3).unwrap();
    let tolerance = 2e-13 + 2e-11 * exact[0].abs();
    assert!(max_abs_diff(&r.y_new, &exact) <= tolerance);
    assert_eq!(r.stages.len(), 8);
}

#[test]
fn rust_krylov_stage_solvers_match_the_same_direct_rodas5p_oracle() {
    let (p, y0) = manufactured_vector_problem(12, 1e3, 0.0, 0.02, 0.0).unwrap();
    let direct_cfg = LinearSolverConfig {
        method: LinearMethod::Direct,
        ..Default::default()
    };
    let mut c = WorkCounters::default();
    let reference = sequential_step(
        &p,
        0.0,
        &y0,
        0.005,
        &direct_cfg,
        None,
        1e-9,
        1e-7,
        true,
        &mut c,
    )
    .unwrap();
    for method in [
        LinearMethod::Gmres,
        LinearMethod::Lgmres,
        LinearMethod::Gcrodr,
    ] {
        let cfg = LinearSolverConfig {
            method,
            rtol: 1e-10,
            atol: 1e-12,
            restart: 20,
            maxiter: 100,
            inner_m: 20,
            recycle_dim: 6,
            ..Default::default()
        };
        let mut counters = WorkCounters::default();
        let got = sequential_step(
            &p,
            0.0,
            &y0,
            0.005,
            &cfg,
            None,
            1e-9,
            1e-7,
            true,
            &mut counters,
        )
        .unwrap();
        assert!(
            max_abs_diff(&got.y_new, &reference.y_new) < 5e-8,
            "method {method:?}"
        );
    }
}

#[test]
fn fixed_step_sequential_rodas5p_is_fifth_order() {
    let (p, y0) = manufactured_vector_problem(6, 80.0, 10.0, 0.0, 0.0).unwrap();
    let hs = [0.08, 0.04, 0.02, 0.01];
    let mut errs = Vec::new();
    for h in hs {
        let r = integrate_fixed(
            &p,
            (0.0, 0.4),
            &y0,
            h,
            IntegrationMethod::Sequential,
            None,
            None,
            1e-13,
            1e-11,
        )
        .unwrap();
        errs.push(max_abs_diff(r.y.last().unwrap(), &p.exact(0.4).unwrap()));
    }
    let orders: Vec<f64> = errs
        .windows(2)
        .map(|e| (e[0] / e[1]).ln() / 2f64.ln())
        .collect();
    assert!(
        orders[1..].iter().copied().sum::<f64>() / 2.0 > 4.6,
        "orders={orders:?}, errs={errs:?}"
    );
}

#[test]
fn adaptive_solver_rejects_overflowing_trial_and_recovers() {
    let (p, y0) = prothero_robinson_problem(-1e4, 1e10, 1.0);
    let r = integrate_adaptive(
        &p,
        (1.0, 1.05),
        &y0,
        0.05,
        IntegrationMethod::Sabr,
        None,
        Some(SabrConfig {
            max_iterations: 2,
            ..Default::default()
        }),
        1e-9,
        1e-6,
        10_000,
        0.05,
    )
    .unwrap();
    assert!(r.success);
    assert!(r.counters.rejected_steps > 0);
    assert!(max_abs_diff(r.y.last().unwrap(), &p.exact(1.05).unwrap()) < 2e-6);
}

#[test]
fn sabr_affine_fast_path_and_transactional_fallback_are_both_exercised() {
    let (p, y0) = manufactured_vector_problem(6, 80.0, 0.0, 0.0, 0.0).unwrap();
    let mut history = StageHistory::default();
    let mut counters = WorkCounters::default();
    let r = rodas5p_integrators::sabr_step(
        &p,
        0.0,
        &y0,
        0.01,
        &SabrConfig {
            max_iterations: 4,
            ..Default::default()
        },
        None,
        &mut history,
        None,
        1e-9,
        1e-7,
        true,
        &mut counters,
    )
    .unwrap();
    assert!(
        !r.used_fallback,
        "affine-in-state problem should close on fast path"
    );

    let (pn, yn) = manufactured_vector_problem(6, 80.0, 1e8, 0.0, 0.0).unwrap();
    let mut poisoned = StageHistory::default();
    poisoned.push(0.01, vec![vec![1e200; 6]; 8]);
    let mut c2 = WorkCounters::default();
    let r2 = rodas5p_integrators::sabr_step(
        &pn,
        0.0,
        &yn,
        0.01,
        &SabrConfig {
            max_iterations: 1,
            ..Default::default()
        },
        None,
        &mut poisoned,
        None,
        1e-9,
        1e-7,
        true,
        &mut c2,
    )
    .unwrap();
    assert!(r2.used_fallback);
    assert!(r2.y_new.iter().all(|x| x.is_finite()));
}
