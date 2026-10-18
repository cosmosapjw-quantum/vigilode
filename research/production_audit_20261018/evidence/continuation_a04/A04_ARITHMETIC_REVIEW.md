# A04 arithmetic closure review

- Checkout: `audit/production-readiness-20261018`
- Continuation base: `c3036c63da02ca9b04cd39ccf580cd67d00ccb0c`
- A04 source: `crates/rodas5p-core/src/leja_action.rs`
- Contract: `research/production_audit_20261018/NEXT_STEPS.json`, node A04
- Decision: `HOLD_PENDING_INDEPENDENT_REVIEW`; `TotalErrorStatus::EstimateOnly` remains mandatory.

## Interval replay continuation

The earlier HOLD list below records the pre-replay implementation and is now
superseded for the four arithmetic paths it described. The current source
replays the represented operator, recurrence, coefficient accumulation, and
output operations using outward intervals:

1. Each computed matrix-vector product is compared with an outward dot-product
   enclosure of the exact represented matrix and input vector. The recurrence
   is then replayed as intervals, and the norm of the actual computed result's
   distance from that interval is stored as its local residual. This captures
   cancellation and additive underflow in both the dot product and recurrence.
2. Each ordinary coefficient product/addition is mirrored in lower/upper
   buffers. `t2` continues to carry coefficient radii; `t3` transports the
   locally enclosed recurrence defect using the correctly indexed suffix and
   `|d| + d_rad` cross term.
3. Column scaling and final fusion are replayed from the interval coefficient
   sums. Their returned-output defect replaces the former relative-only `t4`
   model. Workspace buffers are preallocated by `ensure_capacity`.
4. The normalized spectral interval `(Interval(a,b)-center)/scale` is checked
   directly against `[-2,2]` before coefficient/node/suffix bounds are used.
   The scale is enlarged outward with a bounded loop or the call refuses.
5. The truncation remainder uses the directed upper of `center + 2*scale`
   after normalized-support enlargement, and takes `exp(max(0, support_hi))`.
   Scalar interval-width variation uses the maximum of that directed support
   upper and the verified interval upper. The regression checks an enlarged
   support upper that exceeds the original spectral endpoint `b`.
6. The attempted `certification_ops` proxy is flushed to the workspace and
   report before propagating action and output errors. The failure-path test
   verifies this value remains observable after a refused output.
7. Replay helper proxies are charged before fallible interval operations, so
   overflow refusal retains attempted work. `poly_block_products` and
   `poly_vector_products` are charged before each fallible matrix action; the
   overflow regression checks both these counters and the certification proxy.

Total authority remains held for independent review of the combined
finite-domain proof and coefficient-tail premises. All APIs still return
`EstimateOnly`; no threshold was changed. The new interval work adds a second
O(n²) matrix action per degree and O(n) preallocated buffers; `certification_ops`
is an arithmetic-work proxy, not a measured wall-time or exact instruction
count.

Targeted adversarial tests cover subnormal matvec/output underflow, cancellation
in matvec and coefficient sums, nonfinite refusal, the normalized boundary
case that previously refused after 64 one-ULP increments, and positive outward
support. The existing warmed Newton/scalar allocation contracts remain active.

## Bounded change

The `LEJA_ARITHMETIC_HOLD` reason now records that the outward replay implementation awaits independent review of arithmetic-domain and coefficient-tail premises. The regression `test_a04_subnormal_domain_keeps_total_authority_on_hold` exercises a smallest-subnormal input with a precomputed trusted coefficient cache, asserts the HOLD reason, and confirms the cached table is reused. Public forged insertion remains impossible because `insert_certified` is private; the compile-fail doctest is retained and passed. A fresh scalar-branch regression caught 30 allocations on a warmed h=0 call. The branch now computes its one-node coefficients with a fixed stack array and directed intervals; the warmed call is allocation-free. The test allocator now stores its counter per thread, so parallel test threads cannot reset or contaminate one another's counts.

No numerical threshold or certificate authority changed. Warmed zero-allocation coverage remains in the focused contract target.

## Historical pre-replay HOLD findings

The following findings describe the implementation before the interval replay
continuation above; they are retained as audit history, not current code state.

1. `DenseMatrix::matvec_into` multiplies and sums in ordinary binary64. The recurrence residual uses a relative `gamma_n * rho * ||r||` term; it has no additive underflow allowance for each matrix product and accumulation.
2. The recurrence then performs `h*y`, `center*r`, division by `scale`, and `node*r` in ordinary arithmetic. The current relative terms do not enclose additive underflow in these operations.
3. Newton column accumulation uses ordinary `out += coeff * r`; the summation estimate is relative-only and has no additive underflow floor.
4. Column scaling and final fused output accumulation also use ordinary arithmetic. Their error term likewise does not enclose additive underflow.
5. The coefficient Opitz/Taylor interval engine is outward-rounded and cache entries are generated internally. That does not close the unrelated operator-action and output arithmetic above. The `t3` recurrence-propagation tail accumulation uses `|d| + d_rad` with the suffix bound; the separate `t2` component sums `d_rad * ||r||`. This coefficient-radius factor in `t3` is not the missing additive-underflow enclosure and does not close the other arithmetic paths.
6. `certificate_l2_upper` uses directed operations and rejects nonfinite values, but that norm enclosure cannot repair errors that were omitted from the vectors it receives.

A sound repair needed the outward replay now implemented above. Independent
review must decide whether its assumptions and propagation close the full
mathematical claim before any authority change.

## Evidence

- Initial test edit failed to compile because the test omitted the existing `leja_phi_action_certified_with_cache` import; preserved at `a04_subnormal_cache_initial.log`.
- After adding the import, the adversarial subnormal plus trusted-cache regression passed: `a04_subnormal_cache_baseline.log`.
- Final focused target passes 14/14, including Newton-warmed and scalar-warmed zero-allocation plus thread-local tracker isolation: `a04_leja_contract_final.log`.
- Public caller-supplied coefficient insertion compile-fail doctest passed 1/1: `a04_cache_api_doctest.log`.
- rustfmt applied to owned Rust files; `git diff --check` recorded by the continuation agent after final edits.

## Interval replay continuation outcomes

- First compile attempt caught the missing `DenseMatrix` import:
  `a04_interval_replay_compile_first.log`; fixed before rerun.
- First normalized-boundary regression reproduced typed refusal after the
  original 64 one-ULP increments: `a04_normalized_boundary_regression.log`.
  The repair computes a directed required scale from the centered support;
  `a04_normalized_boundary_after_repair.log` passes and verifies enlargement.
- `cargo test --locked --offline -p rodas5p-core --lib production_audit_suffix -- --nocapture`:
  7/7 passed (`a04_interval_replay_unit_final.log`).
- `cargo test --locked --offline -p rodas5p-core --test leja_total_contract`:
  14/14 passed (`a04_interval_replay_contract_final.log`), including warmed
  zero-allocation paths.
- `cargo clippy --locked --offline -p rodas5p-core --all-targets -- -D warnings`:
  passed (`a04_interval_replay_clippy_final.log`).
- `rustfmt --edition 2024 --check crates/rodas5p-core/src/leja_action.rs` and
  `git diff --check`: passed after the final source edit.

The latest t1/cost follow-up had one transient integration assertion failure:
the HOLD reason text had changed but an old assertion still expected the
former arithmetic-chain wording (`a04_t1_cost_contract.log`). The assertion
now checks the current independent-review/coefficient-tail reason. A clippy
run also flagged the manual propagation of the already-flushed action result;
`outcome?` now propagates it after the counters are stored. Its first failure
is preserved as `a04_t1_cost_clippy.log`. Final focused results are 8/8 unit
tests (`a04_t1_cost_unit_final.log`), 14/14 contract tests
(`a04_t1_cost_contract_final.log`), and warning-free clippy
(`a04_t1_cost_clippy_final.log`). `a04_t1_cost_format.log` records successful
rustfmt checking and `git diff --check`.

An independent review then found that attempted-work charging in interval
matvec happened after the fallible operation, and operator product counters
were delayed until the whole multi-vector stage completed. The red test first
failed with zero helper work (`a04_attempted_work_red_actual.log`). An initial
overflow fixture refused at the Leja range guard before entering the action;
that failure is preserved in `a04_attempted_work_overflow_repaired.log` before
it was overwritten by the repaired run. The reduced spectral interval keeps
the action admissible while its finite input overflows a matrix product. The
regression now passes with nonzero workspace and operator counters in the same
log. A compile failure caused by inspecting the workspace while the returned
result still held its mutable borrow is preserved in
`a04_attempted_work_overflow_diagnosis2.log`; dropping the result before
counter inspection fixed it.

## Scalar allocation repair

The zero-width scalar branch previously called `leja_divided_differences_certified(center, 0, &[2])`, allocating its temporary coefficient and radius vectors on every call. It now computes the one-node `phi_k(center)` values and radii into fixed arrays using outward interval operations. This repairs the warmed scalar allocation contract without changing the arithmetic HOLD or any threshold. For `|c|<=1`, the helper evaluates directed Taylor intervals with an outward geometric tail; for larger negative `c`, it uses the standard interval recurrence `phi_k(c)=(phi_{k-1}(c)-1/(k-1)!)/c`; at `c=0` it uses exact factorial values. Interval overflow or a denominator containing zero remains a typed refusal. A nonzero-center comparison initially differed from the reference by `3.53e-14` under the recurrence-only route; the directed Taylor path fixed the witness, with the first failure and repair logs preserved.

## Fresh scalar allocation audit

The first warmed `h=0`, nonzero-input regression failed with exactly 30 allocations. The source was the scalar branch allocating temporary coefficient and radius vectors on every call. The branch now evaluates its one-node `phi_k(c)` values and outward radii into fixed arrays. The same warmed call passes with zero allocations. The allocator test counter is thread-local (like its tracking flag), removing the old global reset race; a two-thread isolation test passes.

## Zero-center factorial radius review

An in-module regression directly calls `scalar_phi_coefficients(0)` and requires nonzero radii for `phi_3(0)=1/6` and `phi_4(0)=1/24`, with the returned center/radius endpoints reaching beyond the adjacent binary64 floor/ceiling pair that encloses each exact rational. Against the old point intervals the regression failed at `phi_3 needs a nonzero radius` (`a04_zero_radius_first_failure.log`). Replacing those point intervals with `div_down(1,factorial)` / `div_up(1,factorial)` endpoints made it pass (`a04_zero_radius_repair.log`). The integration-level sum test was removed because other component bounds masked this coefficient-radius defect.

The low-level allocation documentation now limits the claim to a successful preallocated call with fixed dimensions, cap, cache state, and adequate unchanged report-string capacity; the convenience wrapper allocation caveat is explicit.
