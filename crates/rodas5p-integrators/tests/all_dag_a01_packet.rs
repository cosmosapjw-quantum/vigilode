//! A01 merged authority and independent numerical checks, without allocator claims.
use std::io::Write;
use std::process::{Command, Stdio};

use rodas5p_integrators::packet_window::{
    PacketCurveOutput, PacketError, PacketFrozenOutput, RepresentedPacketCurve,
};
use rodas5p_integrators::{
    CausalWeights, CollectiveModel, PacketWorkCounters, compile_packet, generate_candidate,
};

fn curve(model: &CollectiveModel, work: &mut PacketWorkCounters) -> RepresentedPacketCurve {
    let packet = compile_packet(model, 1.0 / 32.0, 4, work).unwrap();
    let table = CausalWeights::compile(4, 12);
    let candidate = generate_candidate(&packet, 4, 2, &table, work).unwrap();
    let curve = RepresentedPacketCurve::freeze(&candidate, &packet, work).unwrap();
    curve.validate_source(&candidate, &packet, work).unwrap();
    curve
}

#[test]
fn frozen_weighted_owner_preserves_arithmetic_and_refusal_contract() {
    let model = CollectiveModel::new(
        vec![-0.75, 0.125, 1.0 + 2.0_f64.powi(-49)],
        vec![0.125, 0.375, 0.5 + 2.0_f64.powi(-42)],
        vec![(0.25, -0.125), (-0.1875, 0.0625), (0.125, 0.1875)],
        0.75,
        0.375,
        1.25,
        0.0,
    )
    .unwrap();
    let mut work = PacketWorkCounters::default();
    let frozen = curve(&model, &mut work);
    for radius in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(matches!(
            PacketFrozenOutput::from_weighted_radius(frozen.clone(), radius, &mut work),
            Err(PacketError::OutwardRefusal(_))
        ));
    }
    let radius = 2.0_f64.powi(-30);
    let direct = PacketCurveOutput::new(frozen.clone());
    let before = work;
    let owner = PacketFrozenOutput::from_weighted_radius(frozen, radius, &mut work).unwrap();
    assert_eq!(
        work.curve_frozen_radius_items - before.curve_frozen_radius_items,
        1
    );
    assert_eq!(
        work.curve_frozen_allocation_requests,
        before.curve_frozen_allocation_requests
    );
    let mut direct_workspace = direct.workspace(&mut work).unwrap();
    let mut frozen_workspace = owner.workspace(&mut work).unwrap();
    for t in [0.0, 1.0 / 128.0, 1.0 / 32.0] {
        let direct_view = direct
            .enclose_at_weighted(t, radius, &mut direct_workspace, &mut work)
            .unwrap();
        let before = work;
        let view = owner
            .enclose_at(t, &mut frozen_workspace, &mut work)
            .unwrap();
        assert_eq!(view.modes(), direct_view.modes());
        assert_eq!(view.mode_radii(), direct_view.mode_radii());
        assert_eq!(view.mean(), direct_view.mean());
        assert_eq!(view.mean_radius(), direct_view.mean_radius());
        assert_eq!(
            view.centered_weighted_radius(),
            direct_view.centered_weighted_radius()
        );
        assert!(view.centered_weighted_radius().unwrap() >= view.weighted_radius());
        // Preserve the production same-query displacement API while adding
        // the carryable weighted ball. The outward triangle sum is the bridge.
        assert_eq!(
            view.weighted_center_displacement(),
            direct_view.weighted_center_displacement()
        );
        assert_eq!(
            view.centered_weighted_radius().unwrap(),
            rodas5p_core::directed::add_up(
                view.weighted_radius(),
                view.weighted_center_displacement()
            )
            .unwrap()
        );
        assert_eq!(
            work.curve_frozen_query_attempts - before.curve_frozen_query_attempts,
            1
        );
        assert_eq!(
            work.curve_frozen_query_refusals,
            before.curve_frozen_query_refusals
        );
    }
    let before = work;
    assert!(
        owner
            .enclose_at(f64::NAN, &mut frozen_workspace, &mut work)
            .is_err()
    );
    assert_eq!(
        work.curve_frozen_query_attempts - before.curve_frozen_query_attempts,
        1
    );
    assert_eq!(
        work.curve_frozen_query_refusals - before.curve_frozen_query_refusals,
        1
    );
    assert_eq!(
        work.curve_output_certificate_ops,
        before.curve_output_certificate_ops
    );
    // Refusal leaves the immutable initial tube and curve intact.
    let view = owner
        .enclose_at(0.0, &mut frozen_workspace, &mut work)
        .unwrap();
    assert!(view.centered_weighted_radius().unwrap() >= radius);
}

#[test]
fn frozen_component_reference_is_independent_of_allocator_measurement() {
    let xi = [-0.75, 0.125, 1.0 + 2.0_f64.powi(-49)];
    let weights = [0.125, 0.375, 0.5 + 2.0_f64.powi(-42)];
    let initial = [(0.25, -0.125), (-0.1875, 0.0625), (0.125, 0.1875)];
    let eta = 2.0_f64.powi(-30);
    let perturbations = [(eta, 0.0), (0.0, -eta), (-eta, 0.0)];
    let times = [0.0, 1.0 / 128.0, 1.0 / 64.0, 3.0 / 128.0, 1.0 / 32.0];
    let mut arms = Vec::new();
    for (i, chi) in [0.0, 0.375, -0.375, 0.375].into_iter().enumerate() {
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
        let mut work = PacketWorkCounters::default();
        let frozen = curve(&model, &mut work);
        let uncertain = i == 3;
        let mut input_radii = if uncertain { [eta; 3] } else { [0.0; 3] };
        let owner = PacketFrozenOutput::new(frozen, &input_radii, &mut work).unwrap();
        input_radii.fill(f64::NAN); // Owner copied its premise; caller cannot rebind it.
        let mut workspace = owner.workspace(&mut work).unwrap();
        let mut outputs = Vec::new();
        for t in times {
            let before = work;
            let view = owner.enclose_at(t, &mut workspace, &mut work).unwrap();
            assert_eq!(
                work.curve_output_certificate_ops - before.curve_output_certificate_ops,
                110 + 3 * 3 + 1
            );
            assert!(view.centered_weighted_radius().is_none());
            assert!(view.weighted_center_displacement().is_finite());
            assert!(view.weighted_center_displacement() >= 0.0);
            outputs.push(serde_json::json!({"time":t,"modes":view.modes(),"mode_radii":view.mode_radii(),"mean":view.mean(),"mean_radius":view.mean_radius()}));
        }
        let true_initial: Vec<_> = initial
            .iter()
            .zip(perturbations)
            .map(|(&(re, im), (dr, di))| {
                if uncertain {
                    (re + dr, im + di)
                } else {
                    (re, im)
                }
            })
            .collect();
        arms.push(serde_json::json!({"chi":chi,"uncertain":uncertain,"initial_true":true_initial,"outputs":outputs}));
    }
    let payload = serde_json::json!({"schema_version":1,"xi":xi,"weights":weights,"initial_center":initial,"width":0.75,"carrier_metadata_only":1.25,"kappa":0.0,"horizon":1.0/32.0,"times":times,"eta":eta,"arms":arms});
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packet_unequal_fraction_oracle.py");
    let mut child = Command::new("python3")
        .arg(script)
        .args(["--numeric-only", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&payload).unwrap().as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(result["disk_checks"], 80);
    assert_eq!(result["allocation_authority"], "NOT_EVALUATED");
    println!("{result}");
}
