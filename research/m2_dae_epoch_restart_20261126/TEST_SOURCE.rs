#![cfg(feature = "polynomial-events")]
use serde_json::json;
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn term(c: f64, factors: Vec<F>) -> T {
    T {
        coefficient: c,
        factors,
    }
}
fn p(v: Vec<T>) -> P {
    P::new(v).unwrap()
}
fn y(i: usize) -> F {
    F::component(i, 0.)
}
fn t() -> F {
    F::clock(0.)
}
fn lin(i: usize, c: f64) -> T {
    term(c, vec![y(i)])
}
fn source(kind: &str) -> DaeSource {
    match kind {
        "analytic" => DaeSource::new(
            vec![p(vec![lin(0, -1.)])],
            vec![p(vec![
                lin(1, 1.),
                term(-1., vec![y(0), y(0)]),
                term(-1., vec![t()]),
            ])],
        ),
        "pencil" => DaeSource::new(
            vec![p(vec![lin(0, -1.), lin(1, 0.125)])],
            vec![p(vec![
                lin(1, 1.),
                term(0.125, vec![t(), y(1)]),
                lin(0, -1.),
            ])],
        ),
        "forcing" => DaeSource::new(
            vec![p(vec![term(1., vec![t()])])],
            vec![p(vec![lin(1, 1.), lin(0, -1.), term(-1., vec![t()])])],
        ),
        _ => panic!("fixture"),
    }
    .unwrap()
}
fn opts() -> Options {
    Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        dense_output: true,
        interior_error: InteriorError::Off,
        ..Default::default()
    }
}
fn policy(green: bool) -> EvolutionPolicy {
    EvolutionPolicy {
        init: InitPolicy {
            mode: InitMode::ProjectAlgebraic,
            state_tol: 1e-11,
            residual_tol: 1e-11,
            derivative_tol: 1e-6,
            ..Default::default()
        },
        tube_radius: 0.005,
        global_tol: 0.005,
        green,
    }
}
fn exact(kind: &str, s: f64, t: f64, x0: f64) -> [f64; 2] {
    let x = match kind {
        "analytic" => x0 * (-(t - s)).exp(),
        "pencil" => x0 * (-(t - s)).exp() * (1. + t / 8.) / (1. + s / 8.),
        "forcing" => x0 + 0.5 * (t * t - s * s),
        _ => unreachable!(),
    };
    let z = match kind {
        "analytic" => x * x + t,
        "pencil" => x / (1. + t / 8.),
        "forcing" => x + t,
        _ => unreachable!(),
    };
    [x, z]
}
fn grid(s: f64, e: f64) -> Vec<f64> {
    (0..=16).map(|i| s + (e - s) * i as f64 / 16.).collect()
}
fn run(
    kind: &str,
    epoch: EvolutionEpoch,
    route: EvolutionRoute,
    green: bool,
) -> rodas5p_core::CoreResult<EvolutionRun> {
    let s = epoch.start;
    let e = epoch.end;
    let z = exact(kind, s, s, 1.)[1];
    evolve_epoch(
        &source(kind),
        &[1.],
        &[z],
        &[[-4., 5.]],
        epoch,
        &opts(),
        policy(green),
        route,
        &grid(s, e),
    )
}
fn verify(kind: &str, r: &EvolutionRun, eps: f64) {
    assert!(
        r.pointwise_qualified && r.branch_qualified && r.global_qualified,
        "{kind} {:?} {:?}",
        r.route,
        r.reason
    );
    let s = r.driver().t[0];
    assert_eq!(r.initial.t, s);
    assert_eq!(r.slabs()[0].certificate.as_ref().unwrap().incoming, eps);
    for q in r.accepted.iter().chain(&r.queries) {
        for sign in [-1., 0., 1.] {
            let z = exact(kind, s, q.t, 1. + sign * eps);
            let err = (q.state[0] - z[0]).abs().max((q.state[1] - z[1]).abs());
            assert!(
                err <= q.physical_error_upper.unwrap() + 1e-14,
                "{kind} {:?} t={} err={} bound={:?}",
                r.route,
                q.t,
                err,
                q.physical_error_upper
            );
        }
    }
    assert!(r.query(s - 0.125).is_err());
}
#[test]
fn physical_epoch_and_carried_radius() {
    let ep = EvolutionEpoch {
        start: 0.5,
        end: 1.,
        incoming_x_error: Some(2f64.powi(-12)),
    };
    for route in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced3,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
    ] {
        let r = run("analytic", ep, route, true).unwrap();
        verify("analytic", &r, ep.incoming_x_error.unwrap());
        assert_eq!(r.driver().t.last(), Some(&ep.end));
    }
}
#[test]
fn reanchored_warp_pencil() {
    for s in [-0.5, 0.5, 1.] {
        let ep = EvolutionEpoch {
            start: s,
            end: s + 0.5,
            incoming_x_error: Some(2f64.powi(-12)),
        };
        for route in [EvolutionRoute::WarpDirect, EvolutionRoute::WarpFolded] {
            let r = run("pencil", ep, route, true).unwrap();
            verify("pencil", &r, ep.incoming_x_error.unwrap());
            let w = r.warp.as_ref().unwrap();
            assert_eq!(w.start, s);
            assert_eq!(w.end, ep.end);
            assert!(r.work.warp_predictions > 0);
        }
    }
}
#[test]
fn unknown_and_overlarge_never_certified() {
    for rad in [None, Some(0.01)] {
        let ep = EvolutionEpoch {
            start: 0.5,
            end: 1.,
            incoming_x_error: rad,
        };
        let r = run("analytic", ep, EvolutionRoute::Mass, true).unwrap();
        assert!(r.pointwise_qualified);
        assert!(!r.global_qualified);
        assert!(r.max_physical_error.is_none());
        assert!(r.queries.iter().all(|q| q.physical_error_upper.is_none()));
    }
    for rad in [Some(-1.), Some(f64::NAN), Some(f64::INFINITY)] {
        assert!(
            run(
                "analytic",
                EvolutionEpoch {
                    start: 0.5,
                    end: 1.,
                    incoming_x_error: rad
                },
                EvolutionRoute::Mass,
                true
            )
            .is_err()
        );
    }
    assert!(
        run(
            "analytic",
            EvolutionEpoch {
                start: 0.5,
                end: 0.5,
                incoming_x_error: Some(0.)
            },
            EvolutionRoute::Mass,
            true
        )
        .is_err()
    );
    assert!(
        evolve_epoch(
            &source("forcing"),
            &[1.],
            &[1.5],
            &[[-4., 5.]],
            EvolutionEpoch {
                start: 0.5,
                end: 1.,
                incoming_x_error: Some(0.)
            },
            &opts(),
            policy(true),
            EvolutionRoute::Mass,
            &[0., 1.]
        )
        .is_err()
    );
}
#[test]
fn zero_origin_legacy_equivalence() {
    let src = source("analytic");
    let g = grid(0., 0.5);
    let a = evolve(
        &src,
        &[1.],
        &[1.],
        &[[-4., 5.]],
        0.5,
        &opts(),
        policy(true),
        EvolutionRoute::Mass,
        &g,
    )
    .unwrap();
    let b = evolve_epoch(
        &src,
        &[1.],
        &[1.],
        &[[-4., 5.]],
        EvolutionEpoch {
            start: 0.,
            end: 0.5,
            incoming_x_error: Some(0.),
        },
        &opts(),
        policy(true),
        EvolutionRoute::Mass,
        &g,
    )
    .unwrap();
    assert_eq!(a.driver().t, b.driver().t);
    assert_eq!(a.driver().y, b.driver().y);
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
}
#[test]
fn forcing_uses_absolute_time() {
    for s in [-0.5, 0.5, 1.] {
        let ep = EvolutionEpoch {
            start: s,
            end: s + 0.5,
            incoming_x_error: Some(2f64.powi(-12)),
        };
        for route in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            let r = run("forcing", ep, route, true).unwrap();
            verify("forcing", &r, ep.incoming_x_error.unwrap());
        }
    }
}
#[test]
#[ignore = "candidate campaign"]
fn training() {
    let routes = [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced3,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::WarpDirect,
        EvolutionRoute::WarpFolded,
    ];
    for kind in ["analytic", "pencil", "forcing"] {
        for start in [-0.5, 0.5, 1.] {
            for green in [false, true] {
                for route in routes {
                    let ep = EvolutionEpoch {
                        start,
                        end: start + 0.5,
                        incoming_x_error: Some(2f64.powi(-12)),
                    };
                    match run(kind, ep, route, green) {
                        Ok(r) => {
                            if r.global_qualified {
                                verify(kind, &r, ep.incoming_x_error.unwrap());
                            }
                            let max_err = r
                                .queries
                                .iter()
                                .flat_map(|q| {
                                    [-1., 0., 1.].map(|sign| {
                                        let z = exact(
                                            kind,
                                            start,
                                            q.t,
                                            1. + sign * ep.incoming_x_error.unwrap(),
                                        );
                                        (q.state[0] - z[0]).abs().max((q.state[1] - z[1]).abs())
                                    })
                                })
                                .fold(0., f64::max);
                            println!(
                                "CELL {}",
                                json!({"kind":kind,"start":start,"green":green,"route":route,"global":r.global_qualified,"branch":r.branch_qualified,"pointwise":r.pointwise_qualified,"bound":r.max_physical_error,"sample_error":max_err,"driver_rhs":r.driver().stats.counters.rhs_calls,"work":r.work,"queries":r.queries,"reason":r.reason})
                            );
                        }
                        Err(e) => println!(
                            "CELL {}",
                            json!({"kind":kind,"start":start,"green":green,"route":route,"global":false,"error":e.to_string()})
                        ),
                    }
                }
            }
        }
    }
}

/// Previous R4 Green moment construction is exercised as an actual
/// incoming radius to the new native epoch API. Physical root bracket
/// supplied by the fixture, NOT found by this test.
#[test]
fn green_moment_reset_to_native_epoch() {
    use rodas5p_core::directed::{Interval as I, sub_up};
    use vigilode::retro_research::{retro_dictionary::RealMode, retro_green::RetroTable};
    let s = 0.5;
    let h = 1. / 128.;
    let u = I::new(0., h).unwrap();
    let table = RetroTable::with_dictionary(RealMode::Hybrid).unwrap();
    let d = table
        .phi(u, 1)
        .unwrap()
        .sub(table.phi(u, 2).unwrap())
        .unwrap();
    let d = I::new(d.lo.max(0.), d.hi).unwrap();
    // a=1, B=-1, A=3/2, e=0: exact clock cancellation.
    let x_sync = I::point(1.)
        .unwrap()
        .sub(u.mul(u).unwrap().mul(d).unwrap())
        .unwrap();
    let center = x_sync.lo * 0.5 + x_sync.hi * 0.5;
    let rad = sub_up(center, x_sync.lo)
        .unwrap()
        .max(sub_up(x_sync.hi, center).unwrap());
    let src = DaeSource::new(
        vec![p(vec![lin(0, -1.)])],
        vec![p(vec![
            lin(1, 1.),
            lin(0, -3.),
            term(2., vec![t()]),
            term(-1., vec![]),
        ])],
    )
    .unwrap();
    for route in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
    ] {
        let epoch = EvolutionEpoch {
            start: s,
            end: 1.,
            incoming_x_error: Some(rad),
        };
        let run = evolve_epoch(
            &src,
            &[center],
            &[3. * center],
            &[[-4., 5.]],
            epoch,
            &opts(),
            policy(true),
            route,
            &grid(s, 1.),
        )
        .unwrap();
        assert!(run.global_qualified && run.branch_qualified && run.pointwise_qualified);
        assert_eq!(run.slabs()[0].certificate.as_ref().unwrap().incoming, rad);
        for q in run.accepted.iter().chain(&run.queries) {
            for j in 0..=256 {
                let tau = s - h + h * j as f64 / 256.;
                let x = (1.5 - tau) * (-(q.t - tau)).exp();
                let z = 3. * x - 2. * q.t + 1.;
                let err = (q.state[0] - x).abs().max((q.state[1] - z).abs());
                assert!(err <= q.physical_error_upper.unwrap() + 1e-14);
            }
        }
        println!(
            "RESET {}",
            json!({"route":route,"sync_box":[x_sync.lo,x_sync.hi],"center":center,"radius":rad,"max_bound":run.max_physical_error,"queries":run.queries,"accepted_count":run.accepted.len()})
        );
    }
}
