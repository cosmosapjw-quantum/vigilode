#!/bin/bash
# Loop node V17: the verification commands of the node, one log per command
# under logs/checks/ (each ends with "# exit=<code>"). Nothing is measured.
#
# usage: run_checks.sh [tests|sentinels|clippy|fmt|checker|json ...]   (default: all)
set -u

NODE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$NODE/../../.." && pwd)"
OUT="$NODE/logs/checks"
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v17
CPUS="0-5,12-17"
mkdir -p "$OUT"
cd "$ROOT" || exit 2

V17_TARGETS="--test stage_clients_krylov_v17_contracts --test stage_clients_krylov_v17_alloc --test stage_clients_krylov_v17_measure --test stage_clients_krylov_v17_problems"
V06_TARGETS="--test stage_clients_krylov_contracts --test stage_clients_krylov_alloc --test stage_clients_krylov_measure --test stage_clients_krylov_problems"
RS_FILES="crates/rodas5p-integrators/src/stage_clients/krylov.rs crates/rodas5p-integrators/src/stage_clients/krylov/step_tied.rs crates/rodas5p-integrators/tests/stage_clients_krylov_v17_problems.rs crates/rodas5p-integrators/tests/stage_clients_krylov_v17_contracts.rs crates/rodas5p-integrators/tests/stage_clients_krylov_v17_alloc.rs crates/rodas5p-integrators/tests/stage_clients_krylov_v17_measure.rs"

run() {
    local name="$1"
    shift
    {
        echo "# date=$(date -Is) head=$(git rev-parse HEAD) dirty_tracked=$(git status --porcelain --untracked-files=no | wc -l)"
        echo "# $*"
        "$@" 2>&1
        echo "# exit=$?"
    } >"$OUT/$name.log"
    echo "$name: $(tail -n 1 "$OUT/$name.log")"
}

checks=("$@")
[ ${#checks[@]} -eq 0 ] && checks=(tests sentinels clippy fmt checker json)

for check in "${checks[@]}"; do
    case "$check" in
    tests)
        # shellcheck disable=SC2086
        run tests taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators $V17_TARGETS $V06_TARGETS -- --nocapture
        ;;
    sentinels)
        run sentinels taskset -c "$CPUS" cargo +1.94.1 test --locked -j 4 --release -p rodas5p-integrators --test ppsw1_6b_contracts --test ptsw2b_contracts
        ;;
    clippy)
        # shellcheck disable=SC2086
        run clippy taskset -c "$CPUS" cargo +1.94.1 clippy --locked -j 4 --release -p rodas5p-integrators --lib $V17_TARGETS --no-deps -- -D warnings
        ;;
    fmt)
        # shellcheck disable=SC2086
        run fmt rustfmt +1.94.1 --check --edition 2024 $RS_FILES
        ;;
    checker)
        run checker python3 -I research/production_loop_20261116/check_checkpoint.py
        # The checker stops at its first complaint. This node changes
        # stage_clients/krylov.rs (allowed by its contract), whose hash the
        # checkpoint pins, so the checker stops there. The second run is the
        # same checker with only that one entry taken out of the checkpoint,
        # to see whether everything after it passes.
        run checker_without_krylov_pin python3 -I -B -c '
import importlib.util
spec = importlib.util.spec_from_file_location("check_checkpoint", "research/production_loop_20261116/check_checkpoint.py")
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)
original = checker.strict
def strict_without_pin(path):
    data = original(path)
    if path.name == "CHECKPOINT.json":
        removed = data["implemented_source_sha256"].pop("crates/rodas5p-integrators/src/stage_clients/krylov.rs", None)
        print("entry taken out of implemented_source_sha256:", "crates/rodas5p-integrators/src/stage_clients/krylov.rs", removed is not None)
    return data
checker.strict = strict_without_pin
checker.main()
'
        ;;
    json)
        run json python3 -I -c '
import glob, json, sys
def bad(value):
    raise ValueError("nonfinite JSON constant: " + value)
paths = sorted(glob.glob("research/production_loop_20261116/mf_stage_solve/*.json")
               + glob.glob("research/production_loop_20261116/mf_stage_solve/reference/*.json"))
for path in paths:
    with open(path, encoding="utf-8") as handle:
        json.load(handle, parse_constant=bad)
    print("strict JSON ok:", path)
print(len(paths), "files")
'
        ;;
    *)
        echo "unknown check $check" >&2
        exit 2
        ;;
    esac
done
