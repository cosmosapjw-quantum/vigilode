# Codex Launcher

The orchestrator—not Codex—prepares the implementation branch.

```bash
cd ~/vigilode
git fetch --prune origin main research/a1-post-a2a3-kernel-rerun docs/k0-codex-execution-package-20260827

test "$(git rev-parse origin/main)" = "8d0c79184e09efb5bdadc24a6315c60a71a44264"
test "$(git rev-parse origin/research/a1-post-a2a3-kernel-rerun)" = "e1124586a4029f86669e7489278c61ef676d61aa"

git worktree add ../vigilode-k0-stage-telemetry \
  -b research/k0-stage-telemetry-integration-20260827 \
  e1124586a4029f86669e7489278c61ef676d61aa

cd ../vigilode-k0-stage-telemetry
git status --porcelain=v1
python tools/verify-k0-stage-telemetry-plan.py --repo-root . --check-package
```

Then start Codex in that worktree and paste `CODEX_HANDOFF_PROMPT.md`.

If the implementation branch already exists, do not force-update it. Inspect its exact SHA and either resume an owned clean branch or stop with `BLOCKED_BY_AUTHORITY_DRIFT`.
