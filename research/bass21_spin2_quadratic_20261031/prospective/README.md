# VigilODE BASS21 | Prospective spin-2 angular-shape alignment

Only the **subsequent** alignment calculation is prospectively registered: Git commit `553ab8afd7f2d699ef0b2b172375860bdbbfefa1`, recorded before execution. The prior 231,600-value quadratic angular-power/work study was completed **without prospective registration** and remains exploratory/post-hoc; not reclassified by this package.

Genuine numerical source output: 50 original LRS BASS21 histories ×193 source epochs ×4 cutoffs (L=16,18,20,22), 38,600 eta values and 38,600 alignment values with 200 source-defined null first-epoch entries; 38,400 valid each. Original raw source and prior full science package are **nested unchanged** in `source/PR151_ORIGINAL_SOURCE_FULL.zip` and must hash to `e7e57f937eea61a225b47de91aab7fc79cb394c3b287dc02529decb829573383`.

Science: squared spin-2 linear-polarization angular pattern Q_L(mu)^2, eta=<mu^2>, alignment=1−3eta. This is not photon energy-density, observed CMB E/B, actual truncation error, observer boost map, or dynamically evolved E_l. Frozen low-shear s00 high multipoles inherit quadrature limitations.

Reproduce in a clean directory: `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1 bash VERIFY.sh`. Requires Python 3 with NumPy, SciPy, Matplotlib, pytest. `code/compute_alignment.py` computes entire source-conditional original data with no owner IVP.

Preservation: source-full zip, reproducibility, original provenance, prereg SHA, 21 source-targeted tests, 7 byte-matched CSV/NPZ/SVG/PNG results; 36 previously authored Git source-free PR151 blobs remain intact; new Git results are tracked separately under `prospective/` and a new append-only ledger row.
