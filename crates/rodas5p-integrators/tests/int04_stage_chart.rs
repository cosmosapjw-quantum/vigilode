//! The stage-coordinate candidate adapter on the INT-04 families (research
//! node `research/int04_stage_chart_20261003`, integrated DAG node INT-04):
//! root correspondence, certification of the restored stages by the
//! original target, approximate-W predictors exposed by the certificate, and
//! work accounting. Counter-only; no timing.

#[path = "int04_common/mod.rs"]
mod common;
#[path = "rnext_common/mod.rs"]
mod rnext;

use common::{Scaling, Triangular, families, projections, sequential_root, stages, witness};
use rnext::write_output;
use rodas5p_core::rodas5p_coefficients;
use rodas5p_integrators::{
    ChartStatus, StageChart, StageTarget, certify_stage_target, stage_chart_candidate,
};
use serde_json::json;

#[test]
#[ignore = "research run of research/int04_stage_chart_20261003; release build"]
fn stage_chart_adapter() {
    let target = StageTarget::sequential(rodas5p_coefficients().unwrap()).unwrap();
    let s = target.stages();
    let (atol, rtol) = (1.0e-6, 1.0e-6);
    let mut gate_root = true;
    let mut gate_authority = true;
    let mut gate_accounting = true;
    let mut records = Vec::new();
    for (label, problem) in families() {
        let n = problem.dimension();
        let reference = sequential_root(&target, &problem, target.gamma);
        let scale = reference.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
        let w = witness(&problem, target.gamma);
        let charts: [Box<dyn StageChart>; 2] = [
            Box::new(Triangular { s, n, beta: 0.5 }),
            Box::new(Scaling { s, n }),
        ];
        let mut chart_records = Vec::new();
        for chart in &charts {
            let z0 = chart.inverse(&vec![0.0; s * n]).unwrap();
            let candidate =
                stage_chart_candidate(&target, &problem, chart.as_ref(), &z0, 20, 1.0e-13).unwrap();
            let k: Vec<f64> = candidate.stages.concat();
            let difference = k
                .iter()
                .zip(&reference)
                .fold(0.0_f64, |m, (a, b)| m.max((a - b).abs()))
                / scale.max(f64::MIN_POSITIVE);
            let (y_hat, e_hat) = projections(&target, &problem, &k);
            let certificate = certify_stage_target(
                &target,
                &problem,
                &candidate.stages,
                &y_hat,
                &e_hat,
                &w,
                atol,
                rtol,
            );
            let output = certificate.as_ref().map(|c| c.output_wrms_upper).ok();
            let converged =
                candidate.status == ChartStatus::Converged && candidate.iterations <= 20;
            gate_root &= converged && difference <= 1.0e-12 && output.is_some_and(|o| o <= 1.0e-8);
            let it = candidate.iterations as u64;
            let wk = candidate.work;
            let m = (s * n) as u64;
            gate_accounting &= converged
                && wk.residual_evaluations == it + 1
                && wk.chart_forward == it + 1
                && wk.chart_jvps == m * it
                && wk.residual_jacobian_actions == m * it
                && wk.factorizations == it
                && (it == 0 || wk.factorization_order == m);
            chart_records.push(json!({
                "chart": candidate.chart, "status": candidate.status, "iterations": candidate.iterations,
                "chart_residual_inf": candidate.chart_residual_inf,
                "relative_difference_from_sequential_root": difference,
                "certified_output_wrms_upper": output, "work": wk,
            }));
        }
        // An RVJ-style approximate-W predictor: the original certificate
        // must bound its actual distance from the root.
        let approximate = sequential_root(&target, &problem, target.gamma * (1.0 + 1.0e-3));
        let (y_hat, e_hat) = projections(&target, &problem, &approximate);
        let cert = certify_stage_target(
            &target,
            &problem,
            &stages(&approximate, n),
            &y_hat,
            &e_hat,
            &w,
            atol,
            rtol,
        )
        .unwrap();
        let mut exposed = true;
        let mut worst_ratio = f64::INFINITY;
        for i in 0..s {
            for a in 0..n {
                let distance = (approximate[i * n + a] - reference[i * n + a]).abs();
                exposed &= cert.stage_bound[i][a] >= distance;
                if distance > 0.0 {
                    worst_ratio = worst_ratio.min(cert.stage_bound[i][a] / distance);
                }
            }
        }
        gate_authority &= exposed;
        let record = json!({
            "family": label, "n": n, "h": problem.h, "charts": chart_records,
            "approximate_w": {
                "max_distance": approximate.iter().zip(&reference).fold(0.0_f64, |m, (a, b)| m.max((a - b).abs())),
                "certified_output_wrms_upper": cert.output_wrms_upper,
                "bound_encloses_distance": exposed, "min_bound_over_distance": worst_ratio,
            },
            "direct_sequential_work": {"solves": s, "order": n, "factorizations": 1},
        });
        println!("{record}");
        records.push(record);
    }
    let gate = json!({
        "1_root_correspondence": gate_root,
        "2_chart_identities": "contract tests int04_stage_chart_contracts",
        "3_typed_rejections": "contract tests int04_stage_chart_contracts",
        "4_no_authority": gate_authority,
        "5_accounting": gate_accounting,
    });
    let report = json!({
        "schema": "vigilode-int04-stage-chart-v1",
        "node": "research/int04_stage_chart_20261003",
        "families": records,
        "gate": gate,
        "timing": "not run",
    });
    write_output("INT04_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
}
