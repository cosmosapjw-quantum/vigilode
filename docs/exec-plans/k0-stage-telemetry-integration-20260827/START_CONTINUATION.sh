#!/usr/bin/env bash
# Exact-Git entry point for upgrading the preserved c6-prepared WU-05 branch
# to the semantic-evidence package, then transitioning immediately to repair.
set -euo pipefail

PACKAGE_BRANCH=docs/k0-codex-execution-package-20260827
EXPECTED_BRANCH=research/k0-stage-telemetry-integration-20260827
OLD_PACKAGE=c6ec0121be11f76b86afc21f8ae7a304d35c6d83
PREPARED=f6208a104d2f341157d900294aa30d8edb4446c0
PREPARED_TREE=19c393ca5a1ebb6c440130c9c3155e5625c85ce3
REVIEW=e95ce1e58a603306cb665a6ab91cfe02d279972f
REPO=${IMPLEMENTATION_WORKTREE:-/tmp/vigilode-k0-stage-telemetry.kAguIL/tree}
PACKAGE_SHA=${K0_PACKAGE_SHA:-}

usage() {
  cat >&2 <<'EOF'
usage: START_CONTINUATION.sh --package-sha 40HEX [--repo-root PATH]

Use the exact package commit published on PR #21. No local ZIP is required.
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
command -v python3 >/dev/null || { echo 'STOP_INVALID: python3 missing' >&2; exit 2; }
test -d "$REPO" || { echo 'BLOCKED_BY_AUTHORITY_DRIFT: preserved worktree missing' >&2; exit 2; }
REPO=$(cd "$REPO" && pwd -P)

die() { printf '%s\n' "$1" >&2; exit 2; }
gitq() { git -C "$REPO" "$@"; }

test "$(gitq branch --show-current)" = "$EXPECTED_BRANCH" || die 'BLOCKED_BY_AUTHORITY_DRIFT: wrong implementation branch'
test -z "$(gitq status --porcelain=v1)" || die 'BLOCKED_BY_AUTHORITY_DRIFT: implementation worktree dirty'
test "$(gitq rev-parse "$PREPARED^{tree}")" = "$PREPARED_TREE" || die 'BLOCKED_BY_AUTHORITY_DRIFT: prepared tree object drift'
test "$(gitq show -s --format='%P' "$PREPARED")" = "$REVIEW $OLD_PACKAGE" || die 'BLOCKED_BY_AUTHORITY_DRIFT: prepared parent order drift'

gitq fetch --prune origin "$PACKAGE_BRANCH"
test "$(gitq rev-parse "origin/$PACKAGE_BRANCH")" = "$PACKAGE_SHA" || die 'BLOCKED_BY_AUTHORITY_DRIFT: package ref differs from exact pin'
gitq merge-base --is-ancestor "$OLD_PACKAGE" "$PACKAGE_SHA" || die 'BLOCKED_BY_AUTHORITY_DRIFT: new package is not a descendant of c6 authority'

# The package upgrade may change only control/verification files. A byte mismatch
# here is a control-source issue, never a scientific-output verdict.
while IFS= read -r path; do
  case "$path" in
    AGENTS.md|PACKAGE_MANIFEST.sha256|docs/invariants/K0_STAGE_TELEMETRY.md|docs/quality/P0_P1_POLICY.md) ;;
    docs/exec-plans/k0-stage-telemetry-integration-20260827/*) ;;
    tools/verify-k0-*|tools/k0-wu05-*|tools/test_k0_bootstrap*) ;;
    '') ;;
    *) die "BLOCKED_BY_AUTHORITY_DRIFT: package changes non-control path: $path" ;;
  esac
done < <(gitq diff --name-only "$OLD_PACKAGE" "$PACKAGE_SHA")

HEAD=$(gitq rev-parse HEAD)
MODE=
if [[ $HEAD == "$PREPARED" ]]; then
  MODE=UPGRADE
else
  PARENTS=$(gitq show -s --format='%P' HEAD)
  if [[ $PARENTS == "$PREPARED $PACKAGE_SHA" ]]; then
    MODE=REVALIDATE
  else
    die "BLOCKED_BY_AUTHORITY_DRIFT: expected exact prepared head or exact semantic-upgrade merge; got $HEAD"
  fi
fi
printf '{"status":"PASS","marker":"WU05_SEMANTIC_ENTRY_PASS","mode":"%s","start_head":"%s","package":"%s"}\n' "$MODE" "$HEAD" "$PACKAGE_SHA"

COMMON=$(gitq rev-parse --git-common-dir)
[[ $COMMON = /* ]] || COMMON="$REPO/$COMMON"
COMMON=$(cd "$COMMON" && pwd -P)
LOGROOT="$COMMON/k0-semantic-continuation/$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir -p "$LOGROOT"
RECEIPT="$LOGROOT/receipt.json"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/k0-semantic-package.XXXXXX")
PKGWT="$TMP/package"
cleanup() {
  if [[ -d $PKGWT ]]; then gitq worktree remove --force "$PKGWT" >/dev/null 2>&1 || true; fi
  rm -rf -- "$TMP"
}
trap cleanup EXIT

run_logged() {
  local label=$1; shift
  set +e
  "$@" >"$LOGROOT/$label.stdout" 2>"$LOGROOT/$label.stderr"
  local ec=$?
  set -e
  cat "$LOGROOT/$label.stdout"
  cat "$LOGROOT/$label.stderr" >&2
  printf '%s\t%s\n' "$label" "$ec" >> "$LOGROOT/commands.tsv"
  return "$ec"
}

# Validate the exact package in isolation before mutating the preserved branch.
gitq worktree add --detach "$PKGWT" "$PACKAGE_SHA" >"$LOGROOT/worktree-add.stdout" 2>"$LOGROOT/worktree-add.stderr" || die "STOP_INVALID: cannot create detached package worktree; logs=$LOGROOT"
run_logged pre-package python3 -B "$PKGWT/tools/verify-k0-stage-telemetry-plan.py" --repo-root "$PKGWT" --check-package || die "STOP_INVALID: package contract failed before merge; logs=$LOGROOT"
run_logged pre-supplement python3 -B "$PKGWT/tools/verify-k0-wu05-supplement.py" --repo-root "$PKGWT" --expected-package-sha "$PACKAGE_SHA" --check-supplement-manifest --check-authority --self-test || die "STOP_INVALID: semantic supplement failed before merge; logs=$LOGROOT"
for marker in PACKAGE_CONTRACT_PASS WU05_SUPPLEMENT_MANIFEST_PASS LEGACY_REPAIR_BLOBS_PASS EXTERNAL_PACKAGE_PIN_PASS WU05_SUPPLEMENT_AUTHORITY_PASS HOSTILE_FIXTURES_PASS; do
  grep -Fq "$marker" "$LOGROOT"/*.stdout || die "STOP_INVALID: pre-merge marker absent: $marker; logs=$LOGROOT"
done
gitq worktree remove "$PKGWT" >/dev/null

if [[ $MODE == UPGRADE ]]; then
  export GIT_EDITOR=true GIT_MERGE_AUTOEDIT=no
  set +e
  gitq merge --no-ff --no-edit "$PACKAGE_SHA" >"$LOGROOT/merge.stdout" 2>"$LOGROOT/merge.stderr"
  EC=$?
  set -e
  if (( EC != 0 )); then
    if gitq rev-parse --verify MERGE_HEAD >/dev/null 2>&1; then gitq merge --abort; fi
    test "$(gitq rev-parse HEAD)" = "$PREPARED" || die "BLOCKED_BY_AUTHORITY_DRIFT: failed merge did not restore prepared head; logs=$LOGROOT"
    test -z "$(gitq status --porcelain=v1)" || die "BLOCKED_BY_AUTHORITY_DRIFT: failed merge left dirty state; logs=$LOGROOT"
    die "BLOCKED_BY_AUTHORITY_DRIFT: exact semantic package merge conflicted and was aborted; logs=$LOGROOT"
  fi
fi

FINAL=$(gitq rev-parse HEAD)
FINAL_TREE=$(gitq rev-parse HEAD^{tree})
test "$(gitq show -s --format='%P' HEAD)" = "$PREPARED $PACKAGE_SHA" || die 'BLOCKED_BY_AUTHORITY_DRIFT: semantic upgrade parent order drift'
test -z "$(gitq status --porcelain=v1)" || die 'STOP_INVALID: semantic upgrade left dirty worktree'

run_logged post-package python3 -B "$REPO/tools/verify-k0-stage-telemetry-plan.py" --repo-root "$REPO" --check-package || die "STOP_INVALID: package contract failed after merge; logs=$LOGROOT"
run_logged post-supplement python3 -B "$REPO/tools/verify-k0-wu05-supplement.py" --repo-root "$REPO" --expected-package-sha "$PACKAGE_SHA" --check-supplement-manifest --check-authority --check-repair-merge --self-test || die "STOP_INVALID: semantic supplement failed after merge; preserve merge and inspect logs=$LOGROOT"
for marker in PACKAGE_CONTRACT_PASS WU05_SUPPLEMENT_MANIFEST_PASS LEGACY_REPAIR_BLOBS_PASS EXTERNAL_PACKAGE_PIN_PASS WU05_SUPPLEMENT_AUTHORITY_PASS WU05_REPAIR_MERGE_PASS HOSTILE_FIXTURES_PASS; do
  grep -Fq "$marker" "$LOGROOT"/*.stdout || die "STOP_INVALID: post-merge marker absent: $marker; logs=$LOGROOT"
done

python3 - "$RECEIPT" "$MODE" "$PACKAGE_SHA" "$FINAL" "$FINAL_TREE" <<'PY'
import json, pathlib, sys
path, mode, package, head, tree = sys.argv[1:]
pathlib.Path(path).write_text(json.dumps({
  "schema":"vigilode-k0-semantic-continuation-receipt/v1",
  "status":"LOCAL_WU05_AUTHORITY_READY",
  "mode":mode,
  "package_sha":package,
  "prepared_head":head,
  "prepared_tree":tree,
  "execution_permission":"WU05_REPAIR",
  "claim_admission":"EXPLORATORY_NONAUTHORITATIVE_ONLY"
}, indent=2)+"\n")
PY

echo 'LOCAL_WU05_AUTHORITY_READY'
echo "BOOTSTRAP_RECEIPT=$RECEIPT"
echo "SEMANTIC_HANDOFF=$REPO/docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_REPAIR_HANDOFF.md"
echo "SEMANTIC_CODEX_PROMPT=$REPO/docs/exec-plans/k0-stage-telemetry-integration-20260827/WU05_SEMANTIC_REPAIR_CODEX_PROMPT.md"
