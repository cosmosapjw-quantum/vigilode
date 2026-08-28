#!/usr/bin/env bash
# Controlling continuation entry after the transition-policy false-fail repair.
# The frozen 3f2f771 semantic package is the finite trusted control boundary.
set -euo pipefail

usage() {
  echo "usage: $0 --repo-root PATH --package-sha 40HEX" >&2
  exit 2
}

REPO_ROOT=
PACKAGE_SHA=
while (($#)); do
  case "$1" in
    --repo-root)
      (($# >= 2)) || usage
      REPO_ROOT=$2
      shift 2
      ;;
    --package-sha)
      (($# >= 2)) || usage
      PACKAGE_SHA=$2
      shift 2
      ;;
    *)
      usage
      ;;
  esac
done

[[ -n "$REPO_ROOT" && "$PACKAGE_SHA" =~ ^[0-9a-f]{40}$ ]] || usage
REPO_ROOT=$(cd "$REPO_ROOT" && pwd -P)

BRANCH=research/k0-stage-telemetry-integration-20260827
TRUSTED_SEMANTIC_BASE=3f2f7712d04e12beb3291b8369e82bd3f8d92c45
PREPARED=f6208a104d2f341157d900294aa30d8edb4446c0
PREPARED_TREE=19c393ca5a1ebb6c440130c9c3155e5625c85ce3
REF=origin/docs/k0-codex-execution-package-20260827
ROOT=docs/exec-plans/k0-stage-telemetry-integration-20260827

REQUIRED_EXACT_PATHS=(
  START_CONTINUATION_R2.sh
  HOST_CODEX_CONTINUE_R2.md
  WU05_SEMANTIC_REPAIR_HANDOFF.md
  "$ROOT/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json"
  "$ROOT/policy/BYTE_VS_SEMANTIC_IDENTITY.md"
  "$ROOT/policy/EXECUTION_TRANSITION_ANTI_BUREAUCRACY.md"
  "$ROOT/evidence/EVIDENCE_V3_SEMANTIC_AUTHORITY.json"
  "$ROOT/schemas/stage-receipt-v3-semantic.schema.json"
  "$ROOT/schemas/cell-receipt-v3-semantic.schema.json"
  tools/verify-k0-wu05-semantic-evidence.py
  tools/verify-k0-semantic-transition-scope.py
  tools/test_k0_semantic_transition_scope.py
)

cd "$REPO_ROOT"

[[ $(git branch --show-current) == "$BRANCH" ]] || {
  echo "BLOCKED_BY_AUTHORITY_DRIFT: wrong branch" >&2
  exit 2
}
[[ -z $(git status --porcelain=v1) ]] || {
  echo "BLOCKED_BY_AUTHORITY_DRIFT: dirty worktree" >&2
  exit 2
}

git fetch --prune origin docs/k0-codex-execution-package-20260827
[[ $(git rev-parse "$REF") == "$PACKAGE_SHA" ]] || {
  echo "PROVENANCE_REBIND_REQUIRED: package ref differs from supplied exact commit" >&2
  exit 2
}
git merge-base --is-ancestor "$TRUSTED_SEMANTIC_BASE" "$PACKAGE_SHA" || {
  echo "BLOCKED_BY_AUTHORITY_DRIFT: package does not descend from trusted semantic baseline" >&2
  exit 2
}

START=$(git rev-parse HEAD)
PARENTS=$(git show -s --format=%P HEAD)
MODE=UPGRADE

if [[ "$START" == "$PREPARED" ]]; then
  [[ $(git rev-parse HEAD^{tree}) == "$PREPARED_TREE" ]] || {
    echo "BLOCKED_BY_AUTHORITY_DRIFT: prepared tree drift" >&2
    exit 2
  }
elif [[ "$PARENTS" == "$PREPARED $PACKAGE_SHA" ]]; then
  MODE=REVALIDATE
else
  echo "BLOCKED_BY_AUTHORITY_DRIFT: unexpected local history" >&2
  exit 2
fi

COMMON=$(git rev-parse --git-common-dir)
[[ "$COMMON" = /* ]] || COMMON="$REPO_ROOT/$COMMON"
COMMON=$(cd "$COMMON" && pwd -P)

RUN="$COMMON/k0-semantic-continuation/$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
mkdir -p "$RUN"
exec 9>>"$COMMON/k0-semantic-continuation.lock"
flock -n 9 || {
  echo "STOP_INVALID: continuation lock busy" >&2
  exit 2
}

receipt() {
  python3 - "$RUN/receipt.json" "$1" "$MODE" "$PACKAGE_SHA" "$START" <<'PY'
import json
import pathlib
import subprocess
import sys

path, status, mode, package, start = sys.argv[1:]

def git(*args):
    return subprocess.run(
        ["git", *args], text=True, capture_output=True, check=False
    ).stdout.strip()

payload = {
    "schema": "vigilode-k0-semantic-continuation-receipt/v3",
    "status": status,
    "mode": mode,
    "package_sha": package,
    "start_head": start,
    "end_head": git("rev-parse", "HEAD"),
    "end_tree": git("rev-parse", "HEAD^{tree}"),
    "parents": git("show", "-s", "--format=%P", "HEAD").split(),
    "result_validity": "NOT_EVALUATED",
    "provenance_validity": (
        "REBIND_COMPLETE" if status == "LOCAL_WU05_AUTHORITY_READY"
        else "REBIND_REQUIRED"
    ),
    "packaging_validity": "NONBLOCKING",
    "raw_evidence_modified": False,
    "scientific_campaign_rerun": False,
    "scientific_failure": False,
}
pathlib.Path(path).write_text(json.dumps(payload, indent=2) + "\n")
PY
}

trap 'receipt STOP_INVALID || true; echo "CONTINUATION_RECEIPT=$RUN/receipt.json"' ERR

TMP=$(mktemp -d "${TMPDIR:-/tmp}/k0-semantic-transition.XXXXXX")
WT="$TMP/wt"

cleanup() {
  git worktree remove --force "$WT" >/dev/null 2>&1 || true
  rm -rf "$TMP"
}
trap cleanup EXIT

git worktree add --detach "$WT" "$PACKAGE_SHA" >/dev/null

python3 -B "$WT/tools/verify-k0-semantic-transition-scope.py" \
  --repo-root "$REPO_ROOT" \
  --trusted-base "$TRUSTED_SEMANTIC_BASE" \
  --package-sha "$PACKAGE_SHA" \
  >"$RUN/premerge-transition-scope.json"

grep -F '"marker": "SEMANTIC_TRANSITION_SCOPE_PASS"' \
  "$RUN/premerge-transition-scope.json" >/dev/null

python3 -B "$WT/tools/test_k0_semantic_transition_scope.py" \
  >"$RUN/transition-regression.stdout" \
  2>"$RUN/transition-regression.stderr"

python3 -B "$WT/tools/verify-k0-wu05-semantic-evidence.py" \
  --self-test >"$RUN/premerge-semantic-self-test.json"

grep -F '"marker": "EVIDENCE_V3_PASS"' \
  "$RUN/premerge-semantic-self-test.json" >/dev/null

python3 -m py_compile \
  "$WT/tools/verify-k0-semantic-transition-scope.py" \
  "$WT/tools/test_k0_semantic_transition_scope.py" \
  "$WT/tools/verify-k0-wu05-semantic-evidence.py"

python3 - "$WT" "$ROOT" <<'PY'
import json
import pathlib
import sys

worktree = pathlib.Path(sys.argv[1])
root = sys.argv[2]
for relative in [
    f"{root}/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json",
    f"{root}/evidence/EVIDENCE_V3_SEMANTIC_AUTHORITY.json",
    f"{root}/schemas/stage-receipt-v3-semantic.schema.json",
    f"{root}/schemas/cell-receipt-v3-semantic.schema.json",
]:
    json.loads((worktree / relative).read_text())
PY

cleanup
trap - EXIT

if [[ "$MODE" == UPGRADE ]]; then
  if ! git merge --no-ff --no-edit "$PACKAGE_SHA" \
      >"$RUN/merge.stdout" 2>"$RUN/merge.stderr"; then
    git merge --abort >/dev/null 2>&1 || true
    receipt BLOCKED_BY_AUTHORITY_DRIFT
    echo "CONTINUATION_RECEIPT=$RUN/receipt.json"
    exit 2
  fi
fi

[[ $(git show -s --format=%P HEAD) == "$PREPARED $PACKAGE_SHA" ]] || {
  receipt BLOCKED_BY_AUTHORITY_DRIFT
  echo "CONTINUATION_RECEIPT=$RUN/receipt.json"
  exit 2
}

for path in "${REQUIRED_EXACT_PATHS[@]}"; do
  git diff --quiet "$PACKAGE_SHA" HEAD -- "$path" || {
    receipt BLOCKED_BY_AUTHORITY_DRIFT
    echo "CONTINUATION_RECEIPT=$RUN/receipt.json"
    exit 2
  }
done

[[ -z $(git status --porcelain=v1) ]] || {
  receipt STOP_INVALID
  echo "CONTINUATION_RECEIPT=$RUN/receipt.json"
  exit 2
}

python3 -B tools/verify-k0-semantic-transition-scope.py \
  --repo-root "$REPO_ROOT" \
  --trusted-base "$TRUSTED_SEMANTIC_BASE" \
  --package-sha "$PACKAGE_SHA" \
  >"$RUN/postmerge-transition-scope.json"

grep -F '"marker": "SEMANTIC_TRANSITION_SCOPE_PASS"' \
  "$RUN/postmerge-transition-scope.json" >/dev/null

python3 -B tools/verify-k0-wu05-semantic-evidence.py \
  --self-test >"$RUN/postmerge-semantic-self-test.json"

grep -F '"marker": "EVIDENCE_V3_PASS"' \
  "$RUN/postmerge-semantic-self-test.json" >/dev/null

receipt LOCAL_WU05_AUTHORITY_READY
printf '%s\n' \
  SEMANTIC_TRANSITION_SCOPE_PASS \
  EVIDENCE_V3_SEMANTIC_SELF_TEST_PASS \
  LOCAL_WU05_AUTHORITY_READY
printf 'CONTINUATION_RECEIPT=%s\n' "$RUN/receipt.json"
