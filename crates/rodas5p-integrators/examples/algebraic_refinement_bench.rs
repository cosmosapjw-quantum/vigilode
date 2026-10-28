//! Small current-code kernel and fixed-step trajectory comparison.
//! Emits one JSON record per completed row, preserving unsuccessful candidates.
//! No claim of complete adaptive ODE solver performance follows from this probe.
use rodas5p_core::{DenseMatrix, LuFactorization};
use rodas5p_integrators::polynomial_resolvent::{
    DeclaredPolynomialResolvent as Plan, PolynomialRefinementPolicy as Policy,
    PolynomialRefinementWork as Work, PolynomialRefinementWorkspace as Workspace,
    PolynomialResolventError as Error, PolynomialResolventWorkspace as LegacyWorkspace,
};
use serde_json::{Value, json};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};
const TOL: f64 = 1e-10;
const SAMPLES: usize = 21;
fn apply(e: &[f64], x: &[f64], out: &mut [f64]) -> Result<(), Error> {
    let n = x.len();
    for (i, row) in e.chunks_exact(n).enumerate() {
        out[i] = -100.0 * x[i] + row.iter().zip(x).map(|(a, b)| a * b).sum::<f64>();
    }
    Ok(())
}
fn residual(e: &[f64], b: &[f64], x: &[f64], tmp: &mut [f64]) -> f64 {
    apply(e, x, tmp).unwrap();
    b.iter()
        .zip(x)
        .zip(tmp)
        .map(|((b, x), bx)| (b - x + *bx).abs())
        .fold(0.0, f64::max)
}
fn matrix(e: &[f64], n: usize) -> DenseMatrix {
    DenseMatrix::new(
        n,
        n,
        e.iter()
            .enumerate()
            .map(|(k, a)| if k / n == k % n { 101.0 - a } else { -a })
            .collect(),
    )
    .unwrap()
}
fn perturbation(n: usize) -> Vec<f64> {
    (0..n * n)
        .map(|k| {
            // Exact binary-rational bounded entries; sum |Eij|<=1/4.
            let sign = if ((k as u64)
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407)
                >> 61)
                & 1
                == 0
            {
                1.0
            } else {
                -1.0
            };
            sign * 0.25 / n as f64
        })
        .collect()
}
fn sample<F: FnMut() -> Value>(mut f: F) -> (f64, Value) {
    let start = Instant::now();
    let mut runs = 0;
    let mut value = Value::Null;
    while runs == 0 || start.elapsed() < Duration::from_millis(2) {
        value = black_box(f());
        runs += 1;
    }
    (start.elapsed().as_secs_f64() / runs as f64, value)
}
fn error(x: &[f64], reference: &[f64]) -> f64 {
    x.iter()
        .zip(reference)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max)
}
fn kernel(n: usize) {
    let e = perturbation(n);
    let m = matrix(&e, n);
    let lu = LuFactorization::new(&m).unwrap();
    let exact: Vec<f64> = (0..n).map(|i| ((i % 13) as f64 - 6.0) / 7.0).collect();
    let mut b = vec![0.0; n];
    apply(&e, &exact, &mut b).unwrap();
    for i in 0..n {
        b[i] = exact[i] - b[i];
    }
    let plan = Plan::new(&[100.0, 1.0], 0.25, 100.25, 1e-12).unwrap();
    let mut ws = Workspace::new(n);
    let mut legacy_ws = LegacyWorkspace::new(n);
    let mut out = vec![0.0; n];
    let mut tmp = vec![0.0; n];
    let modes = if n == 2 { 6 } else { 5 };
    let mut times = vec![Vec::new(); modes];
    let mut results = vec![Value::Null; modes];
    for repeat in 0..SAMPLES {
        for offset in 0..modes {
            let mode = (repeat + offset) % modes;
            let (t, result) = sample(|| {
                let mut work = Work::default();
                let mut refused_residual = None;
                let (accepted, corr) = match mode {
                    0 => {
                        let r = plan
                            .apply_checked(&b, &mut out, &mut legacy_ws, TOL, 0.0, |x, y| {
                                apply(&e, x, y)
                            })
                            .unwrap();
                        work.polynomial_actions_attempted = r.candidate_operator_actions as u64;
                        work.residual_actions_attempted = r.residual_operator_actions as u64;
                        work.operator_actions_completed = work.operator_actions_attempted();
                        refused_residual = Some(r.original_residual_norm);
                        (r.residual_accepted, 0)
                    }
                    1 => {
                        let p = Plan::new(&[100.0, 1.0], 0.25, 100.25, 1e-12).unwrap();
                        let mut w = Workspace::new(n);
                        let r = p
                            .apply_refined_checked(
                                &b,
                                &mut out,
                                &mut w,
                                Policy::default(),
                                TOL,
                                0.0,
                                |x, y| apply(&e, x, y),
                                &mut work,
                            )
                            .unwrap();
                        (r.accepted, r.corrections)
                    }
                    2 => {
                        let r = plan
                            .apply_refined_checked(
                                &b,
                                &mut out,
                                &mut ws,
                                Policy::default(),
                                TOL,
                                0.0,
                                |x, y| apply(&e, x, y),
                                &mut work,
                            )
                            .unwrap();
                        (r.accepted, r.corrections)
                    }
                    3 => {
                        out = LuFactorization::new(&m).unwrap().solve(&b).unwrap();
                        (residual(&e, &b, &out, &mut tmp) <= TOL, 0)
                    }
                    4 => {
                        out = lu.solve(&b).unwrap();
                        (residual(&e, &b, &out, &mut tmp) <= TOL, 0)
                    }
                    _ => {
                        let determinant = m[(0, 0)] * m[(1, 1)] - m[(0, 1)] * m[(1, 0)];
                        out[0] = (m[(1, 1)] * b[0] - m[(0, 1)] * b[1]) / determinant;
                        out[1] = (m[(0, 0)] * b[1] - m[(1, 0)] * b[0]) / determinant;
                        (residual(&e, &b, &out, &mut tmp) <= TOL, 0)
                    }
                };
                json!({"accepted":accepted,"absolute_error":if accepted{Some(error(&out,&exact))}else{None},"residual":if accepted{Some(residual(&e,&b,&out,&mut tmp))}else{refused_residual},"corrections":corr,"polynomial_operator_actions":work.operator_actions_attempted()})
            });
            times[mode].push(t);
            results[mode] = result;
        }
    }
    let names = [
        "legacy_single_quotient",
        "refined_with_setup",
        "refined_workspace_reused",
        "dense_lu_with_setup",
        "dense_lu_factor_reused",
        "specialized_direct_2x2",
    ];
    for i in 0..modes {
        println!(
            "{}",
            json!({"schema":"vigilode.algebraic-rescue-benchmark.v1","work_counter_scope":"Operator actions inside polynomial solvers only; LU residual/diagnostic actions and LU arithmetic are outside this counter but inside elapsed time","kind":"linear_system_kernel","case":"dense_near_scalar","dimension":n,"method":names[i],"residual_tolerance":TOL,"samples_seconds":times[i],"result":results[i],"relation_eta":0.25/101.0,"timing":"21 cyclic groups; each >=2ms; single process; output JSON construction and diagnostic checks included for all methods","setup":"matrix and RHS declared/preconstructed for all; setup modes include polynomial/workspace or LU construction; reused modes cache them"})
        );
    }
}
fn trajectory() {
    let n = 64;
    let e = perturbation(n);
    let m = matrix(&e, n);
    let lu = LuFactorization::new(&m).unwrap();
    let p = Plan::new(&[100.0, 1.0], 0.25, 100.25, 1e-12).unwrap();
    let u: Vec<f64> = (0..n).map(|i| ((i % 13) as f64 - 6.0) / 7.0).collect();
    let mut bu = vec![0.0; n];
    apply(&e, &u, &mut bu).unwrap();
    let h = 0.01;
    let steps = 50;
    let solve = |mode: usize| {
        let mut y = u.clone();
        let mut next = vec![0.0; n];
        let mut rhs = vec![0.0; n];
        let mut tmp = vec![0.0; n];
        let mut ws = Workspace::new(n);
        let mut legacy_ws = LegacyWorkspace::new(n);
        let mut work = Work::default();
        let mut completed = 0;
        let mut max_res: f64 = 0.0;
        for step in 1..=steps {
            let g = (-(step as f64) * h).exp();
            for i in 0..n {
                rhs[i] = y[i] - (bu[i] + h * u[i]) * g;
            }
            let accepted = match mode {
                0 => {
                    let r = p
                        .apply_checked(&rhs, &mut next, &mut legacy_ws, TOL, 0.0, |x, z| {
                            apply(&e, x, z)
                        })
                        .unwrap();
                    work.polynomial_actions_attempted += r.candidate_operator_actions as u64;
                    work.residual_actions_attempted += r.residual_operator_actions as u64;
                    work.operator_actions_completed +=
                        (r.candidate_operator_actions + r.residual_operator_actions) as u64;
                    max_res = max_res.max(r.original_residual_norm);
                    r.residual_accepted
                }
                1 => {
                    p.apply_refined_checked(
                        &rhs,
                        &mut next,
                        &mut ws,
                        Policy::default(),
                        TOL,
                        0.0,
                        |x, z| apply(&e, x, z),
                        &mut work,
                    )
                    .unwrap()
                    .accepted
                }
                _ => {
                    next = lu.solve(&rhs).unwrap();
                    residual(&e, &rhs, &next, &mut tmp) <= TOL
                }
            };
            if !accepted {
                break;
            }
            max_res = max_res.max(residual(&e, &rhs, &next, &mut tmp));
            std::mem::swap(&mut y, &mut next);
            completed += 1;
        }
        (y, completed, max_res, work)
    };
    let direct = solve(2).0;
    let exact: Vec<f64> = u.iter().map(|v| v * (-0.5_f64).exp()).collect();
    for mode in 0..3 {
        let mut timings = Vec::new();
        let mut result = Value::Null;
        for _ in 0..11 {
            let start = Instant::now();
            let (y, done, res, work) = solve(mode);
            timings.push(start.elapsed().as_secs_f64());
            result = json!({"accepted":done==steps,"completed_steps":done,"last_accepted_time":done as f64*h,"continuous_exact_error":if done==steps{Some(error(&y,&exact))}else{None},"same_discrete_method_error":if done==steps{Some(error(&y,&direct))}else{None},"max_original_residual":res,"operator_actions":work.operator_actions_attempted()});
        }
        println!(
            "{}",
            json!({"schema":"vigilode.algebraic-rescue-benchmark.v1","work_counter_scope":"Operator actions inside polynomial solvers only; LU residual/diagnostic actions and LU arithmetic are outside this counter but inside elapsed time","kind":"implicit_euler_trajectory","case":"manufactured_nonconstant_forced_stiff","method":(["legacy_single_quotient","refined_workspace_reused","dense_lu_factor_reused"][mode]),"dimension":n,"step":h,"steps":steps,"samples_seconds":timings,"result":result,"scope":"Actual fixed-step implicit Euler trajectory; all methods identical time discretization and source; not a RODAS trajectory speed result","timing":"11 serial samples per method, no speed-promotion inference; factor/plan reused, workspace setup and all steps included"})
        );
    }
}
fn main() {
    faer::set_global_parallelism(faer::Par::Seq);
    for n in [2, 32, 128, 256] {
        kernel(n);
    }
    trajectory();
}
