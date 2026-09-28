//! E-04: observed order of RODAS5P under exact (LU) vs inexact (GMRES) stage solves.
//!
//! Usage: e04_order_krylov <problem> <kmin> <kmax> <arm>[,<arm>...]
//!   problem: p1 | p2 | p1ns | p1pr (pure Prothero-Robinson, no initial transient)
//!   arm: direct | direct_embedded | local | gmres:<eta> | forcing:<rtol> | adaptive:<rtol>
//! Emits one JSON object per line (one per (problem, arm, k)).
use std::sync::Arc;
use std::time::Instant;

use rodas5p_core::{
    CoreResult, DenseMatrix, InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind,
    WorkCounters,
};
use rodas5p_integrators::{
    AdaptiveStepConfig, ControllerKind, OdeProblem, OutputSchedule,
    integrate_sequential_matrix_free_adaptive_observed, semilinear_advection_diffusion_problem,
    sequential_matrix_free_step, sequential_matrix_free_step_with_inner_forcing, sequential_step,
};

struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn gauss(&mut self) -> f64 {
        let u1 = self.uniform().max(1e-300);
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Orthonormalize the columns of an n x n matrix (row-major) with modified Gram-Schmidt, twice.
fn orthonormalize(n: usize, g: &mut [f64]) {
    for _pass in 0..2 {
        for j in 0..n {
            for k in 0..j {
                let mut dot = 0.0;
                for i in 0..n {
                    dot += g[i * n + k] * g[i * n + j];
                }
                for i in 0..n {
                    g[i * n + j] -= dot * g[i * n + k];
                }
            }
            let mut nrm = 0.0;
            for i in 0..n {
                nrm += g[i * n + j] * g[i * n + j];
            }
            let nrm = nrm.sqrt();
            for i in 0..n {
                g[i * n + j] /= nrm;
            }
        }
    }
}

struct P1 {
    problem: OdeProblem,
    y0: Vec<f64>,
    exact: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>,
    lam_max_abs: f64,
    orth_defect: f64,
}

fn build_p1(n: usize, lam_lo: f64, lam_hi: f64, seed: u64, pert_amp: f64) -> CoreResult<P1> {
    let mut rng = Rng(seed);
    let mut q = vec![0.0; n * n];
    for v in q.iter_mut() {
        *v = rng.gauss();
    }
    orthonormalize(n, &mut q);
    // orthogonality defect max |Q^T Q - I|
    let mut defect: f64 = 0.0;
    for a in 0..n {
        for b in 0..n {
            let mut s = 0.0;
            for i in 0..n {
                s += q[i * n + a] * q[i * n + b];
            }
            let target = if a == b { 1.0 } else { 0.0 };
            defect = defect.max((s - target).abs());
        }
    }
    // lambda_k log-spaced in [-|lam_hi|, -|lam_lo|]
    let lam: Vec<f64> = (0..n)
        .map(|k| {
            let f = k as f64 / (n - 1) as f64;
            -(lam_lo.abs().ln() * (1.0 - f) + lam_hi.abs().ln() * f).exp()
        })
        .collect();
    // A = Q^T diag(lam) Q, formed explicitly and bit-symmetric.
    let mut a = vec![0.0; n * n];
    for i in 0..n {
        for j in i..n {
            let mut s = 0.0;
            for k in 0..n {
                s += q[k * n + i] * lam[k] * q[k * n + j];
            }
            a[i * n + j] = s;
            a[j * n + i] = s;
        }
    }
    let a_mat = Arc::new(DenseMatrix::new(n, n, a)?);
    let phi: Vec<f64> = (0..n).map(|_| 2.0 * std::f64::consts::PI * rng.uniform()).collect();
    let phi = Arc::new(phi);
    let g = {
        let phi = phi.clone();
        move |t: f64| -> Vec<f64> { phi.iter().map(|p| (t + p).sin()).collect() }
    };
    let gp = {
        let phi = phi.clone();
        move |t: f64| -> Vec<f64> { phi.iter().map(|p| (t + p).cos()).collect() }
    };
    let gpp = {
        let phi = phi.clone();
        move |t: f64| -> Vec<f64> { phi.iter().map(|p| -(t + p).sin()).collect() }
    };
    let g0 = g(0.0);
    let pert: Vec<f64> = (0..n).map(|_| pert_amp).collect();
    let y0: Vec<f64> = g0.iter().zip(&pert).map(|(a, b)| a + b).collect();
    // w = Q p
    let mut w = vec![0.0; n];
    for k in 0..n {
        let mut s = 0.0;
        for i in 0..n {
            s += q[k * n + i] * pert[i];
        }
        w[k] = s;
    }
    let q_arc = Arc::new(q);
    let lam_arc = Arc::new(lam.clone());
    let exact: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = {
        let q = q_arc.clone();
        let lam = lam_arc.clone();
        let g = g.clone();
        let w = w.clone();
        Arc::new(move |t: f64| {
            let mut out = g(t);
            let ew: Vec<f64> = lam.iter().zip(&w).map(|(l, wk)| (l * t).exp() * wk).collect();
            for i in 0..n {
                let mut s = 0.0;
                for k in 0..n {
                    s += q[k * n + i] * ew[k];
                }
                out[i] += s;
            }
            out
        })
    };
    let rhs = {
        let a = a_mat.clone();
        let g = g.clone();
        let gp = gp.clone();
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            let gt = g(t);
            let d: Vec<f64> = y.iter().zip(&gt).map(|(a, b)| a - b).collect();
            a.matvec_into(&d, out)?;
            let gpt = gp(t);
            for i in 0..n {
                out[i] += gpt[i];
            }
            Ok(())
        })
    };
    let jac = {
        let a = a_mat.clone();
        Arc::new(move |_t: f64, _y: &[f64]| Ok((*a).clone()))
    };
    let jvp = {
        let a = a_mat.clone();
        Arc::new(move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| a.matvec_into(v, out))
    };
    let partial_t = {
        let a = a_mat.clone();
        let gp = gp.clone();
        let gpp = gpp.clone();
        Arc::new(move |t: f64, _y: &[f64], out: &mut [f64]| {
            let gpt = gp(t);
            a.matvec_into(&gpt, out)?;
            let gppt = gpp(t);
            for i in 0..n {
                out[i] = -out[i] + gppt[i];
            }
            Ok(())
        })
    };
    let problem = OdeProblem::new(
        format!("e04-p1-n{n}-lam[{lam_lo},{lam_hi}]-pert{pert_amp}"),
        n,
        rhs,
        None,
        Some(jac),
        Some(jvp),
        Some(partial_t),
        false,
        None,
        Some(exact.clone()),
    )?;
    Ok(P1 {
        problem,
        y0,
        exact,
        lam_max_abs: lam_hi.abs(),
        orth_defect: defect,
    })
}

fn inf_norm(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |m, x| m.max(x.abs()))
}

fn gmres_config(rtol: f64, atol: f64) -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol,
        atol,
        restart: 32,
        maxiter: 20_000,
        inner_m: 30,
        outer_k: 8,
        recycle_dim: 8,
        recycle_rank_tol: 1e-12,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
    }
}

#[derive(serde::Serialize)]
struct Row {
    problem: String,
    arm: String,
    k: u32,
    h: f64,
    steps: usize,
    steps_completed: usize,
    status: String,
    message: String,
    err_max_rel_inf: f64,
    err_end_rel_inf: f64,
    err_local_main: Option<f64>,
    err_local_embedded: Option<f64>,
    err_vector_norm: Option<f64>,
    would_reject_steps: usize,
    max_embedded_wrms: f64,
    rhs_calls: u64,
    ft_calls: u64,
    jvp_calls: u64,
    jvp_vectors: u64,
    linear_iterations: u64,
    linear_matvecs: u64,
    direct_factorizations: u64,
    linear_solve_failures: u64,
    eta_min: Option<f64>,
    eta_mean: Option<f64>,
    eta_max: Option<f64>,
    tau_min: Option<f64>,
    tau_mean: Option<f64>,
    tau_max: Option<f64>,
    achieved_residual_wrms_mean: Option<f64>,
    flow_wrms_mean: Option<f64>,
    rhs_wrms_mean: Option<f64>,
    eta_clamped_at_max_fraction: Option<f64>,
    roundoff_floor_estimate: f64,
    adaptive_accepted: Option<usize>,
    adaptive_rejected: Option<usize>,
    adaptive_h_min: Option<f64>,
    adaptive_h_max: Option<f64>,
    wall_s: f64,
}

#[allow(clippy::too_many_arguments)]
fn run_fixed(
    problem: &OdeProblem,
    y0: &[f64],
    exact: &dyn Fn(f64) -> Vec<f64>,
    pname: &str,
    arm: &str,
    k: u32,
    lam_max_abs: f64,
) -> Row {
    let t0 = Instant::now();
    let steps = 1usize << k;
    let h = 1.0 / steps as f64;
    let direct_cfg = LinearSolverConfig::default();
    let mut counters = WorkCounters::default();
    let mut y = y0.to_vec();
    let mut t = 0.0;
    let mut err_max: f64 = 0.0;
    let mut err_end = 0.0;
    let mut would_reject = 0usize;
    let mut max_wrms: f64 = 0.0;
    let mut status = "ok".to_string();
    let mut message = String::new();
    let mut completed = 0usize;
    let mut etas: Vec<f64> = Vec::new();
    let mut taus: Vec<f64> = Vec::new();
    let mut ach: Vec<f64> = Vec::new();
    let mut flows: Vec<f64> = Vec::new();
    let mut rhsw: Vec<f64> = Vec::new();
    let mut local_main = None;
    let mut local_emb = None;
    let mut errvec_norm = None;
    let (kind, param): (&str, f64) = if let Some((a, b)) = arm.split_once(':') {
        (a, b.parse().expect("arm parameter"))
    } else {
        (arm, 0.0)
    };
    // nominal outer tolerances for the reported embedded WRMS (does not affect the solution
    // for direct/gmres arms; for forcing arms they drive the production forcing rule)
    let (atol_o, rtol_o) = match kind {
        "forcing" => (1e-2 * param, param),
        _ => (1e-10, 1e-8),
    };
    let gm_cfg = match kind {
        "gmres" => gmres_config(param, 1e-2 * param),
        _ => gmres_config(1e-10, 1e-12),
    };
    for step in 0..steps {
        let t_new = (step + 1) as f64 * h;
        let hh = t_new - t;
        let res: CoreResult<(Vec<f64>, Vec<f64>, f64)> = match kind {
            "direct" | "direct_embedded" | "local" => sequential_step(
                problem, t, &y, hh, &direct_cfg, None, atol_o, rtol_o, true, &mut counters,
            )
            .map(|r| (r.y_new, r.error_vector, r.error_norm)),
            "gmres" => sequential_matrix_free_step(
                problem, t, &y, hh, &gm_cfg, None, atol_o, rtol_o, true, &mut counters,
            )
            .map(|r| (r.y_new, r.error_vector, r.error_norm)),
            "forcing" => sequential_matrix_free_step_with_inner_forcing(
                problem, t, &y, hh, &gm_cfg, None, atol_o, rtol_o, true, &mut counters,
            )
            .map(|r| {
                for s in &r.stage_forcing {
                    etas.push(s.eta);
                    taus.push(s.tau);
                    ach.push(s.achieved_residual_wrms);
                    flows.push(s.flow_wrms);
                    rhsw.push(s.rhs_wrms);
                }
                (r.step.y_new, r.step.error_vector, r.step.error_norm)
            }),
            _ => panic!("unknown arm {arm}"),
        };
        let (y_new, error_vector, error_norm) = match res {
            Ok(v) => v,
            Err(e) => {
                status = "failed".into();
                message = format!("step {step} t={t}: {e}");
                break;
            }
        };
        if kind == "local" {
            let ex = exact(t_new);
            let nrm = inf_norm(&ex);
            let d_main: Vec<f64> = y_new.iter().zip(&ex).map(|(a, b)| a - b).collect();
            let y_emb: Vec<f64> = y_new.iter().zip(&error_vector).map(|(a, e)| a - e).collect();
            let d_emb: Vec<f64> = y_emb.iter().zip(&ex).map(|(a, b)| a - b).collect();
            local_main = Some(inf_norm(&d_main) / nrm);
            local_emb = Some(inf_norm(&d_emb) / nrm);
            errvec_norm = Some(inf_norm(&error_vector) / nrm);
            completed = 1;
            break;
        }
        if error_norm > 1.0 || !error_norm.is_finite() {
            would_reject += 1;
        }
        max_wrms = max_wrms.max(error_norm);
        y = if kind == "direct_embedded" {
            y_new.iter().zip(&error_vector).map(|(a, e)| a - e).collect()
        } else {
            y_new
        };
        t = t_new;
        completed += 1;
        let ex = exact(t);
        let nrm = inf_norm(&ex);
        let d: Vec<f64> = y.iter().zip(&ex).map(|(a, b)| a - b).collect();
        let e = inf_norm(&d) / nrm;
        err_max = err_max.max(e);
        err_end = e;
    }
    let mean = |v: &[f64]| {
        if v.is_empty() {
            None
        } else {
            Some(v.iter().sum::<f64>() / v.len() as f64)
        }
    };
    let fmin = |v: &[f64]| v.iter().cloned().reduce(f64::min);
    let fmax = |v: &[f64]| v.iter().cloned().reduce(f64::max);
    let gamma = 0.21193756319429014_f64;
    let kappa = 1.0 + h * gamma * lam_max_abs;
    let clamped = if etas.is_empty() {
        None
    } else {
        Some(etas.iter().filter(|e| **e >= 0.5).count() as f64 / etas.len() as f64)
    };
    Row {
        problem: pname.into(),
        arm: arm.into(),
        k,
        h,
        steps,
        steps_completed: completed,
        status,
        message,
        err_max_rel_inf: err_max,
        err_end_rel_inf: err_end,
        err_local_main: local_main,
        err_local_embedded: local_emb,
        err_vector_norm: errvec_norm,
        would_reject_steps: would_reject,
        max_embedded_wrms: max_wrms,
        rhs_calls: counters.rhs_calls,
        ft_calls: counters.ft_calls,
        jvp_calls: counters.jvp_calls,
        jvp_vectors: counters.jvp_vectors,
        linear_iterations: counters.linear_iterations,
        linear_matvecs: counters.linear_matvecs,
        direct_factorizations: counters.direct_factorizations,
        linear_solve_failures: counters.linear_solve_failures,
        eta_min: fmin(&etas),
        eta_mean: mean(&etas),
        eta_max: fmax(&etas),
        tau_min: fmin(&taus),
        tau_mean: mean(&taus),
        tau_max: fmax(&taus),
        achieved_residual_wrms_mean: mean(&ach),
        flow_wrms_mean: mean(&flows),
        rhs_wrms_mean: mean(&rhsw),
        eta_clamped_at_max_fraction: clamped,
        roundoff_floor_estimate: steps as f64 * f64::EPSILON * kappa,
        adaptive_accepted: None,
        adaptive_rejected: None,
        adaptive_h_min: None,
        adaptive_h_max: None,
        wall_s: t0.elapsed().as_secs_f64(),
    }
}

fn run_adaptive(
    problem: &OdeProblem,
    y0: &[f64],
    exact: &dyn Fn(f64) -> Vec<f64>,
    pname: &str,
    arm: &str,
    rtol: f64,
    lam_max_abs: f64,
) -> Row {
    let t0 = Instant::now();
    let cfg = gmres_config(1e-10, 1e-12);
    let adaptive = AdaptiveStepConfig {
        atol: 1e-2 * rtol,
        rtol,
        initial_step: 1e-3,
        min_step: 1e-14,
        max_step: 1.0,
        max_attempts: 1_000_000,
        safety: 0.9,
        min_factor: 0.2,
        max_factor: 5.0,
        reject_max_factor: 0.9,
        controller: ControllerKind::Integral,
    };
    let sched = OutputSchedule::uniform(0.0, 1.0, 1.0 / 64.0).expect("schedule");
    let res = integrate_sequential_matrix_free_adaptive_observed(
        problem, (0.0, 1.0), y0, &cfg, &adaptive, &sched,
    );
    let mut row = Row {
        problem: pname.into(),
        arm: arm.into(),
        k: 0,
        h: 0.0,
        steps: 0,
        steps_completed: 0,
        status: "ok".into(),
        message: String::new(),
        err_max_rel_inf: f64::NAN,
        err_end_rel_inf: f64::NAN,
        err_local_main: None,
        err_local_embedded: None,
        err_vector_norm: None,
        would_reject_steps: 0,
        max_embedded_wrms: f64::NAN,
        rhs_calls: 0,
        ft_calls: 0,
        jvp_calls: 0,
        jvp_vectors: 0,
        linear_iterations: 0,
        linear_matvecs: 0,
        direct_factorizations: 0,
        linear_solve_failures: 0,
        eta_min: None,
        eta_mean: None,
        eta_max: None,
        tau_min: None,
        tau_mean: None,
        tau_max: None,
        achieved_residual_wrms_mean: None,
        flow_wrms_mean: None,
        rhs_wrms_mean: None,
        eta_clamped_at_max_fraction: None,
        roundoff_floor_estimate: f64::NAN,
        adaptive_accepted: None,
        adaptive_rejected: None,
        adaptive_h_min: None,
        adaptive_h_max: None,
        wall_s: 0.0,
    };
    match res {
        Ok(r) => {
            let obs = &r.observed;
            row.status = if obs.success { "ok".into() } else { "failed".into() };
            row.message = obs.message.clone();
            let mut em: f64 = 0.0;
            let mut ee = 0.0;
            for (t, y) in obs.t.iter().zip(&obs.y) {
                if *t == 0.0 {
                    continue;
                }
                let ex = exact(*t);
                let d: Vec<f64> = y.iter().zip(&ex).map(|(a, b)| a - b).collect();
                let e = inf_norm(&d) / inf_norm(&ex);
                em = em.max(e);
                ee = e;
            }
            row.err_max_rel_inf = em;
            row.err_end_rel_inf = ee;
            row.steps = obs.internal_steps;
            row.steps_completed = obs.internal_steps;
            row.rhs_calls = obs.counters.rhs_calls;
            row.ft_calls = obs.counters.ft_calls;
            row.jvp_calls = obs.counters.jvp_calls;
            row.jvp_vectors = obs.counters.jvp_vectors;
            row.linear_iterations = obs.counters.linear_iterations;
            row.linear_matvecs = obs.counters.linear_matvecs;
            row.linear_solve_failures = obs.counters.linear_solve_failures;
            row.would_reject_steps = obs.counters.rejected_steps as usize;
            row.adaptive_accepted = Some(r.diagnostics.accepted_macro_steps);
            row.adaptive_rejected = Some(r.diagnostics.rejected_macro_steps);
            row.adaptive_h_min = r.diagnostics.accepted_step_sizes.iter().cloned().reduce(f64::min);
            row.adaptive_h_max = r.diagnostics.accepted_step_sizes.iter().cloned().reduce(f64::max);
            let hmean = if r.diagnostics.accepted_macro_steps > 0 {
                1.0 / r.diagnostics.accepted_macro_steps as f64
            } else {
                f64::NAN
            };
            row.h = hmean;
            let gamma = 0.21193756319429014_f64;
            row.roundoff_floor_estimate =
                obs.internal_steps as f64 * f64::EPSILON * (1.0 + hmean * gamma * lam_max_abs);
        }
        Err(e) => {
            row.status = "failed".into();
            row.message = e.to_string();
        }
    }
    row.wall_s = t0.elapsed().as_secs_f64();
    row
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: e04_order_krylov <p1|p2|p1ns> <kmin> <kmax> <arm,arm,...>");
        std::process::exit(2);
    }
    let pname = args[1].as_str();
    let kmin: u32 = args[2].parse().unwrap();
    let kmax: u32 = args[3].parse().unwrap();
    let arms: Vec<&str> = args[4].split(',').collect();
    let (problem, y0, exact, lam_max_abs, note): (
        OdeProblem,
        Vec<f64>,
        Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>,
        f64,
        String,
    ) = match pname {
        "p1" => {
            let p = build_p1(256, 1.0, 1e6, 0x5EED_2026_0927, 1.0).expect("p1");
            let note = format!("orth_defect={:e}", p.orth_defect);
            (p.problem, p.y0, p.exact, p.lam_max_abs, note)
        }
        "p1ns" => {
            let p = build_p1(256, 1.0, 10.0, 0x5EED_2026_0927, 1.0).expect("p1ns");
            let note = format!("orth_defect={:e}", p.orth_defect);
            (p.problem, p.y0, p.exact, p.lam_max_abs, note)
        }
        "p1pr" => {
            // pure Prothero-Robinson: y0 = g(0), exact y(t) = g(t); no transient, stiffness enters only through the forcing.
            let p = build_p1(256, 1.0, 1e6, 0x5EED_2026_0927, 0.0).expect("p1pr");
            let note = format!("orth_defect={:e} pert=0", p.orth_defect);
            (p.problem, p.y0, p.exact, p.lam_max_abs, note)
        }
        "p2" => {
            let (problem, y0) =
                semilinear_advection_diffusion_problem(512, 0.02, 3.0, -1.0, 10.0, 0.0)
                    .expect("p2");
            let exact_fn: Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync> = {
                let (p2, _) =
                    semilinear_advection_diffusion_problem(512, 0.02, 3.0, -1.0, 10.0, 0.0)
                        .expect("p2");
                Arc::new(move |t: f64| p2.exact(t).expect("exact"))
            };
            let dx = 1.0 / 513.0;
            let lam = 4.0 * 0.02 / (dx * dx) + 2.0 * 3.0 / dx + 1.0;
            (problem, y0, exact_fn, lam, format!("n=512 d=0.02 a=3 r=-1 nu=10 spectral_bound~{lam:e}"))
        }
        _ => panic!("unknown problem"),
    };
    eprintln!("# problem {pname} {note}");
    println!(
        "{}",
        serde_json::json!({"meta": true, "problem": pname, "note": note, "dimension": problem.dimension})
    );
    for arm in arms {
        if let Some(r) = arm.strip_prefix("adaptive:") {
            let rtol: f64 = r.parse().unwrap();
            let row = run_adaptive(&problem, &y0, &*exact, pname, arm, rtol, lam_max_abs);
            println!("{}", serde_json::to_string(&row).unwrap());
            continue;
        }
        for k in kmin..=kmax {
            let row = run_fixed(&problem, &y0, &*exact, pname, arm, k, lam_max_abs);
            eprintln!(
                "# {pname} {arm} k={k} err={:.3e} status={} wall={:.1}s lin_it={}",
                row.err_max_rel_inf, row.status, row.wall_s, row.linear_iterations
            );
            println!("{}", serde_json::to_string(&row).unwrap());
            if row.status == "failed" {
                // keep going: smaller h may succeed (kappa shrinks)
                continue;
            }
        }
    }
}
