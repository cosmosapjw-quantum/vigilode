//! Allocation-free small least squares for GMRES-into (research node
//! `research/spd04_ls_workspace_20261007`, speed research node SPD04). Run
//! with `--ignored --release --test-threads=1`: the counting allocator is
//! global to this binary.
//!
//! Three arms on the 56 frozen `rnext02` families (the legacy allocating
//! solve as the bitwise reference, `solve_gmres_into` as of the base as the
//! control, `solve_gmres_into` with `GmresWorkspace::with_ls_workspace()` as
//! the candidate), the six L-0038 problems with the matrix-free U-form driver
//! and `gmres_into` on (control) and with the least-squares workspace
//! (candidate), and a replication of the contract systems of
//! `crates/rodas5p-krylov/tests/spd04_ls_workspace.rs` so the recorded file
//! carries every gate item. The frozen `rnext02_gmres_into_study.rs` is not
//! edited; its fixtures come from `rnext_common`.

#[path = "rnext_common/mod.rs"]
mod common;

use common::{
    Counting, adaptive, allocations_during, cases, convection_diffusion, linear_config,
    splitmix_vector, stage_operator, stage_right_hand_sides, write_output,
};
use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, IdentityPreconditioner, LinearMethod, LinearOperator,
    WorkCounters,
};
use rodas5p_integrators::{
    OutputSchedule, Rodas5pMfFastResult, Rodas5pMfFastWorkspace,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace,
};
use rodas5p_krylov::{
    GmresCapacity, GmresConfig, GmresIntoReport, GmresWorkspace, LeastSquaresWorkspace,
    least_squares, solve_gmres_into, solve_gmres_with_workspace_and_residual_scale,
};
use serde_json::{Value, json};

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Allocations of one `small::least_squares` call (`small.rs`): the faer
/// copy, the right-hand side, `ColPivQr::new`'s owned factor, two
/// permutation vectors, `Q_coeff`, the factor scratch, the split triangle,
/// `solve_lstsq`'s output and scratch, the output `Vec`.
const ALLOCATIONS_PER_LEAST_SQUARES: usize = 11;

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

/// The report fields that must agree between control and candidate (all
/// but `workspace_grew`, which is reported separately).
fn report_key(r: &GmresIntoReport) -> (u64, u64, u64, u64, u64, u64, u64, &'static str) {
    (
        r.residual_norm.to_bits(),
        r.relative_residual.to_bits(),
        r.iterations,
        r.matvecs,
        r.preconditioner_apps,
        r.cycles,
        r.least_squares_solves,
        r.method,
    )
}

fn ls_solves(ws: &GmresWorkspace) -> u64 {
    ws.ls_workspace().map_or(0, LeastSquaresWorkspace::solves)
}

fn ls_growths(ws: &GmresWorkspace) -> u64 {
    ws.ls_workspace()
        .map_or(0, LeastSquaresWorkspace::growth_events)
}

/// One family: a sequence of right-hand sides on one operator, one
/// configuration, one initial-guess mode, one residual scale. Each arm
/// carries its own previous solution as the next warm start.
fn family(
    label: &str,
    op: &dyn LinearOperator,
    systems: &[Vec<f64>],
    config: &GmresConfig,
    previous_x0: bool,
    scale: Option<&[f64]>,
) -> Value {
    let n = op.dimension();
    let pc = IdentityPreconditioner::new(n);
    let mut legacy_ws = GmresWorkspace::default();
    let mut control_ws = GmresWorkspace::default();
    let mut candidate_ws = GmresWorkspace::with_ls_workspace();
    let mut output = vec![0.0; n];
    // Warm-up on the first system (not counted), as rnext02.
    let _ = solve_gmres_with_workspace_and_residual_scale(
        op,
        &pc,
        &systems[0],
        None,
        config,
        scale,
        &mut legacy_ws,
        &mut WorkCounters::default(),
    );
    for ws in [&mut control_ws, &mut candidate_ws] {
        let _ = solve_gmres_into(
            op,
            &pc,
            &systems[0],
            None,
            config,
            scale,
            &mut output,
            ws,
            GmresCapacity::unbounded(),
            &mut WorkCounters::default(),
        );
    }
    let sentinel = vec![f64::from_bits(0x7ff8_dead_beef_0001); n];
    let (mut vs_legacy, mut vs_control, mut control_vs_legacy) = (true, true, true);
    let (mut a_legacy, mut a_control, mut a_candidate) = (0usize, 0usize, 0usize);
    let (mut solves, mut failures) = (0usize, 0usize);
    let (mut cycles, mut iterations, mut ls_total, mut ls_control) = (0u64, 0u64, 0u64, 0u64);
    let mut errors = Vec::new();
    let mut per_solve = Vec::new();
    let mut previous: [Option<Vec<f64>>; 3] = [None, None, None];
    for (index, b) in systems.iter().enumerate() {
        let x0 = |k: usize| {
            if previous_x0 {
                previous[k].clone()
            } else {
                None
            }
        };
        let (x0_legacy, x0_control, x0_candidate) = (x0(0), x0(1), x0(2));
        let mut c_legacy = WorkCounters::default();
        let (legacy, alloc_legacy) = allocations_during(|| {
            solve_gmres_with_workspace_and_residual_scale(
                op,
                &pc,
                b,
                x0_legacy.as_deref(),
                config,
                scale,
                &mut legacy_ws,
                &mut c_legacy,
            )
        });
        let mut out_control = sentinel.clone();
        let mut c_control = WorkCounters::default();
        let capacity_control = control_ws.capacity_f64();
        let (control, alloc_control) = allocations_during(|| {
            solve_gmres_into(
                op,
                &pc,
                b,
                x0_control.as_deref(),
                config,
                scale,
                &mut out_control,
                &mut control_ws,
                GmresCapacity::unbounded(),
                &mut c_control,
            )
        });
        let control_grew = control_ws.capacity_f64() > capacity_control;
        let mut out_candidate = sentinel.clone();
        let mut c_candidate = WorkCounters::default();
        let capacity_candidate = candidate_ws.capacity_f64();
        let (ls_before, growth_before) = (ls_solves(&candidate_ws), ls_growths(&candidate_ws));
        let (candidate, alloc_candidate) = allocations_during(|| {
            solve_gmres_into(
                op,
                &pc,
                b,
                x0_candidate.as_deref(),
                config,
                scale,
                &mut out_candidate,
                &mut candidate_ws,
                GmresCapacity::unbounded(),
                &mut c_candidate,
            )
        });
        let candidate_grew = candidate_ws.capacity_f64() > capacity_candidate;
        let ls_this = ls_solves(&candidate_ws) - ls_before;
        let ls_grew = ls_growths(&candidate_ws) > growth_before;
        solves += 1;
        a_legacy += alloc_legacy;
        a_control += alloc_control;
        a_candidate += alloc_candidate;
        ls_total += ls_this;

        let counters_ok = c_candidate == c_legacy && c_candidate == c_control;
        control_vs_legacy &= c_control == c_legacy;
        let (same_legacy, same_control, same_control_legacy) = match (&legacy, &control, &candidate)
        {
            (Ok(l), Ok(c), Ok(k)) => {
                iterations += k.iterations;
                cycles += k.cycles;
                ls_control += c.least_squares_solves;
                let legacy_ok = |out: &[f64], r: &GmresIntoReport| {
                    bits(out) == bits(&l.x)
                        && r.residual_norm.to_bits() == l.residual_norm.to_bits()
                        && r.iterations == l.iterations
                        && r.matvecs == l.matvecs
                };
                (
                    legacy_ok(&out_candidate, k),
                    bits(&out_candidate) == bits(&out_control)
                        && report_key(k) == report_key(c)
                        && k.least_squares_solves == ls_this,
                    legacy_ok(&out_control, c),
                )
            }
            (Err(l), Err(c), Err(k)) => {
                failures += 1;
                errors.push(l.to_string());
                let kept = bits(&out_candidate) == bits(&sentinel);
                (
                    k.to_string() == l.to_string() && kept,
                    k.to_string() == c.to_string() && kept,
                    c.to_string() == l.to_string() && bits(&out_control) == bits(&sentinel),
                )
            }
            (l, c, k) => {
                errors.push(format!(
                    "outcomes differ: legacy {:?} control {:?} candidate {:?}",
                    l.as_ref().err().map(ToString::to_string),
                    c.as_ref().err().map(ToString::to_string),
                    k.as_ref().err().map(ToString::to_string)
                ));
                (false, false, false)
            }
        };
        vs_legacy &= same_legacy && counters_ok;
        vs_control &= same_control && counters_ok;
        control_vs_legacy &= same_control_legacy;
        // The carried warm start of each arm.
        previous = [
            legacy.ok().map(|r| r.x),
            control.is_ok().then(|| out_control.clone()),
            candidate.is_ok().then(|| out_candidate.clone()),
        ];
        // Carried state: the next warm starts agree bitwise.
        let carried = previous[2].as_deref().map(bits) == previous[0].as_deref().map(bits)
            && previous[2].as_deref().map(bits) == previous[1].as_deref().map(bits);
        vs_legacy &= carried;
        vs_control &= carried;
        per_solve.push(json!({
            "index": index,
            "allocations": {"legacy": alloc_legacy, "control": alloc_control, "candidate": alloc_candidate},
            "least_squares_solves": ls_this,
            "growth": {"control_workspace": control_grew, "candidate_workspace": candidate_grew,
                "least_squares_workspace": ls_grew},
        }));
    }
    let per = |a: usize| a as f64 / solves as f64;
    let any_growth = per_solve.iter().any(|s| {
        s["growth"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| v.as_bool().unwrap())
    });
    let expected = a_control as i64 - (ALLOCATIONS_PER_LEAST_SQUARES as u64 * ls_total) as i64;
    json!({
        "family": label, "dimension": n, "solves": solves, "failures": failures,
        "errors": errors,
        "identity": {"candidate_vs_legacy": vs_legacy, "candidate_vs_control": vs_control,
            "control_vs_legacy": control_vs_legacy},
        "iterations": iterations, "cycles": cycles,
        "least_squares_solves": {"candidate_workspace": ls_total, "control_reported": ls_control},
        "allocations": {"legacy": a_legacy, "control": a_control, "candidate": a_candidate,
            "control_minus_11_per_least_squares": expected},
        "allocations_per_solve": {"legacy": per(a_legacy), "control": per(a_control),
            "candidate": per(a_candidate)},
        "exact_removal": a_candidate as i64 == expected,
        "growth_reported": any_growth,
        "per_solve": per_solve,
    })
}

// ---- Contract replication (the systems of the krylov contract test). ----

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        2.0 * ((z >> 11) as f64 / (1u64 << 53) as f64) - 1.0
    }
}

/// Kinds 0..8: generic, repeated column, zero column, near breakdown, exact
/// breakdown, decaying subdiagonal, tiny entries, huge entries (as the
/// contract test).
fn contract_system(m: usize, kind: usize, seed: u64, gmres_rhs: bool) -> (DenseMatrix, Vec<f64>) {
    let mut rng = Rng(seed);
    let mut a = DenseMatrix::zeros(m + 1, m);
    for j in 0..m {
        for i in 0..=(j + 1) {
            a[(i, j)] = rng.next();
        }
        a[(j + 1, j)] = a[(j + 1, j)].abs() + 0.1;
    }
    match kind {
        1 if m >= 2 => {
            let (from, to) = (m / 3, m - 1);
            for i in 0..=m {
                a[(i, to)] = if i <= from + 1 { a[(i, from)] } else { 0.0 };
            }
        }
        2 => {
            let column = (seed as usize) % m;
            for i in 0..=m {
                a[(i, column)] = 0.0;
            }
        }
        3 => a[(m, m - 1)] = 1.0e-15 * (1.0 + rng.next().abs()),
        4 => a[(m, m - 1)] = 0.0,
        5 => {
            for j in 0..m {
                a[(j + 1, j)] *= 0.5_f64.powi(j as i32);
            }
        }
        6 | 7 => {
            let scale = if kind == 6 { 1.0e-150 } else { 1.0e150 };
            for j in 0..m {
                for i in 0..=m {
                    a[(i, j)] *= scale;
                }
            }
        }
        _ => {}
    }
    let b = if gmres_rhs {
        let mut b = vec![0.0; m + 1];
        b[0] = 1.0 + rng.next().abs();
        b
    } else {
        (0..=m).map(|_| rng.next()).collect()
    };
    (a, b)
}

fn ls_outcome(result: CoreResult<Vec<f64>>) -> Result<Vec<u64>, String> {
    result.map(|x| bits(&x)).map_err(|e| e.to_string())
}

fn ws_outcome(
    ws: &mut LeastSquaresWorkspace,
    a: &DenseMatrix,
    b: &[f64],
    out: &mut Vec<f64>,
) -> Result<Vec<u64>, String> {
    ws.solve_into(a, b, out)
        .map(|()| bits(out))
        .map_err(|e| e.to_string())
}

fn contract() -> Value {
    let mut shared = LeastSquaresWorkspace::default();
    let mut shared_out = Vec::new();
    let (mut systems, mut identical, mut errors) = (0usize, true, 0usize);
    let mut first_mismatch = Value::Null;
    for k in 1..=64usize {
        let m = (k * 37) % 64 + 1;
        for kind in 0..8usize {
            for gmres_rhs in [true, false] {
                let seed = (m as u64) * 1_000 + (kind as u64) * 10 + u64::from(gmres_rhs);
                let (a, b) = contract_system(m, kind, seed, gmres_rhs);
                let expected = ls_outcome(least_squares(&a, &b));
                errors += usize::from(expected.is_err());
                let mut fresh = LeastSquaresWorkspace::default();
                let mut out = Vec::new();
                let same = expected == ws_outcome(&mut fresh, &a, &b, &mut out)
                    && expected == ws_outcome(&mut shared, &a, &b, &mut shared_out);
                if !same && first_mismatch.is_null() {
                    first_mismatch = json!({"m": m, "kind": kind, "seed": seed});
                }
                identical &= same;
                systems += 1;
            }
        }
    }
    // Reuse sequences, m + 1 in {8, 16, 24, 32, 40}.
    let sequences: [&[usize]; 5] = [
        &[7, 15, 23, 31, 39],
        &[39, 31, 23, 15, 7],
        &[15, 7, 39, 23, 7, 31, 39, 15],
        &[23, 23, 7, 7, 31, 15, 39, 39],
        &[7, 39, 7, 39, 15, 31, 23, 7],
    ];
    let (mut reuse_solves, mut growth_ok) = (0usize, true);
    for (s, sequence) in sequences.iter().enumerate() {
        let mut ws = LeastSquaresWorkspace::default();
        let mut out = Vec::new();
        let mut maximum = 0usize;
        for (step, &m) in sequence.iter().enumerate() {
            for kind in 0..8usize {
                let seed = 7_000_000 + (s as u64) * 10_000 + (step as u64) * 100 + kind as u64;
                let (a, b) = contract_system(m, kind, seed, kind % 2 == 0);
                let same =
                    ls_outcome(least_squares(&a, &b)) == ws_outcome(&mut ws, &a, &b, &mut out);
                if !same && first_mismatch.is_null() {
                    first_mismatch = json!({"sequence": s, "step": step, "m": m, "kind": kind});
                }
                identical &= same;
                growth_ok &= ws.last_solve_grew() == (m > maximum);
                maximum = maximum.max(m);
                reuse_solves += 1;
            }
        }
    }
    // Non-finite input: the same error text.
    let mut ws = LeastSquaresWorkspace::default();
    let mut out = Vec::new();
    let mut nonfinite = 0usize;
    let mut nonfinite_same = true;
    for m in [1usize, 4, 15, 39] {
        for (k, poison) in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY]
            .into_iter()
            .enumerate()
        {
            let (mut a, b) = contract_system(m, 0, 11 + k as u64, true);
            a[(m / 2, m / 2)] = poison;
            let expected = ls_outcome(least_squares(&a, &b));
            nonfinite_same &=
                expected.is_err() && expected == ws_outcome(&mut ws, &a, &b, &mut out);
            let (a, mut b) = contract_system(m, 0, 21 + k as u64, false);
            b[m] = poison;
            let expected = ls_outcome(least_squares(&a, &b));
            nonfinite_same &=
                expected.is_err() && expected == ws_outcome(&mut ws, &a, &b, &mut out);
            nonfinite += 2;
        }
    }
    json!({
        "systems": systems, "reference_errors": errors, "reuse_sequences": sequences.len(),
        "reuse_solves": reuse_solves, "nonfinite_inputs": nonfinite,
        "identical": identical && nonfinite_same, "growth_only_above_previous_maximum": growth_ok,
        "first_mismatch": first_mismatch,
        "registered_test": "cargo test --release -p rodas5p-krylov --locked --test spd04_ls_workspace",
    })
}

fn driver_key(r: &Rodas5pMfFastResult) -> Value {
    json!({
        "t_bits": bits(&r.observed.t),
        "y_bits": r.observed.y.iter().map(|y| bits(y)).collect::<Vec<_>>(),
        "success": r.observed.success,
        "attempts": r.attempts, "accepted": r.accepted_steps, "rejected": r.rejected_steps,
        "state_reuses": r.state_reuses,
        "counters": serde_json::to_value(r.observed.counters).unwrap(),
    })
}

#[test]
#[ignore = "research run of research/spd04_ls_workspace_20261007; release build, one test thread"]
fn spd04_ls_workspace_study() {
    let contract = contract();
    println!(
        "contract: {} systems + {} reuse solves, identical {}",
        contract["systems"], contract["reuse_solves"], contract["identical"]
    );

    let mut families = Vec::new();
    let mut excluded = Vec::new();
    let mut problems = Vec::new();
    for case in cases() {
        problems.push(case.id);
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
                    let record = family(&label, &op, &systems, config, previous, None);
                    println!(
                        "{label}: identity {} exact removal {} allocations/solve control {:.2} candidate {:.2}",
                        record["identity"],
                        record["exact_removal"],
                        record["allocations_per_solve"]["control"].as_f64().unwrap(),
                        record["allocations_per_solve"]["candidate"]
                            .as_f64()
                            .unwrap()
                    );
                    families.push(record);
                }
            }
        }
    }
    // Convection-diffusion, n = 200, Peclet 10, tau 1e-3, three right-hand
    // sides (as rnext02).
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
                let record = family(
                    &label,
                    &op,
                    &systems,
                    config,
                    previous,
                    scaled.then_some(scale.as_slice()),
                );
                println!(
                    "{label}: identity {} exact removal {} allocations/solve control {:.2} candidate {:.2}",
                    record["identity"],
                    record["exact_removal"],
                    record["allocations_per_solve"]["control"].as_f64().unwrap(),
                    record["allocations_per_solve"]["candidate"]
                        .as_f64()
                        .unwrap()
                );
                families.push(record);
            }
        }
    }

    // The six L-0038 problems: the U-form driver with gmres_into (control)
    // and with the least-squares workspace too (candidate), adaptive GMRES at
    // rtol 1e-6 (as rnext02 item 4).
    let mut runs = Vec::new();
    for case in cases() {
        let span = case.t_span.1 - case.t_span.0;
        let adapt = adaptive(1.0e-6, case.atol_scale, span);
        let linear = linear_config(LinearMethod::Gmres, 1.0e-11);
        let schedule = OutputSchedule::new(vec![case.t_span.0, case.t_span.1]).unwrap();
        let (control, control_alloc) = allocations_during(|| {
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
        let (candidate, candidate_alloc) = allocations_during(|| {
            integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace(
                &case.problem,
                case.t_span,
                &case.y0,
                &linear,
                &adapt,
                &schedule,
            )
            .unwrap()
        });
        let (control_key, candidate_key) = (driver_key(&control), driver_key(&candidate));
        let identical = control_key == candidate_key;
        let per = |alloc: usize, attempts: usize| alloc as f64 / attempts.max(1) as f64;
        let (control_per, candidate_per) = (
            per(control_alloc, control.attempts),
            per(candidate_alloc, candidate.attempts),
        );
        println!(
            "{}: driver identical {identical}, allocations per attempt control {control_per:.2} candidate {candidate_per:.2}",
            case.id
        );
        runs.push(json!({
            "problem": case.id, "identical": identical,
            "success": {"control": control.observed.success, "candidate": candidate.observed.success},
            "attempts": {"control": control.attempts, "candidate": candidate.attempts},
            "accepted": candidate.accepted_steps, "rejected": candidate.rejected_steps,
            "counters_equal": control.observed.counters == candidate.observed.counters,
            "final_state_bits_equal": control_key["y_bits"] == candidate_key["y_bits"],
            "allocations": {"control": control_alloc, "candidate": candidate_alloc},
            "allocations_per_attempt": {"control": control_per, "candidate": candidate_per,
                "ratio": candidate_per / control_per.max(f64::MIN_POSITIVE)},
            "linear_iterations_per_attempt":
                candidate.observed.counters.linear_iterations as f64 / candidate.attempts.max(1) as f64,
            "linear_solves": candidate.observed.counters.linear_solves,
        }));
    }

    let total_solves: u64 = families.iter().map(|f| f["solves"].as_u64().unwrap()).sum();
    let total_failures: u64 = families
        .iter()
        .map(|f| f["failures"].as_u64().unwrap())
        .sum();
    let report = json!({
        "schema": "vigilode-spd04-ls-workspace-v1",
        "node": "research/spd04_ls_workspace_20261007",
        "allocations_per_least_squares": ALLOCATIONS_PER_LEAST_SQUARES,
        "counts": {"families": families.len(), "solves": total_solves, "failures": total_failures,
            "driver_problems": problems, "excluded_states": excluded},
        "contract": contract,
        "families": families,
        "driver_runs": runs,
    });
    write_output("SPD04_OUTPUT", &report);
    println!(
        "{} families, {} solves, {} driver runs",
        report["counts"]["families"],
        report["counts"]["solves"],
        report["driver_runs"].as_array().unwrap().len()
    );
}
