//! One least-squares solve per cycle in the augmented Arnoldi of LGMRES-into
//! (research node `research/spd05_lgmres_ls_once_20261007`, speed research
//! node SPD05). Run with `--ignored --release --test-threads=1`: the
//! counting allocator is global to this binary.
//!
//! The rev04 fixture set unchanged (6 CDR-120 sequences, the Brusselator-50
//! trajectory set, both also in the forced-failure configuration) with the
//! arms legacy `solve_lgmres_with_workspace_and_residual_scale` (reference),
//! `solve_lgmres_into` default (control), `solve_lgmres_into` with
//! `set_least_squares_once(true)` (candidate) and, reported only, the
//! candidate with SPD04's least-squares workspace. Each arm carries its own
//! recycle state. The frozen `rev04_lgmres_into.rs` is not edited.

mod rev01_common;

use std::collections::BTreeMap;

use rev01_common::{
    Sequence,
    common::{Counting, allocations_during, brusselator, write_output},
    fresh, trajectory_and_one_step,
};
use rodas5p_core::{CoreResult, IdentityPreconditioner, LinearSolveReport, WorkCounters};
use rodas5p_krylov::{
    LeastSquaresWorkspace, LgmresConfig, LgmresIntoReport, LgmresIntoWorkspace, LgmresState,
    LgmresWorkspace, solve_lgmres_into, solve_lgmres_with_workspace_and_residual_scale,
};
use serde_json::{Value, json};

#[global_allocator]
static GLOBAL: Counting = Counting;

const SENTINEL: u64 = 0x7ff8_dead_beef_0001;

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// The failure kind of a legacy error: the intermediate least-squares
/// exception of the preregistration, the final residual rule, or other.
fn failure_kind(error: &str) -> &'static str {
    if error.contains("least-squares solve produced NaN/Inf") {
        "least_squares_nonfinite"
    } else if error.contains("true residual") {
        "true_residual_exceeds_threshold"
    } else {
        "other"
    }
}

/// One `solve_lgmres_into` arm: workspace, carried state, totals.
struct IntoArm {
    workspace: LgmresIntoWorkspace,
    state: LgmresState,
    allocations: usize,
    output_kept_on_failure: bool,
    rolled_back: bool,
    least_squares_solves: u64,
    inner_iterations: u64,
    reports: usize,
}

impl IntoArm {
    fn new(once: bool, ls_workspace: bool) -> Self {
        let mut workspace = LgmresIntoWorkspace::default();
        workspace.set_least_squares_once(once);
        workspace.set_ls_workspace(ls_workspace);
        Self {
            workspace,
            state: LgmresState::default(),
            allocations: 0,
            output_kept_on_failure: true,
            rolled_back: true,
            least_squares_solves: 0,
            inner_iterations: 0,
            reports: 0,
        }
    }

    /// One solve; returns (result, output, state before, counters,
    /// allocations, least-squares-workspace solves during the call).
    #[allow(clippy::type_complexity)]
    fn solve(
        &mut self,
        op: &dyn rodas5p_core::LinearOperator,
        pc: &IdentityPreconditioner,
        b: &[f64],
        config: &LgmresConfig,
        measured: bool,
    ) -> (
        CoreResult<LgmresIntoReport>,
        Vec<f64>,
        LgmresState,
        WorkCounters,
        u64,
    ) {
        let before = self.state.clone();
        let ls_before = self
            .workspace
            .ls_workspace()
            .map_or(0, LeastSquaresWorkspace::solves);
        let mut output = vec![f64::from_bits(SENTINEL); b.len()];
        let mut counters = WorkCounters::default();
        let (result, allocations) = allocations_during(|| {
            solve_lgmres_into(
                op,
                pc,
                b,
                None,
                config,
                &mut self.state,
                None,
                &mut output,
                &mut self.workspace,
                &mut counters,
            )
        });
        if measured {
            self.allocations += allocations;
        }
        match &result {
            Ok(report) => {
                self.least_squares_solves += report.least_squares_solves;
                self.inner_iterations += report.inner_iterations;
                self.reports += 1;
            }
            Err(_) => {
                self.output_kept_on_failure &= output.iter().all(|v| v.to_bits() == SENTINEL);
                self.rolled_back &= self.state == before;
            }
        }
        let ls_after = self
            .workspace
            .ls_workspace()
            .map_or(0, LeastSquaresWorkspace::solves);
        (result, output, before, counters, ls_after - ls_before)
    }
}

/// Bitwise agreement of an into arm with the legacy solve: outcome
/// (solution bits, residual norm, iterations, matvecs or error text),
/// counters, carried state before and after.
#[allow(clippy::too_many_arguments)]
fn agrees(
    legacy: &CoreResult<LinearSolveReport>,
    legacy_counters: &WorkCounters,
    legacy_before: &LgmresState,
    legacy_after: &LgmresState,
    result: &CoreResult<LgmresIntoReport>,
    output: &[f64],
    counters: &WorkCounters,
    before: &LgmresState,
    after: &LgmresState,
) -> bool {
    let outcome = match (legacy, result) {
        (Ok(l), Ok(r)) => {
            bits(&l.x) == bits(output)
                && l.residual_norm.to_bits() == r.residual_norm.to_bits()
                && l.iterations == r.iterations
                && l.matvecs == r.matvecs
        }
        (Err(l), Err(r)) => l.to_string() == r.to_string(),
        _ => false,
    };
    outcome && legacy_counters == counters && legacy_before == before && legacy_after == after
}

fn run(set: &str, seq: &Sequence, config: &LgmresConfig) -> Value {
    let mut legacy_state = LgmresState::default();
    let mut legacy_ws = LgmresWorkspace::default();
    let mut legacy_alloc = 0usize;
    let mut control = IntoArm::new(false, false);
    let mut candidate = IntoArm::new(true, false);
    let mut candidate_ls = IntoArm::new(true, true);
    let (mut solves, mut measured, mut failures) = (0usize, 0usize, 0usize);
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut exceptions = Vec::new();
    let mut mismatches = Vec::new();
    let (mut id_candidate, mut id_control, mut id_candidate_ls) = (true, true, true);
    let (mut columns, mut cycles) = (0u64, 0u64);
    for (group, (op, rhs, group_label)) in seq.groups.iter().enumerate() {
        let pc = IdentityPreconditioner::new(op.dimension());
        for (index, b) in rhs.iter().enumerate() {
            let is_measured = solves > 0;
            let legacy_before = legacy_state.clone();
            let mut legacy_counters = WorkCounters::default();
            let (legacy, a_legacy) = allocations_during(|| {
                solve_lgmres_with_workspace_and_residual_scale(
                    op.as_ref(),
                    &pc,
                    b,
                    None,
                    config,
                    &mut legacy_state,
                    None,
                    &mut legacy_ws,
                    &mut legacy_counters,
                )
            });
            if is_measured {
                legacy_alloc += a_legacy;
                measured += 1;
            }
            solves += 1;
            // Arnoldi columns of this solve (counted on failure too).
            columns += legacy_counters.linear_iterations;
            let (r_control, o_control, b_control, c_control, _) =
                control.solve(op.as_ref(), &pc, b, config, is_measured);
            let (r_candidate, o_candidate, b_candidate, c_candidate, _) =
                candidate.solve(op.as_ref(), &pc, b, config, is_measured);
            let (r_cls, o_cls, b_cls, c_cls, ls_cycles) =
                candidate_ls.solve(op.as_ref(), &pc, b, config, is_measured);
            cycles += ls_cycles;
            let check = |r: &CoreResult<LgmresIntoReport>,
                         o: &[f64],
                         c: &WorkCounters,
                         before: &LgmresState,
                         after: &LgmresState| {
                agrees(
                    &legacy,
                    &legacy_counters,
                    &legacy_before,
                    &legacy_state,
                    r,
                    o,
                    c,
                    before,
                    after,
                )
            };
            let same_control = check(
                &r_control,
                &o_control,
                &c_control,
                &b_control,
                &control.state,
            );
            let same_candidate = check(
                &r_candidate,
                &o_candidate,
                &c_candidate,
                &b_candidate,
                &candidate.state,
            );
            let same_cls = check(&r_cls, &o_cls, &c_cls, &b_cls, &candidate_ls.state);
            // Whether this solve is the preregistered exception (the legacy
            // path aborted on an intermediate non-finite least-squares
            // solution); it is then listed instead of compared.
            let mut exception = false;
            if let Err(error) = &legacy {
                failures += 1;
                let text = error.to_string();
                let kind = failure_kind(&text);
                *kinds.entry(kind).or_default() += 1;
                if kind == "least_squares_nonfinite" {
                    exception = true;
                    // The enumerated deviation: the candidate must then
                    // succeed or fail by the final residual rule.
                    let candidate_ok = match &r_candidate {
                        Ok(_) => true,
                        Err(e) => failure_kind(&e.to_string()) == "true_residual_exceeds_threshold",
                    };
                    exceptions.push(json!({
                        "group": group, "group_label": group_label, "index": index,
                        "legacy_error": text,
                        "candidate": match &r_candidate { Ok(_) => "success".to_string(), Err(e) => e.to_string() },
                        "candidate_by_final_rule": candidate_ok,
                    }));
                }
            }
            if !same_candidate && !exception && mismatches.len() < 10 {
                mismatches.push(json!({"group": group, "index": index, "arm": "candidate",
                    "legacy": legacy.as_ref().err().map(ToString::to_string),
                    "candidate": r_candidate.as_ref().err().map(ToString::to_string)}));
            }
            id_control &= same_control;
            id_candidate &= same_candidate || exception;
            id_candidate_ls &= same_cls || exception;
        }
    }
    let per = |a: usize| {
        if measured == 0 {
            0.0
        } else {
            a as f64 / measured as f64
        }
    };
    let ratio = |a: usize| {
        if control.allocations == 0 {
            0.0
        } else {
            a as f64 / control.allocations as f64
        }
    };
    let v = json!({
        "set": set, "sequence": seq.label, "solves": solves, "measured_solves": measured,
        "failures": failures, "legacy_failure_kinds": kinds, "exceptions": exceptions,
        // Identity over every solve except the listed exceptions.
        "identity": {"candidate_vs_legacy": id_candidate, "control_vs_legacy": id_control,
            "candidate_ls_workspace_vs_legacy": id_candidate_ls},
        "mismatches": mismatches,
        "output_kept_on_failure": {"control": control.output_kept_on_failure,
            "candidate": candidate.output_kept_on_failure,
            "candidate_ls_workspace": candidate_ls.output_kept_on_failure},
        "rolled_back": {"control": control.rolled_back, "candidate": candidate.rolled_back,
            "candidate_ls_workspace": candidate_ls.rolled_back},
        "allocations": {"legacy": legacy_alloc, "control": control.allocations,
            "candidate": candidate.allocations, "candidate_ls_workspace": candidate_ls.allocations},
        "allocations_per_solve": {"legacy": per(legacy_alloc), "control": per(control.allocations),
            "candidate": per(candidate.allocations),
            "candidate_ls_workspace": per(candidate_ls.allocations),
            "candidate_over_control": ratio(candidate.allocations),
            "candidate_ls_workspace_over_control": ratio(candidate_ls.allocations)},
        // Per solve over all solves (columns from the legacy counters, which
        // count failed solves too; cycles from the least-squares workspace
        // of the candidate_ls arm, one solve per cycle).
        "per_solve": {"arnoldi_columns": columns as f64 / solves as f64,
            "cycles": cycles as f64 / solves as f64},
        // Totals of the successful solves' reports.
        "reported": {
            "control": {"reports": control.reports, "least_squares_solves": control.least_squares_solves,
                "inner_iterations": control.inner_iterations},
            "candidate": {"reports": candidate.reports, "least_squares_solves": candidate.least_squares_solves,
                "inner_iterations": candidate.inner_iterations},
        },
    });
    println!(
        "{set} {}: solves {solves} failures {failures} identity {} allocations/solve control {:.2} candidate {:.2} candidate+ls {:.2}",
        seq.label,
        v["identity"],
        per(control.allocations),
        per(candidate.allocations),
        per(candidate_ls.allocations)
    );
    v
}

#[test]
#[ignore = "research run of research/spd05_lgmres_ls_once_20261007; release build, one test thread"]
fn spd05_lgmres_ls_once() {
    let (p, y0) = brusselator(50).unwrap();
    let problem = p.jvp_only_clone().unwrap();
    let (brusselator_sets, _) = trajectory_and_one_step(&problem, &y0, 50);
    let trajectory: Vec<Sequence> = brusselator_sets
        .into_iter()
        .filter(|s| s.set == "trajectory")
        .collect();
    let cdr = fresh();
    let mut records = Vec::new();
    for (name, sequences, failing) in [
        ("cdr", &cdr, false),
        ("brusselator-50-trajectory", &trajectory, false),
        ("cdr-failing", &cdr, true),
        ("brusselator-50-trajectory-failing", &trajectory, true),
    ] {
        for seq in sequences.iter() {
            // The rev04 configurations.
            let config = LgmresConfig {
                rtol: if failing { 1.0e-14 } else { seq.rtol },
                max_outer: if failing {
                    1
                } else {
                    LgmresConfig::default().max_outer
                },
                ..LgmresConfig::default()
            };
            let mut record = run(name, seq, &config);
            record["failing_configuration"] = json!(failing);
            records.push(record);
        }
    }
    let solves: u64 = records.iter().map(|r| r["solves"].as_u64().unwrap()).sum();
    let failures: u64 = records
        .iter()
        .map(|r| r["failures"].as_u64().unwrap())
        .sum();
    let defaults = LgmresConfig::default();
    let report = json!({
        "schema": "vigilode-spd05-lgmres-ls-once-v1",
        "node": "research/spd05_lgmres_ls_once_20261007",
        "config": {"inner_m": defaults.inner_m, "outer_k": defaults.outer_k,
            "max_outer": defaults.max_outer},
        "counts": {"sequences": records.len(), "solves": solves, "failures": failures},
        "records": records,
    });
    write_output("SPD05_OUTPUT", &report);
    println!(
        "{}",
        serde_json::to_string_pretty(&report["counts"]).unwrap()
    );
}
