//! A1 defect (production plan 2026-10-20, section 6): a zero tolerance scale.
//!
//! With `atol = 0` a component that is exactly zero at both ends of a step
//! has the error scale `atol + rtol max(|y|, |y_new|) = 0`. Every driver
//! follows one rule (`rodas5p_core::scaled_error_component`): a component
//! whose scale and error are both zero contributes 0 to the norm; a zero
//! scale with a nonzero error makes the norm +inf (the step is rejected).
//!
//! The problem is `y0' = -y0`, `y1' = 0` with `y(0) = (1, 0)`: `y1` and its
//! embedded error are identically zero, so every driver must integrate it
//! with `atol = 0`, `rtol = 1e-6` like any other problem, and the zero
//! component must stay exactly zero.

use std::sync::Arc;

use rodas5p_core::{DenseMatrix, LinearMethod, LinearSolverConfig, PreconditionerKind};
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedKernel, BatchProblem, IntegrationMethod, OdeProblem,
    OutputSchedule, SmallBatchMember, SmallProblem, integrate_adaptive_observed,
    integrate_rodas5p_fast_banded_observed_with_kernel, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_small_batch, integrate_rodas5p_fast_small_observed,
    integrate_rodas5p_mf_fast_observed,
};

const T_END: f64 = 1.0;
const RTOL: f64 = 1.0e-6;

fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 0.0,
        rtol: RTOL,
        initial_step: 1.0e-3,
        min_step: 1.0e-12,
        max_step: T_END,
        max_attempts: 10_000,
        ..AdaptiveStepConfig::default()
    }
}

fn schedule() -> OutputSchedule {
    OutputSchedule::new(vec![0.0, T_END]).unwrap()
}

fn problem() -> OdeProblem {
    OdeProblem::new(
        "a1-zero-component",
        2,
        Arc::new(|_, y, out| {
            out[0] = -y[0];
            out[1] = 0.0;
            Ok(())
        }),
        None,
        Some(Arc::new(|_, _| {
            DenseMatrix::new(2, 2, vec![-1.0, 0.0, 0.0, 0.0])
        })),
        Some(Arc::new(|_, _, v, out| {
            out[0] = -v[0];
            out[1] = 0.0;
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// The final state is `(e^-1, 0)` to the requested relative tolerance and
/// the zero component is exactly zero.
fn check_final(y: &[f64], what: &str) {
    assert_eq!(y[1].to_bits() & !(1 << 63), 0, "{what}: y1 = {}", y[1]);
    let exact = (-T_END).exp();
    assert!(
        ((y[0] - exact) / exact).abs() < 1.0e-4,
        "{what}: y0 = {} vs {exact}",
        y[0]
    );
}

#[derive(Clone, Copy)]
struct Small;

impl SmallProblem<2> for Small {
    fn rhs(&self, y: &[f64; 2], out: &mut [f64; 2]) {
        out[0] = -y[0];
        out[1] = 0.0;
    }
    fn jacobian(&self, _y: &[f64; 2], out: &mut [[f64; 2]; 2]) {
        *out = [[-1.0, 0.0], [0.0, 0.0]];
    }
}

impl<const B: usize> BatchProblem<2, B> for Small {
    type Lanes = ();
    fn lanes(_member: &Self) {}
    fn set_lane(_lanes: &mut (), _lane: usize, _member: &Self) {}
    fn rhs_batch(_lanes: &(), y: &[[f64; B]; 2], out: &mut [[f64; B]; 2]) {
        for l in 0..B {
            out[0][l] = -y[0][l];
            out[1][l] = 0.0;
        }
    }
}

#[test]
fn fast_dense_driver_accepts_a_zero_component_with_zero_atol() {
    let r = integrate_rodas5p_fast_observed(
        &problem(),
        (0.0, T_END),
        &[1.0, 0.0],
        &adaptive(),
        &schedule(),
    )
    .expect("fast dense driver must not abort");
    assert!(r.observed.success, "fast dense: {}", r.observed.message);
    assert_eq!(r.rejected_steps, 0, "a zero/zero component must not reject");
    check_final(r.observed.y.last().unwrap(), "fast dense");
}

#[test]
fn fast_banded_driver_accepts_a_zero_component_with_zero_atol() {
    let band = BandedJacobian {
        lower: 0,
        upper: 0,
        fill: Arc::new(|_, _, j| {
            j[0] = -1.0;
            j[1] = 0.0;
            Ok(())
        }),
    };
    let r = integrate_rodas5p_fast_banded_observed_with_kernel(
        &problem(),
        &band,
        (0.0, T_END),
        &[1.0, 0.0],
        &adaptive(),
        &schedule(),
        BandedKernel::Slices,
    )
    .expect("fast banded driver must not abort")
    .fast;
    assert!(r.observed.success, "fast banded: {}", r.observed.message);
    assert_eq!(r.rejected_steps, 0);
    check_final(r.observed.y.last().unwrap(), "fast banded");
}

#[test]
fn fast_small_driver_accepts_a_zero_component_with_zero_atol() {
    let r = integrate_rodas5p_fast_small_observed(
        &Small,
        (0.0, T_END),
        &[1.0, 0.0],
        &adaptive(),
        &schedule(),
    )
    .expect("fast small driver must not abort");
    assert!(r.observed.success, "fast small: {}", r.observed.message);
    assert_eq!(r.rejected_steps, 0);
    check_final(r.observed.y.last().unwrap(), "fast small");
}

#[test]
fn fast_small_batch_driver_accepts_a_zero_component_with_zero_atol() {
    let members = vec![
        SmallBatchMember {
            problem: Small,
            t_span: (0.0, T_END),
            y0: [1.0, 0.0],
        };
        3
    ];
    let runs =
        integrate_rodas5p_fast_small_batch::<2, 4, Small>(&members, &adaptive(), &schedule());
    assert_eq!(runs.len(), 3);
    for (k, r) in runs.into_iter().enumerate() {
        let r = r.expect("fast small batch driver must not abort");
        assert!(
            r.observed.success,
            "batch member {k}: {}",
            r.observed.message
        );
        assert_eq!(r.rejected_steps, 0);
        check_final(r.observed.y.last().unwrap(), "fast small batch");
    }
}

#[test]
fn matrix_free_fast_driver_accepts_a_zero_component_with_zero_atol() {
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    };
    let r = integrate_rodas5p_mf_fast_observed(
        &problem(),
        (0.0, T_END),
        &[1.0, 0.0],
        &linear,
        &adaptive(),
        &schedule(),
    )
    .expect("matrix-free fast driver must not abort on a zero scale");
    assert!(r.observed.success, "mf fast: {}", r.observed.message);
    assert_eq!(r.rejected_steps, 0);
    check_final(r.observed.y.last().unwrap(), "mf fast");
}

/// The protected (sequential) driver goes through
/// `rodas5p_core::error_scale` and `wrms`.
#[test]
fn protected_sequential_driver_accepts_a_zero_component_with_zero_atol() {
    let r = integrate_adaptive_observed(
        &problem(),
        (0.0, T_END),
        &[1.0, 0.0],
        1.0e-3,
        IntegrationMethod::Sequential,
        None,
        None,
        0.0,
        RTOL,
        10_000,
        T_END,
        &schedule(),
    )
    .expect("protected driver must not abort on a zero scale");
    assert!(r.success, "protected: {}", r.message);
    check_final(r.y.last().unwrap(), "protected");
}
