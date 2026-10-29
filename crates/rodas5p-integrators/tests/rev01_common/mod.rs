//! Systems of review DAG node REV-01
//! (`research/rev01_gcrodr_start_projection_20261003`), taken from the
//! INT-01 study: the Brusselator trajectory and one-step sets for any cell
//! count, and INT-01's fresh convection-diffusion-reaction family.
#![allow(dead_code)]

#[path = "../rnext_common/mod.rs"]
pub mod common;

use common::{adaptive, linear_config, splitmix_vector, stage_operator, stage_right_hand_sides};
use rodas5p_core::{
    DenseMatrix, DenseOperator, LinearMethod, LinearOperator, WorkCounters, rodas5p_coefficients,
    safe_l2,
};
use rodas5p_integrators::{
    OdeProblem, OutputSchedule, Rodas5pMfFastWorkspace, integrate_rodas5p_mf_fast_observed_traced,
};
use rodas5p_krylov::{GcrodrConfig, GcrodrTrace};
use serde_json::{Value, json};

/// An operator, its right-hand sides and a label.
pub type Group = (Box<dyn LinearOperator>, Vec<Vec<f64>>, String);
/// `(t, y, h, stages)` of one recorded attempt.
pub type AttemptRecord = (f64, Vec<f64>, f64, Vec<Vec<f64>>);

/// One frozen system sequence: an operator per group of right-hand sides,
/// solved in order.
pub struct Sequence {
    pub set: &'static str,
    pub label: String,
    /// (operator, right-hand sides, group label)
    pub groups: Vec<Group>,
    pub rtol: f64,
    pub budget: usize,
}

pub fn trajectory_and_one_step(
    problem: &OdeProblem,
    y0: &[f64],
    cells: usize,
) -> (Vec<Sequence>, Value) {
    // Trajectory: the first 40 attempts of the GMRES U-form run.
    let linear = linear_config(LinearMethod::Gmres, 1.0e-11);
    let adapt = adaptive(1.0e-6, 1.0, 10.0);
    let schedule = OutputSchedule::new(vec![0.0, 10.0]).unwrap();
    let mut records: Vec<AttemptRecord> = Vec::new();
    let mut failed_attempts = 0usize;
    let run = integrate_rodas5p_mf_fast_observed_traced(
        problem,
        (0.0, 10.0),
        y0,
        &linear,
        &adapt,
        &schedule,
        false,
        &mut |t, y, h, outcome, work: &Rodas5pMfFastWorkspace| {
            if records.len() + failed_attempts >= 40 {
                return;
            }
            match outcome {
                Some(_) => records.push((
                    t,
                    y.to_vec(),
                    h,
                    (0..8).map(|i| work.stage(i).to_vec()).collect(),
                )),
                None => failed_attempts += 1,
            }
        },
    )
    .unwrap();
    let mut trajectory = Sequence {
        set: "trajectory",
        label: format!("brusselator-{cells}-gmres-trajectory"),
        groups: Vec::new(),
        rtol: 1.0e-11,
        budget: 200,
    };
    for (k, (t, y, h, stages)) in records.iter().enumerate() {
        let op = stage_operator(problem, *t, y, *h);
        let rhs = stages
            .iter()
            .map(|u| {
                let mut b = vec![0.0; u.len()];
                op.apply(u, &mut b).unwrap();
                b
            })
            .collect();
        trajectory
            .groups
            .push((Box::new(op), rhs, format!("attempt {k} t={t:e} h={h:e}")));
    }
    let mut sequences = vec![trajectory];
    let mut excluded = Vec::new();
    for h in [1.0e-4, 1.0e-2] {
        let mut work = Rodas5pMfFastWorkspace::new(problem, &linear).unwrap();
        if let Err(error) = work.attempt(
            problem,
            0.0,
            y0,
            h,
            true,
            None,
            1.0e-6,
            1.0e-6,
            &mut WorkCounters::default(),
        ) {
            excluded.push(json!({"h": h, "error": error.to_string()}));
            continue;
        }
        let op = stage_operator(problem, 0.0, y0, h);
        let rhs = stage_right_hand_sides(&op, &work);
        sequences.push(Sequence {
            set: "one_step",
            label: format!("brusselator-{cells}-y0-h={h:e}"),
            groups: vec![(Box::new(op), rhs, format!("h={h:e}"))],
            rtol: 1.0e-12,
            budget: 4000,
        });
    }
    let info = json!({
        "trajectory_attempts_recorded": records.len(),
        "trajectory_failed_attempts_seen": failed_attempts,
        "generating_run": {"success": run.observed.success, "attempts": run.attempts},
        "one_step_excluded": excluded,
    });
    (sequences, info)
}

/// `I - tau (D2 - Pe D1 - r I)` on n interior points of (0, 1), central
/// differences, homogeneous Dirichlet ends, r = 10.
pub fn convection_diffusion_reaction(n: usize, peclet: f64, tau: f64) -> DenseMatrix {
    let h = 1.0 / (n as f64 + 1.0);
    let (d, c, r) = (1.0 / (h * h), peclet / (2.0 * h), 10.0);
    let mut a = DenseMatrix::zeros(n, n);
    for i in 0..n {
        a[(i, i)] = 1.0 + tau * (2.0 * d + r);
        if i > 0 {
            a[(i, i - 1)] = -tau * (d + c);
        }
        if i + 1 < n {
            a[(i, i + 1)] = -tau * (d - c);
        }
    }
    a
}

/// The fresh family of the preregistration (not used by any earlier node).
pub fn fresh() -> Vec<Sequence> {
    let n = 120;
    let mut out = Vec::new();
    for peclet in [0.5, 5.0, 50.0] {
        for tau in [1.0e-3, 4.0e-3] {
            let groups = (0..6)
                .map(|k| {
                    let a = convection_diffusion_reaction(n, peclet, tau * (1.0 + 0.02 * k as f64));
                    let op = DenseOperator::new(a).unwrap();
                    let rhs = (0..8)
                        .map(|i| {
                            let noise = splitmix_vector(9001 + i as u64, n);
                            let u: Vec<f64> = (0..n)
                                .map(|j| {
                                    let x = (j + 1) as f64 / (n + 1) as f64;
                                    (std::f64::consts::PI * (i + 1) as f64 * x).sin()
                                        * (1.0 + 0.1 * i as f64)
                                        + 0.1 * noise[j]
                                })
                                .collect();
                            let mut b = vec![0.0; n];
                            op.apply(&u, &mut b).unwrap();
                            b
                        })
                        .collect();
                    let op: Box<dyn LinearOperator> = Box::new(op);
                    (op, rhs, format!("k={k}"))
                })
                .collect();
            out.push(Sequence {
                set: "fresh",
                label: format!("convection-diffusion-reaction-{n}-Pe{peclet}-tau{tau:e}"),
                groups,
                rtol: 1.0e-9,
                budget: 400,
            });
        }
    }
    out
}

pub fn gcrodr_config(rtol: f64, budget: usize) -> GcrodrConfig {
    let gamma = rodas5p_coefficients().unwrap().gamma;
    GcrodrConfig {
        restart: 40,
        max_arnoldi: budget,
        recycle_dim: 8,
        rank_tol: 1.0e-12,
        rtol,
        atol: 1.0e-14 * gamma.abs(),
    }
}

pub fn threshold(config: &GcrodrConfig, b: &[f64]) -> f64 {
    config.atol.max(config.rtol * safe_l2(b))
}

pub fn independent_residual(op: &dyn LinearOperator, b: &[f64], x: &[f64]) -> f64 {
    let mut ax = vec![0.0; b.len()];
    op.apply(x, &mut ax).unwrap();
    safe_l2(&b.iter().zip(&ax).map(|(p, q)| p - q).collect::<Vec<_>>())
}

pub fn trace_json(trace: &GcrodrTrace) -> Value {
    json!({
        "matvecs_total": trace.matvecs_total,
        "matvecs_outside_cycles": trace.matvecs_outside_cycles,
        "cycles": trace.cycles.iter().map(|c| json!({
            "rank": c.recycle_rank, "residual_start": c.residual_start,
            "retained_fraction": c.retained_fraction,
            "residual_after_projection": c.residual_after_projection,
            "arnoldi_columns": c.arnoldi_columns, "ls_residual": c.least_squares_residual,
            "residual_end": c.residual_end, "CtC_defect": c.recycle_gram_defect,
            "CtV": c.recycle_arnoldi_coupling, "VtV_defect": c.arnoldi_gram_defect,
            "matvecs": c.matvecs, "reset": c.reset, "aborted": c.aborted,
        })).collect::<Vec<_>>(),
    })
}
