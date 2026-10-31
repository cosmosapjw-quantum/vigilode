# Executable clean-room reproduction

Extract `VIGILODE_BASS21_NONAXIAL_OBSERVER_EB_20261031_FULL_SOURCE.zip` in a fresh directory and run:

```bash
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 MPLBACKEND=Agg PYTHONDONTWRITEBYTECODE=1 bash VERIFY.sh
```

Requirements Python3.13, NumPy 2.3.5, SciPy 1.17.0, SymPy 1.14, Matplotlib, pytest (the local run also used default BLAS single-thread). The source-full ZIP includes exactly unchanged `source/PR153_FULL_SOURCE.zip`, recursively including PR151/PR148/PR145 owner arrays, not just Git text exports. `VERIFY.sh` verifies first manifest+input source checksum, executes test suite with exact source, regenerates NPZ/CSV/science plots, and compares before/after result SHA. Byte-identical figures are same-environment target; cross-host physically justified angular/quadrature discrepancies are the scientific check.

Git source-free mirror intentionally omits original PR153 ZIP, source-binary NPZ and PNG, but publishes actual physical CSVs, two science SVGs, code, tests, reports, and full source/hash lineage. Replaying only the Git mirror without the original source is **not** a complete original-source experiment.
