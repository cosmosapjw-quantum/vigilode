use std::collections::BTreeSet;

use rodas5p_core::WorkCounters;
use rodas5p_integrators::{
    CorpusPartition, ScientificCorpusV2, ScientificFamily, ScientificProblemCase,
};

#[test]
fn v2_corpus_partitions_families_dimensions_tolerances_and_sources() {
    let calibration = ScientificCorpusV2::calibration_specs();
    let holdout = ScientificCorpusV2::holdout_specs();

    assert_eq!(calibration.len(), 6 * 3 * 3);
    assert_eq!(holdout.len(), 4 * 3);
    assert!(
        calibration
            .iter()
            .all(|case| case.partition == CorpusPartition::Calibration)
    );
    assert!(
        holdout
            .iter()
            .all(|case| case.partition == CorpusPartition::Holdout)
    );

    let calibration_names = calibration
        .iter()
        .map(|case| case.family.as_str())
        .collect::<BTreeSet<_>>();
    let holdout_names = holdout
        .iter()
        .map(|case| case.family.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(calibration_names.len(), 6);
    assert_eq!(holdout_names.len(), 4);
    assert!(calibration_names.is_disjoint(&holdout_names));

    for family in ScientificFamily::CALIBRATION {
        let rows = calibration
            .iter()
            .filter(|case| case.family == family)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 9, "{} cross-product", family.as_str());
        for dimension in [96, 384, 1536] {
            for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
                assert!(
                    rows.iter()
                        .any(|case| case.dimension == dimension && case.rtol == rtol)
                );
            }
        }
    }

    assert_eq!(
        calibration
            .iter()
            .map(|case| case.dimension)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([96, 384, 1536]),
    );
    assert_eq!(
        calibration
            .iter()
            .map(|case| case.rtol.to_bits())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            1.0e-4_f64.to_bits(),
            1.0e-6_f64.to_bits(),
            1.0e-8_f64.to_bits()
        ]),
    );
    assert!(calibration.iter().all(|case| case.atol == 0.01 * case.rtol));

    let threshold_ids = ScientificCorpusV2::calibration_threshold_specs()
        .into_iter()
        .map(|case| case.id)
        .collect::<BTreeSet<_>>();
    let calibration_ids = calibration
        .iter()
        .map(|case| case.id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(threshold_ids, calibration_ids);
    assert!(holdout.iter().all(|case| !threshold_ids.contains(&case.id)));

    for (family, dimension, t_span, breakpoints) in [
        (ScientificFamily::Oregonator, 3, (0.0, 360.0), &[][..]),
        (ScientificFamily::Pollution, 20, (0.0, 60.0), &[][..]),
        (ScientificFamily::MedicalAkzo, 400, (0.0, 20.0), &[5.0][..]),
        (
            ScientificFamily::Brusselator2d,
            512,
            (0.0, 11.5),
            &[1.1][..],
        ),
    ] {
        let rows = holdout
            .iter()
            .filter(|case| case.family == family)
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|case| {
            case.dimension == dimension
                && case.t_span == t_span
                && case.mandatory_breakpoints == breakpoints
        }));
    }

    let brusselator = holdout
        .iter()
        .find(|case| case.family.as_str() == "brusselator-2d")
        .unwrap();
    assert_eq!(brusselator.dimension, 512);
    assert_eq!(brusselator.mandatory_breakpoints, vec![1.1]);
    assert_eq!(
        brusselator.provenance.source_revision,
        "63a13a7301a17feb8cb5e3a4b3ccef4487ae0c52"
    );
    assert_eq!(
        brusselator.provenance.source_blob,
        Some("fea9aaa141f224a97f112e024082966a1a5ee6c2".to_string())
    );
    assert_eq!(
        brusselator.provenance.source_path,
        "docs/src/examples/pde/brusselator.md"
    );

    let expected_sources = [
        (
            "oregonator",
            "aa58d9090f1f581f2e60e29b02b409466197981f5399120ce66bfb2d34f41c27",
        ),
        (
            "pollution",
            "2aba777ee6de34e0ee074951375e029ad5171e937dabb7ab4c6461c0736e6c20",
        ),
        (
            "medical-akzo",
            "3b5a4aa80769cd752e17a64a2ae15b4b07ba2a15f037aed48b7c2158d739861a",
        ),
        (
            "brusselator-2d",
            "688e4642b669e4181cca67d0d7cd9d663e2322d70923daf0240e5a995627351e",
        ),
    ];
    for (family, sha256) in expected_sources {
        let case = holdout
            .iter()
            .find(|case| case.family.as_str() == family)
            .unwrap();
        assert_eq!(case.provenance.source_sha256.as_deref(), Some(sha256));
    }
}

#[test]
fn output_grid_is_101_uniform_points_plus_declared_breakpoints() {
    let holdout = ScientificCorpusV2::holdout_specs();
    for case in &holdout {
        assert_eq!(case.uniform_output_points, 101);
        assert_eq!(case.output_times.first().copied(), Some(case.t_span.0));
        assert_eq!(case.output_times.last().copied(), Some(case.t_span.1));
        assert!(case.output_times.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            case.mandatory_breakpoints
                .iter()
                .all(|breakpoint| case.output_times.contains(breakpoint))
        );
    }
    let medical = holdout
        .iter()
        .find(|case| case.family == ScientificFamily::MedicalAkzo)
        .unwrap();
    assert_eq!(medical.output_times.len(), 101);
    assert_eq!(medical.mandatory_breakpoints, vec![5.0]);
    let brusselator = holdout
        .iter()
        .find(|case| case.family == ScientificFamily::Brusselator2d)
        .unwrap();
    assert_eq!(brusselator.output_times.len(), 102);
    assert_eq!(brusselator.mandatory_breakpoints, vec![1.1]);
}

#[test]
fn builder_fails_closed_if_partition_or_source_metadata_is_relabelled() {
    let mut partition_drift = ScientificCorpusV2::holdout_specs()[0].clone();
    partition_drift.partition = CorpusPartition::Calibration;
    assert!(partition_drift.build().is_err());

    let mut provenance_drift = ScientificCorpusV2::holdout_specs()[0].clone();
    provenance_drift.provenance.source_sha256 = Some("00".repeat(32));
    assert!(provenance_drift.build().is_err());

    let mut grid_drift = ScientificCorpusV2::holdout_specs()[0].clone();
    grid_drift.output_times.pop();
    assert!(grid_drift.build().is_err());
}

fn build_holdout(family: ScientificFamily) -> ScientificProblemCase {
    ScientificCorpusV2::holdout_specs()
        .into_iter()
        .find(|case| case.family == family && case.rtol == 1.0e-6)
        .unwrap()
        .build()
        .unwrap()
}

#[test]
fn source_equation_spot_checks_include_both_sides_of_discontinuities() {
    let oregonator = build_holdout(ScientificFamily::Oregonator);
    assert_eq!(oregonator.y0, vec![1.0, 2.0, 3.0]);
    let mut work = WorkCounters::default();
    let rhs = oregonator
        .problem
        .eval_rhs(0.0, &oregonator.y0, &mut work)
        .unwrap();
    let s = 77.27;
    let q = 8.375e-6;
    assert_eq!(rhs[0], s * (2.0 + (1.0 - q - 2.0)));
    assert_eq!(rhs[1], (3.0 - 4.0) / s);
    assert_eq!(rhs[2], 0.161 * (1.0 - 3.0));
    assert_eq!(work.rhs_calls, 1);
    assert_eq!(work.rhs_evaluations, 1);

    let pollution = build_holdout(ScientificFamily::Pollution);
    let rhs = pollution
        .problem
        .eval_rhs(0.0, &pollution.y0, &mut WorkCounters::default())
        .unwrap();
    assert!((rhs[0] - 0.2128).abs() <= 1.0e-15);
    assert!((rhs[1] + 0.2128).abs() <= 1.0e-15);
    assert!((rhs[2] - 0.0007).abs() <= 1.0e-15);
    assert!((rhs[3] + 0.213514).abs() <= 1.0e-15);
    assert!((rhs[4] - 0.0001733).abs() <= 1.0e-15);

    let medical = build_holdout(ScientificFamily::MedicalAkzo);
    assert_eq!(medical.y0.len(), 400);
    assert!(medical.y0.chunks_exact(2).all(|pair| pair == [0.0, 1.0]));
    let before = medical
        .problem
        .eval_rhs(5.0 - 1.0e-12, &medical.y0, &mut WorkCounters::default())
        .unwrap();
    let after = medical
        .problem
        .eval_rhs(5.0 + 1.0e-12, &medical.y0, &mut WorkCounters::default())
        .unwrap();
    let h = 0.005_f64;
    let zeta = h;
    let a = 2.0 * (zeta - 1.0).powi(3) / 16.0;
    let b = (zeta - 1.0).powi(4) / 16.0;
    let expected_first = a * (-2.0) / (2.0 * h) + b * 2.0 / (h * h);
    assert!((before[0] - expected_first).abs() <= 1.0e-10 * expected_first.abs());
    assert_eq!(after[0], 0.0);

    let brusselator = build_holdout(ScientificFamily::Brusselator2d);
    assert_eq!(brusselator.y0.len(), 512);
    let before = brusselator
        .problem
        .eval_rhs(1.1 - 1.0e-12, &brusselator.y0, &mut WorkCounters::default())
        .unwrap();
    let after = brusselator
        .problem
        .eval_rhs(1.1, &brusselator.y0, &mut WorkCounters::default())
        .unwrap();
    let forced_index = 4 + 16 * 9; // x=4/15, y=9/15 is inside the source disk.
    let x = 4.0_f64 / 15.0;
    let y = 9.0_f64 / 15.0;
    assert_eq!(
        brusselator.y0[forced_index],
        22.0 * (y * (1.0 - y)).powf(1.5)
    );
    assert_eq!(
        brusselator.y0[256 + forced_index],
        27.0 * (x * (1.0 - x)).powf(1.5)
    );
    assert_eq!(after[forced_index] - before[forced_index], 5.0);
    let outside_index = 0;
    assert_eq!(after[outside_index], before[outside_index]);
}

#[test]
fn discontinuous_holdouts_expose_branch_fixed_segments_for_endpoint_stages() {
    let medical = build_holdout(ScientificFamily::MedicalAkzo);
    assert_eq!(medical.integration_segments.len(), 2);
    assert_eq!(medical.integration_segments[0].t_span, (0.0, 5.0));
    assert_eq!(medical.integration_segments[1].t_span, (5.0, 20.0));
    let medical_left = medical.integration_segments[0]
        .problem
        .eval_rhs(5.0, &medical.y0, &mut WorkCounters::default())
        .unwrap();
    let medical_right = medical.integration_segments[1]
        .problem
        .eval_rhs(5.0, &medical.y0, &mut WorkCounters::default())
        .unwrap();
    assert_ne!(medical_left[0].to_bits(), medical_right[0].to_bits());
    // Branch fixation, rather than timestamp equality, governs every stage.
    assert_eq!(
        medical.integration_segments[0]
            .problem
            .eval_rhs(5.0 + 1.0e-6, &medical.y0, &mut WorkCounters::default())
            .unwrap()[0]
            .to_bits(),
        medical_left[0].to_bits()
    );
    assert_eq!(
        medical.integration_segments[1]
            .problem
            .eval_rhs(5.0 - 1.0e-6, &medical.y0, &mut WorkCounters::default())
            .unwrap()[0]
            .to_bits(),
        medical_right[0].to_bits()
    );

    let brusselator = build_holdout(ScientificFamily::Brusselator2d);
    assert_eq!(brusselator.integration_segments.len(), 2);
    assert_eq!(brusselator.integration_segments[0].t_span, (0.0, 1.1));
    assert_eq!(brusselator.integration_segments[1].t_span, (1.1, 11.5));
    let forced_index = 4 + 16 * 9;
    let brusselator_left = brusselator.integration_segments[0]
        .problem
        .eval_rhs(1.1, &brusselator.y0, &mut WorkCounters::default())
        .unwrap();
    let brusselator_right = brusselator.integration_segments[1]
        .problem
        .eval_rhs(1.1, &brusselator.y0, &mut WorkCounters::default())
        .unwrap();
    assert_eq!(
        brusselator_right[forced_index] - brusselator_left[forced_index],
        5.0
    );
    assert_eq!(
        brusselator.integration_segments[0]
            .problem
            .eval_rhs(1.1 + 1.0e-6, &brusselator.y0, &mut WorkCounters::default())
            .unwrap()[forced_index]
            .to_bits(),
        brusselator_left[forced_index].to_bits()
    );
    assert_eq!(
        brusselator.integration_segments[1]
            .problem
            .eval_rhs(1.1 - 1.0e-6, &brusselator.y0, &mut WorkCounters::default())
            .unwrap()[forced_index]
            .to_bits(),
        brusselator_right[forced_index].to_bits()
    );
}

#[test]
fn all_holdout_analytic_jvps_match_centered_finite_differences() {
    for family in ScientificFamily::HOLDOUT {
        let case = build_holdout(family);
        let t = match family {
            ScientificFamily::MedicalAkzo => 4.25,
            ScientificFamily::Brusselator2d => 1.35,
            _ => 0.37 * case.spec.t_span.1,
        };
        let y = case
            .y0
            .iter()
            .enumerate()
            .map(|(i, value)| value + 1.0e-6 * (i + 1) as f64)
            .collect::<Vec<_>>();
        let direction = (0..case.spec.dimension)
            .map(|i| (0.37 * (i + 1) as f64).sin())
            .collect::<Vec<_>>();
        let operator = case.problem.linearize_matrix_free(t, &y).unwrap();
        let mut actual = vec![0.0; case.spec.dimension];
        operator.apply(&direction, &mut actual).unwrap();

        let epsilon = 1.0e-7;
        let plus_state = y
            .iter()
            .zip(&direction)
            .map(|(value, delta)| value + epsilon * delta)
            .collect::<Vec<_>>();
        let minus_state = y
            .iter()
            .zip(&direction)
            .map(|(value, delta)| value - epsilon * delta)
            .collect::<Vec<_>>();
        let plus = case
            .problem
            .eval_rhs(t, &plus_state, &mut WorkCounters::default())
            .unwrap();
        let minus = case
            .problem
            .eval_rhs(t, &minus_state, &mut WorkCounters::default())
            .unwrap();
        for (index, (analytic, pair)) in actual.iter().zip(plus.iter().zip(&minus)).enumerate() {
            let oracle = (pair.0 - pair.1) / (2.0 * epsilon);
            let tolerance = 2.0e-4 + 4.0e-6 * analytic.abs().max(oracle.abs());
            assert!(
                (analytic - oracle).abs() <= tolerance,
                "{} component {index}: analytic={analytic:.17e}, oracle={oracle:.17e}, tolerance={tolerance:.3e}",
                family.as_str(),
            );
        }
    }
}
