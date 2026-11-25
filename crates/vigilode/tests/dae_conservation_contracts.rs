#![cfg(feature = "polynomial-events")]
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn opts(rt: f64) -> Options {
    Options {
        rtol: rt,
        atol: Atol::Scalar(rt * 0.001),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    }
}
fn policy() -> EvolutionPolicy {
    EvolutionPolicy {
        init: InitPolicy {
            mode: InitMode::ProjectAlgebraic,
            state_tol: 1e-11,
            residual_tol: 1e-11,
            derivative_tol: 1e-6,
            ..Default::default()
        },
        tube_radius: 1e-3,
        global_tol: 1e-3,
        green: true,
    }
}
fn mp(method: MultiGuardRoute) -> MultiGuardPolicy {
    MultiGuardPolicy {
        method,
        tolerance: 1e-8,
        root_route: ThresholdRoute::DictionaryHint,
        derivative: GuardDerivativeRoute::Schur,
        value: GuardValueRoute::Transport,
        pending: PendingPolicy {
            method: PendingMethod::PiecewiseProbe,
            max_slabs: 8,
        },
        max_slabs: 128,
    }
}
fn initial() -> RobertsonInitial {
    RobertsonInitial::exact([1., 0.]).unwrap()
}
#[test]
fn native_conservation_resolves_robertson_bootstrap() {
    let r = evolve_robertson(
        3e7,
        initial(),
        [0., 1.],
        &opts(1e-7),
        policy(),
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        &[0., 0.001, 0.1, 1.],
    )
    .unwrap();
    assert!(r.global_qualified && r.branch_qualified && r.pointwise_qualified);
    assert!(
        r.slabs()
            .iter()
            .all(|s| s.certificate.as_ref().unwrap().conservation.is_some())
    );
    assert!(r.max_physical_error.unwrap() < 1e-3);
    let q = r.queries.last().unwrap();
    assert!((q.state[0] - 0.9664597373330038).abs() <= q.physical_error_upper.unwrap() + 4e-12);
    assert!((q.state.iter().sum::<f64>() - 1.).abs() < 1e-12);
    let old = evolve_epoch(
        RobertsonModel::new(3e7).unwrap().source(),
        &[1., 0.],
        &[0.125],
        &[[-0.25, 1.25]],
        EvolutionEpoch {
            start: 0.,
            end: 1.,
            incoming_x_error: Some(0.),
        },
        &opts(1e-7),
        policy(),
        EvolutionRoute::Mass,
        &[],
    )
    .unwrap();
    assert!(!old.global_qualified && old.pointwise_qualified && old.branch_qualified);
    assert_eq!(r.driver().t, old.driver().t);
    assert_eq!(r.driver().y, old.driver().y);
}
#[test]
fn actual_native_clock_reset_carries_l1_authority() {
    let r = evolve_robertson_clock_reset(
        [3e7, 6e7],
        initial(),
        [0., 1. / 32.],
        [1. / 128., 1. / 256.],
        &opts(1e-7),
        policy(),
        EvolutionRoute::Mass,
        ConservationRoute::Integral4,
        mp(MultiGuardRoute::Correlated),
        &[1. / 256., 1. / 32.],
    )
    .unwrap();
    assert_eq!(r.pre().winner(), Some(1));
    assert_eq!(r.event_time, Some(1. / 256.));
    let post = r.post().expect("native certified post");
    assert!(post.global_qualified);
    assert_eq!(post.driver().t[0], 1. / 256.);
    assert_eq!(
        post.slabs()[0].certificate.as_ref().unwrap().incoming,
        r.incoming_l1.unwrap()
    );
    assert!(r.incoming_l1.unwrap() > 0.);
    assert!(r.pre().pre_query(1. / 128.).is_err());
}
