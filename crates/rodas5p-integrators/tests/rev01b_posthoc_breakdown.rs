//! Post-hoc diagnostic of research node
//! `research/rev01b_gcrodr_recycle_reorthogonalization_20261003` (after its
//! recorded run; not part of its gate): for control FG on the Brusselator-120
//! trajectory set, where `[C V]` loses orthogonality, compare `|C^T v|` when
//! each Arnoldi vector is created with `max |C^T V|` at the end of the cycle,
//! and record the smallest `||next|| / column scale`. Run with `--ignored
//! --release --test-threads=1`.

mod rev01_common;

use rev01_common::{common::brusselator, gcrodr_config, trajectory_and_one_step};
use rodas5p_core::{IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    GcrodrSolveOptions, GcrodrState, GcrodrTrace, GcrodrWorkspace, solve_gcrodr_with_options,
};
use serde_json::json;

#[test]
#[ignore = "post-hoc diagnostic of research/rev01b_gcrodr_recycle_reorthogonalization_20261003; release build"]
fn breakdown_diagnostic() {
    let (p, y0) = brusselator(120).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let (sequences, _) = trajectory_and_one_step(&problem, &y0, 120);
    let mut cycles = Vec::new();
    for seq in sequences.iter().filter(|s| s.set == "trajectory") {
        let config = gcrodr_config(seq.rtol, seq.budget);
        let mut state = GcrodrState::default();
        let mut workspace = GcrodrWorkspace::default();
        for (op, rhs, group) in &seq.groups {
            let pc = IdentityPreconditioner::new(op.dimension());
            for (i, b) in rhs.iter().enumerate() {
                let mut trace = GcrodrTrace::default();
                let result = solve_gcrodr_with_options(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    &config,
                    &mut state,
                    None,
                    &mut workspace,
                    GcrodrSolveOptions {
                        orthogonalize_start: true,
                        reorthogonalize_recycle: true,
                        ..GcrodrSolveOptions::default()
                    },
                    &mut trace,
                    &mut WorkCounters::default(),
                );
                for c in trace
                    .cycles
                    .iter()
                    .filter(|c| !c.aborted && c.recycle_rank > 0)
                {
                    cycles.push(json!({
                        "group": group, "rhs": i, "ok": result.is_ok(), "rank": c.recycle_rank,
                        "end_ctv": c.recycle_arnoldi_coupling, "creation_ctv": c.creation_ctv_max,
                        "min_next_ratio": c.min_next_ratio, "CtC_defect": c.recycle_gram_defect,
                        "columns": c.arnoldi_columns,
                    }));
                }
            }
        }
    }
    let bad: Vec<_> = cycles
        .iter()
        .filter(|c| c["end_ctv"].as_f64().unwrap() > 1e-8)
        .collect();
    let bad_creation = bad
        .iter()
        .filter(|c| c["creation_ctv"].as_f64().unwrap() > 1e-8)
        .count();
    let summary = json!({
        "cycles_with_recycle": cycles.len(),
        "cycles_end_ctv_gt_1e-8": bad.len(),
        "of_which_creation_ctv_gt_1e-8": bad_creation,
        "max_CtC_defect_in_bad": bad.iter().map(|c| c["CtC_defect"].as_f64().unwrap()).fold(0.0_f64, f64::max),
        "min_next_ratio_in_bad": bad.iter().map(|c| c["min_next_ratio"].as_f64().unwrap()).fold(f64::INFINITY, f64::min),
        "examples": bad.iter().take(6).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
    if let Ok(path) = std::env::var("REV01B_POSTHOC_OUTPUT") {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&json!({"summary": summary, "cycles": cycles})).unwrap()
                + "\n",
        )
        .unwrap();
    }
}
