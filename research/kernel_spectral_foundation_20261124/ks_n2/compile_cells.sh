#!/bin/sh
# KS-N2 owner-host compile cells: the two legs of ks_n2/ci_feature_matrix.patch.yml from empty
# target directories, with the pinned toolchain. The shared build lock is taken once for the whole
# script, so no leg waits inside its timed command. Records seconds, exit code and the 1-minute load
# before each leg (the host is shared; PLAN.json KS-N2 sets no threshold: the seconds are a record).
# Usage (repository root): compile_cells.sh <empty target root> <build lock>
# Writes research/kernel_spectral_foundation_20261124/results/KS_N2_COMPILE_CELLS.json.
set -eu
ROOT=$1
LOCK=$2
OUT=research/kernel_spectral_foundation_20261124/results/KS_N2_COMPILE_CELLS.json
mkdir -p "$ROOT"
exec 9>"$LOCK"
flock -w 14400 9
leg() {  # name, target subdirectory, cargo arguments...
    name=$1
    dir=$2
    shift 2
    load=$(cut -d' ' -f1 /proc/loadavg)
    stamp=$(date -Iseconds)
    start=$(date +%s%N)
    status=0
    CARGO_TARGET_DIR="$ROOT/$dir" taskset -c 0-5,12-17 cargo +1.94.1 "$@" -j 4 > "$ROOT/$dir.log" 2>&1 || status=$?
    end=$(date +%s%N)
    seconds=$(awk -v a="$start" -v b="$end" 'BEGIN { printf "%.3f", (b - a) / 1e9 }')
    printf '{"leg": "%s", "command": "cargo +1.94.1 %s -j 4", "seconds": %s, "exit": %s, "load1_before": %s, "load_after": "%s", "start": "%s"}' \
        "$name" "$*" "$seconds" "$status" "$load" "$(cat /proc/loadavg)" "$stamp"
}
a=$(leg default default build --locked -p vigilode --release)
b=$(leg research-gates gates check --locked --release -p rodas5p-integrators --all-targets)
printf '{"task": "KS-N2 owner-host compile cells", "host": "owner host, AMD Ryzen 9 5900X; builds pinned to cores 0-5,12-17 with 4 jobs while holding the shared build lock", "head": "%s", "note": "empty target directories; wall time on a shared host (other projects load it) is a record, not a verdict", "legs": [%s, %s]}\n' \
    "$(git rev-parse HEAD)" "$a" "$b" > "$OUT"
cat "$OUT"
