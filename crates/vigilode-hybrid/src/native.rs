use crate::{
    DerivativeMode, Endpoint, Integration, IntegrationConfig, Model, Result, Work, integrate,
};
use rodas5p_core::{
    CoreError, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters,
    rodas5p_coefficients,
};
use rodas5p_integrators::{OdeProblem, ParallelExecution, sequential_step};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use vigilode_coupled::{Config as RootConfig, coupled_step, coupled_step_residual_first};
#[derive(Debug, serde::Serialize)]
pub struct NativeRun {
    pub integration: Integration,
    pub route: String,
    pub workers: usize,
    pub root_receipts: Vec<Value>,
}
fn structured_endpoint(m: &Model, y: &[f64], h: f64, w: &mut Work) -> Result<Vec<f64>> {
    let c = rodas5p_coefficients().map_err(|e| e.to_string())?;
    let n = m.n();
    let shift = h * c.gamma;
    let factor = m.shift_factor(y, shift, w)?;
    let mut u = vec![vec![0.0; n]; 8];
    for i in 0..8 {
        let mut state = y.to_vec();
        let mut mix = vec![0.0; n];
        for j in 0..i {
            for k in 0..n {
                state[k] += c.a[(i, j)] * u[j][k];
                mix[k] += c.c_matrix[(i, j)] * u[j][k];
            }
        }
        let mut b = vec![0.0; n];
        m.rhs_into(&state, &mut b, w)?;
        for k in 0..n {
            b[k] = shift * b[k] + c.gamma * mix[k];
        }
        u[i] = factor.solve(&b, w)?;
        let mut image = vec![0.0; n];
        m.jvp_into(y, &u[i], &mut image, DerivativeMode::Algebraic, w)?;
        let mut residual = 0.0_f64;
        let mut scale = 0.0_f64;
        for k in 0..n {
            residual = residual.max((u[i][k] - shift * image[k] - b[k]).abs());
            scale = scale.max(u[i][k].abs() + shift * image[k].abs() + b[k].abs());
        }
        if !residual.is_finite() || residual > 1e-9 * scale.max(f64::MIN_POSITIVE) {
            return Err(format!(
                "STRUCTURED_CURRENT_TARGET_RESIDUAL:{residual:e}/{scale:e}"
            ));
        }
    }
    let mut out = y.to_vec();
    for i in 0..8 {
        for k in 0..n {
            out[k] += c.b_code[i] * u[i][k];
        }
    }
    m.check(&out)?;
    Ok(out)
}
fn problem(m: &Model, mode: DerivativeMode, cost: Arc<Mutex<Work>>) -> Result<OdeProblem> {
    let a = m.clone();
    let b = m.clone();
    let ra = cost.clone();
    let rb = cost;
    OdeProblem::new(
        "generated-hybrid-path",
        m.n(),
        Arc::new(move |_, y, out| {
            let mut w = Work::default();
            let r = a.rhs_into(y, out, &mut w);
            ra.lock()
                .map_err(|_| CoreError::InvalidInput("callback counter lock".into()))?
                .accumulate(&w);
            r.map_err(CoreError::InvalidInput)
        }),
        None,
        None,
        Some(Arc::new(move |_, y, v, out| {
            let mut w = Work::default();
            let r = b.jvp_into(y, v, out, mode, &mut w);
            rb.lock()
                .map_err(|_| CoreError::InvalidInput("callback counter lock".into()))?
                .accumulate(&w);
            r.map_err(CoreError::InvalidInput)
        })),
        None,
        true,
        None,
        None,
    )
    .map_err(|e| e.to_string())
}
/// Finite route choice by exact algebraic capability, not future/reference data.
/// Imported coupled failures retain their work before the protected MF fallback.
pub fn solve_trajectory(
    m: &Model,
    y0: &[f64],
    times: &[f64],
    cfg: &IntegrationConfig,
    route: &str,
    workers: usize,
) -> Result<NativeRun> {
    if ![
        "eager-full",
        "lazy-full",
        "lazy-local",
        "sequential",
        "structured",
        "auto",
        "cubic-auto",
    ]
    .contains(&route)
    {
        return Err("UNKNOWN_HYBRID_ROUTE".into());
    }
    if ![1, 2, 4].contains(&workers) {
        return Err("UNSUPPORTED_WORKER_COUNT".into());
    }
    let mut cfg = cfg.clone();
    cfg.quintic = route != "cubic-auto";
    cfg.derivative_mode = if route == "eager-full" || route == "lazy-full" {
        DerivativeMode::FullAd
    } else if m.admits_packet() && (route == "auto" || route == "cubic-auto") {
        DerivativeMode::Algebraic
    } else {
        DerivativeMode::LocalAd
    };
    let callback_cost = Arc::new(Mutex::new(Work::default()));
    let p = problem(m, cfg.derivative_mode, callback_cost.clone())?;
    let execution = if workers == 1 {
        ParallelExecution::sequential()
    } else {
        ParallelExecution::rayon(workers).map_err(|e| e.to_string())?
    };
    let rootcfg = RootConfig {
        rtol: cfg.abs_tolerance * 0.05,
        atol: cfg.abs_tolerance * 0.005,
        ..RootConfig::default()
    };
    let linear = LinearSolverConfig {
        method: LinearMethod::Gmres,
        preconditioner: PreconditionerKind::None,
        rtol: 1e-12,
        atol: 1e-14,
        maxiter: 512,
        ..LinearSolverConfig::default()
    };
    let mut receipts = Vec::new();
    let integration = integrate(m, y0, times, &cfg, |t, y, h, w| {
        if (route == "auto" || route == "cubic-auto") && m.admits_packet() {
            return Ok(Endpoint {
                state: m.packet_counted(y, h, w)?,
                method: "exact-collective-candidate".into(),
            });
        }
        if ["structured", "auto", "cubic-auto"].contains(&route) {
            return Ok(Endpoint {
                state: structured_endpoint(m, y, h, w)?,
                method: "band-plus-rank-one-local-AD-raw-U".into(),
            });
        }
        *callback_cost.lock().map_err(|_| "counter lock")? = Work::default();
        let mut state = None;
        let mut label = route.to_owned();
        if route != "sequential" {
            let a = if route == "eager-full" {
                coupled_step(&p, t, y, h, &rootcfg, &execution)
            } else {
                coupled_step_residual_first(&p, t, y, h, &rootcfg, &execution)
            };
            receipts.push(json!({"kind":route,"t":t,"h":h,"work":a.work,"failure":a.failure,"root_candidate":a.candidate.is_some(),"lte_flag":a.candidate.as_ref().map(|c|c.embedded_test_passes)}));
            if let Some(c) = a.candidate {
                state = Some(c.y_new);
            } else {
                w.candidate_failures += 1;
                label = "coupled-declined-then-sequential-MF".into();
            }
        }
        let result = if let Some(x) = state {
            Ok(x)
        } else {
            let mut oldwork = WorkCounters::default();
            let s = sequential_step(
                &p,
                t,
                y,
                h,
                &linear,
                None,
                rootcfg.atol,
                rootcfg.rtol,
                false,
                &mut oldwork,
            );
            receipts.push(json!({"kind":"sequential-native-K","t":t,"h":h,"work":oldwork,"completed":s.is_ok(),"lte_flag":s.as_ref().ok().map(|s|s.accepted)}));
            s.map(|s| s.y_new).map_err(|e| e.to_string())
        };
        w.accumulate(&callback_cost.lock().map_err(|_| "counter lock")?);
        Ok(Endpoint {
            state: result?,
            method: label,
        })
    })?;
    Ok(NativeRun {
        integration,
        route: route.into(),
        workers,
        root_receipts: receipts,
    })
}
