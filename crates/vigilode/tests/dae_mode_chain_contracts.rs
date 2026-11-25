#![cfg(feature = "polynomial-events")]
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn tr(c: f64, factors: Vec<F>) -> T {
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
fn l(i: usize, c: f64) -> T {
    tr(c, vec![y(i)])
}
fn c(v: f64) -> T {
    tr(v, vec![])
}
fn clock(q: f64) -> P {
    p(vec![tr(1., vec![t()]), c(-q)])
}
fn xguard(q: f64) -> P {
    p(vec![l(0, 1.), c(-q)])
}
fn opts() -> Options {
    Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-12),
        h0: 0.125,
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
        tube_radius: 0.005,
        global_tol: 0.005,
        green: true,
    }
}
fn src(v: f64) -> DaeSource {
    DaeSource::new(
        vec![p(vec![c(v)])],
        vec![p(vec![l(1, 1.), l(0, -1.), tr(-1., vec![t()])])],
    )
    .unwrap()
}
fn mp() -> MultiGuardPolicy {
    MultiGuardPolicy {
        method: MultiGuardRoute::Correlated,
        tolerance: 1e-3,
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
fn rp() -> VectorResetPolicy {
    VectorResetPolicy {
        sync: VectorSyncRoute::Centered,
        derivative: GuardDerivativeRoute::Schur,
        branch_route: InitRoute::Reuse,
        subdivisions: 1,
    }
}
fn budget() -> ChainBudget {
    ChainBudget {
        native_attempts: 256,
        scan_slabs: 128,
        transition_attempts: 4,
        reset_cells: 64,
        gap_checks: 256,
        min_dwell: 0.01,
    }
}
fn mode(v: f64, guards: Vec<P>, transitions: Vec<ChainTransition>) -> ChainMode {
    ChainMode {
        source: src(v),
        z_seed: vec![0.],
        branch: vec![[-2., 5.]],
        route: EvolutionRoute::Mass,
        guards,
        transitions,
    }
}
fn edge(target: usize, r: P) -> ChainTransition {
    ChainTransition {
        target,
        reset: vec![r],
        policy: rp(),
    }
}
fn modes() -> Vec<ChainMode> {
    let q = 0.125;
    let d = 2f64.powi(-14);
    vec![
        mode(
            1.,
            vec![xguard(q + d), xguard(q)],
            vec![
                edge(2, p(vec![c(9.)])),
                edge(1, p(vec![c(0.25), l(0, 1.), l(1, -1.)])),
            ],
        ),
        mode(
            2.,
            vec![clock(0.375), clock(0.5)],
            vec![
                edge(2, p(vec![l(0, 1.), c(0.125)])),
                edge(0, p(vec![c(0.)])),
            ],
        ),
        mode(-1., vec![], vec![]),
    ]
}
fn run(m: &[ChainMode], b: ChainBudget, gap: GapGuardRoute) -> HybridChainRun {
    evolve_hybrid_chain(
        m,
        0,
        &[0.],
        &[0.],
        EvolutionEpoch {
            start: 0.,
            end: 0.625,
            incoming_x_error: Some(2f64.powi(-12)),
        },
        &opts(),
        policy(),
        mp(),
        b,
        gap,
    )
    .unwrap()
}
#[test]
fn three_modes_carry_clock_radius_and_order() {
    let m = modes();
    let r = run(&m, budget(), GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::Horizon);
    assert_eq!(r.transitions().len(), 2);
    assert_eq!(r.epochs().len(), 3);
    assert_eq!(r.transitions()[0].guard, 1);
    assert_eq!(r.transitions()[1].guard, 0);
    assert_eq!(r.final_mode(), 2);
    assert!(
        r.transitions()
            .iter()
            .all(|v| v.committed && v.gap.iter().all(|g| g.enclosure[1] < 0.))
    );
    assert!(
        r.transitions()[0]
            .reset
            .as_ref()
            .unwrap()
            .incoming_radius
            .unwrap()
            > 0.
    );
    let first = r.transitions()[0].event_time;
    assert!(r.query(0, first[1] + 1e-6).is_err());
    assert!(r.query(1, (first[0] + first[1]) / 2.).is_err());
    for i in 0..=64 {
        let d = -2f64.powi(-12) + 2. * 2f64.powi(-12) * i as f64 / 64.;
        let tau = 0.125 - d;
        for (k, time) in [(0, 0.0625), (1, 0.25), (2, 0.625)] {
            let x = match k {
                0 => time + d,
                1 => 0.25 - tau + 2. * (time - tau),
                _ => 0.25 - tau + 2. * (0.375 - tau) + 0.125 - (time - 0.375),
            };
            let q = r.query(k, time).unwrap();
            let err = (q.state[0] - x).abs().max((q.state[1] - (x + time)).abs());
            assert!(err <= q.physical_error_upper.unwrap() + 1e-14);
        }
    }
    assert!(r.work().charged_native_attempts <= budget().native_attempts);
}

fn separated() -> Vec<ChainMode> {
    let mut m = modes();
    m[0].guards[0] = xguard(0.5);
    m
}
fn gap_cancel() -> Vec<ChainMode> {
    let mut m = separated();
    let eps = 2f64.powi(-12);
    m[1].guards[0] = p(vec![
        l(1, 1.),
        l(0, -1.),
        tr(-1., vec![t()]),
        tr(eps, vec![F::clock(0.375)]),
    ]);
    m
}
fn gap_excursion() -> Vec<ChainMode> {
    let mut m = separated();
    let q = 0.125;
    let r = 2f64.powi(-12);
    m[1].guards[0] = p(vec![c(r * r / 4.), tr(-1., vec![F::clock(q), F::clock(q)])]);
    m
}
fn gap_rank() -> Vec<ChainMode> {
    let mut m = separated();
    m[1].source = DaeSource::new(
        vec![p(vec![c(2.)])],
        vec![p(vec![tr(1., vec![y(1), y(1)]), l(0, -1.), c(0.25)])],
    )
    .unwrap();
    m[1].branch = vec![[0.01, 1.5]];
    m[1].z_seed = vec![0.25];
    m
}
fn near() -> Vec<ChainMode> {
    let mut m = separated();
    m[1].guards[0] = clock(0.125 + 2f64.powi(-9));
    m
}
fn tied() -> Vec<ChainMode> {
    let mut m = separated();
    m[0].guards = vec![clock(0.125), clock(0.125)];
    m
}
fn pencil() -> Vec<ChainMode> {
    let mut m = separated();
    m[0].guards = vec![clock(0.5), clock(0.125)];
    m[0].transitions[1].reset = vec![p(vec![c(0.25), tr(-1., vec![t()])])];
    for (i, v) in [1., 2., -1.].into_iter().enumerate() {
        m[i].source = DaeSource::new(
            vec![p(vec![c(v)])],
            vec![p(vec![l(1, 1.), tr(0.125, vec![t(), y(1)]), l(0, -1.)])],
        )
        .unwrap();
    }
    m
}
#[test]
fn synchronization_gap_refusal_and_schur_recovery() {
    let m = gap_excursion();
    for route in [GapGuardRoute::Box, GapGuardRoute::Schur] {
        let r = run(&m, budget(), route);
        assert_eq!(r.stop(), ChainStop::GapUnproved);
        assert_eq!(r.final_mode(), 0);
        assert_eq!(r.epochs().len(), 1);
        let e = &r.transitions()[0];
        assert!(!e.committed && e.reset.is_some());
        let rad = 2f64.powi(-12);
        let s = e.event_time[1];
        assert!(rad * rad / 4. - (s - 0.125).powi(2) < 0.);
        assert!(e.event_time[0] < 0.125 && s > 0.125); // H(q)>0 inside the true earliest path.
        assert!(r.query(0, 0.0625).is_ok());
        assert!(r.query(1, s).is_err());
    }
    let m = gap_cancel();
    let a = run(&m, budget(), GapGuardRoute::Box);
    assert_eq!(a.stop(), ChainStop::GapUnproved);
    let b = run(&m, budget(), GapGuardRoute::Schur);
    assert_eq!(b.stop(), ChainStop::Horizon);
    assert!(
        b.transitions()[0]
            .gap
            .iter()
            .any(|g| g.evidence.direct[1] >= 0. && g.enclosure[1] < 0.)
    );
}
#[test]
fn cumulative_budget_dwell_and_branch_refusals() {
    let m = separated();
    let mut b = budget();
    b.transition_attempts = 1;
    let r = run(&m, b, GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::TransitionBudget);
    assert_eq!(r.final_mode(), 1);
    assert!(r.transitions()[0].committed && !r.transitions()[1].committed);
    assert!(r.query(0, 0.0625).is_ok() && r.query(1, 0.25).is_ok());
    b = budget();
    b.gap_checks = 1;
    let r = run(&m, b, GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::GapBudget);
    assert_eq!(r.work().gap_checks, 1);
    assert_eq!(r.final_mode(), 0);
    b = budget();
    b.reset_cells = 1;
    let r = run(&m, b, GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::ResetBudget);
    assert_eq!(r.work().charged_reset_cells, 1);
    assert_eq!(r.final_mode(), 0);
    b = budget();
    b.native_attempts = 2;
    let r = run(&m, b, GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::NativeBudget);
    assert_eq!(r.work().charged_native_attempts, 2);
    b = budget();
    b.scan_slabs = 2;
    let r = run(&m, b, GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::SlabBudget);
    assert_eq!(r.work().charged_scan_slabs, 2);
    let r = run(&near(), budget(), GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::DwellUnproved);
    assert_eq!(r.final_mode(), 1);
    assert!(
        r.transitions()[1].dwell_lower.unwrap() < budget().min_dwell
            && !r.transitions()[1].committed
    );
    let r = run(&gap_rank(), budget(), GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::ResetRefused);
    assert_eq!(r.final_mode(), 0);
    assert!(r.query(0, 0.0625).is_ok());
    let r = run(&tied(), budget(), GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::OrderUnresolved);
    assert!(r.transitions().is_empty());
}
#[test]
fn actual_warp_chain_and_repeated_mode() {
    let mut m = pencil();
    for v in &mut m {
        v.route = EvolutionRoute::WarpFolded;
    }
    let r = run(&m, budget(), GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::Horizon);
    assert_eq!(r.final_mode(), 2);
    assert!(r.work().warp_predictions > 0);
    assert_eq!(r.transitions()[0].event_time, [0.125, 0.125]);
    assert_eq!(r.transitions()[1].event_time, [0.375, 0.375]);
    // A genuine cyclic mode graph with reset, no special terminal.
    let mut m = separated();
    m[0].guards = vec![xguard(0.125), xguard(0.5)];
    m[0].transitions = vec![edge(0, p(vec![c(0.)])), edge(0, p(vec![c(0.)]))];
    let r = evolve_hybrid_chain(
        &m,
        0,
        &[0.],
        &[0.],
        EvolutionEpoch {
            start: 0.,
            end: 0.55,
            incoming_x_error: Some(0.),
        },
        &opts(),
        policy(),
        mp(),
        budget(),
        GapGuardRoute::Schur,
    )
    .unwrap();
    assert_eq!(r.stop(), ChainStop::Horizon);
    assert_eq!(r.work().transitions_committed, 4);
    assert!(
        r.transitions()
            .iter()
            .skip(1)
            .all(|e| e.dwell_lower.unwrap() >= budget().min_dwell)
    );
}
#[test]
#[ignore = "finite chained development campaign"]
fn training() {
    use serde_json::json;
    for (name, build) in [
        ("separated", separated as fn() -> Vec<ChainMode>),
        ("overlap", modes),
        ("gap-cancel", gap_cancel),
        ("gap-excursion", gap_excursion),
        ("near-dwell", near),
        ("rank-loss", gap_rank),
        ("tie", tied),
        ("pencil", pencil),
    ] {
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Prepared5,
            EvolutionRoute::Scalar5,
            EvolutionRoute::WarpFolded,
        ] {
            for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                for sync in [
                    VectorSyncRoute::BoxPicard,
                    VectorSyncRoute::Centered,
                    VectorSyncRoute::GreenMoment,
                ] {
                    for gap in [GapGuardRoute::Box, GapGuardRoute::Schur] {
                        let mut m = build();
                        for v in &mut m {
                            v.route = solver;
                            for e in &mut v.transitions {
                                e.policy.sync = sync;
                                e.policy.branch_route = if solver == EvolutionRoute::Prepared5 {
                                    InitRoute::PreparedBatch
                                } else if solver == EvolutionRoute::Scalar5 {
                                    InitRoute::ScalarRoot
                                } else {
                                    InitRoute::Reuse
                                };
                            }
                        }
                        let mut pol = mp();
                        pol.method = order;
                        let r = evolve_hybrid_chain(
                            &m,
                            0,
                            &[0.],
                            &[0.],
                            EvolutionEpoch {
                                start: 0.,
                                end: 0.625,
                                incoming_x_error: Some(2f64.powi(-12)),
                            },
                            &opts(),
                            policy(),
                            pol,
                            budget(),
                            gap,
                        )
                        .unwrap();
                        let mut epochs = vec![];
                        for (i, e) in r.epochs().iter().enumerate() {
                            let qs = if e.safe > e.start {
                                (0..=8)
                                    .filter_map(|k| {
                                        let t = if k == 8 {
                                            e.safe
                                        } else {
                                            e.start + (e.safe - e.start) * k as f64 / 8.
                                        };
                                        r.query(i, t).ok()
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                vec![]
                            };
                            epochs.push(json!({"mode":e.mode,"start":e.start,"safe":e.safe,"radius":e.incoming_radius,"queries":qs,"ordered":e.ordered().map(|a|json!({"winner":a.winner(),"reason":a.reason(),"guards":a.guards(),"comparisons":a.comparisons(),"work":a.work})),"terminal":e.terminal().map(|t|json!({"attempts":t.driver().stats.attempts,"bound":t.max_physical_error,"work":t.work,"first_incoming":t.slabs()[0].certificate.as_ref().unwrap().incoming}))}));
                        }
                        println!(
                            "CELL {}",
                            json!({"case":name,"solver":solver,"order":order,"sync":sync,"gap_route":gap,"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"transitions":r.transitions(),"epochs":epochs})
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn dimension_changing_nonlinear_chain() {
    let mut m = modes();
    let q = 0.125;
    let rad = 2f64.powi(-12);
    m[0].transitions[1].reset = vec![
        p(vec![c(0.75), l(1, 1.), l(0, -1.)]),
        p(vec![c(1.), tr(1., vec![y(0), y(0)])]),
    ];
    m[1].source = DaeSource::new(
        vec![p(vec![tr(-1., vec![y(2), y(2)])]), p(vec![l(2, 1.)])],
        vec![p(vec![l(2, 1.), l(0, -1.)])],
    )
    .unwrap();
    m[1].z_seed = vec![0.875];
    m[1].branch = vec![[0.5, 1.5]];
    m[1].transitions[0].reset = vec![p(vec![l(0, 1.), l(1, 1.)])];
    m[1].route = EvolutionRoute::Prepared5;
    m[0].transitions[1].policy.branch_route = InitRoute::PreparedBatch;
    m[1].transitions[0].policy.sync = VectorSyncRoute::GreenMoment;
    let r = run(&m, budget(), GapGuardRoute::Schur);
    assert_eq!(r.stop(), ChainStop::Horizon);
    assert_eq!(r.epochs().len(), 3);
    assert_eq!(r.transitions().len(), 2);
    assert_eq!(
        r.transitions()[0]
            .reset
            .as_ref()
            .unwrap()
            .synchronized_x
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        r.transitions()[1]
            .reset
            .as_ref()
            .unwrap()
            .synchronized_x
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    let f = |tau: f64, t: f64| {
        let x0 = 0.75 + tau;
        let dt = t - tau;
        (x0 / (1. + x0 * dt), 1. + q * q + (x0 * dt).ln_1p())
    };
    for i in 0..=64 {
        let delta = -rad + 2. * rad * i as f64 / 64.;
        let tau = q - delta;
        let p = r.query(1, 0.25).unwrap();
        let (x, y) = f(tau, 0.25);
        assert!(
            (p.state[0] - x)
                .abs()
                .max((p.state[1] - y).abs())
                .max((p.state[2] - x).abs())
                <= p.physical_error_upper.unwrap() + 1e-14
        );
        let p = r.query(2, 0.625).unwrap();
        let (x, y) = f(tau, 0.375);
        let x = x + y - 0.25;
        assert!(
            (p.state[0] - x).abs().max((p.state[1] - x - 0.625).abs())
                <= p.physical_error_upper.unwrap() + 1e-14
        );
    }
    println!(
        "DIMENSION_CHAIN {}",
        serde_json::json!({"events":r.transitions(),"work":r.work(),"queries":[r.query(1,0.25).unwrap(),r.query(2,0.625).unwrap()]})
    );
}
