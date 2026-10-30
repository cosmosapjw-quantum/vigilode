# Clean reproduction | source-owned BASS21 radiation stress

Prerequisite: Python 3.11+ with `numpy`, `scipy` (same NumPy/SciPy semantics as archived parent), source-inclusive ZIP **extracted in an empty directory**, memory <2 GiB, CPU ~seconds to tens of seconds. No network, no new initial-value solve of original geometry/chemistry.

```bash
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
bash VERIFY.sh
```

The script verifies all SHA256 manifest entries, confirms exact nested PR132 source and 50 original BASS21 histories, runs 9 new tests, independently replays full 50×193 on GL48/GL64/GL96, checks full NPZ numerical arrays and byte-deterministic CSVs, and prints bounded model diagnostics. `VERIFY.sh` on Git text mirror alone **will not work** without the full nested parent binary ZIP.

Manual re-run:

```bash
python code/radiation_stress_atlas.py \
 --parent-zip parent/VIGILODE_BASS21_COUPLED_THOMSON_20261030_FULL_SOURCE.zip \
 --out /tmp/stress_result
python code/export_physics_csv.py --results /tmp/stress_result
```

Outputs: full 57,900-row delta J_l CSV, 28,950-row matched-pair moments CSV, 9,650-row time-resolved stress/work CSV, endpoint 50/25 CSV, provenance/audit CSV, NPZ full numeric records, `results/SCIENCE_SUMMARY.json`, Wolfram and blind referee theory in `reports/`. Independent-author referee NOT_RUN. No observational CMB, no new source BDF error claim.