#!/usr/bin/env bash
# Commands of loop node V31 (default first-use check of the assembly client on wide declared bands, OD-L1-27).
# Raw output goes to logs/; parse_results.py reads only those logs.
#
#   run.sh build                                 build both recording programs (cores 0-5,12-17, build lock)
#   run.sh identity <label> <cell>               the V08 identity cells with default options (V29 program),
#                                                logs/identity_<label>_<cell>.log
#   run.sh corpus <label> <groups> <executions>  the V26 corpus decisions of the default (V29 program),
#                                                logs/corpus_<label>_<first group>.log
#   run.sh survey <label>                        total_groups and the default of random banded patterns
#                                                (V31 program), logs/survey_<label>.log
#   run.sh wide <label> <families> <executions>  the supplement decisions of the default (V31 program),
#                                                logs/wide_<label>_<first family>.log
#   run.sh unit                                  the tests of this node with their output, logs/unit.log
#   run.sh checks                                tests, sentinels, clippy, fmt, loop checker, logs/checks.log
#   run.sh checker-all                           every complaint of the loop checker, logs/checker_all.log
#   run.sh doc                                   rustdoc of the crate (warnings of the assembly files),
#                                                logs/doc.log
#
# Labels: A = commit A (the code before the change), B = after the change.
# Cells: nlkpp-dense-n400 nlkpp-band-n4096-r24 bruss1d-n131072 bruss1d-n128-dense bruss1d-n128-coloured
# Groups (comma list): controls ring omit128 omit1024 omit131072 narrow correct
# Families (comma list): random band ring narrow
# Executions (comma list): seq t3 t6
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w4-v31
CPUS=0-5,12-17
# Host-wide build lock shared with the other agents of the loop; held for cargo commands only.
LOCK=/tmp/claude-1000/-home-cosmosapjw-vigilode/d057eb25-bf20-49ac-a1df-6dd2adc8a0a4/scratchpad/locks/cargo_a.lock
V29_TARGET=stage_clients_assembly_default_record
V31_TARGET=stage_clients_assembly_wide_band_record
LOGS="$HERE/logs"
mkdir -p "$LOGS" "$(dirname "$LOCK")"

cargo_locked() {
    flock -w 14400 "$LOCK" taskset -c "$CPUS" cargo +1.94.1 "$@"
}

binary() {
    # The newest executable of a test target.
    ls -t "$CARGO_TARGET_DIR"/release/deps/"$1"-* 2>/dev/null | grep -v '\.d$' | head -1
}

header() {
    echo "# node V31 log"
    echo "# date: $(date --iso-8601=seconds)"
    echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
    echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
    echo "# client sources sha256: $(cd "$ROOT" && cat crates/rodas5p-integrators/src/stage_clients/assembly.rs crates/rodas5p-integrators/src/stage_clients/assembly/*.rs | sha256sum | cut -d' ' -f1)"
    echo "# program source sha256: $(sha256sum "$ROOT/crates/rodas5p-integrators/tests/$3.rs" | cut -d' ' -f1)"
    echo "# program: $3"
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

# record <log> <target> <test name> <VAR=value>...: one run of a recording program.
record() {
    local log="$1" target="$2" name="$3"
    shift 3
    local bin
    bin="$(binary "$target")"
    local cmd="$* taskset -c $CPUS $bin --ignored --nocapture --exact $name"
    {
        header "$bin" "$cmd" "$target"
        run env "$@" taskset -c "$CPUS" "$bin" --ignored --nocapture --exact "$name"
        echo "# loadavg at end: $(cat /proc/loadavg)"
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
}

case "${1:-}" in
build)
    cd "$ROOT"
    cargo_locked test --locked -j 4 --release -p rodas5p-integrators \
        --test "$V29_TARGET" --test "$V31_TARGET" --no-run
    echo "binary: $(binary "$V29_TARGET")"
    echo "binary: $(binary "$V31_TARGET")"
    ;;
identity)
    record "$LOGS/identity_${2}_${3}.log" "$V29_TARGET" v29_record V29_MODE=identity V29_CELL="$3"
    ;;
corpus)
    record "$LOGS/corpus_${2}_${3%%,*}.log" "$V29_TARGET" v29_record \
        V29_MODE=corpus V29_GROUP="$3" V29_EXECUTIONS="$4"
    ;;
survey)
    record "$LOGS/survey_${2}.log" "$V31_TARGET" v31_record V31_MODE=survey
    ;;
wide)
    record "$LOGS/wide_${2}_${3%%,*}.log" "$V31_TARGET" v31_record \
        V31_MODE=wide V31_FAMILY="$3" V31_EXECUTIONS="$4"
    ;;
unit)
    # The tests of this node with their printed lines.
    cd "$ROOT"
    log="$LOGS/unit.log"
    tests=(--test stage_clients_assembly_wide_band --test stage_clients_assembly_wide_band_alloc
        --test stage_clients_assembly_default --test stage_clients_assembly_default_alloc)
    cmd="flock -w 14400 $LOCK taskset -c $CPUS cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --no-fail-fast ${tests[*]} -- --nocapture"
    {
        echo "# node V31 log"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# loadavg at start: $(cat /proc/loadavg)"
        echo "# command: $cmd"
        run cargo_locked test --locked -j 4 --release -p rodas5p-integrators --no-fail-fast \
            "${tests[@]}" -- --nocapture
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
doc)
    # rustdoc of the crate; parse_results.py counts the warnings that name an assembly file.
    cd "$ROOT"
    log="$LOGS/doc.log"
    cmd="flock -w 14400 $LOCK taskset -c $CPUS cargo +1.94.1 doc --locked -j 4 -p rodas5p-integrators --no-deps"
    {
        echo "# node V31 log"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# command: $cmd"
        run cargo_locked doc --locked -j 4 -p rodas5p-integrators --no-deps
        echo "# date at end: $(date --iso-8601=seconds)"
    } >"$log" 2>&1
    echo "$log"
    ;;
checker-all)
    # The loop checker without stopping at its first complaint (checker_all.py).
    cd "$ROOT"
    log="$LOGS/checker_all.log"
    {
        echo "# node V31 log"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# command: python3 -I research/production_loop_20261116/assembly_wide_band/checker_all.py"
        run python3 -I research/production_loop_20261116/assembly_wide_band/checker_all.py
    } >"$log" 2>&1
    echo "$log"
    ;;
checks)
    cd "$ROOT"
    log="$LOGS/checks.log"
    : >"$log"
    {
        echo "# node V31 checks"
        echo "# date: $(date --iso-8601=seconds)"
        echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
        echo "# git status (tracked changes): $(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l)"
        echo "# rustc: $(rustc +1.94.1 --version)"
        echo "# build lock: $LOCK"
    } >>"$log"
    # check <name> <command...>: the whole output, then one V31_CHECK_JSON line.
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
print("V31_CHECK_JSON " + json.dumps(record, sort_keys=True))
EOF
        rm -f "$out"
        echo "$name: exit $rc"
    }
    check "stage client tests and sentinels" flock -w 14400 "$LOCK" taskset -c "$CPUS" \
        cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --no-fail-fast \
        --test 'stage_clients_*' --test ppsw1_6b_contracts --test ptsw2b_contracts
    clippy_targets=()
    for path in crates/rodas5p-integrators/tests/stage_clients_assembly_*.rs; do
        clippy_targets+=(--test "$(basename "$path" .rs)")
    done
    check "clippy" flock -w 14400 "$LOCK" taskset -c "$CPUS" cargo +1.94.1 clippy --locked -j 4 \
        --release -p rodas5p-integrators --lib "${clippy_targets[@]}" --no-deps -- -D warnings
    check "fmt" cargo +1.94.1 fmt -p rodas5p-integrators -- --check
    check "loop checker" python3 -I research/production_loop_20261116/check_checkpoint.py
    echo "$log"
    ;;
*)
    sed -n '2,25p' "${BASH_SOURCE[0]}"
    exit 2
    ;;
esac
