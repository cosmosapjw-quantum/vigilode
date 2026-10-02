use rodas5p_integrators::{
    AffineMajorant, ParallelExecution, PathEvaluation, RadiusBox, causal_radius_box,
    evaluate_radius_box,
};
fn fixture() -> AffineMajorant {
    AffineMajorant::new(
        vec![vec![0.0; 2]; 2],
        vec![vec![0.0; 2]; 2],
        vec![vec![0.0, 0.0], vec![1.0, 0.0]],
        vec![1.0, 0.0],
    )
    .unwrap()
}
#[test]
fn negative_alpha_cannot_turn_a_required_positive_radius_into_zero() {
    let mut m = fixture();
    m.alpha_abs[1][0] = -1.0;
    let r = RadiusBox::common(1, 2, 0.0).unwrap();
    let out = evaluate_radius_box(
        &m,
        &r,
        PathEvaluation::ActionFirst,
        &ParallelExecution::sequential(),
    );
    assert!(
        out.is_err(),
        "invalid negative absolute coefficient produced {out:?}"
    );
}
#[test]
fn radius_policies_reject_nonfinite_or_negative_alpha() {
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        let mut m = fixture();
        m.alpha_abs[1][0] = bad;
        assert!(
            causal_radius_box(
                &m,
                0.01,
                PathEvaluation::ActionFirst,
                &ParallelExecution::sequential()
            )
            .is_err()
        );
    }
}
#[test]
fn valid_state_map_requires_one_unit_of_radius() {
    let m = fixture();
    let ex = ParallelExecution::sequential();
    assert!(
        !evaluate_radius_box(
            &m,
            &RadiusBox::common(1, 2, 0.0).unwrap(),
            PathEvaluation::ActionFirst,
            &ex
        )
        .unwrap()
        .closes
    );
    assert!(
        evaluate_radius_box(
            &m,
            &RadiusBox::common(1, 2, 1.0).unwrap(),
            PathEvaluation::ActionFirst,
            &ex
        )
        .unwrap()
        .closes
    );
}
