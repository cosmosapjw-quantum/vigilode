#!/usr/bin/env bash
# Measurement commands of loop node V08 (finite-difference Jacobian assembly).
# Protocol: PREDECLARED.json of this directory. Raw output goes to logs/.
#
#   run_measurements.sh build                         build the measurement binary (cores 0-5,12-17)
#   run_measurements.sh identity <cell>               every arm + the protected driver, with fingerprints
#   run_measurements.sh failclosed                    the negative controls
#   run_measurements.sh timing <cell> <try> [protected] [arms]
#                                                     authoritative timing: timing lock, cores 6-11
#   run_measurements.sh perf <cell> <arm>             one solve under perf stat -e instructions:u
#   run_measurements.sh dump <cell>                   last accepted state of the sequential arm, for reference/
#
# Cells: nlkpp-dense-n400 nlkpp-band-n4096-r24 bruss1d-n131072 bruss1d-n128-dense bruss1d-n128-coloured
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v08
LOCK=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock
BUILD_CPUS=0-5,12-17
TIMING_CPUS=6-11
TARGET=stage_clients_assembly_measure
LOGS="$HERE/logs"
mkdir -p "$LOGS"

binary() {
    # The newest executable of the measurement target.
    ls -t "$CARGO_TARGET_DIR"/release/deps/${TARGET}-* 2>/dev/null | grep -v '\.d$' | head -1
}

header() {
    echo "# node V08 measurement log"
    echo "# date: $(date --iso-8601=seconds)"
    echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
    echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
    echo "# binary: $1"
    echo "# binary sha256: $(sha256sum "$1" | cut -d' ' -f1)"
    echo "# rustc: $(rustc +1.94.1 --version)"
    echo "# host: $(uname -r), $(nproc) logical cpus"
    echo "# loadavg at start: $(cat /proc/loadavg)"
    echo "# command: $2"
}

# Run a command, keep its exit code in the log.
run() {
    local rc=0
    "$@" || rc=$?
    echo "# exit code: $rc"
}

case "${1:-}" in
build)
    cd "$ROOT"
    taskset -c "$BUILD_CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
        --test "$TARGET" --no-run
    echo "binary: $(binary)"
    ;;
identity)
    cell="$2"
    bin="$(binary)"
    log="$LOGS/identity_${cell}.log"
    cmd="V08_MODE=identity V08_CELL=$cell taskset -c $BUILD_CPUS $bin --ignored --nocapture --exact v08_measure"
    {
        header "$bin" "$cmd"
        run env V08_MODE=identity V08_CELL="$cell" taskset -c "$BUILD_CPUS" "$bin" --ignored --nocapture --exact v08_measure
        echo "# loadavg at end: $(cat /proc/loadavg)"
    } >"$log" 2>&1
    echo "$log"
    ;;
failclosed)
    bin="$(binary)"
    log="$LOGS/failclosed.log"
    cmd="V08_MODE=failclosed taskset -c $BUILD_CPUS $bin --ignored --nocapture --exact v08_measure"
    {
        header "$bin" "$cmd"
        run env V08_MODE=failclosed taskset -c "$BUILD_CPUS" "$bin" --ignored --nocapture --exact v08_measure
    } >"$log" 2>&1
    echo "$log"
    ;;
timing)
    cell="$2"
    try="$3"
    backend="${4:-tight}"
    arms="${5:-}"
    bin="$(binary)"
    suffix=""
    envs=(V08_MODE=timing V08_CELL="$cell")
    if [ "$backend" = "protected" ]; then
        suffix="_protected"
        envs+=(V08_BACKEND=protected)
    fi
    if [ -n "$arms" ]; then
        envs+=(V08_ARMS="$arms")
    fi
    log="$LOGS/timing_${cell}${suffix}_try${try}.log"
    cmd="env ${envs[*]} flock $LOCK taskset -c $TIMING_CPUS $bin --ignored --nocapture --exact v08_measure"
    {
        header "$bin" "$cmd"
        run env "${envs[@]}" flock "$LOCK" taskset -c "$TIMING_CPUS" "$bin" --ignored --nocapture --exact v08_measure
        echo "# loadavg at end: $(cat /proc/loadavg)"
    } >"$log" 2>&1
    echo "$log"
    ;;
perf)
    cell="$2"
    arm="$3"
    bin="$(binary)"
    log="$LOGS/perf_${cell}_${arm}.log"
    cmd="V08_MODE=single V08_CELL=$cell V08_ARM=$arm flock $LOCK taskset -c $TIMING_CPUS perf stat -e instructions:u -x, $bin --ignored --nocapture --exact v08_measure"
    {
        header "$bin" "$cmd"
        run env V08_MODE=single V08_CELL="$cell" V08_ARM="$arm" flock "$LOCK" taskset -c "$TIMING_CPUS" \
            perf stat -e instructions:u -x, "$bin" --ignored --nocapture --exact v08_measure
    } >"$log" 2>&1
    echo "$log"
    ;;
dump)
    # The last accepted time and state of the sequential arm (little-endian f64: t, then y).
    cell="$2"
    bin="$(binary)"
    mkdir -p "$HERE/reference"
    out="$HERE/reference/final_state_${cell}.bin"
    log="$LOGS/dump_${cell}.log"
    cmd="V08_MODE=single V08_CELL=$cell V08_ARM=seq V08_DUMP=$out taskset -c $BUILD_CPUS $bin --ignored --nocapture --exact v08_measure"
    {
        header "$bin" "$cmd"
        run env V08_MODE=single V08_CELL="$cell" V08_ARM=seq V08_DUMP="$out" taskset -c "$BUILD_CPUS" \
            "$bin" --ignored --nocapture --exact v08_measure
        echo "# dump sha256: $(sha256sum "$out" | cut -d' ' -f1)"
    } >"$log" 2>&1
    echo "$log"
    ;;
*)
    sed -n '2,13p' "${BASH_SOURCE[0]}"
    exit 2
    ;;
esac
