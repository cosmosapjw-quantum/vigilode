# Node h-hygiene: test isolation, ledger correction row, workspace membership

Branch `production/h-hygiene` from a5a48f7. Commits: 085158c (tests), d248e68 (ledger L-0138), ac0bada (workspace).

## 1. Allocation-counter tests under parallel threads

Root causes differ per binary.
- `production_audit_pr101_provider.rs`: every test already took a mutex, but the counter was process-wide and the harness
  threads (reporting, output capture) allocate without the lock. Counter is now thread-local (`const` init, no destructor, so
  the allocator never allocates or registers a dtor).
- `collective_admission_contracts.rs`: same pattern; `TRACK`, `ALLOCATIONS`, `ALLOCATED_BYTES` are now thread-local.
- `leja_setup_work_contract.rs`: no allocator. The test `counted_action_records_cold_coefficient_table_...` asserted that the
  three process-wide `OnceLock` Leja tables have history, but a sibling test calls the uncounted `leja_points()` and may
  initialize a table first (observation `PreexistingUnattributed`, no history). The assertion is now: history is available
  exactly when the observation is not `PreexistingUnattributed`. No production code changed.

Before: 8 default-harness runs per binary gave 4 failures in pr101 (of 8), 1 in leja_setup (of 8), 0 in collective.
After: 20 consecutive default-harness runs per binary, 60/60 ok (5/7/9 tests); `--test-threads=1` also ok.

## 2. Ledger row L-0138

Supersedes L-0124 (INCONCLUSIVE; claim: hashes re-bound after post-run edits). Outputs: current sha256 of
`CONTINUATION_DAG.json` (51504667...) and `CONTINUATION_FINDINGS.json` (f1e3a966...). Last-changing commits:
DAG 178e18ab1e270b4743a4d1c45b6541fc9d0e7848, FINDINGS c513f478f952068b91e3da368fb2cb156d942b28. The row cites
178e18ab (the later; it holds both files in their current bytes, verified by `git show`), reachable from
`origin/production/vigilode-plan-20261021` and `origin/research/cumulative-evidence-20261020`. The `command` is
`UNRECORDED: ...` (allowed for INCONCLUSIVE); `inputs_sha256` is empty.

`python3 tools/check-research-node.py --base origin/production/vigilode-plan-20261021`: exit 1 before (137 rows) and exit 1
after (138 rows). The same two L-0124 mismatch errors remain: `check_outputs` verifies every row, including superseded ones,
against current bytes, so a correction row cannot clear them. The fix is a small checker change (skip the digest check for
a path that a superseding row re-binds). That edit was blocked by the sandbox classifier as audit-tooling modification and was
not made or worked around; it needs the maintainer's decision. The checker is unchanged.

## 3. Workspace membership

`crates/vigilode-coupled` joined `[workspace] members`. Removed its `[workspace]`, `Cargo.lock` and `[profile.measurement]`;
`serde`/`serde_json` use `workspace = true`. Root `Cargo.lock` diff: +10 lines, only the `vigilode-coupled` package entry.
`cargo test -p vigilode-coupled --locked`: 37 passed (5+2+2+4+10+12+2). fmt clean for the crate; clippy shows only existing warnings.
Side effect: the root `measurement` profile (codegen-units 16) now applies instead of the crate's (codegen-units 1); the README
build commands use `-p vigilode-coupled` and `target/measurement/...`. `vigilode-hybrid` stays standalone.

## 4. coupled-*.yml workflows

All five (`coupled-native`, `coupled-closeout`, `coupled-economy`, `coupled-economy-closeout`, `coupled-reference-recovery`)
left in place. They are research-run workflows triggered on `research/coupled-*` branches that execute benchmarks and commit
preserved results; none exists only to work around the crate being outside the workspace. `coupled-native`'s `run_ci.sh` and the
reference-recovery workflow use `--manifest-path crates/vigilode-coupled/...` and `crates/vigilode-coupled/target/...` paths that
now resolve to the root target dir; they are frozen research-branch artefacts and were not edited.
