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
