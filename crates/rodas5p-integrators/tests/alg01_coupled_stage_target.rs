//! Research nodes `research/alg01_coupled_stage_target_20261008` (ALG01:
//! tolerance-coupled stage targets with in-cycle exit in the matrix-free
//! U-form driver) and `research/alg03_stage_budget_guard_20261008` (ALG03:
//! larger stage budget with stagnation guard and production fallback).
//!
//! `export_base` and `export_base_alg03` were run on the registration
//! commit's solver source before any source change. Each export runs only
//! when its output variable is set (the registered filters `export_base`
//! and `export_runs` also match the ALG03 exports by substring).

mod alg01_common;

use alg01_common::*;
use rodas5p_core::WorkCounters;
use rodas5p_integrators::{Rodas5pMfFastWorkspace, integrate_rodas5p_mf_fast_observed_gmres_into};
use serde_json::{Map, Value, json};

const C1_RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];
const C2_RTOLS: [f64; 3] = [1.0e-9, 1.0e-10, 1.0e-11];
const C3_RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];

/// One adaptive cell: group, problem index, rtol.
struct CellSpec {
    group: &'static str,
    problem: usize,
    rtol: f64,
}

/// The ALG01 problems and adaptive cells C1, C2, C3 and C5.
fn alg01_cells() -> (Vec<Problem>, Vec<CellSpec>) {
    let mut problems = spd07_problems();
    let mut cells = Vec::new();
    for (i, _) in problems.iter().enumerate() {
        for rtol in C1_RTOLS {
            cells.push(CellSpec {
                group: "C1",
                problem: i,
                rtol,
            });
        }
    }
    let index = |problems: &[Problem], id: &str| problems.iter().position(|p| p.id == id).unwrap();
    for id in ["hires", "robertson"] {
        let i = index(&problems, id);
        for rtol in C2_RTOLS {
            cells.push(CellSpec {
                group: "C2",
                problem: i,
                rtol,
            });
        }
    }
    let first_stress = problems.len();
    problems.extend(stress_problems());
    for i in first_stress..problems.len() {
        for rtol in C3_RTOLS {
            cells.push(CellSpec {
                group: "C3",
                problem: i,
                rtol,
            });
        }
    }
    for id in ["brusselator-1d-50", "hires"] {
        let i = index(&problems, id);
        for rtol in half_decades() {
            cells.push(CellSpec {
                group: "C5",
                problem: i,
                rtol,
            });
        }
    }
    (problems, cells)
}

/// The ALG03 problems and cells D1-D4 (D5 is the ALG01 C1 set).
fn alg03_cells() -> (Vec<Problem>, Vec<CellSpec>) {
    let problems = vec![
        brusselator_problem(160),
        brusselator_problem(300),
        robertson_long(),
        stosc(1.0e4),
        e05("e05-s10", 10.0, 0.1, 20.0),
    ];
    let specs = [
        ("D1", 0, 1.0e-4),
        ("D1", 1, 1.0e-4),
        ("D1", 1, 1.0e-6),
        ("D2", 2, 1.0e-5),
        ("D2", 2, 1.0e-7),
        ("D2", 2, 1.0e-9),
        ("D3", 3, 1.0e-4),
        ("D3", 3, 1.0e-6),
        ("D3", 3, 1.0e-8),
        ("D4", 4, 1.0e-4),
    ];
    let cells = specs
        .into_iter()
        .map(|(group, problem, rtol)| CellSpec {
            group,
            problem,
            rtol,
        })
        .collect();
    (problems, cells)
}

/// The `Legacy` stage target: the unchanged GMRES-into driver (SPD07's
/// `gmres_into` arm) with Krylov budget `budget`.
fn legacy(p: &Problem, rtol: f64, budget: usize) -> Value {
    result_json(&integrate_rodas5p_mf_fast_observed_gmres_into(
        &p.problem,
        p.t_span,
        &p.y0,
        &gmres_config(budget),
        &p.adaptive(rtol),
        &p.schedule(),
    ))
}

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = std::time::Instant::now();
    let value = f();
    println!("{label}: {:.1} s", start.elapsed().as_secs_f64());
    value
}

fn references(problems: &[Problem]) -> Map<String, Value> {
    problems
        .iter()
        .map(|p| {
            (
                p.id.clone(),
                timed(&format!("reference {}", p.id), || reference(p)),
            )
        })
        .collect()
}

fn legacy_ladder(p: &Problem, rtol: f64, k: u32, budget: usize) -> Value {
    ladder_mf(p, rtol, k, budget, &mut |_| {}, &mut |_, _| {})
}

/// ALG01 base export: `Legacy` on every cell of C1-C5, the dense twins and
/// the references.
#[test]
#[ignore = "base export of research/alg01_coupled_stage_target_20261008; release build"]
fn export_base() {
    if std::env::var("ALG01_BASE").is_err() {
        println!("ALG01_BASE is not set; export skipped");
        return;
    }
    let (problems, cells) = alg01_cells();
    let refs = references(&problems);
    let mut rows = Vec::new();
    for cell in &cells {
        let p = &problems[cell.problem];
        let label = format!("{} {} {:e}", cell.group, p.id, cell.rtol);
        rows.push(json!({
            "group": cell.group,
            "case": p.id,
            "rtol": cell.rtol,
            "dimension": p.y0.len(),
            "legacy": timed(&format!("{label} legacy"), || legacy(p, cell.rtol, BUDGET)),
            "twin": timed(&format!("{label} twin"), || twin(p, cell.rtol)),
        }));
    }
    let mut ladder_rows = Vec::new();
    let mut ladder_refs = Map::new();
    for ladder in ladders() {
        let p = &ladder.problem;
        ladder_refs.insert(
            ladder.id.to_string(),
            json!({"kind": "exact", "y": hexes(&(p.exact.as_ref().unwrap())(p.t_span.1))}),
        );
        for &rtol in &ladder.rtols {
            for &k in &ladder.rungs {
                let label = format!("C4 {} {rtol:e} k={k}", ladder.id);
                ladder_rows.push(json!({
                    "ladder": ladder.id,
                    "rtol": rtol,
                    "k": k,
                    "dimension": p.y0.len(),
                    "lu": timed(&format!("{label} lu"), || ladder_lu(p, rtol, k)),
                    "legacy": timed(&format!("{label} legacy"), || legacy_ladder(p, rtol, k, BUDGET)),
                    "legacy_pilot_budget": timed(&format!("{label} legacy 20000"), || {
                        legacy_ladder(p, rtol, k, PILOT_LADDER_BUDGET)
                    }),
                }));
            }
        }
    }
    write_output(
        "ALG01_BASE",
        &json!({
            "node": "alg01_coupled_stage_target_20261008",
            "export": "export_base",
            "budget": BUDGET,
            "pilot_ladder_budget": PILOT_LADDER_BUDGET,
            "references": refs,
            "rows": rows,
            "ladder_references": ladder_refs,
            "ladders": ladder_rows,
        }),
    );
}

/// ALG03 base export: `Legacy` with budgets 200 and 2000 on D1-D4, the
/// dense twins and the references.
#[test]
#[ignore = "base export of research/alg03_stage_budget_guard_20261008; release build"]
fn export_base_alg03() {
    if std::env::var("ALG03_BASE").is_err() {
        println!("ALG03_BASE is not set; export skipped");
        return;
    }
    let (problems, cells) = alg03_cells();
    let refs = references(&problems);
    let mut rows = Vec::new();
    for cell in &cells {
        let p = &problems[cell.problem];
        let label = format!("{} {} {:e}", cell.group, p.id, cell.rtol);
        rows.push(json!({
            "group": cell.group,
            "case": p.id,
            "rtol": cell.rtol,
            "dimension": p.y0.len(),
            "max_attempts": p.max_attempts,
            "legacy_200": timed(&format!("{label} legacy 200"), || legacy(p, cell.rtol, BUDGET)),
            "legacy_2000": timed(&format!("{label} legacy 2000"), || legacy(p, cell.rtol, BIG_BUDGET)),
            "twin": timed(&format!("{label} twin"), || twin(p, cell.rtol)),
        }));
    }
    write_output(
        "ALG03_BASE",
        &json!({
            "node": "alg03_stage_budget_guard_20261008",
            "export": "export_base_alg03",
            "references": refs,
            "rows": rows,
        }),
    );
}

/// The E-05 unit right-hand side reproduces the committed E-05 anchor, so
/// the generator and its draw order are E-05's.
#[test]
fn e05_unit_rhs_matches_the_committed_anchor() {
    let mut rng = Lcg(20_260_927);
    let v: Vec<f64> = (0..256).map(|_| rng.uniform()).collect();
    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    assert_eq!(v[0] / norm, -0.031_981_700_657_820_25);
    assert_eq!(v[1] / norm, 0.095_957_793_725_194_96);
    assert_eq!(v[2] / norm, -0.040_942_056_514_667_216);
}

/// The fixed-step direct twin is the U-form step: on one step of a small
/// problem it agrees with the matrix-free workspace solved to 1e-10.
#[test]
fn lu_twin_agrees_with_the_matrix_free_u_form_step() {
    for p in [spd07_problem("quadratic-4"), diag_pr(8, 1.0e3)] {
        let (h, atol, rtol) = (1.0e-2, 1.0e-8, 1.0e-6);
        let (y_lu, err_lu) = lu_u_form_step(&p.full, p.t_span.0, &p.y0, h, atol, rtol).unwrap();
        let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &gmres_config(BUDGET)).unwrap();
        work.set_gmres_into(true);
        let mut counters = WorkCounters::default();
        let err = work
            .attempt(
                &p.problem,
                p.t_span.0,
                &p.y0,
                h,
                true,
                None,
                atol,
                rtol,
                &mut counters,
            )
            .unwrap();
        assert!(relative_max_norm(work.y_new(), &y_lu) < 1.0e-9, "{}", p.id);
        assert!(
            (err - err_lu).abs() <= 1.0e-6 * err_lu.max(1.0e-3),
            "{}",
            p.id
        );
    }
}

#[test]
fn frontier_grid_is_one_to_ten_decades_in_half_decades() {
    let grid = half_decades();
    assert_eq!(grid.len(), 15);
    assert_eq!(grid[0], 1.0e-3);
    assert_eq!(grid[14], 1.0e-10);
    assert!(
        grid.windows(2)
            .all(|w| (w[0] / w[1] - 10f64.sqrt()).abs() < 1.0e-12)
    );
}
