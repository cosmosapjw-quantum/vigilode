//! Actual represented-curve residual refinement and original-output gates.
use rodas5p_core::directed::Interval;
use rodas5p_integrators::packet_window::{
    BERNSTEIN_MAX_DEGREE, PacketFrozenOutput, RepresentedPacketCurve, outward_polynomial_sup_bound,
};
use rodas5p_integrators::{
    CausalWeights, CollectiveModel, PacketWorkCounters, compile_packet, generate_candidate,
};

fn curve() -> (RepresentedPacketCurve, PacketWorkCounters) {
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.5, 0.0)], 0.0, 2.0, 0.0, 0.0).unwrap();
    let mut work = PacketWorkCounters::default();
    let packet = compile_packet(&model, 0.5, 4, &mut work).unwrap();
    let weights = CausalWeights::compile(4, 12);
    let candidate = generate_candidate(&packet, 4, 2, &weights, &mut work).unwrap();
    (
        RepresentedPacketCurve::freeze(&candidate, &packet, &mut work).unwrap(),
        work,
    )
}

#[test]
fn unchanged_physical_gate_recovers_actual_nonlinear_packet() {
    let (curve, mut work) = curve();
    let before = curve.residual_bound();
    let source = curve.mode_coefficients(0).unwrap().to_vec();
    let owner = PacketFrozenOutput::from_weighted_radius(curve.clone(), 1e-10, &mut work).unwrap();
    let mut workspace = owner.workspace(&mut work).unwrap();
    let initial = owner.enclose_at(0.5, &mut workspace, &mut work).unwrap();
    let old_radius = initial.centered_weighted_radius().unwrap();
    let target = before.weighted_residual_sup.hi * 0.9999;
    let (refined, report) = curve
        .with_bernstein_residual_bound(target, 2, &mut work)
        .unwrap();
    assert_eq!(refined.mode_coefficients(0).unwrap(), source.as_slice());
    assert_eq!(
        refined.residual_bound().mean_magnitude_sup,
        before.mean_magnitude_sup
    );
    assert!(
        report.refined_upper < before.weighted_residual_sup.hi,
        "{report:?}"
    );
    let (owner, delegated) = owner
        .with_bernstein_residual_bound(target, 2, &mut work)
        .unwrap();
    assert_eq!(report, delegated);
    let refined_view = owner.enclose_at(0.5, &mut workspace, &mut work).unwrap();
    let new_radius = refined_view.centered_weighted_radius().unwrap();
    // A single physical budget, chosen between the two valid bounds, makes
    // the old gate refuse and the refined gate accept the same curve/state.
    let physical_budget = 0.5 * (old_radius + new_radius);
    assert!(old_radius > physical_budget);
    assert!(new_radius <= physical_budget);
    // Independent exact N=1 formula for the ODE at this binary input.
    let angle: f64 = 2.0 * 0.25 * 0.5;
    let expected = (0.5 * angle.cos(), 0.5 * angle.sin());
    let center = refined_view.modes()[0];
    let discrepancy = (center.0 - expected.0).hypot(center.1 - expected.1);
    assert!(discrepancy + 8.0 * f64::EPSILON <= new_radius);
    assert!(report.threshold_met);
    assert!(work.bernstein_residual_false_rejections_saved >= 2);
    assert!(work.bernstein_residual_attempts >= 2);
    assert!(work.bernstein_allocation_requests > 0);
    assert!(work.bernstein_reserved_items > 0);
    // Independent exact-rational Taylor enclosure at 21 physical outputs.
    let mut cases = Vec::new();
    for k in 0..=20 {
        let t = 0.5 * (k as f64) / 20.0;
        let view = owner.enclose_at(t, &mut workspace, &mut work).unwrap();
        cases.push(serde_json::json!({
            "time": t, "width": 0.0, "xi": 0.0, "chi": 2.0,
            "weights": [1.0], "initial": [[0.5,0.0]],
            "modes": view.modes(), "mode_radii": view.mode_radii(),
            "mean": view.mean(), "mean_radius": view.mean_radius()
        }));
    }
    use std::io::Write;
    use std::process::{Command, Stdio};
    let oracle = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packet_curve_output_fraction_oracle.py");
    let mut child = Command::new("python3")
        .arg(oracle)
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
            serde_json::to_string(&serde_json::json!({"cases":cases}))
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
    println!("{}", String::from_utf8_lossy(&result.stdout));
    println!(
        "A10_WITNESS old_residual={:.17e} refined_residual={:.17e} old_radius={:.17e} refined_radius={:.17e} fixed_physical_budget={:.17e} exact_formula_discrepancy={:.17e} work={}",
        before.weighted_residual_sup.hi,
        report.refined_upper,
        old_radius,
        new_radius,
        physical_budget,
        discrepancy,
        serde_json::to_string(&work).unwrap()
    );
}

#[test]
fn cheap_pass_does_no_transform_or_allocation_and_exhaustion_is_visible() {
    let (curve, mut work) = curve();
    work.reset();
    let bound = curve.residual_bound().weighted_residual_sup.hi;
    let (curve, report) = curve
        .with_bernstein_residual_bound(bound, 8, &mut work)
        .unwrap();
    assert!(report.threshold_met);
    assert_eq!(work, PacketWorkCounters::default());
    let (curve, report) = curve
        .with_bernstein_residual_bound(0.0, 0, &mut work)
        .unwrap();
    assert!(!report.threshold_met);
    assert!(report.refined_upper <= bound);
    assert_eq!(
        curve.residual_bound().weighted_residual_sup.hi,
        report.refined_upper
    );
    assert!(work.bernstein_budget_exhaustions > 0);
    assert_eq!(work.bernstein_refinement_terms, 0);
    assert_eq!(work.bernstein_residual_false_rejections_saved, 0);
}

#[test]
fn exact_binomial_family_and_complex_vector_share_outward_contract() {
    let zero = Interval::point(0.0).unwrap();
    for degree in 0..=16 {
        let mut binomial = 1_u64;
        let mut coefficients = Vec::new();
        for j in 0..=degree {
            coefficients.push((
                Interval::point(if j % 2 == 0 {
                    binomial as f64
                } else {
                    -(binomial as f64)
                })
                .unwrap(),
                zero,
            ));
            if j < degree {
                binomial = binomial * (degree - j) as u64 / (j + 1) as u64;
            }
        }
        let mut work = PacketWorkCounters::default();
        let result = outward_polynomial_sup_bound(&coefficients, 1.001, 0, &mut work).unwrap();
        // Exact polynomial is (1-tau)^degree, with exact supremum 1.
        assert!(result.upper >= 1.0 && result.upper <= 1.001);
    }
    // P=(t(1-t), i t(1-t)): exact weighted norm with w=(1/4,3/4)
    // is t(1-t), attaining 1/4. Refined scalar L1 bounds imply this norm.
    for imaginary in [false, true] {
        let values = [0.0, 1.0, -1.0].map(|v| {
            if imaginary {
                (zero, Interval::point(v).unwrap())
            } else {
                (Interval::point(v).unwrap(), zero)
            }
        });
        let mut work = PacketWorkCounters::default();
        let result = outward_polynomial_sup_bound(&values, 0.251, 4, &mut work).unwrap();
        assert!(result.upper >= 0.25 && result.upper < 0.251);
        assert!(work.bernstein_refinement_terms > 0);
    }
}

#[test]
fn degree_cap_and_invalid_budget_refuse_before_allocating() {
    let zero = Interval::point(0.0).unwrap();
    let coefficients = vec![(zero, zero); BERNSTEIN_MAX_DEGREE + 2];
    let mut work = PacketWorkCounters::default();
    assert!(outward_polynomial_sup_bound(&coefficients, 1.0, 0, &mut work).is_err());
    assert_eq!(work, PacketWorkCounters::default());
    let (curve, mut work) = curve();
    work.reset();
    assert!(
        curve
            .with_bernstein_residual_bound(f64::NAN, 2, &mut work)
            .is_err()
    );
    assert_eq!(work, PacketWorkCounters::default());
}
