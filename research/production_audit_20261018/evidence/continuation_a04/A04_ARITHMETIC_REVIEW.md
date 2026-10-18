# A04 arithmetic closure review

- Checkout: `audit/production-readiness-20261018`
- HEAD: `de990927d0f9197f0e37fcaeccee33bea6c7b8af`
- A04 source: `crates/rodas5p-core/src/leja_action.rs`
- Contract: `research/production_audit_20261018/NEXT_STEPS.json`, node A04
- Decision: `HOLD`; `TotalErrorStatus::EstimateOnly` remains mandatory.

## Bounded change

The `LEJA_ARITHMETIC_HOLD` reason now names the open arithmetic chain: operator products, recurrence, coefficient-weighted sums, and final output scaling/fusion. The regression `test_a04_subnormal_domain_keeps_total_authority_on_hold` exercises a smallest-subnormal input with a precomputed trusted coefficient cache, asserts the HOLD reason, and confirms the cached table is reused. Public forged insertion remains impossible because `insert_certified` is private; the compile-fail doctest is retained and passed. A fresh scalar-branch regression caught 30 allocations on a warmed h=0 call. The branch now computes its one-node coefficients with a fixed stack array and directed intervals; the warmed call is allocation-free. The test allocator now stores its counter per thread, so parallel test threads cannot reset or contaminate one another's counts.

No numerical threshold or certificate authority changed. Warmed zero-allocation coverage remains in the focused contract target.

## Why the total certificate remains held

1. `DenseMatrix::matvec_into` multiplies and sums in ordinary binary64. The recurrence residual uses a relative `gamma_n * rho * ||r||` term; it has no additive underflow allowance for each matrix product and accumulation.
2. The recurrence then performs `h*y`, `center*r`, division by `scale`, and `node*r` in ordinary arithmetic. The current relative terms do not enclose additive underflow in these operations.
3. Newton column accumulation uses ordinary `out += coeff * r`; the summation estimate is relative-only and has no additive underflow floor.
4. Column scaling and final fused output accumulation also use ordinary arithmetic. Their error term likewise does not enclose additive underflow.
5. The coefficient Opitz/Taylor interval engine is outward-rounded and cache entries are generated internally. That does not close the unrelated operator-action and output arithmetic above. The `t3` recurrence-propagation tail accumulation uses `|d| + d_rad` with the suffix bound; the separate `t2` component sums `d_rad * ||r||`. This coefficient-radius factor in `t3` is not the missing additive-underflow enclosure and does not close the other arithmetic paths.
6. `certificate_l2_upper` uses directed operations and rejects nonfinite values, but that norm enclosure cannot repair errors that were omitted from the vectors it receives.

A sound repair needs an outward operator action or an explicit additive roundoff residual bound for every multiply/add/divide in the matrix product, Newton recurrence, coefficient-weighted accumulation, column scaling and final fusion; that bound must be propagated through the suffix polynomials and validated against independent high-precision subnormal/normal/overflow cases. Do not restore certified status until that whole path is covered.

## Evidence

- Initial test edit failed to compile because the test omitted the existing `leja_phi_action_certified_with_cache` import; preserved at `a04_subnormal_cache_initial.log`.
- After adding the import, the adversarial subnormal plus trusted-cache regression passed: `a04_subnormal_cache_baseline.log`.
- Final focused target passes 14/14, including Newton-warmed and scalar-warmed zero-allocation plus thread-local tracker isolation: `a04_leja_contract_final.log`.
- Public caller-supplied coefficient insertion compile-fail doctest passed 1/1: `a04_cache_api_doctest.log`.
- rustfmt applied to owned Rust files; `git diff --check` recorded by the continuation agent after final edits.

## Scalar allocation repair

The zero-width scalar branch previously called `leja_divided_differences_certified(center, 0, &[2])`, allocating its temporary coefficient and radius vectors on every call. It now computes the one-node `phi_k(center)` values and radii into fixed arrays using outward interval operations. This repairs the warmed scalar allocation contract without changing the arithmetic HOLD or any threshold. For `|c|<=1`, the helper evaluates directed Taylor intervals with an outward geometric tail; for larger negative `c`, it uses the standard interval recurrence `phi_k(c)=(phi_{k-1}(c)-1/(k-1)!)/c`; at `c=0` it uses exact factorial values. Interval overflow or a denominator containing zero remains a typed refusal. A nonzero-center comparison initially differed from the reference by `3.53e-14` under the recurrence-only route; the directed Taylor path fixed the witness, with the first failure and repair logs preserved.

## Fresh scalar allocation audit

The first warmed `h=0`, nonzero-input regression failed with exactly 30 allocations. The source was the scalar branch allocating temporary coefficient and radius vectors on every call. The branch now evaluates its one-node `phi_k(c)` values and outward radii into fixed arrays. The same warmed call passes with zero allocations. The allocator test counter is thread-local (like its tracking flag), removing the old global reset race; a two-thread isolation test passes.

## Zero-center factorial radius review

An in-module regression directly calls `scalar_phi_coefficients(0)` and requires nonzero radii for `phi_3(0)=1/6` and `phi_4(0)=1/24`, with the returned center/radius endpoints reaching beyond the adjacent binary64 floor/ceiling pair that encloses each exact rational. Against the old point intervals the regression failed at `phi_3 needs a nonzero radius` (`a04_zero_radius_first_failure.log`). Replacing those point intervals with `div_down(1,factorial)` / `div_up(1,factorial)` endpoints made it pass (`a04_zero_radius_repair.log`). The integration-level sum test was removed because other component bounds masked this coefficient-radius defect.

The low-level allocation documentation now limits the claim to a successful preallocated call with fixed dimensions, cap, cache state, and adequate unchanged report-string capacity; the convenience wrapper allocation caveat is explicit.
