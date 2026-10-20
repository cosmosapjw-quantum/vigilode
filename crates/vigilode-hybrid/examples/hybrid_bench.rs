//! Native trajectory comparison. External methods do not claim hybrid certificates.
use diffsol::{DenseMatrix as _, FaerLU, FaerMat, MatrixCommon, OdeBuilder, OdeSolverMethod};
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
};
use serde_json::{Value, json};
use std::{sync::Arc, time::Instant};
use vigilode_hybrid::{IntegrationConfig, Model, Work, solve_trajectory};
fn initial(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            0.45 + 0.12 * (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos()
                + 0.03 * (7.0 * std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos()
        })
        .collect()
}
fn model(n: usize, eps: f64) -> Model {
    Model::new(n, 0.01 * (n * n) as f64, 2.0, 0.4, eps).unwrap()
}
fn entries(m: &Model, y: &[f64], mut set: impl FnMut(usize, usize, f64)) {
    let n = m.n();
    let [d, lambda, alpha, eps] = m.params();
    let mean = y.iter().sum::<f64>() / n as f64;
    for i in 0..n {
        for j in 0..n {
            let mut v = alpha * (1.0 - y[i]) / n as f64;
            if i == j {
                v += -lambda
                    - alpha * mean
                    - 2.0 * eps * y[i]
                    - d * (if i == 0 || i + 1 == n { 1.0 } else { 2.0 });
            } else if i.abs_diff(j) == 1 {
                v += d;
            }
            set(i, j, v);
        }
    }
}
// Pointers are supplied only by the pinned private Fortran bridge; its buffers
// have the declared lengths. No user-provided pointers enter the safe library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strict_rhs(id: i32, n: i32, _t: f64, y: *const f64, out: *mut f64) {
    let y = unsafe { std::slice::from_raw_parts(y, n as usize) };
    let out = unsafe { std::slice::from_raw_parts_mut(out, n as usize) };
    if model(n as usize, id as f64)
        .rhs_into(y, out, &mut Work::default())
        .is_err()
    {
        out.fill(f64::NAN);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strict_jac(
    id: i32,
    n: i32,
    _ml: i32,
    _mu: i32,
    ld: i32,
    _t: f64,
    y: *const f64,
    pd: *mut f64,
) {
    let n = n as usize;
    let ld = ld as usize;
    let y = unsafe { std::slice::from_raw_parts(y, n) };
    let pd = unsafe { std::slice::from_raw_parts_mut(pd, ld * n) };
    pd.fill(0.0);
    entries(&model(n, id as f64), y, |i, j, v| pd[i + ld * j] = v);
}
unsafe extern "C" {
    fn strict_vode(
        id: i32,
        n: i32,
        nt: i32,
        tf: f64,
        y0: *const f64,
        rtol: f64,
        atol: f64,
        h0: f64,
        times: *const f64,
        band: i32,
        landing: i32,
        values: *mut f64,
        reached: *mut f64,
        completed: *mut i32,
        code: *mut i32,
        stats: *mut i32,
    );
}
struct Run {
    ok: bool,
    states: Vec<Vec<f64>>,
    detail: Value,
}
fn run(m: &Model, ts: &[f64], arm: &str, tol: f64, workers: usize) -> Result<Run, String> {
    let n = m.n();
    let y = initial(n);
    if arm == "diffsol" {
        let a = m.clone();
        let b = m.clone();
        let p = OdeBuilder::<FaerMat<f64>>::new()
            .rtol(tol)
            .atol(vec![tol * 0.01; n])
            .h0(1e-4)
            .rhs_implicit(
                move |x, _p, _t, out| {
                    let xx: Vec<_> = (0..n).map(|i| x[i]).collect();
                    let yy = a.rhs(&xx).unwrap_or_else(|_| vec![f64::NAN; n]);
                    for i in 0..n {
                        out[i] = yy[i];
                    }
                },
                move |x, _p, _t, v, out| {
                    let xx: Vec<_> = (0..n).map(|i| x[i]).collect();
                    let vv: Vec<_> = (0..n).map(|i| v[i]).collect();
                    let yy = b.jvp(&xx, &vv).unwrap_or_else(|_| vec![f64::NAN; n]);
                    for i in 0..n {
                        out[i] = yy[i];
                    }
                },
            )
            .init(
                move |_p, _t, out| {
                    for i in 0..n {
                        out[i] = y[i];
                    }
                },
                n,
            )
            .build()
            .map_err(|e| e.to_string())?;
        let mut solver = p.bdf::<FaerLU<f64>>().map_err(|e| e.to_string())?;
        let (mat, why) = solver.solve_dense(ts).map_err(|e| e.to_string())?;
        let states: Vec<Vec<f64>> = (0..mat.ncols())
            .map(|k| (0..n).map(|i| mat.get_index(i, k)).collect())
            .collect();
        return Ok(Run {
            ok: mat.ncols() == ts.len() && mat.nrows() == n,
            states,
            detail: json!({"statistics":solver.get_statistics(),"stop":format!("{why:?}"),"continuous_certificate":false,"adapter":"slice-copy-to-common-model; allocation cost retained"}),
        });
    }
    if arm == "dvode" {
        let mut vals = vec![f64::NAN; n * ts.len()];
        let (mut reached, mut count, mut code) = (0.0, 0, 0);
        let mut stats = [0i32; 8];
        unsafe {
            strict_vode(
                m.params()[3] as i32,
                n as i32,
                ts.len() as i32,
                *ts.last().unwrap(),
                y.as_ptr(),
                tol,
                0.01 * tol,
                1e-4,
                ts.as_ptr(),
                -1,
                0,
                vals.as_mut_ptr(),
                &mut reached,
                &mut count,
                &mut code,
                stats.as_mut_ptr(),
            );
        }
        let got = (count.max(0) as usize).min(ts.len());
        return Ok(Run {
            ok: code == 2 && got == ts.len(),
            states: vals[..got * n].chunks(n).map(|x| x.to_vec()).collect(),
            detail: json!({"stats":stats,"code":code,"reached":reached,"continuous_certificate":false,"jacobian":"analytic dense native adapter"}),
        });
    }
    if arm == "fast" {
        let a = m.clone();
        let b = m.clone();
        let p = OdeProblem::new(
            "hybrid-generated-fast",
            n,
            Arc::new(move |_, y, out| {
                a.rhs_into(y, out, &mut Work::default())
                    .map_err(rodas5p_core::CoreError::InvalidInput)
            }),
            None,
            Some(Arc::new(move |_, y| {
                let mut j = DenseMatrix::zeros(n, n);
                entries(&b, y, |i, k, v| j[(i, k)] = v);
                Ok(j)
            })),
            None,
            None,
            true,
            None,
            None,
        )
        .map_err(|e| e.to_string())?;
        let cfg = AdaptiveStepConfig {
            rtol: tol,
            atol: tol * 0.01,
            initial_step: 1e-4,
            min_step: 1e-14,
            max_step: 0.2,
            max_attempts: 10000,
            ..AdaptiveStepConfig::default()
        };
        let schedule = OutputSchedule::new(ts.to_vec()).map_err(|e| e.to_string())?;
        let r = integrate_rodas5p_fast_observed(&p, (0.0, 0.2), &y, &cfg, &schedule)
            .map_err(|e| e.to_string())?;
        return Ok(Run {
            ok: r.observed.success,
            states: r.observed.y,
            detail: json!({"work":r.observed.counters,"accepted":r.accepted_steps,"rejected":r.rejected_steps,"continuous_certificate":false}),
        });
    }
    let cfg = IntegrationConfig {
        abs_tolerance: tol,
        ..IntegrationConfig::default()
    };
    let r = solve_trajectory(m, &y, ts, &cfg, arm, workers)?;
    Ok(Run {
        ok: r.integration.success,
        states: r.integration.states.clone(),
        detail: serde_json::to_value(r).map_err(|e| e.to_string())?,
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() != 6 {
        return Err("hybrid_bench N EPS ARM TOL REPS WORKERS".into());
    }
    let n: usize = a[0].parse()?;
    let eps: f64 = a[1].parse()?;
    let tol: f64 = a[3].parse()?;
    let reps: usize = a[4].parse()?;
    let workers: usize = a[5].parse()?;
    if ![16, 64, 256].contains(&n)
        || ![0.0, 1.0].contains(&eps)
        || !tol.is_finite()
        || tol <= 0.0
        || tol > 0.1
        || reps == 0
        || reps > 10
    {
        return Err("unregistered benchmark input".into());
    }
    let m = model(n, eps);
    let ts: Vec<_> = (0..=20).map(|i| 0.2 * i as f64 / 20.0).collect();
    let _warm = run(&m, &ts, &a[2], tol, workers);
    let mut samples = Vec::new();
    let mut outputs = Vec::new();
    for _ in 0..reps {
        let t = Instant::now();
        let r = run(&m, &ts, &a[2], tol, workers);
        samples.push(t.elapsed().as_nanos());
        outputs.push(r);
    }
    let first = outputs[0].as_ref();
    let ok = outputs.iter().all(|r| {
        r.as_ref().is_ok_and(|r| {
            r.ok && r.states.len() == ts.len()
                && r.states
                    .iter()
                    .all(|s| s.len() == n && s.iter().all(|v| v.is_finite()))
        })
    });
    let deterministic = outputs.iter().all(|r| match (first, r.as_ref()) {
        (Ok(a), Ok(b)) => a.states.iter().flatten().map(|x| x.to_bits()).eq(b
            .states
            .iter()
            .flatten()
            .map(|x| x.to_bits())),
        (Err(a), Err(b)) => a == b,
        _ => false,
    });
    println!(
        "{}",
        json!({"schema":"hybrid-trajectory-v1","n":n,"epsilon":eps,"arm":a[2],"tol":tol,"workers":workers,"model":m,"initial":initial(n),"times":ts,"success":ok,"deterministic":deterministic,"elapsed_ns":samples,"warmups":1,"reps":reps,"states":first.ok().map(|r|&r.states),"details":first.ok().map(|r|&r.detail),"error":first.err(),"timer":"whole trajectory including per-trajectory setup/pool and certificate; excluding serialization/result destruction"})
    );
    Ok(())
}
