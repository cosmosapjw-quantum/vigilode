//! Contracts of the column-tested restarted GMRES of loop node V24 (owner
//! decision OD-L1-11), `StagedGmresWorkspace::solve_column_tested_into`:
//!
//! * the projected residual after each Arnoldi column is the true residual
//!   of the iterate of that column, within rounding;
//! * a cycle ends at the first column that meets the threshold, and
//!   otherwise at its upper bound (the configured restart length, or 10
//!   doubled after every cycle that is followed by another one);
//! * an iterate is returned only if its true residual meets the threshold of
//!   the shipped kernel; a failed true test continues the solve; failures
//!   are the shipped kernel's errors and leave the output untouched.

use std::sync::atomic::{AtomicUsize, Ordering};

use rodas5p_core::{
    CoreError, CoreResult, DenseMatrix, DenseOperator, IdentityPreconditioner, LinearOperator,
    WorkCounters, safe_l2,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoOptions, GmresWorkspace, ResidualAccounting,
    StagedGmresWorkspace, solve_gmres_into_with_accounting,
};

fn tridiagonal(n: usize, sub: f64, diag: f64, sup: f64) -> DenseMatrix {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = diag;
        if i > 0 {
            a[(i, i - 1)] = sub;
        }
        if i + 1 < n {
            a[(i, i + 1)] = sup;
        }
    }
    a
}

/// Nonsymmetric, with structure in every row.
fn scattered(n: usize) -> DenseMatrix {
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

/// `I - s J` with a 1D diffusion-advection stencil `J`: the shape of a
/// stage system of the matrix-free driver.
fn shifted_stencil(n: usize, shift: f64) -> DenseMatrix {
    let mut a = DenseMatrix::identity(n);
    for i in 0..n {
        a[(i, i)] += shift * 2.0;
        if i > 0 {
            a[(i, i - 1)] -= shift * 1.25;
        }
        if i + 1 < n {
            a[(i, i + 1)] -= shift * 0.75;
        }
    }
    a
}

/// `I - s J` with strongly nonnormal 2x2 blocks.
fn nonnormal_blocks(n: usize, shift: f64) -> DenseMatrix {
    let mut a = DenseMatrix::identity(n);
    for block in 0..n / 2 {
        let rate = 1.0 + block as f64;
        let (p, q) = (2 * block, 2 * block + 1);
        a[(p, p)] += shift * 2.0 * rate;
        a[(p, q)] -= shift * 64.0 * rate;
        a[(q, p)] -= shift * rate / 64.0;
        a[(q, q)] += shift * 2.0 * rate;
        if q + 1 < n {
            a[(q, q + 1)] -= 0.5 * shift;
        }
    }
    a
}

fn wave(n: usize) -> Vec<f64> {
    (0..n).map(|i| ((i * 37 % 11) as f64 - 5.0) / 3.0).collect()
}

fn frobenius(a: &DenseMatrix) -> f64 {
    let n = a.nrows();
    let mut sum = 0.0;
    for i in 0..n {
        for j in 0..a.ncols() {
            sum += a[(i, j)] * a[(i, j)];
        }
    }
    sum.sqrt()
}

fn residual_norm(a: &DenseMatrix, b: &[f64], x: &[f64]) -> f64 {
    let ax = a.matvec(x).unwrap();
    safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>())
}

fn config(restart: usize, budget: usize, rtol: f64, atol: f64) -> GmresConfig {
    GmresConfig {
        restart,
        max_arnoldi: budget,
        rtol,
        atol,
    }
}

/// One monitored column.
#[derive(Clone, Debug)]
struct Column {
    cycle: u64,
    column: usize,
    projected: f64,
    iterate: Vec<f64>,
}

/// The systems of the column contracts: `(name, A, b, restart, rtol)`.
fn systems() -> Vec<(&'static str, DenseMatrix, Vec<f64>, usize, f64)> {
    vec![
        (
            "tridiagonal-40",
            tridiagonal(40, -1.0, 10.0, -2.0),
            (0..40).map(|i| 1.0 + 0.1 * i as f64).collect(),
            40,
            1.0e-10,
        ),
        ("scattered-60", scattered(60), wave(60), 40, 1.0e-11),
        (
            "shifted-stencil-90",
            shifted_stencil(90, 4.0),
            (0..90).map(|i| (0.3 * i as f64).sin() + 0.2).collect(),
            40,
            1.0e-10,
        ),
        (
            "nonnormal-blocks-24",
            nonnormal_blocks(24, 0.05),
            wave(24),
            24,
            1.0e-10,
        ),
        (
            "shifted-stencil-90-restart-12",
            shifted_stencil(90, 4.0),
            wave(90),
            12,
            1.0e-10,
        ),
    ]
}

/// Solve with a monitor; returns the columns, the report fields the tests
/// use, the solution and the counters.
#[allow(clippy::type_complexity)]
fn monitored(
    a: &DenseMatrix,
    b: &[f64],
    x0: Option<&[f64]>,
    config: &GmresConfig,
    first_cycle: usize,
    accounting: ResidualAccounting,
) -> (
    Vec<Column>,
    CoreResult<(f64, f64, f64, [u64; 7])>,
    Vec<f64>,
    WorkCounters,
) {
    let op = DenseOperator::new(a.clone()).unwrap();
    let mut columns = Vec::new();
    let mut monitor = |cycle: u64, column: usize, projected: f64, iterate: &[f64]| {
        columns.push(Column {
            cycle,
            column,
            projected,
            iterate: iterate.to_vec(),
        });
    };
    let mut x = vec![f64::NAN; b.len()];
    let mut counters = WorkCounters::default();
    let mut workspace = StagedGmresWorkspace::default();
    let report = workspace
        .solve_column_tested_into(
            &op,
            b,
            x0,
            config,
            first_cycle,
            accounting,
            &mut x,
            &mut counters,
            Some(&mut monitor),
        )
        .map(|r| {
            (
                r.threshold,
                r.residual_norm,
                r.projected_residual,
                [
                    r.columns,
                    r.cycles,
                    r.projected_exits,
                    r.failed_confirmations,
                    r.bound_exits,
                    r.longest_cycle,
                    r.true_residuals,
                ],
            )
        });
    (columns, report, x, counters)
}

#[test]
fn the_projected_residual_is_the_true_residual_of_the_iterate_of_every_column() {
    let mut checked = 0;
    for (name, a, b, restart, rtol) in systems() {
        let norm_a = frobenius(&a);
        let norm_b = safe_l2(&b);
        for first_cycle in [restart, 10.min(restart)] {
            let (columns, report, _, _) = monitored(
                &a,
                &b,
                None,
                &config(restart, 200, rtol, 0.0),
                first_cycle,
                ResidualAccounting::RecomputeFinal,
            );
            report.unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(!columns.is_empty(), "{name}");
            for column in &columns {
                let truth = residual_norm(&a, &b, &column.iterate);
                // Rounding of one residual: a few units in the last place of
                // the terms that are subtracted.
                let rounding = 64.0 * f64::EPSILON * (norm_b + norm_a * safe_l2(&column.iterate));
                assert!(
                    (column.projected - truth).abs() <= rounding,
                    "{name} first cycle {first_cycle} cycle {} column {}: projected {:e}, true {:e}, allowed {:e}",
                    column.cycle,
                    column.column,
                    column.projected,
                    truth,
                    rounding
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 200, "{checked} columns checked");
}

/// The cycle lengths of a monitored solve, in order.
fn cycle_lengths(columns: &[Column]) -> Vec<usize> {
    let mut lengths: Vec<usize> = Vec::new();
    for column in columns {
        if column.cycle as usize == lengths.len() {
            lengths.push(0);
        }
        assert_eq!(column.cycle as usize + 1, lengths.len());
        assert_eq!(column.column, lengths[column.cycle as usize] + 1);
        lengths[column.cycle as usize] = column.column;
    }
    lengths
}

#[test]
fn a_cycle_ends_at_the_first_column_that_meets_the_threshold() {
    for (name, a, b, restart, rtol) in systems() {
        for first_cycle in [restart, 10.min(restart)] {
            let cfg = config(restart, 200, rtol, 0.0);
            let (columns, report, x, counters) = monitored(
                &a,
                &b,
                None,
                &cfg,
                first_cycle,
                ResidualAccounting::RecomputeFinal,
            );
            let (threshold, residual, projected, numbers) =
                report.unwrap_or_else(|e| panic!("{name}: {e}"));
            let [
                total,
                cycles,
                projected_exits,
                failed,
                bound_exits,
                longest,
                true_residuals,
            ] = numbers;
            assert_eq!(threshold, rtol * safe_l2(&b), "{name}");
            let lengths = cycle_lengths(&columns);
            assert_eq!(lengths.len() as u64, cycles, "{name}");
            assert_eq!(lengths.iter().sum::<usize>() as u64, total, "{name}");
            assert_eq!(*lengths.iter().max().unwrap() as u64, longest, "{name}");
            let mut bound = first_cycle.min(restart);
            let (mut seen_projected, mut seen_bound) = (0, 0);
            for (cycle, &length) in lengths.iter().enumerate() {
                let of_cycle: Vec<&Column> = columns
                    .iter()
                    .filter(|c| c.cycle as usize == cycle)
                    .collect();
                // No column before the last one met the threshold.
                for column in &of_cycle[..length - 1] {
                    assert!(
                        column.projected > threshold,
                        "{name}: cycle {cycle} ran past column {} with projected {:e} <= {threshold:e}",
                        column.column,
                        column.projected
                    );
                }
                let last = of_cycle[length - 1];
                if last.projected <= threshold {
                    seen_projected += 1;
                } else {
                    // Otherwise the cycle ran to its upper bound.
                    assert_eq!(length, bound, "{name}: cycle {cycle}");
                    seen_bound += 1;
                }
                assert!(
                    length <= bound,
                    "{name}: cycle {cycle} longer than its bound"
                );
                bound = (2 * bound).min(restart);
            }
            assert_eq!(seen_projected, projected_exits, "{name}");
            assert_eq!(seen_bound, bound_exits, "{name}");
            // These systems are well conditioned: every projected exit is
            // confirmed, and the solve ends with one.
            assert_eq!(failed, 0, "{name}");
            assert_eq!(columns.last().unwrap().projected, projected, "{name}");
            assert!(projected <= threshold, "{name}");
            // The returned iterate is the last monitored one and its true
            // residual meets the threshold.
            assert_eq!(x, columns.last().unwrap().iterate, "{name}");
            assert!(residual <= threshold, "{name}");
            let truth = residual_norm(&a, &b, &x);
            assert!(truth <= threshold * (1.0 + 1.0e-6), "{name}: {truth:e}");
            assert!((truth - residual).abs() <= 1.0e-6 * threshold, "{name}");
            // One true residual per cycle (the start is the right-hand
            // side), one diagnostic residual, as the shipped kernel counts.
            assert_eq!(true_residuals, cycles, "{name}");
            assert_eq!(counters.linear_matvecs, total + cycles, "{name}");
            assert_eq!(counters.diagnostic_matvecs, 1, "{name}");
            assert_eq!(counters.linear_iterations, total, "{name}");
            assert_eq!(counters.linear_solves, 1, "{name}");
            assert_eq!(counters.preconditioner_apps, 0, "{name}");
        }
    }
}

#[test]
fn the_upper_bound_is_the_restart_length_or_doubles_from_ten() {
    // Every cycle but the last one of a solve runs to its upper bound here
    // (no projected exit fails its true test on this system).
    let expect = |lengths: &[usize], first: usize, restart: usize| {
        let mut bound = first.min(restart);
        for (cycle, &length) in lengths.iter().enumerate() {
            if cycle + 1 < lengths.len() {
                assert_eq!(length, bound, "cycle {cycle} of {lengths:?}");
            } else {
                assert!(length <= bound, "last cycle of {lengths:?}");
            }
            bound = (2 * bound).min(restart);
        }
    };
    let a = shifted_stencil(90, 12.0);
    let b: Vec<f64> = (0..90).map(|i| (0.3 * i as f64).sin() + 0.2).collect();
    let run = |restart: usize, first: usize| {
        let (columns, report, _, _) = monitored(
            &a,
            &b,
            None,
            &config(restart, 400, 1.0e-10, 0.0),
            first,
            ResidualAccounting::RecomputeFinal,
        );
        let (_, _, _, numbers) = report.unwrap();
        assert_eq!(numbers[3], 0, "no failed confirmation");
        cycle_lengths(&columns)
    };

    // Every cycle at the configured restart length.
    let fixed = run(40, 40);
    assert!(fixed.len() >= 3, "{fixed:?}");
    expect(&fixed, 40, 40);

    // Ten, doubled after every cycle that is followed by another one.
    let doubling = run(40, 10);
    assert!(doubling.len() >= 5, "{doubling:?}");
    assert_eq!(&doubling[..4], &[10, 20, 40, 40], "{doubling:?}");
    expect(&doubling, 10, 40);

    // The bound never exceeds the configured restart length.
    let capped = run(15, 10);
    assert_eq!(&capped[..3], &[10, 15, 15], "{capped:?}");
    expect(&capped, 10, 15);

    // A first cycle longer than the restart length is the restart length.
    let long = run(15, 1000);
    assert_eq!(long[0], 15);
    expect(&long, 15, 15);
}

#[test]
fn the_shipped_kernel_and_the_column_tested_one_meet_the_same_threshold() {
    for (name, a, b, restart, rtol) in systems() {
        let cfg = config(restart, 200, rtol, 1.0e-300);
        let op = DenseOperator::new(a.clone()).unwrap();
        let pc = IdentityPreconditioner::new(b.len());
        let mut shipped = vec![f64::NAN; b.len()];
        let mut shipped_counters = WorkCounters::default();
        let shipped_report = solve_gmres_into_with_accounting(
            &op,
            &pc,
            &b,
            None,
            &cfg,
            None,
            &mut shipped,
            &mut GmresWorkspace::default(),
            GmresCapacity::unbounded(),
            GmresIntoOptions::default(),
            ResidualAccounting::RecomputeFinal,
            &mut shipped_counters,
        )
        .unwrap();
        let (_, report, x, counters) = monitored(
            &a,
            &b,
            None,
            &cfg,
            restart,
            ResidualAccounting::RecomputeFinal,
        );
        let (threshold, residual, _, numbers) = report.unwrap();
        // The same threshold, bit for bit, and both residuals below it.
        assert_eq!(threshold, cfg.atol.max(rtol * safe_l2(&b)), "{name}");
        assert!(shipped_report.residual_norm <= threshold, "{name}");
        assert!(residual <= threshold, "{name}");
        // Never more columns than the shipped kernel, which runs every
        // cycle to its end.
        assert!(numbers[0] <= shipped_report.iterations, "{name}");
        assert!(
            counters.orthogonalization_inner_products
                <= shipped_counters.orthogonalization_inner_products,
            "{name}"
        );
        // Two iterates with residuals below the threshold differ by at most
        // the two residuals mapped through the inverse: a loose bound.
        let difference: Vec<f64> = x.iter().zip(&shipped).map(|(p, q)| p - q).collect();
        assert!(
            residual_norm(&a, &vec![0.0; b.len()], &difference) <= 2.0 * threshold,
            "{name}"
        );
    }
}

#[test]
fn a_start_is_used_and_a_converged_start_needs_no_cycle() {
    let a = scattered(60);
    let b = wave(60);
    let cfg = config(40, 200, 1.0e-11, 0.0);
    let start: Vec<f64> = (0..60).map(|i| 0.01 * ((i % 5) as f64 - 2.0)).collect();
    let (columns, report, x, counters) = monitored(
        &a,
        &b,
        Some(&start),
        &cfg,
        40,
        ResidualAccounting::RecomputeFinal,
    );
    let (threshold, residual, _, numbers) = report.unwrap();
    assert!(residual <= threshold && residual_norm(&a, &b, &x) <= threshold * 1.000001);
    // A nonzero start costs one true residual before the first cycle.
    assert_eq!(numbers[6], numbers[1] + 1);
    assert_eq!(counters.linear_matvecs, numbers[0] + numbers[1] + 1);
    assert!(!columns.is_empty());

    // The solution as a start: no cycle, the start is returned bit for bit.
    let (columns, report, again, counters) = monitored(
        &a,
        &b,
        Some(&x),
        &cfg,
        40,
        ResidualAccounting::RecomputeFinal,
    );
    let (_, _, projected, numbers) = report.unwrap();
    assert!(columns.is_empty() && projected.is_nan());
    assert_eq!(numbers[..2], [0, 0]);
    assert_eq!(again, x);
    assert_eq!(
        (counters.linear_matvecs, counters.diagnostic_matvecs),
        (1, 1)
    );

    // A zero right-hand side: the zero iterate, no Krylov application.
    let zero = vec![0.0; 60];
    let (columns, report, x, counters) = monitored(
        &a,
        &zero,
        None,
        &cfg,
        40,
        ResidualAccounting::RecomputeFinal,
    );
    report.unwrap();
    assert!(columns.is_empty());
    assert_eq!(x, zero);
    assert_eq!((counters.linear_matvecs, counters.linear_solves), (0, 1));
}

#[test]
fn the_accounting_changes_only_the_duplicate_final_residual() {
    for (name, a, b, restart, rtol) in systems() {
        let cfg = config(restart, 200, rtol, 0.0);
        let (_, recompute, x1, c1) = monitored(
            &a,
            &b,
            None,
            &cfg,
            restart,
            ResidualAccounting::RecomputeFinal,
        );
        let (_, reuse, x2, c2) = monitored(
            &a,
            &b,
            None,
            &cfg,
            restart,
            ResidualAccounting::ReuseConfirmed,
        );
        assert_eq!(x1, x2, "{name}");
        let (recompute, reuse) = (recompute.unwrap(), reuse.unwrap());
        assert_eq!(recompute.3, reuse.3, "{name}");
        assert_eq!(recompute.1.to_bits(), reuse.1.to_bits(), "{name}");
        assert_eq!(
            (c1.diagnostic_matvecs, c2.diagnostic_matvecs),
            (1, 0),
            "{name}"
        );
        assert_eq!(c1.linear_matvecs, c2.linear_matvecs, "{name}");
    }
}

/// `A x` with one product, the `spoiled`-th, perturbed: the Arnoldi relation
/// of that cycle no longer holds, so its projected residual is not the true
/// one.
struct SpoiledOnce {
    a: DenseMatrix,
    calls: AtomicUsize,
    spoiled: usize,
    size: f64,
}

impl LinearOperator for SpoiledOnce {
    fn dimension(&self) -> usize {
        self.a.nrows()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.a.matvec_into(x, y)?;
        if self.calls.fetch_add(1, Ordering::Relaxed) == self.spoiled {
            y[0] += self.size;
        }
        Ok(())
    }
    fn token(&self) -> u64 {
        u64::MAX
    }
}

#[test]
fn a_failed_true_test_continues_the_solve() {
    let a = tridiagonal(40, -1.0, 10.0, -2.0);
    let b: Vec<f64> = (0..40).map(|i| 1.0 + 0.1 * i as f64).collect();
    let op = SpoiledOnce {
        a: a.clone(),
        calls: AtomicUsize::new(0),
        spoiled: 3,
        size: 1.0e-5,
    };
    let cfg = config(40, 200, 1.0e-10, 0.0);
    let mut x = vec![f64::NAN; 40];
    let mut counters = WorkCounters::default();
    let report = StagedGmresWorkspace::default()
        .solve_column_tested_into(
            &op,
            &b,
            None,
            &cfg,
            40,
            ResidualAccounting::RecomputeFinal,
            &mut x,
            &mut counters,
            None,
        )
        .unwrap();
    // The first cycle ended on its projected residual, the true residual of
    // its iterate missed the threshold, and a second cycle finished.
    assert_eq!(report.failed_confirmations, 1, "{report:?}");
    assert_eq!(report.cycles, 2, "{report:?}");
    assert_eq!(report.projected_exits, 2, "{report:?}");
    assert!(report.residual_norm <= report.threshold);
    assert!(residual_norm(&a, &b, &x) <= report.threshold * 1.000001);
    assert_eq!(counters.linear_solves, 1);
}

/// `A x` plus an error of size `size` whose direction changes with every
/// product: no iterate has a true residual much below `size`, while the
/// projected residual of a cycle keeps falling.
struct Unreliable {
    a: DenseMatrix,
    calls: AtomicUsize,
    size: f64,
}

impl LinearOperator for Unreliable {
    fn dimension(&self) -> usize {
        self.a.nrows()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        self.a.matvec_into(x, y)?;
        let call = self.calls.fetch_add(1, Ordering::Relaxed);
        let n = y.len();
        y[call % n] += self.size;
        y[(7 * call + 3) % n] -= 0.5 * self.size;
        Ok(())
    }
    fn token(&self) -> u64 {
        u64::MAX
    }
}

#[test]
fn failures_are_the_errors_of_the_shipped_kernel_and_leave_the_output_untouched() {
    // An unreachable threshold: every projected exit fails its true test
    // until the budget is used up.
    let b = vec![1.0; 60];
    let cfg = config(40, 200, 1.0e-12, 0.0);
    for first_cycle in [40, 10] {
        let op = Unreliable {
            a: tridiagonal(60, -1.0, 10.0, -2.0),
            calls: AtomicUsize::new(0),
            size: 1.0e-6,
        };
        let mut x = vec![f64::NAN; 60];
        let mut counters = WorkCounters::default();
        let error = StagedGmresWorkspace::default()
            .solve_column_tested_into(
                &op,
                &b,
                None,
                &cfg,
                first_cycle,
                ResidualAccounting::RecomputeFinal,
                &mut x,
                &mut counters,
                None,
            )
            .unwrap_err();
        assert!(
            matches!(&error, CoreError::LinearSolve(text) if text == "GMRES exhausted 200 Arnoldi vectors"),
            "{error}"
        );
        assert!(x.iter().all(|value| value.is_nan()));
        assert_eq!(counters.linear_solves, 0);
        assert_eq!(counters.linear_iterations, 200);
        assert_eq!(counters.diagnostic_matvecs, 0);
    }

    // A system too stiff for the budget: the same error text as the shipped
    // kernel on the same system.
    let a = shifted_stencil(90, 60.0);
    let b: Vec<f64> = (0..90).map(|i| (0.3 * i as f64).sin() + 0.2).collect();
    let cfg = config(40, 200, 1.0e-10, 2.0e-15);
    let dense = DenseOperator::new(a).unwrap();
    let shipped = solve_gmres_into_with_accounting(
        &dense,
        &IdentityPreconditioner::new(90),
        &b,
        None,
        &cfg,
        None,
        &mut vec![f64::NAN; 90],
        &mut GmresWorkspace::default(),
        GmresCapacity::unbounded(),
        GmresIntoOptions::default(),
        ResidualAccounting::RecomputeFinal,
        &mut WorkCounters::default(),
    )
    .unwrap_err();
    for first_cycle in [40, 10] {
        let mut x = vec![f64::NAN; 90];
        let error = StagedGmresWorkspace::default()
            .solve_column_tested_into(
                &dense,
                &b,
                None,
                &cfg,
                first_cycle,
                ResidualAccounting::RecomputeFinal,
                &mut x,
                &mut WorkCounters::default(),
                None,
            )
            .unwrap_err();
        assert_eq!(error.to_string(), shipped.to_string());
        assert!(x.iter().all(|value| value.is_nan()));
    }

    // Refused inputs.
    let small = DenseOperator::new(tridiagonal(4, 1.0, 3.0, 1.0)).unwrap();
    let mut workspace = StagedGmresWorkspace::default();
    let mut out = vec![0.0; 4];
    let mut counters = WorkCounters::default();
    let cfg = config(4, 8, 1.0e-10, 0.0);
    let mut call = |rhs: &[f64], x0: Option<&[f64]>, first: usize, out: &mut [f64]| {
        workspace
            .solve_column_tested_into(
                &small,
                rhs,
                x0,
                &cfg,
                first,
                ResidualAccounting::RecomputeFinal,
                out,
                &mut counters,
                None,
            )
            .map(|_| ())
    };
    assert!(matches!(
        call(&[1.0; 3], None, 4, &mut out),
        Err(CoreError::Dimension(_))
    ));
    assert!(matches!(
        call(&[1.0; 4], Some(&[0.0; 3]), 4, &mut out),
        Err(CoreError::Dimension(_))
    ));
    assert!(matches!(
        call(&[1.0; 4], None, 4, &mut [0.0; 3]),
        Err(CoreError::Dimension(_))
    ));
    assert!(matches!(
        call(&[1.0, f64::NAN, 1.0, 1.0], None, 4, &mut out),
        Err(CoreError::NonFinite(_))
    ));
    assert!(matches!(
        call(&[1.0; 4], None, 0, &mut out),
        Err(CoreError::InvalidInput(_))
    ));
    assert_eq!(counters, WorkCounters::default());
}

#[test]
fn the_workspace_does_not_grow_after_the_first_solve() {
    let a = shifted_stencil(90, 4.0);
    let b: Vec<f64> = (0..90).map(|i| (0.3 * i as f64).sin() + 0.2).collect();
    let op = DenseOperator::new(a).unwrap();
    let cfg = config(40, 200, 1.0e-10, 0.0);
    let mut workspace = StagedGmresWorkspace::default();
    let mut first = vec![0.0; 90];
    let solve = |workspace: &mut StagedGmresWorkspace, out: &mut [f64], first_cycle| {
        workspace
            .solve_column_tested_into(
                &op,
                &b,
                None,
                &cfg,
                first_cycle,
                ResidualAccounting::RecomputeFinal,
                out,
                &mut WorkCounters::default(),
                None,
            )
            .unwrap()
    };
    let report = solve(&mut workspace, &mut first, 40);
    let bytes = workspace.retained_bytes();
    for first_cycle in [40, 10, 40] {
        let mut again = vec![0.0; 90];
        let repeated = solve(&mut workspace, &mut again, first_cycle);
        assert_eq!(workspace.retained_bytes(), bytes);
        if first_cycle == 40 {
            // A reused workspace returns the bits of a fresh one.
            assert_eq!(again, first);
            assert_eq!(repeated, report);
        }
    }
}
