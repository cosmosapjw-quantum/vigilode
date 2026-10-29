# Production readiness audit — 2026-10-18

Start with [the Korean report](../../docs/reviews/20261018_production_audit/REPORT_KO.md).

- `DECISION.json`: authoritative scoped decision. Production/global accuracy/measured speed remain held.
- `FINDINGS.json`: 27 source-linked findings; `evidence/*audit.json` preserves the original submissions.
- `NEXT_STEPS.json`: 12 implementation units with actual dependencies, APIs, and acceptance/cost contracts.
- `SOURCE_IDENTITY.json`: audited immutable baseline and final changed-source hashes.
- `VALIDATION.json`: actual executions, failures and repairs; compile/test/packaging status is distinct from scientific promotion.
- `RUN_RECORD.json`: embedded physmath 1.0.0 R1–R10/C1–C10 execution record.
- `evidence/independent_review.json`: final independent scope decisions and reviewed input hashes. Earlier author submissions intentionally retain their then-pending review fields; this review and `DECISION.json` resolve them.
- `WOODBURY_THEORY.md`, `woodbury_reference.py`: constructive O(N) direct-baseline proof and exact reference.
- `evidence/damped_oracle_generator.py`: high precision independent fixture generator.

## Reproduction

Use Rust 1.94.1 and the existing locked dependencies. With an ordinary populated
Cargo cache, run the following commands from the repository root. For an offline
directory source, use `bash tools/cargo-offline.sh --vendor-dir /absolute/vendor`
in place of `cargo`; no dependency manifests were changed.

```bash
cargo test --locked --offline -p rodas5p-core --test damped_moment_contracts --test damped_moment_enclosure_regressions --test production_audit_sparse --test leja_total_contract
cargo test --locked --offline -p rodas5p-core production_audit
cargo test --locked --offline -p rodas5p-core --doc
cargo test --locked --offline -p rodas5p-integrators --test production_audit_packet --test production_audit_split --test packet_window_contracts --test packet_trajectory_contracts
cargo test --locked --offline -p rodas5p-fair-ab --test packet_benchmark_contracts
python3 research/production_audit_20261018/validate_package.py
```

The exact reference script can be run with `--output /tmp/woodbury_checks.json`.
Keep original evidence immutable when rerunning. The default-path Rust probes
and their exact compiler/link commands are in `evidence/default_path_evidence/`
and `evidence/default_path_audit.json`. Original baseline-only cache/sparse probes
must run against commit `2fc61b9`, before the repaired private API. For packet
baseline reproduction, copy `evidence/packet_audit_baseline/reproduce_six_cases.rs`
to a test target in a separate checkout of that commit. This fixture was
reconstructed from the six logged scenarios and is explicitly not claimed as
the original test-file byte identity.

## Preservation and scope

Prior Loop12 proof/result directories and preregistrations were not rewritten.
The audit changes narrowly repair source behavior and honestly lower unsupported
authority. No benchmark ranking is inferred from targeted test timing. The
original failed allocation regression is retained beside its corrected rerun.
The library attachments for other projects were not added to this repository.

Author proof names in the independent review map to the byte-identical durable
copies in `REVIEW_BINDINGS.json`. Storage acknowledgement is the Git commit/ref
returned when this package is pushed; the validator checks content integrity,
not remote publication or the correctness of a mathematical theorem.
