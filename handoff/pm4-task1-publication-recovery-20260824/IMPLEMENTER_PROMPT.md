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
5. `AUDIT_COMPILED_EXEC_PLAN.yaml`
6. `P0_P1_THREAT_CATALOG.yaml`
7. `INVARIANT_TEST_MATRIX.yaml`
8. `EVIDENCE_CHAIN.md`
9. `acceptance/README.md`

Treat `AUDIT_COMPILED_EXEC_PLAN.yaml` as the execution contract. Inputs under `inputs/` are immutable witnesses. Do not modify them in place.

## Archive authority gate — run before every other step

The previous Codex run correctly stopped because the original handoff bound an outer archive hash that did not match the authenticated host archive and sidecar. That contradiction is now resolved.

Require exactly:

```text
R4 archive
~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz

canonical outer SHA-256
6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333
```

The prior handoff value:

```text
b33af0b8352aa0b3ccdcc83834cb4696fce787d0733a7e5ce9286e646994a095
```

is withdrawn and must be rejected, not accepted as an alternative.

Before implementing anything:

1. run the archive `.sha256` sidecar check;
2. extract the archive into a fresh directory;
3. run its internal `sha256sum -c SHA256SUMS`;
4. verify embedded Task-1 patch SHA-256:
   `705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3`;
5. verify embedded R4 publication script SHA-256:
   `63c4ae3ca493a6b4ffe03db50a1b1e23850dacf5dcd0f502f594464cbd67ddb7`;
6. run `acceptance/test_archive_authority_contract.py`.

If any check fails, stop with `BLOCKED_BY_UNRESOLVED_SPEC`. Do not choose between hashes.

## Primary task

Fix the PM-4 Task-1 publication transaction by replacing the R4 hard-coded `262` Cargo-vendor count gate with a structurally checked, Cargo-resolved offline closure gate. Produce a deterministic R5 kit, run it on the authenticated host against:

```text
/home/cosmosapjw/Dropbox/rust/bundles/rust-offline-rodas5p-rs-20260806/vendor
```

and publish the unchanged Task-1 source patch to draft PR #11 by one ordinary non-force fast-forward. Stop before merge and before PM-4 Task 2.

## Hard authority and boundaries

```text
canonical main
140f6b5c078c3d8fcd5b6c52310c063ee233dc12

expected feature head before publication
b2d5ec41cb147e01aadbc9c42928da8abfa75c58

target feature branch
research/v38d-exploratory-benchmark-substrate

PR
#11, must remain OPEN / DRAFT / UNMERGED

Task-1 patch SHA-256
705646496b3594adb4f655829dfe2756aca57ce061fef0cae3b080399104f7a3

R4-observed immediate directories
308
```

The value `308` is failure evidence, not a required package count. Determine Cargo package directories using Cargo directory-source semantics: inspect one immediate level, ignore dot-prefixed directories, and treat only directories containing `Cargo.toml` as package candidates. Every package candidate must contain a parseable `.cargo-checksum.json`. Manifestless and dot-prefixed directories must not alter `package_directory_count`.

Forbidden:

- main mutation;
- force push;
- merge;
- changing `.cargo/config.toml`, `Cargo.lock`, dependencies, or the Task-1 patch;
- running wall timing, ranking candidates, or starting Task 2;
- weakening or deleting acceptance tests;
- reporting success after partial verification.

## Required engineering method

1. Create an isolated worktree or fresh clone; verify the exact refs above.
2. Reproduce the R4 `308 (expected 262)` failure and retain the log.
3. Run the package acceptance tests before implementation and retain RED evidence.
4. Copy the R4 inputs into a new R5 working directory.
5. Implement `validate_vendor_source.py` with the exact callable and CLI contract in `acceptance/README.md`.
6. Add `test_validate_vendor_source.py` covering at least:
   - valid 262 package directories, each with `Cargo.toml` and `.cargo-checksum.json`: PASS;
   - valid 308 package directories: PASS;
   - dot-prefixed and manifestless incidental directories do not alter package count: PASS;
   - one manifest-bearing package missing `.cargo-checksum.json`: FAIL;
   - missing `faer-0.24.4`: FAIL;
   - deterministic JSON: PASS.
7. Modify `publish_pm4_task1.sh` minimally:
   - remove exact global package-count enforcement;
   - call the helper and write `VENDOR_VALIDATION.json`;
   - create and verify the exact sibling symlink required by checked-in Cargo config;
   - run `cargo metadata --frozen --format-version 1` before tests and push;
   - hash the metadata JSON;
   - retain all existing source-hash, exact M/A/A/D surface, tests, compilation, Clippy, rustfmt, clean-tree, and ref-drift gates;
   - write dynamic immediate/package/checksum/ignored-directory counts and metadata hash into the receipt;
   - use only normal non-force push.
8. Update R5 README/STATE/MANIFEST/SHA256SUMS honestly. `STATE.json` must set:
   - `schema = vigilode-pm4-task1-publication-kit-r5`;
   - `source_patch_unchanged = true`;
   - `exact_vendor_package_count_enforced = false`;
   - `source_r4_archive_sha256 = 6689544ee9b115fe4cb5c8ba14c179a17ee6615cb454555b0bb2f0ad1826b333`.
9. Run all acceptance tests until GREEN. Do not change tests to fit an incorrect implementation.
10. Rehearse the complete transaction against an isolated clone and a local bare remote using the actual vendor and Rust/Cargo 1.94.1.
11. Deterministically pack R5, extract it into a fresh directory, verify internal SHA-256 values and `bash -n`, then rerun acceptance checks.
12. Run R5 against the actual GitHub refs. Preserve the workdir.
13. Independently verify the remote result:
    - exact new feature head/tree;
    - main unchanged;
    - normal fast-forward relation;
    - exact four-file Task-1 diff and M/A/A/D statuses;
    - exact Rust file SHA-256 values;
    - recovery marker absent from the proposed tree;
    - PR remains open/draft/unmerged;
    - zero wall campaigns.
14. Produce a completion evidence bundle conforming to `templates/COMPLETION_EVIDENCE_SCHEMA.json`.
15. Stop. Do not merge and do not start Task 2.

## Commands that must appear in evidence

```bash
sha256sum -c VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256
python3 acceptance/test_archive_authority_contract.py \
  --archive ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz \
  --sidecar ~/vigilode/VIGILODE_PM4_TASK1_SCHEMA_BOUNDARY_KIT_R4_20260824.tar.gz.sha256
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
```

## Failure policy

If any exact ref, archive/source hash, file surface, Cargo resolution, test, or remote-state condition fails:

```text
BLOCKED_BY_UNRESOLVED_SPEC
```

Then stop without push, preserve the workdir and logs, and report the exact command, exit code, stdout/stderr, and unchanged remote refs.

## Final response format

1. exact base/previous/final SHAs and tree;
2. changed files and statuses;
3. root cause and permanent guardrail;
4. archive-authority, RED/GREEN evidence;
5. vendor-validation and Cargo-metadata evidence;
6. focused/full-relevant verification commands and results;
7. publication receipt and remote verification;
8. P0/P1 ledger;
9. unresolved blockers;
10. explicit statement: no merge, no wall timing, no Task 2.
