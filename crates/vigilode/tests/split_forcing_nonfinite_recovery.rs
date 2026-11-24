//! Parity of the split-forcing wrapper with an ordinary composite right-hand side on trial
//! states outside the domain of the model (stage-lattice referee run, audit probe F-SF1).
//!
//! `y' = -40 sqrt(y)`, `y(0) = 1` has the solution `(1 - 20 t)^2` for `t <= 0.05`. A trial
//! stage with a negative argument makes `sqrt` return NaN. The facade contract
//! (`CallbackError`, `fd.rs`) is that a callback which returns `Ok` with a nonfinite
//! component has violated its contract and ends the run, and that a model reports a state
//! outside its domain with `CallbackError::Recoverable`, which rejects the attempt.
//!
//! The probe asked whether `SplitForcing` is stricter than the ordinary path. It is not:
//! both paths accept, reject and stop identically in every cell below.

use vigilode::split_forcing::SplitForcing;
use vigilode::{CallbackError, FailureReason, Options, Problem, Rodas5p, Solution, Status};

const SPAN: (f64, f64) = (0.0, 0.04);

fn state(strict: bool) -> impl Fn(f64, &[f64], &mut [f64]) -> Result<(), CallbackError> + Clone {
    move |_t, y, out| {
        if strict && y[0] < 0.0 {
            return Err(CallbackError::Recoverable(
                "state outside the domain".into(),
            ));
        }
        out[0] = -40.0 * y[0].sqrt();
        Ok(())
    }
}

fn solve_both(h0: f64, strict: bool) -> (Solution, Solution) {
    let options = Options {
        h0,
        ..Options::default()
    };
    let rhs = state(strict);
    let ordinary = Problem::new(1, move |t, y, dy| rhs(t, y, dy));
    let split = SplitForcing::new(1, 1, state(strict), |_t, out| {
        out[0] = 0.0;
        Ok(())
    })
    .unwrap();
    (
        Rodas5p::solve(&ordinary, SPAN, &[1.0], &options).unwrap(),
        Rodas5p::solve(&split.direct_problem(), SPAN, &[1.0], &options).unwrap(),
    )
}

fn assert_same_run(a: &Solution, b: &Solution, label: &str) {
    assert_eq!(a.stats.accepted_steps, b.stats.accepted_steps, "{label}");
    assert_eq!(a.stats.rejected_steps, b.stats.rejected_steps, "{label}");
    let (ya, yb) = (a.final_state().unwrap(), b.final_state().unwrap());
    assert_eq!(ya[0].to_bits(), yb[0].to_bits(), "{label}");
}

#[test]
fn in_domain_runs_are_bit_identical_and_reach_the_closed_form() {
    for h0 in [1.0e-3, 2.0e-2] {
        let (a, b) = solve_both(h0, false);
        assert_eq!(a.status, Status::Success, "h0 {h0:e}");
        assert_eq!(b.status, Status::Success, "h0 {h0:e}");
        assert_same_run(&a, &b, &format!("h0 {h0:e}"));
        let exact = (1.0f64 - 20.0 * SPAN.1).powi(2);
        assert!((a.final_state().unwrap()[0] - exact).abs() < 2.0e-6);
    }
}

#[test]
fn an_ok_callback_with_a_nan_component_ends_both_paths_at_the_same_point() {
    let (a, b) = solve_both(4.0e-2, false);
    for (name, s) in [("ordinary", &a), ("split", &b)] {
        assert!(
            matches!(
                s.status,
                Status::Failed(FailureReason::CallbackFailed { .. })
            ),
            "{name}: {:?}",
            s.status
        );
        assert_eq!(s.final_state().unwrap(), &[1.0], "{name}");
    }
    assert_same_run(&a, &b, "h0 4e-2, Ok with NaN");
}

#[test]
fn a_recoverable_domain_error_rejects_the_attempt_in_both_paths() {
    let (a, b) = solve_both(4.0e-2, true);
    assert_eq!(a.status, Status::Success);
    assert_eq!(b.status, Status::Success);
    assert!(a.stats.rejected_steps >= 1);
    assert_same_run(&a, &b, "h0 4e-2, recoverable");
    let exact = (1.0f64 - 20.0 * SPAN.1).powi(2);
    assert!((a.final_state().unwrap()[0] - exact).abs() < 2.0e-6);
}
