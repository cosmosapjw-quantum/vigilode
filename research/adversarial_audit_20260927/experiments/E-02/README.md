# E-02 — Reproduce the 18 n=96 scientific-validity-v2 calibration rows (H1 reproduction)

Target tree: b3e8165 (detached, read-only). Committed campaign: code_revision ab8fbcd (records in `../E-01/calibration_all_cases_compact.json`).

## Code path
`run_scientific_validity_v2_case` (scientific_validity_v2_campaign.rs:599) refuses any reference bundle whose
`implementation_revision` (ab8fbcd in the committed manifest) differs from the compiled revision (line 633), so it
cannot be called on this tree. The harness (`harness/src/bin/e0203_v2rows.rs`) therefore replicates `execute_arm`
(scientific_validity_v2_campaign.rs:428-560) verbatim for single-segment calibration cases: same public entry points
(`integrate_sequential_matrix_free_adaptive_observed` / `_dense_observed`), same `LinearSolverConfig`
(GMRES restart 32, maxiter 256, preconditioner none, initial guess previous, fallback rtol 1e-10 / atol 1e-12), same
`AdaptiveStepConfig` (integral controller 0.9/0.2/5/0.9, `initial_step = span/100`, `min_step = 1e-12`,
`max_step = span`, 200 000 attempts), same `OutputSchedule`, same `wrms_basis.metrics` / `discrepancy_wrms`
and the same admission order (`classify_reference_dominance` then `classify_output_policy_dominance`), and the
campaign's output-checksum formula (`vigilode-scientific-v2-mode-output-v1`).

## Commands
```
bin/cargo-wrapped.sh build --profile measurement --bin e0203_v2rows
RAYON_NUM_THREADS=1 cargo-target/measurement/e0203_v2rows --manifest exp/E-02/refdir/reference_manifest.json --out exp/E-02/out/e0203_rows.json
python3 exp/E-02/analyze_e02.py     # -> results.json
```
Raw run log: `out/run_stderr.txt`; full per-arm output (101-grid states, diagnostics, counters): `out/e0203_rows.json`.

## Result: PASS (bitwise reproduction)
| quantity | committed vs re-run |
|---|---|
| campaign output checksum (clipped, dense; 36 arms) | 36/36 bitwise equal |
| `output_policy_discrepancy_wrms` (18 rows) | 18/18 bitwise equal (max rel diff 0.0) |
| clipped/dense `max_grid_wrms`, `endpoint_wrms`, `rms_grid_wrms` | max rel diff 0.0 |
| internal steps, rhs_calls, jvp_calls, accepted/rejected | 18/18 rows identical |
| row status | 18/18 `output-policy-dominated` (as committed) |
| policy config (initial_step, max_step, min_step) | 18/18 identical |
| reference checksum binding | 18/18 identical |

So the Rust source between ab8fbcd and b3e8165 produces byte-identical campaign arms for these rows on this
machine (deterministic, single-threaded).

## Solver's accepted error norms vs measured global WRMS (important unit caveat)
* The committed/measured `*_wrms` metrics use the reference `wrms_basis` = **tight Radau basis**
  (absolute 1e-10, relative 1e-8, anchored on the reference states; global_error.rs:117-126, artifact
  `convergence.wrms_basis`). They are NOT normalised by the case tolerance. A dense `max_grid_wrms` of 2722 at
  rtol 1e-4 is ≈ 0.27 in the case's own (atol=1e-6, rtol=1e-4) weights. The FINDER_BRIEF / E-01 reading
  "WRMS > 1 == beyond tolerance" is therefore only literally true for the rtol=1e-8 rows (where the two bases coincide).
* Re-expressed in case-tolerance weights (see `../E-03/e03a_rows.json`, field `max_grid_wrms_casetol`):
  dense_base max grid error ≤ 1 in 6/18 rows, clipped_base in 11/18 rows; the semilinear family is far beyond
  tolerance at every rtol (17, 53, 207), robertson/rotating/forcing/vdp are 1.4–4.2 at rtol ≤ 1e-6.
* The solver's own control quantity (max embedded error norm over accepted steps, `diagnostics.error_norms`) is
  ≤ 1.0 in all 36 arms (range 0.003–0.997). Median ratio measured tight-basis max_grid_wrms / solver max accepted
  norm: dense 328, clipped 140 — but this ratio mixes the two bases; the case-tolerance numbers above are the
  meaningful comparison: the local error control is satisfied while the global error exceeds tolerance in 12/18
  dense rows, by up to 207x (semilinear, rtol 1e-8).

Per-row table: `results.json` → `rows[]` (committed/rerun values, rel diffs, solver norms).
