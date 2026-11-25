use rodas5p_core::directed::Interval;
use rodas5p_integrators::retro_green::RetroTable;

#[test]
fn immutable_table_and_uncertain_argument_enclose_phi() {
    let t = RetroTable::new().unwrap();
    let x = Interval::new(0.25, 0.5).unwrap();
    let p = t.phi(x, 0).unwrap();
    assert!(p.contains((-0.25_f64).exp()) && p.contains((-0.5_f64).exp()));
    let y = t.phi(Interval::point(0.5).unwrap(), 1).unwrap();
    assert!(y.contains(-(-0.5_f64).exp_m1() / 0.5));
    assert!(y.hi - y.lo < 1e-12);
}

use rodas5p_integrators::retro_green::{
    Model, Trajectory, Work, attempt, certify_polynomial, integrate,
};
use serde_json::json;
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn point(x: f64) -> Interval {
    Interval::point(x).unwrap()
}
fn exact_scalar(k: f64, beta: f64, y0: f64, t: f64) -> f64 {
    if k == 0. {
        y0 / (1. + 2. * beta * y0 * y0 * t).sqrt()
    } else {
        y0 * (-k * t).exp() / (1. + beta / k * y0 * y0 * (-(-2. * k * t).exp_m1())).sqrt()
    }
}
fn scalar(k: f64, beta: f64) -> Model {
    Model::new(vec![k], vec![0.], vec![0.], vec![beta]).unwrap()
}
#[test]
fn table_identity_domain_and_fallback() {
    let t = RetroTable::new().unwrap();
    let words = include_str!("../src/retro_phi_cell.hex");
    let mut changed = words.to_owned();
    changed.replace_range(0..1, "a");
    assert!(RetroTable::from_projection(&changed).is_err());
    assert!(RetroTable::from_projection("").is_err());
    assert!(t.phi(point(-0.1), 0).is_err());
    assert!(t.phi(point(16385.), 0).is_err());
    assert!(t.phi(point(0.), 4).is_err());
    assert!(t.phi(Interval { lo: 1., hi: 0. }, 0).is_err());
    for x in [
        0.,
        0.00000001,
        0.125,
        0.5,
        0.999999999999,
        1.,
        1.000000000001,
        1.25,
        4.,
        16.,
        64.,
        1000.,
        16384.,
    ] {
        for j in 0..=3 {
            let v = t.phi(point(x), j).unwrap();
            assert!(v.hi - v.lo < 2e-13, "x={x}, j={j}: {v:?}");
            println!(
                "PHI_CASE {}",
                json!({"x":bits(x),"j":j,"lo":bits(v.lo),"hi":bits(v.hi)})
            );
        }
    }
    let v = t.phi(Interval::new(0.999, 1.001).unwrap(), 0).unwrap();
    assert!(v.contains((-0.999_f64).exp()) && v.contains((-1.001_f64).exp()));
}
#[test]
fn original_defect_rejects_inaccurate_path_with_small_node_residual() {
    let t = RetroTable::new().unwrap();
    let m = scalar(1., 0.2);
    let y0 = [0.8];
    let h = 0.5;
    let mut work = Work::default();
    let s = attempt(&t, &m, &y0, h, 1e-10, None, &mut work).unwrap();
    let p = s.forcing()[0];
    let mut node = 0.0_f64;
    for theta in [0., 0.5, 1.] {
        let (u, _) = s.evaluate(&t, theta).unwrap();
        let d = p[0] + p[1] * theta + p[2] * theta * theta + 0.2 * u[0].powi(3);
        node = node.max(d.abs());
    }
    assert!(node < 1e-9);
    assert!(s.local_error_upper() > 1e-6);
    let mut observed = 0.0_f64;
    for i in 0..=100 {
        let theta = i as f64 / 100.;
        let (y, b) = s.evaluate(&t, theta).unwrap();
        let err = (y[0] - exact_scalar(1., 0.2, y0[0], h * theta)).abs();
        observed = observed.max(err);
        assert!(err <= b + 2e-15);
    }
    assert!(observed > 1e-6);
    let wrong = certify_polynomial(&t, &m, &y0, &[[0., 0., 0.]], h, &mut Work::default()).unwrap();
    assert!(wrong.local_error_upper() > observed);
    println!(
        "BAD_NODE {}",
        json!({"node_defect":node,"continuous_bound":s.local_error_upper(),"sampled_error":observed,"wrong_forcing_bound":wrong.local_error_upper(),"work":work})
    );
}
#[test]
fn future_forecast_is_only_an_initial_candidate() {
    let t = RetroTable::new().unwrap();
    let m = scalar(1., 0.2);
    let y0 = [0.8];
    let mut normal = Work::default();
    let mut bad = Work::default();
    let too_long = attempt(&t, &m, &y0, 0.125, 1e-5, None, &mut Work::default()).unwrap();
    assert!(too_long.local_error_upper() > 1e-5);
    let a = attempt(&t, &m, &y0, 0.03125, 1e-5, None, &mut normal).unwrap();
    let b = attempt(
        &t,
        &m,
        &y0,
        0.03125,
        1e-5,
        Some(&[vec![1.4], vec![-0.7]]),
        &mut bad,
    )
    .unwrap();
    assert!(a.local_error_upper() < 1e-5 && b.local_error_upper() < 1e-5);
    assert!((a.endpoint()[0] - b.endpoint()[0]).abs() < 1e-7);
    assert_eq!(b.evaluate(&t, 0.).unwrap(), (y0.to_vec(), 0.));
    println!(
        "FORECAST {}",
        json!({"normal_work":normal,"bad_work":bad,"endpoint_delta":(a.endpoint()[0]-b.endpoint()[0]).abs(),"normal_bound":a.local_error_upper(),"bad_bound":b.local_error_upper()})
    );
}
fn check_scalar_run(t: &RetroTable, k: f64, beta: f64, y0: f64, r: &Trajectory, tol: f64) -> f64 {
    let mut worst = 0.0_f64;
    let mut time = 0.;
    for seg in r.segments() {
        assert_eq!(seg.start(), time);
        for theta in [0., 0.125, 0.5, 0.875, 1.] {
            let (y, b) = seg.evaluate(t, theta).unwrap();
            let x = exact_scalar(k, beta, y0, seg.start() + theta * seg.slab().h());
            let e = (y[0] - x).abs();
            worst = worst.max(e);
            assert!(
                e <= b + 4e-15,
                "k={k} theta={theta} t={time} err={e} bound={b}"
            );
        }
        time += seg.slab().h();
    }
    assert_eq!(time, 0.5);
    assert!(r.error_upper() <= tol);
    assert!((r.endpoint()[0] - exact_scalar(k, beta, y0, 0.5)).abs() <= r.error_upper() + 4e-15);
    worst
}
#[test]
fn adaptive_scalar_dense_contract() {
    let t = RetroTable::new().unwrap();
    for (k, beta, tol) in [
        (0., 0.2, 1e-6),
        (1., 0.2, 1e-6),
        (12., 0.2, 1e-6),
        (1000., 0.2, 1e-5),
        (1., 0.2, 1e-8),
    ] {
        let m = scalar(k, beta);
        let r = integrate(&t, &m, &[0.8], 0.5, 0.125, tol).unwrap();
        let e = check_scalar_run(&t, k, beta, 0.8, &r, tol);
        println!(
            "SCALAR {}",
            json!({"k":k,"beta":beta,"tol":tol,"steps":r.segments().len(),"observed_error":e,"endpoint_error":(r.endpoint()[0]-exact_scalar(k,beta,0.8,0.5)).abs(),"endpoint_bound":r.error_upper(),"work":r.work()})
        );
    }
}
#[test]
fn multidimensional_coupling_and_invariant_exact_solution() {
    let t = RetroTable::new().unwrap();
    // y_i=x on the invariant diagonal. Row sums of C are k_i-1.
    // Therefore y_i'=-x-.2x^3 for every i, independently of the
    // different carrier rates and non-symmetric coupling.
    let k = vec![1.5, 2., 3.];
    let c = vec![0., 0.25, 0.25, 0.75, 0., 0.25, 1.5, 0.5, 0.];
    let m = Model::new(k, vec![0.; 3], c, vec![0.2; 3]).unwrap();
    assert!(m.log_norm_upper() <= -1.);
    let r = integrate(&t, &m, &[0.8; 3], 0.5, 0.125, 1e-6).unwrap();
    let mut worst = 0.0_f64;
    for seg in r.segments() {
        for theta in [0., 0.25, 0.5, 0.75, 1.] {
            let (y, b) = seg.evaluate(&t, theta).unwrap();
            let exact = exact_scalar(1., 0.2, 0.8, seg.start() + theta * seg.slab().h());
            for a in y {
                let e = (a - exact).abs();
                worst = worst.max(e);
                assert!(e <= b + 4e-15);
            }
        }
    }
    println!(
        "COUPLED {}",
        json!({"steps":r.segments().len(),"observed_error":worst,"endpoint_bound":r.error_upper(),"work":r.work()})
    );
    assert!(Model::new(vec![1., 1.], vec![0.; 2], vec![0., 2., 0., 0.], vec![0.; 2]).is_err());
    assert!(Model::new(vec![1.], vec![0.], vec![0.], vec![-1.]).is_err());
    assert!(integrate(&t, &m, &[f64::NAN; 3], 0.5, 0.125, 1e-6).is_err());
}

#[test]
#[ignore = "grouped owner-host timing of the actual IVP routes; common observation grid"]
fn diagnostic_work_precision() {
    use rodas5p_core::DenseMatrix;
    use rodas5p_integrators::{
        AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
    };
    use std::{hint::black_box, sync::Arc, time::Instant};
    faer::set_global_parallelism(faer::Par::Seq);
    for k in [1., 12., 1000.] {
        let beta = 0.2;
        let y0 = 0.8;
        let model = scalar(k, beta);
        let problem = OdeProblem::new(
            "retro-scalar",
            1,
            Arc::new(move |_: f64, y: &[f64], out: &mut [f64]| {
                out[0] = -k * y[0] - beta * y[0].powi(3);
                Ok(())
            }),
            None,
            Some(Arc::new(move |_: f64, y: &[f64]| {
                DenseMatrix::new(1, 1, vec![-k - 3. * beta * y[0] * y[0]])
            })),
            None,
            None,
            true,
            None,
            None,
        )
        .unwrap();
        let mut times: Vec<f64> = (0..=64).map(|i| i as f64 / 128.).collect();
        times.extend((0..=32).map(|i| i as f64 / 8192.));
        times.sort_by(f64::total_cmp);
        times.dedup();
        let reference: Vec<_> = times
            .iter()
            .map(|&x| exact_scalar(k, beta, y0, x))
            .collect();
        let output = OutputSchedule::new(times.clone()).unwrap();
        for (arm, tols) in [
            (0, [1e-4, 1e-5, 1e-6, 1e-7, 1e-8]),
            (1, [1e-5, 1e-7, 1e-9, 1e-11, 1e-13]),
        ] {
            for tol in tols {
                let mut samples = Vec::new();
                let mut last = serde_json::Value::Null;
                for rep in 0..6 {
                    let start = Instant::now();
                    let (values, meta) = if arm == 0 {
                        let table = RetroTable::new().unwrap();
                        let r = integrate(&table, &model, &[y0], 0.5, 0.125, tol).unwrap();
                        let mut idx = 0;
                        let mut vals = Vec::with_capacity(times.len());
                        let mut b = 0.0_f64;
                        for &x in &times {
                            while idx + 1 < r.segments().len()
                                && x > r.segments()[idx].start() + r.segments()[idx].slab().h()
                            {
                                idx += 1;
                            }
                            let s = &r.segments()[idx];
                            let (y, err) =
                                s.evaluate(&table, (x - s.start()) / s.slab().h()).unwrap();
                            vals.push(y[0]);
                            b = b.max(err);
                        }
                        (
                            vals,
                            json!({"steps":r.segments().len(),"work":r.work(),"endpoint_bound":r.error_upper(),"grid_bound":b}),
                        )
                    } else {
                        let a = AdaptiveStepConfig {
                            atol: tol,
                            rtol: tol,
                            initial_step: 0.0009765625,
                            min_step: 1e-14,
                            max_step: 0.125,
                            max_attempts: 100000,
                            ..AdaptiveStepConfig::default()
                        };
                        let r = integrate_rodas5p_fast_observed(
                            &problem,
                            (0., 0.5),
                            &[y0],
                            &a,
                            &output,
                        )
                        .unwrap();
                        assert!(r.observed.success, "{:?}", r.observed.message);
                        assert_eq!(r.observed.t, times);
                        (
                            r.observed.y.iter().map(|y| y[0]).collect::<Vec<_>>(),
                            json!({"steps":r.accepted_steps,"attempts":r.attempts,"counters":r.observed.counters}),
                        )
                    };
                    black_box(&values);
                    let ms = start.elapsed().as_secs_f64() * 1000.;
                    let err = values
                        .iter()
                        .zip(&reference)
                        .fold(0.0_f64, |e, (a, b)| e.max((a - b).abs()));
                    assert_eq!(values.len(), reference.len());
                    if arm == 0 {
                        assert!(err <= meta["grid_bound"].as_f64().unwrap() + 4e-15);
                    }
                    if rep > 0 {
                        samples.push(ms);
                    }
                    last = json!({"error":err,"endpoint_error":(values.last().unwrap()-reference.last().unwrap()).abs(),"metrics":meta});
                }
                let mut sorted = samples.clone();
                sorted.sort_by(f64::total_cmp);
                println!(
                    "WP {}",
                    json!({"arm":(if arm==0{"retro-green"}else{"rodas5p-fast"}),"k":k,"tol":tol,"grid":times.len(),"median_ms":sorted[2],"samples_ms":samples,"result":last})
                );
            }
        }
    }
}
