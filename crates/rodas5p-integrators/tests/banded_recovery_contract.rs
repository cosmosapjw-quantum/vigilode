//! Reproduce nonfinite-result and incomplete-base retry defects on the parent.
use rodas5p_core::{CoreError, DenseMatrix};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedKernel, OdeProblem, OutputSchedule,
    RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID, RODAS5P_FAST_BANDED_FIXED2_EXACT_CACHE_DRIVER_ID,
    RODAS5P_FAST_BANDED_SLICES_DRIVER_ID, integrate_rodas5p_fast_banded_observed_with_kernel,
    integrate_rodas5p_fast_observed, rodas5p_fast_banded_solve,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[test]
fn nonfinite_upper_entry_is_not_a_successful_solution() {
    // W has finite diagonal pivots but an infinite superdiagonal.
    let r = rodas5p_fast_banded_solve(2, 0, 1, &[0., f64::INFINITY, 0., 0.], 1., &[0., 0.]);
    assert!(r.is_err(), "nonfinite matrix produced Ok: {r:?}");
}
#[test]
fn nonfinite_rhs_is_rejected() {
    let r = rodas5p_fast_banded_solve(1, 0, 0, &[0.], 1., &[f64::NAN]);
    assert!(r.is_err(), "nonfinite RHS produced Ok: {r:?}");
}
#[test]
fn finite_inputs_with_overflowing_solution_are_rejected() {
    let r = rodas5p_fast_banded_solve(1, 0, 0, &[0.], 1e-320, &[1.]);
    assert!(r.is_err(), "unrepresentable solution produced Ok: {r:?}");
}
#[test]
fn tiny_but_representable_solution_is_not_over_rejected() {
    let (x, _) = rodas5p_fast_banded_solve(1, 0, 0, &[0.], 1e-300, &[1e-300]).unwrap();
    assert_eq!(x, [1.]);
}
#[test]
fn size_arithmetic_overflow_is_a_typed_error_not_a_panic() {
    let result = std::panic::catch_unwind(|| {
        rodas5p_fast_banded_solve(usize::MAX, usize::MAX - 1, usize::MAX - 1, &[], 1., &[])
    });
    assert!(result.is_ok(), "size contract panicked");
    assert!(result.unwrap().is_err());
}

fn retry_probe(banded: bool, fail_jacobian: bool) {
    let fail = Arc::new(AtomicBool::new(true));
    let calls = Arc::new(AtomicUsize::new(0));
    let rf = fail.clone();
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        if !fail_jacobian && t == 0. && rf.swap(false, Ordering::SeqCst) {
            return Err(CoreError::NonFinite(
                "injected first initial RHS failure".into(),
            ));
        }
        out[0] = -y[0];
        Ok(())
    });
    let jc = calls.clone();
    let jf = fail.clone();
    let jac = Arc::new(move |t: f64, _y: &[f64]| {
        if t == 0. {
            jc.fetch_add(1, Ordering::SeqCst);
        }
        if fail_jacobian && t == 0. && jf.swap(false, Ordering::SeqCst) {
            return Err(CoreError::NonFinite(
                "injected first initial Jacobian failure".into(),
            ));
        }
        DenseMatrix::new(1, 1, vec![-1.])
    });
    let p = OdeProblem::new(
        "retry-probe",
        1,
        rhs,
        None,
        Some(jac),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let bc = calls.clone();
    let bf = fail.clone();
    let band = BandedJacobian {
        lower: 0,
        upper: 0,
        fill: Arc::new(move |t, _y, j| {
            if t == 0. {
                bc.fetch_add(1, Ordering::SeqCst);
            }
            if fail_jacobian && t == 0. && bf.swap(false, Ordering::SeqCst) {
                // Writing an entry does not make the callback's failed result valid.
                j[0] = 123.;
                return Err(CoreError::NonFinite("partial band Jacobian".into()));
            }
            j[0] = -1.;
            Ok(())
        }),
    };
    let cfg = AdaptiveStepConfig {
        initial_step: 0.01,
        min_step: 1e-14,
        max_step: 0.01,
        max_attempts: 2,
        rtol: 1e-6,
        atol: 1e-10,
        ..AdaptiveStepConfig::default()
    };
    let times = OutputSchedule::new(vec![0., 0.01]).unwrap();
    let r = if banded {
        integrate_rodas5p_fast_banded_observed_with_kernel(
            &p,
            &band,
            (0., 0.01),
            &[1.],
            &cfg,
            &times,
            BandedKernel::Slices,
        )
        .unwrap()
        .fast
    } else {
        integrate_rodas5p_fast_observed(&p, (0., 0.01), &[1.], &cfg, &times).unwrap()
    };
    assert_eq!(r.attempts, 2);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "incomplete base must be rebuilt at retry; {r:?}"
    );
    assert_eq!(
        r.jacobian_reuses, 0,
        "failed callback snapshot was counted as reusable"
    );
}
#[test]
fn failed_banded_rhs_rebuilds_snapshot() {
    retry_probe(true, false);
}
#[test]
fn failed_banded_jacobian_rebuilds_snapshot() {
    retry_probe(true, true);
}
#[test]
fn failed_dense_rhs_rebuilds_snapshot() {
    retry_probe(false, false);
}
#[test]
fn failed_dense_jacobian_rebuilds_snapshot() {
    retry_probe(false, true);
}

/// Final review item 7: every banded kernel's record key is a public
/// constant, and the values are unchanged.
#[test]
fn banded_kernel_driver_ids_are_public_constants() {
    assert_eq!(
        RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID,
        "rodas5p-fast-banded-fixed2-v1"
    );
    assert_eq!(
        RODAS5P_FAST_BANDED_FIXED2_EXACT_CACHE_DRIVER_ID,
        "rodas5p-fast-banded-fixed2-exact-cache-v1"
    );
    let n = 6;
    let problem = OdeProblem::new(
        "pentadiagonal-decay",
        n,
        Arc::new(|_, y, out| {
            for (o, v) in out.iter_mut().zip(y) {
                *o = -v;
            }
            Ok(())
        }),
        None,
        Some(Arc::new(move |_, _| {
            let mut j = DenseMatrix::zeros(n, n);
            for i in 0..n {
                j[(i, i)] = -1.0;
            }
            Ok(j)
        })),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let band = BandedJacobian {
        lower: 2,
        upper: 2,
        fill: Arc::new(move |_, _, j| {
            j.fill(0.0);
            for i in 0..n {
                j[i * 5 + 2] = -1.0;
            }
            Ok(())
        }),
    };
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-8,
        rtol: 1.0e-6,
        initial_step: 1.0e-3,
        max_step: 1.0,
        ..AdaptiveStepConfig::default()
    };
    let schedule = OutputSchedule::new(vec![0.0, 1.0]).unwrap();
    for (kernel, id) in [
        (BandedKernel::Slices, RODAS5P_FAST_BANDED_SLICES_DRIVER_ID),
        (
            BandedKernel::FixedWidth2,
            RODAS5P_FAST_BANDED_FIXED2_DRIVER_ID,
        ),
        (
            BandedKernel::FixedWidth2ExactCache,
            RODAS5P_FAST_BANDED_FIXED2_EXACT_CACHE_DRIVER_ID,
        ),
    ] {
        let run = integrate_rodas5p_fast_banded_observed_with_kernel(
            &problem,
            &band,
            (0.0, 1.0),
            &vec![1.0; n],
            &adaptive,
            &schedule,
            kernel,
        )
        .unwrap()
        .fast;
        assert!(run.observed.success, "{kernel:?}");
        assert_eq!(run.driver, id, "{kernel:?}");
    }
}
