use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use vigilode::{
    Atol, CallbackError, Jacobian, Options, Problem, Rodas5p, split_forcing::SplitForcing,
};

fn split(threads: usize, calls: Arc<AtomicUsize>) -> SplitForcing {
    SplitForcing::new(
        1,
        threads,
        |_t, y, out| {
            out[0] = -80.0 * y[0] - y[0].powi(3);
            Ok(())
        },
        move |t, out| {
            calls.fetch_add(1, Ordering::Relaxed);
            out[0] = (5.0 * t).sin();
            Ok(())
        },
    )
    .unwrap()
}
fn provided(problem: Problem, analytic_t: bool) -> Problem {
    let p = problem.jacobian(Jacobian::dense(|_t, y, j| {
        j[0] = -80.0 - 3.0 * y[0] * y[0];
        Ok(())
    }));
    if analytic_t {
        p.time_derivative(|t, _y, out| {
            out[0] = 5.0 * (5.0 * t).cos();
            Ok(())
        })
    } else {
        p
    }
}

#[test]
fn nonlinear_native_attempt_and_analytic_partial_t_are_bit_identical() {
    for threads in [1, 3] {
        let calls = Arc::new(AtomicUsize::new(0));
        let split = split(threads, Arc::clone(&calls));
        let h = 0.001;
        let config = Options {
            h0: h,
            ..Default::default()
        };
        let direct = Rodas5p::solve(
            &provided(split.direct_problem(), true),
            (0.0, h),
            &[0.2],
            &config,
        )
        .unwrap();
        let ordinary_calls = calls.swap(0, Ordering::Relaxed);
        let ready = split.prepare_rodas5p_trial(0.0, h).unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 6);
        let result = Rodas5p::solve(&provided(ready, true), (0.0, h), &[0.2], &config).unwrap();
        assert!(direct.is_success() && result.is_success());
        assert_eq!(direct.stats.accepted_steps, 1);
        assert_eq!(result.y, direct.y);
        assert_eq!(result.stats.rejected_steps, 0);
        assert_eq!(calls.load(Ordering::Relaxed), 6);
        assert!(ordinary_calls >= 8);
    }
}

#[test]
fn rejected_attempt_and_fd_time_probe_fall_back_at_their_exact_clocks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let split = split(3, Arc::clone(&calls));
    let h = 0.2;
    let config = Options {
        h0: h,
        atol: Atol::Scalar(1e-12),
        rtol: 1e-10,
        ..Default::default()
    };
    let direct = Rodas5p::solve(
        &provided(split.direct_problem(), false),
        (0.0, h),
        &[0.2],
        &config,
    )
    .unwrap();
    calls.store(0, Ordering::Relaxed);
    let ready = split.prepare_rodas5p_trial(0.0, h).unwrap();
    let result = Rodas5p::solve(&provided(ready, false), (0.0, h), &[0.2], &config).unwrap();
    assert!(direct.is_success() && result.is_success());
    assert!(result.stats.rejected_steps > 0);
    assert_eq!(result.stats.rejected_steps, direct.stats.rejected_steps);
    assert_eq!(result.stats.accepted_steps, direct.stats.accepted_steps);
    assert_eq!(result.y, direct.y);
    assert!(calls.load(Ordering::Relaxed) > 6);
}

#[test]
fn consumes_only_requested_speculative_errors() {
    let split = SplitForcing::new(
        1,
        3,
        |_t, _y, out| {
            out[0] = 2.0;
            Ok(())
        },
        |t, out| {
            if t > 0.75 {
                return Err(CallbackError::Fatal("future forcing".into()));
            }
            out[0] = t;
            Ok(())
        },
    )
    .unwrap();
    let ready = split.prepare_rodas5p_trial(0.0, 1.0).unwrap();
    let mut out = [0.0];
    (ready.rhs_fn())(0.0, &[1.0], &mut out).unwrap();
    assert_eq!(out, [2.0]);
    assert!(matches!(
        (ready.rhs_fn())(1.0, &[1.0], &mut out),
        Err(CallbackError::Fatal(_))
    ));
    assert!(
        split
            .prepare_rodas5p_trial(1.0, f64::EPSILON / 4.0)
            .is_err()
    );
}

/// Warm local pool; elapsed includes preparing the table and the entire production-facade solve.
/// Mode-sum forcing is a synthetic callback-cost sweep on one short nonlinear trial.
#[test]
#[ignore = "wall-time experiment; run --release --ignored --nocapture"]
fn split_forcing_trial_wall_time() {
    use std::time::Instant;
    for modes in [8_usize, 100_000] {
        let mut arms = Vec::new();
        for workers in [1, 1, 2, 4] {
            let split = SplitForcing::new(
                1,
                workers,
                |_t, y, out| {
                    out[0] = -80.0 * y[0] - y[0].powi(3);
                    Ok(())
                },
                move |t, out| {
                    let mut g = 0.0;
                    for i in 1..=modes {
                        let rate = 1.0 + (i as f64) / (modes as f64);
                        g += (-rate * t).exp() / (1.0 + rate);
                    }
                    out[0] = g / (modes as f64);
                    Ok(())
                },
            )
            .unwrap();
            arms.push(split);
        }
        let mut samples = vec![Vec::new(); arms.len()];
        let config = Options {
            h0: 0.001,
            ..Default::default()
        };
        let mut first_endpoint = None;
        for rep in 0..13 {
            for offset in 0..arms.len() {
                let k = (rep + offset) % arms.len();
                let start = Instant::now();
                let p = if k == 0 {
                    arms[k].direct_problem()
                } else {
                    arms[k].prepare_rodas5p_trial(0.0, 0.001).unwrap()
                };
                let p = provided(p, false);
                let s = Rodas5p::solve(&p, (0.0, 0.001), &[0.2], &config).unwrap();
                let ns = start.elapsed().as_nanos() as u64;
                assert!(s.is_success());
                assert_eq!(s.stats.accepted_steps, 1);
                assert_eq!(s.stats.rejected_steps, 0);
                let endpoint = s.final_state().unwrap()[0].to_bits();
                assert_eq!(endpoint, *first_endpoint.get_or_insert(endpoint));
                if rep != 0 {
                    samples[k].push(ns);
                }
            }
        }
        println!(
            "{}",
            serde_json::json!({"modes":modes,"arms":["direct","cache1","cache2","cache4"],
            "samples_ns":samples,"endpoint_bits":first_endpoint.unwrap()})
        );
    }
}

#[test]
fn manufactured_nonlinear_solution_with_analytic_reference() {
    let exact = |t: f64| 0.25 + t.sin();
    let split = SplitForcing::new(
        1,
        2,
        |_t, y, out| {
            out[0] = -80.0 * y[0] - y[0].powi(3);
            Ok(())
        },
        move |t, out| {
            let x = exact(t);
            out[0] = t.cos() + 80.0 * x + x.powi(3);
            Ok(())
        },
    )
    .unwrap();
    for rtol in [1.0e-4, 1.0e-6, 1.0e-8] {
        let config = Options {
            h0: 0.05,
            rtol,
            atol: Atol::Scalar(rtol * 0.01),
            ..Default::default()
        };
        let mut direct_trajectory = None;
        for p in [
            split.direct_problem(),
            split.prepare_rodas5p_trial(0.0, 0.05).unwrap(),
        ] {
            let p = provided(p, false).time_derivative(move |t, _y, out| {
                out[0] = -t.sin() + (80.0 + 3.0 * exact(t).powi(2)) * t.cos();
                Ok(())
            });
            let s = Rodas5p::solve(&p, (0.0, 1.0), &[0.25], &config).unwrap();
            assert!(s.is_success());
            let err = (s.final_state().unwrap()[0] - exact(1.0)).abs();
            assert!(err <= 2.0 * rtol);
            if let Some(rows) = &direct_trajectory {
                assert_eq!(rows, &s.y);
                println!(
                    "{}",
                    serde_json::json!({"case":"manufactured","rtol":rtol,"endpoint_error":err,
                    "accepted":s.stats.accepted_steps,"rejected":s.stats.rejected_steps})
                );
            } else {
                direct_trajectory = Some(s.y);
            }
        }
    }
}

#[test]
fn incomplete_forcing_is_delayed_and_rejected_on_hit_miss_and_direct() {
    use vigilode::split_forcing::ForcingWork;
    let split = SplitForcing::new(
        2,
        2,
        |_t, _y, out| {
            out.fill(1.0);
            Ok(())
        },
        |t, out| {
            out[0] = 1.0;
            if t == 0.0 {
                out[1] = 2.0;
            }
            Ok(())
        },
    )
    .unwrap();
    let ready = split.prepare_rodas5p_trial(0.0, 1.0).unwrap();
    let mut out = [0.0; 2];
    (ready.rhs_fn())(0.0, &[1.0; 2], &mut out).unwrap();
    assert_eq!(out, [2.0, 3.0]);
    for (p, t) in [(&ready, 1.0), (&ready, 0.3), (&split.direct_problem(), 0.3)] {
        let e = (p.rhs_fn())(t, &[0.0; 2], &mut out).unwrap_err();
        assert!(matches!(e,CallbackError::Fatal(message) if message.contains("split forcing")));
    }
    assert_eq!(
        split.work(),
        ForcingWork {
            prepared: 6,
            hits: 2,
            misses: 2,
            failed_evaluations: 7
        }
    );
}

#[test]
fn incomplete_state_takes_precedence_and_does_not_consume_row() {
    let split = SplitForcing::new(
        2,
        1,
        |_t, _y, out| {
            out[0] = 0.0;
            Ok(())
        },
        |_t, _out| Err(CallbackError::Fatal("unavailable".into())),
    )
    .unwrap();
    let p = split.prepare_rodas5p_trial(0.0, 1.0).unwrap();
    let mut out = [1.0; 2];
    assert!(
        matches!((p.rhs_fn())(0.0,&[1.0;2],&mut out),Err(CallbackError::Fatal(message)) if message.contains("state callback"))
    );
    assert_eq!(split.work().hits, 0);
}
