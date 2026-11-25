#![cfg(feature = "polynomial-events")]
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::AdaptiveStepConfig;
use vigilode::polynomial_events::{
    AffineFactor, DeclaredPolynomial, EventTerm, retro_physical_event::*,
};
#[test]
fn physical_terminal_requires_no_later_trials() {
    let source = AffineSource::new(DenseMatrix::new(1, 1, vec![-1.]).unwrap(), vec![0.]).unwrap();
    let p = source
        .integrate(
            &[1.],
            3.,
            &AdaptiveStepConfig {
                rtol: 1e-8,
                atol: 1e-10,
                ..Default::default()
            },
        )
        .unwrap();
    let g = DeclaredPolynomial::new(vec![EventTerm {
        coefficient: 1.,
        factors: vec![AffineFactor::component(0, 0.5)],
    }])
    .unwrap();
    let r = p.certify(&g, PhysicalRoute::SourceAction, 1e-8).unwrap();
    assert!(r.qualified);
    let end = *p.segments().times().last().unwrap();
    assert!(end < 1., "physical terminal path continued to full horizon");
}
