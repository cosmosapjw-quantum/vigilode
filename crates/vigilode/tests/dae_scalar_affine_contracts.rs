#![cfg(feature = "polynomial-events")]
use rodas5p_core::directed::Interval as Iv;
use rodas5p_integrators::{
    retro_dae_evolution as de,
    retro_event_poly::{RepresentedPolynomial, RootRoute},
};
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
fn linear_isolation_allocation_budget() {
    let p = RepresentedPolynomial::from_power(&[-1., 3.]).unwrap();
    // Warm the original dictionary before measuring a real root call.
    let _ = p
        .isolate(RootRoute::DictionaryCompanion, 2f64.powi(-48))
        .unwrap();
    let (r, c) = measure(|| p.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-48)));
    let r = r.unwrap();
    assert_eq!(r.roots.len(), 1);
    assert_eq!(r.roots[0].multiplicity, 1);
    assert!(r.roots[0].theta[0] <= 1. / 3. && 1. / 3. <= r.roots[0].theta[1]);
    assert_eq!(r.work.companion_attempts, 0);
    assert_eq!(r.work.dictionary_uses, 1);
    println!(
        "ROOT_RED_GREEN {}",
        serde_json::json!({"allocation":c,"report":r})
    );
    assert!(
        c.requests() < 1000,
        "bounded linear exact root path uses {} allocation requests",
        c.requests()
    );
}

#[test]
fn genuine_frozen_root_cost() {
    let model = RobertsonModel::new(3e7).unwrap();
    let src = model.source();
    let x = [0.9998658289383162, 6.457305003072096e-5];
    let g = p(vec![
        tr(1., vec![F::component(0, 0.)]),
        tr(1., vec![F::component(1, 0.)]),
        tr(1., vec![F::component(2, 0.)]),
        tr(-1., vec![]),
    ]);
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(src).unwrap()["g"][0]
    );
    let _ = RepresentedPolynomial::frozen_scalar(&g, 0.0005, &x, [-0.25, 1.25])
        .unwrap()
        .isolate(RootRoute::DictionaryCompanion, 2f64.powi(-48))
        .unwrap();
    let (p0, a) = measure(|| RepresentedPolynomial::frozen_scalar(&g, 0.0005, &x, [-0.25, 1.25]));
    let p0 = p0.unwrap();
    let (r, b) = measure(|| p0.isolate(RootRoute::DictionaryCompanion, 2f64.powi(-48)));
    let r = r.unwrap();
    assert_eq!(r.roots.len(), 1);
    let (p1, c) = measure(|| {
        de::project(
            src,
            0.0005,
            &x,
            &[0.125],
            &[[-0.25, 1.25]],
            policy().init,
            InitRoute::ScalarRoot,
            None,
        )
    });
    let p1 = p1.unwrap();
    println!(
        "COST_RED_GREEN {}",
        serde_json::json!({"coefficients":p0.exact_coefficients(),"clock":p0.exact_clock(),"composition":a,"isolate":b,"project":c,"root":r,"init":p1})
    );
    assert!(
        b.requests() < 1000,
        "genuine frozen isolation requests {}",
        b.requests()
    );
    assert!(
        c.requests() < 1400,
        "genuine scalar project requests {}",
        c.requests()
    );
}

const ROOT_ROUTES: [RootRoute; 5] = [
    RootRoute::Sturm,
    RootRoute::NativeBernstein,
    RootRoute::DictionaryBernstein,
    RootRoute::Companion,
    RootRoute::DictionaryCompanion,
];
#[test]
fn affine_singleton_and_branch_boundary() {
    use rodas5p_integrators::retro_event_poly::RootKind;
    let tol = 2f64.powi(-48);
    for (a, b, expected) in [
        (0., 1., None),
        (-1., 1., Some(1.)),
        (-1., 2., Some(0.5)),
        (1., -2., Some(0.5)),
        (-2., 1., None),
        (1., 1., None),
    ] {
        let p = RepresentedPolynomial::from_power(&[a, b]).unwrap();
        let r = p.isolate_affine_exact(tol).unwrap();
        assert_eq!(r.is_some(), expected.is_some());
        if let (Some(v), Some(x)) = (r, expected) {
            assert_eq!(v.theta, [x, x]);
            assert_eq!(v.multiplicity, 1);
            assert_eq!(v.kind, if b > 0. { RootKind::Up } else { RootKind::Down });
        }
    }
    let p = RepresentedPolynomial::from_power(&[-1., 3.]).unwrap();
    let v = p.isolate_affine_exact(tol).unwrap().unwrap();
    assert_eq!(v.exact_theta, ["1/3", "1/3"]);
    assert!(v.theta[0] < v.theta[1]);
    assert!(
        RepresentedPolynomial::from_power(&[1., 0., 1.])
            .unwrap()
            .isolate_affine_exact(tol)
            .is_err()
    );
    assert!(
        RepresentedPolynomial::from_power(&[0.])
            .unwrap()
            .isolate_affine_exact(tol)
            .is_err()
    );
    assert!(p.isolate_affine_exact(0.).is_err());
    let model = RobertsonModel::new(3e7).unwrap();
    let src = model.source();
    for branch in [[0., 1.], [-1., 0.]] {
        // Exact z=0 at a branch endpoint must refuse, even though the
        // normalized half-open root API admits the right endpoint.
        assert!(
            src.initialize(
                0.,
                &[1., 0.],
                &[0.],
                &[branch],
                policy().init,
                InitRoute::ScalarRoot
            )
            .is_err()
        );
    }
}
#[test]
#[ignore = "actual native frozen scalar root and projection portfolio"]
fn scalar_portfolio() {
    use serde_json::json;
    let g = p(vec![
        tr(1., vec![F::component(0, 0.)]),
        tr(1., vec![F::component(1, 0.)]),
        tr(1., vec![F::component(2, 0.)]),
        tr(-1., vec![]),
    ]);
    let tol = 2f64.powi(-48);
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
                assert_eq!(
                    serde_json::to_value(&g).unwrap(),
                    serde_json::to_value(src).unwrap()["g"][0]
                );
                let slabs = phase.original_slab_diagnostics();
                for (tag, k) in [
                    ("first", 0),
                    ("middle", slabs.len() / 2),
                    ("last", slabs.len() - 1),
                ] {
                    let s = &slabs[k];
                    assert!(s.certificate.as_ref().unwrap().global_qualified);
                    let t = s.times[0];
                    let x = &s.native.y0[..2];
                    let detail = json!({"solver":solver,"cert":cert,"mode":phase.mode,"position":tag,"slab":k,"time":t,"x":x,"rate":m[phase.mode].rate});
                    let (p0, a) =
                        measure(|| RepresentedPolynomial::frozen_scalar(&g, t, x, [-0.25, 1.25]));
                    let p0 = p0.unwrap();
                    assert_eq!(p0.degree(), Some(1));
                    println!(
                        "FROZEN {}",
                        json!({"detail":detail,"coefficients":p0.exact_coefficients(),"clock":p0.exact_clock(),"allocation":a})
                    );
                    for route in ROOT_ROUTES {
                        let (v, a) = measure(|| p0.isolate(route, tol));
                        let v = v.unwrap();
                        assert_eq!(v.roots.len(), 1);
                        assert_eq!(v.roots[0].multiplicity, 1);
                        assert_eq!(v.work.companion_attempts, 0);
                        println!(
                            "ROOT {}",
                            json!({"detail":detail,"method":route,"report":v,"allocation":a})
                        );
                    }
                    let (v, a) = measure(|| p0.isolate_affine_exact(tol));
                    let v = v.unwrap().unwrap();
                    println!(
                        "AFFINE {}",
                        json!({"detail":detail,"root":v,"allocation":a})
                    );
                    for route in [
                        InitRoute::Reuse,
                        InitRoute::PreparedBatch,
                        InitRoute::ScalarRoot,
                    ] {
                        let (v, a) = measure(|| {
                            de::project(
                                src,
                                t,
                                x,
                                &[0.125],
                                &[[-0.25, 1.25]],
                                policy().init,
                                route,
                                None,
                            )
                        });
                        let v = v.unwrap();
                        assert!(v.qualified);
                        println!(
                            "INIT {}",
                            json!({"detail":detail,"route":route,"report":v,"allocation":a})
                        );
                    }
                }
            }
        }
    }
}
