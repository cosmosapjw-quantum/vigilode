//! Caller-owned GMRES output and bounded workspace (research node
//! `research/rnext02_gmres_into_20261003`, remaining-only DAG node
//! R-NEXT-02). Run with `--ignored --test-threads=1`: the counting allocator
//! is global to this binary.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{
    Counting, adaptive, allocations_during, cases, convection_diffusion, linear_config,
    sequential_adaptive, splitmix_vector, stage_operator, stage_right_hand_sides, write_output,
};
use rodas5p_core::{
    DenseOperator, IdentityPreconditioner, LinearMethod, LinearOperator, WorkCounters,
};
use rodas5p_integrators::{
    OutputSchedule, Rodas5pMfFastWorkspace, integrate_rodas5p_mf_fast_observed,
    integrate_rodas5p_mf_fast_observed_gmres_into,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresWorkspace, solve_gmres_into,
    solve_gmres_with_workspace_and_residual_scale,
};
use serde_json::{Value, json};

#[global_allocator]
static GLOBAL: Counting = Counting;

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

const CONFIGS: [(&str, GmresConfig); 2] = [
    (
        "restart40-budget200-rtol1e-11",
        GmresConfig {
            restart: 40,
            max_arnoldi: 200,
            rtol: 1.0e-11,
            atol: 1.0e-14,
        },
    ),
    (
        "restart10-budget400-rtol1e-12",
        GmresConfig {
            restart: 10,
            max_arnoldi: 400,
            rtol: 1.0e-12,
            atol: 1.0e-14,
        },
    ),
];

/// One family: a sequence of right-hand sides on one operator, one
/// configuration, one initial-guess mode, one residual scale. Returns
/// (identity holds, family record).
fn family(
    label: &str,
    op: &dyn LinearOperator,
    systems: &[Vec<f64>],
    config: &GmresConfig,
    previous_x0: bool,
    scale: Option<&[f64]>,
) -> (bool, Value) {
    let n = op.dimension();
    let pc = IdentityPreconditioner::new(n);
    let (mut old_ws, mut new_ws) = (GmresWorkspace::default(), GmresWorkspace::default());
    let mut output = vec![0.0; n];
    // Warm-up on the first system (not counted).
    let _ = solve_gmres_with_workspace_and_residual_scale(
        op,
        &pc,
        &systems[0],
        None,
        config,
        scale,
        &mut old_ws,
        &mut WorkCounters::default(),
    );
    let _ = solve_gmres_into(
        op,
        &pc,
        &systems[0],
        None,
        config,
        scale,
        &mut output,
        &mut new_ws,
        GmresCapacity::unbounded(),
        &mut WorkCounters::default(),
    );
    let mut identical = true;
    let (mut old_alloc, mut new_alloc) = (0usize, 0usize);
    let (mut iterations, mut cycles, mut solves, mut failures) = (0u64, 0u64, 0usize, 0usize);
    let mut previous: Option<Vec<f64>> = None;
    let mut errors = Vec::new();
    for b in systems {
        let x0 = if previous_x0 { previous.clone() } else { None };
        let mut old_counters = WorkCounters::default();
        let (old, a_old) = allocations_during(|| {
            solve_gmres_with_workspace_and_residual_scale(
                op,
                &pc,
                b,
                x0.as_deref(),
                config,
                scale,
                &mut old_ws,
                &mut old_counters,
            )
        });
        let sentinel = vec![f64::from_bits(0x7ff8_dead_beef_0001); n];
        output.copy_from_slice(&sentinel);
        let mut new_counters = WorkCounters::default();
        let (new, a_new) = allocations_during(|| {
            solve_gmres_into(
                op,
                &pc,
                b,
                x0.as_deref(),
                config,
                scale,
                &mut output,
                &mut new_ws,
                GmresCapacity::unbounded(),
                &mut new_counters,
            )
        });
        old_alloc += a_old;
        new_alloc += a_new;
        solves += 1;
        identical &= old_counters == new_counters;
        match (old, new) {
            (Ok(old), Ok(new)) => {
                identical &= bits(&output) == bits(&old.x)
                    && new.residual_norm.to_bits() == old.residual_norm.to_bits()
                    && new.iterations == old.iterations;
                iterations += new.iterations;
                cycles += new.cycles;
                previous = Some(old.x);
            }
            (Err(old), Err(new)) => {
                failures += 1;
                let same = old.to_string() == new.to_string();
                identical &= same && bits(&output) == bits(&sentinel);
                errors.push(old.to_string());
                previous = None;
            }
            (old, new) => {
                identical = false;
                errors.push(format!("old {:?} new {:?}", old.err(), new.err()));
            }
        }
    }
    let columns_per_cycle = iterations as f64 / cycles.max(1) as f64;
    let (old_per, new_per) = (
        old_alloc as f64 / solves as f64,
        new_alloc as f64 / solves as f64,
    );
    let ratio = new_per / old_per.max(1.0);
    let gated = columns_per_cycle >= 4.0;
    let alloc_ok = !gated || ratio <= 0.25;
    (
        identical,
        json!({
            "family": label, "dimension": n, "solves": solves, "failures": failures,
            "errors": errors, "bitwise_identical": identical,
            "iterations": iterations, "cycles": cycles, "columns_per_cycle": columns_per_cycle,
            "least_squares_solves": {"old": iterations, "new": cycles},
            "allocations_per_solve": {"old": old_per, "new": new_per, "ratio": ratio},
            "allocation_gated": gated, "allocation_ok": alloc_ok,
        }),
    )
}

#[test]
#[ignore = "research run of research/rnext02_gmres_into_20261003; release build, one test thread"]
fn gmres_into_study() {
    let mut gate_identity = true;
    let mut gate_alloc = true;
    let mut families = Vec::new();
    let mut excluded = Vec::new();
    for case in cases() {
        for h in case.steps {
            let generator = linear_config(LinearMethod::Gmres, 1.0e-11);
            let mut work = Rodas5pMfFastWorkspace::new(&case.problem, &generator).unwrap();
            let attempt = work.attempt(
                &case.problem,
                case.t_span.0,
                &case.y0,
                h,
                true,
                None,
                1.0e-6 * case.atol_scale,
                1.0e-6,
                &mut WorkCounters::default(),
            );
            if let Err(error) = attempt {
                excluded.push(json!({"problem": case.id, "h": h, "error": error.to_string()}));
                continue;
            }
            let op = stage_operator(&case.problem, case.t_span.0, &case.y0, h);
            let systems = stage_right_hand_sides(&op, &work);
            for (name, config) in &CONFIGS {
                for previous in [false, true] {
                    let label = format!(
                        "{}/h={h:e}/{name}/x0={}",
                        case.id,
                        if previous { "previous" } else { "zero" }
                    );
                    let (identical, record) = family(&label, &op, &systems, config, previous, None);
                    gate_identity &= identical;
                    gate_alloc &= record["allocation_ok"].as_bool().unwrap();
                    println!(
                        "{label}: identical {identical}, alloc ratio {:.3}, cols/cycle {:.1}",
                        record["allocations_per_solve"]["ratio"].as_f64().unwrap(),
                        record["columns_per_cycle"].as_f64().unwrap()
                    );
                    families.push(record);
                }
            }
        }
    }
    // Convection-diffusion, n = 200, Peclet 10, tau 1e-3, three right-hand sides.
    let a = convection_diffusion(200, 10.0, 1.0e-3);
    let op = DenseOperator::new(a).unwrap();
    let systems: Vec<Vec<f64>> = (0..3).map(|seed| splitmix_vector(seed, 200)).collect();
    let scale: Vec<f64> = systems[0]
        .iter()
        .map(|v| 1.0e-8 + 1.0e-6 * v.abs())
        .collect();
    for (name, config) in &CONFIGS {
        for previous in [false, true] {
            for scaled in [false, true] {
                let label = format!(
                    "convection-diffusion-200/{name}/x0={}/scale={scaled}",
                    if previous { "previous" } else { "zero" }
                );
                let (identical, record) = family(
                    &label,
                    &op,
                    &systems,
                    config,
                    previous,
                    scaled.then_some(scale.as_slice()),
                );
                gate_identity &= identical;
                gate_alloc &= record["allocation_ok"].as_bool().unwrap();
                println!(
                    "{label}: identical {identical}, alloc ratio {:.3}",
                    record["allocations_per_solve"]["ratio"].as_f64().unwrap()
                );
                families.push(record);
            }
        }
    }

    // Item 4: the U-form driver with the switch off and on, and the
    // sequential MF step, adaptive GMRES at rtol 1e-6.
    let mut gate_driver_identity = true;
    let mut gate_driver_alloc = true;
    let mut runs = Vec::new();
    for case in cases() {
        let span = case.t_span.1 - case.t_span.0;
        let adapt = adaptive(1.0e-6, case.atol_scale, span);
        let linear = linear_config(LinearMethod::Gmres, 1.0e-11);
        let schedule = OutputSchedule::new(vec![case.t_span.0, case.t_span.1]).unwrap();
        let (off, off_alloc) = allocations_during(|| {
            integrate_rodas5p_mf_fast_observed(
                &case.problem,
                case.t_span,
                &case.y0,
                &linear,
                &adapt,
                &schedule,
            )
            .unwrap()
        });
        let (on, on_alloc) = allocations_during(|| {
            integrate_rodas5p_mf_fast_observed_gmres_into(
                &case.problem,
                case.t_span,
                &case.y0,
                &linear,
                &adapt,
                &schedule,
            )
            .unwrap()
        });
        let (seq, seq_alloc) = allocations_during(|| {
            sequential_adaptive(&case.problem, case.t_span, &case.y0, &linear, &adapt)
        });
        let same = off.observed.success
            && on.observed.success
            && bits(off.observed.y.last().unwrap()) == bits(on.observed.y.last().unwrap())
            && off.observed.t == on.observed.t
            && (off.attempts, off.accepted_steps, off.rejected_steps)
                == (on.attempts, on.accepted_steps, on.rejected_steps)
            && off.observed.counters == on.observed.counters;
        gate_driver_identity &= same;
        let per = |alloc: usize, attempts: usize| alloc as f64 / attempts.max(1) as f64;
        let (off_per, on_per, seq_per) = (
            per(off_alloc, off.attempts),
            per(on_alloc, on.attempts),
            per(seq_alloc, seq.attempts),
        );
        let alloc_ok = on_per <= 0.5 * seq_per;
        gate_driver_alloc &= alloc_ok;
        println!(
            "{}: driver identical {same}, allocations per attempt off {off_per:.1} on {on_per:.1} sequential {seq_per:.1}",
            case.id
        );
        runs.push(json!({
            "problem": case.id, "identical": same, "attempts": on.attempts,
            "accepted": on.accepted_steps, "rejected": on.rejected_steps,
            "allocations_per_attempt": {"switch_off": off_per, "switch_on": on_per, "sequential": seq_per,
                "on_over_sequential": on_per / seq_per, "off_over_sequential": off_per / seq_per},
            "linear_iterations_per_attempt": on.observed.counters.linear_iterations as f64 / on.attempts.max(1) as f64,
            "allocation_ok": alloc_ok,
        }));
    }

    let gate = json!({
        "1_bitwise_identity": gate_identity,
        "2_capacity_contract": "rodas5p-krylov tests/rnext02_gmres_into_contracts.rs (recorded separately)",
        "3_solver_allocations": gate_alloc,
        "4_driver_identity": gate_driver_identity,
        "4_driver_allocations": gate_driver_alloc,
    });
    let report = json!({
        "schema": "vigilode-rnext02-gmres-into-v1",
        "node": "research/rnext02_gmres_into_20261003",
        "excluded_states": excluded,
        "families": families,
        "driver_runs": runs,
        "gate": gate,
    });
    write_output("RNEXT02_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
}
