#!/bin/sh
# One series of the width scan of node V22 (PREDECLARED.json), like run_scan.sh, but before
# each cell the script waits until the 1-minute load is at most 7 and the timing lock is free
# (at most 20 minutes; then it measures anyway). The wait looks at the host only, never at a
# result. Cells whose log of this
# tag and index exists are skipped, so an interrupted series can be continued.
# usage: run_scan_quiet.sh <runner binary> <index> <tag>
set -eu
runner=$1
index=$2
tag=$3
here=$(dirname "$0")
attempts() {
    if [ "$1" -le 8 ]; then echo 60
    elif [ "$1" -le 16 ]; then echo 40
    elif [ "$1" -le 32 ]; then echo 24
    else echo 12
    fi
}
lock=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock
quiet() {
    n=0
    while [ "$n" -lt 80 ]; do
        # Quiet: the 1-minute load is at most 7 and no other agent holds the timing lock.
        if awk '{ exit !($1 <= 7.0) }' /proc/loadavg && flock -n "$lock" true; then return 0; fi
        n=$((n + 1))
        sleep 15
    done
    echo "no quiet minute within 20 minutes before $1; measuring anyway"
}
cell() {
    family=$1
    size=$2
    width=$3
    log="$here/logs/drive_${tag}${index}_${family}${size}.jsonl"
    if [ -e "$log" ]; then echo "skip $log"; return 0; fi
    quiet "$family $size"
    python3 -I "$here/measure.py" "$runner" "$family" "$size" "$(attempts "$width")" "$index" --tag "$tag"
}
for w in 1 2 3 4 5 6 8 10 12 16 24 32 64; do cell acstrip "$w" "$w"; done
for m in 1 2 3 4 6 8 12 16 32; do cell brstrip "$m" $((2 * m)); done
