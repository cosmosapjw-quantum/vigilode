use rodas5p_core::{
    CoefficientPrecisionAvailability, DenseMatrix, RODAS5P_COEFFICIENT_SNAPSHOT_SCHEMA_VERSION,
    direct_solve, load_rodas5p_coefficients, safe_l2, wrms,
};

#[test]
fn safe_l2_handles_extreme_scale() {
    let n = safe_l2(&[3.0e200, 4.0e200]);
    assert!((n / 5.0e200 - 1.0).abs() < 1.0e-15);
}

#[test]
fn dense_matrix_matvec_is_row_major_and_exact_for_small_case() {
    let a = DenseMatrix::from_rows(&[&[2.0, -1.0], &[3.0, 4.0]]).unwrap();
    assert_eq!(a.matvec(&[5.0, 2.0]).unwrap(), vec![8.0, 23.0]);
}

#[test]
fn wrms_matches_definition() {
    let got = wrms(&[1.0, -2.0], &[2.0, 4.0]).unwrap();
    assert!((got - 0.5).abs() < 1.0e-15);
}

#[test]
fn coefficient_transform_preserves_rodas5p_structure() {
    let c = load_rodas5p_coefficients().unwrap();
    assert_eq!(c.stages(), 8);
    assert!((c.gamma - 0.211_937_563_194_290_14).abs() < 1.0e-16);
    for i in 0..8 {
        assert!((c.beta[(i, i)] - c.gamma).abs() < 2.0e-13);
        for j in i..8 {
            if i != j {
                assert!(c.l[(i, j)].abs() < 2.0e-13);
            }
        }
    }
}

#[test]
fn coefficient_snapshot_is_bit_identical_to_the_authoritative_sciml_literals() {
    // Defect caught: decimal-looking fractions are parsed through two rounded
    // f64 operands and silently move gamma or a weight by one ulp.
    let coefficients = load_rodas5p_coefficients().unwrap();
    let expected_b_code: [f64; 8] = [
        -7.502846399306121,
        2.561846144803919,
        -11.627539656261098,
        -0.18268767659942256,
        0.030198172008377946,
        1.0,
        1.0,
        1.0,
    ];
    let expected_a: [[f64; 8]; 8] = [
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [
            2.849394379747939,
            0.45842242204463923,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            -6.954028509809101,
            2.489845061869568,
            -10.358996098473584,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            2.8029986275628964,
            0.5072464736228206,
            -0.3988312541770524,
            -0.04721187230404641,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            -7.502846399306121,
            2.561846144803919,
            -11.627539656261098,
            -0.18268767659942256,
            0.030198172008377946,
            0.0,
            0.0,
            0.0,
        ],
        [
            -7.502846399306121,
            2.561846144803919,
            -11.627539656261098,
            -0.18268767659942256,
            0.030198172008377946,
            1.0,
            0.0,
            0.0,
        ],
        [
            -7.502846399306121,
            2.561846144803919,
            -11.627539656261098,
            -0.18268767659942256,
            0.030198172008377946,
            1.0,
            1.0,
            0.0,
        ],
    ];
    let expected_c_matrix: [[f64; 8]; 8] = [
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [-14.155112264123755, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [
            -17.97296035885952,
            -2.859693295451294,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            147.12150275711716,
            -1.41221402718213,
            71.68940251302358,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            165.43517024871676,
            -0.4592823456491126,
            42.90938336958603,
            -5.961986721573306,
            0.0,
            0.0,
            0.0,
            0.0,
        ],
        [
            24.854864614690072,
            -3.0009227002832186,
            47.4931110020768,
            5.5814197821558125,
            -0.6610691825249471,
            0.0,
            0.0,
            0.0,
        ],
        [
            30.91273214028599,
            -3.1208243349937974,
            77.79954646070892,
            34.28646028294783,
            -19.097331116725623,
            -28.087943162872662,
            0.0,
            0.0,
        ],
        [
            37.80277123390563,
            -3.2571969029072276,
            112.26918849496327,
            66.9347231244047,
            -40.06618937091002,
            -54.66780262877968,
            -9.48861652309627,
            0.0,
        ],
    ];
    let expected_c: [f64; 8] = [
        0.0,
        0.6358126895828704,
        0.4095798393397535,
        0.9769306725060716,
        0.4288403609558664,
        1.0,
        1.0,
        1.0,
    ];
    let expected_dense_h: [[f64; 8]; 3] = [
        [
            25.948786856663858,
            -2.5579724845846235,
            10.433815404888879,
            -2.3679251022685204,
            0.524948541321073,
            1.1241088310450404,
            0.4272876194431874,
            -0.17202221070155493,
        ],
        [
            -9.91568850695171,
            -0.9689944594115154,
            3.0438037242978453,
            -24.495224566215796,
            20.176138334709044,
            15.98066361424651,
            -6.789040303419874,
            -6.710236069923372,
        ],
        [
            11.419903575922262,
            2.8879645146136994,
            72.92137995996029,
            80.12511834622643,
            -52.072871366152654,
            -59.78993625266729,
            -0.15582684282751913,
            4.883087185713722,
        ],
    ];

    assert_eq!(
        coefficients.gamma.to_bits(),
        0.21193756319429014_f64.to_bits()
    );
    assert_eq!(coefficients.b_code.len(), expected_b_code.len());
    for (actual, expected) in coefficients.b_code.iter().zip(expected_b_code) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
    for row in 0..8 {
        for column in 0..8 {
            assert_eq!(
                coefficients.a[(row, column)].to_bits(),
                expected_a[row][column].to_bits(),
                "A[{row},{column}]"
            );
            assert_eq!(
                coefficients.c_matrix[(row, column)].to_bits(),
                expected_c_matrix[row][column].to_bits(),
                "C[{row},{column}]"
            );
        }
        assert_eq!(
            coefficients.c[row].to_bits(),
            expected_c[row].to_bits(),
            "c[{row}]"
        );
    }
    for (row, expected_row) in expected_dense_h.iter().enumerate() {
        for (column, expected) in expected_row.iter().enumerate() {
            assert_eq!(
                coefficients.dense_h[(row, column)].to_bits(),
                expected.to_bits(),
                "H[{row},{column}]"
            );
        }
    }

    assert_eq!(
        coefficients.snapshot_schema_version,
        RODAS5P_COEFFICIENT_SNAPSHOT_SCHEMA_VERSION
    );

    assert_eq!(
        coefficients.provenance.author_commit,
        "000230a3e53a445a22c090aff9148430367a0a74"
    );
    assert_eq!(
        coefficients.provenance.parity_commit,
        "0542c1019a8a3be6ea77fb5363d057d8c2cade9e"
    );
    assert_eq!(
        coefficients.provenance.source_repository,
        "https://github.com/SciML/OrdinaryDiffEq.jl"
    );
    assert_eq!(
        coefficients.provenance.author_source_path,
        "src/tableaus/rosenbrock_tableaus.jl"
    );
    assert_eq!(
        coefficients.provenance.author_source_sha256,
        "a556bfaf5fe302617e6665a1c4089398e5e126aee9346e5409c93ad83aae7b8e"
    );
    assert_eq!(
        coefficients.provenance.parity_source_path,
        "lib/OrdinaryDiffEqRosenbrock/src/rosenbrock_tableaus.jl"
    );
    assert_eq!(
        coefficients.provenance.parity_source_sha256,
        "60cd2aa9035717bb94252fea6da05380af3efa8b87fe29f151d66b92c3f5d443"
    );
    assert_eq!(coefficients.provenance.search_as_of, "2026-08-29");
    assert_eq!(
        coefficients.provenance.literal_semantics,
        "official ordinary decimal literals; not original exact rationals"
    );
    assert_eq!(
        coefficients.provenance.higher_precision,
        CoefficientPrecisionAvailability::NoPublicAuthoritativeValues
    );
}

#[test]
fn direct_solve_agrees_with_known_solution() {
    let a = DenseMatrix::from_rows(&[&[4.0, 1.0], &[2.0, 3.0]]).unwrap();
    let x = direct_solve(&a, &[9.0, 8.0]).unwrap();
    assert!((x[0] - 1.9).abs() < 1.0e-14);
    assert!((x[1] - 1.4).abs() < 1.0e-14);
}

#[test]
fn linear_operator_default_row_application_preserves_order() {
    use rodas5p_core::{ClosureOperator, LinearOperator};

    let op = ClosureOperator::new(3, |x, y| {
        y[0] = 2.0 * x[0];
        y[1] = -x[1];
        y[2] = x[0] + x[2];
        Ok(())
    });
    let inputs = vec![vec![1.0, 2.0, 3.0], vec![4.0, -5.0, 6.0]];
    let mut outputs = vec![vec![0.0; 3]; 2];
    op.apply_rows(&inputs, &mut outputs).unwrap();
    assert_eq!(outputs, vec![vec![2.0, -2.0, 4.0], vec![8.0, 5.0, 10.0]]);
}

#[test]
fn linear_operator_row_application_rejects_ragged_shapes() {
    use rodas5p_core::{ClosureOperator, LinearOperator};

    let op = ClosureOperator::new(2, |x, y| {
        y.copy_from_slice(x);
        Ok(())
    });
    let inputs = vec![vec![1.0, 2.0], vec![3.0]];
    let mut outputs = vec![vec![0.0; 2]; 2];
    assert!(op.apply_rows(&inputs, &mut outputs).is_err());
}
