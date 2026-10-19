//! Fixed unequal-frequency A01 reference; see A01_UNEQUAL_REFERENCE_CONTRACT.md.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::fs;
use std::process::{Command, Stdio};

use rodas5p_integrators::packet_window::{PacketCurveOutput, RepresentedPacketCurve};
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
fn fixed_unequal_frequency_fraction_reference() {
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
        let owner = PacketCurveOutput::new(curve);
        let mut workspace = owner.workspace(&mut work).unwrap();
        let radii = if uncertain { [eta; 3] } else { [0.0; 3] };
        let mut outputs = Vec::with_capacity(times.len());
        for &time in &times {
            let before = work.curve_output_certificate_ops;
            let view = owner
                .enclose_at(time, &radii, &mut workspace, &mut work)
                .unwrap();
            assert_eq!(work.curve_output_certificate_ops - before, 120);
            outputs.push(serde_json::json!({
                "time": view.time(), "modes": view.modes(), "mode_radii": view.mode_radii(),
                "mean": view.mean(), "mean_radius": view.mean_radius(),
            }));
        }
        // Measure only the warmed public query; oracle serialization is outside.
        ALLOC_COUNT.with(|n| n.set(0));
        COUNT_ALLOC.with(|on| on.set(true));
        let warm = owner.enclose_at(horizon / 2.0, &radii, &mut workspace, &mut work);
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
    let evidence = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../research/production_audit_20261018/evidence/continuation_a01_unequal_reference",
    );
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
