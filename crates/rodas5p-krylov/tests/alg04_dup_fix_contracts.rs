//! Contracts of `solve_gmres_into_with_options` with `skip_final_residual`
//! (research node `research/alg04_coupled_target_v2_20261010`, arm
//! `DupFix`): the same solution and report bit for bit as
//! `solve_gmres_into`, and counters that differ by exactly one diagnostic
//! operator application per successful solve.

use rodas5p_core::{
    ApplyCategory, DenseMatrix, DenseOperator, IdentityPreconditioner, WorkCounters, apply_counted,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresIntoReport, GmresWorkspace,
    solve_gmres_into, solve_gmres_into_with_options,
};

fn matrix(n: usize) -> DenseMatrix {
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

type Run = (Result<GmresIntoReport, String>, Vec<u64>, WorkCounters);

fn run(
    a: &DenseMatrix,
    b: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    scale: Option<&[f64]>,
    skip: Option<bool>,
) -> Run {
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = IdentityPreconditioner::new(b.len());
    let mut counters = WorkCounters::default();
    let mut output = vec![f64::NAN; b.len()];
    let mut workspace = GmresWorkspace::default();
    let report = match skip {
        None => solve_gmres_into(
            &op,
            &pc,
            b,
            x0,
            config,
            scale,
            &mut output,
            &mut workspace,
            GmresCapacity::unbounded(),
            &mut counters,
        ),
        Some(skip_final_residual) => solve_gmres_into_with_options(
            &op,
            &pc,
            b,
            x0,
            config,
            scale,
            &mut output,
            &mut workspace,
            GmresCapacity::unbounded(),
            GmresIntoOptions {
                skip_final_residual,
            },
            &mut counters,
        ),
    }
    .map_err(|e| e.to_string());
    (
        report,
        output.iter().map(|v| v.to_bits()).collect(),
        counters,
    )
}

/// The work of one diagnostic application of the dense operator.
fn one_diagnostic(a: &DenseMatrix) -> WorkCounters {
    let op = DenseOperator::new(a.clone()).unwrap();
    let n = a.nrows();
    let mut counters = WorkCounters::default();
    apply_counted(
        &op,
        &vec![1.0; n],
        &mut vec![0.0; n],
        &mut counters,
        ApplyCategory::Diagnostic,
    )
    .unwrap();
    counters
}

#[test]
fn skipping_the_final_residual_changes_only_the_diagnostic_counters() {
    let n = 48;
    let a = matrix(n);
    let b: Vec<f64> = (0..n).map(|i| ((i * 37 % 11) as f64 - 5.0) / 3.0).collect();
    let x0: Vec<f64> = (0..n).map(|i| 0.01 * i as f64).collect();
    let scale: Vec<f64> = (0..n).map(|i| 1.0 + 0.5 * (i % 3) as f64).collect();
    let mut solved = 0;
    for (restart, max_arnoldi) in [(40, 200), (5, 400), (3, 6)] {
        for start in [None, Some(&x0[..])] {
            for weights in [None, Some(&scale[..])] {
                let config = GmresConfig {
                    restart,
                    max_arnoldi,
                    rtol: 1.0e-10,
                    atol: 0.0,
                };
                let legacy = run(&a, &b, start, &config, weights, None);
                let explicit = run(&a, &b, start, &config, weights, Some(false));
                assert_eq!(format!("{legacy:?}"), format!("{explicit:?}"));
                let skipped = run(&a, &b, start, &config, weights, Some(true));
                // Same solution bits and the same report.
                assert_eq!(skipped.0, legacy.0);
                assert_eq!(skipped.1, legacy.1);
                let mut expected = skipped.2;
                if legacy.0.is_ok() {
                    solved += 1;
                    assert_eq!(skipped.2.diagnostic_matvecs, 0);
                    expected.accumulate(one_diagnostic(&a));
                }
                assert_eq!(expected, legacy.2, "restart {restart} budget {max_arnoldi}");
            }
        }
    }
    assert!(solved >= 8, "most configurations converge");
}

#[test]
fn a_right_hand_side_already_within_the_threshold_needs_no_operator() {
    let a = matrix(6);
    let b = vec![1.0e-30; 6];
    let config = GmresConfig {
        restart: 6,
        max_arnoldi: 12,
        rtol: 1.0e-10,
        atol: 1.0e-20,
    };
    let legacy = run(&a, &b, None, &config, None, None);
    let skipped = run(&a, &b, None, &config, None, Some(true));
    assert_eq!(skipped.0, legacy.0);
    assert_eq!(skipped.1, vec![0; 6]);
    assert_eq!(legacy.2.diagnostic_matvecs, 1);
    assert_eq!(
        skipped.2,
        WorkCounters {
            linear_solves: 1,
            ..WorkCounters::default()
        }
    );
}
