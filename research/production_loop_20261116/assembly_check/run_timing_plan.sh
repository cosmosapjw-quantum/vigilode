#!/usr/bin/env bash
# Wall-time measurements of node V26 (PREDECLARED cost_cells.timing: pinned cores 6-11 under the timing lock,
# interleaved arms, 2 warm-up + 7 timed rounds; quality rule applied by parse_results.py).
#
#   run_timing_plan.sh <try> [invocation ...]
#
# Before every invocation the host is waited for (1-minute load at most 4 and no running compiler; one check
# per minute; the waiting of all tries together is capped at 20 minutes, logs/timing_wait.log), then the
# invocation runs whatever the load is and the log records it. Every invocation stays below 2 minutes under
# the lock: the full checks of bruss1d-n131072 (FA about 25-50 s and FA_P on 2 workers about 15-25 s per
# evaluation) cannot run 2 + 7 rounds in 2 minutes and run with fewer rounds (diagnostic_only).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUN="$HERE/run.sh"
WAITLOG="$HERE/logs/timing_wait.log"
try="$1"
shift

waited_seconds() {
    if [ -f "$WAITLOG" ]; then
        grep -c '^wait ' "$WAITLOG" | awk '{print $1 * 60}'
    else
        echo 0
    fi
}

quiet_wait() {
    while true; do
        local load compilers
        load="$(cut -d' ' -f1 /proc/loadavg)"
        compilers="$(pgrep -c -x rustc || true)"
        if awk -v l="$load" 'BEGIN { exit !(l <= 4.0) }' && [ "$compilers" = "0" ]; then
            echo "quiet $(date --iso-8601=seconds) load $load compilers $compilers before $1" >>"$WAITLOG"
            return
        fi
        if [ "$(waited_seconds)" -ge 1200 ]; then
            echo "budget $(date --iso-8601=seconds) load $load compilers $compilers before $1 (20 minutes spent)" >>"$WAITLOG"
            return
        fi
        echo "wait $(date --iso-8601=seconds) load $load compilers $compilers before $1" >>"$WAITLOG"
        sleep 60
    done
}

CHEAP_COLOURED="None@seq,S8@seq,MC@seq,MC_P@t6,None@t2,S8@t2,None@t4,S8@t4,None@t6,S8@t6"
ALL_COLOURED_VERIFY="None@seq,S8@seq,FA@seq,MC@seq,None@t2,S8@t2,FA_P@t2,None@t4,S8@t4,FA_P@t4,None@t6,S8@t6,FA_P@t6,MC_P@t6"
ALL_COLOURED_SOLVE="$ALL_COLOURED_VERIFY,MCe@seq"
DENSE_VERIFY="None@seq,S8@seq,FA@seq,None@t2,S8@t2,FA_P@t2,None@t4,S8@t4,FA_P@t4,None@t6,S8@t6,FA_P@t6"

# label | cell | what | arms | rounds
PLAN=(
    "b128-verify|bruss1d-n128-coloured|verify|$ALL_COLOURED_VERIFY|2+7"
    "b128-solve|bruss1d-n128-coloured|solve|$ALL_COLOURED_SOLVE|2+7"
    "dense-verify|nlkpp-dense-n400|verify|$DENSE_VERIFY|2+7"
    "dense-solve-seq|nlkpp-dense-n400|solve|None@seq,S8@seq,FA@seq|2+7"
    "dense-solve-thr|nlkpp-dense-n400|solve|S8@seq,None@t2,S8@t2,FA_P@t2,None@t4,S8@t4,FA_P@t4,None@t6,S8@t6,FA_P@t6|2+7"
    "band-verify|nlkpp-band-n4096-r24|verify|$ALL_COLOURED_VERIFY|2+7"
    "band-solve|nlkpp-band-n4096-r24|solve|$ALL_COLOURED_SOLVE|2+7"
    "b131k-verify-cheap|bruss1d-n131072|verify|$CHEAP_COLOURED|2+7"
    "b131k-verify-fap6|bruss1d-n131072|verify|None@seq,S8@seq,None@t6,FA_P@t6|2+7"
    "b131k-verify-fap4|bruss1d-n131072|verify|None@seq,S8@seq,None@t4,FA_P@t4|2+7"
    "b131k-verify-fap2|bruss1d-n131072|verify|None@seq,S8@seq,None@t2,FA_P@t2|1+3"
    "b131k-verify-fa|bruss1d-n131072|verify|None@seq,S8@seq,FA@seq|0+2"
    "b131k-solve-cheap|bruss1d-n131072|solve|None@seq,S8@seq,MC@seq,MCe@seq,MC_P@t6|2+7"
    "b131k-solve-thr|bruss1d-n131072|solve|S8@seq,None@t2,S8@t2,None@t4,S8@t4,None@t6,S8@t6|2+7"
    "b131k-solve-fap6|bruss1d-n131072|solve|S8@seq,S8@t6,FA_P@t6|2+7"
    "b131k-solve-fap4|bruss1d-n131072|solve|S8@seq,S8@t4,FA_P@t4|1+4"
    "b131k-solve-fap2|bruss1d-n131072|solve|S8@seq,S8@t2,FA_P@t2|1+3"
    "b131k-solve-fa|bruss1d-n131072|solve|S8@seq,FA@seq|0+2"
)

for entry in "${PLAN[@]}"; do
    IFS='|' read -r label cell what arms rounds <<<"$entry"
    if [ "$#" -gt 0 ] && ! printf '%s\n' "$@" | grep -qx "$label"; then
        continue
    fi
    quiet_wait "${label}_${try}"
    "$RUN" timing "${label}_${try}" "$cell" "$what" "$arms" "$rounds"
done
