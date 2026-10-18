//! Isolated JSON process adapter for the component benchmark.
#[path = "component_benchmark_support/mod.rs"]
mod support;
use rodas5p_integrators::{
    AdaptiveStepConfig, BandedJacobian, BdfConfig, BdfOrder, BlockMethod, ControllerKind, ExponentialKrylovConfig,
    FusedPhiKrylovConfig, OutputSamplingPlan, OutputSchedule, ParallelExecution, PredictorKind,
    FastLuPolicy, RadauConfig, RadauIiaStages, Rodas5pFastOptions, SabrConfig, SmallProblem, StageHistory, StructureDeclaration,
    exprb2_step, exprb43_step, integrate_bdf_fixed_dense_observed,
    integrate_pexprb54s4_fused_adaptive_observed, integrate_radau_fixed_dense_observed,
    integrate_rodas5p_fast_banded_observed, integrate_rodas5p_fast_observed_with_options,
    integrate_rodas5p_fast_small_observed, integrate_rodas5p_mf_fast_observed,
    integrate_rodas5p_mf_fast_observed_gmres_into,
    integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace, integrate_rodas5p_routed_observed,
    integrate_sequential_matrix_free_adaptive_dense_observed, sabr_step,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    env, fs,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};
use support::{Case, Request};

struct Small<const N: usize>(Case);
impl<const N: usize> SmallProblem<N> for Small<N> {
    fn rhs(&self, y: &[f64; N], out: &mut [f64; N]) {
        if self.0.rhs(0.0, y, out).is_err() {
            out.fill(f64::NAN);
        }
    }
    fn jacobian(&self, y: &[f64; N], out: &mut [[f64; N]; N]) {
        let mut basis = [0.0; N];
        let mut column = [0.0; N];
        for j in 0..N {
            basis[j] = 1.0;
            if self.0.jvp_into(0.0, y, &basis, &mut column).is_err() {
                for row in out.iter_mut() {
                    row.fill(f64::NAN);
                }
                return;
            }
            for i in 0..N {
                out[i][j] = column[i];
            }
            basis[j] = 0.0;
        }
    }
}
fn small_run<const N: usize>(
    case: &Case,
    span: (f64, f64),
    adaptive: &AdaptiveStepConfig,
    output: &OutputSchedule,
) -> Result<Value, String> {
    let y: [f64; N] = case
        .initial_state
        .as_slice()
        .try_into()
        .map_err(|_| "small dimension mismatch".to_string())?;
    let r = integrate_rodas5p_fast_small_observed(
        &Small::<N>(case.clone()),
        span,
        &y,
        adaptive,
        output,
    )
    .map_err(|e| e.to_string())?;
    let status = if r.observed.success && r.observed.t == case.output_times {
        "OK"
    } else {
        "NUMERICAL_FAILURE"
    };
    Ok(
        json!({"status":status,"reason":r.observed.message,"times":r.observed.t,"states":r.observed.y,"counters":r.observed.counters,"solver_details":{"driver":r.driver,"attempts":r.attempts,"accepted_steps":r.accepted_steps,"rejected_steps":r.rejected_steps},"timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"},"allocations":{"solve":null,"positive_control":null,"scope":"entire solve"},"certificate":null}),
    )
}
fn exponential_fixed(
    case: &Case,
    problem: &rodas5p_integrators::OdeProblem,
    method: &str,
    h_divisor: usize,
) -> Result<Value, String> {
    if !case.autonomous() {
        return Ok(result(
            "UNSUPPORTED_CAPABILITY",
            "exponential step API supports autonomous ODEs only",
        ));
    }
    let mut t = case.t_span[0];
    let end = case.t_span[1];
    let h = (end - t) / h_divisor as f64;
    let mut y = case.initial_state.clone();
    let mut times = Vec::with_capacity(case.output_times.len());
    let mut states = Vec::with_capacity(case.output_times.len());
    let mut work = rodas5p_core::WorkCounters::default();
    let mut steps = 0_usize;
    let mut clipped = 0_usize;
    for &target in &case.output_times {
        while t < target {
            if steps >= 100_000 {
                return Ok(result(
                    "NUMERICAL_FAILURE",
                    "exponential fixed-step budget exhausted",
                ));
            }
            let step = (target - t).min(h);
            if step < h {
                clipped += 1;
            }
            let report = if method == "native-exprb2" {
                exprb2_step(problem, t, &y, step, ExponentialKrylovConfig::default())
            } else {
                exprb43_step(problem, t, &y, step, ExponentialKrylovConfig::default())
            }
            .map_err(|e| e.to_string())?;
            y = report.y_new;
            work.accumulate(report.work);
            steps += 1;
            let next = t + step;
            if next <= t {
                return Ok(result(
                    "NUMERICAL_FAILURE",
                    "exponential fixed-step clock did not advance",
                ));
            }
            t = next;
        }
        times.push(target);
        states.push(y.clone());
    }
    Ok(
        json!({"status":"OK","reason":"success","times":times,"states":states,"counters":work,"solver_details":{"driver":method,"fixed_h":h,"h_divisor":h_divisor,"output_clipped_steps":clipped,"steps":steps,"h_policy":"fixed h with output clipping; compare across declared refinement ladder"},"timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"},"allocations":{"solve":null,"positive_control":null,"scope":"entire solve"},"certificate":null}),
    )
}

struct Counting;
static ON: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if ON.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;

fn result(status: &str, reason: &str) -> Value {
    json!({"status":status,"reason":reason,"times":[],"states":[],"counters":null,"timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"},"allocations":{"solve":null,"positive_control":null,"scope":"entire solve; unavailable on failure"}})
}

fn solve(case: &Case, solver: &str, config: &Value, h_divisor: usize) -> Result<Value, String> {
    if let Some(fields) = config.as_object() {
        if let Some(key) = fields.keys().find(|key| !matches!(key.as_str(), "controller" | "lu_policy" | "threads" | "h_divisor")) {
            return Ok(result("ENVIRONMENT_FAILURE", &format!("unknown solver_config field: {key}")));
        }
    } else if !config.is_null() {
        return Ok(result("ENVIRONMENT_FAILURE", "solver_config must be an object"));
    }
    if config.get("h_divisor").is_some() && !matches!(solver, "native-bdf1" | "native-bdf2" | "native-radau1" | "native-radau3" | "native-exprb2" | "native-exprb43") {
        return Ok(result("UNSUPPORTED_CAPABILITY", "fixed-step h_divisor does not apply to this driver"));
    }
    if config.get("controller").is_some() && !matches!(solver, "native-rodas5p-small" | "native-rodas5p-routed-dense" | "native-rodas5p-routed-matrix-free" | "native-rodas5p-fast" | "rodas5p-fast" | "native-rodas5p-dense" | "native-rodas5p-banded" | "native-rodas5p-matrix-free" | "native-rodas5p-mf-fast" | "native-rodas5p-mf-gmres-into" | "native-rodas5p-mf-ls-workspace" | "native-pexprb54s4") {
        return Ok(result("UNSUPPORTED_CAPABILITY", "adaptive controller does not apply to this driver"));
    }
    if case.mass_matrix.is_some() {
        return Ok(result(
            "UNSUPPORTED_CAPABILITY",
            "native benchmark adapter has no DAE/mass-matrix route",
        ));
    }
    if !case.supported() {
        return Ok(result(
            "UNSUPPORTED_CAPABILITY",
            "model has no established native real-vector equation adapter",
        ));
    }
    if case.atol.len() != case.initial_state.len()
        || case.atol.iter().any(|a| !a.is_finite() || *a < 0.0)
    {
        return Ok(result("ENVIRONMENT_FAILURE", "invalid vector atol"));
    }
    // The native adaptive controller currently has scalar atol. Exact per-component
    // tolerance equivalence is required before admitting a heterogeneous vector.
    if case.atol.iter().any(|a| *a != case.atol[0]) {
        return Ok(result(
            "UNSUPPORTED_CAPABILITY",
            "native scalar-atol controller cannot preserve vector atol",
        ));
    }
    let output = OutputSchedule::new(case.output_times.clone()).map_err(|e| e.to_string())?;
    let span = (case.t_span[0], case.t_span[1]);
    let controller = match config.get("controller").and_then(Value::as_str).unwrap_or("integral") {
        "integral" => ControllerKind::Integral,
        "pi" => ControllerKind::Pi,
        "predictive" => ControllerKind::Predictive,
        "predictive-capped" => ControllerKind::PredictiveCapped,
        "predictive-capped2" => ControllerKind::PredictiveCapped2,
        _ => return Ok(result("ENVIRONMENT_FAILURE", "invalid controller kind")),
    };
    let lu_policy = match config.get("lu_policy").and_then(Value::as_str).unwrap_or("legacy") {
        "legacy" => FastLuPolicy::Legacy,
        "column-extents" => FastLuPolicy::ColumnExtents,
        "column-extents-above64" => FastLuPolicy::ColumnExtentsAbove64,
        _ => return Ok(result("ENVIRONMENT_FAILURE", "invalid fast LU policy")),
    };
    let fast_options = Rodas5pFastOptions { lu_policy, ..Default::default() };
    let adaptive = AdaptiveStepConfig {
        rtol: case.rtol,
        atol: case.atol[0],
        controller,
        initial_step: ((span.1 - span.0) / 1000.0).max(1e-10),
        max_step: span.1 - span.0,
        max_attempts: 100_000,
        ..Default::default()
    };
    if lu_policy != FastLuPolicy::Legacy && !matches!(solver, "native-rodas5p-fast" | "rodas5p-fast" | "native-rodas5p-dense") {
        return Ok(result("UNSUPPORTED_CAPABILITY", "fast LU policy is available only on the dense fast driver"));
    }
    if config.get("threads").is_some() && solver != "native-pexprb54s4" {
        return Ok(result("UNSUPPORTED_CAPABILITY", "parallel execution context is available only on PEXPRB54S4 in this adapter"));
    }
    if solver == "native-rodas5p-small" {
        if !case.autonomous() {
            return Ok(result(
                "UNSUPPORTED_CAPABILITY",
                "small fixed-array API is autonomous only",
            ));
        }
        return match case.initial_state.len() {
            1 => small_run::<1>(case, span, &adaptive, &output),
            2 => small_run::<2>(case, span, &adaptive, &output),
            3 => small_run::<3>(case, span, &adaptive, &output),
            4 => small_run::<4>(case, span, &adaptive, &output),
            8 => small_run::<8>(case, span, &adaptive, &output),
            16 => small_run::<16>(case, span, &adaptive, &output),
            32 => small_run::<32>(case, span, &adaptive, &output),
            64 => small_run::<64>(case, span, &adaptive, &output),
            _ => Ok(result(
                "UNSUPPORTED_CAPABILITY",
                "no compiled small dimension for this case",
            )),
        };
    }
    let problem = case.problem().map_err(|e| e.to_string())?;
    if solver == "native-exprb2" || solver == "native-exprb43" {
        return exponential_fixed(case, &problem, solver, h_divisor);
    }
    let y = &case.initial_state;
    let (observed, extra) = match solver {
        "native-rodas5p-routed-matrix-free" => {
            let jvp_only = problem.jvp_only_clone().map_err(|e| e.to_string())?;
            let r = integrate_rodas5p_routed_observed(&jvp_only, span, y, &adaptive, &output)
                .map_err(|e| e.to_string())?;
            (r.run.observed().clone(), json!({"driver":"structure-router","routing":r.routing,"attempts":r.run.attempts(),"accepted_steps":r.run.accepted_steps(),"rejected_steps":r.run.rejected_steps()}))
        }
        "native-rodas5p-routed-dense" => {
            let declared = problem
                .clone()
                .with_declared_structure(StructureDeclaration::dense(y.len()))
                .map_err(|e| e.to_string())?;
            let r = integrate_rodas5p_routed_observed(&declared, span, y, &adaptive, &output)
                .map_err(|e| e.to_string())?;
            (
                r.run.observed().clone(),
                json!({"driver":"structure-router","routing":r.routing,"attempts":r.run.attempts(),"accepted_steps":r.run.accepted_steps(),"rejected_steps":r.run.rejected_steps()}),
            )
        }
        "native-rodas5p-fast" | "rodas5p-fast" | "native-rodas5p-dense" => {
            let r = integrate_rodas5p_fast_observed_with_options(&problem, span, y, &adaptive, &output, fast_options)
                .map_err(|e| e.to_string())?;
            (
                r.observed,
                json!({"driver":r.driver,"attempts":r.attempts,"accepted_steps":r.accepted_steps,"rejected_steps":r.rejected_steps,"jacobian_reuses":r.jacobian_reuses,"actual_lu":r.lu}),
            )
        }
        "native-rodas5p-banded" => {
            // A full-width band is deliberately refused: it would relabel dense work.
            if !matches!(
                case.model.as_str(),
                "diagonal_stiff" | "periodic_heat" | "advection_diffusion" | "brusselator_1d"
            ) {
                return Ok(result(
                    "UNSUPPORTED_CAPABILITY",
                    "no declared narrow band for this model",
                ));
            }
            let n = y.len();
            if case.model == "periodic_heat" || case.model == "advection_diffusion" {
                return Ok(result(
                    "UNSUPPORTED_CAPABILITY",
                    "periodic wrap-around is not a narrow band",
                ));
            }
            let (lower, upper) = if case.model == "diagonal_stiff" {
                (0, 0)
            } else {
                (2, 2)
            };
            let c = case.clone();
            let band = BandedJacobian {
                lower,
                upper,
                fill: Arc::new(move |_t, state, out| {
                    out.fill(0.0);
                    if c.model == "diagonal_stiff" {
                        let rates = c.parameters["rates"].as_array().ok_or_else(|| {
                            rodas5p_core::CoreError::InvalidInput("diagonal rates absent".into())
                        })?;
                        for i in 0..n {
                            out[i] = rates[i].as_f64().ok_or_else(|| {
                                rodas5p_core::CoreError::InvalidInput(
                                    "diagonal rate invalid".into(),
                                )
                            })?;
                        }
                    } else {
                        let d = c.parameters["diffusion_factor"].as_f64().ok_or_else(|| {
                            rodas5p_core::CoreError::InvalidInput("diffusion factor absent".into())
                        })?;
                        let width = lower + upper + 1;
                        let put = |out: &mut [f64], row: usize, col: usize, value: f64| {
                            out[row * width + col + lower - row] = value;
                        };
                        for cell in 0..n / 2 {
                            let urow = 2 * cell;
                            let vrow = urow + 1;
                            let u = state[urow];
                            let v = state[vrow];
                            put(out, urow, urow, 2.0 * u * v - 4.0 - 2.0 * d);
                            put(out, urow, vrow, u * u);
                            put(out, vrow, urow, 3.0 - 2.0 * u * v);
                            put(out, vrow, vrow, -u * u - 2.0 * d);
                            if cell > 0 {
                                put(out, urow, urow - 2, d);
                                put(out, vrow, vrow - 2, d);
                            }
                            if cell + 1 < n / 2 {
                                put(out, urow, urow + 2, d);
                                put(out, vrow, vrow + 2, d);
                            }
                        }
                    }
                    Ok(())
                }),
            };
            let r = integrate_rodas5p_fast_banded_observed(
                &problem, &band, span, y, &adaptive, &output,
            )
            .map_err(|e| e.to_string())?;
            (
                r.fast.observed,
                json!({"driver":r.fast.driver,"attempts":r.fast.attempts,"accepted_steps":r.fast.accepted_steps,"rejected_steps":r.fast.rejected_steps,"band_work":r.work}),
            )
        }
        "native-rodas5p-matrix-free" => {
            let linear = rodas5p_core::LinearSolverConfig {
                method: rodas5p_core::LinearMethod::Gmres,
                ..Default::default()
            };
            let r = integrate_sequential_matrix_free_adaptive_dense_observed(
                &problem,
                span,
                y,
                &linear,
                &adaptive,
                &OutputSamplingPlan::dense(output),
            )
            .map_err(|e| e.to_string())?;
            (
                r.observed,
                json!({"driver":"sequential-matrix-free-dense","attempts":r.diagnostics.attempts,"rejected_steps":r.diagnostics.rejected_step_sizes.len()}),
            )
        }
        "native-rodas5p-mf-fast"
        | "native-rodas5p-mf-gmres-into"
        | "native-rodas5p-mf-ls-workspace" => {
            let linear = rodas5p_core::LinearSolverConfig {
                method: rodas5p_core::LinearMethod::Gmres,
                ..Default::default()
            };
            let r = match solver {
                "native-rodas5p-mf-fast" => integrate_rodas5p_mf_fast_observed(
                    &problem, span, y, &linear, &adaptive, &output,
                ),
                "native-rodas5p-mf-gmres-into" => integrate_rodas5p_mf_fast_observed_gmres_into(
                    &problem, span, y, &linear, &adaptive, &output,
                ),
                _ => integrate_rodas5p_mf_fast_observed_gmres_into_ls_workspace(
                    &problem, span, y, &linear, &adaptive, &output,
                ),
            }
            .map_err(|e| e.to_string())?;
            (
                r.observed,
                json!({"driver":r.driver,"attempts":r.attempts,"accepted_steps":r.accepted_steps,"rejected_steps":r.rejected_steps,"krylov_workspace":r.krylov_workspace}),
            )
        }
        "native-pexprb54s4" => {
            if !case.autonomous() {
                return Ok(result(
                    "UNSUPPORTED_CAPABILITY",
                    "exponential driver supports autonomous ODEs only",
                ));
            }
            let threads = match config.get("threads").and_then(Value::as_u64).unwrap_or(1) {
                n @ (1 | 2 | 4 | 8) => n as usize,
                _ => return Ok(result("ENVIRONMENT_FAILURE", "invalid parallel thread count")),
            };
            let execution = ParallelExecution::rayon(threads).map_err(|e| e.to_string())?;
            let r = integrate_pexprb54s4_fused_adaptive_observed(
                &problem,
                span,
                y,
                &adaptive,
                &output,
                FusedPhiKrylovConfig::default(),
                &execution,
            )
            .map_err(|e| e.to_string())?;
            (
                r.observed,
                json!({"driver":"pexprb54s4-fused-adaptive","attempts":r.diagnostics.attempts,"accepted_steps":r.diagnostics.accepted_steps,"rejected_steps":r.diagnostics.rejected_steps,"threads":execution.threads(),"parallel_backend":execution.backend()}),
            )
        }
        "native-sabr" => {
            if case.model != "sabr" {
                return Ok(result(
                    "UNSUPPORTED_CAPABILITY",
                    "SABR special fixture only",
                ));
            }
            if case.output_times != vec![span.0, span.1] {
                return Ok(result(
                    "UNSUPPORTED_CAPABILITY",
                    "SABR fixture requires single-step endpoint mesh",
                ));
            }
            let source_config = &case.algorithm_config;
            if source_config["predictor"] != "Zero"
                || source_config["block_method"] != "Nilpotent"
                || source_config["max_iterations"] != 3
                || source_config["defect_budget_fraction"] != 0.1
                || source_config["fallback"] != "Direct"
                || source_config["force_accept"] != true
            {
                return Ok(result(
                    "UNSUPPORTED_CAPABILITY",
                    "SABR source configuration identity mismatch",
                ));
            }
            let mut work = rodas5p_core::WorkCounters::default();
            let cfg = SabrConfig {
                predictor: PredictorKind::Zero,
                max_iterations: 3,
                block_method: BlockMethod::Nilpotent,
                defect_budget_fraction: 0.1,
                ..Default::default()
            };
            let direct = rodas5p_core::LinearSolverConfig {
                method: rodas5p_core::LinearMethod::Direct,
                ..Default::default()
            };
            let step = sabr_step(
                &problem,
                span.0,
                y,
                span.1 - span.0,
                &cfg,
                Some(&direct),
                &mut StageHistory::default(),
                None,
                adaptive.atol,
                adaptive.rtol,
                true,
                &mut work,
            )
            .map_err(|e| e.to_string())?;
            let accepted = step.accepted;
            let observed = rodas5p_integrators::ObservedIntegrationResult {
                t: if accepted {
                    vec![span.0, step.t_new]
                } else {
                    vec![span.0]
                },
                y: if accepted {
                    vec![y.clone(), step.y_new]
                } else {
                    vec![y.clone()]
                },
                success: accepted,
                message: if accepted {
                    "success".into()
                } else {
                    "SABR source step rejected under frozen benchmark tolerance".into()
                },
                counters: work,
                internal_steps: usize::from(accepted),
                output_clipped_steps: 0,
            };
            (
                observed,
                json!({"driver":"sabr-nilpotent-3","attempts":1,"error_norm":step.error_norm,"used_fallback":step.used_fallback,"force_accept":true,"acceptance_scope":"forced source step, not tolerance-controlled acceptance","certificate_scope":"stage only; no ODE trajectory certificate"}),
            )
        }
        "native-bdf1" | "native-bdf2" => {
            let config = BdfConfig {
                order: if solver.ends_with('1') {
                    BdfOrder::One
                } else {
                    BdfOrder::Two
                },
                ..Default::default()
            };
            let h = (span.1 - span.0) / h_divisor as f64;
            let r = integrate_bdf_fixed_dense_observed(
                &problem,
                span,
                y,
                h,
                &config,
                &OutputSamplingPlan::dense(output),
            )
            .map_err(|e| e.to_string())?;
            (
                r,
                json!({"driver":solver,"fixed_h":h,"h_divisor":h_divisor,"h_policy":"declared refinement ladder level"}),
            )
        }
        "native-radau1" | "native-radau3" => {
            let config = RadauConfig {
                stages: if solver.ends_with('1') {
                    RadauIiaStages::One
                } else {
                    RadauIiaStages::Three
                },
                ..Default::default()
            };
            let h = (span.1 - span.0) / h_divisor as f64;
            let r = integrate_radau_fixed_dense_observed(
                &problem,
                span,
                y,
                h,
                &config,
                &OutputSamplingPlan::dense(output),
            )
            .map_err(|e| e.to_string())?;
            (
                r,
                json!({"driver":solver,"fixed_h":h,"h_divisor":h_divisor,"h_policy":"declared refinement ladder level"}),
            )
        }
        _ => {
            return Ok(result(
                "UNSUPPORTED_CAPABILITY",
                "no native solver route with this identifier",
            ));
        }
    };
    let status = if observed.success && observed.t == case.output_times {
        "OK"
    } else {
        "NUMERICAL_FAILURE"
    };
    Ok(
        json!({"status":status,"reason":observed.message,"times":observed.t,"states":observed.y,"counters":observed.counters,"solver_details":extra,"timing":{"elapsed_ns":null,"timing_authority":"diagnostic_only"},"allocations":{"solve":null,"positive_control":null,"scope":"entire solve"},"certificate":null}),
    )
}

fn run() -> Value {
    let args: Vec<String> = env::args().collect();
    if args.len() == 4 && args[1] == "--probe" && args[2] == "--request" {
        return match fs::read_to_string(&args[3])
            .ok()
            .and_then(|s| serde_json::from_str::<Request>(&s).ok())
        {
            Some(req) => req.case.probe(&req.probes),
            None => result("ENVIRONMENT_FAILURE", "probe request read/parse failed"),
        };
    }
    if !(args.len() == 3 || (args.len() == 5 && args[3] == "--measurement-mode"))
        || args[1] != "--request"
    {
        return result(
            "ENVIRONMENT_FAILURE",
            "usage: component_benchmark_native --request PATH [--measurement-mode timing|allocation]",
        );
    }
    let mode = if args.len() == 5 {
        args[4].as_str()
    } else {
        "timing"
    };
    if mode != "timing" && mode != "allocation" {
        return result("ENVIRONMENT_FAILURE", "invalid measurement mode");
    }
    let request: Request = match fs::read_to_string(&args[2])
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(x) => x,
        None => return result("ENVIRONMENT_FAILURE", "request read/parse failed"),
    };
    let h_divisor = match request.solver_config.get("h_divisor") {
        None => 1000,
        Some(v) => match v.as_u64() {
            Some(n @ (1000 | 2000 | 4000 | 8000)) => n as usize,
            _ => return result("ENVIRONMENT_FAILURE", "invalid fixed-step h_divisor"),
        },
    };
    let invoke = || {
        let before = request.case.derivative_rhs_calls();
        let mut value = match solve(&request.case, &request.solver_id, &request.solver_config, h_divisor) {
            Ok(v) => v,
            Err(e) => result("NUMERICAL_FAILURE", &e),
        };
        value["derivative_mode"] = json!(request.case.derivative_policy());
        value["derivative_policy"] = json!(request.case.derivative_policy());
        value["requested_solver_config"] = request.solver_config.clone();
        value["derivative_work"] = json!({"extra_rhs_calls":request.case.derivative_rhs_calls()-before,"included_in_solver_counters":false});
        value
    };
    if mode == "allocation" {
        COUNT.store(0, Ordering::SeqCst);
        ON.store(true, Ordering::SeqCst);
        let mut value = invoke();
        ON.store(false, Ordering::SeqCst);
        let count = COUNT.load(Ordering::SeqCst);
        let before = COUNT.load(Ordering::SeqCst);
        ON.store(true, Ordering::SeqCst);
        let positive = vec![1_u8; 1024];
        std::hint::black_box(&positive);
        ON.store(false, Ordering::SeqCst);
        let control = COUNT.load(Ordering::SeqCst) - before;
        value["measurement_mode"] = json!("allocation");
        value["allocations"] = json!({"solve":count,"positive_control":control,"scope":"entire solve including setup/output; separate process","calibrated":control>0});
        return value;
    }
    let warm_start = Instant::now();
    let warm = invoke();
    let warm_ns = warm_start.elapsed().as_nanos();
    if warm["status"] != "OK" {
        let mut value = warm;
        value["measurement_mode"] = json!("timing");
        value["timing_scope"] = json!("warmed_solver_setup_integration_output_certificate");
        value["timing"] = json!({"elapsed_ns":null,"timed_seconds":null,"warmup_seconds":warm_ns as f64*1e-9,"timing_authority":"diagnostic_only","timing_scope":"warmed_solver_setup_integration_output_certificate"});
        return value;
    }
    let start = Instant::now();
    let mut value = invoke();
    let elapsed = start.elapsed().as_nanos();
    value["measurement_mode"] = json!("timing");
    value["timing_scope"] = json!("warmed_solver_setup_integration_output_certificate");
    value["timing"] = json!({"elapsed_ns":elapsed,"total_seconds":(elapsed as f64)*1e-9,"timed_seconds":(elapsed as f64)*1e-9,"warmup_seconds":(warm_ns as f64)*1e-9,"timing_authority":"diagnostic_only","timing_scope":"warmed_solver_setup_integration_output_certificate"});
    value
}
fn main() {
    println!("{}", run());
}
