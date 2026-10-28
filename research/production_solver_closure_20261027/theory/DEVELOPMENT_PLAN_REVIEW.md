# Local development/handoff integration review

Reviewer: `/root/production_adapter`; read-only integration review requested by
the parent. No native code, native test, benchmark, source manifest or evidence
archive was changed. The only written artifact is this review.

Reviewed snapshot:

| File | SHA256 |
|---|---|
| DEVELOPMENT_PLAN.json | d1eb53ce7e41a5a1cbb9c35b1d9c8301d48d4065495d7194f14e58a4ededc225 |
| build_development_plan.py | 186aaba66a5abd5edbaecd97b991901dd1cf010bfa0624821bf172f176b952be |
| run_local.py | 828223c91e9ce8c6c375dc191b9e69547dea5d9439def953b42c85ca6d137f33 |
| run_smoke.py | e07aa9b77db7d79e3babf15ee12e7be9e913386a4448de0ea03065f972ff56a9 |
| verify_package.py | 091a3428e78e53d4e1232badb2b23c902a5d9b957d0d005f88e88b5afe8e8e64 |

The parent was still generating final docs/manifests. Missing final manifests
or documentation at this snapshot are pending work, not failed scientific
validation. `verify_package.py` was therefore not executed prematurely.

## Confirmed working contracts

* `run_local.py` dry run resolves the repository root and target directory
  correctly, emits argv arrays, and does not create the output directory.
  Paths containing spaces remain separate argv entries rather than shell code.
* The current machine DAG has15 unique nodes, all dependencies resolve, and a
  read-only DFS finds no cycle. These are structure checks only.
* The runner explicitly targets POSIX/WSL; its use of `fcntl` and `/tmp` is
  consistent with that stated platform. `run_smoke.py` is documented for
  invocation under the outer native lock, so it does not introduce a nested
  flock deadlock.
* The actual facade already sets `krylov_ls_workspace: true` in `solve.rs`.
  PR-N01 explicitly preserves this existing SPD04 work; it does not propose
  reimplementing missing workspace reuse.
* PR-N01 correctly reuses the existing nonnormal Givens differential failure
  and requires rank/breakdown/true-residual conditions before replacing QR.
  A naive unconditional Givens promotion is not proposed.

## RPLAN-01 — timeout can leave native descendants running (important)

**Target.** `run_local.py` invokes the older
`research/joint_controller_20261025/tools/run_recorded.py` under the lock.
That recorder uses `subprocess.run(command, timeout=...)` without a new process
group/session, and catches `TimeoutExpired` as exit124.

**Evidence and effect.** A timeout terminates the direct child of this call,
not the entire cargo/rustc/linker or Python-helper descendant group. The
outer recorder can return124 and release the lock while a native descendant
continues. A subsequent supposedly isolated benchmark may then overlap with
the unfinished command. This is the same class of cancellation/provenance
problem corrected in the prior research evidence.

**Minimum correction.** Launch commands in a dedicated process group and
terminate/reap the group on timeout or cancellation before releasing the
native lock. Record the timeout and group-cleanup outcome. A focused helper
process that creates a child is sufficient to verify this execution contract;
another solver campaign is unnecessary.

## RPLAN-02 — changed source can still produce a completed suite (important)

**Target.** The reused recorder records
`source_unchanged_during_command=false` but still returns the child's exit0
and labels status `completed`. `run_local.py` checks only that exit code.

**Effect.** A source edit during compilation/timing can therefore yield a
completed local suite whose receipt is internally stale. The before/after
hashes disclose the problem, but the machine-readable caller does not enforce
the stated source-identity contract.

**Minimum correction.** Have the new local runner read the child receipt and
fail or explicitly mark the result stale when source identity changed.
Preserve the original command exit code separately from the qualification
status. Include the toolchain/config/runner inputs needed by the actual
claim, rather than inferring them from a nominal profile name.

## RPLAN-03 — actual local environment and output paths need alignment

`run_local.py` uses `setdefault`, so caller overrides of thread settings are
legitimate. However, the reused recorder omits `RAYON_NUM_THREADS` and
`RUSTDOCFLAGS` from its environment receipt. The plan's value therefore need
not equal the actual process value. Record the effective allow-listed values
and the compiler identity, and record requested affinity/topology when
qualifying hardware timings. Avoid copying the full environment.

The L-RELEASE output contract currently names
`local/release/RECEIPT.json`, `logs/` and `source_manifest.json`, whereas the
suite runner actually emits `SUITE_RECEIPT.json` and
`command_XX/{RECEIPT.json,SOURCE_INPUTS.json,stdout.log,stderr.log}`. Either
normalize these outputs or name the actual paths/globs in the plan. The smoke
child has its own nested `RECEIPT.json` and `SUMMARY.json` as expected.

Standalone `run_smoke.py` hashes six facade/config inputs but not the whole
core/integrator dependency set; the outer recorder snapshots all crate Rust
files. State this distinction, or make the standalone receipt self-sufficient
if it is intended as a separately portable qualification command.

## RPLAN-04 — already-known local benchmark gaps

The reviewed `local-benchmarks` suite builds the structural example but does
not run it. It runs the two adapter arms once in a fixed order, which does not
fulfill the paired ABBA/BAAB acceptance criterion. The parent already identified
both gaps and was adding a structural command and portable paired helper.
Recheck the generated JSON after those additions; no solver rerun is needed
to verify argv construction. Cubic `--bench128` already implements pairing.

## RPLAN-05 — preserve the explicit residence-loss theory task

The adapter's pending task A-L2 in `adapter/LOCAL_TASKS.json` concerns temporal
transport of fitness and bounded residence/exploration loss. The canonical
PR-N01 currently concentrates on Krylov arithmetic and cost calibration;
these address a related but distinct cause. The measured reentrant slowdown
also depends on short-prefix evidence being used through a new regime.

Add the temporal-variation/residence theorem explicitly to a canonical PR
node, or link A-L2 as its own dependent task after cost evidence. Keep the
current-window sentinel bound separate from the future-regime claim. This
preserves a concrete unresolved scientific obligation rather than leaving it
implicit in a generic cost-model title.

PR-N04 bundles two independently implementable scopes: complete DOPRI session
transfer and generic vector homotopy buffer reuse. They have different state
identities and proof obligations. Subtasks or separate PRs would make local
ownership and rollback clearer. Similarly, causal closure theory does not
mathematically depend on DOPRI session transfer; any operational ordering
dependency should be labeled as such rather than blocking independent theory.

## Packaging behavior to document at finalization

`verify_package.py` checks source hashes and, when present, archive membership
and member hashes. An absent RAW_MANIFEST currently skips archive verification
and still reports a scoped pass with zero members. At publication, either
require the expected raw manifest or report this as source-only verification
so a missing evidence package cannot be read as a full package pass. The
current final artifact generation is still pending, so this is a shipped
verifier contract to clarify rather than evidence of an existing missing file.

The plan builder hardcodes current statuses. If finalization changes the
generated JSON's completion states, update the builder or explicitly separate
the static plan from the evolving run state so regeneration does not silently
reset finalized status. The reviewed snapshot itself was structurally valid.

## Disposition

Conditional acceptance of the planning structure. RPLAN-01 and RPLAN-02 are
the concrete execution/qualification issues to close before calling the
local runner production-ready. Remaining items align the handoff with actual
paths, existing evidence, and independent scientific obligations. This review
adds no native numerical correction pass to any of the three research lanes.
