#![allow(dead_code)]

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use vigilode::{
    Atol, CallbackError, DerivativeFallback, Jacobian, Options, Problem, SwitchingConfig,
    SwitchingSolution, SwitchingSolver,
};

pub const CASES: [&str; 4] = [
    "easy-cubic",
    "stiff-cubic",
    "relaxing-nonlinear",
    "reentrant-nonlinear",
];
pub const POLICIES: [&str; 9] = [
    "rhs-only",
    "pr91",
    "strict-ordinary",
    "ratio-sentinel",
    "strict-sentinel",
    "observed-pr91",
    "observed-strict-ordinary",
    "observed-ratio-sentinel",
    "observed-strict-sentinel",
];
pub const DIMENSION: usize = 16;

fn log_cosh(x: f64) -> f64 {
    let a = x.abs();
    a + (-2.0 * a).exp().ln_1p() - std::f64::consts::LN_2
}

fn transition(t: f64, center: f64) -> (f64, f64, f64) {
    let q = (200.0 * (t - center)).tanh();
    (q, 200.0 * (1.0 - q * q), -80000.0 * q * (1.0 - q * q))
}

pub fn rate(case: &str, t: f64) -> (f64, f64, f64) {
    match case {
        "easy-cubic" => (1.0, 0.0, t),
        "stiff-cubic" => (1000.0, 0.0, 1000.0 * t),
        "relaxing-nonlinear" => {
            let (q, qp, _) = transition(t, 0.5);
            (
                1.0 + 4999.5 * (1.0 - q),
                -4999.5 * qp,
                5000.5 * t - (4999.5 / 200.0) * (log_cosh(200.0 * (t - 0.5)) - log_cosh(-100.0)),
            )
        }
        "reentrant-nonlinear" => {
            let (q1, q1p, _) = transition(t, 0.25);
            let (q2, q2p, _) = transition(t, 0.75);
            (
                1.0 + 4999.5 * (2.0 - q1 + q2),
                4999.5 * (-q1p + q2p),
                10000.0 * t
                    + (4999.5 / 200.0)
                        * (-log_cosh(200.0 * (t - 0.25))
                            + log_cosh(-50.0)
                            + log_cosh(200.0 * (t - 0.75))
                            - log_cosh(-150.0)),
            )
        }
        _ => panic!("unknown case"),
    }
}

pub fn reference_carrier(case: &str, t: f64) -> (f64, f64, f64) {
    if case.ends_with("cubic") {
        return (0.0, 0.0, 0.0);
    }
    let (sn, cs) = (150.0 * t).sin_cos();
    let (s, sp, spp) = if case == "relaxing-nonlinear" {
        let (q, qp, qpp) = transition(t, 0.5);
        (0.5 * (1.0 + q), 0.5 * qp, 0.5 * qpp)
    } else {
        (1.0, 0.0, 0.0)
    };
    (
        t.cos() + 0.01 * s * sn,
        -t.sin() + 0.01 * (sp * sn + 150.0 * s * cs),
        -t.cos() + 0.01 * (spp * sn + 300.0 * sp * cs - 22500.0 * s * sn),
    )
}

pub fn initial(case: &str) -> Vec<f64> {
    let mut y = vec![0.0; DIMENSION];
    for (j, z) in y.iter_mut().enumerate().skip(1) {
        *z = reference_carrier(case, 0.0).0 + 0.25 + 0.01 * j as f64;
    }
    y
}

pub fn exact(case: &str, t: f64, component: usize) -> f64 {
    if component == 0 {
        return t;
    }
    let e0 = 0.25 + 0.01 * component as f64;
    let a = (-rate(case, t).2).exp();
    reference_carrier(case, t).0 + e0 * a / (1.0 + 0.5 * e0 * e0 * (1.0 - a * a)).sqrt()
}

pub fn options() -> Options {
    Options {
        rtol: 1e-7,
        atol: Atol::Scalar(1e-10),
        h0: 1e-4,
        max_steps: 100_000,
        ..Default::default()
    }
}

pub fn config(policy: &str) -> SwitchingConfig {
    if policy == "rhs-only" {
        return SwitchingConfig::default();
    }
    assert!(POLICIES.contains(&policy));
    SwitchingConfig {
        stop_dominated_return_comparator: true,
        seed_entry_pilot_from_explicit_checkpoint: true,
        strict_entry_work_improvement: true,
        strict_return_work_improvement: policy.contains("strict"),
        reuse_observed_entry_step: policy.starts_with("observed"),
        return_sentinel_interval: usize::from(policy.ends_with("sentinel")),
        ..SwitchingConfig::adaptive_evidence(DerivativeFallback::ProvidedOnly)
    }
}

pub fn solve(
    case: &str,
    config: &SwitchingConfig,
    fail_rhs: Option<u64>,
) -> (SwitchingSolution, u64, u64) {
    let rhs = Arc::new(AtomicU64::new(0));
    let jvp = Arc::new(AtomicU64::new(0));
    let rc = rhs.clone();
    let jc = jvp.clone();
    let rhs_case = case.to_string();
    let jvp_case = case.to_string();
    let p = Problem::new(DIMENSION, move |_, y, out| {
        let call = rc.fetch_add(1, Ordering::Relaxed) + 1;
        if fail_rhs == Some(call) {
            return Err(CallbackError::Fatal("strict return injected fatal".into()));
        }
        let (l, _, _) = rate(&rhs_case, y[0]);
        let (g, gp, _) = reference_carrier(&rhs_case, y[0]);
        out[0] = 1.0;
        for j in 1..y.len() {
            let e = y[j] - g;
            out[j] = -l * (e + 0.5 * e * e * e) + gp;
        }
        Ok(())
    })
    .jacobian(Jacobian::jvp(move |_, y, v, out| {
        jc.fetch_add(1, Ordering::Relaxed);
        let (l, lp, _) = rate(&jvp_case, y[0]);
        let (g, gp, gpp) = reference_carrier(&jvp_case, y[0]);
        out[0] = 0.0;
        for j in 1..y.len() {
            let e = y[j] - g;
            let d = 1.0 + 1.5 * e * e;
            out[j] = (-lp * (e + 0.5 * e * e * e) + l * d * gp + gpp) * v[0] - l * d * v[j];
        }
        Ok(())
    }))
    .autonomous();
    let r = SwitchingSolver::solve(&p, (0.0, 1.0), &initial(case), &options(), config).unwrap();
    (r, rhs.load(Ordering::Relaxed), jvp.load(Ordering::Relaxed))
}

pub fn max_error(case: &str, r: &SwitchingSolution) -> f64 {
    r.t.iter()
        .zip(&r.y)
        .flat_map(|(t, y)| {
            y.iter()
                .enumerate()
                .map(move |(j, v)| (v - exact(case, *t, j)).abs())
        })
        .fold(0.0, f64::max)
}

pub fn accuracy_cap(case: &str) -> f64 {
    if case.ends_with("cubic") { 1e-6 } else { 1e-5 }
}

pub fn work(r: &SwitchingSolution) -> u128 {
    r.stats
        .segments
        .iter()
        .map(|s| {
            s.rhs_calls as u128
                + s.jvp_vectors as u128
                + s.ft_calls as u128
                + DIMENSION as u128 * s.jacobian_builds as u128
                + s.linear_solves as u128
        })
        .sum()
}
