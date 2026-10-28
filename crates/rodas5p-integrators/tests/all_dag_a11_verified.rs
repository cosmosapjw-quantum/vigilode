use rodas5p_core::{WorkCounters, directed::Interval};
use rodas5p_integrators::gray_scott::gray_scott_2d;
use rodas5p_integrators::gray_scott_verified::{BoundGrayScottProblem, gray_scott_2d_verified};
use rodas5p_integrators::{
    AdaptiveStepConfig, ContinuousResidualProvider, DenseSegmentWork, integrate_verified_residual,
};

fn config(atol: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol,
        rtol: 1e-7,
        initial_step: 0.025,
        max_step: 0.025,
        max_attempts: 20000,
        ..AdaptiveStepConfig::default()
    }
}

// Independently written coordinate-loop equation, not the provider or point callback.
fn reference_rhs(grid: usize, y: &[f64]) -> Vec<f64> {
    let cells = grid * grid;
    let dx = 1.0 / grid as f64;
    let inv = 1.0 / (dx * dx);
    let mut result = vec![0.0; 2 * cells];
    for row in 0..grid {
        for col in 0..grid {
            let index = row * grid + col;
            let neighbors = [
                row * grid + (col + grid - 1) % grid,
                row * grid + (col + 1) % grid,
                ((row + 1) % grid) * grid + col,
                ((row + grid - 1) % grid) * grid + col,
            ];
            let u = y[index];
            let v = y[cells + index];
            let lap_u = (y[neighbors[0]] + y[neighbors[1]] + y[neighbors[2]] + y[neighbors[3]]
                - 4.0 * u)
                * inv;
            let lap_v = (y[cells + neighbors[0]]
                + y[cells + neighbors[1]]
                + y[cells + neighbors[2]]
                + y[cells + neighbors[3]]
                - 4.0 * v)
                * inv;
            result[index] = 0.002 * lap_u - u * v * v + 0.04 * (1.0 - u);
            result[cells + index] = 0.001 * lap_v + u * v * v - (0.04 + 0.06) * v;
        }
    }
    result
}

fn reference_advance(grid: usize, y: &mut [f64], h: f64, count: usize) {
    for _ in 0..count {
        let k1 = reference_rhs(grid, y);
        let stage: Vec<_> = y.iter().zip(&k1).map(|(a, b)| a + h * 0.5 * b).collect();
        let k2 = reference_rhs(grid, &stage);
        let stage: Vec<_> = y.iter().zip(&k2).map(|(a, b)| a + h * 0.5 * b).collect();
        let k3 = reference_rhs(grid, &stage);
        let stage: Vec<_> = y.iter().zip(&k3).map(|(a, b)| a + h * b).collect();
        let k4 = reference_rhs(grid, &stage);
        for i in 0..y.len() {
            y[i] += h * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0;
        }
    }
}

#[test]
fn gray_scott_verified_homogeneous_exact_decay_with_inherited_radius() {
    let (owner, _) = gray_scott_2d_verified(3).unwrap();
    let mut initial = vec![0.7; 18];
    initial[9..].fill(0.0);
    let times: Vec<_> = (0..=8).map(|k| k as f64 / 40.0).collect();
    let cfg = config(1e-5);
    let run =
        integrate_verified_residual(&owner, (0.0, 0.2), &initial, 1e-8, &times, &cfg).unwrap();
    assert!(run.success, "{:?} attempts={}", run.failure, run.attempts);
    for output in run.outputs() {
        let exact = 1.0 - (1.0 - 0.7) * (-0.04 * output.time()).exp();
        let error = output.center()[..9]
            .iter()
            .map(|u| (u - exact).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            error <= output.radius_l2(),
            "error={error:e} bound={:e}",
            output.radius_l2()
        );
        assert!(output.radius_l2() <= cfg.atol);
    }
    for pair in run.segments().windows(2) {
        assert_eq!(
            pair[0].diagnostics().endpoint_radius,
            pair[1].diagnostics().initial_radius
        );
    }
    println!(
        "A11_EXACT accepted={} rejected={} provider_ops={}",
        run.segments().len(),
        run.rejected_attempts,
        run.verification_work.provider_operations
    );
}

#[test]
fn gray_scott_nonlinear_nonuniform_trajectory_matches_independent_rk4() {
    let (owner, mut initial) = gray_scott_2d_verified(4).unwrap();
    // Nonzero u*v^2 in every cell plus nonconstant diffusion stencil.
    for i in 0..16 {
        initial[i] = 0.7 + 0.01 * (i % 5) as f64;
        initial[16 + i] = 0.2 + 0.005 * (i % 3) as f64;
    }
    let times: Vec<_> = (0..=10).map(|k| k as f64 / 100.0).collect();
    let cfg = config(1e-4);
    let run = integrate_verified_residual(&owner, (0.0, 0.1), &initial, 0.0, &times, &cfg).unwrap();
    assert!(run.success, "{:?} attempts={}", run.failure, run.attempts);
    let mut reference = initial.clone();
    let mut finer = initial;
    let mut max_error = 0.0_f64;
    let mut max_spread = 0.0_f64;
    for (index, output) in run.outputs().iter().enumerate() {
        if index > 0 {
            reference_advance(4, &mut reference, 0.0001, 100);
            reference_advance(4, &mut finer, 0.00005, 200);
        }
        let error = output
            .center()
            .iter()
            .zip(&finer)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        let spread = reference
            .iter()
            .zip(&finer)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        max_error = max_error.max(error);
        max_spread = max_spread.max(spread);
        assert!(error <= output.radius_l2());
        assert!(spread < 1e-12);
        assert!(output.radius_l2() <= cfg.atol);
    }
    assert!(run.verification_work.interval_rhs_requests > 0);
    assert!(run.verification_work.one_sided_log_norm_requests > 0);
    println!(
        "A11_NONLINEAR outputs={} accepted={} rejected={} max_error={max_error:e} ref_spread={max_spread:e} provider_ops={}",
        run.outputs().len(),
        run.segments().len(),
        run.rejected_attempts,
        run.verification_work.provider_operations
    );
}

#[test]
fn fused_callbacks_preserve_stencil_values_and_interval_rhs_encloses_samples() {
    for grid in [2, 3, 4] {
        let (owner, _) = BoundGrayScottProblem::new(grid).unwrap();
        let (problem, _) = gray_scott_2d(grid).unwrap();
        let state: Vec<_> = (0..2 * grid * grid)
            .map(|i| 0.1 + 0.03 * (i % 13) as f64)
            .collect();
        let expected = reference_rhs(grid, &state);
        let actual = problem
            .eval_rhs(0.0, &state, &mut WorkCounters::default())
            .unwrap();
        assert_eq!(actual, expected);
        let boxes: Vec<_> = state
            .iter()
            .map(|v| Interval::new(v - 1e-4, v + 1e-4).unwrap())
            .collect();
        let mut rhs = vec![Interval::point(0.0).unwrap(); state.len()];
        let mut work = DenseSegmentWork::default();
        owner
            .enclose_rhs(
                Interval::new(0.0, 0.1).unwrap(),
                &boxes,
                &mut rhs,
                &mut work,
            )
            .unwrap();
        for sign in [-1.0, 0.0, 1.0] {
            let sample: Vec<_> = state
                .iter()
                .enumerate()
                .map(|(i, v)| v + sign * if i % 2 == 0 { 1e-4 } else { -1e-4 })
                .collect();
            for (bound, value) in rhs.iter().zip(reference_rhs(grid, &sample)) {
                assert!(bound.contains(value));
            }
        }
        let direction: Vec<_> = (0..state.len()).map(|i| 0.001 * (i as f64 - 3.0)).collect();
        let mut jvp = vec![0.0; state.len()];
        problem.jvp_callback().unwrap()(0.0, &state, &direction, &mut jvp).unwrap();
        let epsilon = 1e-4;
        let plus: Vec<_> = state
            .iter()
            .zip(&direction)
            .map(|(a, b)| a + epsilon * b)
            .collect();
        let minus: Vec<_> = state
            .iter()
            .zip(&direction)
            .map(|(a, b)| a - epsilon * b)
            .collect();
        for ((value, plus), minus) in jvp
            .iter()
            .zip(reference_rhs(grid, &plus))
            .zip(reference_rhs(grid, &minus))
        {
            assert!((value - (plus - minus) / (2.0 * epsilon)).abs() < 1e-10);
        }
    }
}

#[test]
fn interval_refusals_keep_attempted_work_and_grid_overflow_is_typed() {
    assert!(BoundGrayScottProblem::new(usize::MAX).is_err());
    let (owner, _) = BoundGrayScottProblem::new(2).unwrap();
    let mut tube = vec![Interval::point(0.2).unwrap(); 8];
    tube[4] = Interval::point(1e200).unwrap();
    let mut work = DenseSegmentWork::default();
    assert!(
        owner
            .one_sided_log_norm_upper(Interval::point(0.0).unwrap(), &tube, &mut work)
            .is_err()
    );
    assert!(work.provider_operations > 10);
    let mut rhs = vec![Interval::point(0.0).unwrap(); 8];
    tube[0] = Interval {
        lo: f64::NAN,
        hi: 0.0,
    };
    let previous = work.provider_operations;
    assert!(
        owner
            .enclose_rhs(Interval::point(0.0).unwrap(), &tube, &mut rhs, &mut work)
            .is_err()
    );
    assert!(work.provider_operations > previous);
}

#[test]
fn full_symmetric_bound_charges_linear_work_in_cell_count() {
    let mut charges = Vec::new();
    for grid in [2, 3, 4] {
        let (owner, _) = BoundGrayScottProblem::new(grid).unwrap();
        let tube = vec![Interval::new(0.1, 0.3).unwrap(); 2 * grid * grid];
        let mut work = DenseSegmentWork::default();
        let upper = owner
            .one_sided_log_norm_upper(Interval::point(0.0).unwrap(), &tube, &mut work)
            .unwrap();
        assert!(upper.is_finite());
        charges.push((grid * grid, work.provider_operations));
    }
    let dc01 = charges[1].0 - charges[0].0;
    let dc12 = charges[2].0 - charges[1].0;
    assert_eq!(
        (charges[1].1 - charges[0].1) * dc12 as u64,
        (charges[2].1 - charges[1].1) * dc01 as u64
    );
    println!("A11_LOGNORM_WORK {:?}", charges);
}

#[test]
fn failed_gray_scott_trial_preserves_only_committed_prefix() {
    let (owner, initial) = gray_scott_2d_verified(4).unwrap();
    let cfg = AdaptiveStepConfig {
        atol: 1e-15,
        initial_step: 1.0,
        max_step: 1.0,
        max_attempts: 1,
        ..config(1e-15)
    };
    let run =
        integrate_verified_residual(&owner, (0.0, 1.0), &initial, 0.0, &[0.0, 0.5, 1.0], &cfg)
            .unwrap();
    assert!(!run.success);
    assert_eq!(run.rejected_attempts, 1);
    assert!(run.segments().is_empty());
    assert_eq!(run.outputs().len(), 1);
    assert!(run.solver_work.rhs_evaluations > 0);
    assert!(run.verification_work.provider_operations > 0);
}

#[test]
fn actual_gray_scott_homogeneous_mode_requires_offdiagonal_symmetric_part() {
    let grid = 3;
    let cells = grid * grid;
    let (owner, _) = BoundGrayScottProblem::new(grid).unwrap();
    let (problem, _) = gray_scott_2d(grid).unwrap();
    let mut state = vec![0.7; 2 * cells];
    state[cells..].fill(0.2);
    let tube: Vec<_> = state.iter().map(|v| Interval::point(*v).unwrap()).collect();
    let bound = owner
        .one_sided_log_norm_upper(
            Interval::point(0.0).unwrap(),
            &tube,
            &mut DenseSegmentWork::default(),
        )
        .unwrap();
    let (a, d, b) = (
        -0.2_f64 * 0.2 - 0.04,
        2.0 * 0.7 * 0.2 - (0.04 + 0.06),
        (0.2 * 0.2 - 2.0 * 0.7 * 0.2) / 2.0,
    );
    let eigenvalue = (a + d) / 2.0 + (((a - d) / 2.0).powi(2) + b * b).sqrt();
    let mut vector = vec![b; 2 * cells];
    vector[cells..].fill(eigenvalue - a);
    let mut product = vec![0.0; 2 * cells];
    problem.jvp_callback().unwrap()(0.0, &state, &vector, &mut product).unwrap();
    let rayleigh = vector.iter().zip(&product).map(|(x, y)| x * y).sum::<f64>()
        / vector.iter().map(|v| v * v).sum::<f64>();
    assert!((rayleigh - eigenvalue).abs() < 1e-14);
    assert!(bound >= eigenvalue);
    assert!(eigenvalue > a.max(d));
}
