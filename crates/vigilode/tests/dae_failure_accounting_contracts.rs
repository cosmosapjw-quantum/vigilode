#![cfg(feature = "polynomial-events")]
use rodas5p_core::directed::Interval as Iv;
use rodas5p_integrators::retro_dae_evolution as de;
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
fn rp(sync: VectorSyncRoute) -> VectorResetPolicy {
    VectorResetPolicy {
        sync,
        derivative: GuardDerivativeRoute::Schur,
        branch_route: InitRoute::Reuse,
        subdivisions: 1,
    }
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
        EvolutionRoute::Affine5 => InitRoute::AffineExact,
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

fn power(s: &SlabRecord) -> Vec<Vec<Iv>> {
    s.power
        .iter()
        .map(|row| row.iter().map(|a| Iv::new(a[0], a[1]).unwrap()).collect())
        .collect()
}
#[test]
fn failed_initialization_keeps_work() {
    let model = RobertsonModel::new(3e7).unwrap();
    let mut policy = policy().init;
    policy.condition_cap = 0.5;
    let (r, w) = model.source().initialize_audited(
        0.,
        &[1., 0.],
        &[0.125],
        &[[-0.25, 1.25]],
        policy,
        InitRoute::Reuse,
    );
    assert!(r.is_err());
    assert!(w.admitted);
    assert!(w.evaluations.returned > 0);
    assert!(w.linear.returned > 0);
    assert!(w.rhs_returned > 0);
    println!(
        "FAILED_INIT {}",
        serde_json::json!({"work":w,"error":r.unwrap_err().to_string()})
    );
}

fn linear_source() -> DaeSource {
    DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![p(vec![
            tr(1., vec![F::component(1, 0.)]),
            tr(-1., vec![F::component(0, 0.)]),
        ])],
    )
    .unwrap()
}
fn validate_projection(w: &ProjectionAttemptWork) {
    for p in [w.calls, w.evaluations, w.linear, w.frozen, w.roots] {
        assert!(p.returned <= p.started);
    }
    assert!(w.admitted <= w.calls.started && w.rhs_returned <= w.rhs_requested);
    assert_eq!(
        w.rhs_requested,
        w.successful_rhs_requested + w.refused_rhs_requested
    );
}
fn validate_native(n: &NativeResourceAudit) {
    for w in [
        &n.initialization,
        &n.point_projection,
        &n.scan_nodes,
        &n.replay_nodes,
    ] {
        validate_projection(w);
    }
    for p in [
        n.scan_reconstruction,
        n.replay_reconstruction,
        n.scan_certificate,
        n.replay_certificate,
        n.guard_build,
        n.warp_setup,
    ] {
        assert!(p.returned <= p.started);
    }
    assert!(
        n.scan_global_certificates <= n.scan_certificate.returned
            && n.replay_global_certificates <= n.replay_certificate.returned
    );
    if let Some(s) = &n.stats {
        assert!(n.driver_invoked);
        assert_eq!(s.attempts, s.accepted_steps + s.rejected_steps);
        assert_eq!(n.measured_attempts(), Some(s.attempts));
    } else if !n.driver_invoked {
        assert_eq!(n.measured_attempts(), Some(0));
    } else {
        assert_eq!(n.measured_attempts(), None);
    }
}
#[test]
fn failed_reconstruction_retains_successful_nodes() {
    let src = linear_source();
    let x = vec![vec![Iv::point(0.).unwrap(), Iv::point(2.).unwrap()]];
    let (r, w) = de::reconstruct_audited(
        &src,
        [0., 1.],
        &x,
        &[0.],
        &[[-0.25, 0.75]],
        policy().init,
        InitRoute::Reuse,
        None,
        5,
    );
    assert!(r.is_err());
    validate_projection(&w);
    assert!(w.calls.returned > 0);
    assert_eq!(w.calls.started, w.calls.returned + 1);
    assert!(w.successful_rhs_requested > 0);
    assert!(w.refused_rhs_requested > 0);
    println!(
        "RECON_REFUSAL {}",
        serde_json::json!({"work":w,"error":r.unwrap_err().to_string()})
    );
}
#[test]
fn setup_driver_and_callback_refusal_boundaries() {
    let src = linear_source();
    let ep = EvolutionEpoch {
        start: 0.,
        end: 0.2,
        incoming_x_error: Some(0.),
    };
    for case in [
        "bad_epoch",
        "condition",
        "warp",
        "bad_driver_options",
        "callback",
    ] {
        let mut p = policy();
        let mut o = opts();
        let mut e = ep;
        let mut route = EvolutionRoute::Reduced5;
        let branch = if case == "callback" {
            [[-0.01, 0.01]]
        } else {
            [[-1., 1.]]
        };
        match case {
            "bad_epoch" => e.end = 0.,
            "condition" => p.init.condition_cap = 0.5,
            "warp" => route = EvolutionRoute::WarpFolded,
            "bad_driver_options" => o.rtol = -1.,
            _ => {}
        }
        let (r, a) = evolve_epoch_audited(&src, &[0.], &[0.], &branch, e, &o, p, route, &[]);
        validate_native(&a);
        match case {
            "bad_epoch" => {
                assert!(r.is_err());
                assert_eq!(a.initialization.calls.started, 0);
                assert!(!a.driver_invoked);
            }
            "condition" => {
                assert!(r.is_err());
                assert_eq!(a.initialization.calls.started, 1);
                assert_eq!(a.initialization.calls.returned, 0);
                assert!(a.initialization.refused_rhs_requested > 0);
                assert!(!a.driver_invoked);
            }
            "warp" => {
                assert!(r.is_err());
                assert_eq!(a.initialization.calls.returned, 1);
                assert_eq!(a.warp_setup.started, 1);
                assert_eq!(a.warp_setup.returned, 0);
                assert!(!a.driver_invoked);
            }
            "bad_driver_options" => {
                assert!(r.is_err());
                assert!(a.driver_invoked);
                assert!(a.stats.is_none());
                assert_eq!(a.measured_attempts(), None);
            }
            "callback" => {
                assert!(a.driver_invoked);
                assert!(a.point_projection.calls.started > a.point_projection.calls.returned);
                assert!(a.point_projection.refused_rhs_requested > 0);
                if let Ok(r) = &r {
                    assert!(!r.driver().is_success());
                    assert_eq!(
                        a.stats.as_ref().unwrap().attempts,
                        r.driver().stats.attempts
                    );
                }
            }
            _ => unreachable!(),
        }
        let outcome = match &r {
            Ok(v) => format!("{:?}", v.driver().status),
            Err(e) => e.to_string(),
        };
        println!(
            "EPOCH_REFUSAL {}",
            serde_json::json!({"case":case,"outcome":outcome,"work":a,"measured":a.measured_attempts()})
        );
    }
}
fn validate_atlas(r: &PhaseAtlasRun, a: &AtlasResourceAudit) {
    assert!(a.admitted);
    let w = &r.work().phase;
    assert_eq!(
        w.charged_native_attempts,
        a.native
            .iter()
            .map(|v| v.charged_native_attempts)
            .sum::<usize>()
    );
    assert_eq!(
        w.charged_scan_slabs,
        a.native.iter().map(|v| v.charged_scan_slabs).sum::<usize>()
    );
    assert_eq!(w.reset_checks, a.reset_guards.started);
    assert_eq!(w.committed, a.committed);
    assert_eq!(w.cache_hits, a.cache_hits);
    assert!(a.negative_reset_guards <= a.reset_guards.returned);
    let mut idx = 0;
    for n in &a.native {
        validate_native(&n.native);
        assert_eq!(n.measured_attempts, n.native.measured_attempts());
        if n.returned {
            let p = &r.phases()[idx];
            idx += 1;
            let mut it = p.ordered().native_stats();
            let s = it.next().unwrap();
            assert!(it.next().is_none());
            assert_eq!(
                serde_json::to_value(s).unwrap(),
                serde_json::to_value(n.native.stats.as_ref().unwrap()).unwrap()
            );
            assert_eq!(n.charged_native_attempts, s.attempts);
            assert_eq!(n.charged_scan_slabs, s.accepted_steps);
            assert_eq!(
                n.native.point_projection.successful_rhs_requested
                    + n.native.replay_nodes.successful_rhs_requested,
                p.ordered().work.replay_projection_rhs
            );
            // On a fully returned reconstruction, all node reports contribute to
            // the legacy scan count. A failed reconstruction can discard a prefix.
            assert!(
                n.native.scan_nodes.successful_rhs_requested
                    >= p.ordered().work.shared_projection.rhs_count
            );
        } else {
            assert_eq!(n.charged_native_attempts, n.native_quota);
            assert_eq!(n.charged_scan_slabs, n.slab_quota);
        }
    }
    assert_eq!(idx, r.phases().len());
}
#[test]
fn native_stats_without_event_authority() {
    let m = graph(
        "overlap",
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
    );
    let (r, a) = evolve_robertson_to_first_guard_audited(
        m[0].rate,
        initial(2f64.powi(-26)),
        [0., 0.002],
        &opts(),
        policy(),
        m[0].route,
        m[0].certificate,
        &m[0].guards,
        pm(MultiGuardRoute::Shared),
    );
    let r = r.unwrap();
    assert!(r.event_authority().is_none());
    assert!(r.winner().is_none());
    validate_native(&a);
    assert_eq!(r.native_stats().count(), 1);
    assert_eq!(a.measured_attempts(), Some(r.work.attempts));
    assert!(r.work.attempts > 0);
    println!(
        "NO_WINNER {}",
        serde_json::json!({"work":r.work,"native":a})
    );
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

fn all_training_impl(audited: bool) {
    let m = graph(
        "alternating",
        EvolutionRoute::Affine5,
        ConservationRoute::Integral4,
    );
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
    use serde_json::json;
    for case in ["alternating", "distinct", "overlap"] {
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Prepared5,
            EvolutionRoute::Scalar5,
            EvolutionRoute::Affine5,
        ] {
            for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
                for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                    for method in ["JointCentered", "JointGreen", "AtlasFresh", "AtlasReuse"] {
                        if audited && !method.starts_with("Atlas") {
                            continue;
                        }
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
                            let ((r, audit), run_alloc) = measure(|| {
                                if audited {
                                    evolve_robertson_phase_atlas_audited(
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
                                } else {
                                    (
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
                                        ),
                                        AtlasResourceAudit::default(),
                                    )
                                }
                            });
                            let r = r.unwrap();
                            if audited {
                                validate_atlas(&r, &audit);
                                println!(
                                    "ATLAS_AUDIT {}",
                                    json!({"case":case,"solver":solver,"cert":cert,"order":order,"method":method,"audit":audit})
                                );
                            }
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

#[test]
#[ignore = "fixed192 physical replay"]
fn training() {
    all_training_impl(false);
}
#[test]
#[ignore = "fixed96 audited Atlas candidates"]
fn audited_training() {
    all_training_impl(true);
}

#[test]
#[ignore = "bounded refusal allocation and debit observations"]
fn audited_refusals() {
    use serde_json::json;
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
        let ((r, audit), a) = measure(|| {
            evolve_robertson_phase_atlas_audited(
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
                validate_atlas(&r, &audit);
                if case == "warp" {
                    assert_eq!(audit.native.len(), 1);
                    assert_eq!(audit.native[0].measured_attempts, Some(0));
                    assert!(!audit.native[0].native.driver_invoked);
                    assert_eq!(audit.native[0].native.initialization.calls.returned, 1);
                }

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
                assert!(!audit.admitted && audit.native.is_empty());
                json!({"case":case,"stop":"AdmissionErr","reason":e.to_string(),"work":null,"alloc":a})
            }
        };
        println!("REFUSAL {}", row);
        println!("REFUSAL_AUDIT {}", json!({"case":case,"audit":audit}));
    }
}

fn h(q: f64) -> P {
    p(vec![
        tr(1., vec![F::clock(q)]),
        tr(1. / 128., vec![F::component(2, 0.)]),
    ])
}

fn reset() -> SimplexReset {
    SimplexReset::new([[1., 0., 0.], [0., 0.5, 0.], [0., 0.5, 1.]]).unwrap()
}
