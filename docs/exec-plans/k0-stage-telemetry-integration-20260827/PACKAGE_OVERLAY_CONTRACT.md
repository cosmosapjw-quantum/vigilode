# Package Overlay Contract

## Observed blocker closed by this contract

The PR #20 source head and the documentation-package head are sibling histories. Creating the implementation branch from the exact PR #20 head and immediately invoking the package validator is impossible because the validator is absent from that source tree.

The repair is an **orchestrator-prepared two-parent merge**, not an implicit copy, cherry-pick, or Codex-side branch operation.

## Frozen inputs

- source parent: `e1124586a4029f86669e7489278c61ef676d61aa` (`research/a1-post-a2a3-kernel-rerun`)
- package parent: the exact fetched tip of `origin/docs/k0-codex-execution-package-20260827` at authority intake
- prepared branch: `research/k0-stage-telemetry-integration-20260827`

## Required prepared-head topology

Before Codex starts, the prepared branch HEAD must be one merge commit whose ordered parents are:

```text
first parent   e1124586a4029f86669e7489278c61ef676d61aa
second parent  <exact package tip fetched from origin/docs/k0-codex-execution-package-20260827>
```

The merge tree is the union of the PR #20 source tree and the package tree. The package side may change only these paths relative to the PR #20 source parent:

```text
AGENTS.md
PACKAGE_MANIFEST.sha256
docs/exec-plans/k0-stage-telemetry-integration-20260827/**
docs/invariants/K0_STAGE_TELEMETRY.md
docs/quality/P0_P1_POLICY.md
tools/verify-k0-stage-telemetry-plan.py
```

No `crates/**`, `research/a1_inner_tolerance_audit_20260825/**`, `Cargo.lock`, solver source, test source, or scientific evidence may be introduced by the overlay.

## Ownership boundary

- The **orchestrator** creates or fast-forwards the clean implementation branch to the prepared merge commit.
- **Codex does not** create, switch, merge, cherry-pick, rebase, reset, or force-update branches.
- A pre-existing local branch exactly at `e1124586a4029f86669e7489278c61ef676d61aa`, with a clean worktree and zero first-parent commits after the base, is an approved pre-overlay state. The orchestrator may fast-forward it to the published prepared branch. This state alone is not WU-00 success.

## Mechanical proof

At Codex start, all of the following must pass:

```bash
BASE=e1124586a4029f86669e7489278c61ef676d61aa
PACKAGE=$(git rev-parse origin/docs/k0-codex-execution-package-20260827)
HEAD_SHA=$(git rev-parse HEAD)

test "$(git branch --show-current)" = "research/k0-stage-telemetry-integration-20260827"
test -z "$(git status --porcelain=v1)"
test "$(git show -s --format='%P' "$HEAD_SHA")" = "$BASE $PACKAGE"
git merge-base --is-ancestor "$BASE" "$HEAD_SHA"
git merge-base --is-ancestor "$PACKAGE" "$HEAD_SHA"
python tools/verify-k0-stage-telemetry-plan.py --repo-root . --check-package --check-overlay-authority
```

Any mismatch is `BLOCKED_BY_AUTHORITY_DRIFT`. Missing overlay authorization is no longer an unresolved specification boundary.
