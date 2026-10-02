//! Native replay of the R4 homotopy radius cap (research node
//! `research/thread_transfer_radius_replay_20261002`, thread-transfer DAG node
//! P0-RADIUS-REPLAY), ported from
//! `docs/reviews/thread_transfer_20261002/probes/native_replay_homotopy.rs`.
//!
//! This does not change the old R4 result or its six-attempt gate: the six-
//! attempt failure at n = 8 and 16 is asserted as a regression, and the
//! extended schedule is a separate diagnosis.

#[path = "thread_transfer_common/mod.rs"]
mod common;

use common::{ATOL, INITIAL_RADIUS, R4_DIMENSIONS, RTOL, r4_fixture, write_output};
use rodas5p_integrators::{
    ParallelExecution, RadiusAttempt, blocked_doubling_certificate_with_execution,
    doubling_certificate_with_execution,
};
use serde_json::{Value, json};

const REFERENCE: &str = include_str!(
    "../../../docs/reviews/thread_transfer_20261002/results/homotopy_radius_probe.json"
);

fn first_closure(attempts: &[RadiusAttempt]) -> Option<usize> {
    attempts.iter().position(|a| a.closes).map(|k| k + 1)
}

fn ledger(attempts: &[RadiusAttempt]) -> Value {
    attempts
        .iter()
        .map(|a| {
            json!({
                "attempt": a.attempt + 1,
                "radius": a.radius,
                "closes": a.closes,
                "max_state_radius": a.max_state_radius,
                "max_state_radius_bits": format!("{:016x}", a.max_state_radius.to_bits()),
            })
        })
        .collect()
}

#[test]
fn extended_radius_schedule_is_separate_from_the_original_failed_gate() {
    let reference: Value = serde_json::from_str(REFERENCE).unwrap();
    let execution = ParallelExecution::sequential();
    let expected_closure = [4_usize, 5, 6, 7, 8];
    let mut rows = Vec::new();
    let mut gate = json!({
        "old_failure_preserved": true,
        "extended_closure_attempts_match": true,
        "full_equals_blocked": true,
        "native_agrees_with_exact_reference": true,
    });
    let mut worst_relative = (f64::INFINITY, f64::NEG_INFINITY);
    for (index, n) in R4_DIMENSIONS.into_iter().enumerate() {
        let f = r4_fixture(n).unwrap();
        let run = |attempts: usize| {
            let blocked = blocked_doubling_certificate_with_execution(
                &f.target,
                &f.problem,
                &f.candidate,
                &f.y_hat,
                &f.e_hat,
                &f.witness,
                ATOL,
                RTOL,
                INITIAL_RADIUS,
                attempts,
                &execution,
            )
            .unwrap();
            let full = doubling_certificate_with_execution(
                &f.target,
                &f.problem,
                &f.candidate,
                &f.y_hat,
                &f.e_hat,
                &f.witness,
                ATOL,
                RTOL,
                INITIAL_RADIUS,
                attempts,
                &execution,
            )
            .unwrap();
            (blocked, full)
        };
        let (old_blocked, old_full) = run(6);
        let (new_blocked, new_full) = run(8);

        // 1. The six-attempt gate of L-0017/L-0024, unchanged.
        let old_ok = if n >= 8 {
            old_blocked.doubling.certificate.is_none()
                && old_full.certificate.is_none()
                && old_blocked.doubling.attempts.len() == 6
                && old_full.attempts.len() == 6
        } else {
            old_blocked.doubling.certificate.is_some() && old_full.certificate.is_some()
        };
        // 2. Extended closure.
        let blocked_first = first_closure(&new_blocked.doubling.attempts);
        let full_first = first_closure(&new_full.attempts);
        let closure_ok = blocked_first == Some(expected_closure[index])
            && full_first == Some(expected_closure[index]);
        // 3. Full = blocked, bounds and every attempt's state radius.
        let identical = match (&new_full.certificate, &new_blocked.doubling.certificate) {
            (Some(full), Some(blocked)) => full
                .stage_bound
                .iter()
                .flatten()
                .zip(blocked.stage_bound.iter().flatten())
                .all(|(x, y)| x.to_bits() == y.to_bits()),
            _ => false,
        } && new_full.attempts.len() == new_blocked.doubling.attempts.len()
            && new_full
                .attempts
                .iter()
                .zip(&new_blocked.doubling.attempts)
                .all(|(x, y)| x.max_state_radius.to_bits() == y.max_state_radius.to_bits());
        // 4. Native against the exact-rational Python reference.
        let trials = reference["rows"][index]["trials"].as_array().unwrap();
        assert_eq!(
            reference["rows"][index]["dimension"].as_u64(),
            Some(n as u64)
        );
        let mut agreement = Vec::new();
        let mut agrees = true;
        for attempt in &new_blocked.doubling.attempts {
            let python = trials[attempt.attempt]["required"].as_f64().unwrap();
            assert_eq!(
                trials[attempt.attempt]["radius"].as_f64(),
                Some(attempt.radius)
            );
            let relative = attempt.max_state_radius / python - 1.0;
            worst_relative = (
                worst_relative.0.min(relative),
                worst_relative.1.max(relative),
            );
            agrees &= (-1.0e-12..=1.0e-9).contains(&relative);
            agreement.push(json!({"attempt": attempt.attempt + 1, "python_required": python, "relative_difference": relative}));
        }
        gate["old_failure_preserved"] =
            json!(gate["old_failure_preserved"].as_bool().unwrap() && old_ok);
        gate["extended_closure_attempts_match"] =
            json!(gate["extended_closure_attempts_match"].as_bool().unwrap() && closure_ok);
        gate["full_equals_blocked"] =
            json!(gate["full_equals_blocked"].as_bool().unwrap() && identical);
        gate["native_agrees_with_exact_reference"] = json!(
            gate["native_agrees_with_exact_reference"]
                .as_bool()
                .unwrap()
                && agrees
        );
        println!(
            "n={n}: six-attempt closes={} (blocked) / {} (full); first closure blocked={blocked_first:?} full={full_first:?}; identical={identical}",
            old_blocked.doubling.certificate.is_some(),
            old_full.certificate.is_some()
        );
        rows.push(json!({
            "dimension": n,
            "six_attempts": {
                "blocked_certificate": old_blocked.doubling.certificate.is_some(),
                "full_certificate": old_full.certificate.is_some(),
                "blocked_attempts": ledger(&old_blocked.doubling.attempts),
                "full_attempts": ledger(&old_full.attempts),
            },
            "eight_attempts": {
                "blocked_first_closure": blocked_first,
                "full_first_closure": full_first,
                "blocked_attempts": ledger(&new_blocked.doubling.attempts),
                "full_attempts": ledger(&new_full.attempts),
                "blocked_directed_operations": new_blocked.work.directed_operations,
                "blocked_allocated_values": new_blocked.work.allocated_values,
                "full_bit_identical_to_blocked": identical,
                "blocked_output_bound_max": new_blocked.doubling.certificate.as_ref().map(|c| c.output_bound.iter().copied().fold(0.0_f64, f64::max)),
            },
            "python_agreement": agreement,
        }));
    }
    let pass = gate
        .as_object()
        .unwrap()
        .values()
        .all(|v| v.as_bool() == Some(true));
    let result = json!({
        "schema": "vigilode-thread-transfer-radius-replay-v1",
        "fixture": "r4_studies::homotopy_cost_study diagonal quadratic, h=0.05, candidate h f(y), D0=1e-3, factor 4",
        "reference": "docs/reviews/thread_transfer_20261002/results/homotopy_radius_probe.json",
        "rows": rows,
        "relative_difference_range": [worst_relative.0, worst_relative.1],
        "gate": gate,
        "verdict": if pass { "PASS" } else { "FAIL" },
    });
    write_output("THREAD_TRANSFER_RADIUS_OUTPUT", &result);
    println!(
        "{}",
        json!({"gate": result["gate"], "verdict": result["verdict"], "relative_difference_range": result["relative_difference_range"]})
    );
    // The regression part is unconditional: the old six-attempt schedule
    // must keep failing at n = 8 and 16 (L-0017/L-0024 stay FAIL).
    assert_eq!(result["gate"]["old_failure_preserved"], json!(true));
}
