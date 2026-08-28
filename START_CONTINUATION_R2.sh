#!/usr/bin/env bash
# Controlling continuation entry. A package self-test is NOT a 12-cell evidence pass.
set -euo pipefail
usage(){ echo "usage: $0 --repo-root PATH --package-sha 40HEX" >&2; exit 2; }
REPO_ROOT= PACKAGE_SHA=
while (($#)); do case "$1" in
  --repo-root) (($#>=2))||usage; REPO_ROOT=$2; shift 2;;
  --package-sha) (($#>=2))||usage; PACKAGE_SHA=$2; shift 2;;
  *) usage;;
esac; done
[[ -n "$REPO_ROOT" && "$PACKAGE_SHA" =~ ^[0-9a-f]{40}$ ]] || usage
REPO_ROOT=$(cd "$REPO_ROOT" && pwd -P)
BRANCH=research/k0-stage-telemetry-integration-20260827
PRIOR=c6ec0121be11f76b86afc21f8ae7a304d35c6d83
PREPARED=f6208a104d2f341157d900294aa30d8edb4446c0
PREPARED_TREE=19c393ca5a1ebb6c440130c9c3155e5625c85ce3
REF=origin/docs/k0-codex-execution-package-20260827
cd "$REPO_ROOT"
[[ $(git branch --show-current) == "$BRANCH" ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: wrong branch' >&2; exit 2; }
[[ -z $(git status --porcelain=v1) ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: dirty worktree' >&2; exit 2; }
git fetch --prune origin docs/k0-codex-execution-package-20260827
[[ $(git rev-parse "$REF") == "$PACKAGE_SHA" ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: package ref drift' >&2; exit 2; }
git merge-base --is-ancestor "$PRIOR" "$PACKAGE_SHA" || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: package ancestry drift' >&2; exit 2; }
allowed(){ case "$1" in
  START_CONTINUATION.sh|START_CONTINUATION_FINAL.sh|START_CONTINUATION_R2.sh|HOST_CODEX_CONTINUE.md|HOST_CODEX_CONTINUE_FINAL.md|HOST_CODEX_CONTINUE_R2.md|CODEX_START_HERE.md|WU05_SEMANTIC_REPAIR_HANDOFF.md) return 0;;
  docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_*) return 0;;
  docs/exec-plans/k0-stage-telemetry-integration-20260827/policy/*) return 0;;
  docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_SEMANTIC_*) return 0;;
  docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/*-semantic.schema.json) return 0;;
  tools/verify-k0-wu05-semantic-evidence.py) return 0;;
  *) return 1;; esac; }
mapfile -d '' DELTA < <(git diff --name-only -z "$PRIOR" "$PACKAGE_SHA")
for p in "${DELTA[@]}"; do allowed "$p" || { printf 'BLOCKED_BY_AUTHORITY_DRIFT: unauthorized path %s\n' "$p" >&2; exit 2; }; done
START=$(git rev-parse HEAD); PARENTS=$(git show -s --format=%P HEAD); MODE=UPGRADE
if [[ "$START" == "$PREPARED" ]]; then
  [[ $(git rev-parse HEAD^{tree}) == "$PREPARED_TREE" ]] || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: prepared tree drift' >&2; exit 2; }
elif [[ "$PARENTS" == "$PREPARED $PACKAGE_SHA" ]]; then MODE=REVALIDATE
else echo 'BLOCKED_BY_AUTHORITY_DRIFT: unexpected local history' >&2; exit 2; fi
COMMON=$(git rev-parse --git-common-dir); [[ "$COMMON" = /* ]] || COMMON="$REPO_ROOT/$COMMON"; COMMON=$(cd "$COMMON"&&pwd -P)
RUN="$COMMON/k0-semantic-continuation/$(python3 -c 'import uuid;print(uuid.uuid4().hex)')"; mkdir -p "$RUN"
exec 9>>"$COMMON/k0-semantic-continuation.lock"; flock -n 9 || { echo 'STOP_INVALID: continuation lock busy' >&2; exit 2; }
receipt(){ python3 - "$RUN/receipt.json" "$1" "$MODE" "$PACKAGE_SHA" "$START" <<'PY'
import json,pathlib,subprocess,sys
p,s,m,k,b=sys.argv[1:]
def g(*a):return subprocess.run(['git',*a],text=True,capture_output=True).stdout.strip()
pathlib.Path(p).write_text(json.dumps({'schema':'vigilode-k0-semantic-continuation-receipt/v2','status':s,'mode':m,'package_sha':k,'start_head':b,'end_head':g('rev-parse','HEAD'),'end_tree':g('rev-parse','HEAD^{tree}'),'parents':g('show','-s','--format=%P','HEAD').split(),'raw_evidence_modified':False,'scientific_campaign_rerun':False},indent=2)+'\n')
PY
}
trap 'receipt STOP_INVALID || true; echo "CONTINUATION_RECEIPT=$RUN/receipt.json"' ERR
TMP=$(mktemp -d "${TMPDIR:-/tmp}/k0-semantic-r2.XXXXXX"); WT="$TMP/wt"
cleanup(){ git worktree remove --force "$WT" >/dev/null 2>&1||true; rm -rf "$TMP"; }
trap cleanup EXIT
git worktree add --detach "$WT" "$PACKAGE_SHA" >/dev/null
python3 -B "$WT/tools/verify-k0-wu05-semantic-evidence.py" --self-test >"$RUN/premerge-self-test.json"
grep -F '"marker": "EVIDENCE_V3_PASS"' "$RUN/premerge-self-test.json" >/dev/null
python3 -m py_compile "$WT/tools/verify-k0-wu05-semantic-evidence.py"
python3 - <<PY
import json,pathlib
r=pathlib.Path(${WT@Q})
for p in ['docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_CONTINUATION_AUTHORITY.json','docs/exec-plans/k0-stage-telemetry-integration-20260827/evidence/EVIDENCE_V3_SEMANTIC_AUTHORITY.json','docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/stage-receipt-v3-semantic.schema.json','docs/exec-plans/k0-stage-telemetry-integration-20260827/schemas/cell-receipt-v3-semantic.schema.json']:json.loads((r/p).read_text())
PY
cleanup; trap - EXIT
if [[ "$MODE" == UPGRADE ]]; then
  if ! git merge --no-ff --no-edit "$PACKAGE_SHA" >"$RUN/merge.stdout" 2>"$RUN/merge.stderr"; then git merge --abort >/dev/null 2>&1||true; receipt BLOCKED_BY_AUTHORITY_DRIFT; echo "CONTINUATION_RECEIPT=$RUN/receipt.json"; exit 2; fi
fi
[[ $(git show -s --format=%P HEAD) == "$PREPARED $PACKAGE_SHA" ]] || { receipt BLOCKED_BY_AUTHORITY_DRIFT; exit 2; }
for p in "${DELTA[@]}"; do git diff --quiet "$PACKAGE_SHA" HEAD -- "$p" || { receipt BLOCKED_BY_AUTHORITY_DRIFT; exit 2; }; done
[[ -z $(git status --porcelain=v1) ]] || { receipt STOP_INVALID; exit 2; }
python3 -B tools/verify-k0-wu05-semantic-evidence.py --self-test >"$RUN/postmerge-self-test.json"
grep -F '"marker": "EVIDENCE_V3_PASS"' "$RUN/postmerge-self-test.json" >/dev/null
receipt LOCAL_WU05_AUTHORITY_READY
printf '%s\n' EVIDENCE_V3_SEMANTIC_SELF_TEST_PASS LOCAL_WU05_AUTHORITY_READY
printf 'CONTINUATION_RECEIPT=%s\n' "$RUN/receipt.json"
