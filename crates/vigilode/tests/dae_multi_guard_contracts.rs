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
fn src() -> DaeSource {
    DaeSource::new(
        vec![p(vec![c(1.)])],
        vec![p(vec![l(1, 1.), l(0, -1.), tr(-1., vec![t()])])],
    )
    .unwrap()
}
fn xguard(q: f64) -> P {
    p(vec![l(0, 1.), c(-q)])
}
fn clock(q: f64) -> P {
    p(vec![tr(1., vec![t()]), c(-q)])
}
fn ep(rad: f64) -> EvolutionEpoch {
    EvolutionEpoch {
        start: 0.,
        end: 1.5,
        incoming_x_error: Some(rad),
    }
}
fn order(method: MultiGuardRoute) -> MultiGuardPolicy {
    MultiGuardPolicy {
        method,
        tolerance: 1e-3,
        root_route: ThresholdRoute::IntervalNewton,
        derivative: GuardDerivativeRoute::Schur,
        value: GuardValueRoute::Transport,
        pending: PendingPolicy {
            method: PendingMethod::Piecewise,
            max_slabs: 8,
        },
        max_slabs: 128,
    }
}
fn run(guards: &[P], rad: f64, method: MultiGuardRoute) -> MultiGuardRun {
    evolve_to_first_guard(
        &src(),
        &[0.],
        &[0.],
        &[[-1., 5.]],
        ep(rad),
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        guards,
        order(method),
    )
    .unwrap()
}
#[test]
fn overlap_is_not_a_tie_when_difference_is_certified() {
    let rad = 2f64.powi(-12);
    let q = 0.125;
    let sep = 2f64.powi(-14);
    let guards = [xguard(q + sep), xguard(q)];
    for m in [MultiGuardRoute::Independent, MultiGuardRoute::Shared] {
        let r = run(&guards, rad, m);
        assert!(r.winner().is_none() && r.event_authority().is_none());
    }
    let r = run(&guards, rad, MultiGuardRoute::Correlated);
    assert_eq!(r.winner(), Some(1));
    let a = r.event_authority().unwrap();
    let b = a.root().unwrap();
    assert!(b[0] <= q - rad && b[1] >= q + rad && b[1] - b[0] <= 1e-3);
    assert!(r.guards()[0].root.unwrap()[0] <= b[1]);
    assert!(
        r.comparisons()
            .iter()
            .any(|v| v.earlier == 1 && v.later == 0 && v.positive)
    );
    assert!(r.pre_query(b[1] + 1e-6).is_err());
}

#[test]
fn genuine_order_switch_and_exact_tie_refuse() {
    let q = 0.125;
    let rad = 2f64.powi(-12);
    for guards in [
        vec![xguard(q), clock(q)],
        vec![clock(q), p(vec![tr(2., vec![t()]), c(-2. * q)])],
        vec![xguard(q), xguard(q)],
    ] {
        for method in [
            MultiGuardRoute::Independent,
            MultiGuardRoute::Shared,
            MultiGuardRoute::Correlated,
        ] {
            let r = run(&guards, rad, method);
            assert!(r.winner().is_none() && r.event_authority().is_none() && r.reason().is_some());
            assert!(r.committed_until() <= q);
            assert!(r.pre_query(q + rad + 1e-6).is_err());
        }
    }
    // A nontransverse competitor cannot be silently treated as absent.
    let r = run(
        &[clock(0.5), p(vec![tr(-1., vec![F::clock(q), F::clock(q)])])],
        rad,
        MultiGuardRoute::Correlated,
    );
    assert!(r.winner().is_none() && r.event_authority().is_none());
}
#[test]
fn shared_native_stop_absence_and_budget() {
    let rad = 2f64.powi(-12);
    let guards = [xguard(0.5), xguard(0.125), xguard(3.)];
    let independent = run(&guards, rad, MultiGuardRoute::Independent);
    let shared = run(&guards, rad, MultiGuardRoute::Shared);
    assert_eq!(independent.winner(), Some(1));
    assert_eq!(shared.winner(), Some(1));
    assert_eq!(shared.work.native_runs, 1);
    assert_eq!(independent.work.native_runs, 3);
    assert!(
        shared.work.attempts < independent.work.attempts
            && shared.work.physical_certificates < independent.work.physical_certificates
    );
    let root = shared.event_authority().unwrap().root().unwrap();
    assert!(shared.guards()[2].root.is_none() && shared.guards()[2].safe >= root[1]);
    assert!(shared.event_authority().unwrap().numerical_end() < 1.5);
    assert!(shared.pre_query(0.05).is_ok() && shared.pre_query(root[1] + 1e-6).is_err());
    let absent = run(&[clock(2.), xguard(3.)], rad, MultiGuardRoute::Shared);
    assert!(
        absent.winner().is_none() && absent.reason().is_none() && absent.committed_until() == 1.5
    );
    assert!(absent.pre_query(1.5).is_ok());
    let mut mp = order(MultiGuardRoute::Shared);
    mp.max_slabs = 1;
    let capped = evolve_to_first_guard(
        &src(),
        &[0.],
        &[0.],
        &[[-1., 5.]],
        ep(rad),
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        &guards,
        mp,
    )
    .unwrap();
    assert!(
        capped.winner().is_none()
            && capped.reason().unwrap().contains("budget")
            && capped.work.physical_certificates == 1
    );
    let mut o = opts();
    o.max_steps = 1;
    let r = evolve_to_first_guard(
        &src(),
        &[0.],
        &[0.],
        &[[-1., 5.]],
        ep(rad),
        &o,
        policy(),
        EvolutionRoute::Mass,
        &guards,
        order(MultiGuardRoute::Independent),
    )
    .unwrap();
    assert!(r.winner().is_none() && r.work.attempts <= 1 && r.event_authority().is_none());
}
#[test]
fn only_selected_vector_transition_is_evaluated() {
    let q = 0.125;
    let rad = 2f64.powi(-12);
    let guards = [xguard(q + 2f64.powi(-14)), xguard(q)];
    let a = run(&guards, rad, MultiGuardRoute::Correlated);
    let post = DaeSource::new(
        vec![p(vec![tr(-1., vec![y(2), y(2)])]), p(vec![l(2, 1.)])],
        vec![p(vec![l(2, 1.), l(0, -1.)])],
    )
    .unwrap();
    let reset = [
        p(vec![c(0.75), l(1, 1.), l(0, -1.)]),
        p(vec![c(1.), tr(1., vec![y(0), y(0)])]),
    ];
    let invalid = [p(vec![l(7, 1.)])];
    let seed = [0.875];
    let branch = [[0.5, 1.5]];
    let pol = VectorResetPolicy {
        sync: VectorSyncRoute::Centered,
        derivative: GuardDerivativeRoute::Schur,
        branch_route: InitRoute::Reuse,
        subdivisions: 1,
    };
    let make = |r| OrderedVectorTransition {
        reset: r,
        source: &post,
        z_seed: &seed,
        branch: &branch,
        route: EvolutionRoute::Mass,
        policy: pol,
    };
    let s = a.event_authority().unwrap().root().unwrap()[1];
    let out = a
        .restart_ordered_vector(
            &[make(&invalid), make(&reset)],
            0.375,
            &opts(),
            policy(),
            &[s, 0.25, 0.375],
        )
        .unwrap();
    assert_eq!(out.event_index, 1);
    let po = out.run.post().expect("winner only");
    for i in 0..=64 {
        let tau = q - rad + 2. * rad * i as f64 / 64.;
        let x0 = 0.75 + tau;
        let y0 = 1. + q * q;
        for v in &po.queries {
            let dt = v.t - tau;
            let x = x0 / (1. + x0 * dt);
            let y = y0 + (x0 * dt).ln_1p();
            assert!(
                (x - v.state[0])
                    .abs()
                    .max((y - v.state[1]).abs())
                    .max((x - v.state[2]).abs())
                    <= v.physical_error_upper.unwrap() + 1e-14
            );
        }
    }
    let bad = a.restart_ordered_vector(
        &[make(&reset), make(&invalid)],
        0.375,
        &opts(),
        policy(),
        &[],
    );
    assert!(bad.is_err());
    assert_eq!(a.winner(), Some(1));
    assert!(a.pre_query(0.05).is_ok());
    let tied = run(&[clock(q), clock(q)], rad, MultiGuardRoute::Correlated);
    assert!(
        tied.restart_ordered_vector(&[make(&reset), make(&reset)], 0.375, &opts(), policy(), &[])
            .is_err()
    );
}
struct Fixture {
    name: &'static str,
    source: DaeSource,
    guards: Vec<P>,
    rad: f64,
    cap: usize,
}
fn fixtures() -> Vec<Fixture> {
    let q = 0.125;
    let d = 2f64.powi(-14);
    let rad = 2f64.powi(-12);
    let base = |name, guards, cap| Fixture {
        name,
        source: src(),
        guards,
        rad,
        cap,
    };
    vec![
        base("separated", vec![xguard(0.5), xguard(q), xguard(3.)], 128),
        base("overlap-fixed", vec![xguard(q + d), xguard(q)], 128),
        base("overlap-swaps", vec![xguard(q), clock(q)], 128),
        base(
            "exact-tie",
            vec![clock(q), p(vec![tr(2., vec![t()]), c(-2. * q)])],
            128,
        ),
        base("absent", vec![xguard(3.), clock(2.)], 128),
        base(
            "constraint-cancel",
            vec![
                p(vec![l(1, 1.), c(-2. * q - d)]),
                p(vec![l(0, 1.), tr(1., vec![t()]), c(-2. * q)]),
            ],
            128,
        ),
        base("slab-budget", vec![xguard(q + d), xguard(q)], 1),
        Fixture {
            name: "pencil",
            source: DaeSource::new(
                vec![p(vec![c(1.), tr(1., vec![t()])])],
                vec![p(vec![l(1, 1.), tr(0.125, vec![t(), y(1)]), l(0, -1.)])],
            )
            .unwrap(),
            guards: vec![p(vec![l(1, 1.), c(-q - d)]), p(vec![l(1, 1.), c(-q)])],
            rad,
            cap: 128,
        },
    ]
}
#[test]
#[ignore = "finite multi-guard development campaign"]
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
            for method in [
                MultiGuardRoute::Independent,
                MultiGuardRoute::Shared,
                MultiGuardRoute::Correlated,
            ] {
                for (root, pending) in [
                    (ThresholdRoute::IntervalNewton, PendingMethod::Piecewise),
                    (
                        ThresholdRoute::DictionaryHint,
                        PendingMethod::PiecewiseProbe,
                    ),
                ] {
                    for derivative in [
                        GuardDerivativeRoute::Schur,
                        GuardDerivativeRoute::PreparedSchur,
                    ] {
                        for value in [GuardValueRoute::Box, GuardValueRoute::Transport] {
                            let mp = MultiGuardPolicy {
                                method,
                                tolerance: 1e-3,
                                root_route: root,
                                derivative,
                                value,
                                pending: PendingPolicy {
                                    method: pending,
                                    max_slabs: 8,
                                },
                                max_slabs: f.cap,
                            };
                            let r = evolve_to_first_guard(
                                &f.source,
                                &[0.],
                                &[0.],
                                &[[-1., 5.]],
                                ep(f.rad),
                                &opts(),
                                policy(),
                                solver,
                                &f.guards,
                                mp,
                            );
                            let mut row = json!({"case":f.name,"solver":solver,"method":method,"root_route":root,"pending":pending,"derivative":derivative,"value":value,"rad":f.rad,"cap":f.cap});
                            match r {
                                Ok(r) => {
                                    row["winner"] = json!(r.winner());
                                    row["safe"] = json!(r.committed_until());
                                    row["reason"] = json!(r.reason());
                                    row["guards"] = json!(r.guards());
                                    row["comparisons"] = json!(r.comparisons());
                                    row["work"] = json!(r.work);
                                    row["root"] = json!(r.event_authority().and_then(|a| a.root()));
                                    if let Some(a) = r.event_authority() {
                                        row["numerical_end"] = json!(a.numerical_end());
                                        row["projection"] = json!(a.projection_work());
                                    }
                                }
                                Err(e) => row["error"] = json!(e.to_string()),
                            }
                            println!("CELL {}", row);
                        }
                    }
                }
            }
        }
    }
}
