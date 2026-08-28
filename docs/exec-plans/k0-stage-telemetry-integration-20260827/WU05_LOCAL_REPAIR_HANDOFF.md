# VigilODE K0 WU-05 local repair handoff — controlling additive supplement

This handoff contains only work that must be performed in the preserved local implementation worktree. Package/control bytes are maintained on the GitHub documentation branch and are not to be edited locally.

## Preserved local state

```text
branch        research/k0-stage-telemetry-integration-20260827
fresh review  e95ce1e58a603306cb665a6ab91cfe02d279972f
tree          e3621a370297a76907e97730ebd18c5c1e0fb83e
worktree      clean
remote        implementation branch still at the pre-WU prepared commit
```

Do not reset, rebase, squash, amend, cherry-pick over, or replace WU-00 through WU-04 or the fresh-review commit.

## External package pin

A commit cannot contain its own SHA. The orchestrator must copy the exact current package SHA from the final publication receipt, PR #21, Jira PM-7, Confluence page `15499267`, or the assistant handback and export it:

```bash
export K0_PACKAGE_SHA=<EXACT_FINAL_40_HEX_PACKAGE_SHA>
```

A moving branch ref is discovery only, never authority.

The validator payload is transparent: `python tools/verify-k0-wu05-supplement.py --dump-source` emits the exact source bound by the supplement manifest.

## Orchestrator-owned merge

```bash
set -euo pipefail
cd /tmp/vigilode-k0-stage-telemetry.kAguIL/tree

BRANCH=research/k0-stage-telemetry-integration-20260827
REVIEW=e95ce1e58a603306cb665a6ab91cfe02d279972f
REVIEW_TREE=e3621a370297a76907e97730ebd18c5c1e0fb83e
PACKAGE_REF=origin/docs/k0-codex-execution-package-20260827

test "${K0_PACKAGE_SHA:-}" != ""
test "${#K0_PACKAGE_SHA}" = 40
test "$(git branch --show-current)" = "$BRANCH"
test "$(git rev-parse HEAD)" = "$REVIEW"
test "$(git rev-parse HEAD^{tree})" = "$REVIEW_TREE"
test -z "$(git status --porcelain=v1)"

git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse "$PACKAGE_REF")" = "$K0_PACKAGE_SHA"
git cat-file -e "$K0_PACKAGE_SHA^{commit}"

git merge --no-ff --no-edit "$K0_PACKAGE_SHA"

test "$(git rev-parse HEAD^1)" = "$REVIEW"
test "$(git rev-parse HEAD^2)" = "$K0_PACKAGE_SHA"
test -z "$(git status --porcelain=v1)"

python tools/verify-k0-stage-telemetry-plan.py \
  --repo-root . \
  --check-package

python tools/verify-k0-wu05-supplement.py \
  --repo-root . \
  --expected-package-sha "$K0_PACKAGE_SHA" \
  --check-supplement-manifest \
  --check-authority \
  --check-repair-merge \
  --self-test
```

Required pre-repair markers:

```text
PACKAGE_CONTRACT_PASS
WU05_SUPPLEMENT_MANIFEST_PASS
LEGACY_REPAIR_BLOBS_PASS
EXTERNAL_PACKAGE_PIN_PASS
WU05_SUPPLEMENT_AUTHORITY_PASS
WU05_REPAIR_MERGE_PASS
HOSTILE_FIXTURES_PASS
```

A merge conflict or marker failure is not permission to guess. Abort the merge when applicable and report `BLOCKED_BY_AUTHORITY_DRIFT`.

## Bounded local repair

1. Preserve the five exact reproducers in `fresh_review_findings.yaml`.
2. Add RED coverage only for the stronger supplement detectors not already reproduced.
3. Implement exactly the two source-audited K0 bridge modules in `PUBLIC_BRIDGE_CONTRACT_V2.md`.
4. Convert aggregate exceptions into structured **cell-v3** `ERROR`/`STOP_INVALID` receipts with the actual partial-stage array, count, and canonical digest.
5. Preserve all twelve raw WU-04 receipts byte-for-byte.
6. Generate **v3** wrappers mechanically from raw cells according to `EVIDENCE_V3_CANONICALIZATION.json`; never type campaign fields, hard gates, or digests by hand.
7. Execute—not merely name—the aggregate-error serialization and signed-residual mutation tests.
8. Run source-derived bridge validation and evidence-v3 validation.
9. Run the existing single read-only fresh repair review using `FRESH_REPAIR_SUPPLEMENT_REVIEW_PROMPT.md`, then the existing final differential audit. Do not add another review layer.
10. Push/open one draft stacked implementation PR only when the five original findings and all supplement findings are closed with P0=0 and P1=0.

## Required local verification

```bash
python tools/verify-k0-wu05-supplement.py \
  --repo-root . \
  --check-public-bridge

python tools/verify-k0-wu05-supplement.py \
  --repo-root . \
  --evidence-dir research/k0_stage_telemetry_20260827/evidence_v3/cells

python tools/verify-k0-wu05-supplement.py \
  --repo-root . \
  --execute-aggregate-error-guard \
  --execute-signed-residual-guard

cargo test -p rodas5p-integrators --test k0_stage_telemetry_contracts
cargo test -p rodas5p-integrators --test a1_two_arm_receipt_contracts
cargo test --workspace --locked
```

Required closure markers:

```text
PUBLIC_BRIDGE_SOURCE_PASS
EVIDENCE_V3_PASS
AGGREGATE_ERROR_GUARD_PASS
SIGNED_RESIDUAL_GUARD_PASS
```

The four pre-existing Clippy P3 lints remain nonblocking. New lints in changed lines must be reported, but this node does not authorize broad cleanup.

## Forbidden changes

No Cargo graph change, production signature/route change, tolerance change, equation change, convergence-authority change, output change, raw campaign substitution, timing/speedup claim, BDF ranking, homotopy-certificate change, tag, release, or merge.
