#!/bin/sh
# Discriminator D3 of node V22 (PREDECLARED.json): format, lints and the suites around the
# banded drivers and the facade. usage: verify.sh <cargo target directory> <tag>
# Writes logs/verify_<tag>_<step>.log and logs/verify_<tag>_summary.log (step, exit code).
set -u
target=$1
tag=$2
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
cd "$root" || exit 2
export CARGO_TARGET_DIR="$target"
summary="$here/logs/verify_${tag}_summary.log"
: > "$summary"
step() {
    name=$1
    shift
    "$@" > "$here/logs/verify_${tag}_${name}.log" 2>&1
    code=$?
    echo "$name exit=$code" >> "$summary"
}
pin="taskset -c 0-5,12-17 nice -n 5"
step fmt $pin cargo +1.94.1 fmt -p vigilode -p rodas5p-integrators -- --check
step clippy_facade $pin cargo +1.94.1 clippy --locked -p vigilode --all-targets --release --no-deps -j 4 -- -D warnings
step clippy_integrators $pin cargo +1.94.1 clippy --locked -p rodas5p-integrators --lib --test banded_kernel_entry_identity --release --no-deps -j 4 -- -D warnings
step integrators $pin cargo +1.94.1 test --locked -p rodas5p-integrators --release --no-fail-fast -j 4 \
    --lib --test banded_kernel_entry_identity --test 'stage_clients_*' \
    --test ppsw1_6b_contracts --test ptsw2b_contracts \
    --test int03_banded_fast --test int03_banded_fast_contracts --test spd03_banded_arm \
    --test fast_driver_golden_digest_contracts --test fast_dense_contracts \
    --test fast_vector_atol_contracts --test banded_recovery_contract \
    --test accept_control_contracts --test declared_structure_routing \
    --test client_stage_system_contracts --test p0_partial_contracts
step facade $pin cargo +1.94.1 test --locked -p vigilode --release --no-fail-fast -j 4
echo "done" >> "$summary"
