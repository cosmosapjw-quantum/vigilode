# WU-05 preparation and local repair — bootstrap v2, revision 2

## A. Authority and Scope

GitHub source and exact commit pins are authoritative. The supplied latest local report preserves `e95ce1e58a603306cb665a6ab91cfe02d279972f`, tree `e3621a370297a76907e97730ebd18c5c1e0fb83e`, on `research/k0-stage-telemetry-integration-20260827`. This is user-reported local evidence, not remotely replayed evidence.

The inspected package baseline is `13aed8dabfbb5da4381d9d73d3cb0c0403ad5354`. Use the NEW exact SHA in the external publication receipt, not that historical baseline, as `K0_PACKAGE_SHA`.

This revision repairs preparation only. It preserves WU-00–04, the raw 12-cell data, equations, tolerances, convergence authority, routing, and the semi-Jacobian-free homotopy lane. Five source-review findings remain for local WU-05 repair; package tests do not close them.

**Host Codex is explicitly authorized to act as HOST_CODEX_ORCHESTRATOR for the pinned script below.** It need not wait for a separate human/orchestrator to merge. Manual Git repair, reset, stash, rebase, and conflict resolution remain prohibited. Switch to implementer role only after readiness.

## B. P0/P1 Threat Catalogue

| Risk | Primary detector |
| --- | --- |
| BR-ENTRY: package helper invoked before it exists | absent-helper end-to-end Git test |
| BR-MARKER: exit zero mistaken for authority | required structured-PASS-marker test |
| BR-SCOPE: source changes hidden in package overlay | non-control-delta rejection test |
| BR-REENTRY: completed preparation cannot be retried | exact-two-parent idempotence test |

All four are P1 preparation/completeness failures here, not new numerical findings. Existing original P0/P1 findings keep their prior classification.

## C. Invariant/Test Matrix

Preparation must preserve the review commit and tree; use only the exact published package; reject missing helper files before merge; require exit status and marker coverage; preserve source bytes; abort a conflicted merge; retain logs; and permit a no-new-commit retry of the exact completed merge.

The single detector suite is `python3 -B tools/test_k0_bootstrap_v2.py`. It uses real Git operations with synthetic histories and explicitly substituted dependency-validator fixtures. It does not represent the user's local commits, production tests, or campaign replay.

## D. Ordered Work Units

### PREPARE — deterministic host orchestration

Obtain `K0_PACKAGE_SHA` from the external publication receipt or paste-ready delivered prompt. Inspect the known worktree. If its path moved, use `git -C ~/vigilode worktree list --porcelain` to locate the exact branch/head; do not create a replacement or lose unpushed commits.

Run from an ordinary shell or host Codex terminal:

```bash
set -euo pipefail
: "${K0_PACKAGE_SHA:?exact publication pin required}"
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test -z "$(git status --porcelain=v1)"
git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$K0_PACKAGE_SHA"
BOOT=$(mktemp "${TMPDIR:-/tmp}/k0-bootstrap-entry.XXXXXX")
trap 'rm -f -- "$BOOT"' EXIT
git show "$K0_PACKAGE_SHA:tools/k0-wu05-bootstrap-v2.sh" > "$BOOT"
bash "$BOOT" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

Do not replace this sequence with `python tools/verify-k0-fresh-review-repair.py` in the old tree. Its absence before merge is expected, not a new solver defect.

The runner validates dependencies in its own detached worktree. It then creates an ordered `[review, package]` merge, or revalidates exactly that already-present merge. Exit zero without required markers never permits merge. A post-merge failure preserves that merge; no automatic reset occurs.

The runner emits `LOCAL_WU05_AUTHORITY_READY` only after all existing package/supplement markers pass. Its final line gives `BOOTSTRAP_RECEIPT=...`. Command output, exit codes, and hashes are retained under the Git common directory's `k0-bootstrap/` directory. They are not tracked source modifications.

### REPAIR — existing WU-05 only

After readiness, read `WU05_LOCAL_CODEX_PROMPT.md`, `WU05_LOCAL_REPAIR_SUPPLEMENT.json`, `PUBLIC_BRIDGE_CONTRACT_V2.md`, evidence-v3 canonicalization/schemas, and the preserved `fresh_review_findings.yaml`. Reproduce and repair the five original findings under those existing contracts. Use the current bootstrap role exception only for preparation; do not edit frozen package files locally.

Preserve raw WU-04 bytes. Generate v3 wrappers mechanically where supported by the raw data; never invent missing historical fields. Reuse evidence only with an explicit current-source binding decision: unchanged historical outputs alone do not prove a changed solver. A rerun needs a distinct changed-behavior reason, not a changed packaging SHA.

## E. Fresh-Context Review Contract

Reuse one read-only fresh repair review. Inputs are exact implementation base/final SHA, diff, controlling contract, actual logs, existing findings, and unresolved boundaries. Do not give the implementer's reasoning as authority. Diagnose before repairing. Require no unresolved P0/P1; retain P2/P3 without inflating their severity.

## F. Final Differential Audit Contract

Audit only the repaired delta and evidence correspondence. Do not restart all historical science or add a review-of-review. A draft stacked implementation PR may be published only under the existing closure conditions; do not merge PR #20/#21 or activate a solver.

## G. Unresolved Specification Boundaries

No bootstrap definition is intentionally left to the local agent. Actual local source, raw data, and five exact reproducers are not available in this runtime and must be inspected locally. Their successful repair is NOT claimed. Newly contradictory raw evidence must be preserved and named rather than fabricated to satisfy a wrapper.

## H. Process-Cost Assessment

This replaces the separate-human-preparation assumption with one host-Codex entry. It reuses existing package and supplement gates, adds no review tier, and does not change receipt schemas. The only added tests falsify concrete bootstrap failures. No Cargo build or 12-cell replay is required for this package-only change.

GitHub commit/tree own bytes; PM-7 owns progress; Confluence 15499267 is a navigation/control mirror. After implementation publication, update/read back all three. An Atlassian outage is `ATLAS_SYNC_PENDING`, not permission to invent synchronization or mark Done.
