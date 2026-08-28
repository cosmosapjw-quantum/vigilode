#!/usr/bin/env bash
# Materialized GitHub entry point for the WU-05 semantic-evidence upgrade.
# Extract this exact file from the exact pinned package commit and run it.
set -euo pipefail

PACKAGE_BRANCH=docs/k0-codex-execution-package-20260827
EXPECTED_BRANCH=research/k0-stage-telemetry-integration-20260827
REPO=${IMPLEMENTATION_WORKTREE:-/tmp/vigilode-k0-stage-telemetry.kAguIL/tree}
PACKAGE_SHA=${K0_PACKAGE_SHA:-}

usage() {
  cat >&2 <<'EOF'
usage: START_CONTINUATION.sh --package-sha 40HEX [--repo-root PATH]

The SHA must be the exact package commit published on PR #21. This script
never discovers authority from an unpinned moving branch.
EOF
  exit 2
}

while (($#)); do
  case "$1" in
    --package-sha) (($# >= 2)) || usage; PACKAGE_SHA=$2; shift 2 ;;
    --repo-root) (($# >= 2)) || usage; REPO=$2; shift 2 ;;
    *) usage ;;
  esac
done

[[ $PACKAGE_SHA =~ ^[0-9a-f]{40}$ ]] || usage
command -v git >/dev/null || { echo 'STOP_INVALID: git missing' >&2; exit 2; }
test -d "$REPO" || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: preserved worktree missing' >&2; exit 2; }
REPO=$(cd "$REPO" && pwd -P)

test "$(git -C "$REPO" branch --show-current)" = "$EXPECTED_BRANCH" || {
  echo 'BLOCKED_BY_AUTHORITY_DRIFT: wrong implementation branch' >&2; exit 2;
}
test -z "$(git -C "$REPO" status --porcelain=v1)" || {
  echo 'BLOCKED_BY_AUTHORITY_DRIFT: implementation worktree dirty' >&2; exit 2;
}

git -C "$REPO" fetch --prune origin "$PACKAGE_BRANCH"
test "$(git -C "$REPO" rev-parse "origin/$PACKAGE_BRANCH")" = "$PACKAGE_SHA" || {
  echo 'BLOCKED_BY_AUTHORITY_DRIFT: package ref differs from exact pin' >&2; exit 2;
}

TMP=$(mktemp -d "${TMPDIR:-/tmp}/k0-semantic-continuation.XXXXXX")
trap 'rm -rf -- "$TMP"' EXIT
BOOT=$TMP/bootstrap.sh
LOG=$TMP/bootstrap.log

git -C "$REPO" show "$PACKAGE_SHA:tools/k0-wu05-bootstrap-v2.sh" > "$BOOT"
chmod 700 "$BOOT"
set +e
bash "$BOOT" --repo-root "$REPO" --package-sha "$PACKAGE_SHA" 2>&1 | tee "$LOG"
EC=${PIPESTATUS[0]}
set -e
test "$EC" = 0 || {
  echo 'STOP_INVALID: semantic package upgrade failed; preserve bootstrap receipt and inspect logs' >&2
  exit "$EC"
}
grep -Fqx 'LOCAL_WU05_AUTHORITY_READY' "$LOG" || {
  echo 'STOP_INVALID: readiness marker absent' >&2; exit 2;
}

echo 'LOCAL_WU05_AUTHORITY_READY'
echo "SEMANTIC_HANDOFF=$REPO/docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_REPAIR_HANDOFF.md"
echo "SEMANTIC_CODEX_PROMPT=$REPO/docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_REPAIR_CODEX_PROMPT.md"
