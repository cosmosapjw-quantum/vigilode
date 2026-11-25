#!/usr/bin/env bash
# Diagnostic of node V32: how long the probe's bounded rendezvous waits (time from the first probe
# task's arrival until a second task is active), on fresh 4-thread pools, by the ignored test
# tests/stage_batch_probe_latency.rs. Wall-clock numbers: diagnostic only, they decide nothing.
# One log per condition in logs/latency/, read by parse_results.py.
#
# usage: latency.sh <test executable> <pools per condition> <condition>...
# conditions:
#   plain     no pinning, no tracer
#   strace1   taskset -c $CPU1 strace -f -qq -e trace=none
#   hog30     strace1 plus 30 busy loops pinned to $CPU1 for the whole run (about the accumulated
#             state of provoke.sh's hog1 runs); unlike provoke.sh, the loops are stopped afterwards
set -u
exe=$1; pools=$2; shift 2
here=$(cd "$(dirname "$0")" && pwd)
out=$here/logs/latency
cpu1=${CPU1:-8}
mkdir -p "$out"
for condition in "$@"; do
  log=$out/$condition.log
  {
    echo "# condition $condition pools $pools cpu1 $cpu1 $(date '+%F %T %Z') loadavg $(cat /proc/loadavg)"
    echo "# executable $exe sha256 $(sha256sum "$exe" | cut -d' ' -f1)"
  } > "$log"
  pids=()
  case $condition in
    plain)   prefix=() ;;
    strace1) prefix=(taskset -c "$cpu1" strace -f -qq -e trace=none) ;;
    hog30)
      prefix=(taskset -c "$cpu1" strace -f -qq -e trace=none)
      for k in $(seq 1 30); do
        taskset -c "$cpu1" timeout 600 sh -c 'while :; do :; done' &
        pids+=($!)
      done ;;
    *) echo "unknown condition $condition" >&2; exit 2 ;;
  esac
  V32_LATENCY_POOLS=$pools "${prefix[@]}" "$exe" --ignored --exact rendezvous_wait_on_fresh_pools_diagnostic \
    --nocapture --test-threads=1 >> "$log" 2>&1
  echo "exit=$?" >> "$log"
  if [ "${#pids[@]}" -gt 0 ]; then kill "${pids[@]}" 2>/dev/null; wait "${pids[@]}" 2>/dev/null; fi
  echo "# end $(date '+%F %T %Z') busy loops left: $(pgrep -fc '^sh -c while :' || true)" >> "$log"
done
