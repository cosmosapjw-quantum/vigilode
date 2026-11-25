use std::f64::consts::PI;
use vigilode::spatial::{BoxTransform, SeparableStages};
use vigilode::{Atol, Jacobian, Options, Problem, Rodas5p, VigilODE};

fn stencil(n: usize, d: f64, v: f64, c: f64, x: &[f64], out: &mut [f64]) {
    for i in 0..n {
        let left = x[(i + n - 1) % n];
        let right = x[(i + 1) % n];
        let adv = if v >= 0.0 {
            -v * (x[i] - left)
        } else {
            -v * (right - x[i])
        };
        out[i] = d * (left - 2.0 * x[i] + right) + adv + c * x[i];
    }
}
fn relative_error(a: &[f64], b: &[f64]) -> f64 {
    (a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>()
        / b.iter().map(|x| x * x).sum::<f64>())
    .sqrt()
}
#[test]
fn upwind_stage_solves_original_stencil() {
    for n in [2, 3, 17, 64, 256] {
        for velocity in [-1.0, 0.0, 1.0] {
            let d = 0.01 * (n as f64).powi(2);
            let v = velocity * n as f64;
            let c = -0.3;
            for z in [0.01, 1.0, 100.0] {
                let x: Vec<_> = (0..n)
                    .map(|i| 0.3 + (1.23 * i as f64).sin() + (0.87 * i as f64).cos())
                    .collect();
                let mut jx = vec![0.; n];
                stencil(n, d, v, c, &x, &mut jx);
                let b: Vec<_> = x.iter().zip(&jx).map(|(x, j)| z * x - j).collect();
                for transform in [BoxTransform::Complex, BoxTransform::PairedReal] {
                    let stages = SeparableStages::periodic_upwind_line(n, 1., 0.01, velocity, c)
                        .unwrap()
                        .with_box_transform(transform);
                    let mut actual = b.clone();
                    stages.solve_stage(z, &mut actual).unwrap();
                    assert!(
                        relative_error(&actual, &x) < 1e-10,
                        "n={n} v={v} z={z} error={}",
                        relative_error(&actual, &x)
                    );
                    stencil(n, d, v, c, &actual, &mut jx);
                    let recovered: Vec<_> =
                        actual.iter().zip(&jx).map(|(x, j)| z * x - j).collect();
                    assert!(relative_error(&recovered, &b) < 1e-10);
                }
            }
        }
    }
}

fn problem(n: usize, velocity: f64, c: f64, eps: f64) -> Problem {
    let d = 0.01 * (n as f64).powi(2);
    let v = velocity * n as f64;
    let apply = move |x: &[f64], out: &mut [f64]| {
        stencil(n, d, v, c, x, out);
        for i in 0..n {
            out[i] += eps * (2.0 * PI * i as f64 / n as f64).cos() * x[i];
        }
    };
    Problem::new(n, move |_, y, out| {
        apply(y, out);
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |_, _, x, out| {
        apply(x, out);
        Ok(())
    }))
    .autonomous()
}

fn initial(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| 1.0 + 0.1 * (2.0 * PI * i as f64 / n as f64).sin())
        .collect()
}
fn reference(n: usize, t: f64, velocity: f64) -> Vec<f64> {
    let th = 2.0 * PI / n as f64;
    let d = 0.01 * (n as f64).powi(2);
    let v = velocity * n as f64;
    let re = d * (2.0 * th.cos() - 2.0) - v.abs() * (1.0 - th.cos());
    let im = -v * th.sin();
    (0..n)
        .map(|j| 1.0 + 0.1 * (re * t).exp() * (th * j as f64 + im * t).sin())
        .collect()
}
fn common_error(y: &[f64], r: &[f64]) -> f64 {
    (y.iter()
        .zip(r)
        .map(|(a, b)| ((a - b) / (1.0 + b.abs())).powi(2))
        .sum::<f64>()
        / r.len() as f64)
        .sqrt()
}
fn options(rtol: f64) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(rtol),
        ..Options::default()
    }
}
fn report(sol: &vigilode::Solution) -> vigilode::spatial::SeparableReport {
    match &sol.stats.driver_work {
        vigilode::DriverWork::SeparableClient { report, .. } => *report,
        other => panic!("unexpected client {other:?}"),
    }
}
#[test]
fn original_periodic_advdiff_ivp_converges() {
    let n = 256;
    let y0 = initial(n);
    let r = reference(n, 1.0, 1.0);
    let p = problem(n, 1.0, 0., 0.);
    let mut errors = Vec::new();
    for tol in [1e-4, 1e-6, 1e-8] {
        let stages = SeparableStages::periodic_upwind_line(n, 1., 0.01, 1., 0.).unwrap();
        let sol = VigilODE::solve_separable(&p, (0., 1.), &y0, &options(tol), &stages).unwrap();
        assert!(sol.is_success(), "{:?}", sol.status);
        let y = sol.final_state().unwrap();
        let e = common_error(y, &r);
        let rep = report(&sol);
        assert_eq!(rep.periodic_upwind_rate, Some(256.0));
        assert_eq!(rep.corrections, 0);
        assert_eq!(rep.solves, rep.residual_checks);
        assert!(rep.max_backward_error <= 1e-10);
        assert_eq!(sol.stats.counters.linear_iterations, 0);
        let drift = (y.iter().sum::<f64>() - y0.iter().sum::<f64>()).abs() / n as f64;
        assert!(drift < 1e-12);
        assert!(e < tol, "tol={tol} err={e}");
        errors.push(e);
        println!("IVP tol={tol} error={e} drift={drift} report={rep:?}");
    }
    assert!(errors.windows(2).all(|e| e[1] < e[0]), "{errors:?}");
}

#[test]
fn upwind_preconditioner_corrects_symmetry_breaking() {
    let n = 17;
    let eps = 0.03;
    let c = -0.2;
    let velocity = 1.0;
    let p = problem(n, velocity, c, eps);
    let y0 = initial(n);
    let stages = SeparableStages::periodic_upwind_line(n, 1., 0.01, velocity, c).unwrap();
    let sol = VigilODE::solve_separable(&p, (0., 0.4), &y0, &options(1e-7), &stages).unwrap();
    assert!(sol.is_success(), "{:?}", sol.status);
    let rep = report(&sol);
    assert!(rep.corrections > 0 && rep.residual_checks > rep.solves);
    assert!(rep.max_backward_error <= 1e-10);
    // Independent time integrator: classical RK4 at dt=1e-4, using original stencil.
    let d = 0.01 * (n as f64).powi(2);
    let v = velocity * n as f64;
    let apply = |x: &[f64], out: &mut [f64]| {
        stencil(n, d, v, c, x, out);
        for i in 0..n {
            out[i] += eps * (2.0 * PI * i as f64 / n as f64).cos() * x[i];
        }
    };
    let mut y = y0;
    let mut k1 = vec![0.; n];
    let mut k2 = k1.clone();
    let mut k3 = k1.clone();
    let mut k4 = k1.clone();
    let mut stage = k1.clone();
    let h = 1e-4;
    for _ in 0..4000 {
        apply(&y, &mut k1);
        for i in 0..n {
            stage[i] = y[i] + 0.5 * h * k1[i];
        }
        apply(&stage, &mut k2);
        for i in 0..n {
            stage[i] = y[i] + 0.5 * h * k2[i];
        }
        apply(&stage, &mut k3);
        for i in 0..n {
            stage[i] = y[i] + h * k3[i];
        }
        apply(&stage, &mut k4);
        for i in 0..n {
            y[i] += h * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0;
        }
    }
    let e = relative_error(sol.final_state().unwrap(), &y);
    println!("BROKEN_SYMMETRY error={e} report={rep:?}");
    assert!(e < 1e-6);
}

#[test]
fn upwind_guard_and_spectral_semantics() {
    for (n, l, d, v, c) in [
        (1, 1., 0.01, 1., 0.),
        (4, 0., 0.01, 1., 0.),
        (4, 1., -0.01, 1., 0.),
        (4, 1., 0.01, f64::NAN, 0.),
        (4, 1., 0.01, 1., f64::INFINITY),
    ] {
        assert!(SeparableStages::periodic_upwind_line(n, l, d, v, c).is_err());
    }
    let stages = SeparableStages::periodic_upwind_line(4, 1., 0.01, 1., 2.).unwrap();
    assert_eq!(stages.inertia(1.0), 1);
    assert!(stages.solve_stage(1.0, &mut [1.; 4]).is_err());
    assert!(stages.spectral_bounds().explicit_step_bound.is_nan());
    assert!(
        stages
            .solve_stage(3.0, &mut [1., f64::NAN, 1., 1.])
            .is_err()
    );
    let huge = SeparableStages::periodic_upwind_line(4, 1., 0., 0., -f64::MAX).unwrap();
    assert!(huge.solve_stage(f64::MAX, &mut [1.; 4]).is_err());
    let pure = SeparableStages::periodic_upwind_line(4, 1., 0.01, 0., 0.).unwrap();
    assert!(pure.spectral_bounds().explicit_step_bound.is_finite());
    let mut prepared = stages.stage_solver(3.0).unwrap();
    assert!(prepared(&mut [1.; 3]).is_err());
    for _ in 0..2 {
        let mut b = [1.; 4];
        prepared(&mut b).unwrap();
        assert!(b.iter().all(|x| (*x - 1.).abs() < 1e-12));
    }
}

#[test]
#[ignore = "diagnostic paired endpoint timing, run pinned, one test thread"]
fn measure_upwind_work_precision() {
    use std::time::Instant;
    let n = 256;
    let p = problem(n, 1., 0., 0.);
    let y0 = initial(n);
    let r = reference(n, 1., 1.);
    for tol in [1e-4, 1e-5, 1e-6, 1e-7, 1e-8] {
        let opts = options(tol);
        let solve = |arm: usize| {
            if arm == 0 {
                Rodas5p::solve(&p, (0., 1.), &y0, &opts)
            } else {
                let s = SeparableStages::periodic_upwind_line(n, 1., 0.01, 1., 0.)?;
                VigilODE::solve_separable(&p, (0., 1.), &y0, &opts, &s)
            }
        };
        for arm in 0..2 {
            assert!(solve(arm).unwrap().is_success());
        }
        let mut times = [Vec::new(), Vec::new()];
        let mut last = [None, None];
        for rep in 0..7 {
            for arm in [rep % 2, 1 - rep % 2] {
                let start = Instant::now();
                let sol = solve(arm).unwrap();
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                assert!(sol.is_success(), "{:?}", sol.status);
                let e = common_error(sol.final_state().unwrap(), &r);
                if let Some((prev, _, _, _, _, _)) = last[arm] {
                    assert_eq!(e, prev);
                }
                times[arm].push(ms);
                let c = &sol.stats.counters;
                last[arm] = Some((
                    e,
                    sol.stats.accepted_steps,
                    sol.stats.rejected_steps,
                    c.rhs_evaluations,
                    c.jvp_vectors,
                    c.linear_iterations,
                ));
            }
        }
        for arm in 0..2 {
            let (error, accepted, rejected, rhs, jvp, iters) = last[arm].unwrap();
            let mut sorted = times[arm].clone();
            sorted.sort_by(f64::total_cmp);
            let row = serde_json::json!({"arm":if arm==0{"shipped-gmres"}else{"verified-fourier-upwind"},"rtol":tol,"error":error,"median_ms":sorted[3],"samples_ms":times[arm],"accepted":accepted,"rejected":rejected,"rhs":rhs,"jvp":jvp,"linear_iterations":iters});
            println!("CSAS_WP {row}");
        }
    }
}
