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
fn old(rad: f64, q: f64) -> ThresholdRun {
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
    .unwrap()
}
fn post() -> DaeSource {
    // z=X1, X1'=-z^2, X2'=z; genuine nonlinear algebraic-dependent RHS.
    DaeSource::new(
        vec![p(vec![tr(-1., vec![y(2), y(2)])]), p(vec![l(2, 1.)])],
        vec![p(vec![l(2, 1.), l(0, -1.)])],
    )
    .unwrap()
}
fn resets() -> Vec<P> {
    vec![
        p(vec![c(0.75), l(1, 1.), l(0, -1.)]),
        p(vec![c(1.), tr(1., vec![y(0), y(0)])]),
    ]
}
#[test]
fn vector_nonlinear_source_coupled_restart() {
    let r = 2f64.powi(-12);
    let q = 0.125;
    let pre = old(r, q);
    let root = pre.root().unwrap();
    for sync in [
        VectorSyncRoute::BoxPicard,
        VectorSyncRoute::SchurPicard,
        VectorSyncRoute::Centered,
        VectorSyncRoute::GreenMoment,
    ] {
        let run = pre
            .restart_vector(
                &resets(),
                &post(),
                &[0.875],
                &[[0.5, 1.5]],
                0.375,
                &opts(),
                policy(),
                EvolutionRoute::Mass,
                VectorResetPolicy {
                    sync,
                    derivative: GuardDerivativeRoute::Schur,
                    branch_route: InitRoute::Reuse,
                    subdivisions: 1,
                },
                &[root[1], 0.25, 0.375],
            )
            .unwrap();
        let po = run.post().expect("certified vector postflow");
        assert_eq!(po.driver().t[0], root[1]);
        assert_eq!(
            po.slabs()[0].certificate.as_ref().unwrap().incoming,
            run.incoming_radius.unwrap()
        );
        assert!(run.cells.len() >= 2 && run.cells.iter().all(|c| c.tube.is_some()));
        for i in 0..=64 {
            let tau = q - r + 2. * r * i as f64 / 64.;
            let x0 = 0.75 + tau;
            let y0 = 1. + q * q;
            for v in &po.queries {
                let dt = v.t - tau;
                let x = x0 / (1. + x0 * dt);
                let y = y0 + (x0 * dt).ln_1p();
                let err = (x - v.state[0])
                    .abs()
                    .max((y - v.state[1]).abs())
                    .max((x - v.state[2]).abs());
                assert!(err <= v.physical_error_upper.unwrap() + 1e-14);
            }
        }
    }
}

struct Fixture {
    name: &'static str,
    q: f64,
    rad: f64,
    clock: bool,
    reset: Vec<P>,
    post: DaeSource,
    seed: Vec<f64>,
    branch: Vec<[f64; 2]>,
    end: f64,
    cap: f64,
}
fn cases() -> Vec<Fixture> {
    let q = 0.125;
    let rad = 2f64.powi(-12);
    let cancel = vec![
        p(vec![
            c(1. + q),
            l(0, 1.),
            l(1, -1.),
            tr(1., vec![F::clock(q), F::clock(q)]),
        ]),
        p(vec![
            c(1. - q),
            l(1, 1.),
            l(0, -1.),
            tr(1., vec![F::clock(q), F::clock(q)]),
        ]),
    ];
    let sqrt_post = DaeSource::new(
        vec![p(vec![tr(-1., vec![y(2), y(2)])]), p(vec![l(2, 1.)])],
        vec![p(vec![tr(1., vec![y(2), y(2)]), l(0, -1.)])],
    )
    .unwrap();
    let pencil = DaeSource::new(
        vec![
            p(vec![l(2, -1.), tr(-0.125, vec![t(), y(2)])]),
            p(vec![l(2, 1.), tr(0.125, vec![t(), y(2)])]),
        ],
        vec![p(vec![l(2, 1.), tr(0.125, vec![t(), y(2)]), l(0, -1.)])],
    )
    .unwrap();
    let rank = DaeSource::new(
        vec![p(vec![l(0, -1.)]), p(vec![c(0.)])],
        vec![p(vec![
            tr(1., vec![y(2), y(2)]),
            l(0, -1.),
            c(1. - 2f64.powi(-11)),
        ])],
    )
    .unwrap();
    let excursion = DaeSource::new(
        vec![p(vec![l(0, -1.)]), p(vec![tr(-64., vec![F::clock(q)])])],
        vec![p(vec![l(2, 1.), l(1, -1.)])],
    )
    .unwrap();
    vec![
        Fixture {
            name: "nonlinear",
            q,
            rad,
            clock: false,
            reset: resets(),
            post: post(),
            seed: vec![0.875],
            branch: vec![[0.5, 1.5]],
            end: 0.375,
            cap: 0.003,
        },
        Fixture {
            name: "cancel",
            q,
            rad,
            clock: false,
            reset: cancel,
            post: post(),
            seed: vec![1.],
            branch: vec![[0.5, 1.5]],
            end: 0.375,
            cap: 1e-5,
        },
        Fixture {
            name: "negative-branch",
            q,
            rad,
            clock: false,
            reset: resets(),
            post: sqrt_post,
            seed: vec![-1.],
            branch: vec![[-1.25, -0.5]],
            end: 0.375,
            cap: 0.003,
        },
        Fixture {
            name: "exact-pencil",
            q,
            rad,
            clock: true,
            reset: vec![
                p(vec![
                    c(1.),
                    tr(1., vec![y(0), y(0)]),
                    tr(0.25, vec![y(1), y(1)]),
                ]),
                p(vec![c(1.), l(0, 1.), l(1, -1.)]),
            ],
            post: pencil,
            seed: vec![1.],
            branch: vec![[0.5, 1.5]],
            end: 0.375,
            cap: 0.003,
        },
        Fixture {
            name: "rank-loss",
            q: 0.5,
            rad,
            clock: false,
            reset: vec![p(vec![c(0.5), tr(1., vec![t()])]), p(vec![c(1.)])],
            post: rank,
            seed: vec![0.015625],
            branch: vec![[0.001, 0.04]],
            end: 0.75,
            cap: 0.003,
        },
        Fixture {
            name: "excursion",
            q,
            rad,
            clock: false,
            reset: vec![p(vec![c(1.)]), p(vec![c(1.)])],
            post: excursion,
            seed: vec![1.],
            branch: vec![[0.5, 1.5]],
            end: 0.15625,
            cap: 0.003,
        },
    ]
}
fn pre_case(f: &Fixture, route: EvolutionRoute) -> ThresholdRun {
    let guard = if f.clock {
        p(vec![tr(1., vec![t()]), c(-f.q)])
    } else {
        p(vec![l(0, 1.), c(-f.q)])
    };
    evolve_to_guard_pending(
        &source(),
        &[0.],
        &[0.],
        &[[-1., 5.]],
        EvolutionEpoch {
            start: 0.,
            end: 1.5,
            incoming_x_error: Some(f.rad),
        },
        &opts(),
        policy(),
        route,
        &guard,
        1e-3,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
        PendingPolicy {
            method: if f.clock {
                PendingMethod::Hull
            } else {
                PendingMethod::Piecewise
            },
            max_slabs: 8,
        },
    )
    .unwrap()
}
fn run(
    f: &Fixture,
    pre: &ThresholdRun,
    sync: VectorSyncRoute,
    subdivisions: usize,
    branch_route: InitRoute,
    route: EvolutionRoute,
) -> VectorRestartRun {
    let root = pre.root().unwrap();
    let grid = (0..=8)
        .map(|i| {
            if i == 8 {
                f.end
            } else {
                root[1] + (f.end - root[1]) * i as f64 / 8.
            }
        })
        .collect::<Vec<_>>();
    pre.restart_vector(
        &f.reset,
        &f.post,
        &f.seed,
        &f.branch,
        f.end,
        &opts(),
        policy(),
        route,
        VectorResetPolicy {
            sync,
            derivative: GuardDerivativeRoute::Schur,
            branch_route,
            subdivisions,
        },
        &grid,
    )
    .unwrap()
}
#[test]
fn intermediate_excursion_requires_flowpipe() {
    let fs = cases();
    let f = &fs[5];
    let pre = pre_case(f, EvolutionRoute::Mass);
    let r = run(
        f,
        &pre,
        VectorSyncRoute::Centered,
        4,
        InitRoute::Reuse,
        EvolutionRoute::Mass,
    );
    assert!(r.post().is_some());
    let earliest = f.q - f.rad;
    let peak = 1. + 32. * (earliest - f.q).powi(2);
    assert!(peak > 1.);
    let s = pre.root().unwrap()[1];
    assert!(s >= f.q + f.rad);
    let exact_sync = 1. + 32. * ((earliest - f.q).powi(2) - (s - f.q).powi(2));
    assert!(exact_sync <= 1. + 1e-14);
    let cell = r
        .cells
        .iter()
        .find(|c| c.times[0] <= earliest && earliest <= c.times[1])
        .unwrap();
    assert!(cell.tube.as_ref().unwrap().tube[1][1] >= peak);
}
#[test]
fn no_rank_loss_or_ambiguous_branch_commit() {
    let fs = cases();
    let f = &fs[4];
    let pre = pre_case(f, EvolutionRoute::Mass);
    for route in [
        VectorSyncRoute::BoxPicard,
        VectorSyncRoute::SchurPicard,
        VectorSyncRoute::Centered,
        VectorSyncRoute::GreenMoment,
    ] {
        let r = run(
            f,
            &pre,
            route,
            1,
            InitRoute::ScalarRoot,
            EvolutionRoute::Mass,
        );
        assert!(r.reason.is_some() && r.post().is_none() && r.post_diagnostic().is_none());
    }
    let f = &fs[2];
    let pre = pre_case(f, EvolutionRoute::Mass);
    let bad = pre
        .restart_vector(
            &f.reset,
            &f.post,
            &[-1.],
            &[[-1.25, 1.25]],
            f.end,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            VectorResetPolicy {
                sync: VectorSyncRoute::Centered,
                derivative: GuardDerivativeRoute::Schur,
                branch_route: InitRoute::ScalarRoot,
                subdivisions: 1,
            },
            &[],
        )
        .unwrap();
    assert!(bad.reason.is_some() && bad.post().is_none());
    assert!(
        pre.restart_vector(
            &f.reset[..1],
            &f.post,
            &f.seed,
            &f.branch,
            f.end,
            &opts(),
            policy(),
            EvolutionRoute::Mass,
            VectorResetPolicy {
                sync: VectorSyncRoute::Centered,
                derivative: GuardDerivativeRoute::Schur,
                branch_route: InitRoute::Reuse,
                subdivisions: 1
            },
            &[]
        )
        .is_err()
    );
}
#[test]
fn coupled_cancellation_and_exact_clock_warp() {
    let fs = cases();
    let f = &fs[1];
    let pre = pre_case(f, EvolutionRoute::Reduced5);
    let box_run = run(
        f,
        &pre,
        VectorSyncRoute::BoxPicard,
        1,
        InitRoute::Reuse,
        EvolutionRoute::Mass,
    );
    let narrow = run(
        f,
        &pre,
        VectorSyncRoute::Centered,
        4,
        InitRoute::Reuse,
        EvolutionRoute::Mass,
    );
    assert!(box_run.post().is_some() && narrow.post().is_some());
    let width = |r: &VectorRestartRun| {
        r.synchronized_x
            .as_ref()
            .unwrap()
            .iter()
            .map(|v| v[1] - v[0])
            .fold(0., f64::max)
    };
    assert!(width(&box_run) > 1e-5 && width(&narrow) <= 1e-5);
    let f = &fs[3];
    let pre = pre_case(f, EvolutionRoute::Mass);
    assert_eq!(pre.root(), Some([0.125, 0.125]));
    let r = run(
        f,
        &pre,
        VectorSyncRoute::GreenMoment,
        1,
        InitRoute::PreparedBatch,
        EvolutionRoute::WarpFolded,
    );
    let post = r.post().expect("exact-clock native WARP");
    assert!(post.warp.is_some() && post.work.warp_predictions > 0);
}
#[test]
#[ignore = "finite vector reset development campaign"]
fn training() {
    use serde_json::json;
    for f in cases() {
        let pre = pre_case(&f, EvolutionRoute::Mass);
        for sync in [
            VectorSyncRoute::BoxPicard,
            VectorSyncRoute::SchurPicard,
            VectorSyncRoute::Centered,
            VectorSyncRoute::GreenMoment,
        ] {
            for subdivisions in [1, 4] {
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
                        let r = run(&f, &pre, sync, subdivisions, branch_route, post_route);
                        let mut row = json!({"case":f.name,"q":f.q,"rad":f.rad,"clock":f.clock,"end":f.end,"cap":f.cap,"sync_route":sync,"subdivisions":subdivisions,"branch_route":branch_route,"post_route":post_route,
       "root":pre.root(),"safe":pre.committed_until(),"pre_attempts":pre.stats().attempts,"pre_rhs":pre.stats().counters.rhs_calls,"pre_work":pre.projection_work(),
       "cells":r.cells,"sync":r.synchronized_x,"radius":r.incoming_radius,"restart_branch":r.restart_branch,"reason":r.reason,"committed":r.post().is_some()});
                        if let Some(po) = r.post_diagnostic() {
                            row["post"] = json!({"global":po.global_qualified,"branch":po.branch_qualified,"pointwise":po.pointwise_qualified,"bound":po.max_physical_error,"attempts":po.driver().stats.attempts,"rhs":po.driver().stats.counters.rhs_calls,"work":po.work,"initial":po.initial,"queries":po.queries,"first_incoming":po.slabs().first().and_then(|v|v.certificate.as_ref().map(|c|c.incoming))});
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}
