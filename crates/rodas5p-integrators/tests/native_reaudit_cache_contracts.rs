//! Current-linearization ownership and atomic refresh regressions.
use rodas5p_core::{CoreError, DenseMatrix, LinearMethod, LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{OdeProblem, Rodas5pMfFastWorkspace};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: 2,
        maxiter: 8,
        rtol: 1e-12,
        atol: 1e-14,
        ..Default::default()
    }
}
fn problem(rate: f64, fail: Arc<AtomicBool>) -> OdeProblem {
    OdeProblem::new(
        "same-name",
        1,
        Arc::new(move |t, y, out| {
            if fail.swap(false, Ordering::SeqCst) {
                out[0] = 1234.0; // A fallible callback may partly write its output.
                return Err(CoreError::NonFinite("one-shot RHS failure".into()));
            }
            out[0] = -rate * (1.0 + t) * y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |t, y, v, out| {
            out[0] = -2.0 * rate * (1.0 + t) * y[0] * v[0];
            Ok(())
        })),
        Some(Arc::new(move |_t, y, out| {
            out[0] = -rate * y[0] * y[0];
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap()
}
fn run(w: &mut Rodas5pMfFastWorkspace, p: &OdeProblem, t: f64, y: f64, fresh: bool) -> f64 {
    w.attempt(
        p,
        t,
        &[y],
        0.001,
        fresh,
        None,
        1e-8,
        1e-6,
        &mut WorkCounters::default(),
    )
    .unwrap();
    w.y_new()[0]
}
fn reference(p: &OdeProblem, t: f64, y: f64) -> f64 {
    run(
        &mut Rodas5pMfFastWorkspace::new(p, &config()).unwrap(),
        p,
        t,
        y,
        true,
    )
}
#[test]
fn a_failed_fresh_callback_cannot_leave_an_old_linearization_reusable() {
    let fail = Arc::new(AtomicBool::new(false));
    let p = problem(1.0, fail.clone());
    let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
    run(&mut w, &p, 0.0, 1.0, true);
    fail.store(true, Ordering::SeqCst);
    assert!(
        w.attempt(
            &p,
            0.1,
            &[0.9],
            0.001,
            true,
            None,
            1e-8,
            1e-6,
            &mut WorkCounters::default()
        )
        .is_err()
    );
    let got = run(&mut w, &p, 0.1, 0.9, false);
    let expected = reference(&p, 0.1, 0.9);
    assert_eq!(
        got.to_bits(),
        expected.to_bits(),
        "stale J / partial RHS survived failed refresh"
    );
}
#[test]
fn a_reuse_hint_does_not_override_state_or_time_identity() {
    let p = problem(1.0, Arc::new(AtomicBool::new(false)));
    for (t, y) in [(0.0, 0.8), (0.25, 1.0)] {
        let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
        run(&mut w, &p, 0.0, 1.0, true);
        assert_eq!(
            run(&mut w, &p, t, y, false).to_bits(),
            reference(&p, t, y).to_bits()
        );
    }
}
#[test]
fn a_same_named_different_callback_is_not_the_same_frozen_problem() {
    let p = problem(1.0, Arc::new(AtomicBool::new(false)));
    let other = problem(3.0, Arc::new(AtomicBool::new(false)));
    let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
    run(&mut w, &p, 0.0, 1.0, true);
    assert_eq!(
        run(&mut w, &other, 0.0, 1.0, false).to_bits(),
        reference(&other, 0.0, 1.0).to_bits()
    );
}
#[test]
fn the_public_attempt_cannot_bypass_the_identity_mass_contract() {
    let p = problem(1.0, Arc::new(AtomicBool::new(false)));
    let mut other = p.clone();
    other.mass_matrix = Some(DenseMatrix::identity(1).scale(2.0));
    let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
    assert!(
        w.attempt(
            &other,
            0.0,
            &[1.0],
            0.001,
            true,
            None,
            1e-8,
            1e-6,
            &mut WorkCounters::default()
        )
        .is_err()
    );
}
#[test]
fn nonfinite_or_negative_output_tolerances_are_rejected_before_work() {
    let p = problem(1.0, Arc::new(AtomicBool::new(false)));
    for (atol, rtol) in [
        (f64::INFINITY, 1e-6),
        (1e-8, f64::INFINITY),
        (-1.0, 1e-6),
        (1e-8, -1.0),
    ] {
        let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
        let mut c = WorkCounters::default();
        assert!(
            w.attempt(&p, 0.0, &[1.0], 0.001, true, None, atol, rtol, &mut c)
                .is_err()
        );
        assert_eq!(c.rhs_evaluations, 0);
    }
}
#[test]
fn a_cloned_problem_same_state_changed_step_still_reuses_the_cache() {
    let p = problem(1.0, Arc::new(AtomicBool::new(false)));
    let same = p.clone();
    let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
    run(&mut w, &p, 0.0, 1.0, true);
    let mut c = WorkCounters::default();
    w.attempt(&same, 0.0, &[1.0], 0.0005, false, None, 1e-8, 1e-6, &mut c)
        .unwrap();
    assert_eq!(c.rhs_evaluations, 7);
}

#[test]
fn a_failed_time_derivative_refresh_is_also_atomic() {
    let fail = Arc::new(AtomicBool::new(false));
    let flag = fail.clone();
    let p = OdeProblem::new(
        "ft-failure",
        1,
        Arc::new(|t, y, out| {
            out[0] = -(1.0 + t) * y[0] * y[0];
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(|t, y, v, out| {
            out[0] = -2.0 * (1.0 + t) * y[0] * v[0];
            Ok(())
        })),
        Some(Arc::new(move |_t, y, out| {
            if flag.swap(false, Ordering::SeqCst) {
                return Err(CoreError::NonFinite("one-shot f_t failure".into()));
            }
            out[0] = -y[0] * y[0];
            Ok(())
        })),
        false,
        None,
        None,
    )
    .unwrap();
    let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
    run(&mut w, &p, 0.0, 1.0, true);
    fail.store(true, Ordering::SeqCst);
    assert!(
        w.attempt(
            &p,
            0.1,
            &[0.9],
            0.001,
            true,
            None,
            1e-8,
            1e-6,
            &mut WorkCounters::default()
        )
        .is_err()
    );
    assert_eq!(
        run(&mut w, &p, 0.1, 0.9, false).to_bits(),
        reference(&p, 0.1, 0.9).to_bits()
    );
}

#[test]
fn a_nonfinite_time_and_overflowing_error_scale_are_not_admissible() {
    let p = problem(1.0, Arc::new(AtomicBool::new(false)));
    let mut w = Rodas5pMfFastWorkspace::new(&p, &config()).unwrap();
    assert!(
        w.attempt(
            &p,
            f64::INFINITY,
            &[1.0],
            0.001,
            true,
            None,
            1e-8,
            1e-6,
            &mut WorkCounters::default()
        )
        .is_err()
    );
    assert!(
        w.attempt(
            &p,
            0.0,
            &[2.0],
            0.001,
            true,
            None,
            1e-8,
            f64::MAX,
            &mut WorkCounters::default()
        )
        .is_err()
    );
}
