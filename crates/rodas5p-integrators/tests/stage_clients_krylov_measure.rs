//! Measurement program of loop node V06 (shared Krylov basis).
//!
//! One process runs one arm on one family at one tolerance and prints two
//! machine-readable lines: `V06_RUN_JSON {...}` (configuration, step
//! counts, every work counter, the client's statistics, the residual
//! audit) and `V06_STATE_JSON {...}` (the final state as hex bit patterns).
//! Nothing is compared or judged here; `shared_krylov/parse_results.py`
//! builds the results from these lines.
//!
//! ```text
//! V06_FAMILY=bruss1d V06_RTOL=1e-4 V06_ARM=C V06_PRECOND=none V06_AUDIT=1 \
//!   cargo test --release -p rodas5p-integrators --test stage_clients_krylov_measure \
//!   -- --ignored --nocapture --exact v06_run
//! ```
//!
//! Variables: `V06_FAMILY` (an id of `problems::declared`), `V06_RTOL`,
//! `V06_ARM` (`SETUP`, `A`, `B0`, `B`, `C`), `V06_PRECOND` (`none`,
//! `jacobi`), `V06_AUDIT` (`0`, `1`), `V06_MAX_DIM` (shared basis, default
//! 200), `V06_KEEP` (`0`, `1`: keep the basis across a rejected attempt),
//! `V06_LABEL` (free text copied into the record).

#[path = "stage_clients_krylov_problems.rs"]
mod problems;

use std::time::Instant;

use problems::{Arm, ArmOptions, adaptive, build_client, declared, run_arm};
use rodas5p_integrators::stage_clients::krylov::{KrylovStats, ResidualAuditReport};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn flag(name: &str) -> bool {
    variable(name).is_some_and(|value| value == "1")
}

fn stats_json(stats: &KrylovStats) -> Value {
    json!({
        "linearizations": stats.linearizations,
        "attempts": stats.attempts,
        "solves": stats.solves,
        "failed_solves": stats.failed_solves,
        "krylov_vectors": stats.krylov_vectors,
        "residual_jvps": stats.residual_jvps,
        "restart_passes": stats.restart_passes,
        "max_solve_vectors": stats.max_solve_vectors,
        "zero_iterate_accepts": stats.zero_iterate_accepts,
        "confirmation_failures": stats.confirmation_failures,
        "stage_krylov_vectors": stats.stage_krylov_vectors,
        "stage_solves": stats.stage_solves,
        "shared_vectors": stats.shared_vectors,
        "shared_accepted_solves": stats.shared_accepted_solves,
        "fallback_solves": stats.fallback_solves,
        "fallback_krylov_vectors": stats.fallback_krylov_vectors,
        "fallback_residual_jvps": stats.fallback_residual_jvps,
        "basis_dim_max": stats.basis_dim_max,
        "basis_dim_sum": stats.basis_dim_sum,
        "basis_dim_attempts": stats.basis_dim_attempts,
        "basis_kept": stats.basis_kept,
        "gmres_storage_bytes": stats.gmres_storage_bytes,
        "shared_storage_bytes": stats.shared_storage_bytes,
    })
}

fn audit_json(audit: &ResidualAuditReport) -> Value {
    json!({
        "checks": audit.checks,
        "failures": audit.failures,
        "max_ratio": audit.max_ratio,
        "zero_threshold_failures": audit.zero_threshold_failures,
        "first_failure": audit.first_failure,
        "jvp_calls": audit.jvp_calls,
    })
}

#[test]
#[ignore = "measurement program: run with --ignored --nocapture and the V06_* variables"]
fn v06_run() {
    let Some(family_id) = variable("V06_FAMILY") else {
        println!("V06_USAGE set V06_FAMILY, V06_RTOL and V06_ARM; see the file header");
        return;
    };
    let rtol: f64 = variable("V06_RTOL")
        .expect("V06_RTOL")
        .parse()
        .expect("V06_RTOL is a number");
    let arm_label = variable("V06_ARM").expect("V06_ARM");
    let precond = variable("V06_PRECOND").unwrap_or_else(|| "none".into());
    assert!(
        precond == "none" || precond == "jacobi",
        "V06_PRECOND is none or jacobi"
    );
    let label = variable("V06_LABEL").unwrap_or_default();
    let family = declared(&family_id).expect("family");
    let config = adaptive(&family, rtol);
    let mut record = json!({
        "schema": "vigilode.loop-l1.v06.run.v1",
        "label": label,
        "family": family_id,
        "n": family.problem.dimension,
        "t_span": [family.span.0, family.span.1],
        "rtol": config.rtol,
        "atol": config.atol,
        "arm": arm_label,
        "precond": precond,
    });
    let fields = record.as_object_mut().expect("object");

    if arm_label == "SETUP" {
        // The instruction baseline: everything a run does before its
        // integration, without building a client.
        fields.insert("setup_only".into(), json!(true));
        println!("V06_RUN_JSON {record}");
        return;
    }

    let arm = Arm::parse(&arm_label).expect("V06_ARM is SETUP, A, B0, B or C");
    let mut options = ArmOptions::new(arm);
    options.jacobi = precond == "jacobi";
    options.audit = flag("V06_AUDIT");
    options.keep = flag("V06_KEEP");
    if let Some(max_dim) = variable("V06_MAX_DIM") {
        options.max_dim = max_dim.parse().expect("V06_MAX_DIM is an integer");
    }
    if arm == Arm::Protected {
        assert!(
            !options.jacobi && !options.audit,
            "arm A has no preconditioner and no audit"
        );
    } else {
        // Constructible before the clock starts (the run builds its own).
        drop(build_client(&family, &options).expect("client"));
    }
    fields.insert("audit".into(), json!(options.audit));
    fields.insert(
        "shared_max_dim".into(),
        if arm == Arm::Shared {
            json!(options.max_dim)
        } else {
            Value::Null
        },
    );
    fields.insert("shared_keep".into(), json!(options.keep));

    let start = Instant::now();
    let outcome = run_arm(&family, rtol, &options);
    let wall = start.elapsed().as_secs_f64();
    fields.insert("wall_seconds_diagnostic_only".into(), json!(wall));
    let run = match outcome {
        Ok(run) => run,
        Err(error) => {
            fields.insert("completed".into(), json!(false));
            fields.insert("error".into(), json!(error.to_string()));
            println!("V06_RUN_JSON {record}");
            return;
        }
    };
    let mut hasher = Sha256::new();
    let mut hex = String::with_capacity(16 * run.y_end.len());
    for value in &run.y_end {
        hasher.update(value.to_le_bytes());
        hex.push_str(&format!("{:016x}", value.to_bits()));
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    fields.insert("completed".into(), json!(true));
    fields.insert("success".into(), json!(run.success));
    fields.insert("attempts".into(), json!(run.attempts));
    fields.insert("accepted".into(), json!(run.accepted));
    fields.insert("rejected".into(), json!(run.rejected));
    fields.insert("t_end".into(), json!(run.t_end));
    fields.insert("jvp_calls".into(), json!(run.counters.jvp_calls));
    fields.insert(
        "counters".into(),
        serde_json::to_value(run.counters).expect("counters"),
    );
    if let Some(stats) = &run.stats {
        // Every JVP the client charged is either a Krylov vector or a
        // stopping-test residual.
        fields.insert(
            "jvp_accounting_closed".into(),
            json!(stats.krylov_vectors + stats.residual_jvps == run.counters.jvp_calls),
        );
        fields.insert("client".into(), stats_json(stats));
    }
    if let Some(audit) = &run.audit {
        fields.insert("residual_audit".into(), audit_json(audit));
    }
    fields.insert("state_sha256".into(), json!(digest));
    println!("V06_RUN_JSON {record}");
    println!(
        "V06_STATE_JSON {}",
        json!({
            "label": label,
            "family": family_id,
            "rtol": config.rtol,
            "arm": arm_label,
            "precond": precond,
            "audit": options.audit,
            "shared_max_dim": if arm == Arm::Shared { json!(options.max_dim) } else { Value::Null },
            "shared_keep": options.keep,
            "n": run.y_end.len(),
            "state_sha256": digest,
            "state_hex": hex,
        })
    );
}
