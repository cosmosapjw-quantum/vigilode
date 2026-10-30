# Reproduce original BASS21 eigenmode/shear-memory data

1. Extract `VIGILODE_BASS21_EIGENMODE_SHEAR_MEMORY_20261030_FULL_SOURCE.zip` to a fresh EMPTY directory. Preserve nested source ZIP bytes.
2. Python 3.13.x (checked 3.13.x), NumPy 2.3.5, SciPy 1.17.0, SymPy 1.14.0, Matplotlib 3.10.x for plots. Exact float bytes on other versions NOT VERIFIED.
3. Run from extracted root: `export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1; bash VERIFY.sh`.
4. The verifier hashes this payload manifest, checks unchanged nested PR141 parent ZIP SHA256, recursively extracts/validates original BASS owner SHA from immutable nesting, checks original 50 NPZ source SHA per prior source receipt, runs exact/self-unit 10/10 tests, then re-evaluates frozen 50-case Volterra Q/I and J4, verifies all 6 saved deterministic outputs **byte-exact**.
5. `python3 code/plot_science.py` regenerates 5 PNG and 5 SVG plots from the actual new NPZ.
6. `results/EIGENMODE_50x193_SOURCE_PHYSICS.npz` has exact U,V, collision/streaming sources, causal shear-memory and source-clock derivative diagnostics. `EXACT_ALGEBRA.json` stores definitions. No further physical or chemistry IVPs, no source replacement and no post hoc correction of measured time-grid mismatches.

Source owner: `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`, SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`. Parent PR141 source ZIP SHA256 `ce2b3507c30f5429fb93244bd7f3de7426f1f158bf52962e24734b6d1237dfe2`.
