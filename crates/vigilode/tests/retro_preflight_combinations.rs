//! Bounded preparation fixtures, not M0 reference adjudication.
#![cfg(feature = "retro-dictionary")]
use rodas5p_core::DenseMatrix;
use rodas5p_core::retro_dictionary::RealMode;
use serde_json::json;
use vigilode::retro_research::{
    ParallelExecution,
    retro_foundation::*,
    retro_green::{self, Model, RetroTable},
};
use vigilode::{Atol, Direction, Jacobian, Options, Problem, Rodas5p};
fn cell(v: serde_json::Value) {
    println!("CELL {v}");
}
fn opts(rtol: f64, dense: bool) -> Options {
    Options {
        rtol,
        atol: Atol::Scalar(rtol * 0.01),
        dense_output: dense,
        ..Options::default()
    }
}
#[test]
fn event_combinations() {
    for rtol in [1e-6, 1e-8] {
        for terminal in [false, true] {
            for analytic in [false, true] {
                let mut p = Problem::new(1, |_, y, f| {
                    f[0] = -y[0];
                    Ok(())
                })
                .autonomous();
                if analytic {
                    p = p.jacobian(Jacobian::dense(|_, _, j| {
                        j[0] = -1.;
                        Ok(())
                    }));
                }
                p = if terminal {
                    p.terminal_event(|_, y| y[0] - 0.5, Direction::Down)
                } else {
                    p.event(|_, y| y[0] - 0.5, Direction::Down)
                };
                let s = Rodas5p::solve(&p, (0., 2.), &[1.], &opts(rtol, true)).unwrap();
                assert!(s.is_success());
                assert_eq!(s.events.len(), 1);
                let e = (s.events[0].t - 2_f64.ln()).abs();
                assert!(e < 1e-4);
                assert!(
                    (s.t.last().unwrap() - if terminal { s.events[0].t } else { 2. }).abs() < 1e-12
                );
                cell(
                    json!({"topic":"M1","rtol":rtol,"terminal":terminal,"analytic_jac":analytic,"error":e,"accepted":s.stats.accepted_steps,"contract":"crossing/postprocessed terminal only"}),
                );
            }
        }
    }
}
#[test]
fn dae_combinations() {
    for rtol in [1e-5, 1e-7] {
        for dense in [false, true] {
            for analytic in [false, true] {
                let mut p = Problem::new(2, |_, y, f| {
                    f[0] = -y[0];
                    f[1] = y[1] - y[0] * y[0];
                    Ok(())
                })
                .mass(vec![1., 0., 0., 0.])
                .autonomous();
                if analytic {
                    p = p.jacobian(Jacobian::dense(|_, y, j| {
                        j.copy_from_slice(&[-1., 0., -2. * y[0], 1.]);
                        Ok(())
                    }));
                }
                let s = Rodas5p::solve(&p, (0., 1.), &[1., 1.], &opts(rtol, dense)).unwrap();
                assert!(s.is_success(), "{:?}", s.status);
                let y = s.final_state().unwrap();
                let e = (y[0] - (-1_f64).exp())
                    .abs()
                    .max((y[1] - (-2_f64).exp()).abs());
                let constraint =
                    s.y.iter()
                        .map(|v| (v[1] - v[0] * v[0]).abs())
                        .fold(0., f64::max);
                assert!(e < 1e-3 && constraint < 1e-3);
                cell(
                    json!({"topic":"M2","rtol":rtol,"dense":dense,"analytic_jac":analytic,"error":e,"constraint":constraint,"accepted":s.stats.accepted_steps,"contract":"consistent index-1 manufactured DAE; accepted nodes"}),
                );
            }
        }
    }
}
#[test]
fn rhs_combinations() {
    for n in [8, 32] {
        for nrhs in [1, 8, 32] {
            for sparse in [false, true] {
                let mut a = DenseMatrix::zeros(n, n);
                for i in 0..n {
                    for j in 0..n {
                        a[(i, j)] = if i == j {
                            2.
                        } else if sparse && i.abs_diff(j) > 1 {
                            0.
                        } else {
                            0.0078125
                        };
                    }
                }
                let exact = (0..nrhs)
                    .map(|j| {
                        (0..n)
                            .map(|i| ((i + 3 * j) % 17) as f64 / 16. - 0.5)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                let b = exact
                    .iter()
                    .map(|v| a.matvec(v).unwrap())
                    .collect::<Vec<_>>();
                let mut owner = PreparedRhs::new(a).unwrap();
                for threads in [1, 4] {
                    let ex = ParallelExecution::rayon(threads).unwrap();
                    for mode in [
                        BatchMode::FreshFactor,
                        BatchMode::SerialReuse,
                        BatchMode::FaerBatch,
                        BatchMode::RayonChunks,
                    ] {
                        for verifier in [
                            ResidualMode::Dense,
                            ResidualMode::Auto,
                            ResidualMode::ExactSupport,
                        ] {
                            let r = owner.solve_with_verifier(&b, mode, verifier, &ex);
                            if !sparse && verifier == ResidualMode::ExactSupport {
                                assert!(r.is_err());
                                cell(
                                    json!({"topic":"M3","n":n,"rhs":nrhs,"sparse":sparse,"threads":threads,"mode":format!("{mode:?}"),"verifier":format!("{verifier:?}"),"outcome":"expected_refusal"}),
                                );
                                continue;
                            }
                            let r = r.unwrap();
                            let e = r
                                .values
                                .iter()
                                .flatten()
                                .zip(exact.iter().flatten())
                                .map(|(x, y)| (x - y).abs())
                                .fold(0., f64::max);
                            assert!(e <= r.error_upper && r.error_upper < 1e-11);
                            cell(
                                json!({"topic":"M3","n":n,"rhs":nrhs,"sparse":sparse,"threads":threads,"mode":format!("{mode:?}"),"verifier":format!("{verifier:?}"),"error":e,"bound":r.error_upper}),
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn dictionary_combinations() {
    for k in [1., 64., 1000.] {
        let m = Model::new(vec![k], vec![0.], vec![0.], vec![0.2]).unwrap();
        for h in [0.015625, 0.03125] {
            for mode in [
                None,
                Some(RealMode::TableFast),
                Some(RealMode::TableDirected),
                Some(RealMode::Hybrid),
            ] {
                let table = match mode {
                    None => RetroTable::new(),
                    Some(m) => RetroTable::with_dictionary(m),
                }
                .unwrap();
                let r = retro_green::integrate(&table, &m, &[0.8], 0.125, h, 1e-5).unwrap();
                let exact = 0.8 * (-k * 0.125).exp()
                    / (1. + 0.2 / k * 0.64 * (-(-2. * k * 0.125).exp_m1())).sqrt();
                let e = (r.endpoint()[0] - exact).abs();
                assert!(e <= r.error_upper() + 2e-15 && r.error_upper() <= 1e-5);
                cell(
                    json!({"topic":"M4","k":k,"hmax":h,"mode":format!("{mode:?}"),"error":e,"bound":r.error_upper()}),
                );
            }
        }
    }
}
#[test]
fn aperiodic_combinations() {
    for n in [2, 3] {
        for count in [7, 37] {
            let g = (0..count)
                .map(|k| {
                    let mut a = DenseMatrix::zeros(n, n);
                    for i in 0..n {
                        a[(i, i)] = 0.5 + ((k + 2 * i) % 5) as f64 / 64.;
                        a[(i, (i + 1 + k % 3) % n)] += 0.0625;
                    }
                    a
                })
                .collect::<Vec<_>>();
            assert_ne!(g[0].matmul(&g[1]).unwrap(), g[1].matmul(&g[0]).unwrap());
            let y = (0..n).map(|i| i as f64 / 16.).collect::<Vec<_>>();
            let exact = (0..count)
                .map(|k| {
                    (0..n)
                        .map(|i| ((i + 2 * k) % 13) as f64 / 32. - 0.25)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let mut prev = y.as_slice();
            let mut b = Vec::new();
            for (a, x) in g.iter().zip(&exact) {
                b.push(
                    x.iter()
                        .zip(a.matvec(prev).unwrap())
                        .map(|(x, v)| x - v)
                        .collect(),
                );
                prev = x;
            }
            let p = Aperiodic::new(g).unwrap();
            for threads in [1, 2, 4] {
                let ex = ParallelExecution::rayon(threads).unwrap();
                for mode in [TimeMode::Sequential, TimeMode::OrderedScan] {
                    let r = p.solve(&y, &b, mode, &ex).unwrap();
                    let e = r
                        .values
                        .iter()
                        .flatten()
                        .zip(exact.iter().flatten())
                        .map(|(a, b)| (a - b).abs())
                        .fold(0., f64::max);
                    assert!(e <= r.error_upper && r.error_upper < 1e-11);
                    cell(
                        json!({"topic":"M5","n":n,"nodes":count,"threads":threads,"mode":format!("{mode:?}"),"error":e,"bound":r.error_upper,"contract":"discrete affine recurrence"}),
                    );
                }
            }
        }
    }
    for k in [1., 100.] {
        for analytic in [false, true] {
            for rtol in [1e-5, 1e-7] {
                let mut p = Problem::new(1, move |t, y, f| {
                    f[0] = -k * (y[0] - t.sin()) + t.cos();
                    Ok(())
                });
                if analytic {
                    p = p
                        .jacobian(Jacobian::dense(move |_, _, j| {
                            j[0] = -k;
                            Ok(())
                        }))
                        .time_derivative(move |t, _, v| {
                            v[0] = k * t.cos() - t.sin();
                            Ok(())
                        });
                }
                let s = Rodas5p::solve(&p, (0., 1.), &[0.], &opts(rtol, true)).unwrap();
                assert!(s.is_success());
                let e = (s.final_state().unwrap()[0] - 1_f64.sin()).abs();
                assert!(e < 1e-3);
                cell(
                    json!({"topic":"M5","k":k,"analytic":analytic,"rtol":rtol,"error":e,"accepted":s.stats.accepted_steps,"contract":"existing nonautonomous facade"}),
                );
            }
        }
    }
}
