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

#[test]
fn compact_residual_actual_portfolio() {
    use rodas5p_core::WorkCounters;
    use vigilode::spatial::{
        ResidualPlan as P, ResidualWorkspace as W, ShiftedAdjointRoute as R,
        ShiftedAdjointWorkspace as SW,
    };
    let plans = [P::Legacy, P::Hybrid, P::CompactDiagonal, P::CompactHybrid];
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
            "COMPACT_GRID {}",
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
                "COMPACT_INPUT {}",
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
            for &(r, _) in &receipts[1..] {
                if r.interval_checks == 1 {
                    assert_eq!(r.residual_l1, receipts[0].0.residual_l1);
                }
            }
            assert_eq!(receipts[0].0.admitted, receipts[3].0.admitted);
            assert!(setups[3].requested_bytes < setups[1].requested_bytes);
            let mut samples: [Vec<u128>; 4] = std::array::from_fn(|_| vec![]);
            for rep in 0..5 {
                for j in 0..4 {
                    let i = (rep + j) % 4;
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
                    "COMPACT_CERT {}",
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
        "COMPACT_EDGE {}",
        serde_json::json!({"overflow_refused":true,"nonfinite_refused":true,"invalidated_prepare":true})
    );
}

#[test]
fn compact_residual_native_actual_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{ResidualPlan as P, solve_spatial_adjoint_shifted_with_residual};
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
            "COMPACT_DUAL {}",
            serde_json::json!({"case":case,"n":n,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>(),"weights":c,"duration":DURATION,"enclosure":exact.iter().map(|v|v.as_pair()).collect::<Vec<_>>() })
        );
        let plans = [P::Hybrid, P::CompactDiagonal, P::CompactHybrid];
        let call =
            |k| solve_spatial_adjoint_shifted_with_residual(&g, (0., DURATION), &c, &adaptive, k);
        let mut receipts = vec![];
        for plan in plans {
            let (r, a) = measure(|| call(plan));
            receipts.push((r, a));
        }
        let mut samples: [Vec<u128>; 3] = std::array::from_fn(|_| vec![]);
        let mut profiles: [Vec<serde_json::Value>; 3] = std::array::from_fn(|_| vec![]);
        for rep in 0..3 {
            for j in 0..3 {
                let i = (rep + j) % 3;
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
                        "COMPACT_IVP {}",
                        serde_json::json!({"case":case,"candidate":plans[i],"qualified":complete,"allocation":a,"work":work,"profiles":profiles[i],"run":{"values":r.observed.y.last().unwrap(),"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs":r.observed.counters.rhs_calls,"linear_solves":r.observed.counters.linear_solves,"linear_iterations":r.observed.counters.linear_iterations,"counters":r.observed.counters},"error_upper":e,"median_ns":samples[i].get(1),"samples_ns":samples[i]})
                    );
                }
                Err(e) => println!(
                    "COMPACT_IVP {}",
                    serde_json::json!({"case":case,"candidate":plans[i],"qualified":false,"allocation":a,"reason":e.to_string()})
                ),
            }
        }
    }
}

#[test]
fn ring_local_actual_portfolio() {
    use rodas5p_core::WorkCounters;
    use vigilode::spatial::{
        ResidualPlan as P, ResidualWorkspace as W, ShiftedAdjointRoute as R,
        ShiftedAdjointWorkspace as SW,
    };
    let plans = [
        P::Legacy,
        P::Hybrid,
        P::CompactDiagonal,
        P::CompactHybrid,
        P::RingLocalDiagonal,
        P::RingLocalHybrid,
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
            "COMPACT_GRID {}",
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
                "COMPACT_INPUT {}",
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
            for &(r, _) in &receipts[1..] {
                if r.interval_checks == 1 {
                    assert_eq!(r.residual_l1, receipts[0].0.residual_l1);
                }
            }
            assert_eq!(receipts[0].0.admitted, receipts[3].0.admitted);
            assert!(setups[3].requested_bytes < setups[1].requested_bytes);
            assert!(setups[4].requested_bytes < setups[2].requested_bytes);
            assert_eq!(receipts[4].0.residual_l1, receipts[2].0.residual_l1);
            assert_eq!(receipts[5].0.residual_l1, receipts[3].0.residual_l1);
            assert_eq!(receipts[5].0.admitted, receipts[3].0.admitted);
            let mut samples: [Vec<u128>; 6] = std::array::from_fn(|_| vec![]);
            for rep in 0..5 {
                for j in 0..6 {
                    let i = (rep + j) % 6;
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
                    "COMPACT_CERT {}",
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
        "COMPACT_EDGE {}",
        serde_json::json!({"overflow_refused":true,"nonfinite_refused":true,"invalidated_prepare":true})
    );
}

#[test]
fn ring_local_native_actual_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{ResidualPlan as P, solve_spatial_adjoint_shifted_with_residual};
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
            "COMPACT_DUAL {}",
            serde_json::json!({"case":case,"n":n,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>(),"weights":c,"duration":DURATION,"enclosure":exact.iter().map(|v|v.as_pair()).collect::<Vec<_>>() })
        );
        let plans = [
            P::Hybrid,
            P::CompactHybrid,
            P::RingLocalDiagonal,
            P::RingLocalHybrid,
        ];
        let call =
            |k| solve_spatial_adjoint_shifted_with_residual(&g, (0., DURATION), &c, &adaptive, k);
        let mut receipts = vec![];
        for plan in plans {
            let (r, a) = measure(|| call(plan));
            receipts.push((r, a));
        }
        let mut samples: [Vec<u128>; 4] = std::array::from_fn(|_| vec![]);
        let mut profiles: [Vec<serde_json::Value>; 4] = std::array::from_fn(|_| vec![]);
        for rep in 0..3 {
            for j in 0..4 {
                let i = (rep + j) % 4;
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
                        "COMPACT_IVP {}",
                        serde_json::json!({"case":case,"candidate":plans[i],"qualified":complete,"allocation":a,"work":work,"profiles":profiles[i],"run":{"values":r.observed.y.last().unwrap(),"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs":r.observed.counters.rhs_calls,"linear_solves":r.observed.counters.linear_solves,"linear_iterations":r.observed.counters.linear_iterations,"counters":r.observed.counters},"error_upper":e,"median_ns":samples[i].get(1),"samples_ns":samples[i]})
                    );
                }
                Err(e) => println!(
                    "COMPACT_IVP {}",
                    serde_json::json!({"case":case,"candidate":plans[i],"qualified":false,"allocation":a,"reason":e.to_string()})
                ),
            }
        }
    }
}

#[test]
fn reusable_spatial_native_contract() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        ResidualPlan, SpatialAdjointDriverWorkspace, solve_spatial_adjoint_shifted_with_residual,
    };
    let g = DiffusionGrid::sphere(8, 16, 1., 0.04).unwrap();
    let a = AdaptiveStepConfig {
        rtol: 1e-8,
        atol: 1e-10,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    let mut y = vec![0.; g.dimension()];
    y[0] = 0.5;
    let plan = ResidualPlan::RingLocalHybrid;
    let mut w = SpatialAdjointDriverWorkspace::new(&g, (0., DURATION), plan).unwrap();
    let ((old, _), before) = measure(|| {
        solve_spatial_adjoint_shifted_with_residual(&g, (0., DURATION), &y, &a, plan).unwrap()
    });
    w.solve(&y, &a).unwrap();
    let ((new, _), after) = measure(|| w.solve(&y, &a).unwrap());
    assert!(new.observed.success);
    assert_eq!(old.observed.y, new.observed.y);
    assert_eq!(old.observed.counters, new.observed.counters);
    assert!(after.requested_bytes < before.requested_bytes);
}

#[test]
fn reusable_spatial_native_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        ResidualPlan as P, SpatialAdjointDriverWorkspace as W,
        solve_spatial_adjoint_shifted_with_residual,
    };
    let a = AdaptiveStepConfig {
        rtol: 1e-8,
        atol: 1e-10,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    for (case, g) in residual_grids() {
        let n = g.dimension();
        let mut c = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        for plan in [P::Hybrid, P::RingLocalDiagonal, P::RingLocalHybrid] {
            let (w, setup) = measure(|| W::new(&g, (0., DURATION), plan));
            let mut w = w.unwrap();
            w.solve(&c, &a).unwrap(); // warm
            let partial = AdaptiveStepConfig {
                max_attempts: 1,
                ..a.clone()
            };
            let (p, _) = w.solve(&c, &partial).unwrap();
            assert!(!p.observed.success);
            assert!(w.solve(&c[..n - 1], &a).is_err());
            for trial in 0..2 {
                let v = if trial == 0 {
                    c.clone()
                } else {
                    c.iter().map(|&x| -0.5 * x).collect()
                };
                let ((old, ow), before) = measure(|| {
                    solve_spatial_adjoint_shifted_with_residual(&g, (0., DURATION), &v, &a, plan)
                        .unwrap()
                });
                let ((new, nw), after) = measure(|| w.solve(&v, &a).unwrap());
                assert!(old.observed.success && new.observed.success);
                assert_eq!(old.observed.t, new.observed.t);
                assert_eq!(old.observed.y, new.observed.y);
                assert_eq!(old.observed.counters, new.observed.counters);
                assert_eq!(
                    (
                        old.attempts,
                        old.accepted_steps,
                        old.rejected_steps,
                        old.jacobian_reuses
                    ),
                    (
                        new.attempts,
                        new.accepted_steps,
                        new.rejected_steps,
                        new.jacobian_reuses
                    )
                );
                assert_eq!(
                    (
                        ow.solves,
                        ow.returned,
                        ow.preparations,
                        ow.max_error_l1,
                        ow.max_residual_l1
                    ),
                    (
                        nw.solves,
                        nw.returned,
                        nw.preparations,
                        nw.max_error_l1,
                        nw.max_residual_l1
                    )
                );
                assert!(after.requested_bytes < before.requested_bytes);
                for (candidate, r, alloc) in [("Fresh", old, before), ("Reusable", new, after)] {
                    println!(
                        "REUSE_IVP {}",
                        serde_json::json!({"case":case,"trial":trial,"plan":plan,"candidate":candidate,"qualified":r.observed.success,"allocation":alloc,"setup":setup,"run":{"values":r.observed.y.last().unwrap(),"accepted":r.accepted_steps,"rejected":r.rejected_steps,"rhs":r.observed.counters.rhs_calls,"linear_solves":r.observed.counters.linear_solves,"linear_iterations":r.observed.counters.linear_iterations,"counters":r.observed.counters}})
                    );
                }
            }
        }
    }
}

#[test]
fn native_arena_zeroheap_contract() {
    use rodas5p_integrators::{AdaptiveStepConfig, FastArenaPlan, FastOutputArena, OutputSchedule};
    use vigilode::spatial::{ResidualPlan, SpatialAdjointDriverWorkspace};
    let g = DiffusionGrid::sphere(8, 16, 1., 0.04).unwrap();
    let n = g.dimension();
    let mut y = vec![0.; n];
    y[0] = 0.5;
    let span = (0., DURATION);
    let a = AdaptiveStepConfig {
        rtol: 1e-8,
        atol: 1e-10,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    let mut w =
        SpatialAdjointDriverWorkspace::new(&g, span, ResidualPlan::RingLocalHybrid).unwrap();
    let sched = OutputSchedule::new(vec![span.0, span.1]).unwrap();
    let mut arena = FastOutputArena::new(n, 2).unwrap();
    let (old, _) = w.solve(&y, &a).unwrap();
    w.solve_into(&y, &a, &sched, FastArenaPlan::Legacy, &mut arena)
        .unwrap();
    let ((r, _), alloc) = measure(|| {
        w.solve_into(&y, &a, &sched, FastArenaPlan::Legacy, &mut arena)
            .unwrap()
    });
    assert!(r.receipt.success);
    assert_eq!(r.states, old.observed.y.as_slice());
    assert_eq!(r.times, old.observed.t.as_slice());
    assert_eq!(r.receipt.counters, old.observed.counters);
    assert_eq!(alloc.requests(), 0);
}

#[test]
fn native_arena_actual_portfolio() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, DriverStopReason, FastArenaPlan as A, FastOutputArena, OutputSchedule,
    };
    use vigilode::spatial::{ResidualPlan as P, SpatialAdjointDriverWorkspace as W};
    let span = (0., DURATION);
    let base = AdaptiveStepConfig {
        rtol: 1e-8,
        atol: 1e-10,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    for (case, g) in residual_grids() {
        let n = g.dimension();
        let mut c = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        let schedules = [
            OutputSchedule::new(vec![span.0, span.1]).unwrap(),
            OutputSchedule::new(vec![span.0, span.1 / 4., span.1 / 2., span.1]).unwrap(),
        ];
        for plan in [P::Hybrid, P::RingLocalDiagonal, P::RingLocalHybrid] {
            let mut w = W::new(&g, span, plan).unwrap();
            let (arena, setup) = measure(|| FastOutputArena::new(n, 4).unwrap());
            let mut arena = arena;
            w.solve_into(&c, &base, &schedules[0], A::Legacy, &mut arena)
                .unwrap();
            let mut small = FastOutputArena::new(n, 1).unwrap();
            assert!(
                w.solve_into(&c, &base, &schedules[0], A::Legacy, &mut small)
                    .is_err()
            );
            assert!(
                w.solve_into(&c[..n - 1], &base, &schedules[0], A::Legacy, &mut arena)
                    .is_err()
            );
            for (schedule_index, schedule) in schedules.iter().enumerate() {
                for partial in [false, true] {
                    let a = if partial {
                        AdaptiveStepConfig {
                            max_attempts: 1,
                            ..base.clone()
                        }
                    } else {
                        base.clone()
                    };
                    let ((old, stop, ow), alloc) =
                        measure(|| w.solve_with_schedule(&c, &a, schedule).unwrap());
                    assert_eq!(old.observed.success, !partial);
                    let expected = stop
                        .as_ref()
                        .map(|x| x.y.as_slice())
                        .unwrap_or(old.observed.y.last().unwrap());
                    let expected_t = stop.as_ref().map(|x| x.t).unwrap_or(span.1);
                    let run = serde_json::json!({"times":old.observed.t,"states":old.observed.y,"last_state":expected,"last_t":expected_t,"success":old.observed.success,"stop":stop.as_ref().map(|x|x.reason),"last_failure":stop.as_ref().and_then(|x|x.last_failure),"counters":old.observed.counters,"attempts":old.attempts,"accepted":old.accepted_steps,"rejected":old.rejected_steps,"reuses":old.jacobian_reuses,"clipped":old.observed.output_clipped_steps,"solves":ow.solves,"returned":ow.returned,"preparations":ow.preparations,"max_error":ow.max_error_l1,"max_residual":ow.max_residual_l1});
                    println!(
                        "ARENA_IVP {}",
                        serde_json::json!({"case":case,"schedule":schedule_index,"partial":partial,"plan":plan,"candidate":"Owning","qualified":true,"allocation":alloc,"setup":setup,"run":run})
                    );
                    for candidate in [A::Legacy, A::Prevalidated] {
                        let ((r, nw), alloc) = measure(|| {
                            w.solve_into(&c, &a, schedule, candidate, &mut arena)
                                .unwrap()
                        });
                        assert_eq!(r.receipt.success, old.observed.success);
                        assert_eq!(r.times, old.observed.t.as_slice());
                        assert_eq!(r.states, old.observed.y.as_slice());
                        assert_eq!(r.last_state, expected);
                        assert_eq!(r.receipt.counters, old.observed.counters);
                        assert_eq!(
                            (
                                r.receipt.attempts,
                                r.receipt.accepted_steps,
                                r.receipt.rejected_steps,
                                r.receipt.jacobian_reuses,
                                r.receipt.output_clipped_steps
                            ),
                            (
                                old.attempts,
                                old.accepted_steps,
                                old.rejected_steps,
                                old.jacobian_reuses,
                                old.observed.output_clipped_steps
                            )
                        );
                        assert_eq!(r.receipt.last_t, expected_t);
                        assert_eq!(r.receipt.stop, stop.as_ref().map(|x| x.reason));
                        assert_eq!(
                            r.receipt.last_failure,
                            stop.as_ref().and_then(|x| x.last_failure)
                        );
                        if partial {
                            assert_eq!(r.receipt.stop, Some(DriverStopReason::AttemptBudget));
                        }
                        assert_eq!(
                            (
                                nw.solves,
                                nw.returned,
                                nw.preparations,
                                nw.max_error_l1,
                                nw.max_residual_l1
                            ),
                            (
                                ow.solves,
                                ow.returned,
                                ow.preparations,
                                ow.max_error_l1,
                                ow.max_residual_l1
                            )
                        );
                        assert_eq!(
                            alloc.requests(),
                            0,
                            "{case} {plan:?} {candidate:?} partial={partial}"
                        );
                        println!(
                            "ARENA_IVP {}",
                            serde_json::json!({"case":case,"schedule":schedule_index,"partial":partial,"plan":plan,"candidate":candidate,"qualified":true,"allocation":alloc,"setup":setup,"run":{"times":r.times,"states":r.states,"last_state":r.last_state,"last_t":r.receipt.last_t,"success":r.receipt.success,"stop":r.receipt.stop,"last_failure":r.receipt.last_failure,"counters":r.receipt.counters,"attempts":r.receipt.attempts,"accepted":r.receipt.accepted_steps,"rejected":r.receipt.rejected_steps,"reuses":r.receipt.jacobian_reuses,"clipped":r.receipt.output_clipped_steps,"solves":nw.solves,"returned":nw.returned,"preparations":nw.preparations,"max_error":nw.max_error_l1,"max_residual":nw.max_residual_l1}})
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn native_arena_generic_ode() {
    use rodas5p_core::{CoreResult, WorkCounters};
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan as A, FastOutputArena, OdeProblem, OutputSchedule,
        ReusableFastClientWorkspace, StageLinearSystem,
    };
    use std::sync::Arc;
    struct Diagonal {
        n: usize,
        shift: f64,
    }
    impl StageLinearSystem for Diagonal {
        fn dimension(&self) -> usize {
            self.n
        }
        fn linearize(
            &mut self,
            _: &OdeProblem,
            _: f64,
            _: &[f64],
            _: &mut WorkCounters,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn factor(&mut self, z: f64, c: &mut WorkCounters) -> CoreResult<()> {
            self.shift = z;
            c.direct_factorizations += 1;
            Ok(())
        }
        fn solve_in_place(&mut self, b: &mut [f64], c: &mut WorkCounters) -> CoreResult<()> {
            for (i, v) in b.iter_mut().enumerate() {
                *v /= self.shift + (i + 1) as f64;
            }
            c.direct_solve_calls += 1;
            Ok(())
        }
    }
    let a = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-12,
        initial_step: 0.005,
        max_attempts: 4096,
        ..Default::default()
    };
    for n in [1, 3, 32] {
        let p = OdeProblem::new(
            "diagonal decay",
            n,
            Arc::new(|_, y, o| {
                for (i, (v, w)) in y.iter().zip(o).enumerate() {
                    *w = -(i as f64 + 1.) * v;
                }
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(|_, _, v, out| {
                for (i, (x, w)) in v.iter().zip(out).enumerate() {
                    *w = -(i as f64 + 1.) * x;
                }
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap();
        let span = (0., 0.125);
        let schedule = OutputSchedule::new(vec![0., 0.03125, 0.0625, 0.125]).unwrap();
        let y = vec![1.; n];
        let atol = vec![a.atol; n];
        let mut w = ReusableFastClientWorkspace::new(Box::new(Diagonal { n, shift: 0. })).unwrap();
        let mut arena = FastOutputArena::new(n, 4).unwrap();
        let (old, _) = w.solve(&p, span, &y, &a, &schedule, &atol).unwrap();
        for candidate in [A::Legacy, A::Prevalidated] {
            let (r, alloc) = measure(|| {
                w.solve_into(&p, span, &y, &a, &schedule, &atol, candidate, &mut arena)
                    .unwrap()
            });
            assert_eq!(alloc.requests(), 0);
            assert!(r.receipt.success);
            assert_eq!(r.states, old.observed.y.as_slice());
            assert_eq!(r.receipt.counters, old.observed.counters);
            let error = r
                .last_state
                .iter()
                .enumerate()
                .map(|(i, v)| (v - (-((i + 1) as f64) * span.1).exp()).abs())
                .fold(0., f64::max);
            assert!(error <= 1e-8);
            println!(
                "ARENA_ODE {}",
                serde_json::json!({"case":n,"candidate":candidate,"qualified":true,"allocation":alloc,"max_error":error,"counters":r.receipt.counters})
            );
        }
    }
}

#[test]
fn flat_arena_native_contract() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan as A, FastFlatOutputArena, OutputSchedule,
    };
    use vigilode::spatial::{ResidualPlan, SpatialAdjointDriverWorkspace};
    let g = DiffusionGrid::sphere(8, 16, 1., 0.04).unwrap();
    let n = g.dimension();
    let mut y = vec![0.; n];
    y[0] = 0.5;
    let span = (0., DURATION);
    let a = AdaptiveStepConfig {
        rtol: 1e-8,
        atol: 1e-10,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    let schedule = OutputSchedule::new(vec![0., DURATION / 2., DURATION]).unwrap();
    let mut w =
        SpatialAdjointDriverWorkspace::new(&g, span, ResidualPlan::RingLocalHybrid).unwrap();
    let (old, _, _) = w.solve_with_schedule(&y, &a, &schedule).unwrap();
    let mut arena = FastFlatOutputArena::new(n, 3).unwrap();
    let ((r, _), alloc) = measure(|| {
        w.solve_flat_into(&y, &a, &schedule, A::Legacy, &mut arena)
            .unwrap()
    });
    assert!(r.receipt.success);
    assert_eq!(alloc.requests(), 0);
    assert_eq!(r.times, old.observed.t.as_slice());
    for (i, row) in old.observed.y.iter().enumerate() {
        assert_eq!(&r.states[i * n..(i + 1) * n], row);
    }
    assert_eq!(r.receipt.counters, old.observed.counters);
}

#[test]
fn flat_arena_actual_portfolio() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan as A, FastArenaReceipt, FastFlatOutputArena,
        FastOutputArena, OutputSchedule,
    };
    use vigilode::spatial::{ResidualPlan as P, SpatialAdjointDriverWorkspace as W};
    let span = (0., DURATION);
    let base = AdaptiveStepConfig {
        rtol: 1e-8,
        atol: 1e-10,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    let mut grids = residual_grids();
    grids.push((
        "holdout_box3",
        DiffusionGrid::periodic_box([16, 8, 8], [16., 8., 8.], [64., 32., 16.]).unwrap(),
    ));
    grids.push((
        "holdout_sphere2048",
        DiffusionGrid::sphere(32, 64, 1., 0.01).unwrap(),
    ));
    let candidates = [
        "RowLegacy",
        "RowPrevalidated",
        "FlatLegacy",
        "FlatPrevalidated",
    ];
    for (case, g) in grids {
        let n = g.dimension();
        let mut c = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        let schedules = [
            OutputSchedule::new(vec![0., DURATION]).unwrap(),
            OutputSchedule::new((0..=16).map(|i| DURATION * i as f64 / 16.).collect()).unwrap(),
        ];
        for plan in [P::Hybrid, P::RingLocalHybrid] {
            let mut w = W::new(&g, span, plan).unwrap();
            let (mut row, row_setup) = measure(|| FastOutputArena::new(n, 17).unwrap());
            let (mut flat, flat_setup) = measure(|| FastFlatOutputArena::new(n, 17).unwrap());
            w.solve_into(&c, &base, &schedules[0], A::Legacy, &mut row)
                .unwrap();
            w.solve_flat_into(&c, &base, &schedules[0], A::Legacy, &mut flat)
                .unwrap();
            let mut small = FastFlatOutputArena::new(n, 1).unwrap();
            assert!(
                w.solve_flat_into(&c, &base, &schedules[0], A::Legacy, &mut small)
                    .is_err()
            );
            assert!(
                w.solve_flat_into(&c[..n - 1], &base, &schedules[0], A::Legacy, &mut flat)
                    .is_err()
            );
            for (schedule_index, schedule) in schedules.iter().enumerate() {
                for partial in [false, true] {
                    let a = if partial {
                        AdaptiveStepConfig {
                            max_attempts: 1,
                            ..base.clone()
                        }
                    } else {
                        base.clone()
                    };
                    let (old, stop, ow) = w.solve_with_schedule(&c, &a, schedule).unwrap();
                    assert_eq!(old.observed.success, !partial);
                    let expected = stop
                        .as_ref()
                        .map(|x| x.y.as_slice())
                        .unwrap_or(old.observed.y.last().unwrap());
                    let expected_t = stop.as_ref().map(|x| x.t).unwrap_or(span.1);
                    let check =
                        |receipt: FastArenaReceipt,
                         times: &[f64],
                         last: &[f64],
                         nw: vigilode::spatial::ShiftedNativeWork| {
                            assert_eq!(receipt.success, old.observed.success);
                            assert_eq!(times, old.observed.t.as_slice());
                            assert_eq!(last, expected);
                            assert_eq!(receipt.counters, old.observed.counters);
                            assert_eq!(receipt.last_t, expected_t);
                            assert_eq!(receipt.stop, stop.as_ref().map(|x| x.reason));
                            assert_eq!(
                                receipt.last_failure,
                                stop.as_ref().and_then(|x| x.last_failure)
                            );
                            assert_eq!(
                                (
                                    receipt.attempts,
                                    receipt.accepted_steps,
                                    receipt.rejected_steps,
                                    receipt.jacobian_reuses,
                                    receipt.output_clipped_steps
                                ),
                                (
                                    old.attempts,
                                    old.accepted_steps,
                                    old.rejected_steps,
                                    old.jacobian_reuses,
                                    old.observed.output_clipped_steps
                                )
                            );
                            assert_eq!(
                                (
                                    nw.solves,
                                    nw.returned,
                                    nw.preparations,
                                    nw.max_error_l1,
                                    nw.max_residual_l1
                                ),
                                (
                                    ow.solves,
                                    ow.returned,
                                    ow.preparations,
                                    ow.max_error_l1,
                                    ow.max_residual_l1
                                )
                            );
                        };
                    let mut counts = [Counts::default(); 4];
                    let mut samples: [Vec<u128>; 4] =
                        std::array::from_fn(|_| Vec::with_capacity(8));
                    for (i, candidate) in [A::Legacy, A::Prevalidated].iter().enumerate() {
                        let ((r, nw), alloc) = measure(|| {
                            w.solve_into(&c, &a, schedule, *candidate, &mut row)
                                .unwrap()
                        });
                        check(r.receipt, r.times, r.last_state, nw);
                        assert_eq!(r.states, old.observed.y.as_slice());
                        assert_eq!(alloc.requests(), 0);
                        counts[i] = alloc;
                        let ((r, nw), alloc) = measure(|| {
                            w.solve_flat_into(&c, &a, schedule, *candidate, &mut flat)
                                .unwrap()
                        });
                        check(r.receipt, r.times, r.last_state, nw);
                        assert_eq!(r.states.len(), old.observed.y.len() * n);
                        for (v, expected) in r.states.chunks_exact(n).zip(&old.observed.y) {
                            assert_eq!(v, expected);
                        }
                        assert_eq!(alloc.requests(), 0);
                        counts[i + 2] = alloc;
                    }
                    // Balanced cyclic order: each candidate occupies each position twice.
                    for rep in 0..8 {
                        for k in 0..4 {
                            let i = (rep + k) % 4;
                            let controller = if i % 2 == 0 {
                                A::Legacy
                            } else {
                                A::Prevalidated
                            };
                            let start = std::time::Instant::now();
                            if i < 2 {
                                let (r, _) = w
                                    .solve_into(&c, &a, schedule, controller, &mut row)
                                    .unwrap();
                                black_box(r.last_state);
                                black_box(r.receipt);
                            } else {
                                let (r, _) = w
                                    .solve_flat_into(&c, &a, schedule, controller, &mut flat)
                                    .unwrap();
                                black_box(r.last_state);
                                black_box(r.receipt);
                            }
                            samples[i].push(start.elapsed().as_nanos());
                        }
                    }
                    let run = serde_json::json!({"times":old.observed.t,"states":old.observed.y,"last_state":expected,"last_t":expected_t,"success":old.observed.success,"stop":stop.as_ref().map(|x|x.reason),"last_failure":stop.as_ref().and_then(|x|x.last_failure),"counters":old.observed.counters,"attempts":old.attempts,"accepted":old.accepted_steps,"rejected":old.rejected_steps,"reuses":old.jacobian_reuses,"clipped":old.observed.output_clipped_steps,"solves":ow.solves,"returned":ow.returned,"preparations":ow.preparations,"max_error":ow.max_error_l1,"max_residual":ow.max_residual_l1});
                    for i in 0..4 {
                        let mut sorted = samples[i].clone();
                        sorted.sort_unstable();
                        println!(
                            "FLAT_IVP {}",
                            serde_json::json!({"case":case,"schedule":schedule_index,"partial":partial,"plan":plan,"candidate":candidates[i],"qualified":true,"allocation":counts[i],"setup":if i<2{row_setup}else{flat_setup},"median_ns":(sorted[3]+sorted[4])/2,"samples_ns":samples[i],"run":run})
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn flat_arena_generic_ode() {
    use rodas5p_core::{CoreResult, WorkCounters};
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan as A, FastFlatOutputArena, OdeProblem, OutputSchedule,
        ReusableFastClientWorkspace, StageLinearSystem,
    };
    use std::sync::Arc;
    struct Diagonal {
        n: usize,
        shift: f64,
    }
    impl StageLinearSystem for Diagonal {
        fn dimension(&self) -> usize {
            self.n
        }
        fn linearize(
            &mut self,
            _: &OdeProblem,
            _: f64,
            _: &[f64],
            _: &mut WorkCounters,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn factor(&mut self, z: f64, c: &mut WorkCounters) -> CoreResult<()> {
            self.shift = z;
            c.direct_factorizations += 1;
            Ok(())
        }
        fn solve_in_place(&mut self, b: &mut [f64], c: &mut WorkCounters) -> CoreResult<()> {
            for (i, v) in b.iter_mut().enumerate() {
                *v /= self.shift + (i + 1) as f64;
            }
            c.direct_solve_calls += 1;
            Ok(())
        }
    }
    let a = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-12,
        initial_step: 0.005,
        max_attempts: 4096,
        ..Default::default()
    };
    for n in [1, 3, 32] {
        let p = OdeProblem::new(
            "diagonal decay",
            n,
            Arc::new(|_, y, o| {
                for (i, (v, w)) in y.iter().zip(o).enumerate() {
                    *w = -(i as f64 + 1.) * v;
                }
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(|_, _, v, out| {
                for (i, (x, w)) in v.iter().zip(out).enumerate() {
                    *w = -(i as f64 + 1.) * x;
                }
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap();
        let span = (0., 0.125);
        let schedule = OutputSchedule::new(vec![0., 0.03125, 0.0625, 0.125]).unwrap();
        let y = vec![1.; n];
        let atol = vec![a.atol; n];
        let mut w = ReusableFastClientWorkspace::new(Box::new(Diagonal { n, shift: 0. })).unwrap();
        let mut arena = FastFlatOutputArena::new(n, 4).unwrap();
        let (old, _) = w.solve(&p, span, &y, &a, &schedule, &atol).unwrap();
        for candidate in [A::Legacy, A::Prevalidated] {
            let (r, alloc) = measure(|| {
                w.solve_flat_into(&p, span, &y, &a, &schedule, &atol, candidate, &mut arena)
                    .unwrap()
            });
            assert_eq!(alloc.requests(), 0);
            assert!(r.receipt.success);
            assert_eq!(r.states.len(), old.observed.y.len() * n);
            for (v, expected) in r.states.chunks_exact(n).zip(&old.observed.y) {
                assert_eq!(v, expected);
            }
            assert_eq!(r.receipt.counters, old.observed.counters);
            let error = r
                .last_state
                .iter()
                .enumerate()
                .map(|(i, v)| (v - (-((i + 1) as f64) * span.1).exp()).abs())
                .fold(0., f64::max);
            assert!(error <= 1e-8);
            println!(
                "FLAT_ODE {}",
                serde_json::json!({"case":n,"candidate":candidate,"qualified":true,"allocation":alloc,"max_error":error,"counters":r.receipt.counters})
            );
        }
    }
}

#[test]
fn shared_spatial_adjoint_contract() {
    use vigilode::spatial::{SpatialAdjointBatchWorkspace, SpatialBatchPlan};
    let g = DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap();
    let n = g.dimension();
    let c = vec![1.; n];
    let y = vec![0.25; 2 * n];
    let mut w = SpatialAdjointBatchWorkspace::new(&g, 2).unwrap();
    let mut out = vec![Iv::point(17.).unwrap(); n];
    let mut receipts = [vigilode::spatial::SpatialAdjointWork::default(); 2];
    let (r, a) = measure(|| {
        w.certify_batch_into(
            &c,
            &y,
            &y,
            0.125,
            SpatialAdjointRoute::ModeResidual,
            SpatialBatchPlan::SharedL1,
            &mut out,
            &mut receipts,
        )
    });
    assert_eq!(r.unwrap().actions, 1);
    assert_eq!(a.requests(), 0);
    assert!(
        receipts
            .iter()
            .all(|r| r.signed_goal_error[0] <= 0. && r.signed_goal_error[1] >= 0.)
    );
}

#[test]
fn shared_spatial_adjoint_actual_portfolio() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan, FastFlatOutputArena, OutputSchedule,
    };
    use vigilode::spatial::{
        ResidualPlan, SpatialAdjointBatchWorkspace as B, SpatialAdjointDriverWorkspace as W,
        SpatialAdjointRoute as R, SpatialBatchPlan as P,
    };
    let opts = Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-11),
        h0: 0.001,
        dense_output: false,
        ..Default::default()
    };
    let adaptive = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    let grids = [
        (
            "box2_small",
            DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap(),
        ),
        (
            "box3_small",
            DiffusionGrid::periodic_box([4, 4, 4], [4., 4., 4.], [0.125, 0.25, 0.0625]).unwrap(),
        ),
        ("sphere_odd", DiffusionGrid::sphere(5, 9, 1., 0.64).unwrap()),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap()),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap(),
        ),
    ];
    for (case, g) in grids {
        let n = g.dimension();
        let problem = g.problem(SpatialRows::Serial);
        let mut initial = vec![];
        let mut endpoint = vec![];
        for trial in 0..4 {
            let u = match trial {
                0 => vec![0.25; n],
                1 => (0..n).map(|i| ((17 * i % 29) as f64 - 14.) / 32.).collect(),
                2 => (0..n).map(|i| if i == n / 2 { 0.5 } else { 0. }).collect(),
                _ => (0..n).map(|i| ((13 * i % 17) as f64 - 8.) / 64.).collect(),
            };
            let r = native(&g, &problem, &u, &opts, NativeRoute::Paired, false);
            initial.extend(u);
            endpoint.extend(r.values);
        }
        println!(
            "BATCH_GRID {}",
            serde_json::json!({"case":case,"n":n,"duration":DURATION,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>(),"initial":initial,"endpoint":endpoint})
        );
        let mut w = B::new(&g, 4).unwrap();
        let mut direct = SpatialAdjointWorkspace::new(&g).unwrap();
        let mut native_w = W::new(&g, (0., DURATION), ResidualPlan::RingLocalHybrid).unwrap();
        let schedule = OutputSchedule::new(vec![0., DURATION]).unwrap();
        let mut arena = FastFlatOutputArena::new(n, 2).unwrap();
        let mut gradient = vec![Iv::point(0.).unwrap(); n];
        let mut base_grad = gradient.clone();
        for goal_name in ["mass", "local"] {
            let c = if goal_name == "mass" {
                (0..n)
                    .map(|i| g.cell_measure(i).unwrap())
                    .collect::<Vec<_>>()
            } else {
                let mut c = vec![0.; n];
                c[0] = 0.5;
                c[n / 2] = -0.25;
                c[n - 1] = 0.125;
                c
            };
            native_w
                .solve_flat_into(&c, &adaptive, &schedule, FastArenaPlan::Legacy, &mut arena)
                .unwrap();
            let ((adj, nw), alloc) = measure(|| {
                native_w
                    .solve_flat_into(&c, &adaptive, &schedule, FastArenaPlan::Legacy, &mut arena)
                    .unwrap()
            });
            assert!(adj.receipt.success);
            assert_eq!(alloc.requests(), 0);
            println!(
                "BATCH_NATIVE {}",
                serde_json::json!({"case":case,"goal":goal_name,"weights":c,"values":adj.last_state,"allocation":alloc,"counters":adj.receipt.counters,"work":nw})
            );
            for route in [R::PoissonAdaptive, R::PoissonWideDecay, R::ModeResidual] {
                let base = (0..4)
                    .map(|i| {
                        direct
                            .certify_into(
                                &c,
                                &initial[i * n..(i + 1) * n],
                                &endpoint[i * n..(i + 1) * n],
                                DURATION,
                                route,
                                &mut base_grad,
                            )
                            .unwrap()
                    })
                    .collect::<Vec<_>>();
                for k in [2, 4] {
                    for candidate in [P::Repeated, P::SharedL1, P::SharedComponent] {
                        let mut receipts =
                            vec![vigilode::spatial::SpatialAdjointWork::default(); k];
                        let (v, a) = measure(|| {
                            w.certify_batch_into(
                                &c,
                                &initial[..k * n],
                                &endpoint[..k * n],
                                DURATION,
                                route,
                                candidate,
                                &mut gradient,
                                &mut receipts,
                            )
                        });
                        let work = v.unwrap();
                        assert_eq!(a.requests(), 0);
                        assert_eq!(gradient, base_grad);
                        assert_eq!(
                            work.actions,
                            if candidate == P::Repeated {
                                base[..k].iter().map(|x| x.actions).sum()
                            } else {
                                base[0].actions
                            }
                        );
                        for (i, r) in receipts.iter().enumerate() {
                            if candidate != P::SharedComponent {
                                assert_eq!(r.signed_goal_error, base[i].signed_goal_error);
                                assert_eq!(r.exact_goal, base[i].exact_goal);
                            }
                            if i == 0 {
                                let exact = c.iter().sum::<f64>() * 0.25;
                                assert!(
                                    r.exact_goal[0] <= exact + 1e-12
                                        && r.exact_goal[1] >= exact - 1e-12
                                );
                            }
                        }
                        println!(
                            "BATCH_ADJOINT {}",
                            serde_json::json!({"case":case,"goal":goal_name,"route":route,"pairs":k,"candidate":candidate,"qualified":true,"allocation":a,"work":work,"gradient":gradient.iter().map(|x|x.as_pair()).collect::<Vec<_>>(),"receipts":receipts})
                        );
                    }
                }
            }
        }
        // Invalid late element is refused before changing caller outputs.
        let c = vec![1.; n];
        let mut bad_initial = initial.clone();
        bad_initial[4 * n - 1] = f64::NAN;
        let old = gradient.clone();
        let mut receipts = [vigilode::spatial::SpatialAdjointWork::default(); 4];
        assert!(
            w.certify_batch_into(
                &c,
                &bad_initial,
                &endpoint,
                DURATION,
                R::PoissonWideDecay,
                P::SharedL1,
                &mut gradient,
                &mut receipts
            )
            .is_err()
        );
        assert_eq!(gradient, old);
        assert!(receipts.iter().all(|v| v.actions == 0));
    }
}

#[test]
fn centered_batch_adjoint_contract() {
    use vigilode::spatial::{
        SpatialAdjointBatchWorkspace as B, SpatialAdjointRoute as R, SpatialBatchPlan as P,
    };
    let g = DiffusionGrid::periodic_box([4, 4], [4., 4.], [0.125, 0.25]).unwrap();
    let n = g.dimension();
    let mut c = vec![0.; n];
    c[0] = 1.;
    let u = vec![32.; n];
    let mut w = B::new(&g, 1).unwrap();
    let mut grad = vec![Iv::point(9.).unwrap(); n];
    let mut rec = vec![vigilode::spatial::SpatialAdjointWork::default(); 1];
    let (v, a) = measure(|| {
        w.certify_batch_budgeted_into(
            &c,
            &u,
            &u,
            DURATION,
            R::ModeResidual,
            P::SharedCenteredIntersect,
            1e-12,
            &mut grad,
            &mut rec,
        )
    });
    let v = v.unwrap();
    assert!(v.accepted);
    assert_eq!(v.accepted_pairs, 1);
    assert_eq!(v.work.actions, 1);
    assert_eq!(a.requests(), 0);
    assert_eq!(rec[0].signed_goal_error, [0., 0.]);
    let mut bad = u.clone();
    bad[0] += 1.;
    let old = grad.clone();
    let old_rec = rec[0].signed_goal_error;
    let v = w
        .certify_batch_budgeted_into(
            &c,
            &u,
            &bad,
            DURATION,
            R::ModeResidual,
            P::SharedCenteredIntersect,
            1e-12,
            &mut grad,
            &mut rec,
        )
        .unwrap();
    assert!(!v.accepted);
    assert_eq!(grad, old);
    assert_eq!(rec[0].signed_goal_error, old_rec);
}

#[test]
fn centered_batch_actual_portfolio() {
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan, FastFlatOutputArena, OutputSchedule,
    };
    use vigilode::spatial::{
        ResidualPlan, SpatialAdjointBatchWorkspace as B, SpatialAdjointDriverWorkspace as W,
        SpatialAdjointRoute as R, SpatialBatchPlan as P,
    };
    let opts = Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-11),
        h0: 0.001,
        dense_output: false,
        ..Default::default()
    };
    let adaptive = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        max_attempts: 4096,
        ..Default::default()
    };
    let grids = [
        (
            "box2_small",
            DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap(),
        ),
        (
            "box3_small",
            DiffusionGrid::periodic_box([4, 4, 4], [4., 4., 4.], [0.125, 0.25, 0.0625]).unwrap(),
        ),
        ("sphere_odd", DiffusionGrid::sphere(5, 9, 1., 0.64).unwrap()),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap()),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap(),
        ),
    ];
    for (case, g) in grids {
        let n = g.dimension();
        let problem = g.problem(SpatialRows::Serial);
        let mut initial = vec![];
        let mut endpoint = vec![];
        for trial in 0..4 {
            let u = match trial {
                0 => vec![32.; n],
                1 => vec![-16.; n],
                2 => (0..n)
                    .map(|i| 32. + ((17 * i % 29) as f64 - 14.) / 1024.)
                    .collect(),
                _ => (0..n)
                    .map(|i| if i == n / 2 { -15.5 } else { -16. })
                    .collect(),
            };
            let r = native(&g, &problem, &u, &opts, NativeRoute::Paired, false);
            initial.extend(u);
            endpoint.extend(r.values);
        }
        println!(
            "CENTER_GRID {}",
            serde_json::json!({"case":case,"n":n,"duration":DURATION,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>(),"initial":initial,"endpoint":endpoint})
        );
        let mut w = B::new(&g, 4).unwrap();
        let mut native_w = W::new(&g, (0., DURATION), ResidualPlan::RingLocalHybrid).unwrap();
        let schedule = OutputSchedule::new(vec![0., DURATION / 2., DURATION]).unwrap();
        let mut arena = FastFlatOutputArena::new(n, 3).unwrap();
        let mut gradient = vec![Iv::point(0.).unwrap(); n];
        for goal_name in ["mass", "local"] {
            let c = if goal_name == "mass" {
                (0..n)
                    .map(|i| g.cell_measure(i).unwrap())
                    .collect::<Vec<_>>()
            } else {
                let mut c = vec![0.; n];
                c[0] = 0.5;
                c[n / 2] = -0.25;
                c[n - 1] = 0.125;
                c
            };
            native_w
                .solve_flat_into(&c, &adaptive, &schedule, FastArenaPlan::Legacy, &mut arena)
                .unwrap();
            let ((adj, nw), alloc) = measure(|| {
                native_w
                    .solve_flat_into(&c, &adaptive, &schedule, FastArenaPlan::Legacy, &mut arena)
                    .unwrap()
            });
            assert!(adj.receipt.success);
            assert_eq!(alloc.requests(), 0);
            println!(
                "CENTER_NATIVE {}",
                serde_json::json!({"case":case,"goal":goal_name,"weights":c,"times":adj.times,"values":adj.states,"allocation":alloc,"counters":adj.receipt.counters,"work":nw})
            );
            for route in [R::ModeResidual, R::PoissonWideDecay] {
                for k in [2, 4] {
                    let mut old = None;
                    for candidate in [P::SharedL1, P::SharedCentered, P::SharedCenteredIntersect] {
                        let mut receipts =
                            vec![vigilode::spatial::SpatialAdjointWork::default(); k];
                        let (v, a) = measure(|| {
                            w.certify_batch_into(
                                &c,
                                &initial[..k * n],
                                &endpoint[..k * n],
                                DURATION,
                                route,
                                candidate,
                                &mut gradient,
                                &mut receipts,
                            )
                        });
                        let work = v.unwrap();
                        assert_eq!(a.requests(), 0);
                        if candidate == P::SharedL1 {
                            old = Some((gradient.clone(), receipts.clone()));
                        } else {
                            let (grad, prev) = old.as_ref().unwrap();
                            assert_eq!(&gradient, grad);
                            if candidate == P::SharedCenteredIntersect {
                                for (p, q) in prev.iter().zip(&receipts) {
                                    assert!(
                                        q.exact_goal[0] >= p.exact_goal[0]
                                            && q.exact_goal[1] <= p.exact_goal[1]
                                    );
                                }
                            }
                        }
                        println!(
                            "CENTER_BATCH {}",
                            serde_json::json!({"case":case,"goal":goal_name,"route":route,"pairs":k,"candidate":candidate,"qualified":true,"allocation":a,"work":work,"gradient":gradient.iter().map(|x|x.as_pair()).collect::<Vec<_>>(),"receipts":receipts})
                        );
                        for budget in [1e-8, 1e-10] {
                            let mut out = vec![vigilode::spatial::SpatialAdjointWork::default(); k];
                            let mut grad = vec![Iv::point(17.).unwrap(); n];
                            let (v, a) = measure(|| {
                                w.certify_batch_budgeted_into(
                                    &c,
                                    &initial[..k * n],
                                    &endpoint[..k * n],
                                    DURATION,
                                    route,
                                    candidate,
                                    budget,
                                    &mut grad,
                                    &mut out,
                                )
                            });
                            let b = v.unwrap();
                            assert_eq!(a.requests(), 0);
                            assert_eq!(
                                b.accepted_pairs,
                                receipts
                                    .iter()
                                    .filter(|r| r
                                        .signed_goal_error
                                        .iter()
                                        .all(|x| x.abs() <= budget))
                                    .count()
                            );
                            assert_eq!(b.accepted, b.accepted_pairs == k);
                            assert_eq!(b.work.actions, work.actions);
                            if b.accepted {
                                assert_eq!(grad, gradient);
                                assert!(
                                    out.iter()
                                        .zip(&receipts)
                                        .all(|(a, b)| a.signed_goal_error == b.signed_goal_error)
                                );
                            } else {
                                assert!(grad.iter().all(|v| *v == Iv::point(17.).unwrap()));
                                assert!(out.iter().all(|v| v.actions == 0));
                            }
                            println!(
                                "CENTER_BUDGET {}",
                                serde_json::json!({"case":case,"goal":goal_name,"route":route,"pairs":k,"candidate":candidate,"budget":budget,"qualified":b.accepted,"allocation":a,"work":b})
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn source_owned_primal_replay_contract() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialPrimalReplayPlan as P, SpatialPrimalReplayPolicy as Policy,
        SpatialPrimalReplayWorkspace as W,
    };
    let g = DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap();
    let n = g.dimension();
    let mut u = vec![-16.; n];
    u[n / 2] = -15.5;
    let mut c = vec![0.; n];
    c[0] = 0.5;
    c[n / 2] = -0.25;
    c[n - 1] = 0.125;
    let adaptive = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let mut w = W::new(&g, DURATION, 1).unwrap();
    let mut endpoint = vec![17.; n];
    let mut rec = [SpatialAdjointWork::default(); 1];
    let (r, a) = measure(|| {
        w.solve_budgeted_into(
            &u,
            &c,
            &adaptive,
            Policy {
                budget: 1e-10,
                max_replays: 3,
                min_scale: 1e-4,
                plan: P::CenteredSelective,
            },
            &mut endpoint,
            &mut rec,
        )
    });
    let r = r.unwrap();
    assert!(r.accepted);
    assert!(r.solves > 0);
    assert_eq!(a.requests(), 0);
    assert!(rec[0].signed_goal_error.iter().all(|v| v.abs() <= 1e-10));
    let old = endpoint.clone();
    let old_rec = rec[0].signed_goal_error;
    let r = w
        .solve_budgeted_into(
            &u,
            &c,
            &adaptive,
            Policy {
                budget: 1e-30,
                max_replays: 0,
                min_scale: 1e-4,
                plan: P::FixedSelective,
            },
            &mut endpoint,
            &mut rec,
        )
        .unwrap();
    assert!(!r.accepted);
    assert_eq!(endpoint, old);
    assert_eq!(rec[0].signed_goal_error, old_rec);
}

#[test]
fn source_owned_primal_replay_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialPrimalReplayPlan as P, SpatialPrimalReplayPolicy as Policy,
        SpatialPrimalReplayWorkspace as W,
    };
    let opts = Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-11),
        h0: 0.001,
        dense_output: false,
        ..Default::default()
    };
    let adaptive = AdaptiveStepConfig {
        rtol: opts.rtol,
        atol: 1e-11,
        initial_step: opts.h0,
        max_attempts: opts.max_steps,
        ..Default::default()
    };
    let grids = [
        (
            "box2_small",
            DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap(),
        ),
        (
            "box3_small",
            DiffusionGrid::periodic_box([4, 4, 4], [4., 4., 4.], [0.125, 0.25, 0.0625]).unwrap(),
        ),
        ("sphere_odd", DiffusionGrid::sphere(5, 9, 1., 0.64).unwrap()),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap()),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap(),
        ),
        (
            "holdout_sphere153",
            DiffusionGrid::sphere(9, 17, 1., 0.20).unwrap(),
        ),
    ];
    for (case, g) in grids {
        let n = g.dimension();
        let problem = g.problem(SpatialRows::Serial);
        let mut initial = vec![];
        let mut old_endpoint = vec![];
        for trial in 0..4 {
            let u = match trial {
                0 => vec![32.; n],
                1 => vec![-16.; n],
                2 => (0..n)
                    .map(|i| 32. + ((17 * i % 29) as f64 - 14.) / 1024.)
                    .collect(),
                _ => (0..n)
                    .map(|i| if i == n / 2 { -15.5 } else { -16. })
                    .collect(),
            };
            let r = native(&g, &problem, &u, &opts, NativeRoute::Paired, false);
            initial.extend(u);
            old_endpoint.extend(r.values);
        }
        println!(
            "REPLAY_GRID {}",
            serde_json::json!({"case":case,"n":n,"duration":DURATION,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>(),"initial":initial,"old_endpoint":old_endpoint})
        );
        let (w, setup) = measure(|| W::new(&g, DURATION, 4).unwrap());
        let mut w = w;
        println!(
            "REPLAY_SETUP {}",
            serde_json::json!({"case":case,"allocation":setup})
        );
        let mut endpoint = vec![17.; 4 * n];
        let mut rec = [SpatialAdjointWork::default(); 4];
        for goal in ["mass", "local"] {
            let c = if goal == "mass" {
                (0..n)
                    .map(|i| g.cell_measure(i).unwrap())
                    .collect::<Vec<_>>()
            } else {
                let mut c = vec![0.; n];
                c[0] = 0.5;
                c[n / 2] = -0.25;
                c[n - 1] = 0.125;
                c
            };
            for budget in [1e-8, 1e-10] {
                for (candidate, plan, replays) in [
                    ("Baseline", P::FixedSelective, 0),
                    ("FixedUniform", P::FixedUniform, 3),
                    ("FixedSelective", P::FixedSelective, 3),
                    ("GuidedSelective", P::GuidedSelective, 3),
                    ("CenteredSelective", P::CenteredSelective, 3),
                    ("CenteredFirst", P::CenteredFirst, 3),
                ] {
                    endpoint.fill(17.);
                    rec.fill(SpatialAdjointWork::default());
                    let (r, a) = measure(|| {
                        w.solve_budgeted_into(
                            &initial,
                            &c,
                            &adaptive,
                            Policy {
                                budget,
                                max_replays: replays,
                                min_scale: 1e-4,
                                plan,
                            },
                            &mut endpoint,
                            &mut rec,
                        )
                    });
                    let r = r.unwrap();
                    assert_eq!(a.requests(), 0);
                    assert_eq!(r.solves, 4 + r.replays);
                    assert_eq!(r.accepted, r.accepted_pairs == 4);
                    let pairs = (0..4).map(|i| w.last_pair(i).unwrap()).collect::<Vec<_>>();
                    if candidate == "Baseline" {
                        for (i, p) in pairs.iter().enumerate() {
                            assert_eq!(
                                p.endpoint,
                                &old_endpoint[i * n..(i + 1) * n],
                                "{case} old native baseline"
                            );
                        }
                    }
                    if r.accepted {
                        for (i, p) in pairs.iter().enumerate() {
                            assert!(p.accepted);
                            assert_eq!(p.endpoint, &endpoint[i * n..(i + 1) * n]);
                            assert_eq!(p.receipt.signed_goal_error, rec[i].signed_goal_error);
                        }
                    } else {
                        assert!(endpoint.iter().all(|v| *v == 17.));
                        assert!(rec.iter().all(|v| v.actions == 0));
                    }
                    println!(
                        "PRIMAL_REPLAY {}",
                        serde_json::json!({"case":case,"goal":goal,"weights":c,"budget":budget,"candidate":candidate,"qualified":r.accepted,"allocation":a,"work":r,"pairs":pairs})
                    );
                }
            }
        }
    }
}

#[test]
fn source_owned_primal_replay_floor_and_partial_contract() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialPrimalReplayPlan as P, SpatialPrimalReplayPolicy as Policy,
        SpatialPrimalReplayWorkspace as W,
    };
    let g = DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap();
    let n = g.dimension();
    let mut u = vec![-16.; n];
    u[n / 2] = -15.5;
    let mut c = vec![0.; n];
    c[0] = 0.5;
    c[n / 2] = -0.25;
    c[n - 1] = 0.125;
    let adaptive = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let mut w = W::new(&g, DURATION, 1).unwrap();
    let mut endpoint = vec![17.; n];
    let mut rec = [SpatialAdjointWork::default(); 1];
    let (r, a) = measure(|| {
        w.solve_budgeted_into(
            &u,
            &c,
            &adaptive,
            Policy {
                budget: 1e-30,
                max_replays: 3,
                min_scale: 1.,
                plan: P::GuidedSelective,
            },
            &mut endpoint,
            &mut rec,
        )
    });
    let r = r.unwrap();
    assert!(!r.accepted);
    assert_eq!(r.solves, 1);
    assert_eq!(r.floor_refusals, 1);
    assert_eq!(r.replays, 0);
    assert_eq!(a.requests(), 0);
    assert!(endpoint.iter().all(|v| *v == 17.));
    assert_eq!(rec[0].actions, 0);
    assert!(!w.last_pair(0).unwrap().accepted);
    println!(
        "REPLAY_EDGE {}",
        serde_json::json!({"case":"floor","allocation":a,"work":r})
    );
    let partial = AdaptiveStepConfig {
        max_attempts: 1,
        ..adaptive.clone()
    };
    let (r, a) = measure(|| {
        w.solve_budgeted_into(
            &u,
            &c,
            &partial,
            Policy {
                budget: 1e-10,
                max_replays: 0,
                min_scale: 1e-4,
                plan: P::FixedSelective,
            },
            &mut endpoint,
            &mut rec,
        )
    });
    let r = r.unwrap();
    assert!(!r.accepted);
    assert_eq!(r.solves, 1);
    assert_eq!(r.primal_stops, 1);
    assert_eq!(a.requests(), 0);
    assert!(endpoint.iter().all(|v| *v == 17.));
    assert_eq!(rec[0].actions, 0);
    assert!(w.last_pair(0).is_none());
    println!(
        "REPLAY_EDGE {}",
        serde_json::json!({"case":"partial","allocation":a,"work":r})
    );
    let mut raw = vec![1.; n];
    raw[0] = 1e-20;
    let policy = Policy {
        budget: 1e-7,
        max_replays: 0,
        min_scale: 1e-4,
        plan: P::CenteredFirst,
    };
    let (r, a) =
        measure(|| w.solve_budgeted_into(&raw, &c, &adaptive, policy, &mut endpoint, &mut rec));
    let r = r.unwrap();
    assert!(r.accepted);
    assert_eq!(r.centered_solves, 0);
    assert!(!w.last_pair(0).unwrap().centered);
    assert_eq!(a.requests(), 0);
    let old = endpoint.clone();
    let r0 = w
        .solve_budgeted_into(
            &raw,
            &c,
            &adaptive,
            Policy {
                plan: P::FixedSelective,
                ..policy
            },
            &mut endpoint,
            &mut rec,
        )
        .unwrap();
    assert!(r0.accepted);
    assert_eq!(endpoint, old);
    println!(
        "REPLAY_EDGE {}",
        serde_json::json!({"case":"inexact_shift_fallback","allocation":a,"work":r})
    );
}

#[test]
fn exact_cache_and_local_replay_contract() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialLocalReplayPlan as LP, SpatialLocalReplayPolicy as LPolicy,
        SpatialLocalReplayWorkspace as L, SpatialPrimalReplayPlan as P,
        SpatialPrimalReplayPolicy as Policy, SpatialPrimalReplayWorkspace as W,
        SpatialReplayCachePlan as K,
    };
    let g = DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap();
    let n = g.dimension();
    let u = vec![0.25; n];
    let mut c = vec![0.; n];
    c[0] = 1.;
    let adaptive = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let mut w = W::new(&g, DURATION, 1).unwrap();
    let p = Policy {
        budget: 1e-10,
        max_replays: 0,
        min_scale: 1e-4,
        plan: P::FixedSelective,
    };
    let mut endpoint = vec![17.; n];
    let mut rec = [SpatialAdjointWork::default(); 1];
    let (a, alloc) = measure(|| {
        w.solve_cached_budgeted_into(&u, &c, &adaptive, p, K::GoalOnly, &mut endpoint, &mut rec)
    });
    let a = a.unwrap();
    assert!(a.accepted);
    assert!(a.adjoint_actions > 0);
    assert_eq!(alloc.requests(), 0);
    let (b, alloc) = measure(|| {
        w.solve_cached_budgeted_into(&u, &c, &adaptive, p, K::GoalOnly, &mut endpoint, &mut rec)
    });
    let b = b.unwrap();
    assert!(b.accepted);
    assert_eq!(b.adjoint_actions, 0);
    assert!(b.cache_hit);
    assert_eq!(alloc.requests(), 0);
    let mut local = L::new(&g, DURATION, 128).unwrap();
    let (r, a) = measure(|| {
        local.solve_into(
            &u,
            &c,
            &adaptive,
            LPolicy {
                budget: 1e-10,
                max_replays: 1,
                min_scale: 1e-4,
                plan: LP::MaxMagnitude,
            },
            &mut endpoint,
            &mut rec[0],
        )
    });
    let r = r.unwrap();
    assert!(r.accepted);
    assert_eq!(a.requests(), 0);
    assert!(r.dense_packets > 0);
    assert_eq!(local.defects().len(), 4);
    assert!(local.defects().iter().all(|v| v.contains(0.)));
}

#[test]
fn cross_call_cache_and_local_actual_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialLocalReplayPlan as LP, SpatialLocalReplayPolicy as LPolicy,
        SpatialLocalReplayWorkspace as L, SpatialPrimalReplayPlan as P,
        SpatialPrimalReplayPolicy as Policy, SpatialPrimalReplayWorkspace as W,
        SpatialReplayCachePlan as K,
    };
    let base = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let grids = [
        (
            "box2_small",
            DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap(),
        ),
        (
            "box3_small",
            DiffusionGrid::periodic_box([4, 4, 4], [4., 4., 4.], [0.125, 0.25, 0.0625]).unwrap(),
        ),
        ("sphere_odd", DiffusionGrid::sphere(5, 9, 1., 0.64).unwrap()),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap()),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap(),
        ),
        (
            "holdout_sphere153",
            DiffusionGrid::sphere(9, 17, 1., 0.20).unwrap(),
        ),
    ];
    for (case, g) in grids {
        let n = g.dimension();
        println!(
            "LOCAL_GRID {}",
            serde_json::json!({"case":case,"n":n,"duration":DURATION,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>()})
        );
        let mut u = vec![];
        for trial in 0..4 {
            u.extend(match trial {
                0 => vec![32.; n],
                1 => vec![-16.; n],
                2 => (0..n)
                    .map(|i| 32. + ((17 * i % 29) as f64 - 14.) / 1024.)
                    .collect(),
                _ => (0..n)
                    .map(|i| if i == n / 2 { -15.5 } else { -16. })
                    .collect(),
            });
        }
        let mut c: Vec<f64> = vec![0.; n];
        c[0] = 0.5;
        c[n / 2] = -0.25;
        c[n - 1] = 0.125;
        let p = Policy {
            budget: 1e-10,
            max_replays: 3,
            min_scale: 1e-4,
            plan: P::GuidedSelective,
        };
        let plans = [K::Off, K::ExactBatch, K::GoalOnly];
        let mut workspaces: Vec<W> = plans
            .iter()
            .map(|_| W::new(&g, DURATION, 4).unwrap())
            .collect();
        let mut endpoint = vec![17.; 4 * n];
        let mut rec = [SpatialAdjointWork::default(); 4];
        let mut changed = u.clone();
        changed[3 * n + 1] = changed[3 * n + 1].next_up();
        let mut changed_goal = c.clone();
        changed_goal[0] = changed_goal[0].next_up();
        for seq in 0..6 {
            let (initial, goal) = match seq {
                3 => (&changed, &c),
                4 => (&u, &changed_goal),
                _ => (&u, &c),
            };
            let cfg = if seq == 2 {
                AdaptiveStepConfig {
                    rtol: base.rtol * 0.5,
                    atol: base.atol * 0.5,
                    ..base.clone()
                }
            } else {
                base.clone()
            };
            let mut old = None;
            for (j, cache) in plans.into_iter().enumerate() {
                endpoint.fill(17.);
                rec.fill(SpatialAdjointWork::default());
                let w = &mut workspaces[j];
                let (r, a) = measure(|| {
                    w.solve_cached_budgeted_into(
                        initial,
                        goal,
                        &cfg,
                        p,
                        cache,
                        &mut endpoint,
                        &mut rec,
                    )
                });
                let r = r.unwrap();
                assert_eq!(a.requests(), 0);
                let hit = match cache {
                    K::Off => false,
                    K::ExactBatch => seq == 1 || seq == 2,
                    K::GoalOnly => seq == 1 || seq == 2 || seq == 3,
                };
                assert_eq!(r.cache_hit, hit);
                assert_eq!(r.adjoint_actions == 0, hit);
                let pairs = (0..4).map(|i| w.last_pair(i).unwrap()).collect::<Vec<_>>();
                let actual = (
                    pairs
                        .iter()
                        .flat_map(|v| v.endpoint.iter().copied())
                        .collect::<Vec<_>>(),
                    pairs
                        .iter()
                        .map(|v| v.receipt.signed_goal_error)
                        .collect::<Vec<_>>(),
                    r.accepted,
                    r.counters,
                );
                if let Some(ref v) = old {
                    assert_eq!(&actual, v);
                } else {
                    old = Some(actual);
                }
                println!(
                    "CACHE_RUN {}",
                    serde_json::json!({"case":case,"seq":seq,"candidate":cache,"qualified":r.accepted,"initial":initial,"weights":goal,"budget":p.budget,"allocation":a,"work":r,"pairs":pairs})
                );
            }
        }
        // Same warm key for each path, cyclic paired execution, native plus cert.
        let mut samples: [Vec<u128>; 3] = std::array::from_fn(|_| Vec::with_capacity(6));
        for rep in 0..6 {
            for j in 0..3 {
                let i = (rep + j) % 3;
                let start = std::time::Instant::now();
                let r = workspaces[i]
                    .solve_cached_budgeted_into(&u, &c, &base, p, plans[i], &mut endpoint, &mut rec)
                    .unwrap();
                std::hint::black_box(r);
                samples[i].push(start.elapsed().as_nanos());
                assert!(r.accepted);
                assert_eq!(r.cache_hit, i != 0);
            }
        }
        for (i, cache) in plans.into_iter().enumerate() {
            let mut sorted = samples[i].clone();
            sorted.sort_unstable();
            println!(
                "CACHE_TIME {}",
                serde_json::json!({"case":case,"candidate":cache,"samples_ns":samples[i],"median_ns":(sorted[2]+sorted[3])/2})
            );
        }
        let (mut local, setup) = measure(|| L::new(&g, DURATION, 128).unwrap());
        let mut global = W::new(&g, DURATION, 1).unwrap();
        let mut output = vec![17.; n];
        let mut receipt = SpatialAdjointWork::default();
        println!(
            "LOCAL_SETUP {}",
            serde_json::json!({"case":case,"allocation":setup})
        );
        for goal_name in ["mass", "local"] {
            let goal = if goal_name == "mass" {
                (0..n)
                    .map(|i| g.cell_measure(i).unwrap())
                    .collect::<Vec<_>>()
            } else {
                c.clone()
            };
            for input in ["offset", "signed"] {
                let initial = if input == "offset" {
                    u[3 * n..4 * n].to_vec()
                } else {
                    (0..n).map(|i| ((13 * i % 17) as f64 - 8.) / 64.).collect()
                };
                for budget in [1e-10, 1e-11] {
                    output.fill(17.);
                    receipt = SpatialAdjointWork::default();
                    let (r, a) = measure(|| {
                        global.solve_budgeted_into(
                            &initial,
                            &goal,
                            &base,
                            Policy { budget, ..p },
                            &mut output,
                            std::slice::from_mut(&mut receipt),
                        )
                    });
                    let r = r.unwrap();
                    assert_eq!(a.requests(), 0);
                    let pair = global.last_pair(0).unwrap();
                    println!(
                        "GLOBAL_LOCAL {}",
                        serde_json::json!({"case":case,"goal":goal_name,"input":input,"budget":budget,"candidate":"GlobalGuided","initial":initial,"weights":goal,"qualified":r.accepted,"allocation":a,"work":r,"endpoint":pair.endpoint,"signed_goal_error":pair.receipt.signed_goal_error})
                    );
                    for (name, plan, rounds) in [
                        ("Segmented", LP::Uniform, 0),
                        ("Uniform", LP::Uniform, 3),
                        ("MaxMagnitude", LP::MaxMagnitude, 3),
                        ("SignedAligned", LP::SignedAligned, 3),
                        ("SuffixWeighted", LP::SuffixWeighted, 3),
                    ] {
                        output.fill(17.);
                        receipt = SpatialAdjointWork::default();
                        let (r, a) = measure(|| {
                            local.solve_into(
                                &initial,
                                &goal,
                                &base,
                                LPolicy {
                                    budget,
                                    max_replays: rounds,
                                    min_scale: 1e-4,
                                    plan,
                                },
                                &mut output,
                                &mut receipt,
                            )
                        });
                        let r = r.unwrap();
                        assert_eq!(a.requests(), 0);
                        assert!(r.completed, "{case} {name} capacity/native stop");
                        assert_eq!(r.solves, 4 + r.replayed_cells);
                        assert_eq!(local.defects().len(), 4);
                        if r.accepted {
                            assert_eq!(output, &local.states()[4 * n..]);
                            assert_eq!(receipt.signed_goal_error, r.signed_goal_error.unwrap());
                        } else {
                            assert!(output.iter().all(|&v| v == 17.));
                            assert_eq!(receipt.actions, 0);
                        }
                        let mut packet_count = 0;
                        let mut samples = vec![];
                        for k in 0..4 {
                            let seg = local.dense_cell(k).unwrap();
                            packet_count += seg.len();
                            assert_eq!(seg.state(0), &local.states()[k * n..(k + 1) * n]);
                            assert_eq!(
                                seg.state(seg.len()),
                                &local.states()[(k + 1) * n..(k + 2) * n]
                            );
                            // Actual captured native quartic, not a surrogate path.
                            for j in 0..seg.len() {
                                let coord = n / 2;
                                let (power, a) = measure(|| seg.power_component(j, coord).unwrap());
                                assert_eq!(a.requests(), 0);
                                assert!(power[0].contains(seg.state(j)[coord]));
                                let end = power
                                    .iter()
                                    .try_fold(Iv::point(0.).unwrap(), |a, b| a.add(*b))
                                    .unwrap();
                                assert!(end.contains(seg.state(j + 1)[coord]));
                                if j == 0 || j + 1 == seg.len() {
                                    samples.push(serde_json::json!({"cell":k,"step":j,"times":[seg.t_old(j),seg.t_new(j)],"y0":seg.state(j)[coord],"y1":seg.state(j+1)[coord],"stage":(0..8).map(|q|seg.stage_rows(j)[q*n+coord]).collect::<Vec<_>>(),"power":power.map(|v|v.as_pair())}));
                                }
                            }
                        }
                        let h = &rodas5p_core::rodas5p_coefficients().unwrap().dense_h;
                        let dense_h = (0..3)
                            .map(|i| (0..8).map(|j| h[(i, j)]).collect::<Vec<_>>())
                            .collect::<Vec<_>>();
                        println!(
                            "LOCAL_REPLAY {}",
                            serde_json::json!({"case":case,"goal":goal_name,"input":input,"budget":budget,"candidate":name,"initial":initial,"weights":goal,"qualified":r.accepted,"allocation":a,"work":r,"times":local.times(),"states":local.states(),"projections":local.projections().iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"defects":local.defects().iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"scales":local.scales(),"packet_count":packet_count,"dense_h":dense_h,"samples":samples})
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn local_capture_capacity_and_cache_edges() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialLocalReplayPlan as P, SpatialLocalReplayPolicy as Policy,
        SpatialLocalReplayWorkspace as L,
    };
    let g = DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap();
    let n = g.dimension();
    let mut u = vec![0.; n];
    u[0] = 1.;
    let mut c = vec![0.; n];
    c[0] = 1.;
    let base = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let mut local = L::new(&g, DURATION, 1).unwrap();
    let mut out = vec![17.; n];
    let mut rec = SpatialAdjointWork::default();
    let (r, a) = measure(|| {
        local.solve_into(
            &u,
            &c,
            &base,
            Policy {
                budget: 1e-10,
                max_replays: 0,
                min_scale: 1e-4,
                plan: P::MaxMagnitude,
            },
            &mut out,
            &mut rec,
        )
    });
    let r = r.unwrap();
    assert!(!r.accepted && !r.completed);
    assert_eq!(r.primal_stops, 1);
    assert_eq!(r.dense_packets, 1);
    assert_eq!(a.requests(), 0);
    assert!(out.iter().all(|v| *v == 17.));
    assert!(local.last_receipt().is_none());
    assert!(local.dense_cell(0).is_none());
    println!(
        "LOCAL_EDGE {}",
        serde_json::json!({"case":"capture_cap","allocation":a,"work":r})
    );
}

#[test]
fn bounded_dense_capture_generic_ode() {
    use rodas5p_core::{CoreResult, WorkCounters};
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan as A, FastDenseCapture, FastFlatOutputArena, OdeProblem,
        OutputSchedule, ReusableFastClientWorkspace, StageLinearSystem,
    };
    use std::sync::Arc;
    struct Diagonal {
        n: usize,
        shift: f64,
    }
    impl StageLinearSystem for Diagonal {
        fn dimension(&self) -> usize {
            self.n
        }
        fn linearize(
            &mut self,
            _: &OdeProblem,
            _: f64,
            _: &[f64],
            _: &mut WorkCounters,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn factor(&mut self, z: f64, c: &mut WorkCounters) -> CoreResult<()> {
            self.shift = z;
            c.direct_factorizations += 1;
            Ok(())
        }
        fn solve_in_place(&mut self, b: &mut [f64], c: &mut WorkCounters) -> CoreResult<()> {
            for (i, v) in b.iter_mut().enumerate() {
                *v /= self.shift + (i + 1) as f64;
            }
            c.direct_solve_calls += 1;
            Ok(())
        }
    }
    let a = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-12,
        initial_step: 0.005,
        max_attempts: 4096,
        ..Default::default()
    };
    for n in [1, 3, 32] {
        let p = OdeProblem::new(
            "diagonal decay",
            n,
            Arc::new(|_, y, o| {
                for (i, (v, w)) in y.iter().zip(o).enumerate() {
                    *w = -(i as f64 + 1.) * v;
                }
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(|_, _, v, out| {
                for (i, (x, w)) in v.iter().zip(out).enumerate() {
                    *w = -(i as f64 + 1.) * x;
                }
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap();
        let span = (0., 0.125);
        let schedule = OutputSchedule::new(vec![0., 0.03125, 0.0625, 0.125]).unwrap();
        let y = vec![1.; n];
        let atol = vec![a.atol; n];
        let mut w = ReusableFastClientWorkspace::new(Box::new(Diagonal { n, shift: 0. })).unwrap();
        let mut arena = FastFlatOutputArena::new(n, 4).unwrap();
        let mut capture = FastDenseCapture::new(n, 128).unwrap();
        let (old, _) = w.solve(&p, span, &y, &a, &schedule, &atol).unwrap();
        for candidate in [A::Legacy, A::Prevalidated] {
            let (r, alloc) = measure(|| {
                w.solve_flat_dense_into(
                    &p,
                    span,
                    &y,
                    &a,
                    &schedule,
                    &atol,
                    candidate,
                    &mut arena,
                    &mut capture,
                )
                .unwrap()
            });
            assert_eq!(alloc.requests(), 0);
            assert!(r.receipt.success);
            assert_eq!(r.states.len(), old.observed.y.len() * n);
            for (v, expected) in r.states.chunks_exact(n).zip(&old.observed.y) {
                assert_eq!(v, expected);
            }
            assert_eq!(r.receipt.counters, old.observed.counters);
            let seg = capture.segments();
            assert_eq!(seg.len(), r.receipt.accepted_steps);
            assert_eq!(seg.state(0), y);
            assert_eq!(seg.state(seg.len()), r.last_state);
            for j in 0..seg.len() {
                for i in 0..n {
                    let coeff = seg.power_component(j, i).unwrap();
                    assert!(coeff[0].contains(seg.state(j)[i]));
                    assert!(
                        coeff
                            .iter()
                            .try_fold(Iv::point(0.).unwrap(), |a, b| a.add(*b))
                            .unwrap()
                            .contains(seg.state(j + 1)[i])
                    );
                }
            }
            let error = r
                .last_state
                .iter()
                .enumerate()
                .map(|(i, v)| (v - (-((i + 1) as f64) * span.1).exp()).abs())
                .fold(0., f64::max);
            assert!(error <= 1e-8);
            println!(
                "DENSE_CAPTURE_ODE {}",
                serde_json::json!({"case":n,"candidate":candidate,"qualified":true,"allocation":alloc,"max_error":error,"packets":seg.len(),"counters":r.receipt.counters})
            );
        }
    }
}

#[test]
fn partial_suffix_accounting_regression() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialLocalReplayPlan as P, SpatialLocalReplayPolicy as Policy,
        SpatialLocalReplayWorkspace as L,
    };
    let g = DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap();
    let n = g.dimension();
    let mut u = vec![-16.; n];
    u[n / 2] = -15.5;
    let mut c = vec![0.; n];
    c[0] = 0.5;
    c[n / 2] = -0.25;
    c[n - 1] = 0.125;
    let base = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let mut probe = L::new(&g, DURATION, 128).unwrap();
    let mut out = vec![17.; n];
    let mut rec = SpatialAdjointWork::default();
    let p = Policy {
        budget: 1e-14,
        max_replays: 0,
        min_scale: 1e-4,
        plan: P::Uniform,
    };
    let r = probe
        .solve_into(&u, &c, &base, p, &mut out, &mut rec)
        .unwrap();
    assert!(r.completed && !r.accepted);
    let cap = (0..4)
        .map(|i| probe.dense_cell(i).unwrap().len())
        .max()
        .unwrap();
    let mut w = L::new(&g, DURATION, cap).unwrap();
    let (r, a) = measure(|| {
        w.solve_into(
            &u,
            &c,
            &base,
            Policy {
                max_replays: 3,
                ..p
            },
            &mut out,
            &mut rec,
        )
    });
    let r = r.unwrap();
    assert!(!r.completed && !r.accepted);
    assert_eq!(r.primal_stops, 1);
    assert!(r.replay_rounds > 0);
    assert_eq!(
        r.replayed_cells,
        r.solves - 4,
        "actual suffix IVPs, not planned cells"
    );
    assert_eq!(a.requests(), 0);
    assert!(out.iter().all(|&v| v == 17.));
    assert!(w.last_receipt().is_none());
    println!(
        "PARTIAL_SUFFIX {}",
        serde_json::json!({"cap":cap,"allocation":a,"work":r})
    );
}

#[test]
fn compact_boundary_replay_contract() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialBoundaryDualPlan as D, SpatialCompactPartition as P,
        SpatialCompactPolicy as Policy, SpatialCompactRefinement as R,
        SpatialCompactReplayWorkspace as W,
    };
    let g = DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap();
    let n = g.dimension();
    let mut u = vec![-16.; n];
    u[n / 2] = -15.5;
    let c = (0..n)
        .map(|i| g.cell_measure(i).unwrap())
        .collect::<Vec<_>>();
    let base = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let policy = Policy {
        budget: 1e-11,
        max_replays: 3,
        min_scale: 1e-4,
        partition: P::Accepted4,
        dual: D::StationaryChain,
        refinement: R::MultiMagnitude,
    };
    let mut w = W::new(&g, DURATION, 256).unwrap();
    let mut out = vec![17.; n];
    let mut rec = SpatialAdjointWork::default();
    let (r, a) = measure(|| w.solve_into(&u, &c, &base, policy, &mut out, &mut rec));
    let r = r.unwrap();
    assert!(r.accepted, "{r:?}");
    assert!(r.stationary_used);
    assert_eq!(r.adjoint_actions, 1);
    assert_eq!(a.requests(), 0);
    assert_eq!(w.states()[..n], u);
    assert_eq!(w.states()[(w.times().len() - 1) * n..], out);
    let first = out.clone();
    let old = rec.signed_goal_error;
    let (r, a) = measure(|| w.solve_into(&u, &c, &base, policy, &mut out, &mut rec));
    let r = r.unwrap();
    assert!(r.accepted && r.dual_cache_hit);
    assert_eq!(r.adjoint_actions, 0);
    assert_eq!(a.requests(), 0);
    assert_eq!(out, first);
    assert_eq!(rec.signed_goal_error, old);
    let mut g1 = c.clone();
    g1[0] = g1[0].next_up();
    let (r, a) = measure(|| w.solve_into(&u, &g1, &base, policy, &mut out, &mut rec));
    let r = r.unwrap();
    assert!(!r.dual_cache_hit);
    assert!(r.adjoint_actions > 0);
    assert_eq!(a.requests(), 0);
    let old = out.clone();
    let oldrec = rec.signed_goal_error;
    let mut cap = W::new(&g, DURATION, 1).unwrap();
    let (r, a) = measure(|| cap.solve_into(&u, &c, &base, policy, &mut out, &mut rec));
    let r = r.unwrap();
    assert!(!r.completed && !r.accepted);
    assert_eq!(r.primal_stops, 1);
    assert_eq!(r.accepted_packets, 1);
    assert_eq!(a.requests(), 0);
    assert_eq!(out, old);
    assert_eq!(rec.signed_goal_error, oldrec);
    assert!(cap.times().is_empty());
}

#[test]
fn compact_adaptive_partition_portfolio() {
    use rodas5p_integrators::AdaptiveStepConfig;
    use vigilode::spatial::{
        SpatialAdjointWork, SpatialBoundaryDualPlan as D, SpatialCompactPartition as P,
        SpatialCompactPolicy as Policy, SpatialCompactRefinement as R,
        SpatialCompactReplayWorkspace as W, SpatialLocalReplayWorkspace,
    };
    let base = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-11,
        initial_step: 0.001,
        ..Default::default()
    };
    let grids = [
        (
            "box2_small",
            DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap(),
        ),
        (
            "box3_small",
            DiffusionGrid::periodic_box([4, 4, 4], [4., 4., 4.], [0.125, 0.25, 0.0625]).unwrap(),
        ),
        ("sphere_odd", DiffusionGrid::sphere(5, 9, 1., 0.64).unwrap()),
        ("sphere128", DiffusionGrid::sphere(8, 16, 1., 0.64).unwrap()),
        (
            "sphere512",
            DiffusionGrid::sphere(16, 32, 1., 0.16).unwrap(),
        ),
        (
            "holdout_sphere153",
            DiffusionGrid::sphere(9, 17, 1., 0.20).unwrap(),
        ),
    ];
    let plans = [
        (
            "Fixed4IndependentUniform",
            P::Fixed4,
            D::Independent,
            R::Uniform,
            3,
        ),
        (
            "Fixed4ChainMulti",
            P::Fixed4,
            D::Chain,
            R::MultiMagnitude,
            3,
        ),
        (
            "Accepted2ChainMulti",
            P::Accepted2,
            D::Chain,
            R::MultiMagnitude,
            3,
        ),
        (
            "Accepted4IndependentMulti",
            P::Accepted4,
            D::Independent,
            R::MultiMagnitude,
            3,
        ),
        (
            "Accepted4ChainMulti",
            P::Accepted4,
            D::Chain,
            R::MultiMagnitude,
            3,
        ),
        (
            "Accepted8ChainMulti",
            P::Accepted8,
            D::Chain,
            R::MultiMagnitude,
            3,
        ),
        (
            "Accepted4StationaryMulti",
            P::Accepted4,
            D::StationaryChain,
            R::MultiMagnitude,
            3,
        ),
        (
            "Accepted4StationaryCost",
            P::Accepted4,
            D::StationaryChain,
            R::MultiCost,
            3,
        ),
        (
            "Accepted4StationarySingle",
            P::Accepted4,
            D::StationaryChain,
            R::Single,
            3,
        ),
        (
            "Accepted4Pilot",
            P::Accepted4,
            D::StationaryChain,
            R::MultiMagnitude,
            0,
        ),
        (
            "Fixed4Pilot",
            P::Fixed4,
            D::StationaryChain,
            R::MultiMagnitude,
            0,
        ),
        (
            "Accepted2Pilot",
            P::Accepted2,
            D::StationaryChain,
            R::MultiMagnitude,
            0,
        ),
        (
            "Accepted8Pilot",
            P::Accepted8,
            D::StationaryChain,
            R::MultiMagnitude,
            0,
        ),
    ];
    for (case, g) in grids {
        let n = g.dimension();
        println!(
            "COMPACT_GRID {}",
            serde_json::json!({"case":case,"n":n,"duration":DURATION,"rows":(0..n).map(|i|g.represented_row(i).unwrap()).collect::<Vec<_>>()})
        );
        let (legacy, legacy_setup) =
            measure(|| SpatialLocalReplayWorkspace::new(&g, DURATION, 128).unwrap());
        let (mut w, setup) = measure(|| W::new(&g, DURATION, 256).unwrap());
        drop(legacy);
        println!(
            "COMPACT_SETUP {}",
            serde_json::json!({"case":case,"allocation":setup,"legacy_allocation":legacy_setup})
        );
        let mut endpoint = vec![17.; n];
        let mut rec = SpatialAdjointWork::default();
        for goal in ["mass", "local"] {
            let c = if goal == "mass" {
                (0..n)
                    .map(|i| g.cell_measure(i).unwrap())
                    .collect::<Vec<_>>()
            } else {
                let mut c = vec![0.; n];
                c[0] = 0.5;
                c[n / 2] = -0.25;
                c[n - 1] = 0.125;
                c
            };
            for input in ["offset", "signed"] {
                let u = if input == "offset" {
                    let mut u = vec![-16.; n];
                    u[n / 2] = -15.5;
                    u
                } else {
                    (0..n).map(|i| ((13 * i % 17) as f64 - 8.) / 64.).collect()
                };
                for budget in [1e-10, 1e-11] {
                    for (candidate, partition, dual, refinement, max_replays) in plans {
                        endpoint.fill(17.);
                        rec = SpatialAdjointWork::default();
                        let policy = Policy {
                            budget,
                            max_replays,
                            min_scale: 1e-4,
                            partition,
                            dual,
                            refinement,
                        };
                        let (r, a) = measure(|| {
                            w.solve_into(&u, &c, &base, policy, &mut endpoint, &mut rec)
                        });
                        let r = r.unwrap();
                        assert_eq!(a.requests(), 0);
                        assert!(r.completed, "{case} {candidate} {r:?}");
                        assert_eq!(r.solves, r.initial_solves + r.replayed_cells);
                        assert_eq!(w.times().len(), r.cells + 1);
                        assert_eq!(w.defects().len(), r.cells);
                        assert_eq!(w.states()[..n], u);
                        if r.accepted {
                            assert_eq!(endpoint, w.states()[r.cells * n..]);
                            assert_eq!(rec.signed_goal_error, r.signed_goal_error.unwrap());
                        } else {
                            assert!(endpoint.iter().all(|v| *v == 17.));
                            assert_eq!(rec.actions, 0);
                        }
                        let first = (
                            r,
                            w.times().to_vec(),
                            w.states().to_vec(),
                            w.projections().to_vec(),
                            w.defects().to_vec(),
                        );
                        // Recompute native trajectory, but reuse exact-key boundary cache.
                        let (warm, allocation) = measure(|| {
                            w.solve_into(&u, &c, &base, policy, &mut endpoint, &mut rec)
                        });
                        let warm = warm.unwrap();
                        assert_eq!(allocation.requests(), 0);
                        assert!(warm.dual_cache_hit);
                        assert_eq!(warm.adjoint_actions, 0);
                        assert_eq!(warm.accepted, r.accepted);
                        assert_eq!(warm.signed_goal_error, r.signed_goal_error);
                        assert_eq!(warm.counters, r.counters);
                        assert_eq!(warm.solves, r.solves);
                        assert_eq!(w.times(), first.1);
                        assert_eq!(w.states(), first.2);
                        assert_eq!(w.projections(), first.3);
                        assert_eq!(w.defects(), first.4);
                        println!(
                            "COMPACT_RUN {}",
                            serde_json::json!({"case":case,"goal":goal,"input":input,"budget":budget,"candidate":candidate,"partition":partition,"dual":dual,"refinement":refinement,"qualified":r.accepted,"allocation":a,"warm_allocation":allocation,"work":r,"warm_work":warm,"initial":u,"weights":c,"times":w.times(),"states":w.states(),"anchor":w.anchor(),"projections":w.projections().iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"defects":w.defects().iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"scales":w.scales(),"pilot_times":w.pilot_times(),"pilot_indices":w.pilot_indices(),"receipt":w.last_receipt().unwrap()})
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn bounded_checkpoint_capture_generic_ode() {
    use rodas5p_core::{CoreResult, WorkCounters};
    use rodas5p_integrators::{
        AdaptiveStepConfig, FastArenaPlan as A, FastCheckpointCapture, FastFlatOutputArena,
        OdeProblem, OutputSchedule, ReusableFastClientWorkspace, StageLinearSystem,
    };
    use std::sync::Arc;
    struct Diagonal {
        n: usize,
        shift: f64,
    }
    impl StageLinearSystem for Diagonal {
        fn dimension(&self) -> usize {
            self.n
        }
        fn linearize(
            &mut self,
            _: &OdeProblem,
            _: f64,
            _: &[f64],
            _: &mut WorkCounters,
        ) -> CoreResult<()> {
            Ok(())
        }
        fn factor(&mut self, z: f64, c: &mut WorkCounters) -> CoreResult<()> {
            self.shift = z;
            c.direct_factorizations += 1;
            Ok(())
        }
        fn solve_in_place(&mut self, b: &mut [f64], c: &mut WorkCounters) -> CoreResult<()> {
            for (i, v) in b.iter_mut().enumerate() {
                *v /= self.shift + (i + 1) as f64;
            }
            c.direct_solve_calls += 1;
            Ok(())
        }
    }
    let a = AdaptiveStepConfig {
        rtol: 1e-9,
        atol: 1e-12,
        initial_step: 0.005,
        max_attempts: 4096,
        ..Default::default()
    };
    for n in [1, 3, 32] {
        let p = OdeProblem::new(
            "diagonal decay",
            n,
            Arc::new(|_, y, o| {
                for (i, (v, w)) in y.iter().zip(o).enumerate() {
                    *w = -(i as f64 + 1.) * v;
                }
                Ok(())
            }),
            None,
            None,
            Some(Arc::new(|_, _, v, out| {
                for (i, (x, w)) in v.iter().zip(out).enumerate() {
                    *w = -(i as f64 + 1.) * x;
                }
                Ok(())
            })),
            None,
            true,
            None,
            None,
        )
        .unwrap();
        let span = (0., 0.125);
        let schedule = OutputSchedule::new(vec![0., 0.03125, 0.0625, 0.125]).unwrap();
        let y = vec![1.; n];
        let atol = vec![a.atol; n];
        let mut w = ReusableFastClientWorkspace::new(Box::new(Diagonal { n, shift: 0. })).unwrap();
        let mut arena = FastFlatOutputArena::new(n, 4).unwrap();
        let mut capture = FastCheckpointCapture::new(n, 128).unwrap();
        let (old, _) = w.solve(&p, span, &y, &a, &schedule, &atol).unwrap();
        for candidate in [A::Legacy, A::Prevalidated] {
            let (r, alloc) = measure(|| {
                w.solve_flat_checkpoints_into(
                    &p,
                    span,
                    &y,
                    &a,
                    &schedule,
                    &atol,
                    candidate,
                    &mut arena,
                    &mut capture,
                )
                .unwrap()
            });
            assert_eq!(alloc.requests(), 0);
            assert!(r.receipt.success);
            assert_eq!(r.states.len(), old.observed.y.len() * n);
            for (v, expected) in r.states.chunks_exact(n).zip(&old.observed.y) {
                assert_eq!(v, expected);
            }
            assert_eq!(r.receipt.counters, old.observed.counters);
            let seg = &capture;
            assert_eq!(seg.len(), r.receipt.accepted_steps);
            assert_eq!(seg.state(0), Some(y.as_slice()));
            assert_eq!(seg.state(seg.len()), Some(r.last_state));
            assert!(seg.times().windows(2).all(|x| x[0] < x[1]));
            let error = r
                .last_state
                .iter()
                .enumerate()
                .map(|(i, v)| (v - (-((i + 1) as f64) * span.1).exp()).abs())
                .fold(0., f64::max);
            assert!(error <= 1e-8);
            println!(
                "CHECKPOINT_ODE {}",
                serde_json::json!({"case":n,"candidate":candidate,"qualified":true,"allocation":alloc,"max_error":error,"packets":seg.len(),"counters":r.receipt.counters})
            );
        }
    }
}
