//! Research node `research/pp15_fourier_comparator_20261004` (RVJ DAG node
//! PP15): the certified native Fourier client (direct and FFT predictors)
//! and RODAS5P on the same real 4-dimensional system, exported for
//! `tools/pp15_comparator_check.py`. No timing.

use std::sync::Arc;

use rodas5p_core::DenseMatrix;
use rodas5p_integrators::fourier_path_candidate::{FftWork, fft_rhs_h};
use rodas5p_integrators::fourier_path_certificate::{FourierModel, FourierPath, run_driver};
use rodas5p_integrators::{
    AdaptiveStepConfig, OdeProblem, OutputSchedule, integrate_rodas5p_fast_observed,
};
use serde_json::{Value, json};

fn hx(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}

const G: f64 = 0.125;
const EPS: f64 = 0.125;
const Q: f64 = 1.25;

/// `y = (Re a, Im a, Re b, Im b)` in the client's rotating variables, `q`
/// constant 5/4 (the leaf): `a' = i q c conj(a) b`, `b' = i (q/2) c a^2`,
/// `c = g + eps (cos(omega t) a1 - sin(omega t) a2)`.
fn c_of(omega: f64, t: f64, y: &[f64]) -> f64 {
    G + EPS * ((omega * t).cos() * y[0] - (omega * t).sin() * y[1])
}

fn rhs_vec(omega: f64, t: f64, y: &[f64], out: &mut [f64]) {
    let c = c_of(omega, t, y);
    let (p, q) = (y[0] * y[2] + y[1] * y[3], y[0] * y[3] - y[1] * y[2]);
    let (r, s) = (y[0] * y[0] - y[1] * y[1], 2.0 * y[0] * y[1]);
    out[0] = -Q * c * q;
    out[1] = Q * c * p;
    out[2] = -0.5 * Q * c * s;
    out[3] = 0.5 * Q * c * r;
}

fn jacobian_matrix(omega: f64, t: f64, y: &[f64]) -> Vec<f64> {
    let c = c_of(omega, t, y);
    let (cs, sn) = ((omega * t).cos(), (omega * t).sin());
    let dc = [EPS * cs, -EPS * sn, 0.0, 0.0];
    let (p, q) = (y[0] * y[2] + y[1] * y[3], y[0] * y[3] - y[1] * y[2]);
    let (r, s) = (y[0] * y[0] - y[1] * y[1], 2.0 * y[0] * y[1]);
    let dq = [y[3], -y[2], -y[1], y[0]];
    let dp = [y[2], y[3], y[0], y[1]];
    let ds = [2.0 * y[1], 2.0 * y[0], 0.0, 0.0];
    let dr = [2.0 * y[0], -2.0 * y[1], 0.0, 0.0];
    let mut m = vec![0.0; 16];
    for k in 0..4 {
        m[k] = -Q * (dc[k] * q + c * dq[k]);
        m[4 + k] = Q * (dc[k] * p + c * dp[k]);
        m[8 + k] = -0.5 * Q * (dc[k] * s + c * ds[k]);
        m[12 + k] = 0.5 * Q * (dc[k] * r + c * dr[k]);
    }
    m
}

/// `y = (Re a, Im a, Re b, Im b)` in the client's rotating variables, `q`
/// constant 5/4 (the leaf): `a' = i q c conj(a) b`, `b' = i (q/2) c a^2`,
/// `c = g + eps (cos(omega t) a1 - sin(omega t) a2)`.
fn problem(omega: f64) -> OdeProblem {
    let rhs = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        rhs_vec(omega, t, y, out);
        Ok(())
    });
    let jac =
        Arc::new(move |t: f64, y: &[f64]| DenseMatrix::new(4, 4, jacobian_matrix(omega, t, y)));
    let ft = Arc::new(move |t: f64, y: &[f64], out: &mut [f64]| {
        let ct = EPS * omega * (-(omega * t).sin() * y[0] - (omega * t).cos() * y[1]);
        let (p, q) = (y[0] * y[2] + y[1] * y[3], y[0] * y[3] - y[1] * y[2]);
        let (r, s) = (y[0] * y[0] - y[1] * y[1], 2.0 * y[0] * y[1]);
        out[0] = -Q * ct * q;
        out[1] = Q * ct * p;
        out[2] = -0.5 * Q * ct * s;
        out[3] = 0.5 * Q * ct * r;
        Ok(())
    });
    OdeProblem::new(
        "fourier-volterra-leaf",
        4,
        rhs,
        None,
        Some(jac),
        None,
        Some(ft),
        false,
        None,
        None,
    )
    .unwrap()
}

fn rodas_arm(omega: f64, tol: f64) -> Value {
    let p = problem(omega);
    let adapt = AdaptiveStepConfig {
        atol: tol,
        rtol: tol,
        initial_step: 1.0e-6,
        min_step: 1.0e-14,
        max_step: 0.5,
        max_attempts: 400_000,
        ..AdaptiveStepConfig::default()
    };
    let out = OutputSchedule::new(vec![0.0, 0.5]).unwrap();
    match integrate_rodas5p_fast_observed(&p, (0.0, 0.5), &[0.25, 0.0, 0.5, 0.0], &adapt, &out) {
        Ok(r) => json!({
            "ok": true, "success": r.observed.success, "message": r.observed.message,
            "final": r.observed.y.last().unwrap().iter().map(|v| hx(*v)).collect::<Vec<_>>(),
            "t_final": hx(*r.observed.t.last().unwrap()),
            "attempts": r.attempts, "accepted": r.accepted_steps, "rejected": r.rejected_steps,
            "counters": serde_json::to_value(r.observed.counters).unwrap(),
            "certified": false,
        }),
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

fn client_arm(omega: f64, fft: bool) -> Value {
    let model = FourierModel::new(omega, 1).unwrap();
    let mut work = FftWork::default();
    let mut rhs = |path: &[FourierPath; 2], m: &FourierModel, phase: &FourierPath, h: f64| {
        fft_rhs_h(path, m, phase, h, None, false, &mut work)
    };
    let result = if fft {
        run_driver(&model, 0.5, 1.0e-8, 0.125, Some(&mut rhs))
    } else {
        run_driver(&model, 0.5, 1.0e-8, 0.125, None)
    };
    match result {
        Ok((records, counts)) => {
            let last = records.last().unwrap();
            let a = last.state.amplitudes();
            json!({
                "ok": true, "certified": true,
                "final": [hx(a[0].re), hx(a[0].im), hx(a[1].re), hx(a[1].im)],
                "t_final": hx(last.state.t()),
                "error_bound": hx(last.state.error()),
                "steps": records.len(),
                "candidate_builds": counts.candidate_builds,
                "candidate_rejections": counts.candidate_rejections,
                "predictor_calls": counts.rhs_calls,
                "certificate_calls": counts.candidate_builds,
                "step_halvings": counts.step_halvings,
            })
        }
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

#[test]
#[ignore = "recorded run of research/pp15_fourier_comparator_20261004; release build"]
fn export_comparator() {
    let mut rows = Vec::new();
    for omega in [1.0, 40.0, 1.0e4] {
        rows.push(json!({
            "omega": hx(omega),
            "client_direct": client_arm(omega, false),
            "client_fft": client_arm(omega, true),
            "rodas_1e-8": rodas_arm(omega, 1.0e-8),
            "rodas_1e-10": rodas_arm(omega, 1.0e-10),
        }));
    }
    let out = json!({"schema": "vigilode-pp15-comparator-v1", "rows": rows});
    if let Ok(path) = std::env::var("PP15_CASES") {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path);
        assert!(
            !path.exists(),
            "immutable output exists: {}",
            path.display()
        );
        std::fs::write(&path, serde_json::to_string_pretty(&out).unwrap() + "\n").unwrap();
        println!("wrote {}", path.display());
    }
}

#[test]
fn jacobian_matches_finite_differences() {
    let y = [0.25, 0.1, 0.5, -0.2];
    let t = 0.3;
    let j = jacobian_matrix(40.0, t, &y);
    let mut f0 = [0.0; 4];
    rhs_vec(40.0, t, &y, &mut f0);
    for k in 0..4 {
        let mut yk = y;
        yk[k] += 1.0e-7;
        let mut f1 = [0.0; 4];
        rhs_vec(40.0, t, &yk, &mut f1);
        for i in 0..4 {
            let fd: f64 = (f1[i] - f0[i]) / 1.0e-7;
            assert!((fd - j[4 * i + k]).abs() < 1e-6, "{i} {k}");
        }
    }
}
