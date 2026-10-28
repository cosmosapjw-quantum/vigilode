//! Research nodes `research/alg04_coupled_target_v2_20261010` (ALG04: the
//! coupled stage target, second test) and
//! `research/alg06_guard_v2_20261010` (ALG06: the stage budget guard, second
//! test).
//!
//! Neither node has a base export of its own: ALG04 reuses ALG01's
//! `BASE.json` and ALG06 reuses ALG03's, both recorded on the unmodified
//! solver source. The cells, references, twins and error metric are those of
//! `alg01_common` (shared with `alg01_coupled_stage_target.rs`, which is not
//! changed). The cell lists below transcribe the ALG01 and ALG03 lists; the
//! contract tests check them against the two base exports.
//!
//! The recorded runs (`export_runs_alg04`, `export_runs_alg06`) run only
//! when `ALG04_RUNS` / `ALG06_RUNS` is set; the gates are evaluated by
//! `tools/alg04_coupled_target_v2_check.py` and `tools/alg06_guard_v2_check.py`.

// (The exports use `write_output`, re-exported by the shared module.)
#[allow(unused_imports)]
mod alg01_common;

use alg01_common::*;
use serde_json::Value;

/// C1 rtols (ALG01 C1, ALG03 D5).
const C1_RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];
/// C2 rtols (HIRES and Robertson).
const C2_RTOLS: [f64; 3] = [1.0e-9, 1.0e-10, 1.0e-11];
/// C3 rtols (stress cells).
const C3_RTOLS: [f64; 3] = [1.0e-4, 1.0e-6, 1.0e-8];

/// ALG04's ladder budget for every arm (the ALG01 registration defect
/// removed); adaptive cells keep [`BUDGET`].
const ALG04_LADDER_BUDGET: usize = PILOT_LADDER_BUDGET;

/// One adaptive cell: group, problem index, rtol.
#[derive(Clone, Copy, Debug)]
struct CellSpec {
    group: &'static str,
    problem: usize,
    rtol: f64,
}

/// The ALG04 adaptive cells: ALG01's C1, C2, C3 and C5, in ALG01's order.
fn alg04_cells() -> (Vec<Problem>, Vec<CellSpec>) {
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

/// The ALG06 cells: ALG03's D1-D4 and the 14 D5 (SPD07) cells.
fn alg06_cells() -> (Vec<Problem>, Vec<CellSpec>) {
    let mut problems = vec![
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
    let mut cells: Vec<CellSpec> = specs
        .into_iter()
        .map(|(group, problem, rtol)| CellSpec {
            group,
            problem,
            rtol,
        })
        .collect();
    let first = problems.len();
    problems.extend(spd07_problems());
    for i in first..problems.len() {
        for rtol in C1_RTOLS {
            cells.push(CellSpec {
                group: "D5",
                problem: i,
                rtol,
            });
        }
    }
    (problems, cells)
}

fn read_json(relative: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

const ALG01_BASE: &str = "research/alg01_coupled_stage_target_20261008/BASE.json";
const ALG03_BASE: &str = "research/alg03_stage_budget_guard_20261008/BASE.json";

type CellKey = (String, String, u64, usize);

fn cell_keys(problems: &[Problem], cells: &[CellSpec]) -> Vec<CellKey> {
    cells
        .iter()
        .map(|c| {
            let p = &problems[c.problem];
            (
                c.group.to_string(),
                p.id.clone(),
                c.rtol.to_bits(),
                p.y0.len(),
            )
        })
        .collect()
}

fn base_keys(rows: &Value) -> Vec<CellKey> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["group"].as_str().unwrap().to_string(),
                r["case"].as_str().unwrap().to_string(),
                r["rtol"].as_f64().unwrap().to_bits(),
                r["dimension"].as_u64().unwrap() as usize,
            )
        })
        .collect()
}

// ------------------------------------------------------- contract tests

/// ALG04's adaptive cells are ALG01's base cells, in the same order, and its
/// ladders are ALG01's.
#[test]
fn alg04_cells_are_the_alg01_base_cells() {
    let base = read_json(ALG01_BASE);
    let (problems, cells) = alg04_cells();
    assert_eq!(cells.len(), 65);
    assert_eq!(cell_keys(&problems, &cells), base_keys(&base["rows"]));
    let mut rungs = Vec::new();
    for ladder in ladders() {
        for &rtol in &ladder.rtols {
            for &k in &ladder.rungs {
                rungs.push((
                    ladder.id.to_string(),
                    rtol.to_bits(),
                    u64::from(k),
                    ladder.problem.y0.len(),
                ));
            }
        }
    }
    let base_rungs: Vec<_> = base["ladders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["ladder"].as_str().unwrap().to_string(),
                r["rtol"].as_f64().unwrap().to_bits(),
                r["k"].as_u64().unwrap(),
                r["dimension"].as_u64().unwrap() as usize,
            )
        })
        .collect();
    assert_eq!(rungs, base_rungs);
    assert_eq!(base["pilot_ladder_budget"], ALG04_LADDER_BUDGET);
    assert_eq!(base["budget"], BUDGET);
}

/// ALG06's D1-D4 cells are ALG03's base cells; D5 is ALG01's C1.
#[test]
fn alg06_cells_are_the_alg03_base_cells_and_the_spd07_set() {
    let base = read_json(ALG03_BASE);
    let (problems, cells) = alg06_cells();
    let keys = cell_keys(&problems, &cells);
    assert_eq!(keys.len(), 24);
    assert_eq!(keys[..10].to_vec(), base_keys(&base["rows"]));
    let (p01, c01) = alg04_cells();
    let c1: Vec<CellKey> = cell_keys(&p01, &c01)
        .into_iter()
        .filter(|k| k.0 == "C1")
        .map(|(_, case, rtol, n)| ("D5".to_string(), case, rtol, n))
        .collect();
    assert_eq!(keys[10..].to_vec(), c1);
    for (cell, row) in cells.iter().zip(base["rows"].as_array().unwrap()) {
        assert_eq!(
            row["max_attempts"].as_u64().unwrap() as usize,
            problems[cell.problem].max_attempts
        );
    }
}

/// The base references are present for every ALG04 and ALG06 problem.
#[test]
fn base_references_cover_every_problem() {
    let alg01 = read_json(ALG01_BASE);
    let (problems, _) = alg04_cells();
    for p in &problems {
        assert!(alg01["references"].get(&p.id).is_some(), "{}", p.id);
    }
    let alg03 = read_json(ALG03_BASE);
    let (problems, cells) = alg06_cells();
    for c in cells.iter().filter(|c| c.group != "D5") {
        let id = &problems[c.problem].id;
        assert!(alg03["references"].get(id).is_some(), "{id}");
    }
}

/// `Legacy` on a ladder rung at the ALG04 ladder budget reproduces ALG01's
/// base row at the pilot budget (the same budget), on the unmodified
/// source; the rung is cheap enough for a debug build.
#[test]
fn legacy_ladder_at_the_alg04_budget_reproduces_the_alg01_base() {
    let base = read_json(ALG01_BASE);
    let ladder = ladders().into_iter().find(|l| l.id == "semilin64").unwrap();
    let (rtol, k) = (1.0e-4, 3u32);
    let row = base["ladders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["ladder"] == "semilin64" && r["rtol"] == rtol && r["k"] == k)
        .unwrap();
    let run = ladder_mf(
        &ladder.problem,
        rtol,
        k,
        ALG04_LADDER_BUDGET,
        &mut |_| {},
        &mut |_, _| {},
    );
    assert_eq!(run, row["legacy_pilot_budget"]);
    assert_eq!(ladder_lu(&ladder.problem, rtol, k), row["lu"]);
}

// ------------------------------------------- implementation contracts

use rodas5p_core::{LinearSolverConfig, WorkCounters};
use rodas5p_integrators::{
    Rodas5pMfFastWorkspace, Rodas5pMfStageTargetResult, StageTargetOptions, StageTargetPolicy,
    integrate_rodas5p_mf_fast_observed_with_stage_target, rodas5p_stage_transfer_constants,
};

fn run_with(
    p: &Problem,
    rtol: f64,
    config: &LinearSolverConfig,
    options: StageTargetOptions,
) -> Rodas5pMfStageTargetResult {
    integrate_rodas5p_mf_fast_observed_with_stage_target(
        &p.problem,
        p.t_span,
        &p.y0,
        config,
        &p.adaptive(rtol),
        &p.schedule(),
        options,
    )
    .unwrap()
}

fn record(run: &Rodas5pMfStageTargetResult) -> Value {
    result_json(&Ok(run.result.clone()))
}

/// The counters `DupFix` changes: the diagnostic applications and the JVP
/// work they charge.
const DIAGNOSTIC_KEYS: [&str; 4] = [
    "diagnostic_matvecs",
    "jvp_calls",
    "jvp_vectors",
    "linear_matvec_vectors",
];

/// `DupFix` is `Legacy` bit for bit in states, attempts, accepted and
/// rejected steps; its JVPs are `Legacy`'s minus exactly the diagnostic
/// residuals counted, and no other counter changes.
#[test]
fn dup_fix_is_legacy_without_the_diagnostic_residual() {
    for id in [
        "quadratic-4",
        "prothero-robinson-forced",
        "robertson",
        "hires",
    ] {
        let p = spd07_problem(id);
        for rtol in C1_RTOLS {
            let config = gmres_config(BUDGET);
            let legacy = run_with(
                &p,
                rtol,
                &config,
                StageTargetOptions::new(StageTargetPolicy::Legacy),
            );
            let dup_fix = run_with(
                &p,
                rtol,
                &config,
                StageTargetOptions::new(StageTargetPolicy::DupFix),
            );
            let (l, d) = (record(&legacy), record(&dup_fix));
            for key in [
                "ok",
                "success",
                "t",
                "y_last",
                "attempts",
                "accepted",
                "rejected",
                "state_reuses",
            ] {
                assert_eq!(l[key], d[key], "{id} {rtol:e} {key}");
            }
            let (lc, dc) = (&l["counters"], &d["counters"]);
            let diagnostic = lc["diagnostic_matvecs"].as_u64().unwrap();
            assert!(diagnostic > 0);
            assert_eq!(dc["diagnostic_matvecs"], 0);
            assert_eq!(
                dc["jvp_vectors"].as_u64().unwrap(),
                lc["jvp_vectors"].as_u64().unwrap() - diagnostic,
                "{id} {rtol:e}"
            );
            assert_eq!(
                diagnostic,
                lc["linear_solves"].as_u64().unwrap(),
                "one diagnostic residual per successful stage solve"
            );
            for (key, value) in lc.as_object().unwrap() {
                if !DIAGNOSTIC_KEYS.contains(&key.as_str()) {
                    assert_eq!(&dc[key], value, "{id} {rtol:e} {key}");
                }
            }
            assert_eq!(dup_fix.statistics, Default::default());
        }
    }
}

#[test]
fn dup_fix_needs_gmres_into() {
    let p = spd07_problem("quadratic-4");
    let config = LinearSolverConfig {
        method: rodas5p_core::LinearMethod::Lgmres,
        ..gmres_config(BUDGET)
    };
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &config).unwrap();
    assert!(
        work.set_stage_target(StageTargetOptions::new(StageTargetPolicy::DupFix))
            .is_err()
    );
    // GMRES without GMRES into: refused at the attempt.
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &gmres_config(BUDGET)).unwrap();
    work.set_stage_target(StageTargetOptions::new(StageTargetPolicy::DupFix))
        .unwrap();
    let mut counters = WorkCounters::default();
    assert!(
        work.attempt(
            &p.problem,
            0.0,
            &p.y0,
            1.0e-3,
            true,
            None,
            1.0e-8,
            1.0e-6,
            &mut counters
        )
        .is_err()
    );
}

/// Above the restart length `CoupledGuarded2` is `CoupledGuarded` bit for
/// bit (exhaustion never applies); at or below it every stage solve is
/// exhausted.
#[test]
fn coupled_guarded2_is_coupled_guarded_above_the_restart_length() {
    let mut p = brusselator_problem(30);
    p.t_span = (0.0, 1.0);
    assert!(p.y0.len() > 40);
    let config = gmres_config(BUDGET);
    for rtol in C1_RTOLS {
        let cg = run_with(
            &p,
            rtol,
            &config,
            StageTargetOptions::new(StageTargetPolicy::CoupledGuarded),
        );
        let cg2 = run_with(
            &p,
            rtol,
            &config,
            StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2),
        );
        assert_eq!(record(&cg2), record(&cg));
        assert_eq!(cg2.statistics, cg.statistics);
        assert_eq!(cg2.statistics.exhaustion_solves, 0);
    }
    for id in ["quadratic-4", "prothero-robinson-forced", "robertson"] {
        let p = spd07_problem(id);
        let run = run_with(
            &p,
            1.0e-6,
            &config,
            StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2),
        );
        assert!(run.result.observed.success, "{id}");
        let s = run.statistics;
        assert!(s.solves > 0);
        assert_eq!(s.exhaustion_solves, s.solves, "{id}");
        assert_eq!(s.failed, 0, "{id}");
        assert!(s.max_columns <= 2 * p.y0.len() as u64, "{id} {s:?}");
        assert_eq!(run.result.observed.counters.diagnostic_matvecs, 0);
        if let Some(exact) = &p.exact {
            let y = run.result.observed.y.last().unwrap();
            assert!(
                relative_max_norm(y, &exact(p.t_span.1)) < 100.0 * 1.0e-6,
                "{id}"
            );
        }
    }
}

/// The ALG06 arms, cumulative on the `CoupledGuarded2` target.
fn alg06_options(g1: bool, g2: bool, g3: bool) -> StageTargetOptions {
    StageTargetOptions {
        stagnation_guard: true,
        classify_guard_aborts: true,
        classify_accepted_guard_aborts: true,
        effective_cycle_overrun: g1,
        floor_at_confirmations: g2,
        charge_fallback_residual: g3,
        ..StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2)
    }
}

/// Where neither the guard, the fallback nor the floor fires, `B3` at
/// budget 2,000 is `CoupledGuarded2` at budget 200 bit for bit.
#[test]
fn b3_is_coupled_guarded2_where_nothing_fires() {
    for id in ["quadratic-4", "prothero-robinson-forced"] {
        let p = spd07_problem(id);
        let rtol = 1.0e-6;
        let b3 = run_with(
            &p,
            rtol,
            &gmres_config(BIG_BUDGET),
            alg06_options(true, true, true),
        );
        let s = b3.statistics;
        assert_eq!(
            s.guard_contraction + s.guard_overrun + s.fallback_accepted + s.floor_accepted,
            0,
            "{id}"
        );
        let cg2 = run_with(
            &p,
            rtol,
            &gmres_config(BUDGET),
            StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2),
        );
        assert_eq!(record(&b3), record(&cg2), "{id}");
        assert!(b3.charges.is_empty());
    }
}

/// A small Brusselator with restart 4 and budget 8: guard aborts that the
/// production fallback accepts.
fn fallback_case() -> (Problem, LinearSolverConfig, f64) {
    let mut p = brusselator_problem(30);
    p.t_span = (0.0, 1.0);
    let config = LinearSolverConfig {
        restart: 4,
        ..gmres_config(8)
    };
    (p, config, 1.0e-6)
}

/// An `e_hat` that makes the coupled targets far tighter than the
/// production rule, so the guard aborts and the fallback accepts.
const E_HAT_TIGHT: f64 = 1.0e-12;

/// `G3`: an attempt's returned error is its embedded error plus
/// `sum_i tau_e,i ||r_i||_WRMS` over its fallback-accepted stages; without
/// `G3` the same charge is only recorded and the stage solves are the same.
/// Two workspaces step in lockstep along one fixed-step trajectory, with
/// the same tight `e_hat` and the floor `G2` off.
#[test]
fn g3_charges_the_fallback_residual_to_the_attempt_error() {
    let (p, config, rtol) = fallback_case();
    let atol = p.atol_scale * rtol;
    let tau_e = rodas5p_stage_transfer_constants().unwrap().tau_e;
    let mut works: Vec<Rodas5pMfFastWorkspace> = [false, true]
        .into_iter()
        .map(|g3| {
            let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &config).unwrap();
            work.set_gmres_into(true);
            work.set_stage_target(alg06_options(true, false, g3))
                .unwrap();
            work.set_stage_target_span(p.span());
            work.record_accepted_error(E_HAT_TIGHT);
            work
        })
        .collect();
    let steps = 20;
    let h = 5.0e-3;
    let mut y = p.y0.clone();
    let mut charged_attempts = 0;
    for step in 0..steps {
        let t = p.t_span.0 + step as f64 * h;
        let mut outcomes = Vec::new();
        for work in works.iter_mut() {
            let mut counters = WorkCounters::default();
            outcomes.push(
                work.attempt(&p.problem, t, &y, h, true, None, atol, rtol, &mut counters)
                    .ok(),
            );
        }
        // A failed stage solve fails both attempts alike; the lockstep ends.
        assert_eq!(outcomes[0].is_some(), outcomes[1].is_some(), "step {step}");
        let Some(errors) = outcomes.iter().copied().collect::<Option<Vec<f64>>>() else {
            break;
        };
        let (off, on) = (&works[0], &works[1]);
        assert_eq!(off.y_new(), on.y_new(), "step {step}");
        assert_eq!(
            off.last_attempt_charge().to_bits(),
            on.last_attempt_charge().to_bits()
        );
        let (s_off, s_on) = (off.stage_statistics(), on.stage_statistics());
        assert_eq!(s_off.fallback_accepted, s_on.fallback_accepted);
        let charge = on.last_attempt_charge();
        if on.charge_log().len() > charged_attempts {
            charged_attempts += 1;
            assert_eq!(on.charge_log().len(), charged_attempts);
            assert_eq!(off.charge_log().len(), charged_attempts);
            let (l_off, l_on) = (
                off.charge_log().last().unwrap(),
                on.charge_log().last().unwrap(),
            );
            assert!(l_on.charged && !l_off.charged);
            assert!(charge > 0.0);
            assert!(charge <= tau_e.iter().sum::<f64>() * f64::from(l_on.stages));
            assert_eq!(l_on.error, errors[0]);
            assert_eq!(l_on.charge, charge);
            assert_eq!(errors[1], errors[0] + charge, "step {step}");
        } else {
            assert_eq!(charge, 0.0);
            assert_eq!(errors[1].to_bits(), errors[0].to_bits(), "step {step}");
        }
        y.copy_from_slice(works[0].y_new());
        for work in works.iter_mut() {
            work.record_accepted_error(E_HAT_TIGHT);
        }
    }
    assert!(charged_attempts > 0, "the fallback accepted a stage");
}

/// With `G3` an attempt whose charged error exceeds 1 is rejected through
/// the local-error path: every charge that crosses 1 is a local-error
/// failure, never a linear-solve failure.
#[test]
fn g3_rejections_take_the_local_error_path() {
    let (p, config, rtol) = fallback_case();
    let b3 = run_with(&p, rtol, &config, alg06_options(true, true, true));
    let b3b = run_with(&p, rtol, &config, alg06_options(true, true, false));
    for run in [&b3, &b3b] {
        assert_eq!(run.statistics.charged_attempts, run.charges.len() as u64);
        let sum: f64 = run.charges.iter().map(|c| c.charge).sum();
        assert!((sum - run.statistics.charge_sum).abs() <= 1.0e-12 * sum.max(1.0));
    }
    assert!(b3.statistics.charged_attempts > 0);
    assert!(b3.charges.iter().all(|c| c.charged));
    assert!(b3b.charges.iter().all(|c| !c.charged));
    let crossing = b3
        .charges
        .iter()
        .filter(|c| c.error <= 1.0 && c.error + c.charge > 1.0)
        .count() as u64;
    assert_eq!(crossing, b3.statistics.charge_crosses_one);
    let counters = b3.result.observed.counters;
    assert!(counters.local_error_failures >= crossing);
    // The charge never turns into a linear-solve failure.
    let failed_solve_attempts = b3.statistics.failed;
    assert!(counters.linear_solve_failures <= failed_solve_attempts);
}

#[test]
fn g3_needs_a_coupled_target() {
    let p = spd07_problem("quadratic-4");
    let mut work = Rodas5pMfFastWorkspace::new(&p.problem, &gmres_config(BUDGET)).unwrap();
    let options = StageTargetOptions {
        charge_fallback_residual: true,
        ..StageTargetOptions::new(StageTargetPolicy::ProjL2)
    };
    assert!(work.set_stage_target(options).is_err());
}

/// Classifying the guard aborts the fallback accepted (ALG06 reporting)
/// changes nothing in a run in which the guard fires (the accepted-abort
/// shadow itself is checked on a single solve in
/// `rodas5p-krylov/tests/alg04_alg06_staged_contracts.rs`).
#[test]
fn classifying_accepted_guard_aborts_does_not_change_the_run() {
    let (p, config, rtol) = fallback_case();
    let classified = run_with(&p, rtol, &config, alg06_options(true, true, true));
    let plain = run_with(
        &p,
        rtol,
        &config,
        StageTargetOptions {
            classify_accepted_guard_aborts: false,
            ..alg06_options(true, true, true)
        },
    );
    let s = classified.statistics;
    assert!(s.guard_contraction + s.guard_overrun > 0, "{s:?}");
    assert_eq!(
        s.guard_accepted_false + s.guard_accepted_true,
        s.fallback_after_guard
    );
    assert_eq!(s.guard_false + s.guard_true, s.failed_guard);
    assert_eq!(record(&classified), record(&plain));
    assert_eq!(classified.charges, plain.charges);
    assert_eq!(
        rodas5p_integrators::StageSolveStatistics {
            guard_accepted_false: 0,
            guard_accepted_true: 0,
            ..s
        },
        plain.statistics
    );
}

// ------------------------------------------------------------------ RUNS

use serde_json::{Map, json};

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = std::time::Instant::now();
    let value = f();
    println!("{label}: {:.1} s", start.elapsed().as_secs_f64());
    value
}

/// The `Legacy` stage target: the unchanged GMRES-into driver (SPD07's
/// `gmres_into` arm, ALG01's base) with Krylov budget `budget`.
fn legacy_run(p: &Problem, rtol: f64, budget: usize) -> Value {
    result_json(
        &rodas5p_integrators::integrate_rodas5p_mf_fast_observed_gmres_into(
            &p.problem,
            p.t_span,
            &p.y0,
            &gmres_config(budget),
            &p.adaptive(rtol),
            &p.schedule(),
        ),
    )
}

/// One run of the stage-target entry point: the SPD07 record, plus the
/// staged solves' statistics and the attempts with a fallback-accepted
/// stage for every staged policy. `Legacy` runs through the GMRES-into
/// driver, exactly as ALG01's base export.
fn staged_run(p: &Problem, rtol: f64, options: StageTargetOptions, budget: usize) -> Value {
    if options.policy == StageTargetPolicy::Legacy {
        return legacy_run(p, rtol, budget);
    }
    let run = integrate_rodas5p_mf_fast_observed_with_stage_target(
        &p.problem,
        p.t_span,
        &p.y0,
        &gmres_config(budget),
        &p.adaptive(rtol),
        &p.schedule(),
        options,
    );
    match run {
        Ok(r) => {
            let mut record = result_json(&Ok(r.result));
            if options.policy.is_staged() {
                record["stage_statistics"] = serde_json::to_value(r.statistics).unwrap();
                record["charges"] = json!(
                    r.charges
                        .iter()
                        .map(|c| json!([c.t, c.h, c.error, c.charge, c.stages, c.charged]))
                        .collect::<Vec<_>>()
                );
            }
            record
        }
        Err(e) => result_json(&Err(e)),
    }
}

/// A rung with a stage-target policy (span 1, `e_hat` = the previous
/// step's embedded error, every step accepted); `Legacy` is ALG01's rung.
fn staged_ladder_run(
    p: &Problem,
    rtol: f64,
    k: u32,
    budget: usize,
    options: StageTargetOptions,
) -> Value {
    if options.policy == StageTargetPolicy::Legacy {
        return ladder_mf(p, rtol, k, budget, &mut |_| {}, &mut |_, _| {});
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
            if options.policy.is_staged() {
                record["stage_statistics"] = serde_json::to_value(work.stage_statistics()).unwrap();
            }
        },
    )
}

/// The ALG04 arms (registered order); `coupled_guarded2` is gated.
fn alg04_arms() -> Vec<(&'static str, StageTargetOptions)> {
    [
        ("legacy", StageTargetPolicy::Legacy),
        ("dup_fix", StageTargetPolicy::DupFix),
        ("proj_l2", StageTargetPolicy::ProjL2),
        ("l2_coupled", StageTargetPolicy::L2Coupled),
        ("coupled_guarded", StageTargetPolicy::CoupledGuarded),
        ("coupled_guarded2", StageTargetPolicy::CoupledGuarded2),
    ]
    .into_iter()
    .map(|(name, policy)| (name, StageTargetOptions::new(policy)))
    .collect()
}

/// ALG04 recorded run: every arm on every adaptive cell (budget 200) and on
/// every ladder rung (budget 20,000), the dense twins and LU rungs again,
/// and `Legacy` on the rungs at budget 200 (ALG01's base `legacy` rows).
#[test]
#[ignore = "recorded run of research/alg04_coupled_target_v2_20261010; release build"]
fn export_runs_alg04() {
    if std::env::var("ALG04_RUNS").is_err() {
        println!("ALG04_RUNS is not set; export skipped");
        return;
    }
    let (problems, cells) = alg04_cells();
    let mut rows = Vec::new();
    for cell in &cells {
        let p = &problems[cell.problem];
        let label = format!("{} {} {:e}", cell.group, p.id, cell.rtol);
        let mut arms = Map::new();
        for (name, options) in alg04_arms() {
            arms.insert(
                name.into(),
                timed(&format!("{label} {name}"), || {
                    staged_run(p, cell.rtol, options, BUDGET)
                }),
            );
        }
        rows.push(json!({
            "group": cell.group,
            "case": p.id,
            "rtol": cell.rtol,
            "dimension": p.y0.len(),
            "arms": arms,
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
                for (name, options) in alg04_arms() {
                    arms.insert(
                        name.into(),
                        timed(&format!("{label} {name} {ALG04_LADDER_BUDGET}"), || {
                            staged_ladder_run(p, rtol, k, ALG04_LADDER_BUDGET, options)
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
                    "legacy_budget_200": timed(&format!("{label} legacy {BUDGET}"), || {
                        staged_ladder_run(p, rtol, k, BUDGET, StageTargetOptions::default())
                    }),
                }));
            }
        }
    }
    write_output(
        "ALG04_RUNS",
        &json!({
            "node": "alg04_coupled_target_v2_20261010",
            "export": "export_runs_alg04",
            "budget": BUDGET,
            "ladder_budget": ALG04_LADDER_BUDGET,
            "arms": alg04_arms().iter().map(|(name, o)| json!({"arm": name, "options": o})).collect::<Vec<_>>(),
            "rows": rows,
            "ladders": ladder_rows,
        }),
    );
}

/// The ALG06 arms: name, options and Krylov budget. `b2`..`b3` are
/// cumulative on the `CoupledGuarded2` target with the stagnation guard and
/// the production fallback; every guard abort is classified by the
/// uncounted shadow continuation. `rbig` is the rival.
fn alg06_arms() -> Vec<(&'static str, StageTargetOptions, usize)> {
    vec![
        ("b2", alg06_options(false, false, false), BIG_BUDGET),
        ("b3a", alg06_options(true, false, false), BIG_BUDGET),
        ("b3b", alg06_options(true, true, false), BIG_BUDGET),
        ("b3", alg06_options(true, true, true), BIG_BUDGET),
        (
            "rbig",
            StageTargetOptions::new(StageTargetPolicy::Legacy),
            BIG_BUDGET,
        ),
    ]
}

/// ALG06 recorded run: B2, B3a, B3b, B3 and Rbig on D1-D5, the
/// `CoupledGuarded2` run at budget 200 on D5 (identity item), and the dense
/// twins of D1-D4 again.
#[test]
#[ignore = "recorded run of research/alg06_guard_v2_20261010; release build"]
fn export_runs_alg06() {
    if std::env::var("ALG06_RUNS").is_err() {
        println!("ALG06_RUNS is not set; export skipped");
        return;
    }
    let (problems, cells) = alg06_cells();
    let mut rows = Vec::new();
    for cell in &cells {
        let p = &problems[cell.problem];
        let rtol = cell.rtol;
        let label = format!("{} {} {rtol:e}", cell.group, p.id);
        let mut arms = Map::new();
        for (name, options, budget) in alg06_arms() {
            arms.insert(
                name.into(),
                timed(&format!("{label} {name}"), || {
                    staged_run(p, rtol, options, budget)
                }),
            );
        }
        if cell.group == "D5" {
            arms.insert(
                "coupled_guarded2_200".into(),
                timed(&format!("{label} coupled_guarded2_200"), || {
                    staged_run(
                        p,
                        rtol,
                        StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2),
                        BUDGET,
                    )
                }),
            );
        }
        let mut row = json!({
            "group": cell.group,
            "case": p.id,
            "rtol": rtol,
            "dimension": p.y0.len(),
            "max_attempts": p.max_attempts,
            "arms": arms,
        });
        if cell.group != "D5" {
            row["twin"] = timed(&format!("{label} twin"), || twin(p, rtol));
        }
        rows.push(row);
    }
    let mut arms_meta: Vec<Value> = alg06_arms()
        .iter()
        .map(|(name, o, b)| json!({"arm": name, "options": o, "budget": b}))
        .collect();
    arms_meta.push(json!({
        "arm": "coupled_guarded2_200",
        "options": StageTargetOptions::new(StageTargetPolicy::CoupledGuarded2),
        "budget": BUDGET,
        "cells": "D5",
    }));
    write_output(
        "ALG06_RUNS",
        &json!({
            "node": "alg06_guard_v2_20261010",
            "export": "export_runs_alg06",
            "charge_columns": ["t", "h", "error", "charge", "stages", "charged"],
            "arms": arms_meta,
            "rows": rows,
        }),
    );
}
