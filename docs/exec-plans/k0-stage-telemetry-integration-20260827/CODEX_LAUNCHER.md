# Codex Launcher

The orchestrator—not Codex—prepares the implementation branch as specified in `PACKAGE_OVERLAY_CONTRACT.md`.

## Fetch and verify the published prepared branch

```bash
set -euo pipefail
cd ~/vigilode

git fetch --prune origin \
  main \
  research/a1-post-a2a3-kernel-rerun \
  docs/k0-codex-execution-package-20260827 \
  research/k0-stage-telemetry-integration-20260827

BASE=e1124586a4029f86669e7489278c61ef676d61aa
PACKAGE="$(git rev-parse origin/docs/k0-codex-execution-package-20260827)"
PREPARED="$(git rev-parse origin/research/k0-stage-telemetry-integration-20260827)"

test "$(git rev-parse origin/main)" = "8d0c79184e09efb5bdadc24a6315c60a71a44264"
test "$(git rev-parse origin/research/a1-post-a2a3-kernel-rerun)" = "$BASE"
test "$(git show -s --format='%P' "$PREPARED")" = "$BASE $PACKAGE"
git merge-base --is-ancestor "$BASE" "$PREPARED"
git merge-base --is-ancestor "$PACKAGE" "$PREPARED"
```

## Existing preserved local branch

The previously preserved local branch at the exact PR #20 head is an approved pre-overlay state only when it is clean and has no commits after the base. The orchestrator may fast-forward it to the published prepared branch:

```bash
cd ~/vigilode

test "$(git branch --show-current)" = "research/k0-stage-telemetry-integration-20260827"
test "$(git rev-parse HEAD)" = "$BASE"
test -z "$(git status --porcelain=v1)"
test "$(git rev-list --first-parent --count "$BASE"..HEAD)" = "0"

git merge --ff-only "origin/research/k0-stage-telemetry-integration-20260827"
```

Do not reset, rebase, cherry-pick, or recreate that branch.

## New worktree when no local implementation branch exists

```bash
cd ~/vigilode
git worktree add ../vigilode-k0-stage-telemetry \
  -b research/k0-stage-telemetry-integration-20260827 \
  "origin/research/k0-stage-telemetry-integration-20260827"
cd ../vigilode-k0-stage-telemetry
```

## Final orchestrator preflight

```bash
PACKAGE="$(git rev-parse origin/docs/k0-codex-execution-package-20260827)"
test "$(git branch --show-current)" = "research/k0-stage-telemetry-integration-20260827"
test -z "$(git status --porcelain=v1)"
test "$(git show -s --format='%P' HEAD)" = "$BASE $PACKAGE"
python tools/verify-k0-stage-telemetry-plan.py \
  --repo-root . \
  --check-package \
  --check-overlay-authority
```

Only after this preflight passes should the orchestrator start Codex and paste `CODEX_HANDOFF_PROMPT.md`.
