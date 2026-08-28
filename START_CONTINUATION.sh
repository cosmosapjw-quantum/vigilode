#!/usr/bin/env bash
# Exact-Git continuation entry; no external ZIP or host-side bundle is required.
set -euo pipefail

usage() {
  echo "usage: $0 --repo-root PATH --package-sha 40HEX" >&2
  exit 2
}

REPO_ROOT=
PACKAGE_SHA=
while (($#)); do
  case "$1" in
    --repo-root) (($# >= 2)) || usage; REPO_ROOT=$2; shift 2 ;;
    --package-sha) (($# >= 2)) || usage; PACKAGE_SHA=$2; shift 2 ;;
    *) usage ;;
  esac
done
[[ -n "$REPO_ROOT" && "$PACKAGE_SHA" =~ ^[0-9a-f]{40}$ ]] || usage
REPO_ROOT=$(cd "$REPO_ROOT" && pwd -P)

BRANCH=research/k0-stage-telemetry-integration-20260827
PRIOR_PACKAGE=c6ec0121be11f76b86afc21f8ae7a304d35c6d83
PREPARED=f6208a104d2f341157d900294aa30d8edb4446c0
PREPARED_TREE=19c393ca5a1ebb6c440130c9c3155e5625c85ce3
REVIEW=e95ce1e58a603306cb665a6ab91cfe02d279972f
PACKAGE_REF=origin/docs/k0-codex-execution-package-20260827

cd "$REPO_ROOT"
[[ $(git branch --show-current) == "$BRANCH" ]] || {
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"wrong implementation branch"}' >&2
  exit 2
}
[[ -z $(git status --porcelain=v1) ]] || {
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"worktree is dirty; nothing was stashed or reset"}' >&2
  exit 2
}

git fetch --prune origin docs/k0-codex-execution-package-20260827
[[ $(git rev-parse "$PACKAGE_REF") == "$PACKAGE_SHA" ]] || {
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"moving package ref differs from exact external pin"}' >&2
  exit 2
}
git cat-file -e "$PACKAGE_SHA^{commit}"
git merge-base --is-ancestor "$PRIOR_PACKAGE" "$PACKAGE_SHA" || {
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"semantic package is not a descendant of the prepared package"}' >&2
  exit 2
}

allowed_path() {
  case "$1" in
    START_CONTINUATION.sh|CODEX_START_HERE.md|WU05_SEMANTIC_REPAIR_HANDOFF.md) return 0 ;;
    docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_*) return 0 ;;
    docs/exec-plans/k0-stage-telemetry-integration-20260827/policy/*) return 0 ;;
    docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_SEMANTIC_*) return 0 ;;
    docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/*-semantic.schema.json) return 0 ;;
    tools/verify-k0-wu05-semantic-evidence.py) return 0 ;;
    *) return 1 ;;
  esac
}

mapfile -d '' PACKAGE_DELTA < <(git diff --name-only -z "$PRIOR_PACKAGE" "$PACKAGE_SHA")
for path in "${PACKAGE_DELTA[@]}"; do
  allowed_path "$path" || {
    printf '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"semantic package changes non-authorized path","path":"%s"}\n' "$path" >&2
    exit 2
  }
done

HEAD=$(git rev-parse HEAD)
PARENTS=$(git show -s --format=%P HEAD)
MODE=UPGRADE
if [[ "$HEAD" == "$PREPARED" ]]; then
  [[ $(git rev-parse HEAD^{tree}) == "$PREPARED_TREE" ]] || {
    echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"prepared tree drift"}' >&2
    exit 2
  }
elif [[ "$PARENTS" == "$PREPARED $PACKAGE_SHA" ]]; then
  MODE=REVALIDATE
else
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"expected exact prepared commit or its exact semantic-package merge"}' >&2
  exit 2
fi

COMMON=$(git rev-parse --git-common-dir)
[[ "$COMMON" = /* ]] || COMMON="$REPO_ROOT/$COMMON"
COMMON=$(cd "$COMMON" && pwd -P)
LOG_ROOT="$COMMON/k0-semantic-continuation"
mkdir -p "$LOG_ROOT"
RUN_ID=$(python3 - <<'PY'
import uuid
print(uuid.uuid4().hex)
PY
)
RUN_DIR="$LOG_ROOT/$RUN_ID"
mkdir "$RUN_DIR"
LOCK="$LOG_ROOT/continuation.lock"
exec 9>>"$LOCK"
flock -n 9 || {
  echo '{"status":"STOP_INVALID","error":"another semantic continuation owns this repository"}' >&2
  exit 2
}

receipt() {
  python3 - "$RUN_DIR/receipt.json" "$1" "$MODE" "$PACKAGE_SHA" "$HEAD" <<'PY'
import json, pathlib, subprocess, sys
path, state, mode, package, start = sys.argv[1:]
def git(*args):
    return subprocess.run(["git", *args], text=True, capture_output=True).stdout.strip()
payload = {
  "schema": "vigilode-k0-semantic-continuation-receipt/v1",
  "status": state,
  "mode": mode,
  "package_sha": package,
  "start_head": start,
  "end_head": git("rev-parse", "HEAD"),
  "end_tree": git("rev-parse", "HEAD^{tree}"),
  "parents": git("show", "-s", "--format=%P", "HEAD").split(),
  "worktree_status": git("status", "--porcelain=v1"),
  "scientific_campaign_rerun": False,
  "raw_evidence_modified": False,
}
pathlib.Path(path).write_text(json.dumps(payload, indent=2) + "\n")
PY
}
trap 'receipt STOP_INVALID || true; echo "CONTINUATION_RECEIPT=$RUN_DIR/receipt.json"' ERR

TMP=$(mktemp -d "${TMPDIR:-/tmp}/k0-semantic-package.XXXXXX")
PKG_WT="$TMP/worktree"
cleanup() {
  git worktree remove --force "$PKG_WT" >/dev/null 2>&1 || true
  rm -rf "$TMP"
}
trap cleanup EXIT

git worktree add --detach "$PKG_WT" "$PACKAGE_SHA" >/dev/null
python3 -B "$PKG_WT/tools/verify-k0-wu05-semantic-evidence.py" --self-test \
  | tee "$RUN_DIR/premerge-semantic-self-test.log"
grep -F '"marker": "EVIDENCE_V3_PASS"' "$RUN_DIR/premerge-semantic-self-test.log" >/dev/null
python3 -m py_compile "$PKG_WT/tools/verify-k0-wu05-semantic-evidence.py"
python3 - <<PY
import json, pathlib
root = pathlib.Path(${PKG_WT@Q})
for rel in [
  'docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json',
  'docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_SEMANTIC_AUTHORITY.json',
  'docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/stage-receipt-v3-semantic.schema.json',
  'docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/cell-receipt-v3-semantic.schema.json',
]:
    json.loads((root / rel).read_text())
PY
cleanup
trap - EXIT

if [[ "$MODE" == UPGRADE ]]; then
  if ! git merge --no-ff --no-edit "$PACKAGE_SHA" >"$RUN_DIR/merge.stdout" 2>"$RUN_DIR/merge.stderr"; then
    git merge --abort >/dev/null 2>&1 || true
    echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"semantic package merge conflicted and was aborted"}' >&2
    receipt BLOCKED_BY_AUTHORITY_DRIFT
    echo "CONTINUATION_RECEIPT=$RUN_DIR/receipt.json"
    exit 2
  fi
fi

[[ $(git show -s --format=%P HEAD) == "$PREPARED $PACKAGE_SHA" ]] || {
  echo '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"semantic continuation parent order drift"}' >&2
  receipt BLOCKED_BY_AUTHORITY_DRIFT
  exit 2
}
for path in "${PACKAGE_DELTA[@]}"; do
  git diff --quiet "$PACKAGE_SHA" HEAD -- "$path" || {
    printf '{"status":"BLOCKED_BY_AUTHORITY_DRIFT","error":"merged continuation path differs from exact package","path":"%s"}\n' "$path" >&2
    receipt BLOCKED_BY_AUTHORITY_DRIFT
    exit 2
  }
done
[[ -z $(git status --porcelain=v1) ]] || {
  echo '{"status":"STOP_INVALID","error":"semantic validation left tracked changes"}' >&2
  receipt STOP_INVALID
  exit 2
}
python3 -B tools/verify-k0-wu05-semantic-evidence.py --self-test \
  | tee "$RUN_DIR/postmerge-semantic-self-test.log"
grep -F '"marker": "EVIDENCE_V3_PASS"' "$RUN_DIR/postmerge-semantic-self-test.log" >/dev/null

receipt LOCAL_WU05_AUTHORITY_READY
printf '%s\n' 'EVIDENCE_V3_PASS'
printf '%s\n' 'LOCAL_WU05_AUTHORITY_READY'
printf 'CONTINUATION_RECEIPT=%s\n' "$RUN_DIR/receipt.json"
