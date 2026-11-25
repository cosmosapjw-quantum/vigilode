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
        h0: 0.125,
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
        vec![p(vec![l(1, 1.), l(0, -1.), tr(-1., vec![t()])])],
    )
    .unwrap()
}
fn pre(rad: f64, route: EvolutionRoute) -> ThresholdRun {
    let h = p(vec![l(0, 1.), c(-0.5)]);
    evolve_to_guard_pending(
        &source(),
        &[0.],
        &[0.],
        &[[-1., 5.]],
        EvolutionEpoch {
            start: 0.,
            end: 1.5,
            incoming_x_error: Some(rad),
        },
        &opts(),
        policy(),
        route,
        &h,
        1e-3,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
        PendingPolicy {
            method: PendingMethod::Piecewise,
            max_slabs: 8,
        },
    )
    .unwrap()
}
fn reset() -> P {
    p(vec![
        c(1.5),
        l(0, 1.),
        l(1, -1.),
        tr(1., vec![F::clock(0.5), F::clock(0.5)]),
    ])
}
fn postg() -> Vec<P> {
    vec![p(vec![tr(1., vec![y(1), y(1)]), l(0, -1.)])]
}
#[test]
fn nonlinear_reset_uses_physical_authority_and_branch() {
    let rad = 2f64.powi(-12);
    let old = pre(rad, EvolutionRoute::Mass);
    let root = old.root().expect("physical root");
    let end = 0.75;
    let grid = [root[1], 0.625, end];
    for (route, seed, branch) in [
        (ResetEnvelopeRoute::Box, 1_f64, [0.75, 1.25]),
        (ResetEnvelopeRoute::Transport, 1_f64, [0.75, 1.25]),
        (ResetEnvelopeRoute::GreenMoment, -1_f64, [-1.25, -0.75]),
    ] {
        let r = old
            .restart_polynomial(
                &reset(),
                DampedResetFlow {
                    rate: 1.,
                    equilibrium: 0.,
                },
                postg(),
                &[seed],
                &[branch],
                end,
                &opts(),
                policy(),
                EvolutionRoute::Mass,
                route,
                GuardDerivativeRoute::Schur,
                InitRoute::Reuse,
                &grid,
            )
            .unwrap();
        let post = r.post().expect("certified post branch");
        assert!(post.global_qualified);
        assert_eq!(post.driver().t[0], root[1]);
        assert_eq!(
            post.slabs()[0].certificate.as_ref().unwrap().incoming,
            r.incoming_radius
        );
        assert!(r.branch_proof.as_ref().unwrap().defect < 1.);
        for j in 0..=64 {
            let delta = -rad + 2. * rad * j as f64 / 64.;
            let tau = 0.5 - delta;
            let x0 = 1.5 - tau + (tau - 0.5).powi(2);
            let synced = x0 * (-(root[1] - tau)).exp();
            assert!(r.synchronized_x[0] <= synced + 1e-14 && r.synchronized_x[1] >= synced - 1e-14);
            for q in &post.queries {
                let x = x0 * (-(q.t - tau)).exp();
                let z = seed.signum() * x.sqrt();
                assert!(
                    (x - q.state[0]).abs().max((z - q.state[1]).abs())
                        <= q.physical_error_upper.unwrap() + 1e-14
                );
            }
        }
    }
    assert_eq!(old.root(), Some(root));
}

fn state_reset() -> P {
    p(vec![
        c(0.5),
        tr(1., vec![y(0), y(0)]),
        tr(0.25, vec![y(1), y(1)]),
    ])
}
fn rank_reset() -> P {
    p(vec![c(0.5), tr(1., vec![t()])])
}
fn rank_constraints() -> Vec<P> {
    vec![p(vec![
        tr(1., vec![y(1), y(1)]),
        l(0, -1.),
        c(1. - 2f64.powi(-11)),
    ])]
}
#[test]
fn nominal_branch_is_not_uniform_branch_authority() {
    let rad = 2f64.powi(-12);
    let old = pre(rad, EvolutionRoute::Mass);
    let root = old.root().unwrap();
    let s = root[1];
    let branch = [[0.001, 0.04]];
    let out = old
        .restart_polynomial(
            &rank_reset(),
            DampedResetFlow {
                rate: 1.,
                equilibrium: 0.,
            },
            rank_constraints(),
            &[0.015625],
            &branch,
            0.75,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            ResetEnvelopeRoute::GreenMoment,
            GuardDerivativeRoute::Schur,
            InitRoute::ScalarRoot,
            &[],
        )
        .unwrap();
    let center = out.synchronized_x[0] * 0.5 + out.synchronized_x[1] * 0.5;
    let post_src = DaeSource::new(vec![p(vec![l(0, -1.)])], rank_constraints()).unwrap();
    assert!(
        post_src
            .initialize(
                s,
                &[center],
                &[0.015625],
                &branch,
                policy().init,
                InitRoute::ScalarRoot
            )
            .is_ok()
    );
    let early = 0.5 - rad;
    let x = (0.5 + early) * (-(s - early)).exp();
    assert!(x < 1. - 2f64.powi(-11));
    assert!(
        out.reason.is_some()
            && out.branch_proof.is_none()
            && out.post().is_none()
            && out.post_diagnostic().is_none()
    );
    assert_eq!(old.root(), Some(root));
    assert!(old.pre_query(root[1] + 1e-6).is_err());
}
#[test]
fn branch_ambiguity_and_wrong_authority_are_refused() {
    let old = pre(2f64.powi(-12), EvolutionRoute::Mass);
    let out = old
        .restart_polynomial(
            &reset(),
            DampedResetFlow {
                rate: 1.,
                equilibrium: 0.,
            },
            postg(),
            &[1.],
            &[[-1.25, 1.25]],
            0.75,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            ResetEnvelopeRoute::GreenMoment,
            GuardDerivativeRoute::Schur,
            InitRoute::ScalarRoot,
            &[],
        )
        .unwrap();
    assert!(out.reason.is_some() && out.post().is_none());
    let mut strict = policy();
    strict.init.mode = InitMode::Strict;
    assert!(
        old.restart_polynomial(
            &reset(),
            DampedResetFlow {
                rate: 1.,
                equilibrium: 0.
            },
            postg(),
            &[1.],
            &[[0.75, 1.25]],
            0.75,
            &opts(),
            strict,
            EvolutionRoute::Mass,
            ResetEnvelopeRoute::GreenMoment,
            GuardDerivativeRoute::Schur,
            InitRoute::Reuse,
            &[]
        )
        .is_err()
    );
    let invalid = p(vec![l(5, 1.)]);
    assert!(
        old.restart_polynomial(
            &invalid,
            DampedResetFlow {
                rate: 1.,
                equilibrium: 0.
            },
            postg(),
            &[1.],
            &[[0.75, 1.25]],
            0.75,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            ResetEnvelopeRoute::GreenMoment,
            GuardDerivativeRoute::Schur,
            InitRoute::Reuse,
            &[]
        )
        .is_err()
    );
}
#[test]
fn pending_multislab_event_is_covered() {
    let q = 0.125;
    let rad = 2f64.powi(-12);
    let old = evolve_to_guard_pending(
        &source(),
        &[0.],
        &[0.],
        &[[-1., 5.]],
        EvolutionEpoch {
            start: 0.,
            end: 1.5,
            incoming_x_error: Some(rad),
        },
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        &p(vec![l(0, 1.), c(-q)]),
        1e-3,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
        PendingPolicy {
            method: PendingMethod::Piecewise,
            max_slabs: 8,
        },
    )
    .unwrap();
    let root = old.root().unwrap();
    let r = p(vec![
        c(1. + q),
        l(0, 1.),
        l(1, -1.),
        tr(1., vec![F::clock(q), F::clock(q)]),
    ]);
    let out = old
        .restart_polynomial(
            &r,
            DampedResetFlow {
                rate: 1.,
                equilibrium: 0.,
            },
            postg(),
            &[1.],
            &[[0.75, 1.25]],
            0.375,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            ResetEnvelopeRoute::GreenMoment,
            GuardDerivativeRoute::Schur,
            InitRoute::PreparedBatch,
            &[root[1], 0.25, 0.375],
        )
        .unwrap();
    assert!(out.post().is_some());
    assert_eq!(out.slab_evidence.len(), 2);
    assert_eq!(out.slab_evidence[0].times[0], root[0]);
    assert_eq!(out.slab_evidence[1].times[1], root[1]);
    assert_eq!(out.slab_evidence[0].times[1], out.slab_evidence[1].times[0]);
}
struct Fixture {
    name: &'static str,
    src: DaeSource,
    x: Vec<f64>,
    z: Vec<f64>,
    pre_branch: Vec<[f64; 2]>,
    threshold: f64,
    rad: f64,
    reset: P,
    g: Vec<P>,
    seed: Vec<f64>,
    branch: Vec<[f64; 2]>,
    end: f64,
    cap: f64,
}
fn fixtures() -> Vec<Fixture> {
    let simple = |name, threshold, reset, g, seed, branch, end, cap| Fixture {
        name,
        src: source(),
        x: vec![0.],
        z: vec![0.],
        pre_branch: vec![[-1., 5.]],
        threshold,
        rad: 2f64.powi(-12),
        reset,
        g,
        seed,
        branch,
        end,
        cap,
    };
    let q = 0.125;
    vec![
        simple(
            "cancel-positive",
            0.5,
            reset(),
            postg(),
            vec![1.],
            vec![[0.75, 1.25]],
            0.75,
            1e-5,
        ),
        simple(
            "state-negative",
            0.5,
            state_reset(),
            postg(),
            vec![-1.],
            vec![[-1.25, -0.75]],
            0.75,
            0.003,
        ),
        simple(
            "time-pencil",
            0.5,
            state_reset(),
            vec![p(vec![l(1, 1.), tr(0.125, vec![t(), y(1)]), l(0, -1.)])],
            vec![1.],
            vec![[0.5, 1.5]],
            0.75,
            0.003,
        ),
        simple(
            "rank-loss",
            0.5,
            rank_reset(),
            rank_constraints(),
            vec![0.015625],
            vec![[0.001, 0.04]],
            0.75,
            0.003,
        ),
        simple(
            "boundary",
            q,
            p(vec![
                c(1. + q),
                l(0, 1.),
                l(1, -1.),
                tr(1., vec![F::clock(q), F::clock(q)]),
            ]),
            postg(),
            vec![1.],
            vec![[0.75, 1.25]],
            0.375,
            1e-5,
        ),
        Fixture {
            name: "coupled",
            src: DaeSource::new(
                vec![p(vec![c(1.)]), p(vec![c(2.)])],
                vec![
                    p(vec![l(2, 1.), l(0, -1.), tr(-1., vec![t()])]),
                    p(vec![l(3, 1.), l(1, -2.), l(2, 1.)]),
                ],
            )
            .unwrap(),
            x: vec![0., 0.],
            z: vec![0., 0.],
            pre_branch: vec![[-1., 5.], [-2., 8.]],
            threshold: 0.5,
            rad: 2f64.powi(-12),
            reset: p(vec![
                c(0.5),
                tr(1., vec![y(0), y(0)]),
                tr(0.25, vec![y(2), y(2)]),
            ]),
            g: vec![
                p(vec![tr(1., vec![y(1), y(1)]), l(0, -1.)]),
                p(vec![l(2, 2.), l(1, 1.), c(-1.)]),
            ],
            seed: vec![1., 0.],
            branch: vec![[0.75, 1.25], [-0.25, 0.25]],
            end: 0.75,
            cap: 0.003,
        },
    ]
}
#[test]
#[ignore = "finite development candidate campaign"]
fn training() {
    use serde_json::json;
    for f in fixtures() {
        for pre_route in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
            let old = evolve_to_guard_pending(
                &f.src,
                &f.x,
                &f.z,
                &f.pre_branch,
                EvolutionEpoch {
                    start: 0.,
                    end: 1.5,
                    incoming_x_error: Some(f.rad),
                },
                &opts(),
                policy(),
                pre_route,
                &p(vec![l(0, 1.), c(-f.threshold)]),
                1e-3,
                ThresholdRoute::DictionaryHint,
                GuardDerivativeRoute::Schur,
                GuardValueRoute::Transport,
                PendingPolicy {
                    method: PendingMethod::Piecewise,
                    max_slabs: 8,
                },
            );
            for envelope in [
                ResetEnvelopeRoute::Box,
                ResetEnvelopeRoute::Transport,
                ResetEnvelopeRoute::GreenMoment,
            ] {
                for derivative in [
                    GuardDerivativeRoute::Schur,
                    GuardDerivativeRoute::PreparedSchur,
                ] {
                    for branch_route in [
                        InitRoute::Reuse,
                        InitRoute::PreparedBatch,
                        InitRoute::ScalarRoot,
                    ] {
                        for post_route in [
                            EvolutionRoute::Mass,
                            EvolutionRoute::Prepared5,
                            EvolutionRoute::Scalar5,
                            EvolutionRoute::WarpFolded,
                        ] {
                            let mut row = json!({"case":f.name,"pre_route":pre_route,"envelope":envelope,"derivative":derivative,"branch_route":branch_route,"post_route":post_route,"rad":f.rad,"threshold":f.threshold,"cap":f.cap,"end":f.end});
                            match &old {
                                Ok(r) => {
                                    row["pre"] = json!({"root":r.root(),"reason":r.reason(),"safe":r.committed_until(),"driver_rhs":r.stats().counters.rhs_calls,"attempts":r.stats().attempts,"projection":r.projection_work(),"work":r.work,"guard_slabs":r.guard_slabs()});
                                    if let Some(root) = r.root() {
                                        let grid = (0..=8)
                                            .map(|i| {
                                                if i == 8 {
                                                    f.end
                                                } else {
                                                    root[1] + (f.end - root[1]) * i as f64 / 8.
                                                }
                                            })
                                            .collect::<Vec<_>>();
                                        match r.restart_polynomial(
                                            &f.reset,
                                            DampedResetFlow {
                                                rate: 1.,
                                                equilibrium: 0.,
                                            },
                                            f.g.clone(),
                                            &f.seed,
                                            &f.branch,
                                            f.end,
                                            &opts(),
                                            policy(),
                                            post_route,
                                            envelope,
                                            derivative,
                                            branch_route,
                                            &grid,
                                        ) {
                                            Ok(v) => {
                                                row["reset"] = json!({"event_time":v.event_time,"reset_x":v.reset_x,"sync_x":v.synchronized_x,"transition_x":v.transition_x,"incoming_radius":v.incoming_radius,"slabs":v.slab_evidence,"branch":v.branch_proof,"reason":v.reason,"committed":v.post().is_some()});
                                                if let Some(po) = v.post_diagnostic() {
                                                    row["post"] = json!({"global":po.global_qualified,"pointwise":po.pointwise_qualified,"branch":po.branch_qualified,"bound":po.max_physical_error,"driver_rhs":po.driver().stats.counters.rhs_calls,"attempts":po.driver().stats.attempts,"work":po.work,"initial":po.initial,"queries":po.queries,"first_incoming":po.slabs().first().and_then(|s|s.certificate.as_ref().map(|c|c.incoming)),"warp":po.warp});
                                                }
                                            }
                                            Err(e) => row["reset_error"] = json!(e.to_string()),
                                        }
                                    }
                                }
                                Err(e) => row["pre_error"] = json!(e.to_string()),
                            }
                            println!("CELL {}", row);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn exact_clock_warp_and_checkpoint_authority() {
    use serde_json::json;
    let rad = 2f64.powi(-12);
    let src = source();
    let o = opts();
    let po = policy();
    let b = [[-1., 5.]];
    let e = EvolutionEpoch {
        start: 0.,
        end: 1.5,
        incoming_x_error: Some(rad),
    };
    let h = p(vec![tr(1., vec![t()]), c(-0.125)]);
    let old = evolve_to_guard_pending(
        &src,
        &[0.],
        &[0.],
        &b,
        e,
        &o,
        po,
        EvolutionRoute::Mass,
        &h,
        1e-6,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
        PendingPolicy {
            method: PendingMethod::Hull,
            max_slabs: 8,
        },
    )
    .unwrap();
    assert_eq!(old.root(), Some([0.125, 0.125]));
    let r = p(vec![
        c(1.),
        tr(1., vec![y(0), y(0)]),
        tr(0.25, vec![y(1), y(1)]),
    ]);
    let g = vec![p(vec![l(1, 1.), tr(0.125, vec![t(), y(1)]), l(0, -1.)])];
    for route in [
        EvolutionRoute::Mass,
        EvolutionRoute::Prepared5,
        EvolutionRoute::Scalar5,
        EvolutionRoute::WarpFolded,
    ] {
        let out = old
            .restart_polynomial(
                &r,
                DampedResetFlow {
                    rate: 1.,
                    equilibrium: 0.,
                },
                g.clone(),
                &[1.],
                &[[0.5, 1.5]],
                0.375,
                &o,
                po,
                route,
                ResetEnvelopeRoute::GreenMoment,
                GuardDerivativeRoute::Schur,
                InitRoute::PreparedBatch,
                &[0.125, 0.25, 0.375],
            )
            .unwrap();
        let post = out.post().expect("exact-clock post branch");
        if route == EvolutionRoute::WarpFolded {
            assert!(post.warp.is_some() && post.work.warp_predictions > 0);
        }
        for i in 0..=64 {
            let delta = -rad + 2. * rad * i as f64 / 64.;
            let x = 0.125 + delta;
            let z = x + 0.125;
            let r0 = 1. + x * x + 0.25 * z * z;
            for q in &post.queries {
                let x = r0 * (-(q.t - 0.125)).exp();
                let z = x / (1. + q.t / 8.);
                assert!(
                    (x - q.state[0]).abs().max((z - q.state[1]).abs())
                        <= q.physical_error_upper.unwrap() + 1e-14
                );
            }
        }
        println!(
            "DIAG {}",
            json!({"route":route,"root":old.root(),"sync":out.synchronized_x,"bound":post.max_physical_error,"warp_predictions":post.work.warp_predictions,"prepared":out.branch_proof.as_ref().unwrap().work.prepared_owners,"queries":post.queries})
        );
    }
    let h = p(vec![tr(1., vec![t()]), c(-0.75)]);
    let cp = evolve_to_guard_checkpoint(
        &src,
        &[0.],
        &[0.],
        &b,
        EvolutionEpoch {
            incoming_x_error: Some(0.),
            ..e
        },
        &o,
        po,
        EvolutionRoute::Mass,
        &h,
        1e-6,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
        PendingPolicy {
            method: PendingMethod::Piecewise,
            max_slabs: 1,
        },
    )
    .unwrap();
    assert!(cp.first().root().is_none() && cp.retry().is_some());
    let authority = cp.event_authority().unwrap();
    let out = authority
        .restart_polynomial(
            &p(vec![c(1.), tr(0.25, vec![y(0), y(0)])]),
            DampedResetFlow {
                rate: 1.,
                equilibrium: 0.,
            },
            postg(),
            &[1.],
            &[[0.75, 1.5]],
            1.,
            &o,
            po,
            EvolutionRoute::Mass,
            ResetEnvelopeRoute::GreenMoment,
            GuardDerivativeRoute::Schur,
            InitRoute::Reuse,
            &[0.75, 1.],
        )
        .unwrap();
    assert!(out.post().is_some());
    assert_eq!(out.event_time, [0.75, 0.75]);
    println!(
        "CHECKPOINT_RESET {}",
        json!({"root":cp.root(),"attempts":cp.attempts_used(),"checkpoint":cp.checkpoint(),"reset_sync":out.synchronized_x,"bound":out.post().unwrap().max_physical_error})
    );
}
