#![cfg(feature = "polynomial-events")]
use rodas5p_core::directed::Interval as Iv;
use rodas5p_integrators::retro_dae_evolution as de;
use vigilode::dae_evolution::*;
use vigilode::polynomial_events::{
    AffineFactor as F, DeclaredPolynomial as P, EventTerm as T, retro_dae_init::*,
};
use vigilode::{Atol, InteriorError, Options};
fn p(v: Vec<T>) -> P {
    P::new(v).unwrap()
}
fn tr(coefficient: f64, factors: Vec<F>) -> T {
    T {
        coefficient,
        factors,
    }
}
fn fixture(n: usize) -> (DaeSource, Vec<f64>, Vec<f64>, Vec<[f64; 2]>, Vec<f64>) {
    let mut f = vec![];
    let mut g = vec![];
    let mut x = vec![];
    let mut goal = vec![0.; 2 * n];
    for j in 0..n / 2 {
        let i = 2 * j;
        f.push(p(vec![tr(
            -((j + 1) as f64),
            vec![
                F::clock(0.),
                F::component(n + i, 0.),
                F::component(n + i, 0.),
            ],
        )]));
        f.push(p(vec![tr(1., vec![F::clock(0.), F::component(n + i, 0.)])]));
        x.extend([0.25, 0.0625]);
        goal[n + i] = if j % 2 == 0 { 0.5 } else { -0.5 };
        goal[i + 1] = 1.;
    }
    for i in 0..n {
        g.push(p(vec![
            tr(1., vec![F::component(n + i, 0.)]),
            tr(-1., vec![F::component(i, 0.)]),
        ]));
    }
    (
        DaeSource::new(f, g).unwrap(),
        x.clone(),
        x,
        vec![[-0.5, 0.75]; n],
        goal,
    )
}
fn run(
    src: &DaeSource,
    x: &[f64],
    z: &[f64],
    b: &[[f64; 2]],
    incoming: Option<f64>,
) -> SlabReplayRun {
    let opts = Options {
        rtol: 1e-8,
        atol: Atol::Scalar(1e-11),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let (r, _) = evolve_epoch_slab_replay_planned(
        src,
        x,
        z,
        b,
        EvolutionEpoch {
            start: 0.,
            end: 0.125,
            incoming_x_error: incoming,
        },
        &opts,
        policy,
        route,
        &[0.0625],
        RetainedOutputRoute {
            queries: QueryArenaLayout::Shared,
            native: NativeRetention::BorrowedRows,
        },
        SlabReplayRoute::Flat,
        EnginePlan::SharedFused,
    );
    r.unwrap()
}
#[test]
fn actual_nonlinear_secant_goal_residual() {
    let (src, x, z, b, c) = fixture(2);
    let r = run(&src, &x, &z, &b, Some(0.));
    assert!(r.global_qualified(), "{:?}", r.reason());
    let mut w = SecantGoalWorkspace::new();
    let mut out = Iv { lo: 0., hi: 0. };
    let a = r
        .secant_goal_into(&c, SecantGoalRoute::Sub4Bernstein, &mut w, &mut out)
        .unwrap();
    assert!(a.cells > 0 && a.width_upper <= 1e-6);
    let y = r.accepted(r.accepted_len() - 1).unwrap();
    let q = 0.25 / (1. + 0.25 * 0.125_f64.powi(2) / 2.);
    let exact = 0.5 * q + 0.0625 + (1. + 0.25 * 0.125_f64.powi(2) / 2.).ln();
    let numeric = c.iter().zip(y.state()).map(|(a, b)| a * b).sum::<f64>();
    assert!(out.lo <= exact - numeric + 1e-15 && out.hi >= exact - numeric - 1e-15);
    println!("SECANT_RED_GREEN {:?}", a);
}
use serde::Serialize;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
};
#[derive(Clone, Copy, Debug, Default, Serialize)]
struct Counts {
    alloc: u64,
    zeroed: u64,
    realloc: u64,
    requested_bytes: u64,
}
impl Counts {
    fn requests(self) -> u64 {
        self.alloc + self.zeroed + self.realloc
    }
}
thread_local! {
 static TRACK:Cell<bool>=const{Cell::new(false)};
 static COUNT:Cell<Counts>=const{Cell::new(Counts{alloc:0,zeroed:0,realloc:0,requested_bytes:0})};
}
fn record(k: usize, n: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNT.try_with(|v| {
            let mut a = v.get();
            match k {
                0 => a.alloc += 1,
                1 => a.zeroed += 1,
                _ => a.realloc += 1,
            }
            a.requested_bytes += n as u64;
            v.set(a);
        });
    }
}
struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(0, l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(1, l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(2, n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        TRACK.with(|v| v.set(false));
    }
}
fn measure<T>(f: impl FnOnce() -> T) -> (T, Counts) {
    assert!(!TRACK.with(Cell::get));
    COUNT.with(|v| v.set(Counts::default()));
    TRACK.with(|v| v.set(true));
    let g = Guard;
    let r = f();
    drop(g);
    (r, COUNT.with(Cell::get))
}
fn other_fixture(case: &str) -> (DaeSource, Vec<f64>, Vec<f64>, Vec<[f64; 2]>, Vec<f64>) {
    if case == "mixed_inverse" {
        let f = vec![
            p(vec![tr(
                -1.,
                vec![F::clock(0.), F::component(2, 0.), F::component(2, 0.)],
            )]),
            p(vec![tr(1., vec![F::clock(0.), F::component(2, 0.)])]),
        ];
        let g = vec![
            p(vec![
                tr(1., vec![F::component(2, 0.)]),
                tr(-1., vec![F::component(0, 0.)]),
                tr(0.125, vec![F::component(3, 0.)]),
                tr(-0.125, vec![F::component(1, 0.)]),
            ]),
            p(vec![
                tr(1., vec![F::component(3, 0.)]),
                tr(-1., vec![F::component(1, 0.)]),
            ]),
        ];
        return (
            DaeSource::new(f, g).unwrap(),
            vec![0.25, 0.0625],
            vec![0.25, 0.0625],
            vec![[-0.5, 0.75]; 2],
            vec![0., 1., 0.5, 0.],
        );
    }
    let f = vec![
        p(vec![tr(0.125, vec![F::component(2, 0.)])]),
        p(vec![tr(1., vec![F::component(0, 0.)])]),
    ];
    let g = vec![
        p(vec![
            tr(1., vec![F::component(2, 0.), F::component(2, 0.)]),
            tr(-1., vec![F::component(0, 0.)]),
            tr(-0.25, vec![]),
        ]),
        p(vec![
            tr(1., vec![F::component(3, 0.)]),
            tr(-1., vec![F::component(1, 0.)]),
        ]),
    ];
    (
        DaeSource::new(f, g).unwrap(),
        vec![0., 0.0625],
        vec![0.5, 0.0625],
        vec![[0.4375, 0.625], [-0.25, 0.5]],
        vec![0., 1., 1., 0.],
    )
}
#[test]
fn secant_actual_portfolio() {
    let (_, positive) = measure(|| black_box(vec![7_f64; 32]));
    assert!(positive.requests() > 0);
    let (_, negative) = measure(|| black_box([7_f64; 32]));
    assert_eq!(negative.requests(), 0);
    let plans = [
        SecantGoalRoute::WholeHorner,
        SecantGoalRoute::Sub4Horner,
        SecantGoalRoute::Sub4Bernstein,
        SecantGoalRoute::Diagonal4,
        SecantGoalRoute::Scaled4,
        SecantGoalRoute::Scaled16,
        SecantGoalRoute::Shifted4,
    ];
    for (case, n) in [
        ("clock2", 2),
        ("clock4", 4),
        ("clock8", 8),
        ("mixed_inverse", 2),
        ("quadratic", 2),
    ] {
        let (src, x, z, b, goal) = if case.starts_with("clock") {
            fixture(n)
        } else {
            other_fixture(case)
        };
        let r = run(&src, &x, &z, &b, Some(0.));
        assert!(r.global_qualified(), "{case}: {:?}", r.reason());
        let q = r.accepted(r.accepted_len() - 1).unwrap();
        let y = q.state().to_vec();
        println!(
            "SECANT_PRIMAL {}",
            serde_json::json!({"case":case,"n":n,"terminal":y,"weights":goal,"slabs":r.slabs_len(),"error":q.physical_error_upper(),"initial":x})
        );
        let mut w = SecantGoalWorkspace::new();
        let mut out = Iv { lo: 0., hi: 0. };
        let before_work = serde_json::to_string(r.work()).unwrap();
        let mut rows: Vec<Option<(SecantGoalWork, Counts)>> = vec![];
        for k in plans {
            let before = out;
            let (v, a) = measure(|| r.secant_goal_into(&goal, k, &mut w, &mut out));
            match v {
                Ok(v) => {
                    assert_eq!(a.requests(), 0);
                    assert_eq!(a.requested_bytes, 0);
                    assert_eq!(out.as_pair(), v.signed_error);
                    let neg = goal.iter().map(|x| -x).collect::<Vec<_>>();
                    let negwork = r.secant_goal_into(&neg, k, &mut w, &mut out).unwrap();
                    println!(
                        "SECANT_SIGN {}",
                        serde_json::json!({"case":case,"candidate":k,"error":negwork.signed_error})
                    );
                    rows.push(Some((v, a)));
                }
                Err(e) => {
                    assert_eq!(out, before);
                    assert!(
                        case == "mixed_inverse"
                            && matches!(
                                k,
                                SecantGoalRoute::Diagonal4
                                    | SecantGoalRoute::Scaled4
                                    | SecantGoalRoute::Scaled16
                            ),
                        "{case} {k:?} {e}"
                    );
                    println!(
                        "SECANT_GOAL {}",
                        serde_json::json!({"case":case,"candidate":k,"qualified":false,"reason":e.to_string(),"allocation":a})
                    );
                    rows.push(None);
                }
            }
        }
        let mut timing: [Vec<u128>; 7] = std::array::from_fn(|_| vec![]);
        for rep in 0..5 {
            for j in 0..7 {
                let i = (rep + j) % 7;
                if rows[i].is_none() {
                    continue;
                }
                let s = std::time::Instant::now();
                black_box(
                    r.secant_goal_into(&goal, plans[i], &mut w, &mut out)
                        .unwrap(),
                );
                timing[i].push(s.elapsed().as_nanos());
            }
        }
        for (i, v) in rows.into_iter().enumerate() {
            if let Some((work, a)) = v {
                timing[i].sort_unstable();
                println!(
                    "SECANT_GOAL {}",
                    serde_json::json!({"case":case,"candidate":plans[i],"qualified":work.width_upper<=1e-6,"allocation":a,"work":work,"median_ns":timing[i][2],"samples_ns":timing[i]})
                );
            }
        }
        let mut scratch = NonlinearAdjointWorkspace::new();
        let mut grad = [Iv { lo: 0., hi: 0. }; 32];
        for k in [
            NonlinearAdjointRoute::Whole,
            NonlinearAdjointRoute::Sub4,
            NonlinearAdjointRoute::Contracted16,
        ] {
            let (v, a) =
                measure(|| r.nonlinear_adjoint_into(&goal, k, &mut scratch, &mut grad[..n]));
            let v = v.unwrap();
            assert_eq!(a.requests(), 0);
            println!(
                "SECANT_GRADIENT {}",
                serde_json::json!({"case":case,"candidate":k,"qualified":v.width_upper<=0.1,"allocation":a,"work":v,"enclosure":grad[..n].iter().map(|v|v.as_pair()).collect::<Vec<_>>()})
            );
        }
        let o = Options {
            rtol: 1e-10,
            atol: Atol::Scalar(1e-12),
            dense_output: false,
            ..Options::default()
        };
        for k in [
            vigilode::DopriStageKernel::Rows,
            vigilode::DopriStageKernel::Block32,
        ] {
            let (v, a) = measure(|| r.nonlinear_adjoint_dopri(&goal, &o, k));
            let v = v.unwrap();
            println!(
                "SECANT_NUMERIC {}",
                serde_json::json!({"case":case,"candidate":k,"proposal":v,"allocation":a})
            );
        }
        assert_eq!(serde_json::to_string(r.work()).unwrap(), before_work);
        for i in 0..r.accepted_len() {
            let q = r.accepted(i).unwrap();
            println!(
                "SECANT_QUERY {}",
                serde_json::json!({"case":case,"t":q.t(),"state":q.state(),"error":q.physical_error_upper()})
            );
        }
    }
    let (src, x, z, b, c) = fixture(2);
    let r = run(&src, &x, &z, &b, Some(1e-12));
    let mut w = SecantGoalWorkspace::new();
    let mut out = Iv { lo: -7., hi: 7. };
    for k in plans {
        assert!(r.secant_goal_into(&c, k, &mut w, &mut out).is_err());
        assert_eq!(out.as_pair(), [-7., 7.]);
    }
    println!(
        "SECANT_EDGE {}",
        serde_json::json!({"nonzero_incoming_refused":true,"unchanged_output":true})
    );
}

#[test]
fn goal_budget_actual_native_endpoint() {
    let (src, x, z, b, c) = fixture(2);
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let r = evolve_epoch_goal_budget(
        &src,
        &x,
        &z,
        &b,
        EvolutionEpoch {
            start: 0.,
            end: 0.125,
            incoming_x_error: Some(0.),
        },
        &opts,
        policy,
        route,
        &[],
        RetainedOutputRoute {
            queries: QueryArenaLayout::Shared,
            native: NativeRetention::BorrowedRows,
        },
        &c,
        GoalBudgetPolicy {
            absolute: 1e-8,
            plan: GoalBudgetPlan::SecantReplay,
        },
    )
    .unwrap();
    assert!(r.run.unwrap().global_qualified());
}
#[test]
fn goal_budget_actual_portfolio() {
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let epoch = EvolutionEpoch {
        start: 0.,
        end: 0.125,
        incoming_x_error: Some(0.),
    };
    let retention = RetainedOutputRoute {
        queries: QueryArenaLayout::Shared,
        native: NativeRetention::BorrowedRows,
    };
    let plans = [
        GoalBudgetPlan::Fixed,
        GoalBudgetPlan::StateReplay,
        GoalBudgetPlan::SecantReplay,
        GoalBudgetPlan::Predictive,
    ];
    for (case, n) in [
        ("clock2", 2),
        ("clock4", 4),
        ("clock8", 8),
        ("mixed_inverse", 2),
        ("quadratic", 2),
    ] {
        let (src, x, z, b, c) = if case.starts_with("clock") {
            fixture(n)
        } else {
            other_fixture(case)
        };
        for (label, absolute) in [("loose", 1e-8), ("tight", 1e-10)] {
            let call = |plan| {
                evolve_epoch_goal_budget(
                    &src,
                    &x,
                    &z,
                    &b,
                    epoch,
                    &opts,
                    policy,
                    route,
                    &[],
                    retention,
                    &c,
                    GoalBudgetPolicy { absolute, plan },
                )
                .unwrap()
            };
            let mut rows = Vec::new();
            for plan in plans {
                let (r, a) = measure(|| call(plan));
                let qualified = r.run.is_some();
                assert_eq!(qualified, r.attempts.last().unwrap().accepted);
                assert!(
                    r.attempts[..r.attempts.len() - 1]
                        .iter()
                        .all(|r| !r.accepted)
                );
                assert!(
                    r.attempts
                        .windows(2)
                        .all(|w| w[0].multiplier > w[1].multiplier)
                );
                assert!(r.attempts.len() <= 7);
                if let Some(run) = &r.run {
                    assert!(run.global_qualified());
                    let q = run.accepted(run.accepted_len() - 1).unwrap();
                    let a = r.attempts.last().unwrap();
                    assert_eq!(a.terminal_state.as_deref(), Some(q.state()));
                    assert!(a.enclosure.unwrap().iter().all(|v| v.abs() <= absolute));
                }
                let rhs = r.total_rhs();
                let steps = r.total_steps();
                assert_eq!(
                    rhs,
                    Some(
                        r.attempts
                            .iter()
                            .map(|a| a.audit.stats.as_ref().map_or(0, |s| s.counters.rhs_calls))
                            .sum()
                    )
                );
                let actions = r
                    .attempts
                    .iter()
                    .flat_map(|a| &a.certificates)
                    .filter_map(|r| r.work)
                    .map(|r| r.matrix_actions)
                    .sum::<usize>();
                rows.push((r.attempts, a, qualified, rhs, steps, actions));
            }
            let mut times: [Vec<u128>; 4] = std::array::from_fn(|_| vec![]);
            for rep in 0..5 {
                for j in 0..4 {
                    let i = (rep + j) % 4;
                    let s = std::time::Instant::now();
                    black_box(call(plans[i]));
                    times[i].push(s.elapsed().as_nanos());
                }
            }
            for (i, (attempts, allocation, qualified, rhs, steps, actions)) in
                rows.into_iter().enumerate()
            {
                times[i].sort_unstable();
                println!(
                    "GOAL_BUDGET {}",
                    serde_json::json!({"case":case,"budget":label,"absolute":absolute,"candidate":plans[i],"qualified":qualified,"allocation":allocation,"total_rhs":rhs,"total_steps":steps,"certificate_actions":actions,"weights":c,"n":n,"attempts":attempts,"median_ns":times[i][2],"samples_ns":times[i]})
                );
            }
        }
    }
    let (src, x, z, b, c) = fixture(2);
    for plan in plans {
        let r = evolve_epoch_goal_budget(
            &src,
            &x,
            &z,
            &b,
            epoch,
            &opts,
            policy,
            route,
            &[],
            retention,
            &c,
            GoalBudgetPolicy {
                absolute: 1e-30,
                plan,
            },
        )
        .unwrap();
        assert!(r.run.is_none() && r.attempts.iter().all(|a| !a.accepted));
        println!(
            "GOAL_EXHAUST {}",
            serde_json::json!({"candidate":plan,"attempts":r.attempts,"total_rhs":r.total_rhs()})
        );
    }
    assert!(
        evolve_epoch_goal_budget(
            &src,
            &x,
            &z,
            &b,
            EvolutionEpoch {
                incoming_x_error: Some(1e-12),
                ..epoch
            },
            &opts,
            policy,
            route,
            &[],
            retention,
            &c,
            GoalBudgetPolicy {
                absolute: 1e-8,
                plan: GoalBudgetPlan::Predictive
            }
        )
        .is_err()
    );
}

#[test]
fn cost_aware_goal_tight_starts_floor() {
    let (src, x, z, b, c) = fixture(4);
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let r = evolve_epoch_goal_budget(
        &src,
        &x,
        &z,
        &b,
        EvolutionEpoch {
            start: 0.,
            end: 0.125,
            incoming_x_error: Some(0.),
        },
        &opts,
        policy,
        route,
        &[],
        RetainedOutputRoute {
            queries: QueryArenaLayout::Shared,
            native: NativeRetention::BorrowedRows,
        },
        &c,
        GoalBudgetPolicy {
            absolute: 1e-10,
            plan: GoalBudgetPlan::CostAware,
        },
    )
    .unwrap();
    println!("COST_RED_GREEN multiplier={}", r.attempts[0].multiplier);
    assert_eq!(r.attempts[0].multiplier, 1.);
    assert!(r.run.is_some());
}
#[test]
fn cost_aware_actual_portfolio() {
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let epoch = EvolutionEpoch {
        start: 0.,
        end: 0.125,
        incoming_x_error: Some(0.),
    };
    let retention = RetainedOutputRoute {
        queries: QueryArenaLayout::Shared,
        native: NativeRetention::BorrowedRows,
    };
    let plans = [
        GoalBudgetPlan::Fixed,
        GoalBudgetPlan::StateReplay,
        GoalBudgetPlan::SecantReplay,
        GoalBudgetPlan::Predictive,
        GoalBudgetPlan::CostAware,
    ];
    for (case, n) in [
        ("clock2", 2),
        ("clock4", 4),
        ("clock8", 8),
        ("mixed_inverse", 2),
        ("quadratic", 2),
    ] {
        let (src, x, z, b, c) = if case.starts_with("clock") {
            fixture(n)
        } else {
            other_fixture(case)
        };
        for (label, absolute) in [("loose", 1e-8), ("middle", 1e-9), ("tight", 1e-10)] {
            let call = |plan| {
                evolve_epoch_goal_budget(
                    &src,
                    &x,
                    &z,
                    &b,
                    epoch,
                    &opts,
                    policy,
                    route,
                    &[],
                    retention,
                    &c,
                    GoalBudgetPolicy { absolute, plan },
                )
                .unwrap()
            };
            let mut rows = Vec::new();
            for plan in plans {
                let (r, a) = measure(|| call(plan));
                let qualified = r.run.is_some();
                assert_eq!(qualified, r.attempts.last().unwrap().accepted);
                assert!(
                    r.attempts[..r.attempts.len() - 1]
                        .iter()
                        .all(|r| !r.accepted)
                );
                assert!(
                    r.attempts
                        .windows(2)
                        .all(|w| w[0].multiplier > w[1].multiplier)
                );
                assert!(r.attempts.len() <= 7);
                if let Some(run) = &r.run {
                    assert!(run.global_qualified());
                    let q = run.accepted(run.accepted_len() - 1).unwrap();
                    let a = r.attempts.last().unwrap();
                    assert_eq!(a.terminal_state.as_deref(), Some(q.state()));
                    assert!(a.enclosure.unwrap().iter().all(|v| v.abs() <= absolute));
                }
                let rhs = r.total_rhs();
                let steps = r.total_steps();
                assert_eq!(
                    rhs,
                    Some(
                        r.attempts
                            .iter()
                            .map(|a| a.audit.stats.as_ref().map_or(0, |s| s.counters.rhs_calls))
                            .sum()
                    )
                );
                let actions = r
                    .attempts
                    .iter()
                    .flat_map(|a| &a.certificates)
                    .filter_map(|r| r.work)
                    .map(|r| r.matrix_actions)
                    .sum::<usize>();
                rows.push((r.attempts, a, qualified, rhs, steps, actions));
            }
            let mut times: [Vec<u128>; 5] = std::array::from_fn(|_| vec![]);
            for rep in 0..5 {
                for j in 0..5 {
                    let i = (rep + j) % 5;
                    let s = std::time::Instant::now();
                    black_box(call(plans[i]));
                    times[i].push(s.elapsed().as_nanos());
                }
            }
            for (i, (attempts, allocation, qualified, rhs, steps, actions)) in
                rows.into_iter().enumerate()
            {
                times[i].sort_unstable();
                println!(
                    "COST_BUDGET {}",
                    serde_json::json!({"case":case,"budget":label,"absolute":absolute,"candidate":plans[i],"qualified":qualified,"allocation":allocation,"total_rhs":rhs,"total_steps":steps,"certificate_actions":actions,"weights":c,"n":n,"attempts":attempts,"median_ns":times[i][2],"samples_ns":times[i]})
                );
            }
        }
    }
    let (src, x, z, b, c) = fixture(2);
    for plan in plans {
        let r = evolve_epoch_goal_budget(
            &src,
            &x,
            &z,
            &b,
            epoch,
            &opts,
            policy,
            route,
            &[],
            retention,
            &c,
            GoalBudgetPolicy {
                absolute: 1e-30,
                plan,
            },
        )
        .unwrap();
        assert!(r.run.is_none() && r.attempts.iter().all(|a| !a.accepted));
        println!(
            "COST_EXHAUST {}",
            serde_json::json!({"candidate":plan,"attempts":r.attempts,"total_rhs":r.total_rhs()})
        );
    }
    assert!(
        evolve_epoch_goal_budget(
            &src,
            &x,
            &z,
            &b,
            EvolutionEpoch {
                incoming_x_error: Some(1e-12),
                ..epoch
            },
            &opts,
            policy,
            route,
            &[],
            retention,
            &c,
            GoalBudgetPolicy {
                absolute: 1e-8,
                plan: GoalBudgetPlan::Predictive
            }
        )
        .is_err()
    );
}

#[test]
fn tiered_goal_uses_intermediate_tolerance() {
    let (src, x, z, b, c) = fixture(2);
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let r = evolve_epoch_goal_budget(
        &src,
        &x,
        &z,
        &b,
        EvolutionEpoch {
            start: 0.,
            end: 0.125,
            incoming_x_error: Some(0.),
        },
        &opts,
        policy,
        route,
        &[],
        RetainedOutputRoute {
            queries: QueryArenaLayout::Shared,
            native: NativeRetention::BorrowedRows,
        },
        &c,
        GoalBudgetPolicy {
            absolute: 1e-9,
            plan: GoalBudgetPlan::TieredCost,
        },
    )
    .unwrap();
    println!("TIER_RED_GREEN multiplier={}", r.attempts[0].multiplier);
    assert_eq!(r.attempts[0].multiplier, 8.);
    assert!(r.run.is_some());
}
fn growth_fixture() -> (DaeSource, Vec<f64>, Vec<f64>, Vec<[f64; 2]>, Vec<f64>) {
    let f = vec![
        p(vec![tr(8., vec![F::component(2, 0.)])]),
        p(vec![tr(1., vec![F::component(2, 0.)])]),
    ];
    let g = (0..2)
        .map(|i| {
            p(vec![
                tr(1., vec![F::component(i + 2, 0.)]),
                tr(-1., vec![F::component(i, 0.)]),
            ])
        })
        .collect();
    let x = vec![1. / 1024., 0.];
    (
        DaeSource::new(f, g).unwrap(),
        x.clone(),
        x,
        vec![[-1., 4.]; 2],
        vec![1., 0., 0., 0.],
    )
}
fn coupled_clock_fixture() -> (DaeSource, Vec<f64>, Vec<f64>, Vec<[f64; 2]>, Vec<f64>) {
    // u=x+y, v=y; x'= -t*x^2, y'=t*x. Both represented f depend on u,v.
    let t = F::clock(0.);
    let u = F::component(2, 0.);
    let v = F::component(3, 0.);
    let f = vec![
        p(vec![
            tr(-1., vec![t.clone(), u.clone(), u.clone()]),
            tr(2., vec![t.clone(), u.clone(), v.clone()]),
            tr(-1., vec![t.clone(), v.clone(), v.clone()]),
            tr(1., vec![t.clone(), u.clone()]),
            tr(-1., vec![t.clone(), v.clone()]),
        ]),
        p(vec![
            tr(1., vec![t.clone(), u.clone()]),
            tr(-1., vec![t.clone(), v.clone()]),
        ]),
    ];
    let g = (0..2)
        .map(|i| {
            p(vec![
                tr(1., vec![F::component(i + 2, 0.)]),
                tr(-1., vec![F::component(i, 0.)]),
            ])
        })
        .collect();
    let x = vec![0.3125, 0.0625];
    (
        DaeSource::new(f, g).unwrap(),
        x.clone(),
        x,
        vec![[-1., 2.]; 2],
        vec![0., 1., 0.5, 0.],
    )
}
#[test]
fn goal_escalation_actual_portfolio() {
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let retention = RetainedOutputRoute {
        queries: QueryArenaLayout::Shared,
        native: NativeRetention::BorrowedRows,
    };
    let plans = [
        GoalBudgetPlan::Fixed,
        GoalBudgetPlan::CostAware,
        GoalBudgetPlan::TieredCost,
    ];
    for (case, n, end) in [
        ("clock2", 2, 0.5),
        ("clock8", 8, 0.5),
        ("growth2", 2, 0.75),
        ("growth2", 2, 1.),
        ("coupled_clock", 2, 1.),
        ("coupled_clock", 2, 2.),
    ] {
        let (src, x, z, b, c) = if case == "growth2" {
            growth_fixture()
        } else if case == "coupled_clock" {
            coupled_clock_fixture()
        } else {
            fixture(n)
        };
        let epoch = EvolutionEpoch {
            start: 0.,
            end,
            incoming_x_error: Some(0.),
        };
        for (label, absolute) in [("loose", 1e-8), ("middle", 1e-9), ("tight", 1e-10)] {
            let call = |plan| {
                evolve_epoch_goal_budget(
                    &src,
                    &x,
                    &z,
                    &b,
                    epoch,
                    &opts,
                    policy,
                    route,
                    &[],
                    retention,
                    &c,
                    GoalBudgetPolicy { absolute, plan },
                )
                .unwrap()
            };
            let mut receipts = vec![];
            for k in plans {
                let (r, a) = measure(|| call(k));
                let qualified = r.run.is_some();
                assert_eq!(qualified, r.attempts.last().unwrap().accepted);
                assert!(
                    r.attempts
                        .windows(2)
                        .all(|w| w[0].multiplier > w[1].multiplier)
                );
                assert!(r.attempts.len() <= 2);
                let rhs = r.total_rhs();
                let steps = r.total_steps();
                let actions = r
                    .attempts
                    .iter()
                    .flat_map(|a| &a.certificates)
                    .filter_map(|r| r.work)
                    .map(|r| r.matrix_actions)
                    .sum::<usize>();
                receipts.push((r.attempts, a, qualified, rhs, steps, actions));
            }
            let mut samples: [Vec<u128>; 3] = std::array::from_fn(|_| vec![]);
            for rep in 0..3 {
                for j in 0..3 {
                    let i = (rep + j) % 3;
                    let start = std::time::Instant::now();
                    black_box(call(plans[i]));
                    samples[i].push(start.elapsed().as_nanos());
                }
            }
            for (i, (attempts, a, qualified, rhs, steps, actions)) in
                receipts.into_iter().enumerate()
            {
                samples[i].sort_unstable();
                println!(
                    "ESC_GOAL {}",
                    serde_json::json!({"case":case,"end":end,"n":n,"budget":label,"absolute":absolute,"candidate":plans[i],"qualified":qualified,"weights":c,"attempts":attempts,"allocation":a,"total_rhs":rhs,"total_steps":steps,"certificate_actions":actions,"median_ns":samples[i][1],"samples_ns":samples[i]})
                );
            }
        }
    }
}

#[test]
fn structured_homogeneous_goal_red_green() {
    let (src, x, z, b, c) = growth_fixture();
    let r = run(&src, &x, &z, &b, Some(0.));
    assert!(r.global_qualified());
    let q = r.accepted(r.accepted_len() - 1).unwrap();
    let truth = (8. * q.t()).exp() / 1024. - q.state()[0];
    for route in [
        AdjointTaylorRoute::Fixed12,
        AdjointTaylorRoute::Fixed20,
        AdjointTaylorRoute::Budgeted,
    ] {
        let mut iv = Iv::point(123.).unwrap();
        let (work, alloc) = measure(|| r.homogeneous_goal_into(&c, route, 1e-12, &mut iv));
        let work = work.unwrap();
        assert_eq!(alloc.requests(), 0);
        assert!(iv.lo <= truth + 1e-15 && iv.hi >= truth - 1e-15 && iv.hi - iv.lo < 1e-8);
        println!(
            "STRUCT_CERT {} {}",
            serde_json::to_string(&work).unwrap(),
            serde_json::to_string(&iv.as_pair()).unwrap()
        );
    }
    let (src, x, z, b, c) = fixture(2);
    let r = run(&src, &x, &z, &b, Some(0.));
    let mut iv = Iv::point(123.).unwrap();
    assert!(
        r.homogeneous_goal_into(&c, AdjointTaylorRoute::Fixed20, 1e-12, &mut iv)
            .is_err()
    );
    assert_eq!(iv.as_pair(), [123., 123.]);
}

#[test]
fn structured_goal_actual_portfolio() {
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let retention = RetainedOutputRoute {
        queries: QueryArenaLayout::Shared,
        native: NativeRetention::BorrowedRows,
    };
    let plans = [
        GoalBudgetPlan::Fixed,
        GoalBudgetPlan::Structured12,
        GoalBudgetPlan::Structured20,
        GoalBudgetPlan::StructuredBudgeted,
    ];
    for (case, end) in [
        ("growth2", 0.75),
        ("growth2", 1.),
        ("clock2", 0.5),
        ("coupled_clock", 2.),
    ] {
        let (src, x, z, b, c) = match case {
            "growth2" => growth_fixture(),
            "clock2" => fixture(2),
            _ => coupled_clock_fixture(),
        };
        for absolute in [1e-8, 1e-9, 1e-10] {
            for plan in plans {
                let start = std::time::Instant::now();
                let (r, a) = measure(|| {
                    evolve_epoch_goal_budget(
                        &src,
                        &x,
                        &z,
                        &b,
                        EvolutionEpoch {
                            start: 0.,
                            end,
                            incoming_x_error: Some(0.),
                        },
                        &opts,
                        policy,
                        route,
                        &[],
                        retention,
                        &c,
                        GoalBudgetPolicy { absolute, plan },
                    )
                    .unwrap()
                });
                let ns = start.elapsed().as_nanos();
                let last = r.attempts.last().unwrap();
                assert_eq!(r.run.is_some(), last.accepted);
                if case == "growth2" && plan != GoalBudgetPlan::Fixed {
                    assert!(last.physical_qualified && last.structured_work.is_some());
                    if plan == GoalBudgetPlan::Structured20 {
                        assert!(last.accepted && last.certificates.is_empty());
                    }
                }
                if case != "growth2" {
                    assert!(last.structured_work.is_none());
                }
                println!(
                    "STRUCT_GOAL {}",
                    serde_json::json!({"case":case,"end":end,"absolute":absolute,"plan":plan,"qualified":last.accepted,"attempt":last,"weights":c,"allocation":a,"rhs":r.total_rhs(),"steps":r.total_steps(),"ns":ns})
                );
            }
        }
    }
}

#[test]
fn signed_goal_actual_portfolio() {
    let opts = Options {
        rtol: 1e-10,
        atol: Atol::Scalar(1e-13),
        dense_output: true,
        interior_error: InteriorError::Off,
        max_steps: 4096,
        ..Default::default()
    };
    let policy = EvolutionPolicy {
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
    };
    let route = EvolutionRoute::WorkspaceQuery5 {
        projection: WorkspaceProjection::PivotedVector,
        certificate: de::SlabWorkspaceRoute::Refined,
        storage: CertificateStorage::Local,
        query: QueryWorkspaceRoute::StreamPairs,
    };
    let retention = RetainedOutputRoute {
        queries: QueryArenaLayout::Shared,
        native: NativeRetention::BorrowedRows,
    };
    let plans = [
        GoalBudgetPlan::Fixed,
        GoalBudgetPlan::SignedPortfolio,
        GoalBudgetPlan::SignedRefined,
    ];
    for (case, end) in [
        ("clock2", 0.5),
        ("clock8", 0.5),
        ("coupled_clock", 1.),
        ("coupled_clock", 2.),
    ] {
        let (src, x, z, b, c) = match case {
            "clock8" => fixture(8),
            "clock2" => fixture(2),
            _ => coupled_clock_fixture(),
        };
        // Inspect the actual physical slab replay without altering admission.
        let (pr, _) = evolve_epoch_slab_replay_planned(
            &src,
            &x,
            &z,
            &b,
            EvolutionEpoch {
                start: 0.,
                end,
                incoming_x_error: Some(0.),
            },
            &opts,
            policy,
            route,
            &[],
            retention,
            SlabReplayRoute::Flat,
            EnginePlan::SharedFused,
        );
        let pr = pr.unwrap();
        let mut first = None;
        for k in 0..pr.slabs_len() {
            let s = pr.slab(k).unwrap();
            if s.certificate.as_ref().is_none_or(|c| !*c.global_qualified) {
                first = Some(
                    serde_json::json!({"index":k,"times":s.times,"reason":s.reason,"certificate":s.certificate}),
                );
                break;
            }
        }
        println!(
            "SIGNED_PHYSICAL {}",
            serde_json::json!({"case":case,"end":end,"qualified":pr.global_qualified(),"reason":pr.reason(),"first":first,"slabs":pr.slabs_len(),"driver_success":pr.driver().is_success()})
        );
        for absolute in [1e-8, 1e-9, 1e-10] {
            for plan in plans {
                let start = std::time::Instant::now();
                let (r, a) = measure(|| {
                    evolve_epoch_goal_budget(
                        &src,
                        &x,
                        &z,
                        &b,
                        EvolutionEpoch {
                            start: 0.,
                            end,
                            incoming_x_error: Some(0.),
                        },
                        &opts,
                        policy,
                        route,
                        &[],
                        retention,
                        &c,
                        GoalBudgetPolicy { absolute, plan },
                    )
                    .unwrap()
                });
                let ns = start.elapsed().as_nanos();
                let last = r.attempts.last().unwrap();
                assert_eq!(r.run.is_some(), last.accepted);
                if !last.physical_qualified {
                    assert!(!last.accepted);
                    assert!(last.certificates.is_empty());
                }
                println!(
                    "SIGNED_GOAL {}",
                    serde_json::json!({"case":case,"end":end,"absolute":absolute,"plan":plan,"qualified":last.accepted,"attempt":last,"weights":c,"allocation":a,"rhs":r.total_rhs(),"steps":r.total_steps(),"ns":ns})
                );
            }
        }
    }
}
