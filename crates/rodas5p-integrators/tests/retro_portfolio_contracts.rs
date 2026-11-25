use rodas5p_integrators::retro_portfolio::Portfolio;
#[test]
fn bounded_adaptive_portfolio_constructs() {
    assert!(Portfolio::new(2, 4).is_ok());
    assert!(Portfolio::new(0, 4).is_err());
}

use rodas5p_core::{CoreError, DenseMatrix};
use rodas5p_integrators::{
    ParallelExecution, fourier_path_certificate as fc, retro_causal as ac, retro_green as green,
    retro_nonlinear as nl,
    retro_portfolio::{AttemptStatus, Route},
    retro_temporal,
};
use serde_json::json;
use std::collections::BTreeSet;
fn print_receipt<T>(
    family: &str,
    call: usize,
    r: &rodas5p_integrators::retro_portfolio::Selection<T>,
) {
    println!(
        "RECEIPT {}",
        json!({"family":family,"call":call,"selected":r.selected,"attempts":r.attempts,"total_ns":r.total_elapsed_ns()})
    );
}
fn diff(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .fold(0.0_f64, |x, (&a, &b)| x.max((a - b).abs()))
}
#[test]
fn exploration_recovery_no_cached_answer_and_contract_isolation() {
    let mut p = Portfolio::new(1, 3).unwrap();
    let routes = [Route::Jet2, Route::Jet4, Route::Jet6];
    let mut selected = BTreeSet::new();
    for i in 0..6 {
        let r = p
            .choose("test-a", &[3], &routes, 1e-6, |r| {
                if r == Route::Jet2 && i == 0 {
                    Ok((i, 1.))
                } else {
                    Ok((i, 1e-8))
                }
            })
            .unwrap();
        assert_eq!(r.value, Some(i));
        if i == 0 {
            assert_eq!(r.attempts.len(), 2);
            assert_eq!(r.attempts[0].status, AttemptStatus::Rejected);
        }
        selected.insert(r.selected.unwrap());
        print_receipt("selector", i, &r);
    }
    assert_eq!(selected.len(), 3);
    let r = p
        .choose("test-b", &[3], &routes, 1e-6, |_| Ok((7, 0.)))
        .unwrap();
    assert_eq!(r.selected, Some(Route::Jet2));
    let r = p
        .choose("test-a", &[3], &routes, 1e-9, |_| Ok((8, 0.)))
        .unwrap();
    assert_eq!(r.selected, Some(Route::Jet2));
    let mut p = Portfolio::new(2, 2).unwrap();
    let r = p
        .choose(
            "negative",
            &[],
            &routes,
            1e-6,
            |r| -> Result<(usize, f64), CoreError> {
                if r == Route::Jet2 {
                    Ok((1, f64::NAN))
                } else {
                    Err(CoreError::InvalidInput("deliberate refusal".into()))
                }
            },
        )
        .unwrap();
    assert!(r.value.is_none() && r.selected.is_none());
    assert_eq!(r.attempts.len(), 2);
    assert!(
        p.choose("bad", &[], &[Route::Jet2, Route::Jet2], 1., |_| Ok((1, 0.)))
            .is_err()
    );
}
#[test]
fn cubic_all_routes_confluent_and_deformed() {
    let w = DenseMatrix::from_rows(&[&[4., 1.], &[0., 3.]]).unwrap();
    let d = DenseMatrix::from_rows(&[&[0., 0.], &[1., 0.]]).unwrap();
    let b = [vec![1., 0.5], vec![-0.25, 0.125], vec![0.75, -0.5]];
    let mut p = Portfolio::new(1, 4).unwrap();
    for modulus in [[0.; 3], [0., -0.25, 0.], [0.125, -0.25, 0.25]] {
        let mut first: Option<[Vec<f64>; 3]> = None;
        let mut seen = BTreeSet::new();
        for i in 0..4 {
            let r = p.cubic(&w, &d, modulus, &b, 1e-10).unwrap();
            let x = r.value.as_ref().unwrap();
            if let Some(y) = &first {
                for j in 0..3 {
                    assert!(diff(&x[j], &y[j]) <= 2e-10);
                }
            } else {
                first = Some(x.clone());
            }
            seen.insert(r.selected.unwrap());
            print_receipt("cubic", i, &r);
        }
        assert_eq!(seen.len(), 2);
    }
}
#[test]
fn temporal_all_routes_original_node_output() {
    let g = [
        DenseMatrix::from_rows(&[&[0.75, 0.125], &[0., 0.5]]).unwrap(),
        DenseMatrix::from_rows(&[&[0.5, 0.], &[-0.25, 0.75]]).unwrap(),
    ];
    let y = [0.25, 0.125];
    let b: Vec<_> = (0..32)
        .map(|i| vec![(i % 7) as f64 / 32., -(i % 5) as f64 / 16.])
        .collect();
    let reference = retro_temporal::solve_sequential(&g, &y, &b, 1e-10).unwrap();
    let mut p = Portfolio::new(1, 6).unwrap();
    let exec = ParallelExecution::rayon(2).unwrap();
    let mut seen = BTreeSet::new();
    for i in 0..8 {
        let r = p.temporal(&g, &y, &b, 1., &exec, 1e-10).unwrap();
        let v = r.value.as_ref().unwrap();
        assert_eq!(v.values.len(), b.len());
        assert!(v.accepted);
        for (a, b) in v.values.iter().zip(&reference.values) {
            assert!(diff(a, b) <= v.error_upper + reference.error_upper);
        }
        seen.insert(r.selected.unwrap());
        print_receipt("temporal", i, &r);
    }
    assert_eq!(seen.len(), 4);
    // Non-power-of-two periods retain the sequential path without pretending FFT admission.
    let r = p.temporal(&g, &y, &b[..30], 1., &exec, 1e-10).unwrap();
    assert_eq!(r.selected, Some(Route::TemporalSequential));
    assert_eq!(r.value.unwrap().values.len(), 30);
}
#[test]
fn fourier_all_paths_retain_original_certificate() {
    let mut p = Portfolio::new(1, 4).unwrap();
    for omega in [40., 10000.] {
        let m = fc::FourierModel::new(omega, 1).unwrap();
        let mut seen = BTreeSet::new();
        for i in 0..4 {
            let r = p.fourier(&m, 0.5, 1e-8, 0.125).unwrap();
            let v = r.value.as_ref().unwrap();
            assert_eq!(v.0.last().unwrap().state.t(), 0.5);
            assert!(v.0.iter().all(|s| s.physical_error <= 1e-8));
            seen.insert(r.selected.unwrap());
            print_receipt("fourier", i, &r);
        }
        assert_eq!(seen.len(), 4);
    }
}
fn scalar(rate: f64, beta: f64) -> nl::Model {
    nl::Model::new(
        DenseMatrix::from_rows(&[&[-rate]]).unwrap(),
        vec![0.],
        vec![nl::Term {
            beta,
            v: vec![1.],
            degree: 3,
        }],
    )
    .unwrap()
}
fn exact(rate: f64, t: f64) -> f64 {
    0.8 * (-rate * t).exp() / (1. + 0.2 / rate * 0.64 * (-(-2. * rate * t).exp_m1())).sqrt()
}
#[test]
fn nonlinear_discrete_and_physical_contracts_not_interchangeable() {
    let m = scalar(1., 0.2);
    let w = nl::Window::new(&m, vec![0.8], 1. / 32., 16).unwrap();
    let mut p = Portfolio::new(1, 8).unwrap();
    let mut seen = BTreeSet::new();
    for i in 0..6 {
        let r = p.nonlinear(&w, 1e-10, None).unwrap();
        let v = r.value.as_ref().unwrap();
        assert!(v.certificate.accepted);
        seen.insert(r.selected.unwrap());
        print_receipt("nonlinear", i, &r);
    }
    assert_eq!(seen.len(), 6);
    let mut seen = BTreeSet::new();
    for i in 0..8 {
        let r = p.physical_window(&w, 1e-3).unwrap();
        let v = r.value.as_ref().unwrap();
        let err = v
            .values
            .iter()
            .enumerate()
            .map(|(i, &v)| (v - exact(1., (i + 1) as f64 / 32.)).abs())
            .fold(0., f64::max);
        assert!(err <= v.error_upper + 3e-15 && v.error_upper <= 1e-3);
        seen.insert(r.selected.unwrap());
        print_receipt("physical", i, &r);
    }
    assert!(seen.contains(&Route::Green) && seen.contains(&Route::NativeRodas));
    let stiff = scalar(1000., 0.2);
    let w = nl::Window::new(&stiff, vec![0.8], 1. / 16., 8).unwrap();
    let mut p = Portfolio::new(8, 1).unwrap();
    let r = p.physical_window(&w, 1e-6).unwrap();
    assert!(r.value.is_none());
    assert_eq!(r.attempts[0].status, AttemptStatus::Rejected);
    print_receipt("stiff-coarse-reject", 0, &r);
    let mut p = Portfolio::new(8, 2).unwrap();
    let r = p.physical_window(&w, 1e-5).unwrap();
    print_receipt("stiff-green-recovery", 0, &r);
    let v = r.value.as_ref().unwrap();
    assert_eq!(r.selected, Some(Route::Green));
    assert!(v.error_upper <= 1e-5);
    let err = v
        .values
        .iter()
        .enumerate()
        .map(|(i, &v)| (v - exact(1000., (i + 1) as f64 / 16.)).abs())
        .fold(0., f64::max);
    assert!(err <= v.error_upper + 3e-15);
}
#[test]
fn affine_forecast_and_green_all_modes() {
    let table = green::RetroTable::new().unwrap();
    let specs = [
        ac::SlabSpec {
            end: 0.25,
            rate: 4.,
            forcing: 1.,
            forecast: 4.,
            radius: 0.,
        },
        ac::SlabSpec {
            end: 0.5,
            rate: 80.,
            forcing: 8.,
            forecast: 60.,
            radius: 20.,
        },
        ac::SlabSpec {
            end: 0.75,
            rate: 0.25,
            forcing: 0.25,
            forecast: 4.,
            radius: 3.75,
        },
    ];
    let mut p = Portfolio::new(1, 4).unwrap();
    for dense in [false, true] {
        let mut targets = vec![ac::Target {
            slab: 2,
            kind: ac::OutputKind::Endpoint,
            tolerance: 1e-6,
        }];
        if dense {
            targets.extend((0..3).map(|slab| ac::Target {
                slab,
                kind: ac::OutputKind::Dense,
                tolerance: 1e-6,
            }));
        }
        let mut seen = BTreeSet::new();
        for i in 0..3 {
            let r = p.affine(&table, &specs, &targets, 1e-2, 0.2).unwrap();
            let v = r.value.as_ref().unwrap();
            assert!(v.slabs.last().unwrap().endpoint_upper() <= 1e-6);
            if dense {
                assert!(v.slabs.iter().all(|s| s.dense_upper().unwrap() <= 1e-6));
            }
            seen.insert(r.selected.unwrap());
            print_receipt("affine", i, &r);
        }
        assert_eq!(seen.len(), 3);
    }
    let mut wrong = specs;
    wrong[2].radius = 0.;
    let target = [ac::Target {
        slab: 2,
        kind: ac::OutputKind::Endpoint,
        tolerance: 1e-6,
    }];
    let r = p.affine(&table, &wrong, &target, 1e-2, 0.2).unwrap();
    assert!(r.value.as_ref().unwrap().plan.forecast_fallback);
    print_receipt("affine-wrong", 0, &r);
    let m = green::Model::new(vec![1.], vec![0.], vec![0.], vec![0.2]).unwrap();
    let mut seen = BTreeSet::new();
    for i in 0..3 {
        let r = p
            .green_endpoint(&table, &m, &[0.8], 0.5, 0.125, 1e-5)
            .unwrap();
        let v = r.value.as_ref().unwrap();
        assert!((v.endpoint()[0] - exact(1., 0.5)).abs() <= v.error_upper() + 3e-15);
        seen.insert(r.selected.unwrap());
        print_receipt("green", i, &r);
    }
    assert_eq!(seen.len(), 3);
}

#[test]
#[ignore = "owner-host adaptive selection cost, including exploration and retries"]
fn measure_policy() {
    use std::{collections::BTreeMap, hint::black_box, time::Instant};
    faer::set_global_parallelism(faer::Par::Seq);
    let exec = ParallelExecution::rayon(4).unwrap();
    let n = 8;
    let mut a = DenseMatrix::zeros(n, n);
    let mut b = a.clone();
    for i in 0..n {
        a[(i, i)] = 0.75;
        b[(i, i)] = 0.5;
        a[(i, (i + 1) % n)] = 0.125;
        b[(i, (i + n - 1) % n)] = -0.125;
    }
    let pattern = [a, b];
    let y = vec![0.25; n];
    let nr = 24;
    let rhos: Vec<Vec<Vec<f64>>> = (0..nr)
        .map(|j| {
            (0..128)
                .map(|i| {
                    (0..n)
                        .map(|k| (((i * 7 + k * 3 + j * 5) % 17) as f64 - 8.) / 32.)
                        .collect()
                })
                .collect()
        })
        .collect();
    let table = green::RetroTable::new().unwrap();
    let rate = [4., 4., 4., 4., 80., 80., 0.25, 0.25, 0.25, 0.25, 0.25, 0.25];
    let eq = [
        1., -0.25, 0.75, 0.25, -0.5, 0.6, 0.2, 1., -0.4, 0.5, 0.7, 0.1,
    ];
    let slabs: Vec<_> = rate
        .into_iter()
        .zip(eq)
        .enumerate()
        .map(|(i, (rate, q))| {
            let (forecast, radius) = if i == 4 || i == 5 {
                (60., 20.)
            } else if i >= 6 {
                (4., 3.75)
            } else {
                (4., 0.)
            };
            ac::SlabSpec {
                end: (i + 1) as f64 / 4.,
                rate,
                forcing: rate * q,
                forecast,
                radius,
            }
        })
        .collect();
    let mut targets = vec![ac::Target {
        slab: 11,
        kind: ac::OutputKind::Endpoint,
        tolerance: 1e-7,
    }];
    targets.extend((0..12).map(|slab| ac::Target {
        slab,
        kind: ac::OutputKind::Dense,
        tolerance: 1e-7,
    }));
    // A fixed cold route vs exploration+exploitation vs continuous exploration.
    // All arms include setup, allocation, original certification and failed work.
    let arms = [
        ("cold-fixed", usize::MAX),
        ("adaptive", 4),
        ("round-robin", 1),
    ];
    for family in ["temporal", "affine-dense"] {
        for rep in 0..6 {
            for off in 0..3 {
                let j = (rep + off) % 3;
                let mut p = Portfolio::new(arms[j].1, 6).unwrap();
                let mut elapsed = 0.;
                let mut late = 0.;
                let mut selected = BTreeMap::<Route, usize>::new();
                let mut failed = 0;
                let mut maxbound = 0.0_f64;
                let mut observed = 0.0_f64;
                for (i, rhs) in rhos.iter().enumerate() {
                    let start = Instant::now();
                    if family == "temporal" {
                        let r = p.temporal(&pattern, &y, rhs, 1., &exec, 1e-10).unwrap();
                        black_box(&r.value);
                        let ms = start.elapsed().as_secs_f64() * 1000.;
                        elapsed += ms;
                        if i >= 16 {
                            late += ms;
                        }
                        let v = r.value.unwrap();
                        assert!(v.accepted);
                        let refv =
                            retro_temporal::solve_sequential(&pattern, &y, rhs, 1e-10).unwrap();
                        let e = v
                            .values
                            .iter()
                            .zip(&refv.values)
                            .map(|(a, b)| diff(a, b))
                            .fold(0., f64::max);
                        assert!(e <= v.error_upper + refv.error_upper);
                        observed = observed.max(e);
                        maxbound = maxbound.max(v.error_upper);
                        *selected.entry(r.selected.unwrap()).or_default() += 1;
                        failed += r.attempts.len() - 1;
                    } else {
                        let r = p.affine(&table, &slabs, &targets, 1e-2, 0.2).unwrap();
                        black_box(&r.value);
                        let ms = start.elapsed().as_secs_f64() * 1000.;
                        elapsed += ms;
                        if i >= 16 {
                            late += ms;
                        }
                        let v = r.value.unwrap();
                        let mut exact = 0.2;
                        let mut t0 = 0.;
                        for (s, seg) in slabs.iter().zip(&v.slabs) {
                            for q in 0..=16 {
                                let t = t0 + (s.end - t0) * q as f64 / 16.;
                                let x = s.rate * (t - t0);
                                let refv = (-x).exp() * exact - s.forcing / s.rate * (-x).exp_m1();
                                let (a, b) = seg.query(&table, t).unwrap();
                                let e = (a - refv).abs();
                                assert!(e <= b + 4e-15);
                                observed = observed.max(e);
                            }
                            let x = s.rate * (s.end - t0);
                            exact = (-x).exp() * exact - s.forcing / s.rate * (-x).exp_m1();
                            t0 = s.end;
                            let e = seg.dense_upper().unwrap();
                            assert!(e <= 1e-7);
                            maxbound = maxbound.max(e);
                        }
                        *selected.entry(r.selected.unwrap()).or_default() += 1;
                        failed += r.attempts.len() - 1;
                    }
                }
                assert_eq!(selected.values().sum::<usize>(), nr);
                if rep > 0 {
                    println!(
                        "POLICY {}",
                        json!({"family":family,"arm":arms[j].0,"rep":rep,"calls":nr,"total_ms":elapsed,"late_ms":late,"selected":selected,"failed_attempts":failed,"bound":maxbound,"observed_error":observed})
                    );
                }
            }
        }
    }
}
