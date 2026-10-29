//! Opt-in native OdeProblem factory for one source-owned nonlinear mass-action law.
//!
//! Every RHS, Jacobian and JVP callback captures the *same* Arc<Mutex<owner>>.
//! The model epoch hook reads that owner too. Each callback first binds the
//! actual (time,state) supplied by the native integrator; moving the clock or
//! state invalidates every prior certified trial. This conservative rule can
//! reduce cache reuse in real integrators. It is intentionally not default.
//! There is no automatic stage gate or full-IVP certification in this factory.

use crate::OdeProblem;
use rodas5p_core::{CoreError, CoreResult, DenseMatrix, N01MassActionOwner};
use std::sync::{Arc, Mutex};

pub type N01MassActionHandle = Arc<Mutex<N01MassActionOwner>>;

fn shape(y: &[f64]) -> CoreResult<[f64; 3]> {
    y.try_into()
        .map_err(|_| CoreError::Dimension("N01_MASS_CALLBACK_DIMENSION".into()))
}

/// A native ODE adapter and the *exact same owner* that freezes trials.
/// Callbacks refuse a negative/out-of-box state; a poisoned mutex is fatal.
/// Prior generic OdeProblem callbacks are not certified by this constructor.
pub fn n01_mass_action_owned_problem(
    owner: N01MassActionOwner,
) -> CoreResult<(OdeProblem, N01MassActionHandle)> {
    let handle = Arc::new(Mutex::new(owner));
    let rhs_handle = Arc::clone(&handle);
    let rhs = Arc::new(
        move |t: f64, y: &[f64], out: &mut [f64]| -> CoreResult<()> {
            if out.len() != 3 {
                return Err(CoreError::Dimension("N01_MASS_RHS_OUTPUT".into()));
            }
            let state = shape(y)?;
            let mut guard = rhs_handle
                .lock()
                .map_err(|_| CoreError::InvalidInput("N01_MASS_SOURCE_MUTEX_POISONED".into()))?;
            guard.advance(t, state)?;
            let mut result = [0.0; 3];
            guard.rhs_at(&state, &mut result)?;
            out.copy_from_slice(&result);
            Ok(())
        },
    );
    let jac_handle = Arc::clone(&handle);
    let jac = Arc::new(move |t: f64, y: &[f64]| -> CoreResult<DenseMatrix> {
        let state = shape(y)?;
        let mut guard = jac_handle
            .lock()
            .map_err(|_| CoreError::InvalidInput("N01_MASS_SOURCE_MUTEX_POISONED".into()))?;
        guard.advance(t, state)?;
        let j = guard.jacobian_at(&state)?;
        DenseMatrix::from_rows(&[&j[0], &j[1], &j[2]])
    });
    let jvp_handle = Arc::clone(&handle);
    let jvp = Arc::new(
        move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| -> CoreResult<()> {
            if out.len() != 3 {
                return Err(CoreError::Dimension("N01_MASS_JVP_OUTPUT".into()));
            }
            let state = shape(y)?;
            let vector = shape(v)?;
            let mut guard = jvp_handle
                .lock()
                .map_err(|_| CoreError::InvalidInput("N01_MASS_SOURCE_MUTEX_POISONED".into()))?;
            guard.advance(t, state)?;
            let mut result = [0.; 3];
            guard.jvp_at(&state, &vector, &mut result)?;
            out.copy_from_slice(&result);
            Ok(())
        },
    );
    let epoch_handle = Arc::clone(&handle);
    let epoch = Arc::new(move || -> u64 {
        // A poisoned owner is not a valid cached Jacobian epoch. All further
        // callbacks error; a poisoned source cannot certify a candidate.
        epoch_handle.lock().map_or(u64::MAX, |g| g.model_epoch())
    });
    let problem = OdeProblem::new(
        "n01-source-owned-robertson",
        3,
        rhs,
        None,
        Some(jac),
        Some(jvp),
        None,
        true,
        None,
        None,
    )?
    .with_model_epoch(epoch);
    Ok((problem, handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodas5p_core::WorkCounters;
    #[test]
    fn callback_sources_match_and_stale_epoch_refused() {
        let model = N01MassActionOwner::new(
            [0.0625, 8., 4.],
            [0.875, 0.0625, 0.0625],
            [0.125; 3],
            0.0625,
            [1.; 3],
            [1., 2., 1.],
            0.,
        )
        .unwrap();
        let (problem, handle) = n01_mass_action_owned_problem(model).unwrap();
        let point = [0.859375, 0.078125, 0.0625];
        let mut f = [0.; 3];
        let mut counters = WorkCounters::default();
        problem
            .eval_rhs_into(0.125, &point, &mut f, &mut counters)
            .unwrap();
        let old = handle.lock().unwrap().freeze();
        let j = problem.linearize(0.125, &point, &mut counters).unwrap();
        let mf = problem.linearize_matrix_free(0.125, &point).unwrap();
        let mut v = [0.; 3];
        let mut v_mf = [0.; 3];
        j.apply(&[1., -1., 0.5], &mut v).unwrap();
        mf.apply(&[1., -1., 0.5], &mut v_mf).unwrap();
        for i in 0..3 {
            assert!((v[i] - v_mf[i]).abs() < 1e-14);
        }
        assert!(f.iter().sum::<f64>().abs() < 1e-14);
        let before = problem.model_epoch().unwrap();
        let mut f2 = [0.; 3];
        problem
            .eval_rhs_into(0.5, &point, &mut f2, &mut counters)
            .unwrap();
        assert!(problem.model_epoch().unwrap() > before);
        let mut output = [88.; 3];
        let guard = handle.lock().unwrap();
        assert!(
            guard
                .try_commit(&old, &[0.; 3], 10., 10., &mut output)
                .is_err()
        );
        assert_eq!(output, [88.; 3]);
    }
    #[test]
    fn callbacks_do_not_commit_partially_on_bad_state() {
        let model = N01MassActionOwner::new(
            [0.0625, 8., 4.],
            [0.875, 0.0625, 0.0625],
            [0.125; 3],
            0.0625,
            [1.; 3],
            [1.; 3],
            0.,
        )
        .unwrap();
        let (problem, _) = n01_mass_action_owned_problem(model).unwrap();
        let mut output = [99.0; 3];
        let mut counters = WorkCounters::default();
        assert!(
            problem
                .eval_rhs_into(0.1, &[-1.0, 0., 2.], &mut output, &mut counters)
                .is_err()
        );
        assert_eq!(output, [99.; 3]);
        assert!(
            problem
                .linearize_matrix_free(0.1, &[0.5, 0.1, 0.4])
                .unwrap()
                .apply(&[1., 1., 1.], &mut output)
                .is_err()
        );
        assert_eq!(output, [99.; 3]);
    }
}
