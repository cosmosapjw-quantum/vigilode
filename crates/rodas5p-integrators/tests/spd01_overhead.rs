//! Speed research node SPD01 (`research/spd01_fast_driver_overhead_20261005`):
//! the banded fast driver with the fused landing and the prevalidated
//! controller against the legacy banded driver on the six INT-03 cases with
//! their 41-point output grid (interior due times), and the library
//! problems of the small driver. Identity only; no timing.

#[path = "rnext_common/mod.rs"]
mod common;

use std::sync::Arc;

use common::{brusselator, write_output};
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, FastLuPolicy, OdeProblem, OutputSchedule,
    Rodas5pFastOptions, Rodas5pFastResult, integrate_rodas5p_fast_banded_observed,
    integrate_rodas5p_fast_banded_observed_with_options, integrate_rodas5p_fast_observed,
    integrate_rodas5p_fast_observed_with_options,
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

const SETS: [(&str, Rodas5pFastOptions); 3] = [
    (
        "val",
        Rodas5pFastOptions {
            prevalidated_controller: true,
            fused_landing: false,
            lu_policy: FastLuPolicy::Legacy,
        },
    ),
    (
        "land",
        Rodas5pFastOptions {
            prevalidated_controller: false,
            fused_landing: true,
            lu_policy: FastLuPolicy::Legacy,
        },
    ),
    (
        "ovh",
        Rodas5pFastOptions {
            prevalidated_controller: true,
            fused_landing: true,
            lu_policy: FastLuPolicy::Legacy,
        },
    ),
];

/// The six INT-03 cases on the 41-point grid: every option set equals the
/// legacy banded driver (and the legacy dense driver) bitwise.
#[test]
fn banded_option_sets_equal_the_legacy_driver_on_the_int03_grid() {
    let mut rows = Vec::new();
    for case in cases() {
        let schedule = grid(case.tf);
        let legacy = integrate_rodas5p_fast_banded_observed(
            &case.problem,
            &case.band,
            (0.0, case.tf),
            &case.y0,
            &adaptive(),
            &schedule,
        )
        .unwrap();
        assert!(legacy.fast.observed.success, "{}", case.label);
        let mut row = json!({"case": case.label, "attempts": legacy.fast.attempts,
            "output_clipped_steps": legacy.fast.observed.output_clipped_steps});
        for (name, options) in SETS {
            let run = integrate_rodas5p_fast_banded_observed_with_options(
                &case.problem,
                &case.band,
                (0.0, case.tf),
                &case.y0,
                &adaptive(),
                &schedule,
                options,
            )
            .unwrap();
            let same = identical(&legacy.fast, &run.fast) && legacy.work == run.work;
            row[name] = json!({"identical": same, "driver": run.fast.driver});
            assert!(same, "{} {name}", case.label);
        }
        if case.problem.dimension <= 512 {
            let dense = integrate_rodas5p_fast_observed_with_options(
                &case.problem,
                (0.0, case.tf),
                &case.y0,
                &adaptive(),
                &schedule,
                SETS[2].1,
            )
            .unwrap();
            let base = integrate_rodas5p_fast_observed(
                &case.problem,
                (0.0, case.tf),
                &case.y0,
                &adaptive(),
                &schedule,
            )
            .unwrap();
            let same = identical(&base, &dense) && identical(&base, &legacy.fast);
            row["dense_ovh_identical"] = json!(same);
            assert!(same, "{} dense", case.label);
        }
        println!("{row}");
        rows.push(row);
    }
    write_output("SPD01_BANDED", &json!({"rows": rows}));
}

/// The options never change a driver's refusal of invalid input.
#[test]
fn option_sets_refuse_what_the_legacy_driver_refuses() {
    let case = cases().remove(0);
    let schedule = grid(case.tf);
    let mut bad = adaptive();
    bad.min_step = -1.0;
    for (_, options) in SETS {
        assert!(
            integrate_rodas5p_fast_banded_observed_with_options(
                &case.problem,
                &case.band,
                (0.0, case.tf),
                &case.y0,
                &bad,
                &schedule,
                options,
            )
            .is_err()
        );
    }
    assert!(
        integrate_rodas5p_fast_banded_observed(
            &case.problem,
            &case.band,
            (0.0, case.tf),
            &case.y0,
            &bad,
            &schedule,
        )
        .is_err()
    );
    let _ = Value::Null;
}
