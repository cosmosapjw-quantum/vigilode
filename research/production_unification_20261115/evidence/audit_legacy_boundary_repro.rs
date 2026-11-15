//! Baseline-only defect reproducer; zero exit means defects were observed.
use rodas5p_integrators::{scalar_linear_problem, integrate_fixed, IntegrationMethod};
fn main() {
    let (p, _) = scalar_linear_problem(0.0, 0.0);
    let nan = integrate_fixed(&p, (f64::NAN, 1.0), &[0.0], 0.1, IntegrationMethod::Sequential, None, None, 1e-9, 1e-6).unwrap();
    println!("legacy_nan_initial_time_success={} attempts={}", nan.success, nan.attempts);
    assert!(nan.success && nan.attempts == 0);
    let empty = integrate_fixed(&p, (0.0, 0.0), &[], 0.1, IntegrationMethod::Sequential, None, None, 1e-9, 1e-6).unwrap();
    println!("legacy_empty_initial_state_success={} final_dimension={}", empty.success, empty.y[0].len());
    assert!(empty.success && empty.y[0].is_empty());
    println!("audit_outcome=LEGACY_BOUNDARY_DEFECTS_REPRODUCED");
}
