//! A1 zero-scale rule (production plan 2026-10-20, section 6) in the coupled
//! crate: with `atol = 0` and `rtol > 0`, a component that is zero at both
//! ends of the step has a zero error scale. As in every RODAS5P driver
//! (`rodas5p_core::scaled_error_component`), a zero scale with a zero error or
//! residual contributes 0, and with a nonzero one makes the norm +inf. Neither
//! is a typed error. `atol = 0` is accepted when `rtol > 0`.
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use std::sync::Arc;
use vigilode_coupled::{Config, FailureKind, coupled_step, coupled_step_residual_first};

fn model(eps: f64) -> OdeProblem {
    OdeProblem::new(
        "zero-scale-three-mode",
        96,
        Arc::new(move |_, y, f| {
            for i in 0..96 {
                f[i] = [-1.0, -20.0, -100.0][i % 3] * y[i] + eps * y[i] * y[i];
            }
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, f| {
            for i in 0..96 {
                f[i] = ([-1.0, -20.0, -100.0][i % 3] + 2.0 * eps * y[i]) * v[i];
            }
            Ok(())
        })),
        None,
        true,
        None,
        None,
    )
    .unwrap()
}

/// Every odd component is identically zero.
fn y0() -> Vec<f64> {
    (0..96)
        .map(|i| if i % 2 == 0 { 0.1 } else { 0.0 })
        .collect()
}

fn zero_atol() -> Config {
    Config {
        atol: 0.0,
        rtol: 1.0e-6,
        ..Config::default()
    }
}

#[test]
fn zero_atol_with_identically_zero_components_gives_a_candidate() {
    for eps in [0.0, 1.0] {
        let p = model(eps);
        for subject in [coupled_step, coupled_step_residual_first] {
            let r = subject(
                &p,
                0.0,
                &y0(),
                1.0e-3,
                &zero_atol(),
                &ParallelExecution::sequential(),
            );
            let c = r
                .candidate
                .as_ref()
                .unwrap_or_else(|| panic!("eps {eps}: {:?}", r.failure));
            assert!(c.embedded_wrms.is_finite() && c.full_residual_wrms.is_finite());
            for (i, v) in c.y_new.iter().enumerate() {
                if i % 2 == 1 {
                    assert_eq!(*v, 0.0, "eps {eps}: component {i} = {v}");
                }
            }
        }
    }
}

#[test]
fn zero_atol_and_zero_rtol_is_still_an_input_error() {
    let r = coupled_step(
        &model(1.0),
        0.0,
        &y0(),
        1.0e-3,
        &Config {
            atol: 0.0,
            rtol: 0.0,
            ..Config::default()
        },
        &ParallelExecution::sequential(),
    );
    assert_eq!(r.failure.as_ref().unwrap().kind, FailureKind::Input);
    // Final review (coupled validation message): each message says what
    // its check allows.
    assert_eq!(
        r.failure.as_ref().unwrap().message,
        "rtol and the solver tolerances must be finite and positive"
    );
    let negative = coupled_step(
        &model(1.0),
        0.0,
        &y0(),
        1.0e-3,
        &Config {
            atol: -1.0e-12,
            ..Config::default()
        },
        &ParallelExecution::sequential(),
    );
    assert_eq!(negative.failure.as_ref().unwrap().kind, FailureKind::Input);
    assert_eq!(
        negative.failure.as_ref().unwrap().message,
        "atol must be finite and nonnegative"
    );
}
