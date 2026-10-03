//! Structural diagonal certificate pipeline on actual transactional runs
//! (research node `research/int02_structural_certificate_20261003`,
//! integrated DAG node INT-02): the dense native-certificate arm against the
//! prepared structured arm on the R4 diagonal quadratic family, n = 1..64.
//! Counter-only; no timing.

#[path = "thread_transfer_common/mod.rs"]
mod common;

use std::sync::Mutex;

use common::write_output;
use rodas5p_core::{CoreResult, WorkCounters};
use rodas5p_integrators::{
    AdaptiveControllerState, AdaptiveStepConfig, DiagonalQuadraticModel, DiagonalStageProblem,
    InverseWitness, ModelBinding, Q2Admission, Q2CertificateSource, QuadraticModel,
    QuadraticStageProblem, StageCertificate, TransactionalQ1Q2Config, TransactionalQ1Q2StepReport,
    rodas_next_step_after_attempt, transactional_q1_q2_step_with_admission,
};
use serde_json::{Value, json};

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

/// Source calls and stored slots, per attempt.
#[derive(Clone, Copy, Debug, Default)]
struct Calls {
    problems: u64,
    witnesses: u64,
    jacobian_slots: usize,
    witness_slots: usize,
}

/// Delegates to a source and counts what it builds.
struct Counting<'a> {
    inner: &'a dyn Q2CertificateSource,
    calls: Mutex<Calls>,
}

impl Counting<'_> {
    fn take(&self) -> Calls {
        std::mem::take(&mut *self.calls.lock().unwrap())
    }
}

impl Q2CertificateSource for Counting<'_> {
    fn stage_problem(&self, t: f64, y: &[f64], h: f64) -> CoreResult<QuadraticStageProblem> {
        let problem = self.inner.stage_problem(t, y, h)?;
        let mut calls = self.calls.lock().unwrap();
        calls.problems += 1;
        calls.jacobian_slots = problem.jacobian.iter().map(Vec::len).sum();
        Ok(problem)
    }
    fn binding(&self) -> ModelBinding {
        self.inner.binding()
    }
    fn witness(&self, problem: &QuadraticStageProblem, gamma: f64) -> CoreResult<InverseWitness> {
        let witness = self.inner.witness(problem, gamma)?;
        let mut calls = self.calls.lock().unwrap();
        calls.witnesses += 1;
        calls.witness_slots = witness.stored_slots();
        Ok(witness)
    }
    fn diagonal_stage_problem(
        &self,
        t: f64,
        y: &[f64],
        h: f64,
    ) -> Option<CoreResult<DiagonalStageProblem>> {
        let result = self.inner.diagonal_stage_problem(t, y, h)?;
        if let Ok(problem) = &result {
            let mut calls = self.calls.lock().unwrap();
            calls.problems += 1;
            calls.jacobian_slots = problem.diagonal.len();
        }
        Some(result)
    }
    fn diagonal_witness(
        &self,
        problem: &DiagonalStageProblem,
        gamma: f64,
    ) -> CoreResult<InverseWitness> {
        let witness = self.inner.diagonal_witness(problem, gamma)?;
        let mut calls = self.calls.lock().unwrap();
        calls.witnesses += 1;
        calls.witness_slots = witness.stored_slots();
        Ok(witness)
    }
}

fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

fn same_bounds(a: &StageCertificate, b: &StageCertificate) -> bool {
    a.stage_bound.len() == b.stage_bound.len()
        && a.stage_bound
            .iter()
            .zip(&b.stage_bound)
            .all(|(x, y)| same_bits(x, y))
        && same_bits(&a.output_bound, &b.output_bound)
        && same_bits(&a.embedded_difference_bound, &b.embedded_difference_bound)
        && a.output_wrms_upper.to_bits() == b.output_wrms_upper.to_bits()
        && a.embedded_difference_wrms_upper.to_bits() == b.embedded_difference_wrms_upper.to_bits()
        && a.embedded_target_wrms_upper.to_bits() == b.embedded_target_wrms_upper.to_bits()
        && a.embedded_target_wrms_lower.to_bits() == b.embedded_target_wrms_lower.to_bits()
}

struct Attempt {
    report: TransactionalQ1Q2StepReport,
    next_h: f64,
    calls: Calls,
}

/// The adaptive transactional run of L-0050 under one admission.
fn run(
    problem: &rodas5p_integrators::OdeProblem,
    y0: &[f64],
    source: &Counting<'_>,
    structured: bool,
) -> Vec<Attempt> {
    let config = TransactionalQ1Q2Config::default();
    let (atol, rtol) = (1.0e-6, 1.0e-6);
    let adaptive = AdaptiveStepConfig {
        atol,
        rtol,
        initial_step: 0.05,
        min_step: 1.0e-10,
        max_step: 0.5,
        ..AdaptiveStepConfig::default()
    };
    let (mut t, tf) = (0.0, 0.5);
    let mut y = y0.to_vec();
    let mut h = adaptive.initial_step;
    let mut controller = AdaptiveControllerState::default();
    let mut attempts = Vec::new();
    while t < tf && attempts.len() < 200 {
        let trial = h.min(tf - t);
        let admission = if structured {
            Q2Admission::PreparedStructuredCertificate(source)
        } else {
            Q2Admission::NativeTargetCertificate(source)
        };
        let report = transactional_q1_q2_step_with_admission(
            problem,
            t,
            &y,
            trial,
            &config,
            atol,
            rtol,
            false,
            admission,
            &mut WorkCounters::default(),
        )
        .unwrap();
        let calls = source.take();
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
        attempts.push(Attempt {
            report,
            next_h: h,
            calls,
        });
    }
    attempts
}

/// Least-squares slope of log y against log x.
fn slope(points: &[(f64, f64)]) -> f64 {
    let m = points.len() as f64;
    let (sx, sy) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x.ln(), b + y.ln()));
    let (mx, my) = (sx / m, sy / m);
    let (num, den) = points.iter().fold((0.0, 0.0), |(n, d), (x, y)| {
        (n + (x.ln() - mx) * (y.ln() - my), d + (x.ln() - mx).powi(2))
    });
    num / den
}

#[test]
#[ignore = "research run of research/int02_structural_certificate_20261003; release build"]
fn structural_certificate() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let l0050: Value = serde_json::from_str(
        &std::fs::read_to_string(
            manifest.join("../../research/rnext06_homotopy_net_cost_20261003/RESULTS.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut gate_same = true;
    let mut gate_slots = true;
    let mut gate_once = true;
    let mut gate_replay = true;
    let mut regimes = Vec::new();
    let (mut dense_ops, mut structured_ops) = (Vec::new(), Vec::new());
    for n in [1usize, 2, 4, 8, 16, 32, 64] {
        let (m, y0) = model(n);
        let problem = m.ode_problem().unwrap();
        let diagonal = DiagonalQuadraticModel::new(m.clone()).unwrap();
        let dense_source = Counting {
            inner: &m,
            calls: Mutex::new(Calls::default()),
        };
        let structured_source = Counting {
            inner: &diagonal,
            calls: Mutex::new(Calls::default()),
        };
        let dense = run(&problem, &y0, &dense_source, false);
        let structured = run(&problem, &y0, &structured_source, true);
        let mut same = dense.len() == structured.len();
        let (mut candidates, mut ops_dense, mut ops_structured) = (0u64, 0u64, 0u64);
        let mut records = Vec::new();
        for (d, s) in dense.iter().zip(&structured) {
            let (rd, rs) = (&d.report, &s.report);
            let mut same_attempt = rd.lane == rs.lane
                && rd.step.accepted == rs.step.accepted
                && same_bits(&rd.step.y_new, &rs.step.y_new)
                && rd.step.error_norm.to_bits() == rs.step.error_norm.to_bits()
                && d.next_h.to_bits() == s.next_h.to_bits();
            let mut record = json!({
                "lane": format!("{:?}", rd.lane), "accepted": rd.step.accepted,
                "dense_calls": {"problems": d.calls.problems, "witnesses": d.calls.witnesses,
                                "jacobian_slots": d.calls.jacobian_slots, "witness_slots": d.calls.witness_slots},
                "structured_calls": {"problems": s.calls.problems, "witnesses": s.calls.witnesses,
                                     "jacobian_slots": s.calls.jacobian_slots, "witness_slots": s.calls.witness_slots},
                "dense_certificate_operations": rd.work.certificate_operations,
                "structured_certificate_operations": rs.work.certificate_operations,
            });
            match (&rd.q2_certificate, &rs.q2_certificate) {
                (Some(a), Some(b)) => {
                    same_attempt &= a.accepted == b.accepted
                        && a.budget_lower.to_bits() == b.budget_lower.to_bits();
                    match (&a.certificate, &b.certificate) {
                        (Some(ca), Some(cb)) => {
                            same_attempt &= same_bounds(ca, cb);
                            candidates += 1;
                            ops_dense += ca.directed_operations;
                            ops_structured += cb.directed_operations;
                            gate_slots &= s.calls.jacobian_slots + s.calls.witness_slots == 2 * n
                                && d.calls.jacobian_slots + d.calls.witness_slots == 2 * n * n;
                            gate_once &= s.calls.problems == 1 && s.calls.witnesses == 1;
                            record["output_wrms_upper"] = json!(ca.output_wrms_upper);
                        }
                        (None, None) => {}
                        _ => same_attempt = false,
                    }
                }
                (None, None) => {}
                _ => same_attempt = false,
            }
            record["same"] = json!(same_attempt);
            same &= same_attempt;
            records.push(record);
        }
        gate_same &= same;
        // Replay of the pre-change default path (L-0050, n <= 16).
        let mut replay = Value::Null;
        if let Some(old) = l0050["regimes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["n"].as_u64() == Some(n as u64))
        {
            let old_attempts = old["attempt_records"].as_array().unwrap();
            let matches = old_attempts.len() == dense.len()
                && old_attempts.iter().zip(&dense).all(|(o, d)| {
                    o["lane"].as_str().unwrap() == format!("{:?}", d.report.lane)
                        && o["accepted"].as_bool().unwrap() == d.report.step.accepted
                        && o["serial_certificate_operations"].as_u64().unwrap()
                            == d.report.work.certificate_operations
                });
            gate_replay &= matches;
            replay = json!(matches);
        }
        if candidates > 0 && n >= 4 {
            dense_ops.push((n as f64, ops_dense as f64 / candidates as f64));
            structured_ops.push((n as f64, ops_structured as f64 / candidates as f64));
        }
        let mean = |total: u64| {
            if candidates == 0 {
                0.0
            } else {
                total as f64 / candidates as f64
            }
        };
        let summary = json!({
            "n": n, "attempts": dense.len(), "certified_candidates": candidates, "identical": same,
            "replays_l0050": replay,
            "mean_certificate_operations": {"dense": mean(ops_dense), "structured": mean(ops_structured)},
            "source_calls_per_attempt_with_candidate": {
                "dense": records.iter().find(|r| r.get("output_wrms_upper").is_some()).map(|r| r["dense_calls"].clone()),
                "structured": records.iter().find(|r| r.get("output_wrms_upper").is_some()).map(|r| r["structured_calls"].clone()),
            },
        });
        println!("{summary}");
        regimes.push(json!({"summary": summary, "attempt_records": records}));
    }
    let (slope_dense, slope_structured) = (slope(&dense_ops), slope(&structured_ops));
    let gate = json!({
        "1_same_bounds_and_decisions": gate_same,
        "2_storage_and_work": gate_slots && slope_structured <= 1.1 && slope_dense >= 1.8,
        "3_one_construction": gate_once,
        "4_rejections": "contract tests int02_structural_certificate_contracts",
        "5_defaults_unchanged": gate_replay,
        "slope_dense": slope_dense,
        "slope_structured": slope_structured,
        "slots_2n_vs_2n2": gate_slots,
    });
    let report = json!({
        "schema": "vigilode-int02-structural-certificate-v1",
        "node": "research/int02_structural_certificate_20261003",
        "regimes": regimes,
        "gate": gate,
        "timing": "not run; timing authority on HOLD",
    });
    write_output("INT02_OUTPUT", &report);
    println!("{}", serde_json::to_string_pretty(&report["gate"]).unwrap());
}
