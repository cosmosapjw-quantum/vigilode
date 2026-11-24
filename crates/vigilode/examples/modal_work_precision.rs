//! Run with cargo run --release -p vigilode --example modal_work_precision
//! Modal coordinates, constant diagonal generator, endpoint-only, one thread.
use std::{hint::black_box, time::Instant};
use vigilode::adaptive_modal::{ModalInput, ModalOutcome, ModalStop};
use vigilode::{Atol, Jacobian, Options, Problem, Rodas5p, VigilODE};

fn main() {
    println!("dimension,rtol,rep,method,elapsed_ns,max_abs_error,accepted,rejected");
    for n in [64, 512] {
        let rates: Vec<f64> = (0..n)
            .map(|k| -1000.0 * (k as f64 / (n - 1) as f64).powi(2))
            .collect();
        let y0: Vec<f64> = (0..n)
            .map(|k| 1.0 / (1.0 + 0.0001 * (k * k) as f64))
            .collect();
        let rhs = rates.clone();
        let jac = rates.clone();
        let problem = Problem::new(n, move |_, y, out| {
            for ((f, &r), &y) in out.iter_mut().zip(&rhs).zip(y) {
                *f = r * y;
            }
            Ok(())
        })
        .jacobian(Jacobian::banded(0, 0, move |_, _, out| {
            out.copy_from_slice(&jac);
            Ok(())
        }))
        .time_derivative(|_, _, out| {
            out.fill(0.);
            Ok(())
        });
        let reference: Vec<_> = rates
            .iter()
            .zip(&y0)
            .map(|(r, y)| (0.5 * r).exp() * y)
            .collect();
        for rtol in [1e-4, 1e-6, 1e-8] {
            let opts = Options {
                rtol,
                atol: Atol::Scalar(rtol * 1e-3),
                h0: 0.3,
                ..Default::default()
            };
            for rep in 0..6 {
                for turn in 0..3 {
                    let method = ["modal", "protected_band0", "closed_exp"][(turn + rep) % 3];
                    let start = Instant::now();
                    let (v, accepted, rejected) = if method == "modal" {
                        let ModalOutcome::Modal(run) =
                            VigilODE::solve_modal(ModalInput::Rates(&rates), (0., 0.5), &y0, &opts)
                                .unwrap()
                        else {
                            panic!("unexpected fallback")
                        };
                        assert_eq!(run.stop, ModalStop::ReachedEnd);
                        (run.state, run.accepted, run.rejected)
                    } else if method == "protected_band0" {
                        let sol = Rodas5p::solve(&problem, (0., 0.5), &y0, &opts).unwrap();
                        assert!(sol.is_success());
                        (
                            sol.final_state().unwrap().to_vec(),
                            sol.stats.counters.accepted_steps as usize,
                            sol.stats.counters.rejected_steps as usize,
                        )
                    } else {
                        let endpoint = black_box(&rates)
                            .iter()
                            .zip(black_box(&y0))
                            .map(|(r, y)| (0.5 * r).exp() * y)
                            .collect::<Vec<_>>();
                        (endpoint, 1, 0)
                    };
                    let elapsed = start.elapsed().as_nanos();
                    let max_abs = black_box(&v)
                        .iter()
                        .zip(&reference)
                        .map(|(a, b)| (a - b).abs())
                        .fold(0., f64::max);
                    if rep != 0 {
                        println!(
                            "{n},{rtol},{rep},{},{elapsed},{max_abs:.16e},{accepted},{rejected}",
                            method
                        );
                    }
                }
            }
        }
    }
}
