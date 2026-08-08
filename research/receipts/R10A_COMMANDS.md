# R10-A command receipt

## Strict paired benchmark

Exact script: `research/receipts/R10A_PAIRED_RUN.sh`

Common environment:

```bash
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
export RAYON_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 NUMEXPR_NUM_THREADS=1
```

Each binary was invoked under `taskset -c 0` and a per-invocation `timeout 120s` guard with:

```bash
rodas5p benchmark \
  --trace <n{64,128,256}/{fixed,slow-drift,abrupt,rotating}_trace.json> \
  --output <paired-output.json> \
  --repetitions 1 --warmups 1 --seed <20260806+rep> \
  --restart 20 --recycle-dim 6 --operator-budget 2000 \
  --rtol 1e-9 --atol 1e-12
```

Three paired repetitions were run. Baseline/current order alternated deterministically.
Exit status: `0`.

## Allocation audit

Exact script: `research/receipts/R10A_ALLOCATION_RUN.sh`

```bash
taskset -c 0 rodas5p-allocation-audit \
  --trace <n{64,128,256}/{fixed,rotating}_trace.json> \
  --output <allocation-output.json> \
  --repetitions 3 --warmups 1 \
  --restart 20 --recycle-dim 6 --operator-budget 2000 \
  --rtol 1e-9 --atol 1e-12
```

Exit status: `0`.

## Raw archive

```text
RODAS5P_Rust_R10A_Raw_v0.2.0-alpha1.tar.zst
SHA-256 2650b170a7c06b20c5e5b77f47c216128ef2a52b113d94bbc20cb981386b00fc
```
