//! Contracts of `solve_gmres_into` (research node
//! `research/rnext02_gmres_into_20261003`): bitwise agreement with the
//! existing GMRES, output written only on success, declared capacity.

use rodas5p_core::{
    DenseMatrix, DenseOperator, IdentityPreconditioner, LinearSolveReport, WorkCounters,
};
use rodas5p_krylov::{
    CapacityGrowth, GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,
    solve_gmres_with_workspace_and_residual_scale,
};

fn matrix(n: usize) -> DenseMatrix {
    // Nonsymmetric, diagonally dominant enough to converge, with structure in
    // every row.
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 4.0 + 0.1 * i as f64;
        if i > 0 {
            a[(i, i - 1)] = -1.3;
        }
        if i + 1 < n {
            a[(i, i + 1)] = 0.7;
        }
        a[(i, (i * 7 + 3) % n)] += 0.2;
    }
    a
}

fn rhs(n: usize) -> Vec<f64> {
    (0..n).map(|i| ((i * 37 % 11) as f64 - 5.0) / 3.0).collect()
}

fn old(
    a: &DenseMatrix,
    b: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    scale: Option<&[f64]>,
) -> (Result<LinearSolveReport, String>, WorkCounters) {
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = IdentityPreconditioner::new(b.len());
    let mut counters = WorkCounters::default();
    let report = solve_gmres_with_workspace_and_residual_scale(
        &op,
        &pc,
        b,
        x0,
        config,
        scale,
        &mut GmresWorkspace::default(),
        &mut counters,
    )
    .map_err(|e| e.to_string());
    (report, counters)
}

#[allow(clippy::too_many_arguments)]
fn new(
    a: &DenseMatrix,
    b: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    scale: Option<&[f64]>,
    output: &mut [f64],
    workspace: &mut GmresWorkspace,
    capacity: GmresCapacity,
) -> (
    Result<rodas5p_krylov::GmresIntoReport, String>,
    WorkCounters,
) {
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = IdentityPreconditioner::new(b.len());
    let mut counters = WorkCounters::default();
    let report = solve_gmres_into(
        &op,
        &pc,
        b,
        x0,
        config,
        scale,
        output,
        workspace,
        capacity,
        &mut counters,
    )
    .map_err(|e| e.to_string());
    (report, counters)
}

#[test]
fn results_and_counters_are_bitwise_those_of_the_existing_gmres() {
    for n in [1, 3, 17, 60] {
        let (a, b) = (matrix(n), rhs(n));
        let x0: Vec<f64> = (0..n).map(|i| 0.01 * i as f64).collect();
        let scale: Vec<f64> = b.iter().map(|v| 1.0e-3 + v.abs()).collect();
        for (restart, budget, rtol) in [(40, 200, 1.0e-11), (5, 400, 1.0e-12), (2, 400, 1.0e-10)] {
            let config = GmresConfig {
                restart,
                max_arnoldi: budget,
                rtol,
                atol: 1.0e-14,
            };
            for (x0, scale) in [
                (None, None),
                (Some(x0.as_slice()), None),
                (None, Some(scale.as_slice())),
            ] {
                let (expected, expected_counters) = old(&a, &b, x0, &config, scale);
                let mut output = vec![f64::NAN; n];
                let (got, counters) = new(
                    &a,
                    &b,
                    x0,
                    &config,
                    scale,
                    &mut output,
                    &mut GmresWorkspace::default(),
                    GmresCapacity::unbounded(),
                );
                let (expected, got) = (expected.unwrap(), got.unwrap());
                let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
                assert_eq!(bits(&output), bits(&expected.x), "n={n} restart={restart}");
                assert_eq!(counters, expected_counters);
                assert_eq!(
                    got.residual_norm.to_bits(),
                    expected.residual_norm.to_bits()
                );
                assert_eq!(got.iterations, expected.iterations);
                assert_eq!(got.least_squares_solves, got.cycles);
            }
        }
    }
}

#[test]
fn a_failed_solve_leaves_the_output_unchanged_with_the_same_error() {
    let n = 40;
    let (a, b) = (matrix(n), rhs(n));
    let config = GmresConfig {
        restart: 2,
        max_arnoldi: 3,
        rtol: 1.0e-14,
        atol: 0.0,
    };
    let (expected, expected_counters) = old(&a, &b, None, &config, None);
    let mut output = vec![7.0; n];
    let (got, counters) = new(
        &a,
        &b,
        None,
        &config,
        None,
        &mut output,
        &mut GmresWorkspace::default(),
        GmresCapacity::unbounded(),
    );
    assert_eq!(got.unwrap_err(), expected.unwrap_err());
    assert_eq!(counters, expected_counters);
    assert!(output.iter().all(|v| *v == 7.0));
}

#[test]
fn a_refused_capacity_fails_before_any_operator_application() {
    let n = 30;
    let (a, b) = (matrix(n), rhs(n));
    let config = GmresConfig::default();
    for capacity in [
        GmresCapacity {
            max_dimension: n - 1,
            max_columns: 40,
            growth: CapacityGrowth::Refuse,
        },
        GmresCapacity {
            max_dimension: n,
            max_columns: 10,
            growth: CapacityGrowth::Refuse,
        },
    ] {
        let mut output = vec![3.0; n];
        let (got, counters) = new(
            &a,
            &b,
            None,
            &config,
            None,
            &mut output,
            &mut GmresWorkspace::default(),
            capacity,
        );
        assert!(got.unwrap_err().contains("KRYLOV_CAPACITY_EXCEEDED"));
        assert_eq!(counters, WorkCounters::default());
        assert!(output.iter().all(|v| *v == 3.0));
    }
    // A declared capacity that suffices: reserve once, no growth afterwards.
    let mut workspace = GmresWorkspace::default();
    workspace.reserve(n, 30).unwrap();
    let mut output = vec![0.0; n];
    let refuse = GmresCapacity {
        max_dimension: n,
        max_columns: 30,
        growth: CapacityGrowth::Refuse,
    };
    let (got, _) = new(
        &a,
        &b,
        None,
        &config,
        None,
        &mut output,
        &mut workspace,
        refuse,
    );
    assert!(!got.unwrap().workspace_grew);
}

#[test]
fn allowed_growth_is_reported_once() {
    let n = 25;
    let (a, b) = (matrix(n), rhs(n));
    let config = GmresConfig::default();
    let mut workspace = GmresWorkspace::default();
    let mut output = vec![0.0; n];
    let first = new(
        &a,
        &b,
        None,
        &config,
        None,
        &mut output,
        &mut workspace,
        GmresCapacity::unbounded(),
    )
    .0
    .unwrap();
    let second = new(
        &a,
        &b,
        None,
        &config,
        None,
        &mut output,
        &mut workspace,
        GmresCapacity::unbounded(),
    )
    .0
    .unwrap();
    assert!(first.workspace_grew && !second.workspace_grew);
}

#[test]
fn a_wrong_output_length_is_refused() {
    let (a, b) = (matrix(5), rhs(5));
    let mut output = vec![0.0; 4];
    let (got, counters) = new(
        &a,
        &b,
        None,
        &GmresConfig::default(),
        None,
        &mut output,
        &mut GmresWorkspace::default(),
        GmresCapacity::unbounded(),
    );
    assert!(got.is_err());
    assert_eq!(counters, WorkCounters::default());
}
