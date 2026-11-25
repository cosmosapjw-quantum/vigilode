#![cfg(feature = "polynomial-events")]
use rodas5p_integrators::AdaptiveStepConfig;
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode::polynomial_events::retro_hybrid_reset::*;
use vigilode::retro_research::retro_portfolio::Portfolio;
const ROUTES: [&str; 8] = [
    "Represented",
    "Carry",
    "LegacyFlow",
    "DictionaryFlow",
    "Hinted",
    "Transport",
    "Combined",
    "Adaptive",
];
fn route(s: &str) -> Option<ResetRoute> {
    match s {
        "Represented" => Some(ResetRoute::Represented),
        "Carry" => Some(ResetRoute::Carry),
        "LegacyFlow" => Some(ResetRoute::LegacyFlow),
        "DictionaryFlow" => Some(ResetRoute::DictionaryFlow),
        "Hinted" => Some(ResetRoute::Hinted),
        "Transport" => Some(ResetRoute::Transport),
        "Combined" => Some(ResetRoute::Combined),
        "Adaptive" => None,
        _ => panic!("route"),
    }
}
fn reset(scale: f64, offset: f64, time: f64) -> ResetMap {
    ResetMap {
        state_scale: scale,
        offset,
        time_scale: time,
    }
}
fn mode(rate: f64, forcing: f64, threshold: f64, r: ResetMap, next: usize) -> Mode {
    Mode {
        rate,
        forcing,
        guard: Some(Guard {
            threshold,
            reset: r,
            next,
        }),
    }
}
fn free(rate: f64, forcing: f64) -> Mode {
    Mode {
        rate,
        forcing,
        guard: None,
    }
}
fn cfg() -> AdaptiveStepConfig {
    AdaptiveStepConfig {
        rtol: 1e-8,
        atol: 1e-10,
        initial_step: 0.001,
        max_step: 0.125,
        ..Default::default()
    }
}
struct Fixture {
    name: &'static str,
    group: &'static str,
    system: HybridSystem,
    y: f64,
    end: f64,
}
fn fixture(name: &'static str, group: &'static str, modes: Vec<Mode>, y: f64, end: f64) -> Fixture {
    Fixture {
        name,
        group,
        system: HybridSystem::new(modes).unwrap(),
        y,
        end,
    }
}
fn fixtures(holdout: bool) -> Vec<Fixture> {
    if holdout {
        vec![
            fixture(
                "single-holdout",
                "single",
                vec![mode(2., 0.25, 0.5, reset(1.5, 0.125, 0.), 1), free(4., 0.5)],
                1.25,
                1.5,
            ),
            fixture(
                "cycle-holdout",
                "cycle",
                vec![mode(2., 0.25, 0.5, reset(0., 1.125, 0.), 0)],
                1.125,
                2.2,
            ),
            fixture(
                "clock-reset-holdout",
                "clock-reset",
                vec![mode(2., 0., 0.25, reset(0., 0.75, 0.125), 1), free(4., 3.5)],
                1.,
                1.5,
            ),
            fixture(
                "no-event-holdout",
                "no-event",
                vec![mode(1., 1., 0.25, reset(0., 1., 0.), 0)],
                2.,
                1.,
            ),
        ]
    } else {
        vec![
            fixture(
                "single",
                "single",
                vec![mode(1., 0., 0.5, reset(2., 0.25, 0.), 1), free(2., 0.5)],
                1.,
                2.5,
            ),
            fixture(
                "stiff",
                "single",
                vec![mode(1000., 0., 0.5, reset(1., 0.75, 0.), 1), free(2., 0.5)],
                1.,
                0.02,
            ),
            fixture(
                "cycle",
                "cycle",
                vec![mode(1., 0., 0.5, reset(0., 1., 0.), 0)],
                1.,
                2.4,
            ),
            fixture(
                "alternate",
                "cycle",
                vec![
                    mode(1., 0., 0.5, reset(0., 0.125, 0.), 1),
                    mode(2., 2., 0.75, reset(0., 1.25, 0.), 0),
                ],
                1.,
                3.5,
            ),
            fixture(
                "clock-reset",
                "clock-reset",
                vec![mode(1., 0., 0.5, reset(1., 0.25, 0.25), 1), free(2., 0.5)],
                1.,
                2.,
            ),
            fixture(
                "correlated-clock",
                "clock-reset",
                vec![mode(1., 0., 0.5, reset(1., 0., 0.5), 1), free(2., 2.2)],
                1.,
                2.,
            ),
            fixture(
                "no-event",
                "no-event",
                vec![mode(1., 0., -0.25, reset(0., 1., 0.), 0)],
                1.,
                1.5,
            ),
        ]
    }
}
fn assert_accounting(r: &HybridRun) {
    assert_eq!(
        r.counters.rhs_calls,
        r.epochs.iter().map(|e| e.counters.rhs_calls).sum::<u64>()
    );
    assert_eq!(
        r.counters.accepted_steps,
        r.epochs
            .iter()
            .map(|e| e.counters.accepted_steps)
            .sum::<u64>()
    );
    assert_eq!(
        r.work.accepted_inspections,
        r.epochs.iter().map(|e| e.inspected_steps).sum::<u64>()
    );
    assert!(
        r.epochs
            .iter()
            .all(|e| e.inspected_steps == e.counters.accepted_steps)
    );
    for (k, e) in r.epochs.iter().enumerate() {
        assert_eq!(e.epoch, k);
        assert!(e.start <= e.committed_end && e.committed_end <= e.numerical_end);
        if k > 0 {
            let prev = &r.events[k - 1];
            assert_eq!(e.start, prev.sync_time);
            assert_eq!(e.start_center, prev.reset_center);
            assert_eq!(e.mode, prev.to);
            assert_eq!(e.start_enclosure, prev.post_enclosure);
        }
        if k < r.events.len() {
            let ev = &r.events[k];
            assert!(e.committed);
            assert_eq!(ev.epoch, k);
            assert_eq!(ev.sync_time, e.committed_end);
            assert_eq!(ev.from, e.mode);
            assert_eq!(ev.bracket[1], ev.sync_time);
            assert!(ev.bracket[0] > e.start);
        }
        if !e.committed {
            assert_eq!(k, r.epochs.len() - 1);
            assert_eq!(e.committed_end, e.start);
        }
    }
    if r.physical_qualified {
        assert_eq!(r.status, HybridStatus::Complete);
        assert!(r.enclosure.is_some());
    }
}
#[test]
fn physical_terminal_requires_no_later_trials() {
    let f = fixtures(false).remove(0);
    let r = f
        .system
        .run(
            0,
            f.y,
            f.end,
            &cfg(),
            Default::default(),
            ResetRoute::Transport,
        )
        .unwrap();
    assert_accounting(&r);
    assert!(r.physical_qualified, "{:?}", r.reason);
    assert_eq!(r.events.len(), 1);
    let ev = &r.events[0];
    let t = 2_f64.ln();
    assert!(ev.bracket[0] <= t && t <= ev.bracket[1]);
    assert!(
        r.epochs[0].numerical_end < 1.,
        "physical terminal path continued to full horizon"
    );
    assert_eq!(r.epochs[0].stop_reason, "ClientStop");
    assert_eq!(r.epochs[1].start, ev.sync_time);
    let expected = 0.25 + (-2. * (f.end - t)).exp();
    let z = r.enclosure.unwrap();
    assert!(z[0] <= expected && expected <= z[1]);
}
#[test]
fn repeated_resets_and_represented_control() {
    let f = fixtures(false).remove(2);
    for c in ROUTES[..7].iter() {
        let r = f
            .system
            .run(0, f.y, f.end, &cfg(), Default::default(), route(c).unwrap())
            .unwrap();
        assert_accounting(&r);
        if *c == "Carry" && !r.physical_qualified {
            assert_eq!(r.status, HybridStatus::Unresolved);
            continue;
        }
        assert_eq!(r.status, HybridStatus::Complete, "{c}: {:?}", r.reason);
        assert_eq!(r.events.len(), 3);
        if *c == "Represented" {
            assert!(!r.physical_qualified);
            assert!(r.enclosure.is_none());
            continue;
        }
        assert!(r.physical_qualified);
        for (i, e) in r.events.iter().enumerate() {
            let t = (i + 1) as f64 * 2_f64.ln();
            assert!(e.bracket[0] <= t && t <= e.bracket[1]);
        }
        let expected = (-(f.end - 3. * 2_f64.ln())).exp();
        let z = r.enclosure.unwrap();
        assert!(z[0] <= expected && expected <= z[1]);
    }
}
#[test]
fn time_dependent_reset_correlation_and_adaptive_accounting() {
    let f = fixtures(false).remove(5);
    let p = ResetPolicy {
        time_tol: 1e-9,
        ..Default::default()
    };
    let r = f
        .system
        .run(0, f.y, f.end, &cfg(), p, ResetRoute::Transport)
        .unwrap();
    assert_accounting(&r);
    assert!(r.physical_qualified, "{:?}", r.reason);
    let ev = &r.events[0];
    assert!(ev.transported_width.unwrap() < ev.rectangle_width.unwrap());
    assert!(r.work.sync_transport > 0 && r.work.gain_queries > 0);
    let f = fixtures(false).remove(0);
    let mut selector = Portfolio::new(1, 6).unwrap();
    let s = f
        .system
        .adaptive(0, f.y, f.end, &cfg(), p, &mut selector)
        .unwrap();
    let v = s.value.unwrap();
    assert!(v.physical_qualified);
    assert_accounting(&v);
    assert_eq!(
        s.charged_native.rhs_calls,
        s.attempted_runs
            .iter()
            .map(|e| e.counters.rhs_calls)
            .sum::<u64>()
    );
    assert_eq!(
        s.charged_work.accepted_inspections,
        s.attempted_runs
            .iter()
            .map(|e| e.work.accepted_inspections)
            .sum::<u64>()
    );
}
#[test]
fn atomic_failure_and_anti_zeno_contract() {
    let f = fixtures(false).remove(2);
    let r = f
        .system
        .run(
            0,
            f.y,
            f.end,
            &cfg(),
            ResetPolicy {
                max_resets: 1,
                ..Default::default()
            },
            ResetRoute::Transport,
        )
        .unwrap();
    assert_accounting(&r);
    assert_eq!(r.status, HybridStatus::ResetLimit);
    assert_eq!(r.events.len(), 1);
    assert_eq!(r.time, r.events[0].sync_time);
    assert!(!r.epochs.last().unwrap().committed);
    assert!(r.epochs.last().unwrap().counters.rhs_calls > 0);
    let g = HybridSystem::new(vec![mode(1., 0., 0.5, reset(0., 0.5, 0.), 0)]).unwrap();
    let r = g
        .run(0, 1., 2., &cfg(), Default::default(), ResetRoute::Transport)
        .unwrap();
    assert_accounting(&r);
    assert_eq!(r.status, HybridStatus::Unresolved);
    assert_eq!(r.time, 0.);
    assert!(r.events.is_empty());
    assert!(r.reason.unwrap().contains("next guard"));
    let r = g
        .run(
            0,
            0.5,
            2.,
            &cfg(),
            Default::default(),
            ResetRoute::Transport,
        )
        .unwrap();
    assert_eq!(r.counters.rhs_calls, 0);
    assert!(r.epochs.is_empty());
    assert_eq!(r.time, 0.);
    let f = fixtures(false).remove(0);
    let r = f
        .system
        .run(
            0,
            f.y,
            f.end,
            &cfg(),
            ResetPolicy {
                min_advance: 1.,
                ..Default::default()
            },
            ResetRoute::Transport,
        )
        .unwrap();
    assert_eq!(r.status, HybridStatus::Unresolved);
    assert!(r.events.is_empty());
    assert!(r.counters.rhs_calls > 0);
    let c = AdaptiveStepConfig {
        max_attempts: 1,
        ..cfg()
    };
    let r = f
        .system
        .run(0, f.y, f.end, &c, Default::default(), ResetRoute::Transport)
        .unwrap();
    assert_accounting(&r);
    assert_eq!(r.status, HybridStatus::DriverFailure);
    assert_eq!(r.time, 0.);
    assert!(r.counters.rhs_calls > 0);
    assert!(HybridSystem::new(vec![mode(1., 0., 0.5, reset(0., 1., 0.), 1)]).is_err());
    assert!(HybridSystem::new(vec![free(0., 1.)]).is_err());
}
fn source(f: &Fixture) {
    println!(
        "SOURCE {}",
        json!({"case":f.name,"group":f.group,"system":f.system,"y":f.y,"end":f.end,"config":{"rtol":cfg().rtol,"atol":cfg().atol,"h0":cfg().initial_step,"max_step":cfg().max_step}})
    );
}
fn measure(f: &Fixture, c: &str, tol: f64) -> Value {
    let p = ResetPolicy {
        time_tol: tol,
        ..Default::default()
    };
    let mut sel = Portfolio::new(4, 6).unwrap();
    let mut ns = vec![];
    let mut last = None;
    for i in 0..4 {
        let now = Instant::now();
        let r = if let Some(m) = route(c) {
            let v = f.system.run(0, f.y, f.end, &cfg(), p, m);
            v.map(|r|{
 assert_accounting(&r);let n=r.counters;let w=r.work.clone();json!({"run":r,"charged_native":n,"charged_work":w,"attempts":[],"attempted_runs":[]})})
        } else {
            f.system.adaptive(0,f.y,f.end,&cfg(),p,&mut sel).map(|s|{
 for r in &s.attempted_runs{assert_accounting(r);}
 assert_eq!(s.charged_native.rhs_calls,s.attempted_runs.iter().map(|r|r.counters.rhs_calls).sum::<u64>());
 json!({"run":s.value,"charged_native":s.charged_native,"charged_work":s.charged_work,"attempts":s.attempts,"attempted_runs":s.attempted_runs})})
        };
        let dt = now.elapsed().as_nanos() as u64;
        if i > 0 {
            ns.push(dt);
        }
        last = Some(black_box(r));
    }
    ns.sort_unstable();
    match last.unwrap() {
        Ok(r) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"time_tol":tol,"state_tol":p.state_tol,"median_ns":ns[1],"samples_ns":ns,"result":r})
        }
        Err(e) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"time_tol":tol,"state_tol":p.state_tol,"median_ns":ns[1],"samples_ns":ns,"result":null,"reason":e.to_string()})
        }
    }
}
#[test]
#[ignore = "finite candidate campaign"]
fn training() {
    for (i, f) in fixtures(false).iter().enumerate() {
        source(f);
        for tol in [1e-6, 1e-9] {
            for j in 0..ROUTES.len() {
                println!("CELL {}", measure(f, ROUTES[(i + j) % ROUTES.len()], tol));
            }
        }
    }
}
#[test]
#[ignore = "frozen selected physical candidates"]
fn holdout() {
    let p = std::env::var("VIGILODE_HYBRID_SELECTION").unwrap();
    let d: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    for f in fixtures(true) {
        source(&f);
        for (k, tol) in [("1e-6", 1e-6), ("1e-9", 1e-9)] {
            if let Some(c) = d["groups"][f.group][k]["selected"].as_str() {
                println!("CELL {}", measure(&f, c, tol));
            }
        }
    }
}

#[test]
fn imminent_guard_and_too_tight_state_budget_fail_closed() {
    let g = HybridSystem::new(vec![
        mode(1., 0., 0.5, reset(0., 0.5, 0.), 1),
        mode(1., 0., 0.5_f64.next_down(), reset(0., 1., 0.), 1),
    ])
    .unwrap();
    let r = g
        .run(0, 1., 2., &cfg(), Default::default(), ResetRoute::Transport)
        .unwrap();
    assert_accounting(&r);
    assert_eq!(r.status, HybridStatus::Unresolved);
    assert!(r.events.is_empty());
    assert_eq!(r.time, 0.);
    assert!(r.reason.unwrap().contains("synchronization gap"));
    let f = fixtures(false).remove(0);
    let r = f
        .system
        .run(
            0,
            f.y,
            f.end,
            &cfg(),
            ResetPolicy {
                state_tol: 1e-18,
                ..Default::default()
            },
            ResetRoute::DictionaryFlow,
        )
        .unwrap();
    assert_accounting(&r);
    assert_eq!(r.status, HybridStatus::Unresolved);
    assert!(!r.physical_qualified);
    assert_eq!(r.time, 0.);
    assert!(r.events.is_empty());
}
