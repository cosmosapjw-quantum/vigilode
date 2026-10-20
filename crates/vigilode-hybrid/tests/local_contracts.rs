#[path = "../src/kernel.rs"]
mod kernel;
use kernel::*;
#[test]
fn supplied_model_validation() {
    assert!(Model::new(0, 1.0, 2.0, 0.4, 1.0).is_err());
    assert!(Model::new(8, f64::NAN, 2.0, 0.4, 1.0).is_err());
}
#[test]
fn local_ad_has_correct_nonlocal_mean_term() {
    let m = Model::new(2, 1.0, 2.0, 0.4, 1.0).unwrap();
    let x = m.jvp(&[0.2, 0.7], &[0.3, -0.5]).unwrap();
    assert!((x[0] + 1.606).abs() < 1e-12);
    assert!((x[1] - 2.578).abs() < 1e-12);
}
#[test]
fn exact_packet_keeps_all_initial_cells() {
    let m = Model::new(8, 1.0, 2.0, 0.4, 0.0).unwrap();
    let y = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
    assert_eq!(m.packet(&y, 0.0).unwrap(), y);
}
#[test]
fn quintic_controls_bind_endpoints() {
    let b = quintic_controls(0.2, 0.1, -0.3, -0.2, 0.4, 0.3, 0.1).unwrap();
    assert_eq!(b[0], 0.2);
    assert_eq!(b[5], 0.1);
    assert!((5.0 * (b[1] - b[0]) / 0.1 + 0.3).abs() < 1e-14);
}
