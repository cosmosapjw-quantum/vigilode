#!/usr/bin/env bash
# Step 6 checks of node V32. One log per check in logs/checks/, each ending with "exit=<status>";
# read by parse_results.py. Run from anywhere; the worktree root is two levels above this folder.
# Environment: CARGO_TARGET_DIR (required), LOCK (the host-wide cargo lock file, required).
# usage: run_checks.sh [check name ...]   (no names: all checks)
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
out=$here/logs/checks
mkdir -p "$out"
: "${CARGO_TARGET_DIR:?}" "${LOCK:?}"
only=" $* "
cargo_locked() { flock -w 14400 "$LOCK" taskset -c 0-5,12-17 cargo +1.94.1 "$@"; }

check() {  # <name> <command...>
  local name=$1; shift
  [ "$only" = "  " ] || [[ "$only" == *" $name "* ]] || return 0
  local log=$out/$name.log
  {
    echo "# check $name"
    echo "# head $(git -C "$root" rev-parse HEAD) stage_batch.rs blob $(git -C "$root" hash-object crates/rodas5p-integrators/src/stage_batch.rs)"
    echo "# start $(date '+%F %T %Z') loadavg $(cat /proc/loadavg)"
    echo "# command: $*"
  } > "$log"
  (cd "$root" && "$@") >> "$log" 2>&1
  local status=$?
  echo "# end $(date '+%F %T %Z')" >> "$log"
  echo "exit=$status" >> "$log"
  echo "$name exit=$status"
}

check cli_contracts cargo_locked test --locked -j 4 --release -p rodas5p-cli --test cli_contracts --no-fail-fast
check integrators_stage_batch_tests cargo_locked test --locked -j 4 --release -p rodas5p-integrators \
  --test stage_batch_contracts --test generic_scope_contracts \
  --test stage_batch_probe_contracts --test stage_batch_probe_late_workers \
  --test stage_batch_probe_latency --no-fail-fast -- --nocapture
check clippy cargo_locked clippy --locked -j 4 --release -p rodas5p-integrators --no-deps \
  --lib --test stage_batch_probe_contracts --test stage_batch_probe_late_workers \
  --test stage_batch_probe_latency -- -D warnings
check fmt cargo +1.94.1 fmt -p rodas5p-integrators -- --check
check loop_checker python3 -I research/production_loop_20261116/check_checkpoint.py
# the checker stops at its first complaint; this runs its other checks too (see loop_checker_rest.py)
check loop_checker_rest python3 -I research/production_loop_20261116/stage_batch_probe/loop_checker_rest.py
