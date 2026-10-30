1. Unzip full archive into an empty directory, preserving `source/PR148_SOURCE_FULL.zip` unchanged.
2. Recommended source code execution environment: Python>=3.10 with NumPy, SciPy, SymPy, Matplotlib, pytest.
3. From the ZIP root run `OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 OMP_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1 bash VERIFY.sh`.
4. `VERIFY.sh` checks SHA256 of every included file except the manifest itself; exact nested original source SHA; 24 targeted physical tests; independently regenerates 1 NPZ + 3 CSV + summary JSON + 3×PNG + 3×SVG = **11 science files** with byte-by-byte hashes. No owner IVP.
5. `python code/spin2_quadratic_budget.py --source source/PR148_SOURCE_FULL.zip --output results` runs the original source-frozen diagnostic; do not execute into sealed source trees for verification, use a fresh temp folder.
6. Original source states/generator from PR145 and opacity/bprime from PR143; all comparisons bind 50 cases, 193 epochs and original normalized E conventions. The parent PR148 archive SHA256 is `626ea52c9fae38db23e0dd9e5f628ed0cba35ee05d54391b02b91ca34a0b3b1c`.
7. Stage JSON files record actual bounded work per stage and shared archive/Git destinations; are not twenty independent physics experiments. Distinct-author R6/C9 and native subagent NOT_RUN.
