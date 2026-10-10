//! Research node `research/sp01_dupfix_adoption_20261010` (SP01): adoption
//! review of the duplicate final GMRES residual alone, under the versioned
//! residual-accounting contract `ResidualAccounting::{RecomputeFinal (v1),
//! ReuseConfirmed (v2)}`.
//!
//! Registered cells (`export_runs`, ignored; runs only its export when
//! `SP01_RUNS` is set):
//!
//! * `uform_into`: the U-form matrix-free driver through GMRES into, on the
//!   SPD07 problem set at rtol 1e-6 and 1e-8 (ALG04's C1 cells, with
//!   ALG04's `DupFix` arm recorded next to the two accountings);
//! * `uform_default`: the same 14 cells through the default (non-into)
//!   GMRES path;
//! * `kform_integrate` and `kform_step`: the sequential matrix-free K-form
//!   (the default library path with GMRES): the protected adaptive driver
//!   `integrate_sequential_matrix_free_adaptive_observed` and the adaptive
//!   loop over `sequential_matrix_free_step` (`rnext_common`'s comparator),
//!   on robertson, van-der-pol-mu1000, hires and brusselator-1d-50 at rtol
//!   1e-6 and 1e-8.
//!
//! Every cell records both accountings, each run's stage-solve reports (a
//! SHA-256 of the residual-norm, relative-residual and iteration sequence)
//! and, independently of the counters, the number of stage solves that left
//! GMRES on a confirmed true residual of the same iterate. That count comes
//! from an observer of the problem's JVP callback: the cell is run once more
//! per accounting on a wrapped problem whose JVP records its inputs. A
//! maximal run of two or more consecutive applications to the same nonzero
//! vector at the same linearization (no right-hand side evaluation between
//! them) is one loop exit followed by the recomputed final residual, and it
//! is a confirmed exit when the application just before the run acted on a
//! unit 2-norm vector (an Arnoldi basis vector) at the same linearization.
//! The wrapped runs must reproduce the plain runs bit for bit (checked by
//! the checker).

// (The export uses `write_output`, re-exported by the shared module.)
#[allow(unused_imports)]
mod alg01_common;

use std::sync::{Arc, Mutex};

use alg01_common::*;
use rodas5p_core::{
    CoreResult, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveControllerState, KrylovState, OdeProblem, Rodas5pMfAccountingResult,
    SequentialAccountingResult, StageSolveAccounting, StageSolveLogEntry, StageTargetPolicy,
    integrate_rodas5p_mf_fast_observed, integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_with_residual_accounting,
    integrate_sequential_matrix_free_adaptive_observed,
    integrate_sequential_matrix_free_adaptive_observed_with_residual_accounting,
    rodas_next_step_after_attempt, sequential_matrix_free_step,
    sequential_matrix_free_step_with_residual_accounting, sequential_step_with_residual_accounting,
};
use rodas5p_krylov::ResidualAccounting;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const RTOLS: [f64; 2] = [1.0e-6, 1.0e-8];
const KFORM_PROBLEMS: [&str; 4] = [
    "robertson",
    "van-der-pol-mu1000",
    "hires",
    "brusselator-1d-50",
];
const ACCOUNTINGS: [(&str, ResidualAccounting); 2] = [
    ("recompute_final", ResidualAccounting::RecomputeFinal),
    ("reuse_confirmed", ResidualAccounting::ReuseConfirmed),
];

// ------------------------------------------------------------- observer

/// Counts of the JVP-callback observer (see the module documentation and
/// [`Observer::close_run`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ObserverCounts {
    jvp_applications: u64,
    /// Runs of applications to the same nonzero input after an application
    /// to a unit vector: a loop exit on a confirmed true residual followed
    /// by the recomputed final residual.
    confirmed_exits: u64,
    /// Such runs otherwise (a nonzero start that met the threshold before
    /// any cycle).
    unconfirmed_duplicates: u64,
    /// Applications to an all-zero vector (the final residual of a zero
    /// iterate).
    zero_inputs: u64,
    /// Right-hand side and time-derivative evaluations (they separate
    /// stage solves; the observer forgets its history at each).
    other_events: u64,
}

/// One application of the JVP callback: linearization (time and state
/// bits) and input bits.
#[derive(Clone, PartialEq, Eq)]
struct Application {
    t: u64,
    state: Vec<u64>,
    input: Vec<u64>,
}

#[derive(Default)]
struct Observer {
    counts: ObserverCounts,
    /// The current run of applications to the same input at the same
    /// linearization: its application, its length, and whether the
    /// application just before the run acted on a unit vector at the same
    /// linearization.
    run: Option<(Application, u64, bool)>,
    /// The last application and whether its input was a unit vector.
    last: Option<(Application, bool)>,
}

impl Observer {
    /// Close the current run. A run of two or more applications to the same
    /// nonzero vector is one recomputed final residual (its last
    /// application) after the loop's true residual of the same iterate; it
    /// is a confirmed exit when the run follows an Arnoldi application.
    fn close_run(&mut self) {
        if let Some((application, length, after_unit)) = self.run.take()
            && length >= 2
            && application.input.iter().any(|b| f64::from_bits(*b) != 0.0)
        {
            if after_unit {
                self.counts.confirmed_exits += 1;
            } else {
                self.counts.unconfirmed_duplicates += 1;
            }
        }
    }

    fn event(&mut self) {
        self.counts.other_events += 1;
        self.close_run();
        self.last = None;
    }

    fn jvp(&mut self, t: f64, y: &[f64], v: &[f64]) {
        self.counts.jvp_applications += 1;
        let application = Application {
            t: t.to_bits(),
            state: y.iter().map(|x| x.to_bits()).collect(),
            input: v.iter().map(|x| x.to_bits()).collect(),
        };
        if v.iter().all(|x| *x == 0.0) {
            self.counts.zero_inputs += 1;
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        let unit = (norm - 1.0).abs() <= 1.0e-10;
        match self.run.as_mut() {
            Some((current, length, _)) if *current == application => *length += 1,
            _ => {
                self.close_run();
                let after_unit = self.last.as_ref().is_some_and(|(last, last_unit)| {
                    *last_unit && last.t == application.t && last.state == application.state
                });
                self.run = Some((application.clone(), 1, after_unit));
            }
        }
        self.last = Some((application, unit));
    }

    /// The counts with the last run closed.
    fn finish(&mut self) -> ObserverCounts {
        self.close_run();
        self.counts
    }
}

/// `problem` with every callback routed through an [`Observer`]: the same
/// values from the same callbacks, so the runs are the plain runs bit for
/// bit (the checker verifies it). Explicit Jacobians are not carried (the
/// observed paths are strictly matrix-free).
fn observed(problem: &OdeProblem) -> (OdeProblem, Arc<Mutex<Observer>>) {
    let observer = Arc::new(Mutex::new(Observer::default()));
    let inner = Arc::new(problem.clone());
    let (rhs_inner, rhs_observer) = (inner.clone(), observer.clone());
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        rhs_observer.lock().unwrap().event();
        rhs_inner.eval_rhs_into(t, y, out, &mut WorkCounters::default())
    });
    let (jvp_inner, jvp_observer) = (inner.clone(), observer.clone());
    let jvp = Arc::new(move |t: f64, y: &[f64], v: &[f64], out: &mut [f64]| {
        jvp_observer.lock().unwrap().jvp(t, y, v);
        jvp_inner.linearize_matrix_free(t, y)?.apply(v, out)
    });
    type PartialT = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
    let partial_t: Option<PartialT> = (!inner.autonomous).then(|| {
        let (ft_inner, ft_observer) = (inner.clone(), observer.clone());
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            ft_observer.lock().unwrap().event();
            let ft = ft_inner.eval_partial_t(t, y, &mut WorkCounters::default())?;
            out.copy_from_slice(&ft);
            Ok(())
        }) as PartialT
    });
    let wrapped = OdeProblem::new(
        inner.name.clone(),
        inner.dimension,
        rhs,
        None,
        None,
        Some(jvp),
        partial_t,
        inner.autonomous,
        inner.mass_matrix.clone(),
        None,
    )
    .unwrap();
    (wrapped, observer)
}

fn counts_json(counts: ObserverCounts) -> Value {
    json!({
        "jvp_applications": counts.jvp_applications,
        "confirmed_exits": counts.confirmed_exits,
        "unconfirmed_duplicates": counts.unconfirmed_duplicates,
        "zero_inputs": counts.zero_inputs,
        "other_events": counts.other_events,
    })
}

// -------------------------------------------------------------- records

fn log_json(log: &[StageSolveLogEntry]) -> Value {
    let mut hasher = Sha256::new();
    let mut iterations = 0_u64;
    for entry in log {
        hasher.update(entry.residual_norm_bits.to_le_bytes());
        hasher.update(entry.relative_residual_bits.to_le_bytes());
        hasher.update(entry.iterations.to_le_bytes());
        iterations += entry.iterations;
    }
    let digest = hasher.finalize();
    json!({
        "count": log.len(),
        "iterations": iterations,
        "sha256": digest.iter().map(|b| format!("{b:02x}")).collect::<String>(),
    })
}

/// The K-form linear configuration: the library default with GMRES.
fn kform_config() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        ..LinearSolverConfig::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Path {
    UformInto,
    UformDefault,
    KformIntegrate,
    KformStep,
}

impl Path {
    fn id(self) -> &'static str {
        match self {
            Self::UformInto => "uform_into",
            Self::UformDefault => "uform_default",
            Self::KformIntegrate => "kform_integrate",
            Self::KformStep => "kform_step",
        }
    }
}

fn uform_record(run: CoreResult<Rodas5pMfAccountingResult>) -> Value {
    match run {
        Ok(r) => {
            let mut record = result_json(&Ok(r.result));
            record["solve_log"] = log_json(&r.solve_log);
            record
        }
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

fn uform(
    p: &Problem,
    problem: &OdeProblem,
    rtol: f64,
    gmres_into: bool,
    policy: StageTargetPolicy,
    accounting: ResidualAccounting,
) -> Value {
    uform_record(integrate_rodas5p_mf_fast_observed_with_residual_accounting(
        problem,
        p.t_span,
        &p.y0,
        &gmres_config(BUDGET),
        &p.adaptive(rtol),
        &p.schedule(),
        gmres_into,
        policy,
        accounting,
    ))
}

fn kform_integrate_record(run: CoreResult<SequentialAccountingResult>) -> Value {
    match run {
        Ok(r) => {
            let o = &r.result.observed;
            let d = &r.result.diagnostics;
            json!({
                "ok": true,
                "success": o.success,
                "message": o.message,
                "t": hexes(&o.t),
                "y_last": hexes(o.y.last().unwrap()),
                "attempts": d.attempts,
                "accepted": d.accepted_macro_steps,
                "rejected": d.rejected_macro_steps,
                "linear_solve_failures": d.linear_solve_failures,
                "error_norms": hexes(&d.error_norms),
                "internal_steps": o.internal_steps,
                "output_clipped_steps": o.output_clipped_steps,
                "counters": serde_json::to_value(o.counters).unwrap(),
                "solve_log": log_json(&r.solve_log),
            })
        }
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

fn kform_integrate(
    p: &Problem,
    problem: &OdeProblem,
    rtol: f64,
    accounting: ResidualAccounting,
) -> Value {
    kform_integrate_record(
        integrate_sequential_matrix_free_adaptive_observed_with_residual_accounting(
            problem,
            p.t_span,
            &p.y0,
            &kform_config(),
            &p.adaptive(rtol),
            &p.schedule(),
            accounting,
        ),
    )
}

/// `rnext_common::sequential_adaptive` (the adaptive loop over
/// `sequential_matrix_free_step`) with an explicit accounting, recording
/// every attempt's error norm and every failed attempt's error.
fn kform_step(
    p: &Problem,
    problem: &OdeProblem,
    rtol: f64,
    accounting: ResidualAccounting,
) -> Value {
    let linear = kform_config();
    let adaptive = p.adaptive(rtol);
    let (mut t, tf) = p.t_span;
    let mut y = p.y0.clone();
    let mut h = adaptive.initial_step;
    let mut controller = AdaptiveControllerState::default();
    let mut counters = WorkCounters::default();
    let mut recycle = KrylovState::for_method(linear.method);
    let (mut attempts, mut accepted, mut rejected) = (0_usize, 0_usize, 0_usize);
    let mut errors = Vec::new();
    let mut failures = Vec::new();
    let mut log = Vec::new();
    while t < tf && attempts < adaptive.max_attempts {
        let trial = h.min(tf - t);
        if trial < adaptive.min_step && t + trial < tf {
            break;
        }
        attempts += 1;
        let step = sequential_matrix_free_step_with_residual_accounting(
            problem,
            t,
            &y,
            trial,
            &linear,
            recycle.as_mut(),
            adaptive.atol,
            adaptive.rtol,
            false,
            StageSolveAccounting {
                accounting,
                solve_log: Some(&mut log),
            },
            &mut counters,
        );
        let (error, ok) = match step {
            Ok(report) if report.accepted => {
                t = if trial == tf - t { tf } else { t + trial };
                y = report.y_new;
                (report.error_norm, true)
            }
            Ok(report) => (report.error_norm, false),
            Err(e) => {
                failures.push(e.to_string());
                (f64::INFINITY, false)
            }
        };
        errors.push(error);
        if ok {
            accepted += 1;
        } else {
            rejected += 1;
        }
        h = rodas_next_step_after_attempt(&mut controller, &adaptive, h, trial, error, ok, false)
            .unwrap();
    }
    json!({
        "ok": true,
        "success": t >= tf,
        "t_last": hexes(&[t]),
        "y_last": hexes(&y),
        "attempts": attempts,
        "accepted": accepted,
        "rejected": rejected,
        "error_norms": errors.iter().map(|e| format!("{:016x}", e.to_bits())).collect::<Vec<_>>(),
        "failed_attempts": failures,
        "counters": serde_json::to_value(counters).unwrap(),
        "solve_log": log_json(&log),
    })
}

fn run_path(
    path: Path,
    p: &Problem,
    problem: &OdeProblem,
    rtol: f64,
    accounting: ResidualAccounting,
) -> Value {
    match path {
        Path::UformInto => uform(
            p,
            problem,
            rtol,
            true,
            StageTargetPolicy::Legacy,
            accounting,
        ),
        Path::UformDefault => uform(
            p,
            problem,
            rtol,
            false,
            StageTargetPolicy::Legacy,
            accounting,
        ),
        Path::KformIntegrate => kform_integrate(p, problem, rtol, accounting),
        Path::KformStep => kform_step(p, problem, rtol, accounting),
    }
}

/// One cell: both accountings on the plain problem, both on the observed
/// problem with the observer counts, and on `uform_into` ALG04's `DupFix`.
fn cell(path: Path, p: &Problem, rtol: f64) -> Value {
    let mut arms = serde_json::Map::new();
    let mut observed_arms = serde_json::Map::new();
    let mut observer = serde_json::Map::new();
    for (name, accounting) in ACCOUNTINGS {
        arms.insert(name.into(), run_path(path, p, &p.problem, rtol, accounting));
        let (wrapped, counts) = observed(&p.problem);
        observed_arms.insert(name.into(), run_path(path, p, &wrapped, rtol, accounting));
        observer.insert(name.into(), counts_json(counts.lock().unwrap().finish()));
    }
    if path == Path::UformInto {
        arms.insert(
            "dup_fix".into(),
            uform(
                p,
                &p.problem,
                rtol,
                true,
                StageTargetPolicy::DupFix,
                ResidualAccounting::DEFAULT,
            ),
        );
    }
    json!({
        "path": path.id(),
        "case": p.id,
        "rtol": rtol,
        "dimension": p.y0.len(),
        "arms": arms,
        "observed_arms": observed_arms,
        "observer": observer,
    })
}

fn cells() -> Vec<(Path, String, f64)> {
    let mut out = Vec::new();
    for path in [Path::UformInto, Path::UformDefault] {
        for p in spd07_problems() {
            for rtol in RTOLS {
                out.push((path, p.id.clone(), rtol));
            }
        }
    }
    for path in [Path::KformIntegrate, Path::KformStep] {
        for id in KFORM_PROBLEMS {
            for rtol in RTOLS {
                out.push((path, id.to_string(), rtol));
            }
        }
    }
    out
}

/// The registered SP01 runs.
#[test]
#[ignore = "recorded export of research/sp01_dupfix_adoption_20261010; release build"]
fn export_runs() {
    let problems = spd07_problems();
    let mut rows = Vec::new();
    for (path, id, rtol) in cells() {
        let start = std::time::Instant::now();
        let p = problems.iter().find(|p| p.id == id).unwrap();
        rows.push(cell(path, p, rtol));
        println!(
            "{} {id} {rtol:e}: {:.1} s",
            path.id(),
            start.elapsed().as_secs_f64()
        );
    }
    write_output(
        "SP01_RUNS",
        &json!({
            "node": "sp01_dupfix_adoption_20261010",
            "export": "export_runs",
            "accountings": {
                "recompute_final": ResidualAccounting::RecomputeFinal.id(),
                "reuse_confirmed": ResidualAccounting::ReuseConfirmed.id(),
            },
            "default_accounting": ResidualAccounting::DEFAULT.id(),
            "uform_budget": BUDGET,
            "kform_linear_config": format!("{:?}", kform_config()),
            "rows": rows,
        }),
    );
}

// ------------------------------------------------------------ contracts

/// The registered cell list: 14 + 14 + 8 + 8 cells.
#[test]
fn the_registered_cells() {
    let cells = cells();
    assert_eq!(cells.len(), 44);
    for (path, count) in [
        (Path::UformInto, 14),
        (Path::UformDefault, 14),
        (Path::KformIntegrate, 8),
        (Path::KformStep, 8),
    ] {
        assert_eq!(cells.iter().filter(|c| c.0 == path).count(), count);
    }
    let ids: Vec<String> = spd07_problems().into_iter().map(|p| p.id).collect();
    assert_eq!(
        ids,
        [
            "robertson",
            "van-der-pol-mu1000",
            "hires",
            "brusselator-1d-50",
            "prothero-robinson-forced",
            "quadratic-4",
            "brusselator-1d-160"
        ]
    );
}

fn without_log(mut record: Value) -> Value {
    record.as_object_mut().unwrap().remove("solve_log");
    record
}

/// With the kernels' default accounting the new entry points are the
/// existing drivers bit for bit.
#[test]
fn the_accounting_entry_points_reproduce_the_default_drivers() {
    for id in ["quadratic-4", "prothero-robinson-forced"] {
        let p = spd07_problem(id);
        let rtol = 1.0e-6;
        let config = gmres_config(BUDGET);
        let into = integrate_rodas5p_mf_fast_observed_gmres_into(
            &p.problem,
            p.t_span,
            &p.y0,
            &config,
            &p.adaptive(rtol),
            &p.schedule(),
        );
        assert_eq!(
            without_log(uform(
                &p,
                &p.problem,
                rtol,
                true,
                StageTargetPolicy::Legacy,
                ResidualAccounting::DEFAULT
            )),
            result_json(&into)
        );
        let default = integrate_rodas5p_mf_fast_observed(
            &p.problem,
            p.t_span,
            &p.y0,
            &config,
            &p.adaptive(rtol),
            &p.schedule(),
        );
        assert_eq!(
            without_log(uform(
                &p,
                &p.problem,
                rtol,
                false,
                StageTargetPolicy::Legacy,
                ResidualAccounting::DEFAULT
            )),
            result_json(&default)
        );
        let library = integrate_sequential_matrix_free_adaptive_observed(
            &p.problem,
            p.t_span,
            &p.y0,
            &kform_config(),
            &p.adaptive(rtol),
            &p.schedule(),
        )
        .unwrap();
        let with = integrate_sequential_matrix_free_adaptive_observed_with_residual_accounting(
            &p.problem,
            p.t_span,
            &p.y0,
            &kform_config(),
            &p.adaptive(rtol),
            &p.schedule(),
            ResidualAccounting::DEFAULT,
        )
        .unwrap();
        assert_eq!(format!("{library:?}"), format!("{:?}", with.result));
    }
}

/// The K-form step loop with the default accounting is `rnext_common`'s
/// comparator bit for bit.
#[test]
fn the_kform_step_loop_is_the_comparator() {
    let p = spd07_problem("hires");
    let rtol = 1.0e-6;
    let reference = rnext::sequential_adaptive(
        &p.problem,
        p.t_span,
        &p.y0,
        &kform_config(),
        &p.adaptive(rtol),
    );
    let record = kform_step(&p, &p.problem, rtol, ResidualAccounting::DEFAULT);
    assert_eq!(record["y_last"], json!(hexes(&reference.y)));
    assert_eq!(record["attempts"], json!(reference.attempts));
    assert_eq!(record["accepted"], json!(reference.accepted));
    assert_eq!(record["rejected"], json!(reference.rejected));
    assert_eq!(record["success"], json!(reference.success));
    assert_eq!(
        record["counters"],
        serde_json::to_value(reference.counters).unwrap()
    );
    // And the library step function is the default-accounting step.
    let mut a = WorkCounters::default();
    let mut b = WorkCounters::default();
    let one = sequential_matrix_free_step(
        &p.problem,
        0.0,
        &p.y0,
        1.0e-3,
        &kform_config(),
        None,
        1.0e-10,
        rtol,
        false,
        &mut a,
    )
    .unwrap();
    let two = sequential_matrix_free_step_with_residual_accounting(
        &p.problem,
        0.0,
        &p.y0,
        1.0e-3,
        &kform_config(),
        None,
        1.0e-10,
        rtol,
        false,
        StageSolveAccounting::default(),
        &mut b,
    )
    .unwrap();
    assert_eq!(format!("{one:?}"), format!("{two:?}"));
    assert_eq!(a, b);
}

/// The counters that a removed final residual changes.
const ACCOUNTING_COUNTERS: [&str; 4] = [
    "diagnostic_matvecs",
    "jvp_calls",
    "jvp_vectors",
    "linear_matvec_vectors",
];

/// Parity and exact accounting on one short cell per path, with the count
/// from the observer; the observed runs are the plain runs. (The observer
/// reads the input stream; in one dimension, where a stage combination can
/// equal a stage bit for bit, two solves' runs can merge, so the K-form
/// contract uses n > 1 as the registered K-form cells do.)
#[test]
fn both_accountings_agree_and_differ_by_the_observed_count() {
    for (path, id) in [
        (Path::UformInto, "hires"),
        (Path::UformDefault, "quadratic-4"),
        (Path::KformIntegrate, "hires"),
        (Path::KformStep, "quadratic-4"),
    ] {
        let p = spd07_problem(id);
        let rtol = 1.0e-6;
        let record = cell(path, &p, rtol);
        let (v1, v2) = (
            &record["arms"]["recompute_final"],
            &record["arms"]["reuse_confirmed"],
        );
        for (name, _) in ACCOUNTINGS {
            assert_eq!(
                record["observed_arms"][name], record["arms"][name],
                "{id} {name}"
            );
        }
        for (key, value) in v1.as_object().unwrap() {
            if key != "counters" {
                assert_eq!(&v2[key], value, "{} {id} {key}", path.id());
            }
        }
        let confirmed = record["observer"]["recompute_final"]["confirmed_exits"]
            .as_u64()
            .unwrap();
        assert!(confirmed > 0, "{} {id}", path.id());
        assert_eq!(
            record["observer"]["reuse_confirmed"]["confirmed_exits"],
            json!(0)
        );
        let (c1, c2) = (&v1["counters"], &v2["counters"]);
        for key in ["jvp_calls", "jvp_vectors", "diagnostic_matvecs"] {
            assert_eq!(
                c1[key].as_u64().unwrap() - c2[key].as_u64().unwrap(),
                confirmed,
                "{} {id} {key}",
                path.id()
            );
        }
        for (key, value) in c1.as_object().unwrap() {
            if !ACCOUNTING_COUNTERS.contains(&key.as_str()) {
                assert_eq!(&c2[key], value, "{} {id} {key}", path.id());
            }
        }
        // Every v1 final residual is a confirmed exit, a no-cycle nonzero
        // start or a zero iterate.
        let o = &record["observer"]["recompute_final"];
        assert_eq!(
            c1["diagnostic_matvecs"].as_u64().unwrap(),
            confirmed
                + o["unconfirmed_duplicates"].as_u64().unwrap()
                + o["zero_inputs"].as_u64().unwrap(),
            "{} {id}",
            path.id()
        );
    }
}

/// ALG04's `DupFix` is unchanged: its skip overrides the accounting, and it
/// keeps `Legacy`'s trajectory with no diagnostic residual at all.
#[test]
fn dup_fix_is_unchanged() {
    let p = spd07_problem("robertson");
    let rtol = 1.0e-6;
    let run = |accounting| {
        uform(
            &p,
            &p.problem,
            rtol,
            true,
            StageTargetPolicy::DupFix,
            accounting,
        )
    };
    let (a, b) = (
        run(ResidualAccounting::RecomputeFinal),
        run(ResidualAccounting::ReuseConfirmed),
    );
    assert_eq!(a, b);
    assert_eq!(a["counters"]["diagnostic_matvecs"], json!(0));
    let legacy = uform(
        &p,
        &p.problem,
        rtol,
        true,
        StageTargetPolicy::Legacy,
        ResidualAccounting::RecomputeFinal,
    );
    for key in [
        "t",
        "y_last",
        "attempts",
        "accepted",
        "rejected",
        "solve_log",
    ] {
        assert_eq!(a[key], legacy[key], "{key}");
    }
}

/// The explicit-W sequential path with Jacobi left preconditioning (the
/// registered kernel boundary case at the driver level): both accountings
/// give the same steps and stage reports; the counters differ by the stage
/// solves that iterated (a nonzero start that met the threshold before any
/// cycle keeps its final residual).
#[test]
fn explicit_w_jacobi_sequential_steps() {
    let p = spd07_problem("hires");
    let config = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::Jacobi,
        restart: 4,
        ..LinearSolverConfig::default()
    };
    let mut runs = Vec::new();
    for (_, accounting) in ACCOUNTINGS {
        let mut counters = WorkCounters::default();
        let mut log = Vec::new();
        let (mut t, mut y, mut h) = (0.0, p.y0.clone(), 1.0e-3);
        let mut steps = Vec::new();
        for _ in 0..12 {
            let step = sequential_step_with_residual_accounting(
                &p.full,
                t,
                &y,
                h,
                &config,
                None,
                1.0e-10,
                1.0e-6,
                false,
                StageSolveAccounting {
                    accounting,
                    solve_log: Some(&mut log),
                },
                &mut counters,
            )
            .unwrap();
            steps.push(format!(
                "{:?} {:?} {}",
                step.y_new, step.stages, step.error_norm
            ));
            if step.accepted {
                t += h;
                y = step.y_new;
            }
            h *= if step.accepted { 1.5 } else { 0.5 };
        }
        runs.push((steps, log, counters));
    }
    let (v1, v2) = (&runs[0], &runs[1]);
    assert_eq!(v1.0, v2.0);
    assert_eq!(v1.1, v2.1);
    let iterated = v1.1.iter().filter(|e| e.iterations > 0).count() as u64;
    assert!(iterated > 0);
    assert!(v1.2.preconditioner_apps > 0);
    assert_eq!(v1.2.diagnostic_matvecs - v2.2.diagnostic_matvecs, iterated);
    let mut expected = v2.2;
    expected.diagnostic_matvecs = v1.2.diagnostic_matvecs;
    expected.linear_matvec_vectors = v1.2.linear_matvec_vectors;
    expected.jacobian_matvecs = v1.2.jacobian_matvecs;
    expected.jvp_calls = v1.2.jvp_calls;
    expected.jvp_vectors = v1.2.jvp_vectors;
    assert_eq!(expected, v1.2);
}
