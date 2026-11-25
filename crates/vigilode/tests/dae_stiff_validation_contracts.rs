#![cfg(feature = "polynomial-events")]
use serde_json::{Value, json};
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
fn clock(q: f64) -> P {
    p(vec![tr(1., vec![t()]), c(-q)])
}
fn robertson(k: f64) -> DaeSource {
    DaeSource::new(
        vec![
            p(vec![l(0, -0.04), tr(1e4, vec![y(1), y(2)])]),
            p(vec![
                l(0, 0.04),
                tr(-1e4, vec![y(1), y(2)]),
                tr(-k, vec![y(1), y(1)]),
            ]),
        ],
        vec![p(vec![l(0, 1.), l(1, 1.), l(2, 1.), c(-1.)])],
    )
    .unwrap()
}
fn sqrt_stiff() -> DaeSource {
    DaeSource::new(
        vec![p(vec![tr(-2048., vec![y(1), F::component(1, 1.)])])],
        vec![p(vec![tr(1., vec![y(1), y(1)]), l(0, -1.)])],
    )
    .unwrap()
}
fn pencil() -> DaeSource {
    DaeSource::new(
        vec![p(vec![c(1024.), l(1, -1024.), tr(-128., vec![t(), y(1)])])],
        vec![p(vec![l(1, 1.), tr(0.125, vec![t(), y(1)]), l(0, -1.)])],
    )
    .unwrap()
}
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
fn policy(green: bool) -> EvolutionPolicy {
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
        green,
    }
}
fn rec(r: &EvolutionRun) -> Value {
    json!({"pointwise":r.pointwise_qualified,"branch":r.branch_qualified,"global":r.global_qualified,"bound":r.max_physical_error,"reason":r.reason,"driver_success":r.driver().is_success(),"driver_end":r.driver().t.last(),"attempts":r.driver().stats.attempts,"counters":r.driver().stats.counters,"work":r.work,"initial":r.initial,"slabs":r.slabs(),"queries":r.queries,"accepted":r.accepted,"warp":r.warp})
}
fn grid(a: f64, b: f64) -> Vec<f64> {
    let mut v = (0..=64)
        .map(|i| {
            if i == 64 {
                b
            } else {
                a + (b - a) * i as f64 / 64.
            }
        })
        .collect::<Vec<_>>();
    for i in 0..=32 {
        let t = a + (b - a) * 10_f64.powf(-6. + 6. * i as f64 / 32.);
        if t > a && t < b {
            v.push(t);
        }
    }
    v.sort_by(f64::total_cmp);
    v.dedup();
    v
}
const ROUTES: [EvolutionRoute; 6] = [
    EvolutionRoute::Mass,
    EvolutionRoute::Reduced3,
    EvolutionRoute::Reduced5,
    EvolutionRoute::Prepared5,
    EvolutionRoute::Scalar5,
    EvolutionRoute::WarpFolded,
];
#[test]
#[ignore = "fixed stiff development matrix"]
fn training() {
    for (name, source, x, z, branch, end) in [
        (
            "robertson",
            robertson(3e7),
            vec![1., 0.],
            vec![0.125],
            vec![[-0.25, 1.25]],
            1.,
        ),
        (
            "sqrt-stiff",
            sqrt_stiff(),
            vec![289. / 256.],
            vec![17. / 16.],
            vec![[0.8, 1.3]],
            1. / 32.,
        ),
        (
            "pencil-stiff",
            pencil(),
            vec![17. / 16.],
            vec![1.],
            vec![[0.5, 1.5]],
            1. / 32.,
        ),
    ] {
        for route in ROUTES {
            for rt in [1e-5, 1e-7, 1e-9] {
                for green in [false, true] {
                    let mut row =
                        json!({"case":name,"route":route,"rtol":rt,"green":green,"end":end});
                    match evolve_epoch(
                        &source,
                        &x,
                        &z,
                        &branch,
                        EvolutionEpoch {
                            start: 0.,
                            end,
                            incoming_x_error: Some(0.),
                        },
                        &opts(rt),
                        policy(green),
                        route,
                        &grid(0., end),
                    ) {
                        Ok(r) => row["run"] = rec(&r),
                        Err(e) => row["error"] = json!(e.to_string()),
                    }
                    println!("CELL {}", row);
                }
            }
        }
    }
}
#[test]
#[ignore = "fixed exact-clock segmented and frozen chain comparison"]
fn hybrid() {
    let tau = 1. / 256.;
    let end = 1. / 32.;
    let source = robertson(3e7);
    let post = robertson(6e7);
    let b = [[-0.25, 1.25]];
    for route in ROUTES {
        for rt in [1e-7, 1e-9] {
            let mut row = json!({"route":route,"rtol":rt,"tau":tau,"end":end});
            let pre = evolve_epoch(
                &source,
                &[1., 0.],
                &[0.125],
                &b,
                EvolutionEpoch {
                    start: 0.,
                    end: tau,
                    incoming_x_error: Some(0.),
                },
                &opts(rt),
                policy(true),
                route,
                &grid(0., tau),
            );
            match pre {
                Ok(r) => {
                    if let Some(q) = r.queries.last().filter(|q| q.t == tau) {
                        let x = [q.state[0], q.state[1] * 0.5];
                        let z = [1. - x[0] - x[1]];
                        row["reset_center"] = json!(x);
                        // Offline research certificate supplies incoming radius separately.
                        // Never treat this provisional numerical post integration as a native
                        // certified hybrid restart. None explicitly means unknown.
                        match evolve_epoch(
                            &post,
                            &x,
                            &z,
                            &b,
                            EvolutionEpoch {
                                start: tau,
                                end,
                                incoming_x_error: None,
                            },
                            &opts(rt),
                            policy(true),
                            route,
                            &grid(tau, end),
                        ) {
                            Ok(v) => row["post"] = rec(&v),
                            Err(e) => row["post_error"] = json!(e.to_string()),
                        }
                    }
                    row["pre"] = rec(&r);
                }
                Err(e) => row["error"] = json!(e.to_string()),
            }
            let rp = VectorResetPolicy {
                sync: VectorSyncRoute::GreenMoment,
                derivative: GuardDerivativeRoute::Schur,
                branch_route: InitRoute::Reuse,
                subdivisions: 1,
            };
            let modes = [
                ChainMode {
                    source: source.clone(),
                    z_seed: vec![0.125],
                    branch: b.to_vec(),
                    route,
                    guards: vec![clock(tau), clock(2. * tau)],
                    transitions: vec![
                        ChainTransition {
                            target: 1,
                            reset: vec![p(vec![l(0, 1.)]), p(vec![l(1, 0.5)])],
                            policy: rp,
                        },
                        ChainTransition {
                            target: 1,
                            reset: vec![p(vec![l(0, 1.)]), p(vec![l(1, 0.5)])],
                            policy: rp,
                        },
                    ],
                },
                ChainMode {
                    source: post.clone(),
                    z_seed: vec![0.],
                    branch: b.to_vec(),
                    route,
                    guards: vec![],
                    transitions: vec![],
                },
            ];
            let mp = MultiGuardPolicy {
                method: MultiGuardRoute::Correlated,
                tolerance: 1e-8,
                root_route: ThresholdRoute::DictionaryHint,
                derivative: GuardDerivativeRoute::Schur,
                value: GuardValueRoute::Transport,
                pending: PendingPolicy {
                    method: PendingMethod::PiecewiseProbe,
                    max_slabs: 8,
                },
                max_slabs: 128,
            };
            let budget = ChainBudget {
                native_attempts: 4096,
                scan_slabs: 4096,
                transition_attempts: 2,
                reset_cells: 64,
                gap_checks: 256,
                min_dwell: 1e-5,
            };
            match evolve_hybrid_chain(
                &modes,
                0,
                &[1., 0.],
                &[0.125],
                EvolutionEpoch {
                    start: 0.,
                    end,
                    incoming_x_error: Some(0.),
                },
                &opts(rt),
                policy(true),
                mp,
                budget,
                GapGuardRoute::Schur,
            ) {
                Ok(v) => {
                    row["chain"] = json!({"stop":v.stop(),"reason":v.reason(),"work":v.work(),"transitions":v.transitions(),"epochs":v.epochs().iter().map(|e|json!({"mode":e.mode,"start":e.start,"safe":e.safe,"reason":e.ordered().and_then(|r|r.reason())})).collect::<Vec<_>>() })
                }
                Err(e) => row["chain_error"] = json!(e.to_string()),
            }
            println!("HYBRID {}", row);
        }
    }
}
