#!/usr/bin/env bash
set -euo pipefail
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
export RAYON_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 NUMEXPR_NUM_THREADS=1
base=/mnt/data/rodas5p-baseline-v0.1.0-alpha1
curr=/mnt/data/rodas5p-rs-recovered/.worktrees/rust-adaptive/target/release/rodas5p
trace_root=/mnt/data/rodas5p-rust-traces-v0.1
out=/mnt/data/rodas5p-rs-recovered/.worktrees/rust-adaptive/research/results/r10a_paired_raw_exact
mkdir -p "$out"
for rep in 0 1 2; do
  seed=$((20260806 + rep))
  for n in 64 128 256; do
    for kind in fixed slow-drift abrupt rotating; do
      if (( (rep + n + ${#kind}) % 2 == 0 )); then
        versions='baseline current'
      else
        versions='current baseline'
      fi
      for version in $versions; do
        if [[ $version == baseline ]]; then bin=$base; else bin=$curr; fi
        timeout 120s taskset -c 0 "$bin" benchmark \
          --trace "$trace_root/n${n}/${kind}_trace.json" \
          --output "$out/${version}_r${rep}_n${n}_${kind}.json" \
          --repetitions 1 --warmups 1 --seed "$seed" \
          --restart 20 --recycle-dim 6 --operator-budget 2000 \
          --rtol 1e-9 --atol 1e-12
      done
    done
  done
done
