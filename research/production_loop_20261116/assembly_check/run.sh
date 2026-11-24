#!/usr/bin/env bash
# Commands of loop node V26 (full pattern check of the assembly client).
# Protocol: PREDECLARED.json of this directory. Raw output goes to logs/.
#
#   run.sh build <target>                       build a test target (cores 0-5,12-17)
#   run.sh corpus <label> <groups> <arms>        corpus program, logs/corpus_<label>.log (cores 0-5,12-17)
#   run.sh unit                                  the unit tests of the multi-colouring check, logs/unit.log
#   run.sh counts <cell>                         cost program, deterministic counts and bit identity
#   run.sh perf <cell> <arm> <action>            one action under perf stat -e instructions:u (timing lock)
#   run.sh timing <label> <cell> <what> <arms> [rounds]
#                                                interleaved rounds under the timing lock (cores 6-11)
#   run.sh batched <action>                      FA_B (exploratory); action timing runs under the lock
#
# Groups (comma list): controls ring omit128 omit1024 omit131072 narrow correct
# Arms (comma list of <arm>@<execution>): S8@seq FA@seq FA@t2 ...
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v26
LOCK=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/timing.lock
BUILD_CPUS=0-5,12-17
TIMING_CPUS=6-11
LOGS="$HERE/logs"
mkdir -p "$LOGS"

binary() {
    # The newest executable of a test target.
    ls -t "$CARGO_TARGET_DIR"/release/deps/"$1"-* 2>/dev/null | grep -v '\.d$' | head -1
}

header() {
    echo "# node V26 log"
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
        --test "$2" --no-run
    echo "binary: $(binary "$2")"
    ;;
corpus)
    label="$2"
    groups="$3"
    arms="$4"
    bin="$(binary stage_clients_assembly_check_corpus)"
    log="$LOGS/corpus_${label}.log"
    cmd="V26_MODE=corpus V26_GROUP=$groups V26_ARMS=$arms taskset -c $BUILD_CPUS $bin --ignored --nocapture --exact v26_corpus"
    {
        header "$bin" "$cmd"
        run env V26_MODE=corpus V26_GROUP="$groups" V26_ARMS="$arms" taskset -c "$BUILD_CPUS" \
            "$bin" --ignored --nocapture --exact v26_corpus
        echo "# loadavg at end: $(cat /proc/loadavg)"
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
unit)
    cd "$ROOT"
    log="$LOGS/unit.log"
    {
        echo "# node V26 log"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# loadavg at start: $(cat /proc/loadavg)"
        echo "# command: taskset -c $BUILD_CPUS cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test stage_clients_assembly_check_mc -- --nocapture --test-threads 1"
        run taskset -c "$BUILD_CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
            --test stage_clients_assembly_check_mc -- --nocapture --test-threads 1
        echo "# loadavg at end: $(cat /proc/loadavg)"
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
counts)
    cell="$2"
    bin="$(binary stage_clients_assembly_check_measure)"
    log="$LOGS/counts_${cell}.log"
    cmd="V26_MODE=counts V26_CELL=$cell taskset -c $BUILD_CPUS $bin --ignored --nocapture --exact v26_measure"
    {
        header "$bin" "$cmd"
        run env V26_MODE=counts V26_CELL="$cell" taskset -c "$BUILD_CPUS" \
            "$bin" --ignored --nocapture --exact v26_measure
        echo "# loadavg at end: $(cat /proc/loadavg)"
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
perf)
    cell="$2"
    arm="$3"
    action="$4"
    bin="$(binary stage_clients_assembly_check_measure)"
    log="$LOGS/perf_${cell}_${arm}_${action}.log"
    cmd="V26_MODE=perf V26_CELL=$cell V26_ARM=$arm V26_ACTION=$action flock $LOCK taskset -c $TIMING_CPUS perf stat -e instructions:u -x, $bin --ignored --nocapture --exact v26_measure"
    {
        header "$bin" "$cmd"
        run env V26_MODE=perf V26_CELL="$cell" V26_ARM="$arm" V26_ACTION="$action" flock "$LOCK" \
            taskset -c "$TIMING_CPUS" perf stat -e instructions:u -x, "$bin" --ignored --nocapture --exact v26_measure
    } >"$log" 2>&1
    echo "$log"
    ;;
timing)
    label="$2"
    cell="$3"
    what="$4"
    arms="$5"
    rounds="${6:-2+7}"
    bin="$(binary stage_clients_assembly_check_measure)"
    log="$LOGS/timing_${label}.log"
    cmd="V26_MODE=timing V26_CELL=$cell V26_WHAT=$what V26_ARMS=$arms V26_ROUNDS=$rounds flock $LOCK taskset -c $TIMING_CPUS $bin --ignored --nocapture --exact v26_measure"
    {
        header "$bin" "$cmd"
        echo "# running compilers at start: $(pgrep -c -x rustc || true)"
        run env V26_MODE=timing V26_CELL="$cell" V26_WHAT="$what" V26_ARMS="$arms" V26_ROUNDS="$rounds" \
            flock "$LOCK" taskset -c "$TIMING_CPUS" "$bin" --ignored --nocapture --exact v26_measure
        echo "# loadavg at end: $(cat /proc/loadavg)"
        echo "# running compilers at end: $(pgrep -c -x rustc || true)"
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
batched)
    action="$2"
    bin="$(binary stage_clients_assembly_check_measure)"
    log="$LOGS/batched_${action}.log"
    case "$action" in
    identity)
        cmd="V26_MODE=batched V26_ACTION=$action taskset -c $BUILD_CPUS $bin --ignored --nocapture --exact v26_measure"
        {
            header "$bin" "$cmd"
            run env V26_MODE=batched V26_ACTION="$action" taskset -c "$BUILD_CPUS" \
                "$bin" --ignored --nocapture --exact v26_measure
        } >"$log" 2>&1
        ;;
    timing)
        cmd="V26_MODE=batched V26_ACTION=$action flock $LOCK taskset -c $TIMING_CPUS $bin --ignored --nocapture --exact v26_measure"
        {
            header "$bin" "$cmd"
            run env V26_MODE=batched V26_ACTION="$action" flock "$LOCK" taskset -c "$TIMING_CPUS" \
                "$bin" --ignored --nocapture --exact v26_measure
            echo "# loadavg at end: $(cat /proc/loadavg)"
        } >"$log" 2>&1
        ;;
    *)
        cmd="V26_MODE=batched V26_ACTION=$action flock $LOCK taskset -c $TIMING_CPUS perf stat -e instructions:u -x, $bin --ignored --nocapture --exact v26_measure"
        {
            header "$bin" "$cmd"
            run env V26_MODE=batched V26_ACTION="$action" flock "$LOCK" taskset -c "$TIMING_CPUS" \
                perf stat -e instructions:u -x, "$bin" --ignored --nocapture --exact v26_measure
        } >"$log" 2>&1
        ;;
    esac
    echo "$log"
    ;;
*)
    sed -n '2,16p' "${BASH_SOURCE[0]}"
    exit 2
    ;;
esac
