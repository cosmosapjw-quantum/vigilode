VERDICT: ALL ADDRESSED

Re-review of fix round 1 (169cbf2..35d6604), p0-fastdense. Paths relative to `crates/`.

### Finding Verdicts

- **M1 (D3 rule undocumented and untested)** — ADDRESSED.
  - Docs: `rodas5p-integrators/src/rodas5p_fast_dense.rs` module doc (lines 8-20 of the fix hunk) and `vigilode/src/options.rs` `InteriorError::WFilteredEnforce` both state that an estimate error rejects the attempt like a stage error, while the sequential route counts it unavailable and accepts.
  - Test: `rodas5p-integrators/tests/fast_dense_contracts.rs` `midpoint_rhs_error_rejects_then_retries` (diff:284-396). It fails call 27, and the clean call log first confirms that call is the 3rd step's midpoint. It asserts `nonfinite_step_failures == 1`, a prefix bitwise equal to the clean run, a smaller retry h, an accepted retry, and a closed sample ledger. I ran it and it passes.
- **M2 (sequential-route provider bit identity untested)** — ADDRESSED.
  - `vigilode/tests/p0_contracts.rs:1202-1228`, inside `banded_provider_on_the_dense_output_route_matches_the_dense_provider`, is a genuine bitwise check under `InteriorError::Enforce`. It asserts `RODAS5P_SEQUENTIAL_DENSE_DRIVER_ID` for both providers (1216-1217), then bitwise-equal `t` and `y` and equal accepted steps (1218-1222).
  - The p0_contracts count stays at 21 because the pair was added to the existing test. The diff has no deleted test, and the fast-route tolerance assertions (1200-1209) are kept. I counted 21 `#[test]` attributes at HEAD, and the test passes.
- **M3 (allocation budget docs)** — ADDRESSED. The docs and comments now state the setup constant and the 16 vs 2-step capacities: `tests/fast_dense_allocations.rs:1-12` and the `DenseSink::new` comment in `rodas5p_fast_dense.rs`. No code changed, as ruled.
- **M4 (zero-allocation claim)** — ADDRESSED. The `rodas5p_fast_dense.rs` module doc "Allocations" paragraph limits the claim to the in-place LUs and says the faer LU allocates once per sample. It also drops the old "no allocation per step" line.
- **M5 (θ = 1/2 polynomial duplicated)** — ADDRESSED, arithmetic bit-identical.
  - `polynomial_component` is now the single form, used by `polynomial_into` and by the estimate's midpoint at θ = 0.5 (`rodas5p_fast_dense.rs`, diff:147-153, 204-205).
  - Old `polynomial_into`: `c*y0 + θ*(y1 + c*(d0 + θ*(d1 + θ*d2)))`. New: the same expression and operation order.
  - Old midpoint: `0.5*y + 0.5*(y1 + 0.5*(d0 + 0.5*(d1 + 0.5*d2)))`. New: the same with `complement = 1.0 - 0.5 = 0.5` exactly.
  - The defect expression is unchanged, with `d[1]` and `d[2]` replacing the destructured `d1` and `d2`.
- **M6 (redundant Jacobian rebuild after an estimate error)** — ADDRESSED, non-error path unchanged.
  - `rodas5p_fast.rs` (diff:28-70): `sink_failed` is set only by `inspect_err` on an `Err` from `sink.after_attempt`. `fresh_state = work.lu_chosen` still runs first, unchanged. `fresh_state = false` is skipped only inside the `Err` arm when `sink_failed` is true.
  - `Ok` paths and attempt errors are untouched. `NoSink` returns its input and never errs, so the Off path is bit-identical. The `Ok(error) if error <= 1.0` guard and the `other => other` arm are unchanged.
  - The new test checks the effect: the retry's first RHS call is a stage call, and `jacobian_reuses` exceeds the clean run's.

### New Breakage in the Fix Diff

None.

### Out-of-Scope Observations

None.

### Checks Run

- `fast_dense_contracts midpoint_rhs`: 1 passed.
- `p0_contracts banded_provider_on`: 1 passed.
- The report's wider claims (the Off bitwise test (a), fast and allocation suites, events) were not rerun. The diff gives no reason to doubt them: M5 and M6 are provably arithmetic-neutral, and the other changes are docs or tests.
- The sequential-route change for D3 was parked by controller ruling and was not judged.

### Verdict

**Fix round:** All findings addressed, no new Critical/Important breakage.
