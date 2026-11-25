//! Finite candidate search with analytically known roots, not proof of all roots.
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use vigilode::{
    Atol, Direction, EventRootMethod, EventTransactionPolicy, Options, Problem, Rodas5p, Status,
};
type Count = Arc<AtomicU64>;
fn ev(
    p: Problem,
    count: &Count,
    terminal: bool,
    direction: Direction,
    f: impl Fn(f64, &[f64]) -> f64 + Send + Sync + 'static,
) -> Problem {
    let c = count.clone();
    let g = move |t, y: &[f64]| {
        c.fetch_add(1, Ordering::Relaxed);
        f(t, y)
    };
    if terminal {
        p.terminal_event(g, direction)
    } else {
        p.event(g, direction)
    }
}
fn case(
    name: &str,
    rtol: f64,
) -> (
    Problem,
    Vec<f64>,
    f64,
    Options,
    Vec<(usize, f64)>,
    bool,
    Count,
    Count,
) {
    let rhs = Arc::new(AtomicU64::new(0));
    let events = Arc::new(AtomicU64::new(0));
    let r = rhs.clone();
    let mut o = Options {
        rtol,
        atol: Atol::Scalar(rtol * 0.01),
        dense_output: true,
        ..Options::default()
    };
    let (p, y, t, expected, terminal) = match name {
        "harmonic-terminal" | "harmonic-crossings" => {
            let terminal = name == "harmonic-terminal";
            let p = Problem::new(2, move |_, y, f| {
                r.fetch_add(1, Ordering::Relaxed);
                f[0] = y[1];
                f[1] = -y[0];
                Ok(())
            })
            .autonomous();
            let p = ev(p, &events, terminal, Direction::Either, |_, y| y[0]);
            let pi = std::f64::consts::PI;
            let expected = if terminal {
                vec![(0, pi / 2.)]
            } else {
                vec![(0, pi / 2.), (0, 3. * pi / 2.), (0, 5. * pi / 2.)]
            };
            (p, vec![1., 0.], 8.5, expected, terminal)
        }
        "decay" | "stiff-decay" => {
            let k = if name == "decay" { 1. } else { 1000. };
            let p = Problem::new(1, move |_, y, f| {
                r.fetch_add(1, Ordering::Relaxed);
                f[0] = -k * y[0];
                Ok(())
            })
            .autonomous();
            (
                ev(p, &events, true, Direction::Down, |_, y| y[0] - 0.5),
                vec![1.],
                if k == 1. { 3. } else { 0.1 },
                vec![(0, 2_f64.ln() / k)],
                true,
            )
        }
        "clock-pair" | "holdout-pair" | "hidden-pair" => {
            o.h0 = 0.75;
            let (a, b) = match name {
                "clock-pair" => (0.311, 0.342),
                "holdout-pair" => (0.423, 0.451),
                _ => (0.431, 0.432),
            };
            let p = Problem::new(1, move |_, _, f| {
                r.fetch_add(1, Ordering::Relaxed);
                f[0] = 0.;
                Ok(())
            })
            .autonomous();
            (
                ev(p, &events, false, Direction::Either, move |t, _| {
                    (t - a) * (t - b)
                }),
                vec![0.],
                1.,
                vec![(0, a), (0, b)],
                false,
            )
        }
        "earliest-ties" | "holdout-ties" => {
            o.h0 = 0.75;
            let (a, b, c) = if name == "earliest-ties" {
                (0.3, 0.4, 0.6)
            } else {
                (0.213, 0.371, 0.719)
            };
            let p = Problem::new(1, move |_, _, f| {
                r.fetch_add(1, Ordering::Relaxed);
                f[0] = 0.;
                Ok(())
            })
            .autonomous();
            let p = ev(p, &events, true, Direction::Up, move |t, _| t - c);
            let p = ev(p, &events, true, Direction::Up, move |t, _| t - b);
            let p = ev(p, &events, false, Direction::Up, move |t, _| t - a);
            let p = ev(p, &events, false, Direction::Up, move |t, _| t - b);
            let p = ev(p, &events, false, Direction::Down, move |t, _| t - b);
            (p, vec![0.], 1., vec![(2, a), (1, b), (3, b)], true)
        }
        "holdout-nonautonomous" => {
            let p = Problem::new(1, move |t, _, f| {
                r.fetch_add(1, Ordering::Relaxed);
                f[0] = t.cos();
                Ok(())
            });
            (
                ev(p, &events, true, Direction::Up, |_, y| y[0] - 0.7),
                vec![0.],
                3.,
                vec![(0, 0.7_f64.asin())],
                true,
            )
        }
        "holdout-direction" => {
            let p = Problem::new(2, move |_, y, f| {
                r.fetch_add(1, Ordering::Relaxed);
                f[0] = y[1];
                f[1] = -y[0];
                Ok(())
            })
            .autonomous();
            (
                ev(p, &events, true, Direction::Up, |_, y| y[0]),
                vec![1., 0.],
                8.5,
                vec![(0, 1.5 * std::f64::consts::PI)],
                true,
            )
        }
        _ => panic!("unknown fixture"),
    };
    (p, y, t, o, expected, terminal, rhs, events)
}
fn policy(p: &Value) -> EventTransactionPolicy {
    EventTransactionPolicy {
        subdivisions: p["subdivisions"].as_u64().unwrap() as usize,
        root_method: match p["root_method"].as_str().unwrap() {
            "Illinois" => EventRootMethod::Illinois,
            "Bisection" => EventRootMethod::Bisection,
            _ => panic!("unknown refinement"),
        },
        max_step: p["max_step"].as_f64(),
    }
}
fn run(name: &str, rtol: f64, candidate: &str, pol: Option<EventTransactionPolicy>) -> Value {
    let (p, y, t, o, expected, terminal, rhs, events) = case(name, rtol);
    let begin = Instant::now();
    let r = if let Some(pol) = pol {
        Rodas5p::solve_event_transaction(&p, (0., t), &y, &o, pol)
    } else {
        Rodas5p::solve(&p, (0., t), &y, &o)
    };
    let elapsed = begin.elapsed().as_nanos();
    let rhs = rhs.load(Ordering::Relaxed);
    let ec = events.load(Ordering::Relaxed);
    match r {
        Err(e) => {
            json!({"candidate":candidate,"case":name,"rtol":rtol,"pass":false,"expected":expected,"roots":[],"error":null,"reason":format!("{e:?}"),"rhs":rhs,"event_calls":ec,"elapsed_ns":elapsed})
        }
        Ok(s) => {
            assert_eq!(rhs, s.stats.counters.rhs_calls);
            assert_eq!(ec, s.stats.event_evaluations);
            let roots = s.events.iter().map(|e| (e.index, e.t)).collect::<Vec<_>>();
            let matches = roots.len() == expected.len()
                && roots.iter().zip(&expected).all(|(x, y)| x.0 == y.0);
            let error = if matches {
                Some(
                    roots
                        .iter()
                        .zip(&expected)
                        .map(|(x, y)| (x.1 - y.1).abs())
                        .fold(0., f64::max),
                )
            } else {
                None
            };
            let end = *s.t.last().unwrap();
            let tx = s.stats.event_transaction.as_ref();
            let transaction_ok = if pol.is_some() {
                let tx = tx.unwrap();
                tx.inspected_steps == s.stats.accepted_steps
                    && tx.committed_end == end
                    && tx.numerical_end >= end
                    && s.at(end.next_up()).is_none()
            } else {
                true
            };
            let terminal_ok = if terminal {
                s.stats.terminal_event.is_some_and(|i| s.events[i].t == end)
                    && (end - expected.last().unwrap().1).abs() < 2e-5
            } else {
                end == t && s.stats.terminal_event.is_none()
            };
            let pass = s.status == Status::Success
                && error.is_some_and(|x| x <= 2e-5)
                && terminal_ok
                && transaction_ok;
            json!({"candidate":candidate,"case":name,"rtol":rtol,"pass":pass,"expected":expected,"roots":roots,"error":error,
                "rhs":rhs,"event_calls":ec,"attempts":s.stats.attempts,"accepted":s.stats.accepted_steps,"rejected":s.stats.rejected_steps,
                "end":end,"numerical_end":tx.map(|t|t.numerical_end),"elapsed_ns":elapsed,"status":format!("{:?}",s.status),"transaction_ok":transaction_ok})
        }
    }
}
#[test]
#[ignore = "explicit finite measured candidate campaign"]
fn training() {
    let f = std::env::var("VIGILODE_M1_CANDIDATES").unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap();
    let c = doc["candidates"].as_array().unwrap();
    assert_eq!(c.len(), 12);
    for rtol in [1e-6, 1e-8] {
        for name in [
            "harmonic-terminal",
            "harmonic-crossings",
            "decay",
            "stiff-decay",
            "clock-pair",
            "earliest-ties",
        ] {
            println!("CELL {}", run(name, rtol, "legacy", None));
            for p in c {
                println!(
                    "CELL {}",
                    run(name, rtol, p["id"].as_str().unwrap(), Some(policy(p)))
                );
            }
        }
    }
}
#[test]
#[ignore = "frozen selection evaluated on held-out cases"]
fn holdout() {
    let f = std::env::var("VIGILODE_M1_SELECTION").unwrap();
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap();
    let p = &doc["selected"];
    let selected = p["id"].as_str().unwrap();
    let pol = policy(p);
    for rtol in [1e-6, 1e-8] {
        for name in [
            "holdout-pair",
            "holdout-ties",
            "holdout-nonautonomous",
            "holdout-direction",
            "hidden-pair",
        ] {
            for (id, po) in [("legacy", None), (selected, Some(pol))] {
                println!("HOLDOUT {}", run(name, rtol, id, po));
            }
        }
    }
}
