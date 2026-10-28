//! Research node `research/pp05_fourier_client_20261004` (RVJ DAG node
//! PP05): the native Fourier-Volterra client's driver runs and negative
//! controls, exported as IEEE bits for `tools/pp05_fourier_check.py`.

use rodas5p_integrators::fourier_path_certificate::{
    Cplx, FourierModel, FourierPath, FourierState, FourierTrial, PredictorRhs, build_trial,
    certificate, commit, run_driver,
};
use serde_json::{Value, json};

fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

fn cx(z: Cplx) -> Value {
    json!([hx(z.re), hx(z.im)])
}

fn path_json(p: &FourierPath) -> Value {
    json!(
        p.coefficients()
            .iter()
            .map(|((k, j), v)| json!([k, j, hx(v.re), hx(v.im)]))
            .collect::<Vec<_>>()
    )
}

fn state_json(s: &FourierState) -> Value {
    json!({"t": hx(s.t()), "a": s.amplitudes().iter().map(|z| cx(*z)).collect::<Vec<_>>(),
           "error": hx(s.error()), "generation": s.generation()})
}

fn trial_json(t: &FourierTrial) -> Value {
    let c = &t.certificate;
    json!({
        "h": hx(t.h), "K": t.k_max, "p": t.p_max, "sweeps": t.sweeps,
        "path": [path_json(&t.path[0]), path_json(&t.path[1])],
        "accepted": c.accepted(), "reason": c.reason(),
        "error": hx(c.error()), "growth": hx(c.growth()), "residual": hx(c.residual()),
        "differential_defect": hx(c.differential_defect()), "phase_defect": hx(c.phase_defect()),
        "start_mismatch": hx(c.start_mismatch()), "sup_bound": hx(c.sup_bound()),
        "endpoint": c.endpoint().iter().map(|z| cx(*z)).collect::<Vec<_>>(),
        "endpoint_rounding": hx(c.endpoint_rounding()),
        "closure_defect": hx(c.closure_defect()), "phase_mode": c.phase_mode(),
    })
}

pub const CASES: [(&str, f64, i32, f64); 6] = [
    ("resonance", 0.0, 1, 1.0e-8),
    ("unit", 1.0, 1, 1.0e-8),
    ("forty", 40.0, 1, 1.0e-8),
    ("negative", -40.0, -1, 1.0e-8),
    ("fast", 1.0e4, 1, 1.0e-8),
    ("forty_tight", 40.0, 1, 1.0e-11),
];

fn negative_controls() -> Value {
    let model = FourierModel::new(40.0, 1).unwrap();
    let state = FourierState::initial();
    let h = 0.125;
    let tol = 1.0e-8;
    let mut out = serde_json::Map::new();
    for (name, kind) in [
        ("independent_scalar", PredictorRhs::IndependentScalar),
        ("wrong_phase", PredictorRhs::WrongPhase),
        ("cyclic_alias", PredictorRhs::CyclicAlias),
        ("coupled_reference", PredictorRhs::Coupled),
    ] {
        let trial = build_trial(&state, h, &model, 3, 6, 4, kind, None).unwrap();
        let committed = commit(&trial, &state, &model, tol / 4.0).is_ok();
        out.insert(
            name.into(),
            json!({"trial": trial_json(&trial), "commit_within_tol_over_4": committed}),
        );
    }
    let good = build_trial(&state, h, &model, 3, 6, 4, PredictorRhs::Coupled, None).unwrap();
    // Stale bindings.
    let mut other_model = model;
    other_model.epoch = 1;
    let mut wrong_h = good.clone();
    wrong_h.h = 0.0625;
    let mut wrong_path = good.clone();
    wrong_path.path[0] = wrong_path.path[1].clone();
    let moved = commit(&good, &state, &model, 1.0).unwrap();
    let stale = json!({
        "other_state_refused": commit(&good, &moved, &model, 1.0).is_err(),
        "other_epoch_refused": commit(&good, &state, &other_model, 1.0).is_err(),
        "other_h_refused": commit(&wrong_h, &state, &model, 1.0).is_err(),
        "other_path_refused": commit(&wrong_path, &state, &model, 1.0).is_err(),
    });
    out.insert("stale_binding".into(), stale);
    // Initial-condition mismatch: shift the start coefficient by 1e-3.
    let shifted: Vec<((i32, u32), Cplx)> = good.path[0]
        .coefficients()
        .iter()
        .map(|(k, v)| {
            if *k == (0, 0) {
                (*k, Cplx::new(v.re + 1.0e-3, v.im))
            } else {
                (*k, *v)
            }
        })
        .collect();
    let mismatch_path = [
        FourierPath::from_coefficients(shifted),
        good.path[1].clone(),
    ];
    let c = certificate(&state, h, &model, &mismatch_path).unwrap();
    out.insert(
        "initial_mismatch".into(),
        json!({"error": hx(c.error()), "start_mismatch": hx(c.start_mismatch()),
               "within_tol_over_4": c.error() <= tol / 4.0,
               "path": [path_json(&mismatch_path[0]), path_json(&mismatch_path[1])]}),
    );
    // h growth >= 1.
    out.insert(
        "noncontractive_slab_refused".into(),
        json!(certificate(&state, 2.0, &model, &good.path).is_err()),
    );
    // B > 1.
    let scaled = |p: &FourierPath| {
        FourierPath::from_coefficients(
            p.coefficients()
                .iter()
                .map(|(k, v)| (*k, Cplx::new(10.0 * v.re, 10.0 * v.im))),
        )
    };
    let big = [scaled(&good.path[0]), scaled(&good.path[1])];
    let c = certificate(&state, h, &model, &big).unwrap();
    out.insert(
        "domain_b_above_one".into(),
        json!({"accepted": c.accepted(), "reason": c.reason()}),
    );
    Value::Object(out)
}

#[test]
fn driver_unit_case_commits_and_binds() {
    let model = FourierModel::new(1.0, 1).unwrap();
    let (records, counts) = run_driver(&model, 0.5, 1.0e-8, 0.125, None).unwrap();
    assert!(!records.is_empty());
    assert!(counts.candidate_builds >= records.len());
    for r in &records {
        assert!(r.trial.certificate.accepted());
        assert_eq!(r.state.generation(), r.start.generation() + 1);
    }
}

#[test]
#[ignore = "recorded run of research/pp05_fourier_client_20261004; release build"]
fn export_fourier_client() {
    let mut cases = Vec::new();
    for (name, omega, sigma, tol) in CASES {
        let model = FourierModel::new(omega, sigma).unwrap();
        let value = match run_driver(&model, 0.5, tol, 0.125, None) {
            Ok((records, counts)) => json!({
                "ok": true,
                "counts": {"candidate_builds": counts.candidate_builds,
                           "candidate_rejections": counts.candidate_rejections,
                           "rhs_calls": counts.rhs_calls, "step_halvings": counts.step_halvings},
                "records": records.iter().map(|r| json!({
                    "start": state_json(&r.start), "state": state_json(&r.state),
                    "trial": trial_json(&r.trial), "physical_error": hx(r.physical_error),
                })).collect::<Vec<_>>(),
            }),
            Err(e) => json!({"ok": false, "error": e.to_string()}),
        };
        cases.push(
            json!({"case": name, "omega": hx(omega), "sigma": sigma, "tol": hx(tol),
                          "g": hx(0.125), "eps": hx(0.125), "T": hx(0.5), "run": value}),
        );
    }
    let out = json!({"schema": "vigilode-pp05-fourier-client-v1", "cases": cases,
                     "negative_controls": negative_controls()});
    if let Ok(path) = std::env::var("PP05_CASES") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

/// Review follow-up: a commit whose `t + h` rounds is refused (the next
/// phase witness would use an uncharged time error).
#[test]
fn commit_refuses_inexact_time_sum() {
    let model = FourierModel::new(1.0, 1).unwrap();
    let state = FourierState::initial();
    let first = build_trial(&state, 0.1, &model, 2, 4, 2, PredictorRhs::Coupled, None).unwrap();
    let next = commit(&first, &state, &model, 1.0).unwrap();
    let second = build_trial(&next, 0.2, &model, 2, 4, 2, PredictorRhs::Coupled, None).unwrap();
    let err = commit(&second, &next, &model, 1.0).unwrap_err();
    assert!(err.to_string().contains("not exact"));
}
