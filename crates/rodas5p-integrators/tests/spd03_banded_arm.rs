//! Speed research node SPD03 (`research/spd03_banded_arm_instructions_20261005`):
//! the slices kernel of the banded pipeline against the indexed kernel on
//! the six INT-03 cases with their 41-point output grid (outputs and
//! `BandedWork` bitwise). Identity only; no timing.

#[path = "rnext_common/mod.rs"]
mod common;

use std::sync::Arc;

use common::{brusselator, write_output};
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BandedKernel, OdeProblem, OutputSchedule,
    Rodas5pFastResult, integrate_rodas5p_fast_banded_observed,
    integrate_rodas5p_fast_banded_observed_with_kernel,
};
use serde_json::{Value, json};

/// The Jacobian's nonzero entries of row `i` at `y`, as `(column, value)`
/// (the INT-03 formulation).
type Entries = Arc<dyn Fn(&[f64], usize, &mut Vec<(usize, f64)>) + Send + Sync>;

fn brusselator_entries(cells: usize) -> Entries {
    let c = (cells as f64 + 1.0).powi(2) / 50.0;
    Arc::new(move |y: &[f64], row: usize, out: &mut Vec<(usize, f64)>| {
        out.clear();
        let i = row / 2;
        let (u, v) = (y[2 * i], y[2 * i + 1]);
        let (a, b) = (2 * i, 2 * i + 1);
        if row == a {
            if i > 0 {
                out.push((a - 2, c));
            }
            out.push((a, 2.0 * u * v - 4.0 - 2.0 * c));
            out.push((b, u * u));
            if i + 1 < cells {
                out.push((a + 2, c));
            }
        } else {
            if i > 0 {
                out.push((b - 2, c));
            }
            out.push((a, 3.0 - 2.0 * u * v));
            out.push((b, -u * u - 2.0 * c));
            if i + 1 < cells {
                out.push((b + 2, c));
            }
        }
    })
}

const NU: f64 = 1.0e-3;

fn burgers(n: usize) -> (OdeProblem, Vec<f64>, Entries) {
    let dx = 1.0 / (n as f64 + 1.0);
    let boundary = 0.5;
    let rhs = Arc::new(move |_t: f64, y: &[f64], out: &mut [f64]| {
        for i in 0..n {
            let left = if i > 0 { y[i - 1] } else { boundary };
            let right = if i + 1 < n { y[i + 1] } else { boundary };
            out[i] =
                -y[i] * (right - left) / (2.0 * dx) + NU * (left - 2.0 * y[i] + right) / (dx * dx);
        }
        Ok(())
    });
    let entries: Entries = Arc::new(move |y: &[f64], i: usize, out: &mut Vec<(usize, f64)>| {
        out.clear();
        let left = if i > 0 { y[i - 1] } else { boundary };
        let right = if i + 1 < n { y[i + 1] } else { boundary };
        if i > 0 {
            out.push((i - 1, y[i] / (2.0 * dx) + NU / (dx * dx)));
        }
        out.push((i, -(right - left) / (2.0 * dx) - 2.0 * NU / (dx * dx)));
        if i + 1 < n {
            out.push((i + 1, -y[i] / (2.0 * dx) + NU / (dx * dx)));
        }
    });
    let dense_entries = entries.clone();
    let jacobian = Arc::new(move |_t: f64, y: &[f64]| {
        let mut j = DenseMatrix::zeros(n, n);
        let mut row = Vec::new();
        for i in 0..n {
            dense_entries(y, i, &mut row);
            for &(c, v) in &row {
                j[(i, c)] = v;
            }
        }
        Ok(j)
    });
    let problem = OdeProblem::new(
        format!("burgers-central-{n}"),
        n,
        rhs,
        None,
        Some(jacobian),
        None,
        None,
        true,
        None,
        None,
    )
    .unwrap();
    let y0 = (0..n)
        .map(|i| (std::f64::consts::PI * (i + 1) as f64 * dx).sin() + 0.5)
        .collect();
    (problem, y0, entries)
}

fn attach(
    problem: OdeProblem,
    entries: &Entries,
    l: usize,
    u: usize,
) -> (OdeProblem, BandedJacobian) {
    let dense = entries.clone();
    let problem =
        problem.with_jacobian_into(Arc::new(move |_t, y: &[f64], out: &mut DenseMatrix| {
            let mut row = Vec::new();
            for i in 0..y.len() {
                dense(y, i, &mut row);
                for &(c, v) in &row {
                    out[(i, c)] = v;
                }
            }
            Ok(())
        }));
    let banded = entries.clone();
    let width = l + u + 1;
    let band = BandedJacobian {
        lower: l,
        upper: u,
        fill: Arc::new(move |_t, y: &[f64], band: &mut [f64]| {
            let mut row = Vec::new();
            for i in 0..y.len() {
                banded(y, i, &mut row);
                for &(c, v) in &row {
                    band[i * width + (c + l - i)] = v;
                }
            }
            Ok(())
        }),
    };
    (problem, band)
}

struct Case {
    label: String,
    problem: OdeProblem,
    band: BandedJacobian,
    y0: Vec<f64>,
    tf: f64,
}

fn cases() -> Vec<Case> {
    let mut out = Vec::new();
    for size in [32_usize, 128, 512] {
        let (problem, y0) = brusselator(size).unwrap();
        let (problem, band) = attach(problem, &brusselator_entries(size), 2, 2);
        out.push(Case {
            label: format!("brusselator-{size}-cells"),
            problem,
            band,
            y0,
            tf: 10.0,
        });
    }
    for size in [64_usize, 256, 1024] {
        let (problem, y0, entries) = burgers(size);
        let (problem, band) = attach(problem, &entries, 1, 1);
        out.push(Case {
            label: format!("burgers-central-{size}"),
            problem,
            band,
            y0,
            tf: 1.0,
        });
    }
    out
}

fn adaptive() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: 1.0e-6,
        rtol: 1.0e-6,
        initial_step: 1.0e-6,
        max_attempts: 100_000,
        ..AdaptiveStepConfig::default()
    }
}

fn grid(tf: f64) -> OutputSchedule {
    OutputSchedule::new((0..=40).map(|k| tf * k as f64 / 40.0).collect()).unwrap()
}

fn identical(a: &Rodas5pFastResult, b: &Rodas5pFastResult) -> bool {
    a.observed.success == b.observed.success
        && a.observed.t == b.observed.t
        && a.observed.y.len() == b.observed.y.len()
        && a.observed.y.iter().zip(&b.observed.y).all(|(p, q)| {
            p.len() == q.len() && p.iter().zip(q).all(|(x, y)| x.to_bits() == y.to_bits())
        })
        && a.observed.counters == b.observed.counters
        && a.observed.internal_steps == b.observed.internal_steps
        && a.observed.output_clipped_steps == b.observed.output_clipped_steps
        && a.attempts == b.attempts
        && a.accepted_steps == b.accepted_steps
        && a.rejected_steps == b.rejected_steps
        && a.jacobian_reuses == b.jacobian_reuses
}

/// The six INT-03 cases on the 41-point grid: the slices kernel equals the
/// indexed kernel bitwise, outputs and counted banded work.
#[test]
fn slices_kernel_equals_the_indexed_kernel_on_the_int03_cases() {
    let mut rows = Vec::new();
    for case in cases() {
        let schedule = grid(case.tf);
        let indexed = integrate_rodas5p_fast_banded_observed(
            &case.problem,
            &case.band,
            (0.0, case.tf),
            &case.y0,
            &adaptive(),
            &schedule,
        )
        .unwrap();
        let slices = integrate_rodas5p_fast_banded_observed_with_kernel(
            &case.problem,
            &case.band,
            (0.0, case.tf),
            &case.y0,
            &adaptive(),
            &schedule,
            BandedKernel::Slices,
        )
        .unwrap();
        assert!(indexed.fast.observed.success, "{}", case.label);
        let same = identical(&indexed.fast, &slices.fast) && indexed.work == slices.work;
        assert_eq!(slices.fast.driver, "rodas5p-fast-banded-v1-slices");
        let row = json!({"case": case.label, "attempts": indexed.fast.attempts,
            "slices_equal_indexed": same, "factor_operations": indexed.work.factor_operations,
            "solve_operations": indexed.work.solve_operations, "stored_slots": indexed.work.stored_slots});
        println!("{row}");
        assert!(same, "{}", case.label);
        rows.push(row);
    }
    write_output("SPD03_INT03", &json!({"int03_rows": rows}));
    let _ = Value::Null;
}
