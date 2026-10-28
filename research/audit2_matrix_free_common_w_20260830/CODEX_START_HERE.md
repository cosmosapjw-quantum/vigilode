# Codex execution prompt — publish and exact-base verify MATRIX_FREE_COMMON_W_SUBSTRATE

Operate as HOST_CODEX_ONLY. Do not use a local model for code, review,
classification, claim admission, or Git mutation.

## 1. Bind authority before mutation

Repository: `https://github.com/cosmosapjw-quantum/vigilode`

Required stacked base:

```text
branch  research/audit2-original-target-bridge-20260830
head    c5fbd6d5703fc396bdf30eb3acfacb6c6bd2b921
tree    a0fec46f857f00054d674fb417812065aeca8a31
PR      #31 OPEN / DRAFT / UNMERGED
```

Abort without mutation if the branch head differs. Do not silently rebase the
scientific meaning. Record a blocker instead.

Create an isolated clean worktree and branch:

```text
research/audit2-matrix-free-common-w-20260830
```

Base it directly on the exact c5 head. Preserve all unrelated untracked files
and worktrees.

## 2. Apply the supplied change

Apply `SOURCE_PATCH_FROM_C5.patch` with ordinary `git apply --check` followed by
`git apply`. Do not use `--3way`, fuzzy reconstruction, content paraphrase, or
manual regeneration unless the exact-base gate first proves a packaging-only
line-offset failure. The patch is expected to touch only:

```text
crates/rodas5p-integrators/src/lib.rs
crates/rodas5p-integrators/src/audit2_matrix_free_research.rs
crates/rodas5p-integrators/tests/audit2_matrix_free_common_w_contracts.rs
tools/check-audit2-readiness.sh
tools/test_a1_receipt_ci_scope.py
research/audit2_matrix_free_common_w_20260830/**
```

Confirm no diff under the seven PR31 paths, `fixtures/`,
`research/scientific_validity_v2_20260829/`, `crates/rodas5p-core/`, or
`Cargo.lock`.

## 3. Mandatory exact-base verification

Run and retain complete logs:

```bash
cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_matrix_free_common_w_contracts -- --nocapture --test-threads=1

cargo test --locked -p rodas5p-integrators --features audit2-research \
  --test audit2_structured_correction_contracts -- --nocapture --test-threads=1

bash tools/check-audit2-readiness.sh

cargo clippy --locked -p rodas5p-integrators -p rodas5p-fair-ab \
  --all-targets --features rodas5p-integrators/audit2-research -- -D warnings

cargo fmt --all -- --check
git diff --check
```

Expected structural facts, not hard-coded result promotion:

- new suite: 6 tests;
- PR31 bridge suite: 16 tests;
- matrix-free candidate uses no explicit W or direct factorization;
- one session setup/workspace serves all eight correction solves;
- no recycle/basis-reuse counters are claimed;
- malformed late RHS retains one completed row and spent work;
- the coupling-sign mutation remains detectable.

If exact c5 numerical values differ, preserve the actual result. Do not widen
or fit a tolerance. A failing fixed criterion is a real result.

## 4. Fresh review

After tests, run exactly one fresh-context read-only review of the final diff.
Review at minimum:

- candidate truly uses the unchanged matrix-free shifted operator;
- same `(operator token, h*gamma)` session identity is preserved;
- workspace reuse is not mislabeled as Krylov-basis reuse;
- all setup/solve attempts, completions, and partial results survive failure;
- projected block-forward coupling signs and JVP state points match the existing
  explicit Audit-2 implementation;
- no production/default dispatcher calls the module;
- claim ledger does not imply scalability, speed, original-target accuracy, or
  a production preconditioner.

One bounded repair round is permitted only for concrete P0/P1 findings. Do not
create a review-of-review loop. Re-run all affected tests after repair.

## 5. Commit and publish

Commit only after fresh verification. Use a non-force push. Open a **Draft
stacked PR** whose base is
`research/audit2-original-target-bridge-20260830`. Do not merge the scientific
PR.

For the user-requested import/push/PR/merge connectivity probe, create two
throwaway branches from c5, change only a clearly labelled non-scientific marker,
open a PR between those throwaway branches, let its minimal check run, and merge
only that disposable PR. Delete neither scientific evidence nor existing
branches. Explicitly record that the mock merge is not scientific evidence.

Append actual head/tree, exact checks, failed-history entries, and claim ceiling
to PR31, the new PR, Jira PM-1, and Confluence page 9732097. Do not alter PM-7,
K0, the existing DAG status, holdouts, tags, or releases.

## 6. Stop boundary

Stop after publication/readback. The next node is
`REUSABLE_PRECONDITIONER_AND_TRANSACTIONAL_STEP`; do not begin it in the same
run. Missing an observable budget blocks accuracy admission, not this bounded
research execution.
