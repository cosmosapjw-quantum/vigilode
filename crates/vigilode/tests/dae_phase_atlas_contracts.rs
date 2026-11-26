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
fn cycle(
    overlap: bool,
    route: EvolutionRoute,
    cert: ConservationRoute,
    sync: VectorSyncRoute,
) -> ConservedMode {
    let v = 1. / 8192.;
    let q = 1. / 16384.;
    let h = |q: f64| p(vec![tr(q, vec![]), tr(-1., vec![F::component(1, 0.)])]);
    let reset = SimplexReset::new([[1. - v; 3], [v; 3], [0.; 3]]).unwrap();
    let mut pol = rp(sync);
    pol.branch_route = match route {
        EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
        EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
        _ => InitRoute::Reuse,
    };
    ConservedMode {
        rate: 3e7,
        route,
        certificate: cert,
        guards: vec![
            h(q),
            h(q - if overlap {
                2f64.powi(-28)
            } else {
                2f64.powi(-18)
            }),
        ],
        transitions: (0..2)
            .map(|_| ConservedTransition {
                target: 0,
                reset: reset.clone(),
                policy: pol,
                sync: SimplexSyncRoute::JointTransport,
            })
            .collect(),
    }
}
fn initial(rad: f64) -> RobertsonInitial {
    let v = 1. / 8192.;
    RobertsonInitial::simplex_ball([1. - v, v], rad).unwrap()
}
fn pm(order: MultiGuardRoute) -> MultiGuardPolicy {
    let mut p = mp(order);
    p.derivative = GuardDerivativeRoute::ConservedSchur;
    p
}
fn limits() -> PhaseCycleBudget {
    PhaseCycleBudget {
        native_attempts: 4096,
        scan_slabs: 512,
        transitions: 3,
        min_dwell: 1e-5,
        absolute_width: 1e-6,
    }
}
fn constant(v: f64) -> SimplexReset {
    SimplexReset::new([[1. - v; 3], [v; 3], [0.; 3]]).unwrap()
}
fn graph(case: &str, solver: EvolutionRoute, cert: ConservationRoute) -> Vec<ConservedMode> {
    let mut a = cycle(case == "overlap", solver, cert, VectorSyncRoute::Centered);
    let mut b = cycle(case == "overlap", solver, cert, VectorSyncRoute::Centered);
    b.rate = 6e7;
    if case == "distinct" {
        let mut c = cycle(false, solver, cert, VectorSyncRoute::Centered);
        for e in &mut a.transitions {
            e.target = 1;
            e.reset = constant(1. / 8192.);
        }
        for e in &mut b.transitions {
            e.target = 2;
            e.reset = constant(5. / 32768.);
        }
        for e in &mut c.transitions {
            e.target = 1;
            e.reset = constant(3. / 16384.);
        }
        {
            a.transitions[1].target = 0;
            a.transitions[1].reset = constant(3. / 16384.);
            vec![a, b, c]
        }
    } else {
        for e in &mut a.transitions {
            e.target = 1;
            e.reset = constant(3. / 16384.);
        }
        for e in &mut b.transitions {
            e.target = 0;
            e.reset = constant(5. / 32768.);
        }
        {
            a.transitions[1].target = 0;
            a.transitions[1].reset = constant(1. / 8192.);
            vec![a, b]
        }
    }
}
fn atlas_budget() -> PhaseAtlasBudget {
    PhaseAtlasBudget {
        limits: PhaseCycleBudget {
            transitions: 10,
            ..limits()
        },
        reset_checks: 64,
        cache_entries: 10,
    }
}
#[test]
fn distinct_mode_entries_do_not_alias() {
    let m = graph(
        "distinct",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    let r = evolve_robertson_phase_atlas(
        &m,
        0,
        initial(2f64.powi(-26)),
        [0., 0.002],
        &opts(),
        policy(),
        pm(MultiGuardRoute::Correlated),
        atlas_budget(),
        PhaseReuseRoute::Reuse,
    )
    .unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::Horizon, "{:?}", r.reason());
    assert_eq!(r.events().iter().filter(|e| e.event.committed).count(), 6);
    let es = r.epochs();
    assert_eq!(
        es.iter().map(|e| e.mode).collect::<Vec<_>>(),
        vec![0, 1, 2, 1, 2, 1, 2]
    );
    assert_ne!(es[1].phase.phase_index, es[3].phase.phase_index);
    assert_eq!(es[3].phase.phase_index, es[5].phase.phase_index);
    assert_eq!(es[2].phase.phase_index, es[4].phase.phase_index);
    assert_eq!(es[4].phase.phase_index, es[6].phase.phase_index);
    assert_eq!(r.work().phase.cache_hits, 3);
    assert_eq!(r.phases().len(), 4);
    assert_eq!(r.phases()[es[1].phase.phase_index].initial.l1_radius(), 0.);
    assert_eq!(
        r.phases()[es[1].phase.phase_index].initial.center(),
        [1. - 1. / 8192., 1. / 8192.]
    );
    assert_eq!(
        r.phases()[es[3].phase.phase_index].initial.center(),
        [1. - 3. / 16384., 3. / 16384.]
    );
    assert!(r.query(6, 0.002, PhaseQueryRoute::Centered).is_ok());
    assert!(
        r.query(
            0,
            r.events()[0].event.absolute[1],
            PhaseQueryRoute::Centered
        )
        .is_err()
    );
}

#[test]
fn same_center_different_mode_and_capacity() {
    let mut m = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    for a in &mut m {
        for e in &mut a.transitions {
            e.reset = constant(1. / 8192.);
        }
    }
    m[0].transitions[1].target = 1;
    let init = initial(0.);
    let run = |b, reuse| {
        evolve_robertson_phase_atlas(
            &m,
            0,
            init,
            [0., 0.001],
            &opts(),
            policy(),
            pm(MultiGuardRoute::Correlated),
            b,
            reuse,
        )
        .unwrap()
    };
    let r = run(atlas_budget(), PhaseReuseRoute::Reuse);
    let f = run(atlas_budget(), PhaseReuseRoute::Fresh);
    assert_eq!(r.stop(), PhaseAtlasStop::Horizon);
    assert_eq!(f.stop(), r.stop());
    assert_eq!(r.phases().len(), 2);
    assert_ne!(
        r.epochs()[0].phase.phase_index,
        r.epochs()[1].phase.phase_index
    );
    assert_eq!(
        r.phases()[0].initial.center(),
        r.phases()[1].initial.center()
    );
    let a = r.events()[0].event.relative;
    let b = r.events()[1].event.relative;
    assert!(a[0] > b[1]);
    assert_eq!(
        r.epochs()[0].phase.phase_index,
        r.epochs()[2].phase.phase_index
    );
    assert_eq!(r.events().len(), f.events().len());
    for (a, b) in r.events().iter().zip(f.events()) {
        assert_eq!(a.event.relative, b.event.relative);
        assert_eq!(a.event.absolute, b.event.absolute);
        assert_eq!(a.target, b.target);
    }
    let mut b = atlas_budget();
    b.cache_entries = 1;
    let c = run(b, PhaseReuseRoute::Reuse);
    assert_eq!(c.stop(), r.stop());
    assert_eq!(c.work().cache_inserts, 1);
    assert!(c.work().cache_full_misses > 0);
    assert!(c.work().phase.native_attempts >= r.work().phase.native_attempts);
    for (a, b) in c.events().iter().zip(r.events()) {
        assert_eq!(a.event.absolute, b.event.absolute);
    }
    b.cache_entries = 0;
    let c = run(b, PhaseReuseRoute::Reuse);
    assert_eq!(c.stop(), r.stop());
    assert_eq!(c.work().phase.cache_hits, 0);
    assert_eq!(
        c.work().phase.native_attempts,
        f.work().phase.native_attempts
    );
}
#[test]
fn selected_edge_and_budget_refusal_are_transactional() {
    let mut m = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    let init = initial(2f64.powi(-26));
    let run = |m: &[ConservedMode], end, b| {
        evolve_robertson_phase_atlas(
            m,
            0,
            init,
            [0., end],
            &opts(),
            policy(),
            pm(MultiGuardRoute::Correlated),
            b,
            PhaseReuseRoute::Reuse,
        )
    };
    let base = run(&m, 0.002, atlas_budget()).unwrap();
    assert_eq!(base.stop(), PhaseAtlasStop::Horizon);
    assert!(base.events().iter().all(|e| e.event.guard == 0));
    let mut b = atlas_budget();
    b.reset_checks = 1;
    let r = run(&m, 0.002, b).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::ResetBudget);
    assert_eq!(r.final_mode(), 0);
    assert_eq!(r.work().phase.reset_checks, 1);
    assert!(!r.events()[0].event.committed);
    assert_eq!(r.epochs().len(), 1);
    b = atlas_budget();
    b.limits.transitions = 1;
    let r = run(&m, 0.002, b).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::TransitionBudget);
    assert_eq!(r.final_mode(), 1);
    b = atlas_budget();
    b.limits.min_dwell = 0.001;
    let r = run(&m, 0.002, b).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::DwellUnproved);
    assert_eq!(r.work().phase.committed, 1);
    b = atlas_budget();
    b.limits.absolute_width = base.events()[0].event.width_upper;
    let r = run(&m, 0.002, b).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::AbsoluteWidth);
    assert_eq!(r.work().phase.committed, 1);
    b = atlas_budget();
    b.limits.native_attempts = 1;
    let r = run(&m, 0.002, b).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::NativeBudget);
    assert!(r.work().phase.charged_native_attempts <= 1);
    b = atlas_budget();
    b.limits.scan_slabs = 1;
    let r = run(&m, 0.002, b).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::SlabBudget);
    assert!(r.work().phase.charged_scan_slabs <= 1);
    // Explicit target admission failure, not an unsupported graph shape.
    m[0].transitions[0].reset = constant(0.);
    let r = run(&m, 0.002, atlas_budget()).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::ResetRefused);
    assert_eq!(r.final_mode(), 0);
    assert!(!r.events()[0].event.committed);
    assert!(r.events()[0].reset_guards[0].enclosure[1] >= 0.);
    assert!(r.query(0, 0.0001, PhaseQueryRoute::Centered).is_ok());
    assert!(
        r.query(
            1,
            r.events()[0].event.absolute[1],
            PhaseQueryRoute::Centered
        )
        .is_err()
    );
    m = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    // An unreachable losing edge onto an active guard must not block the
    // actual selected edge; structural admission remains global.
    m[0].transitions[1].reset = constant(0.);
    let r = run(&m, 0.002, atlas_budget()).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::Horizon);
    m[0].guards[1] = m[0].guards[0].clone();
    let r = run(&m, 0.002, atlas_budget()).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::OrderUnresolved);
    assert!(r.events().is_empty());
}
#[test]
fn authority_boundary_and_horizon() {
    let mut m = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    let init = initial(2f64.powi(-26));
    let run = |m: &[ConservedMode], times| {
        evolve_robertson_phase_atlas(
            m,
            0,
            init,
            times,
            &opts(),
            policy(),
            pm(MultiGuardRoute::Correlated),
            atlas_budget(),
            PhaseReuseRoute::Reuse,
        )
    };
    // At the first horizon the complete root is not yet certified.
    // A separate second-event horizon has an authenticated relative
    // period but overlapping accumulated absolute clock.
    let r = run(&m, [0., 0.00034848]).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::OrderUnresolved);
    assert_eq!(r.work().phase.committed, 0);
    let r = run(&m, [0., 0.0005481714248]).unwrap();
    assert_eq!(
        r.stop(),
        PhaseAtlasStop::HorizonAmbiguous,
        "{:?}",
        r.reason()
    );
    assert_eq!(r.work().phase.committed, 1);
    let r = run(&m, [1., 1.002]).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::Horizon);
    let e = &r.epochs()[1].phase;
    let t = (e.clock[0] + e.clock[1]) * 0.5;
    assert!(r.query(1, t, PhaseQueryRoute::Horner).is_err());
    assert!(r.query(6, 1.002, PhaseQueryRoute::Centered).is_ok());
    m[0].guards[0] = h(0.001);
    assert!(run(&m, [0., 0.002]).is_err());
    m = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    m[0].transitions[0].reset = reset();
    assert!(run(&m, [0., 0.002]).is_err());
    m[0].transitions[0].reset = mixed();
    assert!(run(&m, [0., 0.002]).is_err());
    m = graph(
        "alternating",
        EvolutionRoute::WarpFolded,
        ConservationRoute::Integral4,
    );
    let r = run(&m, [0., 0.002]).unwrap();
    assert_eq!(r.stop(), PhaseAtlasStop::NativeRefused);
}
fn chain_limits() -> ConservedChainBudget {
    ConservedChainBudget {
        limits: ChainBudget {
            native_attempts: 4096,
            scan_slabs: 512,
            transition_attempts: 10,
            reset_cells: 64,
            gap_checks: 256,
            min_dwell: 1e-5,
        },
        picard_attempts: 512,
    }
}
#[test]
#[ignore = "fixed phase atlas development matrix"]
fn training() {
    use serde_json::json;
    for case in ["alternating", "distinct", "overlap"] {
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Prepared5,
            EvolutionRoute::Scalar5,
        ] {
            for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
                for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                    for method in ["JointCentered", "JointGreen", "AtlasFresh", "AtlasReuse"] {
                        let mut m = graph(case, solver, cert);
                        if method == "JointGreen" {
                            for a in &mut m {
                                for e in &mut a.transitions {
                                    e.policy.sync = VectorSyncRoute::GreenMoment;
                                }
                            }
                        }
                        let mut row = json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"radius":2f64.powi(-26)});
                        if method.starts_with("Atlas") {
                            let r = evolve_robertson_phase_atlas(
                                &m,
                                0,
                                initial(2f64.powi(-26)),
                                [0., 0.002],
                                &opts(),
                                policy(),
                                pm(order),
                                atlas_budget(),
                                if method == "AtlasReuse" {
                                    PhaseReuseRoute::Reuse
                                } else {
                                    PhaseReuseRoute::Fresh
                                },
                            )
                            .unwrap();
                            let mut qs = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                let ep = &e.phase;
                                if ep.absolute_safe <= ep.clock[1] {
                                    continue;
                                }
                                for j in 0..=4 {
                                    let t = if j == 4
                                        && r.stop() == PhaseAtlasStop::Horizon
                                        && ep.absolute_safe == 0.002
                                    {
                                        0.002
                                    } else {
                                        ep.clock[1]
                                            + (ep.absolute_safe - ep.clock[1])
                                                * ((j as f64 + 0.5) / 5.)
                                    };
                                    let a = r.query(i, t, PhaseQueryRoute::Horner).unwrap();
                                    let b = r.query(i, t, PhaseQueryRoute::Centered).unwrap();
                                    qs.push(json!({"epoch":i,"horner":a,"centered":b}));
                                }
                            }
                            row["atlas"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"phases":r.phases().iter().map(|p|json!({"mode":p.mode,"initial":p.initial,"winner":p.ordered().winner(),"safe":p.ordered().committed_until(),"reason":p.ordered().reason(),"guards":p.ordered().guards(),"comparisons":p.ordered().comparisons(),"work":p.ordered().work,"slabs":p.original_slab_diagnostics()})).collect::<Vec<_>>(),"epochs":r.epochs(),"events":r.events(),"queries":qs});
                        } else {
                            let r = evolve_conserved_chain(
                                &m,
                                0,
                                initial(2f64.powi(-26)),
                                [0., 0.002],
                                &opts(),
                                policy(),
                                pm(order),
                                chain_limits(),
                                SimplexGapRoute::AffineDual,
                            )
                            .unwrap();
                            let mut es = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                let qs = if e.safe() > e.start() {
                                    (0..=4)
                                        .filter_map(|j| {
                                            r.query(
                                                i,
                                                if j == 4 {
                                                    e.safe()
                                                } else {
                                                    e.start()
                                                        + (e.safe() - e.start()) * (j as f64 / 4.)
                                                },
                                            )
                                            .ok()
                                        })
                                        .collect::<Vec<_>>()
                                } else {
                                    vec![]
                                };
                                es.push(json!({"mode":e.mode(),"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"center":e.center(),"slabs":e.original_slab_diagnostics(),"queries":qs}));
                            }
                            row["joint"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"events":r.transitions(),"epochs":es});
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}
