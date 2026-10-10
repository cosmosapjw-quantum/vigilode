//! Kernel boundary contracts of the GMRES residual accounting (research
//! node `research/sp01_dupfix_adoption_20261010`, SP01).
//!
//! For both production kernels, `solve_gmres_with_workspace_and_accounting`
//! (`gmres.rs`) and `solve_gmres_into_with_accounting` (`gmres_into.rs`),
//! `ResidualAccounting::RecomputeFinal` (v1) and `ReuseConfirmed` (v2) give
//! the same solution bits, the same outcome (report or error) and the same
//! residual norm on every registered boundary case: a zero right-hand side,
//! a nonzero initial guess already within the threshold, budget exhaustion,
//! a happy breakdown (confirmed, and not confirmed so that the budget runs
//! out), Jacobi left preconditioning, a WRMS residual scale and a restart
//! smaller than the iteration count. The counters differ by exactly one
//! diagnostic operator application when the solve left its loop on a
//! confirmed true residual of the same iterate after at least one cycle,
//! and by nothing otherwise. That count is taken independently from the
//! operator's own record of its inputs (the last two applications of the
//! v1 solve act on the same nonzero vector and follow an Arnoldi
//! application to a unit vector), not from the counters.
//!
//! ALG04's `GmresIntoOptions::skip_final_residual` (arm `DupFix`) is
//! unchanged: it skips the final residual at every successful exit and
//! overrides the accounting.

use std::sync::Mutex;

use rodas5p_core::{
    ApplyCategory, CoreResult, DenseMatrix, DenseOperator, IdentityPreconditioner,
    JacobiPreconditioner, LinearOperator, OperatorApplicationWork, Preconditioner, WorkCounters,
    apply_counted,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresWorkspace, ResidualAccounting,
    solve_gmres_into, solve_gmres_into_with_accounting, solve_gmres_into_with_options,
    solve_gmres_with_workspace_and_accounting, solve_gmres_with_workspace_and_residual_scale,
};

/// A dense operator that records the bits of every input it is applied to.
struct Recording {
    inner: DenseOperator,
    inputs: Mutex<Vec<Vec<u64>>>,
}

impl Recording {
    fn new(a: &DenseMatrix) -> Self {
        Self {
            inner: DenseOperator::new(a.clone()).unwrap(),
            inputs: Mutex::new(Vec::new()),
        }
    }

    fn inputs(&self) -> Vec<Vec<u64>> {
        self.inputs.lock().unwrap().clone()
    }
}

impl LinearOperator for Recording {
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.inputs
            .lock()
            .unwrap()
            .push(x.iter().map(|v| v.to_bits()).collect());
        self.inner.apply(x, y)
    }
    fn application_work(&self) -> OperatorApplicationWork {
        self.inner.application_work()
    }
    fn token(&self) -> u64 {
        self.inner.token()
    }
}

/// The confirmed exits seen in an operator's input record: an application
/// to the same nonzero vector as the one before it, which itself followed an
/// application to a unit 2-norm vector (an Arnoldi basis vector).
fn confirmed_exits(inputs: &[Vec<u64>]) -> usize {
    let norm = |v: &[u64]| {
        v.iter()
            .map(|b| f64::from_bits(*b).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    (2..inputs.len())
        .filter(|&k| {
            inputs[k] == inputs[k - 1]
                && inputs[k].iter().any(|b| f64::from_bits(*b) != 0.0)
                && (norm(&inputs[k - 2]) - 1.0).abs() <= 1.0e-10
        })
        .count()
}

fn unit(v: &[u64]) -> bool {
    let norm = v
        .iter()
        .map(|b| f64::from_bits(*b).powi(2))
        .sum::<f64>()
        .sqrt();
    (norm - 1.0).abs() <= 1.0e-10
}

/// Arnoldi cycles in an operator record: maximal runs of applications to
/// unit vectors (the basis vectors of one cycle).
fn arnoldi_cycles(inputs: &[Vec<u64>]) -> usize {
    (0..inputs.len())
        .filter(|&k| unit(&inputs[k]) && (k == 0 || !unit(&inputs[k - 1])))
        .count()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kernel {
    /// `gmres.rs`: `solve_gmres_with_workspace_and_accounting`.
    Legacy,
    /// `gmres_into.rs`: `solve_gmres_into_with_accounting`.
    Into,
}

/// What a solve returned, every float as bits.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Outcome {
    /// residual norm, relative residual, iterations, matvecs, preconditioner
    /// applications; or the error text.
    report: Result<[u64; 5], String>,
    solution: Vec<u64>,
    counters: WorkCounters,
    confirmed_exits: usize,
    /// Restart cycles, counted from the operator record: applications that
    /// follow a non-unit input (the loop's true residual) or start the solve.
    cycles: usize,
}

struct Case<'a> {
    a: &'a DenseMatrix,
    b: &'a [f64],
    x0: Option<&'a [f64]>,
    config: GmresConfig,
    scale: Option<&'a [f64]>,
    jacobi: bool,
}

fn solve(case: &Case<'_>, kernel: Kernel, accounting: ResidualAccounting, skip: bool) -> Outcome {
    let op = Recording::new(case.a);
    let n = case.b.len();
    let pc: Box<dyn Preconditioner> = if case.jacobi {
        Box::new(JacobiPreconditioner::from_matrix(case.a).unwrap())
    } else {
        Box::new(IdentityPreconditioner::new(n))
    };
    let mut counters = WorkCounters::default();
    let mut workspace = GmresWorkspace::default();
    let (report, solution) = match kernel {
        Kernel::Legacy => {
            assert!(!skip, "the skip is a GMRES-into option");
            match solve_gmres_with_workspace_and_accounting(
                &op,
                pc.as_ref(),
                case.b,
                case.x0,
                &case.config,
                case.scale,
                &mut workspace,
                accounting,
                &mut counters,
            ) {
                Ok(r) => (
                    Ok([
                        r.residual_norm.to_bits(),
                        r.relative_residual.to_bits(),
                        r.iterations,
                        r.matvecs,
                        r.preconditioner_apps,
                    ]),
                    r.x.iter().map(|v| v.to_bits()).collect(),
                ),
                Err(e) => (Err(e.to_string()), Vec::new()),
            }
        }
        Kernel::Into => {
            let mut output = vec![f64::NAN; n];
            let report = solve_gmres_into_with_accounting(
                &op,
                pc.as_ref(),
                case.b,
                case.x0,
                &case.config,
                case.scale,
                &mut output,
                &mut workspace,
                GmresCapacity::unbounded(),
                GmresIntoOptions {
                    skip_final_residual: skip,
                },
                accounting,
                &mut counters,
            );
            (
                report
                    .map(|r| {
                        [
                            r.residual_norm.to_bits(),
                            r.relative_residual.to_bits(),
                            r.iterations,
                            r.matvecs,
                            r.preconditioner_apps,
                        ]
                    })
                    .map_err(|e| e.to_string()),
                output.iter().map(|v| v.to_bits()).collect(),
            )
        }
    };
    let inputs = op.inputs();
    Outcome {
        report,
        solution,
        counters,
        confirmed_exits: confirmed_exits(&inputs),
        cycles: arnoldi_cycles(&inputs),
    }
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

/// The registered parity and exact accounting on one case, for both
/// kernels; returns the v1 outcomes (Legacy, Into). `expected_confirmed`,
/// when given, pins the confirmed-exit count of the operator record.
fn check(label: &str, case: &Case<'_>, expected_confirmed: Option<usize>) -> [Outcome; 2] {
    let mut out = Vec::new();
    for kernel in [Kernel::Legacy, Kernel::Into] {
        let v1 = solve(case, kernel, ResidualAccounting::RecomputeFinal, false);
        let v2 = solve(case, kernel, ResidualAccounting::ReuseConfirmed, false);
        // Parity: solution, outcome and every report field, bit for bit.
        assert_eq!(v2.report, v1.report, "{label} {kernel:?} report");
        assert_eq!(v2.solution, v1.solution, "{label} {kernel:?} solution");
        // Exact accounting, with the count taken from the operator record.
        if let Some(expected) = expected_confirmed {
            assert_eq!(
                v1.confirmed_exits, expected,
                "{label} {kernel:?} confirmed exits"
            );
        }
        assert!(v1.confirmed_exits <= 1, "{label} {kernel:?} one solve");
        assert_eq!(v2.confirmed_exits, 0, "{label} {kernel:?} v2 duplicates");
        let mut expected = v2.counters;
        for _ in 0..v1.confirmed_exits {
            expected.accumulate(one_diagnostic(case.a));
        }
        assert_eq!(expected, v1.counters, "{label} {kernel:?} counters");
        if v1.report.is_ok() {
            assert_eq!(
                v1.counters.diagnostic_matvecs, 1,
                "{label} {kernel:?} v1 recomputes"
            );
        }
        // ALG04's DupFix arm, unchanged: one diagnostic fewer than v1 per
        // successful solve, whatever the accounting.
        if kernel == Kernel::Into {
            for accounting in [
                ResidualAccounting::RecomputeFinal,
                ResidualAccounting::ReuseConfirmed,
            ] {
                let skipped = solve(case, kernel, accounting, true);
                assert_eq!(skipped.report, v1.report, "{label} DupFix report");
                assert_eq!(skipped.solution, v1.solution, "{label} DupFix solution");
                let mut expected = skipped.counters;
                if v1.report.is_ok() {
                    assert_eq!(skipped.counters.diagnostic_matvecs, 0);
                    expected.accumulate(one_diagnostic(case.a));
                }
                assert_eq!(expected, v1.counters, "{label} DupFix counters");
            }
        }
        out.push(v1);
    }
    let [legacy, into]: [Outcome; 2] = out.try_into().unwrap();
    // The two kernels agree on the outcome (R-NEXT-02).
    assert_eq!(
        legacy.report.is_ok(),
        into.report.is_ok(),
        "{label} kernels"
    );
    [legacy, into]
}

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

fn rhs(n: usize) -> Vec<f64> {
    (0..n).map(|i| ((i * 37 % 11) as f64 - 5.0) / 3.0).collect()
}

fn config(restart: usize, max_arnoldi: usize, rtol: f64, atol: f64) -> GmresConfig {
    GmresConfig {
        restart,
        max_arnoldi,
        rtol,
        atol,
    }
}

#[test]
fn zero_right_hand_side_keeps_the_recomputation() {
    let a = matrix(12);
    let b = vec![0.0; 12];
    for atol in [0.0, 1.0e-13] {
        let case = Case {
            a: &a,
            b: &b,
            x0: None,
            config: config(5, 20, 1.0e-10, atol),
            scale: None,
            jacobi: false,
        };
        let [legacy, into] = check("zero rhs", &case, Some(0));
        for v1 in [legacy, into] {
            let report = v1.report.unwrap();
            assert_eq!(f64::from_bits(report[0]), 0.0);
            assert_eq!(report[2], 0, "no iteration");
            assert_eq!(v1.counters.linear_matvecs, 0);
        }
    }
}

#[test]
fn a_nonzero_start_within_the_threshold_keeps_the_recomputation() {
    let n = 24;
    let a = matrix(n);
    let b = rhs(n);
    let tight = Case {
        a: &a,
        b: &b,
        x0: None,
        config: config(40, 200, 1.0e-14, 0.0),
        scale: None,
        jacobi: false,
    };
    let [legacy, _] = check("tight solve", &tight, Some(1));
    let x_star: Vec<f64> = legacy.solution.iter().map(|b| f64::from_bits(*b)).collect();
    let case = Case {
        a: &a,
        b: &b,
        x0: Some(&x_star),
        config: config(40, 200, 1.0e-8, 0.0),
        scale: None,
        jacobi: false,
    };
    let [legacy, into] = check("x0 within threshold", &case, Some(0));
    for v1 in [legacy, into] {
        assert_eq!(v1.report.unwrap()[2], 0, "no iteration");
        // The loop's true residual and the recomputation.
        assert_eq!(v1.counters.linear_matvecs, 1);
        assert_eq!(v1.counters.diagnostic_matvecs, 1);
        assert_eq!(
            v1.solution,
            x_star.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn budget_exhaustion_fails_identically() {
    let n = 48;
    let a = matrix(n);
    let b = rhs(n);
    for (restart, max_arnoldi) in [(2, 4), (5, 5), (3, 7)] {
        let case = Case {
            a: &a,
            b: &b,
            x0: None,
            config: config(restart, max_arnoldi, 1.0e-14, 0.0),
            scale: None,
            jacobi: false,
        };
        let [legacy, into] = check("budget exhaustion", &case, Some(0));
        for v1 in [legacy, into] {
            let error = v1.report.unwrap_err();
            assert!(error.contains("exhausted"), "{error}");
            assert_eq!(v1.counters.diagnostic_matvecs, 0);
            assert_eq!(v1.counters.linear_solves, 0);
        }
    }
}

/// `diag(d)` with a right-hand side in a 3-dimensional invariant subspace:
/// the Arnoldi process breaks down happily at its third column.
fn breakdown_system(n: usize) -> (DenseMatrix, Vec<f64>) {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 3.0 + 1.7 * i as f64;
    }
    let mut b = vec![0.0; n];
    b[1] = 0.3;
    b[4] = -1.1;
    b[6] = 0.7;
    (a, b)
}

#[test]
fn a_confirmed_happy_breakdown_reuses_the_residual() {
    let (a, b) = breakdown_system(10);
    let case = Case {
        a: &a,
        b: &b,
        x0: None,
        config: config(8, 40, 1.0e-12, 0.0),
        scale: None,
        jacobi: false,
    };
    let [legacy, into] = check("happy breakdown", &case, Some(1));
    for v1 in [legacy, into] {
        assert_eq!(v1.cycles, 1);
        assert_eq!(v1.report.unwrap()[2], 3, "breakdown at the third column");
    }
}

/// A 3 x 3 nonsymmetric block in the leading coordinates (an invariant
/// subspace that holds the right-hand side) and a diagonal elsewhere: the
/// Arnoldi process breaks down happily at its third column, and the
/// breakdown iterate's true residual is a round-off vector.
fn block_breakdown_system(n: usize, seed: f64) -> (DenseMatrix, Vec<f64>) {
    let mut a = DenseMatrix::zeros(n, n);
    let block = [
        [3.1 + seed, 0.7, -1.3],
        [0.2, 2.9, 0.45 * seed],
        [-0.6, 1.1, 4.3],
    ];
    for (i, row) in block.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            a[(i, j)] = *value;
        }
    }
    for i in 3..n {
        a[(i, i)] = 3.0 + 1.7 * i as f64;
    }
    let mut b = vec![0.0; n];
    b[0] = 0.3 / seed;
    b[1] = -1.1;
    b[2] = 0.7 * seed;
    (a, b)
}

#[test]
fn a_happy_breakdown_without_confirmation_fails_identically() {
    // A zero threshold: when the breakdown iterate's true residual is a
    // round-off vector, not zero, the loop restarts until the budget runs
    // out.
    let mut unconfirmed = 0;
    for seed in [0.37, 1.0 / 3.0, 0.71, 1.9, 2.3] {
        let (a, b) = block_breakdown_system(9, seed);
        let case = Case {
            a: &a,
            b: &b,
            x0: None,
            config: config(8, 9, 0.0, 0.0),
            scale: None,
            jacobi: false,
        };
        let [legacy, into] = check("unconfirmed breakdown", &case, None);
        if legacy.report.is_err() && into.report.is_err() {
            assert_eq!(legacy.confirmed_exits + into.confirmed_exits, 0);
            assert_eq!(legacy.counters.diagnostic_matvecs, 0);
            // Three cycles within a budget of 9 columns at restart 8 and
            // n = 9: every cycle ended in a happy breakdown.
            assert!(legacy.cycles >= 3 && into.cycles >= 3, "{seed}");
            unconfirmed += 1;
        }
    }
    assert!(unconfirmed >= 1, "at least one breakdown is not confirmed");
}

#[test]
fn jacobi_left_preconditioning() {
    let n = 32;
    let mut a = matrix(n);
    for i in 0..n {
        a[(i, i)] *= 1.0 + 10.0 * (i % 5) as f64;
    }
    let b = rhs(n);
    for x0 in [None, Some(vec![0.05; n])] {
        let case = Case {
            a: &a,
            b: &b,
            x0: x0.as_deref(),
            config: config(6, 200, 1.0e-10, 0.0),
            scale: None,
            jacobi: true,
        };
        let [legacy, _] = check("jacobi", &case, Some(1));
        assert!(legacy.counters.preconditioner_apps > 0);
    }
}

#[test]
fn wrms_residual_scale() {
    let n = 40;
    let a = matrix(n);
    let b = rhs(n);
    let scale: Vec<f64> = (0..n)
        .map(|i| 1.0e-3 * (1.0 + 0.5 * (i % 3) as f64))
        .collect();
    for (restart, rtol) in [(40, 1.0e-9), (4, 1.0e-6)] {
        let case = Case {
            a: &a,
            b: &b,
            x0: None,
            config: config(restart, 400, rtol, 1.0e-12),
            scale: Some(&scale),
            jacobi: false,
        };
        check("wrms", &case, Some(1));
    }
}

#[test]
fn a_restart_smaller_than_the_iteration_count() {
    let n = 48;
    let a = matrix(n);
    let b = rhs(n);
    let x0: Vec<f64> = (0..n).map(|i| 0.01 * i as f64).collect();
    for start in [None, Some(&x0[..])] {
        for restart in [2, 3, 5] {
            let case = Case {
                a: &a,
                b: &b,
                x0: start,
                config: config(restart, 400, 1.0e-10, 0.0),
                scale: None,
                jacobi: false,
            };
            let [legacy, into] = check("restart", &case, Some(1));
            for v1 in [legacy, into] {
                assert!(v1.report.unwrap()[2] > restart as u64, "several cycles");
            }
        }
    }
}

/// The entry points without an accounting argument use
/// `ResidualAccounting::DEFAULT`, whatever SP01's decision rule fixed.
#[test]
fn the_entry_points_without_accounting_use_the_default() {
    assert_eq!(ResidualAccounting::default(), ResidualAccounting::DEFAULT);
    let n = 30;
    let a = matrix(n);
    let b = rhs(n);
    let cfg = config(7, 200, 1.0e-10, 0.0);
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc = IdentityPreconditioner::new(n);
    let run_legacy = |accounting: Option<ResidualAccounting>| {
        let mut counters = WorkCounters::default();
        let mut workspace = GmresWorkspace::default();
        let report = match accounting {
            None => solve_gmres_with_workspace_and_residual_scale(
                &op,
                &pc,
                &b,
                None,
                &cfg,
                None,
                &mut workspace,
                &mut counters,
            ),
            Some(accounting) => solve_gmres_with_workspace_and_accounting(
                &op,
                &pc,
                &b,
                None,
                &cfg,
                None,
                &mut workspace,
                accounting,
                &mut counters,
            ),
        }
        .unwrap();
        (format!("{report:?}"), counters)
    };
    assert_eq!(
        run_legacy(None),
        run_legacy(Some(ResidualAccounting::DEFAULT))
    );
    let run_into = |which: u8| {
        let mut counters = WorkCounters::default();
        let mut workspace = GmresWorkspace::default();
        let mut output = vec![0.0; n];
        let report = match which {
            0 => solve_gmres_into(
                &op,
                &pc,
                &b,
                None,
                &cfg,
                None,
                &mut output,
                &mut workspace,
                GmresCapacity::unbounded(),
                &mut counters,
            ),
            1 => solve_gmres_into_with_options(
                &op,
                &pc,
                &b,
                None,
                &cfg,
                None,
                &mut output,
                &mut workspace,
                GmresCapacity::unbounded(),
                GmresIntoOptions::default(),
                &mut counters,
            ),
            _ => solve_gmres_into_with_accounting(
                &op,
                &pc,
                &b,
                None,
                &cfg,
                None,
                &mut output,
                &mut workspace,
                GmresCapacity::unbounded(),
                GmresIntoOptions::default(),
                ResidualAccounting::DEFAULT,
                &mut counters,
            ),
        }
        .unwrap();
        (format!("{report:?}"), output, counters)
    };
    assert_eq!(run_into(0), run_into(2));
    assert_eq!(run_into(1), run_into(2));
}

#[test]
fn accounting_identifiers_are_versioned() {
    assert_eq!(
        ResidualAccounting::RecomputeFinal.id(),
        "gmres-residual-accounting-recompute-final-v1"
    );
    assert_eq!(
        ResidualAccounting::ReuseConfirmed.id(),
        "gmres-residual-accounting-reuse-confirmed-v2"
    );
}
