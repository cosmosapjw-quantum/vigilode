## Task 4: H-hygiene — test isolation, ledger tombstone, workspace membership (node id `h-hygiene`)

Small independent items, one subagent:
1. The three allocation-counter tests that fail under parallel test threads
   (`rodas5p-core/tests/leja_setup_work_contract.rs`,
   `rodas5p-integrators/tests/collective_admission_contracts.rs`,
   `rodas5p-integrators/tests/production_audit_pr101_provider.rs`): make each measured region exclusive
   (a process-wide test mutex around every allocation-counting test in the same binary, or run the counting
   section on a dedicated thread with a thread-local counter — choose the smallest change that makes the
   tests deterministic under the default harness; show 5 consecutive parallel runs passing).
2. Ledger: append a correction row with id `L-0138` to `research/LEDGER.jsonl` that supersedes L-0124 with the current
   `outputs_sha256` of `research/production_audit_20261018/CONTINUATION_DAG.json` and
   `CONTINUATION_FINDINGS.json` (verdict INCONCLUSIVE, claim: hashes re-bound after post-run edits; cite the
   commit that last changed each file), so `tools/check-research-node.py --base <wave2 base>` exits 0; record the exit code.
3. Add `crates/vigilode-coupled` to the root workspace members (remove its own `[workspace]` table and
   `Cargo.lock`, take its dependency versions from `[workspace.dependencies]`); the root `Cargo.lock`
   diff must add only the `vigilode-coupled` package entry (its deps are already in the root lock — verify);
   its 37 tests must pass from the root with `cargo test -p vigilode-coupled --locked`. Leave
   `vigilode-hybrid` standalone (its optional `diffsol` dependency would enter the root lock).
4. Remove the five `coupled-*.yml` workflow steps that existed only because the crate was outside the
   workspace only if they do nothing else; otherwise leave them and say why.

