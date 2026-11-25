#!/usr/bin/env bash
# Loop L1 node V02 (trial merge): run one gate command and keep its raw log.
# Usage: run_gate.sh <log-name> <command...>
# The log gets a header (command, head commit, start), the raw combined output,
# and a footer with the exit code. Failed runs are kept; nothing is retried here.
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
LOGDIR="$ROOT/research/production_loop_20261116/trial_merge/logs"
mkdir -p "$LOGDIR"
name="$1"
shift
log="$LOGDIR/$name"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/trial
cd "$ROOT" || exit 2
{
  echo "# cmd: $*"
  echo "# head: $(git rev-parse HEAD)"
  echo "# toolchain: $(rustc +1.94.1 --version)"
  echo "# start: $(date -Is)"
  "$@" 2>&1
  code=$?
  echo "# exit_code: $code"
  echo "# end: $(date -Is)"
} >"$log" 2>&1
exit 0
