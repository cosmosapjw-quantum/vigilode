#!/usr/bin/env bash
# The closing checks of loop L1 node V18_DENSE_LU_CLIENT. Run from the worktree root:
#
#   bash research/production_loop_20261116/dense_lu_client/run_checks.sh
#
# Every command, its output and its exit code go to logs/checks.log; one summary line per check
# goes to logs/checks_summary.jsonl. Nothing is skipped when a check fails.
set -u
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v18
HERE=research/production_loop_20261116/dense_lu_client
LOG=$HERE/logs/checks.log
SUMMARY=$HERE/logs/checks_summary.jsonl
PIN="taskset -c 0-5,12-17"
CARGO="cargo +1.94.1"
MINE="--test stage_clients_dense_lu_reuse_identity --test stage_clients_dense_lu_reuse_alloc --test stage_clients_dense_lu_reuse_problems"
SOURCES="crates/rodas5p-integrators/src/stage_clients/dense_lu_reuse.rs \
crates/rodas5p-integrators/src/stage_clients/dense_lu_reuse/threaded.rs \
crates/rodas5p-integrators/tests/stage_clients_dense_lu_reuse_identity.rs \
crates/rodas5p-integrators/tests/stage_clients_dense_lu_reuse_alloc.rs \
crates/rodas5p-integrators/tests/stage_clients_dense_lu_reuse_problems.rs \
$HERE/runner/src/main.rs"

: > "$LOG"
: > "$SUMMARY"

check() {
    local name=$1
    shift
    {
        echo "=== $name"
        echo "\$ $*"
    } >> "$LOG"
    "$@" >> "$LOG" 2>&1
    local code=$?
    echo "exit code: $code" >> "$LOG"
    python3 -I -c 'import json, sys
print(json.dumps({"name": sys.argv[1], "exit_code": int(sys.argv[2]), "command": sys.argv[3:]}))' \
        "$name" "$code" "$@" >> "$SUMMARY"
    echo "$name: exit $code"
}

check tests $PIN $CARGO test --locked -j 4 --release -p rodas5p-integrators $MINE \
    --test stage_clients_dense_identity --test stage_clients_alloc
check sentinels $PIN $CARGO test --locked -j 4 --release -p rodas5p-integrators \
    --test ppsw1_6b_contracts --test ptsw2b_contracts
check clippy $PIN $CARGO clippy --locked -j 4 --release -p rodas5p-integrators --lib $MINE \
    --no-deps -- -D warnings
check rustfmt rustfmt +1.94.1 --check --edition 2024 $SOURCES
check loop_checker python3 -I research/production_loop_20261116/check_checkpoint.py
check strict_json python3 -I -c '
import json, pathlib, sys
here = pathlib.Path("research/production_loop_20261116/dense_lu_client")
def refuse(text):
    raise ValueError("non-finite constant " + text)
count = 0
for path in sorted(here.glob("*.json")) + sorted((here / "logs").glob("*.json")):
    json.loads(path.read_text(), parse_constant=refuse)
    count += 1
lines = 0
for path in sorted((here / "logs").glob("*.jsonl")):
    for line in path.read_text().splitlines():
        if line.strip():
            json.loads(line, parse_constant=refuse)
            lines += 1
print(f"{count} JSON files and {lines} JSON lines load strictly")
'
grep -c '^test result: ok' "$LOG" | xargs -I{} echo "test binaries with 'test result: ok': {}"
grep '^test result:' "$LOG"
check loop_checker_remaining python3 -I research/production_loop_20261116/dense_lu_client/check_remaining.py
