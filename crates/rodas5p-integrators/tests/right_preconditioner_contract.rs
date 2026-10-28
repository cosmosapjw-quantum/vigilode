use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, DenseOperator, LinearOperator, LuFactorization,
    Preconditioner, PreconditionerSide, WorkCounters, safe_l2,
};
use rodas5p_integrators::StructureError;
use rodas5p_integrators::gray_scott::gray_scott_2d;
use rodas5p_krylov::{
    GcrodrConfig, GcrodrSolveOptions, GcrodrState, GcrodrWorkspace, GmresCapacity, GmresConfig,
    GmresIntoOptions, GmresWorkspace, LgmresConfig, LgmresIntoWorkspace, LgmresState,
    solve_gcrodr_with_workspace_and_options, solve_gmres_into, solve_gmres_into_with_options,
    solve_lgmres, solve_lgmres_into,
};

struct RightLuPreconditioner {
    lu: LuFactorization,
    dimension: usize,
}

impl Preconditioner for RightLuPreconditioner {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let sol = self.lu.solve(x)?;
        y.copy_from_slice(&sol);
        Ok(())
    }

    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Right
    }
}

struct LeftLuPreconditioner {
    lu: LuFactorization,
    dimension: usize,
}

impl Preconditioner for LeftLuPreconditioner {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        let sol = self.lu.solve(x)?;
        y.copy_from_slice(&sol);
        Ok(())
    }

    fn side(&self) -> PreconditionerSide {
        PreconditionerSide::Left
    }
}

#[test]
fn test_f2_right_preconditioner_gmres_into_solves_accurately() {
    let n = 10;
    // Build diagonally dominant matrix W
    let mut w_mat = DenseMatrix::identity(n);
    for i in 0..n {
        for j in 0..n {
            if i != j {
                w_mat[(i, j)] = 0.05 * ((i + j) as f64).cos();
            } else {
                w_mat[(i, j)] = 2.0;
            }
        }
    }

    // Build preconditioner P = diag(W)
    let mut p_mat = DenseMatrix::zeros(n, n);
    for i in 0..n {
        p_mat[(i, i)] = w_mat[(i, i)];
    }
    let p_lu = LuFactorization::new(&p_mat).unwrap();
    let right_pc = RightLuPreconditioner {
        lu: p_lu,
        dimension: n,
    };

    let op = DenseOperator::new(w_mat).unwrap();

    let mut rhs = vec![0.0f64; n];
    for (i, val) in rhs.iter_mut().enumerate() {
        *val = ((i + 1) as f64).sin();
    }

    let mut x = vec![0.0f64; n];
    let mut ws = GmresWorkspace::default();
    let config = GmresConfig {
        restart: 20,
        max_arnoldi: 50,
        rtol: 1e-12,
        atol: 1e-14,
    };
    let mut counters = WorkCounters::default();

    let report = solve_gmres_into(
        &op,
        &right_pc,
        &rhs,
        None,
        &config,
        None,
        &mut x,
        &mut ws,
        GmresCapacity::unbounded(),
        &mut counters,
    )
    .expect("Right preconditioned GMRES-into must succeed");

    assert!(report.residual_norm <= 1e-12);

    // Verify true unpreconditioned residual b - W x <= 1e-12
    let mut wx = vec![0.0f64; n];
    op.apply(&x, &mut wx).unwrap();
    let mut res = vec![0.0f64; n];
    for i in 0..n {
        res[i] = rhs[i] - wx[i];
    }
    let true_res_norm = safe_l2(&res);
    assert!(
        true_res_norm <= 1e-12,
        "True residual {true_res_norm:.3e} exceeds 1e-12"
    );
}

#[test]
fn test_f2_right_preconditioner_refused_by_gcrodr_and_lgmres() {
    let n = 6;
    let p_mat = DenseMatrix::identity(n);
    let p_lu = LuFactorization::new(&p_mat).unwrap();
    let right_pc = RightLuPreconditioner {
        lu: p_lu,
        dimension: n,
    };
    let op = DenseOperator::new(p_mat).unwrap();
    let rhs = vec![1.0; n];
    let mut out = vec![0.0; n];
    let mut counters = WorkCounters::default();

    // 1. GCRO-DR refusal
    let mut gcrodr_state = GcrodrState::default();
    let mut gcrodr_ws = GcrodrWorkspace::default();
    let gcrodr_cfg = GcrodrConfig::default();
    let gcrodr_err = solve_gcrodr_with_workspace_and_options(
        &op,
        &right_pc,
        &rhs,
        None,
        &gcrodr_cfg,
        &mut gcrodr_state,
        None,
        &mut gcrodr_ws,
        GcrodrSolveOptions::default(),
        &mut counters,
    );
    assert!(
        matches!(gcrodr_err, Err(CoreError::InvalidInput(_))),
        "GCRO-DR must return InvalidInput refusal on Right preconditioner"
    );

    // 2. LGMRES refusal
    let mut lgmres_state = LgmresState::default();
    let lgmres_cfg = LgmresConfig {
        inner_m: 10,
        max_outer: 10,
        outer_k: 2,
        rtol: 1e-6,
        atol: 1e-8,
    };
    let lgmres_err = solve_lgmres(
        &op,
        &right_pc,
        &rhs,
        None,
        &lgmres_cfg,
        &mut lgmres_state,
        &mut counters,
    );
    assert!(
        matches!(lgmres_err, Err(CoreError::InvalidInput(_))),
        "LGMRES must return InvalidInput refusal on Right preconditioner"
    );

    // 3. LGMRES-into refusal
    let mut lgmres_into_ws = LgmresIntoWorkspace::default();
    let lgmres_into_err = solve_lgmres_into(
        &op,
        &right_pc,
        &rhs,
        None,
        &lgmres_cfg,
        &mut lgmres_state,
        None,
        &mut out,
        &mut lgmres_into_ws,
        &mut counters,
    );
    assert!(
        matches!(lgmres_into_err, Err(CoreError::InvalidInput(_))),
        "LGMRES-into must return InvalidInput refusal on Right preconditioner"
    );
}

#[test]
fn test_f2_left_preconditioner_is_bitwise_identical() {
    let n = 8;
    let mut w_mat = DenseMatrix::identity(n);
    for i in 0..n {
        w_mat[(i, i)] = 3.0;
        if i + 1 < n {
            w_mat[(i, i + 1)] = -0.5;
            w_mat[(i + 1, i)] = -0.5;
        }
    }
    let p_lu = LuFactorization::new(&w_mat).unwrap();
    let left_pc = LeftLuPreconditioner {
        lu: p_lu,
        dimension: n,
    };
    let op = DenseOperator::new(w_mat).unwrap();
    let mut rhs = vec![0.0f64; n];
    for (i, val) in rhs.iter_mut().enumerate() {
        *val = (i + 1) as f64;
    }

    let config = GmresConfig {
        restart: 10,
        max_arnoldi: 20,
        rtol: 1e-10,
        atol: 1e-12,
    };

    // Run 1
    let mut x1 = vec![0.0f64; n];
    let mut ws1 = GmresWorkspace::default();
    let mut c1 = WorkCounters::default();
    let r1 = solve_gmres_into_with_options(
        &op,
        &left_pc,
        &rhs,
        None,
        &config,
        None,
        &mut x1,
        &mut ws1,
        GmresCapacity::unbounded(),
        GmresIntoOptions::default(),
        &mut c1,
    )
    .unwrap();

    // Run 2
    let mut x2 = vec![0.0f64; n];
    let mut ws2 = GmresWorkspace::default();
    let mut c2 = WorkCounters::default();
    let r2 = solve_gmres_into_with_options(
        &op,
        &left_pc,
        &rhs,
        None,
        &config,
        None,
        &mut x2,
        &mut ws2,
        GmresCapacity::unbounded(),
        GmresIntoOptions::default(),
        &mut c2,
    )
    .unwrap();

    for i in 0..n {
        assert_eq!(x1[i].to_bits(), x2[i].to_bits(), "x[{i}] bit identity");
    }
    assert_eq!(r1.residual_norm.to_bits(), r2.residual_norm.to_bits());
    assert_eq!(c1, c2);
}

#[test]
fn test_f3_declared_linear_part_verification_passes_and_detects_mismatch() {
    let (prob, y0) = gray_scott_2d(4).expect("Gray-Scott 2D N=4 must construct");
    let mut counters = WorkCounters::default();

    // Verification must pass on the authentic Gray-Scott 2D linear part
    prob.verify_linear_part(0.0, &y0, &mut counters)
        .expect("verify_linear_part must pass on gray-scott-2d");

    // Verification must charge counters
    assert!(counters.rhs_calls >= 2);

    // Negative test: tampered linear part values
    let mut sparse_a = prob.declared_linear_part().unwrap().as_ref().clone();
    sparse_a.values[0] += 1.0; // tamper

    let nonlinear_n = prob.declared_nonlinear_callback().unwrap();
    let tampered_prob = prob
        .clone()
        .with_linear_part(sparse_a, nonlinear_n)
        .unwrap();

    let err = tampered_prob.verify_linear_part(0.0, &y0, &mut counters);
    assert!(
        matches!(err, Err(StructureError::BandVerificationMismatch { .. })),
        "Tampered linear part must be rejected with BandVerificationMismatch"
    );
}

#[test]
fn test_f4_gray_scott_2d_equations_parameters_and_jvp_consistency() {
    let n_grid = 6;
    let (prob, y0) = gray_scott_2d(n_grid).expect("Gray-Scott 2D N=6 must construct");
    let n2 = n_grid * n_grid;
    let n = 2 * n2;
    assert_eq!(prob.dimension, n);

    // Check initial condition: u in [0.75, 1.0], v in [0.0, 0.25], sum == 1.0 everywhere
    for i in 0..n2 {
        let u = y0[i];
        let v = y0[n2 + i];
        assert!(
            (u + v - 1.0).abs() <= 1e-14,
            "Conservation: u + v == 1 at t=0"
        );
        assert!((0.75..=1.0).contains(&u));
        assert!((0.0..=0.25).contains(&v));
    }

    // JVP finite difference check
    let mut counters = WorkCounters::default();
    let mut f0 = vec![0.0f64; n];
    prob.eval_rhs_into(0.0, &y0, &mut f0, &mut counters)
        .unwrap();

    let mut delta = vec![0.0f64; n];
    for (i, val) in delta.iter_mut().enumerate() {
        *val = 1.0e-6 * ((i + 3) as f64).cos();
    }

    let mut y_pert = vec![0.0f64; n];
    for i in 0..n {
        y_pert[i] = y0[i] + delta[i];
    }
    let mut f_pert = vec![0.0f64; n];
    prob.eval_rhs_into(0.0, &y_pert, &mut f_pert, &mut counters)
        .unwrap();

    let mut jvp_exact = vec![0.0f64; n];
    (prob.jvp_callback().unwrap())(0.0, &y0, &delta, &mut jvp_exact).unwrap();

    let mut fd_jvp = vec![0.0f64; n];
    for i in 0..n {
        fd_jvp[i] = f_pert[i] - f0[i];
    }

    let mut diff = vec![0.0f64; n];
    for i in 0..n {
        diff[i] = fd_jvp[i] - jvp_exact[i];
    }

    let rel_err = safe_l2(&diff) / safe_l2(&jvp_exact);
    assert!(
        rel_err <= 1.0e-5,
        "JVP finite difference agreement relative error {rel_err:.3e} exceeds 1e-5"
    );
}
