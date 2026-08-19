use rodas5p_core::LinearOperator;
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{build_step_context, manufactured_vector_problem};

#[test]
fn jvp_only_clone_hides_explicit_jacobian_and_builds_matrix_free_shifted_operator() {
    let (problem, y0) = manufactured_vector_problem(4, 100.0, 10.0, 0.2, 0.0).unwrap();
    assert!(problem.has_explicit_jacobian());
    assert!(problem.has_jvp());
    let matrix_free = problem.jvp_only_clone().unwrap();
    assert!(!matrix_free.has_explicit_jacobian());
    assert!(matrix_free.has_jvp());
    let mut counters = WorkCounters::default();
    let context = build_step_context(&matrix_free, 0.0, &y0, 0.01, &mut counters).unwrap();
    assert_eq!(counters.jacobian_builds, 0);
    assert!(context.shifted.explicit().is_none());
    let mut out = vec![0.0; y0.len()];
    context.shifted.apply(&y0, &mut out).unwrap();
    assert!(out.iter().all(|value| value.is_finite()));
}

#[test]
fn supplied_jvp_matches_the_explicit_jacobian_action() {
    let (problem, y0) = manufactured_vector_problem(8, 10_000.0, 1_000.0, 0.2, 0.0).unwrap();
    let mut explicit_counters = WorkCounters::default();
    let explicit = problem.linearize(0.0, &y0, &mut explicit_counters).unwrap();
    let matrix_free = problem.linearize_matrix_free(0.0, &y0).unwrap();
    let direction = (0..y0.len())
        .map(|index| ((index + 1) as f64).sin())
        .collect::<Vec<_>>();
    let mut expected = vec![0.0; y0.len()];
    let mut actual = vec![0.0; y0.len()];
    explicit.apply(&direction, &mut expected).unwrap();
    matrix_free.apply(&direction, &mut actual).unwrap();
    let maximum_difference = expected
        .iter()
        .zip(&actual)
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f64, f64::max);
    assert!(
        maximum_difference
            <= 64.0 * f64::EPSILON * expected.iter().map(|v| v.abs()).fold(1.0, f64::max)
    );
}
