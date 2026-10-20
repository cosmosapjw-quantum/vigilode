#![cfg(feature = "native")]
use vigilode_hybrid::{IntegrationConfig, Model, solve_trajectory};
#[test]
fn native_routes_complete_with_same_physical_error_budget() {
    let n = 16;
    let ts: Vec<_> = (0..=10).map(|i| 0.2 * i as f64 / 10.0).collect();
    let y: Vec<_> = (0..n)
        .map(|i| 0.45 + 0.12 * (std::f64::consts::PI * (i as f64 + 0.5) / n as f64).cos())
        .collect();
    for eps in [0.0, 1.0] {
        let m = Model::new(n, 2.56, 2.0, 0.4, eps).unwrap();
        let cfg = IntegrationConfig::default();
        let mut outputs = Vec::new();
        for route in [
            "auto",
            "structured",
            "sequential",
            "lazy-local",
            "eager-full",
        ] {
            let r = solve_trajectory(&m, &y, &ts, &cfg, route, 1).unwrap();
            assert!(
                r.integration.success,
                "{route}: {:?}",
                r.integration.failure
            );
            assert!(r.integration.radii.iter().all(|r| *r <= cfg.abs_tolerance));
            outputs.push(r.integration);
        }
        for b in &outputs[1..] {
            for k in 0..ts.len() {
                let err = outputs[0].states[k]
                    .iter()
                    .zip(&b.states[k])
                    .map(|(a, b)| (a - b).abs())
                    .fold(0.0, f64::max);
                assert!(err <= outputs[0].radii[k] + b.radii[k] + 1e-14);
            }
        }
    }
}
#[test]
fn native_auto_never_averages_away_local_nonlinearity() {
    let m = Model::new(16, 2.56, 2.0, 0.4, 1.0).unwrap();
    let r = solve_trajectory(
        &m,
        &[0.5; 16],
        &[0.0, 0.2],
        &IntegrationConfig::default(),
        "auto",
        1,
    )
    .unwrap();
    assert!(r.integration.success);
    assert_eq!(r.integration.work.packet_calls, 0);
    assert!(r.integration.work.partial_ad_cells > 0);
}
#[test]
fn malformed_route_and_worker_fail_before_execution() {
    let m = Model::new(16, 2.56, 2.0, 0.4, 0.0).unwrap();
    assert!(
        solve_trajectory(
            &m,
            &[0.5; 16],
            &[0.0, 0.2],
            &IntegrationConfig::default(),
            "magic",
            1
        )
        .is_err()
    );
    assert!(
        solve_trajectory(
            &m,
            &[0.5; 16],
            &[0.0, 0.2],
            &IntegrationConfig::default(),
            "auto",
            999
        )
        .is_err()
    );
}
