#!/usr/bin/env bash
# KS-N7b: run one check and write its raw output to a log with the header lines that
# ks_n7b/make_results.py parses (# command, # wrapper, # started, # head, # dirty, # load,
# # exit, # finished). Cargo commands run under the shared build lock on the allowed cores.
# Usage: run_logged.sh <log path> <command...>   (LOCK=0 runs without the lock and taskset)
set -u
log="$1"
shift
root=/home/cosmosapjw/worktrees/vigilode-stage-lattice-kschain
lock=/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad/locks/cargo_b.lock
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/kschain
cd "$root" || exit 2
mkdir -p "$(dirname "$log")"
wrapper=(flock -w 14400 "$lock" taskset -c 0-5,12-17)
if [ "${LOCK:-1}" = "0" ]; then
  wrapper=()
fi
{
  printf '# command:'
  printf ' %q' "$@"
  printf '\n'
  printf '# wrapper: %s\n' "${wrapper[*]:-none}"
  printf '# cargo_target_dir: %s\n' "$CARGO_TARGET_DIR"
  printf '# started: %s\n' "$(date --iso-8601=seconds)"
  printf '# head: %s\n' "$(git rev-parse HEAD)"
  printf '# dirty: %s\n' "$(git status --porcelain --untracked-files=no | wc -l)"
  printf '# load: %s\n' "$(cat /proc/loadavg)"
} >"$log"
"${wrapper[@]}" "$@" >>"$log" 2>&1
status=$?
{
  printf '# exit: %s\n' "$status"
  printf '# finished: %s\n' "$(date --iso-8601=seconds)"
} >>"$log"
exit "$status"
