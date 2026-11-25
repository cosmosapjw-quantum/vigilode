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
#[test]
fn phase_resolves_original_wrapping_case() {
    let m = cycle(
        false,
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        VectorSyncRoute::Centered,
    );
    let r = evolve_robertson_phase_cycle(
        &m,
        initial(2f64.powi(-26)),
        [0., 0.001],
        &opts(),
        policy(),
        pm(MultiGuardRoute::Correlated),
        limits(),
        PhaseReuseRoute::Reuse,
    )
    .unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::Horizon, "{:?}", r.reason());
    assert_eq!(r.events().iter().filter(|e| e.committed).count(), 2);
    assert_eq!(r.phases().len(), 2);
    assert_eq!(r.epochs().len(), 3);
    assert_eq!(r.work().cache_hits, 1);
    assert_eq!(r.phases()[1].initial.l1_radius(), 0.);
    assert!(
        r.events()
            .iter()
            .all(|e| e.absolute[1] - e.absolute[0] <= 1e-6)
    );
    let a = r.query(2, 0.001, PhaseQueryRoute::Centered).unwrap();
    assert!(a.phase[0] < a.phase[1] && a.enclosure.iter().all(|x| x[0] <= x[1]));
    assert!(
        r.query(0, r.events()[0].absolute[1], PhaseQueryRoute::Centered)
            .is_err()
    );
}

#[test]
fn phase_admission_and_transaction_limits() {
    let mut m = cycle(
        false,
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        VectorSyncRoute::Centered,
    );
    let init = initial(2f64.powi(-26));
    let pol = pm(MultiGuardRoute::Correlated);
    let run = |m: &ConservedMode, end, b, reuse| {
        evolve_robertson_phase_cycle(m, init, [0., end], &opts(), policy(), pol, b, reuse)
    };
    let base = run(&m, 0.001, limits(), PhaseReuseRoute::Reuse).unwrap();
    let fresh = run(&m, 0.001, limits(), PhaseReuseRoute::Fresh).unwrap();
    assert_eq!(fresh.stop(), PhaseCycleStop::Horizon);
    assert_eq!(fresh.phases().len(), 3);
    assert_eq!(fresh.work().cache_hits, 0);
    assert!(base.work().native_attempts < fresh.work().native_attempts);
    assert_eq!(
        serde_json::to_value(base.events())
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for (x, y) in base.events().iter().zip(fresh.events()) {
        assert_eq!(x.absolute, y.absolute);
        assert_eq!(x.relative, y.relative);
    }
    let mut b = limits();
    b.transitions = 1;
    let r = run(&m, 0.001, b, PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::TransitionBudget);
    assert_eq!(r.work().committed, 1);
    assert!(r.query(1, 0.0005, PhaseQueryRoute::Horner).is_ok());
    b = limits();
    b.min_dwell = 0.0004;
    let r = run(&m, 0.001, b, PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::DwellUnproved);
    assert_eq!(r.work().committed, 1);
    b = limits();
    b.absolute_width = base.events()[0].width_upper;
    let r = run(&m, 0.001, b, PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::AbsoluteWidth);
    assert_eq!(r.work().committed, 1);
    b = limits();
    b.native_attempts = 1;
    let r = run(&m, 0.001, b, PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::NativeBudget);
    assert!(r.work().charged_native_attempts <= 1);
    b = limits();
    b.scan_slabs = 1;
    let r = run(&m, 0.001, b, PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::SlabBudget);
    assert!(r.work().charged_scan_slabs <= 1);
    let r = run(&m, 0.0006969678557, limits(), PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(
        r.stop(),
        PhaseCycleStop::HorizonAmbiguous,
        "{:?}",
        r.reason()
    );
    assert_eq!(r.work().committed, 1);
    let g = m.guards[0].clone();
    m.guards[1] = g;
    let r = run(&m, 0.001, limits(), PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::OrderUnresolved);
    assert_eq!(r.work().committed, 0);
    m = cycle(
        false,
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        VectorSyncRoute::Centered,
    );
    m.guards[0] = h(0.001);
    assert!(run(&m, 0.001, limits(), PhaseReuseRoute::Reuse).is_err());
    m = cycle(
        false,
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        VectorSyncRoute::Centered,
    );
    m.transitions[0].reset = reset();
    assert!(run(&m, 0.001, limits(), PhaseReuseRoute::Reuse).is_err());
    m.transitions[0].reset = mixed();
    assert!(run(&m, 0.001, limits(), PhaseReuseRoute::Reuse).is_err());
    m = cycle(
        false,
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        VectorSyncRoute::Centered,
    );
    for e in &mut m.transitions {
        e.reset = SimplexReset::new([[1.; 3], [0.; 3], [0.; 3]]).unwrap();
    }
    assert!(run(&m, 0.001, limits(), PhaseReuseRoute::Reuse).is_err());
    m = cycle(
        false,
        EvolutionRoute::WarpFolded,
        ConservationRoute::Integral4,
        VectorSyncRoute::Centered,
    );
    let r = run(&m, 0.001, limits(), PhaseReuseRoute::Reuse).unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::NativeRefused);
    assert!(RobertsonInitial::exact([1., 1e-15]).is_err());
}
#[test]
fn absolute_clock_translation_and_query_enclosures() {
    let m = cycle(
        false,
        EvolutionRoute::Scalar5,
        ConservationRoute::Integral4,
        VectorSyncRoute::Centered,
    );
    let r = evolve_robertson_phase_cycle(
        &m,
        initial(2f64.powi(-26)),
        [1., 1.001],
        &opts(),
        policy(),
        pm(MultiGuardRoute::Correlated),
        limits(),
        PhaseReuseRoute::Reuse,
    )
    .unwrap();
    assert_eq!(r.stop(), PhaseCycleStop::Horizon);
    for e in r.events() {
        assert!(e.absolute[0] > 1. && e.absolute[1] - e.absolute[0] <= 1e-6);
    }
    let e = &r.epochs()[1];
    let t = (e.clock[1] + e.absolute_safe) * 0.5;
    let a = r.query(1, t, PhaseQueryRoute::Horner).unwrap();
    let b = r.query(1, t, PhaseQueryRoute::Centered).unwrap();
    assert_eq!(a.phase, b.phase);
    assert!(
        a.enclosure
            .iter()
            .zip(b.enclosure)
            .all(|(a, b)| a[0] <= b[0] && b[1] <= a[1])
    );
    assert!(
        r.query(
            1,
            (e.clock[0] + e.clock[1]) * 0.5,
            PhaseQueryRoute::Centered
        )
        .is_err()
    );
    assert!(
        r.query(0, r.events()[0].absolute[1], PhaseQueryRoute::Horner)
            .is_err()
    );
    assert!(r.query(2, 1.001, PhaseQueryRoute::Centered).is_ok());
}
fn chain_limits() -> ConservedChainBudget {
    ConservedChainBudget {
        limits: ChainBudget {
            native_attempts: 4096,
            scan_slabs: 512,
            transition_attempts: 3,
            reset_cells: 64,
            gap_checks: 256,
            min_dwell: 1e-5,
        },
        picard_attempts: 512,
    }
}
fn rec_phase(p: &RelativePhase) -> serde_json::Value {
    let a = p.ordered();
    serde_json::json!({"initial":p.initial,"winner":a.winner(),"safe":a.committed_until(),"reason":a.reason(),"guards":a.guards(),"comparisons":a.comparisons(),"work":a.work,"slabs":p.original_slab_diagnostics()})
}
#[test]
#[ignore = "fixed phase versus synchronization development matrix"]
fn training() {
    use serde_json::json;
    for case in ["wide", "narrow", "overlap"] {
        let rad = if case == "narrow" {
            2f64.powi(-30)
        } else {
            2f64.powi(-26)
        };
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Prepared5,
            EvolutionRoute::Scalar5,
        ] {
            for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
                for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                    for method in ["JointCentered", "JointGreen", "PhaseFresh", "PhaseReuse"] {
                        let sync = if method == "JointGreen" {
                            VectorSyncRoute::GreenMoment
                        } else {
                            VectorSyncRoute::Centered
                        };
                        let m = cycle(case == "overlap", solver, cert, sync);
                        let mut row = json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"radius":rad});
                        if method.starts_with("Phase") {
                            let r = evolve_robertson_phase_cycle(
                                &m,
                                initial(rad),
                                [0., 0.001],
                                &opts(),
                                policy(),
                                pm(order),
                                limits(),
                                if method == "PhaseReuse" {
                                    PhaseReuseRoute::Reuse
                                } else {
                                    PhaseReuseRoute::Fresh
                                },
                            )
                            .unwrap();
                            let mut qs = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                if e.absolute_safe <= e.clock[1] {
                                    continue;
                                }
                                for j in 0..=8 {
                                    let t = if j == 8
                                        && r.stop() == PhaseCycleStop::Horizon
                                        && e.absolute_safe == 0.001
                                    {
                                        0.001
                                    } else {
                                        e.clock[1]
                                            + (e.absolute_safe - e.clock[1])
                                                * ((j as f64 + 0.5) / 9.)
                                    };
                                    let a = r.query(i, t, PhaseQueryRoute::Horner).unwrap();
                                    let b = r.query(i, t, PhaseQueryRoute::Centered).unwrap();
                                    qs.push(json!({"epoch":i,"horner":a,"centered":b}));
                                }
                            }
                            row["phase"] = json!({"stop":r.stop(),"reason":r.reason(),"work":r.work(),"reset_center":r.reset_center(),"reset_guards":r.reset_guards(),"phases":r.phases().iter().map(rec_phase).collect::<Vec<_>>(),"epochs":r.epochs(),"events":r.events(),"queries":qs});
                        } else {
                            let r = evolve_conserved_chain(
                                &[m],
                                0,
                                initial(rad),
                                [0., 0.001],
                                &opts(),
                                policy(),
                                pm(order),
                                chain_limits(),
                                SimplexGapRoute::AffineDual,
                            )
                            .unwrap();
                            let mut ep = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                let qs = if e.safe() > e.start() {
                                    (0..=8)
                                        .filter_map(|j| {
                                            r.query(
                                                i,
                                                if j == 8 {
                                                    e.safe()
                                                } else {
                                                    e.start()
                                                        + (e.safe() - e.start()) * (j as f64 / 8.)
                                                },
                                            )
                                            .ok()
                                        })
                                        .collect::<Vec<_>>()
                                } else {
                                    vec![]
                                };
                                ep.push(json!({"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"center":e.center(),"slabs":e.original_slab_diagnostics(),"queries":qs,"ordered":e.ordered().map(|a|json!({"winner":a.winner(),"reason":a.reason(),"guards":a.guards(),"comparisons":a.comparisons(),"work":a.work}))}));
                            }
                            row["joint"] = json!({"stop":r.stop(),"reason":r.reason(),"work":r.work(),"events":r.transitions(),"epochs":ep});
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}
