# Loop L1, node V05: theory note for the RODAS5P stage chain

Date 2026-11-16. Worktree commit `2dc9a2c5` (branch `research/production-loop-20261116`); nothing in this
directory is committed. No protected file was touched; the tableau snapshot is read only.

## 0. Provenance, commands, tags

| item | value |
|---|---|
| tableau | `fixtures/rodas5p_coefficients_snapshot.json`, SHA-256 `211255f6125b648c6955833c3da537d9c031ec78168b2918563c29fa88c8bce6` (equals the digest pinned in `crates/rodas5p-core/src/coefficients.rs`, check S0.a) |
| primary script | `verify_tableau.py`: `fractions.Fraction` on the decimal strings, standard library only |
| independent script | `verify_independent.py`: mpmath 1.3.0 at 60 digits, does not import the primary, different algorithms where available (listed in its docstring) |
| results | `RESULTS.json` (written by the two scripts), `RESULTS_TABLES.md` (rendered from `RESULTS.json` by the second script), raw logs `run_verify_tableau.log`, `run_verify_independent.log` |

Commands, run from the worktree root (Python 3.12.3):

```
python3 research/production_loop_20261116/theory/verify_tableau.py      > research/production_loop_20261116/theory/run_verify_tableau.log 2>&1      # exit 0, about 60 s
python3 research/production_loop_20261116/theory/verify_independent.py  > research/production_loop_20261116/theory/run_verify_independent.log 2>&1  # exit 0, about 12 s
```

Outcome of the final run: primary 58 checks, 0 failed; independent 18 checks, 0 failed; 1040 value keys compared,
0 missing, 0 mismatches. Largest scaled difference: 1.7e-57 on the 784 exact-kind keys (tolerance 1e-40),
6.6e-15 on the 236 numerical-kind keys (tolerance 1e-9), 5.9e-44 on the 20 keys compared relatively
(tolerance 1e-6).

Tags used below:

- **[exact]**: holds in exact rational arithmetic on the represented decimals (or is a rational identity).
- **[zero by design within representation: r]**: a design identity; r is the exact residual of the decimal tableau.
  Residuals are between 1e-21 and 6e-15. They are larger than one unit in the last place because the last
  construction step of the method was a numerical least-squares solve (section S4, literature).
- **[numerical]**: floating-point or iterative result, or an experiment at finite step sizes.
- **[conjecture]**: an argument or a model that no script proves. Cost models are marked "model".

Notation. The reconstruction is the one of `coefficients.rs` lines 168-212: `Gamma = (I/gamma - C)^-1`,
`alpha = A Gamma`, `beta = alpha + Gamma`, `N = beta - gamma I`, `b = Gamma^T b_code`. "k-form" is the
untransformed ROW form `(I - h gamma J) k_i = h f(y_n + sum_j alpha_ij k_j) + h J sum_{j<i} gamma_ij k_j`;
"U-form" is the code form `W U_i = h gamma f(y_n + sum_j A_ij U_j) + gamma sum_j C_ij U_j`, `U = Gamma k`.
`g_i = y_n + sum_j alpha_ij k_j` is the argument of the i-th right-hand side.
gamma = 0.21193756319429014.

Run history. The first executed version of the primary script failed 4 of 55 checks; its output was shown in
the session but not saved to a file. (1) S4.e reported order 2 for the main solution: a bug of mine, the
U-form step summed the stages with unit weights instead of `b_code`; corrected, and the experiment was then
repeated in the k-form by the second script. (2) S2.n and S2.q used a threshold of 1e-25 that the represented
tableau cannot meet (see S2); the threshold is now 1e-15 and the measured value is reported. (3) S3.f assumed
`|R_q(inf)| > 1` for every truncated sweep count; that is false for q = 4 (k-form) and q = 7 (U-form), and the
statement was rewritten to what the numbers show.

## Summary of verdicts

| id | verdict |
|---|---|
| S1 | established |
| S2 | established; residuals quantified |
| S3 | formula **corrected** (it describes the k-form sweep, not the U-form sweep of R6); the diagonal-scaling argument is **refuted** (infimum 0) and replaced by a scaling-invariant one |
| S4 | established; literature confirms the design intent for the main weights, formula identification UNVERIFIED |
| S5 | established; g6 has order 1 only |
| S6 | established; one structural limit added (the factorisation needs the next step size) |
| S7 | model stated, closed forms checked; not a measurement |
| S8 | established for the scalar operator; for the eight-stage block the per-sweep contraction has a large polynomial transient |
| S9 | L = 4 no-go (exact); L = 5 conditional go; L = 6 go for a search |

## S1. Depth of the right-hand-side chain

- [exact] `alpha` is strictly lower triangular and all 28 strictly-lower entries are nonzero (checks S1.a, S1.b).
  The smallest is `|alpha_81| = 3.487e-3`. The code matrix `A` is also a full strict lower triangle.
- [exact] The subdiagonal is nonzero: `alpha_{i+1,i}` = 0.6358, 0.09716, -2.195, -0.01001, 0.006400, 0.2119, 0.2119
  (`A_{i+1,i}` = 3, 0.4584, -10.36, -0.04721, 0.03020, 1, 1). Therefore the graph "evaluation j feeds the
  argument of evaluation i" contains the path 1 -> 2 -> ... -> 8 and its longest path is 8 (check S1.c).
- [zero by design within representation: 4.0e-15] `c_i = sum_j alpha_ij`.

Consequence, paper argument (not machine-checked): `g_i` depends on `f(g_{i-1})` through
`A_{i,i-1} W^-1 (h gamma f(g_{i-1}))` with `A_{i,i-1} != 0` and `W` nonsingular. If `f` is available only
through point evaluations, an adversary can fix `f(g_{i-1})` after the query is made, so `g_i` is not known
before evaluation i-1 has returned. Any algorithm that returns the RODAS5P stage values for every `f` needs
eight sequential rounds of queries, whatever the number of processors.

What this excludes: re-scheduling the eight evaluations (and the eight solves between them) of one step of the
unchanged method into fewer than eight sequential rounds; in particular "two stages at a time" for a black-box
`f`. The critical path of a step is `8 (T_f + T_solve)`.

What it does not exclude:

1. parallelism inside one evaluation, one solve, the Jacobian assembly or the factorisation;
2. work that belongs to the next step and overlaps the tail of the chain (S6);
3. spatial decomposition, where every subdomain runs its own chain of depth 8 at the same time (S8);
4. right-hand sides with known structure (affine parts need no new evaluation; source-owned graphs);
5. iterations that use more than eight evaluations, such as simultaneous sweeps: eight rounds of eight
   evaluations are exact, fewer rounds are a different method (S3);
6. speculative evaluation at predicted arguments with verification: it shortens the path only when the
   prediction is exact, otherwise it is item 5;
7. a different tableau (level-structured ROW, S9; peer methods).

## S2. Nilpotency and the stability function

- [exact] `alpha_21 = 3 gamma` (the string `A_21 = "3.0"` times `Gamma_11 = gamma`).
- [zero by design within representation: 6.175e-17] `beta_21 = alpha_21 + gamma_21 = 0`, i.e. `gamma_21 = -3 gamma`.
  With the binary64 values of the strings the residual is 8.641e-17.
- [exact] `N^7` has at most the entry (8,1), equal to the product of the subdiagonal of `N` (check S2.c). With
  `beta_21 := 0`, `N^7 = 0` and `N^6 != 0` (three nonzero entries, largest 1.633e-4): nilpotency index exactly 7.
- [zero by design within representation: 2.882e-21] `N^7 = 0` for the represented tableau.
- [exact] `R(z) = 1 + z b^T (I - z beta)^-1 1 = 1 + sum_{m=0}^{7} mu_m z^{m+1} / (1 - gamma z)^{m+1}`,
  `mu_m = b^T N^m 1` (verified against direct solves at four rational points, check S2.g).
  [zero by design within representation: -6.108e-22] `mu_7 = 0`, hence `R(z) = P(z) / (1 - gamma z)^7`.
- [exact] `R(infinity) = 0`, because `b` equals row 8 of `beta` exactly in the represented tableau.
  [zero by design within representation: -2.882e-21] the coefficient of `z^7` in `P` vanishes, so `deg P <= 6`.
- [zero by design within representation: 5.5e-15] `R(z) - exp(z) = O(z^7)`: the Taylor defects of orders 1 to 6
  are 5.5e-15, 1.6e-15, -9.4e-17, -2.4e-16, -1.3e-16, -5.3e-17.
  [exact] The defect at `z^7` is -1.0153492e-5, so the linear order is exactly 6.
- [zero by design within representation: 6.6e-15] Six conditions fix the six free coefficients of `P`:
  `P = trunc_6[ exp(z) (1 - gamma z)^7 ]`, a function of `gamma` alone.

`P(z) = 1 - 0.483562942360031 z - 0.040294797812623 z^2 + 0.034963420043764 z^3 + 0.003465788280381 z^4
- 0.001229573186541 z^5 - 0.000240486403231 z^6` (design values; the tableau values differ by at most 6.6e-15).

A-stability. With `s = y^2`, `E(s) = (1 + gamma^2 s)^7 - |P(iy)|^2`.

- [exact] For the design function, `E(s) = s^4 (2.5100e-5 + 6.6091e-7 s - 3.4358e-10 s^2 + 3.6890e-10 s^3)` and a
  Sturm sequence shows that the cubic has no positive root (check S2.m; the second script finds the same with a
  root finder). The poles are at `1/gamma > 0`. Hence `|R(z)| <= 1` on the closed left half-plane, and with
  `R(infinity) = 0` the function is L-stable.
- [numerical] For the represented tableau the coefficients of `s^1..s^3` are residuals (-7.9e-15, -2.3e-15,
  -2.7e-16), so `|R(iy)|^2 - 1` is positive for small `y`. Its maximum on a 65-point rational grid
  (`y` from 1e-6 to 7e6, exact evaluation) is 2.50e-18 at `y = 0.02`, forty times below double-precision
  resolution. The cause is the 5.5e-15 defect of `sum b_i = 1`.
- [numerical] `gamma` lies inside the A-stability window computed for seven poles and linear order 6,
  `[0.20408345, 0.37886489]` (S9). These bounds coincide with the values I remember from Hairer and Wanner,
  Solving ODEs II, Table IV.6.4; that citation is UNVERIFIED, the book was not consulted.

Embedded method (weights = row 7 of `beta`): [exact] `R_hat(infinity) = 0`; [zero by design within
representation: 4.0e-15] linear order 4, with [exact] defect -1.2386e-4 at `z^5`; [numerical]
`|R_hat(iy)|^2 - 1 <= 2.4e-21` on the same grid.

## S3. Simultaneous sweeps

Statement as posed: for the sweep of `THEORY_R6.md` Theorem 1 on `y' = lambda y` the error propagation matrix
is `z/(1 - gamma z) N`, stiff limit `-N/gamma`.

- [exact] **Corrected.** Theorem 1 iterates the U variables. Its propagation matrix is
  `M_U(z) = gamma (z A + C) / (1 - gamma z)`. The matrix `M_k(z) = z N / (1 - gamma z)` belongs to the sweep in
  the k variables. They are related by `I - M_U = gamma (I - M_k) Gamma^-1` (checks S3.b, S3.c): the U-form sweep
  is the k-form sweep with a different splitting, and the two matrices are not similar (their nilpotency indices
  differ, next item).
- [exact] Stiff limits: `M_k -> -N/gamma`, `M_U -> -A`. At `z = 0`: `M_k = 0`, `M_U = gamma C`.
- [zero by design within representation: 6e-17] `M_U(z)_{21} = -3` for every `z`. The U-form sweep matrix has
  nilpotency index 8, the k-form one 7. On a linear problem the k-form sweep is exact after 7 sweeps, the
  U-form sweep after 8.

Norms of `(N/gamma)^m` (k-form, stiff limit; 1- and infinity-norm exact, 2-norm numerical):

| m | 1-norm | 2-norm | infinity-norm |
|---|---|---|---|
| 1 | 44.25 | 27.59 | 37.06 |
| 2 | 26.54 | 16.78 | 24.31 |
| 3 | 15.49 | 15.19 | 23.34 |
| 4 | 12.73 | 12.55 | 20.46 |
| 5 | 3.488 | 4.103 | 6.389 |
| 6 | 1.802 | 1.875 | 2.317 |
| 7 | 1.5e-16 (residual; 0 by design) | same | same |

The infinity-norm column reproduces the plan values 37, 24, 23, 20, 6.4, 2.3, 0 (check S3.a).

Norms of the U-form matrices, which are the ones relevant to R6 and R7:

| m | `A^m` 1-norm | 2-norm | infinity-norm | `(gamma C)^m` infinity-norm |
|---|---|---|---|---|
| 1 | 45.64 | 27.63 | 23.91 | 68.77 |
| 2 | 118.9 | 63.13 | 62.54 | 98.08 |
| 3 | 130.3 | 79.59 | 83.81 | 137.1 |
| 4 | 57.97 | 46.58 | 51.31 | 257.3 |
| 5 | 9.344 | 7.233 | 7.573 | 126.1 |
| 6 | 2.678 | 2.658 | 2.665 | 117.6 |
| 7 | 0.02031 | 0.02031 | 0.02031 | 58.55 |
| 8 | 0 | 0 | 0 | 0 |

On a grid of 30 points `z` in `[-5e6, -1e-3]` the supremum of `||M_U(z)^m||_inf` is 68.8, 98.0, 137, 257, 135,
117, 58.5, 0 for m = 1..8, attained at the non-stiff end except for m = 5 ([exact] at the grid points).

Diagonal scalings.

- [exact] **Refuted.** The minimum over diagonal scalings of `||D (N/gamma) D^-1||` does not show that the growth is
  intrinsic. `N` is strictly lower triangular, so `D = diag(eps^i)` gives `||D (N/gamma) D^-1|| -> 0` as
  `eps -> 0`: the infimum is 0 in every norm and for every power. Measured: infinity-norm 37.1, 5.47, 1.56,
  0.535, 0.0499 for eps = 1, 1/2, 1/4, 1/10, 1/100, with cond(D) = 1, 128, 1.6e4, 1e7, 1e14. A scaled norm
  below 1 costs a condition number of about 1e7, which is the same amplification moved into the change of
  variables.
- [exact] A scaling-invariant statement replaces it. Starting from zero, q sweeps define the method with
  stability function `R_q`; `b^T D^-1 (D M D^-1)^m D 1` does not depend on `D`. Values at infinity:

| q | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| k-form `R_q(inf)` | 1 | -3.718 | 2.695 | 2.732 | -0.4628 | -2.582 | -2.317 | 1.5e-16 | 0 |
| U-form `R_q(inf)` | 1 | 14.72 | -60.42 | 83.75 | -47.49 | 5.800 | 2.665 | -0.02031 | 0 |

- [numerical] No truncated sweep count is A-stable. The supremum of `|R_q(iy)|` on 81 points `y` in `[1e-3, 1e5]`
  is 3.72, 2.69, 2.73, 1.029, 2.58, 2.32 for q = 1..6 in the k-form (q = 4 is marginal) and 14.7, 60.4, 87.3,
  107, 102, 42.9, 15.5 for q = 1..7 in the U-form (check S3.g).

Conclusion. Truncating the sweeps before 7 (k-form) or 8 (U-form) gives a method that amplifies stiff components
by factors between 2.3 and 107 per step. This is the structural reason for the R6 and R7 results; the numbers
that apply to those experiments are the U-form ones, not the `N/gamma` ones quoted in the plan.

## S4. W-conditions and the effect of a Jacobian error

With `alpha_i = sum_j alpha_ij` and `gamma_i = sum_j gamma_ij` (diagonal included), the W-method conditions up
to order 3 are: `sum b_i = 1`; `sum b_i alpha_i = 1/2` and `sum b_i gamma_i = 0`; `sum b_i alpha_i^2 = 1/3`,
`w_aa = sum b_i alpha_ij alpha_j = 1/6`, `w_ag = sum b_i alpha_ij gamma_j = 0`, `w_ga = sum b_i gamma_ij alpha_j = 0`,
`w_gg = sum b_i gamma_ij gamma_j = 0`. [exact] The ROW chain condition is the sum of the four coloured trees
(check S4.a).

| quantity | main weights `b` | embedded weights (row 7 of `beta`) |
|---|---|---|
| `sum b_i gamma_i` | -3.32e-16 | -0.0256401 |
| `sum b_i alpha_i - 1/2` | 1.93e-15 | +0.0256401 |
| `sum b_i alpha_i^2 - 1/3` | -6.7e-16 | -1.9e-16 |
| `w_aa - 1/6` | -0.0436114 | +0.121085 |
| `w_ag` | +0.0428546 | -0.116853 |
| `w_ga` | +0.0295222 | -0.159012 |
| `w_gg` | -0.0287654 | +0.154780 |

- [zero by design within representation: -3.32e-16] The main weights satisfy the order-2 W condition
  (binary64 interpretation: 1.13e-15).
- [exact] They fail all three order-3 W trees by 0.03 to 0.04: the W-order of RODAS5P is exactly 2.
- [exact] The embedded weights fail the order-2 W condition: `sum b_hat_i gamma_i = -0.0256401`. W-order 1.

Consequence for `T = J + E` (expansion of the k-form in `h`, classical regime `h|J|, h|E| << 1`):

```
main      delta y     = h^2 (-3.3e-16) E f + h^3 ( 0.0140892 J E f + 0.000756817 E J f - 0.0287654 E^2 f ) + O(h^4)
embedded  delta y_hat = h^2 (-0.0256401) E f + h^3 ( 0.0379266 J E f - 0.00423229 E J f + 0.154780 E^2 f ) + O(h^4)
estimate  delta (y - y_hat) = h^2 (+0.0256401) E f + h^3 ( -0.0238374 J E f + 0.00498911 E J f - 0.183545 E^2 f ) + O(h^4)
```

The coefficients are `sum b_i gamma_i`, `w_ag + w_gg`, `w_ga + w_gg`, `w_gg` [exact].

- [numerical] One step of a two-dimensional polynomial ODE with a fixed `E`, in exact rationals (U-form) and in
  mpmath (k-form): observed orders 2.994 (main), 1.998 (embedded), 1.994 (estimate) at `h = 2^-8`; the
  difference to the prediction above divided by `h^4` converges (1.85e-3, 6.18e-3, 5.82e-3). Check S4.e.
- [numerical] A Jacobian that is one step old (`E = O(h)`) perturbs the main solution by `O(h^4)` (observed 3.983)
  and the estimate by `O(h^3)` (observed 2.997). A lagged-Jacobian RODAS5P therefore has classical order 3, and
  its error estimate is dominated by the lag term once `h` is small. This bears on V09. Check S4.f.
- [conjecture] For stiff components `h E` is not small and the expansion says nothing.

Literature (retrieved 2026-11-16 with an automated page reader that returns short quotes; tables and formulas
of the article are images and were not readable; no PDF was read; Greek letters in the quotes are
transliterated, spelling as returned).

- G. Steinebach, "Construction of Rosenbrock-Wanner method Rodas5P and numerical benchmarks within the Julia
  Differential Equations package", BIT Numer. Math. 63, 27 (2023), doi 10.1007/s10543-023-00967-x.
  Scheme (1.2)-(1.3); `beta_ij = alpha_ij + gamma_ij` in (2.1). Table 2: "Order conditions up to order p=5 for
  ODE problems"; Table 3: "Additional order conditions up to order p=5 for index-1 DAE problems"; Table 4:
  "Additional order conditions". Quotes: "Conditions No. 45-47 are introduced by Jax [5]."; "By these conditions
  at least order p=2 is achieved for index-1 problems using inexact Jacobian matrices."; "The aim is to
  construct a method which fullfills all order conditions stated in Tables 2, 3 and 4."; "In the first step we
  set alpha21 = 3 gamma and beta21 = 0, see [24]", by which conditions "No. 18, 18^, 20, 20^, 30, 47" hold;
  after step 8 "the remaining conditions read No. 9, 11, 15, 24, 25, 26, 29, 44, 45, 46" and "For these
  remaining 10 conditions 7 degrees of freedom are left", solved as a nonlinear least-squares problem.
  In the inexact-Jacobian test "Rodas5P behaves like Rodas4P2".
- Rodas4P2: same article, Section 1: "Conditions up to order two are fullfilled by the Rodas4P2 method", and
  Section 2: "Method Rodas4P2 has been derived for that purpose". Primary source: G. Steinebach, "Improvement
  of Rosenbrock-Wanner Method RODASP", Progress in Differential-Algebraic Equations II, Springer 2020,
  doi 10.1007/978-3-030-53905-4_6; only its abstract was reachable. UNVERIFIED beyond the quotes above.

Reading: RODAS5P was designed with the inexact-Jacobian conditions of Jax for order 2 (No. 45-47 of Table 4),
and `alpha_21 = 3 gamma`, `beta_21 = 0` is step 1 of the construction, which settles the origin of S2. The
least-squares closure explains residuals of 1e-16 to 6e-15 instead of exact zeros. UNVERIFIED: which of
No. 45-47 is `sum b_i gamma_i = 0`, since the formulas were not readable; the computation above establishes the
condition itself. The reader returned hatted (embedded) versions only for No. 6, 18, 19, 20, which agrees with
the embedded weights not having W-order 2; this is also UNVERIFIED against the tables.

## S5. Orders of the stage arguments

- [exact] `alpha_8 = beta_7`, `b = beta_8`, `alpha_7 = beta_6` hold exactly in the represented tableau (rows 6, 7,
  8 of `A` and `b_code` share their decimal strings and differ by unit entries). Hence `g_7 = g_6 + U_6`,
  `g_8 = g_7 + U_7`, `y_{n+1} = g_8 + U_8`, and `g_8` is the embedded solution.
- The 17 ROW order conditions up to order 5 (1, 1, 2, 4, 9 trees; a singly-branched vertex carries `beta`, a
  branching vertex `alpha`) were evaluated for each weight row, by a tree generator in the first script and
  from the 17 formulas typed out in the second:

| weights | approximates | classical order | largest residual of the first failing order |
|---|---|---|---|
| `b` | `y(t_n + h)` | 5 [zero by design within representation: 5.5e-15] | - |
| `alpha_8` (`g_8`) | `y(t_n + h)` | 4 [zero by design within representation: 4.0e-15] | 1.46e-2 (order 5) |
| `alpha_7` (`g_7`) | `y(t_n + h)` | 3 [zero by design within representation: 1.7e-15] | 8.95e-2 (order 4) |
| `alpha_6` (`g_6`) | `y(t_n + h)` | 1 [exact failure of order 2] | 4.69e-2 (order 2) |
| `alpha_2 .. alpha_5` | `y(t_n + c_i h)` | 1 [exact failure of order 2] | 2.9e-3 to 6.7e-2 |

- [exact, classical expansion] Evaluating the Jacobian of step n+1 at `g_7` or `g_8` of step n gives
  `E = O(h^4)` or `O(h^5)`. With S4 the main solution is perturbed by `O(h^7)` or `O(h^8)` (local error is
  `O(h^6)`) and the error estimate by `O(h^6)` or `O(h^7)` (the estimate itself is `O(h^5)`).
- [numerical] Two-step experiment, exact rationals and mpmath: observed orders 6.987 and 6.000 for `g_7`,
  8.017 and 7.057 for `g_8` at `h = 2^-6` (check S5.f).
- [exact] `g_6` cannot be used: order 1 gives `E = O(h^2)` and a perturbation `O(h^5)`, above the local error.
  The overlap window of S6 cannot be extended to three stages by this route.

## S6. Speculation band

Model: `T_step = T_JLU + T_chain`, eight equal stages, `r = T_JLU / T_chain`. If the Jacobian and factorisation of
the next step run on another core during the last k stages, the saving is `min(T_JLU, k T_chain / 8)` and

`S_k(r) = (1 + r) / (1 + r - min(r, k/8))`.

- [exact] `S_2 <= 5/4`, attained at `r = 1/4`; `S_1 <= 9/8`, attained at `r = 1/8` (check S6.a).
- [exact] `S_2(r) >= 23/20` if and only if `3/20 <= r <= 11/12` (0.15 to 0.9167), from `1 + r >= 1.15` on the
  rising branch and `(1 + r)/(r + 3/4) >= 1.15` on the falling one (check S6.b). This is the band frozen in the
  pre-registration for V07.
- [exact, from the code] The factorised matrix is `W = I/(h gamma) - J` (`rodas5p_fast.rs`, `inv = 1.0 / (h * self.gamma)`).
  The next step size is known only after stage 8 and the controller. Without a predicted step size only the
  Jacobian assembly can overlap, and `T_JLU` in the formula becomes `T_J`. Overlapping the factorisation needs a
  step-size prediction with a refactorisation on a miss; that is a design decision for V07, not a property of
  the tableau.
- [conjecture] Equal stage costs, a free core and no memory-bandwidth contention are assumptions.

## S7. Cost model per step (model, to be confronted with V04)

Units: floating-point operations. `n` unknowns, `R = rho n` one right-hand side, vector work `112 n` per step
(two operations per nonzero of `A` and `C`).

| regime | Jacobian | factorisation | chain |
|---|---|---|---|
| dense | analytic `j R` or finite differences `n R` | `2 n^3 / 3` | `8 (R + 2 n^2) + 112 n` |
| banded, half-bandwidth `b` | `j R` or `(2b + 1) R` | `n b^2` | `8 (R + 4 n b) + 112 n` |
| matrix-free, `kbar` iterations | none | none (preconditioner set-up `T_P`) | `8 (R + kbar (T_jvp + T_prec) + 2 kbar^2 n)` |

Shares: `f_J = T_J / T`, `f_LU = T_LU / T`, `f_chain = T_chain / T`, `T = T_J + T_LU + T_chain`.

- [exact within the model] Dense: `f_chain = (16 n + 8 rho + 112) / (2 n^2 / 3 + 16 n + 8 rho + 112 + T_J / n)`.
  The factorisation exceeds the chain iff `n^2 - 24 n - 12 rho - 168 > 0`, i.e. `n > 12 + sqrt(312 + 12 rho)`:
  35.5 for `rho = 20`, 167.9 for `rho = 2000` (check S7.a).
- [exact within the model] Banded: `f_chain = (32 b + 8 rho + 112) / (b^2 + 32 b + 8 rho + 112 + T_J / n)`.
  The factorisation exceeds the chain iff `b > 16 + sqrt(368 + 8 rho)`: 39.0 and 143.9.
- [exact within the model] Finite-difference Jacobian, dense: `T_J / T_chain = rho n / (16 n + 8 rho + 112)`,
  which tends to `rho / 16`; it exceeds the factorisation while `n < 1.5 rho`.
- [exact within the model] Matrix-free: `f_chain = 1 - T_P / T`; the levers on `J` and `LU` are void.

Bounds of the levers (Amdahl; `P` cores):

| lever | bound |
|---|---|
| parallel Jacobian assembly | `1 / (1 - f_J (1 - 1/P))`, at most `1 / (1 - f_J)` |
| reuse of `J` and `LU` over `m` steps at an unchanged step count | `1 / (1 - (f_J + f_LU)(1 - 1/m))`, at most `1 / f_chain`; divide by the step-count ratio, which S4 predicts to grow because the order drops to 3 |
| shared Krylov basis of dimension `8 theta kbar` instead of eight bases of `kbar` | `(8 kbar T_jvp + 16 kbar^2 n) / (8 theta kbar T_jvp + 128 theta^2 kbar^2 n)`; the product count improves by `1/theta <= 8`, the orthogonalisation only if `theta < 1/sqrt(8)` (check S7.b) |
| speculation | `S_2(r)` of S6 with `r = (T_J + T_LU) / T_chain`, or `r = T_J / T_chain` without step-size prediction |

Evaluated table: `RESULTS_TABLES.md`, section S7 (32 cases). Three readings, all [conjecture] until measured:

1. The chain dominates (`f_chain` 0.50 to 0.84) for small dense systems (`n = 8`) and one-dimensional banded
   systems (`b = 2`). There the speculation bound is 1.14 to 1.25 and the other bounds are at most 1.7 on
   6 cores.
2. For dense `n >= 512` and for bandwidths `>= 128` with a cheap right-hand side the factorisation holds 0.63 to
   0.99 of the step. Speculation is bounded by 1.00 to 1.06; reuse is bounded by 2.5 to 3.9 for `m = 4`.
3. With a finite-difference Jacobian and an expensive right-hand side (`rho = 2000`) the assembly holds 0.84 to
   0.93 of the step for dense `n = 64, 512` and for banded systems with `b >= 64` (0.42 at dense `n = 4096`,
   0.38 at `b = 2`); there parallel assembly on 6 cores is bounded by 3.3 to 4.5 (1.5 at dense `n = 4096`).

## S8. Schwarz locality

Scalar operator. `W = I - gamma h D Laplace_h`, `sigma = gamma h D / dx^2 = ell^2`.

- [exact] 1D three-point: the decaying solution of `W u = 0` is `rho^d` with
  `sigma rho^2 - (1 + 2 sigma) rho + sigma = 0`, `rho = (1 + 2 sigma - sqrt(1 + 4 sigma)) / (2 sigma)`.
  For `sigma = m(m+1)`, `rho = m/(m+1)` (checks S8.a). The lattice Green's function is
  `G_d = rho^|d| / sqrt(1 + 4 sigma)` (check S8.b).
- [exact for the formula, numerical to 50 digits] `rho = exp(-theta)`, `theta = 2 asinh(1 / (2 ell))`, so
  `ell ln(1/rho)` = 0.881, 0.962, 0.9959, 0.99958, 0.999996 for `ell` = 0.5, 1, 3.16, 10, 100: the decay length is
  `ell` cells once `ell >= 3`.
- [exact] On a finite Dirichlet grid (121 points, `sigma = 6`) successive ratios of the Green's function equal
  2/3 within 1.3e-15 (image terms); the second script obtains the same from the closed form.
- [numerical] 2D five-point (31 x 31, Dirichlet in x, periodic in y; conjugate gradients in the first script,
  eigen-expansion in the second). The sums over y of the Green's function equal the 1D Green's function
  (relative deviation below 1e-15), so they decay exactly like `rho^d`. Along the axis the ratio
  `G(d+1,0)/G(d,0)` rises towards `rho` from below (0.334 to 0.468 for `rho = 0.5`; 0.434 to 0.625 for
  `rho = 0.667`), as the algebraic prefactor of a 2D Green's function requires. Along the diagonal the ratio
  stays below `exp(-2 acosh(1 + 1/(4 sigma)))`. For strip subdomains the 1D `rho` bounds every transverse mode.

Per-sweep contraction, scalar problem. [exact for the half-line model] Two subdomains whose artificial
boundaries are `D` cells apart exchange Dirichlet data once per parallel sweep; the interface error is
multiplied by `rho^D` per sweep (`rho^{2D}` per alternating double sweep). With a one-sided halo of `delta`
cells added to each subdomain, `D = 2 delta + 1`. For a subdomain of width `H` with data on both sides the
factor is at most `(rho^D + rho^{H-D}) / (1 + rho^H)`. With `delta = m ell`: `rho^D` is about `exp(-2m)`,
i.e. 3.4e-4 for `m = 4`. This matches the reduction of 1e3 to 1e4 per sweep seen in the plan-stage toy if its
"overlap 4 ell" was a one-sided halo; [conjecture] on that reading.

Eight-stage block. A subdomain runs the whole chain with halo values of all stage vectors taken from the
previous sweep. For linear diffusion the stage-error vector `e(d)` (eight components per cell) satisfies
`W e = K e` in the interior, with `K = N beta^-1 = (I + N/gamma)^-1 (N/gamma)` nilpotent.

- [exact] The halo-to-interior transfer is the matrix function `Phi(d) = rho(K)^d`, where `rho(kappa)` is the
  decaying root of `sigma (rho + 1/rho) = 1 + 2 sigma - kappa`. It satisfies the block difference equation and
  `Phi(a) Phi(b) = Phi(a + b)` (check S8.f, `sigma = 6`, exact rationals).
- [exact] A direct simulation of the local chain on an interval `[0, H]`, `H = 12`, with arbitrary stage-error
  data at one end equals `(Phi(d) - Phi(2H - d)) (I - Phi(2H))^-1 E_0` (check S8.g). The second script confirms
  `Phi` on `H = 160` with Taylor coefficients from a contour integral (deviation 1.2e-46).
- [exact] In the U variables the transfer is `Gamma Phi Gamma^-1`: the subdomain solves are exact chains, so the
  two forms differ only by the change of variables.
- [numerical] In the continuum limit `Phi(c ell) = exp(-c (beta/gamma)^{-1/2})`: every eigenvalue is `exp(-c)`,
  but the matrix is non-normal. The discrete and the continuum norms agree within 0.4% at `ell = 10` and within 4e-5
  at `ell = 100`.

How the transient depends on S3. `Phi(D) = rho^D sum_{k=0}^{6} p_k(D) K^k` with polynomials `p_k` of degree `k`
in `D/ell`; the powers of `K` have infinity-norms 41.1, 45.1, 50.3, 35.4, 14.2, 2.32, 0 (compare
`(N/gamma)^m`: 37.1, 24.3, 23.3, 20.5, 6.39, 2.32, 0). The spectral radius of the sweep is `rho^D`, its norm is
larger by a polynomial factor of degree 6 that comes from the same nilpotent coupling that makes truncated
stage sweeps unstable. [numerical], `ell = 10`:

| `D / ell` | `rho^D` | norm(Phi) / `rho^D`, k variables | norm(Phi), k variables | norm(Gamma Phi Gamma^-1) / `rho^D`, U variables | norm(Gamma Phi Gamma^-1), U variables |
|---|---|---|---|---|---|
| 1 | 0.368 | 19.1 | 7.02 | 17.9 | 6.58 |
| 2 | 0.135 | 37.1 | 5.03 | 28.2 | 3.82 |
| 4 | 0.0183 | 75.4 | 1.38 | 48.6 | 0.891 |
| 6 | 2.48e-3 | 120 | 0.298 | 72.9 | 0.181 |
| 8 | 3.37e-4 | 175 | 0.0589 | 117 | 0.0393 |
| 12 | 6.17e-6 | 601 | 3.71e-3 | 302 | 1.86e-3 |
| 16 | 1.13e-7 | 1763 | 2.00e-4 | 519 | 5.88e-5 |

Expected reduction of the stage-error vector (U variables, infinity-norm, worst case) after `q` parallel sweeps,
one-sided halo `ceil(m ell)`, `ell = 10`. Model: two half-line subdomains, linear constant-coefficient diffusion.

| m | halo cells | `rho^D` | q = 1 | q = 2 | q = 3 | q = 4 |
|---|---|---|---|---|---|---|
| 1 | 10 | 0.123 | 3.54 | 0.769 | 0.140 | 0.0302 |
| 2 | 20 | 0.0166 | 0.828 | 0.0345 | 1.43e-3 | 4.40e-5 |
| 3 | 30 | 2.25e-3 | 0.166 | 1.57e-3 | 1.07e-5 | 1.14e-7 |
| 4 | 40 | 3.05e-4 | 0.0368 | 5.07e-5 | 1.23e-7 | 2.36e-10 |
| 6 | 60 | 5.59e-6 | 1.71e-3 | 1.33e-7 | 1.09e-11 | 4.6e-16 |

Reading for V14 [conjecture, to be tested there]: at the pre-registered maximum halo `4 ell` the first sweep
reduces a worst-case halo error by 27, not by 3300; two sweeps reach 5e-5. The pass criterion "median sweeps
<= 2" therefore depends on the halo predictor supplying an initial error well below the stage magnitude. With
a halo of `2 ell` the worst case needs four sweeps for 4e-5, which is the kill boundary. Reaction terms with
negative Jacobian eigenvalues shorten the decay length (`ell_eff^2 = gamma h D / ((1 + gamma h |r|) dx^2)`);
nonlinear and variable-coefficient problems are outside the proof.

## S9. Level-structured ROW tableau: counting and necessary conditions

Setting. Stages are grouped into `L` levels; a stage may depend (through `alpha` and `gamma_ij`) only on stages
of earlier levels, so the stages of one level can be evaluated and solved concurrently and the depth is `L`.

Structural facts [exact]:

- Two stages on level 1 would both have the argument `y_n` and no coupling, hence be identical: level 1 holds
  one useful stage.
- With stiff accuracy (`b` = last row of `beta`) a second stage on the last level has weight 0: the last level
  holds one useful stage. A two-processor stiffly accurate tableau has `s = 2L - 2` stages: 8 for `L = 5` (the
  stage count of RODAS5P), 10 for `L = 6`.
- `N^L = 0`, so `R(z) = P(z) / (1 - gamma z)^L` with `deg P <= L - 1` when `R(infinity) = 0`.

Necessary conditions from the stability function:

- [exact] `L = 4`: order 5 with `R(infinity) = 0` needs the coefficients of `z^4` and `z^5` of
  `exp(z)(1 - gamma z)^4` to vanish together; the two polynomials in `gamma` have a constant gcd (check S9.a).
  **No-go.** [numerical] Without `R(infinity) = 0`, none of the four admissible `gamma`
  (0.0913, 0.1745, 0.3889, 1.3454) is A-stable (check S9.b).
- [numerical] `L = 5`: order 5 with `R(infinity) = 0` forces `gamma` to a root of the coefficient of `z^5` of
  `exp(z)(1 - gamma z)^5`: 0.07911, 0.14113, 0.27805, 0.70751, 3.79420. Exactly one, `gamma = 0.2780538411`, is
  A-stable (check S9.c). `gamma` is not a free parameter, the linear order is exactly 5 with defect 5.30e-4 at
  `z^6`, and the linear order 6 of RODAS5P (defect -1.02e-5 at `z^7`) cannot be kept.
- [numerical] `L = 6`: order at least 5 with `R(infinity) = 0` is A-stable for `gamma` in `[0.18391465, 0.33414237]`;
  `L = 7` with linear order 6 for `[0.20408345, 0.37886489]` (check S9.d; Sturm test in the first script,
  root-finding test in the second). The RODAS5P value 0.21194 lies in both.

Counting of free coefficients [exact]: `gamma`, the allowed entries of `alpha` and `gamma_ij`, and `b` unless tied.
Values in parentheses are not available: in these layouts stages `s-1` and `s-2` share a level, and the second
tie would need `alpha_{s-1,s-2} = gamma != 0`.

| layout | stages | levels | general | stiffly accurate | + `alpha_s = beta_{s-1}` | + `alpha_{s-1} = beta_{s-2}` |
|---|---|---|---|---|---|---|
| RODAS5P, sequential | 8 | 8 | 65 | 57 | 50 | 44 |
| 2 processors, 1-2-2-2-1 | 8 | 5 | 59 | 51 | 44 | (39) |
| 2 processors, 1-2-2-2-2-1 | 10 | 6 | 93 | 83 | 74 | (67) |
| 2 processors, 1-2-2-2-2, not stiffly accurate | 9 | 5 | 74 | - | - | - |
| 3 processors, 1-3-3-3-1 | 11 | 5 | 104 | 93 | 83 | (76) |

Conditions. Order 5 for ODEs: 17 trees. RODAS5P imposes more (Steinebach 2023, Tables 2 to 4): the additional
index-1 conditions of Table 3, and Table 4 with No. 41-44 (singular `M(y)`, Di Marzo), No. 45-47 (inexact
Jacobian, Jax) and No. 48, 49 (Prothero-Robinson, i.e. semi-discretised parabolic problems), plus hatted
conditions for the embedded method and the dense-output system ("3 * 13 = 39 linear equations to be satisfied
by 3 * s = 24 coefficients"). The numbering ends at 49. A count of 23 for Table 3 is inferred from 49 - 17 - 9
and agrees with "the first 40 conditions" required of an order-5 embedded method in Steinebach's Rodas6P
preprint (arXiv 2511.21252); UNVERIFIED against the table. The article states that 40 coefficients remained
to be determined; 44 minus four row-sum normalisations would give that number, an identification that is
UNVERIFIED. No. 48 and 49 each stand for a family of scalar equations (`A_i = 0`, `B_i = 0`, equations
(2.5)-(2.10) of the article), so the number of scalar equations exceeds 49.

Go / no-go by counting:

- `L = 4`: **no-go** [exact].
- `L = 5`, two processors: 44 unknowns against 17 ODE conditions and the 8 order-4 conditions of the embedded
  method leaves a surplus of 19: counting does not exclude an order-5, stiffly accurate, L-stable ROW pair of
  depth 5. Against the numbered RODAS5P set (49 and more) the count is 44, the same as for RODAS5P itself, one
  unknown is consumed because `gamma` is pinned, and RODAS5P needed a least-squares closure with "10 conditions,
  7 degrees of freedom". Counting is therefore inconclusive for the full property set and one property (linear
  order 6) is lost for certain. **Conditional go**: worth a numerical search only for the reduced target
  (ODE order 5, stiff accuracy, embedded order 4, W-order 2), with the Prothero-Robinson conditions as a
  stretch goal.
- `L = 6`, two processors: 74 unknowns, `gamma` free in an A-stable window that contains the RODAS5P value.
  **Go** for a search.
- [conjecture] Pay-off. The depth ratio bounds the gain on the chain: 8/5 = 1.6 for `L = 5` at the same number
  of stages and solves, 8/6 = 1.33 for `L = 6` with 10 stages, i.e. 25% more total work. The gain on the step is
  that factor diluted by `f_chain` of S7 and by the change of the error constants, which is unknown. A new
  tableau is a new method outside the protected RODAS5P path and needs its own qualification.

## Corrections to the plan-stage facts (`PLAN_KO.md` section 1.4)

1. Facts 1, 2, 4 and 5 are confirmed, with residuals: `beta_21` 6.2e-17, `sum b_i gamma_i` -3.3e-16, order-3
   W defects 0.043, 0.030, -0.029, embedded `sum b_hat_i gamma_i` -0.02564, orders 3 and 4 for `g_7` and `g_8`,
   perturbation orders 7 and 8 confirmed by experiment.
2. Fact 3: the numbers 37, 24, 23, 20, 6.4, 2.3, 0 are correct for `N/gamma`, which is the sweep matrix of the
   k variables. R6 and R7 sweep the U variables, where the matrix is `gamma (z A + C)/(1 - gamma z)`, the
   nilpotency index is 8, and the norms are 23.9 to 83.8 in the stiff limit and 68.8 to 257 at the non-stiff
   end. "Amplification by 20 to 40" is norm dependent; the invariant statement is `|R_q(infinity)|` up to 3.7
   (k-form) and 84 (U-form), with no A-stable truncation.
3. New: a lagged Jacobian reduces the classical order to 3 (S4); `g_6` has order 1 (S5); the factorisation
   cannot be speculated without a step-size prediction (S6); the block Schwarz sweep has a transient factor of
   49 to 117 at `D = 4 ell` to `8 ell` in the U variables (S8).

## Sources

- Steinebach, BIT Numer. Math. 63, 27 (2023): https://link.springer.com/article/10.1007/s10543-023-00967-x
- Steinebach, Rodas6P and Tsit5DA preprint: https://arxiv.org/abs/2511.21252
- Steinebach, Improvement of Rosenbrock-Wanner Method RODASP (2020): https://link.springer.com/chapter/10.1007/978-3-030-53905-4_6
- Hairer and Wanner, Solving Ordinary Differential Equations II, sections IV.6 and IV.7: cited from memory, UNVERIFIED.
