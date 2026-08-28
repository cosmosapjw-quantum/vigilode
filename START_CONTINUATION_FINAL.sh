#!/usr/bin/env bash
# Final Git-native continuation entry. External ZIP/bundle is never required.
set -euo pipefail

usage() { echo "usage: $0 --repo-root PATH --package-sha 40HEX" >&2; exit 2; }
REPO_ROOT= PACKAGE_SHA=
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
PACKAGE_REF=origin/docs/k0-codex-execution-package-20260827

cd "$REPO_ROOT"
[[ $(git branch --show-current) == "$BRANCH" ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: wrong branch' >&2; exit 2; }
[[ -z $(git status --porcelain=v1) ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: dirty worktree; no stash/reset performed' >&2; exit 2; }
git fetch --prune origin docs/k0-codex-execution-package-20260827
[[ $(git rev-parse "$PACKAGE_REF") == "$PACKAGE_SHA" ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: package ref differs from selected commit' >&2; exit 2; }
git cat-file -e "$PACKAGE_SHA^{commit}"
git merge-base --is-ancestor "$PRIOR_PACKAGE" "$PACKAGE_SHA" || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: package ancestry drift' >&2; exit 2; }

allowed_path() {
  case "$1" in
    START_CONTINUATION.sh|START_CONTINUATION_FINAL.sh|HOST_CODEX_CONTINUE.md|HOST_CODEX_CONTINUE_FINAL.md|CODEX_START_HERE.md|WU05_SEMANTIC_REPAIR_HANDOFF.md) return 0 ;;
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
  allowed_path "$path" || { printf 'BLOCKED_BY_AUTHORITY_DRIFT: unauthorized package path %s\n' "$path" >&2; exit 2; }
done

START=$(git rev-parse HEAD)
PARENTS=$(git show -s --format=%P HEAD)
MODE=UPGRADE
if [[ "$START" == "$PREPARED" ]]; then
  [[ $(git rev-parse HEAD^{tree}) == "$PREPARED_TREE" ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: prepared tree drift' >&2; exit 2; }
elif [[ "$PARENTS" == "$PREPARED $PACKAGE_SHA" ]]; then
  MODE=REVALIDATE
else
  echo 'BLOCKED_BY_AUTHORITY_DRIFT: expected prepared head or its exact semantic continuation merge' >&2
  exit 2
fi

COMMON=$(git rev-parse --git-common-dir); [[ "$COMMON" = /* ]] || COMMON="$REPO_ROOT/$COMMON"; COMMON=$(cd "$COMMON" && pwd -P)
LOG_ROOT="$COMMON/k0-semantic-continuation"; mkdir -p "$LOG_ROOT"
RUN_ID=$(python3 -c 'import uuid; print(uuid.uuid4().hex)')
RUN_DIR="$LOG_ROOT/$RUN_ID"; mkdir "$RUN_DIR"
exec 9>>"$LOG_ROOT/continuation.lock"; flock -n 9 || { echo 'STOP_INVALID: another continuation owns this repository' >&2; exit 2; }

write_receipt() {
  python3 - "$RUN_DIR/receipt.json" "$1" "$MODE" "$PACKAGE_SHA" "$START" <<'PY'
import json,pathlib,subprocess,sys
path,state,mode,package,start=sys.argv[1:]
def g(*a): return subprocess.run(['git',*a],text=True,capture_output=True).stdout.strip()
x={'schema':'vigilode-k0-semantic-continuation-receipt/v1','status':state,'mode':mode,
   'package_sha':package,'start_head':start,'end_head':g('rev-parse','HEAD'),
   'end_tree':g('rev-parse','HEAD^{tree}'),'parents':g('show','-s','--format=%P','HEAD').split(),
   'worktree_status':g('status','--porcelain=v1'),'scientific_campaign_rerun':False,
   'raw_evidence_modified':False}
pathlib.Path(path).write_text(json.dumps(x,indent=2)+'\n')
PY
}
trap 'write_receipt STOP_INVALID || true; echo "CONTINUATION_RECEIPT=$RUN_DIR/receipt.json"' ERR

TMP=$(mktemp -d "${TMPDIR:-/tmp}/k0-semantic-package.XXXXXX"); PKG_WT="$TMP/worktree"
cleanup(){ git worktree remove --force "$PKG_WT" >/dev/null 2>&1 || true; rm -rf "$TMP"; }
trap cleanup EXIT
git worktree add --detach "$PKG_WT" "$PACKAGE_SHA" >/dev/null
python3 -B "$PKG_WT/tools/verify-k0-wu05-semantic-evidence.py" --self-test | tee "$RUN_DIR/premerge-self-test.log"
grep -F '"marker": "EVIDENCE_V3_PASS"' "$RUN_DIR/premerge-self-test.log" >/dev/null
python3 -m py_compile "$PKG_WT/tools/verify-k0-wu05-semantic-evidence.py"
python3 - <<PY
import json,pathlib
r=pathlib.Path(${PKG_WT@Q})
for p in [
'docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json',
'docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_SEMANTIC_AUTHORITY.json',
'docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/stage-receipt-v3-semantic.schema.json',
'docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/cell-receipt-v3-semantic.schema.json']:
 json.loads((r/p).read_text())
PY
cleanup; trap - EXIT

if [[ "$MODE" == UPGRADE ]]; then
  if ! git merge --no-ff --no-edit "$PACKAGE_SHA" >"$RUN_DIR/merge.stdout" 2>"$RUN_DIR/merge.stderr"; then
    git merge --abort >/dev/null 2>&1 || true
    write_receipt BLOCKED_BY_AUTHORITY_DRIFT
    echo "CONTINUATION_RECEIPT=$RUN_DIR/receipt.json"
    echo 'BLOCKED_BY_AUTHORITY_DRIFT: merge conflict aborted; preserved prepared head retained' >&2
    exit 2
  fi
fi
[[ $(git show -s --format=%P HEAD) == "$PREPARED $PACKAGE_SHA" ]] || { write_receipt BLOCKED_BY_AUTHORITY_DRIFT; echo 'BLOCKED_BY_AUTHORITY_DRIFT: parent order drift' >&2; exit 2; }
for path in "${PACKAGE_DELTA[@]}"; do git diff --quiet "$PACKAGE_SHA" HEAD -- "$path" || { write_receipt BLOCKED_BY_AUTHORITY_DRIFT; printf 'BLOCKED_BY_AUTHORITY_DRIFT: merged path differs %s\n' "$path" >&2; exit 2; }; done
[[ -z $(git status --porcelain=v1) ]] || { write_receipt STOP_INVALID; echo 'STOP_INVALID: validation left tracked changes' >&2; exit 2; }
python3 -B tools/verify-k0-wu05-semantic-evidence.py --self-test | tee "$RUN_DIR/postmerge-self-test.log"
grep -F '"marker": "EVIDENCE_V3_PASS"' "$RUN_DIR/postmerge-self-test.log" >/dev/null
write_receipt LOCAL_WU05_AUTHORITY_READY
printf '%s\n' EVIDENCE_V3_PASS LOCAL_WU05_AUTHORITY_READY
printf 'CONTINUATION_RECEIPT=%s\n' "$RUN_DIR/receipt.json"
