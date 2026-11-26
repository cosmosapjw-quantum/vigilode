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
fn actual_projection_scratch_budget() {
    let model = RobertsonModel::new(3e7).unwrap();
    let (r, a) = measure(|| {
        de::project(
            model.source(),
            0.,
            &[1. - 1. / 8192., 1. / 8192.],
            &[0.125],
            &[[-0.25, 1.25]],
            policy().init,
            InitRoute::Reuse,
            None,
        )
    });
    let r = r.unwrap();
    assert!(r.qualified);
    assert_eq!(r.z, vec![0.]);
    assert_eq!(r.work.rhs_count, 6);
    println!(
        "PROJ_RED_GREEN {}",
        serde_json::json!({"report":r,"allocation":a})
    );
    assert!(
        a.requests() <= 145,
        "projection scratch requests {}",
        a.requests()
    );
}

#[test]
fn affine_authority_workspace_and_refusals() {
    use serde_json::json;
    let f = p(vec![
        tr(2., vec![F::component(1, 0.)]),
        tr(-1., vec![F::component(0, 0.)]),
    ]);
    // g=(1+t+x) z - x; at t=1/4,x=1/8, z=1/11.
    let a = F {
        constant: 1.,
        time: 1.,
        state: vec![(0, 0.5), (0, 0.5)],
    };
    let g = p(vec![
        tr(1., vec![a, F::component(1, 0.)]),
        tr(-1., vec![F::component(0, 0.)]),
    ]);
    let model = DaeSource::new(vec![f.clone()], vec![g.clone()]).unwrap();
    let mut ws = ProjectionWorkspace::new();
    for sign in [1., -1.] {
        let gg = if sign == 1. {
            g.clone()
        } else {
            p(vec![
                tr(
                    -1.,
                    vec![
                        F {
                            constant: 1.,
                            time: 1.,
                            state: vec![(0, 0.5), (0, 0.5)],
                        },
                        F::component(1, 0.),
                    ],
                ),
                tr(1., vec![F::component(0, 0.)]),
            ])
        };
        let src = DaeSource::new(vec![f.clone()], vec![gg]).unwrap();
        for route in [
            InitRoute::Reuse,
            InitRoute::PreparedBatch,
            InitRoute::ScalarRoot,
            InitRoute::AffineExact,
        ] {
            let a = src
                .initialize(
                    0.25,
                    &[0.125],
                    &[0.25],
                    &[[-0.25, 1.25]],
                    policy().init,
                    route,
                )
                .unwrap();
            let b = src
                .initialize_with_workspace(
                    0.25,
                    &[0.125],
                    &[0.25],
                    &[[-0.25, 1.25]],
                    policy().init,
                    route,
                    &mut ws,
                )
                .unwrap();
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&b).unwrap()
            );
            assert!(a.qualified);
            assert_eq!(a.rank, 1);
            if route == InitRoute::AffineExact {
                let r = a.affine_root.as_ref().unwrap();
                assert_eq!(r.exact_theta[0], r.exact_theta[1]);
                assert_eq!(r.multiplicity, 1);
                assert_eq!(a.branch_authority, "exact affine scalar root");
                assert_eq!(a.work.dictionary_uses, 0);
            } else {
                assert!(a.affine_root.is_none());
            }
            println!("GENERAL {}", json!({"sign":sign,"route":route,"report":a}));
        }
    }
    let mut bad_policy = policy().init;
    bad_policy.condition_cap = 0.5;
    assert!(
        model
            .initialize_with_workspace(
                0.25,
                &[0.125],
                &[0.25],
                &[[-0.25, 1.25]],
                bad_policy,
                InitRoute::AffineExact,
                &mut ws
            )
            .is_err()
    );
    bad_policy = policy().init;
    bad_policy.mode = InitMode::Strict;
    assert!(
        model
            .initialize_with_workspace(
                0.25,
                &[0.125],
                &[0.25],
                &[[-0.25, 1.25]],
                bad_policy,
                InitRoute::AffineExact,
                &mut ws
            )
            .is_err()
    );
    assert!(
        model
            .initialize_with_workspace(
                0.25,
                &[0.125],
                &[0.25],
                &[[0.25, 1.25]],
                policy().init,
                InitRoute::AffineExact,
                &mut ws
            )
            .is_err()
    );
    // Exact endpoint, zero and constant, nonlinear degree>1.
    let s = RobertsonModel::new(3e7).unwrap();
    for branch in [[0., 1.], [-1., 0.]] {
        assert!(
            s.source()
                .initialize(
                    0.,
                    &[1., 0.],
                    &[0.],
                    &[branch],
                    policy().init,
                    InitRoute::AffineExact
                )
                .is_err()
        );
    }
    let nonlinear = DaeSource::new(
        vec![f.clone()],
        vec![p(vec![
            tr(1., vec![F::component(1, 0.), F::component(1, 0.)]),
            tr(-0.25, vec![]),
        ])],
    )
    .unwrap();
    assert!(
        nonlinear
            .initialize(
                0.,
                &[0.125],
                &[0.5],
                &[[0., 1.]],
                policy().init,
                InitRoute::AffineExact
            )
            .is_err()
    );
    assert!(
        nonlinear
            .initialize(
                0.,
                &[0.125],
                &[0.5],
                &[[0., 1.]],
                policy().init,
                InitRoute::ScalarRoot
            )
            .is_ok()
    );
    for c in [0., 1.] {
        let s = DaeSource::new(vec![f.clone()], vec![p(vec![tr(c, vec![])])]).unwrap();
        assert!(
            s.initialize(
                0.,
                &[0.125],
                &[0.5],
                &[[0., 1.]],
                policy().init,
                InitRoute::AffineExact
            )
            .is_err()
        );
    }
    // Reuse the same scratch after failures at the maximal column capacity.
    let nx = 15;
    let nz = 17;
    let f = (0..nx)
        .map(|i| {
            p(vec![
                tr(0.25, vec![F::component(i, 0.)]),
                tr(0.5, vec![F::component(nx + i, 0.)]),
            ])
        })
        .collect();
    let g = (0..nz)
        .map(|i| {
            p(vec![
                tr(1., vec![F::component(nx + i, 0.)]),
                tr(-0.5, vec![F::component(i % nx, 0.)]),
            ])
        })
        .collect();
    let src = DaeSource::new(f, g).unwrap();
    let x = vec![0.25; nx];
    let seed = vec![0.; nz];
    let branch = vec![[-1., 1.]; nz];
    assert!(
        src.initialize_with_workspace(
            0.,
            &x,
            &seed,
            &branch,
            policy().init,
            InitRoute::AffineExact,
            &mut ws
        )
        .is_err()
    );
    for route in [InitRoute::Reuse, InitRoute::PreparedBatch] {
        let a = src
            .initialize(0., &x, &seed, &branch, policy().init, route)
            .unwrap();
        let b = src
            .initialize_with_workspace(0., &x, &seed, &branch, policy().init, route, &mut ws)
            .unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_eq!(a.z, vec![0.125; nz]);
        assert_eq!(a.rank, nz);
        assert!(a.qualified);
        println!(
            "MAX_SHAPE {}",
            json!({"route":route,"nx":nx,"nz":nz,"work":a.work,"size":std::mem::size_of::<ProjectionWorkspace>()})
        );
    }
    let a = model
        .initialize_with_workspace(
            0.25,
            &[0.125],
            &[0.25],
            &[[-0.25, 1.25]],
            policy().init,
            InitRoute::AffineExact,
            &mut ws,
        )
        .unwrap();
    assert!(a.qualified);
}

#[test]
#[ignore = "fixed actual native projection portfolio"]
fn projection_portfolio() {
    use serde_json::json;
    let mut ws = ProjectionWorkspace::new();
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
                    for route in [
                        InitRoute::Reuse,
                        InitRoute::PreparedBatch,
                        InitRoute::ScalarRoot,
                        InitRoute::AffineExact,
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
                        let (b, alloc) = measure(|| {
                            src.initialize_with_workspace(
                                t,
                                x,
                                &[0.125],
                                &[[-0.25, 1.25]],
                                policy().init,
                                route,
                                &mut ws,
                            )
                        });
                        let b = b.unwrap();
                        assert_eq!(
                            serde_json::to_string(&v).unwrap(),
                            serde_json::to_string(&b).unwrap()
                        );
                        println!(
                            "INIT {}",
                            json!({"detail":detail,"route":route,"report":v,"allocation":a,"reused_allocation":alloc})
                        );
                    }
                    // Actual original-source g_z at the certified root: scalar Jacobian.
                    use rodas5p_core::DenseMatrix;
                    use rodas5p_integrators::{
                        ParallelExecution,
                        retro_foundation::{BatchMode, PreparedRhs},
                    };
                    let v = src
                        .initialize(
                            t,
                            x,
                            &[0.125],
                            &[[-0.25, 1.25]],
                            policy().init,
                            InitRoute::AffineExact,
                        )
                        .unwrap();
                    let mut y = x.to_vec();
                    y.extend_from_slice(&v.z);
                    let j = src.jacobian_values(t, &y).unwrap();
                    let a = DenseMatrix::new(1, 1, vec![j[8]]).unwrap();
                    let rhs = vec![
                        vec![1.],
                        vec![-j[6]],
                        vec![-j[7]],
                        vec![-v.xdot[0] - v.xdot[1]],
                    ];
                    let (o, setup) = measure(|| PreparedRhs::new(a.clone()));
                    let mut o = o.unwrap();
                    for mode in [
                        BatchMode::FreshFactor,
                        BatchMode::SerialReuse,
                        BatchMode::FaerBatch,
                    ] {
                        let _ = o
                            .solve(&rhs, mode, &ParallelExecution::sequential())
                            .unwrap();
                        let (q, alloc) =
                            measure(|| o.solve(&rhs, mode, &ParallelExecution::sequential()));
                        let q = q.unwrap();
                        assert_eq!(q.values, rhs);
                        assert_eq!(q.residual_upper, 0.);
                        println!(
                            "BATCH {}",
                            json!({"detail":detail,"mode":format!("{mode:?}"),"matrix":a.as_slice(),"rhs":rhs,"values":q.values,"residual":q.residual_upper,"setup":setup,"allocation":alloc,"residual_terms":q.residual_terms})
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn prepared_vector_batch_authority() {
    use rodas5p_core::DenseMatrix;
    use rodas5p_integrators::{
        ParallelExecution,
        retro_foundation::{BatchMode, PreparedRhs},
    };
    let src = DaeSource::new(
        vec![p(vec![tr(1., vec![F::component(0, 0.)])])],
        vec![
            p(vec![
                tr(2., vec![F::component(1, 0.)]),
                tr(0.25, vec![F::component(2, 0.)]),
                tr(-1., vec![F::component(0, 0.)]),
            ]),
            p(vec![
                tr(0.125, vec![F::component(1, 0.)]),
                tr(1.5, vec![F::component(2, 0.)]),
                tr(-0.5, vec![F::component(0, 0.)]),
            ]),
        ],
    )
    .unwrap();
    let j = src.jacobian_values(0., &[0.25, 0., 0.]).unwrap();
    let a = DenseMatrix::new(2, 2, vec![j[4], j[5], j[7], j[8]]).unwrap();
    let rhs = vec![vec![1., 0.], vec![0., 1.], vec![0.25, 0.5]];
    let mut owner = PreparedRhs::new(a.clone()).unwrap();
    let mut old = None;
    for mode in [
        BatchMode::FreshFactor,
        BatchMode::SerialReuse,
        BatchMode::FaerBatch,
    ] {
        let _ = owner
            .solve(&rhs, mode, &ParallelExecution::sequential())
            .unwrap();
        let (q, c) = measure(|| owner.solve(&rhs, mode, &ParallelExecution::sequential()));
        let q = q.unwrap();
        if let Some(v) = &old {
            assert_eq!(&q.values, v);
        } else {
            old = Some(q.values.clone());
        }
        println!(
            "VECTOR_BATCH {}",
            serde_json::json!({"mode":format!("{mode:?}"),"matrix":a.as_slice(),"rhs":rhs,"values":q.values,"residual":q.residual_upper,"error_upper":q.error_upper,"allocation":c})
        );
    }
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

#[test]
#[ignore = "actual opt-in Affine5 candidate matrix"]
fn affine_training() {
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
        for solver in [EvolutionRoute::Affine5] {
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
