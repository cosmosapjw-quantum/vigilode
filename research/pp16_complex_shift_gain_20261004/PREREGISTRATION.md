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
