# WU-05 fresh-review repair handoff

## Preserved implementation authority

```text
branch      research/k0-stage-telemetry-integration-20260827
WU-00       183e24feb39fd7581450ae4380bd8afe09249451
WU-01       faa759de5c54848bb60d4cb8af4b06b6bcbbe514
WU-02       2badcec35b51d23fcd2938d1e15c9e0875a0f9df
WU-03       321c63ee8ca0f216001bf41b30d58c1858a4781a
WU-04       c7a5393a2cb1cf6f6095c6390348dd21fb45efe9
fresh review e95ce1e58a603306cb665a6ab91cfe02d279972f
tree        e3621a370297a76907e97730ebd18c5c1e0fb83e
worktree    clean
remote      still at pre-WU prepared commit; do not reset local work
```

WU-03 and WU-04 are completed evidence, not work to discard. WU-05 remains blocked until the package repair is merged and all five findings are closed.

## Orchestrator-owned package merge

Codex must not create or merge branches. On the preserved clean worktree, the orchestrator performs:

```bash
set -euo pipefail
cd <preserved-k0-worktree>

REVIEW=e95ce1e58a603306cb665a6ab91cfe02d279972f
BRANCH=research/k0-stage-telemetry-integration-20260827
PACKAGE_REF=origin/docs/k0-codex-execution-package-20260827

test "$(git branch --show-current)" = "$BRANCH"
test "$(git rev-parse HEAD)" = "$REVIEW"
test -z "$(git status --porcelain=v1)"

git fetch --prune origin docs/k0-codex-execution-package-20260827
git merge --no-ff --no-edit "$PACKAGE_REF"

test "$(git rev-parse HEAD^1)" = "$REVIEW"
test "$(git rev-parse HEAD^2)" = "$(git rev-parse "$PACKAGE_REF")"
test -z "$(git status --porcelain=v1)"

python tools/verify-k0-stage-telemetry-plan.py \
  --repo-root . \
  --check-package
python tools/verify-k0-fresh-review-repair.py \
  --repo-root . \
  --check-authority \
  --check-repair-merge \
  --self-test
```

Required markers:

```text
PACKAGE_CONTRACT_PASS
FRESH_REVIEW_REPAIR_AUTHORITY_PASS
FRESH_REVIEW_REPAIR_MERGE_PASS
HOSTILE_FIXTURES_PASS
```

A merge conflict is not permission to guess. Abort and report `BLOCKED_BY_AUTHORITY_DRIFT`.

## Repair order

1. Reproduce the five findings from `research/k0_stage_telemetry_20260827/review/fresh_review_findings.yaml` without modifying code.
2. Add the exact RED tests named in `FRESH_REVIEW_REPAIR_AUTHORITY.json`.
3. Implement only the documentation-hidden K0 bridge authorized by `PUBLIC_BRIDGE_CONTRACT.md`.
4. Make aggregate failures serialize cell-v2 `ERROR`/`STOP_INVALID` receipts instead of raising or disappearing.
5. Create `evidence_v2/cells/*.json` wrappers that reference immutable WU-04 raw receipts by SHA-256.
6. Do not rerun the 12-cell campaign if raw receipt and numerical payload digests are unchanged. Rerun only if solver semantics or numerical payload bytes change.
7. Add the vector-aware signed residual mutation regression.
8. Run the new validator, targeted tests, affected regression, and one fresh read-only repair review.
9. Run the final differential audit over fresh-review-head..repair-head only.
10. Push/open the draft stacked implementation PR only after P0=P1=0.

## Required repair verification

```bash
python tools/verify-k0-fresh-review-repair.py --repo-root . --check-public-bridge
python tools/verify-k0-fresh-review-repair.py --repo-root . --check-signed-residual-guard
python tools/verify-k0-fresh-review-repair.py \
  --repo-root . \
  --evidence-dir research/k0_stage_telemetry_20260827/evidence_v2/cells
cargo test -p rodas5p-integrators --test k0_stage_telemetry_contracts \
  aggregate_internal_error_is_serialized_not_raised
cargo test -p rodas5p-integrators --test k0_stage_telemetry_contracts \
  signed_residual_mutation_is_detected
cargo test -p rodas5p-integrators --test k0_stage_telemetry_contracts
cargo test -p rodas5p-integrators --test a1_two_arm_receipt_contracts
cargo test --workspace --locked
```

The four pre-existing Clippy P3 findings remain nonblocking unless the repair introduces a new lint in changed lines. Do not broaden this repair into cleanup.

## Prohibited scope

No tolerance, family, equation, convergence, routing, production default, 12-cell numerical output, timing, speedup, BDF ranking, homotopy certificate, tag, release, merge, or historical receipt mutation is authorized.
