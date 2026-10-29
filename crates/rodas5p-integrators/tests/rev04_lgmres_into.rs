//! LGMRES writing into caller storage against the existing LGMRES (research
//! node `research/rev04_lgmres_into_20261003`, review DAG node REV-04). Run
//! with `--ignored --release --test-threads=1`: the counting allocator is
//! global to this binary.

mod rev01_common;

use rev01_common::{
    Sequence,
    common::{Counting, allocations_during, brusselator, write_output},
    fresh, trajectory_and_one_step,
};
use rodas5p_core::{IdentityPreconditioner, WorkCounters};
use rodas5p_krylov::{
    LgmresConfig, LgmresIntoWorkspace, LgmresState, LgmresWorkspace, solve_lgmres_into,
    solve_lgmres_with_workspace_and_residual_scale,
};
use serde_json::{Value, json};

#[global_allocator]
static GLOBAL: Counting = Counting;

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

struct Outcome {
    identical: bool,
    output_kept_on_failure: bool,
    rolled_back: bool,
    solves: usize,
    failures: usize,
    allocations_old: usize,
    allocations_new: usize,
    measured: usize,
}

fn run(seq: &Sequence, config: &LgmresConfig) -> Outcome {
    let (mut old_state, mut new_state) = (LgmresState::default(), LgmresState::default());
    let mut old_ws = LgmresWorkspace::default();
    let mut new_ws = LgmresIntoWorkspace::default();
    let mut out = Outcome {
        identical: true,
        output_kept_on_failure: true,
        rolled_back: true,
        solves: 0,
        failures: 0,
        allocations_old: 0,
        allocations_new: 0,
        measured: 0,
    };
    for (op, rhs, _) in &seq.groups {
        let pc = IdentityPreconditioner::new(op.dimension());
        for b in rhs {
            let before_old = old_state.clone();
            let (mut c_old, mut c_new) = (WorkCounters::default(), WorkCounters::default());
            let (old, a_old) = allocations_during(|| {
                solve_lgmres_with_workspace_and_residual_scale(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    config,
                    &mut old_state,
                    None,
                    &mut old_ws,
                    &mut c_old,
                )
            });
            let mut output = vec![f64::from_bits(0x7ff8_dead_beef_0001); b.len()];
            let sentinel = bits(&output);
            let (new, a_new) = allocations_during(|| {
                solve_lgmres_into(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    config,
                    &mut new_state,
                    None,
                    &mut output,
                    &mut new_ws,
                    &mut c_new,
                )
            });
            out.solves += 1;
            if out.solves > 1 {
                out.allocations_old += a_old;
                out.allocations_new += a_new;
                out.measured += 1;
            }
            let same = match (&old, &new) {
                (Ok(o), Ok(n)) => {
                    bits(&o.x) == bits(&output)
                        && o.residual_norm.to_bits() == n.residual_norm.to_bits()
                        && o.iterations == n.iterations
                        && o.matvecs == n.matvecs
                }
                (Err(e1), Err(e2)) => {
                    out.failures += 1;
                    out.output_kept_on_failure &= bits(&output) == sentinel;
                    out.rolled_back &= old_state == before_old && new_state == before_old;
                    e1.to_string() == e2.to_string()
                }
                _ => false,
            };
            out.identical &= same && c_old == c_new && old_state == new_state;
        }
    }
    out
}

fn summary(name: &str, seq: &Sequence, o: &Outcome) -> Value {
    let per = |a: usize| {
        if o.measured == 0 {
            0.0
        } else {
            a as f64 / o.measured as f64
        }
    };
    let v = json!({
        "set": name, "sequence": seq.label, "solves": o.solves, "failures": o.failures,
        "identical": o.identical, "output_kept_on_failure": o.output_kept_on_failure,
        "rolled_back": o.rolled_back,
        "allocations_per_solve": {"old": per(o.allocations_old), "new": per(o.allocations_new),
            "ratio": if o.allocations_old == 0 { 0.0 } else { o.allocations_new as f64 / o.allocations_old as f64 }},
    });
    println!("{v}");
    v
}

#[test]
#[ignore = "research run of research/rev04_lgmres_into_20261003; release build"]
fn lgmres_into() {
    let (p, y0) = brusselator(50).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let (brusselator_sets, _) = trajectory_and_one_step(&problem, &y0, 50);
    let trajectory: Vec<Sequence> = brusselator_sets
        .into_iter()
        .filter(|s| s.set == "trajectory")
        .collect();
    let cdr = fresh();
    let mut records = Vec::new();
    let (mut identical, mut kept, mut rolled, mut fewer) = (true, true, true, true);
    let mut failures_seen = 0usize;
    for (name, sequences, failing) in [
        ("cdr", &cdr, false),
        ("brusselator-50-trajectory", &trajectory, false),
        ("cdr-failing", &cdr, true),
        ("brusselator-50-trajectory-failing", &trajectory, true),
    ] {
        for seq in sequences.iter() {
            let config = LgmresConfig {
                rtol: if failing { 1.0e-14 } else { seq.rtol },
                max_outer: if failing {
                    1
                } else {
                    LgmresConfig::default().max_outer
                },
                ..LgmresConfig::default()
            };
            let o = run(seq, &config);
            identical &= o.identical;
            kept &= o.output_kept_on_failure;
            rolled &= o.rolled_back;
            failures_seen += o.failures;
            if !failing {
                fewer &= o.allocations_new < o.allocations_old;
            }
            records.push(summary(name, seq, &o));
        }
    }
    let report = json!({
        "schema": "vigilode-rev04-lgmres-into-v1",
        "records": records,
        "gate": {
            "1_bitwise_identity": identical && rolled,
            "2_output_only_on_success": kept && failures_seen > 0,
            "3_fewer_allocations": fewer,
            "failures_seen": failures_seen,
        },
    });
    write_output("REV04_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
}
