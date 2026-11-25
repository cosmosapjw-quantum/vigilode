#![cfg(feature = "polynomial-events")]
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
fn src() -> DaeSource {
    DaeSource::new(
        vec![p(vec![term(1., vec![])])],
        vec![p(vec![
            term(1., vec![F::component(1, 0.)]),
            term(-1., vec![F::component(0, 0.), F::component(0, 0.)]),
            term(-1., vec![F::clock(0.)]),
        ])],
    )
    .unwrap()
}
fn opts() -> Options {
    Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-12),
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
            ..Default::default()
        },
        tube_radius: 0.005,
        global_tol: 0.005,
        green: true,
    }
}
#[test]
fn first_physical_crossing_stops_native_driver() {
    let ep = EvolutionEpoch {
        start: 0.,
        end: 2.,
        incoming_x_error: Some(0.),
    };
    for route in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
        let run = evolve_to_threshold(
            &src(),
            &[0.],
            &[0.],
            &[[-1., 8.]],
            ep,
            &opts(),
            policy(),
            route,
            0.731,
            1e-6,
            ThresholdRoute::Bisection,
        )
        .unwrap();
        let root = run.root().expect("physical crossing");
        assert!(root[0] <= 0.731 && 0.731 <= root[1] && root[1] - root[0] <= 1e-6);
        assert!(run.numerical_end() < 2.);
        assert!(run.committed_until() <= root[0]);
        assert!(run.pre_query(root[1] + 1e-7).is_err());
        assert!(run.pre_query(run.committed_until()).is_ok());
        let full = evolve_epoch(
            &src(),
            &[0.],
            &[0.],
            &[[-1., 8.]],
            ep,
            &opts(),
            policy(),
            route,
            &[],
        )
        .unwrap();
        assert!(run.stats().counters.rhs_calls < full.driver().stats.counters.rhs_calls);
    }
}

fn reset_constraints() -> Vec<P> {
    vec![p(vec![
        term(1., vec![F::component(1, 0.)]),
        term(-3., vec![F::component(0, 0.)]),
        term(2., vec![F::clock(0.)]),
        term(-1., vec![]),
    ])]
}
#[test]
fn detected_event_resets_at_physical_upper_clock() {
    let ep = EvolutionEpoch {
        start: 0.,
        end: 2.,
        incoming_x_error: Some(0.),
    };
    let tau = 0.501;
    let pre = evolve_to_threshold(
        &src(),
        &[0.],
        &[0.],
        &[[-1., 8.]],
        ep,
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        tau,
        1e-6,
        ThresholdRoute::Bisection,
    )
    .unwrap();
    let root = pre.root().expect("root");
    for reset_route in [
        AffineSyncRoute::Rectangle,
        AffineSyncRoute::GreenMoment,
        AffineSyncRoute::CriticalPoint,
    ] {
        let cfg = AffineDaeReset {
            rate: 1.,
            equilibrium: 0.,
            offset: 1.5,
            time_scale: -1.,
        };
        let end = 1.;
        let grid = (0..=16)
            .map(|i| root[1] + (end - root[1]) * i as f64 / 16.)
            .collect::<Vec<_>>();
        let r = pre
            .restart_affine(
                cfg,
                reset_constraints(),
                &[3.],
                &[[-4., 5.]],
                end,
                &opts(),
                policy(),
                EvolutionRoute::Mass,
                reset_route,
                &grid,
            )
            .unwrap();
        assert!(r.post.global_qualified);
        assert_eq!(r.post.driver().t[0], root[1]);
        assert_eq!(
            r.post.slabs()[0].certificate.as_ref().unwrap().incoming,
            r.incoming_radius
        );
        for q in r.post.accepted.iter().chain(&r.post.queries) {
            for j in 0..=128 {
                let tt = root[0] + (root[1] - root[0]) * j as f64 / 128.;
                let x = (1.5 - tt) * (-(q.t - tt)).exp();
                let z = 3. * x - 2. * q.t + 1.;
                let err = (q.state[0] - x).abs().max((q.state[1] - z).abs());
                assert!(err <= q.physical_error_upper.unwrap() + 1e-14);
            }
        }
    }
}
#[test]
fn negative_controls_and_no_event() {
    let ep = EvolutionEpoch {
        start: 0.,
        end: 0.25,
        incoming_x_error: Some(0.),
    };
    let r = evolve_to_threshold(
        &src(),
        &[0.],
        &[0.],
        &[[-1., 8.]],
        ep,
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        0.731,
        1e-6,
        ThresholdRoute::IntervalNewton,
    )
    .unwrap();
    assert!(r.root().is_none() && r.reason().is_none());
    assert_eq!(r.committed_until(), ep.end);
    assert!(
        r.restart_affine(
            AffineDaeReset {
                rate: 1.,
                equilibrium: 0.,
                offset: 1.,
                time_scale: 0.
            },
            reset_constraints(),
            &[1.],
            &[[-4., 5.]],
            1.,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            AffineSyncRoute::Rectangle,
            &[]
        )
        .is_err()
    );
    assert!(
        evolve_to_threshold(
            &src(),
            &[0.],
            &[0.],
            &[[-1., 8.]],
            EvolutionEpoch {
                incoming_x_error: None,
                ..ep
            },
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            0.1,
            1e-6,
            ThresholdRoute::Bisection
        )
        .is_err()
    );
    assert!(
        evolve_to_threshold(
            &src(),
            &[0.],
            &[0.],
            &[[-1., 8.]],
            ep,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            0.,
            1e-6,
            ThresholdRoute::Bisection
        )
        .is_err()
    );
    let nontransverse = DaeSource::new(
        vec![p(vec![term(0., vec![])])],
        vec![p(vec![term(1., vec![F::component(1, 0.)])])],
    )
    .unwrap();
    let r = evolve_to_threshold(
        &nontransverse,
        &[0.],
        &[0.],
        &[[-1., 1.]],
        ep,
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        0.1,
        1e-6,
        ThresholdRoute::Bisection,
    )
    .unwrap();
    assert!(
        r.reason().is_some()
            && r.root().is_none()
            && r.committed_until() == 0.
            && r.numerical_end() > 0.
    );
    assert!(r.pre_query(r.numerical_end()).is_err());
}

fn training_source(k: &str) -> DaeSource {
    match k {
        "linear" => src(),
        "dissipative" => DaeSource::new(
            vec![p(vec![
                term(1., vec![]),
                term(-1., vec![F::component(0, 0.)]),
            ])],
            vec![p(vec![
                term(1., vec![F::component(1, 0.)]),
                term(-1., vec![F::component(0, 0.), F::component(0, 0.)]),
                term(-1., vec![F::clock(0.)]),
            ])],
        )
        .unwrap(),
        "pencil" => DaeSource::new(
            vec![p(vec![term(1., vec![]), term(1., vec![F::clock(0.)])])],
            vec![p(vec![
                term(1., vec![F::component(1, 0.)]),
                term(0.125, vec![F::clock(0.), F::component(1, 0.)]),
                term(-1., vec![F::component(0, 0.)]),
            ])],
        )
        .unwrap(),
        _ => unreachable!(),
    }
}
#[test]
fn ambiguity_and_projection_authority() {
    let ep = EvolutionEpoch {
        start: 0.,
        end: 2.,
        incoming_x_error: Some(1e-4),
    };
    let r = evolve_to_threshold(
        &src(),
        &[0.],
        &[0.],
        &[[-1., 8.]],
        ep,
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        0.731,
        1e-7,
        ThresholdRoute::IntervalNewton,
    )
    .unwrap();
    assert!(r.root().is_none() && r.reason().is_some());
    assert!(r.committed_until() <= 0.731 - 1e-4);
    let pre = evolve_to_threshold(
        &src(),
        &[0.],
        &[0.],
        &[[-1., 8.]],
        EvolutionEpoch {
            incoming_x_error: Some(0.),
            ..ep
        },
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        0.501,
        1e-6,
        ThresholdRoute::IntervalNewton,
    )
    .unwrap();
    assert!(pre.root().is_some());
    let mut po = policy();
    po.init.mode = InitMode::Strict;
    assert!(
        pre.restart_affine(
            AffineDaeReset {
                rate: 1.,
                equilibrium: 0.,
                offset: 1.5,
                time_scale: -1.
            },
            reset_constraints(),
            &[3.],
            &[[-4., 5.]],
            1.,
            &opts(),
            po,
            EvolutionRoute::Mass,
            AffineSyncRoute::GreenMoment,
            &[]
        )
        .is_err()
    );
}
#[test]
#[ignore = "finite development campaign"]
fn training() {
    use serde_json::json;
    let pre_routes = [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::WarpFolded,
    ];
    let roots = [
        ThresholdRoute::Bisection,
        ThresholdRoute::IntervalNewton,
        ThresholdRoute::DictionaryHint,
    ];
    let syncs = [
        AffineSyncRoute::Rectangle,
        AffineSyncRoute::GreenMoment,
        AffineSyncRoute::CriticalPoint,
    ];
    let post_routes = [EvolutionRoute::Mass, EvolutionRoute::Reduced5];
    for (k, start, x0, z0, threshold) in [
        ("linear", 0., 0., 0., 0.501),
        ("dissipative", 0., 0., 0., 0.4),
        ("pencil", 0.25, 0.125, 0.125 / (1. + 0.25 / 8.), 0.6),
    ] {
        let source = training_source(k);
        let ep = EvolutionEpoch {
            start,
            end: 2.,
            incoming_x_error: Some(0.),
        };
        for pre_route in pre_routes {
            for root_route in roots {
                let pre = evolve_to_threshold(
                    &source,
                    &[x0],
                    &[z0],
                    &[[-1., 8.]],
                    ep,
                    &opts(),
                    policy(),
                    pre_route,
                    threshold,
                    1e-6,
                    root_route,
                );
                for sync_route in syncs {
                    for post_route in post_routes {
                        let mut row = json!({"kind":k,"start":start,"x0":x0,"threshold":threshold,"pre_route":pre_route,"root_route":root_route,"sync_route":sync_route,"post_route":post_route});
                        match &pre {
                            Ok(pr) => {
                                row["pre"] = json!({"root":pr.root(),"reason":pr.reason(),"safe":pr.committed_until(),"numerical_end":pr.numerical_end(),"stats":pr.stats().counters,"accepted_steps":pr.stats().accepted_steps,"work":pr.work});
                                if let Some(root) = pr.root() {
                                    let grid = (0..=16)
                                        .map(|i| {
                                            if i == 16 {
                                                1.
                                            } else {
                                                root[1] + (1. - root[1]) * i as f64 / 16.
                                            }
                                        })
                                        .collect::<Vec<_>>();
                                    let reset = AffineDaeReset {
                                        rate: 1.,
                                        equilibrium: 0.,
                                        offset: 1.5,
                                        time_scale: -1.,
                                    };
                                    match pr.restart_affine(
                                        reset,
                                        reset_constraints(),
                                        &[3.],
                                        &[[-4., 5.]],
                                        1.,
                                        &opts(),
                                        policy(),
                                        post_route,
                                        sync_route,
                                        &grid,
                                    ) {
                                        Ok(r) => {
                                            row["post"] = json!({"box":r.synchronized_x,"radius":r.incoming_radius,"bound":r.post.max_physical_error,"global":r.post.global_qualified,"branch":r.post.branch_qualified,"pointwise":r.post.pointwise_qualified,"stats":r.post.driver().stats.counters,"work":r.post.work,"queries":r.post.queries,"epoch_start":r.post.driver().t[0]});
                                        }
                                        Err(e) => row["post_error"] = json!(e.to_string()),
                                    }
                                }
                            }
                            Err(e) => row["pre_error"] = json!(e.to_string()),
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}
