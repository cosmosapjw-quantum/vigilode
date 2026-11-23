#!/bin/sh
# Verification commands of loop L1 node V03 part B (unified router). One raw log per
# command under logs/. Usage (from the worktree root):
#   sh research/production_loop_20261116/unified_router/run_checks.sh <step> [<step> ...]
# Steps: contracts existing golden suite clippy fmt doc example checker diff
# Every cargo command runs on the build partition (taskset -c 0-5,12-17), never on the
# timing cores; no wall time of these logs is a measurement.
set -u
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT" || exit 2
DIR=research/production_loop_20261116/unified_router
LOGS=$DIR/logs
BASE=587a139501729bbe886bb2b8c49b0ef893705d08
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-loop-20261116/w2-v03b
PIN="taskset -c 0-5,12-17"
mkdir -p "$LOGS"

run() {
    # run <log name> <command...>
    name=$1
    log=$LOGS/$name.log
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
    echo "$name -> exit $code ($log)"
}

for step in "$@"; do
    case $step in
    contracts)
        run contracts $PIN cargo +1.94.1 test --locked -j 4 --release -p vigilode \
            --test unified_router_contracts -- --nocapture --test-threads 1
        ;;
    existing)
        # the unified tests that existed before this node, and the tests of the registry
        # drivers and of the switching adapter that the registry change could reach
        run existing_unified_and_registry_tests $PIN cargo +1.94.1 test --locked -j 4 --release \
            -p vigilode --test unified_production_solver_contracts \
            --test unified_frontend_contracts --test registry_contracts \
            --test switcher_v2_admission_contracts --test switching_economy_contracts \
            --test production_hotloop_allocation_contracts --test sparse_jvp_contracts \
            --test switching_v2_remediation_contracts
        ;;
    golden)
        run default_path_golden $PIN cargo +1.94.1 test --locked -j 4 --release -p vigilode \
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
    example)
        run example $PIN cargo +1.94.1 run --locked -j 4 --release -p vigilode \
            --example unified_production_solver
        ;;
    checker)
        run check_checkpoint python3 -I research/production_loop_20261116/check_checkpoint.py
        ;;
    diff)
        # paths changed against the base commit, the golden oracle and the protected files
        run changed_paths git diff --name-status "$BASE" HEAD
        run golden_diff git diff --stat "$BASE" HEAD -- crates/vigilode/tests/golden \
            crates/vigilode/tests/default_path_golden.rs
        run protected_diff git diff --stat "$BASE" HEAD -- \
            fixtures/rodas5p_coefficients_snapshot.json crates/rodas5p-core crates/rodas5p-krylov \
            crates/rodas5p-integrators crates/vigilode/src/lib.rs crates/vigilode/src/automatic.rs \
            crates/vigilode/src/switching.rs crates/vigilode/src/selection.rs \
            crates/vigilode/src/selection crates/vigilode/src/solve.rs \
            crates/vigilode/src/explicit.rs crates/vigilode/src/production.rs \
            crates/vigilode/src/prelude.rs crates/vigilode/Cargo.toml Cargo.toml Cargo.lock \
            rust-toolchain.toml research/LEDGER.jsonl docs/agent \
            research/production_loop_20261116/PREREGISTRATION.md \
            research/production_loop_20261116/PROGRESS.md \
            research/production_loop_20261116/CHECKPOINT.json
        ;;
    *)
        echo "unknown step $step" >&2
        exit 2
        ;;
    esac
done
