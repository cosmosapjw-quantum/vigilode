//! The work–precision run: references, measured repetitions, rows and the
//! matched-error comparison. The output is one JSON document from which the
//! Markdown report is generated.

use std::time::Instant;

use serde::Serialize;
use serde_json::{Value, json};

use crate::alloc::{self, AllocCounts};
use crate::arms::{self, Arm, Fidelity, RunOutput, RunStatus, Work};
use crate::corpus::{Corpus, ProblemSpec};
use crate::metrics::{interpolate_log, max_wrms, max_wrms_prefix, median};
use crate::model::{self, CallbackCounts};

/// Output schema id.
pub const RESULTS_SCHEMA: &str = "vigilode-bench.wp.v1";

/// One measured repetition.
#[derive(Clone, Debug, Serialize)]
pub struct Rep {
    /// Wall time of the run: setup, integration, grid output in final form,
    /// teardown; statistics formatting excluded.
    pub wall_ns: u64,
    /// Allocations during the run.
    pub allocations: AllocCounts,
    /// Shared-model callback counts during the run.
    pub callbacks: CallbackCounts,
}

/// One problem × arm × tolerance row. Failed runs are rows.
#[derive(Clone, Debug, Serialize)]
pub struct Row {
    /// Problem id.
    pub problem: String,
    /// Arm id.
    pub arm: Arm,
    /// Fidelity tag.
    pub comparator_fidelity: Fidelity,
    /// Relative tolerance.
    pub rtol: f64,
    /// Scalar atol given to the solver.
    pub atol: f64,
    /// Status of the first measured repetition.
    pub status: RunStatus,
    /// Grid points produced.
    pub grid_points_completed: usize,
    /// Max-over-grid WRMS error with the run's own atol vector and rtol
    /// (plan §7 rule 2); `None` unless the full grid was produced.
    pub error_wrms_own: Option<f64>,
    /// Max-over-grid WRMS error with weights `error_rtol * (atol_scale_i +
    /// |y_ref_i|)` (tolerance-independent; the matched-error axis).
    pub error_common: Option<f64>,
    /// `error_common` over the produced prefix of a failed run.
    pub error_common_prefix: Option<f64>,
    /// Solver-reported work.
    pub work: Work,
    /// Shared-model callbacks of the first measured repetition.
    pub callbacks: CallbackCounts,
    /// Allocations of the first measured repetition.
    pub allocations: AllocCounts,
    /// Whether every repetition had the same allocation counts.
    pub allocations_stable: bool,
    /// Whether every repetition produced bit-identical states, status and callbacks.
    pub deterministic: bool,
    /// Median wall time over repetitions, ms (diagnostic only).
    pub wall_ms_median: f64,
    /// Every repetition.
    pub repetitions: Vec<Rep>,
    /// The solver's raw statistics.
    pub native: Value,
    /// Output interface.
    pub output_interface: &'static str,
    /// Deviations from vendor defaults.
    pub deviations: Vec<&'static str>,
}

/// Reference record of a problem.
#[derive(Clone, Debug, Serialize)]
pub struct Reference {
    /// Problem id.
    pub problem: String,
    /// Reference arm.
    pub arm: Arm,
    /// Its rtol.
    pub rtol: f64,
    /// Its atol.
    pub atol: f64,
    /// Status of the reference run.
    pub status: RunStatus,
    /// Decisive cross-check.
    pub crosscheck: CrossCheck,
    /// Informational cross-checks.
    pub informational: Vec<CrossCheck>,
    /// `true` when the decisive cross-check failed or disagreed beyond the threshold.
    pub disputed: bool,
    /// Reference states on the grid (the error baseline of every row).
    pub states: Vec<Vec<f64>>,
}

/// Agreement of one other code with the reference.
#[derive(Clone, Debug, Serialize)]
pub struct CrossCheck {
    /// Arm.
    pub arm: Arm,
    /// The tolerance actually used (`None` when every attempt failed).
    pub rtol_used: Option<f64>,
    /// The difference measured like `error_common` (weights
    /// `error_rtol * (atol_scale_i + |y_ref_i|)`).
    pub disagreement: Option<f64>,
    /// Tolerances attempted, with their status.
    pub attempts: Vec<(f64, RunStatus)>,
}

/// One matched-error comparison at a common error level.
#[derive(Clone, Debug, Serialize)]
pub struct Matched {
    /// Problem id.
    pub problem: String,
    /// Comparator arm.
    pub arm: Arm,
    /// Baseline (VigilODE) arm.
    pub baseline: Arm,
    /// `dense` (baseline: the facade with dense output and interior
    /// `WFilteredEnforce`, the headline) or `grid-clipped` (the v1 continuity
    /// baseline).
    pub basis: &'static str,
    /// The common error level (`error_common`).
    pub error_level: f64,
    /// `MATCHED` or `DIAGNOSTIC_ONLY`.
    pub status: &'static str,
    /// Why not matched (empty when matched).
    pub reason: String,
    /// Comparator median wall time interpolated at the level, ms.
    pub comparator_wall_ms: Option<f64>,
    /// Baseline median wall time interpolated at the level, ms.
    pub baseline_wall_ms: Option<f64>,
    /// `comparator_wall_ms / baseline_wall_ms` (above 1: VigilODE faster);
    /// only when `MATCHED`.
    pub time_ratio: Option<f64>,
    /// Comparator RHS callbacks interpolated at the level.
    pub comparator_rhs: Option<f64>,
    /// Baseline RHS callbacks interpolated at the level.
    pub baseline_rhs: Option<f64>,
    /// `comparator_rhs / baseline_rhs`; only when `MATCHED`.
    pub rhs_ratio: Option<f64>,
    /// Successful points on the comparator's curve.
    pub comparator_points: usize,
    /// Successful points on the baseline's curve.
    pub baseline_points: usize,
    /// Always `diagnostic_only` on this host.
    pub timing_authority: &'static str,
}

/// Interior-control evidence of one problem × rtol: the facade with
/// `WFilteredEnforce` against `Off` (same Jacobian route).
#[derive(Clone, Debug, Serialize)]
pub struct InteriorControl {
    /// Problem id.
    pub problem: String,
    /// Relative tolerance.
    pub rtol: f64,
    /// The `WFilteredEnforce` arm.
    pub enforce_arm: Arm,
    /// The `Off` arm.
    pub off_arm: Arm,
    /// `error_own` of the `WFilteredEnforce` arm.
    pub error_own_enforce: Option<f64>,
    /// `error_own` of the `Off` arm.
    pub error_own_off: Option<f64>,
    /// Accepted steps of the `WFilteredEnforce` arm.
    pub accepted_enforce: Option<u64>,
    /// Accepted steps of the `Off` arm.
    pub accepted_off: Option<u64>,
    /// Interior rejections of the `WFilteredEnforce` arm (as the route reported them).
    pub interior_rejections_enforce: Option<u64>,
    /// Interior rejections of the `Off` arm.
    pub interior_rejections_off: Option<u64>,
    /// Largest interior estimate of an accepted step of the `WFilteredEnforce` arm, tolerance units.
    pub max_estimate_enforce: Option<f64>,
    /// `error_own_off / error_own_enforce`.
    pub error_ratio: Option<f64>,
    /// `accepted_enforce / accepted_off`.
    pub step_ratio: Option<f64>,
    /// Whether `error_own_enforce` is at most [`INTERIOR_TOL_UNITS`].
    pub within_tol_units: Option<bool>,
}

/// The interior-control acceptance level in tolerance units (`error_own`).
pub const INTERIOR_TOL_UNITS: f64 = 2.0;

struct Measured {
    out: RunOutput,
    reps: Vec<Rep>,
    deterministic: bool,
}

fn bits(states: &[Vec<f64>]) -> Vec<u64> {
    states.iter().flatten().map(|x| x.to_bits()).collect()
}

fn once(arm: Arm, m: &model::Model, grid: &[f64], rtol: f64, atol: f64) -> (RunOutput, Rep) {
    model::reset_counts();
    let a0 = alloc::snapshot();
    let t0 = Instant::now();
    let out = arms::run(arm, m, grid, rtol, atol);
    let end = out.end.unwrap_or_else(alloc::Mark::now);
    let wall = end.at.duration_since(t0);
    let a1 = end.alloc;
    let callbacks = model::counts();
    (
        out,
        Rep {
            wall_ns: u64::try_from(wall.as_nanos()).unwrap_or(u64::MAX),
            allocations: a1.since(a0),
            callbacks,
        },
    )
}

/// Progress sink.
pub type Log<'a> = &'a mut dyn FnMut(&str);

fn reference_for(c: &Corpus, p: &ProblemSpec, grid: &[f64], log: Log) -> Result<Reference, String> {
    let m = p.model()?;
    let r = &c.reference;
    let max = Some(r.max_steps);
    let ref_out = arms::run_with(r.arm, &m, grid, r.rtol, p.reference_atol, max);
    log(&format!(
        "  reference {} rtol {:e}: {:?}",
        r.arm.id(),
        r.rtol,
        ref_out.status
    ));
    let ok = ref_out.status == RunStatus::Success;
    let states = ref_out.states.clone();
    let weights = p.atol_vector(c.error_rtol);
    let check = |arm: Arm, rtols: &[f64]| -> CrossCheck {
        let mut attempts = Vec::new();
        for &rt in rtols {
            let o = arms::run_with(arm, &m, grid, rt, p.reference_atol, max);
            attempts.push((rt, o.status.clone()));
            if o.status == RunStatus::Success {
                let d = if ok {
                    max_wrms(&o.states, &states, &weights, c.error_rtol)
                } else {
                    None
                };
                return CrossCheck {
                    arm,
                    rtol_used: Some(rt),
                    disagreement: d,
                    attempts,
                };
            }
        }
        CrossCheck {
            arm,
            rtol_used: None,
            disagreement: None,
            attempts,
        }
    };
    let crosscheck = check(r.crosscheck_arm, &r.crosscheck_rtols);
    log(&format!(
        "  cross-check {} rtol {:?}: disagreement {:?}",
        r.crosscheck_arm.id(),
        crosscheck.rtol_used,
        crosscheck.disagreement
    ));
    let informational: Vec<CrossCheck> = r
        .informational_arms
        .iter()
        .map(|&a| check(a, &[r.rtol]))
        .collect();
    let disputed = !ok
        || crosscheck
            .disagreement
            .is_none_or(|d| d.is_nan() || d > r.dispute_threshold);
    Ok(Reference {
        problem: p.id.clone(),
        arm: r.arm,
        rtol: r.rtol,
        atol: p.reference_atol,
        status: ref_out.status,
        crosscheck,
        informational,
        disputed,
        states,
    })
}

/// Run the whole corpus.
pub fn run_corpus(c: &Corpus, log: Log) -> Result<Value, String> {
    c.validate()?;
    let started = Instant::now();
    let mut references = Vec::new();
    let mut rows: Vec<Row> = Vec::new();
    for p in &c.problems {
        let m = p.model()?;
        let grid = c.grid(p);
        log(&format!("problem {}", p.id));
        let reference = reference_for(c, p, &grid, log)?;
        let arms_here = c.arms_for(p);
        for &rtol in &c.rtols {
            let atol = p.atol_scalar(rtol);
            let atol_vec = p.atol_vector(rtol);
            let common_atol = p.atol_vector(c.error_rtol);
            for &a in &arms_here {
                for _ in 0..c.warmups {
                    let _ = arms::run(a, &m, &grid, rtol, atol);
                }
            }
            let mut measured: Vec<Option<Measured>> = (0..arms_here.len()).map(|_| None).collect();
            // Blocks with a rotated arm order (block b starts at arm b mod k).
            for b in 0..c.repetitions {
                for s in 0..arms_here.len() {
                    let i = (b + s) % arms_here.len();
                    let (out, rep) = once(arms_here[i], &m, &grid, rtol, atol);
                    match &mut measured[i] {
                        None => {
                            measured[i] = Some(Measured {
                                out,
                                reps: vec![rep],
                                deterministic: true,
                            })
                        }
                        Some(ms) => {
                            let same = out.status == ms.out.status
                                && bits(&out.states) == bits(&ms.out.states)
                                && rep.callbacks == ms.reps[0].callbacks;
                            ms.deterministic &= same;
                            ms.reps.push(rep);
                        }
                    }
                }
            }
            for (i, ms) in measured.into_iter().enumerate() {
                let ms = ms.expect("every arm measured");
                let arm = arms_here[i];
                let full = ms.out.status == RunStatus::Success;
                let ref_ok = reference.status == RunStatus::Success;
                let (own, common) = if full && ref_ok {
                    (
                        max_wrms(&ms.out.states, &reference.states, &atol_vec, rtol),
                        max_wrms(
                            &ms.out.states,
                            &reference.states,
                            &common_atol,
                            c.error_rtol,
                        ),
                    )
                } else {
                    (None, None)
                };
                let prefix = if !full && ref_ok {
                    max_wrms_prefix(
                        &ms.out.states,
                        &reference.states,
                        &common_atol,
                        c.error_rtol,
                    )
                } else {
                    None
                };
                let walls: Vec<f64> = ms.reps.iter().map(|r| r.wall_ns as f64 / 1e6).collect();
                let first = ms.reps[0].clone();
                let row = Row {
                    problem: p.id.clone(),
                    arm,
                    comparator_fidelity: arm.fidelity(),
                    rtol,
                    atol,
                    status: ms.out.status.clone(),
                    grid_points_completed: ms.out.states.len(),
                    error_wrms_own: own,
                    error_common: common,
                    error_common_prefix: prefix,
                    work: ms.out.work.clone(),
                    callbacks: first.callbacks,
                    allocations: first.allocations,
                    allocations_stable: ms.reps.iter().all(|r| r.allocations == first.allocations),
                    deterministic: ms.deterministic,
                    wall_ms_median: median(&walls),
                    repetitions: ms.reps,
                    native: ms.out.native.clone(),
                    output_interface: arm.output_interface(),
                    deviations: arm.deviations(),
                };
                log(&format!(
                    "  {:<22} rtol {:e}: {:?} err {:?} {:.3} ms",
                    arm.id(),
                    rtol,
                    row.status,
                    row.error_common,
                    row.wall_ms_median
                ));
                rows.push(row);
            }
        }
        references.push(reference);
    }
    let matched = matched_rows(c, &rows, &references);
    let interior_control = interior_control_rows(&rows);
    let protocol_violation_rows = rows
        .iter()
        .filter(|r| matches!(r.status, RunStatus::ProtocolViolation { .. }))
        .count();
    let elapsed = started.elapsed().as_secs_f64();
    Ok(json!({
        "schema": RESULTS_SCHEMA,
        "timing_authority": "diagnostic_only",
        "timing_note": "in-process repetitions in rotated arm order; no frequency control; host and any CPU pinning are recorded outside this file (host.json), never inferred from it",
        "corpus": c,
        "run_seconds": elapsed,
        "allocation_measurement": {
            "measured": alloc::measured(),
            "scope": "one arm run: setup, integration, grid states in final form, solver teardown; statistics formatting excluded",
            "heap_calls": "C allocator interposer: every malloc/calloc/realloc/posix_memalign/aligned_alloc/memalign call of the process (Rust, Fortran ALLOCATE, SUNDIALS)",
            "rust_calls": "counting global allocator: Rust alloc/alloc_zeroed/realloc (a subset of heap_calls)"
        },
        "references": references,
        "rows": rows,
        "matched": matched,
        "interior_control_tol_units": INTERIOR_TOL_UNITS,
        "interior_control": interior_control,
        "protocol_violation_rows": protocol_violation_rows,
    }))
}

/// The grid-clipped continuity baseline of a comparator: band-structured arms
/// against the banded VigilODE arm, all others against the dense one.
pub fn baseline_of(a: Arm) -> Arm {
    if a.banded() {
        Arm::VigilodeBanded
    } else {
        Arm::Vigilode
    }
}

/// The headline baseline: the facade with dense output and interior
/// `WFilteredEnforce` (banded for band-structured comparators).
pub fn dense_baseline_of(a: Arm) -> Arm {
    if a.banded() {
        Arm::VigilodeBandedDense
    } else {
        Arm::VigilodeDense
    }
}

fn interior_control_rows(rows: &[Row]) -> Vec<InteriorControl> {
    let mut out = Vec::new();
    for (enforce, off) in [
        (Arm::VigilodeDense, Arm::VigilodeDenseOff),
        (Arm::VigilodeBandedDense, Arm::VigilodeBandedDenseOff),
    ] {
        for e in rows.iter().filter(|r| r.arm == enforce) {
            let Some(o) = rows
                .iter()
                .find(|r| r.arm == off && r.problem == e.problem && r.rtol == e.rtol)
            else {
                continue;
            };
            let ratio = |a: Option<f64>, b: Option<f64>| match (a, b) {
                (Some(a), Some(b)) if b != 0.0 => Some(a / b),
                _ => None,
            };
            let (ae, ao) = (e.work.accepted_steps, o.work.accepted_steps);
            out.push(InteriorControl {
                problem: e.problem.clone(),
                rtol: e.rtol,
                enforce_arm: enforce,
                off_arm: off,
                error_own_enforce: e.error_wrms_own,
                error_own_off: o.error_wrms_own,
                interior_rejections_enforce: e.native["ran"]["interior_rejections"].as_u64(),
                interior_rejections_off: o.native["ran"]["interior_rejections"].as_u64(),
                max_estimate_enforce: e.native["ran"]["max_accepted_estimate"].as_f64(),
                accepted_enforce: ae,
                accepted_off: ao,
                error_ratio: ratio(o.error_wrms_own, e.error_wrms_own),
                step_ratio: ratio(ae.map(|x| x as f64), ao.map(|x| x as f64)),
                within_tol_units: e.error_wrms_own.map(|x| x <= INTERIOR_TOL_UNITS),
            });
        }
    }
    out
}

fn matched_rows(c: &Corpus, rows: &[Row], refs: &[Reference]) -> Vec<Matched> {
    let mut out = Vec::new();
    for p in &c.problems {
        let disputed = refs.iter().any(|r| r.problem == p.id && r.disputed);
        let here: Vec<&Row> = rows.iter().filter(|r| r.problem == p.id).collect();
        // (error_common, median ms, rhs callbacks) of the successful rows.
        let curve = |a: Arm| -> Vec<(f64, f64, f64)> {
            here.iter()
                .filter(|r| r.arm == a && r.status == RunStatus::Success)
                .filter_map(|r| {
                    r.error_common
                        .map(|e| (e, r.wall_ms_median, r.callbacks.rhs as f64))
                })
                .collect()
        };
        let deterministic = |a: Arm| here.iter().filter(|r| r.arm == a).all(|r| r.deterministic);
        let at = |pts: &[(f64, f64, f64)], level: f64| {
            let t: Vec<(f64, f64)> = pts.iter().map(|&(e, t, _)| (e, t)).collect();
            let r: Vec<(f64, f64)> = pts.iter().map(|&(e, _, r)| (e, r)).collect();
            (interpolate_log(&t, level), interpolate_log(&r, level))
        };
        for &arm in &c.arms_for(p) {
            if arm.fidelity() != Fidelity::Production {
                continue;
            }
            let arms_here = c.arms_for(p);
            for (base, basis) in [
                (baseline_of(arm), "grid-clipped"),
                (dense_baseline_of(arm), "dense"),
            ] {
                if !arms_here.contains(&base) {
                    continue;
                }
                let (comp, bc) = (curve(arm), curve(base));
                for &level in &c.error_levels {
                    let (ct, cr) = at(&comp, level);
                    let (bt, br) = at(&bc, level);
                    let (status, reason) = if disputed {
                        ("DIAGNOSTIC_ONLY", "reference-disputed".to_string())
                    } else if ct.is_none() || bt.is_none() {
                        let who = match (ct.is_none(), bt.is_none()) {
                            (true, true) => "neither arm brackets the level",
                            (true, false) => "the comparator does not bracket the level",
                            _ => "the baseline does not bracket the level",
                        };
                        ("DIAGNOSTIC_ONLY", who.to_string())
                    } else if !(deterministic(arm) && deterministic(base)) {
                        (
                            "DIAGNOSTIC_ONLY",
                            "non-deterministic outputs across repetitions".into(),
                        )
                    } else {
                        ("MATCHED", String::new())
                    };
                    let matched = status == "MATCHED";
                    let ratio = |a: Option<f64>, b: Option<f64>| match (matched, a, b) {
                        (true, Some(a), Some(b)) => Some(a / b),
                        _ => None,
                    };
                    out.push(Matched {
                        problem: p.id.clone(),
                        arm,
                        baseline: base,
                        basis,
                        error_level: level,
                        status,
                        reason,
                        comparator_wall_ms: ct,
                        baseline_wall_ms: bt,
                        time_ratio: ratio(ct, bt),
                        comparator_rhs: cr,
                        baseline_rhs: br,
                        rhs_ratio: ratio(cr, br),
                        comparator_points: comp.len(),
                        baseline_points: bc.len(),
                        timing_authority: "diagnostic_only",
                    });
                }
            }
        }
    }
    out
}
