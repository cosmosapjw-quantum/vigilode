#![cfg(feature = "polynomial-events")]
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{AffineFactor, DeclaredPolynomial, EventTerm, retro_dae_init::*};
use vigilode::retro_research::retro_portfolio::Portfolio;
use vigilode::{Atol, InteriorError, Options};
fn p(terms: Vec<EventTerm>) -> DeclaredPolynomial {
    DeclaredPolynomial::new(terms).unwrap()
}
fn tr(c: f64, factors: Vec<AffineFactor>) -> EventTerm {
    EventTerm {
        coefficient: c,
        factors,
    }
}
fn y(i: usize) -> AffineFactor {
    AffineFactor::component(i, 0.)
}
fn t() -> AffineFactor {
    AffineFactor::clock(0.)
}
fn l(i: usize, c: f64) -> EventTerm {
    tr(c, vec![y(i)])
}
fn c(c: f64) -> EventTerm {
    tr(c, vec![])
}
fn analytic() -> DaeSource {
    DaeSource::new(
        vec![p(vec![l(0, -1.)])],
        vec![p(vec![
            l(1, 1.),
            tr(-1., vec![y(0), y(0)]),
            tr(-1., vec![t()]),
        ])],
    )
    .unwrap()
}
fn quad() -> DaeSource {
    DaeSource::new(
        vec![p(vec![l(0, -1.)])],
        vec![p(vec![tr(1., vec![y(1), y(1)]), l(0, -1.)])],
    )
    .unwrap()
}
fn pencil() -> DaeSource {
    DaeSource::new(
        vec![p(vec![l(0, -1.), l(1, 0.125)])],
        vec![p(vec![l(1, 1.), tr(0.125, vec![t(), y(1)]), l(0, -1.)])],
    )
    .unwrap()
}
fn robertson() -> DaeSource {
    DaeSource::new(
        vec![
            p(vec![l(0, -0.04), tr(1e4, vec![y(1), y(2)])]),
            p(vec![
                l(0, 0.04),
                tr(-1e4, vec![y(1), y(2)]),
                tr(-3e7, vec![y(1), y(1)]),
            ]),
        ],
        vec![p(vec![l(0, 1.), l(1, 1.), l(2, 1.), c(-1.)])],
    )
    .unwrap()
}
fn coupled() -> DaeSource {
    DaeSource::new(
        vec![
            p(vec![l(0, -2.), l(2, 0.25)]),
            p(vec![l(1, -1.), l(3, 0.25)]),
        ],
        vec![
            p(vec![l(2, 2.), l(3, 0.125), l(0, -1.)]),
            p(vec![l(3, 2.), l(2, 0.125), l(1, -1.)]),
        ],
    )
    .unwrap()
}
fn opts(tight: bool) -> Options {
    let rt = if tight { 1e-7 } else { 1e-5 };
    Options {
        rtol: rt,
        atol: Atol::Scalar(rt * 0.001),
        dense_output: true,
        interior_error: InteriorError::Off,
        ..Default::default()
    }
}
fn policy() -> EvolutionPolicy {
    EvolutionPolicy {
        init: InitPolicy {
            mode: InitMode::ProjectAlgebraic,
            state_tol: 1e-11,
            residual_tol: 1e-11,
            derivative_tol: 1e-6,
            ..Default::default()
        },
        ..Default::default()
    }
}
struct Fixture {
    name: &'static str,
    group: &'static str,
    src: DaeSource,
    x: Vec<f64>,
    z: Vec<f64>,
    branch: Vec<[f64; 2]>,
    end: f64,
}
fn fixtures(holdout: bool) -> Vec<Fixture> {
    if holdout {
        vec![
            Fixture {
                name: "analytic-holdout",
                group: "analytic",
                src: analytic(),
                x: vec![0.75],
                z: vec![0.6],
                branch: vec![[-0.25, 2.]],
                end: 0.75,
            },
            Fixture {
                name: "quad-holdout",
                group: "nonlinear",
                src: quad(),
                x: vec![1.125],
                z: vec![1.],
                branch: vec![[0.5, 1.5]],
                end: 0.5,
            },
            Fixture {
                name: "pencil-holdout",
                group: "pencil",
                src: pencil(),
                x: vec![0.75],
                z: vec![0.7],
                branch: vec![[0., 2.]],
                end: 0.5,
            },
            Fixture {
                name: "coupled-holdout",
                group: "coupled",
                src: coupled(),
                x: vec![0.75, 0.875],
                z: vec![0.5, 0.5],
                branch: vec![[-0.25, 1.], [-0.25, 1.]],
                end: 0.5,
            },
            Fixture {
                name: "robertson-holdout",
                group: "stiff",
                src: robertson(),
                x: vec![1., 0.],
                z: vec![0.],
                branch: vec![[-0.25, 1.25]],
                end: 0.125,
            },
        ]
    } else {
        vec![
            Fixture {
                name: "analytic",
                group: "analytic",
                src: analytic(),
                x: vec![1.],
                z: vec![0.5],
                branch: vec![[-0.25, 2.]],
                end: 1.,
            },
            Fixture {
                name: "quad",
                group: "nonlinear",
                src: quad(),
                x: vec![1.],
                z: vec![1.],
                branch: vec![[0.5, 1.5]],
                end: 0.5,
            },
            Fixture {
                name: "pencil",
                group: "pencil",
                src: pencil(),
                x: vec![1.],
                z: vec![0.75],
                branch: vec![[0., 2.]],
                end: 1.,
            },
            Fixture {
                name: "coupled",
                group: "coupled",
                src: coupled(),
                x: vec![1., 0.75],
                z: vec![0.5, 0.5],
                branch: vec![[-0.25, 1.], [-0.25, 1.]],
                end: 1.,
            },
            Fixture {
                name: "robertson",
                group: "stiff",
                src: robertson(),
                x: vec![1., 0.],
                z: vec![0.125],
                branch: vec![[-0.25, 1.25]],
                end: 1.,
            },
        ]
    }
}
const ROUTES: [&str; 8] = [
    "Mass",
    "Reduced3",
    "Reduced5",
    "Prepared5",
    "WarpDirect",
    "WarpFolded",
    "Scalar5",
    "Adaptive",
];
fn route(s: &str) -> Option<EvolutionRoute> {
    match s {
        "Mass" => Some(EvolutionRoute::Mass),
        "Reduced3" => Some(EvolutionRoute::Reduced3),
        "Reduced5" => Some(EvolutionRoute::Reduced5),
        "Prepared5" => Some(EvolutionRoute::Prepared5),
        "WarpDirect" => Some(EvolutionRoute::WarpDirect),
        "WarpFolded" => Some(EvolutionRoute::WarpFolded),
        "Scalar5" => Some(EvolutionRoute::Scalar5),
        "Adaptive" => None,
        _ => panic!("route"),
    }
}
fn grid(end: f64) -> Vec<f64> {
    (0..=32).map(|i| end * i as f64 / 32.).collect()
}
fn run(f: &Fixture, tight: bool, r: EvolutionRoute) -> EvolutionRun {
    evolve(
        &f.src,
        &f.x,
        &f.z,
        &f.branch,
        f.end,
        &opts(tight),
        policy(),
        r,
        &grid(f.end),
    )
    .unwrap()
}
fn check(r: &EvolutionRun) {
    assert!(r.pointwise_qualified, "{:?}", r.reason);
    assert_eq!(r.accepted.len(), r.driver().t.len());
    assert_eq!(r.slabs().len(), r.driver().t.len() - 1);
    assert!(
        r.accepted
            .iter()
            .chain(&r.queries)
            .all(|q| q.projected_constraint_upper <= 1e-11 && q.projection.qualified)
    );
    assert!(r.query(r.driver().t.last().unwrap().next_up()).is_err());
    assert!(
        r.accepted
            .iter()
            .chain(&r.queries)
            .all(|q| q.raw[..r.initial.x.len()] == q.state[..r.initial.x.len()])
    );
}
#[test]
fn dense_constraint_projection() {
    let f = fixtures(false).remove(0);
    let r = run(&f, false, EvolutionRoute::Mass);
    check(&r);
    let mut raw = 0.0_f64;
    let mut projected = 0.0_f64;
    for i in 0..=100 {
        let q = r.query(i as f64 / 100.).unwrap();
        raw = raw.max(q.raw_constraint_upper.unwrap());
        projected = projected.max(q.projected_constraint_upper);
    }
    assert!(raw > 1e-12);
    assert!(projected <= 1e-12, "projected residual {projected:e}");
    assert!(r.branch_qualified);
    println!(
        "DENSE {}",
        json!({"raw":raw,"projected":projected,"global":r.global_qualified})
    );
}
#[test]
fn physical_tube_and_schur_reconstruction() {
    for route in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced3,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
    ] {
        let f = fixtures(false).remove(0);
        let r = run(&f, true, route);
        check(&r);
        assert!(r.branch_qualified);
        assert!(
            r.global_qualified,
            "{route:?} {:?}",
            r.slabs()
                .iter()
                .find(|s| s.certificate.as_ref().is_none_or(|c| !c.global_qualified))
        );
        for q in &r.queries {
            let x = (-q.t).exp();
            let z = x * x + q.t;
            let err = (x - q.state[0]).abs().max((z - q.state[1]).abs());
            assert!(err <= q.physical_error_upper.unwrap());
        }
        assert!(
            r.slabs()
                .iter()
                .any(|s| s.certificate.as_ref().unwrap().green_used)
        );
    }
}
#[test]
fn warp_is_real_pencil_and_original_projection() {
    let f = fixtures(false).remove(2);
    for route in [EvolutionRoute::WarpDirect, EvolutionRoute::WarpFolded] {
        let r = run(&f, true, route);
        check(&r);
        assert!(r.branch_qualified);
        assert!(r.warp.is_some());
        assert!(r.work.warp_predictions > 0);
        assert!(r.warp.as_ref().unwrap().quotient_error <= 1e-12);
    }
    let f = fixtures(false).remove(0);
    assert!(
        evolve(
            &f.src,
            &f.x,
            &f.z,
            &f.branch,
            f.end,
            &opts(true),
            policy(),
            EvolutionRoute::WarpDirect,
            &grid(f.end)
        )
        .is_err()
    );
}
#[test]
fn rank_loss_ambiguity_and_false_global_claim() {
    let f = Fixture {
        name: "rank-loss",
        group: "negative",
        src: DaeSource::new(
            vec![p(vec![c(-1.)])],
            vec![p(vec![tr(1., vec![y(1), y(1)]), l(0, -1.)])],
        )
        .unwrap(),
        x: vec![0.125],
        z: vec![0.35],
        branch: vec![[0.01, 0.8]],
        end: 0.25,
    };
    let r = evolve(
        &f.src,
        &f.x,
        &f.z,
        &f.branch,
        f.end,
        &opts(true),
        policy(),
        EvolutionRoute::Scalar5,
        &grid(f.end),
    );
    if let Ok(r) = r {
        assert!(!r.branch_qualified && !r.global_qualified);
    }
    let f = fixtures(false).remove(1);
    assert!(
        evolve(
            &f.src,
            &f.x,
            &f.z,
            &[[-2., 2.]],
            f.end,
            &opts(true),
            policy(),
            EvolutionRoute::Scalar5,
            &grid(f.end)
        )
        .is_err()
    );
    let f = fixtures(false).remove(0);
    let mut po = policy();
    po.tube_radius = 1e-16;
    po.global_tol = 1e-16;
    let r = evolve(
        &f.src,
        &f.x,
        &f.z,
        &f.branch,
        f.end,
        &opts(false),
        po,
        EvolutionRoute::Mass,
        &grid(f.end),
    )
    .unwrap();
    assert!(r.pointwise_qualified);
    assert!(!r.global_qualified);
}
fn source(f: &Fixture) {
    println!(
        "SOURCE {}",
        json!({"case":f.name,"group":f.group,"src":f.src,"x":f.x,"z":f.z,"branch":f.branch,"end":f.end,"grid":grid(f.end)})
    );
}
fn measure(f: &Fixture, c: &str, tight: bool) -> Value {
    let mut portfolio = Portfolio::new(4, 7).unwrap();
    let mut ns = vec![];
    let mut last = None;
    let g = grid(f.end);
    let o = opts(tight);
    let po = policy();
    for i in 0..4 {
        let t = Instant::now();
        let r = if let Some(r) = route(c) {
            evolve(&f.src, &f.x, &f.z, &f.branch, f.end, &o, po, r, &g)
                .map(|v| json!({"run":v,"attempts":[]}))
        } else {
            adaptive(
                &f.src,
                &f.x,
                &f.z,
                &f.branch,
                f.end,
                &o,
                po,
                &g,
                &mut portfolio,
            )
            .map(|r| json!({"run":r.value,"attempts":r.attempts}))
        };
        let dt = t.elapsed().as_nanos() as u64;
        if i > 0 {
            ns.push(dt);
        }
        last = Some(black_box(r));
    }
    ns.sort_unstable();
    match last.unwrap() {
        Ok(v) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"tight":tight,"policy":po,"median_ns":ns[1],"samples_ns":ns,"result":v})
        }
        Err(e) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"tight":tight,"policy":po,"median_ns":ns[1],"samples_ns":ns,"result":null,"reason":e.to_string()})
        }
    }
}
#[test]
#[ignore = "finite candidate campaign"]
fn training() {
    for (i, f) in fixtures(false).iter().enumerate() {
        source(f);
        for tight in [false, true] {
            for j in 0..ROUTES.len() {
                println!("CELL {}", measure(f, ROUTES[(i + j) % ROUTES.len()], tight));
            }
        }
    }
}
#[test]
#[ignore = "frozen selected paths"]
fn holdout() {
    let path = std::env::var("VIGILODE_DAE_EVOLUTION_SELECTION").unwrap();
    let s: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for f in fixtures(true) {
        source(&f);
        for (key, tight) in [("loose", false), ("tight", true)] {
            if let Some(c) = s["groups"][f.group][key]["selected"].as_str() {
                println!("CELL {}", measure(&f, c, tight));
            }
        }
    }
}

#[test]
fn red_options_matched_dense_projection() {
    let s = analytic();
    let o = Options {
        rtol: 1e-5,
        atol: Atol::Scalar(1e-8),
        dense_output: true,
        ..Default::default()
    };
    let po = EvolutionPolicy {
        init: InitPolicy {
            mode: InitMode::Strict,
            ..policy().init
        },
        ..policy()
    };
    let grid = (0..=100).map(|i| i as f64 / 100.).collect::<Vec<_>>();
    let r = evolve(
        &s,
        &[1.],
        &[1.],
        &[[0., 2.]],
        1.,
        &o,
        po,
        EvolutionRoute::Mass,
        &grid,
    )
    .unwrap();
    assert!(r.pointwise_qualified && r.branch_qualified);
    let raw = r
        .queries
        .iter()
        .filter_map(|q| q.raw_constraint_upper)
        .fold(0., f64::max);
    let projected = r
        .queries
        .iter()
        .map(|q| q.projected_constraint_upper)
        .fold(0., f64::max);
    assert!(raw > 1e-12);
    assert!(projected <= 1e-12);
    println!(
        "MATCHED_RED {}",
        json!({"raw_constraint":raw,"projected_constraint":projected,"points":101,"global_qualified":r.global_qualified,"physical_bound":r.max_physical_error})
    );
}
#[test]
#[ignore = "separate post-holdout fallback diagnostic"]
fn recovery() {
    for f in fixtures(true) {
        source(&f);
        for tight in [false, true] {
            println!("CELL {}", measure(&f, "Adaptive", tight));
        }
    }
}
