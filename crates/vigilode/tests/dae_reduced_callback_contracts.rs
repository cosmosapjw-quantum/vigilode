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
#[test]
fn actual_reduced_callbacks_noalloc() {
    let src = RobertsonModel::new(3e7).unwrap();
    let x = [1. - 1. / 8192., 1. / 8192.];
    let b = [[-0.25, 1.25]];
    let p = initialize_reduced_problem(
        src.source(),
        0.,
        &x,
        &[0.],
        &b,
        policy(),
        EvolutionRoute::Reduced5,
    )
    .unwrap();
    let t = 0.0005;
    let y = [0.9998658289383162, 6.457305003072096e-5];
    let r = de::project(
        src.source(),
        t,
        &y,
        &p.report.z,
        &b,
        policy().init,
        InitRoute::Reuse,
        None,
    )
    .unwrap();
    let mut rhs = [0.; 2];
    let mut jac = [0.; 4];
    let mut ft = [0.; 2];
    let problem = &p.problem;
    let vigilode::Jacobian::Dense(j) = problem.jacobian_provider() else {
        panic!("not dense")
    };
    let td = problem.time_derivative_fn().unwrap();
    (problem.rhs_fn())(t, &y, &mut rhs).unwrap();
    j(t, &y, &mut jac).unwrap();
    td(t, &y, &mut ft).unwrap();
    let (a, ar) = measure(|| (problem.rhs_fn())(t, &y, &mut rhs));
    a.unwrap();
    let (a, aj) = measure(|| j(t, &y, &mut jac));
    a.unwrap();
    let (a, at) = measure(|| td(t, &y, &mut ft));
    a.unwrap();
    assert_eq!(
        rhs.map(f64::to_bits),
        [r.xdot[0].to_bits(), r.xdot[1].to_bits()]
    );
    assert_eq!(
        jac.map(f64::to_bits),
        std::array::from_fn(|i| r.reduced_jacobian[i / 2][i % 2].to_bits())
    );
    assert_eq!(ft, [0.; 2]);
    println!(
        "CALLBACK_RED_GREEN {}",
        serde_json::json!({"rhs":rhs,"jac":jac,"time":ft,"rhs_alloc":ar,"jac_alloc":aj,"time_alloc":at})
    );
    assert_eq!(
        [ar.requests(), aj.requests(), at.requests()],
        [0; 3],
        "actual reduced callback allocations"
    );
}

fn compare_scalar(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    seed: &[f64],
    b: &[[f64; 2]],
    pol: InitPolicy,
) -> bool {
    let (a, wa) = src.initialize_audited(t, x, seed, b, pol, InitRoute::Reuse);
    let ((r, wb), c) = measure(|| src.initialize_scalar_reuse_audited(t, x, seed, b, pol));
    assert_eq!(
        serde_json::to_string(&wa).unwrap(),
        serde_json::to_string(&wb).unwrap(),
        "work at t={t},x={x:?},seed={seed:?}"
    );
    match (a, r) {
        (Ok(a), Ok(r)) => {
            assert_eq!(c.requests(), 0);
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&r.to_owned_report()).unwrap(),
                "report at t={t},x={x:?},seed={seed:?}"
            );
            true
        }
        (Err(a), Err(b)) => {
            assert_eq!(a.to_string(), b.to_string());
            false
        }
        (a, b) => panic!(
            "scalar outcome mismatch old={} new={}",
            a.is_ok(),
            b.is_ok()
        ),
    }
}
fn test_src(a: f64, degree: usize) -> DaeSource {
    let mut f = vec![
        tr(0.25, vec![]),
        tr(-0.5, vec![F::component(0, 0.)]),
        tr(0.125, vec![F::component(1, 0.)]),
        tr(0.25, vec![F::clock(0.)]),
    ];
    if degree == 2 {
        f.push(tr(0.125, vec![F::component(1, 0.), F::component(1, 0.)]));
    }
    let g = p(vec![
        tr(a, vec![F::component(1, 0.); degree]),
        tr(-0.125, vec![F::component(0, 0.)]),
        tr(-0.125, vec![F::clock(0.)]),
        tr(-0.25, vec![]),
    ]);
    DaeSource::new(vec![p(f)], vec![g]).unwrap()
}
#[test]
fn exact_scalar_receipt_and_refusal_equivalence() {
    let mut qualified = 0;
    let mut refused = 0;
    for a in [0.5, 1., 1.25, 1.5, 3., -0.5, -1., -1.25, -1.5, -3.] {
        for degree in [1, 2] {
            let src = test_src(a, degree);
            let b = if degree == 2 {
                if a < 0. {
                    [[-1.25, 1.25]]
                } else {
                    [[0.125, 1.25]]
                }
            } else {
                [[-2., 2.]]
            };
            for t in [0., 0.125, 0.5] {
                for x in [0.125, 0.25, 0.75] {
                    for seed in [0.25, 0.5, 0.75] {
                        let mut pol = policy().init;
                        for mode in [InitMode::ProjectAlgebraic, InitMode::Strict] {
                            pol.mode = mode;
                            if compare_scalar(&src, t, &[x], &[seed], &b, pol) {
                                qualified += 1;
                            } else {
                                refused += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    // A nonlinear branch whose whole-box contraction can be proved.
    let src = test_src(1.5, 2);
    for t in [0., 0.125, 0.25] {
        for x in [0.125, 0.5] {
            let b = [[0.25, 0.875]];
            for seed in [0.375, 0.5, 0.75] {
                if compare_scalar(&src, t, &[x], &[seed], &b, policy().init) {
                    qualified += 1;
                } else {
                    refused += 1;
                }
            }
        }
    }
    let src = test_src(1.5, 1);
    let mut pol = policy().init;
    pol.condition_cap = 0.5;
    assert!(!compare_scalar(
        &src,
        0.,
        &[0.25],
        &[0.25],
        &[[-1., 1.]],
        pol
    ));
    pol = policy().init;
    pol.derivative_tol = 1e-30;
    let _ = compare_scalar(&src, 0.125, &[0.25], &[0.25], &[[-1., 1.]], pol);
    assert!(!compare_scalar(
        &src,
        0.,
        &[0.25],
        &[0.25],
        &[[0.25, 0.5]],
        policy().init
    ));
    assert!(!compare_scalar(
        &src,
        0.,
        &[f64::NAN],
        &[0.25],
        &[[-1., 1.]],
        policy().init
    ));
    let sing = test_src(0., 1);
    assert!(!compare_scalar(
        &sing,
        0.,
        &[0.25],
        &[0.25],
        &[[-1., 1.]],
        policy().init
    ));
    assert!(qualified > 100 && refused > 100);
    println!(
        "SCALAR_EQUIVALENCE {}",
        serde_json::json!({"qualified":qualified,"refused":refused,"extra":5})
    );
}
fn actual_client(
    src: &DaeSource,
    t: f64,
    x: &[f64],
    seed: &[f64],
    b: &[[f64; 2]],
    route: EvolutionRoute,
) {
    let p = initialize_reduced_problem(src, t, x, seed, b, policy(), route).unwrap();
    let nx = x.len();
    let n = nx + 1;
    let r = de::project(
        src,
        t,
        x,
        &p.report.z,
        b,
        policy().init,
        InitRoute::Reuse,
        None,
    )
    .unwrap();
    let mut rhs = vec![0.; nx];
    let mut jac = vec![0.; nx * nx];
    let mut ft = vec![0.; nx];
    let vigilode::Jacobian::Dense(j) = p.problem.jacobian_provider() else {
        panic!("dense")
    };
    let td = p.problem.time_derivative_fn().unwrap();
    let mut y = x.to_vec();
    y.extend(&r.z);
    let a = src.jacobian_values(t, &y).unwrap();
    let b = src.time_values(t, &y).unwrap();
    let want = (0..nx)
        .map(|i| a[i * n + nx].mul_add(r.zt[0], b[i]))
        .collect::<Vec<_>>();
    let (rr, ar) = measure(|| (p.problem.rhs_fn())(t, x, &mut rhs));
    rr.unwrap();
    let (rr, aj) = measure(|| j(t, x, &mut jac));
    rr.unwrap();
    let (rr, at) = measure(|| td(t, x, &mut ft));
    rr.unwrap();
    assert_eq!([ar.requests(), aj.requests(), at.requests()], [0; 3]);
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(&rhs), bits(&r.xdot));
    assert_eq!(bits(&jac), bits(&r.reduced_jacobian.concat()));
    assert_eq!(bits(&ft), bits(&want));
    println!(
        "CALLBACK {}",
        serde_json::json!({"nx":nx,"route":route,"t":t,"rhs_alloc":ar,"jac_alloc":aj,"time_alloc":at,"rhs":rhs,"jac":jac,"time":ft})
    );
}
#[test]
fn scalar_actual_callback_domain() {
    let src = test_src(1.5, 2);
    for route in [EvolutionRoute::Reduced3, EvolutionRoute::Reduced5] {
        actual_client(&src, 0.125, &[0.5], &[0.5], &[[0.25, 0.875]], route);
        let src = test_src(-1.25, 1);
        actual_client(&src, 0.25, &[0.5], &[-0.25], &[[-1., 1.]], route);
    }
    // Max admitted differential dimension with one algebraic unknown.
    let nx = 31;
    let f = (0..nx)
        .map(|i| {
            p(vec![
                tr(0.125, vec![F::component(i, 0.)]),
                tr(0.25, vec![F::component(nx, 0.)]),
                tr(0.5, vec![F::clock(0.)]),
            ])
        })
        .collect();
    let g = p(vec![
        tr(1.5, vec![F::component(nx, 0.)]),
        tr(-0.125, vec![F::component(0, 0.)]),
        tr(-0.125, vec![F::clock(0.)]),
    ]);
    let src = DaeSource::new(f, vec![g]).unwrap();
    let x = [0.25; 31];
    assert!(compare_scalar(
        &src,
        0.25,
        &x,
        &[0.125],
        &[[-1., 1.]],
        policy().init
    ));
    actual_client(
        &src,
        0.25,
        &x,
        &[0.125],
        &[[-1., 1.]],
        EvolutionRoute::Reduced5,
    );
    // Callback refusal must be atomic and retain the same failure as original project.
    let src = DaeSource::new(
        vec![p(vec![tr(1., vec![])])],
        vec![p(vec![
            tr(1., vec![F::component(1, 0.)]),
            tr(-1., vec![F::component(0, 0.)]),
        ])],
    )
    .unwrap();
    let b = [[-0.25, 0.25]];
    let p = initialize_reduced_problem(
        &src,
        0.,
        &[0.],
        &[0.],
        &b,
        policy(),
        EvolutionRoute::Reduced5,
    )
    .unwrap();
    let err = de::project(
        &src,
        0.,
        &[1.],
        &p.report.z,
        &b,
        policy().init,
        InitRoute::Reuse,
        None,
    )
    .unwrap_err()
    .to_string();
    let mut out = [123.];
    let r = (p.problem.rhs_fn())(0., &[1.], &mut out);
    assert!(matches!(r,Err(vigilode::CallbackError::Fatal(ref e)) if e==&err));
    assert_eq!(out, [123.]);
    println!(
        "CALLBACK_REFUSAL {}",
        serde_json::json!({"error":err,"unchanged_output":out})
    );
}
