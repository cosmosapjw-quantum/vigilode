# Preregistration: optional Darboux/closure chart contract (thread-transfer DAG node P2-CHART-CONTRACT)

## Question

The prior thread repaired RVJ5's two-step no-go on the semilinear model with the ratio chart `w = C / D`. The review
(section 6.1) states the contract any such chart needs. If `L_F C = (-kappa + a) C + r_C` and `L_F D = a D + r_D`
only approximately, then

```
w' = -kappa w + (r_C - w r_D) / D,
```

so small cofactor residuals do not make a chart accurate when `|D|` is small. A chart also changes the state, the
output reconstruction, the initial condition and the branch. Can a model-specific chart adapter state and check
those conditions, failing explicitly where they do not hold?

## Implementation (written after this commit)

`tools/thread_transfer_chart.py` (research module, Python with SymPy and mpmath; no solver change, no native
integration):

- `ratio_chart_identity()`: SymPy proof of the identity above for generic `C`, `D`, `a`, `kappa`, `r_C`, `r_D`;
- `SemilinearChart(kappa, d_min, cond_max)`: for `x' = x^2, y' = (-kappa + 2x) y + x^2`, `C = y - x^2 / kappa`,
  `D = x^2`, so `w = y / x^2 - 1 / kappa` and `w' = -kappa w` exactly. `to_chart` and `from_chart` raise on `|D| <
  d_min`, on an inverse-chart norm `|dy/dw| = x^2` whose reciprocal exceeds `cond_max`, and on a branch change of `x`;
- `moving_frame_connection(S, F)`: `-S^-1 L_F S`, the term a frame change adds;
- a perturbed model (`y' = ... + eps x^3`) with nonzero `r_C`, where the chart equation keeps the forcing
  `(r_C - w r_D) / D`.

## Test and command

`python3 tools/thread_transfer_chart.py --output research/thread_transfer_chart_contract_20261002/RESULTS.json`
(each check below is executed and recorded; the script exits nonzero if any fails).

## Gate

**PASS** if all hold:

1. **Identity.** SymPy reduces `w' - (-kappa w + (r_C - w r_D)/D)` to 0 for generic symbols, and the semilinear chart
   has `r_C = r_D = 0` symbolically.
2. **Fail closed.** `to_chart` refuses `x = 0` (D = 0), `|x|` below the conditioning limit, and a step across `x = 0`
   (branch); `from_chart` refuses the same.
3. **Fast mode kept.** From `w(0) = 1/10` (`y` off the slow graph), the exact chart flow `w(t) = w(0) e^(-kappa t)` and
   the reconstructed `y` match an independent mpmath solution of the original ODE (50 digits) to 1e-30 relative at
   t = 1/10 for kappa = 40 (a nonzero fast mode is not projected away).
4. **Finite-epsilon invariance not replaced.** For the perturbed model, the chart equation with the cofactor
   residual reproduces the mpmath solution to 1e-30 relative, while dropping the residual (the equilibrium-residual
   shortcut) misses it by more than 1e-6 relative at eps = 1e-3.
5. **Connection retained.** For the frame `S = x^2`, the connection `-S^-1 L_F S` equals `-2x` symbolically and is the
   term that cancels `a` in the semilinear chart.

Otherwise **FAIL**. Claim ceiling: a model-specific chart theorem and tests; no generic production replacement, no
RVJ activation and no RODAS5P change.

## Prior information

The prior thread's chart checks (vendored in `research/thread_transfer_negative_controls_20261002`, 89/89 numeric
checks including chart repair). No code of this node exists before this commit.

---

## Results (appended after the run at the execution commit recorded in L-0043)

Output: `RESULTS.json`. Ledger row L-0043.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Identity | holds: the general identity simplifies to 0; for the semilinear chart `a = 2x`, `r_C = 0`, `r_D = 0` |
| 2. Fail closed | holds: `to_chart`/`from_chart` refuse D = 0, `x^2 < d_min`, an inverse-chart norm above the limit and the other branch; a valid point round-trips |
| 3. Fast mode kept | holds: `w(0.1) = 0.1 e^(-4) = 1.83e-3`, not zero, and the reconstructed `y` equals the 50-digit solution of the original ODE to working precision (the reference was checked to respond to a 1e-20 change of `y(0)` with an 8.5e-22 change) |
| 4. Finite eps | holds: with `r_C = eps x^3` kept, exact to working precision; with it dropped, 9.9e-4 relative error in `y` at eps = 1e-3 |
| 5. Connection | holds: `-S^-1 L_F S = -2x` for `S = x^2`, cancelling `a = 2x` |

Reading: the chart is exact for this model only because both cofactor residuals vanish. Any residual enters the chart
equation divided by `D`, so a chart needs a lower bound on `|D|`, a conditioning limit and a fixed branch, and the
adapter refuses to map outside them. This is the contract of one model. It does not enable RVJ, change RODAS5P or
offer a generic chart.

Development disclosure: the script ran once before the recorded run (same outcome), and one sensitivity check of
the reference was run by hand.
