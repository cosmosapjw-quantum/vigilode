use vigilode::affine_events::{
    AFFINE_EVENT_CLOCK_ID, AffineEventBinding, AffineEventSpec, AffineEventStatus,
    AffineScalarModel, Interval,
};
use vigilode::{Atol, Options, Problem, Rodas5p};

fn spec(lambda: f64, b: f64, level: f64) -> AffineEventSpec {
    AffineEventSpec::new(
        AffineEventBinding::new("affine", "analytic", AFFINE_EVENT_CLOCK_ID, "s", "level", 0)
            .unwrap(),
        AffineScalarModel::new(
            Interval::point(lambda).unwrap(),
            Interval::point(b).unwrap(),
        )
        .unwrap(),
        level,
    )
    .unwrap()
}

fn solve(lambda: f64, b: f64, initial: f64, duration: f64, dense: bool) -> vigilode::Solution {
    let problem = Problem::new(1, move |_, x, dx| {
        dx[0] = b - lambda * x[0];
        Ok(())
    });
    let opts = Options {
        dense_output: dense,
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        ..Default::default()
    };
    Rodas5p::solve(&problem, (0.0, duration), &[initial], &opts).unwrap()
}

#[test]
fn certified_windows_enclose_three_exact_roots() {
    for (lambda, b, initial, level) in [
        (1.0_f64, 0.0_f64, 1.0_f64, 0.51_f64),
        (12.0, 24.0, 0.2, 1.1),
        (800.0, -240.0, 0.8, 0.11),
    ] {
        let eq = b / lambda;
        let exact = -((level - eq) / (initial - eq)).ln() / lambda;
        let sol = solve(lambda, b, initial, 2.0 * exact, true);
        let windows = sol
            .certified_affine_event_windows(&spec(lambda, b, level), 0.0)
            .unwrap();
        assert_eq!(windows.len(), 1, "{windows:?}");
        assert_eq!(windows[0].status, AffineEventStatus::UniqueCrossing);
        let [a, b] = windows[0].times;
        println!(
            "lambda={lambda} root={exact:.17e} bracket=[{a:.17e},{b:.17e}] accepted={}",
            sol.t.len() - 1
        );
        assert!(a <= exact && exact <= b);
        assert!(
            b - a < 2e-5 * exact,
            "certified bracket [{a},{b}], root {exact}"
        );
    }
}

#[test]
fn uncertain_and_absent_crossings_and_missing_dense_output() {
    let sol = solve(1.0, 0.0, 1.0, 0.75, true);
    assert!(
        sol.certified_affine_event_windows(&spec(1.0, 0.0, 3.0), 0.0)
            .unwrap()
            .is_empty()
    );
    let unsure = sol
        .certified_affine_event_windows(&spec(1.0, 0.0, 0.75), 2.0)
        .unwrap();
    assert!(
        unsure
            .iter()
            .any(|w| w.status == AffineEventStatus::Unresolved)
    );
    assert!(
        sol.certified_affine_event_windows(&spec(1.0, 0.0, 0.75), f64::NAN)
            .is_err()
    );
    assert!(
        solve(1.0, 0.0, 1.0, 0.75, false)
            .certified_affine_event_windows(&spec(1.0, 0.0, 0.75), 0.0)
            .is_err()
    );
}
