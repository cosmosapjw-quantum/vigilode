//! Action-first path-sum certificate (research node
//! `research/thread_transfer_path_action_20261002`, thread-transfer DAG node
//! P1-PATH-ACTION).

#[path = "thread_transfer_common/mod.rs"]
mod common;

use common::{
    ATOL, INITIAL_RADIUS, R4_DIMENSIONS, RTOL, encloses_root, r4_fixture, root_distances,
    write_output, write_stage_inputs,
};
use rodas5p_integrators::{
    ParallelExecution, PathSumWork, blocked_action_doubling_certificate_with_execution,
    blocked_doubling_certificate_with_execution, certify_stage_target, upper_causal_solve,
    upper_path_sum_action, upper_path_sum_matrix,
};
use serde_json::{Value, json};

type PathSum = fn(&[Vec<f64>], &[f64], &mut PathSumWork) -> rodas5p_core::CoreResult<Vec<f64>>;
const EVALUATIONS: [(&str, PathSum); 3] = [
    ("matrix", upper_path_sum_matrix),
    ("action", upper_path_sum_action),
    ("causal", upper_causal_solve),
];

/// Deterministic integer block: entries and seeds in 0..=3.
fn integer_block(s: usize, salt: u64) -> (Vec<Vec<f64>>, Vec<f64>) {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64 ^ salt;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((state >> 33) % 4) as f64
    };
    let h = (0..s)
        .map(|i| (0..s).map(|j| if j < i { next() } else { 0.0 }).collect())
        .collect();
    let seed = (0..s).map(|_| next()).collect();
    (h, seed)
}

fn exact_identity() -> (bool, Value) {
    let mut rows = Vec::new();
    let mut all = true;
    for s in [1_usize, 3, 8, 9, 16] {
        for salt in 0..4_u64 {
            let (h, seed) = integer_block(s, salt);
            let mut works = Vec::new();
            let results = EVALUATIONS
                .iter()
                .map(|(name, evaluate)| {
                    let mut work = PathSumWork::default();
                    let value = evaluate(&h, &seed, &mut work).unwrap();
                    works.push(json!({"evaluation": name, "work": work}));
                    value
                })
                .collect::<Vec<_>>();
            // Integers below 2^53 throughout: no operation rounds.
            let largest = results.iter().flatten().copied().fold(0.0_f64, f64::max);
            let integral = results
                .iter()
                .flatten()
                .all(|value| value.fract() == 0.0 && *value < 2.0_f64.powi(53));
            let identical = results.windows(2).all(|pair| {
                pair[0]
                    .iter()
                    .zip(&pair[1])
                    .all(|(x, y)| x.to_bits() == y.to_bits())
            });
            all &= identical && integral;
            rows.push(json!({"s": s, "salt": salt, "identical": identical, "integral_below_2_53": integral, "largest": largest, "work": works}));
        }
    }
    (all, Value::Array(rows))
}

fn invalid_inputs_rejected() -> (bool, Value) {
    let good = vec![vec![0.0, 0.0], vec![1.0, 0.0]];
    let seed = vec![1.0, 1.0];
    type Case = (&'static str, Vec<Vec<f64>>, Vec<f64>);
    let cases: Vec<Case> = vec![
        (
            "nonzero-diagonal",
            vec![vec![1.0, 0.0], vec![1.0, 0.0]],
            seed.clone(),
        ),
        (
            "upper-entry",
            vec![vec![0.0, 1.0], vec![1.0, 0.0]],
            seed.clone(),
        ),
        (
            "tiny-upper-entry",
            vec![vec![0.0, 1.0e-300], vec![1.0, 0.0]],
            seed.clone(),
        ),
        (
            "negative-entry",
            vec![vec![0.0, 0.0], vec![-1.0, 0.0]],
            seed.clone(),
        ),
        (
            "nan-entry",
            vec![vec![0.0, 0.0], vec![f64::NAN, 0.0]],
            seed.clone(),
        ),
        (
            "infinite-entry",
            vec![vec![0.0, 0.0], vec![f64::INFINITY, 0.0]],
            seed.clone(),
        ),
        ("ragged", vec![vec![0.0, 0.0], vec![1.0]], seed.clone()),
        (
            "non-square",
            vec![vec![0.0, 0.0, 0.0], vec![1.0, 0.0, 0.0]],
            seed.clone(),
        ),
        ("seed-length", good.clone(), vec![1.0]),
        ("negative-seed", good.clone(), vec![1.0, -1.0]),
        ("nan-seed", good.clone(), vec![1.0, f64::NAN]),
        ("empty", vec![], vec![]),
    ];
    let mut all = true;
    let mut rows = Vec::new();
    for (name, h, s) in &cases {
        for (evaluation, evaluate) in EVALUATIONS {
            let mut work = PathSumWork::default();
            let rejected = evaluate(h, s, &mut work).is_err();
            all &= rejected;
            rows.push(json!({"case": name, "evaluation": evaluation, "rejected": rejected}));
        }
    }
    // The valid block is accepted by all three.
    for (_, evaluate) in EVALUATIONS {
        all &= evaluate(&good, &seed, &mut PathSumWork::default()).is_ok();
    }
    (all, Value::Array(rows))
}

#[test]
fn action_first_path_sum_is_exact_valid_and_cheaper() {
    let (identity, identity_rows) = exact_identity();
    let (rejection, rejection_rows) = invalid_inputs_rejected();
    let sequential = ParallelExecution::sequential();
    let mut enclosure = true;
    let mut fewer = true;
    let mut deterministic = true;
    let mut fixtures = Vec::new();
    for n in R4_DIMENSIONS {
        let f = r4_fixture(n).unwrap();
        let root = root_distances(n);
        let run = |execution: &ParallelExecution, action: bool| {
            let call = if action {
                blocked_action_doubling_certificate_with_execution
            } else {
                blocked_doubling_certificate_with_execution
            };
            call(
                &f.target,
                &f.problem,
                &f.candidate,
                &f.y_hat,
                &f.e_hat,
                &f.witness,
                ATOL,
                RTOL,
                INITIAL_RADIUS,
                8,
                execution,
            )
            .unwrap()
        };
        let matrix = run(&sequential, false);
        let action = run(&sequential, true);
        let serial = certify_stage_target(
            &f.target,
            &f.problem,
            &f.candidate,
            &f.y_hat,
            &f.e_hat,
            &f.witness,
            ATOL,
            RTOL,
        )
        .unwrap();
        let first = |c: &rodas5p_integrators::BlockedDoublingCertificate| {
            c.doubling
                .attempts
                .iter()
                .position(|a| a.closes)
                .map(|k| k + 1)
        };
        let (m_cert, a_cert) = (
            matrix.doubling.certificate.as_ref().unwrap(),
            action.doubling.certificate.as_ref().unwrap(),
        );
        let same_attempt = first(&matrix) == first(&action);
        let action_encloses = encloses_root(a_cert, &root);
        let matrix_encloses = encloses_root(m_cert, &root);
        let serial_encloses = encloses_root(&serial, &root);
        enclosure &= same_attempt && action_encloses && matrix_encloses;
        let cheaper = action.work.directed_operations < matrix.work.directed_operations
            && action.work.allocated_values < matrix.work.allocated_values;
        fewer &= cheaper;
        let mut bitwise = true;
        for workers in [1_usize, 2, 4, 8] {
            let execution = ParallelExecution::rayon(workers).unwrap();
            let other = run(&execution, true);
            bitwise &= other
                .doubling
                .certificate
                .as_ref()
                .unwrap()
                .stage_bound
                .iter()
                .flatten()
                .zip(a_cert.stage_bound.iter().flatten())
                .all(|(x, y)| x.to_bits() == y.to_bits());
        }
        deterministic &= bitwise;
        let relative = m_cert
            .stage_bound
            .iter()
            .flatten()
            .zip(a_cert.stage_bound.iter().flatten())
            .map(|(m, a)| (a / m - 1.0).abs())
            .fold(0.0_f64, f64::max);
        let width = |c: &rodas5p_integrators::StageCertificate| {
            c.output_bound
                .iter()
                .zip(&serial.output_bound)
                .map(|(d, s)| d / s.max(f64::MIN_POSITIVE))
                .fold(0.0_f64, f64::max)
        };
        println!(
            "n={n}: first closure matrix={:?} action={:?}; ops {} -> {} ({:.3}); alloc {} -> {}; encloses {action_encloses}; rel diff {relative:.2e}",
            first(&matrix),
            first(&action),
            matrix.work.directed_operations,
            action.work.directed_operations,
            action.work.directed_operations as f64 / matrix.work.directed_operations as f64,
            matrix.work.allocated_values,
            action.work.allocated_values
        );
        fixtures.push(json!({
            "dimension": n,
            "first_closure": {"matrix": first(&matrix), "action": first(&action)},
            "encloses_exact_root": {"action": action_encloses, "matrix": matrix_encloses, "serial": serial_encloses},
            "directed_operations": {"matrix": matrix.work.directed_operations, "action": action.work.directed_operations,
                "ratio": action.work.directed_operations as f64 / matrix.work.directed_operations as f64},
            "allocated_values": {"matrix": matrix.work.allocated_values, "action": action.work.allocated_values},
            "serial_directed_operations": serial.directed_operations,
            "max_relative_difference_action_vs_matrix": relative,
            "output_bound_over_serial_max": {"matrix": width(m_cert), "action": width(a_cert)},
            "bit_identical_workers_1_2_4_8": bitwise,
        }));
    }
    let gate = json!({
        "exact_identity": identity,
        "invalid_structure_rejected": rejection,
        "enclosure_and_same_closure": enclosure,
        "fewer_operations_and_allocations": fewer,
        "deterministic_workers": deterministic,
    });
    let pass = gate
        .as_object()
        .unwrap()
        .values()
        .all(|v| v.as_bool() == Some(true));
    let result = json!({
        "schema": "vigilode-thread-transfer-path-action-v1",
        "gate": gate,
        "verdict": if pass { "PASS" } else { "FAIL" },
        "r4_fixtures": fixtures,
        "exact_identity_cases": identity_rows,
        "invalid_input_cases": rejection_rows,
    });
    write_output("THREAD_TRANSFER_PATH_ACTION_OUTPUT", &result);
    println!(
        "{}",
        json!({"gate": result["gate"], "verdict": result["verdict"]})
    );
    // Correctness parts are contracts; the operation count is recorded only.
    assert!(identity && rejection && enclosure && deterministic);
}

/// Writes the exact R4 fixture inputs for `tools/thread_transfer_root_oracle.py`.
#[test]
#[ignore = "writes fixtures/thread_transfer_r4_stage_inputs.json"]
fn write_r4_stage_inputs() {
    write_stage_inputs();
}
