//! Calibration of review DAG node REV-01c
//! (`research/rev01c_gcrodr_refreshed_update_20261003`): control C (recycled
//! GCRO-DR, no option) alone on the fresh Brusselator-160 set, to check that
//! the set can fail before any candidate is run on it. Run with
//! `--ignored --release --test-threads=1`.

mod rev01_common;

use rev01_common::{common::brusselator, gcrodr_config, trajectory_and_one_step};
use rodas5p_core::{IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    GcrodrReusePolicy, GcrodrState, GcrodrTrace, GcrodrWorkspace, solve_gcrodr_with_policy,
};

#[test]
#[ignore = "calibration of research/rev01c_gcrodr_refreshed_update_20261003; release build"]
fn fresh_brusselator_160_control_c_only() {
    let (p, y0) = brusselator(160).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let (sequences, info) = trajectory_and_one_step(&problem, &y0, 160);
    println!("{info}");
    for set in ["trajectory", "one_step"] {
        let (mut solves, mut failures) = (0, 0);
        for seq in sequences.iter().filter(|s| s.set == set) {
            let config = gcrodr_config(seq.rtol, seq.budget);
            let mut state = GcrodrState::default();
            let mut workspace = GcrodrWorkspace::default();
            for (op, rhs, _) in &seq.groups {
                let pc = IdentityPreconditioner::new(op.dimension());
                for b in rhs {
                    solves += 1;
                    let result = solve_gcrodr_with_policy(
                        op.as_ref(),
                        &pc,
                        b,
                        None,
                        &config,
                        &mut state,
                        None,
                        &mut workspace,
                        GcrodrReusePolicy::default(),
                        &mut GcrodrTrace::default(),
                        &mut WorkCounters::default(),
                    );
                    failures += usize::from(result.is_err());
                }
            }
        }
        println!("{set}: control C fails on {failures} of {solves} solves");
    }
}
