//! Contracts for the frozen represented packet curve and its interval defect.
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Mutex;

use rodas5p_core::directed::Interval;
use rodas5p_integrators::packet_window::{PacketComplexInterval, RepresentedPacketCurve};
use rodas5p_integrators::{
    compile_packet, generate_candidate, CausalWeights, CollectiveModel, PacketWorkCounters,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, AtomicOrdering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, AtomicOrdering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, AtomicOrdering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn packet_for(
    model: &CollectiveModel,
    horizon: f64,
    order: usize,
    degree: usize,
) -> (
    rodas5p_integrators::packet_window::CompiledPacket,
    rodas5p_integrators::packet_window::PacketCandidate,
    RepresentedPacketCurve,
    PacketWorkCounters,
) {
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(model, horizon, order, &mut work).unwrap();
    let table = CausalWeights::compile(order, 3 * degree);
    let candidate = generate_candidate(&packet, degree, 2, &table, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
    (packet, candidate, curve, work)
}

fn assert_python_fraction_residual_contains(
    model: &CollectiveModel,
    curve: &RepresentedPacketCurve,
    candidate: &[(f64, f64)],
    require_candidate_difference: bool,
) {
    let modes: Vec<Vec<[f64; 2]>> = (0..model.num_modes())
        .map(|mode| {
            curve
                .mode_coefficients(mode)
                .unwrap()
                .iter()
                .map(|&(re, im)| [re, im])
                .collect()
        })
        .collect();
    let residuals: Vec<Vec<serde_json::Value>> = (0..model.num_modes())
        .map(|mode| {
            curve
                .residual_coefficients(mode)
                .unwrap()
                .iter()
                .map(|coefficient| {
                    serde_json::json!({
                        "re": [coefficient.re.lo, coefficient.re.hi],
                        "im": [coefficient.im.lo, coefficient.im.hi]
                    })
                })
                .collect()
        })
        .collect();
    let evaluation_times = [0.0, curve.horizon() * 0.5, curve.horizon()];
    let mut evaluation_checks = Vec::with_capacity(evaluation_times.len());
    for time in evaluation_times {
        let mut mode_boxes = vec![
            PacketComplexInterval {
                re: Interval::point(0.0).unwrap(),
                im: Interval::point(0.0).unwrap(),
            };
            model.num_modes()
        ];
        curve
            .enclose_modes_at_into(time, &mut mode_boxes, &mut PacketWorkCounters::default())
            .unwrap();
        let mean_box = curve
            .enclose_mean_at(time, &mut PacketWorkCounters::default())
            .unwrap();
        evaluation_checks.push(serde_json::json!({
            "time": time,
            "modes": mode_boxes.iter().map(|value| serde_json::json!({
                "re": [value.re.lo, value.re.hi],
                "im": [value.im.lo, value.im.hi]
            })).collect::<Vec<_>>(),
            "mean": {
                "re": [mean_box.re.lo, mean_box.re.hi],
                "im": [mean_box.im.lo, mean_box.im.hi]
            }
        }));
    }
    let curve_bound = curve.residual_bound();
    let payload = serde_json::json!({
        "weights": model.weights(),
        "xi": model.xi(),
        "width": model.width(),
        "chi": model.chi(),
        "horizon": curve.horizon(),
        "modes": modes,
        "candidate": candidate.iter().map(|&(re, im)| [re, im]).collect::<Vec<_>>(),
        "residuals": residuals,
        "evaluation_checks": evaluation_checks,
        "require_candidate_difference": require_candidate_difference,
        "bounds": {
            "weighted_residual_sup": [curve_bound.weighted_residual_sup.lo, curve_bound.weighted_residual_sup.hi],
            "mean_magnitude_sup": [curve_bound.mean_magnitude_sup.lo, curve_bound.mean_magnitude_sup.hi]
        },
    });
    if std::env::var_os("A01_DUMP_FRACTION_FIXTURE").is_some() {
        println!(
            "A01_FRACTION_FIXTURE={}",
            serde_json::to_string(&payload).unwrap()
        );
    }
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packet_curve_fraction_oracle.py");
    let mut child = Command::new("python3")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("stdlib Python Fraction oracle is available");
    serde_json::to_writer(child.stdin.as_mut().unwrap(), &payload).unwrap();
    child.stdin.take().unwrap().flush().unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "Fraction oracle failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("exact Fraction residual and evaluation enclosures verified"));
}

#[test]
fn linear_dyadic_residual_contains_exact_fraction_and_curve_evaluates_consistently() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let model =
        CollectiveModel::new(vec![0.5], vec![1.0], vec![(1.0, 0.0)], 0.5, 0.0, 0.0, 0.0).unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 0.5, 1, &mut work).unwrap();
    let table = CausalWeights::compile(1, 3);
    let candidate = generate_candidate(&packet, 1, 1, &table, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();

    assert_eq!(curve.maximum_degree(), 5);
    assert_eq!(work.curve_setup_interval_ops, 2); // 1/h plus one omega per mode.
    assert_eq!(work.curve_time_normalization_ops, 0);
    let mode = curve.mode_coefficients(0).unwrap();
    assert_eq!(mode[0], (1.0, 0.0));
    assert_eq!(mode[1], (0.0, 0.125));
    let residual = curve.residual_coefficients(0).unwrap();
    assert!(residual[1].re.contains(1.0 / 32.0));
    assert_python_fraction_residual_contains(&model, &curve, &candidate.mean, false);

    let mut centers = [(0.0, 0.0)];
    let mut enclosures = [PacketComplexInterval {
        re: Interval::point(0.0).unwrap(),
        im: Interval::point(0.0).unwrap(),
    }];
    for time in [0.0, 0.25, 0.5] {
        curve
            .evaluate_modes_at_into(time, &mut centers, &mut work)
            .unwrap();
        curve
            .enclose_modes_at_into(time, &mut enclosures, &mut work)
            .unwrap();
        assert!(enclosures[0].re.contains(centers[0].0));
        assert!(enclosures[0].im.contains(centers[0].1));
    }
    assert_eq!(work.curve_time_normalization_ops, 6); // center and mode interval at three times.
    assert_eq!(curve.mode_coefficients(0).unwrap()[0], (1.0, 0.0));
}

#[test]
fn asymmetric_nonlinear_curve_uses_actual_weighted_mean_and_full_degree() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let model = CollectiveModel::new(
        vec![-0.75, f64::from_bits(1.0_f64.to_bits() + 1)],
        vec![0.3, 0.7],
        vec![(0.4, 0.2), (-0.1, 0.3)],
        0.25,
        0.5,
        0.0,
        0.0,
    )
    .unwrap();
    let (packet, candidate, curve, mut work) = packet_for(&model, 0.25, 1, 2);
    let degree = curve.maximum_degree();
    assert_eq!(degree, 8);
    assert_eq!(curve.mode_coefficients(0).unwrap().len(), degree + 1);
    assert_eq!(
        curve.residual_coefficients(0).unwrap().len(),
        3 * degree + 1
    );
    assert_eq!(curve.mean_coefficients().len(), degree + 1);
    assert_eq!(
        curve.residual_coefficients(1).unwrap().len(),
        3 * degree + 1
    );
    let bound = curve.residual_bound();
    assert!(bound.weighted_residual_sup.lo == 0.0);
    assert!(bound.weighted_residual_sup.hi.is_finite());
    assert!(bound.mean_magnitude_sup.lo == 0.0);
    assert!(bound.mean_magnitude_sup.hi.is_finite());
    assert!(bound.mean_magnitude_sup.hi > 0.0);
    assert!(work.curve_setup_allocation_requests > 0);
    assert!(work.curve_setup_reserved_items > 0);
    assert_eq!(work.curve_setup_interval_ops, 3); // reciprocal plus two mode omegas.
    assert_eq!(work.curve_time_normalization_ops, 0);
    assert_eq!(work.curve_model_constructor_payload_items, 4 * 2 + 4);
    assert_eq!(work.curve_model_constructor_arithmetic_ops, 6 * 2 + 2);
    assert_eq!(work.curve_model_revalidation_items, 0);
    curve
        .validate_source(&candidate, &packet, &mut work)
        .unwrap();
    assert_eq!(work.curve_model_revalidation_items, 3 * (4 * 2 + 5));
    assert_eq!(work.curve_model_constructor_payload_items, 4 * 2 + 4);
    assert_eq!(work.curve_model_constructor_arithmetic_ops, 6 * 2 + 2);
    let mut accumulated = PacketWorkCounters::default();
    accumulated.accumulate(&work);
    assert_eq!(accumulated.total_packet_terms(), work.total_packet_terms());
    assert_eq!(
        accumulated.curve_model_constructor_payload_items,
        work.curve_model_constructor_payload_items
    );
    assert_eq!(
        accumulated.curve_model_constructor_arithmetic_ops,
        work.curve_model_constructor_arithmetic_ops
    );

    assert_python_fraction_residual_contains(&model, &curve, &candidate.mean, true);

    let mut mutated_candidate = candidate.clone();
    mutated_candidate.mean[0].0 = mutated_candidate.mean[0].0.next_up();
    assert!(curve
        .validate_source(&mutated_candidate, &packet, &mut work)
        .is_err());
    let before = curve.mode_coefficients(0).unwrap().to_vec();
    let mut output = vec![(0.0, 0.0); model.num_modes()];
    curve
        .evaluate_modes_at_into(0.125, &mut output, &mut work)
        .unwrap();
    assert_eq!(curve.mode_coefficients(0).unwrap(), before.as_slice());

    let other_model = CollectiveModel::new(
        vec![-0.75, f64::from_bits(1.0_f64.to_bits() + 1)],
        vec![0.3, 0.7],
        vec![(0.4, 0.2), (-0.1, 0.3)],
        0.5,
        0.5,
        0.0,
        0.0,
    )
    .unwrap();
    let other_packet = compile_packet(&other_model, 0.25, 1, &mut work).unwrap();
    assert!(curve
        .validate_source(&candidate, &other_packet, &mut work)
        .is_err());

    assert!(curve.mode_coefficients(0).unwrap()[degree] != (0.0, 0.0));
    assert!(
        curve.mean_coefficients()[degree].re.mag() > 0.0
            || curve.mean_coefficients()[degree].im.mag() > 0.0
    );
    let highest_residual = curve.residual_coefficients(0).unwrap()[3 * degree];
    assert!(highest_residual.re.mag() > 0.0 || highest_residual.im.mag() > 0.0);
    assert!(work.certificate_items > 0);
}

#[test]
fn evaluation_reuses_frozen_storage_without_allocating_and_refuses_bad_times() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let model = CollectiveModel::new(
        vec![0.25],
        vec![1.0],
        vec![(0.125, 0.0)],
        0.5,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let (_packet, _candidate, curve, mut work) = packet_for(&model, 0.5, 1, 1);
    let mut centers = [(0.0, 0.0)];
    let mut boxes = [PacketComplexInterval {
        re: Interval::point(0.0).unwrap(),
        im: Interval::point(0.0).unwrap(),
    }];
    let before = ALLOCATIONS.load(AtomicOrdering::SeqCst);
    for _ in 0..128 {
        curve
            .evaluate_modes_at_into(0.25, &mut centers, &mut work)
            .unwrap();
        curve
            .enclose_modes_at_into(0.25, &mut boxes, &mut work)
            .unwrap();
        let _mean = curve.enclose_mean_at(0.25, &mut work).unwrap();
    }
    let allocations = ALLOCATIONS.load(AtomicOrdering::SeqCst) - before;
    assert_eq!(allocations, 0);
    assert!(curve
        .evaluate_modes_at_into(f64::NAN, &mut centers, &mut work)
        .is_err());
    assert!(curve
        .evaluate_modes_at_into(0.75, &mut centers, &mut work)
        .is_err());
    assert!(curve
        .evaluate_modes_at_into(f64::INFINITY, &mut centers, &mut work)
        .is_err());
    assert!(curve
        .evaluate_modes_at_into(0.25, &mut [], &mut work)
        .is_err());
    assert!(work.output_horner_terms > 0);
}

#[test]
fn malformed_degree_and_overflowing_frequency_refuse_with_attempted_work() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let model =
        CollectiveModel::new(vec![0.5], vec![1.0], vec![(0.25, 0.0)], 1.0, 0.0, 0.0, 0.0).unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 0.25, 1, &mut work).unwrap();
    let table = CausalWeights::compile(1, 3);
    let candidate = generate_candidate(&packet, 1, 1, &table, &mut work).unwrap();
    let mut nonfinite_candidate = candidate.clone();
    nonfinite_candidate.mean[0].0 = f64::NAN;
    assert!(RepresentedPacketCurve::freeze(&nonfinite_candidate, &packet, &mut work).is_err());
    let mut candidate = candidate;
    candidate.degree = usize::MAX;
    assert!(RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).is_err());
    assert!(work.certificate_items > 0 || work.identity_comparison_items > 0);
}

#[test]
fn subnormal_initial_value_is_retained_and_frequency_overflow_refuses() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let subnormal = f64::from_bits(1);
    let model = CollectiveModel::new(
        vec![0.0],
        vec![1.0],
        vec![(subnormal, 0.0)],
        0.5,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let (_packet, _candidate, curve, mut work) = packet_for(&model, 1.0, 0, 0);
    assert_eq!(
        curve.mode_coefficients(0).unwrap()[0].0.to_bits(),
        subnormal.to_bits()
    );
    let mut interval = [PacketComplexInterval {
        re: Interval::point(0.0).unwrap(),
        im: Interval::point(0.0).unwrap(),
    }];
    curve
        .enclose_modes_at_into(0.5, &mut interval, &mut work)
        .unwrap();
    assert!(interval[0].re.contains(subnormal));

    let overflow_model = CollectiveModel::new(
        vec![1.0],
        vec![1.0],
        vec![(0.25, 0.0)],
        f64::MAX,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let mut overflow_work = PacketWorkCounters::default();
    let overflow_packet = compile_packet(&overflow_model, 2.0, 0, &mut overflow_work).unwrap();
    let weights = CausalWeights::compile(0, 0);
    let candidate =
        generate_candidate(&overflow_packet, 0, 1, &weights, &mut overflow_work).unwrap();
    assert!(
        RepresentedPacketCurve::freeze(&candidate, &overflow_packet, &mut overflow_work).is_err()
    );
    assert!(overflow_work.reconstruction_terms > 0);
    assert!(overflow_work.curve_setup_allocation_requests > 0);
    assert!(overflow_work.curve_setup_reserved_items > 0);
}
