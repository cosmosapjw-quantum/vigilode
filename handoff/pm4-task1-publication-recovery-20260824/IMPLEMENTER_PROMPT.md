# Codex prompt — PM-4 Task-1 R5 vendor closure

You are the implementation agent. Work autonomously. Do not ask the user questions, and do not guess across a specification boundary.

## Repository and handoff

Repository:

```text
https://github.com/cosmosapjw-quantum/vigilode
```

Read the handoff branch without merging it:

```text
handoff/pm4-task1-publication-recovery-20260824
```

Read in order:

```text
handoff/pm4-task1-publication-recovery-20260824/AGENTS.md
handoff/pm4-task1-publication-recovery-20260824/README.md
handoff/pm4-task1-publication-recovery-20260824/CURRENT_STATE.json
handoff/pm4-task1-publication-recovery-20260824/AUDIT_COMPILED_EXEC_PLAN.yaml
handoff/pm4-task1-publication-recovery-20260824/P0_P1_THREAT_CATALOG.yaml
handoff/pm4-task1-publication-recovery-20260824/INVARIANT_TEST_MATRIX.yaml
handoff/pm4-task1-publication-recovery-20260824/EVIDENCE_CHAIN.md
handoff/pm4-task1-publication-recovery-20260824/acceptance/README.md
```

Treat `AUDIT_COMPILED_EXEC_PLAN.yaml` as the execution contract.

## Primary task

Fix the PM-4 Task-1 publication transaction by replacing the R4 hard-coded `262` Cargo-vendor package-count gate with a structurally checked, Cargo-resolved offline closure gate. Create a deterministic R5 publication kit, run it on the authenticated host against:

```text
/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
```

and publish the unchanged Task-1 source patch to draft PR #11 by one ordinary non-force fast-forward. Stop before merge and before PM-4 Task 2.

## Exact authority

```text
canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

expected feature head before publication
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

target feature branch
research/v38d-exploratory-benchmark-substrate

PR
#11 — must remain OPEN / DRAFT / UNMERGED

R4 input archive
~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz

Task-1 patch SHA-256
705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3

R4-observed immediate directories
308
```

The value 308 is failure evidence, not a required package count. Determine Cargo package directories using Cargo directory-source semantics: inspect one immediate level, ignore dot-prefixed directories, and treat only directories containing `Cargo.toml` as package candidates. Every such candidate must contain a parseable `.cargo-checksum.json`. Manifestless and dot-prefixed directories must not alter `package_directory_count`.

## Forbidden operations

- Do not modify `main`.
- Do not force-push or merge.
- Do not change `.cargo/config.toml`, `Cargo.lock`, dependency versions, or any byte of the Task-1 patch.
- Do not run real wall timing, rank candidates, or start Task 2.
- Do not weaken tests to fit an implementation.
- Do not report success after partial verification.

## Required method

1. Create an isolated worktree or fresh clone. Verify the exact main and feature refs above.
2. Reproduce the R4 failure `308 (expected 262)` and retain stdout/stderr as RED evidence.
3. Extract the immutable R4 archive into a new R5 working directory. Do not edit the original R4 directory or archive.
4. Add `validate_vendor_source.py` with both a Python API and CLI:

```bash
python3 validate_vendor_source.py \
  --vendor-dir PATH \
  --require-package faer-0.24.4 \
  --json-out VENDOR_VALIDATION.json
```

Required deterministic JSON fields:

```text
schema = vigilode-cargo-directory-source-validation-v1
vendor_dir = canonical absolute path
immediate_directory_count = observed integer
package_directory_count = non-hidden immediate directories containing Cargo.toml
checksum_record_count = checksum records among package directories
ignored_hidden_directory_count = observed integer
ignored_manifestless_directory_count = observed integer
required_packages_present = sorted list
missing_checksum_packages = sorted list
exact_package_count_enforced = false
```

5. Write tests before the fix and preserve RED/GREEN logs. Required cases:
   - structurally valid 262 package directories, each with `Cargo.toml` and `.cargo-checksum.json`: PASS;
   - structurally valid 308 package directories: PASS;
   - dot-prefixed and manifestless incidental directories do not alter package count: PASS;
   - one manifest-bearing package directory missing `.cargo-checksum.json`: FAIL;
   - missing `faer-0.24.4`: FAIL;
   - repeated validation emits identical JSON: PASS.
6. Modify `publish_pm4_task1.sh` minimally:
   - remove exact global package-count enforcement;
   - call the helper and save `VENDOR_VALIDATION.json`;
   - create the exact sibling symlink required by checked-in Cargo config;
   - verify `readlink -f` resolves to the provided vendor;
   - run `cargo metadata --frozen --format-version 1 > "$WORK/cargo-metadata.json"` before tests and before push;
   - hash `cargo-metadata.json`;
   - retain all existing archive/hash, exact M/A/A/D surface, focused tests, all-target compile, Clippy, rustfmt, clean-tree, and ref-drift gates;
   - put observed directory/package/checksum counts and metadata hash into the receipt dynamically;
   - use only ordinary non-force push.
7. Update R5 `README.md`, `STATE.json`, `MANIFEST.json`, and `SHA256SUMS` honestly. `STATE.json` must contain:

```json
{
  "schema": "vigilode-pm4-task1-publication-kit-r5",
  "source_patch_unchanged": true,
  "exact_vendor_package_count_enforced": false
}
```

8. Run these commands and retain complete logs and exit codes:

```bash
python3 -m unittest test_validate_vendor_source.py -v
cargo metadata --frozen --format-version 1
cargo test -p rodas5p-integrators --test v38d_performance_probe_contracts --offline --locked
cargo test -p rodas5p-integrators --all-targets --no-run --offline --locked
TERM=dumb cargo clippy -p rodas5p-integrators --all-targets --offline --locked -- -D warnings
cargo fmt --all -- --check
git diff --check
git status --porcelain
```

9. Rehearse the complete R5 transaction against an isolated clone and local bare remote using the same actual vendor path and Rust/Cargo 1.94.1.
10. Deterministically package R5, extract it fresh, verify all internal hashes and `bash -n`, and rerun the helper and script-contract tests.
11. Run R5 against the real GitHub refs with `--keep-workdir`.
12. Independently verify after push:
    - `main` unchanged;
    - feature is a normal fast-forward from `b2d5ec41...`;
    - exact four-file Task-1 M/A/A/D diff;
    - three Rust file SHA-256 values match the sealed list;
    - recovery marker absent from proposed tree;
    - PR #11 remains open/draft/unmerged;
    - zero wall campaigns.
13. Produce a completion evidence bundle containing all SHAs, name-status, hashes, vendor JSON, Cargo metadata hash, logs, publication receipt, remote verification, and unresolved blockers.
14. Stop. Do not merge and do not start Task 2.

## Fail-closed policy

If any exact ref, source hash, file surface, Cargo resolution, test, or remote-state condition fails, print:

```text
BLOCKED_BY_UNRESOLVED_SPEC
```

Then stop without push, preserve the workdir and logs, and report the exact command, exit code, stdout/stderr, and unchanged remote refs.

## Final response format

1. exact main/previous-feature/final-feature SHAs and final tree;
2. exact changed files and statuses;
3. root cause and permanent guardrail;
4. RED/GREEN evidence;
5. vendor-validation and Cargo-metadata evidence;
6. focused/full-relevant verification results;
7. publication and remote-verification receipts;
8. P0/P1 ledger;
9. unresolved blockers;
10. explicit statement: no merge, no wall timing, no Task 2.
