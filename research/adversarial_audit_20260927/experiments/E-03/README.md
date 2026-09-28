# E-03 — H1a (0.1 output-policy rule on the same code path / on scipy Radau) and H1b (interpolation attribution)

Inputs: the E-02 run (`../E-02/out/e0203_rows.json`, all 18 n=96 rows, 6 policies each) and the six exact n=96 reference
artifacts. Scripts: `analyze_e03a.py` (E-03a from Rust output), `families.py` (verbatim line-range copy of the six family
RHS/Jacobian definitions from `tree/tools/reference_v2/generate_references{,_v2}.py`; the tree is untouched),
`e03bc_scipy.py` (E-03b control + E-03c attribution), `aggregate_e03.py` (-> `results.json`). Raw scipy output:
`scipy/<family>.json`, `scipy/run_stdout.txt`. Verdicts fixed before running: H1a SUPPORTED if >= 80% of both-correct pairs
violate gap <= 0.1*dense_error; H1b SUPPORTED if interpolated-grid error > 5x accepted-endpoint error in most rows.

All WRMS values below are in the campaign's reference basis (abs 1e-10, rel 1e-8, anchored on the reference), the same
basis as the committed `max_grid_wrms` / `output_policy_discrepancy_wrms`; values in parentheses are the same errors in the
case's own (atol, rtol) weights. "Both-correct" = both arms reference-admissible under the campaign rule
(reference_uncertainty <= 0.1 * max_grid_wrms); all 108 VigilODE arms and 71/72 scipy arms were admissible.

## Verdicts
* **H1a: SUPPORTED.** Same dense code path, two admissible policies:
  initial_step h0 vs 0.7 h0 -> 16/18 pairs violate the 0.1 rule (89%; median gap/error 0.59);
  max_step = span vs grid spacing -> 17/18 (94%; median 0.91).
  Restricting "correct" to case-tolerance max error <= 1 leaves 6 pairs, all 6 violate.
  The clipped-vs-clipped pair (h0 vs 0.7 h0) violates in only 2/18 (median ratio 1.9e-03): clipping
  to the grid makes runs reproducible against each other, not more accurate.
* **Solver-independent control (scipy Radau, analytic sparse Jacobian, atol = 0.01 rtol):** first_step h0 vs 0.7 h0 ->
  17/18 violate (94%; median ratio 1.29, range 0.05-15.4);
  max_step = grid spacing vs unlimited -> 15/17 violate (88%).
  dense_output vs t_eval on the same run -> 18/18 bitwise identical (scipy evaluates t_eval from the same interpolant; it is
  not an independent policy). An established solver therefore does **not** satisfy the 0.1 rule either: the rule compares two
  realisations of the same O(tol) global error, whose difference is O(tol), so the ratio is O(1) by construction.
* **H1b: REFUTED.** Exact accepted-step states of the VigilODE dense_base run (union-schedule run; identical step sequence
  in 18/18 rows) vs a tight Radau (1e-12/1e-14) dense reference (interpolant uncertainty <= 4.0e-03 in the same units;
  the tight rerun's grid states are bitwise identical to the committed artifacts in 6/6 families):
  max error over interpolated grid points / max error over accepted endpoints = median 1.08, range
  0.70-2.43; 0/18 rows exceed 5x. The dense-output interpolation adds at most ~2.4x; the global error is carried by
  the accepted steps themselves. (scipy Radau's own interpolant: median 2.19, 3/18 rows > 5x.)
  The E-01 observation "max_grid / endpoint ~ 50" (here median 42, 18/18 > 5) is a **transient-decay** effect: the error
  peaks mid-interval during each family's ramp and is contracted by the stiff dynamics before t_f (the t_f state is itself an
  accepted endpoint, and its error is 10-5000x below the peak accepted-endpoint error). Endpoint error is therefore not a
  proxy for max-grid error, but interpolation is not the cause.

## Side observation (not a hypothesis; evidence for the report)
At the same (rtol, atol) and same problems, VigilODE's dense global max error is larger than scipy Radau's in 17/18 rows
(median ~2.6x; semilinear rtol 1e-4: 1.70e5 vs 2.41e3 = 70x; semilinear is 17-207x beyond case tolerance at every rtol
in VigilODE, 0.02-0.5x in scipy). The solver's own accepted embedded error norms are <= 1 in every arm (E-02), so this is
global-error growth under satisfied local control, not a controller violation.

## Per-row table
| case | VigilODE dense_base maxT (caseTol) | h0 pair ratio | max_step pair ratio | scipy A maxT (caseTol) | scipy A/B ratio | scipy A/D ratio | VigilODE accepted-max | grid-interior max | ratio |
|---|---|---|---|---|---|---|---|---|---|
| robertson-ramped-rtol-1e-4 | 2.72e+03 (0.27) | 1.11 | 0.98 | 1.66e+03 (0.17) | 1.34 | 1.00 | 2.01e+03 | 2.72e+03 | 1.35 |
| robertson-ramped-rtol-1e-6 | 370 (3.7) | 0.67 | 1.00 | 82.5 (0.82) | 1.37 | 1.00 | 163 | 370 | 2.27 |
| robertson-ramped-rtol-1e-8 | 2.28 (2.3) | 0.63 | 0.12 | 0.407 (0.41) | 0.94 | 1.03 | 2.25 | 2.28 | 1.01 |
| hires-ramped-rtol-1e-4 | 3.92e+03 (0.39) | 1.52 | 1.00 | 1.14e+03 (0.11) | 1.74 | 1.00 | 2.73e+03 | 3.92e+03 | 1.43 |
| hires-ramped-rtol-1e-6 | 91.8 (0.92) | 1.19 | 1.00 | 19 (0.19) | 1.25 | 0.86 | 74.9 | 91.8 | 1.23 |
| hires-ramped-rtol-1e-8 | 0.409 (0.41) | 0.86 | 1.03 | 0.177 (0.18) | 1.56 | 0.47 | 0.342 | 0.409 | 1.20 |
| van-der-pol-ramped-rtol-1e-4 | 7.97e+03 (0.8) | 0.98 | 0.88 | 2.91e+03 (0.29) | 1.38 | 1.00 | 4.88e+03 | 7.97e+03 | 1.63 |
| van-der-pol-ramped-rtol-1e-6 | 255 (2.6) | 0.85 | 0.91 | 41.5 (0.41) | 0.90 | 1.42 | 162 | 255 | 1.57 |
| van-der-pol-ramped-rtol-1e-8 | 0.51 (0.51) | 0.19 | 1.03 | 0.566 (0.57) | 1.19 | 1.00 | 0.21 | 0.51 | 2.43 |
| rotating-nonnormal-rtol-1e-4 | 1.39e+04 (1.4) | 0.54 | 0.95 | 2.94e+03 (0.29) | 0.05 | 1.00 | 1.25e+04 | 1.39e+04 | 1.11 |
| rotating-nonnormal-rtol-1e-6 | 365 (3.7) | 0.12 | 0.59 | 28.3 (0.28) | 1.53 | 0.88 | 520 | 365 | 0.70 |
| rotating-nonnormal-rtol-1e-8 | 1.62 (1.6) | 0.05 | 0.00 | 0.141 (0.14) | 2.36 | 0.00 | 2.29 | 1.62 | 0.71 |
| nonautonomous-stiff-forcing-rtol-1e-4 | 4.22e+04 (4.2) | 0.51 | 0.74 | 8.48e+03 (0.85) | 1.74 | 0.97 | 4.09e+04 | 4.22e+04 | 1.03 |
| nonautonomous-stiff-forcing-rtol-1e-6 | 233 (2.3) | 0.18 | 0.11 | 36.7 (0.37) | 1.11 | 0.99 | 223 | 233 | 1.05 |
| nonautonomous-stiff-forcing-rtol-1e-8 | 2.28 (2.3) | 0.14 | 0.10 | 0.261 (0.26) | 0.79 | 0.00 | 2.18 | 2.28 | 1.05 |
| semilinear-advection-diffusion-ramped-grid8x12-rtol-1e-4 | 1.7e+05 (17) | 2.18 | 0.79 | 2.41e+03 (0.24) | 15.40 | 1.00 | 1.67e+05 | 1.7e+05 | 1.02 |
| semilinear-advection-diffusion-ramped-grid8x12-rtol-1e-6 | 5.32e+03 (53) | 0.11 | 1.00 | 157 (1.6) | 0.58 | 1.00 | 5.31e+03 | 5.32e+03 | 1.00 |
| semilinear-advection-diffusion-ramped-grid8x12-rtol-1e-8 | 207 (2.1e+02) | 0.00 | 0.91 | 0.473 (0.47) | 0.38 | 0.95 | 207 | 207 | 1.00 |

Columns: maxT = max_grid_wrms (tight basis), caseTol = same in case weights; "ratio" columns = gap / max_grid_wrms of the
left arm; last three columns = E-03c (VigilODE dense_base): max WRMS at accepted step endpoints, max WRMS at interpolated
(non-coincident) grid points, and their ratio.
