use rodas5p_core::{CoreError, DenseMatrix, rodas5p_coefficients};
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use std::sync::Arc;
use vigilode_coupled::{Config, coupled_step};
fn model(n: usize, eps: f64) -> OdeProblem {
    let rhs = Arc::new(move |_: f64, y: &[f64], f: &mut [f64]| {
        for i in 0..n {
            let a = [-1.0, -20.0, -100.0][i % 3];
            f[i] = a * y[i] + eps * y[i] * y[i];
        }
        Ok(())
    });
    let jvp = Arc::new(move |_: f64, y: &[f64], v: &[f64], f: &mut [f64]| {
        for i in 0..n {
            f[i] = ([-1.0, -20.0, -100.0][i % 3] + 2.0 * eps * y[i]) * v[i];
        }
        Ok(())
    });
    let forbidden = Arc::new(|_: f64, _: &[f64]| -> Result<DenseMatrix, CoreError> {
        panic!("explicit Jacobian must never be called")
    });
    OdeProblem::new(
        "three-mode",
        n,
        rhs,
        None,
        Some(forbidden),
        Some(jvp),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}
fn raw_reference(n: usize, eps: f64, h: f64, y: &[f64]) -> Vec<f64> {
    let c = rodas5p_coefficients().unwrap();
    let mut u = vec![vec![0.0; n]; 8];
    for i in 0..8 {
        for k in 0..n {
            let mut state = y[k];
            let mut mix = 0.0;
            for j in 0..i {
                state += c.a[(i, j)] * u[j][k];
                mix += c.c_matrix[(i, j)] * u[j][k];
            }
            let a = [-1.0, -20.0, -100.0][k % 3];
            u[i][k] = (h * c.gamma * (a * state + eps * state * state) + c.gamma * mix)
                / (1.0 - h * c.gamma * (a + 2.0 * eps * y[k]));
        }
    }
    let mut out = y.to_vec();
    for i in 0..8 {
        for k in 0..n {
            out[k] += c.b_code[i] * u[i][k];
        }
    }
    out
}
#[test]
fn builds_three_mode_candidate_from_real_jvp_only() {
    let p = model(96, 1.0);
    let y = vec![0.1; 96];
    let r = coupled_step(
        &p,
        0.0,
        &y,
        0.01,
        &Config::default(),
        &ParallelExecution::sequential(),
    );
    let c = r.candidate.as_ref().expect("coupled candidate missing");
    assert_eq!(c.basis_dimension, 3);
    assert!(c.full_residual_wrms < 1e-4);
    let reference = raw_reference(96, 1.0, 0.01, &y);
    for (a, b) in c.y_new.iter().zip(reference) {
        assert!((a - b).abs() < 1e-11, "{a} {b}");
    }
}
#[test]
fn thread_count_changes_no_output_bits() {
    let p = model(384, 10.0);
    let y = vec![0.1; 384];
    let cfg = Config::default();
    let a = coupled_step(&p, 0.0, &y, 0.01, &cfg, &ParallelExecution::sequential())
        .candidate
        .unwrap();
    let b = coupled_step(
        &p,
        0.0,
        &y,
        0.01,
        &cfg,
        &ParallelExecution::rayon(4).unwrap(),
    )
    .candidate
    .unwrap();
    assert!(
        a.y_new
            .iter()
            .zip(b.y_new)
            .all(|(x, y)| x.to_bits() == y.to_bits())
    );
}
#[test]
fn equilibrium_is_complete_not_zero_basis_failure() {
    let p = model(12, 1.0);
    let y = vec![0.0; 12];
    let r = coupled_step(
        &p,
        0.0,
        &y,
        0.01,
        &Config::default(),
        &ParallelExecution::sequential(),
    );
    assert_eq!(r.candidate.unwrap().y_new, y);
}
#[test]
fn partial_basis_cannot_pass_only_projected_residual() {
    let n = 64;
    let p = OdeProblem::new(
        "broad",
        n,
        Arc::new(move |_, y, f| {
            for i in 0..n {
                f[i] = -10f64.powf(4.0 * i as f64 / 63.0) * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, _, v, f| {
            for i in 0..n {
                f[i] = -10f64.powf(4.0 * i as f64 / 63.0) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let cfg = Config {
        initial_basis: 1,
        max_basis: 1,
        ..Config::default()
    };
    let r = coupled_step(
        &p,
        0.0,
        &vec![1.0; n],
        0.02,
        &cfg,
        &ParallelExecution::sequential(),
    );
    assert!(r.candidate.is_none());
    assert!(r.failure.is_some());
}
#[test]
fn invalid_time_is_rejected() {
    let p = model(3, 1.0);
    assert!(
        coupled_step(
            &p,
            0.0,
            &[0.1; 3],
            f64::NAN,
            &Config::default(),
            &ParallelExecution::sequential()
        )
        .candidate
        .is_none()
    );
}
