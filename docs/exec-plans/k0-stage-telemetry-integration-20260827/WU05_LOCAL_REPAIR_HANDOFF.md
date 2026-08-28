# WU-05 semantic-evidence repair handoff — resume execution, do not add process layers

## Current local authority

The user reports a clean prepared merge:

```text
branch        research/k0-stage-telemetry-integration-20260827
prepared      f6208a10…
prepared tree 19c393ca…
parents       [e95ce1e…, c6ec0121…]
raw WU-04     12/12 byte-identical
```

Treat these as local evidence to verify, not as remote facts to overwrite. Do not reset/rebase/stash/amend/force-update or recreate the worktree.

## What the new package changes

This is one bounded representation false-fail repair. The raw result is not modified.

- no raw top-level `status` required;
- no raw `tolerance_arm` label required: derive LegacyFixed only from `linear_rtol=1e-10`, `linear_atol=1e-12`;
- source head/tree are uniquely derived from the raw outer envelope;
- `error: null` is no error, not `ERROR`;
- historical missing signed-residual digest is kept `null` with campaign `LEGACY_NOT_RECORDED`;
- any non-null backfill absent from raw is rejected;
- numerical digest excludes raw labels, Git/source identity, wrapper and transport metadata;
- raw SHA continues to bind the immutable file as provenance, but SHA/serialization differences are not substituted for scientific comparison.

No new schema version, marker, review tier, or campaign rerun is introduced.

## Host preparation

Use the exact `K0_PACKAGE_SHA` delivered with this handoff. The runner accepts the existing c6-prepared merge as `UPGRADE`, validates the new package in a detached worktree, then creates an ordered `[f620…, K0_PACKAGE_SHA]` merge. Exact successful retry is idempotent.

```bash
set -euo pipefail
: "${K0_PACKAGE_SHA:?exact package SHA required}"
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test -z "$(git status --porcelain=v1)"
git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$K0_PACKAGE_SHA"
BOOT=$(mktemp "${TMPDIR:-/tmp}/k0-semantic-upgrade.XXXXXX")
trap 'rm -f -- "$BOOT"' EXIT
git show "$K0_PACKAGE_SHA:tools/k0-wu05-bootstrap-v2.sh" > "$BOOT"
bash "$BOOT" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

Require `mode: UPGRADE`, all existing package/supplement markers, and `LOCAL_WU05_AUTHORITY_READY`. If local full SHA/tree differ from the reported prefixes, stop as authority drift; do not guess or discard commits.

## Immediate execution after readiness

Do not reopen WU-00–04 and do not add a representation-review cycle. Resume the five original WU-05 findings from their existing RED reproducers.

Generate v3 wrappers from immutable raw 12 cells using the repaired semantic projection. Preserve the twelve raw SHA-256 values in provenance. A changed wrapper/package SHA is not a reason to rerun the campaign. Rerun only if equations, tolerance, routing, convergence decisions, stage work, or numerical payload actually changed.

Then close the public bridge, aggregate ERROR preservation, information-rich failure schema, and current-code signed-residual mutation guard; run targeted/workspace tests, one existing read-only fresh repair review, and the existing differential audit. Publish only a draft stacked implementation PR after P0/P1=0.

Local Qwen output in the prior run is `REJECTED_NONAUTHORITY`: it interpreted `error:null` as ERROR and proposed fields the validator never read. It may assist implementation but cannot override raw evidence or host tests.

## Prohibited

No invented historical digest/status/tolerance label, no raw rewrite, no unnecessary 12-cell rerun, no Cargo graph or production semantic change, no timing/ranking/speedup, no homotopy-certificate change, no PR merge/tag/release.

## Materialization rule

No external ZIP or host-side continuation bundle is required after this package is published. Extract the entry point directly from the exact package commit:

```bash
git show "$K0_PACKAGE_SHA:docs/exec-plans/k0-stage-telemetry-integration-20260827/START_CONTINUATION.sh" > /tmp/k0-start-continuation.sh
chmod 700 /tmp/k0-start-continuation.sh
/tmp/k0-start-continuation.sh --repo-root /tmp/vigilode-k0-stage-telemetry.kAguIL/tree --package-sha "$K0_PACKAGE_SHA"
```

A missing old local ZIP is packaging/transport absence, not a scientific blocker. The exact Git commit now materializes every required control byte.
