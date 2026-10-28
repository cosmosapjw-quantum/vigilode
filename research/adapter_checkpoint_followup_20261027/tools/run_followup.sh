#!/usr/bin/env bash
# Registered follow-up run (PREREGISTRATION.md). Usage: run_followup.sh OUT_DIR BASE_TRACE BASE_PILOT
# BASE_* are the trace/pilot examples built from dda8d2f1's library (see PREREGISTRATION.md, M2).
set -euo pipefail
OUT=$1; BASE_TRACE=$2; BASE_PILOT=$3
ROOT=$(git rev-parse --show-toplevel); cd "$ROOT"
N=research/adapter_checkpoint_followup_20261027
T=${CARGO_TARGET_DIR:-$ROOT/target}
export RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 CARGO_INCREMENTAL=0
R=research/joint_controller_20261025/tools/run_recorded.py
mkdir -p "$OUT"
python3 $R --output "$OUT/tests_vigilode" --timeout 1800 -- cargo test --locked --offline -p vigilode --lib --tests
python3 $R --output "$OUT/tests_integrators" --timeout 1800 -- cargo test --locked --offline -p rodas5p-integrators --test declared_structure_routing --test p0_partial_contracts
python3 $R --output "$OUT/tests_krylov" --timeout 1800 -- cargo test --locked --offline -p rodas5p-krylov --test spd04_ls_workspace
python3 $R --output "$OUT/build" --timeout 1800 -- cargo build --profile measurement --locked --offline -p vigilode --example adapter_integration_pilot --example adapter_checkpoint_trace
mkdir -p "$OUT/bin"
cp "$T/measurement/examples/adapter_integration_pilot" "$T/measurement/examples/adapter_checkpoint_trace" "$OUT/bin/"
cp "$BASE_TRACE" "$OUT/bin/base_adapter_checkpoint_trace"; cp "$BASE_PILOT" "$OUT/bin/base_adapter_integration_pilot"
sha256sum "$OUT"/bin/* > "$OUT/BINARY_SHA256.txt"
taskset -c 11 python3 $N/tools/run_pairs_followup.py "$OUT/bin/adapter_integration_pilot" "$OUT/identity" --identity
taskset -c 11 python3 $N/tools/alloc_probe.py "$OUT/bin/base_adapter_checkpoint_trace" "$OUT/bin/adapter_checkpoint_trace" "$OUT/ALLOC.json"
taskset -c 11 python3 $N/tools/run_pairs_followup.py "$OUT/bin/adapter_integration_pilot" "$OUT/pairs"
taskset -c 11 python3 $N/tools/cross_binary.py "$OUT/bin/base_adapter_integration_pilot" "$OUT/bin/adapter_integration_pilot" "$OUT/cross_binary"
taskset -c 11 python3 $N/tools/run_checkpoint_traces.py "$OUT/bin/adapter_checkpoint_trace" research/adapter_integration_20261027/benchmark/raw_r2 "$OUT/traces"
python3 $N/tools/analyze_followup.py "$OUT" "$OUT/RESULTS.json"
