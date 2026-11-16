#!/usr/bin/env bash
# Runs the measurement programs of loop node V07 (stage a) and writes their raw
# output to logs/<program>.log, with the command, the commit, the toolchain, the
# load average before and after and the exit code. No wall time is measured.
#
# Usage: run_measurements.sh [program ...]
#   programs: measure_model_probes measure_adaptive measure_fixed_step (default: all)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(git -C "$HERE" rev-parse --show-toplevel)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v07}"
cd "$ROOT" || exit 2
mkdir -p "$HERE/logs"
programs=("$@")
if [ "${#programs[@]}" -eq 0 ]; then
  programs=(measure_model_probes measure_adaptive measure_fixed_step)
fi
status=0
for program in "${programs[@]}"; do
  log="$HERE/logs/$program.log"
  cmd=(taskset -c 0-9,12-21 cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators
       --test stage_clients_speculative_measure -- --ignored --exact "$program" --nocapture
       --test-threads=1)
  {
    echo "# command: ${cmd[*]}"
    echo "# CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
    echo "# git HEAD: $(git rev-parse HEAD)"
    echo "# uncommitted changes under crates/: $(git status --porcelain -- crates | wc -l)"
    echo "# rustc: $(rustc +1.94.1 --version)"
    echo "# started: $(date -u +%Y-%m-%dT%H:%M:%SZ); load average: $(cut -d' ' -f1-3 /proc/loadavg)"
  } > "$log"
  "${cmd[@]}" >> "$log" 2>&1
  code=$?
  {
    echo "# exit code: $code"
    echo "# finished: $(date -u +%Y-%m-%dT%H:%M:%SZ); load average: $(cut -d' ' -f1-3 /proc/loadavg)"
  } >> "$log"
  echo "$program exit=$code"
  if [ "$code" -ne 0 ]; then
    status=1
  fi
done
exit "$status"
