#![cfg(feature = "polynomial-events")]
use rodas5p_core::{directed::Interval as Iv, retro_dictionary::RealMode};
use rodas5p_integrators::{retro_dae_evolution as de, retro_green::RetroTable};
use serde::Serialize;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Counts {
    alloc: u64,
    zeroed: u64,
    realloc: u64,
    requested_bytes: u64,
    dealloc: u64,
    dealloc_bytes: u64,
}
impl Counts {
    fn requests(self) -> u64 {
        self.alloc + self.zeroed + self.realloc
    }
    fn add(&mut self, o: Self) {
        self.alloc += o.alloc;
        self.zeroed += o.zeroed;
        self.realloc += o.realloc;
        self.requested_bytes += o.requested_bytes;
        self.dealloc += o.dealloc;
        self.dealloc_bytes += o.dealloc_bytes;
    }
}
thread_local! {
    static TRACK:Cell<bool>=const{Cell::new(false)};
    static COUNT:Cell<Counts>=const{Cell::new(Counts{alloc:0,zeroed:0,realloc:0,requested_bytes:0,dealloc:0,dealloc_bytes:0})};
}
fn record(kind: u8, bytes: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|v| {
            let mut c = v.get();
            match kind {
                0 => {
                    c.alloc += 1;
                    c.requested_bytes += bytes as u64;
                }
                1 => {
                    c.zeroed += 1;
                    c.requested_bytes += bytes as u64;
                }
                2 => {
                    c.realloc += 1;
                    c.requested_bytes += bytes as u64;
                }
                _ => {
                    c.dealloc += 1;
                    c.dealloc_bytes += bytes as u64;
                }
            }
            v.set(c);
        });
    }
}
struct CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(0, l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(1, l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(2, n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        record(3, l.size());
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator;
struct Tracking;
impl Drop for Tracking {
    fn drop(&mut self) {
        TRACK.with(|v| v.set(false));
    }
}
fn measure<T>(f: impl FnOnce() -> T) -> (T, Counts) {
    assert!(!TRACK.with(Cell::get));
    COUNT.with(|v| v.set(Counts::default()));
    TRACK.with(|v| v.set(true));
    let guard = Tracking;
    let r = f();
    drop(guard);
    (r, COUNT.with(Cell::get))
}
#[derive(Default, Serialize)]
struct QueryWork {
    calls: usize,
    refused: usize,
    cells: usize,
    allocation: Counts,
}
impl QueryWork {
    fn query<T>(
        &mut self,
        f: impl FnOnce() -> Result<T, rodas5p_core::CoreError>,
    ) -> Result<T, rodas5p_core::CoreError> {
        let (r, a) = measure(f);
        self.calls += 1;
        self.refused += usize::from(r.is_err());
        self.allocation.add(a);
        r
    }
}
#[test]
fn allocator_positive_and_zero_control() {
    // Explicit direct calls with escaping pointer; no optimization-dependent Vec count.
    let (_, c) = measure(|| unsafe {
        let l = Layout::from_size_align(64, 8).unwrap();
        let p = std::alloc::alloc(l);
        assert!(!p.is_null());
        black_box(p);
        p.write(7);
        let q = std::alloc::realloc(p, l, 128);
        assert!(!q.is_null());
        black_box(q);
        assert_eq!(q.read(), 7);
        std::alloc::dealloc(q, Layout::from_size_align(128, 8).unwrap());
        let z = std::alloc::alloc_zeroed(l);
        assert!(!z.is_null());
        black_box(z);
        assert_eq!(z.read(), 0);
        std::alloc::dealloc(z, l);
    });
    assert_eq!(
        c,
        Counts {
            alloc: 1,
            zeroed: 1,
            realloc: 1,
            requested_bytes: 256,
            dealloc: 2,
            dealloc_bytes: 192
        }
    );
    let (_, z) = measure(|| black_box([1., 2., 3.]));
    assert_eq!(z, Counts::default());
    println!(
        "CONTROL {}",
        serde_json::json!({"positive":c,"stack_zero":z})
    );
}

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
#[ignore = "fixed resource audit of identical phase atlas matrix"]
fn training() {
    warm();
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
                            let (r, run_alloc) = measure(|| {
                                evolve_robertson_phase_atlas(
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
                            });
                            let r = r.unwrap();
                            let mut hwork = QueryWork::default();
                            let mut cwork = QueryWork::default();
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
                                    let a = hwork
                                        .query(|| r.query(i, t, PhaseQueryRoute::Horner))
                                        .unwrap();
                                    hwork.cells += a.cells.len();
                                    let b = cwork
                                        .query(|| r.query(i, t, PhaseQueryRoute::Centered))
                                        .unwrap();
                                    cwork.cells += b.cells.len();
                                    qs.push(json!({"epoch":i,"horner":a,"centered":b}));
                                }
                            }
                            let driver_stats = r
                                .phases()
                                .iter()
                                .map(|p| p.ordered().event_authority().map(|a| a.stats()))
                                .collect::<Vec<_>>();
                            println!(
                                "RESOURCE {}",
                                json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"solve_alloc":run_alloc,"horner":hwork,"centered":cwork,"driver_stats":driver_stats})
                            );
                            row["atlas"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"phases":r.phases().iter().map(|p|json!({"mode":p.mode,"initial":p.initial,"winner":p.ordered().winner(),"safe":p.ordered().committed_until(),"reason":p.ordered().reason(),"guards":p.ordered().guards(),"comparisons":p.ordered().comparisons(),"work":p.ordered().work,"slabs":p.original_slab_diagnostics()})).collect::<Vec<_>>(),"epochs":r.epochs(),"events":r.events(),"queries":qs});
                        } else {
                            let (r, run_alloc) = measure(|| {
                                evolve_conserved_chain(
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
                            });
                            let r = r.unwrap();
                            let mut qwork = QueryWork::default();
                            let mut es = vec![];
                            for (i, e) in r.epochs().iter().enumerate() {
                                let qs = if e.safe() > e.start() {
                                    (0..=4)
                                        .filter_map(|j| {
                                            qwork
                                                .query(|| {
                                                    r.query(
                                                        i,
                                                        if j == 4 {
                                                            e.safe()
                                                        } else {
                                                            e.start()
                                                                + (e.safe() - e.start())
                                                                    * (j as f64 / 4.)
                                                        },
                                                    )
                                                })
                                                .ok()
                                        })
                                        .collect::<Vec<_>>()
                                } else {
                                    vec![]
                                };
                                es.push(json!({"mode":e.mode(),"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"center":e.center(),"slabs":e.original_slab_diagnostics(),"queries":qs}));
                            }
                            let driver_stats = r
                                .epochs()
                                .iter()
                                .map(|e| {
                                    e.ordered()
                                        .and_then(|a| a.event_authority())
                                        .map(|a| a.stats())
                                })
                                .collect::<Vec<_>>();
                            println!(
                                "RESOURCE {}",
                                json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"solve_alloc":run_alloc,"point":qwork,"driver_stats":driver_stats})
                            );
                            row["joint"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"events":r.transitions(),"epochs":es});
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}

fn warm() {
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
    ] {
        let m = graph("alternating", solver, ConservationRoute::Integral4);
        let _ = evolve_robertson_phase_atlas(
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
    }
    black_box(RetroTable::with_dictionary(RealMode::Hybrid).unwrap());
}
fn init_route(solver: EvolutionRoute) -> InitRoute {
    match solver {
        EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
        EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
        _ => InitRoute::Reuse,
    }
}
fn power(s: &SlabRecord) -> Vec<Vec<Iv>> {
    s.power
        .iter()
        .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
        .collect()
}
fn same<T: Serialize, U: Serialize>(a: &T, b: &U) {
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
}
fn component(
    case: &str,
    solver: EvolutionRoute,
    cert: ConservationRoute,
    op: &str,
    alloc: Counts,
    detail: serde_json::Value,
) {
    println!(
        "COMPONENT {}",
        serde_json::json!({"case":case,"solver":solver,"cert":cert,"operation":op,"allocation":alloc,"detail":detail})
    );
}
#[test]
#[ignore = "authentic native slab and reset component allocation replay"]
fn component_replay() {
    use serde_json::json;
    warm();
    const BRANCH: [[f64; 2]; 1] = [[-0.25, 1.25]];
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let m = graph("alternating", solver, cert);
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
            assert_eq!(r.stop(), PhaseAtlasStop::Horizon);
            for phase in r.phases().iter().take(2) {
                let model = RobertsonModel::new(m[phase.mode].rate).unwrap();
                let src = model.source();
                let ir = init_route(solver);
                let init = src
                    .initialize(
                        0.,
                        &phase.initial.center(),
                        &[0.125],
                        &BRANCH,
                        policy().init,
                        ir,
                    )
                    .unwrap();
                let slabs = phase.original_slab_diagnostics();
                for (tag, k) in [
                    ("first", 0),
                    ("middle", slabs.len() / 2),
                    ("last", slabs.len() - 1),
                ] {
                    let s = &slabs[k];
                    let c = s.certificate.as_ref().unwrap();
                    assert!(c.global_qualified);
                    let detail = json!({"mode":phase.mode,"slab":k,"position":tag,"times":s.times});
                    let (p, a) = measure(|| {
                        de::dense_power(
                            &s.native.y0,
                            &s.native.y1,
                            &s.native.stages,
                            s.native.basis,
                        )
                    });
                    let mut p = p.unwrap();
                    component("slab", solver, cert, "dense_power", a, detail.clone());
                    if solver != EvolutionRoute::Mass {
                        let (z, a) = measure(|| {
                            de::reconstruct(
                                src,
                                s.times,
                                &p,
                                &init.z,
                                &BRANCH,
                                policy().init,
                                ir,
                                None,
                                5,
                            )
                        });
                        let (z, w) = z.unwrap();
                        let mut d = detail.clone();
                        d["init_work"] = json!(w);
                        component("slab", solver, cert, "reconstruct", a, d);
                        p.extend(z);
                    }
                    same(
                        &p.iter()
                            .map(|row| row.iter().map(|v| v.as_pair()).collect::<Vec<_>>())
                            .collect::<Vec<_>>(),
                        &s.power,
                    );
                    let (proof, a) = measure(|| {
                        model.certify(
                            s.times,
                            &p,
                            &BRANCH,
                            policy().tube_radius,
                            Some(c.incoming),
                            policy().init.condition_cap,
                            cert,
                        )
                    });
                    let proof = proof.unwrap();
                    same(&proof, c);
                    let mut d = detail.clone();
                    d["subcells"] = json!(proof.conservation.as_ref().unwrap().cells.len());
                    component("slab", solver, cert, "residual", a, d);
                    let guard = &m[phase.mode].guards[0];
                    let (g, a) = measure(|| {
                        de::GuardProof::new(
                            src,
                            guard,
                            &p,
                            c,
                            GuardDerivativeRoute::ConservedSchur,
                            GuardValueRoute::Transport,
                        )
                    });
                    let g = g.unwrap();
                    let mut d = detail.clone();
                    d["prepared_work"] = json!(g.evidence.prepared_work);
                    component("slab", solver, cert, "guard_build", a, d);
                    if let Some(s0) = phase.ordered().guards()[0]
                        .slabs
                        .iter()
                        .find(|v| v.times == s.times)
                    {
                        let mut a0 = serde_json::to_value(&g.evidence).unwrap();
                        let mut b0 = serde_json::to_value(s0).unwrap();
                        a0["evaluations"] = json!([]);
                        b0["evaluations"] = json!([]);
                        assert_eq!(a0, b0);
                        if let Some(v) = s0.evaluations.first() {
                            let (v0, a) = measure(|| g.value_at(v.time, v.differential_radius));
                            same(&v0.unwrap().as_pair(), &v.value);
                            component("slab", solver, cert, "guard_value", a, detail.clone());
                        }
                    }
                    // Exact native differential state. For reduced routes this is
                    // precisely the de::project invoked by Engine::fixed in callbacks.
                    let seed = if solver == EvolutionRoute::Mass {
                        &s.native.y0[2..]
                    } else {
                        &init.z
                    };
                    let (pr, a) = measure(|| {
                        de::project(
                            src,
                            s.times[0],
                            &s.native.y0[..2],
                            seed,
                            &BRANCH,
                            policy().init,
                            ir,
                            None,
                        )
                    });
                    let pr = pr.unwrap();
                    let mut d = detail.clone();
                    d["init_work"] = json!(pr.work);
                    component("slab", solver, cert, "project", a, d);
                    if solver == EvolutionRoute::Mass {
                        let prepared = vigilode::polynomial_events::initialize_mass_problem(
                            src,
                            0.,
                            &phase.initial.center(),
                            &[0.125],
                            &BRANCH,
                            policy().init,
                            ir,
                        )
                        .unwrap();
                        let mut dy = [0.; 3];
                        let mut jac = [0.; 9];
                        let (x, a) = measure(|| {
                            (prepared.problem.rhs_fn())(s.times[0], &s.native.y0, &mut dy)
                        });
                        x.unwrap();
                        black_box(dy);
                        same(&dy, &src.rhs_values(s.times[0], &s.native.y0).unwrap());
                        component("slab", solver, cert, "mass_rhs_callback", a, detail.clone());
                        let vigilode::Jacobian::Dense(j) = prepared.problem.jacobian_provider()
                        else {
                            panic!("mass Jacobian provider")
                        };
                        let (x, a) = measure(|| j(s.times[0], &s.native.y0, &mut jac));
                        x.unwrap();
                        black_box(jac);
                        same(
                            &jac,
                            &src.jacobian_values(s.times[0], &s.native.y0).unwrap(),
                        );
                        component(
                            "slab",
                            solver,
                            cert,
                            "mass_jacobian_callback",
                            a,
                            detail.clone(),
                        );
                        let (x, a) = measure(|| src.time_values(s.times[0], &s.native.y0));
                        black_box(x.unwrap());
                        component("slab", solver, cert, "mass_time_values", a, detail.clone());
                    }
                }
            }
            // Compare actual old Joint transport components on the first committed edge.
            for sync in [VectorSyncRoute::Centered, VectorSyncRoute::GreenMoment] {
                let mut m = graph("alternating", solver, cert);
                for a in &mut m {
                    for e in &mut a.transitions {
                        e.policy.sync = sync;
                    }
                }
                let r = evolve_conserved_chain(
                    &m,
                    0,
                    initial(2f64.powi(-26)),
                    [0., 0.002],
                    &opts(),
                    policy(),
                    pm(MultiGuardRoute::Correlated),
                    chain_limits(),
                    SimplexGapRoute::AffineDual,
                )
                .unwrap();
                let ev = &r.transitions()[0];
                assert!(ev.committed);
                let prep = ev.reset.as_ref().unwrap();
                let trans = ev.transport.as_ref().unwrap();
                let src = RobertsonModel::new(m[ev.from].rate).unwrap();
                let post = RobertsonModel::new(m[ev.target].rate).unwrap();
                let edge = &m[ev.from].transitions[ev.guard];
                let pol = edge.reset.polynomials().unwrap();
                let (table, a) = measure(|| RetroTable::with_dictionary(RealMode::Hybrid));
                let table = table.unwrap();
                component(
                    "reset",
                    solver,
                    cert,
                    "green_table",
                    a,
                    json!({"sync":sync}),
                );
                for (i, cell) in prep.cells.iter().enumerate() {
                    let s = r.epochs()[0]
                        .original_slab_diagnostics()
                        .iter()
                        .find(|v| v.times[0] <= cell.times[0] && cell.times[1] <= v.times[1])
                        .unwrap();
                    let y = power(s);
                    let c = s.certificate.as_ref().unwrap();
                    let mut rr = vec![];
                    for (j, g) in pol.iter().enumerate() {
                        let (x, a) = measure(|| {
                            de::vector_reset_component(
                                src.source(),
                                g,
                                &y,
                                c,
                                cell.times,
                                sync,
                                edge.policy.derivative,
                            )
                        });
                        let x = x.unwrap();
                        same(&x, &cell.reset[j]);
                        component(
                            "reset",
                            solver,
                            cert,
                            "reset_component",
                            a,
                            json!({"sync":sync,"cell":i,"component":j,"init_work":x.prepared_work}),
                        );
                        rr.push(x);
                    }
                    let boxes = rr.iter().map(|v| v.enclosure).collect::<Vec<_>>();
                    let (tube, a) = measure(|| {
                        de::certify_vector_tube(
                            post.source(),
                            [cell.times[0], ev.event_time[1]],
                            &boxes,
                            &[0.125],
                            &BRANCH,
                            policy().init,
                            edge.policy.branch_route,
                        )
                    });
                    let tube = tube.unwrap();
                    same(&tube, cell.tube.as_ref().unwrap());
                    component(
                        "reset",
                        solver,
                        cert,
                        "picard_tube",
                        a,
                        json!({"sync":sync,"cell":i,"attempts":tube.attempts.len()}),
                    );
                    let (x, a) = measure(|| {
                        de::synchronize_vector(
                            &rr,
                            cell.times,
                            ev.event_time[1],
                            &tube,
                            sync,
                            &table,
                        )
                    });
                    let x = x.unwrap();
                    same(&x, cell.sync.as_ref().unwrap());
                    component(
                        "reset",
                        solver,
                        cert,
                        "synchronize",
                        a,
                        json!({"sync":sync,"cell":i,"green_calls":x.green_calls}),
                    );
                    let (x, a) = measure(|| {
                        de::simplex_transport_cell(
                            &edge.reset,
                            &y,
                            c,
                            cell.times,
                            ev.event_time[1],
                            &tube,
                            trans.center,
                        )
                    });
                    let x = x.unwrap();
                    same(&x, &trans.cells[i]);
                    component(
                        "reset",
                        solver,
                        cert,
                        "joint_transport",
                        a,
                        json!({"sync":sync,"cell":i}),
                    );
                }
                for g in &ev.gap {
                    let a = &g.evidence;
                    let (x, c) = measure(|| {
                        post.simplex_gap_enclosure(
                            &m[ev.target].guards[g.guard],
                            a.times,
                            a.differential_box,
                            a.center,
                            a.incoming_l1,
                            a.route,
                        )
                    });
                    let x = x.unwrap();
                    same(&x, a);
                    component(
                        "reset",
                        solver,
                        cert,
                        "gap_guard",
                        c,
                        json!({"sync":sync,"cell":g.cell,"guard":g.guard}),
                    );
                }
            }
        }
    }
}
#[test]
#[ignore = "bounded refusal allocation and debit observations"]
fn refusals() {
    use serde_json::json;
    warm();
    for case in [
        "native",
        "slab",
        "transition",
        "reset_quota",
        "active_target",
        "tie",
        "width",
        "dwell",
        "warp",
        "time_guard",
        "nonconstant_reset",
    ] {
        let mut m = graph(
            "alternating",
            if case == "warp" {
                EvolutionRoute::WarpFolded
            } else {
                EvolutionRoute::Mass
            },
            ConservationRoute::Integral4,
        );
        let mut b = atlas_budget();
        match case {
            "native" => b.limits.native_attempts = 1,
            "slab" => b.limits.scan_slabs = 1,
            "transition" => b.limits.transitions = 1,
            "reset_quota" => b.reset_checks = 1,
            "active_target" => m[0].transitions[0].reset = constant(0.),
            "tie" => m[0].guards[1] = m[0].guards[0].clone(),
            "width" => b.limits.absolute_width = 1e-8,
            "dwell" => b.limits.min_dwell = 0.001,
            "time_guard" => m[0].guards[0] = h(0.001),
            "nonconstant_reset" => m[0].transitions[0].reset = reset(),
            _ => {}
        }
        let (r, a) = measure(|| {
            evolve_robertson_phase_atlas(
                &m,
                0,
                initial(2f64.powi(-26)),
                [0., 0.002],
                &opts(),
                policy(),
                pm(MultiGuardRoute::Correlated),
                b,
                PhaseReuseRoute::Reuse,
            )
        });
        let row = match r {
            Ok(r) => {
                let expected = match case {
                    "native" => PhaseAtlasStop::NativeBudget,
                    "slab" => PhaseAtlasStop::SlabBudget,
                    "transition" => PhaseAtlasStop::TransitionBudget,
                    "reset_quota" => PhaseAtlasStop::ResetBudget,
                    "active_target" => PhaseAtlasStop::ResetRefused,
                    "tie" => PhaseAtlasStop::OrderUnresolved,
                    "width" => PhaseAtlasStop::AbsoluteWidth,
                    "dwell" => PhaseAtlasStop::DwellUnproved,
                    "warp" => PhaseAtlasStop::NativeRefused,
                    _ => panic!("expected admission rejection"),
                };
                assert_eq!(r.stop(), expected);
                assert!(
                    r.work().phase.charged_native_attempts <= b.limits.native_attempts
                        && r.work().phase.charged_scan_slabs <= b.limits.scan_slabs
                        && r.work().phase.reset_checks <= b.reset_checks
                );
                json!({"case":case,"stop":r.stop(),"reason":r.reason(),"work":r.work(),"alloc":a,"epochs":r.epochs(),"events":r.events(),"final_mode":r.final_mode()})
            }
            Err(e) => {
                assert!(matches!(case, "time_guard" | "nonconstant_reset"));
                json!({"case":case,"stop":"AdmissionErr","reason":e.to_string(),"work":null,"alloc":a})
            }
        };
        println!("REFUSAL {}", row);
    }
}

#[test]
#[ignore = "bounded native guard builder before/after portfolio"]
fn guard_workspace_portfolio() {
    warm();
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let run = evolve_robertson_phase_atlas(
                &modes,
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
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let s = &phase.original_slab_diagnostics()[0];
            let c = s.certificate.as_ref().unwrap();
            assert!(c.global_qualified);
            let power: Vec<Vec<Iv>> = s
                .power
                .iter()
                .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
                .collect();
            let guard = &modes[phase.mode].guards[0];
            for derivative in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::ConservedSchur,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, a) = measure(|| {
                        de::GuardProof::new_recorded(
                            model.source(),
                            guard,
                            &power,
                            c,
                            derivative,
                            value,
                            &mut audit,
                        )
                    });
                    let result = match r {
                        Ok(g) => {
                            let t = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                            let v = g.value_at(t, c.whole_x_error.unwrap()).unwrap();
                            serde_json::json!({"evidence":g.evidence,"mid_value":v.as_pair()})
                        }
                        Err(e) => serde_json::json!({"error":e.to_string()}),
                    };
                    println!(
                        "GUARD {}",
                        serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"allocation":a,"audit":audit,"result":result})
                    );
                }
            }
        }
    }
}

#[test]
fn guard_source_defect_allocation_contract() {
    warm();
    let modes = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    let run = evolve_robertson_phase_atlas(
        &modes,
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
    assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
    let phase = &run.phases()[0];
    let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
    let s = &phase.original_slab_diagnostics()[0];
    let c = s.certificate.as_ref().unwrap();
    let power: Vec<Vec<Iv>> = s
        .power
        .iter()
        .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
        .collect();
    let (r, a) = measure(|| {
        de::GuardProof::new(
            model.source(),
            &modes[phase.mode].guards[0],
            &power,
            c,
            GuardDerivativeRoute::ConservedSchur,
            GuardValueRoute::Box,
        )
    });
    let r = r.unwrap();
    assert!(r.evidence.velocity[0] > 0.);
    println!("GUARD_ALLOC_REGRESSION {:?}", a);
    assert!(
        a.requests() <= 30,
        "source/defect workspace still requests {} allocations",
        a.requests()
    );
}

#[test]
#[ignore = "algorithm mutation portfolio on authentic native guard slabs"]
fn guard_defect_mutation_portfolio() {
    warm();
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let run = evolve_robertson_phase_atlas(
                &modes,
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
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let s = &phase.original_slab_diagnostics()[0];
            let c = s.certificate.as_ref().unwrap();
            assert!(c.global_qualified);
            let power: Vec<Vec<Iv>> = s
                .power
                .iter()
                .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
                .collect();
            let guard = &modes[phase.mode].guards[0];
            for derivative in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::ConservedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::ConservedSchurRefine2,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, a) = measure(|| {
                        de::GuardProof::new_recorded(
                            model.source(),
                            guard,
                            &power,
                            c,
                            derivative,
                            value,
                            &mut audit,
                        )
                    });
                    let result = match r {
                        Ok(g) => {
                            let t = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                            let v = g.value_at(t, c.whole_x_error.unwrap()).unwrap();
                            let left = g.value_at(c.times[0], c.whole_x_error.unwrap()).unwrap();
                            let right = g.value_at(c.times[1], c.whole_x_error.unwrap()).unwrap();
                            serde_json::json!({"evidence":g.evidence,"mid_value":v.as_pair(),"left_value":left.as_pair(),"right_value":right.as_pair()})
                        }
                        Err(e) => serde_json::json!({"error":e.to_string()}),
                    };
                    println!(
                        "GUARD {}",
                        serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"allocation":a,"audit":audit,"result":result})
                    );
                }
            }
        }
    }
}

// Exact manufactured branch x=t, z=(t,2t), with time-dependent coupled g_z.
#[test]
fn guard_defect_vector_mutations() {
    warm();
    let iv = |v: f64| Iv::point(v).unwrap();
    let lin = |k: usize, w: f64| tr(w, vec![F::component(k, 0.)]);
    let clock = |w: f64| tr(w, vec![F::clock(0.)]);
    let cross = |k: usize, w: f64| tr(w, vec![F::clock(0.), F::component(k, 0.)]);
    let covers = |v: [f64; 2], x: f64| v[0] <= x && x <= v[1];
    for (case, a, b, step) in [
        ("weak", 0.5, 0.25, 0.125),
        ("moderate", 1., 0.5, 0.125),
        ("coupled", 2., -1., 0.125),
    ] {
        let src = DaeSource::new(
            vec![p(vec![tr(1., vec![])])],
            vec![
                p(vec![
                    lin(1, 1.),
                    lin(0, -1.),
                    cross(2, a),
                    cross(0, -2. * a),
                ]),
                p(vec![lin(2, 1.), lin(0, -2.), cross(1, b), cross(0, -b)]),
            ],
        )
        .unwrap();
        let power = vec![
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(2. * step)],
        ];
        let cert = de::certify_slab(
            &src,
            [0., step],
            &power,
            &[[-1., 1.], [-1., 1.]],
            0.01,
            Some(0.),
            true,
            1e6,
        )
        .unwrap();
        assert!(cert.global_qualified, "{}", case);
        for (name, guard, dx, dt) in [
            (
                "algebraic",
                p(vec![lin(1, 1.), lin(2, 0.25), tr(-0.0625, vec![])]),
                1.5,
                0.,
            ),
            (
                "cancellation",
                p(vec![
                    lin(2, 1.),
                    lin(0, -2.),
                    clock(0.25),
                    tr(-0.015625, vec![]),
                ]),
                0.,
                0.25,
            ),
        ] {
            for route in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, allocation) = measure(|| {
                        de::GuardProof::new_recorded(
                            &src, &guard, &power, &cert, route, value, &mut audit,
                        )
                    });
                    let r = r.unwrap();
                    assert!(
                        covers(r.evidence.reduced_gradient[0], dx),
                        "{case} {route:?} gradient"
                    );
                    assert!(
                        covers(r.evidence.reduced_clock, dt),
                        "{case} {route:?} clock"
                    );
                    assert!(
                        covers(r.evidence.velocity, dx + dt),
                        "{case} {route:?} velocity"
                    );
                    let mut values = Vec::new();
                    for t in [0., step / 2., step] {
                        let v = r
                            .value_at(t, cert.whole_x_error.unwrap())
                            .unwrap()
                            .as_pair();
                        let exact = if name == "algebraic" {
                            1.5 * t - 0.0625
                        } else {
                            0.25 * t - 0.015625
                        };
                        assert!(
                            covers(v, exact),
                            "{case} {name} {route:?} {value:?} time {t}"
                        );
                        values.push(v);
                    }
                    println!(
                        "VECTOR {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"allocation":allocation,"audit":audit,"evidence":r.evidence,"values":values,"branch_defect":cert.branch_defect})
                    );
                }
            }
            let mut audit = de::GuardKernelAudit::default();
            let fail = de::GuardProof::new_recorded(
                &src,
                &guard,
                &power,
                &cert,
                GuardDerivativeRoute::ConservedSchurRefine2,
                GuardValueRoute::Box,
                &mut audit,
            );
            assert!(
                fail.is_err()
                    && audit.builds.started == 1
                    && audit.builds.returned == 0
                    && audit.refinement_sweeps.started == 0
            );
        }
    }
}

// The library remains forbid(unsafe_code); allocation accounting lives only
// in this existing integration test executable.
#[test]
fn guard_column_workspace_noalloc_and_transactional() {
    warm();
    use rodas5p_core::DenseMatrix;
    let iv = |v| Iv::point(v).unwrap();
    let src = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![
            p(vec![
                tr(1., vec![F::component(1, 0.)]),
                tr(1., vec![F::clock(0.), F::component(2, 0.)]),
            ]),
            p(vec![
                tr(1., vec![F::component(2, 0.)]),
                tr(-0.5, vec![F::clock(0.), F::component(1, 0.)]),
            ]),
        ],
    )
    .unwrap();
    let t = Iv::new(0., 0.25).unwrap();
    let state = [Iv::new(-0.25, 0.25).unwrap(); 3];
    let c = DenseMatrix::identity(2);
    let rhs = [iv(1.), iv(2.)];
    let mut workspace = de::GuardColumnWorkspace::new();
    let mut outputs = [[iv(0.); 2]; 3];
    for (k, out) in outputs.iter_mut().enumerate() {
        let mut work = AttemptCount::default();
        let (q, a) = measure(|| workspace.enclose(&src, t, &state, &c, &rhs, k, out, &mut work));
        assert_eq!(a.requests(), 0);
        assert_eq!((work.started, work.returned), (k, k));
        assert!(q.unwrap() < 1.);
        println!(
            "KERNEL_NOALLOC depth={k} requests={} bytes={} enclosure={:?}",
            a.requests(),
            a.requested_bytes,
            out
        );
        for j in 0..=64 {
            let t = j as f64 / 256.;
            let exact = [
                (1. - 2. * t) / (1. + 0.5 * t * t),
                (2. + 0.5 * t) / (1. + 0.5 * t * t),
            ];
            for i in 0..2 {
                assert!(out[i].lo <= exact[i] && exact[i] <= out[i].hi);
            }
        }
    }
    for i in 0..2 {
        for k in 1..=2 {
            assert!(
                outputs[k - 1][i].lo <= outputs[k][i].lo
                    && outputs[k][i].hi <= outputs[k - 1][i].hi
            );
        }
    }
    assert!(
        outputs[2]
            .iter()
            .zip(outputs[0])
            .any(|(a, b)| a.lo > b.lo || a.hi < b.hi)
    );
    let sentinel = [iv(123.); 2];
    let mut out = sentinel;
    let mut work = AttemptCount::default();
    assert!(
        workspace
            .enclose(
                &src,
                t,
                &state,
                &DenseMatrix::zeros(2, 2),
                &rhs,
                2,
                &mut out,
                &mut work
            )
            .is_err()
    );
    assert_eq!(out.map(|x| x.as_pair()), sentinel.map(|x| x.as_pair()));
    assert_eq!((work.started, work.returned), (0, 0));
    assert!(
        workspace
            .enclose(&src, t, &state, &c, &rhs, 3, &mut out, &mut work)
            .is_err()
    );
    assert_eq!(out.map(|x| x.as_pair()), sentinel.map(|x| x.as_pair()));
}

#[test]
fn guard_poly_ownership_allocation_contract() {
    warm();
    let modes = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    let run = evolve_robertson_phase_atlas(
        &modes,
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
    assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
    let phase = &run.phases()[0];
    let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
    let s = &phase.original_slab_diagnostics()[0];
    let c = s.certificate.as_ref().unwrap();
    let power: Vec<Vec<Iv>> = s
        .power
        .iter()
        .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
        .collect();
    for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
        let (r, a) = measure(|| {
            de::GuardProof::new(
                model.source(),
                &modes[phase.mode].guards[0],
                &power,
                c,
                GuardDerivativeRoute::ConservedSchur,
                value,
            )
        });
        let r = r.unwrap();
        assert!(r.evidence.velocity[0] > 0.);
        println!("GUARD_POLY_ALLOC {:?} {:?}", value, a);
        assert!(
            a.requests() <= 5,
            "guard polynomial ownership still requests {} allocations",
            a.requests()
        );
    }
}

#[test]
#[ignore = "polynomial ownership and GS native portfolio"]
fn guard_poly_gs_native_portfolio() {
    warm();
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let run = evolve_robertson_phase_atlas(
                &modes,
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
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let s = &phase.original_slab_diagnostics()[0];
            let c = s.certificate.as_ref().unwrap();
            assert!(c.global_qualified);
            let power: Vec<Vec<Iv>> = s
                .power
                .iter()
                .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
                .collect();
            let guard = &modes[phase.mode].guards[0];
            for derivative in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::ConservedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::ConservedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
                GuardDerivativeRoute::ConservedSchurGsAdaptive2,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, a) = measure(|| {
                        de::GuardProof::new_recorded(
                            model.source(),
                            guard,
                            &power,
                            c,
                            derivative,
                            value,
                            &mut audit,
                        )
                    });
                    let result = match r {
                        Ok(g) => {
                            let t = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                            let v = g.value_at(t, c.whole_x_error.unwrap()).unwrap();
                            let left = g.value_at(c.times[0], c.whole_x_error.unwrap()).unwrap();
                            let right = g.value_at(c.times[1], c.whole_x_error.unwrap()).unwrap();
                            serde_json::json!({"evidence":g.evidence,"mid_value":v.as_pair(),"left_value":left.as_pair(),"right_value":right.as_pair()})
                        }
                        Err(e) => serde_json::json!({"error":e.to_string()}),
                    };
                    println!(
                        "GUARD {}",
                        serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"allocation":a,"audit":audit,"result":result})
                    );
                }
            }
        }
    }
}

#[test]
fn guard_poly_gs_vector_portfolio() {
    warm();
    let iv = |v: f64| Iv::point(v).unwrap();
    let lin = |k: usize, w: f64| tr(w, vec![F::component(k, 0.)]);
    let clock = |w: f64| tr(w, vec![F::clock(0.)]);
    let cross = |k: usize, w: f64| tr(w, vec![F::clock(0.), F::component(k, 0.)]);
    let covers = |v: [f64; 2], x: f64| v[0] <= x && x <= v[1];
    for (case, a, b, step) in [
        ("weak", 0.5, 0.25, 0.125),
        ("moderate", 1., 0.5, 0.125),
        ("coupled", 2., -1., 0.125),
    ] {
        let src = DaeSource::new(
            vec![p(vec![tr(1., vec![])])],
            vec![
                p(vec![
                    lin(1, 1.),
                    lin(0, -1.),
                    cross(2, a),
                    cross(0, -2. * a),
                ]),
                p(vec![lin(2, 1.), lin(0, -2.), cross(1, b), cross(0, -b)]),
            ],
        )
        .unwrap();
        let power = vec![
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(2. * step)],
        ];
        let cert = de::certify_slab(
            &src,
            [0., step],
            &power,
            &[[-1., 1.], [-1., 1.]],
            0.01,
            Some(0.),
            true,
            1e6,
        )
        .unwrap();
        assert!(cert.global_qualified, "{}", case);
        for (name, guard, dx, dt) in [
            (
                "algebraic",
                p(vec![lin(1, 1.), lin(2, 0.25), tr(-0.0625, vec![])]),
                1.5,
                0.,
            ),
            (
                "cancellation",
                p(vec![
                    lin(2, 1.),
                    lin(0, -2.),
                    clock(0.25),
                    tr(-0.015625, vec![]),
                ]),
                0.,
                0.25,
            ),
        ] {
            for route in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, allocation) = measure(|| {
                        de::GuardProof::new_recorded(
                            &src, &guard, &power, &cert, route, value, &mut audit,
                        )
                    });
                    let r = r.unwrap();
                    assert!(
                        covers(r.evidence.reduced_gradient[0], dx),
                        "{case} {route:?} gradient"
                    );
                    assert!(
                        covers(r.evidence.reduced_clock, dt),
                        "{case} {route:?} clock"
                    );
                    assert!(
                        covers(r.evidence.velocity, dx + dt),
                        "{case} {route:?} velocity"
                    );
                    let mut values = Vec::new();
                    for t in [0., step / 2., step] {
                        let v = r
                            .value_at(t, cert.whole_x_error.unwrap())
                            .unwrap()
                            .as_pair();
                        let exact = if name == "algebraic" {
                            1.5 * t - 0.0625
                        } else {
                            0.25 * t - 0.015625
                        };
                        assert!(
                            covers(v, exact),
                            "{case} {name} {route:?} {value:?} time {t}"
                        );
                        values.push(v);
                    }
                    println!(
                        "VECTOR {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"allocation":allocation,"audit":audit,"evidence":r.evidence,"values":values,"branch_defect":cert.branch_defect})
                    );
                }
            }
            let mut audit = de::GuardKernelAudit::default();
            let fail = de::GuardProof::new_recorded(
                &src,
                &guard,
                &power,
                &cert,
                GuardDerivativeRoute::ConservedSchurRefine2,
                GuardValueRoute::Box,
                &mut audit,
            );
            assert!(
                fail.is_err()
                    && audit.builds.started == 1
                    && audit.builds.returned == 0
                    && audit.refinement_sweeps.started == 0
            );
        }
    }
}

#[test]
fn guard_gs_shared_kernel_noalloc() {
    warm();
    use de::GuardColumnRefinement as R;
    use rodas5p_core::DenseMatrix;
    let iv = |v| Iv::point(v).unwrap();
    let src = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![
            p(vec![
                tr(1., vec![F::component(1, 0.)]),
                tr(1., vec![F::clock(0.), F::component(2, 0.)]),
            ]),
            p(vec![
                tr(1., vec![F::component(2, 0.)]),
                tr(-0.5, vec![F::clock(0.), F::component(1, 0.)]),
            ]),
        ],
    )
    .unwrap();
    let t = Iv::new(0., 0.25).unwrap();
    let state = [Iv::new(-0.25, 0.25).unwrap(); 3];
    let c = DenseMatrix::new(2, 2, vec![0.875, 0., 0., 1.125]).unwrap();
    let rhs = [iv(1.), iv(2.)];
    let mut ws = de::GuardColumnWorkspace::new();
    let mut boxes = [[iv(0.); 2]; 6];
    let mut counts = [0; 6];
    for (k, route) in [
        R::None,
        R::Jacobi1,
        R::Jacobi2,
        R::GaussSeidel1,
        R::GaussSeidel2,
        R::AdaptiveGaussSeidel2,
    ]
    .into_iter()
    .enumerate()
    {
        let mut work = AttemptCount::default();
        let (q, a) = measure(|| {
            ws.enclose_refined(&src, t, &state, &c, &rhs, route, &mut boxes[k], &mut work)
        });
        let q = q.unwrap();
        assert!(q < 1. && q > 0.);
        assert_eq!(a.requests(), 0);
        assert_eq!(a.requested_bytes, 0);
        assert_eq!(work.started, work.returned);
        counts[k] = work.started;
        for j in 0..=64 {
            let t = j as f64 / 256.;
            let exact = [
                (1. - 2. * t) / (1. + 0.5 * t * t),
                (2. + 0.5 * t) / (1. + 0.5 * t * t),
            ];
            for i in 0..2 {
                assert!(boxes[k][i].lo <= exact[i] && exact[i] <= boxes[k][i].hi);
            }
        }
        println!(
            "GS_KERNEL {:?} q={} requests={} bytes={} sweeps={} box={:?}",
            route,
            q,
            a.requests(),
            a.requested_bytes,
            work.started,
            boxes[k]
        );
    }
    assert_eq!(&counts[..5], &[0, 1, 2, 1, 2]);
    assert!(counts[5] >= 1 && counts[5] <= 2);
    for i in 0..2 {
        for (child, parent) in [(1, 0), (2, 1), (3, 0), (4, 3), (5, 0)] {
            assert!(
                boxes[parent][i].lo <= boxes[child][i].lo
                    && boxes[child][i].hi <= boxes[parent][i].hi
            );
        }
    }
    // Adaptive skip when q=0, with an exact identity constraint source.
    let ident = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![
            p(vec![tr(1., vec![F::component(1, 0.)])]),
            p(vec![tr(1., vec![F::component(2, 0.)])]),
        ],
    )
    .unwrap();
    let identity = DenseMatrix::identity(2);
    let mut out = [iv(123.); 2];
    let mut work = AttemptCount::default();
    let (q, a) = measure(|| {
        ws.enclose_refined(
            &ident,
            t,
            &state,
            &identity,
            &rhs,
            R::AdaptiveGaussSeidel2,
            &mut out,
            &mut work,
        )
    });
    assert_eq!(q.unwrap(), 0.);
    assert_eq!((a.requests(), work.started, work.returned), (0, 0, 0));
    assert_eq!(out.map(|x| x.as_pair()), rhs.map(|x| x.as_pair()));
    println!("GS_ADAPTIVE_EXACT q=0 requests=0 sweeps=0");
    let sentinel = [iv(123.); 2];
    out = sentinel;
    assert!(
        ws.enclose_refined(
            &src,
            t,
            &state,
            &DenseMatrix::zeros(2, 2),
            &rhs,
            R::GaussSeidel2,
            &mut out,
            &mut work
        )
        .is_err()
    );
    assert_eq!(out.map(|v| v.as_pair()), sentinel.map(|v| v.as_pair()));
    assert_eq!(work.started, 0);
}

#[test]
fn guard_borrowed_c_allocation_contract() {
    warm();
    let modes = graph(
        "alternating",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    let run = evolve_robertson_phase_atlas(
        &modes,
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
    assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
    let phase = &run.phases()[0];
    let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
    let s = &phase.original_slab_diagnostics()[0];
    let c = s.certificate.as_ref().unwrap();
    let power: Vec<Vec<Iv>> = s
        .power
        .iter()
        .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
        .collect();
    for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
        let (r, a) = measure(|| {
            de::GuardProof::new(
                model.source(),
                &modes[phase.mode].guards[0],
                &power,
                c,
                GuardDerivativeRoute::ConservedSchur,
                value,
            )
        });
        let r = r.unwrap();
        assert!(r.evidence.velocity[0] > 0.);
        println!("GUARD_BORROWED_C {:?} {:?}", value, a);
        assert!(
            a.requests() <= 2,
            "guard borrowed C still requests {} allocations",
            a.requests()
        );
    }
}

#[test]
#[ignore = "borrowed core and centered residual native portfolio"]
fn guard_centered_native_portfolio() {
    warm();
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let (run, scan_alloc) = measure(|| {
                evolve_robertson_phase_atlas(
                    &modes,
                    0,
                    initial(2f64.powi(-26)),
                    [0., 0.002],
                    &opts(),
                    policy(),
                    pm(MultiGuardRoute::Correlated),
                    atlas_budget(),
                    PhaseReuseRoute::Reuse,
                )
            });
            let run = run.unwrap();
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            println!(
                "SCAN {}",
                serde_json::json!({"solver":solver,"cert":cert,"allocation":scan_alloc,"stop":run.stop()})
            );
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let s = &phase.original_slab_diagnostics()[0];
            let c = s.certificate.as_ref().unwrap();
            assert!(c.global_qualified);
            let power: Vec<Vec<Iv>> = s
                .power
                .iter()
                .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
                .collect();
            let guard = &modes[phase.mode].guards[0];
            for derivative in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::ConservedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::ConservedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
                GuardDerivativeRoute::ConservedSchurGsAdaptive2,
                GuardDerivativeRoute::SchurCentered,
                GuardDerivativeRoute::SchurCenteredGrouped,
                GuardDerivativeRoute::SchurCenteredGs2,
                GuardDerivativeRoute::PreparedSchurCenteredGs2,
                GuardDerivativeRoute::ConservedSchurCenteredGs2,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, a) = measure(|| {
                        de::GuardProof::new_recorded(
                            model.source(),
                            guard,
                            &power,
                            c,
                            derivative,
                            value,
                            &mut audit,
                        )
                    });
                    let result = match r {
                        Ok(g) => {
                            let mut caudit = de::GuardKernelAudit::default();
                            let ((core, queries), core_alloc) = measure(|| {
                                let core = de::GuardProofCore::new_recorded(
                                    model.source(),
                                    guard,
                                    &power,
                                    c,
                                    derivative,
                                    value,
                                    &mut caudit,
                                )
                                .unwrap();
                                let mid = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                                let queries = [c.times[0], mid, c.times[1]].map(|t| {
                                    core.value_at(t, c.whole_x_error.unwrap())
                                        .unwrap()
                                        .as_pair()
                                });
                                (core, queries)
                            });
                            let (report, report_alloc) = measure(|| core.owned_evidence());
                            assert_eq!(
                                serde_json::to_value(&report).unwrap(),
                                serde_json::to_value(&g.evidence).unwrap()
                            );
                            assert_eq!(
                                serde_json::to_value(&caudit).unwrap(),
                                serde_json::to_value(&audit).unwrap()
                            );
                            if !matches!(
                                derivative,
                                GuardDerivativeRoute::PreparedSchur
                                    | GuardDerivativeRoute::PreparedSchurRefine2
                                    | GuardDerivativeRoute::PreparedSchurGs2
                                    | GuardDerivativeRoute::PreparedSchurCenteredGs2
                            ) {
                                assert_eq!(core_alloc.requests(), 0);
                                assert_eq!(core_alloc.requested_bytes, 0);
                            }
                            println!(
                                "CORE {}",
                                serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"core":core_alloc,"report":report_alloc,"queries":queries})
                            );
                            let t = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                            let v = g.value_at(t, c.whole_x_error.unwrap()).unwrap();
                            let left = g.value_at(c.times[0], c.whole_x_error.unwrap()).unwrap();
                            let right = g.value_at(c.times[1], c.whole_x_error.unwrap()).unwrap();
                            assert_eq!(queries, [left.as_pair(), v.as_pair(), right.as_pair()]);
                            serde_json::json!({"evidence":g.evidence,"mid_value":v.as_pair(),"left_value":left.as_pair(),"right_value":right.as_pair()})
                        }
                        Err(e) => serde_json::json!({"error":e.to_string()}),
                    };
                    println!(
                        "GUARD {}",
                        serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"allocation":a,"audit":audit,"result":result})
                    );
                }
            }
        }
    }
}

#[test]
fn guard_centered_vector_portfolio() {
    warm();
    let iv = |v: f64| Iv::point(v).unwrap();
    let lin = |k: usize, w: f64| tr(w, vec![F::component(k, 0.)]);
    let clock = |w: f64| tr(w, vec![F::clock(0.)]);
    let cross = |k: usize, w: f64| tr(w, vec![F::clock(0.), F::component(k, 0.)]);
    let covers = |v: [f64; 2], x: f64| v[0] <= x && x <= v[1];
    for (case, a, b, step) in [
        ("weak", 0.5, 0.25, 0.125),
        ("moderate", 1., 0.5, 0.125),
        ("coupled", 2., -1., 0.125),
    ] {
        let src = DaeSource::new(
            vec![p(vec![tr(1., vec![])])],
            vec![
                p(vec![
                    lin(1, 1.),
                    lin(0, -1.),
                    cross(2, a),
                    cross(0, -2. * a),
                ]),
                p(vec![lin(2, 1.), lin(0, -2.), cross(1, b), cross(0, -b)]),
            ],
        )
        .unwrap();
        let power = vec![
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(2. * step)],
        ];
        let cert = de::certify_slab(
            &src,
            [0., step],
            &power,
            &[[-1., 1.], [-1., 1.]],
            0.01,
            Some(0.),
            true,
            1e6,
        )
        .unwrap();
        assert!(cert.global_qualified, "{}", case);
        for (name, guard, dx, dt) in [
            (
                "algebraic",
                p(vec![lin(1, 1.), lin(2, 0.25), tr(-0.0625, vec![])]),
                1.5,
                0.,
            ),
            (
                "cancellation",
                p(vec![
                    lin(2, 1.),
                    lin(0, -2.),
                    clock(0.25),
                    tr(-0.015625, vec![]),
                ]),
                0.,
                0.25,
            ),
        ] {
            for route in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
                GuardDerivativeRoute::SchurCentered,
                GuardDerivativeRoute::SchurCenteredGrouped,
                GuardDerivativeRoute::SchurCenteredGs2,
                GuardDerivativeRoute::PreparedSchurCenteredGs2,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, allocation) = measure(|| {
                        de::GuardProof::new_recorded(
                            &src, &guard, &power, &cert, route, value, &mut audit,
                        )
                    });
                    let r = r.unwrap();
                    let mut caudit = de::GuardKernelAudit::default();
                    let ((core, queries), core_alloc) = measure(|| {
                        let core = de::GuardProofCore::new_recorded(
                            &src,
                            &guard,
                            &power,
                            &cert,
                            route,
                            value,
                            &mut caudit,
                        )
                        .unwrap();
                        let queries = [0., step / 2., step].map(|t| {
                            core.value_at(t, cert.whole_x_error.unwrap())
                                .unwrap()
                                .as_pair()
                        });
                        (core, queries)
                    });
                    let (report, report_alloc) = measure(|| core.owned_evidence());
                    assert_eq!(
                        serde_json::to_value(&report).unwrap(),
                        serde_json::to_value(&r.evidence).unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(&caudit).unwrap(),
                        serde_json::to_value(&audit).unwrap()
                    );
                    if !matches!(
                        route,
                        GuardDerivativeRoute::PreparedSchur
                            | GuardDerivativeRoute::PreparedSchurRefine2
                            | GuardDerivativeRoute::PreparedSchurGs2
                            | GuardDerivativeRoute::PreparedSchurCenteredGs2
                    ) {
                        assert_eq!(core_alloc.requests(), 0);
                        assert_eq!(core_alloc.requested_bytes, 0);
                    }
                    println!(
                        "VECTOR_CORE {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"core":core_alloc,"report":report_alloc,"queries":queries})
                    );
                    assert!(
                        covers(r.evidence.reduced_gradient[0], dx),
                        "{case} {route:?} gradient"
                    );
                    assert!(
                        covers(r.evidence.reduced_clock, dt),
                        "{case} {route:?} clock"
                    );
                    assert!(
                        covers(r.evidence.velocity, dx + dt),
                        "{case} {route:?} velocity"
                    );
                    let mut values = Vec::new();
                    for t in [0., step / 2., step] {
                        let v = r
                            .value_at(t, cert.whole_x_error.unwrap())
                            .unwrap()
                            .as_pair();
                        let exact = if name == "algebraic" {
                            1.5 * t - 0.0625
                        } else {
                            0.25 * t - 0.015625
                        };
                        assert!(
                            covers(v, exact),
                            "{case} {name} {route:?} {value:?} time {t}"
                        );
                        values.push(v);
                    }
                    assert_eq!(values, queries);
                    println!(
                        "VECTOR {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"allocation":allocation,"audit":audit,"evidence":r.evidence,"values":values,"branch_defect":cert.branch_defect})
                    );
                }
            }
            let mut audit = de::GuardKernelAudit::default();
            let fail = de::GuardProof::new_recorded(
                &src,
                &guard,
                &power,
                &cert,
                GuardDerivativeRoute::ConservedSchurCenteredGs2,
                GuardValueRoute::Box,
                &mut audit,
            );
            assert!(
                fail.is_err()
                    && audit.builds.started == 1
                    && audit.builds.returned == 0
                    && audit.refinement_sweeps.started == 0
            );
        }
    }
}

#[test]
#[ignore = "dual residual and bounded inverse native portfolio"]
fn guard_dual_native_portfolio() {
    warm();
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let (run, scan_alloc) = measure(|| {
                evolve_robertson_phase_atlas(
                    &modes,
                    0,
                    initial(2f64.powi(-26)),
                    [0., 0.002],
                    &opts(),
                    policy(),
                    pm(MultiGuardRoute::Correlated),
                    atlas_budget(),
                    PhaseReuseRoute::Reuse,
                )
            });
            let run = run.unwrap();
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            println!(
                "SCAN {}",
                serde_json::json!({"solver":solver,"cert":cert,"allocation":scan_alloc,"stop":run.stop()})
            );
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let s = &phase.original_slab_diagnostics()[0];
            let c = s.certificate.as_ref().unwrap();
            assert!(c.global_qualified);
            let power: Vec<Vec<Iv>> = s
                .power
                .iter()
                .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
                .collect();
            let guard = &modes[phase.mode].guards[0];
            for derivative in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::ConservedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::ConservedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
                GuardDerivativeRoute::ConservedSchurGsAdaptive2,
                GuardDerivativeRoute::SchurCentered,
                GuardDerivativeRoute::SchurCenteredGrouped,
                GuardDerivativeRoute::SchurCenteredGs2,
                GuardDerivativeRoute::PreparedSchurCenteredGs2,
                GuardDerivativeRoute::ConservedSchurCenteredGs2,
                GuardDerivativeRoute::SchurDualGrouped,
                GuardDerivativeRoute::SchurDualCentered,
                GuardDerivativeRoute::SchurDualCenteredGs2,
                GuardDerivativeRoute::BoundedPreparedSchur,
                GuardDerivativeRoute::BoundedPreparedDualCenteredGs2,
                GuardDerivativeRoute::ConservedSchurDualCentered,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, a) = measure(|| {
                        de::GuardProof::new_recorded(
                            model.source(),
                            guard,
                            &power,
                            c,
                            derivative,
                            value,
                            &mut audit,
                        )
                    });
                    let result = match r {
                        Ok(g) => {
                            let mut caudit = de::GuardKernelAudit::default();
                            let ((core, queries), core_alloc) = measure(|| {
                                let core = de::GuardProofCore::new_recorded(
                                    model.source(),
                                    guard,
                                    &power,
                                    c,
                                    derivative,
                                    value,
                                    &mut caudit,
                                )
                                .unwrap();
                                let mid = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                                let queries = [c.times[0], mid, c.times[1]].map(|t| {
                                    core.value_at(t, c.whole_x_error.unwrap())
                                        .unwrap()
                                        .as_pair()
                                });
                                (core, queries)
                            });
                            let (report, report_alloc) = measure(|| core.owned_evidence());
                            assert_eq!(
                                serde_json::to_value(&report).unwrap(),
                                serde_json::to_value(&g.evidence).unwrap()
                            );
                            assert_eq!(
                                serde_json::to_value(&caudit).unwrap(),
                                serde_json::to_value(&audit).unwrap()
                            );
                            if !matches!(
                                derivative,
                                GuardDerivativeRoute::PreparedSchur
                                    | GuardDerivativeRoute::PreparedSchurRefine2
                                    | GuardDerivativeRoute::PreparedSchurGs2
                                    | GuardDerivativeRoute::PreparedSchurCenteredGs2
                            ) {
                                assert_eq!(core_alloc.requests(), 0);
                                assert_eq!(core_alloc.requested_bytes, 0);
                            }
                            println!(
                                "CORE {}",
                                serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"core":core_alloc,"report":report_alloc,"queries":queries})
                            );
                            let t = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                            let v = g.value_at(t, c.whole_x_error.unwrap()).unwrap();
                            let left = g.value_at(c.times[0], c.whole_x_error.unwrap()).unwrap();
                            let right = g.value_at(c.times[1], c.whole_x_error.unwrap()).unwrap();
                            assert_eq!(queries, [left.as_pair(), v.as_pair(), right.as_pair()]);
                            serde_json::json!({"evidence":g.evidence,"mid_value":v.as_pair(),"left_value":left.as_pair(),"right_value":right.as_pair()})
                        }
                        Err(e) => serde_json::json!({"error":e.to_string()}),
                    };
                    println!(
                        "GUARD {}",
                        serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"allocation":a,"audit":audit,"result":result})
                    );
                }
            }
        }
    }
}

#[test]
fn guard_dual_vector_portfolio() {
    warm();
    let iv = |v: f64| Iv::point(v).unwrap();
    let lin = |k: usize, w: f64| tr(w, vec![F::component(k, 0.)]);
    let clock = |w: f64| tr(w, vec![F::clock(0.)]);
    let cross = |k: usize, w: f64| tr(w, vec![F::clock(0.), F::component(k, 0.)]);
    let covers = |v: [f64; 2], x: f64| v[0] <= x && x <= v[1];
    for (case, a, b, step) in [
        ("weak", 0.5, 0.25, 0.125),
        ("moderate", 1., 0.5, 0.125),
        ("coupled", 2., -1., 0.125),
    ] {
        let src = DaeSource::new(
            vec![p(vec![tr(1., vec![])])],
            vec![
                p(vec![
                    lin(1, 1.),
                    lin(0, -1.),
                    cross(2, a),
                    cross(0, -2. * a),
                ]),
                p(vec![lin(2, 1.), lin(0, -2.), cross(1, b), cross(0, -b)]),
            ],
        )
        .unwrap();
        let power = vec![
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(2. * step)],
        ];
        let cert = de::certify_slab(
            &src,
            [0., step],
            &power,
            &[[-1., 1.], [-1., 1.]],
            0.01,
            Some(0.),
            true,
            1e6,
        )
        .unwrap();
        assert!(cert.global_qualified, "{}", case);
        for (name, guard, dx, dt) in [
            (
                "algebraic",
                p(vec![lin(1, 1.), lin(2, 0.25), tr(-0.0625, vec![])]),
                1.5,
                0.,
            ),
            (
                "cancellation",
                p(vec![
                    lin(2, 1.),
                    lin(0, -2.),
                    clock(0.25),
                    tr(-0.015625, vec![]),
                ]),
                0.,
                0.25,
            ),
        ] {
            for route in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
                GuardDerivativeRoute::SchurCentered,
                GuardDerivativeRoute::SchurCenteredGrouped,
                GuardDerivativeRoute::SchurCenteredGs2,
                GuardDerivativeRoute::PreparedSchurCenteredGs2,
                GuardDerivativeRoute::SchurDualGrouped,
                GuardDerivativeRoute::SchurDualCentered,
                GuardDerivativeRoute::SchurDualCenteredGs2,
                GuardDerivativeRoute::BoundedPreparedSchur,
                GuardDerivativeRoute::BoundedPreparedDualCenteredGs2,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, allocation) = measure(|| {
                        de::GuardProof::new_recorded(
                            &src, &guard, &power, &cert, route, value, &mut audit,
                        )
                    });
                    let r = r.unwrap();
                    let mut caudit = de::GuardKernelAudit::default();
                    let ((core, queries), core_alloc) = measure(|| {
                        let core = de::GuardProofCore::new_recorded(
                            &src,
                            &guard,
                            &power,
                            &cert,
                            route,
                            value,
                            &mut caudit,
                        )
                        .unwrap();
                        let queries = [0., step / 2., step].map(|t| {
                            core.value_at(t, cert.whole_x_error.unwrap())
                                .unwrap()
                                .as_pair()
                        });
                        (core, queries)
                    });
                    let (report, report_alloc) = measure(|| core.owned_evidence());
                    assert_eq!(
                        serde_json::to_value(&report).unwrap(),
                        serde_json::to_value(&r.evidence).unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(&caudit).unwrap(),
                        serde_json::to_value(&audit).unwrap()
                    );
                    if !matches!(
                        route,
                        GuardDerivativeRoute::PreparedSchur
                            | GuardDerivativeRoute::PreparedSchurRefine2
                            | GuardDerivativeRoute::PreparedSchurGs2
                            | GuardDerivativeRoute::PreparedSchurCenteredGs2
                    ) {
                        assert_eq!(core_alloc.requests(), 0);
                        assert_eq!(core_alloc.requested_bytes, 0);
                    }
                    println!(
                        "VECTOR_CORE {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"core":core_alloc,"report":report_alloc,"queries":queries})
                    );
                    assert!(
                        covers(r.evidence.reduced_gradient[0], dx),
                        "{case} {route:?} gradient"
                    );
                    assert!(
                        covers(r.evidence.reduced_clock, dt),
                        "{case} {route:?} clock"
                    );
                    assert!(
                        covers(r.evidence.velocity, dx + dt),
                        "{case} {route:?} velocity"
                    );
                    let mut values = Vec::new();
                    for t in [0., step / 2., step] {
                        let v = r
                            .value_at(t, cert.whole_x_error.unwrap())
                            .unwrap()
                            .as_pair();
                        let exact = if name == "algebraic" {
                            1.5 * t - 0.0625
                        } else {
                            0.25 * t - 0.015625
                        };
                        assert!(
                            covers(v, exact),
                            "{case} {name} {route:?} {value:?} time {t}"
                        );
                        values.push(v);
                    }
                    assert_eq!(values, queries);
                    println!(
                        "VECTOR {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"allocation":allocation,"audit":audit,"evidence":r.evidence,"values":values,"branch_defect":cert.branch_defect})
                    );
                }
            }
            let mut audit = de::GuardKernelAudit::default();
            let fail = de::GuardProof::new_recorded(
                &src,
                &guard,
                &power,
                &cert,
                GuardDerivativeRoute::ConservedSchurDualCentered,
                GuardValueRoute::Box,
                &mut audit,
            );
            assert!(
                fail.is_err()
                    && audit.builds.started == 1
                    && audit.builds.returned == 0
                    && audit.refinement_sweeps.started == 0
            );
        }
    }
}

#[test]
fn bounded_guard_inverse_workspace_contract() {
    warm();
    let mut ws = de::GuardInverseWorkspace::new();
    // exact inverse of [[2,1/2],[-1/4,2]], determinant 33/8.
    let a = [2., 0.5, -0.25, 2.];
    let exact = [16. / 33., -4. / 33., 2. / 33., 16. / 33.];
    let mut out = [123.; 4];
    for _ in 0..2 {
        let (result, alloc) = measure(|| ws.invert(2, &a, &mut out));
        let error = result.unwrap();
        assert_eq!(alloc.requests(), 0);
        assert_eq!(alloc.requested_bytes, 0);
        for (got, refv) in out.iter().zip(exact) {
            // exact rational check: the returned inverse entry bound is
            // independently checked by a directed residual against A.
            assert!((*got - refv).abs() <= error + 2. * f64::EPSILON);
        }
        println!(
            "BOUNDED_INVERSE n=2 error={} allocations={:?} inverse={:?}",
            error, alloc, out
        );
    }
    // change matrix: reuse scratch, never reuse inverse by dimension.
    let (result, alloc) = measure(|| ws.invert(2, &[2., 0., 0., 4.], &mut out));
    assert_eq!(result.unwrap(), 0.);
    assert_eq!(out, [0.5, 0., 0., 0.25]);
    assert_eq!(alloc.requests(), 0);
    let sentinel = out;
    for bad in [[1., 1., 1., 1.], [f64::NAN, 0., 0., 1.], [0., 1., 1., 0.]] {
        assert!(ws.invert(2, &bad, &mut out).is_err());
        assert_eq!(out, sentinel);
    }
    assert!(ws.invert(3, &a, &mut out).is_err());
    assert_eq!(out, sentinel);
}

#[test]
fn scalar_reuse_reconstruct_allocation_contract() {
    warm();
    const BRANCH: [[f64; 2]; 1] = [[-0.25, 1.25]];
    let modes = graph(
        "alternating",
        EvolutionRoute::Reduced5,
        ConservationRoute::Integral4,
    );
    let run = evolve_robertson_phase_atlas(
        &modes,
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
    assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
    let phase = &run.phases()[0];
    let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
    let s = &phase.original_slab_diagnostics()[0];
    let src = model.source();
    let init = src
        .initialize(
            0.,
            &phase.initial.center(),
            &[0.125],
            &BRANCH,
            policy().init,
            InitRoute::Reuse,
        )
        .unwrap();
    let power =
        de::dense_power(&s.native.y0, &s.native.y1, &s.native.stages, s.native.basis).unwrap();
    let (r, a) = measure(|| {
        de::reconstruct(
            src,
            s.times,
            &power,
            &init.z,
            &BRANCH,
            policy().init,
            InitRoute::Reuse,
            None,
            5,
        )
    });
    let (z, _) = r.unwrap();
    same(
        &z.iter()
            .map(|r| r.iter().map(|v| v.as_pair()).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        &s.power[power.len()..].to_vec(),
    );
    println!("RECONSTRUCT_REUSE_ALLOC {:?}", a);
    assert!(
        a.requests() <= 3,
        "scalar reuse reconstruct owns {} allocations",
        a.requests()
    );
}

#[test]
#[ignore = "affine graph native portfolio"]
fn guard_affine_graph_native_portfolio() {
    warm();
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let (run, scan_alloc) = measure(|| {
                evolve_robertson_phase_atlas(
                    &modes,
                    0,
                    initial(2f64.powi(-26)),
                    [0., 0.002],
                    &opts(),
                    policy(),
                    pm(MultiGuardRoute::Correlated),
                    atlas_budget(),
                    PhaseReuseRoute::Reuse,
                )
            });
            let run = run.unwrap();
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            println!(
                "SCAN {}",
                serde_json::json!({"solver":solver,"cert":cert,"allocation":scan_alloc,"stop":run.stop()})
            );
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let s = &phase.original_slab_diagnostics()[0];
            let c = s.certificate.as_ref().unwrap();
            assert!(c.global_qualified);
            let power: Vec<Vec<Iv>> = s
                .power
                .iter()
                .map(|row| row.iter().map(|v| Iv::new(v[0], v[1]).unwrap()).collect())
                .collect();
            let guard = &modes[phase.mode].guards[0];
            for derivative in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::ConservedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::ConservedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
                GuardDerivativeRoute::ConservedSchurGsAdaptive2,
                GuardDerivativeRoute::SchurCentered,
                GuardDerivativeRoute::SchurCenteredGrouped,
                GuardDerivativeRoute::SchurCenteredGs2,
                GuardDerivativeRoute::PreparedSchurCenteredGs2,
                GuardDerivativeRoute::ConservedSchurCenteredGs2,
                GuardDerivativeRoute::SchurDualGrouped,
                GuardDerivativeRoute::SchurDualCentered,
                GuardDerivativeRoute::SchurDualCenteredGs2,
                GuardDerivativeRoute::BoundedPreparedSchur,
                GuardDerivativeRoute::BoundedPreparedDualCenteredGs2,
                GuardDerivativeRoute::ConservedSchurDualCentered,
                GuardDerivativeRoute::SchurAffineGraph,
                GuardDerivativeRoute::ConservedSchurAffineGraph,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, a) = measure(|| {
                        de::GuardProof::new_recorded(
                            model.source(),
                            guard,
                            &power,
                            c,
                            derivative,
                            value,
                            &mut audit,
                        )
                    });
                    let result = match r {
                        Ok(g) => {
                            let mut caudit = de::GuardKernelAudit::default();
                            let ((core, queries), core_alloc) = measure(|| {
                                let core = de::GuardProofCore::new_recorded(
                                    model.source(),
                                    guard,
                                    &power,
                                    c,
                                    derivative,
                                    value,
                                    &mut caudit,
                                )
                                .unwrap();
                                let mid = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                                let queries = [c.times[0], mid, c.times[1]].map(|t| {
                                    core.value_at(t, c.whole_x_error.unwrap())
                                        .unwrap()
                                        .as_pair()
                                });
                                (core, queries)
                            });
                            let (report, report_alloc) = measure(|| core.owned_evidence());
                            assert_eq!(
                                serde_json::to_value(&report).unwrap(),
                                serde_json::to_value(&g.evidence).unwrap()
                            );
                            assert_eq!(
                                serde_json::to_value(&caudit).unwrap(),
                                serde_json::to_value(&audit).unwrap()
                            );
                            if !matches!(
                                derivative,
                                GuardDerivativeRoute::PreparedSchur
                                    | GuardDerivativeRoute::PreparedSchurRefine2
                                    | GuardDerivativeRoute::PreparedSchurGs2
                                    | GuardDerivativeRoute::PreparedSchurCenteredGs2
                            ) {
                                assert_eq!(core_alloc.requests(), 0);
                                assert_eq!(core_alloc.requested_bytes, 0);
                            }
                            println!(
                                "CORE {}",
                                serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"core":core_alloc,"report":report_alloc,"queries":queries})
                            );
                            let t = c.times[0] + (c.times[1] - c.times[0]) * 0.5;
                            let v = g.value_at(t, c.whole_x_error.unwrap()).unwrap();
                            let left = g.value_at(c.times[0], c.whole_x_error.unwrap()).unwrap();
                            let right = g.value_at(c.times[1], c.whole_x_error.unwrap()).unwrap();
                            assert_eq!(queries, [left.as_pair(), v.as_pair(), right.as_pair()]);
                            serde_json::json!({"evidence":g.evidence,"mid_value":v.as_pair(),"left_value":left.as_pair(),"right_value":right.as_pair()})
                        }
                        Err(e) => serde_json::json!({"error":e.to_string()}),
                    };
                    println!(
                        "GUARD {}",
                        serde_json::json!({"solver":solver,"cert":cert,"derivative":derivative,"value":value,"allocation":a,"audit":audit,"result":result})
                    );
                }
            }
        }
    }
}

#[test]
fn guard_affine_graph_vector_portfolio() {
    warm();
    let iv = |v: f64| Iv::point(v).unwrap();
    let lin = |k: usize, w: f64| tr(w, vec![F::component(k, 0.)]);
    let clock = |w: f64| tr(w, vec![F::clock(0.)]);
    let cross = |k: usize, w: f64| tr(w, vec![F::clock(0.), F::component(k, 0.)]);
    let covers = |v: [f64; 2], x: f64| v[0] <= x && x <= v[1];
    for (case, a, b, step) in [
        ("weak", 0.5, 0.25, 0.125),
        ("moderate", 1., 0.5, 0.125),
        ("coupled", 2., -1., 0.125),
    ] {
        let src = DaeSource::new(
            vec![p(vec![tr(1., vec![])])],
            vec![
                p(vec![
                    lin(1, 1.),
                    lin(0, -1.),
                    cross(2, a),
                    cross(0, -2. * a),
                ]),
                p(vec![lin(2, 1.), lin(0, -2.), cross(1, b), cross(0, -b)]),
            ],
        )
        .unwrap();
        let power = vec![
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(step)],
            vec![iv(0.), iv(2. * step)],
        ];
        let cert = de::certify_slab(
            &src,
            [0., step],
            &power,
            &[[-1., 1.], [-1., 1.]],
            0.01,
            Some(0.),
            true,
            1e6,
        )
        .unwrap();
        assert!(cert.global_qualified, "{}", case);
        for (name, guard, dx, dt) in [
            (
                "algebraic",
                p(vec![lin(1, 1.), lin(2, 0.25), tr(-0.0625, vec![])]),
                1.5,
                0.,
            ),
            (
                "cancellation",
                p(vec![
                    lin(2, 1.),
                    lin(0, -2.),
                    clock(0.25),
                    tr(-0.015625, vec![]),
                ]),
                0.,
                0.25,
            ),
        ] {
            for route in [
                GuardDerivativeRoute::Tangent,
                GuardDerivativeRoute::Schur,
                GuardDerivativeRoute::PreparedSchur,
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
                GuardDerivativeRoute::SchurCentered,
                GuardDerivativeRoute::SchurCenteredGrouped,
                GuardDerivativeRoute::SchurCenteredGs2,
                GuardDerivativeRoute::PreparedSchurCenteredGs2,
                GuardDerivativeRoute::SchurDualGrouped,
                GuardDerivativeRoute::SchurDualCentered,
                GuardDerivativeRoute::SchurDualCenteredGs2,
                GuardDerivativeRoute::BoundedPreparedSchur,
                GuardDerivativeRoute::BoundedPreparedDualCenteredGs2,
                GuardDerivativeRoute::SchurAffineGraph,
            ] {
                for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                    let mut audit = de::GuardKernelAudit::default();
                    let (r, allocation) = measure(|| {
                        de::GuardProof::new_recorded(
                            &src, &guard, &power, &cert, route, value, &mut audit,
                        )
                    });
                    let r = r.unwrap();
                    if route == GuardDerivativeRoute::SchurAffineGraph {
                        assert_eq!(r.evidence.reduced_gradient[0], [dx, dx]);
                        assert_eq!(r.evidence.reduced_clock, [dt, dt]);
                        assert_eq!(r.evidence.velocity, [dx + dt, dx + dt]);
                    }
                    let mut caudit = de::GuardKernelAudit::default();
                    let ((core, queries), core_alloc) = measure(|| {
                        let core = de::GuardProofCore::new_recorded(
                            &src,
                            &guard,
                            &power,
                            &cert,
                            route,
                            value,
                            &mut caudit,
                        )
                        .unwrap();
                        let queries = [0., step / 2., step].map(|t| {
                            core.value_at(t, cert.whole_x_error.unwrap())
                                .unwrap()
                                .as_pair()
                        });
                        (core, queries)
                    });
                    let (report, report_alloc) = measure(|| core.owned_evidence());
                    assert_eq!(
                        serde_json::to_value(&report).unwrap(),
                        serde_json::to_value(&r.evidence).unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(&caudit).unwrap(),
                        serde_json::to_value(&audit).unwrap()
                    );
                    if !matches!(
                        route,
                        GuardDerivativeRoute::PreparedSchur
                            | GuardDerivativeRoute::PreparedSchurRefine2
                            | GuardDerivativeRoute::PreparedSchurGs2
                            | GuardDerivativeRoute::PreparedSchurCenteredGs2
                    ) {
                        assert_eq!(core_alloc.requests(), 0);
                        assert_eq!(core_alloc.requested_bytes, 0);
                    }
                    println!(
                        "VECTOR_CORE {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"core":core_alloc,"report":report_alloc,"queries":queries})
                    );
                    assert!(
                        covers(r.evidence.reduced_gradient[0], dx),
                        "{case} {route:?} gradient"
                    );
                    assert!(
                        covers(r.evidence.reduced_clock, dt),
                        "{case} {route:?} clock"
                    );
                    assert!(
                        covers(r.evidence.velocity, dx + dt),
                        "{case} {route:?} velocity"
                    );
                    let mut values = Vec::new();
                    for t in [0., step / 2., step] {
                        let v = r
                            .value_at(t, cert.whole_x_error.unwrap())
                            .unwrap()
                            .as_pair();
                        let exact = if name == "algebraic" {
                            1.5 * t - 0.0625
                        } else {
                            0.25 * t - 0.015625
                        };
                        assert!(
                            covers(v, exact),
                            "{case} {name} {route:?} {value:?} time {t}"
                        );
                        values.push(v);
                    }
                    assert_eq!(values, queries);
                    println!(
                        "VECTOR {}",
                        serde_json::json!({"case":case,"guard":name,"derivative":route,"value":value,"allocation":allocation,"audit":audit,"evidence":r.evidence,"values":values,"branch_defect":cert.branch_defect})
                    );
                }
            }
            let mut audit = de::GuardKernelAudit::default();
            let fail = de::GuardProof::new_recorded(
                &src,
                &guard,
                &power,
                &cert,
                GuardDerivativeRoute::ConservedSchurAffineGraph,
                GuardValueRoute::Box,
                &mut audit,
            );
            assert!(
                fail.is_err()
                    && audit.builds.started == 1
                    && audit.builds.returned == 0
                    && audit.refinement_sweeps.started == 0
            );
        }
    }
}

#[test]
#[ignore = "authentic scalar reconstruction portfolio with fixed and continuation seed"]
fn scalar_reconstruction_portfolio() {
    use de::ReconstructionSeedRoute as S;
    use rodas5p_integrators::retro_dae_init::{InitWork, ProjectionAttemptWork};
    warm();
    const BRANCH: [[f64; 2]; 1] = [[-0.25, 1.25]];
    for solver in [
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let run = evolve_robertson_phase_atlas(
                &modes,
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
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let src = model.source();
            let init = src
                .initialize(
                    0.,
                    &phase.initial.center(),
                    &[0.125],
                    &BRANCH,
                    policy().init,
                    InitRoute::Reuse,
                )
                .unwrap();
            let slabs = phase.original_slab_diagnostics();
            for (position, k) in [
                ("first", 0),
                ("middle", slabs.len() / 2),
                ("last", slabs.len() - 1),
            ] {
                let s = &slabs[k];
                let c = s.certificate.as_ref().unwrap();
                let x =
                    de::dense_power(&s.native.y0, &s.native.y1, &s.native.stages, s.native.basis)
                        .unwrap();
                assert_eq!(x.len(), src.nx());
                let ir = match solver {
                    EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
                    EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
                    EvolutionRoute::Affine5 => InitRoute::AffineExact,
                    _ => InitRoute::Reuse,
                };
                let ((legacy, la), alloc) = measure(|| {
                    de::reconstruct_audited(
                        src,
                        s.times,
                        &x,
                        &init.z,
                        &BRANCH,
                        policy().init,
                        ir,
                        None,
                        5,
                    )
                });
                let (legacy, legacy_work) = legacy.unwrap();
                // Compare complete original-source slab certificates separately;
                // non-default candidate powers are never committed without qualification.
                let emit = |name: &str,
                            poly: &[Iv],
                            work: &[InitWork],
                            audit: &ProjectionAttemptWork,
                            allocation: Counts| {
                    let mut full = x.clone();
                    full.push(poly.to_vec());
                    let r = model.certify(
                        s.times,
                        &full,
                        &BRANCH,
                        policy().tube_radius,
                        Some(c.incoming),
                        policy().init.condition_cap,
                        cert,
                    );
                    let result = match r {
                        Ok(c) => {
                            serde_json::json!({"qualified":c.global_qualified&&c.whole_x_error.is_some_and(|v|v<=policy().global_tol)&&c.whole_z_error.is_some_and(|v|v<=policy().global_tol),"whole_x":c.whole_x_error,"whole_z":c.whole_z_error,"original_g":c.original_g_upper})
                        }
                        Err(e) => serde_json::json!({"qualified":false,"error":e.to_string()}),
                    };
                    println!(
                        "RECON {}",
                        serde_json::json!({"solver":solver,"cert":cert,"position":position,"candidate":name,"allocation":allocation,"power":poly.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"work":work,"audit":audit,"result":result})
                    );
                };
                emit("existing", &legacy[0], &legacy_work, &la, alloc);
                let mut scratch = de::ScalarReconstructionWorkspace::new();
                let mut out = [Iv::point(123.).unwrap(); 5];
                let mut work: [InitWork; 5] = std::array::from_fn(|_| InitWork::default());
                for (name, route) in [("fixed", S::Fixed), ("previous", S::PreviousNode)] {
                    let mut audit = ProjectionAttemptWork::default();
                    let (r, alloc) = measure(|| {
                        de::reconstruct_scalar_reuse_into(
                            src,
                            s.times,
                            &x,
                            &init.z,
                            &BRANCH,
                            policy().init,
                            route,
                            &mut out,
                            &mut work,
                            &mut audit,
                            &mut scratch,
                        )
                    });
                    r.unwrap();
                    assert_eq!(alloc.requests(), 0);
                    assert_eq!(alloc.requested_bytes, 0);
                    assert_eq!((audit.calls.started, audit.calls.returned), (5, 5));
                    if route == S::Fixed && ir == InitRoute::Reuse {
                        same(
                            &out.map(|v| v.as_pair()),
                            &legacy[0].iter().map(|v| v.as_pair()).collect::<Vec<_>>(),
                        );
                        same(&work, &legacy_work);
                        same(&audit, &la);
                    }
                    emit(name, &out, &work, &audit, alloc);
                }
                let saved = out;
                let saved_work = serde_json::to_value(&work).unwrap();
                let mut audit = ProjectionAttemptWork::default();
                assert!(
                    de::reconstruct_scalar_reuse_into(
                        src,
                        s.times,
                        &x,
                        &[f64::NAN],
                        &BRANCH,
                        policy().init,
                        S::Fixed,
                        &mut out,
                        &mut work,
                        &mut audit,
                        &mut scratch
                    )
                    .is_err()
                );
                assert_eq!(out.map(|v| v.as_pair()), saved.map(|v| v.as_pair()));
                assert_eq!(serde_json::to_value(&work).unwrap(), saved_work);
                assert_eq!((audit.calls.started, audit.calls.returned), (1, 0));
            }
        }
    }
}

#[test]
#[ignore = "bounded prepared scalar actual callback/reconstruction/atlas candidates"]
fn bounded_prepared_scalar_portfolio() {
    use de::ReconstructionSeedRoute as S;
    use rodas5p_integrators::retro_dae_init::{InitWork, ProjectionAttemptWork};
    warm();
    const BRANCH: [[f64; 2]; 1] = [[-0.25, 1.25]];
    for solver in [
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
        EvolutionRoute::BoundedPrepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let (run, allocation) = measure(|| {
                evolve_robertson_phase_atlas(
                    &modes,
                    0,
                    initial(2f64.powi(-26)),
                    [0., 0.002],
                    &opts(),
                    policy(),
                    pm(MultiGuardRoute::Correlated),
                    atlas_budget(),
                    PhaseReuseRoute::Reuse,
                )
            });
            let run = run.unwrap();
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            let phase = &run.phases()[0];
            let model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let src = model.source();
            let init = src
                .initialize(
                    0.,
                    &phase.initial.center(),
                    &[0.125],
                    &BRANCH,
                    policy().init,
                    InitRoute::Reuse,
                )
                .unwrap();
            println!(
                "PREPARED_ATLAS {}",
                serde_json::json!({"solver":solver,"cert":cert,"allocation":allocation,"stop":run.stop(),"phases":run.phases().len(),"slabs":run.phases().iter().map(|p|p.original_slab_diagnostics().len()).sum::<usize>()})
            );
            let prepared = initialize_reduced_problem(
                src,
                0.,
                &phase.initial.center(),
                &[0.125],
                &BRANCH,
                policy(),
                solver,
            )
            .unwrap();
            let slabs = phase.original_slab_diagnostics();
            let ir = match solver {
                EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
                EvolutionRoute::BoundedPrepared5 => InitRoute::BoundedPreparedScalar,
                EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
                EvolutionRoute::Affine5 => InitRoute::AffineExact,
                _ => InitRoute::Reuse,
            };
            for (position, k) in [
                ("first", 0),
                ("middle", slabs.len() / 2),
                ("last", slabs.len() - 1),
            ] {
                let s = &slabs[k];
                let time = s.times[0];
                let x = &s.native.y0;
                let mut rhs = [0.; 2];
                let mut jac = [0.; 4];
                let mut ft = [0.; 2];
                let (r, ar) = measure(|| (prepared.problem.rhs_fn())(time, x, &mut rhs));
                r.unwrap();
                let vigilode::Jacobian::Dense(j) = prepared.problem.jacobian_provider() else {
                    panic!("dense")
                };
                let (r, aj) = measure(|| j(time, x, &mut jac));
                r.unwrap();
                let (r, at) =
                    measure(|| prepared.problem.time_derivative_fn().unwrap()(time, x, &mut ft));
                r.unwrap();
                if solver == EvolutionRoute::BoundedPrepared5 {
                    for a in [ar, aj, at] {
                        assert_eq!(a.requests(), 0);
                    }
                }
                println!(
                    "PREPARED_CALLBACK {}",
                    serde_json::json!({"solver":solver,"cert":cert,"position":position,"rhs":rhs,"jac":jac,"ft":ft,"allocations":[ar,aj,at]})
                );
                let (old, ow) = src.initialize_audited(
                    time,
                    x,
                    &init.z,
                    &BRANCH,
                    policy().init,
                    InitRoute::PreparedBatch,
                );
                let old = old.unwrap();
                let ((r, w), a) = measure(|| {
                    src.initialize_scalar_bounded_prepared_audited(
                        time,
                        x,
                        &init.z,
                        &BRANCH,
                        policy().init,
                    )
                });
                let r = r.unwrap();
                assert_eq!(a.requests(), 0);
                let owned = r.to_owned_report();
                assert_eq!(owned.route, InitRoute::BoundedPreparedScalar);
                let mut u = serde_json::to_value(&old).unwrap();
                let mut v = serde_json::to_value(&owned).unwrap();
                for field in ["route", "work"] {
                    u.as_object_mut().unwrap().remove(field);
                    v.as_object_mut().unwrap().remove(field);
                }
                assert_eq!(u, v);
                assert_eq!(ow.rhs_requested, w.rhs_requested);
                assert_eq!(w.legacy.bounded_prepared_batches, w.legacy.linear_batches);
                assert_eq!(w.legacy.bounded_residual_terms, w.rhs_returned);
                println!(
                    "PREPARED_POINT {}",
                    serde_json::json!({"solver":solver,"cert":cert,"position":position,"allocation":a,"legacy_work":ow,"bounded_work":w})
                );
                let power =
                    de::dense_power(&s.native.y0, &s.native.y1, &s.native.stages, s.native.basis)
                        .unwrap();
                let c = s.certificate.as_ref().unwrap();
                let ((r, wa), aa) = measure(|| {
                    de::reconstruct_audited(
                        src,
                        s.times,
                        &power,
                        &init.z,
                        &BRANCH,
                        policy().init,
                        ir,
                        None,
                        5,
                    )
                });
                let (r, ww) = r.unwrap();
                let emit = |candidate: &str,
                            poly: &[Iv],
                            work: &[InitWork],
                            audit: &ProjectionAttemptWork,
                            alloc: Counts| {
                    let mut full = power.clone();
                    full.push(poly.to_vec());
                    let cert = model
                        .certify(
                            s.times,
                            &full,
                            &BRANCH,
                            policy().tube_radius,
                            Some(c.incoming),
                            policy().init.condition_cap,
                            cert,
                        )
                        .unwrap();
                    assert!(cert.global_qualified);
                    println!(
                        "PREPARED_RECON {}",
                        serde_json::json!({"solver":solver,"cert":c.conservation.as_ref().unwrap().route,"position":position,"candidate":candidate,"allocation":alloc,"work":work,"audit":audit,"qualified":cert.global_qualified,"whole_x":cert.whole_x_error,"whole_z":cert.whole_z_error,"original_g":cert.original_g_upper,"power":poly.iter().map(|v|v.as_pair()).collect::<Vec<_>>()})
                    );
                };
                emit("existing", &r[0], &ww, &wa, aa);
                let mut scratch = de::ScalarReconstructionWorkspace::new();
                let mut out = [Iv::point(123.).unwrap(); 5];
                let mut work: [InitWork; 5] = std::array::from_fn(|_| InitWork::default());
                for (candidate, route) in [
                    ("bounded_fixed", S::Fixed),
                    ("bounded_previous", S::PreviousNode),
                ] {
                    let mut audit = ProjectionAttemptWork::default();
                    let (res, alloc) = measure(|| {
                        de::reconstruct_scalar_bounded_prepared_into(
                            src,
                            s.times,
                            &power,
                            &init.z,
                            &BRANCH,
                            policy().init,
                            route,
                            &mut out,
                            &mut work,
                            &mut audit,
                            &mut scratch,
                        )
                    });
                    res.unwrap();
                    assert_eq!(alloc.requests(), 0);
                    assert_eq!((audit.calls.started, audit.calls.returned), (5, 5));
                    assert_eq!(audit.bounded_residual_terms, audit.rhs_returned);
                    if matches!(
                        ir,
                        InitRoute::PreparedBatch
                            | InitRoute::BoundedPreparedScalar
                            | InitRoute::Reuse
                    ) {
                        same(
                            &out.map(|v| v.as_pair()),
                            &r[0].iter().map(|v| v.as_pair()).collect::<Vec<_>>(),
                        );
                    }
                    emit(candidate, &out, &work, &audit, alloc);
                }
            }
        }
    }
}

#[test]
fn legendre_residual_portfolio() {
    use de::SlabResidualRoute as R;
    warm();
    let iv = |x| Iv::point(x).unwrap();
    let lin = |k, w| tr(w, vec![F::component(k, 0.)]);
    let clk = || F::clock(0.);
    let y = |k| F::component(k, 0.);
    let source = |quartic: bool| {
        DaeSource::new(
            vec![p(vec![
                tr(1., vec![]),
                tr(
                    0.0625,
                    if quartic {
                        vec![y(0); 4]
                    } else {
                        vec![y(0); 2]
                    },
                ),
            ])],
            vec![p(if quartic {
                vec![
                    lin(1, 1.),
                    lin(0, -1.),
                    tr(1., vec![clk(), clk(), clk(), y(1)]),
                    tr(-1., vec![clk(), clk(), clk(), y(0)]),
                ]
            } else {
                vec![
                    lin(1, 1.),
                    lin(0, -1.),
                    tr(0.125, vec![clk(), y(1)]),
                    tr(-0.125, vec![clk(), y(0)]),
                ]
            })],
        )
        .unwrap()
    };
    let routes = [
        R::Bernstein,
        R::LegendreInterval,
        R::LegendreCentered,
        R::HybridInterval,
        R::HybridCentered,
    ];
    // Oscillatory represented residual, full source still re-certified. A
    // point coefficient in the Legendre basis is not assumed exact.
    let src = source(true);
    let eps = 2f64.powi(-20);
    let x = vec![iv(0.), iv(0.125), iv(0.), iv(0.), iv(0.)];
    let z = vec![
        iv(eps),
        iv(0.125 - 20. * eps),
        iv(90. * eps),
        iv(-140. * eps),
        iv(70. * eps),
    ];
    let poly = vec![x, z];
    let c = de::certify_slab(
        &src,
        [0., 0.125],
        &poly,
        &[[-1., 1.]],
        0.01,
        Some(0.),
        true,
        1e6,
    )
    .unwrap();
    assert!(c.global_qualified);
    for route in routes {
        let (r, alloc) = measure(|| {
            de::slab_residual_bounds(&src, [0., 0.125], &poly, &c.preconditioner, route)
        });
        let r = r.unwrap();
        assert_eq!(alloc.requests(), 0);
        if matches!(route, R::LegendreCentered | R::HybridCentered) {
            assert!(r.dictionary_center_products > 0);
        }
        let cert = de::certify_slab_with_residual(
            &src,
            [0., 0.125],
            &poly,
            &[[-1., 1.]],
            0.01,
            Some(0.),
            true,
            1e6,
            route,
        )
        .unwrap();
        assert!(cert.global_qualified);
        for k in 0..=64 {
            let u = k as f64 / 64.;
            let t = u * 0.125;
            let g = (1. + t * t * t)
                * eps
                * (70. * u.powi(4) - 140. * u.powi(3) + 90. * u * u - 20. * u + 1.);
            let dx = 0.125 - (0.125 * (1. + t.powi(4) * 0.0625));
            assert!(g.abs() <= r.original_g_upper && dx.abs() <= r.differential_defect);
        }
        println!(
            "LEGENDRE {}",
            serde_json::json!({"case":"oscillatory","solver":"manufactured","position":"one","route":route,"allocation":alloc,"bounds":r,"qualified":cert.global_qualified,"whole_x":cert.whole_x_error,"whole_z":cert.whole_z_error})
        );
    }
    for (case, quartic) in [("quadratic", false), ("quartic", true)] {
        let src = source(quartic);
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Reduced5,
            EvolutionRoute::Prepared5,
        ] {
            let (run, a) = evolve_epoch_audited(
                &src,
                &[0.],
                &[0.],
                &[[-1., 1.]],
                EvolutionEpoch {
                    start: 0.,
                    end: 0.125,
                    incoming_x_error: Some(0.),
                },
                &opts(),
                policy(),
                solver,
                &[],
            );
            let run = run.unwrap();
            assert!(run.global_qualified, "{case} {solver:?} {:?}", run.reason);
            let slabs = run.slabs();
            for (position, k) in [
                ("first", 0),
                ("middle", slabs.len() / 2),
                ("last", slabs.len() - 1),
            ] {
                let s = &slabs[k];
                let c = s.certificate.as_ref().unwrap();
                let power = power(s);
                for route in routes {
                    let (r, alloc) = measure(|| {
                        de::slab_residual_bounds(&src, s.times, &power, &c.preconditioner, route)
                    });
                    let r = r.unwrap();
                    assert_eq!(alloc.requests(), 0);
                    if matches!(route, R::LegendreCentered | R::HybridCentered) {
                        assert!(r.dictionary_center_products > 0);
                    }
                    let v = de::certify_slab_with_residual(
                        &src,
                        s.times,
                        &power,
                        &[[-1., 1.]],
                        policy().tube_radius,
                        Some(c.incoming),
                        policy().green,
                        policy().init.condition_cap,
                        route,
                    )
                    .unwrap();
                    if route == R::Bernstein {
                        same(&v, c);
                        assert_eq!(r.original_g_upper, c.original_g_upper);
                        assert_eq!(r.differential_defect, c.differential_defect);
                    } else {
                        assert!(v.residual_variant.is_some());
                    }
                    if matches!(route, R::HybridCentered | R::HybridInterval) {
                        assert!(
                            v.original_g_upper <= c.original_g_upper
                                && v.algebraic_defect <= c.algebraic_defect
                                && v.differential_defect <= c.differential_defect
                        );
                    }
                    println!(
                        "LEGENDRE {}",
                        serde_json::json!({"case":case,"solver":solver,"position":position,"route":route,"allocation":alloc,"bounds":r,"qualified":v.global_qualified,"whole_x":v.whole_x_error,"whole_z":v.whole_z_error})
                    );
                }
            }
            black_box(a);
        }
        for solver in [
            EvolutionRoute::Reduced5,
            EvolutionRoute::Prepared5,
            EvolutionRoute::BoundedPrepared5,
            EvolutionRoute::Reduced5Legendre,
            EvolutionRoute::BoundedPreparedLegendre5,
        ] {
            let ((run, a), alloc) = measure(|| {
                evolve_epoch_audited(
                    &src,
                    &[0.],
                    &[0.],
                    &[[-1., 1.]],
                    EvolutionEpoch {
                        start: 0.,
                        end: 0.125,
                        incoming_x_error: Some(0.),
                    },
                    &opts(),
                    policy(),
                    solver,
                    &[],
                )
            });
            let run = run.unwrap();
            assert!(run.global_qualified, "{case} {solver:?} {:?}", run.reason);
            let mut products = 0;
            for s in run.slabs() {
                if let Some(r) = s.certificate.as_ref().unwrap().residual_variant {
                    products += r.dictionary_center_products;
                }
            }
            if matches!(
                solver,
                EvolutionRoute::Reduced5Legendre | EvolutionRoute::BoundedPreparedLegendre5
            ) {
                assert!(products > 0);
            }
            println!(
                "SPECTRAL_EPOCH {}",
                serde_json::json!({"case":case,"solver":solver,"allocation":alloc,"qualified":run.global_qualified,"max_physical_error":run.max_physical_error,"slabs":run.slabs().len(),"dictionary_center_products":products,"audit":a})
            );
        }
    }
}

#[test]
fn bounded_affine_actual_callbacks_noalloc() {
    warm();
    let src = RobertsonModel::new(3e7).unwrap();
    let x = [1. - 1. / 8192., 1. / 8192.];
    let branch = [[-0.25, 1.25]];
    let client = initialize_reduced_problem(
        src.source(),
        0.,
        &x,
        &[0.],
        &branch,
        policy(),
        EvolutionRoute::BoundedAffine5,
    )
    .unwrap();
    let old = initialize_reduced_problem(
        src.source(),
        0.,
        &x,
        &[0.],
        &branch,
        policy(),
        EvolutionRoute::Affine5,
    )
    .unwrap();
    let t = 0.0005;
    let y = [0.9998658289383162, 6.457305003072096e-5];
    let mut rhs = [0.; 2];
    let mut jac = [0.; 4];
    let mut ft = [0.; 2];
    let mut orhs = [0.; 2];
    let mut ojac = [0.; 4];
    let mut oft = [0.; 2];
    (old.problem.rhs_fn())(t, &y, &mut orhs).unwrap();
    let vigilode::Jacobian::Dense(j) = old.problem.jacobian_provider() else {
        panic!("dense")
    };
    j(t, &y, &mut ojac).unwrap();
    old.problem.time_derivative_fn().unwrap()(t, &y, &mut oft).unwrap();
    let (r, a) = measure(|| (client.problem.rhs_fn())(t, &y, &mut rhs));
    r.unwrap();
    let vigilode::Jacobian::Dense(j) = client.problem.jacobian_provider() else {
        panic!("dense")
    };
    let (r, b) = measure(|| j(t, &y, &mut jac));
    r.unwrap();
    let (r, c) = measure(|| client.problem.time_derivative_fn().unwrap()(t, &y, &mut ft));
    r.unwrap();
    assert_eq!(rhs.map(f64::to_bits), orhs.map(f64::to_bits));
    assert_eq!(jac.map(f64::to_bits), ojac.map(f64::to_bits));
    assert_eq!(ft.map(f64::to_bits), oft.map(f64::to_bits));
    println!(
        "AFFINE_RED_GREEN {}",
        serde_json::json!({"rhs":rhs,"jac":jac,"time":ft,"allocations":[a,b,c]})
    );
    assert_eq!(
        [a.requests(), b.requests(), c.requests()],
        [0; 3],
        "actual bounded affine callback allocations"
    );
}

#[test]
#[ignore = "actual affine root and reconstruction measured portfolio"]
fn bounded_affine_root_portfolio() {
    use de::ReconstructionSeedRoute as S;
    use rodas5p_integrators::retro_dae_init::{InitWork, ProjectionAttemptWork};
    warm();
    const BRANCH: [[f64; 2]; 1] = [[-0.25, 1.25]];
    let model = RobertsonModel::new(3e7).unwrap();
    let src = model.source();
    for solver in [
        EvolutionRoute::Affine5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::BoundedPrepared5,
        EvolutionRoute::BoundedAffine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            let modes = graph("alternating", solver, cert);
            let (run, a) = measure(|| {
                evolve_robertson_phase_atlas(
                    &modes,
                    0,
                    initial(2f64.powi(-26)),
                    [0., 0.002],
                    &opts(),
                    policy(),
                    pm(MultiGuardRoute::Correlated),
                    atlas_budget(),
                    PhaseReuseRoute::Reuse,
                )
            });
            let run = run.unwrap();
            assert_eq!(run.stop(), PhaseAtlasStop::Horizon);
            let phase = &run.phases()[0];
            let src_model = RobertsonModel::new(modes[phase.mode].rate).unwrap();
            let src = src_model.source();
            let seed = src
                .initialize(
                    0.,
                    &phase.initial.center(),
                    &[0.125],
                    &BRANCH,
                    policy().init,
                    InitRoute::Reuse,
                )
                .unwrap()
                .z;
            println!(
                "AFFINE_ATLAS {}",
                serde_json::json!({"solver":solver,"cert":cert,"allocation":a,"stop":run.stop(),"phases":run.phases().len(),"slabs":run.phases().iter().map(|p|p.original_slab_diagnostics().len()).sum::<usize>()})
            );
            let client = initialize_reduced_problem(
                src,
                0.,
                &phase.initial.center(),
                &[0.125],
                &BRANCH,
                policy(),
                solver,
            )
            .unwrap();
            let slabs = phase.original_slab_diagnostics();
            for (position, k) in [
                ("first", 0),
                ("middle", slabs.len() / 2),
                ("last", slabs.len() - 1),
            ] {
                let s = &slabs[k];
                let time = s.times[0];
                let x = &s.native.y0;
                let c = s.certificate.as_ref().unwrap();
                let mut rhs = [0.; 2];
                let mut jac = [0.; 4];
                let mut ft = [0.; 2];
                let (r, ar) = measure(|| (client.problem.rhs_fn())(time, x, &mut rhs));
                r.unwrap();
                let vigilode::Jacobian::Dense(j) = client.problem.jacobian_provider() else {
                    panic!("dense")
                };
                let (r, aj) = measure(|| j(time, x, &mut jac));
                r.unwrap();
                let (r, at) =
                    measure(|| client.problem.time_derivative_fn().unwrap()(time, x, &mut ft));
                r.unwrap();
                if matches!(
                    solver,
                    EvolutionRoute::BoundedAffine5 | EvolutionRoute::BoundedPrepared5
                ) {
                    for a in [ar, aj, at] {
                        assert_eq!(a.requests(), 0);
                    }
                }
                println!(
                    "AFFINE_CALLBACK {}",
                    serde_json::json!({"solver":solver,"cert":cert,"position":position,"rhs":rhs,"jac":jac,"time":ft,"allocations":[ar,aj,at]})
                );
                for route in [
                    InitRoute::AffineExact,
                    InitRoute::ScalarRoot,
                    InitRoute::BoundedAffineIntervalScalar,
                    InitRoute::BoundedAffineScalar,
                    InitRoute::BoundedPreparedScalar,
                ] {
                    let ((r, w), alloc) = measure(|| {
                        src.initialize_audited(time, x, &seed, &BRANCH, policy().init, route)
                    });
                    let r = r.unwrap();
                    assert!(
                        r.qualified
                            && r.residual_upper <= policy().init.residual_tol
                            && r.state_error_upper <= policy().init.state_tol
                    );
                    println!(
                        "AFFINE_POINT {}",
                        serde_json::json!({"solver":solver,"cert":cert,"position":position,"route":route,"x":x,"report":r,"audit":w,"allocation":alloc})
                    );
                }
                let ((r, w), alloc) = measure(|| {
                    src.initialize_scalar_bounded_affine_audited(
                        time,
                        x,
                        &seed,
                        &BRANCH,
                        policy().init,
                    )
                });
                let r = r.unwrap();
                assert_eq!(alloc.requests(), 0);
                assert_eq!(w.legacy.affine_dyadic_success, 1);
                println!(
                    "AFFINE_CORE {}",
                    serde_json::json!({"solver":solver,"cert":cert,"position":position,"allocation":alloc,"z":r.z(),"work":w})
                );
                let xp =
                    de::dense_power(&s.native.y0, &s.native.y1, &s.native.stages, s.native.basis)
                        .unwrap();
                let ir = match solver {
                    EvolutionRoute::Affine5 => InitRoute::AffineExact,
                    EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
                    EvolutionRoute::BoundedAffine5 => InitRoute::BoundedAffineScalar,
                    _ => InitRoute::BoundedPreparedScalar,
                };
                let ((old, ow), alloc) = measure(|| {
                    de::reconstruct_audited(
                        src,
                        s.times,
                        &xp,
                        &seed,
                        &BRANCH,
                        policy().init,
                        ir,
                        None,
                        5,
                    )
                });
                let (old, oldw) = old.unwrap();
                let emit = |candidate: &str,
                            z: &[Iv],
                            work: &[InitWork],
                            audit: &ProjectionAttemptWork,
                            allocation: Counts| {
                    let mut full = xp.clone();
                    full.push(z.to_vec());
                    let r = src_model
                        .certify(
                            s.times,
                            &full,
                            &BRANCH,
                            policy().tube_radius,
                            Some(c.incoming),
                            policy().init.condition_cap,
                            cert,
                        )
                        .unwrap();
                    assert!(
                        r.global_qualified
                            && r.whole_x_error.unwrap() <= policy().global_tol
                            && r.whole_z_error.unwrap() <= policy().global_tol
                    );
                    println!(
                        "AFFINE_RECON {}",
                        serde_json::json!({"solver":solver,"cert":cert,"position":position,"candidate":candidate,"allocation":allocation,"work":work,"audit":audit,"whole_x":r.whole_x_error,"whole_z":r.whole_z_error,"original_g":r.original_g_upper,"power":z.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"qualified":r.global_qualified})
                    );
                };
                emit("existing", &old[0], &oldw, &ow, alloc);
                let mut scratch = de::ScalarReconstructionWorkspace::new();
                let mut out = [Iv::point(123.).unwrap(); 5];
                let mut work: [InitWork; 5] = std::array::from_fn(|_| InitWork::default());
                for (candidate, seed_route) in [
                    ("bounded_fixed", S::Fixed),
                    ("bounded_previous", S::PreviousNode),
                ] {
                    let mut audit = ProjectionAttemptWork::default();
                    let (r, alloc) = measure(|| {
                        de::reconstruct_scalar_bounded_affine_into(
                            src,
                            s.times,
                            &xp,
                            &seed,
                            &BRANCH,
                            policy().init,
                            seed_route,
                            &mut out,
                            &mut work,
                            &mut audit,
                            &mut scratch,
                        )
                    });
                    r.unwrap();
                    assert_eq!(alloc.requests(), 0);
                    assert_eq!((audit.calls.started, audit.calls.returned), (5, 5));
                    emit(candidate, &out, &work, &audit, alloc);
                }
            }
        }
    }
    black_box(src);
}

#[test]
fn bounded_affine_source_refusals_and_cancellation() {
    let z = || F::component(1, 0.);
    let x = || F::component(0, 0.);
    let mk = |terms| DaeSource::new(vec![p(vec![tr(1., vec![])])], vec![p(terms)]).unwrap();
    let cases = [
        (
            "affine",
            mk(vec![tr(3., vec![z()]), tr(-1., vec![x()])]),
            true,
        ),
        (
            "cancelled_degree4",
            mk(vec![
                tr(1., vec![z(), z(), z(), z()]),
                tr(-1., vec![z(), z(), z(), z()]),
                tr(1., vec![z()]),
                tr(-1., vec![x()]),
            ]),
            true,
        ),
        (
            "nonlinear",
            mk(vec![
                tr(1., vec![z()]),
                tr(-1., vec![x()]),
                tr(2f64.powi(-40), vec![z(), z()]),
            ]),
            false,
        ),
        ("zero_slope", mk(vec![tr(1., vec![x()])]), false),
        (
            "outside",
            mk(vec![tr(1., vec![z()]), tr(-2., vec![])]),
            false,
        ),
        (
            "boundary",
            mk(vec![tr(1., vec![z()]), tr(-1., vec![])]),
            false,
        ),
    ];
    for (case, src, ok) in cases {
        let (r, w) = src.initialize_scalar_bounded_affine_audited(
            0.,
            &[0.125],
            &[0.],
            &[[-1., 1.]],
            policy().init,
        );
        assert_eq!(r.is_ok(), ok, "{case} {:?}", r.as_ref().err());
        if let Ok(r) = r {
            let report = r.to_owned_report();
            assert!(report.affine_root.is_none() && report.branch_defect_upper.is_none());
            let value = if case == "affine" { 0.125 / 3. } else { 0.125 };
            assert!(report.root_box[0][0] <= value && value <= report.root_box[0][1]);
        }
        println!(
            "AFFINE_REFUSAL {}",
            serde_json::json!({"case":case,"accepted":ok,"audit":w})
        );
    }
}

#[test]
fn source_picard_endpoint_portfolio() {
    use de::PicardEndpointRoute as R;
    warm();
    let iv = |x| Iv::point(x).unwrap();
    let y = |k| F::component(k, 0.);
    let t = || F::clock(0.);
    let source = |case: &str| {
        DaeSource::new(
            if case == "noncommuting" {
                vec![
                    p(vec![tr(3., vec![y(1)]), tr(3., vec![t(), t()])]),
                    p(vec![tr(2., vec![t()])]),
                ]
            } else {
                vec![
                    p(vec![tr(-0.5, vec![y(0)]), tr(0.125, vec![t(), t()])]),
                    p(vec![tr(-0.25, vec![y(1)])]),
                ]
            },
            vec![p(vec![
                tr(1., vec![y(2)]),
                tr(-1., vec![y(0)]),
                tr(-1., vec![y(1)]),
            ])],
        )
        .unwrap()
    };
    let routes = [
        R::OrderedWords,
        R::Polynomial3,
        R::Polynomial5,
        R::Polynomial7,
        R::Adaptive7,
    ];
    let src = source("noncommuting");
    // Exact represented solution x0=2*t^3, x1=t^2; perturb with
    // epsilon*u*(1-u) to create nonzero residual but exact endpoint.
    for (case, h) in [("short", 0.125), ("long", 0.5)] {
        let eps = 2f64.powi(-20);
        let q = 2. * h * h * h;
        let r = h * h;
        let power = vec![
            vec![iv(0.), iv(eps), iv(-eps), iv(q)],
            vec![iv(0.), iv(0.), iv(r)],
            vec![iv(0.), iv(eps), iv(r - eps), iv(q)],
        ];
        let c = de::certify_slab(
            &src,
            [0., h],
            &power,
            &[[-2., 2.]],
            0.01,
            Some(0.),
            true,
            1e6,
        )
        .unwrap();
        assert!(c.global_qualified);
        for route in routes {
            let (v, a) = measure(|| de::source_picard_endpoint(&src, [0., h], &power, 0., route));
            let v = v.unwrap();
            if route != R::OrderedWords {
                assert_eq!(a.requests(), 0);
            }
            let cc = de::certify_slab_with_picard(
                &src,
                [0., h],
                &power,
                &[[-2., 2.]],
                0.01,
                Some(0.),
                true,
                1e6,
                route,
            )
            .unwrap();
            assert_eq!(cc.whole_x_error, c.whole_x_error);
            assert_eq!(cc.whole_z_error, c.whole_z_error);
            assert!(cc.outgoing.unwrap() <= c.outgoing.unwrap());
            if route == R::Adaptive7 {
                assert_eq!(v.tail_upper, 0.);
                assert_eq!(v.candidate_error, 0.);
            }
            println!(
                "PICARD_BOUND {}",
                serde_json::json!({"case":case,"solver":"manufactured","position":"one","route":route,"allocation":a,"bounds":v,"original_outgoing":c.outgoing,"outgoing":cc.outgoing,"whole_x":cc.whole_x_error,"qualified":cc.global_qualified})
            );
        }
    }
    for case in ["noncommuting", "stable"] {
        let src = source(case);
        let x = if case == "stable" {
            [0.25, -0.125]
        } else {
            [0., 0.]
        };
        let z = [x[0] + x[1]];
        let branch = [[-2., 2.]];
        let epoch = EvolutionEpoch {
            start: 0.,
            end: 0.125,
            incoming_x_error: Some(0.),
        };
        for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            let run =
                evolve_epoch(&src, &x, &z, &branch, epoch, &opts(), policy(), solver, &[]).unwrap();
            assert!(run.global_qualified, "{case} {solver:?} {:?}", run.reason);
            let slabs = run.slabs();
            for (position, k) in [
                ("first", 0),
                ("middle", slabs.len() / 2),
                ("last", slabs.len() - 1),
            ] {
                let s = &slabs[k];
                let c = s.certificate.as_ref().unwrap();
                let power = power(s);
                for route in routes {
                    let (v, a) = measure(|| {
                        de::source_picard_endpoint(&src, s.times, &power, c.incoming, route)
                    });
                    let v = v.unwrap();
                    if route != R::OrderedWords {
                        assert_eq!(a.requests(), 0);
                    }
                    let cc = de::certify_slab_with_picard(
                        &src,
                        s.times,
                        &power,
                        &branch,
                        policy().tube_radius,
                        Some(c.incoming),
                        policy().green,
                        policy().init.condition_cap,
                        route,
                    )
                    .unwrap();
                    assert!(cc.global_qualified && cc.outgoing.unwrap() <= c.outgoing.unwrap());
                    assert_eq!(cc.whole_x_error, c.whole_x_error);
                    assert_eq!(cc.whole_z_error, c.whole_z_error);
                    println!(
                        "PICARD_BOUND {}",
                        serde_json::json!({"case":case,"solver":solver,"position":position,"route":route,"allocation":a,"bounds":v,"original_outgoing":c.outgoing,"outgoing":cc.outgoing,"whole_x":cc.whole_x_error,"qualified":cc.global_qualified,"times":s.times,"power":s.power,"incoming":c.incoming})
                    );
                }
            }
        }
        for solver in [
            EvolutionRoute::Reduced5,
            EvolutionRoute::Reduced5OrderedPicard,
            EvolutionRoute::Reduced5PolynomialPicard,
            EvolutionRoute::BoundedAffinePicard5,
        ] {
            let ((run, a), alloc) = measure(|| {
                evolve_epoch_audited(&src, &x, &z, &branch, epoch, &opts(), policy(), solver, &[])
            });
            let run = run.unwrap();
            assert!(run.global_qualified, "{case} {solver:?} {:?}", run.reason);
            let mut words = 0;
            let mut actions = 0;
            let mut tightened = 0;
            let mut receipts = 0;
            for s in run.slabs() {
                let c = s.certificate.as_ref().unwrap();
                if let Some(r) = c.picard_endpoint {
                    receipts += 1;
                    actions += r.matrix_actions;
                    if r.route == R::OrderedWords {
                        words += r.matrix_actions;
                    }
                    let b = de::certify_slab(
                        &src,
                        s.times,
                        &power(s),
                        &branch,
                        policy().tube_radius,
                        Some(c.incoming),
                        policy().green,
                        policy().init.condition_cap,
                    )
                    .unwrap();
                    tightened += usize::from(c.outgoing.unwrap() < b.outgoing.unwrap());
                }
            }
            if solver == EvolutionRoute::Reduced5OrderedPicard {
                assert!(words > 0);
            }
            if solver != EvolutionRoute::Reduced5 {
                assert_eq!(receipts, run.slabs().len());
            }
            println!(
                "PICARD_EPOCH {}",
                serde_json::json!({"case":case,"solver":solver,"allocation":alloc,"slabs":run.slabs().len(),"qualified":run.global_qualified,"max_physical_error":run.max_physical_error,"outgoing":run.slabs().last().unwrap().certificate.as_ref().unwrap().outgoing,"receipts":receipts,"ordered_words":words,"matrix_actions":actions,"tightened":tightened,"audit":a})
            );
        }
    }
    let nonlinear = DaeSource::new(
        vec![p(vec![tr(1., vec![y(0), y(0)])])],
        vec![p(vec![tr(1., vec![y(1)])])],
    )
    .unwrap();
    let algebraic = DaeSource::new(
        vec![p(vec![tr(1., vec![y(1)])])],
        vec![p(vec![tr(1., vec![y(1)])])],
    )
    .unwrap();
    let highclock = DaeSource::new(
        vec![p(vec![tr(1., vec![t(), t(), t()])])],
        vec![p(vec![tr(1., vec![y(1)])])],
    )
    .unwrap();
    for (case, src) in [
        ("nonlinear", nonlinear),
        ("algebraic", algebraic),
        ("high_clock", highclock),
    ] {
        let r = de::source_picard_endpoint(
            &src,
            [0., 0.125],
            &[vec![iv(0.)], vec![iv(0.)]],
            0.,
            R::Adaptive7,
        );
        assert!(r.is_err());
        println!(
            "PICARD_REFUSAL {}",
            serde_json::json!({"case":case,"error":r.err().unwrap().to_string()})
        );
    }
}

fn vector_projection_fixture(a: f64, b: f64) -> DaeSource {
    let lin = |k, w| tr(w, vec![F::component(k, 0.)]);
    let cross = |k, w| tr(w, vec![F::clock(0.), F::component(k, 0.)]);
    DaeSource::new(
        vec![p(vec![tr(1., vec![]), lin(1, 0.125), lin(2, -0.0625)])],
        vec![
            p(vec![
                lin(1, 1.),
                lin(0, -1.),
                cross(2, a),
                cross(0, -2. * a),
            ]),
            p(vec![lin(2, 1.), lin(0, -2.), cross(1, b), cross(0, -b)]),
        ],
    )
    .unwrap()
}
#[test]
fn vector_projection_actual_callbacks_noalloc() {
    warm();
    let src = vector_projection_fixture(2., -1.);
    let branch = [[-1., 1.], [-1., 1.]];
    let candidate = initialize_reduced_problem(
        &src,
        0.,
        &[0.],
        &[0., 0.],
        &branch,
        policy(),
        EvolutionRoute::BoundedVector5,
    )
    .unwrap();
    let old = initialize_reduced_problem(
        &src,
        0.,
        &[0.],
        &[0., 0.],
        &branch,
        policy(),
        EvolutionRoute::Reduced5,
    )
    .unwrap();
    let mut values = [0.; 3];
    let mut alloc = [Counts::default(); 3];
    let t = 0.0625;
    let x = [0.0625];
    let vigilode::Jacobian::Dense(j) = candidate.problem.jacobian_provider() else {
        panic!("dense")
    };
    let (r, a) = measure(|| (candidate.problem.rhs_fn())(t, &x, &mut values[0..1]));
    r.unwrap();
    alloc[0] = a;
    let (r, a) = measure(|| j(t, &x, &mut values[1..2]));
    r.unwrap();
    alloc[1] = a;
    let (r, a) =
        measure(|| candidate.problem.time_derivative_fn().unwrap()(t, &x, &mut values[2..3]));
    r.unwrap();
    alloc[2] = a;
    let mut refv = [0.; 3];
    let vigilode::Jacobian::Dense(j) = old.problem.jacobian_provider() else {
        panic!("dense")
    };
    (old.problem.rhs_fn())(t, &x, &mut refv[0..1]).unwrap();
    j(t, &x, &mut refv[1..2]).unwrap();
    old.problem.time_derivative_fn().unwrap()(t, &x, &mut refv[2..3]).unwrap();
    for (v, r) in values.into_iter().zip(refv) {
        assert!((v - r).abs() <= 1e-12);
    }
    println!(
        "VECTOR_RED_GREEN {}",
        serde_json::json!({"values":values,"reference":refv,"allocations":alloc})
    );
    assert_eq!(
        alloc.map(|a| a.requests()),
        [0; 3],
        "actual vector callback allocations"
    );
}

#[test]
fn bounded_vector_portfolio() {
    use de::ReconstructionSeedRoute as S;
    warm();
    let branch = [[-1., 1.], [-1., 1.]];
    let epoch = EvolutionEpoch {
        start: 0.,
        end: 0.125,
        incoming_x_error: Some(0.),
    };
    for (case, a, b) in [
        ("weak", 0.5, 0.25),
        ("moderate", 1., 0.5),
        ("strong", 2., -1.),
    ] {
        let src = vector_projection_fixture(a, b);
        let baseline = evolve_epoch(
            &src,
            &[0.],
            &[0., 0.],
            &branch,
            epoch,
            &opts(),
            policy(),
            EvolutionRoute::Reduced5,
            &[],
        )
        .unwrap();
        assert!(baseline.global_qualified, "{case} {:?}", baseline.reason);
        for solver in [
            EvolutionRoute::Reduced5,
            EvolutionRoute::Prepared5,
            EvolutionRoute::BoundedVector5,
            EvolutionRoute::BoundedVectorRefined5,
        ] {
            let ((r, receipt), allocation) = measure(|| {
                evolve_epoch_audited(
                    &src,
                    &[0.],
                    &[0., 0.],
                    &branch,
                    epoch,
                    &opts(),
                    policy(),
                    solver,
                    &[],
                )
            });
            let r = r.unwrap();
            assert!(r.global_qualified, "{case} {solver:?} {:?}", r.reason);
            println!(
                "VECTOR_EPOCH {}",
                serde_json::json!({"case":case,"solver":solver,"allocation":allocation,"slabs":r.slabs().len(),"qualified":r.global_qualified,"max_physical_error":r.max_physical_error,"audit":receipt})
            );
        }
        let mut scratch = VectorProjectionWorkspace::new();
        let mut recon = de::VectorReconstructionWorkspace::new();
        let slabs = baseline.slabs();
        for (position, k) in [
            ("first", 0),
            ("middle", slabs.len() / 2),
            ("last", slabs.len() - 1),
        ] {
            let s = &slabs[k];
            let c = s.certificate.as_ref().unwrap();
            let x = &s.native.y0;
            let t = s.times[0];
            for (route, refined) in [("legacy", false), ("bounded", false), ("refined", true)] {
                let (r, receipt, alloc) = if route == "legacy" {
                    let ((r, w), a) = measure(|| {
                        src.initialize_audited(
                            t,
                            x,
                            &[0., 0.],
                            &branch,
                            policy().init,
                            InitRoute::PreparedBatch,
                        )
                    });
                    (r.unwrap(), w, a)
                } else {
                    let ((r, w), a) = measure(|| {
                        src.initialize_bounded_vector_audited(
                            t,
                            x,
                            &[0., 0.],
                            &branch,
                            policy().init,
                            refined,
                            &mut scratch,
                        )
                    });
                    assert_eq!(a.requests(), 0);
                    (r.unwrap().to_owned_report(), w, a)
                };
                assert!(r.qualified);
                for (i, z) in r.root_box.iter().enumerate() {
                    let exact = x[0] * (i + 1) as f64;
                    assert!(z[0] <= exact && exact <= z[1]);
                }
                assert!((r.xdot[0] - 1.).abs() <= 1e-12);
                assert!((r.reduced_jacobian[0][0]).abs() <= 1e-12);
                println!(
                    "VECTOR_POINT {}",
                    serde_json::json!({"case":case,"position":position,"route":route,"x":x,"time":t,"allocation":alloc,"report":r,"audit":receipt})
                );
            }
            let xp = de::dense_power(&s.native.y0, &s.native.y1, &s.native.stages, s.native.basis)
                .unwrap();
            let mut out = [Iv::point(123.).unwrap(); 10];
            let mut work: [InitWork; 5] = std::array::from_fn(|_| InitWork::default());
            for (candidate, refined, seed) in [
                ("legacy", false, S::Fixed),
                ("bounded_fixed", false, S::Fixed),
                ("bounded_previous", false, S::PreviousNode),
                ("refined_fixed", true, S::Fixed),
                ("refined_previous", true, S::PreviousNode),
            ] {
                let (z, receipt, allocation) = if candidate == "legacy" {
                    let ((r, w), a) = measure(|| {
                        de::reconstruct_audited(
                            &src,
                            s.times,
                            &xp,
                            &[0., 0.],
                            &branch,
                            policy().init,
                            InitRoute::PreparedBatch,
                            None,
                            5,
                        )
                    });
                    let (z, _) = r.unwrap();
                    (z, w, a)
                } else {
                    let mut w = ProjectionAttemptWork::default();
                    let (r, a) = measure(|| {
                        de::reconstruct_bounded_vector_into(
                            &src,
                            s.times,
                            &xp,
                            &[0., 0.],
                            &branch,
                            policy().init,
                            refined,
                            seed,
                            &mut out,
                            &mut work,
                            &mut w,
                            &mut recon,
                        )
                    });
                    r.unwrap();
                    assert_eq!(a.requests(), 0);
                    assert_eq!((w.calls.started, w.calls.returned), (5, 5));
                    (out.chunks_exact(5).map(|v| v.to_vec()).collect(), w, a)
                };
                let mut full = xp.clone();
                full.extend(z);
                let r = de::certify_slab(
                    &src,
                    s.times,
                    &full,
                    &branch,
                    policy().tube_radius,
                    Some(c.incoming),
                    policy().green,
                    policy().init.condition_cap,
                )
                .unwrap();
                assert!(
                    r.global_qualified
                        && r.whole_x_error.unwrap() <= policy().global_tol
                        && r.whole_z_error.unwrap() <= policy().global_tol
                );
                println!(
                    "VECTOR_RECON {}",
                    serde_json::json!({"case":case,"position":position,"candidate":candidate,"allocation":allocation,"audit":receipt,"qualified":r.global_qualified,"whole_x":r.whole_x_error,"whole_z":r.whole_z_error,"original_g":r.original_g_upper})
                );
            }
        }
    }
    // Nonlinear perturbation with the same unique physical graph.
    let lin = |k, w| tr(w, vec![F::component(k, 0.)]);
    let delta = F {
        constant: 0.,
        time: 0.,
        state: vec![(1, 1.), (0, -1.)],
    };
    let src = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![
            p(vec![
                lin(1, 1.),
                lin(0, -1.),
                tr(0.03125, vec![delta.clone(), delta]),
            ]),
            p(vec![lin(2, 1.), lin(0, -2.)]),
        ],
    )
    .unwrap();
    for refined in [false, true] {
        let (r, w) = src.initialize_bounded_vector_audited(
            0.125,
            &[0.125],
            &[0., 0.],
            &branch,
            policy().init,
            refined,
            &mut scratch_for_vector(),
        );
        let r = r.unwrap().to_owned_report();
        assert!(r.root_box[0][0] <= 0.125 && 0.125 <= r.root_box[0][1]);
        println!(
            "VECTOR_NONLINEAR {}",
            serde_json::json!({"refined":refined,"report":r,"audit":w})
        );
    }
    let singular = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![
            p(vec![lin(1, 1.), lin(2, 1.)]),
            p(vec![lin(1, 1.), lin(2, 1.)]),
        ],
    )
    .unwrap();
    let (r, w) = singular.initialize_bounded_vector_audited(
        0.,
        &[0.],
        &[0., 0.],
        &branch,
        policy().init,
        false,
        &mut scratch_for_vector(),
    );
    assert!(r.is_err());
    println!(
        "VECTOR_REFUSAL {}",
        serde_json::json!({"error":r.err().unwrap().to_string(),"audit":w})
    );
}
fn scratch_for_vector() -> VectorProjectionWorkspace {
    VectorProjectionWorkspace::new()
}

#[test]
fn source_linear_ls_portfolio() {
    use de::LinearLsEndpointRoute as R;
    warm();
    let iv = |x| Iv::point(x).unwrap();
    let source = |rate: f64| {
        DaeSource::new(
            vec![p(vec![tr(rate, vec![F::component(0, 0.)])])],
            vec![p(vec![
                tr(1., vec![F::component(1, 0.)]),
                tr(-1., vec![F::component(0, 0.)]),
            ])],
        )
        .unwrap()
    };
    let routes = [
        R::NormalEquations,
        R::BoundedNormal,
        R::QrProposal,
        R::RationalResponse,
    ];
    for rate in [0., -0.125, -1., -4., -16., -64.] {
        let src = source(rate);
        let power = vec![vec![iv(1.)], vec![iv(1.)]];
        for order in [1, 3, 6] {
            for route in routes {
                let (v, allocation) = measure(|| {
                    de::source_linear_ls_endpoint(&src, [0., 1.], &power, 0., order, route)
                });
                let v = v.unwrap();
                if matches!(route, R::BoundedNormal | R::RationalResponse) {
                    assert_eq!(allocation.requests(), 0);
                }
                assert!((rate.exp() - 1.).abs() <= v.candidate_error);
                println!(
                    "LS_BOUND {}",
                    serde_json::json!({"case":"manufactured","rate":rate,"order":order,"route":route,"allocation":allocation,"bounds":v,"qualified":true})
                );
            }
        }
    }
    for rate in [-1., -8.] {
        let src = source(rate);
        let branch = [[-2., 2.]];
        for solver in [
            EvolutionRoute::Reduced5,
            EvolutionRoute::Reduced5NormalLs,
            EvolutionRoute::Reduced5BoundedLs,
        ] {
            let ((r, a), alloc) = measure(|| {
                evolve_epoch_audited(
                    &src,
                    &[1.],
                    &[1.],
                    &branch,
                    EvolutionEpoch {
                        start: 0.,
                        end: 0.125,
                        incoming_x_error: Some(0.),
                    },
                    &opts(),
                    policy(),
                    solver,
                    &[],
                )
            });
            let r = r.unwrap();
            assert!(r.global_qualified, "{rate} {solver:?} {:?}", r.reason);
            let mut receipts = 0;
            let mut tighter = 0;
            for s in r.slabs() {
                let c = s.certificate.as_ref().unwrap();
                if let Some(v) = c.linear_ls_endpoint {
                    receipts += 1;
                    let b = de::certify_slab(
                        &src,
                        s.times,
                        &power(s),
                        &branch,
                        policy().tube_radius,
                        Some(c.incoming),
                        policy().green,
                        policy().init.condition_cap,
                    )
                    .unwrap();
                    assert_eq!(c.whole_x_error, b.whole_x_error);
                    assert!(c.outgoing.unwrap() <= b.outgoing.unwrap());
                    tighter += usize::from(c.outgoing.unwrap() < b.outgoing.unwrap());
                    println!(
                        "LS_NATIVE {}",
                        serde_json::json!({"rate":rate,"solver":solver,"times":s.times,"power":s.power,"incoming":c.incoming,"bounds":v,"outgoing":c.outgoing,"base_outgoing":b.outgoing})
                    );
                }
            }
            if solver != EvolutionRoute::Reduced5 {
                assert_eq!(receipts, r.slabs().len());
            }
            println!(
                "LS_EPOCH {}",
                serde_json::json!({"rate":rate,"solver":solver,"allocation":alloc,"qualified":r.global_qualified,"slabs":r.slabs().len(),"receipts":receipts,"tighter":tighter,"max_physical_error":r.max_physical_error,"audit":a})
            );
        }
    }
    let r = de::source_linear_ls_endpoint(
        &source(1.),
        [0., 1.],
        &[vec![iv(1.)], vec![iv(1.)]],
        0.,
        3,
        R::BoundedNormal,
    );
    assert!(r.is_err());
}

fn nonlinear_scalar_fixture() -> DaeSource {
    let x = || F::component(0, 0.);
    let z = || F::component(1, 0.);
    DaeSource::new(
        vec![p(vec![tr(0.125, vec![]), tr(0.125, vec![z()])])],
        vec![p(vec![
            tr(1., vec![z(), z()]),
            tr(-1., vec![x()]),
            tr(-0.25, vec![]),
        ])],
    )
    .unwrap()
}
#[test]
fn bounded_sturm_actual_callbacks_noalloc() {
    warm();
    let src = nonlinear_scalar_fixture();
    let branch = [[0.25, 1.25]];
    let candidate = initialize_reduced_problem(
        &src,
        0.,
        &[0.],
        &[0.5],
        &branch,
        policy(),
        EvolutionRoute::BoundedSturm5,
    )
    .unwrap();
    let old = initialize_reduced_problem(
        &src,
        0.,
        &[0.],
        &[0.5],
        &branch,
        policy(),
        EvolutionRoute::Scalar5,
    )
    .unwrap();
    let t = 0.0625;
    let x = [0.0625];
    let mut values = [0.; 3];
    let mut alloc = [Counts::default(); 3];
    let mut expected = [0.; 3];
    let vigilode::Jacobian::Dense(j) = candidate.problem.jacobian_provider() else {
        panic!("dense")
    };
    let (r, a) = measure(|| (candidate.problem.rhs_fn())(t, &x, &mut values[..1]));
    r.unwrap();
    alloc[0] = a;
    let (r, a) = measure(|| j(t, &x, &mut values[1..2]));
    r.unwrap();
    alloc[1] = a;
    let (r, a) =
        measure(|| candidate.problem.time_derivative_fn().unwrap()(t, &x, &mut values[2..]));
    r.unwrap();
    alloc[2] = a;
    let vigilode::Jacobian::Dense(j) = old.problem.jacobian_provider() else {
        panic!("dense")
    };
    (old.problem.rhs_fn())(t, &x, &mut expected[..1]).unwrap();
    j(t, &x, &mut expected[1..2]).unwrap();
    old.problem.time_derivative_fn().unwrap()(t, &x, &mut expected[2..]).unwrap();
    for (v, e) in values.into_iter().zip(expected) {
        assert!((v - e).abs() <= 1e-12);
    }
    println!(
        "STURM_RED_GREEN {}",
        serde_json::json!({"values":values,"reference":expected,"allocation":alloc})
    );
    assert_eq!(
        alloc.map(|a| a.requests()),
        [0; 3],
        "actual nonlinear root-count callback allocations"
    );
}
