use rodas5p_core::{ClosureOperator, IdentityPreconditioner, WorkCounters};
use rodas5p_integrators::{
    ScientificCorpusV2, ScientificFamily, ScientificProblemCase, v2_diversity_multiplier,
};
use rodas5p_krylov::{GmresConfig, solve_gmres_givens};
use std::collections::BTreeSet;

#[test]
fn v2_radical_inverse_multiplier_is_prefix_stable_nonperiodic_and_bounded() {
    let prefix = (0..32).map(v2_diversity_multiplier).collect::<Vec<_>>();
    let longer = (0..256).map(v2_diversity_multiplier).collect::<Vec<_>>();
    assert_eq!(prefix, longer[..prefix.len()]);
    assert!(longer.iter().all(|value| (0.9..=1.1).contains(value)));
    assert_eq!(
        longer
            .iter()
            .map(|value| value.to_bits())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        256
    );
}

fn calibration_case(family: ScientificFamily, dimension: usize) -> ScientificProblemCase {
    ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|case| case.family == family && case.dimension == dimension && case.rtol == 1.0e-6)
        .unwrap()
        .build()
        .unwrap()
}

fn repeated_local_state(dimension: usize, width: usize) -> Vec<f64> {
    (0..dimension)
        .map(|i| 0.15 + 0.025 * (i % width) as f64)
        .collect()
}

#[test]
fn all_five_diversified_families_are_prefix_stable_in_the_actual_operators() {
    for family in [
        ScientificFamily::RobertsonRamped,
        ScientificFamily::HiresRamped,
        ScientificFamily::VanDerPolRamped,
        ScientificFamily::RotatingNonnormal,
        ScientificFamily::NonautonomousStiffForcing,
    ] {
        let width = family.block_width().unwrap();
        let small = calibration_case(family, 96);
        let large = calibration_case(family, 384);
        let small_state = repeated_local_state(96, width);
        let mut large_state = repeated_local_state(384, width);
        large_state[..96].copy_from_slice(&small_state);
        let small_rhs = small
            .problem
            .eval_rhs(0.37, &small_state, &mut WorkCounters::default())
            .unwrap();
        let large_rhs = large
            .problem
            .eval_rhs(0.37, &large_state, &mut WorkCounters::default())
            .unwrap();
        assert_eq!(
            small_rhs
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            large_rhs[..96]
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            "{} RHS prefix drifted",
            family.as_str(),
        );

        let direction = repeated_local_state(96, width)
            .into_iter()
            .map(|value| value - 0.1)
            .collect::<Vec<_>>();
        let mut large_direction = vec![0.0; 384];
        large_direction[..96].copy_from_slice(&direction);
        let small_operator = small
            .problem
            .linearize_matrix_free(0.37, &small_state)
            .unwrap();
        let large_operator = large
            .problem
            .linearize_matrix_free(0.37, &large_state)
            .unwrap();
        let mut small_image = vec![0.0; 96];
        let mut large_image = vec![0.0; 384];
        small_operator.apply(&direction, &mut small_image).unwrap();
        large_operator
            .apply(&large_direction, &mut large_image)
            .unwrap();
        assert_eq!(
            small_image
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            large_image[..96]
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            "{} JVP prefix drifted",
            family.as_str(),
        );
    }
}

fn localized_block_signatures(case: &ScientificProblemCase, width: usize) -> BTreeSet<u64> {
    let state = repeated_local_state(case.spec.dimension, width);
    let operator = case.problem.linearize_matrix_free(0.37, &state).unwrap();
    let mut signatures = BTreeSet::new();
    for block in 0..case.spec.dimension / width {
        let offset = block * width;
        let mut direction = vec![0.0; case.spec.dimension];
        for local in 0..width {
            direction[offset + local] = 0.2 + 0.03 * local as f64;
        }
        let mut image = vec![0.0; case.spec.dimension];
        operator.apply(&direction, &mut image).unwrap();
        let norm = image[offset..offset + width]
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        signatures.insert(norm.to_bits());
    }
    signatures
}

#[test]
fn actual_block_or_component_operator_classes_grow_with_dimension() {
    for family in [
        ScientificFamily::RobertsonRamped,
        ScientificFamily::HiresRamped,
        ScientificFamily::VanDerPolRamped,
        ScientificFamily::RotatingNonnormal,
        ScientificFamily::NonautonomousStiffForcing,
    ] {
        let width = family.block_width().unwrap();
        let counts = [96, 384, 1536].map(|dimension| {
            let case = calibration_case(family, dimension);
            localized_block_signatures(&case, width).len()
        });
        assert!(
            counts[0] < counts[1] && counts[1] < counts[2],
            "{} retained a dimension-independent operator class: {counts:?}",
            family.as_str(),
        );
        assert_eq!(counts[0], 96 / width);
    }

    let semilinear_counts = [96, 384, 1536].map(|dimension| {
        let case = calibration_case(
            ScientificFamily::SemilinearAdvectionDiffusionRamped,
            dimension,
        );
        let operator = case.problem.linearize_matrix_free(0.37, &case.y0).unwrap();
        let direction = vec![1.0; dimension];
        let mut image = vec![0.0; dimension];
        operator.apply(&direction, &mut image).unwrap();
        image
            .into_iter()
            .map(f64::to_bits)
            .collect::<BTreeSet<_>>()
            .len()
    });
    assert!(semilinear_counts[0] < semilinear_counts[1]);
    assert!(semilinear_counts[1] < semilinear_counts[2]);
}

#[test]
fn v2_calibration_partial_t_callbacks_match_the_implemented_rhs() {
    for family in ScientificFamily::CALIBRATION {
        let case = calibration_case(family, 96);
        let t = 0.37;
        let state = case
            .y0
            .iter()
            .enumerate()
            .map(|(i, value)| value + 2.0e-4 * (1.0 + (i % 13) as f64))
            .collect::<Vec<_>>();
        let mut work = WorkCounters::default();
        let actual = case.problem.eval_partial_t(t, &state, &mut work).unwrap();
        assert_eq!(work.ft_calls, 1);
        assert_eq!(work.rhs_calls, 0);

        let epsilon = 1.0e-6;
        let plus = case
            .problem
            .eval_rhs(t + epsilon, &state, &mut WorkCounters::default())
            .unwrap();
        let minus = case
            .problem
            .eval_rhs(t - epsilon, &state, &mut WorkCounters::default())
            .unwrap();
        for (index, (analytic, pair)) in actual.iter().zip(plus.iter().zip(&minus)).enumerate() {
            let oracle = (pair.0 - pair.1) / (2.0 * epsilon);
            let tolerance = 5.0e-4 + 3.0e-6 * analytic.abs().max(oracle.abs());
            assert!(
                (analytic - oracle).abs() <= tolerance,
                "{} component {index}: analytic={analytic:.17e}, oracle={oracle:.17e}, tolerance={tolerance:.3e}",
                family.as_str(),
            );
        }
    }
}

fn gmres_iterations_for_nonautonomous(dimension: usize) -> u64 {
    let case = calibration_case(ScientificFamily::NonautonomousStiffForcing, dimension);
    let t = 0.72;
    let (ramp, _) = {
        let z = (t - 0.45_f64) / 0.07_f64;
        let th = z.tanh();
        (0.5 * (1.0 + th), 0.5 * (1.0 - th * th) / 0.07)
    };
    let frequency = 2.0 + 28.0 * ramp;
    let state = (0..dimension)
        .map(|i| (frequency * t + (i % 11) as f64 * 0.17).sin())
        .collect::<Vec<_>>();
    let jacobian = case.problem.linearize_matrix_free(t, &state).unwrap();
    let tau = 0.025;
    let shifted = ClosureOperator::new(dimension, move |input, output| {
        jacobian.apply(input, output)?;
        for (value, source) in output.iter_mut().zip(input) {
            *value = source - tau * *value;
        }
        Ok(())
    });
    // Exercise a nested prefix of independently scaled modes.  Every fourfold dimension increase
    // adds four modes.  A repeated scalar-identity construction would still converge in one
    // iteration for every row; the v2 diagonal has one distinct value per active component.
    let active_modes = match dimension {
        96 => 3,
        384 => 7,
        1536 => 11,
        _ => unreachable!("v2 calibration dimension"),
    };
    let mut rhs = vec![0.0; dimension];
    for (i, value) in rhs[..active_modes].iter_mut().enumerate() {
        *value = 0.75 + 0.25 * (0.31 * (i + 1) as f64).sin();
    }
    let report = solve_gmres_givens(
        &shifted,
        &IdentityPreconditioner::new(dimension),
        &rhs,
        None,
        &GmresConfig {
            restart: 64,
            max_arnoldi: 128,
            rtol: 2.0e-13,
            atol: 0.0,
        },
        &mut WorkCounters::default(),
    )
    .unwrap();
    assert!(report.converged);
    report.iterations
}

#[test]
fn deterministic_matrix_free_gmres_probe_is_not_dimension_constant() {
    let iterations = [96, 384, 1536].map(gmres_iterations_for_nonautonomous);
    eprintln!("v2 nonautonomous GMRES iterations: {iterations:?}");
    assert!(
        iterations.into_iter().collect::<BTreeSet<_>>().len() > 1,
        "v2 operator diversification must make the deterministic GMRES probe dimension-sensitive"
    );
}
