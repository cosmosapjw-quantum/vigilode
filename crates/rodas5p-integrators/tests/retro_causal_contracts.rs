use rodas5p_integrators::{retro_causal::*, retro_green::RetroTable};

#[test]
fn endpoint_plan_transports_early_error_and_retains_dense_constraints() {
    let table = RetroTable::new().unwrap();
    let slabs = [
        SlabSpec {
            end: 1.,
            rate: 1.,
            forcing: 1.,
            forecast: 1.,
            radius: 0.,
        },
        SlabSpec {
            end: 2.,
            rate: 4.,
            forcing: 1.,
            forecast: 4.,
            radius: 0.,
        },
    ];
    let targets = [Target {
        slab: 1,
        kind: OutputKind::Endpoint,
        tolerance: 1e-6,
    }];
    let p = plan(&table, &slabs, &targets, Mode::Retro, 1e-2).unwrap();
    assert!(p.budgets[0] > 10. * p.budgets[1]);
    assert!(p.gain_upper[1] * p.budgets[0] + p.budgets[1] <= 1e-6);
    let dense = [
        targets[0],
        Target {
            slab: 0,
            kind: OutputKind::Dense,
            tolerance: 1e-6,
        },
    ];
    let p = plan(&table, &slabs, &dense, Mode::Retro, 1e-2).unwrap();
    assert!(p.budgets[0] <= 1e-6);
}

use serde_json::json;
fn fixture() -> Vec<SlabSpec> {
    let rates = [4., 4., 4., 4., 80., 80., 0.25, 0.25, 0.25, 0.25, 0.25, 0.25];
    let equilibria = [
        1., -0.25, 0.75, 0.25, -0.5, 0.6, 0.2, 1., -0.4, 0.5, 0.7, 0.1,
    ];
    rates
        .into_iter()
        .zip(equilibria)
        .enumerate()
        .map(|(i, (rate, q))| {
            // Deterministic forecast, with the actual value in a nontrivial box.
            let (forecast, radius) = if i == 4 || i == 5 {
                (60., 20.)
            } else if i >= 6 {
                (4., 3.75)
            } else {
                (4., 0.)
            };
            SlabSpec {
                end: (i + 1) as f64 / 4.,
                rate,
                forcing: rate * q,
                forecast,
                radius,
            }
        })
        .collect()
}
fn targets(n: usize, tol: f64, dense: bool) -> Vec<Target> {
    let mut t = vec![Target {
        slab: n - 1,
        kind: OutputKind::Endpoint,
        tolerance: tol,
    }];
    if dense {
        t.extend((0..n).map(|slab| Target {
            slab,
            kind: OutputKind::Dense,
            tolerance: tol,
        }));
    }
    t
}
fn exact(s: SlabSpec, t0: f64, t: f64, y: f64) -> f64 {
    let x = s.rate * (t - t0);
    (-x).exp() * y - s.forcing / s.rate * (-x).exp_m1()
}
fn report(table: &RetroTable, spec: &[SlabSpec], run: &Run, y0: f64) -> serde_json::Value {
    let (mut time, mut y) = (0., y0);
    let (mut maxerr, mut maxdense, mut maxratio) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut endpoint_err = 0.;
    let mut dense_available = true;
    for (&s, r) in spec.iter().zip(&run.slabs) {
        assert_eq!(r.start(), time);
        assert_eq!(r.end(), s.end);
        for j in 0..=16 {
            let t = time + (s.end - time) * (j as f64 / 16.);
            let (v, b) = r.query(table, t).unwrap();
            let e = (v - exact(s, time, t, y)).abs();
            assert!(e <= b + 4e-15, "t={t} err={e} bound={b}");
            maxerr = maxerr.max(e);
            if b > 0. {
                maxratio = maxratio.max(e / b);
            }
            if let Some(d) = r.dense_upper() {
                assert!(e <= d + 4e-15);
                maxdense = maxdense.max(d);
            } else {
                dense_available = false;
            }
        }
        y = exact(s, time, s.end, y);
        time = s.end;
        endpoint_err = (r.endpoint() - y).abs();
        assert!(endpoint_err <= r.endpoint_upper() + 4e-15);
    }
    json!({"error_endpoint":endpoint_err,"bound_endpoint":run.slabs.last().unwrap().endpoint_upper(),
        "max_observed_error":maxerr,"max_query_error_ratio":maxratio,
        "max_dense_upper":if dense_available{Some(maxdense)}else{None},
        "accepted_steps":run.slabs.iter().map(|s|s.native_steps()).sum::<usize>(),
        "work":run.work,"plan":run.plan,"slabs":run.slabs.iter().map(|r|json!({
            "end":r.end(),"endpoint":r.endpoint(),"endpoint_upper":r.endpoint_upper(),
            "local_endpoint":r.local_endpoint_upper(),"local_dense":r.local_dense_upper(),
            "incoming":r.incoming_upper(),"atol":r.accepted_atol(),"steps":r.native_steps()
        })).collect::<Vec<_>>()})
}
#[test]
fn wrong_forecast_fails_closed_and_invalid_contracts_refuse() {
    let t = RetroTable::new().unwrap();
    let mut s = fixture();
    let req = targets(s.len(), 1e-7, false);
    let robust = plan(&t, &s, &req, Mode::Retro, 1e-2).unwrap();
    assert!(!robust.forecast_fallback);
    assert!(robust.gain_upper[6] >= (-0.25_f64 / 4.).exp());
    s[6].radius = 0.;
    let p = plan(&t, &s, &req, Mode::Retro, 1e-2).unwrap();
    assert!(p.forecast_fallback);
    assert!(p.gain_upper.iter().all(|&g| g == 1.));
    // An arbitrarily overconfident but finite bad forecast must not force
    // evaluation of a table argument outside its declared domain.
    s[6].forecast = 1e100;
    assert!(
        plan(&t, &s, &req, Mode::Retro, 1e-2)
            .unwrap()
            .forecast_fallback
    );
    assert!(plan(&t, &s, &[], Mode::Retro, 1e-2).is_err());
    assert!(plan(&t, &s, &req, Mode::Retro, f64::NAN).is_err());
    s[0].end = 0.;
    assert!(plan(&t, &s, &req, Mode::Retro, 1e-2).is_err());
}
#[test]
fn native_stiff_transition_endpoint_and_dense_contracts() {
    let t = RetroTable::new().unwrap();
    let s = fixture();
    for dense in [false, true] {
        let req = targets(s.len(), 1e-7, dense);
        for mode in [Mode::Forward, Mode::Uniform, Mode::Retro] {
            let r = integrate(&t, &s, &req, mode, 1e-2, 0.2).unwrap();
            assert_eq!(r.slabs.len(), s.len());
            let m = report(&t, &s, &r, 0.2);
            assert!(m["bound_endpoint"].as_f64().unwrap() <= 1e-7);
            if dense {
                assert!(m["max_dense_upper"].as_f64().unwrap() <= 1e-7);
            }
            println!(
                "RUN {}",
                json!({"mode":mode,"dense":dense,"tol":1e-7,"metrics":m})
            );
        }
    }
}
#[test]
fn qualified_nonterminal_event_bracket_and_unresolved_boundary() {
    let t = RetroTable::new().unwrap();
    let s = [SlabSpec {
        end: 1.,
        rate: 2.,
        forcing: 1.,
        forecast: 2.,
        radius: 0.,
    }];
    let r = integrate(&t, &s, &targets(1, 1e-8, true), Mode::Retro, 1e-2, 2.).unwrap();
    let event = bracket_event(&t, &r.slabs[0], 1., 1e-6).unwrap();
    let root = 3.0_f64.ln() / 2.;
    match event {
        EventBracket::Unique { lo, hi, qualified } => {
            assert!(lo <= root && root <= hi);
            assert!(qualified);
        }
        _ => panic!("expected qualified unique crossing, got {event:?}"),
    }
    assert_eq!(
        bracket_event(&t, &r.slabs[0], 3., 1e-6).unwrap(),
        EventBracket::NoCrossing
    );
    assert_eq!(
        bracket_event(&t, &r.slabs[0], 2., 1e-6).unwrap(),
        EventBracket::Unresolved
    );
    assert!(r.slabs[0].query(&t, -0.1).is_err());
    println!(
        "EVENT {}",
        json!({"bracket":event,"analytic_root":root,"work":r.work})
    );
}
#[test]
#[ignore = "owner-host diagnostic timing; common actual native solver and certificates"]
fn measure_retro_causal() {
    use std::time::Instant;
    faer::set_global_parallelism(faer::Par::Seq);
    let s = fixture();
    for (dense, tol) in [
        (false, 1e-5),
        (false, 1e-7),
        (false, 1e-9),
        (true, 1e-5),
        (true, 1e-7),
    ] {
        let req = targets(s.len(), tol, dense);
        let modes = [Mode::Forward, Mode::Uniform, Mode::Retro];
        let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
        let mut metrics: [serde_json::Value; 3] = std::array::from_fn(|_| serde_json::Value::Null);
        for rep in 0..6 {
            for offset in 0..3 {
                let a = (rep + offset) % 3;
                let start = Instant::now();
                let table = RetroTable::new().unwrap();
                let r = integrate(&table, &s, &req, modes[a], 1e-2, 0.2).unwrap();
                let ms = start.elapsed().as_secs_f64() * 1000.;
                let m = report(&table, &s, &r, 0.2);
                assert!(m["bound_endpoint"].as_f64().unwrap() <= tol);
                if dense {
                    assert!(m["max_dense_upper"].as_f64().unwrap() <= tol);
                }
                if rep > 0 {
                    samples[a].push(ms);
                }
                metrics[a] = m;
            }
        }
        for a in 0..3 {
            let mut sorted = samples[a].clone();
            sorted.sort_by(f64::total_cmp);
            println!(
                "WP {}",
                json!({"mode":modes[a],"dense":dense,"tol":tol,"median_ms":sorted[2],"samples_ms":samples[a],"metrics":metrics[a]})
            );
        }
    }
}
