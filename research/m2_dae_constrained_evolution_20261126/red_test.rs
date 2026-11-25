#![cfg(feature = "polynomial-events")]
use vigilode::polynomial_events::{
    AffineFactor, DeclaredPolynomial, EventTerm, initialize_mass_problem, retro_dae_init::*,
};
use vigilode::{Atol, Options, Rodas5p};
fn p(terms: Vec<EventTerm>) -> DeclaredPolynomial {
    DeclaredPolynomial::new(terms).unwrap()
}
fn tr(c: f64, factors: Vec<AffineFactor>) -> EventTerm {
    EventTerm {
        coefficient: c,
        factors,
    }
}
fn y(i: usize) -> AffineFactor {
    AffineFactor::component(i, 0.)
}
fn analytic() -> DaeSource {
    DaeSource::new(
        vec![p(vec![tr(-1., vec![y(0)])])],
        vec![p(vec![
            tr(1., vec![y(1)]),
            tr(-1., vec![y(0), y(0)]),
            tr(-1., vec![AffineFactor::clock(0.)]),
        ])],
    )
    .unwrap()
}
#[test]
fn dense_constraint_projection() {
    let src = analytic();
    let init = initialize_mass_problem(
        &src,
        0.,
        &[1.],
        &[1.],
        &[[0., 2.]],
        InitPolicy::default(),
        InitRoute::Reuse,
    )
    .unwrap();
    let o = Options {
        rtol: 1e-5,
        atol: Atol::Scalar(1e-8),
        dense_output: true,
        ..Default::default()
    };
    let s = Rodas5p::solve(&init.problem, (0., 1.), &init.initial_state, &o).unwrap();
    assert!(s.is_success());
    let e = (0..=100)
        .map(|i| {
            let t = i as f64 / 100.;
            let v = s.at(t).unwrap();
            src.rhs_values(t, &v).unwrap()[1].abs()
        })
        .fold(0., f64::max);
    assert!(e <= 1e-12, "raw dense constraint defect {e:e}");
}
