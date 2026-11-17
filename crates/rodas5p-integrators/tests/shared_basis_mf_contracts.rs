use std::f64::consts::PI;
use std::sync::Arc;

use rodas5p_core::{CoreResult, DenseMatrix, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{AdaptiveStepConfig, OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace};
use rodas5p_krylov::shared_basis::SharedBasisConfig;

fn heat_exact_solution(n: usize, nu: f64, t: f64) -> Vec<f64> {
    let dx = 1.0 / (n + 1) as f64;
    let mode1_decay = (-nu * PI * PI * t).exp();
    let mode7_decay = (-49.0 * nu * PI * PI * t).exp();
    (0..n)
        .map(|i| {
            let x = (i + 1) as f64 * dx;
            mode1_decay * (PI * x).sin() + 0.5 * mode7_decay * (7.0 * PI * x).sin()
        })
        .collect()
}

fn heat_problem(n: usize, nu: f64) -> CoreResult<(OdeProblem, Vec<f64>, (f64, f64))> {
    let dx = 1.0 / (n + 1) as f64;
    let c = nu / (dx * dx);

    let y0: Vec<f64> = (0..n)
        .map(|i| {
            let x = (i + 1) as f64 * dx;
            (PI * x).sin() + 0.5 * (7.0 * PI * x).sin()
        })
        .collect();

    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let left = if i == 0 { 0.0 } else { y[i - 1] };
            let right = if i + 1 == n { 0.0 } else { y[i + 1] };
            out[i] = c * (left - 2.0 * y[i] + right);
        }
        Ok(())
    });

    let jacobian = Arc::new(move |_t: f64, _y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        for i in 0..n {
            j[(i, i)] = -2.0 * c;
            if i > 0 {
                j[(i, i - 1)] = c;
            }
            if i + 1 < n {
                j[(i, i + 1)] = c;
            }
        }
        Ok(j)
    });

    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let left = if i == 0 { 0.0 } else { v[i - 1] };
            let right = if i + 1 == n { 0.0 } else { v[i + 1] };
            out[i] = c * (left - 2.0 * v[i] + right);
        }
        Ok(())
    });

    let problem = OdeProblem::new(
        "heat_1d",
        n,
        rhs,
        None,
        Some(jacobian),
        Some(jvp),
        None,
        true,
        None,
        None,
    )?;

    Ok((problem, y0, (0.0, 0.05)))
}

fn rotation_problem() -> CoreResult<(OdeProblem, Vec<f64>, (f64, f64))> {
    let n = 2;
    let omega = 1.0;

    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -omega * y[1];
        out[1] = omega * y[0];
        Ok(())
    });

    let jacobian = Arc::new(move |_t: f64, _y: &[f64]| {
        let mut j = DenseMatrix::zeros(2, 2);
        j[(0, 0)] = 0.0;
        j[(0, 1)] = -omega;
        j[(1, 0)] = omega;
        j[(1, 1)] = 0.0;
        Ok(j)
    });

    let jvp = Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = -omega * v[1];
        out[1] = omega * v[0];
        Ok(())
    });

    let problem = OdeProblem::new(
        "rotation_2d",
        n,
        rhs,
        None,
        Some(jacobian),
        Some(jvp),
        None,
        true,
        None,
        None,
    )?;

    Ok((problem, vec![1.0, 0.0], (0.0, 1.0)))
}

#[test]
fn shared_basis_off_is_bit_identical() {
    let test_cases = vec![
        heat_problem(64, 1.0).expect("heat problem"),
        rotation_problem().expect("rotation problem"),
    ];

    for (problem, y0, t_span) in test_cases {
        let linear_cfg = LinearSolverConfig {
            method: LinearMethod::Gmres,
            rtol: 1.0e-6,
            atol: 1.0e-14,
            preconditioner: PreconditionerKind::None,
            ..LinearSolverConfig::default()
        };
        let adaptive = AdaptiveStepConfig {
            atol: 1.0e-8,
            rtol: 1.0e-6,
            initial_step: 1.0e-4,
            min_step: 1.0e-7,
            max_step: t_span.1 - t_span.0,
            max_attempts: 150,
            ..AdaptiveStepConfig::default()
        };
        let output = OutputSchedule::new(vec![t_span.0, t_span.1]).expect("valid output schedule");

        // Run 1: No setter call
        let mut work1 = Rodas5pMfFastWorkspace::new(&problem, &linear_cfg).expect("work1 create");
        let res1 = work1
            .integrate_observed(&problem, t_span, &y0, &adaptive, &output)
            .expect("work1 run");

        // Run 2: Explicit set_shared_basis(None)
        let mut work2 = Rodas5pMfFastWorkspace::new(&problem, &linear_cfg).expect("work2 create");
        work2.set_shared_basis(None).expect("disable shared basis");
        let res2 = work2
            .integrate_observed(&problem, t_span, &y0, &adaptive, &output)
            .expect("work2 run");

        assert_eq!(res1.observed.t.len(), res2.observed.t.len());
        assert_eq!(res1.observed.y.len(), res2.observed.y.len());

        for (t1, t2) in res1.observed.t.iter().zip(&res2.observed.t) {
            assert_eq!(t1.to_bits(), t2.to_bits(), "output times differ in bits");
        }

        for (y1, y2) in res1.observed.y.iter().zip(&res2.observed.y) {
            assert_eq!(y1.len(), y2.len());
            for (v1, v2) in y1.iter().zip(y2) {
                assert_eq!(v1.to_bits(), v2.to_bits(), "states differ in bits");
            }
        }

        assert_eq!(
            res1.observed.counters, res2.observed.counters,
            "counters differ between no-setter and set_shared_basis(None)"
        );
    }
}

#[test]
fn shared_basis_on_heat_uses_fewer_jvps_at_matched_steps() {
    let (problem, y0, t_span) = heat_problem(64, 1.0).expect("heat problem");

    let linear_cfg = LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1.0e-6,
        atol: 1.0e-14,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    };
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-4,
        min_step: 1.0e-7,
        max_step: t_span.1 - t_span.0,
        max_attempts: 150,
        ..AdaptiveStepConfig::default()
    };
    let output = OutputSchedule::new(vec![t_span.0, t_span.1]).expect("valid output schedule");

    // 1. GMRES baseline run
    let mut work_gmres = Rodas5pMfFastWorkspace::new(&problem, &linear_cfg).expect("gmres work");
    let res_gmres = work_gmres
        .integrate_observed(&problem, t_span, &y0, &adaptive, &output)
        .expect("gmres run");
    assert!(res_gmres.observed.success);

    // 2. Shared basis run
    let mut work_shared = Rodas5pMfFastWorkspace::new(&problem, &linear_cfg).expect("shared work");
    work_shared
        .set_shared_basis(Some(SharedBasisConfig {
            max_dim: 60,
            theta: 1.0e-3,
        }))
        .expect("enable shared basis");
    let res_shared = work_shared
        .integrate_observed(&problem, t_span, &y0, &adaptive, &output)
        .expect("shared run");
    assert!(res_shared.observed.success);

    // Criterion 1: Accepted steps within ±1 of GMRES run
    let diff_steps = (res_shared.observed.counters.accepted_steps as i64
        - res_gmres.observed.counters.accepted_steps as i64)
        .abs();
    assert!(
        diff_steps <= 1,
        "accepted steps difference {diff_steps} exceeds ±1 (shared={}, gmres={})",
        res_shared.observed.counters.accepted_steps,
        res_gmres.observed.counters.accepted_steps
    );

    // Criterion 2: Final error within 10x of GMRES run's error
    let exact_final = heat_exact_solution(64, 1.0, t_span.1);
    let y_gmres = res_gmres.observed.y.last().expect("last gmres point");
    let y_shared = res_shared.observed.y.last().expect("last shared point");

    let mut err_gmres = 0.0_f64;
    let mut err_shared = 0.0_f64;
    for i in 0..64 {
        err_gmres = err_gmres.max((y_gmres[i] - exact_final[i]).abs());
        err_shared = err_shared.max((y_shared[i] - exact_final[i]).abs());
    }
    assert!(
        err_shared <= 10.0 * err_gmres + 1.0e-10,
        "shared error {err_shared} exceeds 10x gmres error {err_gmres}"
    );

    // Criterion 3: Total JVPs at most half of GMRES run
    let jvp_gmres = res_gmres.observed.counters.jvp_vectors;
    let jvp_shared = res_shared.observed.counters.jvp_vectors;
    assert!(
        jvp_shared <= jvp_gmres / 2,
        "shared JVPs ({jvp_shared}) not at most half of GMRES JVPs ({jvp_gmres})"
    );
}

#[test]
fn shared_basis_workspace_configuration_and_accessors() {
    let (problem, _y0, _t_span) = rotation_problem().expect("rotation problem");
    let linear_cfg = LinearSolverConfig {
        method: LinearMethod::Gmres,
        ..LinearSolverConfig::default()
    };
    let mut work = Rodas5pMfFastWorkspace::new(&problem, &linear_cfg).expect("work create");

    assert!(work.shared_basis().is_none());

    // Invalid config rejected
    assert!(
        work.set_shared_basis(Some(SharedBasisConfig {
            max_dim: 0,
            theta: 1.0e-3,
        }))
        .is_err()
    );

    // Valid config accepted
    work.set_shared_basis(Some(SharedBasisConfig {
        max_dim: 20,
        theta: 1.0e-4,
    }))
    .expect("valid shared basis");

    assert!(work.shared_basis().is_some());
    assert_eq!(work.shared_basis().unwrap().dim(), 0);
    assert_eq!(
        work.shared_basis().unwrap().config(),
        SharedBasisConfig {
            max_dim: 20,
            theta: 1.0e-4,
        }
    );

    // Cleared back to None
    work.set_shared_basis(None).expect("clear shared basis");
    assert!(work.shared_basis().is_none());

    // Verify error-weight scaling (atol + rtol * |y_i|)^-1 with 3D y0 = [100.0, 2.0, 1.0] and atol = 1.0, rtol = 0.5:
    // - True weights: w = [1/51, 0.5, 1/1.5] -> 1D shared basis residual h_gamma * r_w ~ 0.003 <= theta (0.03), so jvp_vectors == 1.
    // - Mutant (+ -> -): w[1] = (1.0 - 0.5 * 2.0)^-1 = +inf -> r_w = +inf -> falls back to GMRES (jvp_vectors > 1).
    // - Mutant (* -> /): w[0] = (1.0 + 0.5 / 100.0)^-1 ~ 0.995 (50x larger) -> h_gamma * r_w ~ 0.14 > theta -> falls back to GMRES (jvp_vectors > 1).
    let rhs = Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
        out[0] = -y[0];
        out[1] = -y[1];
        out[2] = -1.0e4 * y[2];
        Ok(())
    });
    let jvp = Arc::new(|_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
        out[0] = -v[0];
        out[1] = -v[1];
        out[2] = -1.0e4 * v[2];
        Ok(())
    });
    let prob = OdeProblem::new(
        "weight_scaling_3d",
        3,
        rhs,
        None,
        None,
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .expect("weight_scaling_3d");
    let mut work_w = Rodas5pMfFastWorkspace::new(&prob, &linear_cfg).expect("work_w");
    work_w
        .set_shared_basis(Some(SharedBasisConfig {
            max_dim: 1,
            theta: 0.03,
        }))
        .expect("enable shared basis");
    let adaptive_w = AdaptiveStepConfig {
        atol: 1.0,
        rtol: 0.5,
        initial_step: 0.01,
        min_step: 1.0e-5,
        max_step: 0.01,
        max_attempts: 50,
        ..AdaptiveStepConfig::default()
    };
    let out_sched = OutputSchedule::new(vec![0.0, 0.01]).expect("schedule");
    let res_w = work_w
        .integrate_observed(
            &prob,
            (0.0, 0.01),
            &[100.0, 2.0, 1.0],
            &adaptive_w,
            &out_sched,
        )
        .expect("weight scaling run must succeed");
    assert!(res_w.observed.success);
    assert_eq!(
        res_w.observed.counters.jvp_vectors, 1,
        "shared basis with true error weights must converge at dim=1 (1 JVP total, no GMRES fallback)"
    );
}
