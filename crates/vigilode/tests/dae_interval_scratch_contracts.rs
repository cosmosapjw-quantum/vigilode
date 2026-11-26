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
fn actual_constraint_and_projection_scratch() {
    let model = RobertsonModel::new(3e7).unwrap();
    let src = model.source();
    let v = 1. / 8192.;
    let y = [1. - v, v, 0.];
    let branch = [[-0.25, 1.25]];
    let (r, a) = measure(|| src.constraint_upper(0., &y));
    assert_eq!(r.unwrap(), 0.);
    let (r, b) = measure(|| {
        de::project(
            src,
            0.,
            &y[..2],
            &y[2..],
            &branch,
            policy().init,
            InitRoute::Reuse,
            None,
        )
    });
    let r = r.unwrap();
    assert!(r.qualified);
    assert_eq!(r.work.evaluations, 5);
    assert_eq!(r.z, vec![0.]);
    println!(
        "SCRATCH {}",
        serde_json::json!({"constraint":a,"project":b})
    );
    assert_eq!(a.requests(), 0, "constraint-only evaluation scratch");
    assert!(
        b.requests() <= 150,
        "initial projection temporary allocation budget: {}",
        b.requests()
    );
}
use rodas5p_core::CoreResult;
fn iv_point(v: f64) -> CoreResult<Iv> {
    Iv::point(v)
}
fn z() -> Iv {
    Iv { lo: 0., hi: 0. }
}
fn legacy_evaluate(polys: &[P], t: Iv, y: &[Iv]) -> CoreResult<(Vec<Iv>, Vec<Vec<Iv>>)> {
    let mut v = Vec::with_capacity(polys.len());
    let mut j = Vec::with_capacity(polys.len());
    for eq in polys {
        let mut sum = z();
        let mut row = vec![z(); 1 + y.len()];
        for term in eq.terms() {
            let vals = term
                .factors
                .iter()
                .map(|f| {
                    f.state.iter().try_fold(
                        iv_point(f.constant)?.add(t.scale(f.time)?)?,
                        |s, &(i, a)| s.add(y[i].scale(a)?),
                    )
                })
                .collect::<CoreResult<Vec<_>>>()?;
            let c = iv_point(term.coefficient)?;
            sum = sum.add(vals.iter().try_fold(c, |s, &v| s.mul(v))?)?;
            for (i, f) in term.factors.iter().enumerate() {
                let other = vals
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| *k != i)
                    .try_fold(c, |s, (_, &v)| s.mul(v))?;
                row[0] = row[0].add(other.scale(f.time)?)?;
                for &(k, a) in &f.state {
                    row[k + 1] = row[k + 1].add(other.scale(a)?)?;
                }
            }
        }
        v.push(sum);
        j.push(row);
    }
    Ok((v, j))
}
fn source_equations(src: &DaeSource) -> Vec<P> {
    let s = serde_json::to_value(src).unwrap();
    let mut out = vec![];
    for group in ["f", "g"] {
        for eq in s[group].as_array().unwrap() {
            let terms = eq["terms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    let factors = t["factors"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|f| F {
                            constant: f["constant"].as_f64().unwrap(),
                            time: f["time"].as_f64().unwrap(),
                            state: f["state"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| (v[0].as_u64().unwrap() as usize, v[1].as_f64().unwrap()))
                                .collect(),
                        })
                        .collect();
                    T {
                        coefficient: t["coefficient"].as_f64().unwrap(),
                        factors,
                    }
                })
                .collect();
            let v = P::new(terms).unwrap();
            assert_eq!(serde_json::to_value(&v).unwrap(), *eq);
            out.push(v);
        }
    }
    out
}
fn same_bits(a: &[Iv], b: &[Iv]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.lo.to_bits(), b.lo.to_bits());
        assert_eq!(a.hi.to_bits(), b.hi.to_bits());
    }
}
fn compare_interval(
    src: &DaeSource,
    t: Iv,
    y: &[Iv],
    detail: serde_json::Value,
) -> serde_json::Value {
    use serde_json::json;
    let eq = source_equations(src);
    let n = y.len();
    let (a, legacy) = measure(|| legacy_evaluate(&eq, t, y));
    let (a, j) = a.unwrap();
    let (b, owned) = measure(|| src.interval_values(t, y));
    let (b, k) = b.unwrap();
    same_bits(&a, &b);
    for (a, b) in j.iter().zip(&k) {
        same_bits(a, b);
    }
    let mut w = IntervalEvalWorkspace::new();
    let mut v = vec![z(); n];
    let mut d = vec![z(); n * (n + 1)];
    let (r, into) = measure(|| src.interval_into(t, y, &mut v, &mut d, &mut w));
    r.unwrap();
    assert_eq!(into.requests(), 0);
    same_bits(&a, &v);
    for (i, row) in j.iter().enumerate() {
        same_bits(row, &d[i * (n + 1)..(i + 1) * (n + 1)]);
    }
    assert!(owned.requests() < legacy.requests());
    json!({"detail":detail,"source":src,"t":t.as_pair(),"y":y.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),
        "values":a.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),
        "jacobian":j.iter().map(|r|r.iter().map(|v|v.as_pair()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "legacy":legacy,"owned":owned,"into":into,"bit_checks":2*2*(n+n*(n+1))})
}
#[test]
fn generic_interval_shape_and_error_atomicity() {
    use serde_json::json;
    for n in [2, 3, 8, 16, 32] {
        let eq = (0..n)
            .map(|i| {
                p((0..16)
                    .map(|j| {
                        let f = F {
                            constant: (j as f64 - 7.) / 16.,
                            time: (i as f64 + 1.) / 32.,
                            state: (0..16)
                                .map(|k| ((i + k / 2) % n, ((k % 5) as f64 - 2.) / 32.))
                                .collect(),
                        };
                        tr(
                            (j as f64 - 8.) / 64.,
                            vec![
                                f.clone(),
                                f,
                                F::component((i + 1) % n, 0.125),
                                F::clock(-0.25),
                            ],
                        )
                    })
                    .collect())
            })
            .collect::<Vec<_>>();
        let src = DaeSource::new(eq[..n - 1].to_vec(), eq[n - 1..].to_vec()).unwrap();
        for k in 0..3 {
            let y = (0..n)
                .map(|i| {
                    let c = (((k + 1) * (i + 3) % 17) as f64 - 8.) / 16.;
                    Iv::new(c - 1. / 64., c + 1. / 128.).unwrap()
                })
                .collect::<Vec<_>>();
            let t = Iv::new((k as f64 - 2.) / 8., (k as f64 - 2.) / 8. + 1. / 64.).unwrap();
            println!(
                "GENERIC {}",
                compare_interval(&src, t, &y, json!({"dimension":n,"case":k}))
            );
        }
    }
    let src = RobertsonModel::new(3e7).unwrap();
    let mut w = IntervalEvalWorkspace::new();
    let sentinel = Iv { lo: -0., hi: 0. };
    let mut v = [sentinel; 3];
    let mut d = [sentinel; 12];
    for (t, y) in [
        (Iv { lo: 1., hi: 0. }, vec![z(); 3]),
        (
            z(),
            vec![
                Iv {
                    lo: 0.,
                    hi: f64::INFINITY,
                },
                z(),
                z(),
            ],
        ),
        (z(), vec![Iv { lo: 1., hi: 0. }, z(), z()]),
        (z(), vec![z(); 2]),
    ] {
        assert!(src.source().interval_values(t, &y).is_err());
        assert!(
            src.source()
                .interval_into(t, &y, &mut v, &mut d, &mut w)
                .is_err()
        );
        same_bits(&v, &[sentinel; 3]);
        same_bits(&d, &[sentinel; 12]);
    }
    assert!(
        src.source()
            .interval_into(z(), &[z(); 3], &mut v[..2], &mut d, &mut w)
            .is_err()
    );
    same_bits(&v, &[sentinel; 3]);
    same_bits(&d, &[sentinel; 12]);
    let over = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![p(vec![tr(
            f64::MAX,
            vec![F {
                constant: 0.,
                time: 0.,
                state: vec![(0, 2.)],
            }],
        )])],
    )
    .unwrap();
    assert!(legacy_evaluate(&source_equations(&over), z(), &[z(); 2]).is_err());
    assert!(
        over.interval_into(z(), &[z(); 2], &mut v[..2], &mut d[..6], &mut w)
            .is_err()
    );
    same_bits(&v, &[sentinel; 3]);
    same_bits(&d, &[sentinel; 12]);
    let (r, a) = measure(|| {
        src.source()
            .interval_into(z(), &[z(); 3], &mut v, &mut d, &mut w)
    });
    r.unwrap();
    assert_eq!(a.requests(), 0);
    let (a, b) = src.source().interval_values(z(), &[z(); 3]).unwrap();
    same_bits(&a, &v);
    same_bits(&b.into_iter().flatten().collect::<Vec<_>>(), &d);
    // Signed zero and subnormal endpoints.
    let q = f64::from_bits(1);
    let y = [
        Iv { lo: -0., hi: 0. },
        Iv { lo: -q, hi: q },
        Iv { lo: 0., hi: q },
    ];
    println!(
        "TINY {}",
        compare_interval(src.source(), z(), &y, json!({"case":"subnormal"}))
    );
}
#[test]
#[ignore = "fixed actual native interval slab portfolio"]
fn interval_portfolio() {
    use serde_json::json;
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
                let slabs = phase.original_slab_diagnostics();
                for (tag, k) in [
                    ("first", 0),
                    ("middle", slabs.len() / 2),
                    ("last", slabs.len() - 1),
                ] {
                    let s = &slabs[k];
                    let c = s.certificate.as_ref().unwrap();
                    assert!(c.global_qualified);
                    let y = c
                        .x_range
                        .iter()
                        .chain(&c.branch_root_box)
                        .map(|v| Iv::new(v[0], v[1]).unwrap())
                        .collect::<Vec<_>>();
                    let t = Iv::new(s.times[0], s.times[1]).unwrap();
                    println!(
                        "INTERVAL {}",
                        compare_interval(
                            model.source(),
                            t,
                            &y,
                            json!({"solver":solver,"cert":cert,"mode":phase.mode,"position":tag,"slab":k,"rate":m[phase.mode].rate,"native_times":s.times})
                        )
                    );
                }
            }
        }
    }
}
