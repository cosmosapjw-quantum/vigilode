#!/usr/bin/env bash
# Commands of loop node V26 (full pattern check of the assembly client).
# Protocol: PREDECLARED.json of this directory. Raw output goes to logs/.
#
#   run.sh build <target>                       build a test target (cores 0-5,12-17)
#   run.sh corpus <label> <groups> <arms>        corpus program, logs/corpus_<label>.log (cores 0-5,12-17)
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
*)
    sed -n '2,10p' "${BASH_SOURCE[0]}"
    exit 2
    ;;
esac
