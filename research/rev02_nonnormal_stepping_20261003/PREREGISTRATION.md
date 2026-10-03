# Preregistration: certified stepping, automatic metric and directed exponential for nonnormal exp actions (review DAG node REV-02)

## Question

The critical review (C2) found three limits in INT-05 (L-0056):

- The useful metric for VIG-A02 was chosen by hand from prior knowledge.
- One Taylor step is useless once `tau |W|` is large, which is the stiff case.
- The growth factor uses the platform `exp` with a margin instead of a directed exponential.

Can all three be removed, with every bound still enclosing a 50-digit reference, and with useful bounds on stiff
convection-diffusion and on fresh random nonnormal matrices?

## Method (fixed now)

**Directed exponential.** `directed::exp_interval(x)` encloses `e^x` for finite `x`:

- `k = round(x / ln 2)`;
- `r = x - k [ln 2]` in interval arithmetic, with `ln 2` enclosed by two adjacent doubles;
- `e^r` from the degree-20 Taylor sum in interval arithmetic, plus the remainder `|r|^21 / 21! e^|r|`;
- scaling by `2^k`.

An overflow is an error; a result below the normal range gives `[0, tiny]`. The INT-05 truncation bound switches to
it. In the INT-05 function this replaces the platform `exp` and its 4 eps margin, and nothing else.

**Osborne metric.** `osborne_metric(A)`: the classical balancing iteration with powers of two (as in LAPACK
`gebal` without permutation). It repeatedly sets `d_i <- d_i 2^e` so that the off-diagonal row and column 1-norms of
row/column `i` of `D A D^-1` are within a factor 2, until a sweep changes nothing (at most 100 sweeps). `D` is
computed from `A` alone, and `D A D^-1` is exact in binary64.

**Certified stepping.** `certify_exp_action_stepped(A, v, tau, metric, degree, steps)` works with `h = tau / N`,
`B = D A D^-1` enclosed, and `Omega` as in INT-05. Starting from `x_0 = D v`, for each `j`:

- `P_j` is the interval Horner enclosure of `p_m(h B) x_j`, and `x_(j+1) = mid(P_j)`;
- `rho_j = ||rad(P_j)||_2` (upper) is the local rounding;
- `t_j = (1 + sqrt 2) S_h ||x_j||_2` is the local truncation, with `S_h >= sup_{h Omega} |e^z - p_m(z)|`.

The error in `B` coordinates is bounded by Crouzeix-Palencia applied once per term, not compounded:

`E_B <= (1 + sqrt 2) sum_j e^{(N - 1 - j) h a_hi} (t_j + rho_j)`.

The physical candidate is `x = D^-1 x_N`, and `error_upper = ||D^-1||_2 E_B + ||x - D^-1 x_N||` (back-transform
rounding, enclosed).

- Fixed rules: degree `m = 20` and `N = max(1, ceil(tau R))`, with `R` the largest modulus on `Omega`.
- **Automatic certificate:** `certify_exp_action_auto(A, v, tau)` computes the stepped certificate for the identity
  and for `osborne_metric(A)`, and returns the one with the smaller `error_upper`. Both are valid bounds, so their
  minimum is too.

## Families (fixed now)

- F1. VIG-A02 `[[-2, 2^k], [2^-k, -2]]`, k in {10, 46}, `tau = 1`, `v = e1`.
- F2. Central convection-diffusion `D2 - Pe D1`, n = 32, Pe in {10, 50, 200}, tau in {1e-2, 1e-1},
  `v_i = sin(pi x_i)`. These are stiff: `tau ||A||` up to about 440.
- F3. Fresh random nonnormal, n = 16. `A = -diag(lambda) + U`, with `lambda_i = 10^(2 i / 15)` and `U` strictly upper
  with entries uniform in `[-mu, mu]` (SplitMix64 seeds 1 to 4). mu in {1, 30}, `tau = 0.5`, `v = (1, ..., 1)`.
- F4. INT-05's Jordan family, n = 8, mu in {10, 100}, `tau = 1` (seen; reported).

## Commands

1. `REV02_CASES=research/rev02_nonnormal_stepping_20261003/cases.json cargo test --release -p rodas5p-core --locked --test rev02_nonnormal_stepping -- --ignored --nocapture --test-threads=1`
2. `python3 tools/rev02_nonnormal_check.py --cases research/rev02_nonnormal_stepping_20261003/cases.json --output research/rev02_nonnormal_stepping_20261003/RESULTS.json`
   (50-digit `mpmath.expm` and `mpmath.exp`)

`cargo test -p rodas5p-core --locked --test rev02_nonnormal_contracts`

## Gate

**PASS** if all hold:

1. **Enclosure.** In every `Bounded` stepped certificate (both metrics, all families), the 50-digit error is at most
   `error_upper`.
2. **Directed exponential.** `exp_interval(x)` contains the 50-digit `e^x` at 201 points evenly spaced in
   `[-700, 700]` and at 50 points in `[-1, 1]`.
3. **Stiff usefulness.** For every F2 case, the automatic certificate has `error_upper <= 1e-8 ||exp(tau A) v||`
   (50-digit norm).
4. **Automatic metric.** For F1 with k = 46, the Osborne-metric stepped certificate has
   `error_upper <= 1e-10 ||exp(tau A) v||`, where INT-05 needed a hand-chosen metric.
5. **Contracts.** Zero steps, a degree outside `1..=60`, a non-finite or negative `tau`, a non-positive metric entry
   and shape mismatches are rejected. `exp_interval` rejects NaN and overflow.

Otherwise **FAIL**. Reported, not gated (P1): F3 and F4 usefulness (`error_upper` over the true error), step counts,
which metric the automatic certificate chose, and the INT-05 single-step bound on the same cases for comparison.

## Prior information

L-0056 (INT-05) and the critical review C2. Gate thresholds 1e-8 and 1e-10 are relative to the 50-digit solution
norm, not absolute WRMS levels (P1). No code of this node exists before this commit.
