#!/bin/bash
set -u
repo=/home/cosmosapjw/worktrees/vigilode-csas-20261125
dest=/home/cosmosapjw/vigilode-preserved/20261125-adversarial-rebaseline
export CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-adversarial-rebaseline-20261125
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 RAYON_NUM_THREADS=1 JULIA_NUM_THREADS=1
cd "$repo"
run(){
  local name="$1";shift
  printf '%s\t%s\t' "$name" "$(date -Is)" >> "$dest/receipts/commands.tsv";printf '%q ' "$@" >> "$dest/receipts/commands.tsv";printf '\n' >> "$dest/receipts/commands.tsv"
  "$@" > "$dest/receipts/$name.log" 2>&1
  local rc=$?
  printf '%s\t%s\t%s\n' "$name" "$rc" "$(date -Is)" >> "$dest/receipts/exit.tsv"
  echo "$name EXIT=$rc"
  return "$rc"
}
# First compile standalone Rust/C/Fortran benchmark on the pinned original compiler.
if run build_cross timeout 360 cargo +stable build --locked --offline -j 8 --release --manifest-path crates/vigilode-bench/Cargo.toml; then
  sha256sum "$CARGO_TARGET_DIR/release/vigilode-bench" >> "$dest/receipts/BINARY_SHA256SUMS"
  run cross_rust timeout 90 taskset -c 0 "$CARGO_TARGET_DIR/release/vigilode-bench" wp --corpus "$dest/harness/cross_solver_v1_valid.json" --out "$dest/run/cross_solver_v1_rust.json" --source-root "$repo"
  run spatial_rust timeout 120 taskset -c 0 "$CARGO_TARGET_DIR/release/vigilode-bench" wp --corpus "$dest/harness/adversarial_spatial_battery_v3.json" --out "$dest/run/spatial_battery_v3_rust.json" --source-root "$repo"
fi
# Native extended suite. Exact old profile and original physics/solver arms.
if run build_full timeout 420 cargo +stable build --locked --offline -j 8 --profile measurement --manifest-path "$dest/harness/Cargo.toml"; then
  sha256sum "$CARGO_TARGET_DIR/measurement/full-scope-suite" >> "$dest/receipts/BINARY_SHA256SUMS"
  run full_rust timeout 240 taskset -c 0 "$CARGO_TARGET_DIR/measurement/full-scope-suite" "$dest/run/full_scope_rust.json"
  run full_ens timeout 90 "$CARGO_TARGET_DIR/measurement/full-scope-suite" "$dest/run/full_scope_ensemble_unpinned.json" --ensemble-only
fi
# Opt route battery. Prior example compiler not independently pinned; current workspace's pinned compiler.
if run build_opt timeout 240 env CARGO_TARGET_DIR=/home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/kschain cargo +1.94.1 build --locked --offline -j 8 --release -p vigilode --example opt_baseline_battery; then
  sha256sum /home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/kschain/release/examples/opt_baseline_battery >> "$dest/receipts/BINARY_SHA256SUMS"
  printf '%s\n' 'opt_battery taskset -c 0 existing exact example; stdout JSON, stderr separate' >> "$dest/receipts/commands.tsv"
  timeout 180 taskset -c 0 /home/cosmosapjw/.cargo-targets/vigilode-stage-lattice-20261124/kschain/release/examples/opt_baseline_battery > "$dest/run/opt_battery_head.json" 2> "$dest/receipts/opt_battery.log"
  rc=$?;printf 'opt_battery\t%s\t%s\n' "$rc" "$(date -Is)" >> "$dest/receipts/exit.tsv";echo "opt_battery EXIT=$rc"
fi
# Reuse archived reference values so external comparator input stays identical.
run cross_julia timeout 240 taskset -c 0 julia --startup-file=no --project=tools/benchmarks/component_julia bench/julia_comparator_bench.jl --reference "$dest/original/cross_solver_v1_rust.json" --out "$dest/run/cross_solver_v1_julia.json"
run full_julia timeout 300 taskset -c 0 julia --startup-file=no --project=tools/benchmarks/component_julia "$dest/harness/full_scope_julia.jl" "$dest/run/full_scope_julia.json" "$dest/original/full_scope_rust.json"
run full_scipy timeout 90 taskset -c 0 python3 "$dest/harness/full_scope_scipy.py" "$dest/run/full_scope_scipy.json" "$dest/original/full_scope_rust.json"
echo RUN_ALL_FINISHED
