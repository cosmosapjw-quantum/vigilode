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
fn mixed() -> SimplexReset {
    SimplexReset::new([
        [1023. / 1024., 0., 0.25],
        [0., 0.5, 0.],
        [1. / 1024., 0.5, 0.75],
    ])
    .unwrap()
}
fn budget() -> ConservedChainBudget {
    ConservedChainBudget {
        limits: ChainBudget {
            native_attempts: 4096,
            scan_slabs: 512,
            transition_attempts: 4,
            reset_cells: 64,
            gap_checks: 256,
            min_dwell: 1e-4,
        },
        picard_attempts: 512,
    }
}
fn mode(
    rate: f64,
    guards: Vec<P>,
    targets: Vec<usize>,
    solver: EvolutionRoute,
    sync: VectorSyncRoute,
    l1: SimplexSyncRoute,
) -> ConservedMode {
    let mut pol = rp(sync);
    pol.branch_route = match solver {
        EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
        EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
        _ => InitRoute::Reuse,
    };
    let edges = targets
        .into_iter()
        .map(|target| ConservedTransition {
            target,
            reset: if rate == 3e7 { reset() } else { mixed() },
            policy: pol,
            sync: l1,
        })
        .collect();
    ConservedMode {
        rate,
        route: solver,
        certificate: ConservationRoute::Integral4,
        guards,
        transitions: edges,
    }
}
fn graph(
    case: &str,
    solver: EvolutionRoute,
    sync: VectorSyncRoute,
    l1: SimplexSyncRoute,
) -> Vec<ConservedMode> {
    let q = 1. / 256.;
    let b = 1. / 64.;
    let g1 = if case == "overlap" {
        h(b + 2f64.powi(-28))
    } else {
        h(1. / 40.)
    };
    let g2 = if case == "cancel" {
        p(vec![
            tr(1., vec![F::component(0, 0.)]),
            tr(1., vec![F::component(1, 0.)]),
            tr(1., vec![F::component(2, 0.)]),
            tr(-1., vec![]),
            tr(1. / 4096., vec![F::clock(b)]),
            tr(1. / 524288., vec![F::component(2, 0.)]),
        ])
    } else {
        h(b)
    };
    vec![
        mode(3e7, vec![h(2. * q), h(q)], vec![2, 1], solver, sync, l1),
        mode(6e7, vec![g1, g2], vec![0, 2], solver, sync, l1),
        mode(3e7, vec![], vec![], solver, sync, l1),
    ]
}
fn run(
    m: &[ConservedMode],
    order: MultiGuardRoute,
    gap: SimplexGapRoute,
    b: ConservedChainBudget,
) -> ConservedChainRun {
    evolve_conserved_chain(
        m,
        0,
        RobertsonInitial::simplex_ball([1., 0.], 2f64.powi(-20)).unwrap(),
        [0., 1. / 32.],
        &opts(),
        policy(),
        mp(order),
        b,
        gap,
    )
    .unwrap()
}
#[test]
fn conserved_two_async_transitions() {
    let m = graph(
        "separated",
        EvolutionRoute::Mass,
        VectorSyncRoute::Centered,
        SimplexSyncRoute::JointTransport,
    );
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        budget(),
    );
    assert_eq!(r.stop(), ChainStop::Horizon, "{:?}", r.reason());
    assert_eq!(r.final_mode(), 2);
    assert_eq!(r.epochs().len(), 3);
    assert_eq!(r.transitions().len(), 2);
    for (i, e) in r.transitions().iter().enumerate() {
        assert!(e.committed && e.event_time[0] < e.event_time[1]);
        assert_eq!(e.guard, 1);
        assert_eq!(
            e.transport.as_ref().unwrap().selected_l1,
            r.epochs()[i + 1].incoming_l1()
        );
        assert!(e.gap.iter().all(|g| g.enclosure[1] < 0.));
        assert!(r.query(i, e.event_time[1]).is_err());
    }
    assert!(r.transitions()[1].dwell_lower.unwrap() >= budget().limits.min_dwell);
    let a = r.query(2, 1. / 32.).unwrap();
    assert!((a.state.iter().sum::<f64>() - 1.).abs() < 1e-12);
}

#[test]
fn exact_l1_restart_set_and_dual_cancellation() {
    let model = RobertsonModel::new(3e7).unwrap();
    let center = [0.5, 0.25];
    let r = 2f64.powi(-10);
    let guard = |q: f64| {
        p(vec![
            tr(1., vec![F::component(0, 0.)]),
            tr(1., vec![F::component(1, 0.)]),
            tr(-q, vec![]),
        ])
    };
    let small = [
        [center[0] - r / 8., center[0] + r / 8.],
        [center[1] - r / 8., center[1] + r / 8.],
    ];
    let g = guard(0.75 + 1.5 * r);
    let direct = model
        .simplex_gap_enclosure(
            &g,
            [0.125, 0.125],
            small,
            center,
            Some(r),
            SimplexGapRoute::Box,
        )
        .unwrap();
    let dual = model
        .simplex_gap_enclosure(
            &g,
            [0.125, 0.125],
            small,
            center,
            Some(r),
            SimplexGapRoute::AffineDual,
        )
        .unwrap();
    assert!(direct.enclosure[1] > 0. && dual.enclosure[1] < 0.);
    assert_eq!(dual.enclosure[1], -r / 2.);
    let bad = guard(0.75 + 0.75 * r);
    let tube = model
        .simplex_gap_enclosure(
            &bad,
            [0.124, 0.125],
            small,
            center,
            None,
            SimplexGapRoute::AffineDual,
        )
        .unwrap();
    let restart = model
        .simplex_gap_enclosure(
            &bad,
            [0.125, 0.125],
            small,
            center,
            Some(r),
            SimplexGapRoute::AffineDual,
        )
        .unwrap();
    assert!(tube.enclosure[1] < 0. && restart.enclosure[1] > 0.);
    assert_eq!(restart.differential_box[0], [center[0] - r, center[0] + r]);
}
#[test]
fn conserved_gap_hidden_event_and_tie() {
    // Fixed dyadic clock chosen between independently located roots from
    // the two physical initial vertices; no holdout or parameter tuning.
    let center = 4293935398_f64 * 2f64.powi(-40);
    let eps = 1024_f64 * 2f64.powi(-40);
    let mut m = graph(
        "separated",
        EvolutionRoute::Mass,
        VectorSyncRoute::Centered,
        SimplexSyncRoute::JointTransport,
    );
    m[1].guards[1] = p(vec![
        tr(eps * eps, vec![]),
        tr(-1., vec![F::clock(center), F::clock(center)]),
    ]);
    for g in [SimplexGapRoute::Box, SimplexGapRoute::AffineDual] {
        let r = run(&m, MultiGuardRoute::Correlated, g, budget());
        assert_eq!(r.stop(), ChainStop::GapUnproved, "{:?}", r.reason());
        assert_eq!(r.final_mode(), 0);
        assert_eq!(r.epochs().len(), 1);
        let e = &r.transitions()[0];
        assert!(!e.committed && e.reset.is_some() && e.transport.is_some());
        assert!(e.event_time[0] < center - eps && e.event_time[1] > center + eps);
        assert!(eps * eps - (e.event_time[1] - center).powi(2) < 0.);
        assert!(
            e.gap
                .iter()
                .any(|g| g.cell.is_some() && g.guard == 1 && g.enclosure[1] >= 0.)
        );
        assert!(r.query(0, 1. / 512.).is_ok() && r.query(1, e.event_time[1]).is_err());
    }
    let mut m = graph(
        "separated",
        EvolutionRoute::Mass,
        VectorSyncRoute::Centered,
        SimplexSyncRoute::JointTransport,
    );
    m[1].guards[0] = m[1].guards[1].clone();
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        budget(),
    );
    assert_eq!(r.stop(), ChainStop::OrderUnresolved);
    assert_eq!(r.final_mode(), 1);
    assert!(r.transitions()[0].committed && r.query(1, 1. / 128.).is_ok());
}
#[test]
fn cumulative_conserved_chain_budgets_and_dwell() {
    let m = graph(
        "separated",
        EvolutionRoute::Mass,
        VectorSyncRoute::Centered,
        SimplexSyncRoute::JointTransport,
    );
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        budget(),
    );
    assert_eq!(r.stop(), ChainStop::Horizon);
    let mut b = budget();
    b.limits.transition_attempts = 1;
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        b,
    );
    assert_eq!(r.stop(), ChainStop::TransitionBudget);
    assert_eq!(r.final_mode(), 1);
    assert!(r.query(1, 1. / 128.).is_ok());
    b = budget();
    b.limits.native_attempts = 1;
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        b,
    );
    assert_eq!(r.stop(), ChainStop::NativeBudget);
    assert!(r.work().common.charged_native_attempts <= 1);
    b = budget();
    b.limits.scan_slabs = 1;
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        b,
    );
    assert_eq!(r.stop(), ChainStop::SlabBudget);
    assert!(r.work().common.charged_scan_slabs <= 1);
    b = budget();
    b.limits.gap_checks = 1;
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        b,
    );
    assert_eq!(r.stop(), ChainStop::GapBudget);
    assert_eq!(r.work().common.gap_checks, 1);
    assert_eq!(r.final_mode(), 0);
    b = budget();
    b.limits.reset_cells = 1;
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        b,
    );
    assert_eq!(r.stop(), ChainStop::ResetBudget);
    assert_eq!(r.work().common.charged_reset_cells, 1);
    b = budget();
    b.picard_attempts = 9;
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        b,
    );
    assert_eq!(r.stop(), ChainStop::ResetBudget);
    assert_eq!(r.work().charged_picard_attempts, 9);
    assert_eq!(r.final_mode(), 1);
    b = budget();
    b.limits.min_dwell = 0.02;
    let r = run(
        &m,
        MultiGuardRoute::Correlated,
        SimplexGapRoute::AffineDual,
        b,
    );
    assert_eq!(r.stop(), ChainStop::DwellUnproved);
    assert_eq!(r.final_mode(), 1);
    assert!(r.query(1, 1. / 128.).is_ok());
    assert!(!r.transitions()[1].committed);
}
#[test]
fn repeated_mode_cycle_and_uncertainty_limit() {
    let v = 1. / 8192.;
    let h = |q: f64| p(vec![tr(q, vec![]), tr(-1., vec![F::component(1, 0.)])]);
    let mut m = mode(
        3e7,
        vec![h(1. / 16384.), h(1. / 16384. - 2f64.powi(-18))],
        vec![0, 0],
        EvolutionRoute::Mass,
        VectorSyncRoute::Centered,
        SimplexSyncRoute::JointTransport,
    );
    let r = SimplexReset::new([[1. - v; 3], [v; 3], [0.; 3]]).unwrap();
    for e in &mut m.transitions {
        e.reset = r.clone();
    }
    let mut b = budget();
    b.limits.transition_attempts = 3;
    b.limits.min_dwell = 1e-5;
    let init = RobertsonInitial::simplex_ball([1. - v, v], 2f64.powi(-26)).unwrap();
    let graph = [m];
    let old = evolve_conserved_chain(
        &graph,
        0,
        init,
        [0., 0.001],
        &opts(),
        policy(),
        mp(MultiGuardRoute::Correlated),
        b,
        SimplexGapRoute::AffineDual,
    )
    .unwrap();
    let old_guards = old
        .epochs()
        .first()
        .and_then(|e| e.ordered())
        .map(|r| r.guards());
    println!(
        "CYCLE_OLD {}",
        serde_json::json!({"stop":old.stop(),"reason":old.reason(),"work":old.work(),"guards":old_guards})
    );
    assert_eq!(old.stop(), ChainStop::OrderUnresolved);
    let mut pol = mp(MultiGuardRoute::Correlated);
    pol.derivative = GuardDerivativeRoute::ConservedSchur;
    let run = evolve_conserved_chain(
        &graph,
        0,
        init,
        [0., 0.001],
        &opts(),
        policy(),
        pol,
        b,
        SimplexGapRoute::AffineDual,
    )
    .unwrap();
    println!(
        "CYCLE_DIAGNOSTIC {}",
        serde_json::json!({"stop":run.stop(),"reason":run.reason(),"work":run.work(),"events":run.transitions(),"epochs":run.epochs().iter().map(|e|serde_json::json!({"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"guards":e.ordered().map(|r|r.guards())})).collect::<Vec<_>>()})
    );
    assert_eq!(run.stop(), ChainStop::OrderUnresolved);
    assert_eq!(run.work().common.transitions_committed, 1);
    assert!(run.transitions()[0].committed);
    assert!(run.epochs()[1].ordered().unwrap().guards().iter().any(|g| {
        g.reason
            .as_ref()
            .is_some_and(|v| v.contains("width exceeds tolerance"))
    }));
    assert!(run.query(0, 1e-4).is_ok() && run.query(1, 5e-4).is_ok());
    // A separately declared smaller-uncertainty diagnostic, same graph,
    // rates, physical caps, root tolerance and budgets. Original failure
    // above is not relabelled as success.
    let narrow = RobertsonInitial::simplex_ball([1. - v, v], 2f64.powi(-30)).unwrap();
    let run = evolve_conserved_chain(
        &graph,
        0,
        narrow,
        [0., 0.001],
        &opts(),
        policy(),
        pol,
        b,
        SimplexGapRoute::AffineDual,
    )
    .unwrap();
    println!(
        "CYCLE_NARROW_DIAGNOSTIC {}",
        serde_json::json!({"stop":run.stop(),"reason":run.reason(),"work":run.work(),"events":run.transitions(),"epochs":run.epochs().iter().map(|e|serde_json::json!({"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"guards":e.ordered().map(|r|r.guards())})).collect::<Vec<_>>()})
    );
    assert!(
        run.work().common.transitions_committed >= 2,
        "{:?} {:?}",
        run.stop(),
        run.reason()
    );
    assert!(matches!(
        run.stop(),
        ChainStop::Horizon | ChainStop::TransitionBudget
    ));
    assert!(
        run.transitions()
            .iter()
            .filter(|e| e.committed)
            .all(|e| e.from == 0
                && e.target == 0
                && e.guard == 0
                && e.transport.as_ref().unwrap().operator_l1 == 0.)
    );
    assert!(
        run.transitions()
            .iter()
            .skip(1)
            .filter(|e| e.committed)
            .all(|e| e.dwell_lower.unwrap() >= b.limits.min_dwell)
    );
    println!(
        "CYCLE {}",
        serde_json::json!({"stop":run.stop(),"reason":run.reason(),"work":run.work(),"events":run.transitions()})
    );
}
fn rec(r: &EvolutionRun) -> serde_json::Value {
    serde_json::json!({"pointwise":r.pointwise_qualified,"branch":r.branch_qualified,"global":r.global_qualified,"bound":r.max_physical_error,"driver_success":r.driver().is_success(),"driver_end":r.driver().t.last(),"attempts":r.driver().stats.attempts,"counters":r.driver().stats.counters,"work":r.work,"initial":r.initial,"slabs":r.slabs(),"queries":r.queries,"accepted":r.accepted})
}
#[test]
#[ignore = "fixed conserved chain development campaign"]
fn training() {
    use serde_json::json;
    for case in ["separated", "overlap", "cancel"] {
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Prepared5,
            EvolutionRoute::Scalar5,
        ] {
            for sync in [VectorSyncRoute::Centered, VectorSyncRoute::GreenMoment] {
                for l1 in [SimplexSyncRoute::BoxLift, SimplexSyncRoute::JointTransport] {
                    for gap in [SimplexGapRoute::Box, SimplexGapRoute::AffineDual] {
                        for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                            let m = graph(case, solver, sync, l1);
                            let r = run(&m, order, gap, budget());
                            let mut epochs = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                let queries = if e.safe() > e.start() {
                                    (0..=8)
                                        .filter_map(|j| {
                                            r.query(
                                                i,
                                                if j == 8 {
                                                    e.safe()
                                                } else {
                                                    e.start()
                                                        + (e.safe() - e.start()) * j as f64 / 8.
                                                },
                                            )
                                            .ok()
                                        })
                                        .collect::<Vec<_>>()
                                } else {
                                    vec![]
                                };
                                epochs.push(json!({"mode":e.mode(),"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"center":e.center(),"queries":queries,
          "slabs":e.original_slab_diagnostics(),
          "ordered":e.ordered().map(|a|json!({"winner":a.winner(),"reason":a.reason(),"guards":a.guards(),"comparisons":a.comparisons(),"work":a.work})),
          "terminal":e.terminal().map(rec)}));
                            }
                            println!(
                                "CELL {}",
                                json!({"case":case,"solver":solver,"sync":sync,"l1_route":l1,"gap_route":gap,"order":order,"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"events":r.transitions(),"epochs":epochs})
                            );
                        }
                    }
                }
            }
        }
    }
}
