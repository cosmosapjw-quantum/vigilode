#!/usr/bin/env bash
# Provocation runs for node V32 (research/production_loop_20261116/stage_batch_probe/NOTES.txt, steps 1 and 4).
#
# Runs `rodas5p stage-batch-feasibility --profile smoke --output <file>` RUNS times per condition and keeps
# every raw report. With several binaries the runs are interleaved: for each index every binary runs
# once under the same condition, so that host load (other agents' jobs) hits old and new alike.
# Read by parse_results.py (same folder), which counts the runs whose stage_parallelism_observed is
# false. Wall times in runs.tsv are diagnostic only.
#
# usage: BINARIES="old=<path> new=<path>" provoke.sh <batch label> <runs> <condition>...
# output: logs/<batch>/<binary label>/<condition>/run_NNN.json, runs.tsv; logs/<batch>/meta_*.txt, batches.txt
# conditions:
#   strace1   taskset -c $CPU1 strace -f -qq -e trace=none <cmd>        (task text, step 1)
#   strace2   taskset -c $CPU2 strace -f -qq -e trace=none <cmd>        (task text, step 1)
#   hog1      like strace1, plus two busy loops pinned to $CPU1 for the duration of each run
#   hog4      like strace1, plus four busy loops pinned to $CPU1
#   hogonly1  no strace: taskset -c $CPU1 <cmd>, plus two busy loops pinned to $CPU1
#   shared1   one `strace -f -qq -e trace=none` on $CPU1 traces 4 concurrent runs of one binary (one
#             tracer for several processes, as on the hosted runner); RUNS counts runs, not batches
#   (hog1, hog4, hogonly1 and shared1 are additions to the task text: strace1/strace2 alone did not
#   provoke the failure; see NOTES.txt)
# environment: BINARIES (required), CPU1 (default 8), CPU2 (default 8,9), OUT (default: logs/ here)
set -u
batch=$1; runs=$2; shift 2
here=$(cd "$(dirname "$0")" && pwd)
root=${OUT:-$here/logs}/$batch
cpu1=${CPU1:-8}
cpu2=${CPU2:-8,9}
read -r -a pairs <<< "${BINARIES:?set BINARIES=\"label=path ...\"}"
mkdir -p "$root"
{
  echo "batch=$batch"
  for pair in "${pairs[@]}"; do
    echo "binary ${pair%%=*}=${pair#*=} sha256=$(sha256sum "${pair#*=}" | cut -d' ' -f1)"
  done
  echo "date=$(date '+%F %T %Z')"
  echo "kernel=$(uname -r)"
  echo "nproc=$(nproc)"
  echo "strace=$(strace -V | head -1)"
  echo "cpu1=$cpu1 cpu2=$cpu2"
  echo "conditions=$* runs=$runs"
  echo "loadavg_start=$(cat /proc/loadavg)"
} > "$root/meta_$(date +%s).txt"

run_one() {  # <binary> <dir> <index> <command prefix...>
  local binary=$1 dir=$2 index=$3; shift 3
  local file; file=$(printf '%s/run_%03d.json' "$dir" "$index")
  local load; load=$(cut -d' ' -f1-3 /proc/loadavg)
  local start end status
  start=$(date +%s.%N)
  "$@" "$binary" stage-batch-feasibility --profile smoke --output "$file" 2> "$file.stderr"
  status=$?
  end=$(date +%s.%N)
  [ -s "$file.stderr" ] || rm -f "$file.stderr"
  printf '%d\t%d\t%s\t%s\t%s\n' "$index" "$status" "$start" "$end" "$load" >> "$dir/runs.tsv"
}

hog() {  # <cpu> <seconds>: one busy loop pinned to <cpu>
  taskset -c "$1" timeout "$2" sh -c 'while :; do :; done' 2>/dev/null
}

with_hogs() {  # <number of busy loops> <run_one arguments...>
  local count=$1 pids=() k; shift
  for k in $(seq 1 "$count"); do hog "$cpu1" 30 & pids+=($!); done
  run_one "$@"
  kill "${pids[@]}" 2>/dev/null; wait "${pids[@]}" 2>/dev/null
}

prepare() {  # <dir>: never overwrite earlier runs; prints the next free index
  mkdir -p "$1"
  [ -f "$1/runs.tsv" ] || printf 'index\texit\tstart\tend\tloadavg\n' > "$1/runs.tsv"
  echo $(( $(find "$1" -maxdepth 1 -name 'run_*.json' | wc -l) + 1 ))
}

export -f run_one
for condition in "$@"; do
  declare -A first=()
  for pair in "${pairs[@]}"; do
    label=${pair%%=*}
    first[$label]=$(prepare "$root/$label/$condition")
    echo "$label/$condition: runs ${first[$label]}..$(( ${first[$label]} + runs - 1 ))" >> "$root/batches.txt"
  done
  i=0
  while [ "$i" -lt "$runs" ]; do
    step=1
    for pair in "${pairs[@]}"; do
      label=${pair%%=*}; binary=${pair#*=}
      dir=$root/$label/$condition
      index=$(( ${first[$label]} + i ))
      case $condition in
        strace1)  run_one "$binary" "$dir" "$index" taskset -c "$cpu1" strace -f -qq -e trace=none ;;
        strace2)  run_one "$binary" "$dir" "$index" taskset -c "$cpu2" strace -f -qq -e trace=none ;;
        hog1)     with_hogs 2 "$binary" "$dir" "$index" taskset -c "$cpu1" strace -f -qq -e trace=none ;;
        hog4)     with_hogs 4 "$binary" "$dir" "$index" taskset -c "$cpu1" strace -f -qq -e trace=none ;;
        hogonly1) with_hogs 2 "$binary" "$dir" "$index" taskset -c "$cpu1" ;;
        shared1)
          # one tracer for four concurrent runs of this binary on one CPU; the tracer's own output
          # (signal lines of the bash wrapper) goes to tracer.stderr
          step=4
          taskset -c "$cpu1" strace -f -qq -e trace=none bash -c '
            for k in 0 1 2 3; do run_one "$1" "$2" $(( $3 + k )) & done; wait' _ "$binary" "$dir" "$index" \
            2>> "$dir/tracer.stderr" ;;
        *) echo "unknown condition $condition" >&2; exit 2 ;;
      esac
    done
    i=$(( i + step ))
  done
  unset first
done
echo "loadavg_end=$(cat /proc/loadavg)" >> "$root/meta_end_$(date +%s).txt"
