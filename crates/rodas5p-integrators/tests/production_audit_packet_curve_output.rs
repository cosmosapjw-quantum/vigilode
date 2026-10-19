//! Focused T1 transport checks for the same frozen represented curve.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::Write;
use std::process::{Command, Stdio};

use rodas5p_integrators::packet_window::{PacketCurveOutput, PacketError, RepresentedPacketCurve};
use rodas5p_integrators::{
    CausalWeights, CollectiveModel, PacketWorkCounters, compile_packet, generate_candidate,
};

thread_local! {
    static COUNT_ALLOC: Cell<bool> = const { Cell::new(false) };
    static ALLOC_COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT_ALLOC.with(|flag| {
            if flag.get() {
                ALLOC_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        COUNT_ALLOC.with(|flag| {
            if flag.get() {
                ALLOC_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNT_ALLOC.with(|flag| {
            if flag.get() {
                ALLOC_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn output(model: &CollectiveModel, horizon: f64) -> (PacketCurveOutput, PacketWorkCounters) {
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(model, horizon, 4, &mut work).unwrap();
    let weights = CausalWeights::compile(4, 12);
    let candidate = generate_candidate(&packet, 4, 2, &weights, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
    (PacketCurveOutput::new(curve), work)
}

fn oracle_case(
    view: &rodas5p_integrators::packet_window::PacketCurveOutputView<'_>,
    width: f64,
    xi: f64,
    chi: f64,
    weights: &[f64],
    initial: &[(f64, f64)],
) -> serde_json::Value {
    serde_json::json!({
        "time": view.time(), "width": width, "xi": xi, "chi": chi,
        "weights": weights, "initial": initial,
        "modes": view.modes(), "mode_radii": view.mode_radii(),
        "mean": view.mean(), "mean_radius": view.mean_radius(),
    })
}

fn check_fraction_oracle(cases: &[serde_json::Value]) {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packet_curve_output_fraction_oracle.py");
    let mut child = Command::new("python3")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            serde_json::to_string(&serde_json::json!({"cases": cases}))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn one_mode_cubic_and_linear_midpoint_endpoint_and_initial_uncertainty() {
    for &chi in &[0.0, 0.3] {
        let z = (0.3_f64, -0.2_f64);
        let model =
            CollectiveModel::new(vec![0.5], vec![1.0], vec![z], 0.8, chi, 0.0, 0.0).unwrap();
        let (output, mut work) = output(&model, 0.1);
        let mut workspace = output.workspace(&mut work).unwrap();
        let before = work.curve_output_certificate_ops;
        let mut cases = Vec::new();
        for &time in &[0.0, 0.05, 0.1] {
            let view = output
                .enclose_at(time, &[1.0e-7], &mut workspace, &mut work)
                .unwrap();
            cases.push(oracle_case(&view, 0.8, 0.5, chi, &[1.0], &[z]));
            let perturbed = (z.0 + 5.0e-8, z.1);
            cases.push(oracle_case(&view, 0.8, 0.5, chi, &[1.0], &[perturbed]));
            assert!(view.weighted_radius() >= 1.0e-7);
        }
        assert_eq!(work.curve_output_certificate_ops - before, 3 * 56);
        check_fraction_oracle(&cases);
    }
}

#[test]
fn asymmetric_equal_frequency_multimode_and_weight_sum_not_exact_one() {
    let z = [(0.3, 0.1), (-0.2, 0.15)];
    let w = [0.3, 0.7000000000005];
    let chi = 0.2;
    let model =
        CollectiveModel::new(vec![0.5, 0.5], w.to_vec(), z.to_vec(), 0.4, chi, 0.0, 0.0).unwrap();
    let (output, mut work) = output(&model, 0.05);
    let mut workspace = output.workspace(&mut work).unwrap();
    let before = work.curve_output_certificate_ops;
    let mut cases = Vec::new();
    for &time in &[0.0, 0.025, 0.05] {
        let view = output
            .enclose_at(time, &[0.0, 0.0], &mut workspace, &mut work)
            .unwrap();
        cases.push(oracle_case(&view, 0.4, 0.5, chi, &w, &z));
    }
    check_fraction_oracle(&cases);
    assert_eq!(work.curve_output_certificate_ops - before, 3 * 88);
}

#[test]
fn candidate_mean_bound_can_exceed_initial_energy_bound() {
    let model =
        CollectiveModel::new(vec![1.0], vec![1.0], vec![(0.5, 0.0)], 1.0, 0.1, 0.0, 0.0).unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 0.1, 4, &mut work).unwrap();
    let weights = CausalWeights::compile(4, 12);
    let candidate = generate_candidate(&packet, 4, 2, &weights, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
    let m_y = curve.residual_bound().mean_magnitude_sup.hi;
    assert!(m_y * m_y > 0.25, "control must exercise M_Y^2 > S E");
    let output = PacketCurveOutput::new(curve);
    let mut workspace = output.workspace(&mut work).unwrap();
    let view = output
        .enclose_at(0.1, &[0.0], &mut workspace, &mut work)
        .unwrap();
    assert!(view.weighted_radius().is_finite());
}

#[test]
fn frozen_output_is_independent_of_later_candidate_mutation() {
    let model = CollectiveModel::new(
        vec![0.0],
        vec![1.0],
        vec![(0.25, 0.125)],
        0.0,
        0.2,
        0.0,
        0.0,
    )
    .unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 0.05, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let mut candidate = generate_candidate(&packet, 4, 2, &table, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
    candidate.mean[0] = (999.0, -999.0);
    let output = PacketCurveOutput::new(curve);
    let mut workspace = output.workspace(&mut work).unwrap();
    let view = output
        .enclose_at(0.025, &[0.0], &mut workspace, &mut work)
        .unwrap();
    check_fraction_oracle(&[oracle_case(&view, 0.0, 0.0, 0.2, &[1.0], &[(0.25, 0.125)])]);
}

#[test]
fn refusal_preserves_work_and_warmed_evaluation_does_not_allocate() {
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.2, 0.0)], 0.0, 0.1, 0.0, 0.0).unwrap();
    let (output, mut work) = output(&model, 0.1);
    let mut workspace = output.workspace(&mut work).unwrap();
    assert_eq!(work.curve_output_allocation_requests, 3);
    let before_validation = work.curve_output_certificate_ops;
    assert!(matches!(
        output.enclose_at(f64::NAN, &[0.0], &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert!(matches!(
        output.enclose_at(0.0, &[], &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert!(matches!(
        output.enclose_at(0.11, &[0.0], &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert!(matches!(
        output.enclose_at(0.0, &[-1.0], &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert_eq!(work.curve_output_certificate_ops, before_validation);
    assert!(matches!(
        output.enclose_at(0.0, &[f64::INFINITY], &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    let before_overflow = work.curve_output_certificate_ops;
    assert!(matches!(
        output.enclose_at(0.0, &[f64::MAX], &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert_eq!(work.curve_output_certificate_ops - before_overflow, 7);
    assert!(matches!(
        output.enclose_at(0.0, &[f64::from_bits(1)], &mut workspace, &mut work),
        Ok(_)
    ));
    output
        .enclose_at(0.05, &[0.0], &mut workspace, &mut work)
        .unwrap();
    ALLOC_COUNT.with(|count| count.set(0));
    COUNT_ALLOC.with(|flag| flag.set(true));
    let result = output.enclose_at(0.05, &[0.0], &mut workspace, &mut work);
    COUNT_ALLOC.with(|flag| flag.set(false));
    assert!(result.is_ok());
    assert_eq!(ALLOC_COUNT.with(Cell::get), 0);
    assert!(work.curve_output_certificate_ops > 0);
    let mut accumulated = PacketWorkCounters::default();
    accumulated.accumulate(&work);
    assert_eq!(
        accumulated.curve_output_certificate_ops,
        work.curve_output_certificate_ops
    );
    assert_eq!(
        accumulated.curve_output_allocation_requests,
        work.curve_output_allocation_requests
    );
}
