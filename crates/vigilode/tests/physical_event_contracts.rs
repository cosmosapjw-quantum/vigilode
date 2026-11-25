#![cfg(feature = "polynomial-events")]
use rodas5p_core::DenseMatrix;
use rodas5p_integrators::AdaptiveStepConfig;
use serde_json::json;
use std::{hint::black_box, time::Instant};
use vigilode::polynomial_events::{
    AffineFactor, DeclaredPolynomial, EventTerm, RootKind, retro_physical_event::*,
};
use vigilode::retro_research::retro_portfolio::Portfolio;
const TOL: f64 = 1e-5;
const ROUTES: [&str; 7] = [
    "Unity",
    "AffineCarry",
    "Green",
    "Weighted",
    "WeightedHint",
    "SourceAction",
    "Adaptive",
];
fn route(s: &str) -> Option<PhysicalRoute> {
    match s {
        "Unity" => Some(PhysicalRoute::Unity),
        "AffineCarry" => Some(PhysicalRoute::AffineCarry),
        "Green" => Some(PhysicalRoute::Green),
        "Weighted" => Some(PhysicalRoute::Weighted),
        "WeightedHint" => Some(PhysicalRoute::WeightedHint),
        "SourceAction" => Some(PhysicalRoute::SourceAction),
        "Adaptive" => None,
        _ => panic!("route"),
    }
}
fn poly(i: usize, r: f64, m: usize) -> DeclaredPolynomial {
    DeclaredPolynomial::new(vec![EventTerm {
        coefficient: 1.,
        factors: vec![AffineFactor::component(i, r); m],
    }])
    .unwrap()
}
struct Fixture {
    name: &'static str,
    group: &'static str,
    a: Vec<f64>,
    b: Vec<f64>,
    y: Vec<f64>,
    end: f64,
    g: DeclaredPolynomial,
    expected: &'static str,
}
fn fixtures(holdout: bool) -> Vec<Fixture> {
    if holdout {
        vec![
            Fixture {
                name: "forced-holdout",
                group: "scalar",
                a: vec![-2.],
                b: vec![0.25],
                y: vec![1.25],
                end: 1.5,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
            Fixture {
                name: "touch-holdout",
                group: "touch",
                a: vec![-4.],
                b: vec![1.],
                y: vec![1.25],
                end: 1.,
                g: poly(0, 0.625, 2),
                expected: "Unique",
            },
            Fixture {
                name: "triangular-holdout",
                group: "triangular",
                a: vec![-4., 2., 0., -4.],
                b: vec![0.25, 0.5],
                y: vec![1., 0.75],
                end: 1.,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
            Fixture {
                name: "coupled-holdout",
                group: "coupled",
                a: vec![-3., -0.5, 0.25, -2.],
                b: vec![0., 0.],
                y: vec![1., 0.75],
                end: 1.,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
        ]
    } else {
        vec![
            Fixture {
                name: "decay",
                group: "scalar",
                a: vec![-1.],
                b: vec![0.],
                y: vec![1.],
                end: 2.,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
            Fixture {
                name: "forced",
                group: "scalar",
                a: vec![-2.],
                b: vec![0.25],
                y: vec![1.],
                end: 1.5,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
            Fixture {
                name: "no-root",
                group: "scalar",
                a: vec![-1.],
                b: vec![0.],
                y: vec![1.],
                end: 1.,
                g: poly(0, -0.25, 1),
                expected: "NoRoot",
            },
            Fixture {
                name: "stiff",
                group: "scalar",
                a: vec![-1000.],
                b: vec![0.],
                y: vec![1.],
                end: 0.01,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
            Fixture {
                name: "structural-touch",
                group: "touch",
                a: vec![-2.],
                b: vec![0.],
                y: vec![1.],
                end: 1.,
                g: poly(0, 0.5, 2),
                expected: "Unique",
            },
            Fixture {
                name: "triangular",
                group: "triangular",
                a: vec![-2., 1., 0., -2.],
                b: vec![0., 0.],
                y: vec![1., 1.],
                end: 1.5,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
            Fixture {
                name: "triangular-no-root",
                group: "triangular",
                a: vec![-2., 1., 0., -2.],
                b: vec![0., 0.],
                y: vec![1., 1.],
                end: 1.,
                g: poly(0, -0.25, 1),
                expected: "NoRoot",
            },
            Fixture {
                name: "coupled",
                group: "coupled",
                a: vec![-3., 1.5, 0.25, -2.],
                b: vec![0., 0.],
                y: vec![1., 0.4],
                end: 1.5,
                g: poly(0, 0.5, 1),
                expected: "Unique",
            },
            Fixture {
                name: "coupled-polynomial",
                group: "coupled",
                a: vec![-3., 1.5, 0.25, -2.],
                b: vec![0., 0.],
                y: vec![1., 0.4],
                end: 1.5,
                g: DeclaredPolynomial::new(vec![
                    EventTerm {
                        coefficient: 1.,
                        factors: vec![
                            AffineFactor::component(0, 0.),
                            AffineFactor::component(1, 0.),
                        ],
                    },
                    EventTerm {
                        coefficient: -0.125,
                        factors: vec![],
                    },
                ])
                .unwrap(),
                expected: "Unique",
            },
        ]
    }
}
fn path(f: &Fixture) -> PhysicalPath {
    let n = f.y.len();
    let a = DenseMatrix::new(n, n, f.a.clone()).unwrap();
    AffineSource::new(a, f.b.clone())
        .unwrap()
        .integrate(
            &f.y,
            f.end,
            &AdaptiveStepConfig {
                rtol: 1e-8,
                atol: 1e-10,
                initial_step: 0.01_f64.min(f.end / 32.),
                max_step: 0.125_f64.min(f.end / 4.),
                ..Default::default()
            },
        )
        .unwrap()
}
fn fixture_record(f: &Fixture, p: &PhysicalPath) {
    let s = p.segments();
    let c = rodas5p_core::rodas5p_coefficients().unwrap();
    let h = (0..3)
        .map(|i| (0..8).map(|j| c.dense_h[(i, j)]).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let packets=(0..s.len()).map(|k|json!({"times":[s.t_old(k),s.t_new(k)],"y0":s.state(k),"y1":s.state(k+1),"u":s.stage_rows(k)})).collect::<Vec<_>>();
    println!(
        "SOURCE {}",
        json!({"name":f.name,"group":f.group,"a":f.a,"b":f.b,"y":f.y,"end":f.end,"g":f.g,"expected":f.expected,"H":h,"packets":packets,"native_counters":p.native_counters()})
    );
}
fn measured(f: &Fixture, p: &PhysicalPath, c: &str) -> serde_json::Value {
    measured_tol(f, p, c, TOL)
}
fn measured_tol(f: &Fixture, p: &PhysicalPath, c: &str, tol: f64) -> serde_json::Value {
    let mut sel = Portfolio::new(4, 6).unwrap();
    let mut ns = vec![];
    let mut last = None;
    for i in 0..4 {
        let t = Instant::now();
        let r = if let Some(m) = route(c) {
            p.certify(&f.g, m, tol).map(|r| (Some(r), vec![]))
        } else {
            p.adaptive(&f.g, &mut sel, tol)
                .map(|r| (r.value, r.attempts))
        };
        let dt = t.elapsed().as_nanos() as u64;
        if i > 0 {
            ns.push(dt);
        }
        last = Some(black_box(r));
    }
    ns.sort_unstable();
    match last.unwrap() {
        Ok((r, attempts)) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"expected":f.expected,"median_ns":ns[1],"samples_ns":ns,"report":r,"attempts":attempts})
        }
        Err(e) => {
            json!({"case":f.name,"group":f.group,"candidate":c,"expected":f.expected,"median_ns":ns[1],"samples_ns":ns,"report":null,"reason":e.to_string()})
        }
    }
}
#[test]
fn scalar_physical_root_and_no_root() {
    let f = fixtures(false).remove(0);
    let p = path(&f);
    for c in ROUTES {
        let r = measured(&f, &p, c);
        let v = &r["report"];
        assert_eq!(v["status"], "Unique", "{c}: {v}");
        assert_eq!(v["qualified"], true);
        let b = v["bracket"].as_array().unwrap();
        assert!(
            b[0].as_f64().unwrap() <= 2_f64.ln() && b[1].as_f64().unwrap() >= 2_f64.ln(),
            "{c}: {b:?}"
        );
        assert!(v["width"].as_f64().unwrap() <= TOL);
    }
    let r = p
        .certify(&poly(0, -0.25, 1), PhysicalRoute::Weighted, TOL)
        .unwrap();
    assert_eq!(r.status, PhysicalStatus::NoRoot);
    assert!(r.qualified);
}
#[test]
fn real_touch_is_structural_and_coupled_source() {
    let f = fixtures(false).remove(4);
    let p = path(&f);
    let r = p.certify(&f.g, PhysicalRoute::WeightedHint, TOL).unwrap();
    assert!(r.qualified);
    assert_eq!(r.kind, Some(RootKind::Touch));
    assert_eq!(r.structural_multiplicity, Some(2));
    assert!(r.bracket.unwrap()[0] <= 2_f64.ln() / 2. && r.bracket.unwrap()[1] >= 2_f64.ln() / 2.);
    assert!(r.work.represented_hints > 0);
    let f = fixtures(false).remove(5);
    let p = path(&f);
    let r = p.certify(&f.g, PhysicalRoute::SourceAction, TOL).unwrap();
    assert!(r.qualified);
    assert!(r.work.nilpotent_queries > 0);
}
#[test]
fn unresolved_grazing_and_initial_root_and_admission() {
    let f = fixtures(false).remove(0);
    let p = path(&f);
    let clock = AffineFactor {
        constant: 0.,
        time: 1.,
        state: vec![],
    };
    let g = DeclaredPolynomial::new(vec![
        EventTerm {
            coefficient: 1.,
            factors: vec![clock.clone(), clock.clone()],
        },
        EventTerm {
            coefficient: -1.,
            factors: vec![clock],
        },
        EventTerm {
            coefficient: 0.25,
            factors: vec![],
        },
    ])
    .unwrap();
    for c in [
        PhysicalRoute::Unity,
        PhysicalRoute::Weighted,
        PhysicalRoute::SourceAction,
    ] {
        let r = p.certify(&g, c, TOL).unwrap();
        assert!(!r.qualified);
        assert_eq!(r.status, PhysicalStatus::Unresolved);
        assert!(r.bracket.is_none());
        println!(
            "NEGATIVE {}",
            json!({"case":"nonfactored-touch","report":r})
        );
    }
    let r = p
        .certify(&poly(0, 1., 1), PhysicalRoute::Weighted, TOL)
        .unwrap();
    assert_eq!(r.status, PhysicalStatus::Unresolved);
    assert!(AffineSource::new(DenseMatrix::new(1, 1, vec![1.]).unwrap(), vec![0.]).is_err());
    assert!(
        p.certify(&poly(9, 0.5, 1), PhysicalRoute::Weighted, TOL)
            .is_err()
    );
    assert!(p.certify(&f.g, PhysicalRoute::Weighted, 0.).is_err());
}
#[test]
#[ignore = "explicit finite campaign with independent physical oracle"]
fn training() {
    for (i, f) in fixtures(false).iter().enumerate() {
        let p = path(f);
        fixture_record(f, &p);
        for j in 0..ROUTES.len() {
            let c = ROUTES[(i + j) % ROUTES.len()];
            println!("CELL {}", measured(f, &p, c));
        }
    }
}
#[test]
#[ignore = "frozen selection only"]
fn holdout() {
    let f = std::env::var("VIGILODE_PHYSICAL_SELECTION").unwrap();
    let d: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap();
    for f in fixtures(true) {
        let p = path(&f);
        fixture_record(&f, &p);
        let c = d["groups"][f.group]["selected"].as_str().unwrap();
        println!("CELL {}", measured(&f, &p, c));
    }
}

#[test]
#[ignore = "separately registered tight event-time campaign"]
fn precision_campaign() {
    for (i, f) in fixtures(false).iter().enumerate() {
        let p = path(f);
        fixture_record(f, &p);
        for tol in [1e-8, 1e-10] {
            for j in 0..ROUTES.len() {
                let c = ROUTES[(i + j) % ROUTES.len()];
                let mut r = measured_tol(f, &p, c, tol);
                r["time_tol"] = json!(tol);
                println!("CELL {r}");
            }
        }
    }
}
#[test]
#[ignore = "tight selection frozen before holdout"]
fn precision_holdout() {
    let file = std::env::var("VIGILODE_PHYSICAL_PRECISION_SELECTION").unwrap();
    let d: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    for f in fixtures(true) {
        let p = path(&f);
        fixture_record(&f, &p);
        for (key, tol) in [("1e-8", 1e-8), ("1e-10", 1e-10)] {
            if let Some(c) = d["groups"][f.group][key]["selected"].as_str() {
                let mut r = measured_tol(&f, &p, c, tol);
                r["time_tol"] = json!(tol);
                println!("CELL {r}");
            }
        }
    }
}
