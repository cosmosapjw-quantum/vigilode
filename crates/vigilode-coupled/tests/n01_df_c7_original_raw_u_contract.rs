//! C7 scientific oracle: the source-owned value-only BE/diagonal-RK stage
//! is NOT the original 8-stage RODAS5P raw-U target.
//!
//! This is TEST-ONLY: the dense J is an independent 3x3 correctness oracle.
//! The production derivative-free route, JVP priority, coefficients and defaults
//! are untouched. Do not promote a successful auxiliary BE certificate to raw-U.
use std::sync::Arc;

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, LinearSolverConfig, N01MassActionOwner, WorkCounters,
    rodas5p_coefficients,
};
use rodas5p_integrators::{OdeProblem, sequential_step};
use vigilode_coupled::n01_derivative_free::N01ValueStage;

fn manufactured() -> N01MassActionOwner {
    N01MassActionOwner::new(
        [0.5, 1.0, 0.5],
        [35.0 / 64.0, 27.0 / 128.0, 31.0 / 128.0],
        [1.0; 3],
        0.25,
        [1.0; 3],
        [1.0; 3],
        0.0,
    )
    .unwrap()
}

#[test]
fn exact_be_certificate_does_not_certify_first_raw_u_stage() {
    let source = manufactured();
    let y = [35.0 / 64.0, 27.0 / 128.0, 31.0 / 128.0];
    let trial = source.freeze();
    let be = N01ValueStage::new(&source, &trial, 0.25).unwrap();
    let mut z = [99.0; 3];
    let cert = be.try_commit(0.25, 1e-12, 1e-12, &mut z).unwrap();
    assert!(cert.accepted, "exact BE stage must be admitted: {cert:?}");
    assert_eq!(z, [0.5, 0.25, 0.25]);

    let tau = 0.25;
    let d: [f64; 3] = std::array::from_fn(|i| z[i] - y[i]);
    let mut f0 = [0.0; 3];
    source.rhs_at(&y, &mut f0).unwrap();
    let j0 = source.jacobian_at(&y).unwrap();
    let r: [f64; 3] = std::array::from_fn(|i| {
        let jv: f64 = (0..3).map(|j| j0[i][j] * d[j]).sum();
        d[i] - tau * jv - tau * f0[i]
    });
    let expected = [5.0 / 65536.0, -35.0 / 131072.0, 25.0 / 131072.0];
    for i in 0..3 {
        assert!((r[i] - expected[i]).abs() < 1e-17, "component {i}: {:?}", r);
    }
    assert!((r[0] + r[1] + r[2]).abs() < 1e-17);
    let max_abs = r.iter().map(|x| x.abs()).fold(0.0_f64, f64::max);
    assert!(max_abs > 2.0e-4, "a false raw-U promotion would be caught");
    assert!((max_abs - 35.0 / 131072.0).abs() < 1e-17);
    println!("N01_C7_BE_RAW_U0_COUNTEREXAMPLE residual={r:?} max={max_abs:.17e}");
}

/// A separate three-component dense-oracle problem for the original tableau.
/// This ONLY tests whether the existing native RODAS5P eight-stage target
/// is reconstructed correctly. It is not derivative-free production routing.
fn oracle_problem(owner: Arc<N01MassActionOwner>) -> OdeProblem {
    let f_source = Arc::clone(&owner);
    let rhs = Arc::new(
        move |_t: f64, state: &[f64], out: &mut [f64]| -> CoreResult<()> {
            let state: [f64; 3] = state
                .try_into()
                .map_err(|_| CoreError::Dimension("c7_rhs_shape".into()))?;
            if out.len() != 3 {
                return Err(CoreError::Dimension("c7_rhs_out".into()));
            }
            let mut val = [0.0; 3];
            f_source.rhs_at(&state, &mut val)?;
            out.copy_from_slice(&val);
            Ok(())
        },
    );
    let j_source = Arc::clone(&owner);
    let jac = Arc::new(move |_t: f64, state: &[f64]| -> CoreResult<DenseMatrix> {
        let state: [f64; 3] = state
            .try_into()
            .map_err(|_| CoreError::Dimension("c7_j_shape".into()))?;
        let j = j_source.jacobian_at(&state)?;
        DenseMatrix::from_rows(&[&j[0], &j[1], &j[2]])
    });
    OdeProblem::new(
        "n01-c7-robertson-reference-only",
        3,
        rhs,
        None,
        Some(jac),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn actual_native_eight_stage_rodas_reconstruction_is_mass_neutral_and_original_residual_small() {
    let y = [0.75, 0.125, 0.125];
    let owner = Arc::new(
        N01MassActionOwner::new([0.5, 1.0, 0.5], y, [1.0; 3], 0.0, [1.0; 3], [1.0; 3], 0.0)
            .unwrap(),
    );
    let problem = oracle_problem(Arc::clone(&owner));
    let h = 0.005;
    let mut work = WorkCounters::default();
    let result = sequential_step(
        &problem,
        0.0,
        &y,
        h,
        &LinearSolverConfig::default(),
        None,
        1e-10,
        1e-7,
        true,
        &mut work,
    )
    .unwrap();
    let c = rodas5p_coefficients().unwrap();
    assert_eq!(c.stages(), 8);
    assert_eq!(result.stages.len(), 8);
    let mut u = [[0.0_f64; 3]; 8];
    for (i, u_i) in u.iter_mut().enumerate() {
        for j in 0..=i {
            let gamma_ij = if i == j {
                c.gamma
            } else {
                c.gamma_matrix[(i, j)]
            };
            for (k, u_ik) in u_i.iter_mut().enumerate() {
                *u_ik += gamma_ij * result.stages[j][k];
            }
        }
    }
    let j0 = owner.jacobian_at(&y).unwrap();
    let mut max_original_residual = 0.0_f64;
    let mut max_stage_mass_defect = 0.0_f64;
    for (i, u_i) in u.iter().enumerate() {
        let mut state = y;
        let mut mix = [0.0; 3];
        for (j, u_j) in u[..i].iter().enumerate() {
            for (k, state_k) in state.iter_mut().enumerate() {
                *state_k += c.a[(i, j)] * u_j[k];
                mix[k] += c.c_matrix[(i, j)] * u_j[k];
            }
        }
        let mut f = [0.0; 3];
        owner.rhs_at(&state, &mut f).unwrap();
        for (k, &f_k) in f.iter().enumerate() {
            let jv: f64 = (0..3).map(|jj| j0[k][jj] * u_i[jj]).sum();
            let residual = u_i[k] - h * c.gamma * jv - c.gamma * mix[k] - h * c.gamma * f_k;
            max_original_residual = max_original_residual.max(residual.abs());
        }
        max_stage_mass_defect = max_stage_mass_defect.max(u_i.iter().sum::<f64>().abs());
    }
    let mut y_raw = y;
    for (i, u_i) in u.iter().enumerate() {
        for (k, y_k) in y_raw.iter_mut().enumerate() {
            *y_k += c.b_code[i] * u_i[k];
        }
    }
    let max_native_output_difference = (0..3)
        .map(|k| (y_raw[k] - result.y_new[k]).abs())
        .fold(0.0_f64, f64::max);
    let endpoint_mass_defect = (y_raw.iter().sum::<f64>() - y.iter().sum::<f64>()).abs();
    println!(
        "N01_C7_NATIVE_RODAS8 raw_residual={max_original_residual:.17e} stage_mass={max_stage_mass_defect:.17e} endpoint_mass={endpoint_mass_defect:.17e} raw_vs_K={max_native_output_difference:.17e} linear_solves={}",
        work.linear_solves
    );
    assert!(
        max_original_residual < 1.0e-9,
        "native raw-U residual not small; do not silently relax or claim authority"
    );
    assert!(
        max_stage_mass_defect < 1.0e-10,
        "stage mass invariance failed"
    );
    assert!(
        endpoint_mass_defect < 1.0e-10,
        "endpoint mass invariance failed"
    );
    assert!(
        max_native_output_difference < 1.0e-9,
        "raw/native K coefficient transport mismatch"
    );
    assert!(
        work.linear_solves >= 8,
        "native reference did not solve original stages"
    );
}

#[test]
fn wrong_auxiliary_stage_is_never_silent_original_target_admission() {
    // This is a negative control: an original-RK certificate cannot be inferred
    // from a smaller BE-only residual. The exact target residual is load bearing.
    let source = manufactured();
    let trial = source.freeze();
    let be = N01ValueStage::new(&source, &trial, 0.25).unwrap();
    let mut out = [9.0; 3];
    assert!(
        be.try_commit(0.25, 1e-12, 1e-12, &mut out)
            .unwrap()
            .accepted
    );
    let d = [
        out[0] - 35.0 / 64.0,
        out[1] - 27.0 / 128.0,
        out[2] - 31.0 / 128.0,
    ];
    let nonlinear = 0.25 * (d[1] * d[2] + 0.5 * d[1] * d[1]);
    assert!((nonlinear - 35.0 / 131072.0).abs() < 1e-17);
    assert!(nonlinear > 1e-12);
}
