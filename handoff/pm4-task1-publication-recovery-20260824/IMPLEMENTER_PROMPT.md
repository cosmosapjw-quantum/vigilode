# Codex implementation prompt — PM-4 Task-1 R5 vendor closure

You are the implementation agent. Work autonomously, do not ask the user questions, and do not guess across a specification boundary.

## Repository and handoff

Repository: `https://github.com/cosmosapjw-quantum/vigilode`

Handoff branch:

```text
handoff/pm4-task1-publication-recovery-20260824
```

Read in order:

1. `AGENTS.md`
2. `README.md`
3. `CURRENT_STATE.json`
4. `ARCHIVE_AUTHORITY_CORRECTION.json`
5. `HANDOFF_COMPLETENESS_CORRECTION.json`
6. `AUDIT_COMPILED_EXEC_PLAN.yaml`
7. `P0_P1_THREAT_CATALOG.yaml`
8. `INVARIANT_TEST_MATRIX.yaml`
9. `EVIDENCE_CHAIN.md`
10. `templates/COMPLETION_EVIDENCE_SCHEMA.json`
11. `templates/COMPLETION_EVIDENCE_EXAMPLE.json`
12. `acceptance/README.md`

Treat `AUDIT_COMPILED_EXEC_PLAN.yaml` as the execution contract. Inputs under `inputs/` are immutable witnesses. Do not modify them in place.

## Mandatory control-plane preflight

Before any R5 implementation or archive mutation, run:

```bash
python3 -m unittest acceptance.test_handoff_completeness_contract -v
python3 -m unittest acceptance.test_completion_evidence_contract -v
python3 acceptance/test_completion_evidence_schema_contract.py \
  --schema templates/COMPLETION_EVIDENCE_SCHEMA.json \
  --instance templates/COMPLETION_EVIDENCE_EXAMPLE.json
```

All must pass. Do not invent, defer, or silently replace a missing repository-local contract file.

## Archive authority gate

Require exactly:

```text
R4 archive
~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz

canonical outer SHA-256
6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333
```

The prior `b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095` value is withdrawn and must be rejected, not accepted as an alternative.

Run the local sidecar, internal manifest, sealed patch, and R4 script checks, including:

```bash
python3 acceptance/test_archive_authority_contract.py \
  --archive ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz \
  --sidecar ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256
```

## Primary task

Replace the R4 hard-coded `262` Cargo-vendor count gate with a structurally checked, Cargo-resolved offline closure gate. Produce a deterministic R5 kit, run it against:

```text
/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
```

and publish the unchanged Task-1 source patch to draft PR #11 by one ordinary non-force fast-forward. Stop before merge and before PM-4 Task 2.

## Exact remote authority

```text
canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

expected feature head before publication
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

target feature branch
research/v38d-exploratory-benchmark-substrate

PR
#11 — must remain OPEN / DRAFT / UNMERGED

Task-1 patch SHA-256
705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3
```

The observed value `308` is failure evidence, not a required count. At one immediate level, ignore dot-prefixed and manifestless directories; only directories containing `Cargo.toml` are package candidates; each package candidate must contain parseable `.cargo-checksum.json`; require `faer-0.24.4`. Then let `cargo metadata --frozen --format-version 1` prove the checked-in lockfile/config/vendor closure.

## Forbidden operations

- no `main` mutation;
- no force push or merge;
- no change to `.cargo/config.toml`, `Cargo.lock`, dependencies, or any Task-1 patch byte;
- no wall timing, candidate ranking, speedup claim, or Task 2;
- no weakening/deleting acceptance tests;
- no success report after partial verification.

## Required engineering method

1. Create an isolated worktree or fresh clone and verify the exact refs.
2. Reproduce R4 `308 (expected 262)` and retain RED evidence.
3. Run acceptance tests before implementation and retain RED evidence.
4. Copy immutable R4 inputs into a new R5 directory.
5. Implement `validate_vendor_source.py` with the API/CLI contract in `acceptance/README.md`.
6. Add tests for valid 262, valid 308, incidental hidden/manifestless directories, missing checksum, missing `faer-0.24.4`, and deterministic JSON.
7. Modify `publish_pm4_task1.sh` minimally: remove exact count; call helper; create and verify exact sibling symlink; run and hash frozen Cargo metadata; retain all existing source hash, M/A/A/D, test, compile, Clippy, rustfmt, clean-tree, and ref-drift gates; write dynamic evidence fields; use normal non-force push only.
8. Update R5 README/STATE/MANIFEST/SHA256SUMS honestly. `STATE.json` must identify R5, keep `source_patch_unchanged=true`, set `exact_vendor_package_count_enforced=false`, and record the canonical R4 archive hash.
9. Run all acceptance and implementation tests until GREEN without changing tests to fit an incorrect implementation.
10. Rehearse the complete transaction against an isolated clone and local bare remote using the actual vendor and Rust/Cargo 1.94.1.
11. Deterministically package R5, extract fresh, verify all internal hashes and shell syntax, and rerun acceptance.
12. Run R5 against the actual GitHub refs with the workdir preserved.
13. Independently verify final head/tree, unchanged `main`, ordinary fast-forward, exact four-file M/A/A/D diff, exact Rust hashes, recovery-marker deletion, open/draft/unmerged PR, and zero wall campaigns.
14. Write `COMPLETION_EVIDENCE.json` conforming to `templates/COMPLETION_EVIDENCE_SCHEMA.json` and run:

```bash
python3 acceptance/validate_completion_evidence.py \
  --evidence COMPLETION_EVIDENCE.json
```

15. Produce the evidence bundle including the schema, example, validator logs, every referenced command log/receipt, and SHA-256 values.
16. Stop. Do not merge and do not start Task 2.

## Commands required in evidence

```bash
python3 -m unittest acceptance.test_handoff_completeness_contract -v
python3 -m unittest acceptance.test_completion_evidence_contract -v
python3 acceptance/test_completion_evidence_schema_contract.py --schema templates/COMPLETION_EVIDENCE_SCHEMA.json --instance templates/COMPLETION_EVIDENCE_EXAMPLE.json
sha256sum -c ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256
python3 acceptance/test_archive_authority_contract.py --archive ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz --sidecar ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256
python3 -m unittest test_validate_vendor_source.py -v
PM4_R5_DIR="$R5_DIR" python3 -m unittest acceptance.test_vendor_validator_contract -v
python3 acceptance/test_publication_script_contract.py --r5-dir "$R5_DIR"
cargo metadata --frozen --format-version 1
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --offline --locked
cargo test -p rodas5p-integrators --all-targets --no-run --offline --locked
TERM=dumb cargo clippy -p rodas5p-integrators --all-targets --offline --locked -- -D warnings
cargo fmt --all -- --check
git diff --check
git status --porcelain
python3 acceptance/validate_completion_evidence.py --evidence COMPLETION_EVIDENCE.json
```

## Failure policy

On any failed exact-ref, archive/source-hash, handoff-completeness, file-surface, Cargo-resolution, test, or remote-state gate, print:

```text
BLOCKED_BY_UNRESOLVED_SPEC
```

Stop without push, preserve workdir/logs, and report the exact command, exit code, stdout/stderr, and unchanged remote refs. A blocked or partial run must not emit a success `COMPLETION_EVIDENCE.json`.

## Final response format

1. exact base/previous/final SHAs and tree;
2. changed paths/statuses;
3. root cause and permanent guardrails;
4. RED/GREEN evidence;
5. vendor-validation and Cargo-metadata evidence;
6. all required verification commands/results;
7. R5 archive/manifest hashes;
8. publication and remote-verification receipts;
9. P0/P1/P2/P3 ledger;
10. unresolved blockers;
11. explicit statement: no merge, no wall timing, no Task 2.
