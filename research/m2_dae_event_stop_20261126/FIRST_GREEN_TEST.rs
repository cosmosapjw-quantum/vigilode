#![cfg(feature = "polynomial-events")]
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn term(c: f64, factors: Vec<F>) -> T {
    T {
        coefficient: c,
        factors,
    }
}
fn p(v: Vec<T>) -> P {
    P::new(v).unwrap()
}
fn src() -> DaeSource {
    DaeSource::new(
        vec![p(vec![term(1., vec![])])],
        vec![p(vec![
            term(1., vec![F::component(1, 0.)]),
            term(-1., vec![F::component(0, 0.), F::component(0, 0.)]),
            term(-1., vec![F::clock(0.)]),
        ])],
    )
    .unwrap()
}
fn opts() -> Options {
    Options {
        rtol: 1e-9,
        atol: Atol::Scalar(1e-12),
        dense_output: true,
        interior_error: InteriorError::Off,
        ..Default::default()
    }
}
fn policy() -> EvolutionPolicy {
    EvolutionPolicy {
        init: InitPolicy {
            mode: InitMode::ProjectAlgebraic,
            state_tol: 1e-11,
            residual_tol: 1e-11,
            ..Default::default()
        },
        tube_radius: 0.005,
        global_tol: 0.005,
        green: true,
    }
}
#[test]
fn first_physical_crossing_stops_native_driver() {
    let ep = EvolutionEpoch {
        start: 0.,
        end: 2.,
        incoming_x_error: Some(0.),
    };
    for route in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
        let run = evolve_to_threshold(
            &src(),
            &[0.],
            &[0.],
            &[[-1., 8.]],
            ep,
            &opts(),
            policy(),
            route,
            0.731,
            1e-6,
            ThresholdRoute::Bisection,
        )
        .unwrap();
        let root = run.root.expect("physical crossing");
        assert!(root[0] <= 0.731 && 0.731 <= root[1] && root[1] - root[0] <= 1e-6);
        assert!(run.numerical_end < 2.);
        assert!(run.committed_until <= root[0]);
        assert!(run.pre_query(root[1] + 1e-7).is_err());
        assert!(run.pre_query(run.committed_until).is_ok());
        let full = evolve_epoch(
            &src(),
            &[0.],
            &[0.],
            &[[-1., 8.]],
            ep,
            &opts(),
            policy(),
            route,
            &[],
        )
        .unwrap();
        assert!(run.stats().counters.rhs_calls < full.driver().stats.counters.rhs_calls);
    }
}
