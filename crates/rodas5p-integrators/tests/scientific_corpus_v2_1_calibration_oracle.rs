use rodas5p_core::WorkCounters;
use rodas5p_integrators::{ScientificCorpusV2, ScientificFamily};
use serde::Deserialize;

#[derive(Deserialize)]
struct Oracle {
    schema_version: String,
    origin: Origin,
    time: String,
    sampling: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Origin {
    engine: String,
    version: String,
    decimal_digits: usize,
}

#[derive(Deserialize)]
struct Case {
    family: String,
    dimension: usize,
    samples: Vec<Sample>,
}

#[derive(Deserialize)]
struct Sample {
    index: usize,
    y0: String,
    rhs: String,
    jvp: String,
}

fn family(name: &str) -> ScientificFamily {
    ScientificFamily::CALIBRATION
        .into_iter()
        .find(|family| family.as_str() == name)
        .unwrap()
}

fn check(actual: f64, expected: &str, label: &str) -> f64 {
    let expected = expected.parse::<f64>().unwrap();
    let scaled = (actual - expected).abs() / expected.abs().max(1.0);
    // Robertson combines 1e4/3e7 terms around a 1e-2 perturbation, so its
    // direct f64 evaluation loses slightly more than the 5e-14 used by the
    // smoother PDE oracle.  The measured worst case is 7.821e-14.
    assert!(scaled <= 1.0e-13, "{label}: scaled error {scaled:.3e}");
    scaled
}

#[test]
fn five_calibration_equations_match_the_shared_mpmath_oracle() {
    let oracle: Oracle = serde_json::from_str(include_str!(
        "../../../fixtures/scientific_corpus_v2_1_calibration_oracle.json"
    ))
    .unwrap();
    assert_eq!(
        oracle.schema_version,
        "scientific-corpus-v2.1-calibration-mpmath-oracle-v1"
    );
    assert_eq!(oracle.origin.engine, "mpmath");
    assert_eq!(oracle.origin.version, "1.3.0");
    assert_eq!(oracle.origin.decimal_digits, 80);
    assert_eq!(
        oracle.sampling,
        "all components of the first, middle, and last replicated block; scalar forcing uses first, middle, and last components"
    );
    assert_eq!(oracle.cases.len(), 5);
    let time = oracle.time.parse::<f64>().unwrap();

    for expected in oracle.cases {
        let expected_indices: Vec<usize> = match expected.family.as_str() {
            "robertson-ramped" => [0..3, 48..51, 93..96].into_iter().flatten().collect(),
            "hires-ramped" => [0..8, 48..56, 88..96].into_iter().flatten().collect(),
            "van-der-pol-ramped" | "rotating-nonnormal" => {
                vec![0, 1, 48, 49, 94, 95]
            }
            "nonautonomous-stiff-forcing" => vec![0, 48, 95],
            other => panic!("unexpected calibration oracle family {other}"),
        };
        assert_eq!(
            expected
                .samples
                .iter()
                .map(|sample| sample.index)
                .collect::<Vec<_>>(),
            expected_indices,
            "oracle must cover every component in each selected block"
        );
        let case = ScientificCorpusV2::calibration_specs()
            .into_iter()
            .find(|spec| {
                spec.family == family(&expected.family)
                    && spec.dimension == expected.dimension
                    && spec.rtol == 1.0e-6
            })
            .unwrap()
            .build()
            .unwrap();
        let state = case
            .y0
            .iter()
            .enumerate()
            .map(|(index, value)| value + 0.01 * (0.17 * (index + 1) as f64).cos())
            .collect::<Vec<_>>();
        let direction = (0..expected.dimension)
            .map(|index| (0.23 * (index + 1) as f64).sin())
            .collect::<Vec<_>>();
        let rhs = case
            .problem
            .eval_rhs(time, &state, &mut WorkCounters::default())
            .unwrap();
        let operator = case.problem.linearize_matrix_free(time, &state).unwrap();
        let mut jvp = vec![0.0; expected.dimension];
        operator.apply(&direction, &mut jvp).unwrap();
        let mut maximum = 0.0_f64;
        for sample in expected.samples {
            maximum = maximum.max(check(case.y0[sample.index], &sample.y0, "y0"));
            maximum = maximum.max(check(rhs[sample.index], &sample.rhs, "rhs"));
            maximum = maximum.max(check(jvp[sample.index], &sample.jvp, "jvp"));
        }
        println!("{} max scaled oracle error={maximum:.3e}", expected.family);
    }
}
