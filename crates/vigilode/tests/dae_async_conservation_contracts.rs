#![cfg(feature = "polynomial-events")]
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn p(v: Vec<T>) -> P {
    P::new(v).unwrap()
}
fn tr(coefficient: f64, factors: Vec<F>) -> T {
    T {
        coefficient,
        factors,
    }
}
fn h(q: f64) -> P {
    p(vec![
        tr(1., vec![F::clock(q)]),
        tr(1. / 128., vec![F::component(2, 0.)]),
    ])
}
fn opts() -> Options {
    Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
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
        tube_radius: 1e-3,
        global_tol: 1e-3,
        green: true,
    }
}
fn mp(method: MultiGuardRoute) -> MultiGuardPolicy {
    MultiGuardPolicy {
        method,
        tolerance: 1e-6,
        root_route: ThresholdRoute::DictionaryHint,
        derivative: GuardDerivativeRoute::Schur,
        value: GuardValueRoute::Transport,
        pending: PendingPolicy {
            method: PendingMethod::PiecewiseProbe,
            max_slabs: 8,
        },
        max_slabs: 128,
    }
}
fn reset() -> SimplexReset {
    SimplexReset::new([[1., 0., 0.], [0., 0.5, 0.], [0., 0.5, 1.]]).unwrap()
}
fn rp(sync: VectorSyncRoute) -> VectorResetPolicy {
    VectorResetPolicy {
        sync,
        derivative: GuardDerivativeRoute::Schur,
        branch_route: InitRoute::Reuse,
        subdivisions: 1,
    }
}
#[test]
fn async_physical_guard_and_mass_reset() {
    let q = 1. / 256.;
    let r = evolve_robertson_async_reset(
        [3e7, 6e7],
        RobertsonInitial::simplex_ball([1., 0.], 2f64.powi(-20)).unwrap(),
        [0., 1. / 32.],
        &[h(2. * q), h(q)],
        &reset(),
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        mp(MultiGuardRoute::Correlated),
        rp(VectorSyncRoute::Centered),
        SimplexSyncRoute::JointTransport,
        &[1. / 64., 1. / 32.],
    )
    .unwrap();
    assert_eq!(r.pre().winner(), Some(1));
    let event = r.event_time.unwrap();
    assert!(event[0] < event[1] && event[1] - event[0] <= 1e-6);
    let po = r.post().expect("native conserved asynchronous postflow");
    assert_eq!(po.driver().t[0], event[1]);
    assert_eq!(
        po.slabs()[0].certificate.as_ref().unwrap().incoming,
        r.incoming_l1.unwrap()
    );
    assert!(r.incoming_l1.unwrap() > 0.);
    assert!(r.pre().pre_query(event[1]).is_err());
    assert!(r.transport.as_ref().unwrap().operator_l1 <= 1.);
}

fn mixed() -> SimplexReset {
    SimplexReset::new([
        [1023. / 1024., 0., 0.25],
        [0., 0.5, 0.],
        [1. / 1024., 0.5, 0.75],
    ])
    .unwrap()
}
fn run(
    family: &str,
    route: EvolutionRoute,
    sync: VectorSyncRoute,
    ls: SimplexSyncRoute,
    order: MultiGuardRoute,
) -> RobertsonAsyncRun {
    let q = 1. / 256.;
    let (q2, rad, m) = match family {
        "separated" => (2. * q, 2f64.powi(-20), reset()),
        "overlap" => (q + 2f64.powi(-28), 2f64.powi(-17), reset()),
        "mixed" => (2. * q, 2f64.powi(-17), mixed()),
        _ => panic!("family"),
    };
    let mut rp = rp(sync);
    rp.branch_route = match route {
        EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
        EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
        _ => InitRoute::Reuse,
    };
    evolve_robertson_async_reset(
        [3e7, 6e7],
        RobertsonInitial::simplex_ball([1., 0.], rad).unwrap(),
        [0., 1. / 32.],
        &[h(q2), h(q)],
        &m,
        &opts(),
        policy(),
        route,
        ConservationRoute::Integral4,
        mp(order),
        rp,
        ls,
        &[1. / 128., 1. / 64., 1. / 32.],
    )
    .unwrap()
}
#[test]
fn simplex_matrix_exact_admission_and_norm() {
    let m = mixed();
    assert_eq!(m.operator_l1(), 0.75);
    assert!(
        SimplexReset::new([[1., 0., 0.], [0., 1., 0.], [0., 0., 1.0_f64.next_down()]]).is_err()
    );
    assert!(SimplexReset::new([[1., 0., 0.], [-0.125, 1., 0.], [0.125, 0., 1.]]).is_err());
    let swap = SimplexReset::new([[0., 0., 1.], [0., 0.5, 0.], [1., 0.5, 0.]]).unwrap();
    assert_eq!(swap.operator_l1(), 1.5);
    assert!(RobertsonInitial::exact([1., 1e-15]).is_err());
}
#[test]
fn correlated_order_and_tie_fail_closed() {
    let shared = run(
        "overlap",
        EvolutionRoute::Mass,
        VectorSyncRoute::Centered,
        SimplexSyncRoute::JointTransport,
        MultiGuardRoute::Shared,
    );
    assert!(shared.post().is_none() && shared.pre().winner().is_none());
    let r = run(
        "overlap",
        EvolutionRoute::Mass,
        VectorSyncRoute::Centered,
        SimplexSyncRoute::JointTransport,
        MultiGuardRoute::Correlated,
    );
    assert_eq!(r.pre().winner(), Some(1));
    assert!(r.post().is_some(), "{:?}", r.reason);
    assert!(
        r.pre()
            .comparisons()
            .iter()
            .any(|c| c.earlier == 1 && c.later == 0 && c.positive)
    );
    let q = 1. / 256.;
    let init = RobertsonInitial::simplex_ball([1., 0.], 2f64.powi(-20)).unwrap();
    let tied = evolve_robertson_async_reset(
        [3e7, 6e7],
        init,
        [0., 1. / 32.],
        &[h(q), h(q)],
        &reset(),
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        mp(MultiGuardRoute::Correlated),
        rp(VectorSyncRoute::Centered),
        SimplexSyncRoute::JointTransport,
        &[],
    )
    .unwrap();
    assert!(tied.pre().winner().is_none() && tied.preparation.is_none() && tied.post().is_none());
    let mut o = opts();
    o.max_steps = 1;
    let budget = evolve_robertson_async_reset(
        [3e7, 6e7],
        init,
        [0., 1. / 32.],
        &[h(2. * q), h(q)],
        &reset(),
        &o,
        policy(),
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        mp(MultiGuardRoute::Correlated),
        rp(VectorSyncRoute::Centered),
        SimplexSyncRoute::JointTransport,
        &[],
    )
    .unwrap();
    assert!(budget.post().is_none() && budget.charged_native_attempts <= 1);
    assert!(
        evolve_robertson_async_reset(
            [3e7, 6e7],
            init,
            [0., 1. / 32.],
            &[h(2. * q), h(q)],
            &reset(),
            &opts(),
            policy(),
            EvolutionRoute::WarpFolded,
            ConservationRoute::Integral4,
            mp(MultiGuardRoute::Correlated),
            rp(VectorSyncRoute::Centered),
            SimplexSyncRoute::JointTransport,
            &[]
        )
        .is_err()
    );
}
#[test]
fn joint_l1_and_original_simplex_authority() {
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
    ] {
        let a = run(
            "mixed",
            solver,
            VectorSyncRoute::Centered,
            SimplexSyncRoute::BoxLift,
            MultiGuardRoute::Correlated,
        );
        let b = run(
            "mixed",
            solver,
            VectorSyncRoute::Centered,
            SimplexSyncRoute::JointTransport,
            MultiGuardRoute::Correlated,
        );
        assert!(
            a.post().is_some() && b.post().is_some(),
            "{:?} {:?}",
            a.reason,
            b.reason
        );
        assert_eq!(a.event_time, b.event_time);
        let c = b.transport.as_ref().unwrap();
        assert_eq!(c.operator_l1, 0.75);
        assert!(c.selected_l1 <= c.component_l1 && c.selected_l1 <= c.joint_l1.unwrap());
        assert!(b.incoming_l1.unwrap() < a.incoming_l1.unwrap());
        assert!(
            b.post()
                .unwrap()
                .queries
                .iter()
                .all(|q| (q.state.iter().sum::<f64>() - 1.).abs() < 1e-12)
        );
        let guard = &b.pre().guards()[1];
        assert!(
            guard
                .slabs
                .iter()
                .all(|s| s.reduced_lip <= 1. / 128. && s.algebraic_guard_lip == 0.)
        );
    }
}
fn rec(r: &EvolutionRun) -> serde_json::Value {
    serde_json::json!({"pointwise":r.pointwise_qualified,"branch":r.branch_qualified,"global":r.global_qualified,"bound":r.max_physical_error,"reason":r.reason,"driver_success":r.driver().is_success(),"driver_end":r.driver().t.last(),"attempts":r.driver().stats.attempts,"counters":r.driver().stats.counters,"work":r.work,"initial":r.initial,"slabs":r.slabs(),"queries":r.queries,"accepted":r.accepted,"warp":r.warp})
}
#[test]
#[ignore = "fixed conserved async development matrix"]
fn training() {
    use serde_json::json;
    for family in ["separated", "overlap", "mixed"] {
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Prepared5,
            EvolutionRoute::Scalar5,
        ] {
            for sync in [
                VectorSyncRoute::BoxPicard,
                VectorSyncRoute::Centered,
                VectorSyncRoute::GreenMoment,
            ] {
                for ls in [SimplexSyncRoute::BoxLift, SimplexSyncRoute::JointTransport] {
                    for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                        let r = run(family, solver, sync, ls, order);
                        let safe = r.pre().committed_until();
                        let queries = if safe > 0. {
                            (0..=16)
                                .filter_map(|i| {
                                    r.pre()
                                        .pre_query(if i == 16 {
                                            safe
                                        } else {
                                            safe * i as f64 / 16.
                                        })
                                        .ok()
                                })
                                .collect::<Vec<_>>()
                        } else {
                            vec![]
                        };
                        let mut row = json!({"case":family,"solver":solver,"sync":sync,"l1_route":ls,"order":order,"event":r.event_time,"winner":r.pre().winner(),"safe":safe,"reason":r.reason,"pre_guards":r.pre().guards(),"comparisons":r.pre().comparisons(),"pre_work":r.pre().work,"pre_slabs":r.original_slab_diagnostics(),"pre_queries":queries,"pre_projection":r.pre().event_authority().map(|a|a.projection_work()),"preparation":r.preparation,"transport":r.transport,"incoming_l1":r.incoming_l1,"attempts":r.native_attempts,"charged_attempts":r.charged_native_attempts,"committed":r.post().is_some()});
                        if let Some(po) = r.post_diagnostic() {
                            row["post"] = rec(po);
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}
