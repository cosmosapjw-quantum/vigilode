use rodas5p_core::directed::Interval as Iv;
use vigilode::spatial::{DiffusionGrid, SpatialAdjointRoute, SpatialAdjointWorkspace};
#[test]
fn actual_mass_goal_uses_no_time_integration() {
    let g = DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap();
    let n = g.dimension();
    let c = vec![1.; n];
    let y = (0..n).map(|i| i as f64 / 64.).collect::<Vec<_>>();
    let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
    let mut out = vec![Iv { lo: 0., hi: 0. }; n];
    let r = w
        .certify_into(
            &c,
            &y,
            &y,
            0.125,
            SpatialAdjointRoute::ModeResidual,
            &mut out,
        )
        .unwrap();
    assert_eq!(r.actions, 1);
    assert_eq!(r.cells, 0);
    assert!(out.iter().all(|v| v.contains(1.)));
    assert!(r.signed_goal_error[0] <= 0. && r.signed_goal_error[1] >= 0.);
    assert!(r.width_upper <= 1e-12);
    println!("SPATIAL_RED_GREEN {:?}", r);
}

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
// Separate peak-delta measurement; preserve the original Counts schema
// and all historical allocator replay records.
thread_local! {
    static LIVE_DELTA:Cell<Option<(i64,i64)>>=const{Cell::new(None)};
}
fn live_delta(change: i64) {
    let _ = LIVE_DELTA.try_with(|v| {
        if let Some((cur, peak)) = v.get() {
            let next = cur + change;
            v.set(Some((next, peak.max(next))));
        }
    });
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
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            live_delta(l.size() as i64);
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(1, l.size());
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            live_delta(l.size() as i64);
        }
        p
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(2, n);
        let p = unsafe { System.realloc(p, l, n) };
        if !p.is_null() {
            live_delta(n as i64 - l.size() as i64);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        record(3, l.size());
        live_delta(-(l.size() as i64));
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
#[derive(Clone, Copy, Debug, Serialize)]
struct LiveHeap {
    end_delta: i64,
    peak_delta: i64,
}
fn measure_live<T>(f: impl FnOnce() -> T) -> (T, Counts, LiveHeap) {
    assert!(LIVE_DELTA.with(Cell::get).is_none());
    LIVE_DELTA.with(|v| v.set(Some((0, 0))));
    let (r, a) = measure(f);
    let (end_delta, peak_delta) = LIVE_DELTA.with(|v| v.replace(None).unwrap());
    (
        r,
        a,
        LiveHeap {
            end_delta,
            peak_delta,
        },
    )
}

use vigilode::spatial::{BoxTransform, SpatialRows};
use vigilode::{Atol, Dopri5, Options, Problem, Rodas5p, VigilODE};
const DURATION: f64 = 0.125;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
enum NativeRoute {
    Dopri,
    Rodas,
    Paired,
    Complex,
}
#[derive(Debug, Serialize)]
struct NativeReceipt {
    values: Vec<f64>,
    rhs: u64,
    jvp: u64,
    linear_solves: u64,
    linear_iterations: u64,
    accepted: usize,
    rejected: usize,
}
fn native(
    g: &DiffusionGrid,
    p: &Problem,
    initial: &[f64],
    o: &Options,
    route: NativeRoute,
    adjoint: bool,
) -> NativeReceipt {
    if route == NativeRoute::Dopri {
        let r = Dopri5::solve(p, (0., DURATION), initial, o)
            .unwrap()
            .ensure_success()
            .unwrap();
        return NativeReceipt {
            values: r.final_state,
            rhs: r.stats.counters.rhs_calls,
            jvp: 0,
            linear_solves: 0,
            linear_iterations: 0,
            accepted: r.stats.accepted_steps,
            rejected: r.stats.rejected_steps,
        };
    }
    let r = if route == NativeRoute::Rodas {
        Rodas5p::solve(p, (0., DURATION), initial, o)
            .unwrap()
            .ensure_success()
            .unwrap()
    } else {
        // In exact reversible FV arithmetic L^T=D L D^-1. Represented
        // spherical area weights may not be an exact similarity: numerical
        // candidate only, checked independently against original L^T.
        let v = if adjoint {
            initial
                .iter()
                .enumerate()
                .map(|(i, &c)| c / g.cell_measure(i).unwrap())
                .collect::<Vec<_>>()
        } else {
            initial.to_vec()
        };
        let stages =
            g.separable_stages(0.)
                .unwrap()
                .with_box_transform(if route == NativeRoute::Paired {
                    BoxTransform::PairedReal
                } else {
                    BoxTransform::Complex
                });
        VigilODE::solve_separable(
            &g.problem(SpatialRows::Serial),
            (0., DURATION),
            &v,
            o,
            &stages,
        )
        .unwrap()
        .ensure_success()
        .unwrap()
    };
    let mut values = r.final_state().unwrap().to_vec();
    if adjoint && matches!(route, NativeRoute::Paired | NativeRoute::Complex) {
        for (i, v) in values.iter_mut().enumerate() {
            *v *= g.cell_measure(i).unwrap();
        }
    }
    NativeReceipt {
        values,
        rhs: r.stats.counters.rhs_calls,
        jvp: r.stats.counters.jvp_calls,
        linear_solves: r.stats.counters.linear_solves,
        linear_iterations: r.stats.counters.linear_iterations,
        accepted: r.stats.accepted_steps,
        rejected: r.stats.rejected_steps,
    }
}
fn fixtures() -> Vec<(&'static str, DiffusionGrid, usize)> {
    vec![
        (
            "box2_small",
            DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap(),
            8,
        ),
        (
            "box2_medium",
            DiffusionGrid::periodic_box([16, 8], [16., 8.], [0.125, 0.25]).unwrap(),
            16,
        ),
        (
            "box3_small",
            DiffusionGrid::periodic_box([4, 4, 4], [4., 4., 4.], [0.125, 0.25, 0.0625]).unwrap(),
            4,
        ),
        (
            "box3_medium",
            DiffusionGrid::periodic_box([8, 4, 4], [8., 4., 4.], [0.125, 0.25, 0.0625]).unwrap(),
            8,
        ),
        (
            "sphere_small",
            DiffusionGrid::sphere(4, 8, 1., 0.01).unwrap(),
            8,
        ),
        (
            "sphere_medium",
            DiffusionGrid::sphere(8, 16, 1., 0.01).unwrap(),
            16,
        ),
    ]
}
fn max_error(v: &[Iv], x: &[f64]) -> f64 {
    v.iter()
        .zip(x)
        .map(|(a, &b)| a.sub(Iv::point(b).unwrap()).unwrap().mag())
        .fold(0., f64::max)
}
#[test]
fn spatial_vector_adjoint_portfolio() {
    let opts = Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-11),
        h0: 0.001,
        dense_output: false,
        ..Options::default()
    };
    let plans = [
        SpatialAdjointRoute::Taylor18,
        SpatialAdjointRoute::Poisson18,
        SpatialAdjointRoute::PoissonAdaptive,
        SpatialAdjointRoute::ModeResidual,
    ];
    let natives = [
        NativeRoute::Dopri,
        NativeRoute::Rodas,
        NativeRoute::Paired,
        NativeRoute::Complex,
    ];
    for (case, g, width) in fixtures() {
        let n = g.dimension();
        let y = (0..n)
            .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
            .collect::<Vec<_>>();
        let rows = (0..n)
            .map(|i| g.represented_row(i).unwrap())
            .collect::<Vec<_>>();
        let mass = (0..n)
            .map(|i| g.cell_measure(i).unwrap())
            .collect::<Vec<_>>();
        let mut u = vec![0.; n];
        let mut f = vec![0.; n];
        let mut t = vec![0.; n];
        // Check original forward callback independently by exact represented
        // basis rows and transpose bilinear identity (rounded implementation).
        let mut basis_max = 0_f64;
        for j in 0..n {
            u[j] = 1.;
            g.apply(&u, &mut f, SpatialRows::Serial).unwrap();
            g.apply_transpose(&u, &mut t).unwrap();
            for i in 0..n {
                let a = rows[i]
                    .iter()
                    .filter(|&&(k, _)| k != i)
                    .try_fold(Iv::point(0.).unwrap(), |s, &(k, r)| {
                        if j == k {
                            s.add(Iv::point(r)?)
                        } else if j == i {
                            s.sub(Iv::point(r)?)
                        } else {
                            Ok(s)
                        }
                    })
                    .unwrap();
                let b = rows[j]
                    .iter()
                    .filter(|&&(k, _)| k != j)
                    .try_fold(Iv::point(0.).unwrap(), |s, &(k, r)| {
                        if i == k {
                            s.add(Iv::point(r)?)
                        } else if i == j {
                            s.sub(Iv::point(r)?)
                        } else {
                            Ok(s)
                        }
                    })
                    .unwrap();
                let e = a
                    .sub(Iv::point(f[i]).unwrap())
                    .unwrap()
                    .mag()
                    .max(b.sub(Iv::point(t[i]).unwrap()).unwrap().mag());
                basis_max = basis_max.max(e);
            }
            u[j] = 0.;
        }
        assert!(basis_max <= 1e-12);
        let (work, setup) = measure(|| SpatialAdjointWorkspace::new(&g));
        let mut w = work.unwrap();
        println!(
            "SPATIAL_GRID {}",
            serde_json::json!({"case":case,"n":n,"width":width,"duration":DURATION,"rows":rows,"mass":mass,"initial":y,"setup":setup,"basis_comparison_upper":basis_max})
        );
        let p = g.problem(SpatialRows::Serial);
        let adj = g.adjoint_problem();
        let mut primals = Vec::new();
        for route in natives {
            let (r, allocation) = measure(|| native(&g, &p, &y, &opts, route, false));
            primals.push((r, allocation));
        }
        let mut primal_times: [Vec<u128>; 4] = std::array::from_fn(|_| vec![]);
        for rep in 0..5 {
            for j in 0..4 {
                let i = (rep + j) % 4;
                let start = std::time::Instant::now();
                black_box(native(&g, &p, &y, &opts, natives[i], false));
                primal_times[i].push(start.elapsed().as_nanos());
            }
        }
        for (i, (r, a)) in primals.iter().enumerate() {
            primal_times[i].sort_unstable();
            println!(
                "SPATIAL_PRIMAL {}",
                serde_json::json!({"case":case,"candidate":natives[i],"allocation":a,"run":r,"median_ns":primal_times[i][2],"samples_ns":primal_times[i]})
            );
        }
        for goal_name in ["mass", "alternating", "local"] {
            let c = match goal_name {
                "mass" => mass.clone(),
                "alternating" => mass
                    .iter()
                    .enumerate()
                    .map(|(i, &a)| if i % width % 2 == 0 { a } else { -a })
                    .collect(),
                _ => {
                    let mut a = vec![0.; n];
                    a[0] = 0.5;
                    a[n / 2] = -0.25;
                    a[n - 1] = 0.125;
                    a
                }
            };
            println!(
                "SPATIAL_GOAL {}",
                serde_json::json!({"case":case,"goal":goal_name,"weights":c})
            );
            let (r, a) = measure(|| g.apply_transpose(&c, &mut t));
            r.unwrap();
            assert_eq!(a.requests(), 0);
            let mut receipts = Vec::new();
            let mut gradient = vec![Iv::point(0.).unwrap(); n];
            let mut exact_reference = gradient.clone();
            for plan in plans {
                let (r, allocation) = measure(|| {
                    w.certify_into(&c, &y, &primals[0].0.values, DURATION, plan, &mut gradient)
                });
                let r = r.unwrap();
                assert_eq!(allocation.requests(), 0);
                let qualified = r.width_upper <= 1e-6;
                if plan != SpatialAdjointRoute::ModeResidual {
                    assert!(qualified, "{case} {goal_name} {plan:?} {:?}", r);
                }
                if plan == SpatialAdjointRoute::PoissonAdaptive {
                    exact_reference.copy_from_slice(&gradient);
                }
                receipts.push((r, allocation, gradient.clone(), qualified));
            }
            let mut times: [Vec<u128>; 4] = std::array::from_fn(|_| vec![]);
            for rep in 0..5 {
                for j in 0..4 {
                    let i = (rep + j) % 4;
                    let start = std::time::Instant::now();
                    black_box(
                        w.certify_into(
                            &c,
                            &y,
                            &primals[0].0.values,
                            DURATION,
                            plans[i],
                            &mut gradient,
                        )
                        .unwrap(),
                    );
                    times[i].push(start.elapsed().as_nanos());
                }
            }
            for (i, (r, a, v, qualified)) in receipts.iter().enumerate() {
                times[i].sort_unstable();
                println!(
                    "SPATIAL_CERT {}",
                    serde_json::json!({"case":case,"goal":goal_name,"candidate":plans[i],"qualified":qualified,"allocation":a,"work":r,"enclosure":v.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"median_ns":times[i][2],"samples_ns":times[i]})
                );
            }
            // The actual represented endpoint error is certified from the
            // true dual and the stored primal vector, not numerical solver
            // local LTE or an analytic continuum PDE mode assumption.
            for (i, (r, _)) in primals.iter().enumerate() {
                let a = w
                    .certify_into(
                        &c,
                        &y,
                        &r.values,
                        DURATION,
                        SpatialAdjointRoute::PoissonAdaptive,
                        &mut gradient,
                    )
                    .unwrap();
                assert!(
                    a.signed_goal_error.iter().all(|x| x.abs() <= 1e-6),
                    "{case} {goal_name} {:?}",
                    a.signed_goal_error
                );
                println!(
                    "SPATIAL_QOI {}",
                    serde_json::json!({"case":case,"goal":goal_name,"candidate":natives[i],"error":a.signed_goal_error})
                );
            }
            let mut numerics = Vec::new();
            for route in natives {
                let (r, a) = measure(|| native(&g, &adj, &c, &opts, route, true));
                let err = max_error(&exact_reference, &r.values);
                assert!(err <= 1e-6, "{case} {goal_name} {route:?} {err}");
                numerics.push((r, a, err));
            }
            let mut nt: [Vec<u128>; 4] = std::array::from_fn(|_| vec![]);
            for rep in 0..5 {
                for j in 0..4 {
                    let i = (rep + j) % 4;
                    let start = std::time::Instant::now();
                    black_box(native(&g, &adj, &c, &opts, natives[i], true));
                    nt[i].push(start.elapsed().as_nanos());
                }
            }
            for (i, (r, a, e)) in numerics.iter().enumerate() {
                nt[i].sort_unstable();
                println!(
                    "SPATIAL_NUMERIC {}",
                    serde_json::json!({"case":case,"goal":goal_name,"candidate":natives[i],"qualified":true,"allocation":a,"run":r,"error_upper":e,"median_ns":nt[i][2],"samples_ns":nt[i]})
                );
            }
            if matches!(case, "box2_small" | "box3_small") {
                // All rates and diagonal sums here are exact dyadics.
                // Reuse the existing dense nonnormal action certificate
                // on the identical represented operator, not a fitted A.
                let mut a = vec![vec![0.; n]; n];
                for (i, row) in rows.iter().enumerate() {
                    for &(j, r) in row {
                        if i != j {
                            a[j][i] += r;
                            a[i][i] -= r;
                        }
                    }
                }
                let (r, alloc) = measure(|| {
                    rodas5p_core::nonnormal_certificate::certify_exp_action(
                        &a,
                        &c,
                        DURATION,
                        None,
                        24,
                        Some(&numerics[0].0.values),
                    )
                });
                let r = r.unwrap();
                assert!(r.error_upper <= 1e-6);
                println!(
                    "SPATIAL_DENSE {}",
                    serde_json::json!({"case":case,"goal":goal_name,"allocation":alloc,"certificate":r})
                );
            }
        }
    }
}
#[test]
fn spatial_adjoint_transaction_and_degenerate_edges() {
    // Two-cell axes have duplicated neighbors. Zero-diffusivity and
    // zero-time paths must preserve exact states, even at signed goals.
    for g in [
        DiffusionGrid::periodic_box([2, 2, 2], [2., 2., 2.], [0.125, 0.25, 0.0625]).unwrap(),
        DiffusionGrid::sphere(2, 2, 1., 0.01).unwrap(),
        DiffusionGrid::sphere(2, 2, 1., 0.).unwrap(),
    ] {
        let n = g.dimension();
        let c = (0..n).map(|i| i as f64 - 2.).collect::<Vec<_>>();
        let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
        let mut out = vec![Iv { lo: -7., hi: 7. }; n];
        for plan in [
            SpatialAdjointRoute::Taylor18,
            SpatialAdjointRoute::Poisson18,
            SpatialAdjointRoute::PoissonAdaptive,
            SpatialAdjointRoute::ModeResidual,
        ] {
            let before = out.clone();
            assert!(w.certify_into(&c, &c, &c, -1., plan, &mut out).is_err());
            assert_eq!(out, before);
            let r = w.certify_into(&c, &c, &c, 0., plan, &mut out).unwrap();
            assert_eq!(r.actions, 0);
            assert_eq!(r.width_upper, 0.);
            assert!(out.iter().zip(&c).all(|(v, &c)| v.lo == c && v.hi == c));
            let (r, a) = measure(|| w.certify_into(&c, &c, &c, DURATION, plan, &mut out));
            assert!(r.is_ok());
            assert_eq!(a.requests(), 0);
        }
        let mut t = vec![0.; n];
        g.apply_transpose(&c, &mut t).unwrap();
        assert!(t.iter().sum::<f64>().abs() <= 1e-12);
    }
    println!(
        "SPATIAL_EDGE {}",
        serde_json::json!({"duplicate_neighbors":true,"polar_two_rings":true,"zero_time":true,"zero_diffusion":true,"transactional_refusal":true})
    );
}

#[test]
fn spherical_transpose_gather_actual_portfolio() {
    let opts = Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-11),
        h0: 0.001,
        dense_output: false,
        ..Options::default()
    };
    for (nt, np) in [(4, 8), (8, 16), (16, 32), (32, 64)] {
        let g = DiffusionGrid::sphere(nt, np, 1., 0.01).unwrap();
        let n = g.dimension();
        let c = (0..n)
            .map(|i| ((i * 17 % 31) as f64 - 15.) / 32.)
            .collect::<Vec<_>>();
        let mut reference = vec![Iv::point(0.).unwrap(); n];
        // Exact directed scatter on the declared original row edges.
        for (i, &v) in c.iter().enumerate() {
            for (j, a) in g.represented_row(i).unwrap() {
                if i != j && a != 0. {
                    let term = Iv::point(v).unwrap().scale(a).unwrap();
                    reference[i] = reference[i].sub(term).unwrap();
                    reference[j] = reference[j].add(term).unwrap();
                }
            }
        }
        let mut out = vec![0.; n];
        let mut samples: [Vec<u128>; 2] = std::array::from_fn(|_| vec![]);
        let mut receipts = Vec::new();
        let apply = |k: usize, out: &mut [f64]| {
            if k == 0 {
                g.apply_transpose(&c, out)
            } else {
                g.apply_transpose_gather(&c, out)
            }
        };
        for k in 0..2 {
            let (r, a) = measure(|| apply(k, &mut out));
            r.unwrap();
            assert_eq!(a.requests(), 0);
            let err = max_error(&reference, &out);
            assert!(err <= 1e-10);
            receipts.push((a, err, out.clone()));
        }
        for rep in 0..7 {
            for j in 0..2 {
                let k = (rep + j) % 2;
                let s = std::time::Instant::now();
                for _ in 0..2048 {
                    apply(k, &mut out).unwrap();
                    black_box(&out);
                }
                samples[k].push(s.elapsed().as_nanos() / 2048);
            }
        }
        let case = format!("sphere_{nt}x{np}");
        println!(
            "GATHER_GRID {}",
            serde_json::json!({"case":case,"n":n,"weights":c,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>()})
        );
        for (k, (allocation, error, values)) in receipts.into_iter().enumerate() {
            samples[k].sort_unstable();
            println!(
                "GATHER_KERNEL {}",
                serde_json::json!({"case":case,"candidate":(["Scatter","Gather"][k]),"qualified":true,"allocation":allocation,"error_upper":error,"values":values,"median_ns":samples[k][3],"samples_ns":samples[k]})
            );
        }
        if nt <= 8 {
            let p = [g.adjoint_problem(), g.adjoint_problem_gather()];
            let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
            let mut exact = vec![Iv::point(0.).unwrap(); n];
            w.certify_into(
                &c,
                &c,
                &c,
                DURATION,
                SpatialAdjointRoute::PoissonAdaptive,
                &mut exact,
            )
            .unwrap();
            let mut rows = vec![];
            let mut timing: [Vec<u128>; 2] = std::array::from_fn(|_| vec![]);
            for k in 0..2 {
                let (r, a) = measure(|| native(&g, &p[k], &c, &opts, NativeRoute::Dopri, true));
                let e = max_error(&exact, &r.values);
                assert!(e <= 1e-6);
                rows.push((r, a, e));
            }
            for rep in 0..7 {
                for j in 0..2 {
                    let k = (rep + j) % 2;
                    let start = std::time::Instant::now();
                    black_box(native(&g, &p[k], &c, &opts, NativeRoute::Dopri, true));
                    timing[k].push(start.elapsed().as_nanos());
                }
            }
            for (k, (r, allocation, e)) in rows.into_iter().enumerate() {
                timing[k].sort_unstable();
                println!(
                    "GATHER_IVP {}",
                    serde_json::json!({"case":case,"candidate":(["Scatter","Gather"][k]),"qualified":true,"allocation":allocation,"run":r,"error_upper":e,"median_ns":timing[k][3],"samples_ns":timing[k]})
                );
            }
        }
    }
}

#[test]
fn wide_poisson_actual_fewer_cells() {
    let g = DiffusionGrid::periodic_box([16, 8], [16., 8.], [32., 16.]).unwrap();
    let n = g.dimension();
    let y = (0..n)
        .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
        .collect::<Vec<_>>();
    let mut c = vec![0.; n];
    c[0] = 0.5;
    c[n / 2] = -0.25;
    let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
    let mut out = vec![Iv { lo: 0., hi: 0. }; n];
    let a = w
        .certify_into(
            &c,
            &y,
            &y,
            DURATION,
            SpatialAdjointRoute::PoissonAdaptive,
            &mut out,
        )
        .unwrap();
    let b = w
        .certify_into(
            &c,
            &y,
            &y,
            DURATION,
            SpatialAdjointRoute::PoissonWide,
            &mut out,
        )
        .unwrap();
    println!(
        "WIDE_RED_GREEN old={} new={} actions={}/{}",
        a.cells, b.cells, a.actions, b.actions
    );
    assert!(b.cells < a.cells && b.actions < a.actions && b.width_upper <= 1e-6);
}
#[test]
fn stiff_spatial_adjoint_actual_portfolio() {
    let opts = Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-11),
        h0: 0.001,
        dense_output: false,
        ..Options::default()
    };
    let plans = [
        SpatialAdjointRoute::PoissonAdaptive,
        SpatialAdjointRoute::PoissonWide,
        SpatialAdjointRoute::PoissonWideDecay,
    ];
    for (case, g) in [
        (
            "stiff2",
            DiffusionGrid::periodic_box([16, 8], [16., 8.], [32., 16.]).unwrap(),
        ),
        (
            "stiff3",
            DiffusionGrid::periodic_box([8, 8, 4], [8., 8., 4.], [32., 16., 8.]).unwrap(),
        ),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.04).unwrap(),
        ),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.16).unwrap()),
    ] {
        let n = g.dimension();
        let y = (0..n)
            .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
            .collect::<Vec<_>>();
        let mut c = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        let rows = (0..n)
            .map(|i| g.represented_row(i).unwrap())
            .collect::<Vec<_>>();
        let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
        let mut reference = vec![Iv::point(0.).unwrap(); n];
        let q = g
            .separable_stages(0.)
            .unwrap()
            .spectral_bounds()
            .radius_bound;
        println!(
            "STIFF_GRID {}",
            serde_json::json!({"case":case,"n":n,"rows":rows,"initial":y,"goal":c,"duration":DURATION,"spectral_radius_bound_proposal":q})
        );
        // Baseline actual numerical result, all certificate candidates share
        // its original represented endpoint. No continuum exactness claim.
        let p = g.problem(SpatialRows::Serial);
        let adj = g.adjoint_problem();
        let gather = g.adjoint_problem_gather();
        let mut primals = vec![];
        let mut numerics = vec![];
        let native_routes = [
            NativeRoute::Dopri,
            NativeRoute::Rodas,
            NativeRoute::Paired,
            NativeRoute::Complex,
        ];
        for route in native_routes {
            // Large native Krylov is separately deferred to keep this
            // bounded campaign on the same shared host. No extrapolation.
            if route == NativeRoute::Rodas && n > 128 {
                continue;
            }
            let (r, a) = measure(|| native(&g, &p, &y, &opts, route, false));
            let mut ts = vec![];
            for _ in 0..3 {
                let s = std::time::Instant::now();
                black_box(native(&g, &p, &y, &opts, route, false));
                ts.push(s.elapsed().as_nanos());
            }
            ts.sort_unstable();
            println!(
                "STIFF_PRIMAL {}",
                serde_json::json!({"case":case,"candidate":route,"qualified":true,"allocation":a,"run":r,"median_ns":ts[1],"samples_ns":ts})
            );
            primals.push((route, r));
            let (r, a) = measure(|| native(&g, &adj, &c, &opts, route, true));
            let mut ts = vec![];
            for _ in 0..3 {
                let s = std::time::Instant::now();
                black_box(native(&g, &adj, &c, &opts, route, true));
                ts.push(s.elapsed().as_nanos());
            }
            ts.sort_unstable();
            numerics.push((format!("{route:?}"), r, a, ts));
        }
        if case.starts_with("sphere") {
            let (r, a) = measure(|| native(&g, &gather, &c, &opts, NativeRoute::Dopri, true));
            let mut ts = vec![];
            for _ in 0..3 {
                let s = std::time::Instant::now();
                black_box(native(&g, &gather, &c, &opts, NativeRoute::Dopri, true));
                ts.push(s.elapsed().as_nanos());
            }
            ts.sort_unstable();
            numerics.push(("GatherDopri".to_string(), r, a, ts));
        }
        let mut reports = vec![];
        let mut out = reference.clone();
        for plan in plans {
            let (r, a) =
                measure(|| w.certify_into(&c, &y, &primals[0].1.values, DURATION, plan, &mut out));
            let r = r.unwrap();
            assert_eq!(a.requests(), 0);
            assert!(r.width_upper <= 1e-6);
            if plan == SpatialAdjointRoute::PoissonAdaptive {
                reference.copy_from_slice(&out);
            }
            reports.push((r, a, out.clone()));
        }
        let mut timing: [Vec<u128>; 3] = std::array::from_fn(|_| vec![]);
        for rep in 0..5 {
            for j in 0..3 {
                let i = (rep + j) % 3;
                let s = std::time::Instant::now();
                black_box(
                    w.certify_into(&c, &y, &primals[0].1.values, DURATION, plans[i], &mut out)
                        .unwrap(),
                );
                timing[i].push(s.elapsed().as_nanos());
            }
        }
        for (i, (r, a, v)) in reports.into_iter().enumerate() {
            timing[i].sort_unstable();
            println!(
                "STIFF_CERT {}",
                serde_json::json!({"case":case,"candidate":plans[i],"qualified":true,"allocation":a,"work":r,"enclosure":v.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"median_ns":timing[i][2],"samples_ns":timing[i]})
            );
        }
        for (route, r, a, ts) in numerics {
            let e = max_error(&reference, &r.values);
            assert!(e <= 1e-6);
            println!(
                "STIFF_NUMERIC {}",
                serde_json::json!({"case":case,"candidate":route,"qualified":true,"allocation":a,"run":r,"error_upper":e,"median_ns":ts[1],"samples_ns":ts})
            );
        }
        for (route, r) in primals {
            let q = w
                .certify_into(
                    &c,
                    &y,
                    &r.values,
                    DURATION,
                    SpatialAdjointRoute::PoissonAdaptive,
                    &mut out,
                )
                .unwrap();
            assert!(q.signed_goal_error.iter().all(|v| v.abs() <= 1e-6));
            println!(
                "STIFF_QOI {}",
                serde_json::json!({"case":case,"candidate":route,"error":q.signed_goal_error})
            );
        }
    }
}

#[test]
fn shifted_adjoint_original_residual() {
    use vigilode::spatial::{ShiftedAdjointRoute, ShiftedAdjointWorkspace};
    let g = DiffusionGrid::sphere(4, 8, 1., 0.16).unwrap();
    let n = g.dimension();
    let b = (0..n)
        .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
        .collect::<Vec<_>>();
    let mut x = vec![0.; n];
    let mut w = ShiftedAdjointWorkspace::new(&g, ShiftedAdjointRoute::Longitude).unwrap();
    w.prepare(4.).unwrap();
    let a = w
        .solve_into(&b, &mut x, &mut rodas5p_core::WorkCounters::default())
        .unwrap();
    assert!(a.error_l1 <= 1e-8 && a.iterations > 0);
    let mut v = vec![0.; n];
    g.apply_transpose_gather(&x, &mut v).unwrap();
    assert!(
        b.iter()
            .zip(&x)
            .zip(&v)
            .map(|((&b, &x), &v)| (b - 4. * x + v).abs())
            .sum::<f64>()
            <= 1e-8
    );
}
fn shifted_grids() -> Vec<(&'static str, DiffusionGrid)> {
    vec![
        (
            "box2",
            DiffusionGrid::periodic_box([16, 8], [16., 8.], [512., 256.]).unwrap(),
        ),
        (
            "box3",
            DiffusionGrid::periodic_box([8, 8, 4], [8., 8., 4.], [128., 64., 32.]).unwrap(),
        ),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap()),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap(),
        ),
    ]
}
#[test]
fn shifted_stage_actual_portfolio() {
    use rodas5p_core::WorkCounters;
    use vigilode::spatial::{
        ShiftedAdjointRoute as R, ShiftedAdjointWorkspace as W, shifted_adjoint_residual_into,
    };
    for (case, g) in shifted_grids() {
        let n = g.dimension();
        let b = (0..n)
            .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
            .collect::<Vec<_>>();
        let mut out = vec![0.; n];
        let mut scratch = vec![Iv::point(0.).unwrap(); n];
        println!(
            "SHIFT_GRID {}",
            serde_json::json!({"case":case,"n":n,"rhs":b,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>()})
        );
        for z in [4., 32., 256.] {
            for k in [R::Identity, R::Longitude, R::Meridional] {
                let (r, setup) = measure(|| W::new(&g, k));
                let mut w = match r {
                    Ok(w) => w,
                    Err(e) => {
                        assert!(!case.starts_with("sphere") && k != R::Identity);
                        println!(
                            "SHIFT_RUN {}",
                            serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":false,"reason":e.to_string(),"allocation":setup})
                        );
                        continue;
                    }
                };
                let (p, prepare) = measure(|| w.prepare(z));
                p.unwrap();
                assert_eq!(prepare.requests(), 0);
                let mut c = WorkCounters::default();
                let first = w.solve_into(&b, &mut out, &mut c);
                if let Err(e) = first {
                    println!(
                        "SHIFT_RUN {}",
                        serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":false,"reason":e.to_string(),"allocation":setup,"prepare":prepare,"counters":c})
                    );
                    continue;
                }
                let mut c = WorkCounters::default();
                let (r, a) = measure(|| w.solve_into(&b, &mut out, &mut c));
                let r = r.unwrap();
                assert!(r.error_l1 <= 1e-8, "{case} {z} {k:?} {r:?}");
                let reference = out.clone();
                let mut times = vec![];
                for _ in 0..5 {
                    let t = std::time::Instant::now();
                    black_box(
                        w.solve_into(&b, &mut out, &mut WorkCounters::default())
                            .unwrap(),
                    );
                    times.push(t.elapsed().as_nanos());
                    assert_eq!(out, reference);
                }
                times.sort_unstable();
                println!(
                    "SHIFT_RUN {}",
                    serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":true,"allocation":a,"setup":setup,"prepare":prepare,"report":r,"counters":c,"values":out,"median_ns":times[2],"samples_ns":times})
                );
                let old = out.clone();
                assert!(w.prepare(0.).is_err());
                assert!(
                    w.solve_into(&b, &mut out, &mut WorkCounters::default())
                        .is_err()
                );
                assert_eq!(out, old);
            }
            let stages = g.separable_stages(0.).unwrap();
            let (f, setup) = measure(|| stages.stage_solver(z));
            let mut f = f.unwrap();
            let areas = (0..n)
                .map(|i| g.cell_measure(i).unwrap())
                .collect::<Vec<_>>();
            let call = |f: &mut dyn FnMut(&mut [f64]) -> Result<(), vigilode::SolveError>,
                        out: &mut [f64],
                        scratch: &mut [Iv]| {
                for (i, v) in out.iter_mut().enumerate() {
                    *v = b[i] / areas[i];
                }
                f(out).unwrap();
                for (i, v) in out.iter_mut().enumerate() {
                    *v *= areas[i];
                }
                shifted_adjoint_residual_into(&g, z, &b, out, scratch).unwrap()
            };
            call(&mut f, &mut out, &mut scratch);
            let (r, a) = measure(|| call(&mut f, &mut out, &mut scratch));
            assert!(r.error_l1 <= 1e-8);
            let mut times = vec![];
            for _ in 0..5 {
                let t = std::time::Instant::now();
                black_box(call(&mut f, &mut out, &mut scratch));
                times.push(t.elapsed().as_nanos());
            }
            times.sort_unstable();
            println!(
                "SHIFT_RUN {}",
                serde_json::json!({"case":case,"shift":z,"candidate":"Fourier","qualified":true,"allocation":a,"setup":setup,"report":r,"values":out,"median_ns":times[2],"samples_ns":times})
            );
        }
    }
}
#[test]
fn shifted_stage_actual_native_adjoint() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{ShiftedAdjointRoute as R, solve_spatial_adjoint_shifted};
    let o = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        h0: 0.001,
        dense_output: false,
        max_steps: 4096,
        ..Default::default()
    };
    let adaptive = AdaptiveStepConfig {
        rtol: o.rtol,
        atol: 1e-10,
        initial_step: o.h0,
        max_attempts: o.max_steps,
        ..Default::default()
    };
    for (case, g) in shifted_grids() {
        let n = g.dimension();
        let mut c = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
        let mut exact = vec![Iv::point(0.).unwrap(); n];
        let cert = w
            .certify_into(
                &c,
                &c,
                &c,
                DURATION,
                SpatialAdjointRoute::PoissonWideDecay,
                &mut exact,
            )
            .unwrap();
        assert!(cert.width_upper <= 1e-6);
        println!(
            "SHIFT_DUAL {}",
            serde_json::json!({"case":case,"weights":c,"duration":DURATION,"enclosure":exact.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"work":cert})
        );
        let p = g.adjoint_problem_gather();
        for k in [
            NativeRoute::Dopri,
            NativeRoute::Paired,
            NativeRoute::Complex,
        ] {
            let start = std::time::Instant::now();
            let (r, a) = measure(|| native(&g, &p, &c, &o, k, true));
            let elapsed = start.elapsed().as_nanos();
            let e = max_error(&exact, &r.values);
            assert!(e <= 1e-6);
            println!(
                "SHIFT_IVP {}",
                serde_json::json!({"case":case,"candidate":k,"qualified":true,"allocation":a,"run":r,"error_upper":e,"median_ns":elapsed,"timing_repetitions":1})
            );
        }
        for k in [R::Identity, R::Longitude, R::Meridional] {
            if !case.starts_with("sphere") && k != R::Identity {
                continue;
            }
            let start = std::time::Instant::now();
            let (r, a) =
                measure(|| solve_spatial_adjoint_shifted(&g, (0., DURATION), &c, &adaptive, k));
            let elapsed = start.elapsed().as_nanos();
            match r {
                Ok((r, work)) => {
                    let complete = r.observed.success && r.observed.t.last() == Some(&DURATION);
                    let err = if complete {
                        Some(max_error(&exact, r.observed.y.last().unwrap()))
                    } else {
                        None
                    };
                    if complete {
                        assert!(err.unwrap() <= 1e-6);
                        assert_eq!(work.solves as u64, r.observed.counters.linear_solves);
                        assert_eq!(work.returned, work.solves);
                    }
                    println!(
                        "SHIFT_IVP {}",
                        serde_json::json!({"case":case,"candidate":k,"qualified":complete,"allocation":a,"work":work,"run":{"values":r.observed.y.last().unwrap(),"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs":r.observed.counters.rhs_calls,"jvp":r.observed.counters.jvp_calls,"linear_solves":r.observed.counters.linear_solves,"linear_iterations":r.observed.counters.linear_iterations,"counters":r.observed.counters},"error_upper":err,"median_ns":elapsed,"timing_repetitions":1})
                    );
                }
                Err(e) => println!(
                    "SHIFT_IVP {}",
                    serde_json::json!({"case":case,"candidate":k,"qualified":false,"allocation":a,"reason":e.to_string(),"median_ns":elapsed,"timing_repetitions":1})
                ),
            }
        }
    }
}

#[test]
fn early_shifted_native_removes_unnecessary_columns() {
    use rodas5p_core::WorkCounters;
    use vigilode::spatial::{ShiftedAdjointRoute as R, ShiftedAdjointWorkspace as W};
    let g = DiffusionGrid::sphere(4, 8, 1., 0.16).unwrap();
    let n = g.dimension();
    let b = vec![0.125; n];
    let mut v = vec![0.; n];
    let mut a = W::new(&g, R::Identity).unwrap();
    let mut e = W::new(&g, R::EarlyIdentity).unwrap();
    let mut f = W::new(&g, R::FourierTranspose).unwrap();
    a.prepare(4096.).unwrap();
    e.prepare(4096.).unwrap();
    f.prepare(4096.).unwrap();
    let base = a
        .solve_into(&b, &mut v, &mut WorkCounters::default())
        .unwrap();
    let fast = e
        .solve_into(&b, &mut v, &mut WorkCounters::default())
        .unwrap();
    let direct = f
        .solve_into(&b, &mut v, &mut WorkCounters::default())
        .unwrap();
    println!(
        "EARLY_RED_GREEN baseline={} early={} Fourier={}",
        base.iterations, fast.iterations, direct.iterations
    );
    assert!(
        fast.iterations < base.iterations
            && direct.iterations == 0
            && fast.error_l1 <= 1e-8
            && direct.error_l1 <= 1e-8
    );
}
fn early_grids() -> Vec<(&'static str, DiffusionGrid)> {
    vec![
        ("sphere_odd", DiffusionGrid::sphere(5, 9, 1., 0.64).unwrap()),
        (
            "box2",
            DiffusionGrid::periodic_box([16, 8], [16., 8.], [512., 256.]).unwrap(),
        ),
        (
            "box3",
            DiffusionGrid::periodic_box([8, 8, 4], [8., 8., 4.], [128., 64., 32.]).unwrap(),
        ),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap()),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap(),
        ),
    ]
}
#[test]
fn early_shifted_actual_portfolio() {
    use rodas5p_core::WorkCounters;
    use vigilode::spatial::{
        ShiftedAdjointRoute as R, ShiftedAdjointWorkspace as W, shifted_adjoint_residual_into,
    };
    for (case, g) in early_grids() {
        let n = g.dimension();
        let b = (0..n)
            .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
            .collect::<Vec<_>>();
        let mut out = vec![0.; n];
        let mut scratch = vec![Iv::point(0.).unwrap(); n];
        println!(
            "EARLY_GRID {}",
            serde_json::json!({"case":case,"n":n,"rhs":b,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>()})
        );
        for z in [4., 32., 256., 4096.] {
            for k in [
                R::Identity,
                R::Longitude,
                R::Meridional,
                R::ShortIdentity,
                R::EarlyIdentity,
                R::EarlyLongitude,
                R::FourierTranspose,
            ] {
                let (r, setup) = measure(|| W::new(&g, k));
                let mut w = match r {
                    Ok(w) => w,
                    Err(e) => {
                        assert!(
                            !case.starts_with("sphere")
                                && matches!(k, R::Longitude | R::Meridional | R::EarlyLongitude)
                        );
                        println!(
                            "EARLY_RUN {}",
                            serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":false,"reason":e.to_string(),"allocation":setup})
                        );
                        continue;
                    }
                };
                let (p, prepare) = measure(|| w.prepare(z));
                p.unwrap();
                assert_eq!(prepare.requests(), 0);
                let mut c = WorkCounters::default();
                let first = w.solve_into(&b, &mut out, &mut c);
                if let Err(e) = first {
                    println!(
                        "EARLY_RUN {}",
                        serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":false,"reason":e.to_string(),"allocation":setup,"prepare":prepare,"counters":c})
                    );
                    continue;
                }
                let mut c = WorkCounters::default();
                let (r, a) = measure(|| w.solve_into(&b, &mut out, &mut c));
                let r = r.unwrap();
                assert!(r.error_l1 <= 1e-8, "{case} {z} {k:?} {r:?}");
                let reference = out.clone();
                let mut times = vec![];
                for _ in 0..5 {
                    let t = std::time::Instant::now();
                    black_box(
                        w.solve_into(&b, &mut out, &mut WorkCounters::default())
                            .unwrap(),
                    );
                    times.push(t.elapsed().as_nanos());
                    assert_eq!(out, reference);
                }
                times.sort_unstable();
                println!(
                    "EARLY_RUN {}",
                    serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":true,"allocation":a,"setup":setup,"prepare":prepare,"report":r,"counters":c,"values":out,"median_ns":times[2],"samples_ns":times})
                );
                let old = out.clone();
                assert!(w.prepare(0.).is_err());
                assert!(
                    w.solve_into(&b, &mut out, &mut WorkCounters::default())
                        .is_err()
                );
                assert_eq!(out, old);
            }
            let stages = g.separable_stages(0.).unwrap();
            let (f, setup) = measure(|| stages.stage_solver(z));
            let mut f = f.unwrap();
            let areas = (0..n)
                .map(|i| g.cell_measure(i).unwrap())
                .collect::<Vec<_>>();
            let call = |f: &mut dyn FnMut(&mut [f64]) -> Result<(), vigilode::SolveError>,
                        out: &mut [f64],
                        scratch: &mut [Iv]| {
                for (i, v) in out.iter_mut().enumerate() {
                    *v = b[i] / areas[i];
                }
                f(out).unwrap();
                for (i, v) in out.iter_mut().enumerate() {
                    *v *= areas[i];
                }
                shifted_adjoint_residual_into(&g, z, &b, out, scratch).unwrap()
            };
            call(&mut f, &mut out, &mut scratch);
            let (r, a) = measure(|| call(&mut f, &mut out, &mut scratch));
            assert!(r.error_l1 <= 1e-8);
            let mut times = vec![];
            for _ in 0..5 {
                let t = std::time::Instant::now();
                black_box(call(&mut f, &mut out, &mut scratch));
                times.push(t.elapsed().as_nanos());
            }
            times.sort_unstable();
            println!(
                "EARLY_RUN {}",
                serde_json::json!({"case":case,"shift":z,"candidate":"Fourier","qualified":true,"allocation":a,"setup":setup,"report":r,"values":out,"median_ns":times[2],"samples_ns":times})
            );
        }
    }
}
#[test]
fn early_native_adjoint_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{ShiftedAdjointRoute as R, solve_spatial_adjoint_shifted};
    let o = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        h0: 0.001,
        dense_output: false,
        max_steps: 4096,
        ..Default::default()
    };
    let adaptive = AdaptiveStepConfig {
        rtol: o.rtol,
        atol: 1e-10,
        initial_step: o.h0,
        max_attempts: o.max_steps,
        ..Default::default()
    };
    let mut grids = early_grids();
    grids.push((
        "box2_strong",
        DiffusionGrid::periodic_box([8, 4], [8., 4.], [4096., 2048.]).unwrap(),
    ));
    for (case, g) in grids {
        println!(
            "EARLY_NATIVE_GRID {}",
            serde_json::json!({"case":case,"n":g.dimension(),"rows":(0..g.dimension()).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>()})
        );
        let n = g.dimension();
        let mut c = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
        let mut exact = vec![Iv::point(0.).unwrap(); n];
        let cert = w
            .certify_into(
                &c,
                &c,
                &c,
                DURATION,
                SpatialAdjointRoute::PoissonWideDecay,
                &mut exact,
            )
            .unwrap();
        assert!(cert.width_upper <= 1e-6);
        println!(
            "EARLY_DUAL {}",
            serde_json::json!({"case":case,"weights":c,"duration":DURATION,"enclosure":exact.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"work":cert})
        );
        let p = g.adjoint_problem_gather();
        for k in [
            NativeRoute::Dopri,
            NativeRoute::Paired,
            NativeRoute::Complex,
        ] {
            let (r, a) = measure(|| native(&g, &p, &c, &o, k, true));
            let mut samples = vec![];
            for _ in 0..3 {
                let s = std::time::Instant::now();
                let v = native(&g, &p, &c, &o, k, true);
                samples.push(s.elapsed().as_nanos());
                assert_eq!(v.values, r.values);
            }
            samples.sort_unstable();
            let elapsed = samples[1];
            let e = max_error(&exact, &r.values);
            assert!(e <= 1e-6);
            println!(
                "EARLY_IVP {}",
                serde_json::json!({"case":case,"candidate":k,"qualified":true,"allocation":a,"run":r,"error_upper":e,"median_ns":elapsed,"timing_repetitions":3,"samples_ns":samples})
            );
        }
        for k in [
            R::Identity,
            R::Longitude,
            R::Meridional,
            R::ShortIdentity,
            R::EarlyIdentity,
            R::EarlyLongitude,
            R::FourierTranspose,
        ] {
            if !case.starts_with("sphere")
                && matches!(k, R::Longitude | R::Meridional | R::EarlyLongitude)
            {
                continue;
            }
            let start = std::time::Instant::now();
            let (r, a) =
                measure(|| solve_spatial_adjoint_shifted(&g, (0., DURATION), &c, &adaptive, k));
            let mut elapsed = start.elapsed().as_nanos();
            let mut samples = vec![elapsed];
            if let Ok((v, _)) = &r {
                if v.observed.success {
                    for _ in 0..3 {
                        let s = std::time::Instant::now();
                        let (u, _) =
                            solve_spatial_adjoint_shifted(&g, (0., DURATION), &c, &adaptive, k)
                                .unwrap();
                        samples.push(s.elapsed().as_nanos());
                        assert_eq!(u.observed.y, v.observed.y);
                    }
                    samples.remove(0);
                    samples.sort_unstable();
                    elapsed = samples[1];
                }
            }
            match r {
                Ok((r, work)) => {
                    let complete = r.observed.success && r.observed.t.last() == Some(&DURATION);
                    let err = if complete {
                        Some(max_error(&exact, r.observed.y.last().unwrap()))
                    } else {
                        None
                    };
                    if complete {
                        assert!(err.unwrap() <= 1e-6);
                        assert_eq!(work.solves as u64, r.observed.counters.linear_solves);
                        assert_eq!(work.returned, work.solves);
                    }
                    println!(
                        "EARLY_IVP {}",
                        serde_json::json!({"case":case,"candidate":k,"qualified":complete,"allocation":a,"work":work,"run":{"values":r.observed.y.last().unwrap(),"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs":r.observed.counters.rhs_calls,"jvp":r.observed.counters.jvp_calls,"linear_solves":r.observed.counters.linear_solves,"linear_iterations":r.observed.counters.linear_iterations,"counters":r.observed.counters},"error_upper":err,"median_ns":elapsed,"timing_repetitions":samples.len(),"samples_ns":samples})
                    );
                }
                Err(e) => println!(
                    "EARLY_IVP {}",
                    serde_json::json!({"case":case,"candidate":k,"qualified":false,"allocation":a,"reason":e.to_string(),"median_ns":elapsed,"timing_repetitions":samples.len(),"samples_ns":samples})
                ),
            }
        }
    }
}

#[test]
fn fused_residual_fast_and_fallback() {
    use vigilode::spatial::{ResidualPlan as P, ResidualWorkspace as W};
    let g = DiffusionGrid::periodic_box([8, 4], [8., 4.], [4096., 2048.]).unwrap();
    let n = g.dimension();
    let x = vec![1.; n];
    let b = vec![4096.; n];
    let mut w = W::new(&g, P::Hybrid).unwrap();
    w.prepare(4096.).unwrap();
    let a = w.evaluate(&b, &x).unwrap();
    println!("RESIDUAL_RED_GREEN {:?}", a);
    assert!(a.admitted && a.fast_checks == 1 && a.interval_checks == 0);
    let z = 2f64.powi(-20);
    w.prepare(z).unwrap();
    let b = vec![z; n];
    let a = w.evaluate(&b, &x).unwrap();
    println!("FALLBACK_RED_GREEN {:?}", a);
    assert!(a.admitted && a.fast_checks == 1 && a.interval_checks == 1 && a.fallbacks == 1);
}
fn residual_grids() -> Vec<(&'static str, DiffusionGrid)> {
    let mut grids = early_grids();
    grids.push((
        "box2_strong",
        DiffusionGrid::periodic_box([8, 4], [8., 4.], [4096., 2048.]).unwrap(),
    ));
    grids.push((
        "sphere1024",
        DiffusionGrid::sphere(16, 64, 1., 0.04).unwrap(),
    ));
    grids
}
#[test]
fn fast_residual_actual_portfolio() {
    use rodas5p_core::WorkCounters;
    use vigilode::spatial::{
        ResidualPlan as P, ResidualWorkspace as W, ShiftedAdjointRoute as R,
        ShiftedAdjointWorkspace as SW,
    };
    let plans = [
        P::Legacy,
        P::PackedInterval,
        P::FusedEdges,
        P::FusedDiagonal,
        P::Hybrid,
    ];
    let mut grids = residual_grids();
    grids.push((
        "duplicate",
        DiffusionGrid::periodic_box([2, 2, 2], [2., 2., 2.], [1., 2., 4.]).unwrap(),
    ));
    for (case, g) in grids {
        let n = g.dimension();
        let base = (0..n)
            .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
            .collect::<Vec<_>>();
        let mut x = vec![0.; n];
        let mut solver = SW::new(&g, R::FourierTranspose).unwrap();
        solver.prepare(256.).unwrap();
        solver
            .solve_into(&base, &mut x, &mut WorkCounters::default())
            .unwrap();
        println!(
            "RES_GRID {}",
            serde_json::json!({"case":case,"n":n,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>()})
        );
        let mut workspaces = vec![];
        let mut setups = vec![];
        for plan in plans {
            let (r, a) = measure(|| W::new(&g, plan));
            workspaces.push(r.unwrap());
            setups.push(a);
        }
        for scenario in ["nominal", "perturbed", "cancellation", "subnormal", "large"] {
            let (z, b, v) = match scenario {
                "nominal" => (256., base.clone(), x.clone()),
                "perturbed" => {
                    let mut v = x.clone();
                    v[n / 2] += 2f64.powi(-20);
                    (256., base.clone(), v)
                }
                "cancellation" => {
                    let z = 2f64.powi(-20);
                    (z, vec![z; n], vec![1.; n])
                }
                "subnormal" => {
                    let eta = f64::from_bits(1);
                    (
                        0.5,
                        (0..n).map(|i| eta * ((i % 11) as f64 - 5.)).collect(),
                        (0..n).map(|i| eta * ((i % 7) as f64 - 3.)).collect(),
                    )
                }
                _ => (
                    256.,
                    base.iter().map(|v| v * 2f64.powi(800)).collect(),
                    x.iter().map(|v| v * 2f64.powi(800)).collect(),
                ),
            };
            println!(
                "RES_INPUT {}",
                serde_json::json!({"case":case,"scenario":scenario,"shift":z,"b":b,"x":v})
            );
            let mut preps = vec![];
            let mut receipts = vec![];
            for w in &mut workspaces {
                let (p, a) = measure(|| w.prepare(z));
                p.unwrap();
                assert_eq!(a.requests(), 0);
                preps.push(a);
                w.evaluate(&b, &v).unwrap();
                let (r, a) = measure(|| w.evaluate(&b, &v));
                let r = r.unwrap();
                assert_eq!(a.requests(), 0);
                receipts.push((r, a));
            }
            assert_eq!(receipts[0].0.residual_l1, receipts[1].0.residual_l1);
            let mut samples: [Vec<u128>; 5] = std::array::from_fn(|_| vec![]);
            for rep in 0..5 {
                for j in 0..5 {
                    let i = (rep + j) % 5;
                    let start = std::time::Instant::now();
                    for _ in 0..32 {
                        black_box(workspaces[i].evaluate(&b, &v).unwrap());
                    }
                    samples[i].push(start.elapsed().as_nanos() / 32);
                }
            }
            for (i, (r, a)) in receipts.into_iter().enumerate() {
                samples[i].sort_unstable();
                println!(
                    "RES_CERT {}",
                    serde_json::json!({"case":case,"scenario":scenario,"candidate":plans[i],"qualified":true,"receipt":r,"allocation":a,"setup":setups[i],"prepare":preps[i],"median_ns":samples[i][2],"samples_ns":samples[i]})
                );
            }
        }
    }
    let g = DiffusionGrid::periodic_box([2, 2], [2., 2.], [1., 1.]).unwrap();
    let n = g.dimension();
    for p in plans {
        let mut w = W::new(&g, p).unwrap();
        assert!(w.evaluate(&vec![1.; n], &vec![1.; n]).is_err());
        w.prepare(4.).unwrap();
        assert!(w.evaluate(&vec![f64::NAN; n], &vec![1.; n]).is_err());
        assert!(w.evaluate(&vec![1.; n], &vec![f64::MAX; n]).is_err());
        assert!(w.prepare(0.).is_err());
        assert!(w.evaluate(&vec![1.; n], &vec![1.; n]).is_err());
    }
    println!(
        "RES_EDGE {}",
        serde_json::json!({"overflow_refused":true,"nonfinite_refused":true,"invalidated_prepare":true})
    );
}
#[test]
fn fast_residual_native_actual_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        ResidualPlan as P, ShiftedAdjointRoute as R, solve_spatial_adjoint_shifted,
        solve_spatial_adjoint_shifted_with_residual,
    };
    let o = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-10),
        h0: 0.001,
        dense_output: false,
        max_steps: 4096,
        ..Default::default()
    };
    let adaptive = AdaptiveStepConfig {
        rtol: o.rtol,
        atol: 1e-10,
        initial_step: o.h0,
        max_attempts: o.max_steps,
        ..Default::default()
    };
    for (case, g) in residual_grids() {
        let n = g.dimension();
        let mut c = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
        let mut exact = vec![Iv::point(0.).unwrap(); n];
        let cert = w
            .certify_into(
                &c,
                &c,
                &c,
                DURATION,
                SpatialAdjointRoute::PoissonWideDecay,
                &mut exact,
            )
            .unwrap();
        assert!(cert.width_upper <= 1e-6);
        println!(
            "FAST_DUAL {}",
            serde_json::json!({"case":case,"n":n,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>(),"weights":c,"duration":DURATION,"enclosure":exact.iter().map(|v|v.as_pair()).collect::<Vec<_>>() })
        );
        let p = g.adjoint_problem_gather();
        for k in [NativeRoute::Dopri, NativeRoute::Paired] {
            let (r, a) = measure(|| native(&g, &p, &c, &o, k, true));
            let e = max_error(&exact, &r.values);
            assert!(e <= 1e-6);
            let mut samples = vec![];
            for _ in 0..3 {
                let start = std::time::Instant::now();
                let v = native(&g, &p, &c, &o, k, true);
                samples.push(start.elapsed().as_nanos());
                assert_eq!(v.values, r.values);
            }
            samples.sort_unstable();
            println!(
                "FAST_IVP {}",
                serde_json::json!({"case":case,"candidate":k,"qualified":true,"allocation":a,"run":r,"error_upper":e,"median_ns":samples[1],"samples_ns":samples})
            );
        }
        let plans = [
            P::Legacy,
            P::PackedInterval,
            P::FusedEdges,
            P::FusedDiagonal,
            P::Hybrid,
        ];
        let call =
            |k| solve_spatial_adjoint_shifted_with_residual(&g, (0., DURATION), &c, &adaptive, k);
        let mut receipts = vec![];
        for plan in plans {
            let (r, a) = measure(|| call(plan));
            receipts.push((r, a));
        }
        let mut samples: [Vec<u128>; 5] = std::array::from_fn(|_| vec![]);
        let mut profiles: [Vec<serde_json::Value>; 5] = std::array::from_fn(|_| vec![]);
        for rep in 0..3 {
            for j in 0..5 {
                let i = (rep + j) % 5;
                if let Ok((r, _)) = &receipts[i].0 {
                    if r.observed.success {
                        let start = std::time::Instant::now();
                        let (v, work) = call(plans[i]).unwrap();
                        samples[i].push(start.elapsed().as_nanos());
                        assert_eq!(v.observed.y, r.observed.y);
                        profiles[i].push(serde_json::to_value(work.residual_profile).unwrap());
                    }
                }
            }
        }
        let mut old = None;
        for (i, (r, a)) in receipts.into_iter().enumerate() {
            match r {
                Ok((r, work)) => {
                    let complete = r.observed.success && r.observed.t.last() == Some(&DURATION);
                    let e = if complete {
                        Some(max_error(&exact, r.observed.y.last().unwrap()))
                    } else {
                        None
                    };
                    if complete {
                        assert!(e.unwrap() <= 1e-6);
                        assert_eq!(work.solves as u64, r.observed.counters.linear_solves);
                        assert_eq!(work.returned, work.solves);
                        if i == 0 {
                            old = Some(r.observed.y.clone());
                        } else {
                            assert_eq!(old.as_ref(), Some(&r.observed.y));
                        }
                    }
                    samples[i].sort_unstable();
                    println!(
                        "FAST_IVP {}",
                        serde_json::json!({"case":case,"candidate":plans[i],"qualified":complete,"allocation":a,"work":work,"profiles":profiles[i],"run":{"values":r.observed.y.last().unwrap(),"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs":r.observed.counters.rhs_calls,"linear_solves":r.observed.counters.linear_solves,"linear_iterations":r.observed.counters.linear_iterations,"counters":r.observed.counters},"error_upper":e,"median_ns":samples[i].get(1),"samples_ns":samples[i]})
                    );
                }
                Err(e) => println!(
                    "FAST_IVP {}",
                    serde_json::json!({"case":case,"candidate":plans[i],"qualified":false,"allocation":a,"reason":e.to_string()})
                ),
            }
        }
        // Existing per-column and previously unused projected-trigger paths.
        // Separate comparison strata, not a new default.
        if matches!(case, "box2" | "sphere128" | "sphere512") {
            let routes = if case.starts_with("sphere") {
                vec![
                    R::EarlyIdentity,
                    R::TriggeredIdentity,
                    R::EarlyLongitude,
                    R::TriggeredLongitude,
                ]
            } else {
                vec![R::EarlyIdentity, R::TriggeredIdentity]
            };
            for k in routes {
                let call = || solve_spatial_adjoint_shifted(&g, (0., DURATION), &c, &adaptive, k);
                let (r, a) = measure(call);
                match r {
                    Ok((r, work)) => {
                        let complete = r.observed.success && r.observed.t.last() == Some(&DURATION);
                        let e = if complete {
                            Some(max_error(&exact, r.observed.y.last().unwrap()))
                        } else {
                            None
                        };
                        let mut samples = vec![];
                        if complete {
                            assert!(e.unwrap() <= 1e-6);
                            assert_eq!(work.solves as u64, r.observed.counters.linear_solves);
                            for _ in 0..3 {
                                let s = std::time::Instant::now();
                                let (v, _) = call().unwrap();
                                samples.push(s.elapsed().as_nanos());
                                assert_eq!(v.observed.y, r.observed.y);
                            }
                            samples.sort_unstable();
                        }
                        println!(
                            "TRIGGER_IVP {}",
                            serde_json::json!({"case":case,"candidate":k,"qualified":complete,"allocation":a,"work":work,"run":{"values":r.observed.y.last().unwrap(),"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs":r.observed.counters.rhs_calls,"linear_solves":r.observed.counters.linear_solves,"linear_iterations":r.observed.counters.linear_iterations,"counters":r.observed.counters},"error_upper":e,"median_ns":samples.get(1),"samples_ns":samples})
                        );
                    }
                    Err(e) => println!(
                        "TRIGGER_IVP {}",
                        serde_json::json!({"case":case,"candidate":k,"qualified":false,"allocation":a,"reason":e.to_string()})
                    ),
                }
            }
        }
    }
}
#[test]
fn triggered_fixed_actual_portfolio() {
    use rodas5p_core::WorkCounters;
    use vigilode::spatial::{ShiftedAdjointRoute as R, ShiftedAdjointWorkspace as W};
    for (case, g) in early_grids() {
        if !matches!(case, "box2" | "sphere128" | "sphere512") {
            continue;
        }
        let n = g.dimension();
        let b = (0..n)
            .map(|i| ((17 * i % 29) as f64 - 14.) / 32.)
            .collect::<Vec<_>>();
        let mut out = vec![0.; n];
        let routes = if case.starts_with("sphere") {
            vec![
                R::EarlyIdentity,
                R::TriggeredIdentity,
                R::EarlyLongitude,
                R::TriggeredLongitude,
            ]
        } else {
            vec![R::EarlyIdentity, R::TriggeredIdentity]
        };
        for z in [4., 4096.] {
            for &k in &routes {
                let mut w = W::new(&g, k).unwrap();
                w.prepare(z).unwrap();
                let first = w.solve_into(&b, &mut out, &mut WorkCounters::default());
                if let Err(e) = first {
                    println!(
                        "TRIGGER_FIXED {}",
                        serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":false,"reason":e.to_string()})
                    );
                    continue;
                }
                let mut c = WorkCounters::default();
                let (r, a) = measure(|| w.solve_into(&b, &mut out, &mut c));
                let r = r.unwrap();
                assert!(r.error_l1 <= 1e-8);
                let expected = out.clone();
                let mut samples = vec![];
                for _ in 0..5 {
                    let s = std::time::Instant::now();
                    black_box(
                        w.solve_into(&b, &mut out, &mut WorkCounters::default())
                            .unwrap(),
                    );
                    samples.push(s.elapsed().as_nanos());
                    assert_eq!(out, expected);
                }
                samples.sort_unstable();
                println!(
                    "TRIGGER_FIXED {}",
                    serde_json::json!({"case":case,"shift":z,"candidate":k,"qualified":true,"allocation":a,"report":r,"counters":c,"b":b,"values":out,"median_ns":samples[2],"samples_ns":samples})
                );
            }
        }
    }
}
