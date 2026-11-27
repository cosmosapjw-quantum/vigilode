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
