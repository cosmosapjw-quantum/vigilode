//! U04 contracts for the opt-in PPSW1 matrix-free common-W backend.
//!
//! Dense LU (`LinearBackend::Dense`, the unchanged PPSW1 path) is the parity reference.  Each
//! measurement prints one `QUAL {json}` line; the log is the raw evidence.  The crossover test
//! is `#[ignore]` (timing) and is run explicitly with `--ignored`.  Scoped evidence, not G0.

#![allow(clippy::needless_range_loop)]

use std::sync::Arc;
use std::time::Instant;

use rodas5p_core::{CoreError, DenseMatrix, WorkCounters};
use rodas5p_integrators::ppsw1::linear_backend::{
    LinearBackend, MatrixFreeConfig, StarterPolicy, solve_stages_with_backend, start_with_policy,
};
use rodas5p_integrators::ppsw1::{self, Config, Decision, Session, State, Tolerances};
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use rodas5p_krylov::{BlockGmresConfig, GmresConfig};
use serde_json::{Value, json};

fn qual(tag: &str, v: Value) {
    println!("QUAL {}", json!({"test": tag, "data": v}));
}

fn indep(rtol: f64, atol: f64) -> LinearBackend {
    indep_r(60, rtol, atol)
}

/// Production `solve_gmres` tests its threshold only between restart cycles, so `restart`
/// is the effective minimum iteration count of a cycle.
fn indep_r(restart: usize, rtol: f64, atol: f64) -> LinearBackend {
    LinearBackend::MatrixFree(MatrixFreeConfig::independent(GmresConfig {
        restart,
        max_arnoldi: 600,
        rtol,
        atol,
    }))
}

fn block(rtol: f64, atol: f64) -> LinearBackend {
    LinearBackend::MatrixFree(MatrixFreeConfig::block(BlockGmresConfig {
        max_basis: 600,
        rtol,
        atol,
        rank_tolerance: 1e-12,
    }))
}

/// Linear `y' = L y + g(t)` with optional explicit Jacobian and/or JVP.
fn linear_problem(l: Vec<Vec<f64>>, jac: bool, jvp: bool) -> OdeProblem {
    let n = l.len();
    let (lr, lj, lv) = (l.clone(), l.clone(), l);
    OdeProblem::new(
        "u04-linear",
        n,
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            for i in 0..n {
                out[i] = (0..n).map(|k| lr[i][k] * y[k]).sum::<f64>() + (t + i as f64).sin();
            }
            Ok(())
        }),
        None,
        if jac {
            Some(Arc::new(move |_t: f64, _y: &[f64]| {
                DenseMatrix::from_vec_rows(lj.clone())
            }))
        } else {
            None
        },
        if jvp {
            Some(Arc::new(
                move |_t: f64, _y: &[f64], v: &[f64], out: &mut [f64]| {
                    for i in 0..n {
                        out[i] = (0..n).map(|k| lv[i][k] * v[k]).sum::<f64>();
                    }
                    Ok(())
                },
            ))
        } else {
            None
        },
        None,
        false,
        None,
        None,
    )
    .unwrap()
}

fn diagonal(n: usize) -> Vec<Vec<f64>> {
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        l[i][i] = -(10f64).powf(4.0 * i as f64 / (n - 1) as f64);
    }
    l
}

fn nonnormal(n: usize) -> Vec<Vec<f64>> {
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        l[i][i] = -1.0 - 50.0 * i as f64;
        if i + 1 < n {
            l[i][i + 1] = 200.0;
        }
    }
    l
}

fn smooth_state(n: usize, h: f64, constant: bool) -> State {
    let c = ppsw1::nodes();
    State {
        t: 0.2,
        y: std::array::from_fn(|j| {
            (0..n)
                .map(|i| {
                    if constant {
                        1.0 + 0.01 * i as f64
                    } else {
                        (0.3 * i as f64 + c[j]).cos()
                    }
                })
                .collect()
        }),
        f: std::array::from_fn(|j| {
            (0..n)
                .map(|i| {
                    if constant {
                        0.0
                    } else {
                        -0.3 * (0.2 * i as f64 - c[j]).sin()
                    }
                })
                .collect()
        }),
        h_prev: h,
    }
}

fn max_rel_diff(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    let scale = b
        .iter()
        .flatten()
        .fold(f64::MIN_POSITIVE, |m, x| m.max(x.abs()));
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
        / scale
}

#[test]
fn c1_dense_parity_diagonal_nonnormal_and_rank_deficient() {
    let mut rows = Vec::new();
    for (case, l, constant) in [
        ("diagonal_stiff_n30", diagonal(30), false),
        ("nonnormal_bidiag_n40", nonnormal(40), false),
        ("rank_deficient_rhs_n40", nonnormal(40), true),
    ] {
        let p = linear_problem(l, true, true);
        let h = 0.05;
        let s = smooth_state(p.dimension, h, constant);
        let inputs = ppsw1::stage_inputs(&s, h).unwrap();
        let exec = ParallelExecution::sequential();
        let mut wd = WorkCounters::default();
        let dense = ppsw1::solve_stages(&p, &s, h, &inputs, &exec, &mut wd).unwrap();
        for (name, backend) in [
            ("independent", indep(1e-12, 1e-14)),
            ("block", block(1e-12, 1e-14)),
        ] {
            let mut w = WorkCounters::default();
            let (mf, rep) =
                solve_stages_with_backend(&p, &s, h, &inputs, &backend, &exec, &mut w).unwrap();
            let diff = max_rel_diff(&mf.y, &dense.y);
            let worst = rep
                .true_residual_norms
                .iter()
                .zip(&rep.thresholds)
                .map(|(r, t)| r / t)
                .fold(0.0, f64::max);
            rows.push(
                json!({"case": case, "solver": name, "max_rel_diff_vs_dense": diff,
                             "true_residual_norms": rep.true_residual_norms,
                             "worst_residual_over_threshold": worst,
                             "krylov_iterations": rep.krylov_iterations,
                             "initial_block_rank": rep.initial_block_rank,
                             "jvp_calls": w.jvp_calls, "linear_matvecs": w.linear_matvecs,
                             "diagnostic_matvecs": w.diagnostic_matvecs,
                             "inner_products": w.orthogonalization_inner_products,
                             "jacobian_builds": w.jacobian_builds,
                             "direct_factorizations": w.direct_factorizations}),
            );
            assert!(diff <= 1e-8, "{case} {name}: {diff:e}");
            assert_eq!(rep.true_residual_norms.len(), 6);
            assert_eq!((w.jacobian_builds, w.direct_factorizations), (0, 0));
            if case == "rank_deficient_rhs_n40" && name == "block" {
                assert_eq!(rep.initial_block_rank, Some(1));
            }
        }
        // stage-parallel independent GMRES is bitwise identical to sequential, counters equal
        let mut ws = WorkCounters::default();
        let (seq, _) =
            solve_stages_with_backend(&p, &s, h, &inputs, &indep(1e-12, 1e-14), &exec, &mut ws)
                .unwrap();
        let mut wp = WorkCounters::default();
        let (par, _) = solve_stages_with_backend(
            &p,
            &s,
            h,
            &inputs,
            &indep(1e-12, 1e-14),
            &ParallelExecution::rayon(6).unwrap(),
            &mut wp,
        )
        .unwrap();
        assert_eq!(seq.y, par.y, "{case}");
        assert_eq!(ws, wp, "{case}");
    }
    qual("c1_dense_parity", json!(rows));
}

const TOL6: Tolerances = Tolerances {
    atol: 1e-6,
    rtol: 1e-6,
};

#[test]
fn c2_stagnation_and_nonfinite_failures_are_typed_and_charged() {
    let p = linear_problem(nonnormal(40), true, true);
    let h = 0.05;
    let s = smooth_state(40, h, false);
    let starved = LinearBackend::MatrixFree(MatrixFreeConfig::independent(GmresConfig {
        restart: 2,
        max_arnoldi: 2,
        rtol: 1e-12,
        atol: 1e-14,
    }));
    let mut rows = Vec::new();
    for threads in [1usize, 4] {
        let mut sess = Session::new(
            &p,
            s.clone(),
            Config::default(),
            ParallelExecution::rayon(threads).unwrap(),
        )
        .unwrap()
        .with_linear_backend(starved.clone())
        .unwrap();
        let r = sess.try_step(h, &TOL6);
        let c = sess.counters();
        rows.push(json!({"case": "gmres_stagnation", "threads": threads,
                         "outcome": format!("{r:?}"), "jvp_calls": c.jvp_calls,
                         "linear_matvecs": c.linear_matvecs, "linear_solves": c.linear_solves}));
        assert!(matches!(r, Err(CoreError::LinearSolve(_))), "{r:?}");
        assert_eq!(sess.state(), &s);
        assert!(c.jvp_calls > 0, "failed Krylov work must stay charged");
        assert_eq!(c.rhs_calls, 0);
    }
    // NaN JVP: the stage RHS is non-finite -> typed failure, state kept, work charged
    let n = 3;
    let pnan = OdeProblem::new(
        "u04-nan-jvp",
        n,
        Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
            out.copy_from_slice(y);
            Ok(())
        }),
        None,
        None,
        Some(Arc::new(
            |_t: f64, _y: &[f64], _v: &[f64], out: &mut [f64]| {
                out.fill(f64::NAN);
                Ok(())
            },
        )),
        None,
        false,
        None,
        None,
    )
    .unwrap();
    let sn = smooth_state(n, h, false);
    let mut sess = Session::new(
        &pnan,
        sn.clone(),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap()
    .with_linear_backend(indep(1e-10, 1e-14))
    .unwrap();
    let r = sess.try_step(h, &TOL6);
    let c = sess.counters();
    rows.push(
        json!({"case": "nan_jvp", "outcome": format!("{r:?}"), "jvp_calls": c.jvp_calls,
                     "rejected": c.rejected_steps}),
    );
    assert!(matches!(r, Ok(Decision::Rejected { .. }) | Err(_)), "{r:?}");
    assert_eq!(sess.state(), &sn);
    assert!(c.jvp_calls >= 1);
    qual("c2_failures", json!(rows));
}

#[test]
fn c3_permission_refusals_happen_before_any_callback() {
    // explicit Jacobian only: no JVP permission -> matrix-free refused
    let p = linear_problem(nonnormal(10), true, false);
    let s = smooth_state(10, 0.05, false);
    let sess = Session::new(
        &p,
        s.clone(),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    assert!(matches!(
        sess.with_linear_backend(indep(1e-10, 1e-14)),
        Err(CoreError::InvalidInput(_))
    ));
    let mut w = WorkCounters::default();
    let r = ppsw1::advance_with_backend(
        &p,
        &s,
        0.05,
        &indep(1e-10, 1e-14),
        &ParallelExecution::sequential(),
        &mut w,
    );
    assert!(matches!(r, Err(CoreError::InvalidInput(_))));
    assert_eq!(w, WorkCounters::default());
    // strict starter: dense or Newton forbidden -> refused before callbacks
    let pj = linear_problem(nonnormal(10), true, true);
    for policy in [
        StarterPolicy {
            allow_dense_matrix: false,
            allow_newton: true,
        },
        StarterPolicy {
            allow_dense_matrix: true,
            allow_newton: false,
        },
    ] {
        let mut w = WorkCounters::default();
        let r = start_with_policy(&pj, 0.0, &[1.0; 10], 1e-3, policy, &mut w);
        assert!(matches!(r, Err(CoreError::InvalidInput(_))));
        assert_eq!(w, WorkCounters::default());
    }
    let mut w1 = WorkCounters::default();
    let mut w2 = WorkCounters::default();
    let a = start_with_policy(
        &pj,
        0.0,
        &[1.0; 10],
        1e-3,
        StarterPolicy::default(),
        &mut w1,
    )
    .unwrap();
    let b = ppsw1::start(&pj, 0.0, &[1.0; 10], 1e-3, &mut w2).unwrap();
    assert_eq!(a, b);
    assert_eq!(w1, w2);
    qual(
        "c3_refusals",
        json!({"no_jvp": "InvalidInput before callbacks",
               "starter_forbid_dense_or_newton": "InvalidInput before callbacks",
               "default_policy": "identical to ppsw1::start"}),
    );
}

// ------------------------------------------------------------------------------------------
// Full-IVP: Fisher MOL with J and JVP; adaptive PPSW1 under each backend
// ------------------------------------------------------------------------------------------

fn fisher(n: usize) -> (OdeProblem, Vec<f64>) {
    let dx = 1.0 / (n as f64 + 1.0);
    let inv = 1.0 / (dx * dx);
    let p = OdeProblem::new(
        "u04-fisher",
        n,
        Arc::new(move |_t: f64, u: &[f64], o: &mut [f64]| {
            for i in 0..n {
                let l = if i > 0 { u[i - 1] } else { 0.0 };
                let r = if i + 1 < n { u[i + 1] } else { 0.0 };
                o[i] = (l - 2.0 * u[i] + r) * inv + u[i] * (1.0 - u[i]);
            }
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, u: &[f64]| {
            let mut j = vec![vec![0.0; n]; n];
            for i in 0..n {
                j[i][i] = -2.0 * inv + 1.0 - 2.0 * u[i];
                if i > 0 {
                    j[i][i - 1] = inv;
                }
                if i + 1 < n {
                    j[i][i + 1] = inv;
                }
            }
            DenseMatrix::from_vec_rows(j)
        })),
        Some(Arc::new(
            move |_t: f64, u: &[f64], v: &[f64], o: &mut [f64]| {
                for i in 0..n {
                    let l = if i > 0 { v[i - 1] } else { 0.0 };
                    let r = if i + 1 < n { v[i + 1] } else { 0.0 };
                    o[i] = (l - 2.0 * v[i] + r) * inv + (1.0 - 2.0 * u[i]) * v[i];
                }
                Ok(())
            },
        )),
        None,
        false,
        None,
        None,
    )
    .unwrap();
    let y0 = (0..n)
        .map(|i| (std::f64::consts::PI * (i as f64 + 1.0) * dx).sin())
        .collect();
    (p, y0)
}

struct Run {
    y: Vec<f64>,
    status: String,
    startup: WorkCounters,
    steps: WorkCounters,
    seconds: f64,
}

fn run(
    p: &OdeProblem,
    y0: &[f64],
    t_end: f64,
    tol: f64,
    backend: &LinearBackend,
    threads: usize,
) -> Run {
    let clock = Instant::now();
    let mut sw = WorkCounters::default();
    let bound = ppsw1::jacobian_norm_bound(p, 0.0, y0, &mut sw).unwrap();
    let h0 = ppsw1::select_h0(bound, t_end / 100.0).unwrap();
    let s0 = start_with_policy(p, 0.0, y0, h0, StarterPolicy::default(), &mut sw).unwrap();
    let mut sess = Session::new(
        p,
        s0,
        Config::default(),
        ParallelExecution::rayon(threads).unwrap(),
    )
    .unwrap()
    .with_linear_backend(backend.clone())
    .unwrap();
    let tols = Tolerances {
        atol: tol * 1e-4,
        rtol: tol,
    };
    let mut h = h0;
    let mut status = "ok".to_string();
    while sess.state().t < t_end {
        let hh = h.min(t_end - sess.state().t);
        match sess.try_step(hh, &tols) {
            Ok(Decision::Accepted { next_ratio, .. })
            | Ok(Decision::Rejected { next_ratio, .. }) => h = hh * next_ratio,
            Err(e) => {
                status = format!("t={:.4e}: {e:?}", sess.state().t);
                break;
            }
        }
    }
    Run {
        y: sess.state().y[5].clone(),
        status,
        startup: sw,
        steps: sess.counters(),
        seconds: clock.elapsed().as_secs_f64(),
    }
}

fn wrms_diff(a: &[f64], b: &[f64], tol: f64) -> f64 {
    let s: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| ((x - y) / (tol * 1e-4 + tol * y.abs())).powi(2))
        .sum();
    (s / a.len() as f64).sqrt()
}

fn work_json(c: &WorkCounters) -> Value {
    json!({"rhs_calls": c.rhs_calls, "jacobian_builds": c.jacobian_builds,
           "lu": c.direct_factorizations, "direct_solves": c.direct_solve_calls,
           "jvp_calls": c.jvp_calls, "linear_iterations": c.linear_iterations,
           "linear_matvecs": c.linear_matvecs, "diagnostic_matvecs": c.diagnostic_matvecs,
           "inner_products": c.orthogonalization_inner_products,
           "vector_updates": c.orthogonalization_vector_updates,
           "linear_solves": c.linear_solves, "accepted": c.accepted_steps,
           "rejected": c.rejected_steps, "newton_iterations": c.nonlinear_iterations})
}

#[test]
fn c4_residual_to_step_error_budget_on_full_ivp() {
    let (p, y0) = fisher(40);
    let mut rows = Vec::new();
    for tol in [1e-5, 1e-8] {
        let dense = run(&p, &y0, 1.0, tol, &LinearBackend::Dense, 1);
        assert_eq!(dense.status, "ok");
        rows.push(
            json!({"tol": tol, "backend": "dense", "status": dense.status,
                         "startup": work_json(&dense.startup), "steps": work_json(&dense.steps)}),
        );
        for lin in [1e-2, 1e-4, 1e-6, 1e-8, 1e-10, 1e-12] {
            for (name, b) in [
                ("independent", indep(lin, 1e-300)),
                ("independent_restart10", indep_r(10, lin, 1e-300)),
                ("block", block(lin, 1e-300)),
            ] {
                let r = run(&p, &y0, 1.0, tol, &b, 1);
                let d = if r.status == "ok" {
                    json!(wrms_diff(&r.y, &dense.y, tol))
                } else {
                    Value::Null
                };
                rows.push(json!({"tol": tol, "backend": name, "linear_rtol": lin,
                                 "status": r.status, "endpoint_wrms_diff_vs_dense_in_tol_units": d,
                                 "steps": work_json(&r.steps)}));
            }
        }
    }
    qual("c4_budget", json!(rows));
    // with linear rtol <= 1e-3 * tol the backend must not move the endpoint by > 0.1 tol units
    for r in &rows {
        if let (Some(lin), Some(tol), Some(d)) = (
            r["linear_rtol"].as_f64(),
            r["tol"].as_f64(),
            r["endpoint_wrms_diff_vs_dense_in_tol_units"].as_f64(),
        ) && lin <= 1e-3 * tol
        {
            assert!(d <= 0.1, "{r}");
        }
    }
}

#[test]
#[ignore = "timing: run with --ignored for the crossover evidence"]
fn c5_measured_crossover_paired_interleaved() {
    let tol = 1e-6;
    let mut rows = Vec::new();
    for n in [25usize, 50, 100, 200, 400, 800] {
        let (p, y0) = fisher(n);
        let arms: Vec<(&str, LinearBackend, usize)> = vec![
            ("dense", LinearBackend::Dense, 1),
            ("mf_independent_seq", indep(1e-9, 1e-300), 1),
            ("mf_independent_par6", indep(1e-9, 1e-300), 6),
            ("mf_independent_r10_seq", indep_r(10, 1e-9, 1e-300), 1),
            ("mf_independent_r10_par6", indep_r(10, 1e-9, 1e-300), 6),
            ("mf_block", block(1e-9, 1e-300), 1),
        ];
        let mut times: Vec<Vec<f64>> = vec![Vec::new(); arms.len()];
        let mut last: Vec<Option<Run>> = (0..arms.len()).map(|_| None).collect();
        for rep in 0..9 {
            // rotate the arm order every repetition (interleaved, order-balanced)
            for k in 0..arms.len() {
                let a = (k + rep) % arms.len();
                let r = run(&p, &y0, 1.0, tol, &arms[a].1, arms[a].2);
                if rep >= 2 {
                    times[a].push(r.seconds);
                }
                last[a] = Some(r);
            }
        }
        let dense_y = last[0].as_ref().unwrap().y.clone();
        let median = |v: &[f64]| {
            let mut v = v.to_vec();
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        };
        let dense_med = median(&times[0]);
        for (a, (name, _, threads)) in arms.iter().enumerate() {
            let r = last[a].as_ref().unwrap();
            let med = median(&times[a]);
            rows.push(json!({"n": n, "arm": name, "threads": threads, "status": r.status,
                             "median_seconds": med, "repetitions": times[a].len(), "warmups": 2,
                             "speedup_vs_dense": dense_med / med,
                             "endpoint_wrms_diff_vs_dense_in_tol_units": wrms_diff(&r.y, &dense_y, tol),
                             "startup": work_json(&r.startup), "steps": work_json(&r.steps),
                             "stored_f64_model": if *name == "dense" { 3 * n * n } else { 6 * 61 * n }}));
        }
        eprintln!("crossover n={n} done");
    }
    qual("c5_crossover", json!(rows));
}

#[test]
fn c6_linear_budget_guard_refuses_before_any_callback() {
    let (p, y0) = fisher(20);
    let mut sw = WorkCounters::default();
    let s0 = ppsw1::start(&p, 0.0, &y0, 1e-4, &mut sw).unwrap();
    let mut rows = Vec::new();
    for (lin, tol, refused) in [
        (1e-8, 1e-8, true),
        (1e-9, 1e-8, true),
        (1e-10, 1e-8, false),
        (1e-7, 1e-5, false),
    ] {
        let mut sess = Session::new(
            &p,
            s0.clone(),
            Config::default(),
            ParallelExecution::sequential(),
        )
        .unwrap()
        .with_linear_backend(block(lin, 1e-300))
        .unwrap();
        let r = sess.try_step(
            1e-4,
            &Tolerances {
                atol: tol,
                rtol: tol,
            },
        );
        rows.push(json!({"linear_rtol": lin, "step_rtol": tol, "outcome": format!("{r:?}")}));
        if refused {
            assert!(matches!(r, Err(CoreError::InvalidInput(_))), "{r:?}");
            assert_eq!(sess.counters(), WorkCounters::default());
            assert_eq!(sess.state(), &s0);
        } else {
            assert!(r.is_ok(), "{r:?}");
        }
    }
    // Dense is never budget-guarded
    let mut sess =
        Session::new(&p, s0, Config::default(), ParallelExecution::sequential()).unwrap();
    assert!(
        sess.try_step(
            1e-4,
            &Tolerances {
                atol: 1e-8,
                rtol: 1e-8
            }
        )
        .is_ok()
    );
    qual("c6_budget_guard", json!(rows));
}

fn proc_status_kb(key: &str) -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    s.lines()
        .find(|l| l.starts_with(key))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
}

fn proc_cpu_ticks() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/self/stat").ok()?;
    // fields after the parenthesised command name; utime = 14th, stime = 15th overall
    let rest = &s[s.rfind(')')? + 2..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    Some((f.get(11)?.parse().ok()?, f.get(12)?.parse().ok()?))
}

/// One (n, arm) integration per process for isolated peak memory and CPU time.
/// Run by `research/production_unification_20261115/matrix_free_w/process_metrics.py`.
#[test]
#[ignore = "process metrics: one arm per process, driven by process_metrics.py"]
fn c7_single_arm_process_metrics() {
    let n: usize = std::env::var("U04_N").map_or(50, |v| v.parse().unwrap());
    let arm = std::env::var("U04_ARM").unwrap_or_else(|_| "dense".into());
    let (backend, threads) = match arm.as_str() {
        "dense" => (LinearBackend::Dense, 1),
        "mf_independent_seq" => (indep(1e-9, 1e-300), 1),
        "mf_independent_r10_par6" => (indep_r(10, 1e-9, 1e-300), 6),
        "mf_block" => (block(1e-9, 1e-300), 1),
        other => panic!("unknown arm {other}"),
    };
    let (p, y0) = fisher(n);
    let base_hwm = proc_status_kb("VmHWM:");
    let t0 = proc_cpu_ticks().unwrap();
    let r = run(&p, &y0, 1.0, 1e-6, &backend, threads);
    let t1 = proc_cpu_ticks().unwrap();
    qual(
        "c7_process_metrics",
        json!({"n": n, "arm": arm, "threads": threads, "status": r.status,
               "wall_seconds": r.seconds,
               "cpu_user_ticks": t1.0 - t0.0, "cpu_system_ticks": t1.1 - t0.1,
               "vm_hwm_kb_before": base_hwm, "vm_hwm_kb_after": proc_status_kb("VmHWM:"),
               "vm_rss_kb_after": proc_status_kb("VmRSS:"),
               "steps": work_json(&r.steps), "startup": work_json(&r.startup)}),
    );
}
