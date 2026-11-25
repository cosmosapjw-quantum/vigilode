use rodas5p_integrators::retro_nonlinear::JetTable;
#[test]
fn total_degree_table_retains_mixed_channels() {
    let t = JetTable::new(3, 4).unwrap();
    assert_eq!(t.channels(), 35);
}

use rodas5p_core::DenseMatrix;
use rodas5p_integrators::retro_nonlinear::{Mode, Model, Term, Window, Work};
use serde_json::json;
fn maxdiff(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .fold(0.0_f64, |e, (&a, &b)| e.max((a - b).abs()))
}
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn enc(x: &[f64]) -> Vec<String> {
    x.iter().map(|&x| bits(x)).collect()
}
fn model(n: usize) -> (Model, DenseMatrix, Vec<f64>, Vec<Term>, Vec<f64>) {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = -1. - 0.125 * (i % 3) as f64;
        if i + 1 < n {
            a[(i, i + 1)] = 0.75;
            a[(i + 1, i)] = -0.75;
        }
    }
    let b = (0..n)
        .map(|i| ((i % 5) as f64 - 2.) / 16.)
        .collect::<Vec<_>>();
    let y = (0..n)
        .map(|i| ((i % 5) as f64 - 2.) / 4.)
        .collect::<Vec<_>>();
    let mut terms = Vec::new();
    for i in 0..n {
        let mut v = vec![0.; n];
        v[i] = 1.;
        v[(i + 1) % n] += 0.25;
        terms.push(Term {
            beta: 0.125,
            v,
            degree: [3, 5, 7][i % 3],
        });
    }
    (
        Model::new(a.clone(), b.clone(), terms.clone()).unwrap(),
        a,
        b,
        terms,
        y,
    )
}
#[test]
fn offline_table_product_projection_and_identity() {
    use std::collections::BTreeMap;
    let bytes = include_str!("../src/retro_jet_table.txt");
    assert!(JetTable::from_bytes(&(bytes.to_owned() + "\n"), 3, 6).is_err());
    assert!(JetTable::new(0, 3).is_err());
    assert!(JetTable::new(4, 3).is_err());
    assert!(JetTable::new(3, 7).is_err());
    for r in 1..=3 {
        for d in 1..=6 {
            let t = JetTable::new(r, d).unwrap();
            let a: Vec<_> = (0..t.channels())
                .map(|i| ((3 * i % 11) as f64 - 5.) / 8.)
                .collect();
            let b: Vec<_> = (0..t.channels())
                .map(|i| ((7 * i % 13) as f64 - 6.) / 16.)
                .collect();
            let c = t.multiply(&a, &b).unwrap();
            let mut refc = BTreeMap::new();
            for (i, x) in t.exponents().iter().enumerate() {
                for (j, y) in t.exponents().iter().enumerate() {
                    let z = std::array::from_fn::<usize, 3, _>(|k| x[k] + y[k]);
                    if z.iter().sum::<usize>() <= d {
                        *refc.entry(z).or_insert(0.) += a[i] * b[j];
                    }
                }
            }
            for (i, e) in t.exponents().iter().enumerate() {
                assert_eq!(c[i], refc[e]);
            }
        }
    }
}
#[test]
fn mixed_jet_lifting_and_diagonal_restriction() {
    let (m, a, b, terms, y) = model(3);
    let h = 1. / 16.;
    let steps = 4;
    let w = Window::new(&m, y.clone(), h, steps).unwrap();
    let center = w.forecast(&mut Work::default()).unwrap();
    for degree in [2, 4, 6] {
        let t = JetTable::new(3, degree).unwrap();
        let u = JetTable::new(1, degree).unwrap();
        let mut wa = Work::default();
        let mut wb = Work::default();
        let packet = w.lift(&t, &center, &mut wa).unwrap();
        let collapsed = w.lift(&u, &center, &mut wb).unwrap();
        let mut discrepancy = 0.0_f64;
        for d in 0..=degree {
            let mut sum = vec![0.; center.len()];
            for (e, c) in packet.exponents().iter().zip(packet.coefficients()) {
                if e.iter().sum::<usize>() == d {
                    for (x, y) in sum.iter_mut().zip(c) {
                        *x += y;
                    }
                }
            }
            discrepancy = discrepancy.max(maxdiff(&sum, &collapsed.coefficients()[d]));
        }
        assert!(discrepancy < 2e-15);
        let full = packet.evaluate(&[1., 1., 1.]).unwrap();
        let diag = collapsed.evaluate(&[1.]).unwrap();
        assert!(maxdiff(&full, &diag) < 2e-15);
        assert_eq!(wa.factorizations, steps);
        assert_eq!(wb.factorizations, steps);
        assert_eq!(wa.linear_solves, steps * (t.channels() - 1));
        assert_eq!(wb.linear_solves, steps * degree);
        let r = w.homotopy_residual(&center, &full, &[1., 1., 1.]).unwrap();
        let q = packet.evaluate(&[0.125, -0.25, 0.5]).unwrap();
        let off = w
            .homotopy_residual(&center, &q, &[0.125, -0.25, 0.5])
            .unwrap();
        assert!(off < r + 1e-13);
        println!(
            "DIAG {}",
            json!({"degree":degree,"channels":t.channels(),"collapsed_channels":u.channels(),"coefficient_discrepancy":discrepancy,"full_residual":r,"off_diagonal_residual":off,"multi_work":wa,"collapsed_work":wb})
        );
        if degree == 4 {
            println!(
                "JET_CASE {}",
                json!({"n":3,"steps":steps,"h":bits(h),"a":enc(a.as_slice()),"b":enc(&b),"y":enc(&y),
                "terms":terms.iter().map(|t|json!({"beta":bits(t.beta),"v":enc(&t.v),"degree":t.degree})).collect::<Vec<_>>(),
                "exponents":packet.exponents(),"coefficients":packet.coefficients().iter().map(|v|enc(v)).collect::<Vec<_>>()})
            );
        }
    }
}
#[test]
fn original_residual_not_formal_lift_controls_acceptance() {
    let a = DenseMatrix::from_rows(&[&[-1.]]).unwrap();
    let m = Model::new(
        a,
        vec![0.],
        vec![Term {
            beta: 10.,
            v: vec![1.],
            degree: 3,
        }],
    )
    .unwrap();
    let w = Window::new(&m, vec![1.], 0.5, 4).unwrap();
    let center = vec![1.; 4];
    let t = JetTable::new(2, 2).unwrap();
    let p = w.lift(&t, &center, &mut Work::default()).unwrap();
    let x = p.evaluate(&[1., 1.]).unwrap();
    let c = w.certify(&x, 1e-10).unwrap();
    assert!(!c.accepted);
    let s = w.solve(&t, Mode::Sequential, 1e-10, None).unwrap();
    assert!(s.certificate.accepted);
    let mut wrong = s.values.clone();
    wrong[2] += 0.125;
    let b = w.certify(&wrong, 1e-10).unwrap();
    assert!(!b.accepted);
    println!(
        "BAD {}",
        json!({"jet_error_bound":c.error_upper,"jet_residual":c.residual_upper,"wrong_error_bound":b.error_upper})
    );
    assert!(Model::new(DenseMatrix::identity(1), vec![0.], vec![]).is_err());
    assert!(
        Model::new(
            DenseMatrix::from_rows(&[&[-1.]]).unwrap(),
            vec![0.],
            vec![Term {
                beta: -1.,
                v: vec![1.],
                degree: 3
            }]
        )
        .is_err()
    );
    assert!(w.certify(&[f64::NAN; 4], 1e-10).is_err());
    assert!(w.certify(&s.values, 0.).is_err());
}
fn exact_scalar(k: f64, b: f64, y: f64, t: f64) -> f64 {
    y * (-k * t).exp() / (1. + b / k * y * y * (-(-2. * k * t).exp_m1())).sqrt()
}
#[test]
fn continuous_defect_and_discrete_solver_error_are_distinct() {
    for rate in [1., 12., 1000.] {
        let m = Model::new(
            DenseMatrix::from_rows(&[&[-rate]]).unwrap(),
            vec![0.],
            vec![Term {
                beta: 0.2,
                v: vec![1.],
                degree: 3,
            }],
        )
        .unwrap();
        let t = JetTable::new(1, 4).unwrap();
        for steps in [8, 32, 128] {
            let h = 0.5 / steps as f64;
            let w = Window::new(&m, vec![0.8], h, steps).unwrap();
            let r = w.solve(&t, Mode::Sequential, 1e-10, None).unwrap();
            assert!(r.certificate.accepted);
            let c = w.continuous_certificate(&r.values).unwrap();
            let mut e = 0.0_f64;
            for (i, &v) in r.values.iter().enumerate() {
                let err = (v - exact_scalar(rate, 0.2, 0.8, h * (i + 1) as f64)).abs();
                e = e.max(err);
                assert!(err <= c.node_upper[i] + 4e-15);
            }
            assert!(c.error_upper >= e && e > r.certificate.error_upper);
            println!(
                "IVP {}",
                json!({"rate":rate,"steps":steps,"discrete_bound":r.certificate.error_upper,"continuous_bound":c.error_upper,"observed_error":e})
            );
        }
    }
}
#[test]
fn all_window_paths_agree_on_coupled_polynomial_model() {
    let (m, a, b, terms, y) = model(3);
    let steps = 16;
    let h = 1. / 32.;
    let w = Window::new(&m, y.clone(), h, steps).unwrap();
    let t = JetTable::new(1, 4).unwrap();
    let multi = JetTable::new(3, 4).unwrap();
    let seq = w.solve(&t, Mode::Sequential, 1e-10, None).unwrap();
    assert!(seq.certificate.accepted);
    for (name, mode, table) in [
        ("sequential", Mode::Sequential, &t),
        ("chord", Mode::Chord, &t),
        ("collapsed", Mode::Jet, &t),
        ("multi", Mode::Jet, &multi),
    ] {
        let r = w.solve(table, mode, 1e-10, None).unwrap();
        assert!(r.certificate.accepted, "{name}: {:?}", r.certificate);
        assert!(
            maxdiff(&seq.values, &r.values)
                <= r.certificate.error_upper + seq.certificate.error_upper
        );
        let c = w.continuous_certificate(&r.values).unwrap();
        println!(
            "COUPLED {}",
            json!({"name":name,"n":3,"steps":steps,"h":bits(h),"a":enc(a.as_slice()),"b":enc(&b),"y":enc(&y),
            "terms":terms.iter().map(|t|json!({"beta":bits(t.beta),"v":enc(&t.v),"degree":t.degree})).collect::<Vec<_>>(),
            "x":enc(&r.values),"certificate":r.certificate,"continuous":c,"work":r.work})
        );
    }
}
fn median(samples: &[f64]) -> f64 {
    let mut x = samples.to_vec();
    x.sort_by(f64::total_cmp);
    x[x.len() / 2]
}
#[test]
#[ignore = "diagnostic owner-host work precision with full original discrete certificate"]
fn measure_window() {
    use std::{hint::black_box, time::Instant};
    faer::set_global_parallelism(faer::Par::Seq);
    for (n, steps) in [(3, 16), (8, 64), (16, 128)] {
        let (m, _, _, _, y) = model(n);
        let w = Window::new(&m, y, 0.5 / steps as f64, steps).unwrap();
        let t = JetTable::new(1, 4).unwrap();
        let multi = JetTable::new(3, 4).unwrap();
        let low = JetTable::new(1, 2).unwrap();
        let arms = [
            ("sequential", Mode::Sequential, &t),
            ("chord", Mode::Chord, &t),
            ("collapsed", Mode::Jet, &t),
            ("multi", Mode::Jet, &multi),
            ("degree2", Mode::Jet, &low),
        ];
        let seq = w.solve(&t, Mode::Sequential, 1e-10, None).unwrap();
        assert!(seq.certificate.accepted);
        let mut times: [Vec<f64>; 5] = std::array::from_fn(|_| Vec::new());
        let mut meta: [serde_json::Value; 5] = std::array::from_fn(|_| serde_json::Value::Null);
        for rep in 0..6 {
            for off in 0..5 {
                let i = (rep + off) % 5;
                let (_, mode, tab) = arms[i];
                let start = Instant::now();
                let r = w.solve(tab, mode, 1e-10, None).unwrap();
                black_box(&r.values);
                let ms = start.elapsed().as_secs_f64() * 1000.;
                assert!(
                    r.certificate.accepted,
                    "n={n} mode={mode:?}: {:?}",
                    r.certificate
                );
                let err = maxdiff(&seq.values, &r.values);
                assert!(err <= r.certificate.error_upper + seq.certificate.error_upper);
                if rep > 0 {
                    times[i].push(ms);
                }
                meta[i] = json!({"work":r.work,"bound":r.certificate.error_upper,"difference":err});
            }
        }
        for i in 0..5 {
            println!(
                "WP {}",
                json!({"n":n,"steps":steps,"arm":arms[i].0,"tol":1e-10,"median_ms":median(&times[i]),"samples_ms":times[i],"metrics":meta[i]})
            );
        }
    }
}
#[test]
#[ignore = "actual scalar IVP and native RODAS common mesh diagnostic"]
fn measure_native() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
    };
    use std::{hint::black_box, sync::Arc, time::Instant};
    faer::set_global_parallelism(faer::Par::Seq);
    let t = JetTable::new(1, 4).unwrap();
    for rate in [1., 12., 1000.] {
        let m = Model::new(
            DenseMatrix::from_rows(&[&[-rate]]).unwrap(),
            vec![0.],
            vec![Term {
                beta: 0.2,
                v: vec![1.],
                degree: 3,
            }],
        )
        .unwrap();
        let model = Arc::new(m.clone());
        let jac = model.clone();
        let problem = OdeProblem::new(
            "retro-nonlinear-scalar",
            1,
            Arc::new(move |_, y, out| {
                out.copy_from_slice(&model.rhs(y)?);
                Ok(())
            }),
            None,
            Some(Arc::new(move |_, y| jac.jacobian(y))),
            None,
            None,
            true,
            None,
            None,
        )
        .unwrap();
        for steps in [8, 32, 128] {
            let h = 0.5 / steps as f64;
            let w = Window::new(&m, vec![0.8], h, steps).unwrap();
            let times: Vec<_> = (0..=steps).map(|i| h * i as f64).collect();
            let output = OutputSchedule::new(times.clone()).unwrap();
            let reference: Vec<_> = times
                .iter()
                .map(|&x| exact_scalar(rate, 0.2, 0.8, x))
                .collect();
            let seq = w.solve(&t, Mode::Sequential, 1e-10, None).unwrap();
            let target = maxdiff(&seq.values, &reference[1..]);
            let mut timings: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
            let mut errors = [0.; 3];
            let mut accepted = [true; 3];
            // Choose a native tolerance on the basis of the observed midpoint
            // error for this diagnostic only; no exact achieved-error matching.
            let native_tol = (target * 0.05).clamp(1e-12, 1e-3);
            for rep in 0..6 {
                for off in 0..3 {
                    let arm = (rep + off) % 3;
                    let start = Instant::now();
                    let values = if arm < 2 {
                        let r = w
                            .solve(
                                &t,
                                if arm == 0 {
                                    Mode::Sequential
                                } else {
                                    Mode::Jet
                                },
                                1e-10,
                                None,
                            )
                            .unwrap();
                        accepted[arm] = r.certificate.accepted;
                        let mut values = vec![0.8];
                        values.extend(r.values);
                        values
                    } else {
                        let cfg = AdaptiveStepConfig {
                            atol: native_tol,
                            rtol: native_tol,
                            initial_step: h.min(1. / rate),
                            min_step: 1e-14,
                            max_step: 0.125,
                            max_attempts: 100000,
                            ..AdaptiveStepConfig::default()
                        };
                        let r = integrate_rodas5p_fast_observed(
                            &problem,
                            (0., 0.5),
                            &[0.8],
                            &cfg,
                            &output,
                        )
                        .unwrap();
                        assert!(r.observed.success);
                        assert_eq!(r.observed.t, times);
                        r.observed.y.iter().map(|v| v[0]).collect()
                    };
                    black_box(&values);
                    let elapsed = start.elapsed().as_secs_f64() * 1000.;
                    assert_eq!(values.len(), reference.len());
                    errors[arm] = maxdiff(&values, &reference);
                    if rep > 0 {
                        timings[arm].push(elapsed);
                    }
                }
            }
            for i in 0..3 {
                println!(
                    "NATIVE {}",
                    json!({"rate":rate,"steps":steps,"arm":(["midpoint-sequential","midpoint-jet","rodas5p-fast"][i]),"native_tol":native_tol,
                "accepted_discrete":if i==2{None}else{Some(accepted[i])},"observed_error":errors[i],"median_ms":median(&timings[i]),"samples_ms":timings[i]})
                );
            }
        }
    }
}

#[test]
fn one_stage_rk_stiff_limit_and_jet_independence() {
    let t = JetTable::new(1, 4).unwrap();
    let multi = JetTable::new(2, 4).unwrap();
    for rate in [1., 12., 1000.] {
        let a = DenseMatrix::from_rows(&[&[-rate]]).unwrap();
        let term = Term {
            beta: 0.2,
            v: vec![1.],
            degree: 3,
        };
        let model = Model::new(a.clone(), vec![0.], vec![term.clone()]).unwrap();
        assert!(Window::with_theta(&model, vec![0.8], 0.125, 4, 0.499).is_err());
        for steps in [8, 32, 128] {
            let h = 0.5 / steps as f64;
            let mut errors = [0.; 3];
            for (k, theta) in [0.5, 0.75, 1.].into_iter().enumerate() {
                let w = Window::with_theta(&model, vec![0.8], h, steps, theta).unwrap();
                let seq = w.solve(&t, Mode::Sequential, 1e-10, None).unwrap();
                let jet = w.solve(&multi, Mode::Jet, 1e-10, None).unwrap();
                assert!(seq.certificate.accepted && jet.certificate.accepted);
                assert!(
                    maxdiff(&seq.values, &jet.values)
                        <= seq.certificate.error_upper + jet.certificate.error_upper
                );
                let cont = w.continuous_certificate(&jet.values).unwrap();
                let mut err = 0.0_f64;
                for (i, &v) in jet.values.iter().enumerate() {
                    let e = (v - exact_scalar(rate, 0.2, 0.8, h * (i + 1) as f64)).abs();
                    err = err.max(e);
                    assert!(e <= cont.node_upper[i] + 4e-15);
                }
                errors[k] = err;
                println!(
                    "THETA {}",
                    json!({"rate":rate,"steps":steps,"theta":theta,"observed_error":err,
                    "discrete_bound":jet.certificate.error_upper,"continuous_bound":cont.error_upper,"work":jet.work})
                );
                if rate == 1000. && steps == 8 {
                    println!(
                        "THETA_CASE {}",
                        json!({"name":format!("theta-{theta}"),"theta":bits(theta),"n":1,"steps":steps,
                        "h":bits(h),"a":enc(a.as_slice()),"b":enc(&[0.]),"y":enc(&[0.8]),
                        "terms":[{"beta":bits(term.beta),"v":enc(&term.v),"degree":term.degree}],
                        "x":enc(&jet.values),"certificate":jet.certificate,"continuous":cont})
                    );
                }
            }
            if rate == 1000. {
                assert!(errors[2] < errors[0]);
            }
        }
    }
}
