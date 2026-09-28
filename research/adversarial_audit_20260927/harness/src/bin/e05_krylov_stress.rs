//! E-05: Krylov stress on nonnormal operators A = Q (D + N) Q^T, n = 256.
//! Reported residual/converged vs recomputed true residual, RHS scale 1e-14/1/1e14,
//! stale-image test (same operator object, preconditioner swap), GCRO-DR dimension change.
use rodas5p_core::{
    CoreResult, DenseMatrix, DenseOperator, ExactPreconditionerIdentity, IdentityPreconditioner,
    JacobiPreconditioner, LinearOperator, Preconditioner, WorkCounters, ClosureOperator,
};
use rodas5p_krylov::{
    GcrodrConfig, GcrodrState, GmresConfig, LgmresConfig, LgmresState, solve_gcrodr, solve_gmres,
    solve_gmres_givens, solve_lgmres,
};
use serde::Serialize;
use std::sync::{Arc, RwLock};

struct Lcg(u64);
impl Lcg {
    fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn uniform(&mut self) -> f64 {
        2.0 * self.next_f64() - 1.0
    }
}

/// Orthogonal Q from Householder QR of a random matrix (explicit product of reflectors).
fn random_orthogonal(n: usize, rng: &mut Lcg) -> DenseMatrix {
    let mut q = DenseMatrix::identity(n);
    for k in 0..n {
        let mut v: Vec<f64> = (0..n).map(|i| if i < k { 0.0 } else { rng.uniform() }).collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm == 0.0 {
            continue;
        }
        for x in &mut v {
            *x /= norm;
        }
        // q = q (I - 2 v v^T)
        let mut qv = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                qv[i] += q[(i, j)] * v[j];
            }
        }
        for i in 0..n {
            for j in 0..n {
                q[(i, j)] -= 2.0 * qv[i] * v[j];
            }
        }
    }
    q
}

fn build_a(n: usize, s: f64, dmax: f64, rng: &mut Lcg) -> (DenseMatrix, Vec<f64>) {
    let q = random_orthogonal(n, rng);
    let mut inner = DenseMatrix::zeros(n, n);
    let mut diag = Vec::with_capacity(n);
    for i in 0..n {
        // negative, log-spaced from -1 to -dmax
        let d = -(10f64).powf((i as f64) / ((n - 1) as f64) * dmax.log10());
        inner[(i, i)] = d;
        diag.push(d);
        for j in (i + 1)..n {
            inner[(i, j)] = s * rng.uniform();
        }
    }
    let qt = q.transpose();
    let a = q.matmul(&inner).unwrap().matmul(&qt).unwrap();
    (a, diag)
}

fn true_rel_residual(a: &DenseMatrix, b: &[f64], x: &[f64]) -> f64 {
    let ax = a.matvec(x).unwrap();
    let r: f64 = b.iter().zip(&ax).map(|(bi, axi)| (bi - axi) * (bi - axi)).sum::<f64>().sqrt();
    let bn: f64 = b.iter().map(|v| v * v).sum::<f64>().sqrt();
    r / bn
}

#[derive(Serialize, Clone)]
struct SolveRow {
    matrix: String,
    n_scale: f64,
    rhs_scale: f64,
    solver: String,
    preconditioner: String,
    ok: bool,
    error: Option<String>,
    reported_converged: Option<bool>,
    reported_residual_norm: Option<f64>,
    reported_relative_residual: Option<f64>,
    recomputed_relative_residual_f64: Option<f64>,
    ratio_reported_over_recomputed: Option<f64>,
    within_threshold_true: Option<bool>,
    iterations: Option<u64>,
    matvecs: Option<u64>,
    preconditioner_apps: Option<u64>,
    linear_matvecs: u64,
    diagnostic_matvecs: u64,
    recycle_refresh_matvecs: u64,
    x_norm: Option<f64>,
}

/// A preconditioner with no exact identity (forces refresh semantics).
struct OpaqueJacobi(Vec<f64>);
impl Preconditioner for OpaqueJacobi {
    fn dimension(&self) -> usize {
        self.0.len()
    }
    fn apply(&self, x: &[f64], y: &mut [f64]) -> CoreResult<()> {
        for i in 0..x.len() {
            y[i] = self.0[i] * x[i];
        }
        Ok(())
    }
    fn exact_identity(&self) -> Option<ExactPreconditionerIdentity> {
        None
    }
}

fn run_one(
    label: &str,
    s: f64,
    rhs_scale: f64,
    a: &DenseMatrix,
    b: &[f64],
    solver: &str,
    pc_name: &str,
    rtol: f64,
) -> SolveRow {
    let n = a.nrows();
    let op = DenseOperator::new(a.clone()).unwrap();
    let pc: Box<dyn Preconditioner> = match pc_name {
        "none" => Box::new(IdentityPreconditioner::new(n)),
        "jacobi" => Box::new(JacobiPreconditioner::from_matrix(a).unwrap()),
        _ => unreachable!(),
    };
    let mut counters = WorkCounters::default();
    let bn: f64 = b.iter().map(|v| v * v).sum::<f64>().sqrt();
    let threshold = rtol * bn;
    let res = match solver {
        "gmres" => solve_gmres(
            &op,
            pc.as_ref(),
            b,
            None,
            &GmresConfig { restart: 40, max_arnoldi: 4000, rtol, atol: 0.0 },
            &mut counters,
        ),
        "gmres_givens" => solve_gmres_givens(
            &op,
            pc.as_ref(),
            b,
            None,
            &GmresConfig { restart: 40, max_arnoldi: 4000, rtol, atol: 0.0 },
            &mut counters,
        ),
        "lgmres" => {
            let mut st = LgmresState::default();
            solve_lgmres(
                &op,
                pc.as_ref(),
                b,
                None,
                &LgmresConfig { inner_m: 30, max_outer: 120, outer_k: 8, rtol, atol: 0.0 },
                &mut st,
                &mut counters,
            )
        }
        "gcrodr" => {
            let mut st = GcrodrState::default();
            solve_gcrodr(
                &op,
                pc.as_ref(),
                b,
                None,
                &GcrodrConfig {
                    restart: 40,
                    max_arnoldi: 4000,
                    recycle_dim: 8,
                    rank_tol: 1e-12,
                    rtol,
                    atol: 0.0,
                },
                &mut st,
                &mut counters,
            )
        }
        _ => unreachable!(),
    };
    match res {
        Ok(rep) => {
            let rec = true_rel_residual(a, b, &rep.x);
            let xn = rep.x.iter().map(|v| v * v).sum::<f64>().sqrt();
            SolveRow {
                matrix: label.into(),
                n_scale: s,
                rhs_scale,
                solver: solver.into(),
                preconditioner: pc_name.into(),
                ok: true,
                error: None,
                reported_converged: Some(rep.converged),
                reported_residual_norm: Some(rep.residual_norm),
                reported_relative_residual: Some(rep.relative_residual),
                recomputed_relative_residual_f64: Some(rec),
                ratio_reported_over_recomputed: Some(rep.relative_residual / rec),
                within_threshold_true: Some(rec * bn <= threshold),
                iterations: Some(rep.iterations),
                matvecs: Some(rep.matvecs),
                preconditioner_apps: Some(rep.preconditioner_apps),
                linear_matvecs: counters.linear_matvecs,
                diagnostic_matvecs: counters.diagnostic_matvecs,
                recycle_refresh_matvecs: counters.recycle_refresh_matvecs,
                x_norm: Some(xn),
            }
        }
        Err(e) => SolveRow {
            matrix: label.into(),
            n_scale: s,
            rhs_scale,
            solver: solver.into(),
            preconditioner: pc_name.into(),
            ok: false,
            error: Some(format!("{e}")),
            reported_converged: None,
            reported_residual_norm: None,
            reported_relative_residual: None,
            recomputed_relative_residual_f64: None,
            ratio_reported_over_recomputed: None,
            within_threshold_true: None,
            iterations: None,
            matvecs: None,
            preconditioner_apps: None,
            linear_matvecs: counters.linear_matvecs,
            diagnostic_matvecs: counters.diagnostic_matvecs,
            recycle_refresh_matvecs: counters.recycle_refresh_matvecs,
            x_norm: None,
        },
    }
}

#[derive(Serialize)]
struct StaleRow {
    scenario: String,
    step: String,
    ok: bool,
    error: Option<String>,
    recomputed_relative_residual_f64: Option<f64>,
    within_threshold_true: Option<bool>,
    iterations: Option<u64>,
    recycle_same_operator_uses: u64,
    recycle_cross_operator_refreshes: u64,
    recycle_refresh_matvecs: u64,
    linear_matvecs: u64,
    state_directions_or_basis: usize,
    state_images_present: usize,
    previous_solution_len: Option<usize>,
}

fn stale_lgmres(
    scenario: &str,
    op: &dyn LinearOperator,
    a_for_residual: &DenseMatrix,
    pcs: &[(&str, &dyn Preconditioner)],
    rhss: &[&[f64]],
    st: &mut LgmresState,
    rtol: f64,
) -> Vec<StaleRow> {
    let mut rows = Vec::new();
    let cfg = LgmresConfig { inner_m: 30, max_outer: 120, outer_k: 8, rtol, atol: 0.0 };
    for (i, ((pname, pc), b)) in pcs.iter().zip(rhss).enumerate() {
        let mut c = WorkCounters::default();
        let r = solve_lgmres(op, *pc, b, None, &cfg, st, &mut c);
        let (ok, err, rec, within, it) = match &r {
            Ok(rep) => {
                let rec = true_rel_residual(a_for_residual, b, &rep.x);
                (true, None, Some(rec), Some(rec <= rtol), Some(rep.iterations))
            }
            Err(e) => (false, Some(format!("{e}")), None, None, None),
        };
        rows.push(StaleRow {
            scenario: scenario.into(),
            step: format!("{i}:{pname}"),
            ok,
            error: err,
            recomputed_relative_residual_f64: rec,
            within_threshold_true: within,
            iterations: it,
            recycle_same_operator_uses: c.recycle_same_operator_uses,
            recycle_cross_operator_refreshes: c.recycle_cross_operator_refreshes,
            recycle_refresh_matvecs: c.recycle_refresh_matvecs,
            linear_matvecs: c.linear_matvecs,
            state_directions_or_basis: st.directions.len(),
            state_images_present: st.images.iter().filter(|x| x.is_some()).count(),
            previous_solution_len: st.previous_solution.as_ref().map(|v| v.len()),
        });
    }
    rows
}

fn stale_gcrodr(
    scenario: &str,
    op: &dyn LinearOperator,
    a_for_residual: &DenseMatrix,
    pcs: &[(&str, &dyn Preconditioner)],
    rhss: &[&[f64]],
    st: &mut GcrodrState,
    rtol: f64,
) -> Vec<StaleRow> {
    let mut rows = Vec::new();
    let cfg = GcrodrConfig { restart: 40, max_arnoldi: 4000, recycle_dim: 8, rank_tol: 1e-12, rtol, atol: 0.0 };
    for (i, ((pname, pc), b)) in pcs.iter().zip(rhss).enumerate() {
        let mut c = WorkCounters::default();
        let r = solve_gcrodr(op, *pc, b, None, &cfg, st, &mut c);
        let (ok, err, rec, within, it) = match &r {
            Ok(rep) => {
                let rec = true_rel_residual(a_for_residual, b, &rep.x);
                (true, None, Some(rec), Some(rec <= rtol), Some(rep.iterations))
            }
            Err(e) => (false, Some(format!("{e}")), None, None, None),
        };
        rows.push(StaleRow {
            scenario: scenario.into(),
            step: format!("{i}:{pname}"),
            ok,
            error: err,
            recomputed_relative_residual_f64: rec,
            within_threshold_true: within,
            iterations: it,
            recycle_same_operator_uses: c.recycle_same_operator_uses,
            recycle_cross_operator_refreshes: c.recycle_cross_operator_refreshes,
            recycle_refresh_matvecs: c.recycle_refresh_matvecs,
            linear_matvecs: c.linear_matvecs,
            state_directions_or_basis: st.basis.len(),
            state_images_present: st.image.len(),
            previous_solution_len: st.previous_solution.as_ref().map(|v| v.len()),
        });
    }
    rows
}

fn child_dimchange(which: &str) {
    // Solve n=32 system, then reuse the SAME state on an n=16 system with a different operator.
    let mut rng = Lcg(7);
    let (a1, _) = build_a(32, 1.0, 1e3, &mut rng);
    let (a2, _) = build_a(16, 1.0, 1e3, &mut rng);
    let b1: Vec<f64> = (0..32).map(|_| rng.uniform()).collect();
    let b2: Vec<f64> = (0..16).map(|_| rng.uniform()).collect();
    let op1 = DenseOperator::new(a1).unwrap();
    let op2 = DenseOperator::new(a2).unwrap();
    let mut c = WorkCounters::default();
    match which {
        "gcrodr" => {
            let cfg = GcrodrConfig { restart: 10, max_arnoldi: 400, recycle_dim: 4, rank_tol: 1e-12, rtol: 1e-10, atol: 0.0 };
            let mut st = GcrodrState::default();
            let r1 = solve_gcrodr(&op1, &IdentityPreconditioner::new(32), &b1, None, &cfg, &mut st, &mut c);
            eprintln!("first solve ok={} prev_len={:?} basis={}", r1.is_ok(), st.previous_solution.as_ref().map(|v| v.len()), st.basis.len());
            let r2 = solve_gcrodr(&op2, &IdentityPreconditioner::new(16), &b2, None, &cfg, &mut st, &mut c);
            match r2 {
                Ok(rep) => println!("{{\"outcome\":\"ok\",\"iterations\":{},\"residual\":{:e}}}", rep.iterations, rep.relative_residual),
                Err(e) => println!("{{\"outcome\":\"err\",\"message\":{:?}}}", format!("{e}")),
            }
        }
        "lgmres" => {
            let cfg = LgmresConfig { inner_m: 10, max_outer: 40, outer_k: 4, rtol: 1e-10, atol: 0.0 };
            let mut st = LgmresState::default();
            let r1 = solve_lgmres(&op1, &IdentityPreconditioner::new(32), &b1, None, &cfg, &mut st, &mut c);
            eprintln!("first solve ok={} prev_len={:?} dirs={}", r1.is_ok(), st.previous_solution.as_ref().map(|v| v.len()), st.directions.len());
            let r2 = solve_lgmres(&op2, &IdentityPreconditioner::new(16), &b2, None, &cfg, &mut st, &mut c);
            match r2 {
                Ok(rep) => println!("{{\"outcome\":\"ok\",\"iterations\":{},\"residual\":{:e}}}", rep.iterations, rep.relative_residual),
                Err(e) => println!("{{\"outcome\":\"err\",\"message\":{:?}}}", format!("{e}")),
            }
        }
        _ => unreachable!(),
    }
}

#[derive(Serialize)]
struct ChildOutcome {
    solver: String,
    exit_code: Option<i32>,
    signal_or_abort: bool,
    stdout: String,
    stderr_tail: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--child-dimchange" {
        child_dimchange(&args[2]);
        return;
    }
    if args.len() >= 3 && args[1] == "--stale-only" {
        stale_suite(&args[2]);
        return;
    }
    let out_dir = args.get(1).cloned().unwrap_or_else(|| ".".into());
    let n = 256usize;
    let rtol = 1e-10;
    let mut rng = Lcg(20260927);
    let scales = [0.0, 1.0, 10.0, 100.0];
    let rhs_scales = [1e-14, 1.0, 1e14];
    let mut rows: Vec<SolveRow> = Vec::new();
    let mut matrices = serde_json::Map::new();
    let b_unit: Vec<f64> = {
        let v: Vec<f64> = (0..n).map(|_| rng.uniform()).collect();
        let nn = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        v.into_iter().map(|x| x / nn).collect()
    };
    let mut built = Vec::new();
    for &s in &scales {
        let (a, diag) = build_a(n, s, 1e4, &mut rng);
        let label = format!("s{s}");
        matrices.insert(
            label.clone(),
            serde_json::json!({"s": s, "diag": diag, "a_rows": (0..n).map(|i| a.row(i).to_vec()).collect::<Vec<_>>()}),
        );
        built.push((label, s, a));
    }
    for (label, s, a) in &built {
        for &rs in &rhs_scales {
            let b: Vec<f64> = b_unit.iter().map(|x| x * rs).collect();
            for solver in ["gmres", "gmres_givens", "lgmres", "gcrodr"] {
                for pc in ["none", "jacobi"] {
                    let row = run_one(label, *s, rs, a, &b, solver, pc, rtol);
                    eprintln!(
                        "{label} rhs={rs:e} {solver}/{pc}: ok={} it={:?} rep={:?} rec={:?} err={:?}",
                        row.ok, row.iterations, row.reported_relative_residual, row.recomputed_relative_residual_f64, row.error
                    );
                    rows.push(row);
                }
            }
        }
    }

    // ---- Stale-image / previous_solution scenarios on the s=10 matrix ----
    let (label10, _, a10) = &built[2];
    let _ = label10;
    let jac = JacobiPreconditioner::from_matrix(a10).unwrap();
    let ident = IdentityPreconditioner::new(n);
    // A second, deliberately *wrong-ish* Jacobi preconditioner built from A scaled by 3 (still a valid PC).
    let jac_other = JacobiPreconditioner::from_matrix(&a10.scale(3.0)).unwrap();
    let opaque = OpaqueJacobi(jac_other_inv(&a10.scale(3.0)));
    let b1: Vec<f64> = b_unit.clone();
    let b2: Vec<f64> = b_unit.iter().map(|x| x * 1e14).collect();
    let b3: Vec<f64> = b_unit.iter().map(|x| x * 1e-14).collect();
    let op10 = DenseOperator::new(a10.clone()).unwrap();
    let mut stale_rows = Vec::new();
    {
        // S1: same operator object, PC jacobi -> none -> jacobi_other -> opaque, same RHS.
        let pcs: Vec<(&str, &dyn Preconditioner)> = vec![("jacobi", &jac), ("none", &ident), ("jacobi_other", &jac_other), ("opaque", &opaque), ("opaque_again", &opaque)];
        let rhss: Vec<&[f64]> = vec![&b1, &b1, &b1, &b1, &b1];
        let mut st = LgmresState::default();
        stale_rows.extend(stale_lgmres("S1-lgmres-pc-swap-same-op", &op10, a10, &pcs, &rhss, &mut st, rtol));
        let mut sg = GcrodrState::default();
        stale_rows.extend(stale_gcrodr("S1-gcrodr-pc-swap-same-op", &op10, a10, &pcs, &rhss, &mut sg, rtol));
    }
    {
        // S2: same operator+PC, RHS scale 1e14 then 1e-14 (previous_solution used as x0 by default).
        let pcs: Vec<(&str, &dyn Preconditioner)> = vec![("jacobi", &jac), ("jacobi", &jac), ("jacobi", &jac)];
        let rhss: Vec<&[f64]> = vec![&b2, &b3, &b1];
        let mut st = LgmresState::default();
        stale_rows.extend(stale_lgmres("S2-lgmres-rhs-1e14-then-1e-14", &op10, a10, &pcs, &rhss, &mut st, rtol));
        let mut sg = GcrodrState::default();
        stale_rows.extend(stale_gcrodr("S2-gcrodr-rhs-1e14-then-1e-14", &op10, a10, &pcs, &rhss, &mut sg, rtol));
    }
    {
        // S3: SAME operator object whose interior matrix is mutated between solves (ClosureOperator,
        // identity = Instance(token)) with the same PC: are stale images/basis reused?
        let shared = Arc::new(RwLock::new(a10.clone()));
        let shared_c = shared.clone();
        let op_mut = ClosureOperator::new(n, move |x, y| shared_c.read().unwrap().matvec_into(x, y));
        let pcs: Vec<(&str, &dyn Preconditioner)> = vec![("none", &ident), ("none", &ident)];
        let rhss: Vec<&[f64]> = vec![&b1, &b1];
        let mut st = LgmresState::default();
        let mut sg = GcrodrState::default();
        let mut c = WorkCounters::default();
        let cfg_l = LgmresConfig { inner_m: 30, max_outer: 120, outer_k: 8, rtol, atol: 0.0 };
        let cfg_g = GcrodrConfig { restart: 40, max_arnoldi: 4000, recycle_dim: 8, rank_tol: 1e-12, rtol, atol: 0.0 };
        let _ = solve_lgmres(&op_mut, &ident, &b1, None, &cfg_l, &mut st, &mut c);
        let _ = solve_gcrodr(&op_mut, &ident, &b1, None, &cfg_g, &mut sg, &mut c);
        // mutate: scale interior by 2
        {
            let mut w = shared.write().unwrap();
            *w = w.scale(2.0);
        }
        let a_mut = shared.read().unwrap().clone();
        let _ = pcs;
        let _ = rhss;
        let rows_l = stale_lgmres("S3-lgmres-mutated-same-token", &op_mut, &a_mut, &[("none", &ident)], &[&b1], &mut st, rtol);
        let rows_g = stale_gcrodr("S3-gcrodr-mutated-same-token", &op_mut, &a_mut, &[("none", &ident)], &[&b1], &mut sg, rtol);
        stale_rows.extend(rows_l);
        stale_rows.extend(rows_g);
    }

    // ---- GCRO-DR / LGMRES dimension change with stale previous_solution, in child processes ----
    let exe = std::env::current_exe().unwrap();
    let mut children = Vec::new();
    for which in ["gcrodr", "lgmres"] {
        let out = std::process::Command::new(&exe).arg("--child-dimchange").arg(which).output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        let tail: String = stderr.chars().rev().take(600).collect::<String>().chars().rev().collect();
        children.push(ChildOutcome {
            solver: which.into(),
            exit_code: out.status.code(),
            signal_or_abort: out.status.code().is_none(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr_tail: tail,
        });
    }

    let result = serde_json::json!({
        "n": n, "rtol": rtol, "atol": 0.0, "scales": scales, "rhs_scales": rhs_scales,
        "rows": rows, "stale_rows": stale_rows, "children": children, "b_unit": b_unit,
    });
    std::fs::write(format!("{out_dir}/e05_raw.json"), serde_json::to_string_pretty(&result).unwrap()).unwrap();
    std::fs::write(format!("{out_dir}/e05_matrices.json"), serde_json::to_string(&matrices).unwrap()).unwrap();
    println!("wrote {out_dir}/e05_raw.json and e05_matrices.json");
}

fn jac_other_inv(a: &DenseMatrix) -> Vec<f64> {
    a.diagonal().unwrap().into_iter().map(|d| 1.0 / d).collect()
}

/// Re-run of the stale-image scenarios on a matrix every solver converges on
/// (s=1, cond 1e2), plus cross-operator same-dimension previous_solution reuse.
fn stale_suite(out_dir: &str) {
    let n = 256usize;
    let rtol = 1e-10;
    let mut rng = Lcg(424242);
    let (a1, _) = build_a(n, 1.0, 1e2, &mut rng);
    let (a2, _) = build_a(n, 1.0, 1e2, &mut rng);
    let b_unit: Vec<f64> = {
        let v: Vec<f64> = (0..n).map(|_| rng.uniform()).collect();
        let nn = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        v.into_iter().map(|x| x / nn).collect()
    };
    let jac = JacobiPreconditioner::from_matrix(&a1).unwrap();
    let ident = IdentityPreconditioner::new(n);
    let jac_other = JacobiPreconditioner::from_matrix(&a1.scale(3.0)).unwrap();
    let opaque = OpaqueJacobi(jac_other_inv(&a1.scale(3.0)));
    let b1: Vec<f64> = b_unit.clone();
    let b2: Vec<f64> = b_unit.iter().map(|x| x * 1e14).collect();
    let b3: Vec<f64> = b_unit.iter().map(|x| x * 1e-14).collect();
    let op1 = DenseOperator::new(a1.clone()).unwrap();
    let op2 = DenseOperator::new(a2.clone()).unwrap();
    let mut stale_rows = Vec::new();
    {
        let pcs: Vec<(&str, &dyn Preconditioner)> = vec![("jacobi", &jac), ("none", &ident), ("jacobi_other", &jac_other), ("opaque", &opaque), ("opaque_again", &opaque), ("jacobi", &jac)];
        let rhss: Vec<&[f64]> = vec![&b1; 6];
        let mut st = LgmresState::default();
        stale_rows.extend(stale_lgmres("S1-lgmres-pc-swap-same-op", &op1, &a1, &pcs, &rhss, &mut st, rtol));
        let mut sg = GcrodrState::default();
        stale_rows.extend(stale_gcrodr("S1-gcrodr-pc-swap-same-op", &op1, &a1, &pcs, &rhss, &mut sg, rtol));
    }
    {
        let pcs: Vec<(&str, &dyn Preconditioner)> = vec![("jacobi", &jac), ("jacobi", &jac), ("jacobi", &jac)];
        let rhss: Vec<&[f64]> = vec![&b2, &b3, &b1];
        let mut st = LgmresState::default();
        stale_rows.extend(stale_lgmres("S2-lgmres-rhs-1e14-then-1e-14", &op1, &a1, &pcs, &rhss, &mut st, rtol));
        let mut sg = GcrodrState::default();
        stale_rows.extend(stale_gcrodr("S2-gcrodr-rhs-1e14-then-1e-14", &op1, &a1, &pcs, &rhss, &mut sg, rtol));
    }
    {
        let shared = Arc::new(RwLock::new(a1.clone()));
        let shared_c = shared.clone();
        let op_mut = ClosureOperator::new(n, move |x, y| shared_c.read().unwrap().matvec_into(x, y));
        let mut st = LgmresState::default();
        let mut sg = GcrodrState::default();
        stale_rows.extend(stale_lgmres("S3-lgmres-before-mutation", &op_mut, &a1, &[("none", &ident)], &[&b1], &mut st, rtol));
        stale_rows.extend(stale_gcrodr("S3-gcrodr-before-mutation", &op_mut, &a1, &[("none", &ident)], &[&b1], &mut sg, rtol));
        {
            let mut w = shared.write().unwrap();
            *w = w.scale(2.0);
        }
        let a_mut = shared.read().unwrap().clone();
        stale_rows.extend(stale_lgmres("S3-lgmres-mutated-same-token", &op_mut, &a_mut, &[("none", &ident)], &[&b1], &mut st, rtol));
        stale_rows.extend(stale_gcrodr("S3-gcrodr-mutated-same-token", &op_mut, &a_mut, &[("none", &ident)], &[&b1], &mut sg, rtol));
    }
    {
        // S4: cross-operator, same dimension, stale state carried from op1 to op2.
        let mut sg = GcrodrState::default();
        stale_rows.extend(stale_gcrodr("S4-gcrodr-op1-then-op2", &op1, &a1, &[("none", &ident)], &[&b1], &mut sg, rtol));
        let prev_before = sg.previous_solution.clone();
        stale_rows.extend(stale_gcrodr("S4-gcrodr-op1-then-op2", &op2, &a2, &[("none", &ident)], &[&b1], &mut sg, rtol));
        let mut fresh = GcrodrState::default();
        stale_rows.extend(stale_gcrodr("S4-gcrodr-op2-fresh-baseline", &op2, &a2, &[("none", &ident)], &[&b1], &mut fresh, rtol));
        let mut sl = LgmresState::default();
        stale_rows.extend(stale_lgmres("S4-lgmres-op1-then-op2", &op1, &a1, &[("none", &ident)], &[&b1], &mut sl, rtol));
        let prev_before_l = sl.previous_solution.clone();
        stale_rows.extend(stale_lgmres("S4-lgmres-op1-then-op2", &op2, &a2, &[("none", &ident)], &[&b1], &mut sl, rtol));
        let mut freshl = LgmresState::default();
        stale_rows.extend(stale_lgmres("S4-lgmres-op2-fresh-baseline", &op2, &a2, &[("none", &ident)], &[&b1], &mut freshl, rtol));
        eprintln!("S4 gcrodr prev_solution carried into op2 solve: {:?} lgmres: {:?}", prev_before.map(|v| v.len()), prev_before_l.map(|v| v.len()));
    }
    let result = serde_json::json!({"n": n, "rtol": rtol, "cond": 1e2, "s": 1.0, "stale_rows": stale_rows});
    std::fs::write(format!("{out_dir}/e05_stale_raw.json"), serde_json::to_string_pretty(&result).unwrap()).unwrap();
    println!("wrote {out_dir}/e05_stale_raw.json");
}
