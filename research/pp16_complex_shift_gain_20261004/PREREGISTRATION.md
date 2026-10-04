# PP16 — native directed certificate for complex shifts with inverse gain |gamma| / Re gamma (prospective registration)

RVJ DAG node `PP16` (depends on PP02). Base commit: the commit that adds
this file. The theorem is not new: Loop02 note section 10, T2.9 (archived,
not in this repository) proves it for an H-dissipative J. Only its native
directed use is new here; H = I only.

## Statement used (H = I)

If `J + J^T <= 0`, `h >= 0` and `Re gamma > 0`, then for every x,
`Re <x, x/gamma - h J x> >= Re(1/gamma) ||x||^2 = (Re gamma / |gamma|^2)
||x||^2`, so `||(I - gamma h J) x|| >= (Re gamma / |gamma|) ||x||` and
`||(I - gamma h J)^{-1}||_2 <= |gamma| / Re gamma`. For a candidate `u`,
`||x - u||_2 <= (|gamma| / Re gamma) ||r||_2` with `r = b - (I - gamma h J) u`.
The real-positive gain 1 is not used for complex gamma.

## Implementation (opt-in, new module)

- `certify_complex_shift_candidate(J, h, gamma_re, gamma_im, b_re, b_im,
  u_re, u_im, tol)`: dissipativity row witness as in PP01; outward
  `Re gamma > 0` (else error); the residual's real and imaginary parts in
  interval arithmetic from the exact binary inputs; bound
  `gain_up * ||r||_2,up` with `gain_up` an upward enclosure of
  `|gamma| / Re gamma`; Certified iff bound <= tol.
- `complex_shift_jet(J, h, gamma0, B, gammas_complex, degree)`: candidates
  from the real normalized jet evaluated at complex `z = (gamma -
  gamma0)/gamma0`, refused unless outward `|z| < 1`; each candidate
  certified by the function above.
- `certify_partial_fraction(c0, weights, ...)`: output `y = c0 b + sum_i
  w_i u_i` computed in interval arithmetic; bound `sum_i |w_i|_up err_i +
  radius(y)`; conjugate pairs are not merged unless both members are
  present.

## Fixtures (fixed now)

Seeded dissipative J, n in {4, 6, 16, 24}, h in {0.05, 0.5}; shifts
`gamma_i = 1 / p_i` with `p_i` the binary64-rounded denominator roots of
the [3/3] and [6/6] Pade approximants of `e^z` (one real plus one conjugate
pair; three conjugate pairs), and their partial-fraction weights. This is a
3- and 6-pole proxy for the RVJ structure, not the archived RVJ shifts
(stage 02 is not in the repository). Plus random complex shifts with
`Re gamma` in [1e-3, 2] and `|Im gamma|` up to 5.

Negative controls: `Re gamma = 0`, `Re gamma < 0`, a non-dissipative J,
complex `z` with `|z| >= 1` for the jet, nonfinite inputs, a wrong
candidate.

## Gate

G1 Exact-rational (n <= 6) and 50-digit mpmath (n = 16, 24) oracles: every
   reported bound (single shift and partial fraction) is at least the
   actual Euclidean error of the exact binary target.
G2 All negative controls fail closed (error or Rejected, never Certified).
G3 Prediction: among the random complex shifts, at least one case has an
   actual error above `||r||_2` (so gain 1 would have under-bounded),
   recorded verbatim; if none occurs the gain is still used and the
   prediction is reported as not observed.
G4 Contract tests pass; fmt and clippy clean.

## Results (append only after the recorded run)

Recorded run at source commit `ba3dbbd` (implementation, tests and tools, no outputs), from the repository root
with `CARGO_TARGET_DIR=/home/user/target-pp16`, Rust 1.94.1, Python 3 with mpmath 1.3.0:

```
D=research/pp16_complex_shift_gain_20261004
python3 tools/pp16_pade_shifts.py --output $D/pade_shifts.json \
 && cargo fmt --all -- --check \
 && cargo clippy -p rodas5p-core --all-targets --locked -- -D warnings \
 && cargo test -p rodas5p-core --locked --test pp16_complex_shift_contracts \
 && PP16_PADE=$D/pade_shifts.json PP16_CASES=$D/cases.json cargo test --release -p rodas5p-core --locked \
      --test pp16_complex_shift_study -- --ignored --nocapture \
 && python3 tools/pp16_complex_shift_check.py --cases $D/cases.json --output $D/RESULTS.json \
      --g4-contract-and-lint-passed
```

The chain exited 0. Outputs: `pade_shifts.json` (Pade shifts and weights), `cases.json` (native certificates, all
inputs and outputs as IEEE-754 hex), `RESULTS.json` (oracle). Before the commit, `cargo test -p rodas5p-core --locked`
(whole core crate) also exited 0.

**Verdict: PASS** (G1, G2, G4 hold; the G3 prediction was observed).

| Gate | Outcome |
|---|---|
| G1 enclosure | **holds**: 480 single-column certificates (80 random shifts, 72 Pade poles, 72 partial-fraction term re-certifications, 256 jet columns) and 16 partial-fraction outputs. Every reported bound is at least the actual Euclidean error of the exact binary target. n <= 6 uses exact Fraction complex Gauss elimination (240 columns); n = 16, 24 uses mpmath at 50 digits (240 columns). The oracle also confirmed four things exactly: each `gain_upper` encloses `|gamma|/Re gamma`, each `residual_l2_upper` encloses the exact `||r||_2`, every operator passes the dissipativity row test, and every jet target has exact `|z| < 1` below the reported `z_modulus_upper < 1`. Every Certified status has its bound and actual error at or below the tolerance `1e-10` |
| G2 negative controls | **holds**: 36 of 36 failed closed: 29 errors and 7 Rejected, none Certified. Errors cover `Re gamma` = 0, -0.0 or < 0, two non-dissipative `J`, NaN/Inf in `J`, `h`, `gamma`, `b`, `u`, weight and `c0`, negative `h`, jet `|z| >= 1` (five variants), zero center, resource cap and an empty partial fraction. Rejected cover a zero candidate, a candidate perturbed by 1e-6, the conjugate shift's solution, swapped real/imaginary parts, an under-resolved degree-0 jet, a partial fraction with one wrong term and one with a conjugate pair's candidates swapped |
| G3 prediction | **observed**: 16 of the 80 random complex shifts have an actual error above the exact `||r||_2`, by factors from 1.024 to 2.163. In those cases gain 1 would have under-bounded. All 16 are at h = 0.05: 4 at n = 4, 7 at n = 6, 2 at n = 16 and 3 at n = 24. The largest is `n4_h0.05_random6`, `gamma = 0.0232 - 2.459i`, error 5.81e-16 vs `||r||_2` 2.68e-16, reported bound 1.38e-13. The full list is in `RESULTS.json` (`G3_cases_actual_error_above_residual`) |
| G4 contracts and lint | **holds**: fmt check, clippy `-D warnings` on all targets, and the 6 contract tests passed in the same `&&` chain. The oracle records the flag; it does not re-run them |

Bound over actual error:

| Family | rows | Certified | min | median | max |
|---|---|---|---|---|---|
| random complex shifts (LU candidates) | 80 | 80 | 8.14 | 655 | 2.86e5 |
| Pade [3/3] poles | 24 | 24 | 3.22 | 8.01 | 17.9 |
| Pade [6/6] poles | 48 | 48 | 3.47 | 7.91 | 17.5 |
| jet columns (degree 32, `gamma0 = 1`) | 256 | 176 | 1.17 | 11.8 | 101 |
| partial-fraction outputs | 16 | 16 | 7.21 | 24.5 | 67.1 |

Notes on the table:

- The tightest single bound is `n6_h0.05_jet_t12_c1`: bound 1.26e-13 vs exact error 1.08e-13 (ratio 1.17). On the
  50-digit rows, the tightest is 1.31 (`n16_h0.05_jet_t12_c0`).
- The 80 rejected jet columns are exactly the `|z| = 0.6` targets of the five operators with h = 0.5 or n = 24. Their degree-32 truncation
  errors reach 9.7e-9, and each one is enclosed.
- Partial-fraction bounds range from 1.9e-14 to 6.5e-12, against actual errors from 9.4e-16 to 1.6e-13. For
  [6/6], `|w_i|` reaches 315.7, and that drives the weighted term part.
- On the random shifts the gain ranges from 1.07 to 3064. The large over-estimates (up to 2.9e5) come from this
  worst-case operator norm, which the residual direction does not attain.

Fixture choices the preregistration did not fix, all made before any output existed:

- Seeding. `J` = skew part of scale 10 plus symmetric part of scale 1, with each diagonal at least 0.05 below the
  outward symmetric-part row bound. Seeded SplitMix64, one `J` per n, shared by both h.
- Tolerance `1e-10`.
- Pade inputs. For each Pade pole, `b` is real. `p_i` is rounded to binary64 component-wise, and its conjugate
  partner is that rounded value conjugated. `gamma_i` is the binary64 rounding of `1/p_i`, taken from the
  rounded `p_i`. `w_i` is the binary64 rounding of `-a_i/p_i`, with `a_i` the exact-root residue, and
  `c0 = (-1)^k`. The rounded partial fraction matches the [k/k] Pade value within 1e-14 at four test points
  (`pade_shifts.json`, `checks`).
- Random shifts. Ten per (n, h), with `Re gamma` log-uniform in [1e-3, 2] and `Im gamma` uniform in [-5, 5].
  Candidates come from an LU solve of the real 2n embedding.
- Jet family. This family was added to exercise `complex_shift_jet` beyond its negative controls: `gamma0 = 1`,
  degree 32, two complex RHS columns, and `z = rho e^{i theta}` with `rho` in {0.25, 0.6} and 8 angles.

Deviations and disclosures:

- **Signatures.** `complex_shift_jet` takes `(J, h, gamma0, rhs_re, rhs_im, gammas, config)`, with the degree
  inside a budget config like `shared_shift_jet`. `certify_partial_fraction` takes
  `(J, h, c0_re, c0_im, terms, b_re, b_im, tol)`, where each term carries its shift, weight and candidate. It
  re-certifies every term from the current exact inputs instead of accepting certificates. Every listed term is
  evaluated, and conjugate pairs are never merged.
- **Shared helper.** The dissipativity row helper of `shared_shift_jet.rs` became `pub(crate)` and is reused, so
  its refusal message keeps the prefix "shared shift jet:".
- **Dry runs.** Before the commit, development runs of the exporter and the oracle wrote to a scratch directory
  outside the repository. They exposed an oracle bug: Python `sum` over complex tuples concatenated them in the
  residual, which made the residual check fail. It was fixed before the commit. No fixture, tolerance or gate
  was changed after any dry run, and the recorded `cases.json` equals the last dry run's file except for its
  `pade_input` path field.
- **Claim ceiling.** H = I only, dense `J` up to n = 24, and LU or jet candidates. No speed claims and no
  timings. The default solver is unchanged.
