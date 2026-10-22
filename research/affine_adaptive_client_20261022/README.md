# Opt-in affine30 BDF client

This additive research unit executes the unchanged BASS compact33 owner in 30 affine coordinates using a local SciPy 1.17.0 BDF subclass and the unchanged VigilODE C99 full33 lifted metric. It does not install a global monkeypatch or modify production solvers. It is not a native Rust hook.

The one completed actual matched pair on x=[0,0.01] passes: maximum sampled full33 WRMS discrepancy 7.222972526677901e-08 against the frozen <=1 bound; all 13 inherited gates pass for each trajectory. Nine common samples, both endpoints, signed stresses, all 21 species and tau are retained in `evidence/pair_002/trajectory.npz` and `RESULT.json`. Physical HOLD persists. There is no speed claim.

The original pair process saved the raw NPZ, then failed serializing NumPy scalar metadata. Numerical results were recovered from that NPZ without another cosmological integration. Counters and the common first-step value were lost before being flushed and remain explicitly UNKNOWN. The instrumented client supports these counters for future runs. The process duration 5.422 seconds includes fixture, both trajectories, diagnostics and failed serialization; it is not an isolated solve benchmark. Failures and their diagnoses are retained. Exactly two cosmological IVPs completed, constituting one matched pair; zero full-interval/cohort integrations were run.

The persisted manufactured check injects a C status-1 invalid scale during order adaptation, after upstream temporary accepted-state assignments. It verifies rollback of t, y, D, h_abs, J, LU, order and n_equal_steps with zero accepted steps. The serializer check roundtrips actual NumPy boolean metadata. Neither check repeats a scientific trajectory.

## Execute a new, explicitly requested run

Requirements: Python 3.12-compatible environment, NumPy, SciPy exactly 1.17.0 with source matching `vendor/scipy_bdf_1_17_0.py`, C99 compiler, and sufficiently wide `long double`. Unsupported C arithmetic fails closed. Source/license and dependency hashes are in `SOURCE_MANIFEST.json`; copied owner dependencies and data are self-contained under `vendor/bass_compact`. No prior large trajectories are bundled.

```sh
cc -std=c99 -O2 -Wall -Wextra -Werror -shared -fPIC code/lifted_error_norm.c -lm -o code/liblifted_error_norm.so
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 timeout 30s python code/run_pair.py --output evidence/new_explicit_run
```

An optional `--baseline /path/to/rei_hheli_compact_pilot_20261022` selects an explicit unchanged owner directory instead of the bundled one. Output paths must not already exist. The fixture runs before the pair. The initial step is selected on full33 using SciPy's unchanged selector, then passed explicitly to both solvers. All adaptive reduced decisions use tangent errors and full33 state scales. `code/inherited_gates.py` copies original gate expressions unchanged; `code/recover_receipt.py` is a recorded recovery operation, not a general run command.

See `CONTRACT.md`, `THEORY.md`, `RUN_RECORD.json`, `DAG.json`, `EXECUTION_LEDGER.json`, and `AUTHORITY_INTAKE.json` for scope and provenance. Independent review in `INDEPENDENT_REVIEW_REI_AUTHORITY.md` passes the scoped adaptive hook and saved finite pair. Work-count reporting remains incomplete because the counters are UNKNOWN. Publication remains parent-managed.
