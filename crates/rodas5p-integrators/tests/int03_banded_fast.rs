//! The banded fast RODAS5P pipeline against v2 (research node
//! `research/int03_banded_fast_20261003`, integrated DAG node INT-03):
//! Brusselator 1-D (interleaved, l = u = 2) and viscous Burgers with central
//! differences (l = u = 1). Counter-only; no timing.

#[path = "rnext_common/mod.rs"]
mod common;

use std::sync::Arc;

use common::{brusselator, write_output};
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, OdeProblem, OutputSchedule,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_observed,
};
use serde_json::{Value, json};

/// The Jacobian's nonzero entries of row `i` at `y`, as `(column, value)`.
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
    let boundary = 0.5; // sin(0) + 0.5 = sin(pi) + 0.5
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

/// v2's in-place dense Jacobian and the band provider, from one formula.
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

fn relative_difference(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    let mut worst = 0.0_f64;
    for (x, y) in a.iter().zip(b) {
        let scale = y
            .iter()
            .fold(0.0_f64, |m, v| m.max(v.abs()))
            .max(f64::MIN_POSITIVE);
        let diff = x
            .iter()
            .zip(y)
            .fold(0.0_f64, |m, (p, q)| m.max((p - q).abs()));
        worst = worst.max(diff / scale);
    }
    worst
}

/// Least-squares slope of log y against log x.
fn slope(points: &[(f64, f64)]) -> f64 {
    let m = points.len() as f64;
    let (mx, my) = points.iter().fold((0.0, 0.0), |(a, b), (x, y)| {
        (a + x.ln() / m, b + y.ln() / m)
    });
    let (num, den) = points.iter().fold((0.0, 0.0), |(n, d), (x, y)| {
        (n + (x.ln() - mx) * (y.ln() - my), d + (x.ln() - mx).powi(2))
    });
    num / den
}

struct Case {
    label: String,
    n: usize,
    l: usize,
    u: usize,
    problem: OdeProblem,
    band: BandedJacobian,
    y0: Vec<f64>,
    tf: f64,
}

fn run_case(case: &Case, gate_parity: &mut bool, gate_slots: &mut bool) -> (Value, f64) {
    let adaptive = AdaptiveStepConfig {
        atol: 1.0e-6,
        rtol: 1.0e-6,
        initial_step: 1.0e-6,
        max_attempts: 100_000,
        ..AdaptiveStepConfig::default()
    };
    let times: Vec<f64> = (0..=40).map(|k| case.tf * k as f64 / 40.0).collect();
    let schedule = OutputSchedule::new(times).unwrap();
    let banded = integrate_rodas5p_fast_banded_observed(
        &case.problem,
        &case.band,
        (0.0, case.tf),
        &case.y0,
        &adaptive,
        &schedule,
    )
    .unwrap();
    let ops = banded.work.factor_operations + banded.work.solve_operations;
    let per_attempt = ops as f64 / banded.fast.attempts as f64;
    let slots_expected = case.n * (3 * case.l + 2 * case.u + 2);
    *gate_slots &= banded.work.stored_slots == slots_expected;
    let mut record = json!({
        "case": case.label, "n": case.n, "lower": case.l, "upper": case.u,
        "banded": {
            "success": banded.fast.observed.success, "attempts": banded.fast.attempts,
            "accepted": banded.fast.accepted_steps, "rejected": banded.fast.rejected_steps,
            "jacobian_reuses": banded.fast.jacobian_reuses,
            "factor_operations": banded.work.factor_operations,
            "solve_operations": banded.work.solve_operations,
            "operations_per_attempt": per_attempt,
            "stored_slots": banded.work.stored_slots, "stored_slots_expected": slots_expected,
        },
    });
    if case.n <= 1024 {
        let v2 = integrate_rodas5p_fast_observed(
            &case.problem,
            (0.0, case.tf),
            &case.y0,
            &adaptive,
            &schedule,
        )
        .unwrap();
        let difference = relative_difference(&banded.fast.observed.y, &v2.observed.y);
        let bitwise = banded.fast.observed.y.len() == v2.observed.y.len()
            && banded
                .fast
                .observed
                .y
                .iter()
                .zip(&v2.observed.y)
                .all(|(a, b)| a.iter().zip(b).all(|(p, q)| p.to_bits() == q.to_bits()));
        let same = v2.attempts == banded.fast.attempts
            && v2.accepted_steps == banded.fast.accepted_steps
            && v2.rejected_steps == banded.fast.rejected_steps
            && v2.jacobian_reuses == banded.fast.jacobian_reuses
            && v2.observed.success == banded.fast.observed.success
            && v2.observed.t == banded.fast.observed.t
            && v2.observed.counters == banded.fast.observed.counters
            && difference <= 1.0e-13;
        *gate_parity &= same;
        record["v2"] = json!({
            "success": v2.observed.success, "attempts": v2.attempts,
            "accepted": v2.accepted_steps, "rejected": v2.rejected_steps,
            "jacobian_reuses": v2.jacobian_reuses, "lu": format!("{:?}", v2.lu),
            "dense_slots": 2 * case.n * case.n,
            "dense_assembly_and_scan_entries_per_factorization": 2 * case.n * case.n,
        });
        record["parity"] = json!({
            "same": same, "bitwise_outputs": bitwise, "max_relative_output_difference": difference,
            "counters_equal": v2.observed.counters == banded.fast.observed.counters,
        });
    }
    println!("{record}");
    (record, per_attempt)
}

#[test]
#[ignore = "research run of research/int03_banded_fast_20261003; release build"]
fn banded_fast_pipeline() {
    let mut gate_parity = true;
    let mut gate_slots = true;
    let mut records = Vec::new();
    let mut slopes = serde_json::Map::new();
    let mut slope_ok = true;
    for family in ["brusselator", "burgers"] {
        let mut points = Vec::new();
        let sizes: [usize; 4] = if family == "brusselator" {
            [32, 128, 512, 2048]
        } else {
            [64, 256, 1024, 4096]
        };
        for size in sizes {
            let case = if family == "brusselator" {
                let (problem, y0) = brusselator(size).unwrap();
                let (problem, band) = attach(problem, &brusselator_entries(size), 2, 2);
                Case {
                    label: format!("brusselator-{size}-cells"),
                    n: 2 * size,
                    l: 2,
                    u: 2,
                    problem,
                    band,
                    y0,
                    tf: 10.0,
                }
            } else {
                let (problem, y0, entries) = burgers(size);
                let (problem, band) = attach(problem, &entries, 1, 1);
                Case {
                    label: format!("burgers-central-{size}"),
                    n: size,
                    l: 1,
                    u: 1,
                    problem,
                    band,
                    y0,
                    tf: 1.0,
                }
            };
            let (record, per_attempt) = run_case(&case, &mut gate_parity, &mut gate_slots);
            points.push((case.n as f64, per_attempt));
            records.push(record);
        }
        let s = slope(&points);
        slope_ok &= (0.95..=1.05).contains(&s);
        slopes.insert(family.into(), json!(s));
    }
    let gate = json!({
        "1_parity_with_v2": gate_parity,
        "2_no_quadratic_floor": slope_ok && gate_slots,
        "3_pivoting": "contract tests int03_banded_fast_contracts",
        "4_contracts": "contract tests int03_banded_fast_contracts",
        "operation_slopes": slopes,
        "stored_slots_as_expected": gate_slots,
    });
    let report = json!({
        "schema": "vigilode-int03-banded-fast-v1",
        "node": "research/int03_banded_fast_20261003",
        "cases": records,
        "gate": gate,
        "timing": "not run; timing authority on HOLD",
    });
    write_output("INT03_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
}
