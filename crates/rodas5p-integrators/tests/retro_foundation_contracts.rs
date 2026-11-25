#![cfg(feature = "retro-dictionary")]
use rodas5p_core::{
    DenseMatrix,
    directed::Interval,
    retro_dictionary::{Dictionary, RealMode},
};
use rodas5p_integrators::{
    ParallelExecution,
    retro_foundation::*,
    retro_green::{Model, RetroTable, integrate},
    retro_portfolio::{AttemptStatus, Portfolio, Route},
};
use serde_json::json;
use std::{collections::BTreeSet, hint::black_box, time::Instant};
fn mat(n: usize, v: Vec<f64>) -> DenseMatrix {
    DenseMatrix::new(n, n, v).unwrap()
}
fn pt(x: f64) -> Interval {
    Interval::point(x).unwrap()
}
fn scalar(k: f64, beta: f64, y: f64, t: f64) -> f64 {
    if k == 0. {
        y / (1. + 2. * beta * y * y * t).sqrt()
    } else {
        y * (-k * t).exp() / (1. + beta / k * y * y * (-(-2. * k * t).exp_m1())).sqrt()
    }
}
#[test]
fn green_full_table_reaches_original_solver() {
    for k in [1., 64., 1000.] {
        let m = Model::new(vec![k], vec![0.], vec![0.], vec![0.2]).unwrap();
        for mode in [
            RealMode::TableFast,
            RealMode::TableDirected,
            RealMode::Hybrid,
        ] {
            let t = RetroTable::with_dictionary(mode).unwrap();
            assert_eq!(t.dictionary_mode(), Some(mode));
            let r = integrate(&t, &m, &[0.8], 0.125, 0.03125, 1e-5).unwrap();
            let e = (r.endpoint()[0] - scalar(k, 0.2, 0.8, 0.125)).abs();
            assert!(e <= r.error_upper() + 2e-15 && r.error_upper() <= 1e-5);
            println!(
                "GREEN_FULL {}",
                json!({"k":k,"mode":format!("{mode:?}"),"error":e,"bound":r.error_upper(),"work":r.work()})
            );
        }
    }
    let m = Model::new(vec![1.], vec![0.], vec![0.], vec![0.2]).unwrap();
    let mut p = Portfolio::new(1, 6).unwrap();
    let mut seen = BTreeSet::new();
    for _ in 0..6 {
        let r = p
            .green_dictionary_endpoint(&m, &[0.8], 0.125, 0.03125, 1e-5)
            .unwrap();
        assert!(r.value.is_some());
        seen.insert(r.selected.unwrap());
    }
    assert_eq!(seen.len(), 6);
}
fn fixture(n: usize, k: usize) -> (DenseMatrix, Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 2.;
        if i + 1 < n {
            a[(i, i + 1)] = 0.125;
        }
        if i > 0 {
            a[(i, i - 1)] = -0.25;
        }
    }
    let exact = (0..k)
        .map(|j| {
            (0..n)
                .map(|i| (((i + 3 * j) % 17) as f64 - 8.) / 16.)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let b = exact.iter().map(|v| a.matvec(v).unwrap()).collect();
    (a, b, exact)
}
#[test]
fn prepared_rhs_modes_and_rejection() {
    let (a, b, x) = fixture(17, 19);
    let mut w = PreparedRhs::new(a.clone()).unwrap();
    let exec = ParallelExecution::rayon(4).unwrap();
    let mut changed = a.clone();
    changed[(0, 0)] = 3.;
    assert!(w.matches(&a) && !w.matches(&changed));
    assert!(PreparedRhs::new(mat(2, vec![1., 2., 3., 1.])).is_err());
    assert!(
        w.solve(&[vec![f64::NAN; 17]], BatchMode::SerialReuse, &exec)
            .is_err()
    );
    for mode in [
        BatchMode::SerialReuse,
        BatchMode::FaerBatch,
        BatchMode::RayonChunks,
    ] {
        let r = w.solve(&b, mode, &exec).unwrap();
        let e = r
            .values
            .iter()
            .flatten()
            .zip(x.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(e <= r.error_upper && r.error_upper < 1e-12);
        println!(
            "BATCH {}",
            json!({"mode":format!("{mode:?}"),"error":e,"bound":r.error_upper,"setup_ns":r.setup_ns})
        );
    }
    let mut p = Portfolio::new(1, 3).unwrap();
    let mut seen = BTreeSet::new();
    for _ in 0..3 {
        seen.insert(
            w.adaptive(&mut p, &b, &exec, 1e-12)
                .unwrap()
                .selected
                .unwrap(),
        );
    }
    assert_eq!(
        seen,
        BTreeSet::from([
            Route::PreparedSerial,
            Route::PreparedFaerBatch,
            Route::PreparedRayon
        ])
    );
}
fn time_fixture(n: usize, m: usize) -> (Vec<DenseMatrix>, Vec<Vec<f64>>, Vec<f64>) {
    let g = (0..m)
        .map(|k| {
            let mut a = DenseMatrix::zeros(n, n);
            for i in 0..n {
                a[(i, i)] = 0.5 + ((k + 2 * i) % 5) as f64 / 64.;
                a[(i, (i + 1 + k % 3) % n)] += 0.0625;
            }
            a
        })
        .collect();
    let b = (0..m)
        .map(|k| {
            (0..n)
                .map(|i| (((i + 2 * k) % 13) as f64 - 6.) / 32.)
                .collect()
        })
        .collect();
    let y = (0..n).map(|i| i as f64 / 16.).collect();
    (g, b, y)
}
#[test]
fn arbitrary_order_noncommuting_scan() {
    let (g, b, y) = time_fixture(3, 37);
    assert_ne!(g[0].matmul(&g[1]).unwrap(), g[1].matmul(&g[0]).unwrap());
    let p = Aperiodic::new(g.clone()).unwrap();
    let ex = ParallelExecution::rayon(4).unwrap();
    for mode in [TimeMode::Sequential, TimeMode::OrderedScan] {
        let r = p.solve(&y, &b, mode, &ex).unwrap();
        assert!(r.error_upper < 1e-12 && r.values.len() == 37);
        println!(
            "TIME {}",
            json!({"mode":format!("{mode:?}"),"g":g.iter().map(|a|a.as_slice()).collect::<Vec<_>>(),"b":b,"y":y,"x":r.values,"bounds":r.node_upper})
        );
        let mut wrong = r.values;
        wrong[3][1] += 0.25;
        assert!(p.certify(&y, &b, wrong).unwrap().error_upper > 0.1);
    }
    let mut selector = Portfolio::new(1, 2).unwrap();
    assert_eq!(
        p.adaptive(&mut selector, &y, &b, &ex, 1e-12)
            .unwrap()
            .selected,
        Some(Route::AperiodicSequential)
    );
    assert_eq!(
        p.adaptive(&mut selector, &y, &b, &ex, 1e-12)
            .unwrap()
            .selected,
        Some(Route::AperiodicScan)
    );
}
#[test]
fn ordered_picard_and_nilpotent_green_are_source_actions() {
    let a = [
        mat(3, vec![0., 0.25, 0., 0., 0., 0., 0., 0., 0.]),
        mat(3, vec![0., 0., 0., 0., 0., 0.5, 0., 0., 0.]),
        DenseMatrix::zeros(3, 3),
    ];
    let r = ordered_picard(&a, &[0., 0., 1.]).unwrap();
    assert_eq!(r.words, 363);
    assert!((r.values[0] - 1. / 48.).abs() < 1e-15 && (r.values[1] - 0.25).abs() < 1e-15);
    assert!(r.error_upper >= 0. && r.error_upper < 1e-4);
    println!(
        "PICARD {}",
        json!({"values":r.values,"bound":r.error_upper,"tail":r.tail_upper})
    );
    let e = mat(2, vec![0., 0.25, 0., 0.]);
    let y = [0.5, -0.25];
    let f = vec![vec![0.25, 0.5]; 3];
    for a in [0., 1., 128., 1024.] {
        for mode in [
            RealMode::TableFast,
            RealMode::TableDirected,
            RealMode::Hybrid,
        ] {
            let r = nilpotent_green(a, &e, 1., &y, &f, mode).unwrap();
            let v = if a == 0. {
                [0.75, 0.25]
            } else {
                let z = -a;
                let ex = z.exp();
                let p1 = -z.exp_m1() / a;
                let q = (1. - (1. + a) * ex) / (a * a);
                [
                    ex * (y[0] + 0.25 * y[1]) + p1 * 0.25 + q * 0.125,
                    ex * y[1] + p1 * 0.5,
                ]
            };
            for (z, v) in r.iter().zip(v) {
                assert!(z.lo - 2e-15 <= v && v <= z.hi + 2e-15);
            }
            println!(
                "NIL_GREEN {}",
                json!({"a":a,"mode":format!("{mode:?}"),"lo":r.iter().map(|x|x.lo).collect::<Vec<_>>(),"hi":r.iter().map(|x|x.hi).collect::<Vec<_>>()})
            );
        }
    }
    assert!(nilpotent_green(0., &DenseMatrix::identity(2), 1., &y, &f, RealMode::Hybrid).is_err());
}
#[test]
fn multiaxis_deformation_and_failed_folded_fallback() {
    let w = mat(2, vec![4., 0.125, 0., 3.]);
    let terms = [
        PencilTerm {
            powers: vec![1, 0],
            operator: mat(2, vec![0., 0., 0.125, 0.]),
        },
        PencilTerm {
            powers: vec![0, 1],
            operator: mat(2, vec![0., 0.25, 0., 0.]),
        },
        PencilTerm {
            powers: vec![4, 2],
            operator: mat(2, vec![0.125, 0., 0., -0.125]),
        },
    ];
    let b = (0..12)
        .map(|j| vec![j as f64 / 16., -(j as f64) / 32.])
        .collect::<Vec<_>>();
    for p in [
        vec![vec![0.; 4], vec![0.; 3]],
        vec![vec![0.125, -0.25, 0.125, 0.], vec![0.125, 0., 0.25]],
    ] {
        let mut s = TensorPencil::new(w.clone(), &p, &terms).unwrap();
        assert_eq!(s.channels(), 12);
        for mode in [QuotientMode::DirectBlock, QuotientMode::Folded] {
            let r = s.solve(&b, mode, 1e-11, 32).unwrap();
            assert!(r.error_upper <= 1e-11);
            println!(
                "QUOTIENT {}",
                json!({"mode":format!("{mode:?}"),"p":p,"w":w.as_slice(),"terms":terms.iter().map(|t|json!({"powers":t.powers,"operator":t.operator.as_slice()})).collect::<Vec<_>>(),"b":b,"x":r.values,"bound":r.error_upper,"iters":r.iterations})
            );
            let mut bad = r.values;
            bad[0][0] += 0.5;
            assert!(s.certify(&b, bad, 1).unwrap().error_upper > 0.1);
        }
        let mut pol = Portfolio::new(1, 2).unwrap();
        let r = s.adaptive(&mut pol, &b, 1e-11, 1).unwrap();
        assert_eq!(r.selected, Some(Route::QuotientDirect));
        assert_eq!(r.attempts[0].status, AttemptStatus::Rejected);
    }
    assert!(
        TensorPencil::new(
            DenseMatrix::identity(2),
            &[vec![0.; 2]],
            &[PencilTerm {
                powers: vec![1],
                operator: mat(2, vec![3., 0., 0., 3.])
            }]
        )
        .is_err()
    );
}
#[test]
fn table_parallel_outputs_identical() {
    let x = (0..257).map(|i| i as f64 * 64.).collect::<Vec<_>>();
    let s = ParallelExecution::sequential();
    let p = ParallelExecution::rayon(4).unwrap();
    let mut a = vec![[pt(0.); 9]; x.len()];
    let mut b = a.clone();
    phi_batch_into(&x, &mut a, &s).unwrap();
    phi_batch_into(&x, &mut b, &p).unwrap();
    for (a, b) in a.iter().flatten().zip(b.iter().flatten()) {
        assert_eq!(
            (a.lo.to_bits(), a.hi.to_bits()),
            (b.lo.to_bits(), b.hi.to_bits())
        );
    }
    let mut a = vec![[pt(0.); 36]; x.len()];
    let mut b = a.clone();
    moment_batch_into(&x, &mut a, &s).unwrap();
    moment_batch_into(&x, &mut b, &p).unwrap();
    for (a, b) in a.iter().flatten().zip(b.iter().flatten()) {
        assert_eq!(
            (a.lo.to_bits(), a.hi.to_bits()),
            (b.lo.to_bits(), b.hi.to_bits())
        );
    }
    assert!(phi_batch_into(&[0.], &mut [], &p).is_err());
}
fn median(mut ns: Vec<u128>) -> u128 {
    ns.sort_unstable();
    ns[ns.len() / 2]
}
#[test]
#[ignore = "explicit owner-host timing"]
fn measure_foundation() {
    let ex = ParallelExecution::rayon(4).unwrap();
    let seq = ParallelExecution::sequential();
    let d = Dictionary::get().unwrap();
    let x = (0..8192)
        .map(|i| ((i * 7919) % 163840) as f64 / 10.)
        .collect::<Vec<_>>();
    let mut a = vec![[pt(0.); 9]; x.len()];
    let mut b = vec![[pt(0.); 36]; x.len()];
    for k in [9, 36] {
        let mut times = [vec![], vec![], vec![]];
        for rep in 0..6 {
            for i in 0..3 {
                let route = (i + rep) % 3;
                let start = Instant::now();
                if k == 9 {
                    match route {
                        0 => {
                            for (x, a) in x.iter().zip(&mut a) {
                                for (j, z) in a.iter_mut().enumerate() {
                                    *z = d.phi(pt(*x), j, RealMode::TableFast).unwrap();
                                }
                            }
                        }
                        1 => phi_batch_into(&x, &mut a, &seq).unwrap(),
                        _ => phi_batch_into(&x, &mut a, &ex).unwrap(),
                    };
                    black_box(&a);
                } else {
                    match route {
                        0 => {
                            for (x, a) in x.iter().zip(&mut b) {
                                let mut j = 0;
                                for p in 0..8 {
                                    for l in 0..=p {
                                        a[j] = d.moment(pt(*x), p, l, RealMode::TableFast).unwrap();
                                        j += 1;
                                    }
                                }
                            }
                        }
                        1 => moment_batch_into(&x, &mut b, &seq).unwrap(),
                        _ => moment_batch_into(&x, &mut b, &ex).unwrap(),
                    };
                    black_box(&b);
                }
                if rep > 0 {
                    times[route].push(start.elapsed().as_nanos());
                }
            }
        }
        println!(
            "TIMING {}",
            json!({"kind":"table","lanes":k,"points":x.len(),"scalar_ns":median(times[0].clone()),"packed_ns":median(times[1].clone()),"rayon_ns":median(times[2].clone()),"samples":times})
        );
    }
    for n in [8, 32, 128] {
        for k in [1, 8, 64] {
            let (a, b, _) = fixture(n, k);
            let mut w = PreparedRhs::new(a).unwrap();
            let mut times = [vec![], vec![], vec![], vec![]];
            let mut bounds = [0.0_f64; 4];
            for rep in 0..6 {
                for i in 0..4 {
                    let j = (i + rep) % 4;
                    let start = Instant::now();
                    let r = w
                        .solve(
                            &b,
                            [
                                BatchMode::SerialReuse,
                                BatchMode::FaerBatch,
                                BatchMode::RayonChunks,
                                BatchMode::FreshFactor,
                            ][j],
                            if j == 2 { &ex } else { &seq },
                        )
                        .unwrap();
                    let ns = start.elapsed().as_nanos();
                    bounds[j] = bounds[j].max(r.error_upper);
                    black_box(r);
                    if rep > 0 {
                        times[j].push(ns);
                    }
                }
            }
            assert!(bounds.iter().all(|v| *v < 1e-12));
            println!(
                "TIMING {}",
                json!({"kind":"prepared","n":n,"rhs":k,"setup_ns":w.setup_ns(),"serial_ns":median(times[0].clone()),"faer_ns":median(times[1].clone()),"rayon_ns":median(times[2].clone()),"fresh_ns":median(times[3].clone()),"bounds":bounds,"samples":times})
            );
        }
    }
    for n in [2, 8, 32] {
        let (g, b, y) = time_fixture(n, 256);
        let a = Aperiodic::new(g).unwrap();
        let mut times = [vec![], vec![]];
        for rep in 0..6 {
            for i in 0..2 {
                let j = (i + rep) % 2;
                let start = Instant::now();
                let r = a
                    .solve(
                        &y,
                        &b,
                        [TimeMode::Sequential, TimeMode::OrderedScan][j],
                        &ex,
                    )
                    .unwrap();
                assert!(r.error_upper < 1e-12);
                black_box(r);
                if rep > 0 {
                    times[j].push(start.elapsed().as_nanos());
                }
            }
        }
        println!(
            "TIMING {}",
            json!({"kind":"aperiodic","n":n,"steps":256,"serial_ns":median(times[0].clone()),"scan_ns":median(times[1].clone()),"samples":times})
        );
    }
}

#[test]
fn preflight_batch_invalid_input_leaves_buffers_unchanged() {
    let x = [0.5, 1., -1.];
    for execution in [
        ParallelExecution::sequential(),
        ParallelExecution::rayon(4).unwrap(),
    ] {
        let sentinel = Interval::point(123.).unwrap();
        let mut phi = [[sentinel; 9]; 3];
        let mut moments = [[sentinel; 36]; 3];
        assert!(phi_batch_into(&x, &mut phi, &execution).is_err());
        assert!(moment_batch_into(&x, &mut moments, &execution).is_err());
        assert_eq!(phi, [[sentinel; 9]; 3]);
        assert_eq!(moments, [[sentinel; 36]; 3]);
    }
}
