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

## Amendment (before any recorded run, while writing the code)

`tau / N` must be exact in binary64, or the method integrates to `N fl(tau / N)` instead of `tau`. The step rule
therefore rounds `N = max(1, ceil(tau R))` up to the next power of two. That can only increase `N`, by less than a
factor of 2, so the bound stays valid. The stepped certificate refuses a `(tau, N)` whose quotient is not exact. A
step count above 100,000 makes the certificate `Unbounded`: the identity metric of VIG-A02 at k = 46 would need
3.5e13 steps. Nothing else changes, and no study code had run when this was written.

## Prior information

L-0056 (INT-05) and the critical review C2. Gate thresholds 1e-8 and 1e-10 are relative to the 50-digit solution
norm, not absolute WRMS levels (P1). No code of this node exists before this commit.

---

## Results (appended after the run at `979a2c8`)

Outputs: `cases.json` (native certificates) and `RESULTS.json` (50-digit check, mpmath 1.3.0). Ledger row L-0060.
Contract tests `rev02_nonnormal_contracts` 3/3 and `int05_nonnormal_contracts` 3/3. One change came after the
preregistration and before the run: the range reduction in `exp_interval` uses the two-part (Cody-Waite) `ln 2`.
`LN2_HI + LN2_LO` is below `ln 2` by 1.16e-26, less than one ulp of `LN2_LO`, checked at 60 digits. This keeps the
enclosure's width at a few ulps instead of `|k|` ulps; the contract test that caught the wide version stays.

**Gate: FAIL** (items 1, 2, 4 and 5 hold; item 3 fails).

| Gate item | Outcome |
|---|---|
| 1. Enclosure | **holds**: every bounded stepped certificate (both metrics, 18 cases) is above its 50-digit error |
| 2. Directed exponential | **holds**: all 251 grid points enclose the 50-digit `e^x` |
| 3. Stiff usefulness | **fails**: three F2 cases meet `1e-8 ||exp(tau A) v||` (relative bounds 3.5e-14 to 1.9e-12), three do not. In those three the solution has decayed to 2e-7, 3e-24 and 1e-91, while the certified absolute bound stays at 9e-14 to 2e-13 |
| 4. Automatic metric | **holds**: for VIG-A02 at k = 46, Osborne balancing, computed from `A` alone, gives a stepped bound of 2.2e-15 relative in 4 steps. The identity would need 3.5e13 steps (`Unbounded`) |
| 5. Contracts | **holds** |

What the run shows:

- **Stepping removes the stiffness limit on the absolute error.** On all six stiff convection-diffusion cases the
  stepped bound is 9e-14 to 7e-13 absolute (`||v|| = 4`), 99 to 257 times the true error where the solution has not
  decayed. INT-05's single Taylor step on the same cases certifies nothing usable: 2e17 to 6e147 relative.
- **The bound does not decay with the solution.** For central differences, the symmetric part of `A` is the diffusion
  matrix, and its Gershgorin upper bound is 0 although its largest eigenvalue is about -10. So the bound gives early
  rounding errors no decay, while the solution decays by `e^{-55}` to `e^{-210}`. Item 3's threshold, set relative
  to the solution norm, could not be met there. Choosing a reference that varies over 90 orders of magnitude without
  calibration is the P1 mistake the critical review named.
- **Strongly nonnormal random matrices stay out of reach.** For F3 with mu = 30 and the Jordan block with mu = 100,
  the certified bounds are 1e9 to 1e19 relative, valid but useless. Gershgorin bounds on the numerical range of such
  matrices reach far into the right half plane (transient growth), and Osborne balancing helps on one seed out of
  four. For mu = 1 the bounds are within 70 to 2,500 times the true error (relative 5e-14 to 7e-14).
- **Metric choice.** The automatic choice picked the identity in 16 of 18 cases and Osborne in 2. Osborne helped on
  VIG-A02 and on one random seed, and was worse or equal elsewhere.

**What would remove the two limits** (not done here): a verified upper bound on the largest eigenvalue of the
symmetric part (an inertia count of `H - mu I`, for example) in place of Gershgorin, which would make the bound decay;
and a metric that minimizes the numerical range rather than balancing norms. Claim ceiling: dense matrices up to
n = 32, degree 20, the fixed step rule; no timing.
