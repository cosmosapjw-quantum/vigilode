//! Net-cost admission of causal homotopy candidates (research node
//! `research/rnext06_homotopy_net_cost_20261003`, remaining-only DAG node
//! R-NEXT-06): serial vs action-first certificates on the actual q=2
//! candidates of certified transactional runs, with every cost charged, and
//! the per-regime abstention rule. Counter-only; no timing.

#[path = "thread_transfer_common/mod.rs"]
mod common;

use common::write_output;
use rodas5p_core::{
    LinearMethod, LinearSolverConfig, PreconditionerKind, WorkCounters, rodas5p_coefficients,
};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, DiagonalMajorant, InverseWitness,
    ParallelExecution, PathEvaluation, Q2Admission, Q2CertificateSource, QuadraticModel,
    StageTarget, TransactionalQ1Q2Config, TransactionalQ1Q2Lane,
    blocked_box_certificate_with_execution, certify_stage_target, residual_seeded_common_radius,
    rodas_next_step_after_attempt, sequential_matrix_free_step,
    transactional_q1_q2_step_with_admission,
};
use serde_json::json;

fn model(n: usize) -> (QuadraticModel, Vec<f64>) {
    let a = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { -1.0 - i as f64 } else { 0.0 })
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let q = (0..n)
        .map(|i| -0.05 * (1 + i % 3) as f64)
        .collect::<Vec<_>>();
    let y = (0..n).map(|i| 1.0 + 0.1 * i as f64).collect::<Vec<_>>();
    (
        QuadraticModel::new(format!("diagonal-quadratic-{n}"), a, q).unwrap(),
        y,
    )
}

fn weighted(weights: &[f64], rows: &[Vec<f64>], n: usize) -> Vec<f64> {
    let mut out = vec![0.0; n];
    for (&w, row) in weights.iter().zip(rows) {
        for (o, r) in out.iter_mut().zip(row) {
            *o += w * r;
        }
    }
    out
}

/// Operations of the sequential step's counted vector work (prereg unit).
fn solve_operations(c: &WorkCounters, n: usize, nnz: usize) -> u64 {
    let n = n as u64;
    c.linear_matvecs.saturating_add(c.diagnostic_matvecs) * 2 * nnz as u64
        + (c.orthogonalization_inner_products + c.orthogonalization_vector_updates) * 2 * n
        + c.preconditioner_apps * n
}

#[test]
#[ignore = "research run of research/rnext06_homotopy_net_cost_20261003; release build"]
fn homotopy_net_cost() {
    let coeffs = rodas5p_coefficients().unwrap();
    let target = StageTarget::sequential(coeffs).unwrap();
    let execution = ParallelExecution::sequential();
    let config = TransactionalQ1Q2Config::default();
    let gmres = LinearSolverConfig {
        method: LinearMethod::Gmres,
        restart: config.gmres_restart,
        maxiter: config.gmres_max_arnoldi,
        rtol: config.gmres_rtol,
        atol: config.gmres_atol,
        preconditioner: PreconditionerKind::None,
        ..LinearSolverConfig::default()
    };
    let (atol, rtol) = (1.0e-6, 1.0e-6);
    let adaptive = AdaptiveStepConfig {
        atol,
        rtol,
        initial_step: 0.05,
        min_step: 1.0e-10,
        max_step: 0.5,
        ..AdaptiveStepConfig::default()
    };
    let mut gate_same_target = true;
    let mut gate_charged = true;
    let mut regimes = Vec::new();
    for n in [1usize, 2, 4, 8, 16] {
        let (m, y0) = model(n);
        let problem = m.ode_problem().unwrap();
        let (mut t, tf) = (0.0, 0.5);
        let mut y = y0.clone();
        let mut h = adaptive.initial_step;
        let mut controller = AdaptiveControllerState::default();
        let mut attempts = Vec::new();
        let (mut lanes_q1, mut lanes_q2, mut lanes_fb) = (0usize, 0usize, 0usize);
        let (mut margin_serial, mut margin_action) = (0.0_f64, 0.0_f64);
        while t < tf && attempts.len() < 200 {
            let trial = h.min(tf - t);
            let mut counters = WorkCounters::default();
            let report = transactional_q1_q2_step_with_admission(
                &problem,
                t,
                &y,
                trial,
                &config,
                atol,
                rtol,
                false,
                Q2Admission::NativeTargetCertificate(&m),
                &mut counters,
            )
            .unwrap();
            // Solve-cost unit: the sequential step on the same (t, y, h).
            let mut seq_counters = WorkCounters::default();
            let _ = sequential_matrix_free_step(
                &problem,
                t,
                &y,
                trial,
                &gmres,
                None,
                atol,
                rtol,
                true,
                &mut seq_counters,
            )
            .unwrap();
            let c_solve = (solve_operations(&seq_counters, n, n) as f64 / 8.0).max(1.0);
            match report.lane {
                TransactionalQ1Q2Lane::Q1Fast => lanes_q1 += 1,
                TransactionalQ1Q2Lane::Q2Escalated => lanes_q2 += 1,
                TransactionalQ1Q2Lane::SequentialFallback => lanes_fb += 1,
            }
            let depth = report.critical_path_depth as f64;
            let serial_ops = report.work.certificate_operations as f64;
            let mut record = json!({
                "t": t, "h": trial, "lane": format!("{:?}", report.lane), "accepted": report.step.accepted,
                "critical_path_depth": report.critical_path_depth, "c_solve_operations": c_solve,
                "serial_certificate_operations": report.work.certificate_operations,
            });
            let mut action_ops_charged = serial_ops; // when no candidate, both arms charge the same
            if let (Some(stages), Some(admission)) =
                (&report.q2_candidate_stages, &report.q2_certificate)
            {
                let sp = m.stage_problem(t, &y, trial).unwrap();
                let witness = InverseWitness::diagonal(&sp, target.gamma).unwrap();
                let update = weighted(&coeffs.b, stages, n);
                let y_hat: Vec<f64> = y.iter().zip(update).map(|(a, b)| a + b).collect();
                let e_hat = weighted(&coeffs.btilde, stages, n);
                let serial = certify_stage_target(
                    &target, &sp, stages, &y_hat, &e_hat, &witness, atol, rtol,
                );
                let same = match (&serial, &admission.certificate) {
                    (Ok(s), Some(c)) => {
                        s.directed_operations == c.directed_operations
                            && s.output_wrms_upper.to_bits() == c.output_wrms_upper.to_bits()
                            && s.stage_bound == c.stage_bound
                    }
                    (Err(_), None) => true,
                    _ => false,
                };
                gate_same_target &= same;
                let entries = DiagonalMajorant::new(&target, &sp, stages, &witness).unwrap();
                let proposal = residual_seeded_common_radius(
                    &entries,
                    2.0,
                    PathEvaluation::ActionFirst,
                    &execution,
                )
                .unwrap();
                let evaluation_ops: u64 = proposal
                    .evaluations
                    .iter()
                    .map(|e| e.work.directed_operations)
                    .sum();
                let witness_ops = witness.work().directed_operations;
                let mut action = json!({"closes": proposal.closes, "evaluations": proposal.evaluations.len(),
                                        "reason": proposal.reason});
                let action_ops = evaluation_ops + witness_ops;
                if let Some(radii) = &proposal.radii {
                    let boxed = blocked_box_certificate_with_execution(
                        &target,
                        &sp,
                        stages,
                        &y_hat,
                        &e_hat,
                        &witness,
                        atol,
                        rtol,
                        radii,
                        PathEvaluation::ActionFirst,
                        &execution,
                    )
                    .unwrap();
                    // The box certificate re-evaluates the proposal's final box;
                    // charged once (the identical evaluation) plus the witness.
                    let identical = proposal
                        .evaluations
                        .last()
                        .map(|e| e.work.directed_operations)
                        == Some(boxed.evaluation.work.directed_operations);
                    gate_charged &= identical;
                    if let Some(c) = &boxed.certificate {
                        gate_charged &= c.directed_operations
                            == boxed.evaluation.work.directed_operations + witness_ops;
                        action["output_wrms_upper"] = json!(c.output_wrms_upper);
                        action["accepts"] = json!(c.output_wrms_upper <= admission.budget_lower);
                        if let Ok(s) = &serial {
                            action["output_over_serial"] = json!(
                                c.output_wrms_upper / s.output_wrms_upper.max(f64::MIN_POSITIVE)
                            );
                        }
                    }
                }
                action["operations"] = json!(action_ops);
                action_ops_charged = action_ops as f64;
                record["serial"] = json!({
                    "closes": serial.is_ok(), "accepts": admission.accepted,
                    "operations": serial.as_ref().map(|s| s.directed_operations).ok(),
                    "output_wrms_upper": serial.as_ref().map(|s| s.output_wrms_upper).ok(),
                    "budget_lower": admission.budget_lower,
                });
                record["action_first"] = action;
            }
            // Net margins in solve units (P = 8 ideal workers, threads = 1: no
            // dispatch charge).
            let m_serial = 8.0 - depth - serial_ops / c_solve;
            let m_action = 8.0 - depth - action_ops_charged / c_solve;
            margin_serial += m_serial;
            margin_action += m_action;
            record["margin_serial"] = json!(m_serial);
            record["margin_action_first"] = json!(m_action);
            attempts.push(record);
            let ok = report.step.accepted;
            if ok {
                t = if trial == tf - t { tf } else { t + trial };
                y = report.step.y_new.clone();
            }
            h = rodas_next_step_after_attempt(
                &mut controller,
                &adaptive,
                h,
                trial,
                report.step.error_norm,
                ok,
                false,
            )
            .unwrap();
        }
        let count = attempts.len() as f64;
        let best = margin_serial.max(margin_action);
        let admit = best > 0.0;
        println!(
            "n={n}: attempts {} (q1 {lanes_q1}, q2 {lanes_q2}, fallback {lanes_fb}); mean margin serial {:.2} action {:.2}; {}",
            attempts.len(),
            margin_serial / count,
            margin_action / count,
            if admit { "admit" } else { "abstain" }
        );
        regimes.push(json!({
            "n": n, "completed": t >= tf, "attempts": attempts.len(),
            "lanes": {"q1": lanes_q1, "q2": lanes_q2, "fallback": lanes_fb},
            "total_margin": {"serial": margin_serial, "action_first": margin_action},
            "mean_margin": {"serial": margin_serial / count, "action_first": margin_action / count},
            "decision": if admit { "admit" } else { "abstain" },
            "attempt_records": attempts,
        }));
    }
    let report = json!({
        "schema": "vigilode-rnext06-homotopy-net-cost-v1",
        "node": "research/rnext06_homotopy_net_cost_20261003",
        "regimes": regimes,
        "gate": {
            "1_same_target": gate_same_target,
            "3_everything_charged": gate_charged,
            "4_decision_for_every_n": true,
        },
        "stop_condition": "parallel expansion stops in every regime whose total margin with the cheaper certificate arm is not positive",
        "timing": "not run; timing authority on HOLD; SPEEDUP_UNPROVEN retained",
    });
    write_output("RNEXT06_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
}
