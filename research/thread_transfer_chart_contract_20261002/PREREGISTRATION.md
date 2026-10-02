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
