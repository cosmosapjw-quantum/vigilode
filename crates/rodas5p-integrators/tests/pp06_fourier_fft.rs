//! Research node `research/pp06_fourier_fft_candidate_20261004` (RVJ DAG
//! node PP06): the PP05 driver with the native FFT predictor, exported for
//! `tools/pp06_fft_check.py`.

use rodas5p_integrators::fourier_path_candidate::{
    FftWork, fft_rhs_h, relative_difference, required_shape,
};
use rodas5p_integrators::fourier_path_certificate::{
    Cplx, FourierModel, FourierPath, FourierState, FourierTrial, predictor_rhs_h, run_driver,
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
        "accepted": c.accepted(), "error": hx(c.error()), "growth": hx(c.growth()),
        "residual": hx(c.residual()), "start_mismatch": hx(c.start_mismatch()),
        "endpoint": c.endpoint().iter().map(|z| cx(*z)).collect::<Vec<_>>(),
    })
}

/// One driver run with the FFT predictor; every call also computes the
/// direct product for G1. `alias_nk` forces an under-padded harmonic axis.
fn run(omega: f64, tol: f64, alias_nk: Option<usize>) -> Value {
    let model = FourierModel::new(omega, 1).unwrap();
    let mut work = FftWork::default();
    let mut worst_rel = 0.0_f64;
    let mut calls = 0usize;
    let mut refused_without_override = 0usize;
    let mut rhs = |path: &[FourierPath; 2], m: &FourierModel, phase: &FourierPath, h: f64| {
        let direct = predictor_rhs_h(path, m, phase, h, 1.0);
        let shape = alias_nk.map(|nk| (nk, required_shape(path, phase).3));
        if shape.is_some() {
            let mut scratch = FftWork::default();
            if fft_rhs_h(path, m, phase, h, shape, false, &mut scratch).is_err() {
                refused_without_override += 1;
            }
        }
        let fft = fft_rhs_h(path, m, phase, h, shape, alias_nk.is_some(), &mut work)?;
        calls += 1;
        for j in 0..2 {
            worst_rel = worst_rel.max(relative_difference(&fft[j], &direct[j]));
        }
        Ok(fft)
    };
    let result = run_driver(&model, 0.5, tol, 0.125, Some(&mut rhs));
    let records = match &result {
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
    json!({
        "omega": hx(omega), "sigma": 1, "tol": hx(tol), "alias_nk": alias_nk,
        "run": records,
        "fft_work": {"rhs_calls": work.rhs_calls, "forward_2d": work.forward_2d,
                     "inverse_2d": work.inverse_2d, "padded_grid_points": work.padded_grid_points,
                     "max_grid_points": work.max_grid_points},
        "predictor_calls": calls,
        "worst_relative_difference_fft_vs_direct": worst_rel,
        "refused_without_override": refused_without_override,
    })
}

#[test]
#[ignore = "recorded run of research/pp06_fourier_fft_candidate_20261004; release build"]
fn export_fft_runs() {
    let out = json!({
        "schema": "vigilode-pp06-fourier-fft-v1",
        "runs": {
            "forty": run(40.0, 1.0e-8, None),
            "fast": run(1.0e4, 1.0e-8, None),
            "forty_alias": run(40.0, 1.0e-8, Some(8)),
        },
    });
    if let Ok(path) = std::env::var("PP06_CASES") {
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
