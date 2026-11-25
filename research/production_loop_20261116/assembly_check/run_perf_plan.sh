#!/usr/bin/env bash
# The instruction counts of node V26 (perf stat -e instructions:u, one action per process, under the timing
# lock on cores 6-11): for every cost cell the problem alone, the client construction of every execution,
# the first linearization of every arm and the whole capped solve of every arm. Logs: logs/perf_*.log.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUN="$HERE/run.sh"

COLOURED_ARMS="None@seq S8@seq FA@seq MC@seq None@t2 S8@t2 FA_P@t2 None@t4 S8@t4 FA_P@t4 None@t6 S8@t6 FA_P@t6 MC_P@t6"
DENSE_ARMS="None@seq S8@seq FA@seq None@t2 S8@t2 FA_P@t2 None@t4 S8@t4 FA_P@t4 None@t6 S8@t6 FA_P@t6"

for cell in bruss1d-n128-coloured nlkpp-dense-n400 nlkpp-band-n4096-r24 bruss1d-n131072; do
    if [ "$cell" = nlkpp-dense-n400 ]; then arms="$DENSE_ARMS"; solve_extra=""; else arms="$COLOURED_ARMS"; solve_extra="MCe@seq"; fi
    "$RUN" perf "$cell" None@seq problem
    for execution in seq t2 t4 t6; do
        "$RUN" perf "$cell" "None@$execution" setup
    done
    for arm in $arms; do
        "$RUN" perf "$cell" "$arm" verify
    done
    for arm in $arms $solve_extra; do
        "$RUN" perf "$cell" "$arm" solve
    done
done
