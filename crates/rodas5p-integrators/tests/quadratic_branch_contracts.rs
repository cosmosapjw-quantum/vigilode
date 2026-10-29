use rodas5p_integrators::quadratic_branch::{
    QuadraticBranchError as E, quadratic_homotopy_root as root,
};

#[test]
fn regular_branch_tracks_original_equation() {
    for a in [0.01, 1.0, 100.0] {
        for c in [-0.1, 0.0, 1.0, 1000.0] {
            for lambda in [0.0, 0.01, 0.25, 1.0] {
                let expected = 0.02;
                if a + 2.0 * lambda * c * expected <= 0.0 {
                    continue;
                }
                let b = a * expected + lambda * c * expected * expected;
                let r = root(a, b, c, lambda).unwrap();
                assert!((r.value - expected).abs() < 2e-15);
                assert!(r.relative_residual <= 128.0 * f64::EPSILON);
            }
        }
    }
}

#[test]
fn common_scaling_preserves_branch_and_handles_large_coefficients() {
    let reference = root(3.0, 2.0, 1.0, 1.0).unwrap().value;
    for s in [1e-300, 1e-100, 1.0, 1e100, 1e300] {
        let r = root(3.0 * s, 2.0 * s, s, 1.0).unwrap();
        assert!((r.value - reference).abs() < 8e-16);
    }
    assert_eq!(root(1.0, 0.0, 1.0, 1.0).unwrap().value, 0.0);
}

#[test]
fn stable_root_avoids_subtractive_cancellation() {
    let r = root(1.0, 1e-16, 1.0, 1.0).unwrap();
    assert!((r.value / 1e-16 - 1.0).abs() < 3e-16);
}

#[test]
fn wall_degree_drop_and_input_refusals_are_explicit() {
    assert_eq!(root(1.0, 1.0, -0.25, 1.0).unwrap_err(), E::DiscriminantWall);
    assert_eq!(root(1.0, 1.0, -1.0, 1.0).unwrap_err(), E::DiscriminantWall);
    assert_eq!(root(0.0, 1.0, 1.0, 1.0).unwrap_err(), E::Input);
    assert_eq!(root(1.0, 1.0, 1.0, 1.1).unwrap_err(), E::Input);
    assert_eq!(
        root(1e-300, 1e300, 1.0, 1.0).unwrap_err(),
        E::ScaleUnderflow
    );
    assert_eq!(root(2.0, 3.0, 10.0, 0.0).unwrap().value, 1.5);
    assert_eq!(root(1e-300, 0.0, 1e300, 1.0).unwrap().value, 0.0);
    assert_eq!(root(1e-200, 1.0, 0.0, 1.0).unwrap().value, 1e200);
    assert_eq!(root(1e-200, -1.0, 1e300, 0.0).unwrap().value, -1e200);
}

#[test]
fn implicit_reaction_step_is_positive_contracting_and_stiffly_decaying() {
    // Backward Euler for y'=-a*y-c*y^2 is an exact use of this declared chart.
    for h in [1e-6, 0.01, 1.0, 1e4] {
        let y1 = root(1.0 + 10.0 * h, 1.0, 100.0 * h, 1.0).unwrap().value;
        let y2 = root(1.0 + 10.0 * h, 1.01, 100.0 * h, 1.0).unwrap().value;
        assert!(y1 > 0.0 && y1 <= 1.0);
        assert!(y2 > y1 && y2 - y1 <= 0.01 / (1.0 + 10.0 * h) * (1.0 + 1e-12));
    }
    assert!(root(1e12, 1.0, 1.0, 1.0).unwrap().value <= 1e-12);
}
