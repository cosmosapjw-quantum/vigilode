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
