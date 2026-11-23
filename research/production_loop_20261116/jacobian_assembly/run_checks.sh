#!/usr/bin/env bash
# Checks of loop node V08: the commands the task lists, with their exit codes.
# Writes logs/tests.log (output of this node's test targets, with the allocation
# JSON lines) and logs/checks.log (one V08_CHECK_JSON line per command).
#
#   run_checks.sh            every check
#   run_checks.sh quick      without the two cargo test runs of other targets
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v08
CPUS=0-5,12-17
LOGS="$HERE/logs"
mkdir -p "$LOGS"
CHECKS="$LOGS/checks.log"
SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

MINE=(
    stage_clients_assembly_identity
    stage_clients_assembly_alloc
    stage_clients_assembly_problems
    stage_clients_assembly_measure
)
SOURCES=(
    crates/rodas5p-integrators/src/stage_clients/assembly.rs
    crates/rodas5p-integrators/src/stage_clients/assembly/engine.rs
    crates/rodas5p-integrators/src/stage_clients/assembly/pattern.rs
    crates/rodas5p-integrators/tests/stage_clients_assembly_identity.rs
    crates/rodas5p-integrators/tests/stage_clients_assembly_alloc.rs
    crates/rodas5p-integrators/tests/stage_clients_assembly_problems.rs
    crates/rodas5p-integrators/tests/stage_clients_assembly_measure.rs
    research/production_loop_20261116/jacobian_assembly/runner/src/main.rs
)

{
    echo "# node V08 checks"
    echo "# date: $(date --iso-8601=seconds)"
    echo "# git head: $(git -C "$ROOT" rev-parse HEAD)"
    echo "# rustc: $(rustc +1.94.1 --version)"
} >"$CHECKS"

# check <name> <output file or - > <command...>: run, record the exit code and a result line.
check() {
    local name="$1" keep="$2"
    shift 2
    local out="$SCRATCH/out.txt" rc=0
    (cd "$ROOT" && "$@") >"$out" 2>&1 || rc=$?
    if [ "$keep" != "-" ]; then
        {
            echo "# command: $*"
            cat "$out"
            echo "# exit code: $rc"
        } >>"$keep"
    fi
    python3 -I - "$name" "$rc" "$out" "$*" >>"$CHECKS" <<'EOF'
import json, re, sys
name, rc, path, command = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4]
text = open(path, encoding="utf-8", errors="replace").read()
results = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
if results:
    passed = sum(int(r[1]) for r in results)
    failed = sum(int(r[2]) for r in results)
    ignored = sum(int(r[3]) for r in results)
    result = f"{len(results)} test binaries: {passed} passed, {failed} failed, {ignored} ignored"
else:
    lines = [line for line in text.strip().splitlines() if line.strip()]
    result = lines[-1][:300] if lines else "no output"
print("V08_CHECK_JSON " + json.dumps({"name": name, "command": command, "exit_code": rc, "result": result}))
EOF
    echo "$name: exit $rc"
}

tests=()
for target in "${MINE[@]}"; do tests+=(--test "$target"); done

: >"$LOGS/tests.log"
check "tests of this node (identity, alloc; measure is ignored)" "$LOGS/tests.log" \
    taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
    "${tests[@]}" -- --nocapture
if [ "${1:-}" != "quick" ]; then
    check "source-pinning sentinels" - \
        taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
        --test ppsw1_6b_contracts --test ptsw2b_contracts
    check "stage client kit tests of earlier nodes (unchanged files)" - \
        taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators \
        --test stage_clients_dense_identity --test stage_clients_alloc \
        --test stage_clients_banded_tight_identity
fi
check "clippy -D warnings (lib and this node's test targets)" - \
    taskset -c "$CPUS" cargo +1.94.1 clippy --locked -j 4 --release -p rodas5p-integrators --lib \
    "${tests[@]}" --no-deps -- -D warnings
check "rustfmt --check" - rustfmt +1.94.1 --check --edition 2024 "${SOURCES[@]}"
check "loop checker" - python3 -I research/production_loop_20261116/check_checkpoint.py
check "strict JSON load of the JSON files of this node" - python3 -I -c '
import json, sys
def refuse(text):
    raise ValueError("non-strict constant " + text)
base = "research/production_loop_20261116/jacobian_assembly/"
for name in ("PREDECLARED.json", "RESULTS.json"):
    json.load(open(base + name, encoding="utf-8"), parse_constant=refuse)
    print(name, "strict JSON ok")
'
cat "$CHECKS"
