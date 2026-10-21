use rodas5p_integrators::packet_window::{
    VerifiedPacketAdaptiveConfig, VerifiedPacketConfig, VerifiedPacketDiagnostics,
    VerifiedPacketError, VerifiedPacketState, advance_verified_packet,
    integrate_verified_packet_adaptive,
};
use rodas5p_integrators::{CollectiveModel, PacketWorkCounters};
use std::io::Write;
use std::process::{Command, Stdio};

fn model() -> CollectiveModel {
    CollectiveModel::new(
        vec![-0.75, 0.125, 1.0],
        vec![0.2, 0.3, 0.5000000000002],
        vec![(0.2, -0.1), (-0.1, 0.2), (0.15, 0.1)],
        0.4,
        0.2,
        0.0,
        0.0,
    )
    .unwrap()
}

fn config() -> VerifiedPacketConfig {
    VerifiedPacketConfig {
        order: 6,
        degree: 6,
        sweeps: 3,
        mean_tolerance: 1e-8,
        mode_tolerance: 1e-8,
    }
}

#[test]
fn rejected_window_is_transactional_and_failed_work_is_retained() {
    let model = model();
    let mut work = PacketWorkCounters::default();
    let mut state =
        VerifiedPacketState::new(&model, &[1e-10; 3], 0.0, config(), &mut work).unwrap();
    let mut diagnostics = VerifiedPacketDiagnostics::default();
    let before_modes = state.modes().to_vec();
    let before_radius = state.weighted_radius();
    let before_work = work;
    assert!(advance_verified_packet(&mut state, 8.0, &mut diagnostics, &mut work).is_err());
    assert_eq!(state.time(), 0.0);
    assert_eq!(state.modes(), before_modes);
    assert_eq!(state.weighted_radius(), before_radius);
    assert_eq!(state.config(), config());
    assert_eq!(state.accepted_windows(), 0);
    assert_eq!(diagnostics.rejected(), 1);
    assert_eq!(diagnostics.attempts()[0].work_before, before_work);
    assert_eq!(diagnostics.attempts()[0].work_after, work);
    assert!(work.total_packet_terms() > before_work.total_packet_terms());
    let rejected_work = work;
    let point = advance_verified_packet(&mut state, 0.03125, &mut diagnostics, &mut work).unwrap();
    assert_eq!(state.time(), point.time());
    assert_eq!(state.modes(), point.modes());
    assert_eq!(state.weighted_radius(), point.weighted_radius());
    assert_eq!(diagnostics.rejected(), 1);
    assert_eq!(diagnostics.accepted(), 1);
    assert_eq!(diagnostics.attempts()[1].work_before, rejected_work);
    assert_eq!(state.accepted_windows(), 1);
    assert!(point.mean_radius() <= config().mean_tolerance);
    assert!(
        point
            .mode_radii()
            .iter()
            .all(|r| *r <= config().mode_tolerance)
    );
}

#[test]
fn adaptive_requested_mesh_and_nonzero_initial_ball_match_independent_oracle() {
    let model = model();
    let mut cfg = config();
    cfg.order = 1;
    cfg.degree = 1;
    cfg.mean_tolerance = 1e-4;
    cfg.mode_tolerance = 1e-4;
    let mesh = [0.0, 0.0625, 0.125];
    let mut work = PacketWorkCounters::default();
    let mut diagnostics = VerifiedPacketDiagnostics::default();
    let flow = integrate_verified_packet_adaptive(
        &model,
        &[1e-9; 3],
        &mesh,
        cfg,
        VerifiedPacketAdaptiveConfig {
            max_bisections: 16,
            max_attempts: 2000,
            bernstein_max_depth: Some(2),
        },
        &mut diagnostics,
        &mut work,
    )
    .unwrap();
    assert!(
        diagnostics.rejected() > 0,
        "fixture must exercise actual rejection"
    );
    assert!(diagnostics.accepted() > flow.points().len());
    assert!(work.bernstein_transform_terms > 0);
    assert!(
        diagnostics
            .attempts()
            .iter()
            .any(|a| a.pre_refinement_error.is_some())
    );

    assert_eq!(flow.points().len(), mesh.len() - 1);
    assert_eq!(flow.initial_radii(), &[1e-9; 3]);
    for (point, &time) in flow.points().iter().zip(&mesh[1..]) {
        assert_eq!(point.time().to_bits(), time.to_bits());
        assert!(point.mean_radius() <= cfg.mean_tolerance);
        assert!(point.mode_radii().iter().all(|r| *r <= cfg.mode_tolerance));
    }
    let mut cases = Vec::new();
    for perturbation in [-1e-10, 0.0, 1e-10] {
        let mut initial = model.initial_amplitudes().to_vec();
        initial[0].0 += perturbation;
        initial[2].1 -= perturbation;
        cases.push(serde_json::json!({
            "initial": initial, "initial_time": flow.initial_time(),
            "width": model.width(), "chi": model.chi(),
            "weights": model.weights(), "xi": model.xi(),
            "outputs": flow.points().iter().map(|p| serde_json::json!({
                "time":p.time(), "modes":p.modes(), "mode_radii":p.mode_radii(),
                "mean":p.mean(), "mean_radius":p.mean_radius(),
                "weighted_radius":p.weighted_radius(),
            })).collect::<Vec<_>>()
        }));
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
    println!(
        "accepted={} rejected={}",
        diagnostics.accepted(),
        diagnostics.rejected()
    );
}

#[test]
fn resource_exhaustion_preserves_failure_and_never_relaxes_budget() {
    let model = model();
    let mut diagnostics = VerifiedPacketDiagnostics::default();
    let mut work = PacketWorkCounters::default();
    let error = integrate_verified_packet_adaptive(
        &model,
        &[0.0; 3],
        &[0.0, 8.0],
        config(),
        VerifiedPacketAdaptiveConfig {
            max_bisections: 16,
            max_attempts: 1,
            bernstein_max_depth: None,
        },
        &mut diagnostics,
        &mut work,
    )
    .unwrap_err();
    assert_eq!(error, VerifiedPacketError::AttemptLimit { attempts: 1 });
    assert_eq!(diagnostics.rejected(), 1);
    assert_eq!(diagnostics.accepted(), 0);
    assert!(diagnostics.attempts()[0].outcome.is_err());
    let mut cfg = config();
    cfg.mean_tolerance = 1e-15;
    cfg.mode_tolerance = 1e-15;
    let mut strict = VerifiedPacketDiagnostics::default();
    let error = integrate_verified_packet_adaptive(
        &model,
        &[1e-8; 3],
        &[0.0, 0.03125],
        cfg,
        VerifiedPacketAdaptiveConfig {
            max_bisections: 2,
            max_attempts: 20,
            bernstein_max_depth: None,
        },
        &mut strict,
        &mut work,
    )
    .unwrap_err();
    assert_eq!(
        error,
        VerifiedPacketError::RefinementExhausted { index: 0, depth: 2 }
    );
    assert_eq!(strict.accepted(), 0);
    assert_eq!(strict.rejected(), 3);
    assert!(
        strict
            .attempts()
            .iter()
            .all(|a| matches!(a.outcome, Err(VerifiedPacketError::BudgetExceeded { .. })))
    );
}

#[test]
fn invalid_and_overflow_inputs_refuse_without_state_mutation() {
    let model = model();
    let mut work = PacketWorkCounters::default();
    let mut state = VerifiedPacketState::new(&model, &[0.0; 3], 0.0, config(), &mut work).unwrap();
    let mut diagnostics = VerifiedPacketDiagnostics::default();
    for end in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        assert!(matches!(
            advance_verified_packet(&mut state, end, &mut diagnostics, &mut work),
            Err(VerifiedPacketError::InvalidInput(_))
        ));
        assert_eq!(state.time(), 0.0);
        assert_eq!(state.modes(), model.initial_amplitudes());
        assert_eq!(state.weighted_radius(), 0.0);
    }
    assert_eq!(diagnostics.rejected(), 4);
    assert!(VerifiedPacketState::new(&model, &[f64::MAX; 3], 0.0, config(), &mut work).is_err());
    let mut huge = config();
    huge.degree = usize::MAX;
    assert!(VerifiedPacketState::new(&model, &[0.0; 3], 0.0, huge, &mut work).is_err());
    let mut cfg = config();
    cfg.sweeps = 0;
    assert!(VerifiedPacketState::new(&model, &[0.0; 3], 0.0, cfg, &mut work).is_err());
    let prior_len = diagnostics.attempts().len();
    assert!(matches!(
        integrate_verified_packet_adaptive(
            &model,
            &[0.0; 3],
            &[-1e20, 1.0],
            config(),
            VerifiedPacketAdaptiveConfig {
                max_bisections: 8,
                max_attempts: 100,
                bernstein_max_depth: None,
            },
            &mut diagnostics,
            &mut work
        ),
        Err(VerifiedPacketError::InexactDuration { .. })
    ));
    assert_eq!(diagnostics.attempts().len(), prior_len);
}

#[test]
fn subnormal_time_resolution_and_depth_size_refuse_explicitly() {
    let model =
        CollectiveModel::new(vec![0.0], vec![1.0], vec![(0.25, 0.0)], 0.0, 0.0, 0.0, 0.0).unwrap();
    let cfg = VerifiedPacketConfig {
        order: 0,
        degree: 0,
        sweeps: 1,
        mean_tolerance: 1e-6,
        mode_tolerance: 1e-6,
    };
    let mut diagnostics = VerifiedPacketDiagnostics::default();
    let mut work = PacketWorkCounters::default();
    let result = integrate_verified_packet_adaptive(
        &model,
        &[0.0],
        &[0.0, f64::from_bits(1)],
        cfg,
        VerifiedPacketAdaptiveConfig {
            max_bisections: 4,
            max_attempts: 20,
            bernstein_max_depth: None,
        },
        &mut diagnostics,
        &mut work,
    );
    assert!(matches!(
        result,
        Err(VerifiedPacketError::UnrepresentableSubdivision { index: 0 })
    ));
    assert_eq!(diagnostics.rejected(), 1);
    assert_eq!(diagnostics.accepted(), 0);
    assert!(
        integrate_verified_packet_adaptive(
            &model,
            &[0.0],
            &[0.0, 0.25],
            cfg,
            VerifiedPacketAdaptiveConfig {
                max_bisections: usize::MAX,
                max_attempts: 20,
                bernstein_max_depth: None,
            },
            &mut diagnostics,
            &mut work,
        )
        .is_err()
    );
    assert_eq!(diagnostics.rejected(), 1);
}
