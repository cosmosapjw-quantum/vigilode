# E-07 — RODAS5P coefficient / order-condition audit (mpmath 50 digits)

**Result: PASS.** Main weights satisfy all 17 Rosenbrock order conditions through order 5 (max |resid| = 1.2e-15 on the exact fixture, 2.0e-15 on the crate's f64 coefficients). Embedded weights are exactly order 4 (order<=4 max 9.4e-16; all nine order-5 trees violated, 1.2e-4..1.5e-2).

## What was checked
* ROW form reconstructed as `coefficients.rs:134-141`: Gamma=(I/gamma-C)^-1, alpha=A Gamma, beta=alpha+Gamma, b=Gamma^T b_code, btilde=Gamma[7,:], gamma_rows=rowsum(Gamma).
* Stage equation confirmed in `sequential.rs:335-363` (k-form with gamma_ij=Gamma_ij, gamma_ii=gamma, f_t weight gamma_rows_i).
* Order conditions: first-principles B-series recursion over all rooted trees |t|<=5 (17 trees), a_i(t)=prod_children sum_j alpha_ij a_j(c) + [single child] sum_j Gamma_ij a_j(c); condition sum_i b_i a_i(t) = 1/density(t). Cross-checked vs Hairer-Wanner IV.7 closed forms (p<=4), agreement 5.5e-15.
* Structure: alpha strictly lower, Gamma lower with diag gamma, beta_ii=gamma, c_i = sum_j alpha_ij (4.0e-15), stiff accuracy b = beta[8,:] (1e-50), embedded bhat = alpha[8,:] = beta[7,:].
* f64 rounding vs mpmath: Gamma/alpha/beta/l abs diff <= 5.6e-16; b up to ~807 ulp, btilde ~165 ulp (cond_1 = 4.1e4). Order residuals on the f64 coefficients <= 2.0e-15.
* Observed order (E-04 `p1ns`, lambda in [-10,-1], direct LU): main slopes 4.48, 5.44, 5.73, 5.87; embedded 4.40, 4.29, 4.19, 4.12, 4.07, 4.04, 4.02.

## Per-tree residuals (exact fixture)
| order | tree | main | embedded |
|---|---|---|---|
| 1 | `t` | +5.52e-15 | +4.04e-15 |
| 2 | `[t]` | +1.59e-15 | +2.67e-15 |
| 3 | `[t,t]` | -6.68e-16 | -1.90e-16 |
| 3 | `[[t]]` | -9.43e-17 | +8.24e-16 |
| 4 | `[t,t,t]` | -1.14e-15 | -8.73e-16 |
| 4 | `[t,[t]]` | -9.55e-17 | -8.17e-17 |
| 4 | `[[t,t]]` | -1.28e-15 | -9.36e-16 |
| 4 | `[[[t]]]` | -2.41e-16 | +1.32e-16 |
| 5 | `[t,t,t,t]` | +6.17e-17 | +7.26e-03 |
| 5 | `[t,t,[t]]` | -1.21e-16 | +6.39e-04 |
| 5 | `[t,[t,t]]` | +2.63e-16 | -7.51e-04 |
| 5 | `[t,[[t]]]` | +4.79e-17 | -3.76e-04 |
| 5 | `[[t],[t]]` | -1.96e-16 | -2.34e-03 |
| 5 | `[[t,t,t]]` | -1.23e-15 | -7.43e-04 |
| 5 | `[[t,[t]]]` | +3.16e-16 | -1.46e-02 |
| 5 | `[[[t,t]]]` | -5.15e-16 | -2.48e-04 |
| 5 | `[[[[t]]]]` | -1.27e-16 | -1.24e-04 |

## Files
`e07_mpmath_check.py` (script), `mpmath_exact.{json,out}`, `mpmath_f64.{json,out}`, `coeff_f64.json` (crate dump, 17 sig digits + IEEE bits), `results.json`.
