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

#[test]
fn affine_graph_real_event_and_nonaffine_refusal() {
    for f in fixtures()
        .into_iter()
        .filter(|f| f.name == "coupled-cancellation" || f.name == "algebraic")
    {
        for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            let r = run(
                &f,
                solver,
                ThresholdRoute::IntervalNewton,
                GuardDerivativeRoute::SchurAffineGraph,
                GuardValueRoute::Transport,
            )
            .unwrap();
            if f.name == "coupled-cancellation" {
                let b = r.root().expect("original-source exact affine graph event");
                assert!(b[0] <= f.expected && f.expected <= b[1] && b[1] - b[0] <= 1e-6);
                assert!(r.guard_slabs().iter().all(|s| s.velocity[0] > 0.));
                println!(
                    "AFFINE_EVENT {}",
                    serde_json::json!({"fixture":f.name,"solver":solver,"root":b,"reason":r.reason()})
                );
            } else {
                assert!(r.root().is_none());
                println!(
                    "AFFINE_REFUSAL {}",
                    serde_json::json!({"fixture":f.name,"solver":solver,"reason":r.reason()})
                );
            }
        }
    }
}

#[test]
fn time_coupled_affine_graph_end_to_end() {
    let source = DaeSource::new(
        vec![p(vec![c(1.)])],
        vec![
            p(vec![
                l(1, 1.),
                l(0, -1.),
                tr(2., vec![t(), y(2)]),
                tr(-4., vec![t(), y(0)]),
            ]),
            p(vec![
                l(2, 1.),
                l(0, -2.),
                tr(-1., vec![t(), y(1)]),
                tr(1., vec![t(), y(0)]),
            ]),
        ],
    )
    .unwrap();
    let guard = p(vec![l(2, 1.), l(0, -2.), tr(0.25, vec![t()]), c(-0.015625)]);
    for solver in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
        for derivative in [
            GuardDerivativeRoute::SchurDualCenteredGs2,
            GuardDerivativeRoute::SchurAffineGraph,
        ] {
            let r = evolve_to_guard(
                &source,
                &[0.],
                &[0., 0.],
                &[[-1., 1.], [-1., 1.]],
                EvolutionEpoch {
                    start: 0.,
                    end: 0.125,
                    incoming_x_error: Some(0.),
                },
                &opts(),
                policy(),
                solver,
                &guard,
                1e-6,
                ThresholdRoute::IntervalNewton,
                derivative,
                GuardValueRoute::Transport,
            )
            .unwrap();
            if let Some(b) = r.root() {
                assert!(b[0] <= 0.0625 && 0.0625 <= b[1] && b[1] - b[0] <= 1e-6);
            }
            if derivative == GuardDerivativeRoute::SchurAffineGraph {
                for s in r.guard_slabs() {
                    assert_eq!(s.velocity, [0.25, 0.25]);
                }
            }
            println!(
                "COUPLED_EVENT {}",
                serde_json::json!({"solver":solver,"derivative":derivative,"root":r.root(),"reason":r.reason(),"slabs":r.guard_slabs().len()})
            );
        }
    }
}

#[test]
fn affine_graph_degree_four_exact_and_tiny_defect_refusal() {
    use rodas5p_core::directed::Interval as Iv;
    use rodas5p_integrators::retro_dae_evolution as de;
    let iv = |x| Iv::point(x).unwrap();
    let power = vec![vec![iv(0.), iv(0.125)], vec![iv(0.), iv(0.125)]];
    let guard = p(vec![l(1, 1.), l(0, -1.), tr(0.25, vec![t()])]);
    for perturb in [0., 2f64.powi(-40)] {
        let src = DaeSource::new(
            vec![p(vec![c(1.)])],
            vec![p(vec![
                l(1, 1.),
                l(0, -1.),
                tr(1., vec![t(), t(), t(), y(1)]),
                tr(-1., vec![t(), t(), t(), y(0)]),
                tr(perturb, vec![y(0), y(0)]),
            ])],
        )
        .unwrap();
        let cert = de::certify_slab(
            &src,
            [0., 0.125],
            &power,
            &[[-1., 1.]],
            0.01,
            Some(0.),
            true,
            1e6,
        )
        .unwrap();
        assert!(cert.global_qualified);
        let r = de::GuardProof::new(
            &src,
            &guard,
            &power,
            &cert,
            GuardDerivativeRoute::SchurAffineGraph,
            GuardValueRoute::Box,
        );
        if perturb == 0. {
            assert_eq!(r.unwrap().evidence.velocity, [0.25, 0.25]);
            println!("AFFINE_DEGREE4 exact source identity");
        } else {
            let err = r
                .err()
                .expect("nonzero source polynomial must refuse graph")
                .to_string();
            assert!(err.contains("affine graph identity not proved"), "{err}");
            println!("AFFINE_DEGREE4 perturb={perturb} refusal={err}");
        }
    }
}

#[test]
fn coupled_driver_diagnosis() {
    let source = DaeSource::new(
        vec![p(vec![c(1.)])],
        vec![
            p(vec![
                l(1, 1.),
                l(0, -1.),
                tr(2., vec![t(), y(2)]),
                tr(-4., vec![t(), y(0)]),
            ]),
            p(vec![
                l(2, 1.),
                l(0, -2.),
                tr(-1., vec![t(), y(1)]),
                tr(1., vec![t(), y(0)]),
            ]),
        ],
    )
    .unwrap();
    let branch = [[-1., 1.], [-1., 1.]];
    let epoch = EvolutionEpoch {
        start: 0.,
        end: 0.125,
        incoming_x_error: Some(0.),
    };
    for solver in [
        EvolutionRoute::Mass,
        EvolutionRoute::Reduced5,
        EvolutionRoute::Prepared5,
    ] {
        let (r, a) = evolve_epoch_audited(
            &source,
            &[0.],
            &[0., 0.],
            &branch,
            epoch,
            &opts(),
            policy(),
            solver,
            &[],
        );
        match r {
            Ok(r) => println!(
                "DRIVER_DIAG {}",
                serde_json::json!({"solver":solver,"status":format!("{:?}",r.driver().status),"last":r.driver().t.last(),"reason":r.reason,"stats":r.driver().stats,"audit":a})
            ),
            Err(e) => println!(
                "DRIVER_DIAG {}",
                serde_json::json!({"solver":solver,"error":e.to_string(),"audit":a})
            ),
        }
    }
    let prepared = initialize_reduced_problem(
        &source,
        0.,
        &[0.],
        &[0., 0.],
        &branch,
        policy(),
        EvolutionRoute::Reduced5,
    )
    .unwrap();
    for time in [0., 1e-8, 1e-6, 1e-5, 0.001, 0.01, 0.05, 0.125] {
        let mut rhs = [0.];
        let mut jac = [0.];
        let mut ft = [0.];
        let a = (prepared.problem.rhs_fn())(time, &[time], &mut rhs);
        let vigilode::Jacobian::Dense(j) = prepared.problem.jacobian_provider() else {
            panic!("dense")
        };
        let b = j(time, &[time], &mut jac);
        let c = prepared.problem.time_derivative_fn().unwrap()(time, &[time], &mut ft);
        println!(
            "CALLBACK_DIAG {}",
            serde_json::json!({"time":time,"rhs":format!("{a:?}"),"jac":format!("{b:?}"),"ft":format!("{c:?}"),"values":[rhs,jac,ft]})
        );
    }
}
