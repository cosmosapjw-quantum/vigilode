#!/bin/sh
# One cell of node V22 (PREDECLARED.json, AMENDMENT_1.json) measured after a quiet minute:
# the script waits until the 1-minute load is at most 7 and the timing lock is free (at most
# 20 minutes; then it measures anyway), like run_scan_quiet.sh. The wait looks at the host
# only, never at a result. A cell whose log exists is skipped.
# usage: quiet_measure.sh <runner binary> <family> <size> <attempts> <index> <tag> [--kernels a,b]
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
n=0
while [ "$n" -lt 80 ]; do
    if awk '{ exit !($1 <= 7.0) }' /proc/loadavg && flock -n "$lock" true; then break; fi
    n=$((n + 1))
    sleep 15
done
if [ "$n" -ge 80 ]; then echo "no quiet minute within 20 minutes before $family $size; measuring anyway"; fi
python3 -I "$here/measure.py" "$runner" "$family" "$size" "$attempts" "$index" --tag "$tag" "$@"
