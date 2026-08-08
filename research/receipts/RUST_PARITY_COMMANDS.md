# Rust parity benchmark command contract

All canonical timings were run with:

```bash
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
export RAYON_NUM_THREADS=1 FAER_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1
export NUMEXPR_NUM_THREADS=1

taskset -c 0 target/release/rodas5p benchmark \
  --trace <immutable-trace.json> \
  --output <benchmark.json> \
  --repetitions 5 --warmups 1 --seed 20260806 \
  --restart 20 --recycle-dim 6 --operator-budget 4000 \
  --rtol 1e-9 --atol 1e-12 --preconditioner none
```

Jacobi runs changed only `--preconditioner jacobi`.  Zero-guess runs changed only `--zero-guess`.
