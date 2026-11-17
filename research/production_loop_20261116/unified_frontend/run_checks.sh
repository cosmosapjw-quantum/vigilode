#!/bin/sh
# Verification commands of loop L1 node V03 part A. One raw log per command under logs/.
# Usage (from the worktree root):
#   sh research/production_loop_20261116/unified_frontend/run_checks.sh <step> [<step> ...]
# Steps: contracts existing golden1 golden2 suite clippy fmt doc checker diff
# Every cargo command runs on the build partition (taskset -c 0-5,12-17), never on the
# timing cores; no wall time of these logs is a measurement.
set -u
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT" || exit 2
DIR=research/production_loop_20261116/unified_frontend
LOGS=$DIR/logs
BASE=c468478e73123a4625c6d20f7459a9f79cbf33de
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v03
PIN="taskset -c 0-5,12-17"
mkdir -p "$LOGS"

run() {
    # run <log name> <command...>
    log=$LOGS/$1.log
    shift
    {
        echo "# command: $*"
        echo "# commit: $(git rev-parse HEAD)"
        echo "# worktree_dirty_files: $(git status --porcelain --untracked-files=no | wc -l)"
        echo "# date: $(date -Is)"
        echo "# loadavg_before: $(cat /proc/loadavg)"
    } >"$log"
    "$@" >>"$log" 2>&1
    code=$?
    {
        echo "# loadavg_after: $(cat /proc/loadavg)"
        echo "# exit_code: $code"
    } >>"$log"
    echo "$1 -> exit $code ($log)"
}

for step in "$@"; do
    case $step in
    contracts)
        run contracts $PIN cargo +1.94.1 test --locked -j 4 --release -p vigilode \
            --test unified_frontend_contracts -- --nocapture --test-threads 1
        ;;
    existing)
        run existing_registry_tests $PIN cargo +1.94.1 test --locked -j 4 --release -p vigilode \
            --test unified_method_capabilities --test unified_method_sessions \
            --test production_selection_contracts --test geometry_mol_contracts
        ;;
    golden1)
        run default_path_golden_run1 $PIN cargo +1.94.1 test --locked -j 4 --release -p vigilode \
            --test default_path_golden -- --nocapture --test-threads 1
        ;;
    golden2)
        run default_path_golden_run2 $PIN cargo +1.94.1 test --locked -j 4 --release -p vigilode \
            --test default_path_golden -- --nocapture --test-threads 1
        ;;
    suite)
        run full_suite $PIN cargo +1.94.1 test --locked -j 4 --release -p vigilode --no-fail-fast
        ;;
    clippy)
        run clippy $PIN cargo +1.94.1 clippy --locked -j 4 --release -p vigilode --all-targets \
            --no-deps -- -D warnings
        ;;
    fmt)
        run fmt_check $PIN cargo +1.94.1 fmt -p vigilode -- --check
        ;;
    doc)
        run doc env RUSTDOCFLAGS="-D warnings" $PIN cargo +1.94.1 doc -j 4 -p vigilode --no-deps \
            --locked
        ;;
    checker)
        run check_checkpoint python3 -I research/production_loop_20261116/check_checkpoint.py
        ;;
    diff)
        # paths changed against the base commit, and the golden oracle in particular
        run changed_paths git diff --name-status "$BASE" HEAD
        run golden_diff git diff --stat "$BASE" HEAD -- crates/vigilode/tests/golden \
            crates/vigilode/tests/default_path_golden.rs
        run protected_diff git diff --stat "$BASE" HEAD -- \
            fixtures/rodas5p_coefficients_snapshot.json crates/rodas5p-core crates/rodas5p-krylov \
            crates/rodas5p-integrators crates/vigilode/src/lib.rs crates/vigilode/src/automatic.rs \
            crates/vigilode/src/switching.rs crates/vigilode/Cargo.toml Cargo.toml Cargo.lock \
            rust-toolchain.toml research/LEDGER.jsonl docs/agent
        ;;
    *)
        echo "unknown step $step" >&2
        exit 2
        ;;
    esac
done
