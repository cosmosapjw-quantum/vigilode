//! U03 independent qualification of the opt-in PPSW1-6b implementation.
//!
//! Oracles are independent of `ppsw1::tableau`'s closed form: coefficient fixture from
//! `research/production_unification_20261115/peer_qualification/oracle_coefficients.py`
//! (paper route R1: order-condition solves of eqs. (14)/(15) with beta from (23)), and the
//! theta-axis numpy backend `theta_oracle.py` (exact-J row used for cross-backend agreement).
//! Exact solutions are analytic (manufactured / Prothero-Robinson).  Each scan prints one
//! `QUAL {json}` line; the log is the raw evidence.  Assertions pin only properties the
//! evidence supports; unresolved regimes are printed, not hidden.  Scoped evidence, not G0.

#![allow(clippy::needless_range_loop)]

use std::sync::Arc;

use rodas5p_core::{CoreError, DenseMatrix, WorkCounters};
use rodas5p_integrators::ppsw1::{self, Config, Decision, Session, State, Tolerances};
use rodas5p_integrators::{OdeProblem, ParallelExecution};
use serde_json::{Value, json};

const ORACLE_TXT: &str = include_str!(
    "../../../research/production_unification_20261115/peer_qualification/coefficients_oracle.txt"
);
const THETA_JSON: &str = include_str!(
    "../../../research/production_unification_20261115/peer_qualification/theta_oracle_results.json"
);
/// Largest ratio used in schedules: strictly inside the 1.10 cap after rounding of h*sigma/h.
const SIG_HI: f64 = 1.099_999_999_999_9;

fn qual(tag: &str, v: Value) {
    println!("QUAL {}", json!({"test": tag, "data": v}));
}

type VecFn = Arc<dyn Fn(f64) -> Vec<f64> + Send + Sync>;

/// `y' = L (y - phi(t)) + phi'(t)`; exact solution `phi` when started on `phi`.
#[derive(Clone)]
struct Pr {
    l: Vec<Vec<f64>>,
    phi: VecFn,
    dphi: VecFn,
}

fn problem(pr: &Pr, mass: Option<DenseMatrix>) -> OdeProblem {
    let n = pr.l.len();
    let (l, phi, dphi) = (pr.l.clone(), pr.phi.clone(), pr.dphi.clone());
    let lj = pr.l.clone();
    OdeProblem::new(
        "u03-prothero-robinson",
        n,
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            let (p, dp) = (phi(t), dphi(t));
            for i in 0..n {
                out[i] = dp[i] + (0..n).map(|k| l[i][k] * (y[k] - p[k])).sum::<f64>();
            }
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, _y: &[f64]| {
            DenseMatrix::from_vec_rows(lj.clone())
        })),
        None,
        None,
        false,
        mass,
        None,
    )
    .unwrap()
}

fn scalar_pr(lambda: f64) -> Pr {
    Pr {
        l: vec![vec![lambda]],
        phi: Arc::new(|t| vec![(2.0 * t).sin() + 0.5 * (t / 3.0).exp()]),
        dphi: Arc::new(|t| vec![2.0 * (2.0 * t).cos() + (t / 3.0).exp() / 6.0]),
    }
}

/// LIN3-type nonnormal stiff system (same L and phi as theta_oracle.py).
fn lin3() -> Pr {
    Pr {
        l: vec![
            vec![-1.0, 1e2, 0.0],
            vec![0.0, -50.0, 1e3],
            vec![0.0, 0.0, -1000.0],
        ],
        phi: Arc::new(|t| vec![t.sin(), (2.0 * t).cos(), (-0.5 * t).exp()]),
        dphi: Arc::new(|t| vec![t.cos(), -2.0 * (2.0 * t).sin(), -0.5 * (-0.5 * t).exp()]),
    }
}

/// Exact peer data: previous stages at `t + h_prev (c_j - 1)`.
fn exact_state(pr: &Pr, t: f64, h_prev: f64) -> State {
    let c = ppsw1::nodes();
    State {
        t,
        y: std::array::from_fn(|j| (pr.phi)(t + h_prev * (c[j] - 1.0))),
        f: std::array::from_fn(|j| (pr.dphi)(t + h_prev * (c[j] - 1.0))),
        h_prev,
    }
}

fn err_at(pr: &Pr, s: &State) -> f64 {
    let p = (pr.phi)(s.t);
    s.y[5]
        .iter()
        .zip(&p)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}

fn run_fixed(pr: &Pr, hs: &[f64]) -> Result<(f64, State, WorkCounters), CoreError> {
    let p = problem(pr, None);
    let mut s = exact_state(pr, 0.0, hs[0]);
    let mut w = WorkCounters::default();
    let exec = ParallelExecution::sequential();
    for &h in hs {
        s = ppsw1::advance(&p, &s, h, &exec, &mut w)?.next;
    }
    Ok((err_at(pr, &s), s, w))
}

fn uniform(n: usize, t_end: f64) -> Vec<f64> {
    vec![t_end / n as f64; n]
}

/// Variable schedule with cycle product 1 (ratios SIG_HI x4, 1.05, compensating ~0.65).
fn variable(n: usize, t_end: f64) -> Vec<f64> {
    let cyc = [
        SIG_HI,
        SIG_HI,
        SIG_HI,
        SIG_HI,
        1.05,
        1.0 / (SIG_HI.powi(4) * 1.05),
    ];
    let mut raw = vec![1.0];
    for m in 1..n {
        raw.push(raw[m - 1] * cyc[(m - 1) % 6]);
    }
    let base = t_end / raw.iter().sum::<f64>();
    let mut hs = vec![base];
    for m in 1..n {
        hs.push(hs[m - 1] * cyc[(m - 1) % 6]);
    }
    hs
}

/// Ratio-pattern schedule: one step at `sigma`, then growth back to the base with SIG_HI.
fn pattern(sigma: f64, n: usize, t_end: f64) -> Vec<f64> {
    let mut ratios = vec![1.0];
    while ratios.len() < n {
        ratios.push(sigma);
        let mut level = sigma;
        while level < 1.0 / SIG_HI && ratios.len() < n {
            ratios.push(SIG_HI);
            level *= SIG_HI;
        }
        if ratios.len() < n {
            ratios.push((1.0 / level).min(SIG_HI));
        }
    }
    ratios.truncate(n);
    let mut raw = vec![1.0];
    for m in 1..n {
        raw.push(raw[m - 1] * ratios[m]);
    }
    let base = t_end / raw.iter().sum::<f64>();
    let mut hs = vec![base];
    for m in 1..n {
        hs.push(hs[m - 1] * ratios[m]);
    }
    hs
}

fn orders(e: &[f64]) -> Vec<f64> {
    e.windows(2).map(|w| (w[0] / w[1]).log2()).collect()
}

// -------------------------------------------------------------------------------------------
// q01: implementation tableau vs. independent route-R1 oracle
// -------------------------------------------------------------------------------------------

#[test]
fn q01_tableau_matches_independent_route_r1_oracle() {
    let mut gamma_oracle = f64::NAN;
    let mut by_sigma: std::collections::BTreeMap<u64, Vec<(String, usize, usize, f64)>> =
        Default::default();
    for line in ORACLE_TXT.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f[0] == "gamma" {
            gamma_oracle = f[1].parse().unwrap();
            continue;
        }
        let sigma: f64 = f[0].parse().unwrap();
        by_sigma.entry(sigma.to_bits()).or_default().push((
            f[1].to_string(),
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
            f[4].parse().unwrap(),
        ));
    }
    assert_eq!(ppsw1::gamma(), gamma_oracle, "gamma rounds to the same f64");
    assert_eq!(by_sigma.len(), 15);
    let mut rows = Vec::new();
    for (bits, entries) in &by_sigma {
        let sigma = f64::from_bits(*bits);
        let t = ppsw1::tableau(sigma).unwrap();
        let scale = entries.iter().map(|e| e.3.abs()).fold(1.0, f64::max);
        let mut max_abs = 0.0_f64;
        for (name, i, j, want) in entries {
            let got = match name.as_str() {
                "B" => t.b_matrix[*i][*j],
                "A" => t.a_matrix[*i][*j],
                "Gamma" => t.gamma_matrix[*i][*j],
                "beta" => t.beta[*i][*j],
                other => panic!("unknown matrix {other}"),
            };
            max_abs = max_abs.max((got - want).abs());
        }
        let rel = max_abs / (scale * f64::EPSILON);
        rows.push(
            json!({"sigma": sigma, "max_abs_coef": scale, "max_abs_diff": max_abs,
                         "diff_in_scale_eps": rel}),
        );
        assert!(
            rel <= 64.0,
            "sigma={sigma}: |impl - oracle| = {max_abs:e} ({rel} scale-eps)"
        );
    }
    qual("q01_coefficients", json!(rows));
}

// -------------------------------------------------------------------------------------------
// q02: W-method polynomial exactness through the actual stepping code
// -------------------------------------------------------------------------------------------

fn poly_pr(deg: i32) -> Pr {
    let coef = [[0.3, -1.2, 0.7], [1.0, 0.5, -0.25], [-0.4, 0.9, 0.6]];
    let phi = move |t: f64| -> Vec<f64> {
        (0..3)
            .map(|i| coef[i][0] + coef[i][1] * t + coef[i][2] * t.powi(deg))
            .collect()
    };
    let dphi = move |t: f64| -> Vec<f64> {
        (0..3)
            .map(|i| coef[i][1] + coef[i][2] * deg as f64 * t.powi(deg - 1))
            .collect()
    };
    Pr {
        l: vec![
            vec![-2.0, 40.0, 0.0],
            vec![0.0, -30.0, 400.0],
            vec![0.0, 0.0, -500.0],
        ],
        phi: Arc::new(phi),
        dphi: Arc::new(dphi),
    }
}

#[test]
fn q02_degree5_polynomials_are_reproduced_for_any_sigma_and_degree6_is_not() {
    // ratios: 1.1-cap, 0.5, a tiny clipping ratio, regrowth, and 0.9
    let mut hs = vec![0.1];
    for r in [SIG_HI, 0.5, 1e-3, SIG_HI, SIG_HI, 0.9, 1.0, 0.5] {
        let last = *hs.last().unwrap();
        hs.push(last * r);
    }
    let (e5, s5, _) = run_fixed(&poly_pr(5), &hs).unwrap();
    let (e6, _, _) = run_fixed(&poly_pr(6), &hs).unwrap();
    let mag = (poly_pr(5).phi)(s5.t)
        .iter()
        .fold(1.0_f64, |a, b| a.max(b.abs()));
    qual(
        "q02_polynomial_exactness",
        json!({"steps": hs.len(), "degree5_err": e5, "degree6_err": e6, "scale": mag}),
    );
    assert!(e5 <= 1e-11 * mag, "degree 5 not reproduced: {e5:e}");
    assert!(
        e6 > 1e3 * e5.max(1e-15),
        "degree 6 unexpectedly exact: {e6:e}"
    );
}

// -------------------------------------------------------------------------------------------
// q03/q05: Prothero-Robinson stiffness scan, uniform/variable/sigma patterns and clipping
// -------------------------------------------------------------------------------------------

#[test]
fn q03_prothero_robinson_stiffness_and_schedule_scan() {
    let ns = [16usize, 32, 64, 128];
    let mut rows = Vec::new();
    for lambda in [-1.0, -1e2, -1e4, -1e6] {
        for (kind, sched) in [
            ("uniform", uniform as fn(usize, f64) -> Vec<f64>),
            ("variable", variable),
        ] {
            let pr = scalar_pr(lambda);
            let errs: Vec<f64> = ns
                .iter()
                .map(|&n| {
                    run_fixed(&pr, &sched(n, 1.0))
                        .map(|r| r.0)
                        .unwrap_or(f64::INFINITY)
                })
                .collect();
            let ord = orders(&errs);
            rows.push(json!({"lambda": lambda, "schedule": kind, "N": ns,
                             "endpoint_error": errs, "observed_orders": ord}));
            assert!(
                errs.iter().all(|e| e.is_finite()),
                "lambda={lambda} {kind}: {errs:?}"
            );
            if lambda == -1.0 {
                let best = ord.iter().cloned().fold(f64::MIN, f64::max);
                assert!(best >= 4.8, "nonstiff {kind} order {ord:?}");
            }
        }
    }
    qual("q03_prothero_robinson", json!(rows));
}

#[test]
fn q05_sigma_patterns_and_final_clipping_ratios() {
    let ns = [24usize, 48, 96];
    let mut rows = Vec::new();
    for lambda in [-1.0, -1e4] {
        let pr = scalar_pr(lambda);
        for sigma in [0.5, 0.6, 0.7, 0.8, 0.9, 1.0, SIG_HI] {
            let errs: Vec<f64> = ns
                .iter()
                .map(|&n| run_fixed(&pr, &pattern(sigma, n, 1.0)).unwrap().0)
                .collect();
            rows.push(
                json!({"kind": "pattern", "lambda": lambda, "sigma": sigma, "N": ns,
                             "endpoint_error": errs, "observed_orders": orders(&errs)}),
            );
            assert!(errs.iter().all(|e| e.is_finite()));
        }
        // final clipping: uniform N = 48 to t = 1, then one clipped step sigma_clip * h
        let base = uniform(48, 1.0);
        let (e_base, _, _) = run_fixed(&pr, &base).unwrap();
        for clip in [0.3, 1e-2, 1e-4, 1e-6, 1e-9, 1e-12] {
            let mut hs = base.clone();
            hs.push(base[0] * clip);
            let r = run_fixed(&pr, &hs);
            let e = r.as_ref().map(|x| x.0).unwrap_or(f64::INFINITY);
            rows.push(
                json!({"kind": "final_clip", "lambda": lambda, "clip_ratio": clip,
                             "error_before_clip": e_base, "error_after_clip": e,
                             "outcome": r.as_ref().map(|_| "ok".to_string())
                                 .unwrap_or_else(|x| format!("{x:?}"))}),
            );
            assert!(
                e.is_finite() && e <= 10.0 * e_base.max(1e-14),
                "clip {clip}: {e:e}"
            );
        }
    }
    qual("q05_sigma_and_clipping", json!(rows));
}

// -------------------------------------------------------------------------------------------
// q04: LIN3 nonnormal stiff, cross-backend agreement with the independent numpy oracle
// -------------------------------------------------------------------------------------------

#[test]
fn q04_lin3_nonnormal_agrees_with_independent_theta1_backend() {
    let theta: Value = serde_json::from_str(THETA_JSON).unwrap();
    let rows = theta["nonnormal_pr_system"].as_array().unwrap();
    let mut out = Vec::new();
    for kind in ["uniform", "variable"] {
        let row = rows
            .iter()
            .find(|r| r["theta"] == 1.0 && r["eps"] == 0.0 && r["schedule"] == kind)
            .unwrap();
        let ns: Vec<usize> = row["N"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let py: Vec<f64> = row["endpoint_error"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let pr = lin3();
        let rs: Vec<f64> = ns
            .iter()
            .map(|&n| {
                let hs = if kind == "uniform" {
                    uniform(n, 1.0)
                } else {
                    // numpy schedule uses ratio 1.1 exactly; SIG_HI differs by 1e-13 relative
                    variable(n, 1.0)
                };
                run_fixed(&pr, &hs).unwrap().0
            })
            .collect();
        out.push(
            json!({"schedule": kind, "N": ns, "rust_error": rs, "numpy_error": py,
                        "rust_orders": orders(&rs)}),
        );
        // agreement where truncation error dominates roundoff (first two grids)
        for k in 0..2 {
            let ratio = rs[k] / py[k];
            assert!(
                (0.5..=2.0).contains(&ratio),
                "{kind} N={} rust {:e} numpy {:e}",
                ns[k],
                rs[k],
                py[k]
            );
        }
        assert!(rs.iter().all(|e| e.is_finite() && *e < 1e-5));
    }
    qual("q04_lin3_cross_backend", json!(out));
}

// -------------------------------------------------------------------------------------------
// q06: startup (Radau IIA starter, h0 rule) vs exact start
// -------------------------------------------------------------------------------------------

#[test]
fn q06_startup_h0_and_endpoint() {
    let mut rows = Vec::new();
    for (name, pr) in [
        ("pr_-1", scalar_pr(-1.0)),
        ("pr_-1e4", scalar_pr(-1e4)),
        ("lin3", lin3()),
    ] {
        let p = problem(&pr, None);
        let y0 = (pr.phi)(0.0);
        let mut wn = WorkCounters::default();
        let bound = ppsw1::jacobian_norm_bound(&p, 0.0, &y0, &mut wn).unwrap();
        let h_rule = ppsw1::select_h0(bound, 0.05).unwrap();
        for h0 in [h_rule, 1e-2, 1e-3] {
            let mut w = WorkCounters::default();
            let s = ppsw1::start(&p, 0.0, &y0, h0, &mut w).unwrap();
            assert_eq!(s.t, 2.0 * h0, "starter endpoint is t_init + 2 h0");
            let starter_iters = w.nonlinear_iterations;
            let c = ppsw1::nodes();
            let node_err = (0..6)
                .map(|j| {
                    let e = (pr.phi)(h0 * (1.0 + c[j]));
                    s.y[j]
                        .iter()
                        .zip(&e)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0, f64::max)
                })
                .fold(0.0, f64::max);
            // grow from h0 with the capped ratio to h_run, then uniform steps to t = 1
            let h_run = 1.0 / 48.0;
            let exec = ParallelExecution::sequential();
            let mut st = s.clone();
            let mut h = h0;
            let mut outcome = "ok".to_string();
            let mut steps = 0usize;
            while st.t < 1.0 {
                h = (h * SIG_HI).min(h_run).min(1.0 - st.t);
                match ppsw1::advance(&p, &st, h, &exec, &mut w) {
                    Ok(o) => st = o.next,
                    Err(e) => {
                        outcome = format!("{e:?}");
                        break;
                    }
                }
                steps += 1;
            }
            rows.push(
                json!({"problem": name, "h0": h0, "h0_from_rule": h_rule == h0,
                             "jacobian_norm_bound": bound, "starter_node_error": node_err,
                             "starter_newton_iterations": starter_iters, "steps": steps,
                             "endpoint_error_from_starter": err_at(&pr, &st), "t_final": st.t,
                             "outcome": outcome}),
            );
            assert!(node_err.is_finite());
        }
    }
    qual("q06_startup", json!(rows));
}

// -------------------------------------------------------------------------------------------
// q07: estimator vs true local error (exact previous data)
// -------------------------------------------------------------------------------------------

#[test]
fn q07_estimator_vs_true_local_error_and_rounding_floor() {
    let mut rows = Vec::new();
    for lambda in [-1.0, -1e4] {
        let pr = scalar_pr(lambda);
        let p = problem(&pr, None);
        for sigma in [0.5, 1.0, SIG_HI] {
            for h in [0.2, 0.1, 0.05, 0.025, 0.0125] {
                let s = exact_state(&pr, 0.3, h / sigma);
                let mut w = WorkCounters::default();
                let next = ppsw1::advance(&p, &s, h, &ParallelExecution::sequential(), &mut w)
                    .unwrap()
                    .next;
                let local = err_at(&pr, &next);
                let est = ppsw1::estimator_vector(&s, &next.y[5], next.h_prev / s.h_prev)[0].abs();
                rows.push(json!({"lambda": lambda, "sigma": sigma, "h": h,
                                 "true_local_error": local, "estimator": est,
                                 "estimator_over_local": est / local}));
            }
        }
    }
    let wsum: Vec<f64> = [0.5, 1.0, SIG_HI]
        .iter()
        .map(|&s| {
            ppsw1::estimator_weights(s)
                .iter()
                .map(|w| w.abs())
                .sum::<f64>()
        })
        .collect();
    let floor: Vec<f64> = wsum.iter().map(|w| (1.0 + w) * f64::EPSILON).collect();
    qual(
        "q07_estimator",
        json!({"rows": rows, "sum_abs_weights_sigma_0.5_1_1.1": wsum,
               "relative_rounding_floor": floor}),
    );
}

// -------------------------------------------------------------------------------------------
// q08: adaptive try_step driver (test-local), achieved error vs tolerance, full work
// -------------------------------------------------------------------------------------------

fn adaptive(pr: &Pr, tol: f64, t_end: f64, threads: usize, h_max: f64) -> Value {
    let p = problem(pr, None);
    let y0 = (pr.phi)(0.0);
    let mut sw = WorkCounters::default();
    let bound = ppsw1::jacobian_norm_bound(&p, 0.0, &y0, &mut sw).unwrap();
    let h0 = ppsw1::select_h0(bound, h_max).unwrap();
    let s0 = ppsw1::start(&p, 0.0, &y0, h0, &mut sw).unwrap();
    let exec = ParallelExecution::rayon(threads).unwrap();
    let mut sess = Session::new(&p, s0, Config::default(), exec).unwrap();
    let tolr = Tolerances {
        atol: tol,
        rtol: tol,
    };
    let mut h = h0;
    let (mut warn, mut min_clip, mut iters) = (0usize, f64::INFINITY, 0usize);
    let mut status = "ok".to_string();
    let mut trace: std::collections::VecDeque<Value> = Default::default();
    while sess.state().t < t_end {
        iters += 1;
        if iters > 200_000 {
            status = "step_limit".into();
            break;
        }
        let rem = t_end - sess.state().t;
        let mut hh = h;
        if hh >= rem {
            hh = rem;
            min_clip = min_clip.min(hh / sess.state().h_prev);
        }
        let (t_before, hp) = (sess.state().t, sess.state().h_prev);
        let res = sess.try_step(hh, &tolr);
        if trace.len() == 12 {
            trace.pop_front();
        }
        trace.push_back(json!({"t": t_before, "h": hh, "sigma": hh / hp,
                               "decision": format!("{res:?}")}));
        match res {
            Ok(Decision::Accepted {
                next_ratio,
                rounding_floor_warning,
                ..
            })
            | Ok(Decision::Rejected {
                next_ratio,
                rounding_floor_warning,
                ..
            }) => {
                warn += rounding_floor_warning as usize;
                h = hh * next_ratio;
            }
            Err(e) => {
                status = format!("{e:?}");
                break;
            }
        }
    }
    let c = sess.counters();
    let err = err_at(pr, sess.state());
    json!({"tol": tol, "threads": threads, "h_max_for_h0_rule": h_max, "status": status, "t_final": sess.state().t,
           "achieved_error": err, "h0": h0, "accepted": c.accepted_steps,
           "rejected": c.rejected_steps, "rhs_calls": c.rhs_calls + sw.rhs_calls,
           "jacobian_builds": c.jacobian_builds + sw.jacobian_builds,
           "lu": c.direct_factorizations + sw.direct_factorizations,
           "linear_solves": c.linear_solves + sw.linear_solves,
           "starter_newton_iterations": sw.nonlinear_iterations,
           "rounding_floor_warnings": warn, "min_final_clip_ratio": min_clip,
           "last_attempts": if status == "ok" { json!([]) } else { json!(trace) }})
}

#[test]
fn q08_adaptive_achieved_error_and_work() {
    let mut rows = Vec::new();
    for (name, pr) in [
        ("pr_-1", scalar_pr(-1.0)),
        ("pr_-1e4", scalar_pr(-1e4)),
        ("lin3", lin3()),
    ] {
        for tol in [1e-3, 1e-5, 1e-7, 1e-9, 1e-12] {
            let mut r = adaptive(&pr, tol, 1.0, 1, 0.01);
            r["problem"] = json!(name);
            // supported scope: tol >= 1e-9; tighter tolerances are recorded raw, not asserted
            if tol >= 1e-9 {
                assert_eq!(r["status"], "ok", "{name} tol {tol}: {r}");
                assert!(r["achieved_error"].as_f64().unwrap().is_finite());
            }
            rows.push(r);
        }
        // parallel execution reproduces the sequential result in error and counters
        let mut par = adaptive(&pr, 1e-7, 1.0, 4, 0.01);
        let seq = adaptive(&pr, 1e-7, 1.0, 1, 0.01);
        for k in [
            "achieved_error",
            "accepted",
            "rejected",
            "rhs_calls",
            "linear_solves",
        ] {
            assert_eq!(par[k], seq[k], "{name} {k}");
        }
        par["problem"] = json!(name);
        rows.push(par);
        // h0 rule with h_max = t_end / 2: the uncontrolled starter may span the whole interval
        for tol in [1e-3, 1e-9] {
            let mut r = adaptive(&pr, tol, 1.0, 1, 0.5);
            r["problem"] = json!(name);
            r["variant"] = json!("h_max_half_interval");
            rows.push(r);
        }
    }
    qual("q08_adaptive", json!(rows));
}

// -------------------------------------------------------------------------------------------
// q09: unsupported capabilities refuse before callbacks
// -------------------------------------------------------------------------------------------

#[test]
fn q09_mass_matrix_refused_before_any_callback() {
    let pr = scalar_pr(-1.0);
    let p = problem(&pr, Some(DenseMatrix::identity(1)));
    let mut w = WorkCounters::default();
    let r = ppsw1::start(&p, 0.0, &[1.0], 0.01, &mut w);
    assert!(matches!(r, Err(CoreError::InvalidInput(_))));
    assert_eq!(w, WorkCounters::default());
    let s = exact_state(&pr, 0.0, 0.01);
    let mut sess = Session::new(
        &p,
        s.clone(),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    assert!(matches!(
        sess.attempt(0.01),
        Err(CoreError::InvalidInput(_))
    ));
    assert_eq!(sess.counters(), WorkCounters::default());
    assert_eq!(sess.state(), &s);
    qual(
        "q09_refusals",
        json!({"mass_matrix": "InvalidInput before callbacks",
               "events": "no PPSW1 event API (unsupported by absence)",
               "dense_output": "none; endpoint reached only by final clipping"}),
    );
}

// -------------------------------------------------------------------------------------------
// q10: failure accounting of stage solves (scheduled 6 vs executed)
// -------------------------------------------------------------------------------------------

fn const_jac_problem(j: f64) -> OdeProblem {
    OdeProblem::new(
        "u03-failing-jacobian",
        1,
        Arc::new(|_t: f64, y: &[f64], out: &mut [f64]| {
            out[0] = -y[0];
            Ok(())
        }),
        None,
        Some(Arc::new(move |_t: f64, _y: &[f64]| {
            DenseMatrix::from_vec_rows(vec![vec![j]])
        })),
        None,
        None,
        false,
        None,
        None,
    )
    .unwrap()
}

fn one_state(h: f64, y: f64) -> State {
    State {
        t: 0.0,
        y: std::array::from_fn(|_| vec![y]),
        f: std::array::from_fn(|_| vec![-y]),
        h_prev: h,
    }
}

const TOL6: Tolerances = Tolerances {
    atol: 1e-6,
    rtol: 1e-6,
};

#[test]
fn q10_nan_jacobian_is_rejected_at_build_without_factorization_or_solves() {
    let p = const_jac_problem(f64::NAN);
    let s = one_state(0.1, 1.0);
    let mut sess = Session::new(
        &p,
        s.clone(),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    let d = sess.try_step(0.1, &TOL6).unwrap();
    assert!(matches!(d, Decision::Rejected { .. }));
    let c = sess.counters();
    assert_eq!(sess.state(), &s);
    assert_eq!(
        (c.jacobian_builds, c.direct_factorizations, c.linear_solves),
        (1, 0, 0),
        "{c:?}"
    );
    assert_eq!((c.rhs_calls, c.rejected_steps), (0, 1));
}

/// Finite J = f64::MAX with |transfer| > 1: W and its LU are finite, but every stage RHS
/// `explicit + h J transfer` overflows, so `LuFactorization::solve` refuses stage 0 (NonFinite).
fn overflow_session(threads: usize) -> (WorkCounters, Decision, bool) {
    let p = const_jac_problem(f64::MAX);
    let s = one_state(0.1, 10.0);
    let mut sess = Session::new(
        &p,
        s.clone(),
        Config::default(),
        ParallelExecution::rayon(threads).unwrap(),
    )
    .unwrap();
    let d = sess.try_step(0.1, &TOL6).unwrap();
    (sess.counters(), d, sess.state() == &s)
}

#[test]
fn q10_failed_stage_solve_charges_only_entered_sequential_solves() {
    let (c, d, same) = overflow_session(1);
    assert!(matches!(d, Decision::Rejected { .. }));
    assert!(same);
    assert_eq!((c.jacobian_builds, c.direct_factorizations), (1, 1));
    assert_eq!((c.linear_solves, c.direct_solve_calls), (1, 1), "{c:?}");
    assert_eq!((c.rhs_calls, c.rejected_steps), (0, 1));
}

#[test]
fn q10_failed_stage_solve_joins_and_charges_all_parallel_solves() {
    for _ in 0..20 {
        let (c, d, same) = overflow_session(4);
        assert!(matches!(d, Decision::Rejected { .. }));
        assert!(same);
        assert_eq!((c.linear_solves, c.direct_solve_calls), (6, 6), "{c:?}");
        assert_eq!(c.rhs_calls, 0);
    }
}

#[test]
fn q10_singular_w_linear_solve_error_preserves_state_and_counts_entered_solve() {
    let g = ppsw1::gamma();
    // find h with 1 - (h g) * (1 / (h g)) == 0 exactly so W is the exact zero matrix
    let (h, j) = (1..200)
        .map(|k| 0.05 + k as f64 * 1e-4)
        .map(|h| (h, 1.0 / (h * g)))
        .find(|&(h, j)| 1.0 - (h * g) * j == 0.0)
        .expect("exact singular shift");
    let p = const_jac_problem(j);
    let s = one_state(h, 1.0);
    for (threads, want) in [(1usize, 1u64), (4, 6)] {
        let mut sess = Session::new(
            &p,
            s.clone(),
            Config::default(),
            ParallelExecution::rayon(threads).unwrap(),
        )
        .unwrap();
        let r = sess.try_step(h, &TOL6);
        assert!(
            matches!(
                r,
                Err(CoreError::LinearSolve(_)) | Ok(Decision::Rejected { .. })
            ),
            "{r:?}"
        );
        let c = sess.counters();
        assert_eq!(sess.state(), &s);
        assert_eq!(c.linear_solves, want, "threads={threads} {c:?}");
        qual(
            "q10_singular_w",
            json!({"threads": threads, "outcome": format!("{r:?}"),
                   "linear_solves": c.linear_solves,
                   "direct_factorizations": c.direct_factorizations}),
        );
    }
}

// ===========================================================================================
// n-series: nonlinear qualification (turn 2). Formulas match nonlinear_reference.py exactly.
// Error E = endpoint WRMS with fixed weights |y_ref| + floor, plus the reference uncertainty
// U_ref (Radau 1e-13 vs 1e-12 + declared second method). Output grid = {t_end}.
// ===========================================================================================

const NL_REF: &str = include_str!(
    "../../../research/production_unification_20261115/peer_qualification/nonlinear_reference.json"
);

type RhsCl = Arc<dyn Fn(f64, &[f64], &mut [f64]) + Send + Sync>;
type JacCl = Arc<dyn Fn(f64, &[f64]) -> Vec<Vec<f64>> + Send + Sync>;

#[derive(Clone)]
struct Nl {
    name: &'static str,
    n: usize,
    f: RhsCl,
    j: JacCl,
    y0: Vec<f64>,
    t_end: f64,
    /// absolute tolerance = rtol * atol_factor (same for every solver arm)
    atol_factor: f64,
}

fn vdp(name: &'static str, eps: f64) -> Nl {
    Nl {
        name,
        n: 2,
        f: Arc::new(move |_t, y, o| {
            o[0] = y[1];
            o[1] = ((1.0 - y[0] * y[0]) * y[1] - y[0]) / eps;
        }),
        j: Arc::new(move |_t, y| {
            vec![
                vec![0.0, 1.0],
                vec![(-2.0 * y[0] * y[1] - 1.0) / eps, (1.0 - y[0] * y[0]) / eps],
            ]
        }),
        y0: vec![2.0, -0.66],
        t_end: 0.5,
        atol_factor: 1.0,
    }
}

fn robertson() -> Nl {
    Nl {
        name: "robertson",
        n: 3,
        f: Arc::new(|_t, y, o| {
            o[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
            o[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
            o[2] = 3e7 * y[1] * y[1];
        }),
        j: Arc::new(|_t, y| {
            vec![
                vec![-0.04, 1e4 * y[2], 1e4 * y[1]],
                vec![0.04, -1e4 * y[2] - 6e7 * y[1], -1e4 * y[1]],
                vec![0.0, 6e7 * y[1], 0.0],
            ]
        }),
        y0: vec![1.0, 0.0, 0.0],
        t_end: 40.0,
        atol_factor: 1e-6,
    }
}

fn fisher() -> Nl {
    const N: usize = 20;
    let dx = 1.0 / (N as f64 + 1.0);
    let inv = 1.0 / (dx * dx);
    Nl {
        name: "fisher_mol_n20",
        n: N,
        f: Arc::new(move |_t, u, o| {
            for i in 0..N {
                let left = if i > 0 { u[i - 1] } else { 0.0 };
                let right = if i < N - 1 { u[i + 1] } else { 0.0 };
                o[i] = (left - 2.0 * u[i] + right) * inv + u[i] * (1.0 - u[i]);
            }
        }),
        j: Arc::new(move |_t, u| {
            let mut j = vec![vec![0.0; N]; N];
            for i in 0..N {
                j[i][i] = -2.0 * inv + 1.0 - 2.0 * u[i];
                if i > 0 {
                    j[i][i - 1] = inv;
                }
                if i < N - 1 {
                    j[i][i + 1] = inv;
                }
            }
            j
        }),
        y0: (0..N)
            .map(|i| (std::f64::consts::PI * (i as f64 + 1.0) * dx).sin())
            .collect(),
        t_end: 1.0,
        atol_factor: 1e-4,
    }
}

fn nl_problems() -> Vec<Nl> {
    vec![
        vdp("vdp_eps_1e-1", 1e-1),
        vdp("vdp_eps_1e-3", 1e-3),
        robertson(),
        fisher(),
    ]
}

/// (callback count, cached Jacobian rows)
type StaleJacobian = (usize, Option<Vec<Vec<f64>>>);

/// `stale_every = k`: the Jacobian callback recomputes J only on every k-th call and otherwise
/// returns the cached (stale) matrix. k = 1 is the exact-J contract.
fn nl_problem(p: &Nl, stale_every: usize) -> OdeProblem {
    let f = p.f.clone();
    let j = p.j.clone();
    let cache: Arc<std::sync::Mutex<StaleJacobian>> = Default::default();
    OdeProblem::new(
        p.name,
        p.n,
        Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
            f(t, y, out);
            Ok(())
        }),
        None,
        Some(Arc::new(move |t: f64, y: &[f64]| {
            let mut c = cache.lock().unwrap();
            if c.0.is_multiple_of(stale_every) || c.1.is_none() {
                c.1 = Some(j(t, y));
            }
            c.0 += 1;
            DenseMatrix::from_vec_rows(c.1.clone().unwrap())
        })),
        None,
        None,
        false,
        None,
        None,
    )
    .unwrap()
}

struct NlRef {
    y: Vec<f64>,
    floor: f64,
    u_wrms: f64,
}

fn nl_ref(name: &str) -> NlRef {
    let v: Value = serde_json::from_str(NL_REF).unwrap();
    let p = &v["problems"][name];
    let arr = |k: &str| -> Vec<f64> {
        p[k].as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    };
    NlRef {
        y: arr("y_ref"),
        floor: p["weight_floor"].as_f64().unwrap(),
        u_wrms: p["u_ref_wrms"].as_f64().unwrap(),
    }
}

fn nl_wrms(y: &[f64], r: &NlRef) -> f64 {
    if y.len() != r.y.len() || !y.iter().all(|v| v.is_finite()) {
        return f64::INFINITY;
    }
    let s: f64 = y
        .iter()
        .zip(&r.y)
        .map(|(a, b)| ((a - b) / (b.abs() + r.floor)).powi(2))
        .sum();
    (s / y.len() as f64).sqrt()
}

fn counters_json(c: &WorkCounters) -> Value {
    json!({"rhs_calls": c.rhs_calls, "jacobian_builds": c.jacobian_builds,
           "lu": c.direct_factorizations, "linear_solves": c.linear_solves,
           "newton_iterations": c.nonlinear_iterations, "accepted": c.accepted_steps,
           "rejected": c.rejected_steps})
}

/// Fixed schedule after the real starter: h0 = h/16, growth by SIG_HI to h, final clip.
fn nl_fixed(p: &Nl, n_steps: usize, stale_every: usize) -> Value {
    let prob = nl_problem(p, stale_every);
    let h = p.t_end / n_steps as f64;
    let h0 = h / 16.0;
    let mut w = WorkCounters::default();
    let s0 = match ppsw1::start(&prob, 0.0, &p.y0, h0, &mut w) {
        Ok(s) => s,
        Err(e) => return json!({"status": format!("starter: {e:?}"), "error_wrms": null}),
    };
    let exec = ParallelExecution::sequential();
    let mut st = s0;
    let mut steps = 0usize;
    let mut status = "ok".to_string();
    while st.t < p.t_end {
        let rem = p.t_end - st.t;
        let mut hn = (st.h_prev * SIG_HI).min(h);
        if rem - hn < 0.01 * h {
            hn = rem;
        }
        match ppsw1::advance(&prob, &st, hn, &exec, &mut w) {
            Ok(o) => st = o.next,
            Err(e) => {
                status = format!("step {steps} t={:.4e}: {e:?}", st.t);
                break;
            }
        }
        steps += 1;
    }
    let r = nl_ref(p.name);
    let e = nl_wrms(&st.y[5], &r);
    json!({"status": status, "N": n_steps, "steps": steps, "t_final": st.t,
           "error_wrms": if status == "ok" { json!(e) } else { Value::Null },
           "counters": counters_json(&w)})
}

#[test]
fn n1_nonlinear_fixed_step_order_with_real_starter() {
    let mut rows = Vec::new();
    for p in nl_problems() {
        let r = nl_ref(p.name);
        let runs: Vec<Value> = [32usize, 64, 128, 256]
            .iter()
            .map(|&n| nl_fixed(&p, n, 1))
            .collect();
        let errs: Vec<f64> = runs
            .iter()
            .map(|x| x["error_wrms"].as_f64().unwrap_or(f64::NAN))
            .collect();
        rows.push(
            json!({"problem": p.name, "u_ref_wrms": r.u_wrms, "runs": runs,
                         "observed_orders": orders(&errs)}),
        );
    }
    qual("n1_nonlinear_fixed_order", json!(rows));
    // the nonstiff-ish VdP must converge on every grid (pins the stepping path, not stiff scope)
    let v = &rows[0]["runs"];
    for k in 0..4 {
        assert_eq!(v[k]["status"], "ok", "{}", rows[0]);
    }
}

#[test]
fn n2_jacobian_staleness_axis() {
    let mut rows = Vec::new();
    for p in nl_problems() {
        for k in [1usize, 2, 4, 8, 32] {
            let mut r = nl_fixed(&p, 128, k);
            r["problem"] = json!(p.name);
            r["jacobian_refresh_every"] = json!(k);
            rows.push(r);
        }
    }
    qual("n2_jacobian_staleness", json!(rows));
}

fn ppsw1_adaptive_nl(p: &Nl, tol: f64, h_max: f64, h0_override: Option<f64>) -> Value {
    let prob = nl_problem(p, 1);
    let (atol, rtol) = (tol * p.atol_factor, tol);
    let clock = std::time::Instant::now();
    let mut sw = WorkCounters::default();
    let bound = ppsw1::jacobian_norm_bound(&prob, 0.0, &p.y0, &mut sw).unwrap();
    let h0 = h0_override.unwrap_or_else(|| ppsw1::select_h0(bound, h_max).unwrap());
    let s0 = match ppsw1::start(&prob, 0.0, &p.y0, h0, &mut sw) {
        Ok(s) => s,
        Err(e) => {
            return json!({"solver": "ppsw1", "status": format!("starter: {e:?}"), "h0": h0,
                          "startup": counters_json(&sw)});
        }
    };
    let mut sess = Session::new(
        &prob,
        s0,
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    let tols = Tolerances { atol, rtol };
    let mut h = h0;
    let mut status = "ok".to_string();
    let mut iters = 0usize;
    while sess.state().t < p.t_end {
        iters += 1;
        if iters > 500_000 {
            status = "step_limit".into();
            break;
        }
        let rem = p.t_end - sess.state().t;
        let hh = h.min(rem);
        match sess.try_step(hh, &tols) {
            Ok(Decision::Accepted { next_ratio, .. })
            | Ok(Decision::Rejected { next_ratio, .. }) => h = hh * next_ratio,
            Err(e) => {
                status = format!("t={:.4e}: {e:?}", sess.state().t);
                break;
            }
        }
    }
    let wall = clock.elapsed().as_secs_f64();
    let r = nl_ref(p.name);
    let e = nl_wrms(&sess.state().y[5], &r);
    json!({"solver": "ppsw1", "status": status, "h0": h0, "t_final": sess.state().t,
           "error_wrms": if status == "ok" { json!(e) } else { Value::Null },
           "startup": counters_json(&sw), "steps": counters_json(&sess.counters()),
           "wall_seconds": wall, "timing_authority": "diagnostic_only"})
}

fn rodas5p_adaptive_nl(p: &Nl, tol: f64, h0: f64) -> Value {
    let prob = nl_problem(p, 1);
    let clock = std::time::Instant::now();
    let res = rodas5p_integrators::integrate_adaptive(
        &prob,
        (0.0, p.t_end),
        &p.y0,
        h0,
        rodas5p_integrators::IntegrationMethod::Sequential,
        None,
        None,
        tol * p.atol_factor,
        tol,
        1_000_000,
        p.t_end,
    );
    let wall = clock.elapsed().as_secs_f64();
    let r = nl_ref(p.name);
    match res {
        Ok(res) => {
            let e = nl_wrms(res.y.last().unwrap(), &r);
            let ok = res.success && (res.t.last().unwrap() - p.t_end).abs() <= 1e-12 * p.t_end;
            json!({"solver": "rodas5p_sequential_direct", "status": if ok { "ok".to_string() } else { res.message.clone() },
                   "h0": h0, "t_final": res.t.last(), "error_wrms": if ok { json!(e) } else { Value::Null },
                   "steps": counters_json(&res.counters), "attempts": res.attempts,
                   "wall_seconds": wall, "timing_authority": "diagnostic_only"})
        }
        Err(e) => json!({"solver": "rodas5p_sequential_direct", "status": format!("{e:?}"),
                         "h0": h0, "error_wrms": null}),
    }
}

#[test]
fn n3_matched_error_sweep_ppsw1_vs_production_rodas5p() {
    let mut rows = Vec::new();
    for p in nl_problems() {
        let r = nl_ref(p.name);
        for tol in [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9] {
            let h_max = p.t_end / 100.0;
            let mut a = ppsw1_adaptive_nl(&p, tol, h_max, None);
            let h0 = a["h0"].as_f64().unwrap();
            let mut b = rodas5p_adaptive_nl(&p, tol, h0);
            for x in [&mut a, &mut b] {
                x["problem"] = json!(p.name);
                x["tol"] = json!(tol);
                x["u_ref_wrms"] = json!(r.u_wrms);
                if let Some(e) = x["error_wrms"].as_f64() {
                    let cons = e + r.u_wrms;
                    x["conservative_error"] = json!(cons);
                    x["reference_admissible"] = json!(r.u_wrms <= 0.1 * cons);
                }
            }
            rows.push(a);
            rows.push(b);
        }
    }
    qual("n3_matched_error_sweep", json!(rows));
    // production comparator must complete on the calibration set (sanity of the harness)
    let fails: Vec<&Value> = rows
        .iter()
        .filter(|x| x["solver"] == "rodas5p_sequential_direct" && x["status"] != "ok")
        .collect();
    assert!(fails.len() <= 2, "comparator failures: {fails:?}");
}

#[test]
fn n4_robertson_starter_h0_scan_and_caller_h0_runs() {
    let p = robertson();
    let prob = nl_problem(&p, 1);
    let mut rows = Vec::new();
    for h0 in [0.4, 1e-2, 1e-3, 1e-4, 1e-5, 1e-6] {
        let mut w = WorkCounters::default();
        let r = ppsw1::start(&prob, 0.0, &p.y0, h0, &mut w);
        rows.push(json!({"h0": h0, "outcome": match &r { Ok(_) => "ok".to_string(), Err(e) => format!("{e:?}") },
                         "counters": counters_json(&w), "nonlinear_failures": w.nonlinear_failures}));
        if r.is_err() {
            assert_eq!(
                w.nonlinear_failures, 1,
                "h0={h0}: starter failure must be charged once"
            );
        }
    }
    // caller-chosen small h0 (outside the documented h0 rule) to separate starter from stepping
    let reference = nl_ref(p.name);
    let mut runs = Vec::new();
    for tol in [1e-3, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9] {
        let mut a = ppsw1_adaptive_nl(&p, tol, p.t_end / 100.0, Some(1e-6));
        a["problem"] = json!(p.name);
        a["tol"] = json!(tol);
        a["variant"] = json!("caller_h0_1e-6");
        if let Some(e) = a["error_wrms"].as_f64() {
            let cons = e + reference.u_wrms;
            a["conservative_error"] = json!(cons);
            a["reference_admissible"] = json!(reference.u_wrms <= 0.1 * cons);
        }
        runs.push(a);
    }
    qual(
        "n4_robertson_starter",
        json!({"starter_scan": rows, "caller_h0_runs": runs}),
    );
}

#[test]
fn n5_controller_cap_proposal_is_admissible_after_rounding() {
    let cap = ppsw1::ACCEPTED_STEP_SIGMA_MAX;
    // a previous step for which fl(fl(h_prev * cap) / h_prev) > cap
    let h_prev = (0..100_000)
        .map(|k| 1e-3 * (1.0 + k as f64 * 1e-5))
        .find(|&hp| (hp * cap) / hp > cap)
        .expect("rounding witness");
    let h = h_prev * cap;
    let pr = scalar_pr(-1.0);
    let p = problem(&pr, None);
    let s = exact_state(&pr, 0.3, h_prev);
    let mut sess = Session::new(
        &p,
        s.clone(),
        Config::default(),
        ParallelExecution::sequential(),
    )
    .unwrap();
    let ok = sess.attempt(h);
    qual(
        "n5_cap_rounding",
        json!({"h_prev": h_prev, "h": h, "h_over_h_prev": h / h_prev,
               "outcome": match &ok { Ok(_) => "ok".to_string(), Err(e) => format!("{e:?}") }}),
    );
    assert!(
        ok.is_ok(),
        "controller proposal h_prev*cap refused: {:?}",
        ok.err()
    );
    let mut sess2 =
        Session::new(&p, s, Config::default(), ParallelExecution::sequential()).unwrap();
    assert!(matches!(
        sess2.attempt(h.next_up()),
        Err(CoreError::InvalidInput(_))
    ));
    assert_eq!(sess2.counters(), WorkCounters::default());
}
