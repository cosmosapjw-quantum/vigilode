#![cfg(feature = "polynomial-events")]
use vigilode::polynomial_events::{AffineFactor, DeclaredPolynomial, EventTerm, retro_dae_init::*};
fn term(c: f64, fs: Vec<AffineFactor>) -> EventTerm {
    EventTerm {
        coefficient: c,
        factors: fs,
    }
}
fn p(ts: Vec<EventTerm>) -> DeclaredPolynomial {
    DeclaredPolynomial::new(ts).unwrap()
}
#[test]
fn projection_keeps_x_and_computes_tangent() {
    let x = AffineFactor::component(0, 0.);
    let z = AffineFactor::component(1, 0.);
    let src = DaeSource::new(
        vec![p(vec![term(-1., vec![x.clone()])])],
        vec![p(vec![term(1., vec![z]), term(-1., vec![x.clone(), x])])],
    )
    .unwrap();
    let r = src
        .initialize(
            0.,
            &[1.],
            &[0.5],
            &[[0., 2.]],
            InitPolicy {
                mode: InitMode::ProjectAlgebraic,
                ..Default::default()
            },
            InitRoute::Direct,
        )
        .unwrap();
    assert_eq!(r.x, vec![1.]);
    assert_eq!(r.z, vec![1.]);
    assert_eq!(r.zdot, vec![-2.]);
}

use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};
use vigilode::polynomial_events::initialize_mass_problem;
use vigilode::retro_research::retro_portfolio::Portfolio;
use vigilode::{Atol, Options, Rodas5p};
const ROUTES: [&str; 8] = [
    "Direct",
    "Reuse",
    "FaerBatch",
    "PreparedSerial",
    "PreparedBatch",
    "PreparedRayon",
    "ScalarRoot",
    "Adaptive",
];
fn route(s: &str) -> Option<InitRoute> {
    match s {
        "Direct" => Some(InitRoute::Direct),
        "Reuse" => Some(InitRoute::Reuse),
        "FaerBatch" => Some(InitRoute::FaerBatch),
        "PreparedSerial" => Some(InitRoute::PreparedSerial),
        "PreparedBatch" => Some(InitRoute::PreparedBatch),
        "PreparedRayon" => Some(InitRoute::PreparedRayon),
        "ScalarRoot" => Some(InitRoute::ScalarRoot),
        "Adaptive" => None,
        _ => panic!("route"),
    }
}
fn y(i: usize) -> AffineFactor {
    AffineFactor::component(i, 0.)
}
fn c(v: f64) -> EventTerm {
    term(v, vec![])
}
fn clock() -> AffineFactor {
    AffineFactor::clock(0.)
}
fn linear(i: usize, v: f64) -> EventTerm {
    term(v, vec![y(i)])
}
fn policy(mode: InitMode, tight: bool) -> InitPolicy {
    InitPolicy {
        mode,
        state_tol: if tight { 1e-12 } else { 1e-9 },
        residual_tol: if tight { 1e-10 } else { 1e-9 },
        derivative_tol: if tight { 1e-9 } else { 1e-7 },
        ..Default::default()
    }
}
struct Fixture {
    name: &'static str,
    group: &'static str,
    src: DaeSource,
    t: f64,
    x: Vec<f64>,
    z: Vec<f64>,
    branch: Vec<[f64; 2]>,
    root: Vec<f64>,
    mode: InitMode,
    expected: bool,
}
fn analytic() -> DaeSource {
    DaeSource::new(
        vec![p(vec![linear(0, -1.)])],
        vec![p(vec![
            linear(1, 1.),
            term(-1., vec![y(0), y(0)]),
            term(-1., vec![clock()]),
        ])],
    )
    .unwrap()
}
fn robertson() -> DaeSource {
    DaeSource::new(
        vec![
            p(vec![linear(0, -0.04), term(1e4, vec![y(1), y(2)])]),
            p(vec![
                linear(0, 0.04),
                term(-1e4, vec![y(1), y(2)]),
                term(-3e7, vec![y(1), y(1)]),
            ]),
        ],
        vec![p(vec![linear(0, 1.), linear(1, 1.), linear(2, 1.), c(-1.)])],
    )
    .unwrap()
}
fn quad() -> DaeSource {
    DaeSource::new(
        vec![p(vec![linear(0, -1.), linear(1, 0.25)])],
        vec![p(vec![
            term(1., vec![y(1), y(1)]),
            linear(0, -1.),
            term(-1., vec![clock()]),
        ])],
    )
    .unwrap()
}
fn cubic() -> DaeSource {
    DaeSource::new(
        vec![p(vec![linear(0, -1.), linear(1, 0.125)])],
        vec![p(vec![
            term(1., vec![y(1), y(1), y(1)]),
            linear(1, 1.),
            linear(0, -1.),
        ])],
    )
    .unwrap()
}
fn coupled(n: usize, nd: usize, sparse: bool, nonlinear: bool) -> Fixture {
    let x = (0..nd)
        .map(|i| 0.25 + (i % 3) as f64 / 16.)
        .collect::<Vec<_>>();
    let root = (0..n)
        .map(|i| 0.5 + (i % 5) as f64 / 16.)
        .collect::<Vec<_>>();
    let f = (0..nd)
        .map(|i| p(vec![linear(i, -1.), linear(nd + i % n, 0.25)]))
        .collect::<Vec<_>>();
    let g = (0..n)
        .map(|i| {
            let mut ts = vec![
                term(
                    if sparse { 2. } else { 4. },
                    vec![AffineFactor::component(nd + i, root[i])],
                ),
                term(-0.125, vec![AffineFactor::component(i % nd, x[i % nd])]),
            ];
            let j = (i + 1) % n;
            if sparse {
                ts.push(term(0.125, vec![AffineFactor::component(nd + j, root[j])]));
            } else {
                for j in 0..n {
                    if j != i {
                        ts.push(term(0.0625, vec![AffineFactor::component(nd + j, root[j])]));
                    }
                }
            }
            if nonlinear {
                let a = AffineFactor::component(nd + i, root[i]);
                ts.push(term(0.25, vec![a.clone(), a]));
            }
            p(ts)
        })
        .collect();
    let src = DaeSource::new(f, g).unwrap();
    Fixture {
        name: if nonlinear {
            "coupled-nonlinear"
        } else if sparse {
            "sparse-batch"
        } else {
            "dense-batch"
        },
        group: "multi",
        src,
        t: 0.,
        x,
        z: root.iter().map(|v| v + 0.125).collect(),
        branch: root.iter().map(|v| [v - 0.25, v + 0.25]).collect(),
        root,
        mode: InitMode::ProjectAlgebraic,
        expected: true,
    }
}
fn nondd() -> Fixture {
    let a = AffineFactor::component(1, 1.);
    let b = AffineFactor::component(2, -0.5);
    Fixture {
        name: "nondominant",
        group: "nondd",
        src: DaeSource::new(
            vec![p(vec![linear(0, -1.), linear(1, 0.25)])],
            vec![
                p(vec![
                    term(1., vec![a.clone()]),
                    term(2., vec![b.clone()]),
                    term(-0.25, vec![AffineFactor::component(0, 0.5)]),
                ]),
                p(vec![term(3., vec![a]), term(1., vec![b])]),
            ],
        )
        .unwrap(),
        t: 0.,
        x: vec![0.5],
        z: vec![0.75, -0.25],
        branch: vec![[0., 2.], [-1., 0.]],
        root: vec![1., -0.5],
        mode: InitMode::ProjectAlgebraic,
        expected: true,
    }
}
fn fixtures(holdout: bool) -> Vec<Fixture> {
    if holdout {
        let mut sparse = coupled(6, 4, true, false);
        sparse.name = "sparse-holdout";
        let mut nd = nondd();
        nd.name = "nondd-holdout";
        nd.z = vec![1.25, -0.75];
        return vec![
            Fixture {
                name: "analytic-holdout",
                group: "scalar",
                src: analytic(),
                t: 0.25,
                x: vec![0.5],
                z: vec![0.75],
                branch: vec![[0., 1.]],
                root: vec![0.5],
                mode: InitMode::ProjectAlgebraic,
                expected: true,
            },
            Fixture {
                name: "quadratic-holdout",
                group: "branch",
                src: quad(),
                t: 0.25,
                x: vec![2.],
                z: vec![1.25],
                branch: vec![[0.5, 2.5]],
                root: vec![1.5],
                mode: InitMode::ProjectAlgebraic,
                expected: true,
            },
            sparse,
            nd,
        ];
    }
    vec![
        Fixture {
            name: "analytic",
            group: "scalar",
            src: analytic(),
            t: 0.25,
            x: vec![1.],
            z: vec![0.5],
            branch: vec![[0., 2.]],
            root: vec![1.25],
            mode: InitMode::ProjectAlgebraic,
            expected: true,
        },
        Fixture {
            name: "robertson",
            group: "scalar",
            src: robertson(),
            t: 0.,
            x: vec![1., 0.],
            z: vec![0.125],
            branch: vec![[-0.25, 0.25]],
            root: vec![0.],
            mode: InitMode::ProjectAlgebraic,
            expected: true,
        },
        Fixture {
            name: "strict",
            group: "scalar",
            src: analytic(),
            t: 0.,
            x: vec![1.],
            z: vec![1.],
            branch: vec![[0., 2.]],
            root: vec![1.],
            mode: InitMode::Strict,
            expected: true,
        },
        Fixture {
            name: "quad",
            group: "branch",
            src: quad(),
            t: 0.,
            x: vec![2.],
            z: vec![1.25],
            branch: vec![[0.5, 2.]],
            root: vec![2f64.sqrt()],
            mode: InitMode::ProjectAlgebraic,
            expected: true,
        },
        Fixture {
            name: "cubic-wide",
            group: "branch",
            src: cubic(),
            t: 0.,
            x: vec![2.],
            z: vec![0.5],
            branch: vec![[-2., 2.]],
            root: vec![1.],
            mode: InitMode::ProjectAlgebraic,
            expected: true,
        },
        coupled(8, 8, true, false),
        coupled(6, 4, false, false),
        coupled(4, 3, true, true),
        nondd(),
    ]
}
#[test]
fn strict_preserves_input_and_branch_refusals() {
    let src = analytic();
    let r = src
        .initialize(
            0.,
            &[1.],
            &[1.],
            &[[0., 2.]],
            policy(InitMode::Strict, false),
            InitRoute::Direct,
        )
        .unwrap();
    assert!(r.exact_center_consistency);
    assert_eq!(r.correction_upper, 0.);
    assert_eq!(r.zdot, vec![-1.]); // g=z-x²-t => zdot=1-2
    let r = src
        .initialize(
            0.,
            &[1.],
            &[1. + 2f64.powi(-40)],
            &[[0., 2.]],
            policy(InitMode::Strict, false),
            InitRoute::Direct,
        )
        .unwrap();
    assert_eq!(r.z[0].to_bits(), (1. + 2f64.powi(-40)).to_bits());
    assert!(!r.exact_center_consistency);
    assert!(r.root_box[0][0] <= 1. && r.root_box[0][1] >= 1.);
    assert!(
        src.initialize(
            0.,
            &[1.],
            &[0.5],
            &[[0., 2.]],
            policy(InitMode::Strict, false),
            InitRoute::Direct
        )
        .is_err()
    );
    let src = quad();
    for route in [InitRoute::Direct, InitRoute::ScalarRoot] {
        assert!(
            src.initialize(
                0.,
                &[1.],
                &[0.5],
                &[[-2., 2.]],
                policy(InitMode::ProjectAlgebraic, false),
                route
            )
            .is_err()
        );
        assert!(
            src.initialize(
                0.,
                &[0.],
                &[0.],
                &[[-1., 1.]],
                policy(InitMode::ProjectAlgebraic, false),
                route
            )
            .is_err()
        );
    }
    let src = cubic();
    assert!(
        src.initialize(
            0.,
            &[2.],
            &[0.5],
            &[[-2., 2.]],
            policy(InitMode::ProjectAlgebraic, false),
            InitRoute::Direct
        )
        .is_err()
    );
    let r = src
        .initialize(
            0.,
            &[2.],
            &[0.5],
            &[[-2., 2.]],
            policy(InitMode::ProjectAlgebraic, false),
            InitRoute::ScalarRoot,
        )
        .unwrap();
    assert_eq!(r.z, vec![1.]);
    assert_eq!(r.branch_authority, "exact scalar root count");
}
#[test]
fn rank_condition_and_prepared_admission() {
    let f = nondd();
    let r = f
        .src
        .initialize(
            f.t,
            &f.x,
            &f.z,
            &f.branch,
            policy(f.mode, false),
            InitRoute::FaerBatch,
        )
        .unwrap();
    assert!(r.qualified);
    assert_eq!(r.rank, 2);
    assert!(
        f.src
            .initialize(
                f.t,
                &f.x,
                &f.z,
                &f.branch,
                policy(f.mode, false),
                InitRoute::PreparedBatch
            )
            .is_err()
    );
    let f = coupled(8, 8, true, false);
    let r = f
        .src
        .initialize(
            f.t,
            &f.x,
            &f.z,
            &f.branch,
            policy(f.mode, false),
            InitRoute::PreparedBatch,
        )
        .unwrap();
    assert!(r.work.prepared_support_batches > 0 && r.work.faer_batches > 0);
    let f = DaeSource::new(
        vec![p(vec![linear(0, -1.)])],
        vec![
            p(vec![term(
                2f64.powi(-40),
                vec![AffineFactor::component(1, 1.)],
            )]),
            p(vec![term(1., vec![AffineFactor::component(2, 1.)])]),
        ],
    )
    .unwrap();
    assert!(
        f.initialize(
            0.,
            &[1.],
            &[1., 1.],
            &[[0., 2.], [0., 2.]],
            policy(InitMode::Strict, false),
            InitRoute::Direct
        )
        .is_err()
    );
}
#[test]
fn mass_client_same_source() {
    let o = Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        dense_output: true,
        ..Default::default()
    };
    for (name, src, x, z, b, end) in [
        (
            "analytic",
            analytic(),
            vec![1.],
            vec![0.5],
            vec![[0., 2.]],
            1.,
        ),
        (
            "robertson",
            robertson(),
            vec![1., 0.],
            vec![0.125],
            vec![[-0.25, 0.25]],
            1.,
        ),
    ] {
        let init = initialize_mass_problem(
            &src,
            0.,
            &x,
            &z,
            &b,
            policy(InitMode::ProjectAlgebraic, false),
            InitRoute::Direct,
        )
        .unwrap();
        let r = Rodas5p::solve(&init.problem, (0., end), &init.initial_state, &o).unwrap();
        assert!(r.is_success(), "{name}: {:?}", r.status);
        let mut residual = 0.0_f64;
        for (t, y) in r.t.iter().zip(&r.y) {
            let q = src.rhs_values(*t, y).unwrap();
            residual = residual.max(q[src.nx()..].iter().map(|v| v.abs()).fold(0., f64::max));
        }
        assert!(residual < 1e-5);
        if name == "analytic" {
            let v = r.final_state().unwrap();
            assert!((v[0] - (-1_f64).exp()).abs() < 1e-5);
            assert!((v[1] - 1. - (-2_f64).exp()).abs() < 1e-5);
        }
        println!(
            "MASS {}",
            json!({"case":name,"source":src,"init":init.report,"end":end,"times":r.t,"states":r.y,"constraint_max_center":residual,"counters":r.stats.counters})
        );
    }
}
fn source(f: &Fixture) {
    println!(
        "SOURCE {}",
        json!({"case":f.name,"group":f.group,"src":f.src,"t":f.t,"x":f.x,"z":f.z,"branch":f.branch,"root_hint":f.root,"mode":f.mode,"expected":f.expected})
    );
}
fn measure(f: &Fixture, c: &str, tight: bool) -> Value {
    let pol = policy(f.mode, tight);
    let mut portfolio = Portfolio::new(4, 7).unwrap();
    let mut times = vec![];
    let mut last = None;
    for i in 0..4 {
        let t = Instant::now();
        let r = if let Some(route) = route(c) {
            f.src
                .initialize(f.t, &f.x, &f.z, &f.branch, pol, route)
                .map(|r| json!({"report":r,"attempts":[]}))
        } else {
            f.src
                .adaptive(f.t, &f.x, &f.z, &f.branch, pol, &mut portfolio)
                .map(|r| json!({"report":r.value,"attempts":r.attempts}))
        };
        let ns = t.elapsed().as_nanos() as u64;
        if i > 0 {
            times.push(ns);
        }
        last = Some(black_box(r));
    }
    times.sort_unstable();
    match last.unwrap() {
        Ok(r) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"tight":tight,"policy":pol,"median_ns":times[1],"samples_ns":times,"result":r})
        }
        Err(e) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"tight":tight,"policy":pol,"median_ns":times[1],"samples_ns":times,"result":null,"reason":e.to_string()})
        }
    }
}
#[test]
#[ignore = "finite candidate campaign"]
fn training() {
    for (i, f) in fixtures(false).iter().enumerate() {
        source(f);
        for tight in [false, true] {
            for j in 0..ROUTES.len() {
                println!("CELL {}", measure(f, ROUTES[(i + j) % ROUTES.len()], tight));
            }
        }
    }
}
#[test]
#[ignore = "frozen candidate selection"]
fn holdout() {
    let file = std::env::var("VIGILODE_DAE_SELECTION").unwrap();
    let s: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    for f in fixtures(true) {
        source(&f);
        for (key, tight) in [("loose", false), ("tight", true)] {
            if let Some(c) = s["groups"][f.group][key]["selected"].as_str() {
                println!("CELL {}", measure(&f, c, tight));
            }
        }
    }
}
