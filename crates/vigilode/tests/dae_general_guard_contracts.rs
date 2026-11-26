#![cfg(feature = "polynomial-events")]
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn tr(c: f64, factors: Vec<F>) -> T {
    T {
        coefficient: c,
        factors,
    }
}
fn p(v: Vec<T>) -> P {
    P::new(v).unwrap()
}
fn y(i: usize) -> F {
    F::component(i, 0.)
}
fn t() -> F {
    F::clock(0.)
}
fn l(i: usize, c: f64) -> T {
    tr(c, vec![y(i)])
}
fn c(v: f64) -> T {
    tr(v, vec![])
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
            derivative_tol: 1e-6,
            ..Default::default()
        },
        tube_radius: 0.005,
        global_tol: 0.005,
        green: true,
    }
}
fn source() -> DaeSource {
    DaeSource::new(
        vec![p(vec![c(1.)])],
        vec![p(vec![
            l(1, 1.),
            tr(-1., vec![y(0), y(0)]),
            tr(-1., vec![t()]),
        ])],
    )
    .unwrap()
}
fn epoch() -> EvolutionEpoch {
    EvolutionEpoch {
        start: 0.,
        end: 2.,
        incoming_x_error: Some(0.),
    }
}
#[test]
fn algebraic_guard_is_physical() {
    let h = p(vec![l(1, 1.), c(-0.75)]);
    for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
        let run = evolve_to_guard(
            &source(),
            &[0.],
            &[0.],
            &[[-1., 8.]],
            epoch(),
            &opts(),
            policy(),
            solver,
            &h,
            1e-6,
            ThresholdRoute::IntervalNewton,
            GuardDerivativeRoute::Schur,
            GuardValueRoute::Transport,
        )
        .unwrap();
        let b = run.root().expect("physical z threshold");
        assert!(b[0] <= 0.5 && 0.5 <= b[1] && b[1] - b[0] <= 1e-6);
        assert!(run.numerical_end() < 2. && run.committed_until() <= b[0]);
        assert!(run.pre_query(b[1] + 1e-7).is_err());
        assert!(run.guard_slabs().iter().all(|s| s.velocity[0] > 0.));
    }
}

struct Fixture {
    name: &'static str,
    source: DaeSource,
    guard: P,
    x: Vec<f64>,
    z: Vec<f64>,
    branch: Vec<[f64; 2]>,
    incoming: f64,
    expected: f64,
}
fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "algebraic",
            source: source(),
            guard: p(vec![l(1, 1.), c(-0.75)]),
            x: vec![0.],
            z: vec![0.],
            branch: vec![[-1., 8.]],
            incoming: 0.,
            expected: 0.5,
        },
        Fixture {
            name: "coupled-cancellation",
            source: DaeSource::new(
                vec![p(vec![c(8.), tr(64., vec![t()])]), p(vec![c(1.)])],
                vec![
                    p(vec![l(2, 1.), l(0, 1.), tr(-1., vec![t()])]),
                    p(vec![l(3, 2.), l(1, 1.), l(2, -1.)]),
                ],
            )
            .unwrap(),
            guard: p(vec![l(0, 1.), l(2, 1.), c(-0.4375)]),
            x: vec![0., 0.],
            z: vec![0., 0.],
            branch: vec![[-128., 128.], [-128., 128.]],
            incoming: 2f64.powi(-12),
            expected: 0.4375,
        },
        Fixture {
            name: "pencil",
            source: DaeSource::new(
                vec![p(vec![c(1.), tr(1., vec![t()])])],
                vec![p(vec![l(1, 1.), tr(0.125, vec![t(), y(1)]), l(0, -1.)])],
            )
            .unwrap(),
            guard: p(vec![l(1, 1.), tr(0.25, vec![t()]), c(-0.625)]),
            x: vec![0.],
            z: vec![0.],
            branch: vec![[-1., 8.]],
            incoming: 0.,
            expected: 2. * 0.625
                / ((1.25_f64 - 0.625 / 8.)
                    + ((1.25_f64 - 0.625 / 8.).powi(2) + 4. * (17. / 32.) * 0.625).sqrt()),
        },
        Fixture {
            name: "nonlinear",
            source: source(),
            guard: p(vec![
                tr(1., vec![y(1), y(1)]),
                l(0, 1.),
                tr(0.25, vec![t()]),
                c(-3009. / 4096.),
            ]),
            x: vec![0.],
            z: vec![0.],
            branch: vec![[-1., 8.]],
            incoming: 0.,
            expected: 0.375,
        },
    ]
}
fn run(
    f: &Fixture,
    solver: EvolutionRoute,
    root: ThresholdRoute,
    derivative: GuardDerivativeRoute,
    value: GuardValueRoute,
) -> rodas5p_core::CoreResult<ThresholdRun> {
    evolve_to_guard(
        &f.source,
        &f.x,
        &f.z,
        &f.branch,
        EvolutionEpoch {
            incoming_x_error: Some(f.incoming),
            ..epoch()
        },
        &opts(),
        policy(),
        solver,
        &f.guard,
        1e-6,
        root,
        derivative,
        value,
    )
}
#[test]
fn schur_keeps_constraint_cancellation() {
    let fs = fixtures();
    let f = &fs[1];
    let direct = run(
        f,
        EvolutionRoute::Mass,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Tangent,
        GuardValueRoute::Transport,
    )
    .unwrap();
    assert!(direct.root().is_none() && direct.reason().is_some());
    let box_value = run(
        f,
        EvolutionRoute::Mass,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Box,
    )
    .unwrap();
    assert!(box_value.root().is_none() && box_value.reason().is_some());
    for route in [
        GuardDerivativeRoute::Schur,
        GuardDerivativeRoute::PreparedSchur,
    ] {
        let r = run(
            f,
            EvolutionRoute::Mass,
            ThresholdRoute::IntervalNewton,
            route,
            GuardValueRoute::Transport,
        )
        .unwrap();
        let b = r.root().expect("constraint cancellation root");
        assert!(b[0] <= f.expected && f.expected <= b[1] && b[1] - b[0] <= 1e-6);
        assert!(
            r.guard_slabs()
                .iter()
                .all(|s| s.velocity == [1., 1.] && s.reduced_lip == 0.)
        );
        if route == GuardDerivativeRoute::PreparedSchur {
            assert!(
                r.guard_slabs()
                    .iter()
                    .all(|s| s.prepared_work.prepared_owners > 0)
            );
        }
    }
}
#[test]
fn general_guard_fail_closed() {
    let f = &fixtures()[0];
    let h = p(vec![tr(
        -1.,
        vec![F::component(0, 0.4), F::component(0, 0.4)],
    )]);
    let r = evolve_to_guard(
        &f.source,
        &f.x,
        &f.z,
        &f.branch,
        epoch(),
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        &h,
        1e-6,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
    )
    .unwrap();
    assert!(r.root().is_none() && r.reason().is_some());
    let h = p(vec![l(9, 1.)]);
    assert!(
        evolve_to_guard(
            &f.source,
            &f.x,
            &f.z,
            &f.branch,
            epoch(),
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            &h,
            1e-6,
            ThresholdRoute::IntervalNewton,
            GuardDerivativeRoute::Schur,
            GuardValueRoute::Transport
        )
        .is_err()
    );
    let h = p(vec![c(1.), tr(1., vec![t()])]);
    let r = evolve_to_guard(
        &f.source,
        &f.x,
        &f.z,
        &f.branch,
        epoch(),
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        &h,
        1e-6,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
    )
    .unwrap();
    assert!(r.root().is_none() && r.reason().is_some() && r.committed_until() == 0.);
}
#[test]
#[ignore = "finite development campaign"]
fn training() {
    use serde_json::json;
    for f in fixtures() {
        for solver in [
            EvolutionRoute::Mass,
            EvolutionRoute::Reduced5,
            EvolutionRoute::Prepared5,
            EvolutionRoute::Scalar5,
            EvolutionRoute::WarpFolded,
        ] {
            for root in [
                ThresholdRoute::IntervalNewton,
                ThresholdRoute::DictionaryHint,
            ] {
                for derivative in [
                    GuardDerivativeRoute::Tangent,
                    GuardDerivativeRoute::Schur,
                    GuardDerivativeRoute::PreparedSchur,
                ] {
                    for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                        let row = match run(&f, solver, root, derivative, value) {
                            Ok(r) => {
                                json!({"case":f.name,"solver":solver,"root_route":root,"derivative":derivative,"value":value,"root":r.root(),"safe":r.committed_until(),"numerical_end":r.numerical_end(),"reason":r.reason(),"work":r.work,"driver_rhs":r.stats().counters.rhs_calls,"accepted_steps":r.stats().accepted_steps,"slabs":r.guard_slabs()})
                            }
                            Err(e) => {
                                json!({"case":f.name,"solver":solver,"root_route":root,"derivative":derivative,"value":value,"error":e.to_string()})
                            }
                        };
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}

#[test]
fn refined_schur_native_event_roots() {
    for f in fixtures() {
        for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            for derivative in [
                GuardDerivativeRoute::SchurRefine1,
                GuardDerivativeRoute::SchurRefine2,
                GuardDerivativeRoute::PreparedSchurRefine2,
            ] {
                let r = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    derivative,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                let b = r.root();
                if let Some(b) = b {
                    assert!(b[0] <= f.expected && f.expected <= b[1] && b[1] - b[0] <= 1e-6);
                    assert!(r.guard_slabs().iter().all(|s| s.velocity[0] > 0.));
                }
                let parent = if derivative == GuardDerivativeRoute::PreparedSchurRefine2 {
                    GuardDerivativeRoute::PreparedSchur
                } else {
                    GuardDerivativeRoute::Schur
                };
                let old = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    parent,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                if old.root().is_some() {
                    assert!(b.is_some(), "{} {solver:?} {derivative:?}", f.name);
                }
                println!(
                    "NATIVE_MUTATION {}",
                    serde_json::json!({"fixture":f.name,"solver":solver,"derivative":derivative,"root":b,"parent_root":old.root(),"reason":r.reason()})
                );
            }
        }
    }
}

#[test]
fn gs_schur_native_event_roots() {
    for f in fixtures() {
        for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            for derivative in [
                GuardDerivativeRoute::SchurGs1,
                GuardDerivativeRoute::SchurGs2,
                GuardDerivativeRoute::SchurGsAdaptive2,
                GuardDerivativeRoute::PreparedSchurGs2,
            ] {
                let r = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    derivative,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                let b = r.root();
                if let Some(b) = b {
                    assert!(b[0] <= f.expected && f.expected <= b[1] && b[1] - b[0] <= 1e-6);
                    assert!(r.guard_slabs().iter().all(|s| s.velocity[0] > 0.));
                }
                let parent = if derivative == GuardDerivativeRoute::PreparedSchurGs2 {
                    GuardDerivativeRoute::PreparedSchur
                } else {
                    GuardDerivativeRoute::Schur
                };
                let old = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    parent,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                if old.root().is_some() {
                    assert!(b.is_some(), "{} {solver:?} {derivative:?}", f.name);
                }
                println!(
                    "NATIVE_MUTATION {}",
                    serde_json::json!({"fixture":f.name,"solver":solver,"derivative":derivative,"root":b,"parent_root":old.root(),"reason":r.reason()})
                );
            }
        }
    }
}

#[test]
fn centered_schur_native_event_roots() {
    for f in fixtures() {
        for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            for derivative in [
                GuardDerivativeRoute::SchurCentered,
                GuardDerivativeRoute::SchurCenteredGs2,
                GuardDerivativeRoute::SchurCenteredGrouped,
                GuardDerivativeRoute::PreparedSchurCenteredGs2,
            ] {
                let r = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    derivative,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                let b = r.root();
                if let Some(b) = b {
                    assert!(b[0] <= f.expected && f.expected <= b[1] && b[1] - b[0] <= 1e-6);
                    assert!(r.guard_slabs().iter().all(|s| s.velocity[0] > 0.));
                }
                let parent = if derivative == GuardDerivativeRoute::PreparedSchurCenteredGs2 {
                    GuardDerivativeRoute::PreparedSchur
                } else {
                    GuardDerivativeRoute::Schur
                };
                let old = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    parent,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                if old.root().is_some() {
                    assert!(b.is_some(), "{} {solver:?} {derivative:?}", f.name);
                }
                println!(
                    "NATIVE_MUTATION {}",
                    serde_json::json!({"fixture":f.name,"solver":solver,"derivative":derivative,"root":b,"parent_root":old.root(),"reason":r.reason()})
                );
            }
        }
    }
}

#[test]
fn dual_schur_native_event_roots() {
    for f in fixtures() {
        for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            for derivative in [
                GuardDerivativeRoute::SchurDualCentered,
                GuardDerivativeRoute::SchurDualCenteredGs2,
                GuardDerivativeRoute::SchurDualGrouped,
                GuardDerivativeRoute::BoundedPreparedDualCenteredGs2,
                GuardDerivativeRoute::BoundedPreparedSchur,
            ] {
                let r = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    derivative,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                let b = r.root();
                if let Some(b) = b {
                    assert!(b[0] <= f.expected && f.expected <= b[1] && b[1] - b[0] <= 1e-6);
                    assert!(r.guard_slabs().iter().all(|s| s.velocity[0] > 0.));
                }
                let parent = if matches!(
                    derivative,
                    GuardDerivativeRoute::BoundedPreparedDualCenteredGs2
                        | GuardDerivativeRoute::BoundedPreparedSchur
                ) {
                    GuardDerivativeRoute::PreparedSchur
                } else {
                    GuardDerivativeRoute::Schur
                };
                let old = run(
                    &f,
                    solver,
                    ThresholdRoute::IntervalNewton,
                    parent,
                    GuardValueRoute::Transport,
                )
                .unwrap();
                if old.root().is_some() {
                    assert!(b.is_some(), "{} {solver:?} {derivative:?}", f.name);
                }
                println!(
                    "NATIVE_MUTATION {}",
                    serde_json::json!({"fixture":f.name,"solver":solver,"derivative":derivative,"root":b,"parent_root":old.root(),"reason":r.reason()})
                );
            }
        }
    }
}
