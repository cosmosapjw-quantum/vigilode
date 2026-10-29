use rodas5p_integrators::packet_window::{
    VerifiedPacketConfig, VerifiedPacketError, VerifiedPacketTrajectory, integrate_verified_packet,
};
use rodas5p_integrators::{CollectiveModel, PacketWorkCounters};
use std::io::Write;
use std::process::{Command, Stdio};

fn config() -> VerifiedPacketConfig {
    VerifiedPacketConfig {
        order: 6,
        degree: 6,
        sweeps: 3,
        mean_tolerance: 1e-8,
        mode_tolerance: 1e-8,
    }
}

fn oracle_case(flow: &VerifiedPacketTrajectory, initial: &[(f64, f64)]) -> serde_json::Value {
    let model = flow.source_model();
    serde_json::json!({
        "initial": initial, "initial_time": flow.initial_time(), "width": model.width(),
        "chi": model.chi(), "weights": model.weights(), "xi": model.xi(),
        "outputs": flow.points().iter().map(|p| serde_json::json!({
            "time": p.time(), "modes": p.modes(), "mode_radii": p.mode_radii(),
            "mean": p.mean(), "mean_radius": p.mean_radius(),
            "weighted_radius": p.weighted_radius(),
        })).collect::<Vec<_>>()
    })
}

#[test]
fn unequal_frequencies_multwindow_against_independent_rational_solution() {
    let mut cases = Vec::new();
    for chi in [-0.2, 0.0, 0.2] {
        let z = vec![(0.2, -0.1), (-0.1, 0.2), (0.15, 0.1)];
        let model = CollectiveModel::new(
            vec![-0.75, 0.125, 1.0],
            vec![0.2, 0.3, 0.5000000000002],
            z.clone(),
            0.4,
            chi,
            0.0,
            0.0,
        )
        .unwrap();
        let mut work = PacketWorkCounters::default();
        let flow = integrate_verified_packet(
            &model,
            &[1e-10; 3],
            &[0.0, 0.03125, 0.0625, 0.09375, 0.125],
            config(),
            &mut work,
        )
        .unwrap();
        assert_eq!(flow.points().len(), 4);
        assert_eq!(flow.source_model().xi(), model.xi());
        assert!(flow.points().iter().all(|p| p.weighted_radius() >= 1e-10));
        cases.push(oracle_case(&flow, &z));
        let mut perturbed = z.clone();
        perturbed[0].0 += 1e-11;
        perturbed[2].1 -= 1e-11;
        cases.push(oracle_case(&flow, &perturbed));
    }
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/packet_unequal_frequency_oracle.py");
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
            serde_json::to_string(&serde_json::json!({"cases":cases}))
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn weighted_ball_does_not_inflate_by_sqrt_modes_each_window() {
    let n = 64;
    let model = CollectiveModel::new(
        vec![0.0; n],
        vec![1.0 / n as f64; n],
        vec![(0.25, 0.125); n],
        0.0,
        0.0,
        0.0,
        0.0,
    )
    .unwrap();
    let mesh: Vec<_> = (0..=32).map(|k| k as f64 / 32.0).collect();
    let mut cfg = config();
    cfg.order = 0;
    cfg.degree = 0;
    cfg.sweeps = 1;
    cfg.mean_tolerance = 1e-6;
    cfg.mode_tolerance = 1e-6;
    let flow = integrate_verified_packet(
        &model,
        &vec![1e-8; n],
        &mesh,
        cfg,
        &mut PacketWorkCounters::default(),
    )
    .unwrap();
    for point in flow.points() {
        assert!(point.weighted_radius() >= 1e-8);
        assert!(point.weighted_radius() <= 1.0000001e-8);
        assert_eq!(point.modes(), model.initial_amplitudes());
    }
}

#[test]
fn refuses_wrong_frame_bad_mesh_and_unsatisfied_original_tolerance() {
    let model =
        CollectiveModel::new(vec![0.5], vec![1.0], vec![(0.2, 0.1)], 0.4, 0.2, 0.0, 0.0).unwrap();
    let mut work = PacketWorkCounters::default();
    assert!(matches!(
        integrate_verified_packet(&model, &[0.0], &[-1e20, 1.0], config(), &mut work),
        Err(VerifiedPacketError::InexactDuration { .. })
    ));
    assert!(matches!(
        integrate_verified_packet(&model, &[0.0], &[0.0, 0.0], config(), &mut work),
        Err(VerifiedPacketError::InvalidInput(_))
    ));
    let mut strict = config();
    strict.mean_tolerance = 1e-30;
    strict.mode_tolerance = 1e-30;
    assert!(matches!(
        integrate_verified_packet(&model, &[0.0], &[0.0, 0.125], strict, &mut work),
        Err(VerifiedPacketError::BudgetExceeded { .. })
    ));
    let carrier =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.2, 0.1)], 0.0, 0.2, 1.0, 0.0).unwrap();
    assert_eq!(
        integrate_verified_packet(&carrier, &[0.0], &[0.0, 0.125], config(), &mut work)
            .unwrap_err(),
        VerifiedPacketError::UnsupportedModel
    );
}
