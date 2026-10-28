# Coupled matrix-free/stage-parallel/homotopy exploration

Read REPORT_KO.md and SOURCES.json. Research source pinned to c03ca90f19a9e64ba7751c069325a48b54b39c2e. The raw coefficient subset is transcribed from the pinned decimal fixture, not extracted from a native compiled tableau; no native-K bit-identity claim.

Run locally with Python, NumPy, SciPy:

```sh
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 probe.py
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 projected_parallel.py
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 kernel_parallel.py
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 coordinate_probe.py
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -m unittest -v test_probe
```

`probe.py`:30small homotopy studies, determinant/causal-transient checks, matrix-free Arnoldi affine predictor. The small nonlinear-path seed is an explicitly disclosed reference oracle; its cost is not included.

`projected_parallel.py`:constructs its own basis and seed using operator actions. The candidate stores only V(n,m), projected J and an8m small stage matrix. The synthetic three-mode diagonal quadratic model closes in m=3. Actual thread overlap is recorded; no solver acceleration is claimed. Projected-derivative vector-equivalent work is reported separately from basis JVP calls. Independent sequential stage root is used ONLY to validate the result.

`kernel_parallel.py`:12block kernel-only sequential/threaded comparison. It does not benchmark full nonlinear solves, the native Rust scheduler, or application speed.

`test_probe.py`:13bounded tests; TESTS.log contains the actual execution. Most tests concern mathematical/code identities, not every physical regime.

The scripts overwrite result JSONs on rerun; copy this directory to a new run directory before rerunning so historical measurements remain intact. Outputs are full small-probe traces; no datasets or toolchain binaries are required. Only SUMMARY/METADATA refer to this execution, not to a user's repository test status.
