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
use rodas5p_integrators::{
    Rodas5pMfFastWorkspace, StageTargetOptions, StageTargetPolicy,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_stage_target, rodas5p_stage_transfer_constants,
};
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

// ------------------------------------------------------------------ RUNS

/// The ALG01 arms: name and stage-target options (budget 200).
fn alg01_arms() -> Vec<(&'static str, StageTargetOptions)> {
    [
        ("legacy", StageTargetPolicy::Legacy),
        ("proj_l2", StageTargetPolicy::ProjL2),
        ("l2_coupled", StageTargetPolicy::L2Coupled),
        ("coupled", StageTargetPolicy::Coupled),
        ("coupled_guarded", StageTargetPolicy::CoupledGuarded),
    ]
    .into_iter()
    .map(|(name, policy)| (name, StageTargetOptions::new(policy)))
    .collect()
}

/// The ALG03 arms: name, options and Krylov budget. All but `rbig` use the
/// `CoupledGuarded` stage target; guard aborts of the guarded arms are
/// classified by an uncounted shadow continuation.
fn alg03_arms() -> Vec<(&'static str, StageTargetOptions, usize)> {
    let guarded = StageTargetOptions::new(StageTargetPolicy::CoupledGuarded);
    let with_guard = StageTargetOptions {
        stagnation_guard: true,
        classify_guard_aborts: true,
        ..guarded
    };
    vec![
        ("b0", guarded, BUDGET),
        ("b1", guarded, BIG_BUDGET),
        ("b2", with_guard, BIG_BUDGET),
        (
            "b2nf",
            StageTargetOptions {
                production_fallback: false,
                ..with_guard
            },
            BIG_BUDGET,
        ),
        (
            "rbig",
            StageTargetOptions::new(StageTargetPolicy::Legacy),
            BIG_BUDGET,
        ),
    ]
}

/// One run of the stage-target entry point: the SPD07 record, plus the
/// staged solves' statistics for every policy but `Legacy` (whose record
/// must equal the base export's).
fn staged(p: &Problem, rtol: f64, options: StageTargetOptions, budget: usize) -> Value {
    let run = integrate_rodas5p_mf_fast_observed_with_stage_target(
        &p.problem,
        p.t_span,
        &p.y0,
        &gmres_config(budget),
        &p.adaptive(rtol),
        &p.schedule(),
        options,
    );
    let (result, statistics) = match run {
        Ok(r) => (Ok(r.result), Some(r.statistics)),
        Err(e) => (Err(e), None),
    };
    let mut record = result_json(&result);
    if options.policy != StageTargetPolicy::Legacy
        && let Some(statistics) = statistics
    {
        record["stage_statistics"] = serde_json::to_value(statistics).unwrap();
    }
    record
}

/// A rung with a stage-target policy (span 1, `e_hat` = the previous
/// step's embedded error, every step accepted).
fn staged_ladder(
    p: &Problem,
    rtol: f64,
    k: u32,
    budget: usize,
    options: StageTargetOptions,
) -> Value {
    if options.policy == StageTargetPolicy::Legacy {
        return legacy_ladder(p, rtol, k, budget);
    }
    let span = p.span();
    ladder_mf_finish(
        p,
        rtol,
        k,
        budget,
        &mut |work| {
            work.set_stage_target(options).unwrap();
            work.set_stage_target_span(span);
            work.clear_accepted_error();
        },
        &mut |work, err| work.record_accepted_error(err),
        &mut |work, record| {
            record["stage_statistics"] = serde_json::to_value(work.stage_statistics()).unwrap();
        },
    )
}

fn arms_json(p: &Problem, rtol: f64, label: &str) -> Value {
    let mut arms = Map::new();
    for (name, options) in alg01_arms() {
        arms.insert(
            name.into(),
            timed(&format!("{label} {name}"), || {
                staged(p, rtol, options, BUDGET)
            }),
        );
    }
    Value::Object(arms)
}

/// ALG01 recorded run: every arm on every cell of C1-C5 (`Legacy`
/// included, for the identity gate), the dense twins again, and the
/// reported finite-difference Brusselator-50 variant.
#[test]
#[ignore = "recorded run of research/alg01_coupled_stage_target_20261008; release build"]
fn export_runs() {
    if std::env::var("ALG01_RUNS").is_err() {
        println!("ALG01_RUNS is not set; export skipped");
        return;
    }
    let (problems, cells) = alg01_cells();
    let mut rows = Vec::new();
    for cell in &cells {
        let p = &problems[cell.problem];
        let label = format!("{} {} {:e}", cell.group, p.id, cell.rtol);
        rows.push(json!({
            "group": cell.group,
            "case": p.id,
            "rtol": cell.rtol,
            "dimension": p.y0.len(),
            "arms": arms_json(p, cell.rtol, &label),
            "twin": timed(&format!("{label} twin"), || twin(p, cell.rtol)),
        }));
    }
    let mut ladder_rows = Vec::new();
    for ladder in ladders() {
        let p = &ladder.problem;
        for &rtol in &ladder.rtols {
            for &k in &ladder.rungs {
                let label = format!("C4 {} {rtol:e} k={k}", ladder.id);
                let mut arms = Map::new();
                let mut pilot = Map::new();
                for (name, options) in alg01_arms() {
                    arms.insert(
                        name.into(),
                        timed(&format!("{label} {name}"), || {
                            staged_ladder(p, rtol, k, BUDGET, options)
                        }),
                    );
                    pilot.insert(
                        name.into(),
                        timed(&format!("{label} {name} 20000"), || {
                            staged_ladder(p, rtol, k, PILOT_LADDER_BUDGET, options)
                        }),
                    );
                }
                ladder_rows.push(json!({
                    "ladder": ladder.id,
                    "rtol": rtol,
                    "k": k,
                    "dimension": p.y0.len(),
                    "lu": ladder_lu(p, rtol, k),
                    "arms": arms,
                    "arms_pilot_budget": pilot,
                }));
            }
        }
    }
    let fd = brusselator_fd_problem(50);
    let mut fd_rows = Vec::new();
    for rtol in C1_RTOLS {
        let mut arms = Map::new();
        for (name, options) in alg01_arms()
            .into_iter()
            .filter(|(name, _)| matches!(*name, "legacy" | "coupled_guarded"))
        {
            arms.insert(
                name.into(),
                timed(&format!("FD bruss-50 {rtol:e} {name}"), || {
                    staged(&fd, rtol, options, BUDGET)
                }),
            );
        }
        fd_rows.push(json!({
            "case": fd.id,
            "reference_case": "brusselator-1d-50",
            "rtol": rtol,
            "dimension": fd.y0.len(),
            "arms": arms,
        }));
    }
    write_output(
        "ALG01_RUNS",
        &json!({
            "node": "alg01_coupled_stage_target_20261008",
            "export": "export_runs",
            "budget": BUDGET,
            "pilot_ladder_budget": PILOT_LADDER_BUDGET,
            "arms": alg01_arms().iter().map(|(name, o)| json!({"arm": name, "options": o})).collect::<Vec<_>>(),
            "rows": rows,
            "ladders": ladder_rows,
            "fd_variant": fd_rows,
        }),
    );
}

/// ALG03 recorded run: B0, B1, B2, B2nf and Rbig on D1-D5, and the dense
/// twins of D1-D4 again.
#[test]
#[ignore = "recorded run of research/alg03_stage_budget_guard_20261008; release build"]
fn export_runs_alg03() {
    if std::env::var("ALG03_RUNS").is_err() {
        println!("ALG03_RUNS is not set; export skipped");
        return;
    }
    let (problems, cells) = alg03_cells();
    let spd07 = spd07_problems();
    let mut specs: Vec<(&'static str, &Problem, f64)> = cells
        .iter()
        .map(|c| (c.group, &problems[c.problem], c.rtol))
        .collect();
    for p in &spd07 {
        for rtol in C1_RTOLS {
            specs.push(("D5", p, rtol));
        }
    }
    let mut rows = Vec::new();
    for (group, p, rtol) in specs {
        let label = format!("{group} {} {rtol:e}", p.id);
        let mut arms = Map::new();
        for (name, options, budget) in alg03_arms() {
            arms.insert(
                name.into(),
                timed(&format!("{label} {name}"), || {
                    staged(p, rtol, options, budget)
                }),
            );
        }
        let mut row = json!({
            "group": group,
            "case": p.id,
            "rtol": rtol,
            "dimension": p.y0.len(),
            "max_attempts": p.max_attempts,
            "arms": arms,
        });
        if group != "D5" {
            row["twin"] = timed(&format!("{label} twin"), || twin(p, rtol));
        }
        rows.push(row);
    }
    write_output(
        "ALG03_RUNS",
        &json!({
            "node": "alg03_stage_budget_guard_20261008",
            "export": "export_runs_alg03",
            "arms": alg03_arms().iter().map(|(name, o, b)| json!({"arm": name, "options": o, "budget": b})).collect::<Vec<_>>(),
            "rows": rows,
        }),
    );
}

// ------------------------------------------------------- contract tests

/// The pilot table (`pilot/phase3_code/target/coupled_target.py`, rounded
/// to three decimals).
const PILOT_TAU_Y: [f64; 8] = [1.729, 0.511, 6.246, 6.081, 3.82, 4.771, 1.011, 1.0];
const PILOT_TAU_E: [f64; 8] = [1.393, 2.057, 2.266, 2.16, 1.51, 1.792, 2.011, 1.0];

#[test]
fn transfer_constants_reproduce_the_pilot_table() {
    let tau = rodas5p_stage_transfer_constants().unwrap();
    for (computed, pilot) in [(&tau.tau_y, PILOT_TAU_Y), (&tau.tau_e, PILOT_TAU_E)] {
        assert_eq!(computed.len(), 8);
        for (c, p) in computed.iter().zip(pilot) {
            assert!((c - p).abs() <= 6.0e-4, "computed {c} pilot {p}");
        }
    }
    // Pilot sums 25.2 and 14.2.
    assert!((tau.tau_y.iter().sum::<f64>() - 25.2).abs() < 0.05);
    assert!((tau.tau_e.iter().sum::<f64>() - 14.2).abs() < 0.05);
}

#[test]
fn coupled_targets_follow_the_registered_rule() {
    let p = spd07_problem("hires");
    let tau = rodas5p_stage_transfer_constants().unwrap();
    let options = StageTargetOptions::new(StageTargetPolicy::Coupled);
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &gmres_config(BUDGET)).unwrap();
    work.set_stage_target(options).unwrap();
    let span = p.span();
    work.set_stage_target_span(span);
    let (atol, rtol) = (1.0e-10, 1.0e-6);
    for (h, e_hat) in [(1.0e-6, None), (1.0e-2, Some(0.02)), (0.5, Some(0.9))] {
        work.clear_accepted_error();
        if let Some(e) = e_hat {
            work.record_accepted_error(e);
        }
        let mut counters = WorkCounters::default();
        work.attempt(
            &p.problem,
            0.0,
            &p.y0,
            h,
            true,
            None,
            atol,
            rtol,
            &mut counters,
        )
        .unwrap();
        let e: f64 = e_hat.unwrap_or(1.0e-6);
        let theta = 0.2 * (h / span) * (e / 0.5).min(1.0).powf(1.2);
        for i in 0..8 {
            let mut expected = theta / (8.0 * tau.tau_y[i].max(tau.tau_e[i]));
            if i == 7 {
                expected = expected.min(0.1 * 0.18_f64.powi(5));
            }
            let got = work.stage_eps()[i];
            assert!((got - expected).abs() <= 1.0e-12 * expected, "h={h} i={i}");
        }
        assert_eq!(
            counters.diagnostic_matvecs, 0,
            "no duplicate final residual"
        );
    }
    // The U8 cap binds at a large step on a long-enough span.
    work.set_stage_target_span(1.0);
    work.record_accepted_error(1.0);
    let mut counters = WorkCounters::default();
    let _ = work.attempt(
        &p.problem,
        0.0,
        &p.y0,
        1.0,
        true,
        None,
        atol,
        rtol,
        &mut counters,
    );
    assert_eq!(work.stage_eps()[7], 0.1 * (0.9_f64 / 5.0).powi(5));
}

#[test]
fn legacy_policy_is_the_gmres_into_driver_bit_for_bit() {
    for id in ["quadratic-4", "prothero-robinson-forced", "robertson"] {
        let p = spd07_problem(id);
        for rtol in C1_RTOLS {
            let options = StageTargetOptions::new(StageTargetPolicy::Legacy);
            let run = integrate_rodas5p_mf_fast_observed_with_stage_target(
                &p.problem,
                p.t_span,
                &p.y0,
                &gmres_config(BUDGET),
                &p.adaptive(rtol),
                &p.schedule(),
                options,
            )
            .unwrap();
            assert_eq!(run.statistics, Default::default());
            assert_eq!(
                staged(&p, rtol, options, BUDGET),
                legacy(&p, rtol, BUDGET),
                "{id} {rtol:e}"
            );
        }
    }
}

#[test]
fn legacy_rows_reproduce_the_base_export_on_small_cases() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/alg01_coupled_stage_target_20261008/BASE.json");
    let base: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for p in [
        spd07_problem("quadratic-4"),
        spd07_problem("prothero-robinson-forced"),
    ] {
        for rtol in C1_RTOLS {
            let row = base["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["group"] == "C1" && r["case"] == p.id.as_str() && r["rtol"] == rtol)
                .unwrap();
            let options = StageTargetOptions::new(StageTargetPolicy::Legacy);
            assert_eq!(staged(&p, rtol, options, BUDGET), row["legacy"]);
        }
    }
}

#[test]
fn staged_policies_integrate_small_cases_without_a_duplicate_residual() {
    for id in ["quadratic-4", "prothero-robinson-forced"] {
        let p = spd07_problem(id);
        let exact = (p.exact.as_ref().unwrap())(p.t_span.1);
        let rtol = 1.0e-6;
        let legacy_run = legacy(&p, rtol, BUDGET);
        for (name, options) in alg01_arms().into_iter().skip(1) {
            let run = staged(&p, rtol, options, BUDGET);
            assert_eq!(run["success"], true, "{id} {name}");
            let stats = &run["stage_statistics"];
            assert!(stats["solves"].as_u64().unwrap() > 0);
            assert_eq!(stats["failed"], 0);
            assert_eq!(run["counters"]["diagnostic_matvecs"], 0, "{id} {name}");
            assert!(
                run["counters"]["jvp_vectors"].as_u64().unwrap()
                    < legacy_run["counters"]["jvp_vectors"].as_u64().unwrap(),
                "{id} {name}"
            );
            let y: Vec<f64> = run["y_last"]
                .as_array()
                .unwrap()
                .iter()
                .map(|h| f64::from_bits(u64::from_str_radix(h.as_str().unwrap(), 16).unwrap()))
                .collect();
            assert!(relative_max_norm(&y, &exact) < 100.0 * rtol, "{id} {name}");
        }
    }
}

#[test]
fn staged_policies_refuse_other_solvers() {
    let p = spd07_problem("quadratic-4");
    let config = rodas5p_core::LinearSolverConfig {
        method: rodas5p_core::LinearMethod::Lgmres,
        ..gmres_config(BUDGET)
    };
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &config).unwrap();
    assert!(
        work.set_stage_target(StageTargetOptions::new(StageTargetPolicy::Coupled))
            .is_err()
    );
    assert!(
        work.set_stage_target(StageTargetOptions::new(StageTargetPolicy::Legacy))
            .is_ok()
    );
}

/// With restart 4 and a budget of 8 columns, solves that need a restart
/// meet the guard; classifying the aborts changes nothing in the run.
#[test]
fn guard_classification_does_not_change_the_run() {
    let mut p = brusselator_problem(30);
    p.t_span = (0.0, 1.0);
    let rtol = 1.0e-9;
    let config = rodas5p_core::LinearSolverConfig {
        restart: 4,
        ..gmres_config(8)
    };
    let run = |options: StageTargetOptions| {
        integrate_rodas5p_mf_fast_observed_with_stage_target(
            &p.problem,
            p.t_span,
            &p.y0,
            &config,
            &p.adaptive(rtol),
            &p.schedule(),
            options,
        )
        .unwrap()
    };
    let guarded = StageTargetOptions {
        stagnation_guard: true,
        ..StageTargetOptions::new(StageTargetPolicy::CoupledGuarded)
    };
    let plain = run(guarded);
    let classified = run(StageTargetOptions {
        classify_guard_aborts: true,
        ..guarded
    });
    let stats = classified.statistics;
    assert!(
        stats.guard_contraction + stats.guard_overrun > 0,
        "the guard fired: {stats:?}"
    );
    assert_eq!(stats.guard_false + stats.guard_true, stats.failed_guard);
    assert_eq!(
        result_json(&Ok(classified.result)),
        result_json(&Ok(plain.result))
    );
    assert_eq!(
        rodas5p_integrators::StageSolveStatistics {
            guard_false: 0,
            guard_true: 0,
            ..stats
        },
        plain.statistics
    );
}
