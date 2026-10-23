//! Reproduction driver. Controls preselected from the Python accuracy sweep;
//! runtime cannot consult the exact solution to choose its step count.
mod pde_kernels;
use pde_kernels::{Graph, Work, Workspace};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;
struct CountAlloc;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for CountAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: CountAlloc = CountAlloc;
fn run(
    g: &Graph,
    y: &[f64],
    out: &mut [f64],
    method: &str,
    control: f64,
    ws: &mut Workspace,
) -> Work {
    match method {
        "euler" => g.euler(y, out, 0.1, control as usize, ws),
        "rkl2" => g.rkl2(y, out, 0.1, control as usize, ws),
        "uniformization" => g.uniformization(y, out, 0.1, control, ws),
        _ => panic!("unknown method"),
    }
    .unwrap()
}
const CASES: &[(&str, &str, usize, usize, &str, f64)] = &[
    (
        "periodic2d_n16",
        "periodic",
        16,
        2,
        "euler",
        1.40800000000000000e+03,
    ),
    (
        "periodic2d_n16",
        "periodic",
        16,
        2,
        "rkl2",
        3.20000000000000000e+01,
    ),
    (
        "periodic2d_n16",
        "periodic",
        16,
        2,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "periodic2d_n32",
        "periodic",
        32,
        2,
        "euler",
        2.62400000000000000e+03,
    ),
    (
        "periodic2d_n32",
        "periodic",
        32,
        2,
        "rkl2",
        1.60000000000000000e+01,
    ),
    (
        "periodic2d_n32",
        "periodic",
        32,
        2,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "periodic2d_n8",
        "periodic",
        8,
        2,
        "euler",
        1.53600000000000000e+03,
    ),
    (
        "periodic2d_n8",
        "periodic",
        8,
        2,
        "rkl2",
        3.20000000000000000e+01,
    ),
    (
        "periodic2d_n8",
        "periodic",
        8,
        2,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "periodic3d_n12",
        "periodic",
        12,
        3,
        "euler",
        2.30400000000000000e+03,
    ),
    (
        "periodic3d_n12",
        "periodic",
        12,
        3,
        "rkl2",
        3.20000000000000000e+01,
    ),
    (
        "periodic3d_n12",
        "periodic",
        12,
        3,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "periodic3d_n4",
        "periodic",
        4,
        3,
        "euler",
        2.04800000000000000e+03,
    ),
    (
        "periodic3d_n4",
        "periodic",
        4,
        3,
        "rkl2",
        3.20000000000000000e+01,
    ),
    (
        "periodic3d_n4",
        "periodic",
        4,
        3,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "periodic3d_n8",
        "periodic",
        8,
        3,
        "euler",
        2.04800000000000000e+03,
    ),
    (
        "periodic3d_n8",
        "periodic",
        8,
        3,
        "rkl2",
        3.20000000000000000e+01,
    ),
    (
        "periodic3d_n8",
        "periodic",
        8,
        3,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "sphere_10x20",
        "sphere",
        10,
        20,
        "euler",
        3.52000000000000000e+02,
    ),
    (
        "sphere_10x20",
        "sphere",
        10,
        20,
        "rkl2",
        4.00000000000000000e+00,
    ),
    (
        "sphere_10x20",
        "sphere",
        10,
        20,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "sphere_16x32",
        "sphere",
        16,
        32,
        "euler",
        2.77000000000000000e+02,
    ),
    (
        "sphere_16x32",
        "sphere",
        16,
        32,
        "rkl2",
        4.00000000000000000e+00,
    ),
    (
        "sphere_16x32",
        "sphere",
        16,
        32,
        "uniformization",
        1.00000000000000005e-04,
    ),
    (
        "sphere_6x12",
        "sphere",
        6,
        12,
        "euler",
        2.24000000000000000e+02,
    ),
    (
        "sphere_6x12",
        "sphere",
        6,
        12,
        "rkl2",
        4.00000000000000000e+00,
    ),
    (
        "sphere_6x12",
        "sphere",
        6,
        12,
        "uniformization",
        1.00000000000000005e-04,
    ),
];
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let repeat = args
        .get(1)
        .map(|s| s.parse::<usize>().expect("integer repeat"))
        .unwrap_or(1);
    assert!((1..=1000).contains(&repeat));
    for &(name, family, n, d, method, control) in CASES {
        let (g, mode, rate) = if family == "sphere" {
            pde_kernels::sphere(n, d)
        } else {
            pde_kernels::periodic(n, d)
        };
        let y: Vec<_> = mode.iter().map(|x| 1. + 0.2 * x).collect();
        let exact: Vec<_> = mode
            .iter()
            .map(|x| 1. + 0.2 * (rate * 0.1).exp() * x)
            .collect();
        let mut out = vec![0.; g.len()];
        let mut ws = Workspace::new(g.len());
        run(&g, &y, &mut out, method, control, &mut ws);
        for rep in 0..repeat {
            ALLOCS.store(0, Ordering::Relaxed);
            COUNTING.store(true, Ordering::Relaxed);
            let start = Instant::now();
            let work = run(&g, &y, &mut out, method, control, &mut ws);
            let ns = start.elapsed().as_nanos();
            COUNTING.store(false, Ordering::Relaxed);
            let allocations = ALLOCS.load(Ordering::Relaxed);
            std::hint::black_box(&out);
            let err: Vec<_> = out.iter().zip(&exact).map(|(a, b)| a - b).collect();
            let relative_error = g.norm2(&err) / g.norm2(&exact);
            let drift = (g.mass(&out) - g.mass(&y)).abs() / g.mass(&y).abs();
            assert!(relative_error <= 1e-5 && drift < 1e-9 && allocations == 0);
            println!("{{\"case\":\"{}\",\"cells\":{},\"edges\":{},\"method\":\"{}\",\"control\":{},\"repeat\":{},\"duration_ns\":{},\"operator_actions\":{},\"macrosteps\":{},\"hot_allocations\":{},\"relative_weighted_l2_error\":{},\"relative_mass_drift\":{},\"minimum\":{},\"q_max_exit\":{}}}",name,g.len(),g.edges(),method,control,rep,ns,work.actions,work.macrosteps,allocations,relative_error,drift,out.iter().copied().fold(f64::INFINITY,f64::min),g.q());
        }
    }
}
