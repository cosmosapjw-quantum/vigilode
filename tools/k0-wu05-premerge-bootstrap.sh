#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --repo-root PATH --package-sha 40HEX" >&2
  exit 2
}

REPO_ROOT=
PACKAGE_SHA=
while (($#)); do
  case "$1" in
    --repo-root) REPO_ROOT=${2:-}; shift 2 ;;
    --package-sha) PACKAGE_SHA=${2:-}; shift 2 ;;
    *) usage ;;
  esac
done
[[ -n "$REPO_ROOT" && -n "$PACKAGE_SHA" ]] || usage
REPO_ROOT=$(cd "$REPO_ROOT" && pwd)
VALIDATOR_REL=tools/verify-k0-wu05-premerge-bootstrap.py
TMP=$(mktemp -d "${TMPDIR:-/tmp}/vigilode-k0-wu05-bootstrap.XXXXXX")
PKG_WT="$TMP/package-worktree"
cleanup() {
  git -C "$REPO_ROOT" worktree remove --force "$PKG_WT" >/dev/null 2>&1 || true
  rm -rf "$TMP"
}
trap cleanup EXIT

cd "$REPO_ROOT"
git fetch --prune origin docs/k0-codex-execution-package-20260827
[[ $(git rev-parse origin/docs/k0-codex-execution-package-20260827) == "$PACKAGE_SHA" ]] || {
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"package branch tip differs from exact external pin"}' >&2
  exit 2
}

git show "$PACKAGE_SHA:$VALIDATOR_REL" > "$TMP/validator.py"
python "$TMP/validator.py" --repo-root "$REPO_ROOT" --package-sha "$PACKAGE_SHA" --check-premerge

git worktree add --detach "$PKG_WT" "$PACKAGE_SHA" >/dev/null
python "$PKG_WT/tools/verify-k0-stage-telemetry-plan.py" --repo-root "$PKG_WT" --check-package
python "$PKG_WT/tools/verify-k0-wu05-supplement.py" \
  --repo-root "$PKG_WT" \
  --expected-package-sha "$PACKAGE_SHA" \
  --check-supplement-manifest \
  --check-authority \
  --self-test

git worktree remove --force "$PKG_WT" >/dev/null

git merge --no-ff --no-edit "$PACKAGE_SHA" || {
  git merge --abort >/dev/null 2>&1 || true
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"package merge conflict"}' >&2
  exit 2
}

python tools/verify-k0-wu05-premerge-bootstrap.py \
  --repo-root "$REPO_ROOT" \
  --package-sha "$PACKAGE_SHA" \
  --check-postmerge
python tools/verify-k0-stage-telemetry-plan.py --repo-root "$REPO_ROOT" --check-package
python tools/verify-k0-wu05-supplement.py \
  --repo-root "$REPO_ROOT" \
  --expected-package-sha "$PACKAGE_SHA" \
  --check-supplement-manifest \
  --check-authority \
  --check-repair-merge \
  --self-test

test -z "$(git status --porcelain=v1)"
printf '%s\n' 'LOCAL_WU05_AUTHORITY_READY'
