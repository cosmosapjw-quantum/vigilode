#![cfg(feature = "polynomial-events")]
use vigilode::polynomial_events::{AffineFactor, DeclaredPolynomial, EventTerm, retro_dae_init::*};
fn term(c: f64, fs: Vec<AffineFactor>) -> EventTerm {
    EventTerm {
        coefficient: c,
        factors: fs,
    }
}
fn p(ts: Vec<EventTerm>) -> DeclaredPolynomial {
    DeclaredPolynomial::new(ts).unwrap()
}
#[test]
fn projection_keeps_x_and_computes_tangent() {
    let x = AffineFactor::component(0, 0.);
    let z = AffineFactor::component(1, 0.);
    let src = DaeSource::new(
        vec![p(vec![term(-1., vec![x.clone()])])],
        vec![p(vec![term(1., vec![z]), term(-1., vec![x.clone(), x])])],
    )
    .unwrap();
    let r = src
        .initialize(
            0.,
            &[1.],
            &[0.5],
            &[[0., 2.]],
            InitPolicy {
                mode: InitMode::ProjectAlgebraic,
            },
            InitRoute::Direct,
        )
        .unwrap();
    assert_eq!(r.x, vec![1.]);
    assert_eq!(r.z, vec![1.]);
    assert_eq!(r.zdot, vec![-2.]);
}
