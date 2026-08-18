use rodas5p_integrators::pexprb54s4_remainder_vector_geometry;

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= 32.0 * f64::EPSILON.max(expected.abs() * f64::EPSILON));
}

#[test]
fn vector_remainder_geometry_recovers_collinear_orthogonal_and_antiparallel_limits() {
    let collinear =
        pexprb54s4_remainder_vector_geometry(&[1.0, 0.0], &[2.0, 0.0], &[3.0, 0.0], 2).unwrap();
    assert_eq!(collinear.norm_component_count, 2);
    assert_close(collinear.chi23.unwrap(), 1.0);
    assert_close(collinear.chi34.unwrap(), 1.0);
    assert_close(collinear.chi24.unwrap(), 1.0);
    assert_close(collinear.q34_perp.unwrap(), 0.0);
    assert_close(collinear.delta_chi.unwrap(), 0.0);

    let orthogonal =
        pexprb54s4_remainder_vector_geometry(&[1.0, 0.0], &[0.0, 2.0], &[-3.0, 0.0], 2).unwrap();
    assert_close(orthogonal.chi23.unwrap(), 0.0);
    assert_close(orthogonal.chi34.unwrap(), 0.0);
    assert_close(orthogonal.chi24.unwrap(), -1.0);
    assert_close(orthogonal.q34_perp.unwrap(), 1.0);
    assert_close(orthogonal.delta_chi.unwrap(), 0.0);

    let antiparallel =
        pexprb54s4_remainder_vector_geometry(&[1.0, 0.0], &[-2.0, 0.0], &[3.0, 0.0], 2).unwrap();
    assert_close(antiparallel.chi23.unwrap(), -1.0);
    assert_close(antiparallel.chi34.unwrap(), -1.0);
    assert_close(antiparallel.chi24.unwrap(), 1.0);
    assert_close(antiparallel.q34_perp.unwrap(), 0.0);
    assert_close(antiparallel.delta_chi.unwrap(), 0.0);
}

#[test]
fn vector_remainder_geometry_excludes_trailing_clock_components() {
    let geometry = pexprb54s4_remainder_vector_geometry(
        &[1.0, 0.0, 1000.0],
        &[2.0, 0.0, -1000.0],
        &[3.0, 0.0, 500.0],
        2,
    )
    .unwrap();
    assert_eq!(geometry.state_dimension, 3);
    assert_eq!(geometry.norm_component_count, 2);
    assert_eq!(geometry.excluded_trailing_components, 1);
    assert_close(geometry.chi23.unwrap(), 1.0);
    assert_close(geometry.chi34.unwrap(), 1.0);
    assert_close(geometry.chi24.unwrap(), 1.0);
}

#[test]
fn vector_remainder_geometry_is_fail_closed_for_zero_norm_and_nonfinite_inputs() {
    let zero =
        pexprb54s4_remainder_vector_geometry(&[0.0, 0.0], &[1.0, 0.0], &[2.0, 0.0], 2).unwrap();
    assert_eq!(zero.chi23, None);
    assert_eq!(zero.chi24, None);
    assert_close(zero.chi34.unwrap(), 1.0);
    assert_close(zero.q34_perp.unwrap(), 0.0);
    assert_eq!(zero.delta_chi, None);

    let nonfinite =
        pexprb54s4_remainder_vector_geometry(&[1.0, 0.0], &[1.0, 0.0], &[f64::INFINITY, 0.0], 2)
            .unwrap();
    assert_close(nonfinite.chi23.unwrap(), 1.0);
    assert_eq!(nonfinite.chi34, None);
    assert_eq!(nonfinite.chi24, None);
    assert_eq!(nonfinite.q34_perp, None);
    assert_eq!(nonfinite.delta_chi, None);
}

#[test]
fn vector_remainder_geometry_rejects_invalid_component_contracts() {
    assert!(pexprb54s4_remainder_vector_geometry(&[1.0], &[1.0], &[1.0], 0).is_err());
    assert!(pexprb54s4_remainder_vector_geometry(&[1.0], &[1.0], &[1.0], 2).is_err());
    assert!(pexprb54s4_remainder_vector_geometry(&[1.0], &[1.0, 2.0], &[1.0], 1).is_err());
}
