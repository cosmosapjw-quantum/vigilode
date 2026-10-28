#!/bin/bash
# Bounded local campaign: warm-up 1 + 5 timed runs per cell, pinned to one core.
# usage: run_campaign.sh BIN_DIR RAW_DIR [CPU]
# Cells: evidence_switch_bench {rotation,stiff-forced} x {evidence,screened-reset,pressure-only,dopri,rodas};
#        local_bottleneck_suite case 5 (external endpoint reference when present in the repo) x
#        {switching,rodas5p-mf,rodas5p-dense}; case 10 at the registered tight tolerance x
#        {switching,dopri5,rodas5p-mf}; case 10 at the suite default tolerance x {switching,dopri5}
#        (default-tolerance case 10 fails its cap for every method and is reported only).
set -u
B=${1:?BIN_DIR}; RAW=${2:?RAW_DIR}; CPU=${3:-0}
REPO=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
REF="$REPO/research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json"
mkdir -p "$RAW"
if ls "$RAW"/bench_*.json "$RAW"/suite_*.jsonl >/dev/null 2>&1; then echo "refusing to overwrite existing raw in $RAW" >&2; exit 2; fi
run_bench() { taskset -c "$CPU" "$B/evidence_switch_bench" "$1" "$2" 1e-7 > "$RAW/bench_$1_$2_r$3.json" 2> "$RAW/bench_$1_$2_r$3.err"; echo "bench $1 $2 r$3 exit=$?"; }
run_suite() { local tag=$1 case=$2 method=$3 r=$4; shift 4; taskset -c "$CPU" "$B/local_bottleneck_suite" --case "$case" --size standard --method "$method" --emit-state "$@" > "$RAW/suite_${tag}_${method}_r${r}.jsonl" 2> "$RAW/suite_${tag}_${method}_r${r}.err"; echo "suite $tag $method r$r exit=$?"; }
for case in rotation stiff-forced; do for mode in evidence screened-reset pressure-only dopri rodas; do for r in 0 1 2 3 4 5; do run_bench $case $mode $r; done; done; done
REFARGS=()
if [ -f "$REF" ]; then REFARGS=(--reference-json "$REF" --reference-sha256 "$(sha256sum "$REF" | cut -d' ' -f1)"); echo "case5 reference: $REF"; else echo "case5 reference missing: $REF (accuracy unvalidated)"; fi
for method in switching rodas5p-mf rodas5p-dense; do for r in 0 1 2 3 4 5; do run_suite c5 5 $method $r "${REFARGS[@]}"; done; done
for method in switching dopri5 rodas5p-mf; do for r in 0 1 2 3 4 5; do run_suite c10t 10 $method $r --rtol 1e-9 --atol 1e-12; done; done
for method in switching dopri5; do for r in 0 1 2 3 4 5; do run_suite c10 10 $method $r; done; done
