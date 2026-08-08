#!/usr/bin/env bash
set -euo pipefail
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
export RAYON_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 NUMEXPR_NUM_THREADS=1
bin=/mnt/data/rodas5p-rs-recovered/.worktrees/rust-adaptive/target/release/rodas5p-allocation-audit
root=/mnt/data/rodas5p-rust-traces-v0.1
out=/mnt/data/rodas5p-rs-recovered/.worktrees/rust-adaptive/research/results/r10a_allocations_exact
mkdir -p "$out"
for n in 64 128 256; do
  for kind in fixed rotating; do
    timeout 120s taskset -c 0 "$bin" \
      --trace "$root/n${n}/${kind}_trace.json" \
      --output "$out/n${n}_${kind}.json" \
      --repetitions 3 --warmups 1 \
      --restart 20 --recycle-dim 6 --operator-budget 2000 \
      --rtol 1e-9 --atol 1e-12
  done
done
