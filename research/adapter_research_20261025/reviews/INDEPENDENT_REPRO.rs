extern crate self as vigilode;
#[path="../../../crates/vigilode/src/cofactor_chart.rs"] mod cofactor_chart;
#[path="../../../crates/vigilode/src/oscillatory_chart.rs"] mod oscillatory_chart;
pub use cofactor_chart::*;
pub use oscillatory_chart::*;
#[path="../../../crates/vigilode/tests/oscillatory_chart_contracts.rs"] mod existing_chirp_contracts;
#[test]
fn cofactor_scaled_duration_retains_exp_minus_two() {
    let c=ExponentialCofactor::new(-1e308,1.0,1e-308,1e-308).unwrap();
    assert!((c.value(1e308).unwrap()/(-2.0_f64).exp()-1.0).abs()<1e-14);
    assert!((c.primitive(1e308).unwrap()/(-(-2.0_f64).exp_m1())-1.0).abs()<1e-14);
}
#[test]
fn cofactor_underflowed_exponent_does_not_erase_finite_primitive() {
    let c=ExponentialCofactor::new(0.0,1.0,1e-300,1e8).unwrap();
    assert!((c.primitive(1e-100).unwrap()/1e-92-1.0).abs()<1e-12);
}
#[test]
fn smallest_nonzero_beta_has_constant_frequency_limit() {
    let c=ExponentialChirpChart::new(1.0,f64::from_bits(1),0.0,(0.0,0.1)).unwrap();
    assert!((c.phase(0.1).unwrap()-0.1).abs()<1e-15);
}
