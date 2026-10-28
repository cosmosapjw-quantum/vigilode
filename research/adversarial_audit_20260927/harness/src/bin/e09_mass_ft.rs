//! E-09: (a) mass-matrix fixed-step convergence (direct + GMRES) on three mass problems incl. M=diag(1,1e-3);
//! (b) analytic partial_t vs FD partial_t on Prothero-Robinson over an h ladder.
//! Usage: e09_mass_ft <out.json>
use rodas5p_core::{CoreResult, DenseMatrix, InitialGuess, LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters};
use rodas5p_integrators::{
    IntegrationMethod, OdeProblem, constant_affine_mass_problem, integrate_fixed, manufactured_mass_nonlinear_problem,
    prothero_robinson_problem,
};
use std::sync::Arc;

type RhsFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type JacFn = Arc<dyn Fn(f64, &[f64]) -> CoreResult<DenseMatrix> + Send + Sync>;
type JvpFn = Arc<dyn Fn(f64, &[f64], &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type FtFn = Arc<dyn Fn(f64, &[f64], &mut [f64]) -> CoreResult<()> + Send + Sync>;
type ExactFn = Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>;

/// M y' = J y + r(t), M = diag(1, eps), exact y = (sin t, cos t + 0.5).
fn diag_mass_problem(eps: f64) -> (OdeProblem, Vec<f64>) {
    let m = DenseMatrix::from_rows(&[&[1.0, 0.0], &[0.0, eps]]).unwrap();
    let j = DenseMatrix::from_rows(&[&[-1.0, 0.5], &[0.2, -1.0]]).unwrap();
    let exact_v = |t: f64| vec![t.sin(), t.cos() + 0.5];
    let dexact = |t: f64| [t.cos(), -t.sin()];
    let ddexact = |t: f64| [-t.sin(), -t.cos()];
    let mj = m.clone();
    let jj = j.clone();
    let r = move |t: f64| {
        let y = exact_v(t);
        let yp = dexact(t);
        let jy = jj.matvec(&y).unwrap();
        [mj[(0, 0)] * yp[0] - jy[0], mj[(1, 1)] * yp[1] - jy[1]]
    };
    let mj2 = m.clone();
    let jj2 = j.clone();
    let rp = move |t: f64| {
        let yp = dexact(t);
        let ypp = ddexact(t);
        let jyp = jj2.matvec(&yp).unwrap();
        [mj2[(0, 0)] * ypp[0] - jyp[0], mj2[(1, 1)] * ypp[1] - jyp[1]]
    };
    let j1 = j.clone();
    let rhs: RhsFn = Arc::new(move |t, y, out: &mut [f64]| {
        j1.matvec_into(y, out)?;
        let rr = r(t);
        out[0] += rr[0];
        out[1] += rr[1];
        Ok(())
    });
    let j2 = j.clone();
    let jac: JacFn = Arc::new(move |_t, _y| Ok(j2.clone()));
    let j3 = j.clone();
    let jvp: JvpFn = Arc::new(move |_t, _y, v, out: &mut [f64]| j3.matvec_into(v, out));
    let ft: FtFn = Arc::new(move |t, _y, out: &mut [f64]| {
        let rr = rp(t);
        out[0] = rr[0];
        out[1] = rr[1];
        Ok(())
    });
    let exact: ExactFn = Arc::new(exact_v);
    let p = OdeProblem::new(format!("diagmass-eps{eps:e}"), 2, rhs, None, Some(jac), Some(jvp), Some(ft), false, Some(m), Some(exact)).unwrap();
    (p, exact_v(0.0))
}

/// Twin of `inner` with partial_t = None (forces the central-FD f_t path in OdeProblem::eval_partial_t).
fn fd_twin(inner: OdeProblem) -> OdeProblem {
    let inner = Arc::new(inner);
    let dim = inner.dimension;
    let i1 = inner.clone();
    let rhs: RhsFn = Arc::new(move |t, y, out: &mut [f64]| {
        let mut d = WorkCounters::default();
        let v = i1.eval_rhs(t, y, &mut d)?;
        out.copy_from_slice(&v);
        Ok(())
    });
    let i2 = inner.clone();
    let jac: JacFn = Arc::new(move |t, y| {
        let mut d = WorkCounters::default();
        i2.dense_jacobian(t, y, &mut d)
    });
    let i3 = inner.clone();
    let jvp: JvpFn = Arc::new(move |t, y, v, out: &mut [f64]| i3.linearize_matrix_free(t, y)?.apply(v, out));
    let i4 = inner.clone();
    let exact: ExactFn = Arc::new(move |t| i4.exact(t).unwrap());
    OdeProblem::new(format!("{}-fdft", inner.name), dim, rhs, None, Some(jac), Some(jvp), None, false, inner.mass_matrix.clone(), Some(exact)).unwrap()
}

fn gmres_cfg() -> LinearSolverConfig {
    LinearSolverConfig {
        method: LinearMethod::Gmres,
        rtol: 1e-13,
        atol: 1e-16,
        restart: 40,
        maxiter: 400,
        inner_m: 30,
        outer_k: 8,
        recycle_dim: 8,
        recycle_rank_tol: 1e-12,
        preconditioner: PreconditionerKind::None,
        x0_strategy: InitialGuess::Previous,
    }
}

/// Reference for problems without an exact solution: Direct fixed-step run at h = span/2^13, looked up by time.
fn fine_reference(problem: &OdeProblem, y0: &[f64], span: (f64, f64)) -> Box<dyn Fn(f64) -> Option<Vec<f64>>> {
    let h = (span.1 - span.0) / (2f64).powi(13);
    let r = integrate_fixed(problem, span, y0, h, IntegrationMethod::Sequential, Some(&LinearSolverConfig::default()), None, 1e-12, 1e-12).expect("fine reference");
    let (ts, ys) = (r.t.clone(), r.y.clone());
    Box::new(move |t: f64| {
        let i = ts.iter().enumerate().min_by(|a, b| (a.1 - t).abs().partial_cmp(&(b.1 - t).abs()).unwrap()).map(|(i, _)| i)?;
        if (ts[i] - t).abs() <= 1e-9 { Some(ys[i].clone()) } else { None }
    })
}

fn ladder(problem: &OdeProblem, y0: &[f64], span: (f64, f64), cfg: &LinearSolverConfig, ks: std::ops::RangeInclusive<u32>) -> Vec<serde_json::Value> {
    let mut rows = Vec::new();
    let fine: Option<Box<dyn Fn(f64) -> Option<Vec<f64>>>> = if problem.exact(span.0).is_none() { Some(fine_reference(problem, y0, span)) } else { None };
    let exact_of = |t: f64| -> Vec<f64> { match &fine { Some(f) => f(t).expect("reference time missing"), None => problem.exact(t).unwrap() } };
    let mut prev_err: Option<f64> = None;
    let mut prev_end: Option<f64> = None;
    for k in ks {
        let h = (span.1 - span.0) / (2f64).powi(k as i32);
        let res = integrate_fixed(problem, span, y0, h, IntegrationMethod::Sequential, Some(cfg), None, 1e-12, 1e-12);
        let mut row = serde_json::json!({"k": k, "h": h});
        match res {
            Ok(r) => {
                let mut emax = 0.0f64;
                let mut ymax = 0.0f64;
                let mut eend = 0.0f64;
                for (i, (t, y)) in r.t.iter().zip(&r.y).enumerate() {
                    let ex = exact_of(*t);
                    let e = y.iter().zip(&ex).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
                    let yn = ex.iter().map(|v| v.abs()).fold(0.0, f64::max);
                    emax = emax.max(e);
                    ymax = ymax.max(yn);
                    if i + 1 == r.t.len() {
                        eend = e / yn.max(1e-300);
                    }
                }
                let err = emax / ymax;
                let slope = prev_err.map(|p| (p / err).log2());
                let slope_end = prev_end.map(|p| (p / eend).log2());
                row["ok"] = serde_json::json!(r.success);
                row["message"] = serde_json::json!(r.message);
                row["steps"] = serde_json::json!(r.t.len() - 1);
                row["err_grid_rel_inf"] = serde_json::json!(err);
                row["err_end_rel_inf"] = serde_json::json!(eend);
                row["slope_grid"] = serde_json::json!(slope);
                row["slope_end"] = serde_json::json!(slope_end);
                row["rhs_calls"] = serde_json::json!(r.counters.rhs_calls);
                row["ft_calls"] = serde_json::json!(r.counters.ft_calls);
                row["linear_matvecs"] = serde_json::json!(r.counters.linear_matvecs);
                row["linear_solve_failures"] = serde_json::json!(r.counters.linear_solve_failures);
                row["mass_matvecs"] = serde_json::json!(r.counters.mass_matvecs);
                prev_err = Some(err);
                prev_end = Some(eend);
                eprintln!("  k={k} h={h:.3e} err={err:.3e} end={eend:.3e} slope={slope:?} ok={}", r.success);
            }
            Err(e) => {
                row["ok"] = serde_json::json!(false);
                row["message"] = serde_json::json!(format!("Err: {e}"));
                prev_err = None;
                prev_end = None;
                eprintln!("  k={k} h={h:.3e} ERR {e}");
            }
        }
        rows.push(row);
    }
    rows
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = args.get(1).cloned().unwrap_or_else(|| "e09_out.json".into());
    let direct = LinearSolverConfig::default();
    let gm = gmres_cfg();
    let mut mass = Vec::new();
    let (p1, y1) = diag_mass_problem(1e-3);
    let (p2, y2, _, _) = manufactured_mass_nonlinear_problem(1e3, 1.0, 1.0, 0.0).unwrap();
    let (p3, y3, _, _) = constant_affine_mass_problem();
    for (name, p, y0) in [("diagmass_eps1e-3", &p1, &y1), ("manufactured_mass_nonlinear_s1e3", &p2, &y2), ("constant_affine_mass", &p3, &y3)] {
        for (arm, cfg) in [("direct", &direct), ("gmres", &gm)] {
            eprintln!("{name} / {arm}");
            let rows = ladder(p, y0, (0.0, 1.0), cfg, 2..=10);
            eprintln!("  (reference: {})", if p.exact(0.0).is_some() { "exact" } else { "direct h=2^-13 self-reference" });
            mass.push(serde_json::json!({"problem": name, "arm": arm, "rows": rows}));
        }
    }
    let mut ft = Vec::new();
    for lambda in [-1e2, -1e4] {
        for mu in [0.0, 1.0] {
            let (pa, y0) = prothero_robinson_problem(lambda, mu, 0.0);
            let pf = fd_twin(pa.clone());
            for (arm, p) in [("analytic_ft", &pa), ("fd_ft", &pf)] {
                eprintln!("PR lambda={lambda} mu={mu} / {arm}");
                let rows = ladder(p, &y0, (0.0, 1.0), &direct, 2..=10);
                ft.push(serde_json::json!({"lambda": lambda, "mu": mu, "arm": arm, "rows": rows}));
            }
        }
    }
    let out = serde_json::json!({"mass": mass, "ft": ft});
    std::fs::write(&out_path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    println!("wrote {out_path}");
}
