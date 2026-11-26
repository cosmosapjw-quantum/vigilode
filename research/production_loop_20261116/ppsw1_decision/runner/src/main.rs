//! Measurement program of loop L1 node V10_PPSW1_DECISION (see ../PREDECLARED.json).
//!
//! `v10-runner <mode> key=value ...`; every mode prints JSON lines on stdout.
//! Modes: selftest, fidelity, counts, cost, timed, calibrate, forced.

use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::Instant;

use serde_json::{Value, json};
use v10_runner::driver::{self, PeerOptions, PeerRun};
use v10_runner::ppsw1::{self, Config, TableauCacheMode};
use v10_runner::problems::{Pid, core_problem, facade_problem, scalar_problem};
use v10_runner::util::{
    bits_eq, cpus_allowed, hash_outputs, hash_vec, load1, loadavg_line, runs_eq, states_eq,
};
use v10_runner::{OdeProblem, ParallelExecution};
use vigilode::{
    Atol, CapabilitySet, Options, PeerOde, PeerSolution, PeerSolveOptions, Problem, Rodas5p,
};

const RTOLS: [f64; 7] = [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9];
const CAPS: [f64; 4] = [1.10, 1.2, 1.3, 1.5];

fn args() -> (String, BTreeMap<String, String>) {
    let mut it = std::env::args().skip(1);
    let mode = it.next().unwrap_or_default();
    let kv = it
        .filter_map(|a| a.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
        .collect();
    (mode, kv)
}

fn get<T: std::str::FromStr>(kv: &BTreeMap<String, String>, key: &str, default: Option<T>) -> T {
    match kv.get(key) {
        Some(v) => v
            .parse()
            .unwrap_or_else(|_| panic!("argument {key}={v} does not parse")),
        None => default.unwrap_or_else(|| panic!("missing argument {key}")),
    }
}

fn emit(v: Value) {
    println!("{}", serde_json::to_string(&v).expect("json"));
}

fn host() -> Value {
    json!({"load1_start": load1(), "loadavg_start": loadavg_line(), "cpus_allowed": cpus_allowed(),
           "pid": std::process::id()})
}

/// An arm of PREDECLARED.json.
#[derive(Clone, Copy, Debug)]
enum Arm {
    /// `vigilode::Rodas5p::solve`.
    R,
    /// `vigilode::PeerOde::solve` with this many threads.
    C(usize),
    /// The research copy: cap, threads, cache mode.
    P(f64, usize, TableauCacheMode),
}

fn parse_arm(s: &str) -> Arm {
    if s == "R" {
        return Arm::R;
    }
    if let Some(t) = s.strip_prefix("C-t") {
        return Arm::C(t.parse().expect("threads"));
    }
    let body = s.strip_prefix('P').expect("arm must be R, C-t<n> or P<cap>-t<n>-<cache>");
    let parts: Vec<&str> = body.split('-').collect();
    assert!(parts.len() == 3, "arm {s}");
    let cap: f64 = parts[0].parse().expect("cap");
    let threads: usize = parts[1].strip_prefix('t').expect("t").parse().expect("threads");
    let cache = match parts[2] {
        "off" => TableauCacheMode::Off,
        "on" => TableauCacheMode::On,
        "audit" => TableauCacheMode::Audit,
        other => panic!("cache mode {other}"),
    };
    Arm::P(cap, threads, cache)
}

fn exec_for(threads: usize) -> ParallelExecution {
    ParallelExecution::rayon(threads).expect("pool")
}

fn peer_options(pid: Pid, rtol: f64, max_attempts: usize) -> PeerOptions {
    let (t0, t1) = pid.span();
    PeerOptions {
        rtol,
        atol: 1e-2 * rtol,
        h0_rule_h_max: (t1 - t0) / 100.0,
        h0: None,
        max_attempts,
    }
}

fn crate_options(pid: Pid, rtol: f64, threads: usize, max_attempts: usize) -> PeerSolveOptions {
    let (t0, t1) = pid.span();
    PeerSolveOptions {
        rtol,
        atol: 1e-2 * rtol,
        h0_rule_h_max: (t1 - t0) / 100.0,
        h0: None,
        threads,
        max_attempts,
    }
}

fn allowed() -> CapabilitySet {
    CapabilitySet {
        explicit_jacobian: true,
        dense_matrix: true,
        newton: true,
        ..CapabilitySet::NONE
    }
}

fn rodas_options(rtol: f64, max_steps: usize) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(1e-2 * rtol),
        max_steps,
        ..Options::default()
    }
}

/// Context of one problem: both problem forms and the persistent pools of the process.
struct Ctx {
    pid: Pid,
    facade: Problem,
    core: OdeProblem,
    y0: Vec<f64>,
    seq: ParallelExecution,
    pool6: Option<ParallelExecution>,
}

impl Ctx {
    fn new(pid: Pid, mult: u32, need_pool6: bool) -> Self {
        Ctx {
            pid,
            facade: facade_problem(pid, mult),
            core: core_problem(pid, mult).expect("core problem"),
            y0: pid.y0(),
            seq: ParallelExecution::sequential(),
            pool6: if need_pool6 { Some(exec_for(6)) } else { None },
        }
    }
    fn exec(&self, threads: usize) -> &ParallelExecution {
        match threads {
            1 => &self.seq,
            6 => self.pool6.as_ref().expect("pool of 6 threads"),
            other => panic!("threads {other} not declared"),
        }
    }
}

fn research_run(
    ctx: &Ctx,
    rtol: f64,
    cap: f64,
    threads: usize,
    cache: TableauCacheMode,
    max_attempts: usize,
) -> PeerRun {
    driver::solve(
        &ctx.core,
        ctx.pid.span(),
        &ctx.y0,
        &peer_options(ctx.pid, rtol, max_attempts),
        Config {
            step_ratio_cap: cap,
            tableau_cache: cache,
        },
        ctx.exec(threads),
    )
}

fn crate_run(ctx: &Ctx, rtol: f64, threads: usize, max_attempts: usize) -> PeerSolution {
    PeerOde::solve(
        &ctx.facade,
        ctx.pid.span(),
        &ctx.y0,
        &crate_options(ctx.pid, rtol, threads, max_attempts),
        allowed(),
    )
    .expect("PeerOde refused the declared problem")
}

/// Bitwise equality of a crate PeerOde solution and a research run in every compared field.
fn crate_eq(c: &PeerSolution, r: &PeerRun) -> bool {
    bits_eq(&c.t, &r.t)
        && states_eq(&c.y, &r.y)
        && c.success == r.success
        && c.h0.to_bits() == r.h0.to_bits()
        && c.starter.h0_requested.to_bits() == r.starter.h0_requested.to_bits()
        && c.starter.halvings == r.starter.halvings
        && c.starter.estimate.to_bits() == r.starter.estimate.to_bits()
        && c.starter.covers_span == r.starter.covers_span
        && c.startup_work == r.startup_work
        && c.step_work == r.step_work
}

fn sigma_stats(run: &PeerRun, cap: f64) -> Value {
    let acc: Vec<f64> = run
        .trace
        .iter()
        .filter(|a| a.accepted)
        .map(|a| a.sigma)
        .collect();
    let growth = acc.iter().filter(|&&s| s > 1.0).count();
    let cap_hits = acc.iter().filter(|&&s| s >= cap * (1.0 - 1e-12)).count();
    let max_sigma = acc.iter().copied().fold(0.0_f64, f64::max);
    let min_sigma = acc.iter().copied().fold(f64::INFINITY, f64::min);
    json!({"accepted": acc.len(), "rejected": run.trace.len() - acc.len(),
           "growth_steps": growth, "cap_hits": cap_hits,
           "max_sigma": if acc.is_empty() { Value::Null } else { json!(max_sigma) },
           "min_sigma": if acc.is_empty() { Value::Null } else { json!(min_sigma) }})
}

fn run_record(run: &PeerRun) -> Value {
    let y_end = run.y.last().cloned().unwrap_or_default();
    json!({
        "success": run.success, "failure": run.failure,
        "t_last": run.t.last(), "y_end": y_end, "y_end_hash": hash_vec(&y_end),
        "outputs_hash": hash_outputs(&run.t, &run.y), "outputs": run.t.len(),
        "attempts": run.attempts,
        "accepted": run.step_work.accepted_steps, "rejected": run.step_work.rejected_steps,
        "h0": run.h0,
        "starter": {"h0_requested": run.starter.h0_requested, "halvings": run.starter.halvings,
                    "estimate": if run.starter.estimate.is_finite() { json!(run.starter.estimate) } else { Value::Null },
                    "covers_span": run.starter.covers_span},
        "startup_work": run.startup_work, "step_work": run.step_work,
        "cache": run.cache.as_ref().map(|c| json!({
            "lookups": c.lookups, "hits": c.hits, "misses": c.misses,
            "audit_compared": c.audit_compared, "audit_mismatches": c.audit_mismatches,
            "distinct_sigmas": c.distinct_sigmas})),
    })
}

fn rodas_record(sol: &Result<vigilode::Solution, vigilode::SolveError>) -> Value {
    match sol {
        Ok(s) => {
            let y_end = s.final_state().map(<[f64]>::to_vec).unwrap_or_default();
            json!({"success": s.is_success(), "y_end": y_end, "y_end_hash": hash_vec(&y_end),
                   "t_last": s.t.last(),
                   "attempts": s.stats.attempts, "accepted": s.stats.accepted_steps,
                   "rejected": s.stats.rejected_steps, "driver_id": s.stats.route.driver_id,
                   "entry_point": s.stats.route.entry_point, "counters": s.stats.counters})
        }
        Err(e) => json!({"success": false, "error": e.to_string()}),
    }
}

fn mode_selftest() {
    let mut fails = 0;
    for pid in [
        Pid::D4,
        Pid::Kaps,
        Pid::Burgers64,
        Pid::Ac1d1000,
        Pid::Oregonator,
        Pid::Hires,
        Pid::BrussMol32,
    ] {
        let y = pid.y0();
        let (mut a, mut b) = (vec![0.0; pid.n()], vec![0.0; pid.n()]);
        pid.rhs(&y, &mut a);
        pid.rhs_mult(37, &y, &mut b);
        let same = bits_eq(&a, &b);
        if !same {
            fails += 1;
        }
        emit(json!({"mode": "selftest", "problem": pid.name(), "multiplier_keeps_bits": same}));
    }
    emit(json!({"mode": "selftest_summary", "failures": fails}));
}

fn mode_fidelity(kv: &BTreeMap<String, String>) {
    let pid = Pid::parse(&get::<String>(kv, "problem", None)).expect("problem");
    let rtol: f64 = get(kv, "rtol", None);
    let threads: usize = get(kv, "threads", Some(1));
    let extra: u32 = get(kv, "extra", Some(0));
    let ctx = Ctx::new(pid, 1, threads == 6 || extra == 1);
    let control = research_run(&ctx, rtol, 1.10, threads, TableauCacheMode::Off, 1_000_000);
    let crate_sol = crate_run(&ctx, rtol, threads, 1_000_000);
    let f2 = crate_eq(&crate_sol, &control);
    let mut rec = json!({"mode": "fidelity", "problem": pid.name(), "rtol": rtol,
        "threads": threads, "F2_control_equals_peerode": f2,
        "control": run_record(&control),
        "peerode_outputs_hash": hash_outputs(&crate_sol.t, &crate_sol.y),
        "peerode_success": crate_sol.success, "host": host()});
    if extra == 1 {
        let mut caps = Vec::new();
        for cap in CAPS {
            let off = research_run(&ctx, rtol, cap, threads, TableauCacheMode::Off, 1_000_000);
            let audit = research_run(&ctx, rtol, cap, threads, TableauCacheMode::Audit, 1_000_000);
            let mismatches = audit.cache.as_ref().map(|c| c.audit_mismatches);
            let f3 = runs_eq(&off, &audit) && mismatches == Some(0);
            let f4 = if threads == 1 {
                let on6 = research_run(&ctx, rtol, cap, 6, TableauCacheMode::On, 1_000_000);
                Some(runs_eq(&off, &on6))
            } else {
                None
            };
            caps.push(json!({"cap": cap, "F3_cache_changes_no_bit": f3,
                "audit_mismatches": mismatches,
                "distinct_sigmas": audit.cache.as_ref().map(|c| c.distinct_sigmas),
                "lookups": audit.cache.as_ref().map(|c| c.lookups),
                "F4_threads6_equals_threads1": f4,
                "outputs_hash": hash_outputs(&off.t, &off.y), "success": off.success}));
        }
        rec["caps"] = json!(caps);
    }
    emit(rec);
}

fn mode_counts(kv: &BTreeMap<String, String>) {
    let pid = Pid::parse(&get::<String>(kv, "problem", None)).expect("problem");
    let rtols: Vec<f64> = match kv.get("rtols") {
        Some(s) => s.split(',').map(|x| x.parse().expect("rtol")).collect(),
        None => RTOLS.to_vec(),
    };
    let arms: Vec<String> = match kv.get("arms") {
        Some(s) => s.split(',').map(str::to_string).collect(),
        None => [
            "R",
            "P1.10-t1-audit",
            "P1.2-t1-audit",
            "P1.3-t1-audit",
            "P1.5-t1-audit",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
    };
    let ctx = Ctx::new(pid, 1, false);
    emit(json!({"mode": "counts_header", "problem": pid.name(), "n": pid.n(),
                "rtols": rtols, "arms": arms, "host": host()}));
    for &rtol in &rtols {
        for arm_name in &arms {
            let clock = Instant::now();
            let (body, cap) = match parse_arm(arm_name) {
                Arm::R => {
                    let sol = Rodas5p::solve(
                        &ctx.facade,
                        pid.span(),
                        &ctx.y0,
                        &rodas_options(rtol, 100_000),
                    );
                    (rodas_record(&sol), None)
                }
                Arm::C(th) => {
                    assert_eq!(th, 1, "count runs use one thread");
                    let c = crate_run(&ctx, rtol, 1, 1_000_000);
                    let y_end = c.y.last().cloned().unwrap_or_default();
                    (
                        json!({"success": c.success, "failure": c.failure,
                               "y_end": y_end, "y_end_hash": hash_vec(&y_end),
                               "outputs_hash": hash_outputs(&c.t, &c.y),
                               "accepted": c.step_work.accepted_steps,
                               "rejected": c.step_work.rejected_steps, "h0": c.h0,
                               "startup_work": c.startup_work, "step_work": c.step_work}),
                        None,
                    )
                }
                Arm::P(cap, th, cache) => {
                    assert_eq!(th, 1, "count runs use one thread");
                    let run = research_run(&ctx, rtol, cap, 1, cache, 1_000_000);
                    let mut rec = run_record(&run);
                    rec["sigma_stats"] = sigma_stats(&run, cap);
                    (rec, Some(cap))
                }
            };
            let wall = clock.elapsed().as_secs_f64();
            emit(json!({"mode": "count", "problem": pid.name(), "n": pid.n(), "rtol": rtol,
                        "atol": 1e-2 * rtol, "arm": arm_name, "cap": cap,
                        "result": body, "wall_s_diagnostic": wall}));
        }
    }
}

/// One solve of an arm; returns (wall seconds, attempts, success, end-state hash, record).
fn one_solve(
    ctx: &Ctx,
    arm: Arm,
    rtol: f64,
    budget: usize,
) -> (f64, Option<u64>, bool, String, Value) {
    match arm {
        Arm::R => {
            let opts = rodas_options(rtol, budget);
            let clock = Instant::now();
            let sol = Rodas5p::solve(&ctx.facade, ctx.pid.span(), &ctx.y0, &opts);
            let wall = clock.elapsed().as_secs_f64();
            let (att, ok, hash) = match &sol {
                Ok(s) => (
                    Some(s.stats.attempts as u64),
                    s.is_success(),
                    hash_vec(s.final_state().unwrap_or(&[])),
                ),
                Err(_) => (None, false, String::new()),
            };
            (wall, att, ok, hash, rodas_record(&sol))
        }
        Arm::C(th) => {
            let opts = crate_options(ctx.pid, rtol, th, budget);
            let clock = Instant::now();
            let c = PeerOde::solve(&ctx.facade, ctx.pid.span(), &ctx.y0, &opts, allowed())
                .expect("PeerOde refused");
            let wall = clock.elapsed().as_secs_f64();
            let att = c.step_work.accepted_steps + c.step_work.rejected_steps;
            let y_end = c.y.last().cloned().unwrap_or_default();
            let rec = json!({"success": c.success, "accepted": c.step_work.accepted_steps,
                             "rejected": c.step_work.rejected_steps,
                             "outputs_hash": hash_outputs(&c.t, &c.y)});
            (wall, Some(att), c.success, hash_vec(&y_end), rec)
        }
        Arm::P(cap, th, cache) => {
            let clock = Instant::now();
            let run = research_run(ctx, rtol, cap, th, cache, budget);
            let wall = clock.elapsed().as_secs_f64();
            let y_end = run.y.last().cloned().unwrap_or_default();
            let rec = json!({"success": run.success, "accepted": run.step_work.accepted_steps,
                             "rejected": run.step_work.rejected_steps,
                             "outputs_hash": hash_outputs(&run.t, &run.y),
                             "cache_hits": run.cache.as_ref().map(|c| c.hits),
                             "cache_misses": run.cache.as_ref().map(|c| c.misses)});
            (
                wall,
                Some(run.attempts as u64),
                run.success,
                hash_vec(&y_end),
                rec,
            )
        }
    }
}

fn repeated(kv: &BTreeMap<String, String>, mode: &str) {
    let pid = Pid::parse(&get::<String>(kv, "problem", None)).expect("problem");
    let arm_name: String = get(kv, "arm", None);
    let arm = parse_arm(&arm_name);
    let rtol: f64 = get(kv, "rtol", None);
    let mult: u32 = get(kv, "mult", Some(1));
    let warm: usize = get(kv, "warm", Some(2));
    let reps: usize = get(kv, "reps", Some(7));
    let default_budget = match arm {
        Arm::R => 100_000,
        _ => 1_000_000,
    };
    let budget: usize = get(kv, "budget", Some(default_budget));
    let tag: String = get(kv, "tag", Some(String::new()));
    let host_start = host();
    let need6 = matches!(arm, Arm::P(_, 6, _));
    let ctx = Ctx::new(pid, mult, need6);
    let mut walls = Vec::new();
    let mut attempts = Vec::new();
    let mut hashes = Vec::new();
    let mut oks = Vec::new();
    let mut last = Value::Null;
    for _ in 0..warm + reps {
        let (w, a, ok, h, rec) = one_solve(&ctx, arm, rtol, budget);
        walls.push(w);
        attempts.push(a);
        hashes.push(h);
        oks.push(ok);
        last = rec;
    }
    let measured: Vec<f64> = walls[warm..].to_vec();
    let mut sorted = measured.clone();
    sorted.sort_by(f64::total_cmp);
    let median = if sorted.is_empty() {
        Value::Null
    } else {
        json!(sorted[sorted.len() / 2])
    };
    emit(json!({"mode": mode, "tag": tag, "problem": pid.name(), "n": pid.n(), "arm": arm_name,
                "rtol": rtol, "mult": mult, "budget": budget, "warm": warm, "reps": reps,
                "walls_s": walls, "median_wall_s": median, "attempts": attempts,
                "success": oks, "end_hashes": hashes, "last": last,
                "host_start": host_start, "load1_end": load1()}));
}

fn mode_calibrate(kv: &BTreeMap<String, String>) {
    let pid = Pid::parse(&get::<String>(kv, "problem", None)).expect("problem");
    let blocks: usize = get(kv, "blocks", Some(11));
    let calls: usize = get(kv, "calls", Some(200_000));
    let host_start = host();
    let y = pid.y0();
    let mut d = vec![0.0; pid.n()];
    let mut per_call = Vec::new();
    for _ in 0..blocks {
        let clock = Instant::now();
        for _ in 0..calls {
            pid.rhs(black_box(&y), &mut d);
            black_box(&mut d);
        }
        per_call.push(clock.elapsed().as_secs_f64() / calls as f64);
    }
    let mut sorted = per_call.clone();
    sorted.sort_by(f64::total_cmp);
    emit(json!({"mode": "calibrate", "problem": pid.name(), "n": pid.n(), "blocks": blocks,
                "calls_per_block": calls, "per_call_s": per_call,
                "median_per_call_s": sorted[sorted.len() / 2], "host_start": host_start,
                "load1_end": load1()}));
}

/// Executed stability run: a forced periodic ratio sequence through the research session.
fn mode_forced(kv: &BTreeMap<String, String>) {
    let cap: f64 = get(kv, "cap", None);
    let seq: Vec<f64> = get::<String>(kv, "seq", None)
        .split(',')
        .map(|x| x.parse().expect("ratio"))
        .collect();
    let z0: f64 = get(kv, "z0", None);
    let steps: usize = get(kv, "steps", None);
    let kind: String = get(kv, "kind", None);
    let label: String = get(kv, "label", Some(String::new()));
    let pr = kind == "pr";
    let lambda: f64 = if pr { get(kv, "lambda", Some(-1e4)) } else { -1.0 };
    let h0 = (z0 / lambda).abs();
    let ode = scalar_problem(lambda, pr).expect("scalar problem");
    let exec = ParallelExecution::sequential();
    let c = ppsw1::nodes();
    let exact = |t: f64| if pr { t.cos() } else { (lambda * t).exp() };
    // Start data: exact on the solution when representable (2 h0 |lambda| <= 600 for the
    // linear problem), otherwise ones (AMENDMENT_A1.json); F = f(t, Y).
    let unit_start = !pr && 2.0 * h0 * lambda.abs() > 600.0;
    let t0 = 0.0;
    let start = |h: f64| -> ppsw1::State {
        let mut ys: Vec<Vec<f64>> = Vec::new();
        let mut fs: Vec<Vec<f64>> = Vec::new();
        for cj in c {
            let tj = t0 + h * (cj - 1.0);
            let yj = if unit_start { 1.0 } else { exact(tj) };
            let fj = if pr {
                lambda * (yj - tj.cos()) - tj.sin()
            } else {
                lambda * yj
            };
            ys.push(vec![yj]);
            fs.push(vec![fj]);
        }
        ppsw1::State {
            t: t0,
            y: <[Vec<f64>; 6]>::try_from(ys).expect("six"),
            f: <[Vec<f64>; 6]>::try_from(fs).expect("six"),
            h_prev: h,
        }
    };
    let run = |ratios: &[f64]| -> Value {
        let st = start(h0);
        let norm0 = st.y.iter().map(|v| v[0].abs()).fold(0.0_f64, f64::max);
        let mut session = ppsw1::Session::new(
            &ode,
            st,
            Config {
                step_ratio_cap: cap,
                tableau_cache: TableauCacheMode::On,
            },
            &exec,
        )
        .expect("session");
        let p = ratios.len();
        let mut amp_max = 0.0_f64;
        let mut err_max = 0.0_f64;
        let mut period_log = Vec::new();
        let mut failure = Value::Null;
        let mut done = 0usize;
        for m in 0..steps {
            let h = ratios[m % p] * session.state().h_prev;
            match session.attempt(h) {
                Ok(a) => {
                    session.accept(a).expect("accept");
                }
                Err(e) => {
                    failure = json!(e.to_string());
                    break;
                }
            }
            done = m + 1;
            let st = session.state();
            let amp = st.y.iter().map(|v| v[0].abs()).fold(0.0_f64, f64::max) / norm0;
            amp_max = amp_max.max(amp);
            let err = (st.y[5][0] - exact(st.t)).abs();
            if pr {
                err_max = err_max.max(err);
            }
            if (m + 1) % p == 0 && period_log.len() < 400 {
                period_log.push(json!([m + 1, amp.log10()]));
            }
        }
        json!({"steps_done": done, "amp_max": amp_max, "err_max": err_max,
               "failure": failure, "period_log10_amp": period_log,
               "t_end": session.state().t})
    };
    let forced = run(&seq);
    let constant = if pr { run(&[1.0]) } else { Value::Null };
    emit(json!({"mode": "forced", "label": label, "kind": kind, "cap": cap, "seq": seq,
                "z0": z0, "lambda": lambda, "h0": h0, "steps": steps,
                "unit_start": unit_start, "forced": forced, "constant": constant}));
}

fn main() {
    let (mode, kv) = args();
    match mode.as_str() {
        "selftest" => mode_selftest(),
        "fidelity" => mode_fidelity(&kv),
        "counts" => mode_counts(&kv),
        "cost" => repeated(&kv, "cost"),
        "timed" => repeated(&kv, "timed"),
        "calibrate" => mode_calibrate(&kv),
        "forced" => mode_forced(&kv),
        other => {
            eprintln!("unknown mode {other:?}");
            std::process::exit(2);
        }
    }
}
