# RVJ-N1: nonzero damping-preserving collective causal Volterra candidate (2026-10-30)

## Publication type and source lock

This is an **isolated research checkpoint and source-archive receipt**. This branch is NOT a Rust implementation of RODAS8, NOT a full review, and NOT production admission. The entire executable Python source, tests, parent ZIPs, and measured output are in the immutable ZIP linked below, not in this GitHub tree.

* Full archive: [Google Drive exact binary](https://drive.google.com/file/d/19PXc1Lnbb0UsKY04Fy2o3x4Wh8wgcN_4/view), **7,984,786 bytes**, SHA-256 `605123cfebfcab73593edc9f100be622b5741c3c7fca435308131a5ea95875ae`.
* Exact binary was downloaded back from Google Drive and compared byte-for-byte with the local source ZIP: **PASS**. Its internal 21 manifest payloads and ZIP CRC independently verify PASS.
* Also uploaded to ChatGPT Library `/VIGILODE_RESEARCH/VIGILODE_RVJ_N1_DAMPED_AND_STAGE_PREFIX_20261030_FULL.zip`.
* Full next-thread handoff in the ZIP `HANDOFF_NEXT_THREAD_KO.md`, separately uploaded to Library and Drive.

**Original attached source identities:** RVJ loop12 ZIP SHA-256 `2735bf3ea1c52abae2e97a3310714eb4a9b47e4e08e786c7c0fc32831d97835c`; RVJ NAG ZIP SHA-256 `04727ed33a2fecd92a13e12c2230cf57a79e1a09aa90b63658d4319e9c37301b`.

## Source-specific theory

Dimensionless source-owned family with `w_j>0, Σw_j=1, κ≥0, χ∈ℝ, E(0)=Σw_j|a_j(0)|²≤1`:
```text
a_j' = (-κ+iωξ_j) a_j + iχ|m|²m,       m=Σ_j w_j a_j .
z_j=-κ+iωξ_j
a_j(t)=e^(z_j t)a_j(0)+ ∫_0^t e^(z_j(t-s)) iχ|m(s)|²m(s)ds .
E(t)=E(0)e^(-2κt) .
```
Factor common damping **exactly**; Taylor expand only frequency oscillations. Construct frozen free mean `m0(t)=Σw_j e^(z_jt)a_j(0)` and truncated `m0tilde(t)=e^(-κt)P_p(t/T)`, using original initial moment coefficients including odd moments.

With `Qbar=min(T,1/κ)` (κ>0; else T), `Abar≥Σw_j|a_j(0)|`, `τ_p=(|ω|T)^(p+1)/(p+1)!`, `q=3|χ|Qbar<1`, `Abar(1+τ_p)≤1`, and `b=|χ|Abar³Qbar`, the exact-real one-Picard mode candidate has uniform analytic bound
```text
max_{j, 0≤t≤T} |a_j(t)-a_hat_j(t)| ≤ q*b/(1-q)+q*Abar*τ_p.
```
The **source** computes and enforces a conservative rational `Abar`, refusing caller-forged undersized bounds. This is a source-model-specific analytical error bound, NOT a directed-rounding claim for mpmath numerical evaluation.

For an *affine strictly lower-triangular* eight-stage map `U=b+LU, L^8=0`: `(I-L)^-1=(I+L)(I+L²)(I+L⁴)`. This can inform a parallel prefix only for verified affine/current linearized packets with original-space RODAS5P residual checks. It does **not** imply unconditional nonlinear Rosenbrock stage parallelization.

## Actual bounded evidence

* New Python unit tests: 13/13 PASS, including source-bound negative forging, unchanged odd initial data, exact damped coalescent limit, domain refusal, and nonlinear prefix non-equivalence. Re-run 2026-10-30, 13/13 PASS.
* Replayed parent RVJ loop12 tests 23/23 PASS and parent RVJ NAG `VALIDATE.py` with manifest 329/329 PASS. These are separate inherited scopes.
* Experimental configurations: n=17/65/257 with κ=1,000, n=17 κ=10,000. At specified `tol=1e-7`, analytical bounds respectively 2.72148e-8, 2.71553e-8, 2.71593e-8, 2.63081e-8. For n=17 κ=1000, t=.001 comparison with SciPy BDF max diff 2.01436e-11; diagnostic comparison, NOT full physical correctness/rigorous arithmetic bound. Nonzero damping never approximated by Taylor time stepping.
* Protected small-system rule `n≤8 -> baseline` avoids speculative expensive route; **no proof of native speedup**. New Rust full-source compile, native all-eight interval certificate and matched physical performance **NOT_RUN**.
* Same-model blind-style audit found and repaired source upper-bound forgery; it is **not** an actual distinct author C9 review.

## Authority and DAG

```text
OFFICIAL_VIGILODE=7/23     OFFICIAL_REI=28/88
SUCCESSOR_ADMITTED=0/12    RVJ_LOOP12_FOLLOWON_N1=1/8_SCOPED
PR131=b289d8e8165bf7d5b1d3255c9a02e89bc4a20b9e
PR131_NATIVE_CI_SOURCE=6c060d963fa3257dd982d09a562c639bbd63fa3e
PR131_TESTS=49/49_SCOPED_PASS (historical, not rerun in this N1 experiment)
C9_REAL_DISTINCT_AUTHOR=0/1_NOT_RUN
C10=NOT_STARTED; PRODUCTION=HOLD; MERGE_READY=false; AUTO_NEXT_NODE=false
```

## Next bounded actions

1. Source-lock native stage eight all-residual/outward intervals for PR131, without moving C9 distinct-author receipt.
2. Add independent outward-rounded complex exponential moment evaluation, or exact algebraic enclosure, before claiming numerical certificate.
3. Connect an **opt-in** original-RODAS tableau frozen-source adapter: source-aware slow/fast detector, unchanged RHS-only homotopy as primary, JVP fallback only, exact stage residual, rejected work accounted.
4. Compare n=3/8/17/65/257 at matched achieved error including method setup/certificate/rejected attempt. Do not infer small-n speed from a hard skip alone.
5. Preserve the independent RVJ-NAG F5 geometry partial correction; remote `rvj-full-history-20261004` has **index/metadata only** for original 4550 paths, not raw full historical source payload.

Original historical PRs #74/#77/#90/#91/#97/#125/#129/#131 were inspected as conceptual/technical premises. Publication of this receipt does not modify production, base/head of any existing PR, or original parent archive.
