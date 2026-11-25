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
fn ep(rad: f64, end: f64) -> EvolutionEpoch {
    EvolutionEpoch {
        start: 0.,
        end,
        incoming_x_error: Some(rad),
    }
}
fn guard() -> P {
    p(vec![l(1, 1.), c(-0.25)])
}
#[test]
fn boundary_event_pending_then_certified() {
    let o = opts();
    let s = source();
    let h = guard();
    let b = [[-1., 5.]];
    for route in [EvolutionRoute::Mass, EvolutionRoute::Reduced5] {
        let legacy = evolve_to_guard(
            &s,
            &[0.],
            &[0.],
            &b,
            ep(0., 2.),
            &o,
            policy(),
            route,
            &h,
            1e-6,
            ThresholdRoute::IntervalNewton,
            GuardDerivativeRoute::Schur,
            GuardValueRoute::Transport,
        )
        .unwrap();
        assert!(legacy.root().is_none() && legacy.reason().unwrap().contains("uncertain right"));
        for method in [
            PendingMethod::Hull,
            PendingMethod::Piecewise,
            PendingMethod::PiecewiseProbe,
        ] {
            let r = evolve_to_guard_pending(
                &s,
                &[0.],
                &[0.],
                &b,
                ep(0., 2.),
                &o,
                policy(),
                route,
                &h,
                1e-6,
                ThresholdRoute::DictionaryHint,
                GuardDerivativeRoute::Schur,
                GuardValueRoute::Transport,
                PendingPolicy {
                    method,
                    max_slabs: 8,
                },
            )
            .unwrap();
            let root = r.root().expect("boundary event");
            assert!(root[0] <= 0.125 && 0.125 <= root[1] && root[1] - root[0] <= 1e-6);
            assert!(r.numerical_end() > 0.125 && r.numerical_end() < 2.);
            assert!(r.committed_until() <= root[0]);
            assert!(r.pre_query(root[1] + 1e-7).is_err());
            let pending = r.pending_report().unwrap();
            assert!(pending.records.len() >= 2 && pending.records[0].safe_before == 0.);
            assert!(pending.records.iter().all(|v| v.safe_before == 0.));
            assert_eq!(r.stats().accepted_steps, r.work.inspected);
        }
    }
}

fn pending(
    method: PendingMethod,
    cap: usize,
    rad: f64,
    end: f64,
    h: &P,
    route: EvolutionRoute,
) -> ThresholdRun {
    evolve_to_guard_pending(
        &source(),
        &[0.],
        &[0.],
        &[[-1., 5.]],
        ep(rad, end),
        &opts(),
        policy(),
        route,
        h,
        1e-6,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
        PendingPolicy {
            method,
            max_slabs: cap,
        },
    )
    .unwrap()
}
#[test]
fn cap_horizon_and_uncertainty_floor() {
    let h = guard();
    let r = pending(
        PendingMethod::Piecewise,
        1,
        0.,
        2.,
        &h,
        EvolutionRoute::Mass,
    );
    assert!(r.root().is_none() && r.reason().is_some() && r.committed_until() == 0.);
    assert_eq!(r.pending_report().unwrap().resolution, Some("slab-cap"));
    assert!(r.pre_query(0.125).is_err());
    let r = pending(PendingMethod::Hull, 8, 0., 0.125, &h, EvolutionRoute::Mass);
    assert!(r.root().is_none() && r.reason().is_some() && r.committed_until() == 0.);
    assert_eq!(
        r.pending_report().unwrap().resolution,
        Some("horizon-unresolved")
    );
    let rad = 2f64.powi(-12);
    for method in [
        PendingMethod::Hull,
        PendingMethod::Piecewise,
        PendingMethod::PiecewiseProbe,
    ] {
        let r = pending(method, 8, rad, 2., &h, EvolutionRoute::Mass);
        let p = r.pending_report().unwrap();
        assert!(r.root().is_none() && r.reason().is_some());
        assert_eq!(p.resolution, Some("width-unresolved"));
        let b = p.candidate.unwrap();
        assert!(b[0] <= 0.125 - rad / 2. && b[1] >= 0.125 + rad / 2.);
        assert!(r.committed_until() <= 0.125 - rad / 2.);
        assert!(r.pre_query(0.125).is_err());
    }
}
#[test]
fn piecewise_derivative_reduces_dependency() {
    let h = p(vec![l(0, 1.), tr(4., vec![t(), t()]), c(-3. / 16.)]);
    let rad = 5. * 2f64.powi(-23);
    let a = pending(PendingMethod::Hull, 8, rad, 2., &h, EvolutionRoute::Mass);
    assert!(a.root().is_none());
    let b = pending(
        PendingMethod::Piecewise,
        8,
        rad,
        2.,
        &h,
        EvolutionRoute::Mass,
    );
    let r = b.root().expect("piecewise should qualify");
    let exact =
        |delta: f64| 2. * (3. / 16. - delta) / (1. + (1. + 16. * (3. / 16. - delta)).sqrt());
    assert!(r[0] <= exact(rad) && r[1] >= exact(-rad) && r[1] - r[0] <= 1e-6);
    assert!(
        a.pending_report().unwrap().candidate.unwrap()[1]
            - a.pending_report().unwrap().candidate.unwrap()[0]
            > 1e-6
    );
}
#[test]
fn checkpoint_restart_preserves_old_prefix_and_charges_both_runs() {
    let src = source();
    let b = [[-1., 5.]];
    let o = opts();
    let po = policy();
    let full = evolve_epoch(
        &src,
        &[0.],
        &[0.],
        &b,
        ep(0., 2.),
        &o,
        po,
        EvolutionRoute::Mass,
        &[],
    )
    .unwrap();
    assert!(full.driver().t.len() >= 4);
    let target = full.driver().t[2];
    let checkpoint = full.driver().t[1];
    let h = p(vec![tr(1., vec![t()]), c(-target)]);
    let r = evolve_to_guard_checkpoint(
        &src,
        &[0.],
        &[0.],
        &b,
        ep(0., 2.),
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
    assert!(r.first().root().is_none() && r.first().reason().is_some());
    let cp = r.checkpoint().expect("checkpoint");
    assert_eq!(cp.time, checkpoint);
    assert_eq!(
        r.retry()
            .unwrap()
            .pending_report()
            .unwrap()
            .policy
            .max_slabs,
        1
    );
    let root = r.root().expect("checkpoint rescue");
    assert!(root[0] <= target && root[1] >= target && root[1] - root[0] <= 1e-6);
    assert!(cp.radius >= 0.);
    assert_eq!(
        r.attempts_used(),
        r.first().stats().attempts + r.retry().unwrap().stats().attempts
    );
    assert!(r.attempts_used() <= o.max_steps);
    assert!(
        r.pre_query(checkpoint / 2.).is_ok() && r.pre_query((checkpoint + root[0]) / 2.).is_ok()
    );
    assert!(r.pre_query(root[1] + 1e-6).is_err());
}

#[test]
fn checkpoint_cannot_erase_real_uncertainty_or_budget() {
    let s = source();
    let b = [[-1., 5.]];
    let h = guard();
    let rad = 2f64.powi(-12);
    let r = evolve_to_guard_checkpoint(
        &s,
        &[0.],
        &[0.],
        &b,
        ep(rad, 2.),
        &opts(),
        policy(),
        EvolutionRoute::Mass,
        &h,
        1e-6,
        ThresholdRoute::IntervalNewton,
        GuardDerivativeRoute::Schur,
        GuardValueRoute::Transport,
        PendingPolicy {
            method: PendingMethod::Piecewise,
            max_slabs: 8,
        },
    )
    .unwrap();
    assert!(r.root().is_none() && r.event_authority().is_none());
    assert!(r.checkpoint().unwrap().radius >= rad);
    assert!(r.retry().unwrap().root().is_none());
    assert!(r.pre_query(0.125).is_err());
    let mut o = opts();
    o.max_steps = 2;
    let r = evolve_to_guard_checkpoint(
        &s,
        &[0.],
        &[0.],
        &b,
        ep(0., 2.),
        &o,
        policy(),
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
    assert!(r.root().is_none() && r.retry().is_none() && r.attempts_used() == 2);
    assert!(r.retry_error().unwrap().contains("budget"));
    let initial = p(vec![l(0, 1.)]);
    let r = pending(
        PendingMethod::Piecewise,
        8,
        0.,
        2.,
        &initial,
        EvolutionRoute::Mass,
    );
    assert!(
        r.root().is_none()
            && r.pending_report().unwrap().blocked_initial
            && r.pre_query(0.).is_err()
    );
}
struct Fixture {
    name: &'static str,
    src: DaeSource,
    guard: P,
    x: Vec<f64>,
    z: Vec<f64>,
    branch: Vec<[f64; 2]>,
    rad: f64,
    cap: usize,
}
fn fixtures() -> Vec<Fixture> {
    let linear = |name, guard, rad, cap| Fixture {
        name,
        src: source(),
        guard,
        x: vec![0.],
        z: vec![0.],
        branch: vec![[-1., 5.]],
        rad,
        cap,
    };
    vec![
        linear("boundary", guard(), 0., 8),
        linear(
            "variable",
            p(vec![l(0, 1.), tr(4., vec![t(), t()]), c(-3. / 16.)]),
            5. * 2f64.powi(-23),
            8,
        ),
        linear("physical-floor", guard(), 2f64.powi(-12), 8),
        linear(
            "checkpoint-cap",
            p(vec![tr(1., vec![t()]), c(-0.75)]),
            0.,
            1,
        ),
        Fixture {
            name: "nonlinear-z",
            src: DaeSource::new(
                vec![p(vec![c(1.)])],
                vec![p(vec![
                    l(1, 1.),
                    tr(-1., vec![y(0), y(0)]),
                    tr(-1., vec![t()]),
                ])],
            )
            .unwrap(),
            guard: p(vec![l(1, 1.), c(-9. / 64.)]),
            x: vec![0.],
            z: vec![0.],
            branch: vec![[-1., 8.]],
            rad: 0.,
            cap: 8,
        },
        Fixture {
            name: "coupled",
            src: DaeSource::new(
                vec![p(vec![c(8.), tr(64., vec![t()])]), p(vec![c(1.)])],
                vec![
                    p(vec![l(2, 1.), l(0, 1.), tr(-1., vec![t()])]),
                    p(vec![l(3, 2.), l(1, 1.), l(2, -1.)]),
                ],
            )
            .unwrap(),
            guard: p(vec![l(0, 1.), l(2, 1.), c(-0.125)]),
            x: vec![0., 0.],
            z: vec![0., 0.],
            branch: vec![[-128., 128.], [-128., 128.]],
            rad: 2f64.powi(-12),
            cap: 8,
        },
        Fixture {
            name: "pencil",
            src: DaeSource::new(
                vec![p(vec![c(1.), tr(1., vec![t()])])],
                vec![p(vec![
                    l(1, 1.),
                    tr(0.125, vec![t(), y(1)]),
                    l(0, -1.),
                    tr(0.375, vec![t(), t()]),
                ])],
            )
            .unwrap(),
            guard: p(vec![l(1, 1.), c(-0.125)]),
            x: vec![0.],
            z: vec![0.],
            branch: vec![[-1., 8.]],
            rad: 0.,
            cap: 8,
        },
    ]
}
fn receipt(r: &ThresholdRun) -> serde_json::Value {
    use serde_json::json;
    json!({"root":r.root(),"safe":r.committed_until(),"numerical_end":r.numerical_end(),"reason":r.reason(),"pending":r.pending_report(),"driver":r.stats().counters,"attempts":r.stats().attempts,"accepted_steps":r.stats().accepted_steps,"work":r.work,"projection":r.projection_work(),"guard_slabs":r.guard_slabs()})
}
#[test]
#[ignore = "bounded development candidate campaign"]
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
                for (derivative, value) in [
                    (GuardDerivativeRoute::Schur, GuardValueRoute::Transport),
                    (GuardDerivativeRoute::Schur, GuardValueRoute::Box),
                    (
                        GuardDerivativeRoute::PreparedSchur,
                        GuardValueRoute::Transport,
                    ),
                ] {
                    for mode in [
                        "Legacy",
                        "Hull",
                        "Piecewise",
                        "PiecewiseProbe",
                        "Checkpoint",
                    ] {
                        let pp = PendingPolicy {
                            method: match mode {
                                "Hull" => PendingMethod::Hull,
                                "PiecewiseProbe" => PendingMethod::PiecewiseProbe,
                                _ => PendingMethod::Piecewise,
                            },
                            max_slabs: f.cap,
                        };
                        let o = opts();
                        let po = policy();
                        let e = ep(f.rad, 2.);
                        let mut row = json!({"case":f.name,"solver":solver,"root_route":root,"derivative":derivative,"value":value,"mode":mode,"rad":f.rad,"cap":f.cap,"h0":o.h0});
                        if mode == "Checkpoint" {
                            match evolve_to_guard_checkpoint(
                                &f.src, &f.x, &f.z, &f.branch, e, &o, po, solver, &f.guard, 1e-6,
                                root, derivative, value, pp,
                            ) {
                                Ok(r) => {
                                    row["root"] = json!(r.root());
                                    row["safe"] = json!(r.committed_until());
                                    row["first"] = receipt(r.first());
                                    row["retry"] = json!(r.retry().map(receipt));
                                    row["checkpoint"] = json!(r.checkpoint());
                                    row["retry_error"] = json!(r.retry_error());
                                    row["attempts_used"] = json!(r.attempts_used());
                                }
                                Err(e) => row["error"] = json!(e.to_string()),
                            }
                        } else {
                            let r = if mode == "Legacy" {
                                evolve_to_guard(
                                    &f.src, &f.x, &f.z, &f.branch, e, &o, po, solver, &f.guard,
                                    1e-6, root, derivative, value,
                                )
                            } else {
                                evolve_to_guard_pending(
                                    &f.src, &f.x, &f.z, &f.branch, e, &o, po, solver, &f.guard,
                                    1e-6, root, derivative, value, pp,
                                )
                            };
                            match r {
                                Ok(r) => {
                                    row["root"] = json!(r.root());
                                    row["safe"] = json!(r.committed_until());
                                    row["first"] = receipt(&r);
                                    row["attempts_used"] = json!(r.stats().attempts);
                                }
                                Err(e) => row["error"] = json!(e.to_string()),
                            }
                        }
                        println!("CELL {}", row);
                    }
                }
            }
        }
    }
}
