#!/bin/sh
# One cell of node V22 (PREDECLARED.json, AMENDMENT_1.json) measured after a quiet minute:
# the script waits until the 1-minute load is at most QUIET_LOAD (default 7), the timing
# lock is free and, if QUIET_CORE is set to 1, the measurement core 9 and its sibling 21
# were at most 3 percent busy over two seconds (at most 20 minutes; then it measures
# anyway), like run_scan_quiet.sh. The wait looks at the host only, never at a result.
# A cell whose log exists is skipped.
# usage: [QUIET_LOAD=<x>] [QUIET_CORE=1] quiet_measure.sh <runner binary> <family> <size> <attempts> <index> <tag> [--kernels a,b]
set -eu
runner=$1
family=$2
size=$3
attempts=$4
index=$5
tag=$6
shift 6
here=$(dirname "$0")
lock=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock
log="$here/logs/drive_${tag}${index}_${family}${size}.jsonl"
if [ -e "$log" ]; then echo "skip $log"; exit 0; fi
limit=${QUIET_LOAD:-7.0}
core_idle() {
    # Busy share of cpu9 and cpu21 over two seconds, from /proc/stat (user nice system idle iowait irq softirq steal).
    [ "${QUIET_CORE:-0}" = 1 ] || return 0
    a=$(grep -E '^cpu(9|21) ' /proc/stat)
    sleep 2
    b=$(grep -E '^cpu(9|21) ' /proc/stat)
    printf '%s\n%s\n' "$a" "$b" | awk '
        { busy = $2 + $3 + $4 + $7 + $8 + $9; total = busy + $5 + $6
          if ($1 in b0) { db = busy - b0[$1]; dt = total - t0[$1]; if (dt > 0 && db / dt > 0.03) bad = 1 }
          else { b0[$1] = busy; t0[$1] = total } }
        END { exit bad }'
}
n=0
while [ "$n" -lt 80 ]; do
    if awk -v limit="$limit" '{ exit !($1 <= limit) }' /proc/loadavg && flock -n "$lock" true && core_idle; then break; fi
    n=$((n + 1))
    sleep 15
done
if [ "$n" -ge 80 ]; then echo "no quiet minute within 20 minutes before $family $size; measuring anyway"; fi
python3 -I "$here/measure.py" "$runner" "$family" "$size" "$attempts" "$index" --tag "$tag" "$@"
