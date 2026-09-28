//! E-08: WorkCounters vs atomic ground-truth counters on wrapped RHS/JVP/partial_t/Jacobian closures.
//! Usage: e08_work_counters <out.jsonl>
use rodas5p_core::{CoreResult, DenseMatrix, InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters};
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, FusedPhiKrylovConfig, IntegrationMethod, OdeProblem, OutputSamplingPlan,
    OutputSchedule, ParallelExecution, ScientificCaseSpec, ScientificCorpusV2, ScientificFamily,
    integrate_adaptive_observed_with_config, integrate_fixed, integrate_pexprb54s4_fused_adaptive_observed,
    integrate_sequential_matrix_free_adaptive_dense_observed, integrate_sequential_matrix_free_adaptive_observed,
    prothero_robinson_problem,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
struct Atomics {
    rhs: AtomicU64,
    jvp: AtomicU64,
    ft: AtomicU64,
    jac: AtomicU64,
}
impl Atomics {
    fn reset(&self) {
        for a in [&self.rhs, &self.jvp, &self.ft, &self.jac] {
            a.store(0, Ordering::SeqCst);
        }
    }
    fn snap(&self) -> serde_json::Value {
        serde_json::json!({"rhs": self.rhs.load(Ordering::SeqCst), "jvp": self.jvp.load(Ordering::SeqCst), "ft": self.ft.load(Ordering::SeqCst), "jac": self.jac.load(Ordering::SeqCst)})
    }
}

type RhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type JacFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type FtFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type ExactFn = Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>;

fn wrap(inner: OdeProblem, name: &str, with_jac: bool) -> (OdeProblem, Arc<Atomics>) {
    let at = Arc::new(Atomics::default());
    let inner = Arc::new(inner);
    let dim = inner.dimension;
    let (i1, a1) = (inner.clone(), at.clone());
    let rhs: RhsFn = Arc::new(move |t, y, out: &mut [f64]| {
        a1.rhs.fetch_add(1, Ordering::SeqCst);
        let mut d = WorkCounters::default();
        let v = i1.eval_rhs(t, y, &mut d)?;
        out.copy_from_slice(&v);
        Ok(())
    });
    let (i2, a2) = (inner.clone(), at.clone());
    let jvp: JvpFn = Arc::new(move |t, y, v, out: &mut [f64]| {
        a2.jvp.fetch_add(1, Ordering::SeqCst);
        let op = i2.linearize_matrix_free(t, y)?;
        op.apply(v, out)
    });
    let (i3, a3) = (inner.clone(), at.clone());
    let ft: Option<FtFn> = if inner.autonomous {
        None
    } else {
        Some(Arc::new(move |t, y, out: &mut [f64]| {
            a3.ft.fetch_add(1, Ordering::SeqCst);
            let mut d = WorkCounters::default();
            let v = i3.eval_partial_t(t, y, &mut d)?;
            out.copy_from_slice(&v);
            Ok(())
        }))
    };
    let (i4, a4) = (inner.clone(), at.clone());
    let jac: Option<JacFn> = if with_jac {
        Some(Arc::new(move |t, y| {
            a4.jac.fetch_add(1, Ordering::SeqCst);
            let mut d = WorkCounters::default();
            i4.dense_jacobian(t, y, &mut d)
        }))
    } else {
        None
    };
    let exact: Option<ExactFn> = if inner.exact(0.0).is_some() {
        let i5 = inner.clone();
        Some(Arc::new(move |t| i5.exact(t).unwrap()))
    } else {
        None
    };
    let p = OdeProblem::new(name, dim, rhs, None, jac, Some(jvp), ft, inner.autonomous, inner.mass_matrix.clone(), exact).unwrap();
    (p, at)
}

fn gm(maxiter: usize, restart: usize) -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1e-10,
        atol: 1e-14,
        restart,
        maxiter,
        inner_m: 30,
        outer_k: 8,
        recycle_dim: 8,
        recycle_rank_tol: 1e-12,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
    }
}

fn adaptive(rtol: f64, h0: f64, hmax: f64) -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        atol: rtol * 1e-2,
        rtol,
        initial_step: h0,
        min_step: 1e-14,
        max_step: hmax,
        max_attempts: 200_000,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 5.0,
        reject_max_factor: 0.9,
        controller: ControllerKind::Integral,
    }
}

fn report(out: &mut Vec<String>, problem: &str, arm: &str, ok: bool, msg: &str, c: &WorkCounters, at: &Atomics, extra: serde_json::Value) {
    let a = at.snap();
    let jvp_atomic = at.jvp.load(Ordering::SeqCst);
    let jvp_accounted = c.jvp_calls + c.linear_matvecs + c.recycle_refresh_matvecs + c.diagnostic_matvecs + c.block_matvecs + c.phi_krylov_vectors + c.mass_matvecs;
    let row = serde_json::json!({
        "problem": problem, "arm": arm, "ok": ok, "message": msg,
        "atomics": a,
        "counters": c,
        "check": {
            "rhs_calls_minus_atomic": c.rhs_calls as i64 - at.rhs.load(Ordering::SeqCst) as i64,
            "rhs_evaluations_minus_atomic": c.rhs_evaluations as i64 - at.rhs.load(Ordering::SeqCst) as i64,
            "ft_calls_minus_atomic": c.ft_calls as i64 - at.ft.load(Ordering::SeqCst) as i64,
            "jacobian_builds_minus_atomic_jac": c.jacobian_builds as i64 - at.jac.load(Ordering::SeqCst) as i64,
            "jvp_calls_minus_atomic": c.jvp_calls as i64 - jvp_atomic as i64,
            "jvp_accounted_sum": jvp_accounted,
            "jvp_accounted_minus_atomic": jvp_accounted as i64 - jvp_atomic as i64,
            "linear_matvecs_minus_atomic_jvp": c.linear_matvecs as i64 - jvp_atomic as i64,
        },
        "extra": extra,
    });
    eprintln!("{problem} {arm}: ok={ok} rhs Δ={} ft Δ={} jvp_calls={} lin_mv={} refresh={} diag={} atomic_jvp={} acc-atomic={} acc={} rej={} lsf={} msg={}",
        c.rhs_calls as i64 - at.rhs.load(Ordering::SeqCst) as i64, c.ft_calls as i64 - at.ft.load(Ordering::SeqCst) as i64,
        c.jvp_calls, c.linear_matvecs, c.recycle_refresh_matvecs, c.diagnostic_matvecs, jvp_atomic, jvp_accounted as i64 - jvp_atomic as i64,
        c.accepted_steps, c.rejected_steps, c.linear_solve_failures, msg);
    out.push(row.to_string());
}

fn run_problem(out: &mut Vec<String>, pname: &str, problem: &OdeProblem, at: &Atomics, y0: &[f64], span: (f64, f64)) {
    let (t0, tf) = span;
    let sched = OutputSchedule::uniform(t0, tf, (tf - t0) / 20.0).unwrap();
    let arms: Vec<(&str, LinearSolverConfig, AdaptiveStepConfig)> = vec![
        ("mf_adaptive_reject", gm(200, 40), adaptive(1e-8, 1e-1 * (tf - t0), 1e-1 * (tf - t0))),
        ("mf_adaptive_gmresfail", gm(4, 4), adaptive(1e-8, 1e-1 * (tf - t0), 1e-1 * (tf - t0))),
    ];
    for (arm, cfg, ad) in &arms {
        at.reset();
        match integrate_sequential_matrix_free_adaptive_observed(problem, span, y0, cfg, ad, &sched) {
            Ok(r) => report(out, pname, arm, r.observed.success, &r.observed.message, &r.observed.counters, at, serde_json::json!({"internal_steps": r.observed.internal_steps, "n_out": r.observed.t.len()})),
            Err(e) => report(out, pname, arm, false, &format!("Err: {e}"), &WorkCounters::default(), at, serde_json::json!({"hard_error": true})),
        }
        at.reset();
        let plan = OutputSamplingPlan::dense(sched.clone());
        let darm = arm.replace("mf_adaptive", "mf_dense");
        match integrate_sequential_matrix_free_adaptive_dense_observed(problem, span, y0, cfg, ad, &plan) {
            Ok(r) => report(out, pname, &darm, r.observed.success, &r.observed.message, &r.observed.counters, at, serde_json::json!({"internal_steps": r.observed.internal_steps, "n_out": r.observed.t.len()})),
            Err(e) => report(out, pname, &darm, false, &format!("Err: {e:?}"), &WorkCounters::default(), at, serde_json::json!({"hard_error": true})),
        }
    }
    // (c) exponential candidate
    at.reset();
    let ad = adaptive(1e-8, 1e-1 * (tf - t0), 1e-1 * (tf - t0));
    match integrate_pexprb54s4_fused_adaptive_observed(problem, span, y0, &ad, &sched, FusedPhiKrylovConfig::default(), &ParallelExecution::sequential()) {
        Ok(r) => report(out, pname, "expo_pexprb54s4_fused", r.observed.success, &r.observed.message, &r.observed.counters, at, serde_json::json!({"internal_steps": r.observed.internal_steps})),
        Err(e) => report(out, pname, "expo_pexprb54s4_fused", false, &format!("Err: {e}"), &WorkCounters::default(), at, serde_json::json!({"hard_error": true})),
    }
    // (d) fixed direct and adaptive direct with config
    let direct = LinearSolverConfig::default();
    at.reset();
    let h = (tf - t0) / 50.0;
    match integrate_fixed(problem, span, y0, h, IntegrationMethod::Sequential, Some(&direct), None, 1e-10, 1e-8) {
        Ok(r) => report(out, pname, "fixed_direct", r.success, &r.message, &r.counters, at, serde_json::json!({"steps": r.t.len() - 1, "attempts": r.attempts})),
        Err(e) => report(out, pname, "fixed_direct", false, &format!("Err: {e}"), &WorkCounters::default(), at, serde_json::json!({"hard_error": true})),
    }
    at.reset();
    match integrate_adaptive_observed_with_config(problem, span, y0, IntegrationMethod::Sequential, Some(&direct), None, &ad, &sched) {
        Ok(r) => report(out, pname, "adaptive_direct_reject", r.observed.success, &r.observed.message, &r.observed.counters, at, serde_json::json!({"internal_steps": r.observed.internal_steps})),
        Err(e) => report(out, pname, "adaptive_direct_reject", false, &format!("Err: {e}"), &WorkCounters::default(), at, serde_json::json!({"hard_error": true})),
    }
    at.reset();
    match integrate_adaptive_observed_with_config(problem, span, y0, IntegrationMethod::Sequential, Some(&gm(4, 4)), None, &ad, &sched) {
        Ok(r) => report(out, pname, "adaptive_gmres_gmresfail_generic", r.observed.success, &r.observed.message, &r.observed.counters, at, serde_json::json!({"internal_steps": r.observed.internal_steps})),
        Err(e) => report(out, pname, "adaptive_gmres_gmresfail_generic", false, &format!("Err: {e}"), &WorkCounters::default(), at, serde_json::json!({"hard_error": true})),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = args.get(1).cloned().unwrap_or_else(|| "e08_out.jsonl".into());
    let mut out = Vec::new();
    eprintln!("debug_assertions={}", cfg!(debug_assertions));
    // Prothero-Robinson, JVP-only wrapper and explicit-Jacobian wrapper.
    let (pr, y0) = prothero_robinson_problem(-1e4, 1.0, 0.0);
    let (p_jvp, at) = wrap(pr.clone(), "PR-jvp-only", false);
    run_problem(&mut out, "PR-jvp-only", &p_jvp, &at, &y0, (0.0, 1.0));
    let (p_jac, at2) = wrap(pr, "PR-with-jac", true);
    run_problem(&mut out, "PR-with-jac", &p_jac, &at2, &y0, (0.0, 1.0));
    // robertson-ramped n=96 from ScientificCorpusV2 (segment 0, span clipped to 10 time units).
    let spec = ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|s| matches!(s.family, ScientificFamily::RobertsonRamped) && s.dimension == 96 && s.rtol == 1e-8)
        .expect("robertson-ramped n=96 rtol 1e-8 spec");
    let case = ScientificCorpusV2::build(&spec).expect("build");
    let seg = &case.integration_segments[0];
    let (t0, tf_full) = seg.t_span;
    let tf = tf_full.min(t0 + 10.0);
    eprintln!("robertson spec id={} segments={} seg0 span=({t0},{tf_full}) clipped tf={tf} autonomous={} has_jvp={} has_jac={}", spec.id, case.integration_segments.len(), seg.problem.autonomous, seg.problem.has_jvp(), seg.problem.has_explicit_jacobian());
    let (p_rob, at3) = wrap(seg.problem.clone(), "robertson-ramped-n96-jvp-only", false);
    run_problem(&mut out, "robertson-ramped-n96-jvp-only", &p_rob, &at3, &case.y0, (t0, tf));
    std::fs::write(&out_path, out.join("\n") + "\n").unwrap();
    println!("wrote {out_path} ({} rows) debug_assertions={}", out.len(), cfg!(debug_assertions));
}
