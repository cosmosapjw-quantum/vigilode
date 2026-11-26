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
use serde_json::{Value, json};
fn num(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn arr<const N: usize>(v: &Value) -> [f64; N] {
    let a = v.as_array().unwrap();
    assert_eq!(a.len(), N);
    std::array::from_fn(|i| num(&a[i]))
}
fn branch_route(route: EvolutionRoute) -> InitRoute {
    match route {
        EvolutionRoute::Prepared5 => InitRoute::PreparedBatch,
        EvolutionRoute::Scalar5 => InitRoute::ScalarRoot,
        EvolutionRoute::Affine5 => InitRoute::AffineExact,
        _ => InitRoute::Reuse,
    }
}
fn make_graph(
    spec: &Value,
    solver: EvolutionRoute,
    cert: ConservationRoute,
    sync: VectorSyncRoute,
) -> Vec<ConservedMode> {
    spec["modes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            let guards = m["guards"]
                .as_array()
                .unwrap()
                .iter()
                .map(|g| {
                    let q = num(&g["value"]);
                    match g["kind"].as_str().unwrap() {
                        "v" => p(vec![tr(q, vec![]), tr(-1., vec![F::component(1, 0.)])]),
                        "clock" => p(vec![tr(-q, vec![]), tr(1., vec![F::clock(0.)])]),
                        _ => unreachable!(),
                    }
                })
                .collect();
            let edges = m["edges"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| {
                    let a = e["matrix"].as_array().unwrap();
                    let mat = [arr::<3>(&a[0]), arr::<3>(&a[1]), arr::<3>(&a[2])];
                    ConservedTransition {
                        target: e["target"].as_u64().unwrap() as usize,
                        reset: SimplexReset::new(mat).unwrap(),
                        policy: VectorResetPolicy {
                            branch_route: branch_route(solver),
                            sync,
                            derivative: GuardDerivativeRoute::Schur,
                            subdivisions: 1,
                        },
                        sync: SimplexSyncRoute::JointTransport,
                    }
                })
                .collect();
            ConservedMode {
                rate: num(&m["rate"]),
                route: solver,
                certificate: cert,
                guards,
                transitions: edges,
            }
        })
        .collect()
}
fn pm2(order: MultiGuardRoute) -> MultiGuardPolicy {
    let mut p = mp(order);
    p.derivative = GuardDerivativeRoute::ConservedSchur;
    p
}
fn ab() -> PhaseAtlasBudget {
    PhaseAtlasBudget {
        limits: PhaseCycleBudget {
            native_attempts: 4096,
            scan_slabs: 512,
            transitions: 10,
            min_dwell: 1e-5,
            absolute_width: 1e-6,
        },
        reset_checks: 64,
        cache_entries: 10,
    }
}
fn cb() -> ConservedChainBudget {
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
fn ts(a: f64, b: f64) -> Vec<f64> {
    if b <= a {
        return vec![];
    }
    [0., 0.125, 0.375, 0.625, 0.875, 1.]
        .into_iter()
        .map(|u| if u == 1. { b } else { a + (b - a) * u })
        .collect()
}
fn hybrid(spec: &Value) {
    let initial_state = arr::<3>(&spec["initial"]);
    let initial =
        RobertsonInitial::simplex_ball([initial_state[0], initial_state[1]], num(&spec["radius"]))
            .unwrap();
    let end = num(&spec["end"]);
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::Affine5,
    ] {
        for cert in [ConservationRoute::Max, ConservationRoute::Integral4] {
            for order in [MultiGuardRoute::Shared, MultiGuardRoute::Correlated] {
                for method in ["JointCentered", "JointGreen", "AtlasFresh", "AtlasReuse"] {
                    let m = make_graph(
                        spec,
                        solver,
                        cert,
                        if method == "JointGreen" {
                            VectorSyncRoute::GreenMoment
                        } else {
                            VectorSyncRoute::Centered
                        },
                    );
                    let mut row = json!({"case":spec["id"],"solver":solver,"cert":cert,"order":order,"method":method});
                    if method.starts_with("Atlas") {
                        let ((r, a), alloc) = measure(|| {
                            evolve_robertson_phase_atlas_audited(
                                &m,
                                0,
                                initial,
                                [0., end],
                                &opts(),
                                policy(),
                                pm2(order),
                                ab(),
                                if method == "AtlasReuse" {
                                    PhaseReuseRoute::Reuse
                                } else {
                                    PhaseReuseRoute::Fresh
                                },
                            )
                        });
                        row["audit"] = json!(a);
                        row["allocation"] = json!(alloc);
                        match r {
                            Err(e) => row["error"] = json!(e.to_string()),
                            Ok(r) => {
                                let mut qs = vec![];
                                for (i, e) in r.epochs().iter().enumerate() {
                                    for t in ts(e.phase.clock[1], e.phase.absolute_safe) {
                                        for route in
                                            [PhaseQueryRoute::Horner, PhaseQueryRoute::Centered]
                                        {
                                            let v = match r.query(i, t, route) {
                                                Ok(v) => json!({"query":v}),
                                                Err(e) => json!({"error":e.to_string()}),
                                            };
                                            qs.push(
                                                json!({"epoch":i,"t":t,"route":route,"result":v}),
                                            );
                                        }
                                    }
                                }
                                row["atlas"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"phases":r.phases().iter().map(|p|json!({"mode":p.mode,"initial":p.initial,"winner":p.ordered().winner(),"safe":p.ordered().committed_until(),"reason":p.ordered().reason(),"guards":p.ordered().guards(),"comparisons":p.ordered().comparisons(),"work":p.ordered().work,"slabs":p.original_slab_diagnostics()})).collect::<Vec<_>>(),"epochs":r.epochs(),"events":r.events(),"queries":qs});
                            }
                        }
                    } else {
                        let ((r, a), alloc) = measure(|| {
                            evolve_conserved_chain_audited(
                                &m,
                                0,
                                initial,
                                [0., end],
                                &opts(),
                                policy(),
                                pm2(order),
                                cb(),
                                SimplexGapRoute::AffineDual,
                            )
                        });
                        row["audit"] = json!(a);
                        row["allocation"] = json!(alloc);
                        match r {
                            Err(e) => row["error"] = json!(e.to_string()),
                            Ok(r) => {
                                let mut es = vec![];
                                for (i, e) in r.epochs().iter().enumerate() {
                                    let qs = ts(e.start(), e.safe())
                                        .into_iter()
                                        .map(|t| {
                                            let v = match r.query(i, t) {
                                                Ok(v) => json!({"query":v}),
                                                Err(e) => json!({"error":e.to_string()}),
                                            };
                                            json!({"t":t,"result":v})
                                        })
                                        .collect::<Vec<_>>();
                                    es.push(json!({"mode":e.mode(),"start":e.start(),"safe":e.safe(),"radius":e.incoming_l1(),"center":e.center(),"slabs":e.original_slab_diagnostics(),"queries":qs}));
                                }
                                row["joint"] = json!({"stop":r.stop(),"reason":r.reason(),"final_mode":r.final_mode(),"work":r.work(),"events":r.transitions(),"epochs":es});
                            }
                        }
                    }
                    println!("CHALLENGE {}", row);
                }
            }
        }
    }
}
fn analytic(spec: &Value) {
    let c = num(&spec["c"]);
    let e = num(&spec["e"]);
    let v = num(&spec["v"]);
    let a = num(&spec["alpha"]);
    let k = num(&spec["kappa"]);
    let rad = num(&spec["radius"]);
    let end = num(&spec["end"]);
    let grid = (0..=spec["grid_divisions"].as_u64().unwrap())
        .map(|i| end * i as f64 / num(&spec["grid_divisions"]))
        .collect::<Vec<_>>();
    for degree in spec["degrees"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
    {
        for l in spec["lambdas"].as_array().unwrap().iter().map(num) {
            let zf = vec![F::component(1, 0.); degree];
            let xx = vec![F::component(0, 0.); 2];
            let g = p(vec![
                tr(1., zf.clone()),
                tr(-1., xx.clone()),
                tr(-a, vec![F::clock(0.)]),
            ]);
            let f = p(vec![
                tr(v + l * c, vec![]),
                tr(-l, vec![F::component(0, 0.)]),
                tr(l * v - k * a, vec![F::clock(0.)]),
                tr(k, zf),
                tr(-k, xx),
            ]);
            let src = DaeSource::new(vec![f], vec![g]).unwrap();
            let branch = if degree == 1 {
                [[-0.25, 1.25]]
            } else {
                [[0.25, 1.25]]
            };
            for solver in [
                EvolutionRoute::Mass,
                EvolutionRoute::Prepared5,
                EvolutionRoute::Scalar5,
                EvolutionRoute::Affine5,
            ] {
                let ((r, a), alloc) = measure(|| {
                    evolve_epoch_audited(
                        &src,
                        &[c + e],
                        &[0.5],
                        &branch,
                        EvolutionEpoch {
                            start: 0.,
                            end,
                            incoming_x_error: Some(rad),
                        },
                        &opts(),
                        policy(),
                        solver,
                        &grid,
                    )
                });
                let mut row = json!({"degree":degree,"lambda":l,"solver":solver,"audit":a,"allocation":alloc});
                match r {
                    Err(e) => row["error"] = json!(e.to_string()),
                    Ok(r) => {
                        row["run"] = json!({"initial":r.initial,"pointwise":r.pointwise_qualified,"branch":r.branch_qualified,"global":r.global_qualified,"bound":r.max_physical_error,"last":r.driver().t.last(),"stats":r.driver().stats,"reason":r.reason,"accepted":r.accepted,"queries":r.queries,"slabs":r.slabs()})
                    }
                }
                println!("ANALYTIC {}", row);
            }
        }
    }
}
#[test]
#[ignore = "one-shot preregistered independent M2 challenge; use only through sealed verdict script"]
fn sealed_validation() {
    let spec: Value = serde_json::from_str(include_str!(
        "../../../research/m2_dae_independent_validation_20261126/CASE_SPEC.json"
    ))
    .unwrap();
    for s in spec["hybrid"].as_array().unwrap() {
        hybrid(s);
    }
    analytic(&spec["analytic"]);
}
