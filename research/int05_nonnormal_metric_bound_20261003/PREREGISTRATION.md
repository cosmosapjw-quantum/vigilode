# Preregistration: a certified output bound for exponential actions of nonnormal matrices (integrated DAG node INT-05)

## Question

External review finding TF-09 (task V1): for nonnormal operators, a sampled residual and a scalar spectrum do not
certify the output error. VIG-A02 is the pinned case: an Arnoldi residual of 6e-15 hid a true error of 7e-2. The
certified polynomial bounds in the repository are symmetric-only (L-0039, L-0047). Can a total output bound for
`exp(tau A) v` be certified for nonnormal `A`? The proposed route has three parts:

- a caller-supplied diagonal metric `D`, whose quality is judged rather than trusted;
- a verified enclosure of the numerical range of `B = D A D^-1`;
- the Crouzeix-Palencia theorem, `||g(B)||_2 <= (1 + sqrt 2) sup_{W(B)} |g|`.

The bound must enclose a 50-digit reference, expose the VIG-A02 candidate, and be useful when the metric is good.

## Contract (fixed now)

For a candidate `x`, `exp(tau A) v - x = D^-1 [e^{tau B} - p_m(tau B)] D v + (D^-1 p_m(tau B) D v - x)`, where
`p_m` is the degree-`m` Taylor polynomial. Then:

- **Truncation:** `||D^-1 [e^{tau B} - p_m(tau B)] D v||_2 <= ||D^-1||_2 (1 + sqrt 2) S ||D v||_2`, with
  `S = sup_{z in tau Omega} |e^z - p_m(z)| <= R^{m+1}/(m+1)! max(1, e^{tau a_hi})`. Here
  `Omega = [a_lo, a_hi] x i[-b, b]` contains `W(B)`. `a_lo` and `a_hi` are Gershgorin bounds of the symmetric part
  `H = (B + B^T)/2`, and `b` is the max row sum of the skew part `S = (B - B^T)/2`, which bounds its spectral radius.
  `R` is the largest modulus on `tau Omega`. The remainder bound follows from
  `e^z - p_m(z) = z^{m+1}/m! int_0^1 (1-t)^m e^{tz} dt`.
- **Evaluation:** `p_m(tau B) D v` is evaluated by Horner in outward interval arithmetic, with `B` enclosed entrywise
  from `d_i A_ij / d_j`. Mapped back by `D^-1`, it gives a componentwise enclosure `P` of the exact
  `D^-1 p_m(tau B_exact) D v`. Then `||x - (exact)||_2` lies in `[sqrt(sum mig(x_i - P_i)^2), sqrt(sum
  mag(x_i - P_i)^2)]`.
- **Total:** `error_upper = distance_upper + truncation_upper`, and
  `error_lower = max(0, distance_lower - truncation_upper)`. Every operation rounds outward.

`A`, `v`, `tau`, `D` and `x` are exact binary64 reals. No sampled residual, Ritz value or declared spectrum enters
the certificate, and the API has no parameter for one. The polynomial value `D^-1 mid(p_m(tau B) D v)` is returned
as the method's own candidate.

## Changes

`rodas5p-core/src/nonnormal_certificate.rs` (new):
`certify_exp_action(a, v, tau, metric: Option<&[f64]>, degree, candidate: Option<&[f64]>) -> CoreResult<NonnormalExpCertificate>`.
The result carries the candidate used, `error_upper`, `error_lower`, `truncation_upper`, `distance_upper`,
`transport = ||D^-1||_2 ||D v||_2`, the `Omega` box, the degree and a status: `Bounded`, or `Unbounded` when an
upper bound overflows. Inputs rejected: non-finite data, a non-positive or non-finite metric entry, dimension
mismatches, `degree` outside `1..=60`, and non-finite or negative `tau`. The existing symmetric polynomial
certificates are unchanged.

## Families (fixed now)

1. **VIG-A02:** `A = [[-2, 2^k], [2^-k, -2]]`, k in {0, 10, 20, 46}, `tau = 1`, `v = e1`. Metric `I`, and
   `D = diag(1, 2^k)` (which makes `B` symmetric).
2. **Jordan-like:** `A = -I + mu N` (`N` the upper shift), n = 8, mu in {1, 10, 100}, `tau = 1`,
   `v = (1, ..., 1)`. Metric `I`, and `d_(i+1) = d_i mu / rho` with `rho = 0.5`, `d_0 = 1`.
3. **Convection-diffusion, central:** `A = D2 - Pe D1` on n = 32 interior points (h_x = 1/33), Pe in {10, 50},
   `tau = 1e-3`, `v_i = sin(pi x_i)`. Metric `I`, and the symmetrizing weights `d_i = exp(-Pe x_i / 2)`, which are
   rounded to binary64 and used as given.

Degrees m in {10, 20, 30}. Candidates are the method's own polynomial value. For VIG-A02 there is also the
Arnoldi near-breakdown output `exp(-2 tau) e1`, the candidate the residual check accepted.

## Commands

1. `INT05_CASES=research/int05_nonnormal_metric_bound_20261003/cases.json cargo test --release -p rodas5p-core --locked --test int05_nonnormal_metric_bound -- --ignored --nocapture --test-threads=1`
2. `python3 tools/int05_nonnormal_check.py --cases research/int05_nonnormal_metric_bound_20261003/cases.json --output research/int05_nonnormal_metric_bound_20261003/RESULTS.json`
   (50-digit `mpmath.expm`)

`cargo test -p rodas5p-core --locked --test int05_nonnormal_contracts`

## Gate

**PASS** if all hold:

1. **Enclosure.** In every `Bounded` case, the 50-digit error `||exp(tau A) v - x||_2` is in `[error_lower,
   error_upper]`.
2. **The counterexample is exposed.** For VIG-A02 with k in {10, 20, 46} and the metric `D`, the Arnoldi candidate
   `exp(-2) e1` gets `error_lower >= 1e-2`.
3. **A good metric makes the bound useful.** For VIG-A02 with k = 46 and m = 30, the bound for the method's own value
   is at most 1e-10 under `D`. Under `I` it is `Unbounded` or above 1.
4. **Typed rejections** (contract tests): the invalid inputs listed above are `Err`.

Otherwise **FAIL**. Reported, not gated: `error_upper` over the true error per case, the effect of the metric on the
Jordan and convection-diffusion families, and the cases where the bound is valid but useless (large `R`). No timing,
and no change to `EstimateOnly` labels elsewhere.

## Stop condition

A bound below a 50-digit error, or any sampled quantity entering the certificate, stops the method.

## Prior information

VIG-A02 and its closed form, from `crates/rodas5p-integrators/tests/phi_nonnormal_breakdown_contracts.rs`. RA-03
noted that dyadic balancing makes it symmetric. L-0039 and L-0047 cover the symmetric case. Crouzeix and Palencia
(2017) is used as published; no other external source is applied. No code of this node exists before this commit.

---

## Results (appended after the run at `3079dd1`)

Outputs: `cases.json` (native certificates) and `RESULTS.json` (50-digit check, mpmath 1.3.0). Ledger row L-0056.
Contract tests `int05_nonnormal_contracts` 3/3.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Enclosure | **holds**: the 50-digit error lies in `[error_lower, error_upper]` in all 66 `Bounded` cases (78 cases, 12 `Unbounded`) |
| 2. Counterexample exposed | **holds**: with the metric `D = diag(1, 2^k)`, the Arnoldi candidate `exp(-2) e1` gets `error_lower` of 6.46e-2 (m = 10) and 7.3498e-2 (m = 20, 30) at k = 10, 20 and 46. At m = 30 the true error 7.3498136e-2 is pinned by bounds agreeing to 12 digits |
| 3. Metric useful | **holds**: VIG-A02 at k = 46 and m = 30 certifies the method's own value at 1.11e-15 (true error 6.6e-17) under `D`. Under `I` it is `Unbounded`: the Euclidean numerical range reaches `Re z = 3.5e13` |
| 4. Typed rejections | **holds** (contract tests) |

Degree 30, method's own value (true error / certified upper bound / transport `||D^-1|| ||D v||`):

| Family | metric `I` | metric `D` |
|---|---|---|
| VIG-A02 k = 10 | 6.6e-17 / 4.3e276 / 1 | 6.6e-17 / 1.1e-15 / 1 |
| VIG-A02 k = 20, 46 | 6.6e-17 / unbounded | 6.6e-17 / 1.1e-15 / 1 |
| Jordan mu = 1 | 1.0e-16 / 8.2e-16 / 2.8 | 1.0e-16 / 8.2e-16 / 148 |
| Jordan mu = 10 | 4.7e-13 / 1.5e7 / 2.8 | 4.8e-13 / 7.6e-12 / 1.3e9 |
| Jordan mu = 100 | 2.6e-6 / 4.4e76 / 2.8 | 1.3e-6 / 3.1e-5 / 1.3e16 |
| Convection-diffusion Pe = 10 | 1.7e-15 / 1.3e-13 / 4.1 | 1.7e-15 / 1.5e-12 / 87 |
| Convection-diffusion Pe = 50 | 2.4e-15 / 6.6e-13 / 4.1 | 2.6e-15 / 2.0e-7 / 2.4e9 |

What this shows:

- **A metric is a choice, not a free gain.** For VIG-A02 and the Jordan blocks with large `mu`, the diagonal metric
  turns an unbounded or useless Euclidean bound into one within 8 to 24 times the true error. For central
  convection-diffusion, the exponential weights (the continuum symmetrizer, which is not the exact one for central
  differences) make the bound worse than the identity, because their transport factor (87 and 2.4e9) outweighs the
  smaller numerical range. Gershgorin already keeps `W(A)` in the left half plane there.
- **The method's own value differs with the coordinates.** Taylor evaluation in `B` coordinates rounds differently.
  For Jordan mu = 100 the degree-30 truncation error is real (true error about 1e-6), and both bounds report it.
- **Where the bound is useless.** It is valid but large when `tau |W|` is large, since Taylor needs scaling that this
  node does not add (degree 10 for mu = 100: 1.2e11). In those cases a user reads the bound and rejects the value.
  Nothing is relabelled as an estimate.

Disclosure: the growth factor `e^{tau re_hi}` uses the platform `exp` with a relative margin of `4 eps` instead of a
directed exponential. It is 1 whenever `re_hi <= 0`, which holds for every bounded case with the metric except
VIG-A02 at k = 0. Every other operation rounds outward. Claim ceiling: dense matrices up to n = 32, a Taylor
polynomial without scaling and squaring, and caller-supplied diagonal metrics. No change to `EstimateOnly` labels
or to the symmetric certificates; no timing.
