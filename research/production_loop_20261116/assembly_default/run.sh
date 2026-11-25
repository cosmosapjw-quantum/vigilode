#!/usr/bin/env bash
# Commands of loop node V29 (default first-use check of the assembly client, OD-L1-24).
# Raw output goes to logs/; parse_results.py reads only those logs.
#
#   run.sh build <target>                        build a test target (cores 0-5,12-17)
#   run.sh identity <label> <cell>               the V08 identity cells with default options,
#                                                logs/identity_<label>_<cell>.log
#   run.sh corpus <label> <groups> <executions>  the V26 corpus decisions of the default check,
#                                                logs/corpus_<label>_<first group>.log
#   run.sh unit                                  the tests of this node with their output, logs/unit.log
#   run.sh checks                                tests, sentinels, clippy, fmt, loop checker, logs/checks.log
#   run.sh checker-all                           every complaint of the loop checker, logs/checker_all.log
#
# Labels: A = commit A (the code before the change), B = after the change.
# Cells: nlkpp-dense-n400 nlkpp-band-n4096-r24 bruss1d-n131072 bruss1d-n128-dense bruss1d-n128-coloured
# Groups (comma list): controls ring omit128 omit1024 omit131072 narrow correct
# Executions (comma list): seq t3 t6
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w3-v29
CPUS=0-5,12-17
TARGET=stage_clients_assembly_default_record
LOGS="$HERE/logs"
mkdir -p "$LOGS"

binary() {
    # The newest executable of a test target.
    ls -t "$CARGO_TARGET_DIR"/release/deps/"$1"-* 2>/dev/null | grep -v '\.d$' | head -1
}

header() {
    echo "# node V29 log"
    echo "# date: $(date --iso-8601=seconds)"
    echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
    echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
    echo "# client sources sha256: $(cd "$ROOT" && cat crates/rodas5p-integrators/src/stage_clients/assembly.rs crates/rodas5p-integrators/src/stage_clients/assembly/*.rs | sha256sum | cut -d' ' -f1)"
    echo "# program source sha256: $(sha256sum "$ROOT/crates/rodas5p-integrators/tests/$TARGET.rs" | cut -d' ' -f1)"
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
    taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
        --test "$2" --no-run
    echo "binary: $(binary "$2")"
    ;;
identity)
    label="$2"
    cell="$3"
    bin="$(binary "$TARGET")"
    log="$LOGS/identity_${label}_${cell}.log"
    cmd="V29_MODE=identity V29_CELL=$cell taskset -c $CPUS $bin --ignored --nocapture --exact v29_record"
    {
        header "$bin" "$cmd"
        run env V29_MODE=identity V29_CELL="$cell" taskset -c "$CPUS" \
            "$bin" --ignored --nocapture --exact v29_record
        echo "# loadavg at end: $(cat /proc/loadavg)"
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
corpus)
    label="$2"
    groups="$3"
    executions="$4"
    bin="$(binary "$TARGET")"
    log="$LOGS/corpus_${label}_${groups%%,*}.log"
    cmd="V29_MODE=corpus V29_GROUP=$groups V29_EXECUTIONS=$executions taskset -c $CPUS $bin --ignored --nocapture --exact v29_record"
    {
        header "$bin" "$cmd"
        run env V29_MODE=corpus V29_GROUP="$groups" V29_EXECUTIONS="$executions" taskset -c "$CPUS" \
            "$bin" --ignored --nocapture --exact v29_record
        echo "# loadavg at end: $(cat /proc/loadavg)"
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
unit)
    # The tests of this node with their printed lines.
    cd "$ROOT"
    log="$LOGS/unit.log"
    cmd="taskset -c $CPUS cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test stage_clients_assembly_default --test stage_clients_assembly_default_alloc -- --nocapture"
    {
        echo "# node V29 log"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# loadavg at start: $(cat /proc/loadavg)"
        echo "# command: $cmd"
        run taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
            --test stage_clients_assembly_default --test stage_clients_assembly_default_alloc -- --nocapture
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
checker-all)
    # The loop checker without stopping at its first complaint (checker_all.py).
    cd "$ROOT"
    log="$LOGS/checker_all.log"
    {
        echo "# node V29 log"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# command: python3 -I research/production_loop_20261116/assembly_default/checker_all.py"
        run python3 -I research/production_loop_20261116/assembly_default/checker_all.py
    } >"$log" 2>&1
    echo "$log"
    ;;
checks)
    cd "$ROOT"
    log="$LOGS/checks.log"
    : >"$log"
    {
        echo "# node V29 checks"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# rustc: $(rustc +1.94.1 --version)"
    } >>"$log"
    # check <name> <command...>: the whole output, then one V29_CHECK_JSON line.
    check() {
        local name="$1"
        shift
        local out rc=0
        out="$(mktemp)"
        (cd "$ROOT" && "$@") >"$out" 2>&1 || rc=$?
        {
            echo "# check: $name"
            echo "# command: $*"
            cat "$out"
            echo "# exit code: $rc"
        } >>"$log"
        python3 -I - "$name" "$rc" "$out" "$*" >>"$log" <<'EOF'
import json, re, sys
name, rc, path, command = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4]
text = open(path, encoding="utf-8", errors="replace").read()
results = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
record = {"name": name, "command": command, "exit_code": rc}
if results:
    record["binaries"] = len(results)
    record["passed"] = sum(int(r[1]) for r in results)
    record["failed"] = sum(int(r[2]) for r in results)
    record["ignored"] = sum(int(r[3]) for r in results)
lines = [line.strip() for line in text.strip().splitlines() if line.strip()]
record["last_line"] = lines[-1][:400] if lines else ""
print("V29_CHECK_JSON " + json.dumps(record, sort_keys=True))
EOF
        rm -f "$out"
        echo "$name: exit $rc"
    }
    check "stage client tests" taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release \
        -p rodas5p-integrators --test 'stage_clients_*'
    check "sentinels" taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release \
        -p rodas5p-integrators --test ppsw1_6b_contracts --test ptsw2b_contracts
    clippy_targets=()
    for path in crates/rodas5p-integrators/tests/stage_clients_assembly_*.rs; do
        clippy_targets+=(--test "$(basename "$path" .rs)")
    done
    check "clippy" taskset -c "$CPUS" cargo +1.94.1 clippy --locked -j 4 --release \
        -p rodas5p-integrators --lib "${clippy_targets[@]}" --no-deps -- -D warnings
    check "fmt" cargo +1.94.1 fmt -p rodas5p-integrators -- --check
    check "loop checker" python3 -I research/production_loop_20261116/check_checkpoint.py
    echo "$log"
    ;;
*)
    sed -n '2,16p' "${BASH_SOURCE[0]}"
    exit 2
    ;;
esac
