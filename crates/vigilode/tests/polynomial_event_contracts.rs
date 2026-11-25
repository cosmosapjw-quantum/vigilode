#![cfg(feature = "polynomial-events")]
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::polynomial_events::*;
use vigilode::{Atol, Direction, FailureReason, InteriorError, Options, Problem, Rodas5p, Status};
const TOL: f64 = 1. / 1099511627776.;
const ROUTES: [RootRoute; 5] = [
    RootRoute::Sturm,
    RootRoute::NativeBernstein,
    RootRoute::DictionaryBernstein,
    RootRoute::Companion,
    RootRoute::DictionaryCompanion,
];
fn factors(f: Vec<AffineFactor>) -> DeclaredPolynomial {
    DeclaredPolynomial::new(vec![EventTerm {
        coefficient: 1.,
        factors: f,
    }])
    .unwrap()
}
fn clocks(r: &[f64]) -> DeclaredPolynomial {
    factors(r.iter().map(|&x| AffineFactor::clock(x)).collect())
}
fn options() -> Options {
    Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        dense_output: true,
        ..Options::default()
    }
}
fn clock() -> Problem {
    Problem::new(1, |_, _, f| {
        f[0] = 0.;
        Ok(())
    })
    .autonomous()
}
fn oscillator() -> Problem {
    Problem::new(2, |_, y, f| {
        f[0] = y[1];
        f[1] = -y[0];
        Ok(())
    })
    .autonomous()
}
fn emit(p: &RepresentedPolynomial, r: &RootReport, name: &str) {
    println!(
        "EXACT {}",
        json!({"name":name,"coefficients":p.exact_coefficients(),"clock":p.exact_clock(),"report":r})
    );
}
#[test]
fn exact_multiplicities_and_boundary_convention() {
    let cases = [
        (
            "hidden",
            vec![0.431, 0.432],
            vec![(0.431, 1, RootKind::Down), (0.432, 1, RootKind::Up)],
        ),
        (
            "touch",
            vec![0.331, 0.331],
            vec![(0.331, 2, RootKind::Touch)],
        ),
        (
            "endpoints",
            vec![0., 0., 0.5, 1.],
            vec![(0.5, 1, RootKind::Down), (1., 1, RootKind::Up)],
        ),
        (
            "triple",
            vec![0.125, 0.125, 0.125, 0.875],
            vec![(0.125, 3, RootKind::Down), (0.875, 1, RootKind::Up)],
        ),
        (
            "cluster",
            vec![0.5, 0.5 + 2f64.powi(-48)],
            vec![
                (0.5, 1, RootKind::Down),
                (0.5 + 2f64.powi(-48), 1, RootKind::Up),
            ],
        ),
    ];
    for (name, rs, expected) in cases {
        let p = RepresentedPolynomial::clock_only(&clocks(&rs), 0., 1.).unwrap();
        for route in ROUTES {
            let r = p.isolate(route, TOL).unwrap();
            assert_eq!(r.roots.len(), expected.len(), "{name} {route:?}");
            for (a, b) in r.roots.iter().zip(&expected) {
                assert!(a.time[0] <= b.0 && b.0 <= a.time[1], "{name} {:?}", a);
                assert_eq!((a.multiplicity, a.kind), (b.1, b.2));
            }
            assert!(r.max_theta_width <= TOL);
            emit(&p, &r, name);
        }
    }
}
#[test]
fn degree16_and_sign_screens_and_refusals() {
    let mut c = vec![1.];
    for _ in 0..16 {
        let mut out = vec![0.; c.len() + 1];
        for (i, &x) in c.iter().enumerate() {
            out[i] -= 0.5 * x;
            out[i + 1] += x;
        }
        c = out;
    }
    let p = RepresentedPolynomial::from_power(&c).unwrap();
    for route in ROUTES {
        let r = p.isolate(route, TOL).unwrap();
        assert_eq!(r.roots.len(), 1);
        assert_eq!(r.roots[0].multiplicity, 16);
        assert_eq!(r.roots[0].kind, RootKind::Touch);
        emit(&p, &r, "degree16");
    }
    let p = RepresentedPolynomial::from_power(&[1., -1., 1.]).unwrap();
    for route in ROUTES {
        let r = p.isolate(route, TOL).unwrap();
        assert!(r.roots.is_empty());
        emit(&p, &r, "positive");
    }
    let d = p.isolate(RootRoute::DictionaryBernstein, TOL).unwrap();
    assert_eq!(d.work.dictionary_uses, 1);
    assert_eq!(d.work.screened_intervals, 1);
    assert_eq!(d.work.sturm_chains, 0);
    assert!(d.work.conditioned_horner_uses > 0);
    let b = p.isolate(RootRoute::NativeBernstein, TOL).unwrap();
    assert_eq!(b.work.native_bernstein_uses, 1);
    assert!(b.work.screened_intervals > 0);
    let zero = RepresentedPolynomial::from_power(&[0., 0.]).unwrap();
    for route in ROUTES {
        assert!(zero.isolate(route, TOL).is_err());
    }
    assert!(RepresentedPolynomial::from_power(&[f64::NAN]).is_err());
    assert!(RepresentedPolynomial::from_power(&[1.; 18]).is_err());
    assert!(p.isolate(RootRoute::Sturm, 0.).is_err());
}
#[test]
fn actual_adaptive_portfolio_recomputes_original_count() {
    use vigilode::retro_research::retro_portfolio::Portfolio;
    let a = RepresentedPolynomial::clock_only(&clocks(&[0.431, 0.432]), 0., 1.).unwrap();
    let b = RepresentedPolynomial::from_power(&[1., -1., 1.]).unwrap();
    let mut selector = Portfolio::new(1, 5).unwrap();
    let mut routes = std::collections::BTreeSet::new();
    for _ in 0..5 {
        let s = a.adaptive_isolate(&mut selector, TOL).unwrap();
        let r = s.value.unwrap();
        assert_eq!(r.roots.len(), 2);
        routes.insert(s.selected.unwrap());
    }
    assert_eq!(routes.len(), 5);
    let r = b
        .adaptive_isolate(&mut selector, TOL)
        .unwrap()
        .value
        .unwrap();
    assert!(r.roots.is_empty());
}
fn assert_trace(r: &PolynomialEventRun) {
    assert!(
        r.solution.is_success(),
        "{:?} {:?}",
        r.solution.status,
        r.trace.failure
    );
    assert_eq!(r.trace.inspected_steps, r.solution.stats.accepted_steps);
    assert_eq!(r.trace.committed_end, *r.solution.t.last().unwrap());
    assert_eq!(r.solution.stats.event_evaluations, 0);
    assert!(r.solution.at(r.trace.committed_end.next_up()).is_none());
    assert!(r.trace.numerical_end >= r.trace.committed_end);
    assert!(r.solution.events.windows(2).all(|w| w[0].t < w[1].t));
}
fn run(
    p: &Problem,
    span: (f64, f64),
    y: &[f64],
    o: &Options,
    s: PolynomialEventSpec,
    route: Option<RootRoute>,
) -> PolynomialEventRun {
    Rodas5p::solve_polynomial_event(
        p,
        span,
        y,
        o,
        &s,
        PolynomialEventPolicy {
            route,
            capture_all: true,
            ..Default::default()
        },
    )
    .unwrap()
}
#[test]
fn hidden_pair_and_touch_on_real_driver() {
    let o = Options {
        h0: 0.75,
        ..options()
    };
    for route in ROUTES.into_iter().map(Some).chain([None]) {
        let r = run(
            &clock(),
            (0., 1.),
            &[0.],
            &o,
            PolynomialEventSpec {
                polynomial: clocks(&[0.431, 0.432]),
                direction: Direction::Either,
                terminal: false,
            },
            route,
        );
        assert_trace(&r);
        assert_eq!(r.solution.events.len(), 2);
        assert_eq!(r.solution.events[0].direction, Direction::Down);
        assert_eq!(r.solution.events[1].direction, Direction::Up);
        for (e, x) in r.solution.events.iter().zip([0.431, 0.432]) {
            assert!((e.t - x).abs() < TOL);
        }
        let r = run(
            &clock(),
            (0., 1.),
            &[0.],
            &o,
            PolynomialEventSpec {
                polynomial: clocks(&[0.331, 0.331]),
                direction: Direction::Touch,
                terminal: true,
            },
            route,
        );
        assert_trace(&r);
        assert_eq!(r.solution.events.len(), 1);
        assert_eq!(r.solution.stats.terminal_event, Some(0));
        assert_eq!(r.solution.events[0].direction, Direction::Touch);
        assert!((r.trace.committed_end - 0.331).abs() < TOL);
        assert_eq!(r.trace.numerical_end, 0.75);
        assert_eq!(r.solution.stats.accepted_steps, 1);
    }
}
#[test]
fn native_state_and_multicomponent_composition() {
    let o = options();
    for route in [
        Some(RootRoute::NativeBernstein),
        Some(RootRoute::DictionaryCompanion),
        None,
    ] {
        let r = run(
            &oscillator(),
            (0., 4.),
            &[1., 0.],
            &o,
            PolynomialEventSpec {
                polynomial: factors(vec![
                    AffineFactor::component(0, 0.5),
                    AffineFactor::component(0, 0.5),
                ]),
                direction: Direction::Touch,
                terminal: true,
            },
            route,
        );
        assert_trace(&r);
        assert_eq!(r.solution.events.len(), 1);
        let e = &r.solution.events[0];
        assert_eq!(e.direction, Direction::Touch);
        assert!((e.t - std::f64::consts::FRAC_PI_3).abs() < 2e-7);
        assert!((e.y[0] - 0.5).abs() < 2e-7);
        assert_eq!(
            r.trace.slabs.last().unwrap().report.roots[0].multiplicity,
            2
        );
        println!(
            "NATIVE {}",
            json!({"case":"state-touch","route":route,"trace":r.trace})
        );
    }
    // Linear projection of both state components: cos(t)-sin(t)=0.
    let r = run(
        &oscillator(),
        (0., 4.),
        &[1., 0.],
        &o,
        PolynomialEventSpec {
            polynomial: factors(vec![AffineFactor {
                constant: 0.,
                time: 0.,
                state: vec![(0, 1.), (1, 1.)],
            }]),
            direction: Direction::Down,
            terminal: true,
        },
        Some(RootRoute::DictionaryBernstein),
    );
    assert_trace(&r);
    assert!((r.trace.committed_end - std::f64::consts::FRAC_PI_4).abs() < 2e-7);
    println!("NATIVE {}", json!({"case":"two-state","trace":r.trace}));
}
#[test]
fn zero_and_invalid_declarations_fail_closed() {
    let p = clock();
    let o = Options {
        h0: 0.5,
        ..options()
    };
    let r = run(
        &p,
        (0., 1.),
        &[0.],
        &o,
        PolynomialEventSpec {
            polynomial: factors(vec![AffineFactor::component(0, 0.)]),
            direction: Direction::Either,
            terminal: true,
        },
        None,
    );
    assert!(matches!(
        r.solution.status,
        Status::Failed(FailureReason::PolynomialEventUnresolved { .. })
    ));
    assert_eq!(r.solution.t, vec![0.]);
    assert_eq!(r.trace.inspected_steps, 1);
    assert_eq!(r.trace.numerical_end, 0.5);
    assert!(r.solution.events.is_empty());
    assert!(r.solution.stats.counters.rhs_calls > 0);
    let n = Arc::new(AtomicU64::new(0));
    let c = n.clone();
    let p = Problem::new(1, move |_, _, f| {
        c.fetch_add(1, Ordering::Relaxed);
        f[0] = 0.;
        Ok(())
    })
    .autonomous();
    let spec = PolynomialEventSpec {
        polynomial: factors(vec![AffineFactor::component(1, 0.)]),
        direction: Direction::Either,
        terminal: true,
    };
    assert!(
        Rodas5p::solve_polynomial_event(&p, (0., 1.), &[0.], &o, &spec, Default::default())
            .is_err()
    );
    let spec = PolynomialEventSpec {
        polynomial: clocks(&[0.5]),
        direction: Direction::Either,
        terminal: true,
    };
    for bad in [
        Options {
            dense_output: false,
            ..o.clone()
        },
        Options {
            interior_error: InteriorError::Enforce,
            ..o.clone()
        },
    ] {
        assert!(
            Rodas5p::solve_polynomial_event(&p, (0., 1.), &[0.], &bad, &spec, Default::default())
                .is_err()
        );
    }
    assert!(
        Rodas5p::solve_polynomial_event(
            &p.clone().event(|t, _| t, Direction::Either),
            (0., 1.),
            &[0.],
            &o,
            &spec,
            Default::default()
        )
        .is_err()
    );
    assert!(
        Rodas5p::solve_polynomial_event(
            &p.clone().mass(vec![1.]),
            (0., 1.),
            &[0.],
            &o,
            &spec,
            Default::default()
        )
        .is_err()
    );
    assert_eq!(n.load(Ordering::Relaxed), 0);
}
#[test]
fn represented_clock_overlap_is_explicit() {
    let a = 1e16;
    let b = a + 8.;
    let p = factors(vec![
        AffineFactor {
            constant: -30000000000000004.,
            time: 3.,
            state: vec![],
        },
        AffineFactor {
            constant: -50000000000000008.,
            time: 5.,
            state: vec![],
        },
    ]);
    let q = RepresentedPolynomial::clock_only(&p, a, b).unwrap();
    let v = q.isolate(RootRoute::Sturm, TOL).unwrap();
    assert_eq!(v.roots.len(), 2);
    assert!(v.roots[0].time[1] >= v.roots[1].time[0]);
    emit(&q, &v, "clock-overlap");
    let r = run(
        &clock(),
        (a, b),
        &[0.],
        &Options {
            h0: 8.,
            ..options()
        },
        PolynomialEventSpec {
            polynomial: p,
            direction: Direction::Either,
            terminal: false,
        },
        Some(RootRoute::Sturm),
    );
    assert!(matches!(
        r.solution.status,
        Status::Failed(FailureReason::PolynomialEventUnresolved { .. })
    ));
    assert_eq!(r.solution.t, vec![a]);
    assert!(r.solution.events.is_empty());
}

#[test]
fn actual_robertson_packet_export_for_independent_composition() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastInteriorControl, integrate_rodas5p_fast_dense_observed,
        robertson_problem,
    };
    let (p, y) = robertson_problem().unwrap();
    let run = integrate_rodas5p_fast_dense_observed(
        &p,
        (0., 0.0001),
        &y,
        &AdaptiveStepConfig {
            initial_step: 1e-6,
            rtol: 1e-8,
            atol: 1e-11,
            ..Default::default()
        },
        FastInteriorControl::WFilteredEnforce,
    )
    .unwrap();
    assert!(run.fast.observed.success);
    let seg = run.segments;
    let coeff = rodas5p_core::rodas5p_coefficients().unwrap();
    let h = (0..3)
        .map(|i| (0..8).map(|j| coeff.dense_h[(i, j)]).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let event = DeclaredPolynomial::new(vec![
        EventTerm {
            coefficient: 0.125,
            factors: vec![
                AffineFactor {
                    constant: -0.25,
                    time: 0.5,
                    state: vec![(0, 1.), (1, -2.)],
                },
                AffineFactor {
                    constant: 0.03125,
                    time: -0.25,
                    state: vec![(2, 4.)],
                },
            ],
        },
        EventTerm {
            coefficient: -0.125,
            factors: vec![
                AffineFactor::clock(0.00005),
                AffineFactor::component(0, 0.5),
            ],
        },
    ])
    .unwrap();
    for k in [0, seg.len() / 2, seg.len() - 1] {
        let q = RepresentedPolynomial::from_segment(&seg, k, &event).unwrap();
        let r = q.isolate(RootRoute::DictionaryCompanion, TOL).unwrap();
        println!(
            "PACKET {}",
            json!({"index":k,"n":seg.dimension(),"times":[seg.t_old(k),seg.t_new(k)],"y0":seg.state(k),"y1":seg.state(k+1),"H":h,"stage_rows":seg.stage_rows(k),"declaration":event,"coefficients":q.exact_coefficients(),"clock":q.exact_clock(),"report":r})
        );
    }
}
