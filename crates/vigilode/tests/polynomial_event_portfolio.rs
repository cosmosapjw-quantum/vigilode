#![cfg(feature = "polynomial-events")]
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode::polynomial_events::*;
use vigilode::retro_research::retro_portfolio::Portfolio;
use vigilode::{Atol, Direction, Options, Problem, Rodas5p};
const TOL: f64 = 1. / 1099511627776.;
fn factor(fs: Vec<AffineFactor>) -> DeclaredPolynomial {
    DeclaredPolynomial::new(vec![EventTerm {
        coefficient: 1.,
        factors: fs,
    }])
    .unwrap()
}
fn clocks(r: &[f64]) -> DeclaredPolynomial {
    factor(r.iter().map(|&x| AffineFactor::clock(x)).collect())
}
fn policy(s: &str) -> Option<RootRoute> {
    match s {
        "Sturm" => Some(RootRoute::Sturm),
        "NativeBernstein" => Some(RootRoute::NativeBernstein),
        "DictionaryBernstein" => Some(RootRoute::DictionaryBernstein),
        "Companion" => Some(RootRoute::Companion),
        "DictionaryCompanion" => Some(RootRoute::DictionaryCompanion),
        "Adaptive" => None,
        _ => panic!("unknown route"),
    }
}
fn pow_fixture(rs: &[f64]) -> RepresentedPolynomial {
    RepresentedPolynomial::clock_only(&clocks(rs), 0., 1.).unwrap()
}
fn load(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(std::env::var(name).unwrap()).unwrap()).unwrap()
}
fn kernel_cases(holdout: bool) -> Vec<(&'static str, &'static str, RepresentedPolynomial)> {
    if holdout {
        vec![
            (
                "near-cancellation",
                "kernel-screen",
                RepresentedPolynomial::from_power(&[1., -2., 1. + 2f64.powi(-45)]).unwrap(),
            ),
            (
                "cluster-holdout",
                "kernel-roots",
                pow_fixture(&[0.5 - 2f64.powi(-41), 0.5 + 2f64.powi(-41), 0.875]),
            ),
            (
                "endpoint-holdout",
                "kernel-roots",
                pow_fixture(&[0., 0., 1., 1.]),
            ),
            (
                "high-degree-holdout",
                "kernel-high",
                RepresentedPolynomial::from_power(&[
                    -0.25, 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1.,
                ])
                .unwrap(),
            ),
        ]
    } else {
        vec![
            (
                "positive-quadratic",
                "kernel-screen",
                RepresentedPolynomial::from_power(&[1., -1., 1.]).unwrap(),
            ),
            (
                "positive-cubic",
                "kernel-screen",
                RepresentedPolynomial::from_power(&[1., -0.25, 0., 0.125]).unwrap(),
            ),
            (
                "positive-degree6",
                "kernel-screen",
                RepresentedPolynomial::from_power(&[1., 0., -0.5, 0., 0.125, 0., 1.]).unwrap(),
            ),
            ("hidden", "kernel-roots", pow_fixture(&[0.431, 0.432])),
            ("touch", "kernel-roots", pow_fixture(&[0.331, 0.331])),
            (
                "four",
                "kernel-roots",
                pow_fixture(&[0.125, 0.25, 0.625, 0.875]),
            ),
            (
                "triple",
                "kernel-roots",
                pow_fixture(&[0.125, 0.125, 0.125, 0.875]),
            ),
            (
                "degree12",
                "kernel-high",
                RepresentedPolynomial::from_power(&[
                    -0.5, 1., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1.,
                ])
                .unwrap(),
            ),
            (
                "degree16-positive",
                "kernel-high",
                RepresentedPolynomial::from_power(&[
                    1., 0., 0., 0., 0., 0., 0., 0., -0.25, 0., 0., 0., 0., 0., 0., 0., 1.,
                ])
                .unwrap(),
            ),
        ]
    }
}
fn native_case(
    name: &str,
) -> (
    Problem,
    Vec<f64>,
    f64,
    Options,
    PolynomialEventSpec,
    Vec<f64>,
) {
    let mut o = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        dense_output: true,
        ..Options::default()
    };
    let (p, y, t, poly, terminal, expected) = match name {
        "clock-pair" | "clock-holdout" => {
            o.h0 = 0.75;
            let rs = if name == "clock-pair" {
                vec![0.431, 0.432]
            } else {
                vec![0.213, 0.213, 0.719, 0.719]
            };
            let expected = if name == "clock-pair" {
                rs.clone()
            } else {
                vec![0.213, 0.719]
            };
            (
                Problem::new(1, |_, _, f| {
                    f[0] = 0.;
                    Ok(())
                })
                .autonomous(),
                vec![0.],
                1.,
                clocks(&rs),
                false,
                expected,
            )
        }
        "harmonic-touch" | "projection-holdout" => {
            let p = Problem::new(2, |_, y, f| {
                f[0] = y[1];
                f[1] = -y[0];
                Ok(())
            })
            .autonomous();
            let f = if name == "harmonic-touch" {
                vec![
                    AffineFactor::component(0, 0.5),
                    AffineFactor::component(0, 0.5),
                ]
            } else {
                vec![AffineFactor {
                    constant: 0.,
                    time: 0.,
                    state: vec![(0, 1.), (1, 1.)],
                }]
            };
            (
                p,
                vec![1., 0.],
                4.,
                factor(f),
                true,
                vec![if name == "harmonic-touch" {
                    std::f64::consts::FRAC_PI_3
                } else {
                    std::f64::consts::FRAC_PI_4
                }],
            )
        }
        "decay" => (
            Problem::new(1, |_, y, f| {
                f[0] = -y[0];
                Ok(())
            })
            .autonomous(),
            vec![1.],
            3.,
            factor(vec![AffineFactor::component(0, 0.5)]),
            true,
            vec![2_f64.ln()],
        ),
        _ => panic!("native fixture"),
    };
    (
        p,
        y,
        t,
        o,
        PolynomialEventSpec {
            polynomial: poly,
            direction: Direction::Either,
            terminal,
        },
        expected,
    )
}
fn kernel_row(name: &str, group: &str, p: &RepresentedPolynomial, route: &str) -> Value {
    let mut selector = Portfolio::new(4, 5).unwrap();
    let mut ns = vec![];
    let mut last = None;
    // Actual runtime, three measured calls after one warmup. Adaptive history
    // intentionally retained across repeated queries of the same polynomial.
    for i in 0..4 {
        let t = Instant::now();
        let r = if let Some(mode) = policy(route) {
            p.isolate(mode, TOL).map(|r| (r, vec![]))
        } else {
            p.adaptive_isolate(&mut selector, TOL).and_then(|s| {
                s.value
                    .map(|r| (r, s.attempts))
                    .ok_or_else(|| rodas5p_core::CoreError::InvalidInput("adaptive refused".into()))
            })
        };
        let dt = t.elapsed().as_nanos() as u64;
        if i > 0 {
            ns.push(dt);
        }
        last = Some(black_box(r));
    }
    ns.sort_unstable();
    let result = last.unwrap();
    match result {
        Ok((r, a)) => {
            json!({"kind":"kernel","name":name,"group":group,"candidate":route,"median_ns":ns[1],"samples_ns":ns,"coefficients":p.exact_coefficients(),"clock":p.exact_clock(),"report":r,"attempts":a,"ok":true})
        }
        Err(e) => {
            json!({"kind":"kernel","name":name,"group":group,"candidate":route,"median_ns":ns[1],"samples_ns":ns,"coefficients":p.exact_coefficients(),"clock":p.exact_clock(),"ok":false,"reason":e.to_string()})
        }
    }
}
fn native_row(name: &str, route: &str) -> Value {
    let (p, y, t, o, spec, expected) = native_case(name);
    let mut ns = vec![];
    let mut last = None;
    // Capture coefficient evidence on every run; not a root-kernel-only time.
    for i in 0..4 {
        let t0 = Instant::now();
        let r = Rodas5p::solve_polynomial_event(
            &p,
            (0., t),
            &y,
            &o,
            &spec,
            PolynomialEventPolicy {
                route: policy(route),
                capture_all: true,
                ..Default::default()
            },
        );
        let dt = t0.elapsed().as_nanos() as u64;
        if i > 0 {
            ns.push(dt);
        }
        last = Some(black_box(r));
    }
    ns.sort_unstable();
    match last.unwrap() {
        Ok(r) => {
            json!({"kind":"native","name":name,"group":"native","candidate":route,"median_ns":ns[1],"samples_ns":ns,"expected":expected,"events":r.solution.events,"status":format!("{:?}",r.solution.status),"ok":r.solution.is_success(),"rhs":r.solution.stats.counters.rhs_calls,"accepted":r.solution.stats.accepted_steps,"terminal_event":r.solution.stats.terminal_event,"trace":r.trace})
        }
        Err(e) => {
            json!({"kind":"native","name":name,"group":"native","candidate":route,"median_ns":ns[1],"samples_ns":ns,"expected":expected,"ok":false,"reason":e.to_string()})
        }
    }
}
#[test]
#[ignore = "explicit finite campaign, exact oracle external"]
fn training() {
    let doc = load("VIGILODE_POLY_CANDIDATES");
    let candidates = doc["routes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    for (i, (name, g, p)) in kernel_cases(false).iter().enumerate() {
        for j in 0..candidates.len() {
            println!(
                "CELL {}",
                kernel_row(name, g, p, candidates[(i + j) % candidates.len()])
            );
        }
    }
    for (i, name) in ["clock-pair", "harmonic-touch", "decay"].iter().enumerate() {
        for j in 0..candidates.len() {
            println!(
                "CELL {}",
                native_row(name, candidates[(i + j) % candidates.len()])
            );
        }
    }
}
#[test]
#[ignore = "frozen selected routes only"]
fn holdout() {
    let doc = load("VIGILODE_POLY_SELECTION");
    for (name, g, p) in kernel_cases(true) {
        let route = doc["groups"][g]["selected"].as_str().unwrap();
        println!("CELL {}", kernel_row(name, g, &p, route));
    }
    let route = doc["groups"]["native"]["selected"].as_str().unwrap();
    for name in ["clock-holdout", "projection-holdout"] {
        println!("CELL {}", native_row(name, route));
    }
}
