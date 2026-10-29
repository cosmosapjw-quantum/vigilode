//! Fixed unequal-frequency A01 reference; see A01_UNEQUAL_REFERENCE_CONTRACT.md.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::fs;
use std::process::{Command, Stdio};

use rodas5p_integrators::packet_window::{PacketError, PacketFrozenOutput, RepresentedPacketCurve};
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
        COUNT_ALLOC.with(|on| {
            if on.get() {
                ALLOC_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        COUNT_ALLOC.with(|on| {
            if on.get() {
                ALLOC_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNT_ALLOC.with(|on| {
            if on.get() {
                ALLOC_COUNT.with(|n| n.set(n.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn frozen_unequal_frequency_fraction_reference() {
    let xi = [-0.75, 0.125, 1.0 + 2.0_f64.powi(-49)];
    let weights = [0.125, 0.375, 0.5 + 2.0_f64.powi(-42)];
    let initial = [(0.25, -0.125), (-0.1875, 0.0625), (0.125, 0.1875)];
    let eta = 2.0_f64.powi(-30);
    let perturbations = [(eta, 0.0), (0.0, -eta), (-eta, 0.0)];
    let horizon = 1.0 / 32.0;
    let times = [
        0.0,
        horizon / 4.0,
        horizon / 2.0,
        3.0 * horizon / 4.0,
        horizon,
    ];
    let mut arms = Vec::with_capacity(4);
    let mut allocation_calibration_calls = 0usize;
    for (arm_index, &chi) in [0.0, 0.375, -0.375, 0.375].iter().enumerate() {
        let uncertain = arm_index == 3;
        let model = CollectiveModel::new(
            xi.to_vec(),
            weights.to_vec(),
            initial.to_vec(),
            0.75,
            chi,
            1.25,
            0.0,
        )
        .unwrap();
        if arm_index == 0 {
            // Calibrate the exact allocator used for the warmed query below.
            // CollectiveModel owns three Vec buffers, so a library clone must
            // be observable before any zero-allocation result is admitted.
            ALLOC_COUNT.with(|n| n.set(0));
            COUNT_ALLOC.with(|on| on.set(true));
            let clone = std::hint::black_box(model.clone());
            std::hint::black_box(clone.xi().as_ptr());
            std::hint::black_box(clone.weights().as_ptr());
            std::hint::black_box(clone.initial_amplitudes().as_ptr());
            COUNT_ALLOC.with(|on| on.set(false));
            allocation_calibration_calls = ALLOC_COUNT.with(Cell::get);
            assert!(
                allocation_calibration_calls >= 3,
                "ALLOCATION_CALIBRATION_FAILED: library clone of three Vec buffers measured {} allocations",
                allocation_calibration_calls
            );
        }
        let mut work = PacketWorkCounters::default();
        let packet = compile_packet(&model, horizon, 4, &mut work).unwrap();
        let table = CausalWeights::compile(4, 12);
        let candidate = generate_candidate(&packet, 4, 2, &table, &mut work).unwrap();
        let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
        curve
            .validate_source(&candidate, &packet, &mut work)
            .unwrap();
        let radii = if uncertain { [eta; 3] } else { [0.0; 3] };
        let owner = PacketFrozenOutput::new(curve, &radii, &mut work).unwrap();
        let mut workspace = owner.workspace(&mut work).unwrap();
        let mut outputs = Vec::with_capacity(times.len());
        for &time in &times {
            let before = work;
            let view = owner.enclose_at(time, &mut workspace, &mut work).unwrap();
            assert_eq!(
                work.curve_output_certificate_ops - before.curve_output_certificate_ops,
                120
            );
            assert_eq!(
                work.curve_frozen_query_attempts - before.curve_frozen_query_attempts,
                1
            );
            assert_eq!(
                work.curve_frozen_query_refusals - before.curve_frozen_query_refusals,
                0
            );
            if arm_index == 0 && time == 0.0 {
                println!(
                    "first_success_work_before={}",
                    serde_json::to_string(&before).unwrap()
                );
                println!(
                    "first_success_work_after={}",
                    serde_json::to_string(&work).unwrap()
                );
            }
            outputs.push(serde_json::json!({
                "time": view.time(), "modes": view.modes(), "mode_radii": view.mode_radii(),
                "mean": view.mean(), "mean_radius": view.mean_radius(),
            }));
        }
        // Measure only the warmed public query; oracle serialization is outside.
        ALLOC_COUNT.with(|n| n.set(0));
        COUNT_ALLOC.with(|on| on.set(true));
        let warm = owner.enclose_at(horizon / 2.0, &mut workspace, &mut work);
        COUNT_ALLOC.with(|on| on.set(false));
        assert!(warm.is_ok());
        assert_eq!(ALLOC_COUNT.with(Cell::get), 0);
        let true_initial: Vec<(f64, f64)> = initial
            .iter()
            .enumerate()
            .map(|(j, &z)| {
                if uncertain {
                    (z.0 + perturbations[j].0, z.1 + perturbations[j].1)
                } else {
                    z
                }
            })
            .collect();
        arms.push(serde_json::json!({
            "chi": chi, "uncertain": uncertain, "initial_true": true_initial,
            "outputs": outputs,
        }));
    }
    let payload = serde_json::json!({
        "schema_version": 1, "xi": xi, "weights": weights, "initial_center": initial,
        "width": 0.75, "carrier_metadata_only": 1.25, "kappa": 0.0,
        "horizon": horizon, "times": times, "eta": eta, "arms": arms,
        "allocation_calibration_clone_allocations": allocation_calibration_calls,
    });
    let evidence = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/production_audit_20261018/evidence/continuation_a01_lcert");
    fs::create_dir_all(&evidence).unwrap();
    fs::write(
        evidence.join("rust_payload.json"),
        serde_json::to_vec_pretty(&payload).unwrap(),
    )
    .unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packet_unequal_fraction_oracle.py");
    let result = Command::new("timeout")
        .args(["-s", "KILL", "600s", "python3"])
        .arg(script)
        .arg(evidence.join("rust_payload.json"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    fs::write(evidence.join("oracle_stdout.json"), &result.stdout).unwrap();
    fs::write(evidence.join("oracle_stderr.log"), &result.stderr).unwrap();
    fs::write(
        evidence.join("oracle_exit.txt"),
        result.status.code().unwrap_or(-1).to_string(),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "oracle exit {:?}: {}",
        result.status.code(),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn frozen_one_mode_exact_fraction_and_typed_refusals() {
    let z = (0.3_f64, -0.2_f64);
    let model = CollectiveModel::new(vec![0.5], vec![1.0], vec![z], 0.8, 0.3, 1.25, 0.0).unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 0.1, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let candidate = generate_candidate(&packet, 4, 2, &table, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
    let before_constructor = work;
    assert!(matches!(
        PacketFrozenOutput::new(curve.clone(), &[], &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert_eq!(
        work.curve_frozen_allocation_requests,
        before_constructor.curve_frozen_allocation_requests
    );
    assert!(matches!(
        PacketFrozenOutput::new(curve.clone(), &[f64::NAN], &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert!(matches!(
        PacketFrozenOutput::new(curve.clone(), &[f64::INFINITY], &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    let overflow_owner = PacketFrozenOutput::new(curve.clone(), &[f64::MAX], &mut work).unwrap();
    let mut overflow_workspace = overflow_owner.workspace(&mut work).unwrap();
    let before_overflow = work;
    assert!(matches!(
        overflow_owner.enclose_at(0.0, &mut overflow_workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert_eq!(
        work.curve_frozen_query_refusals - before_overflow.curve_frozen_query_refusals,
        1
    );
    assert_eq!(
        work.curve_frozen_query_attempts - before_overflow.curve_frozen_query_attempts,
        1
    );
    assert_eq!(
        work.curve_output_certificate_ops - before_overflow.curve_output_certificate_ops,
        7
    );
    println!(
        "overflow_work_before={}",
        serde_json::to_string(&before_overflow).unwrap()
    );
    println!(
        "overflow_work_after={}",
        serde_json::to_string(&work).unwrap()
    );
    let subnormal_owner =
        PacketFrozenOutput::new(curve.clone(), &[f64::from_bits(1)], &mut work).unwrap();
    let mut subnormal_workspace = subnormal_owner.workspace(&mut work).unwrap();
    let before_subnormal = work;
    let subnormal_result =
        subnormal_owner.enclose_at(f64::from_bits(1), &mut subnormal_workspace, &mut work);
    let subnormal_outcome = match subnormal_result {
        Ok(_) => "included",
        Err(PacketError::OutwardRefusal(_)) => "typed_refusal",
        Err(other) => panic!("unexpected subnormal error: {other:?}"),
    };
    println!("subnormal_outcome={subnormal_outcome}");
    println!(
        "subnormal_work_before={}",
        serde_json::to_string(&before_subnormal).unwrap()
    );
    println!(
        "subnormal_work_after={}",
        serde_json::to_string(&work).unwrap()
    );
    let owner = PacketFrozenOutput::new(curve, &[1.0e-7], &mut work).unwrap();
    assert_eq!(
        work.curve_frozen_allocation_requests - before_constructor.curve_frozen_allocation_requests,
        3
    );
    assert_eq!(
        work.curve_frozen_radius_items - before_constructor.curve_frozen_radius_items,
        3
    );
    let mut workspace = owner.workspace(&mut work).unwrap();
    let before_refusal = work;
    assert!(matches!(
        owner.enclose_at(f64::NAN, &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert_eq!(
        work.curve_frozen_query_attempts - before_refusal.curve_frozen_query_attempts,
        1
    );
    assert_eq!(
        work.curve_frozen_query_refusals - before_refusal.curve_frozen_query_refusals,
        1
    );
    assert_eq!(
        work.curve_output_certificate_ops,
        before_refusal.curve_output_certificate_ops
    );
    assert!(matches!(
        owner.enclose_at(0.101, &mut workspace, &mut work),
        Err(PacketError::OutwardRefusal(_))
    ));
    assert_eq!(
        work.curve_frozen_query_attempts - before_refusal.curve_frozen_query_attempts,
        2
    );
    assert_eq!(
        work.curve_frozen_query_refusals - before_refusal.curve_frozen_query_refusals,
        2
    );
    assert_eq!(
        work.curve_output_certificate_ops,
        before_refusal.curve_output_certificate_ops
    );
    let mut cases = Vec::new();
    for &time in &[0.0, 0.05, 0.1] {
        let view = owner.enclose_at(time, &mut workspace, &mut work).unwrap();
        cases.push(serde_json::json!({
            "time": view.time(), "width": 0.8, "xi": 0.5, "chi": 0.3,
            "weights": [1.0], "initial": [z], "modes": view.modes(),
            "mode_radii": view.mode_radii(), "mean": view.mean(),
            "mean_radius": view.mean_radius(),
        }));
    }
    assert_eq!(
        work.curve_frozen_query_attempts - before_refusal.curve_frozen_query_attempts,
        5
    );
    assert_eq!(
        work.curve_frozen_query_refusals - before_refusal.curve_frozen_query_refusals,
        2
    );
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packet_curve_output_fraction_oracle.py");
    let mut child = Command::new("python3")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
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
    let evidence = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/production_audit_20261018/evidence/continuation_a01_lcert");
    fs::create_dir_all(&evidence).unwrap();
    fs::write(evidence.join("one_mode_oracle_stdout.log"), &result.stdout).unwrap();
    fs::write(evidence.join("one_mode_oracle_stderr.log"), &result.stderr).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn subnormal_initial_amplitude_is_enclosed_or_typed_refused() {
    // For width=chi=0 the exact one-mode solution is the initial binary64
    // subnormal itself at every physical time.
    let tiny = f64::from_bits(1);
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(tiny, 0.0)], 0.0, 0.0, 0.0, 0.0).unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 0.125, 4, &mut work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let candidate = generate_candidate(&packet, 4, 2, &table, &mut work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap();
    let owner = PacketFrozenOutput::new(curve, &[0.0], &mut work).unwrap();
    let mut workspace = owner.workspace(&mut work).unwrap();
    for &time in &[0.0, 0.0625, 0.125] {
        let before = work;
        match owner.enclose_at(time, &mut workspace, &mut work) {
            Ok(view) => {
                let mode_delta = (view.modes()[0].0 - tiny).abs();
                let mean_delta = (view.mean().0 - tiny).abs();
                assert!(view.mode_radii()[0] >= mode_delta);
                assert!(view.mean_radius() >= mean_delta);
                assert!(view.mode_radii()[0] >= view.modes()[0].1.abs());
                assert!(view.mean_radius() >= view.mean().1.abs());
                println!("subnormal_initial_time={time} outcome=included");
            }
            Err(PacketError::OutwardRefusal(reason)) => {
                println!("subnormal_initial_time={time} outcome=typed_refusal reason={reason}");
                assert_eq!(
                    work.curve_frozen_query_refusals - before.curve_frozen_query_refusals,
                    1
                );
            }
            Err(other) => panic!("unexpected subnormal initial error: {other:?}"),
        }
        assert_eq!(
            work.curve_frozen_query_attempts - before.curve_frozen_query_attempts,
            1
        );
        println!(
            "subnormal_initial_work_before={}",
            serde_json::to_string(&before).unwrap()
        );
        println!(
            "subnormal_initial_work_after={}",
            serde_json::to_string(&work).unwrap()
        );
    }
}
