# Preregistration: certified chart coordinate-error provider and controller (remaining-only DAG node R-NEXT-05)

## Question

PR #70's `certify_reconstruction` transports coordinate-error bounds that are already certified into a physical
error bound for the chart `y = x^2 (w + 1/kappa)`. Nothing yet produces such coordinate bounds. For the
model-specific semilinear system of L-0043,

`x' = x^2`, `y' = (-kappa + 2x) y + x^2 + eps x^3`, chart `w = y / x^2 - 1/kappa`, `w' = -kappa w + eps x`,

can an opt-in stepper do the following?

- advance a cheap approximation in chart coordinates;
- produce certified coordinate-error bounds for every step, over the whole step and at dense points;
- convert them to physical bounds that a step-size controller uses;
- keep the nonzero initial fast mode and the cofactor forcing, and refuse to act outside the chart?

It does not replace RODAS5P and is not called by any default path.

## Stepper (written after this commit; `chart_transport.rs`, feature `audit2-research`)

- **Identity.** Each run carries `(kappa, eps, branch sign)` as bits. Every step and dense evaluation checks it, and
  a mismatch is refused.
- **Enclosure (the provider).** From boxes `X`, `W` containing the exact chart state, outward-rounded interval
  arithmetic gives the exact flow over `h`. For `x`: `phi(x) = x / (1 - x h)` (monotone, defined while
  `1 - x h >= 1/2` on the box). For `w`: `e^{-kappa h} W + eps * T * (1 - e^{-kappa h}) / kappa`, where `T` is the
  whole-step `x` tube `[x_lo, phi(x_hi)]`. `x` is monotone in time, so the tube covers the whole step. The
  exponential is enclosed with directed rounding.
- **Method (the approximation).** `x~ <- x~ + h x~^2 + h^2 x~^3` and `w~ <- R(-kappa h) w~ + eps h x~`, with the
  L-stable `R(z) = 1 / (1 - z + z^2 / 2)`. The approximation is never re-centred on the enclosure, so its global
  error is what the bounds measure. The coordinate bounds are `rx = sup_{X'} |x~ - X'|` and `rw = sup_{W'} |w~ - W'|`.
- **Physical bound.** `certify_reconstruction(x~, w~, kappa, rx, rw, stored y~, d_min)` over the tube. A tube that
  touches 0, changes branch or has `x^2 < d_min` is refused.
- **Controller.** Accept iff `B_phys <= atol + rtol |y~|`; otherwise halve `h`, and refuse below `h_min`. After an
  acceptance `h` grows by 1.5. A separate field `embedded_proxy`
  (`|R(-kappa h) - (1 + z/2)/(1 - z/2)| |w~| x~^2` plus the `x` Taylor term) is reported and never used.
- **Dense output.** At `theta h` from the step's start boxes, with the same formulas and identity check.

## Cases and commands

`kappa` in {40, 1000}; `x0` in {1 (to t = 0.5), -1 (to t = 5)}; `w0 = 1/10` (a nonzero fast mode); `eps` in
{0, 1e-3}; `atol = rtol` in {1e-6, 1e-9}; `d_min = 1e-6`; dense output at `theta` in {1/4, 1/2, 3/4} of every
accepted step, and outputs at `t` in {1/(4 kappa), 1/kappa, 4/kappa, t_end}. Refusal cases: `x0 = 1` to `t = 1.2`
(blow-up at 1); `x0 = -1` to `t = 50` with `d_min = 1e-3` (`x^2` falls below it); a dense call with another `kappa`.
Negative control: `eps = 1e-3` in the model but the stepper told `eps = 0`.

`RNEXT05_OUTPUT=research/rnext05_chart_provider_20261003/runs.json cargo test -p rodas5p-integrators --features audit2-research --locked --test rnext05_chart_provider -- --nocapture --test-threads=1`
`python3 tools/rnext05_chart_check.py --runs research/rnext05_chart_provider_20261003/runs.json --output research/rnext05_chart_provider_20261003/RESULTS.json`
(50-digit mpmath reference: closed form for `x`, `w` by exact quadrature of the forcing.)

## Gate

**PASS** if all hold:

1. **Tube and enclosure.** Every accepted step's whole-step `x` tube is regular. Every reported `(x, w)` box contains
   the 50-digit reference.
2. **Physical bound.** `|y~ - y_ref| <= B_phys` at every endpoint, output time and dense point, for `eps = 0` and
   for `eps = 1e-3` with the forcing.
3. **Fast mode retained.** At `t = 1/kappa`, `|y_ref - x^2/kappa| > 10 B_phys` (the bound resolves the fast
   amplitude), and the check in item 2 holds there.
4. **Controller.** Every accepted step has `B_phys <= atol + rtol |y~|`; acceptance never uses the proxy.
5. **Fail closed.** The blow-up, `d_min` and identity-mismatch cases are refused, with no accepted step or dense
   value outside the domain.
6. **Cofactor forcing is necessary.** In the negative control at least one output violates the enclosure (the
   bound is not valid without the forcing term).

Otherwise **FAIL**. Claim ceiling: one model with fixed `kappa` and a known chart; certified coordinate errors from
an exact-flow enclosure, not a general ODE error estimator. No stage-target proxy is promoted to an ODE error. No
general-ODE, speed or timing claim. The protected solver remains the default.

## Prior information

L-0043 (chart identity, fail-closed rules, fast mode, finite-eps residual) and PR #70's `certify_reconstruction`
with its regressions. No code of this node exists before this commit.

## Amendment before the recorded run (development observation, disclosed)

A development run of the native test showed two design errors (the Python check had not run):

1. The controller compared the *global* certified bound (distance of the never re-centred approximation to the
   enclosure of the exact solution) with the tolerance. Once the accumulated error exceeded it, no step could be
   accepted, so runs with `eps = 0` and `x0 = 1` were refused near t = 0.28 "below the minimum step".
2. The method's forcing `eps h x~` is not consistent for `kappa h >> 1`, so at tolerance 1e-9 runs needed up to
   8e5 steps, and one run was killed for memory.

Changes (nothing else changes; the gate items keep their numbers):

- **Method.** `x~ <- x~ / (1 - x~ h)` (the flow formula in binary64).
  `w~ <- R w~ + eps [x~ (1 - R)/kappa + x~^2 (h/kappa - (1 - R)/kappa^2)]` with `R = R(-kappa h)`, the L-stable
  (1,2) Pade approximant `(1 + z/3) / (1 - 2z/3 + z^2/6)`. This is the exponential-integrator form for `x` linear in
  time over the step, with `e^z` replaced by `R`.
- **Two bounds.** The *local* physical bound `B_loc` comes from the exact-flow enclosure started at the point
  `(x~_n, w~_n)` (the method's local error, transported by `certify_reconstruction`). The *global* bound `B_phys`
  comes from the enclosure of the exact solution propagated from the exact initial data, as before.
- **Controller.** Accept iff `B_loc <= atol + rtol |y~|` (gate item 4 now refers to `B_loc`). Gate items 1-3 check
  the global boxes and `B_phys` against the 50-digit reference, as before. The proxy stays unused.

---

## Results (appended after the run at `7767e75`)

Outputs: `runs.json` (native), `RESULTS.json` (50-digit check). Ledger row L-0048.

**Gate: PASS** (items 1-6 hold).

| Gate item | Outcome |
|---|---|
| 1. Tube and enclosure | **holds**: every accepted step's whole-step `x` tube is regular, and all 19,652 step, dense and output points of the 16 runs have `x` and `w` boxes that contain the 50-digit chart solution |
| 2. Physical bound | **holds**: `|y~ - y_ref| <= B_phys` at all 19,652 points. The worst ratio is 1.0 at `eps = 0` (the enclosure is nearly a point, so the global bound is essentially the actual error plus rounding) and 0.68-0.9999 at `eps = 1e-3` (the forcing integral widens the box) |
| 3. Fast mode retained | **holds**: at `t = 1/kappa` the fast amplitude `|y_ref - x^2/kappa|` is at least 2.2e4 times the bound (2.4e4 for `kappa = 40`); at `t = 1/(4 kappa)` about 1.4e6-1.7e6 times |
| 4. Controller | **holds**: every accepted step has `B_loc <= atol + rtol |y~|`; the proxy is reported only |
| 5. Fail closed | **holds**: `x0 = 1` to `t = 1.2` is refused at `t = 1 - 2.3e-12` ("within a factor 2 of the blow-up"), with every step satisfying `x t <= 1/2` on its start box and ending before 1; `x0 = -1` with `d_min = 1e-3` is refused at `t = 30.62` (`x^2` reaches 1e-3), with every tube above the margin; dense queries with another `kappa` or `eps` are refused |
| 6. Cofactor forcing necessary | **holds**: with the model's `eps = 1e-3` but the stepper told 0, none of the 828 points is enclosed (worst error/bound 4.3e7) |

Steps (accepted / rejected): 50-1,042 accepted per run, 8-596 rejected; `tol = 1e-9` with `eps = 1e-3` needs the
most (the `x`-linear forcing model limits the step).

Development disclosure: before the recorded run the native test ran three times, and the Python check not at all.
The first run showed the two design errors amended above (global-bound controller, inconsistent forcing). The
second showed a wrong check in the test: the blow-up condition was applied to the end of the tube instead of the
start box. The third was the recorded run. Claim ceiling: one model with a known chart and fixed `kappa`;
certified coordinate errors come from an exact-flow enclosure that this model admits. This is not a general ODE
error estimator, no stage-target proxy is promoted, and nothing replaces the protected solver.
