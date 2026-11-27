use rodas5p_core::directed::Interval as Iv;
use vigilode::spatial::{DiffusionGrid, SpatialAdjointRoute, SpatialAdjointWorkspace};
#[test]
fn actual_mass_goal_uses_no_time_integration() {
    let g = DiffusionGrid::periodic_box([8, 4], [8., 4.], [0.125, 0.25]).unwrap();
    let n = g.dimension();
    let c = vec![1.; n];
    let y = (0..n).map(|i| i as f64 / 64.).collect::<Vec<_>>();
    let mut w = SpatialAdjointWorkspace::new(&g).unwrap();
    let mut out = vec![Iv { lo: 0., hi: 0. }; n];
    let r = w
        .certify_into(
            &c,
            &y,
            &y,
            0.125,
            SpatialAdjointRoute::ModeResidual,
            &mut out,
        )
        .unwrap();
    assert_eq!(r.actions, 1);
    assert_eq!(r.cells, 0);
    assert!(out.iter().all(|v| v.contains(1.)));
    assert!(r.signed_goal_error[0] <= 0. && r.signed_goal_error[1] >= 0.);
    assert!(r.width_upper <= 1e-12);
    println!("SPATIAL_RED_GREEN {:?}", r);
}
