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
        if pr.global_qualified() {
            let y = pr.accepted(pr.accepted_len() - 1).unwrap();
            let mut w = SecantGoalWorkspace::new();
            let mut out = Iv::point(0.).unwrap();
            for k in [
                SecantGoalRoute::WholeHorner,
                SecantGoalRoute::Sub4Bernstein,
                SecantGoalRoute::Diagonal4,
                SecantGoalRoute::Scaled4,
                SecantGoalRoute::Scaled8,
                SecantGoalRoute::Scaled16,
                SecantGoalRoute::Shifted4,
                SecantGoalRoute::Shifted8,
            ] {
                let old = out;
                let start = std::time::Instant::now();
                let (r, a) = measure(|| pr.secant_goal_into(&c, k, &mut w, &mut out));
                let ns = start.elapsed().as_nanos();
                match r {
                    Ok(v) => {
                        assert_eq!(a.requests(), 0);
                        assert_eq!(out.as_pair(), v.signed_error);
                        println!(
                            "SIGNED_DIRECT {}",
                            serde_json::json!({"case":case,"end":end,"candidate":k,"qualified":true,"allocation":a,"work":v,"terminal":y.state(),"weights":c,"ns":ns})
                        );
                    }
                    Err(e) => {
                        assert_eq!(out, old);
                        println!(
                            "SIGNED_DIRECT {}",
                            serde_json::json!({"case":case,"end":end,"candidate":k,"qualified":false,"allocation":a,"reason":e.to_string()})
                        );
                    }
                }
            }
        }
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

#[test]
fn weighted_clock_actual_contract() {
    let (src, x, z, b, _) = coupled_clock_fixture();
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
    let call = |replay| {
        evolve_epoch_slab_replay_planned(
            &src,
            &x,
            &z,
            &b,
            EvolutionEpoch {
                start: 0.,
                end: 2.,
                incoming_x_error: Some(0.),
            },
            &opts,
            policy,
            route,
            &[],
            retention,
            replay,
            EnginePlan::SharedFused,
        )
        .0
        .unwrap()
    };
    let old = call(SlabReplayRoute::Flat);
    assert!(!old.global_qualified());
    let new = call(SlabReplayRoute::ClockWeighted(de::ClockTubePlan::Mean4));
    assert!(new.global_qualified(), "{:?}", new.reason());
    assert_eq!(old.driver().t, new.driver().t);
    assert_eq!(old.driver().y, new.driver().y);
    assert!(new.max_physical_error().unwrap() < 1e-3);
}

fn weighted_settings() -> (
    Options,
    EvolutionPolicy,
    EvolutionRoute,
    RetainedOutputRoute,
) {
    (
        Options {
            rtol: 1e-10,
            atol: Atol::Scalar(1e-13),
            dense_output: true,
            interior_error: InteriorError::Off,
            max_steps: 4096,
            ..Default::default()
        },
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
        },
        EvolutionRoute::WorkspaceQuery5 {
            projection: WorkspaceProjection::PivotedVector,
            certificate: de::SlabWorkspaceRoute::Refined,
            storage: CertificateStorage::Local,
            query: QueryWorkspaceRoute::StreamPairs,
        },
        RetainedOutputRoute {
            queries: QueryArenaLayout::Shared,
            native: NativeRetention::BorrowedRows,
        },
    )
}
#[test]
fn weighted_clock_actual_portfolio() {
    use de::{ClockTubeCarry as Carry, ClockTubePlan as P};
    let (opts, policy, route, retention) = weighted_settings();
    let plans = [P::Sup4, P::Mean4, P::Mean8, P::Hybrid4];
    for case in ["clock2", "coupled_clock"] {
        let (src, x, z, b, c) = if case == "clock2" {
            fixture(2)
        } else {
            coupled_clock_fixture()
        };
        assert!(Carry::new(&src, P::Mean4, 0., &x, Some(1e-12)).is_err());
        assert!(Carry::new(&src, P::Mean4, -0.1, &x, Some(0.)).is_err());
        for end in [0.5, 1., 2., 3.] {
            // The existing clock2 branch ends at z=0.75; only the coupled
            // fixture has a source branch containing its T=3 trajectory.
            if case == "clock2" && end == 3. {
                continue;
            }
            let epoch = EvolutionEpoch {
                start: 0.,
                end,
                incoming_x_error: Some(0.),
            };
            let call = |replay| {
                evolve_epoch_slab_replay_planned(
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
                    replay,
                    EnginePlan::SharedFused,
                )
            };
            let ((old, audit), alloc) = measure(|| call(SlabReplayRoute::Flat));
            let old = old.unwrap();
            println!(
                "TUBE_RUN {}",
                serde_json::json!({"case":case,"end":end,"candidate":"Flat","qualified":old.global_qualified(),"bound":old.max_physical_error(),"allocation":alloc,"rhs":audit.stats.as_ref().map(|s|s.counters.rhs_calls),"slabs":old.slabs_len()})
            );
            for plan in plans {
                let ((r, audit), alloc) = measure(|| call(SlabReplayRoute::ClockWeighted(plan)));
                let r = r.unwrap();
                assert_eq!(r.driver().t, old.driver().t);
                assert_eq!(r.driver().y, old.driver().y);
                if end <= 2. || plan == P::Mean8 {
                    assert!(
                        r.global_qualified(),
                        "{case} {end} {plan:?} {:?}",
                        r.reason()
                    );
                } else {
                    assert!(!r.global_qualified());
                }
                let mut carry = Carry::new(&src, plan, 0., &x, Some(0.)).unwrap();
                let mut scratch = de::SlabCertificateWorkspace::new();
                let mut out = de::SlabCertificate::workspace_output(2, 2).unwrap();
                let mut qualified_slabs = 0;
                let mut total = 0;
                for k in 0..old.slabs_len() {
                    let sl = old.slab(k).unwrap();
                    let rows: Vec<Vec<Iv>> = match sl.power {
                        RetainedPowerRows::Owned(a) => a
                            .iter()
                            .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                            .collect(),
                        RetainedPowerRows::Flat(a) => a
                            .iter()
                            .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                            .collect(),
                    };
                    let (v, a) = measure(|| {
                        carry.certify_into(
                            sl.times,
                            &rows,
                            &b,
                            policy.tube_radius,
                            policy.green,
                            policy.init.condition_cap,
                            de::SlabWorkspaceRoute::Refined,
                            &mut scratch,
                            &mut out,
                        )
                    });
                    let actual = r.slab(k).unwrap();
                    match v {
                        Ok(work) => {
                            let ac = actual.certificate.as_ref().unwrap();
                            assert_eq!(out.global_qualified, *ac.global_qualified);
                            assert_eq!(out.whole_x_error, *ac.whole_x_error);
                            assert_eq!(out.whole_z_error, *ac.whole_z_error);
                            assert_eq!(out.outgoing, *ac.outgoing);
                            assert!(out.global_qualified);
                            qualified_slabs += 1;
                            total += a.requests();
                            let power: Vec<Vec<[f64; 2]>> = rows
                                .iter()
                                .map(|r| r.iter().map(|v| v.as_pair()).collect())
                                .collect();
                            println!(
                                "TUBE_SLAB {}",
                                serde_json::json!({"case":case,"end":end,"candidate":plan,"index":k,"times":sl.times,"power":power,"work":work,"allocation":a,"x_error":out.whole_x_error,"z_error":out.whole_z_error})
                            );
                        }
                        Err(e) => {
                            assert!(!out.global_qualified && out.whole_x_error.is_none());
                            assert!(actual.certificate.is_none());
                            assert!(!r.global_qualified());
                            assert!(
                                carry
                                    .certify_into(
                                        sl.times,
                                        &rows,
                                        &b,
                                        policy.tube_radius,
                                        policy.green,
                                        policy.init.condition_cap,
                                        de::SlabWorkspaceRoute::Refined,
                                        &mut scratch,
                                        &mut out
                                    )
                                    .is_err()
                            );
                            assert!(!out.global_qualified);
                            println!(
                                "TUBE_REFUSAL {}",
                                serde_json::json!({"case":case,"end":end,"candidate":plan,"index":k,"times":sl.times,"reason":e.to_string()})
                            );
                            break;
                        }
                    }
                }
                assert_eq!(total, 0, "{case} {plan:?} successful local allocation");
                let terminal = r.accepted(r.accepted_len() - 1).unwrap();
                println!(
                    "TUBE_RUN {}",
                    serde_json::json!({"case":case,"end":end,"candidate":plan,"qualified":r.global_qualified(),"bound":r.max_physical_error(),"allocation":alloc,"rhs":audit.stats.as_ref().map(|s|s.counters.rhs_calls),"slabs":r.slabs_len(),"qualified_slabs":qualified_slabs,"warm_allocations":total,"terminal":terminal.state(),"terminal_bound":terminal.physical_error_upper()})
                );
                if r.global_qualified() {
                    let mut sw = SecantGoalWorkspace::new();
                    let mut iv = Iv::point(0.).unwrap();
                    for candidate in [
                        SecantGoalRoute::Scaled4,
                        SecantGoalRoute::Scaled8,
                        SecantGoalRoute::Scaled16,
                        SecantGoalRoute::Budgeted,
                        SecantGoalRoute::BudgetedEconomy,
                    ] {
                        let (v, a) = measure(|| match candidate {
                            SecantGoalRoute::Budgeted | SecantGoalRoute::BudgetedEconomy => {
                                r.secant_goal_budgeted_into(&c, candidate, 1e-10, &mut sw, &mut iv)
                            }
                            _ => r.secant_goal_into(&c, candidate, &mut sw, &mut iv),
                        });
                        let work = v.unwrap();
                        assert_eq!(a.requests(), 0);
                        if let Some(d) = work.adaptive_cells {
                            assert_eq!(d.iter().sum::<usize>(), work.slabs);
                            assert_eq!(d[0] * 4 + d[1] * 8 + d[2] * 16, work.cells);
                        }
                        println!(
                            "TUBE_ADJOINT {}",
                            serde_json::json!({"case":case,"end":end,"candidate":plan,"route":candidate,"work":work,"allocation":a,"terminal":terminal.state(),"weights":c})
                        );
                    }
                }
            }
        }
    }
    let (src, x, _, _, _) = growth_fixture();
    assert!(Carry::new(&src, P::Mean4, 0., &x, Some(0.)).is_err());
    let (src, x, _, _, _) = fixture(8);
    assert!(Carry::new(&src, P::Mean4, 0., &x, Some(0.)).is_err());
}
#[test]
fn adaptive_signed_actual_portfolio() {
    let (opts, policy, route, retention) = weighted_settings();
    for (case, end) in [
        ("clock2", 0.5),
        ("clock8", 0.5),
        ("coupled_clock", 1.),
        ("coupled_clock", 2.),
    ] {
        let (src, x, z, b, c) = if case == "coupled_clock" {
            coupled_clock_fixture()
        } else {
            fixture(if case == "clock8" { 8 } else { 2 })
        };
        for absolute in [1e-8, 1e-9, 1e-10] {
            for plan in [
                GoalBudgetPlan::Fixed,
                GoalBudgetPlan::SignedRefined,
                GoalBudgetPlan::AdaptiveSigned,
                GoalBudgetPlan::EconomySigned,
                GoalBudgetPlan::WeightedAdaptiveSigned,
            ] {
                if case == "clock8" && plan == GoalBudgetPlan::WeightedAdaptiveSigned {
                    continue;
                }
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
                let last = r.attempts.last().unwrap();
                assert_eq!(r.run.is_some(), last.accepted);
                if last.accepted {
                    assert!(last.physical_qualified);
                }
                println!(
                    "ADAPTIVE_GOAL {}",
                    serde_json::json!({"case":case,"end":end,"absolute":absolute,"plan":plan,"qualified":last.accepted,"attempt":last,"weights":c,"allocation":a,"rhs":r.total_rhs(),"steps":r.total_steps()})
                );
            }
        }
    }
}

#[test]
fn triangular_goal_actual_contract() {
    let (src, x, z, b, c) = fixture(8);
    let (opts, policy, route, retention) = weighted_settings();
    let call = |plan| {
        evolve_epoch_goal_budget(
            &src,
            &x,
            &z,
            &b,
            EvolutionEpoch {
                start: 0.,
                end: 0.5,
                incoming_x_error: Some(0.),
            },
            &opts,
            policy,
            route,
            &[],
            retention,
            &c,
            GoalBudgetPolicy {
                absolute: 1e-10,
                plan,
            },
        )
        .unwrap()
    };
    let old = call(GoalBudgetPlan::Fixed);
    assert!(!old.attempts.last().unwrap().accepted);
    let new = call(GoalBudgetPlan::TriangularAdaptive);
    assert!(new.attempts.last().unwrap().accepted);
    assert_eq!(old.total_rhs(), new.total_rhs());
    assert_eq!(
        old.attempts.last().unwrap().terminal_state,
        new.attempts.last().unwrap().terminal_state
    );
}

#[test]
fn triangular_goal_actual_portfolio() {
    use de::TriangularGoalRoute as T;
    let (opts, policy, route, retention) = weighted_settings();
    let plans = [
        GoalBudgetPlan::Fixed,
        GoalBudgetPlan::EconomySigned,
        GoalBudgetPlan::WeightedAdaptiveSigned,
        GoalBudgetPlan::Triangular12,
        GoalBudgetPlan::Triangular24,
        GoalBudgetPlan::TriangularAdaptive,
        GoalBudgetPlan::TriangularRootReduced,
    ];
    for (case, end) in [
        ("clock2", 0.5),
        ("clock8", 0.5),
        ("coupled_clock", 1.),
        ("coupled_clock", 2.),
        ("coupled_clock", 3.),
    ] {
        let (src, x, z, b, c) = if case == "coupled_clock" {
            coupled_clock_fixture()
        } else {
            fixture(if case == "clock8" { 8 } else { 2 })
        };
        let epoch = EvolutionEpoch {
            start: 0.,
            end,
            incoming_x_error: Some(0.),
        };
        for absolute in [1e-8, 1e-9, 1e-10] {
            for plan in plans {
                if plan == GoalBudgetPlan::WeightedAdaptiveSigned && (case == "clock8" || end == 3.)
                {
                    continue;
                }
                let (r, a) = measure(|| {
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
                });
                let last = r.attempts.last().unwrap();
                let is_tri = matches!(
                    plan,
                    GoalBudgetPlan::Triangular12
                        | GoalBudgetPlan::Triangular24
                        | GoalBudgetPlan::TriangularAdaptive
                        | GoalBudgetPlan::TriangularRootReduced
                );
                if is_tri {
                    assert!(
                        last.physical_qualified,
                        "{case} {end} {plan:?} {:?}",
                        last.reason
                    );
                    if end <= 2. {
                        assert!(
                            last.accepted,
                            "{case} {end} {absolute} {plan:?} {:?}",
                            last.reason
                        );
                    }
                }
                println!(
                    "TRI_GOAL {}",
                    serde_json::json!({"case":case,"end":end,"absolute":absolute,"plan":plan,"qualified":last.accepted,"attempt":last,"weights":c,"allocation":a,"rhs":r.total_rhs(),"steps":r.total_steps()})
                );
            }
        }
        let replay = if case == "clock8" {
            SlabReplayRoute::Flat
        } else {
            SlabReplayRoute::ClockWeighted(if end > 2. {
                de::ClockTubePlan::Mean8
            } else {
                de::ClockTubePlan::Mean4
            })
        };
        let (r, _) = evolve_epoch_slab_replay_planned(
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
            replay,
            EnginePlan::SharedFused,
        );
        let r = r.unwrap();
        assert!(r.global_qualified());
        let terminal = r.accepted(r.accepted_len() - 1).unwrap();
        for candidate in [T::Atanh12, T::Atanh24, T::Adaptive, T::RootReduced] {
            let mut iv = Iv::point(0.).unwrap();
            let (w, a) = measure(|| r.triangular_goal_into(&c, candidate, &mut iv));
            let work = w.unwrap();
            assert_eq!(a.requests(), 0);
            assert_eq!(work.signed_error, iv.as_pair());
            println!(
                "TRI_DIRECT {}",
                serde_json::json!({"case":case,"end":end,"candidate":candidate,"terminal":terminal.state(),"weights":c,"allocation":a,"work":work,"qualified":true})
            );
        }
    }
}
#[test]
fn triangular_goal_arithmetic_and_refusals() {
    use de::TriangularGoalRoute as T;
    let (src, _, _, _, _) = fixture(2);
    let (wrong, xw, zw, _, cw) = growth_fixture();
    let mut iv = Iv::point(17.).unwrap();
    let mut endpoint = xw.clone();
    endpoint.extend(&zw);
    assert!(
        de::triangular_clock_goal(&wrong, 0., 1., &xw, &endpoint, &cw, T::Adaptive, &mut iv)
            .is_err()
    );
    assert_eq!(iv, Iv::point(17.).unwrap());
    assert!(
        de::triangular_clock_goal(
            &src,
            0.,
            1.,
            &[-0.25, 0.],
            &[0.; 4],
            &[1.; 4],
            T::Adaptive,
            &mut iv
        )
        .is_err()
    );
    assert!(
        de::triangular_clock_goal(
            &src,
            -1.,
            1.,
            &[0.25, 0.],
            &[0.; 4],
            &[1.; 4],
            T::Adaptive,
            &mut iv
        )
        .is_err()
    );
    assert_eq!(iv, Iv::point(17.).unwrap());
    // Additional exact source perturbation: q' uses a different coefficient.
    let badsrc = DaeSource::new(
        vec![
            p(vec![tr(
                -1.,
                vec![F::clock(0.), F::component(2, 0.), F::component(2, 0.)],
            )]),
            p(vec![tr(
                1.0000000000000002,
                vec![F::clock(0.), F::component(2, 0.)],
            )]),
        ],
        (0..2)
            .map(|i| {
                p(vec![
                    tr(1., vec![F::component(i + 2, 0.)]),
                    tr(-1., vec![F::component(i, 0.)]),
                ])
            })
            .collect(),
    )
    .unwrap();
    assert!(
        de::triangular_clock_goal(
            &badsrc,
            0.,
            1.,
            &[0.25, 0.],
            &[0.; 4],
            &[1.; 4],
            T::Adaptive,
            &mut iv
        )
        .is_err()
    );
    assert_eq!(iv, Iv::point(17.).unwrap());
    for (case, start, end, x) in [
        ("zero_p", 0., 1., [0., 0.125]),
        ("zero_duration", 1., 1., [0.25, 0.125]),
        ("nonzero_start", 0.25, 1., [0.25, 0.125]),
        ("tiny", 0., 1., [f64::from_bits(1), 0.]),
        ("large", 0., 16., [0.25, 0.125]),
    ] {
        let y = [0.; 4];
        let c = [1., 1., 0., 0.];
        for candidate in [T::Atanh12, T::Atanh24, T::Adaptive, T::RootReduced] {
            let (r, a) = measure(|| {
                de::triangular_clock_goal(&src, start, end, &x, &y, &c, candidate, &mut iv)
            });
            let work = r.unwrap();
            assert_eq!(a.requests(), 0);
            println!(
                "TRI_ARITH {}",
                serde_json::json!({"case":case,"start":start,"end":end,"initial":x,"terminal":y,"weights":c,"candidate":candidate,"allocation":a,"work":work,"qualified":true})
            );
        }
    }
    println!(
        "TRI_NEGATIVE {}",
        serde_json::json!({"unsupported":true,"altered_source":true,"negative_p":true,"negative_start":true,"transactional":true})
    );
}

#[test]
fn structured_clock_adjoint_contract() {
    let (src, x, _, _, c) = fixture(2);
    let mut out = [Iv::point(17.).unwrap(); 2];
    let (w, a) = measure(|| de::structured_clock_adjoint_into(&src, 0., 1., &x, &c, &mut out));
    assert_eq!(w.unwrap().blocks, 1);
    assert_eq!(a.requests(), 0);
    assert!(out[0].contains(0.5 / (1.125 * 1.125) + 0.5 / 1.125));
    assert!(out[1].contains(1.));
    let mut iv = Iv::point(0.).unwrap();
    let w = de::triangular_clock_goal(
        &src,
        0.,
        1.,
        &[f64::from_bits(1), 0.],
        &[0.; 4],
        &[1., 1., 0., 0.],
        de::TriangularGoalRoute::TinyAdaptive,
        &mut iv,
    )
    .unwrap();
    assert_eq!(w.series_terms, 0);
}

fn monomial_fixture(
    k: usize,
    bcoef: f64,
) -> (DaeSource, Vec<f64>, Vec<f64>, Vec<[f64; 2]>, Vec<f64>) {
    let mut f = vec![];
    let n = 2;
    let mut pf = vec![F::clock(0.); k];
    pf.extend([F::component(n, 0.), F::component(n, 0.)]);
    let mut qf = vec![F::clock(0.); k];
    qf.push(F::component(n, 0.));
    f.push(p(vec![tr(-1.5, pf)]));
    f.push(p(vec![tr(bcoef, qf)]));
    let g = (0..n)
        .map(|i| {
            p(vec![
                tr(1., vec![F::component(n + i, 0.)]),
                tr(-1., vec![F::component(i, 0.)]),
            ])
        })
        .collect();
    let x = vec![0.25, 0.0625];
    (
        DaeSource::new(f, g).unwrap(),
        x.clone(),
        x,
        vec![[-1., 2.]; n],
        vec![0., 1., 0.5, 0.],
    )
}
#[test]
fn structured_clock_actual_portfolio() {
    use de::TriangularGoalRoute as T;
    let (opts, policy, route, retention) = weighted_settings();
    for (case, k, bcoef, start, end) in [
        ("clock2", 1, 1., 0., 0.5),
        ("clock8", 1, 1., 0., 0.5),
        ("coupled_clock", 1, 1., 0., 1.),
        ("mono0", 0, -0.75, 0.125, 0.5),
        ("mono1", 1, 1.25, 0.125, 0.5),
        ("mono2", 2, 0.5, 0.125, 0.5),
    ] {
        let (src, x, z, b, c) = match case {
            "clock2" => fixture(2),
            "clock8" => fixture(8),
            "coupled_clock" => coupled_clock_fixture(),
            _ => monomial_fixture(k, bcoef),
        };
        let epoch = EvolutionEpoch {
            start,
            end,
            incoming_x_error: Some(0.),
        };
        let (r, _) = evolve_epoch_slab_replay_planned(
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
            SlabReplayRoute::Flat,
            EnginePlan::SharedFused,
        );
        let r = r.unwrap();
        assert!(r.global_qualified(), "{case}");
        let term = r.accepted(r.accepted_len() - 1).unwrap();
        let mut output = vec![Iv::point(17.).unwrap(); x.len()];
        let mut w = NonlinearAdjointWorkspace::new();
        for candidate in [
            NonlinearAdjointRoute::Whole,
            NonlinearAdjointRoute::Sub4,
            NonlinearAdjointRoute::Contracted16,
        ] {
            let (res, a) = measure(|| r.nonlinear_adjoint_into(&c, candidate, &mut w, &mut output));
            match res {
                Ok(work) => println!(
                    "CLOCK_GRAD {}",
                    serde_json::json!({"case":case,"k":k,"b":bcoef,"start":start,"end":end,"initial":x,"weights":c,"candidate":candidate,"qualified":true,"allocation":a,"gradient":output.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"width":work.width_upper,"actions":work.matrix_actions})
                ),
                Err(e) => println!(
                    "CLOCK_GRAD {}",
                    serde_json::json!({"case":case,"candidate":candidate,"qualified":false,"allocation":a,"reason":e.to_string()})
                ),
            }
        }
        let (res, a) = measure(|| r.structured_clock_adjoint_into(&c, &mut output));
        let work = res.unwrap();
        assert_eq!(a.requests(), 0);
        println!(
            "CLOCK_GRAD {}",
            serde_json::json!({"case":case,"k":k,"b":bcoef,"start":start,"end":end,"initial":x,"weights":c,"candidate":"Structured","qualified":true,"allocation":a,"gradient":output.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"width":work.max_gradient_width,"actions":0})
        );
        for candidate in [
            T::Adaptive,
            T::RootReduced,
            T::TinyAdaptive,
            T::TinyRootReduced,
        ] {
            let mut iv = Iv::point(17.).unwrap();
            let (res, a) = measure(|| r.structured_clock_goal_into(&c, candidate, &mut iv));
            let work = res.unwrap();
            assert_eq!(a.requests(), 0);
            println!(
                "CLOCK_GOAL {}",
                serde_json::json!({"case":case,"k":k,"b":bcoef,"start":start,"end":end,"initial":x,"weights":c,"terminal":term.state(),"candidate":candidate,"qualified":true,"allocation":a,"work":work})
            );
        }
    }
}
#[test]
fn tiny_log_actual_portfolio() {
    use de::TriangularGoalRoute as T;
    let (src, _, _, _, _) = fixture(2);
    for (case, p0, end) in [
        ("subnormal", f64::from_bits(1), 1.),
        ("minnormal", f64::MIN_POSITIVE, 1.),
        ("tiny", 2f64.powi(-70), 1.),
        ("threshold", 2f64.powi(-51), 1.),
        ("above", 2f64.powi(-50), 1.),
        ("ordinary", 0.25, 1.),
        ("large", 0.25, 16.),
    ] {
        let x = [p0, 0.];
        let y = [0.; 4];
        let c = [0., 1., 0., 0.];
        for candidate in [
            T::Adaptive,
            T::RootReduced,
            T::TinyAdaptive,
            T::TinyRootReduced,
        ] {
            let mut iv = Iv::point(17.).unwrap();
            let (res, a) = measure(|| {
                de::triangular_clock_goal(&src, 0., end, &x, &y, &c, candidate, &mut iv)
            });
            let work = res.unwrap();
            assert_eq!(a.requests(), 0);
            if matches!(candidate, T::TinyAdaptive | T::TinyRootReduced)
                && p0 <= 2f64.powi(-51)
                && end == 1.
            {
                assert_eq!(work.series_terms, 0);
            }
            println!(
                "TINY_LOG {}",
                serde_json::json!({"case":case,"start":0.,"end":end,"initial":x,"weights":c,"terminal":y,"candidate":candidate,"qualified":true,"allocation":a,"work":work})
            );
        }
    }
    let (wrong, x, _, _, c) = growth_fixture();
    let mut grad = vec![Iv::point(17.).unwrap(); x.len()];
    assert!(de::structured_clock_adjoint_into(&wrong, 0., 1., &x, &c, &mut grad).is_err());
    assert!(grad.iter().all(|x| *x == Iv::point(17.).unwrap()));
}

#[test]
fn polynomial_clock_fused_contract() {
    use de::{PolynomialClockRoute as C, TriangularGoalRoute as T};
    let f = vec![
        p(vec![
            tr(-1., vec![F::component(2, 0.), F::component(2, 0.)]),
            tr(
                -0.5,
                vec![F::clock(0.), F::component(2, 0.), F::component(2, 0.)],
            ),
        ]),
        p(vec![
            tr(0.75, vec![F::component(2, 0.)]),
            tr(0.375, vec![F::clock(0.), F::component(2, 0.)]),
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
    let src = DaeSource::new(f, g).unwrap();
    let x = [0.25, 0.0625];
    let weights = [0., 1., 0.5, 0.];
    let mut grad = [Iv::point(17.).unwrap(); 2];
    let mut goal = Iv::point(17.).unwrap();
    let (res, a) = measure(|| {
        de::polynomial_clock_fused_into(
            &src,
            0.,
            1.,
            &x,
            &[0.; 4],
            &weights,
            C::Factored,
            T::TinyAdaptive,
            &mut grad,
            &mut goal,
        )
    });
    assert_eq!(res.unwrap().blocks, 1);
    assert_eq!(a.requests(), 0);
    let m = 1.25;
    let d = 1. + 0.25 * m;
    assert!(grad[0].contains(0.5 / (d * d) + 0.75 * m / d));
    assert!(grad[1].contains(1.));
}

fn polynomial_fixture(
    a: &[[f64; 3]],
    b: &[[f64; 3]],
) -> (DaeSource, Vec<f64>, Vec<f64>, Vec<[f64; 2]>, Vec<f64>) {
    let n = 2 * a.len();
    assert_eq!(a.len(), b.len());
    let mut f = vec![];
    let mut weights = vec![0.; 2 * n];
    let mut x = vec![];
    for j in 0..a.len() {
        let i = 2 * j;
        let terms = |c: [f64; 3], power: usize| -> Vec<T> {
            let mut rows = vec![];
            // reverse degree order exercises order-independent exact matcher
            for k in (0..3).rev() {
                if c[k] != 0. {
                    let mut factors = vec![F::clock(0.); k];
                    factors.extend((0..power).map(|_| F::component(n + i, 0.)));
                    factors.reverse();
                    rows.push(tr(c[k], factors));
                }
            }
            if rows.is_empty() {
                rows.push(tr(0., vec![F::component(n + i, 0.); power]));
            }
            rows
        };
        f.push(p(terms(a[j].map(|x| -x), 2)));
        f.push(p(terms(b[j], 1)));
        x.extend([0.25, 0.0625]);
        weights[i + 1] = if j % 2 == 0 { 1. } else { -0.5 };
        weights[n + i] = 0.5;
    }
    let g = (0..n)
        .map(|i| {
            p(vec![
                tr(1., vec![F::component(n + i, 0.)]),
                tr(-1., vec![F::component(i, 0.)]),
            ])
        })
        .collect();
    (
        DaeSource::new(f, g).unwrap(),
        x.clone(),
        x,
        vec![[-2., 3.]; n],
        weights,
    )
}
#[test]
fn polynomial_clock_actual_portfolio() {
    use de::{ClockTubePlan as CP, PolynomialClockRoute as C, TriangularGoalRoute as T};
    let (opts, policy, route, retention) = weighted_settings();
    let cases = [
        (
            "affine",
            vec![[1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.]],
            0.,
            0.5,
        ),
        (
            "quadratic",
            vec![[0.25, 0.5, 0.125]],
            vec![[-0.125, -0.25, -0.0625]],
            0.125,
            1.,
        ),
        (
            "ratio_third",
            vec![[3., 1.5, 0.75]],
            vec![[1., 0.5, 0.25]],
            0.125,
            0.5,
        ),
        (
            "long",
            vec![[0.5, 0.5, 0.125]],
            vec![[0.75, 0.75, 0.1875]],
            0.,
            2.,
        ),
        (
            "block4",
            vec![[0.25, 0.5, 0.125], [0.5, 0., 0.25]],
            vec![[-0.125, -0.25, -0.0625], [0.75, 0., 0.375]],
            0.125,
            0.5,
        ),
        (
            "zero_q",
            vec![[0.25, 0.25, 0.125]],
            vec![[0., 0., 0.]],
            0.,
            0.5,
        ),
        (
            "coupled_clock",
            vec![[0., 1., 0.]],
            vec![[0., 1., 0.]],
            0.,
            1.,
        ),
    ];
    for (case, a, b, start, end) in cases {
        let (src, x, z, branch, c) = if case == "coupled_clock" {
            coupled_clock_fixture()
        } else {
            polynomial_fixture(&a, &b)
        };
        let epoch = EvolutionEpoch {
            start,
            end,
            incoming_x_error: Some(0.),
        };
        let mut retained = None;
        let mut baseline = None;
        for (name, plan) in [
            ("Flat", SlabReplayRoute::Flat),
            ("Mean4", SlabReplayRoute::ClockWeighted(CP::PolynomialMean4)),
            ("Mean8", SlabReplayRoute::ClockWeighted(CP::PolynomialMean8)),
            (
                "Hybrid4",
                SlabReplayRoute::ClockWeighted(CP::PolynomialHybrid4),
            ),
        ] {
            if x.len() != 2 && name != "Flat" {
                continue;
            }
            let ((r, audit), alloc) = measure(|| {
                evolve_epoch_slab_replay_planned(
                    &src,
                    &x,
                    &z,
                    &branch,
                    epoch,
                    &opts,
                    policy,
                    route,
                    &[],
                    retention,
                    plan,
                    EnginePlan::SharedFused,
                )
            });
            let r = r.unwrap();
            let state = r.driver().y.last().unwrap();
            let identity = (r.driver().t.clone(), r.driver().y.clone());
            if let Some(old) = &baseline {
                assert_eq!(&identity, old);
            } else {
                baseline = Some(identity);
            }
            let terminal = r.accepted(r.accepted_len() - 1).unwrap();
            let qualified = r.global_qualified();
            println!(
                "POLY_TUBE {}",
                serde_json::json!({"case":case,"candidate":name,"qualified":qualified,"a":a,"b":b,"start":start,"end":end,"initial":x,"weights":c,"terminal":terminal.state(),"bound":terminal.physical_error_upper(),"rhs":audit.stats.as_ref().unwrap().counters.rhs_calls,"allocation":alloc,"driver_last":state,"reason":r.reason()})
            );
            if qualified && retained.is_none() {
                retained = Some(r);
            }
        }
        let r = retained.expect("at least one physical route");
        let term = r.accepted(r.accepted_len() - 1).unwrap();
        let mut output = vec![Iv::point(17.).unwrap(); x.len()];
        let mut goal = Iv::point(17.).unwrap();
        for candidate in [C::Termwise, C::Factored, C::Intersection] {
            for log in [T::TinyAdaptive, T::TinyRootReduced] {
                let (res, alloc) = measure(|| {
                    r.polynomial_clock_fused_into(&c, candidate, log, &mut output, &mut goal)
                });
                let work = res.unwrap();
                assert_eq!(alloc.requests(), 0);
                println!(
                    "POLY_DIRECT {}",
                    serde_json::json!({"case":case,"candidate":candidate,"log":log,"qualified":true,"a":a,"b":b,"start":start,"end":end,"initial":x,"weights":c,"terminal":term.state(),"gradient":output.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"allocation":alloc,"work":work})
                );
            }
        }
        let mut w = NonlinearAdjointWorkspace::new();
        for candidate in [
            NonlinearAdjointRoute::Whole,
            NonlinearAdjointRoute::Sub4,
            NonlinearAdjointRoute::Contracted16,
        ] {
            let (res, alloc) =
                measure(|| r.nonlinear_adjoint_into(&c, candidate, &mut w, &mut output));
            match res {
                Ok(work) => println!(
                    "POLY_GRAD {}",
                    serde_json::json!({"case":case,"candidate":candidate,"qualified":true,"a":a,"b":b,"start":start,"end":end,"initial":x,"weights":c,"gradient":output.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"allocation":alloc,"width":work.width_upper,"actions":work.matrix_actions})
                ),
                Err(e) => println!(
                    "POLY_GRAD {}",
                    serde_json::json!({"case":case,"candidate":candidate,"qualified":false,"allocation":alloc,"reason":e.to_string()})
                ),
            }
        }
        for absolute in [1e-8, 1e-10] {
            for plan in [
                GoalBudgetPlan::Fixed,
                GoalBudgetPlan::EconomySigned,
                GoalBudgetPlan::PolynomialFlatTermwise,
                GoalBudgetPlan::PolynomialFlatFactored,
                GoalBudgetPlan::PolynomialMean4,
                GoalBudgetPlan::PolynomialMean8,
                GoalBudgetPlan::PolynomialHybrid4,
            ] {
                if x.len() != 2
                    && matches!(
                        plan,
                        GoalBudgetPlan::PolynomialMean4
                            | GoalBudgetPlan::PolynomialMean8
                            | GoalBudgetPlan::PolynomialHybrid4
                    )
                {
                    continue;
                }
                let (run, alloc) = measure(|| {
                    evolve_epoch_goal_budget(
                        &src,
                        &x,
                        &z,
                        &branch,
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
                });
                let last = run.attempts.last().unwrap();
                println!(
                    "POLY_POLICY {}",
                    serde_json::json!({"case":case,"candidate":plan,"absolute":absolute,"qualified":last.accepted,"a":a,"b":b,"start":start,"end":end,"initial":x,"weights":c,"allocation":alloc,"attempt":last,"rhs":run.total_rhs(),"steps":run.total_steps()})
                );
            }
        }
    }
}
#[test]
fn polynomial_clock_refusals() {
    use de::{PolynomialClockRoute as C, TriangularGoalRoute as T};
    for (a, b) in [
        (vec![[1., 0.5, 0.]], vec![[0.75, 0.375f64.next_up(), 0.]]),
        (vec![[1., -0.5, 0.]], vec![[0.75, -0.375, 0.]]),
        (vec![[1., 0.5, 0.]], vec![[0.75, 0.375, 0.125]]),
        (
            vec![[1., 0.5, 0.], [1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.], [0.75, 0.375f64.next_up(), 0.]],
        ),
    ] {
        let (src, x, _, _, c) = polynomial_fixture(&a, &b);
        let mut grad = vec![Iv::point(17.).unwrap(); x.len()];
        let mut goal = Iv::point(17.).unwrap();
        assert!(
            de::polynomial_clock_fused_into(
                &src,
                0.,
                1.,
                &x,
                &vec![0.; 2 * x.len()],
                &c,
                C::Factored,
                T::TinyRootReduced,
                &mut grad,
                &mut goal
            )
            .is_err()
        );
        assert_eq!(goal, Iv::point(17.).unwrap());
        assert!(grad.iter().all(|v| *v == goal));
    }
    println!(
        "POLY_REFUSAL {}",
        serde_json::json!({"one_ulp_mismatch":true,"negative_rate":true,"extra_power":true,"late_block_transaction":true})
    );
}

#[test]
fn adaptive_block_parameter_contract() {
    use de::{
        ClockTubePlan as CP, ParameterCovectorRoute as R, PolynomialClockRoute as C,
        TriangularGoalRoute as T,
    };
    let (src, x, z, b, c) = polynomial_fixture(
        &[[1., 0.5, 0.], [0.25, 0.5, 0.125]],
        &[[0.75, 0.375, 0.], [-0.125, -0.25, -0.0625]],
    );
    let (opts, policy, route, retention) = weighted_settings();
    let (run, _) = evolve_epoch_slab_replay_planned(
        &src,
        &x,
        &z,
        &b,
        EvolutionEpoch {
            start: 0.,
            end: 0.5,
            incoming_x_error: Some(0.),
        },
        &opts,
        policy,
        route,
        &[],
        retention,
        SlabReplayRoute::ClockWeighted(CP::BlockAdaptiveMean),
        EnginePlan::SharedFused,
    );
    let run = run.unwrap();
    assert!(run.global_qualified());
    let mut out = [Iv::point(17.).unwrap(); 8];
    let (r, a) = measure(|| {
        run.polynomial_parameter_covector_into(
            &c,
            C::Termwise,
            T::TinyAdaptive,
            R::Factored,
            &mut out,
        )
    });
    assert_eq!(r.unwrap().blocks, 2);
    assert_eq!(a.requests(), 0);
    let m = 0.5625;
    let d = 1. + 0.25 * m;
    assert!(out[0].contains(0.5 * (-0.5 * 0.25 * 0.25 / (d * d) + 0.75 * 0.25 / d)));
    assert!(out[3].contains((1.0_f64 + 0.25 * m).ln()));
}

#[test]
fn adaptive_block_actual_portfolio() {
    use de::{
        ClockTubePlan as CP, ParameterCovectorRoute as R, PolynomialClockRoute as C,
        TriangularGoalRoute as T,
    };
    let (opts, policy, route, retention) = weighted_settings();
    let cases = [
        (
            "affine",
            vec![[1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.]],
            0.,
            0.5,
        ),
        (
            "quadratic",
            vec![[0.25, 0.5, 0.125]],
            vec![[-0.125, -0.25, -0.0625]],
            0.125,
            1.,
        ),
        (
            "ratio_third",
            vec![[3., 1.5, 0.75]],
            vec![[1., 0.5, 0.25]],
            0.125,
            0.5,
        ),
        (
            "long",
            vec![[0.5, 0.5, 0.125]],
            vec![[0.75, 0.75, 0.1875]],
            0.,
            2.,
        ),
        (
            "block4",
            vec![[0.25, 0.5, 0.125], [0.5, 0., 0.25]],
            vec![[-0.125, -0.25, -0.0625], [0.75, 0., 0.375]],
            0.125,
            0.5,
        ),
        (
            "zero_q",
            vec![[0.25, 0.25, 0.125]],
            vec![[0., 0., 0.]],
            0.,
            0.5,
        ),
        (
            "coupled_clock",
            vec![[0., 1., 0.]],
            vec![[0., 1., 0.]],
            0.,
            1.,
        ),
        (
            "holdout_block6",
            vec![[1., 0.25, 0.125], [0.125, 0.5, 0.25], [0.5, 0., 0.25]],
            vec![[0.25, 0.0625, 0.03125], [-0.25, -1., -0.5], [0., 0., 0.]],
            0.25,
            1.5,
        ),
    ];
    for (case, a, b, start, end) in cases {
        let (src, x, z, branch, c) = if case == "coupled_clock" {
            coupled_clock_fixture()
        } else {
            polynomial_fixture(&a, &b)
        };
        let epoch = EvolutionEpoch {
            start,
            end,
            incoming_x_error: Some(0.),
        };
        let call = |replay| {
            evolve_epoch_slab_replay_planned(
                &src,
                &x,
                &z,
                &branch,
                epoch,
                &opts,
                policy,
                route,
                &[],
                retention,
                replay,
                EnginePlan::SharedFused,
            )
        };
        let ((flat, audit), alloc) = measure(|| call(SlabReplayRoute::Flat));
        let flat = flat.unwrap();
        let term = flat.accepted(flat.accepted_len() - 1).unwrap();
        println!(
            "BLOCK_RUN {}",
            serde_json::json!({"case":case,"candidate":"Flat","qualified":flat.global_qualified(),"bound":flat.max_physical_error(),"rhs":audit.stats.as_ref().unwrap().counters.rhs_calls,"allocation":alloc,"terminal":term.state(),"terminal_bound":term.physical_error_upper(),"a":a,"b":b,"initial":x,"start":start,"end":end,"weights":c})
        );
        let mut retained = None;
        for candidate in [
            CP::PolynomialMean4,
            CP::PolynomialMean8,
            CP::BlockAdaptiveMean,
            CP::BlockAdaptiveHybrid,
            CP::BlockSplit,
        ] {
            if x.len() != 2 && matches!(candidate, CP::PolynomialMean4 | CP::PolynomialMean8) {
                continue;
            }
            let ((r, audit), alloc) = measure(|| call(SlabReplayRoute::ClockWeighted(candidate)));
            let r = r.unwrap();
            assert_eq!(r.driver().t, flat.driver().t);
            assert_eq!(r.driver().y, flat.driver().y);
            let mut carry = de::ClockTubeCarry::new(&src, candidate, start, &x, Some(0.)).unwrap();
            let mut scratch = de::SlabCertificateWorkspace::new();
            let mut out = de::SlabCertificate::workspace_output(x.len(), x.len()).unwrap();
            let mut good = 0;
            for k in 0..flat.slabs_len() {
                let sl = flat.slab(k).unwrap();
                let rows: Vec<Vec<Iv>> = match sl.power {
                    RetainedPowerRows::Owned(a) => a
                        .iter()
                        .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                        .collect(),
                    RetainedPowerRows::Flat(a) => a
                        .iter()
                        .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                        .collect(),
                };
                let (v, allocation) = measure(|| {
                    carry.certify_into(
                        sl.times,
                        &rows,
                        &branch,
                        policy.tube_radius,
                        policy.green,
                        policy.init.condition_cap,
                        de::SlabWorkspaceRoute::Refined,
                        &mut scratch,
                        &mut out,
                    )
                });
                let actual = r.slab(k).unwrap();
                match v {
                    Ok(work) => {
                        let ac = actual.certificate.as_ref().unwrap();
                        assert_eq!(out.global_qualified, *ac.global_qualified);
                        assert_eq!(out.whole_x_error, *ac.whole_x_error);
                        if !out.global_qualified {
                            break;
                        }
                        assert_eq!(allocation.requests(), 0);
                        good += 1;
                        println!(
                            "BLOCK_SLAB {}",
                            serde_json::json!({"case":case,"candidate":candidate,"index":k,"times":sl.times,"power":rows.iter().map(|r|r.iter().map(|v|v.as_pair()).collect::<Vec<_>>()).collect::<Vec<_>>(),"work":work,"allocation":allocation,"x_error":out.whole_x_error,"z_error":out.whole_z_error})
                        );
                    }
                    Err(e) => {
                        assert!(!out.global_qualified && out.whole_x_error.is_none());
                        assert!(actual.certificate.is_none());
                        println!(
                            "BLOCK_REFUSAL {}",
                            serde_json::json!({"case":case,"candidate":candidate,"index":k,"reason":e.to_string()})
                        );
                        break;
                    }
                }
            }
            let terminal = r.accepted(r.accepted_len() - 1).unwrap();
            println!(
                "BLOCK_RUN {}",
                serde_json::json!({"case":case,"candidate":candidate,"qualified":r.global_qualified(),"bound":r.max_physical_error(),"rhs":audit.stats.as_ref().unwrap().counters.rhs_calls,"allocation":alloc,"terminal":terminal.state(),"terminal_bound":terminal.physical_error_upper(),"a":a,"b":b,"initial":x,"start":start,"end":end,"weights":c,"qualified_slabs":good})
            );
            if r.global_qualified() && retained.is_none() {
                retained = Some(r);
            }
        }
        let r = retained.unwrap_or(flat);
        assert!(r.global_qualified(), "{case}");
        let mut out = vec![Iv::point(17.).unwrap(); 2 * x.len()];
        for candidate in [R::Separate, R::Factored, R::Intersection] {
            for measure_route in [C::Termwise, C::Factored] {
                let (v, alloc) = measure(|| {
                    r.polynomial_parameter_covector_into(
                        &c,
                        measure_route,
                        T::TinyRootReduced,
                        candidate,
                        &mut out,
                    )
                });
                let work = v.unwrap();
                assert_eq!(alloc.requests(), 0);
                println!(
                    "PARAM_COV {}",
                    serde_json::json!({"case":case,"candidate":candidate,"measure":measure_route,"qualified":true,"a":a,"b":b,"initial":x,"weights":c,"start":start,"end":end,"gradient":out.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"allocation":alloc,"work":work})
                );
            }
        }
        for absolute in [1e-8, 1e-10] {
            for plan in [
                GoalBudgetPlan::PolynomialFlatTermwise,
                GoalBudgetPlan::PolynomialBlockAdaptive,
                GoalBudgetPlan::PolynomialBlockHybrid,
                GoalBudgetPlan::PolynomialBlockSplit,
            ] {
                let (r, alloc) = measure(|| {
                    evolve_epoch_goal_budget(
                        &src,
                        &x,
                        &z,
                        &branch,
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
                });
                let last = r.attempts.last().unwrap();
                println!(
                    "BLOCK_POLICY {}",
                    serde_json::json!({"case":case,"candidate":plan,"absolute":absolute,"qualified":last.accepted,"a":a,"b":b,"initial":x,"weights":c,"start":start,"end":end,"attempt":last,"rhs":r.total_rhs(),"steps":r.total_steps(),"allocation":alloc})
                );
            }
        }
    }
}

#[test]
fn parameter_covector_refusal_and_zero() {
    use de::{ParameterCovectorRoute as R, PolynomialClockRoute as C, TriangularGoalRoute as T};
    let (src, x, _, _, c) = polynomial_fixture(
        &[[1., 0.5, 0.], [1., 0.5, 0.]],
        &[[0.75, 0.375, 0.], [0.75, 0.375f64.next_up(), 0.]],
    );
    let mut out = [Iv::point(17.).unwrap(); 8];
    assert!(
        de::polynomial_parameter_covector_into(
            &src,
            0.,
            1.,
            &x,
            &c,
            C::Factored,
            T::TinyAdaptive,
            R::Intersection,
            &mut out
        )
        .is_err()
    );
    assert!(out.iter().all(|v| *v == Iv::point(17.).unwrap()));
    let (src, mut x, _, _, c) = polynomial_fixture(&[[1., 0.5, 0.]], &[[0.75, 0.375, 0.]]);
    x[0] = 0.;
    let mut out = [Iv::point(17.).unwrap(); 4];
    let (r, a) = measure(|| {
        de::polynomial_parameter_covector_into(
            &src,
            0.,
            1.,
            &x,
            &c,
            C::Termwise,
            T::TinyRootReduced,
            R::Intersection,
            &mut out,
        )
    });
    assert!(r.is_ok());
    assert_eq!(a.requests(), 0);
    assert!(out.iter().all(|v| v.contains(0.)));
    println!(
        "PARAM_REFUSAL {}",
        serde_json::json!({"late_block":true,"zero_p":true,"allocation":a})
    );
}

#[test]
fn decay_and_verified_dense_contract() {
    use de::{ClockTubePlan as CP, PolynomialClockRoute as C, TriangularGoalRoute as T};
    use rodas5p_integrators::OutputSchedule;
    let (src, x, z, b, c) = polynomial_fixture(&[[1., 0.5, 0.]], &[[0.75, 0.375, 0.]]);
    let (opts, policy, route, retention) = weighted_settings();
    let (r, _) = evolve_epoch_slab_replay_planned(
        &src,
        &x,
        &z,
        &b,
        EvolutionEpoch {
            start: 0.,
            end: 0.5,
            incoming_x_error: Some(0.),
        },
        &opts,
        policy,
        route,
        &[],
        retention,
        SlabReplayRoute::ClockWeighted(CP::BlockSplitDecay4),
        EnginePlan::SharedFused,
    );
    let r = r.unwrap();
    assert!(r.global_qualified());
    let sched = OutputSchedule::new(vec![0., 0.01, 0.02, 0.125, 0.5]).unwrap();
    let mut w = r
        .verified_clock_dense_workspace(&sched, C::Termwise, T::TinyRootReduced)
        .unwrap();
    let (v, a) = measure(|| r.replay_verified_clock_dense_into(&mut w));
    assert!(v.is_ok());
    assert_eq!(a.requests(), 0);
    let (t, y, rad) = w.view();
    assert_eq!(t, sched.times());
    assert_eq!(y.len(), 5);
    assert_eq!(rad.len(), 5);
    assert_eq!(
        y.last().unwrap(),
        r.accepted(r.accepted_len() - 1).unwrap().state()
    );
    let m = 0.5625;
    let d = 1. + 0.25 * m;
    assert!((y[4][0] - 0.25 / d).abs() <= rad[4][0] + 1e-16);
    assert_eq!(c.len(), 4);
}

#[test]
fn split_decay_dense_actual_portfolio() {
    use de::{ClockTubePlan as CP, PolynomialClockRoute as C, TriangularGoalRoute as T};
    use rodas5p_integrators::OutputSchedule;
    let (opts, policy, route, retention) = weighted_settings();
    let cases = [
        (
            "affine",
            vec![[1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.]],
            0.,
            0.5,
        ),
        (
            "quadratic",
            vec![[0.25, 0.5, 0.125]],
            vec![[-0.125, -0.25, -0.0625]],
            0.125,
            1.,
        ),
        (
            "ratio_third",
            vec![[3., 1.5, 0.75]],
            vec![[1., 0.5, 0.25]],
            0.125,
            0.5,
        ),
        (
            "long",
            vec![[0.5, 0.5, 0.125]],
            vec![[0.75, 0.75, 0.1875]],
            0.,
            2.,
        ),
        (
            "block4",
            vec![[0.25, 0.5, 0.125], [0.5, 0., 0.25]],
            vec![[-0.125, -0.25, -0.0625], [0.75, 0., 0.375]],
            0.125,
            0.5,
        ),
        (
            "zero_q",
            vec![[0.25, 0.25, 0.125]],
            vec![[0., 0., 0.]],
            0.,
            0.5,
        ),
        (
            "coupled_clock",
            vec![[0., 1., 0.]],
            vec![[0., 1., 0.]],
            0.,
            1.,
        ),
        (
            "holdout_block6",
            vec![[1., 0.25, 0.125], [0.125, 0.5, 0.25], [0.5, 0., 0.25]],
            vec![[0.25, 0.0625, 0.03125], [-0.25, -1., -0.5], [0., 0., 0.]],
            0.25,
            1.5,
        ),
        (
            "zero_p",
            vec![[1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.]],
            0.,
            0.5,
        ),
        (
            "fast_rate",
            vec![[32., 8., 2.]],
            vec![[4., 1., 0.25]],
            0.,
            0.5,
        ),
    ];
    for (case, a, b, start, end) in cases {
        let (src, mut x, mut z, branch, c) = if case == "coupled_clock" {
            coupled_clock_fixture()
        } else {
            polynomial_fixture(&a, &b)
        };
        if case == "zero_p" {
            x[0] = 0.;
            z[0] = 0.;
        }
        let epoch = EvolutionEpoch {
            start,
            end,
            incoming_x_error: Some(0.),
        };
        let call = |replay| {
            evolve_epoch_slab_replay_planned(
                &src,
                &x,
                &z,
                &branch,
                epoch,
                &opts,
                policy,
                route,
                &[],
                retention,
                replay,
                EnginePlan::SharedFused,
            )
        };
        let ((flat, audit), alloc) = measure(|| call(SlabReplayRoute::Flat));
        let flat = flat.unwrap();
        let term = flat.accepted(flat.accepted_len() - 1).unwrap();
        println!(
            "DECAY_RUN {}",
            serde_json::json!({"case":case,"candidate":"Flat","qualified":flat.global_qualified(),"rhs":audit.stats.as_ref().unwrap().counters.rhs_calls,"allocation":alloc,"terminal":term.state(),"terminal_bound":term.physical_error_upper(),"a":a,"b":b,"initial":x,"start":start,"end":end,"weights":c,"whole_bound":flat.max_physical_error()})
        );
        let mut retained = None;
        for candidate in [
            CP::BlockSplit,
            CP::BlockSplitDecay1,
            CP::BlockSplitDecay4,
            CP::BlockSplitRational4,
        ] {
            let ((r, audit), alloc) = measure(|| call(SlabReplayRoute::ClockWeighted(candidate)));
            let r = r.unwrap();
            assert_eq!(r.driver().t, flat.driver().t);
            assert_eq!(r.driver().y, flat.driver().y);
            let mut carry = de::ClockTubeCarry::new(&src, candidate, start, &x, Some(0.)).unwrap();
            let mut scratch = de::SlabCertificateWorkspace::new();
            let mut out = de::SlabCertificate::workspace_output(x.len(), x.len()).unwrap();
            let mut good = 0;
            for k in 0..flat.slabs_len() {
                let sl = flat.slab(k).unwrap();
                let rows: Vec<Vec<Iv>> = match sl.power {
                    RetainedPowerRows::Owned(a) => a
                        .iter()
                        .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                        .collect(),
                    RetainedPowerRows::Flat(a) => a
                        .iter()
                        .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                        .collect(),
                };
                let (v, allocation) = measure(|| {
                    carry.certify_into(
                        sl.times,
                        &rows,
                        &branch,
                        policy.tube_radius,
                        policy.green,
                        policy.init.condition_cap,
                        de::SlabWorkspaceRoute::Refined,
                        &mut scratch,
                        &mut out,
                    )
                });
                let actual = r.slab(k).unwrap();
                match v {
                    Ok(work) => {
                        let ac = actual.certificate.as_ref().unwrap();
                        assert_eq!(out.global_qualified, *ac.global_qualified);
                        assert_eq!(out.whole_x_error, *ac.whole_x_error);
                        assert_eq!(out.outgoing, *ac.outgoing);
                        if !out.global_qualified {
                            break;
                        }
                        assert_eq!(allocation.requests(), 0);
                        good += 1;
                        assert!(out.outgoing.unwrap() <= out.whole_x_error.unwrap());
                        println!(
                            "DECAY_SLAB {}",
                            serde_json::json!({"case":case,"candidate":candidate,"index":k,"times":sl.times,"power":rows.iter().map(|r|r.iter().map(|v|v.as_pair()).collect::<Vec<_>>()).collect::<Vec<_>>(),"work":work,"allocation":allocation,"x_error":out.whole_x_error,"z_error":out.whole_z_error,"outgoing":out.outgoing})
                        );
                    }
                    Err(e) => {
                        assert!(!out.global_qualified && out.whole_x_error.is_none());
                        assert!(actual.certificate.is_none());
                        println!(
                            "DECAY_REFUSAL {}",
                            serde_json::json!({"case":case,"candidate":candidate,"index":k,"reason":e.to_string()})
                        );
                        break;
                    }
                }
            }
            let term = r.accepted(r.accepted_len() - 1).unwrap();
            println!(
                "DECAY_RUN {}",
                serde_json::json!({"case":case,"candidate":candidate,"qualified":r.global_qualified(),"rhs":audit.stats.as_ref().unwrap().counters.rhs_calls,"allocation":alloc,"terminal":term.state(),"terminal_bound":term.physical_error_upper(),"a":a,"b":b,"initial":x,"start":start,"end":end,"weights":c,"whole_bound":r.max_physical_error(),"qualified_slabs":good})
            );
            if r.global_qualified() && retained.is_none() {
                retained = Some(r);
            }
        }
        let r = retained.unwrap_or(flat);
        assert!(r.global_qualified(), "{case} no qualified trajectory");
        let mut ts = vec![start, end];
        for k in 0..r.slabs_len() {
            let [l, h] = r.slab(k).unwrap().times;
            for t in [l + (h - l) * 0.25, l + (h - l) * 0.75, h] {
                if t > l && t <= h {
                    ts.push(t);
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup();
        let sched = OutputSchedule::new(ts).unwrap();
        let mut baseline = None;
        for candidate in [C::Termwise, C::Factored, C::Intersection] {
            for log in [T::TinyAdaptive, T::TinyRootReduced] {
                let mut w = r
                    .verified_clock_dense_workspace(&sched, candidate, log)
                    .unwrap();
                r.replay_verified_clock_dense_into(&mut w).unwrap(); // warm
                let (v, allocation) = measure(|| r.replay_verified_clock_dense_into(&mut w));
                let work = v.unwrap();
                assert_eq!(allocation.requests(), 0);
                let (ts, ys, rad) = w.view();
                assert_eq!(ts, sched.times());
                assert_eq!(work.requests, ts.len() - 1);
                if let Some(ref old) = baseline {
                    assert_eq!(ys, old);
                } else {
                    baseline = Some(ys.to_vec());
                }
                for k in 0..r.accepted_len() {
                    let q = r.accepted(k).unwrap();
                    let j = ts.binary_search_by(|t| t.total_cmp(&q.t())).unwrap();
                    assert_eq!(ys[j], q.state());
                }
                println!(
                    "CLOCK_DENSE {}",
                    serde_json::json!({"case":case,"candidate":candidate,"log":log,"qualified":true,"allocation":allocation,"times":ts,"centers":ys,"radii":rad,"work":work})
                );
            }
        }
    }
}

#[test]
fn verified_clock_dense_late_refusal() {
    use de::{ClockDenseSampler, PolynomialClockRoute as C, TriangularGoalRoute as T};
    use rodas5p_integrators::OutputSchedule;
    let (src, x, z, _, _) = polynomial_fixture(&[[1., 0.5, 0.]], &[[0.75, 0.375, 0.]]);
    let mut center = x.clone();
    center.extend(z);
    let sched = OutputSchedule::new(vec![0., 0.125, 0.875, 1.]).unwrap();
    let mut w = ClockDenseSampler::new(
        &src,
        (0., 1.),
        &x,
        &center,
        &sched,
        C::Termwise,
        T::TinyRootReduced,
    )
    .unwrap();
    let mut rows = [[Iv::point(0.).unwrap(); 5]; 4];
    rows[0][1] = Iv::point(f64::MAX).unwrap();
    rows[0][2] = Iv::point(f64::MAX).unwrap();
    assert!(w.advance([0., 1.], &rows, &center).is_err());
    assert_eq!(w.view().0, &[0.]);
    assert_eq!(w.view().1[0], center);
    assert_eq!(w.work().requests, 0);
    let rows = [[Iv::point(0.).unwrap(); 5]; 4];
    let (r, a) = measure(|| w.advance([0., 1.], &rows, &center));
    assert!(r.is_ok());
    assert_eq!(a.requests(), 0);
    assert!(w.complete());
    assert_eq!(w.view().0, sched.times());
    w.reset().unwrap();
    assert_eq!(w.view().0, &[0.]);
    println!(
        "DENSE_REFUSAL {}",
        serde_json::json!({"late_overflow":true,"unchanged_prefix":true,"retry_zeroheap":true,"reset":true})
    );
}

#[test]
fn decay_goal_budget_portfolio() {
    let (opts, policy, route, retention) = weighted_settings();
    for (case, a, b, start, end) in [
        (
            "long",
            vec![[0.5, 0.5, 0.125]],
            vec![[0.75, 0.75, 0.1875]],
            0.,
            2.,
        ),
        (
            "block4",
            vec![[0.25, 0.5, 0.125], [0.5, 0., 0.25]],
            vec![[-0.125, -0.25, -0.0625], [0.75, 0., 0.375]],
            0.125,
            0.5,
        ),
        (
            "zero_p",
            vec![[1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.]],
            0.,
            0.5,
        ),
        (
            "fast_rate",
            vec![[32., 8., 2.]],
            vec![[4., 1., 0.25]],
            0.,
            0.5,
        ),
    ] {
        let (src, mut x, mut z, branch, c) = polynomial_fixture(&a, &b);
        if case == "zero_p" {
            x[0] = 0.;
            z[0] = 0.;
        }
        let epoch = EvolutionEpoch {
            start,
            end,
            incoming_x_error: Some(0.),
        };
        for absolute in [1e-8, 1e-10] {
            let mut old = None;
            for plan in [
                GoalBudgetPlan::PolynomialBlockSplit,
                GoalBudgetPlan::PolynomialBlockDecay1,
                GoalBudgetPlan::PolynomialBlockDecay4,
                GoalBudgetPlan::PolynomialBlockRational4,
            ] {
                let (r, alloc) = measure(|| {
                    evolve_epoch_goal_budget(
                        &src,
                        &x,
                        &z,
                        &branch,
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
                });
                let last = r.attempts.last().unwrap();
                let ident = (r.total_rhs(), r.total_steps());
                if let Some(ref v) = old {
                    assert_eq!(&ident, v);
                } else {
                    old = Some(ident);
                }
                println!(
                    "DECAY_POLICY {}",
                    serde_json::json!({"case":case,"candidate":plan,"absolute":absolute,"qualified":last.accepted,"attempt":last,"rhs":r.total_rhs(),"steps":r.total_steps(),"allocation":alloc})
                );
            }
        }
    }
}

#[test]
fn split_decay_cost_and_dense_parameter_portfolio() {
    use de::{
        ClockTubePlan as CP, ParameterCovectorRoute as R, PolynomialClockRoute as C,
        TriangularGoalRoute as T,
    };
    use rodas5p_integrators::OutputSchedule;
    let (opts, policy, route, retention) = weighted_settings();
    let cases = [
        (
            "affine",
            vec![[1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.]],
            0.,
            0.5,
        ),
        (
            "quadratic",
            vec![[0.25, 0.5, 0.125]],
            vec![[-0.125, -0.25, -0.0625]],
            0.125,
            1.,
        ),
        (
            "ratio_third",
            vec![[3., 1.5, 0.75]],
            vec![[1., 0.5, 0.25]],
            0.125,
            0.5,
        ),
        (
            "long",
            vec![[0.5, 0.5, 0.125]],
            vec![[0.75, 0.75, 0.1875]],
            0.,
            2.,
        ),
        (
            "block4",
            vec![[0.25, 0.5, 0.125], [0.5, 0., 0.25]],
            vec![[-0.125, -0.25, -0.0625], [0.75, 0., 0.375]],
            0.125,
            0.5,
        ),
        (
            "zero_q",
            vec![[0.25, 0.25, 0.125]],
            vec![[0., 0., 0.]],
            0.,
            0.5,
        ),
        (
            "coupled_clock",
            vec![[0., 1., 0.]],
            vec![[0., 1., 0.]],
            0.,
            1.,
        ),
        (
            "holdout_block6",
            vec![[1., 0.25, 0.125], [0.125, 0.5, 0.25], [0.5, 0., 0.25]],
            vec![[0.25, 0.0625, 0.03125], [-0.25, -1., -0.5], [0., 0., 0.]],
            0.25,
            1.5,
        ),
        (
            "zero_p",
            vec![[1., 0.5, 0.]],
            vec![[0.75, 0.375, 0.]],
            0.,
            0.5,
        ),
        (
            "fast_rate",
            vec![[32., 8., 2.]],
            vec![[4., 1., 0.25]],
            0.,
            0.5,
        ),
    ];
    let plans = [
        CP::BlockSplit,
        CP::BlockSplitDecay1,
        CP::BlockSplitDecay2,
        CP::BlockSplitRational2,
        CP::BlockSplitDecay4,
        CP::BlockSplitRational4,
    ];
    for (case, a, b, start, end) in cases {
        let (src, mut x, mut z, branch, c) = if case == "coupled_clock" {
            coupled_clock_fixture()
        } else {
            polynomial_fixture(&a, &b)
        };
        if case == "zero_p" {
            x[0] = 0.;
            z[0] = 0.;
        }
        let epoch = EvolutionEpoch {
            start,
            end,
            incoming_x_error: Some(0.),
        };
        let call = |replay| {
            evolve_epoch_slab_replay_planned(
                &src,
                &x,
                &z,
                &branch,
                epoch,
                &opts,
                policy,
                route,
                &[],
                retention,
                replay,
                EnginePlan::SharedFused,
            )
        };
        let (flat, audit) = call(SlabReplayRoute::Flat);
        let flat = flat.unwrap();
        let rhs = audit.stats.as_ref().unwrap().counters.rhs_calls;
        let mut rows = vec![];
        for k in 0..flat.slabs_len() {
            let sl = flat.slab(k).unwrap();
            let power: Vec<Vec<Iv>> = match sl.power {
                RetainedPowerRows::Owned(a) => a
                    .iter()
                    .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                    .collect(),
                RetainedPowerRows::Flat(a) => a
                    .iter()
                    .map(|r| r.iter().map(|p| Iv::new(p[0], p[1]).unwrap()).collect())
                    .collect(),
            };
            rows.push((sl.times, power));
        }
        println!(
            "DECAY_COST_MODEL {}",
            serde_json::json!({"case":case,"a":a,"b":b,"initial":x,"start":start,"end":end,"weights":c,"rhs":rhs})
        );
        let (mut scratch, mut out) = (
            de::SlabCertificateWorkspace::new(),
            de::SlabCertificate::workspace_output(x.len(), x.len()).unwrap(),
        );
        let mut results = vec![];
        let mut selected = None;
        for candidate in plans {
            let ((run, audit), alloc) = measure(|| call(SlabReplayRoute::ClockWeighted(candidate)));
            let run = run.unwrap();
            assert!(
                run.global_qualified(),
                "{case} {candidate:?} {:?}",
                run.reason()
            );
            assert_eq!(run.driver().t, flat.driver().t);
            assert_eq!(run.driver().y, flat.driver().y);
            assert_eq!(audit.stats.as_ref().unwrap().counters.rhs_calls, rhs);
            let (mut carry, setup) =
                measure(|| de::ClockTubeCarry::new(&src, candidate, start, &x, Some(0.)).unwrap());
            let mut slab_results = Vec::with_capacity(rows.len());
            let ((), warm) = measure(|| {
                for (i, (times, power)) in rows.iter().enumerate() {
                    let w = carry
                        .certify_into(
                            *times,
                            power,
                            &branch,
                            policy.tube_radius,
                            policy.green,
                            policy.init.condition_cap,
                            de::SlabWorkspaceRoute::Refined,
                            &mut scratch,
                            &mut out,
                        )
                        .unwrap();
                    let actual = run.slab(i).unwrap();
                    let ac = actual.certificate.as_ref().unwrap();
                    assert!(out.global_qualified);
                    assert_eq!(out.whole_x_error, *ac.whole_x_error);
                    assert_eq!(out.outgoing, *ac.outgoing);
                    assert!(out.outgoing.unwrap() <= out.whole_x_error.unwrap());
                    slab_results.push((
                        w,
                        out.whole_x_error.unwrap(),
                        out.whole_z_error.unwrap(),
                        out.outgoing.unwrap(),
                    ));
                }
            });
            assert_eq!(warm.requests(), 0);
            for (k, ((times, power), (w, ex, ez, ep))) in rows.iter().zip(&slab_results).enumerate()
            {
                println!(
                    "DECAY_COST_SLAB {}",
                    serde_json::json!({"case":case,"candidate":candidate,"index":k,"times":times,"power":power.iter().map(|r|r.iter().map(|v|v.as_pair()).collect::<Vec<_>>()).collect::<Vec<_>>(),"work":w,"x_error":ex,"z_error":ez,"outgoing":ep})
                );
            }
            let term = run.accepted(run.accepted_len() - 1).unwrap();
            let val = (
                term.physical_error_upper().unwrap(),
                run.max_physical_error().unwrap(),
                term.state().to_vec(),
                setup,
                warm,
                alloc,
                slab_results.last().unwrap().3,
            );
            results.push(val);
            if candidate == CP::BlockSplitDecay2 {
                selected = Some(run);
            }
        }
        // Balanced cyclic order, one candidate in each position across six repeats.
        // Only direct carry certification is timed; row conversion/setup are excluded.
        let mut samples: [Vec<u128>; 6] = std::array::from_fn(|_| Vec::with_capacity(6));
        for rep in 0..6 {
            for j in 0..6 {
                let i = (rep + j) % 6;
                let candidate = plans[i];
                let mut carry =
                    de::ClockTubeCarry::new(&src, candidate, start, &x, Some(0.)).unwrap();
                let t = std::time::Instant::now();
                for (times, power) in &rows {
                    let r = carry
                        .certify_into(
                            *times,
                            power,
                            &branch,
                            policy.tube_radius,
                            policy.green,
                            policy.init.condition_cap,
                            de::SlabWorkspaceRoute::Refined,
                            &mut scratch,
                            &mut out,
                        )
                        .unwrap();
                    std::hint::black_box(r);
                }
                samples[i].push(t.elapsed().as_nanos());
                assert!(out.global_qualified);
                assert_eq!(out.outgoing.unwrap(), results[i].6);
            }
        }
        for (i, candidate) in plans.iter().enumerate() {
            let v = &results[i];
            let mut sorted = samples[i].clone();
            sorted.sort_unstable();
            println!(
                "DECAY_COST {}",
                serde_json::json!({"case":case,"candidate":candidate,"qualified":true,"rhs":rhs,"terminal_bound":v.0,"whole_bound":v.1,"terminal":v.2,"setup":v.3,"allocation":v.4,"full_allocation":v.5,"slabs":rows.len(),"median_ns":(sorted[2]+sorted[3])/2,"samples_ns":samples[i]})
            );
        }
        let run = selected.unwrap();
        let ts = vec![
            start,
            start + (end - start) * 0.25,
            start + (end - start) * 0.75,
            end,
        ];
        let sched = OutputSchedule::new(ts).unwrap();
        let mut dense = run
            .verified_clock_dense_workspace(&sched, C::Termwise, T::TinyRootReduced)
            .unwrap();
        let (res, alloc) = measure(|| run.replay_verified_clock_dense_into(&mut dense));
        assert!(res.is_ok());
        assert_eq!(alloc.requests(), 0);
        let (times, centers, radii) = dense.view();
        let mut grad = vec![Iv::point(17.).unwrap(); 2 * x.len()];
        let mut terminal = grad.clone();
        run.polynomial_parameter_covector_into(
            &c,
            C::Termwise,
            T::TinyRootReduced,
            R::Intersection,
            &mut terminal,
        )
        .unwrap();
        for (i, &t) in times.iter().enumerate() {
            for candidate in [R::Separate, R::Factored, R::Intersection] {
                let (r, allocation) = measure(|| {
                    run.polynomial_dense_parameter_covector_into(
                        &dense,
                        i,
                        &c,
                        C::Termwise,
                        T::TinyRootReduced,
                        candidate,
                        &mut grad,
                    )
                });
                let work = r.unwrap();
                assert_eq!(allocation.requests(), 0);
                if i == 0 {
                    assert!(grad.iter().all(|v| v.contains(0.)));
                }
                if i + 1 == times.len() && candidate == R::Intersection {
                    assert_eq!(grad, terminal);
                }
                println!(
                    "DENSE_PARAMETER {}",
                    serde_json::json!({"case":case,"request":i,"time":t,"candidate":candidate,"qualified":true,"center":centers[i],"radius":radii[i],"gradient":grad.iter().map(|v|v.as_pair()).collect::<Vec<_>>(),"allocation":allocation,"work":work})
                );
            }
        }
        grad.fill(Iv::point(17.).unwrap());
        assert!(
            run.polynomial_dense_parameter_covector_into(
                &dense,
                times.len(),
                &c,
                C::Termwise,
                T::TinyRootReduced,
                R::Intersection,
                &mut grad
            )
            .is_err()
        );
        assert!(grad.iter().all(|v| *v == Iv::point(17.).unwrap()));
        dense.reset().unwrap();
        assert!(
            run.polynomial_dense_parameter_covector_into(
                &dense,
                0,
                &c,
                C::Termwise,
                T::TinyRootReduced,
                R::Intersection,
                &mut grad
            )
            .is_err()
        );
        assert!(grad.iter().all(|v| *v == Iv::point(17.).unwrap()));
    }
}
