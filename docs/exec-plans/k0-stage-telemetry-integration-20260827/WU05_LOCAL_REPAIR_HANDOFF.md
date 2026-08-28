# WU-05 semantic-evidence repair handoff — resume execution, do not add process layers

## Current local authority

The user reports a clean prepared merge:

```text
branch        research/k0-stage-telemetry-integration-20260827
prepared      f6208a104d2f341157d900294aa30d8edb4446c0
prepared tree 19c393ca5a1ebb6c440130c9c3155e5625c85ce3
parents       [e95ce1e58a603306cb665a6ab91cfe02d279972f, c6ec0121be11f76b86afc21f8ae7a304d35c6d83]
raw WU-04     12/12 byte-identical by the latest local report
```

Treat these as local evidence to verify, not remote facts to overwrite. Do not reset/rebase/stash/amend/force-update or recreate the worktree.

## Bounded semantic repair

The immutable raw result is not modified. No raw top-level `status` or `tolerance_arm` label is required; LegacyFixed is derived from `linear_rtol=1e-10` and `linear_atol=1e-12`. Source head/tree are derived from the unique raw outer envelope. `error:null` is not ERROR. A historical missing signed-residual digest remains `null` with `LEGACY_NOT_RECORDED`; any invented backfill is rejected. Numerical identity excludes wrapper, source and transport metadata while retaining scientific/work/gate/audit content. Raw SHA remains exact provenance.

No new schema version, marker, review tier, or campaign rerun is introduced.

## Exact Git materialization and upgrade

Use the exact package SHA supplied after publication:

```bash
set -euo pipefail
: "${K0_PACKAGE_SHA:?exact package SHA required}"
REPO=/tmp/vigilode-k0-stage-telemetry.kAguIL/tree
cd "$REPO"
test "$(git branch --show-current)" = research/k0-stage-telemetry-integration-20260827
test "$(git rev-parse HEAD)" = f6208a104d2f341157d900294aa30d8edb4446c0
test "$(git rev-parse HEAD^{tree})" = 19c393ca5a1ebb6c440130c9c3155e5625c85ce3
test -z "$(git status --porcelain=v1)"
git fetch --prune origin docs/k0-codex-execution-package-20260827
test "$(git rev-parse origin/docs/k0-codex-execution-package-20260827)" = "$K0_PACKAGE_SHA"
START=$(mktemp "${TMPDIR:-/tmp}/k0-start-continuation.XXXXXX")
trap 'rm -f -- "$START"' EXIT
git show "$K0_PACKAGE_SHA:docs/exec-plans/k0-stage-telemetry-integration-20260827/START_CONTINUATION.sh" > "$START"
chmod 700 "$START"
"$START" --repo-root "$REPO" --package-sha "$K0_PACKAGE_SHA"
```

The runner must classify the existing c6 merge as `UPGRADE`, validate the exact new package, create ordered parents `[f6208a10..., K0_PACKAGE_SHA]`, and emit `LOCAL_WU05_AUTHORITY_READY`. A missing old ZIP is packaging/transport absence, not a scientific blocker.

## Immediate transition after readiness

Do not reopen WU-00–04 or add a representation-review cycle. Resume the five original WU-05 findings from their existing RED reproducers. Generate v3 wrappers mechanically from the twelve immutable raw cells, preserving all twelve raw SHA-256 values. Wrapper/package SHA changes alone do not authorize a campaign rerun. Rerun only for a real change in equations, tolerance, routing, convergence decisions, stage work, or numerical payload.

Close the public bridge, aggregate ERROR preservation, information-rich failure schema, and current-code signed-residual mutation guard. Run targeted/workspace tests, the existing one read-only fresh repair review, and the existing differential audit. Publish only a draft stacked implementation PR after P0/P1=0.

The prior Qwen output is `REJECTED_NONAUTHORITY`: it interpreted `error:null` as ERROR and proposed fields the validator did not read.

## Prohibited

No invented historical digest/status/tolerance label, raw rewrite, unnecessary 12-cell rerun, Cargo graph or production semantic change, timing/ranking/speedup, homotopy-certificate change, PR merge, tag, or release.
