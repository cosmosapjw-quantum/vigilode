#![cfg(feature = "polynomial-events")]

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
fn bits(a: &[f64], b: &[f64]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert_eq!(x.to_bits(), y.to_bits());
    }
}
#[test]
fn mass_callbacks_noalloc() {
    let model = RobertsonModel::new(3e7).unwrap();
    let src = model.source();
    let a = vigilode::polynomial_events::initialize_mass_problem(
        src,
        0.,
        &[1. - 1. / 8192., 1. / 8192.],
        &[0.125],
        &[[-0.25, 1.25]],
        policy().init,
        InitRoute::Reuse,
    )
    .unwrap();
    let mut f = [0.; 3];
    let mut j = [0.; 9];
    let y = &a.initial_state;
    let vigilode::Jacobian::Dense(jac) = a.problem.jacobian_provider() else {
        panic!("dense jacobian")
    };
    let (r, fa) = measure(|| (a.problem.rhs_fn())(0., y, &mut f));
    r.unwrap();
    let (r, ja) = measure(|| jac(0., y, &mut j));
    r.unwrap();
    bits(&f, &src.rhs_values(0., y).unwrap());
    bits(&j, &src.jacobian_values(0., y).unwrap());
    println!("CALLBACK {}", serde_json::json!({"rhs":fa,"jac":ja}));
    assert_eq!(
        fa.requests() + ja.requests(),
        0,
        "actual mass callbacks must not allocate"
    );
}

fn run_into(
    src: &DaeSource,
    t: f64,
    y: &[f64],
    kind: SourceEvalKind,
    out: &mut [f64],
) -> rodas5p_core::CoreResult<()> {
    match kind {
        SourceEvalKind::Rhs => src.rhs_into(t, y, out),
        SourceEvalKind::Jacobian => src.jacobian_into(t, y, out),
        SourceEvalKind::Time => src.time_into(t, y, out),
    }
}
fn old_values(
    src: &DaeSource,
    t: f64,
    y: &[f64],
    kind: SourceEvalKind,
) -> rodas5p_core::CoreResult<Vec<f64>> {
    match kind {
        SourceEvalKind::Rhs => src.rhs_values(t, y),
        SourceEvalKind::Jacobian => src.jacobian_values(t, y),
        SourceEvalKind::Time => src.time_values(t, y),
    }
}
const KINDS: [SourceEvalKind; 3] = [
    SourceEvalKind::Rhs,
    SourceEvalKind::Jacobian,
    SourceEvalKind::Time,
];
#[test]
fn source_into_full_shape_and_order() {
    let mut checked = 0usize;
    let mut work = SourceEvalWorkspace::new();
    // Repeated affine factors and duplicate indices, nonautonomous clock,
    // maximal 16 terms, four factors, 16 entries per factor, all dimensions.
    for n in [2, 3, 8, 16, 32] {
        let equations = (0..n)
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
        let src = DaeSource::new(equations[..n - 1].to_vec(), equations[n - 1..].to_vec()).unwrap();
        for k in 0..7 {
            let y = (0..n)
                .map(|i| (((k + 1) * (i + 3) % 17) as f64 - 8.) / 16.)
                .collect::<Vec<_>>();
            let t = (k as f64 - 3.) / 8.;
            for kind in KINDS {
                let v = old_values(&src, t, &y, kind).unwrap();
                let mut a = vec![-999.; v.len()];
                let mut b = a.clone();
                let (r, x) = measure(|| run_into(&src, t, &y, kind, &mut a));
                r.unwrap();
                let (r, z) = measure(|| src.evaluate_into(t, &y, kind, &mut b, &mut work));
                r.unwrap();
                assert_eq!(x.requests() + z.requests(), 0);
                bits(&a, &v);
                bits(&b, &v);
                checked += 2 * v.len();
            }
        }
    }
    // Exact analytic dyadic model with repeated factor and repeated state index.
    // f=(1+t+2x)^2*(1-z); g=x+z-1.
    let a = F {
        constant: 1.,
        time: 1.,
        state: vec![(0, 1.), (0, 1.)],
    };
    let src = DaeSource::new(
        vec![p(vec![tr(
            1.,
            vec![
                a.clone(),
                a,
                F {
                    constant: 1.,
                    time: 0.,
                    state: vec![(1, -1.)],
                },
            ],
        )])],
        vec![p(vec![
            tr(1., vec![F::component(0, 0.)]),
            tr(1., vec![F::component(1, 0.)]),
            tr(-1., vec![]),
        ])],
    )
    .unwrap();
    let mut f = [0.; 2];
    let mut j = [0.; 4];
    let mut t = [0.; 2];
    src.rhs_into(0.25, &[0.25, 0.75], &mut f).unwrap();
    src.jacobian_into(0.25, &[0.25, 0.75], &mut j).unwrap();
    src.time_into(0.25, &[0.25, 0.75], &mut t).unwrap();
    bits(&f, &[49. / 64., 0.]);
    bits(&j, &[1.75, -49. / 16., 1., 1.]);
    bits(&t, &[0.875, 0.]);
    // Finite tiny/negative-zero inputs, original represented rounding.
    for y in [
        [-0., 0.],
        [f64::from_bits(1), -f64::from_bits(1)],
        [-1e-300, 1e-300],
    ] {
        for kind in KINDS {
            let v = old_values(&src, -0., &y, kind).unwrap();
            let mut out = vec![0.; v.len()];
            let (r, a) = measure(|| run_into(&src, -0., &y, kind, &mut out));
            r.unwrap();
            assert_eq!(a.requests(), 0);
            bits(&out, &v);
        }
    }
    println!(
        "SHAPE {}",
        serde_json::json!({"bit_checks":checked,"dimensions":[2,3,8,16,32],"workspace_bytes":std::mem::size_of::<SourceEvalWorkspace>()})
    );
}
#[test]
fn source_into_failure_is_atomic() {
    let normal = RobertsonModel::new(3e7).unwrap();
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
    let mut work = SourceEvalWorkspace::new();
    for kind in KINDS {
        let n = if kind == SourceEvalKind::Jacobian {
            4
        } else {
            2
        };
        let mut out = vec![-0.; n];
        // Value itself is zero, but an unrequested derivative overflows in
        // the last equation. Preserve old refusal semantics for all kinds.
        assert!(old_values(&over, 0., &[0., 0.], kind).is_err());
        assert!(run_into(&over, 0., &[0., 0.], kind, &mut out).is_err());
        bits(&out, &vec![-0.; n]);
        assert!(
            over.evaluate_into(0., &[0., 0.], kind, &mut out, &mut work)
                .is_err()
        );
        bits(&out, &vec![-0.; n]);
        for (t, y) in [
            (f64::NAN, vec![0., 0., 0.]),
            (0., vec![0., f64::INFINITY, 0.]),
            (0., vec![0., 0.]),
        ] {
            let n = if kind == SourceEvalKind::Jacobian {
                9
            } else {
                3
            };
            let mut out = vec![42.; n];
            assert!(
                normal
                    .source()
                    .evaluate_into(t, &y, kind, &mut out, &mut work)
                    .is_err()
            );
            bits(&out, &vec![42.; n]);
            assert!(run_into(normal.source(), t, &y, kind, &mut out).is_err());
            bits(&out, &vec![42.; n]);
        }
        let mut wrong = [42.; 1];
        assert!(run_into(normal.source(), 0., &[1., 0., 0.], kind, &mut wrong).is_err());
        bits(&wrong, &[42.]);
        let v = old_values(normal.source(), 0., &[1., 0., 0.], kind).unwrap();
        let mut out = vec![42.; v.len()];
        let (r, a) = measure(|| {
            normal
                .source()
                .evaluate_into(0., &[1., 0., 0.], kind, &mut out, &mut work)
        });
        r.unwrap();
        assert_eq!(a.requests(), 0);
        bits(&out, &v);
    }
}
#[test]
fn actual_time_and_error_callbacks() {
    let model = RobertsonModel::new(6e7).unwrap();
    let src = model.source();
    let a = vigilode::polynomial_events::initialize_mass_problem(
        src,
        0.,
        &[1., 0.],
        &[0.125],
        &[[-0.25, 1.25]],
        policy().init,
        InitRoute::Reuse,
    )
    .unwrap();
    let mut f = [0.; 3];
    let ft = a.problem.time_derivative_fn().unwrap();
    let (r, c) = measure(|| ft(0., &a.initial_state, &mut f));
    r.unwrap();
    assert_eq!(c.requests(), 0);
    bits(&f, &src.time_values(0., &a.initial_state).unwrap());
    let mut bad = [42.; 1];
    assert!((a.problem.rhs_fn())(0., &a.initial_state, &mut bad).is_err());
    bits(&bad, &[42.]);
    let mut f = [42.; 3];
    assert!(ft(f64::NAN, &a.initial_state, &mut f).is_err());
    bits(&f, &[42.; 3]);
}
fn candidate_problem(src: &DaeSource, base: &vigilode::Problem, method: &str) -> vigilode::Problem {
    if method == "StackInto" {
        return base.clone();
    }
    use std::sync::Mutex;
    let callback = |kind: SourceEvalKind| {
        let a = src.clone();
        let method = method == "ReusedInto";
        let work = Mutex::new(SourceEvalWorkspace::new());
        move |t: f64, y: &[f64], out: &mut [f64]| -> Result<(), vigilode::CallbackError> {
            let r = if method {
                let mut w = work.lock().map_err(|_| {
                    vigilode::CallbackError::Fatal("source workspace poisoned".into())
                })?;
                a.evaluate_into(t, y, kind, out, &mut w)
            } else {
                old_values(&a, t, y, kind).map(|v| out.copy_from_slice(&v))
            };
            r.map_err(|e| vigilode::CallbackError::Fatal(e.to_string()))
        }
    };
    vigilode::Problem::new(src.nx() + src.nz(), callback(SourceEvalKind::Rhs))
        .jacobian(vigilode::Jacobian::dense(callback(
            SourceEvalKind::Jacobian,
        )))
        .time_derivative(callback(SourceEvalKind::Time))
        .mass(base.mass_matrix().unwrap().to_vec())
}
#[test]
#[ignore = "fixed actual source evaluator mass driver portfolio"]
fn source_portfolio() {
    use serde_json::json;
    for rate in [3e7, 6e7] {
        for (case, x) in [
            ("vertex", [1., 0.]),
            ("cycle", [1. - 1. / 8192., 1. / 8192.]),
        ] {
            for rt in [1e-5, 1e-7, 1e-9] {
                let model = RobertsonModel::new(rate).unwrap();
                let src = model.source();
                let a = vigilode::polynomial_events::initialize_mass_problem(
                    src,
                    0.,
                    &x,
                    &[0.125],
                    &[[-0.25, 1.25]],
                    policy().init,
                    InitRoute::Reuse,
                )
                .unwrap();
                let o = Options {
                    rtol: rt,
                    atol: Atol::Scalar(rt * 0.001),
                    ..opts()
                };
                let mut oracle: Option<vigilode::Solution> = None;
                for method in ["LegacyVec", "StackInto", "ReusedInto"] {
                    let p = candidate_problem(src, &a.problem, method);
                    // Warm callback and solver infrastructure identically outside measured solve.
                    let warm =
                        vigilode::Rodas5p::solve(&p, (0., 0.002), &a.initial_state, &o).unwrap();
                    assert!(warm.is_success());
                    drop(warm);
                    let (sol, alloc) =
                        measure(|| vigilode::Rodas5p::solve(&p, (0., 0.002), &a.initial_state, &o));
                    let sol = sol.unwrap();
                    assert!(sol.is_success());
                    if let Some(ref q) = oracle {
                        bits(&sol.t, &q.t);
                        for (y, z) in sol.y.iter().zip(&q.y) {
                            bits(y, z);
                        }
                        assert_eq!(
                            serde_json::to_value(&sol).unwrap(),
                            serde_json::to_value(q).unwrap()
                        );
                        for t in (0..=64).map(|i| 0.002 * i as f64 / 64.) {
                            bits(&sol.at(t).unwrap(), &q.at(t).unwrap());
                        }
                    }
                    let mut cb = [Counts::default(); 3];
                    let mut checked = 0;
                    let vigilode::Jacobian::Dense(j) = p.jacobian_provider() else {
                        panic!("dense jac")
                    };
                    let ft = p.time_derivative_fn().unwrap();
                    for (&t, y) in sol.t.iter().zip(&sol.y) {
                        for (i, kind) in KINDS.into_iter().enumerate() {
                            let len = if kind == SourceEvalKind::Jacobian {
                                9
                            } else {
                                3
                            };
                            let mut out = [0.; 9];
                            let (r, c) = measure(|| match kind {
                                SourceEvalKind::Rhs => (p.rhs_fn())(t, y, &mut out[..len]),
                                SourceEvalKind::Jacobian => j(t, y, &mut out[..len]),
                                SourceEvalKind::Time => ft(t, y, &mut out[..len]),
                            });
                            r.unwrap();
                            cb[i].add(c);
                            bits(&out[..len], &old_values(src, t, y, kind).unwrap());
                            checked += len;
                            if method != "LegacyVec" {
                                assert_eq!(c.requests(), 0);
                            }
                        }
                    }
                    if method == "LegacyVec" {
                        assert!(cb.iter().all(|c| c.requests() > 0));
                    }
                    println!(
                        "SOURCE {}",
                        json!({"case":case,"rate":rate,"rtol":rt,"method":method,"alloc":alloc,"callbacks":cb,"callback_points":sol.t.len(),"bit_checks":checked,"stats":sol.stats,"final_state":sol.final_state().unwrap(),"dense":[sol.at(0.001).unwrap(),sol.at(0.0017).unwrap()]})
                    );
                    if oracle.is_none() {
                        oracle = Some(sol);
                    }
                }
            }
        }
    }
}
