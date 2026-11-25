#!/usr/bin/env bash
# Repeated, interleaved runs of built test executables of node V32 (old probe vs new probe).
# Read by parse_results.py through the log it writes. Each run's full libtest output is kept.
#
# usage: repeat_tests.sh <log file> <runs> <label>=<test executable> ...
# The executables are copies of
#   target/release/deps/stage_batch_probe_late_workers-*   (and _contracts-*)
# built from d4e2772f's stage_batch.rs ("old") and from the node's stage_batch.rs ("new").
set -u
log=$1; runs=$2; shift 2
{
  echo "# $(date '+%F %T %Z') kernel $(uname -r) nproc $(nproc) loadavg $(cat /proc/loadavg)"
  for pair in "$@"; do echo "# executable ${pair%%=*}=${pair#*=} sha256=$(sha256sum "${pair#*=}" | cut -d' ' -f1)"; done
} >> "$log"
for i in $(seq 1 "$runs"); do
  for pair in "$@"; do
    label=${pair%%=*}; exe=${pair#*=}
    echo "## run=$i label=$label start=$(date +%s.%N) loadavg=$(cut -d' ' -f1-3 /proc/loadavg)" >> "$log"
    "$exe" --nocapture --test-threads=1 >> "$log" 2>&1
    echo "## run=$i label=$label exit=$? end=$(date +%s.%N)" >> "$log"
  done
done
