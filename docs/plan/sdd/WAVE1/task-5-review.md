# Task 5 review: A1-api (`vigilode` facade), 9b5d083..f76d60f

Line references are to the review diff `/home/claude/wt/a1-api/.superpowers/sdd/IMPLEMENTATION_WAVE1/review-9b5d083..f76d60f.diff` (D:n) unless a source path is given. I read the diff in three passes and did not re-read any changed file. Tests were not re-run, because no doubt came up that the recorded runs leave open.

### Spec Compliance

- ❌ Issues found:
  1. **The stats do not carry every work counter or run identifier.** `Stats` (D:1043-1093) records `observed.counters` plus attempts, accepted, rejected, internal and clipped steps. It drops the following fields of the driver result:
     - `Rodas5pFastResult::jacobian_reuses` and `lu` (`crates/rodas5p-integrators/src/rodas5p_fast.rs:416-426`).
     - `Rodas5pFastBandedResult::work` (factor_operations, solve_operations, stored_slots; `rodas5p_fast.rs:181-193`).
     - `Rodas5pMfFastResult::state_reuses` and the run-policy ids `gcrodr_policy`, `krylov_workspace`, `start_policy` and `recycle_scope` (`rodas5p_matrix_free_fast.rs:1189-1208`).

     These fields are collapsed into `Run` at D:1248-1261 and D:1280-1291. The brief says "stats serializes every option and every work counter".
  2. **The stiff-example comparator is not the example's driver, and the report does not say so.** `examples/solve_stiff.rs:57` calls `integrate_sequential_matrix_free_adaptive_dense_observed`, a sequential dense-output path with a six-point grid. The test (D:1871-1904) instead compares the facade with `integrate_rodas5p_mf_fast_observed_gmres_into` on the grid `[0, 1]`. It reuses the example's problem, tolerances, h0, span and max_attempts (10_000, which matches `DEFAULT_MAX_ATTEMPTS`). The comparison is a valid "facade = the direct driver it routes to" check. It is not "the example solved through the facade equals the example's driver call", and deviation list items 1-10 (D:2573-2597) do not mention the substitution. The facade cannot reach the sequential path under the brief's routing rule, so the fix is to record this as a deviation, or for the controller to rule on it.
- ⚠️ Cannot verify from the diff alone:
  - **Thin partial solution.** On a failure the partial solution holds `t0` only (D:2178). The drivers expose no last accepted state (`Rodas5pFastResult` has none), so the contract "a failed run returns its partial solution" is met only in form. The work and the typed reason are kept. Wave 2 or the controller needs to decide whether a new opt-in driver field is acceptable.
  - **JVP or Jacobian callback fatal inside GMRES** may return `Err` (`SolveError::Driver`, D:1355-1363) and lose the partial result. This path is untested (report concern 7).

Named-risk checks:
- **Risk 1: `.mass()` and `dense_output` refusals.** I checked the fast, banded and MF drivers and the sequential dense path.
  - `rodas5p_fast.rs:1112,1325`, `rodas5p_matrix_free_fast.rs:1354` and `routing.rs:429` all refuse a non-identity mass. None of these takes an `OutputSamplingPlan`.
  - `dense_output_v2.rs:1423` (`integrate_adaptive_dense_observed_with_config`, Sequential/SABR) does support a mass matrix (`mass_or_identity`, `dense_output_v2.rs:1179`). It also samples the continuous extension without clipping steps.
  - That driver is not in the brief's allowed route set, and plan §1 line 9 names only fast, banded and small-N as protected. **Verdict:** both refusals are consistent with the brief, so this is not a spec gap. Plan §3 P0 (line 102) still lists mass matrix and dense output as P0 features, so wave 2 must either admit the sequential path or extend the fast driver. The stated rationale (D:1430) is accurate for the allowed drivers only.
- **Risk 2: FD increment and CPR grouping.** I read `fd.rs` (D:302-383).
  - The increment is √ε·max(|y_j|,1), taken as the represented difference.
  - Groups are `j mod (l+u+1)`. Columns in a group are `width` apart, so their row supports `[j-u, j+l]` (length `width`) are disjoint. That is correct for a band.
  - Band index `i*width + (j+lower-i)` matches `dense_from_band` (D:392) and the `BandJacobianFn` layout (D:670-672).
  - The unit test (D:414-436) checks the grouped band bit for bit against the dense difference. Call counts are `n+1` and `groups+1`. Correct.
- **Risk 3: Cargo.lock.** The diff (D:16-30) adds one `[[package]] vigilode` entry. Its five dependencies are all workspace or existing crates. Nothing else changed. Pass.
- **Risk 4: bit identity.** I checked the comparators and their settings.
  - Robertson (D:1800-1840): the facade (router, rule 3) is compared with direct `integrate_rodas5p_fast_observed(robertson_problem())`, using `adaptive_for(opts)` and the same `[t0, t1]` schedule. `stats.adaptive == adaptive` is asserted, plus counters, attempts, accepted and rejected steps.
  - Banded (D:2021-2084): the facade is compared with direct `integrate_rodas5p_fast_banded_observed`, with the same band fill, autonomous problem, adaptive config and schedule.
  - MF (D:1871-1904): the comparator uses `RoutingOptions::default().matrix_free`, the same configuration the router uses (`routing.rs:436-443`).

  All three are genuine direct-driver comparisons with identical settings. The only caveat is the comparator choice in spec item 2.

### Strengths

- **The callback guard (D:208-300)** maps Recoverable, Fatal and post-fatal calls to `CoreError::NonFinite`. The fast drivers therefore reject the step instead of aborting, which addresses plan risk 4 (plan line 300) without touching any driver. The guard does no arithmetic, so it does not affect bit identity.
- **Work accounting is careful.** FD calls are counted on attempt (D:290), and driver and merged counters are kept apart (D:1295-1299). Short-circuited RHS calls are counted separately, and the test checks the user-entry identity exactly (D:2185-2188).
- **Refusals never fall back silently.**
  - Vector atol, mass, dense_output and band-on-JVP are refused with `Unsupported` (D:1401-1462).
  - Malformed input gets `InvalidInput` before any step, with broad negative tests (D:2293-2365).
- **A failed run is `Ok(Solution{status: Failed(reason)})`,** and `ensure_success` converts it (D:1126-1134). `Status::Success` is documented as an estimate, not a certificate (D:949-950), in line with the global constraint on type-distinct statuses.
- **Stats round-trip and replay.** The options round-trip through serde, and the replayed run is bit-identical (D:2448-2459).
- **The crate doctest** (D:487-515) covers a real Robertson solve.

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
1. **Work counters and run-policy ids are missing from the stats** (D:1188-1195, D:1248-1291, D:1043-1093). The brief requires every work counter. Without `jacobian_reuses`, `state_reuses`, `BandedWork` and the MF policy ids, the record is incomplete for exactly the Jacobian and LU reuse questions the plan's B1 gate asks about.
   - **Fix:** add a serialized `driver_work` enum to `Stats`, filled from `RoutedRun::{Dense,Banded,MatrixFree}` and the direct banded result. Assert its presence in `stats_serialize_every_option_and_counter`.
2. **The stiff-example test compares against the MF fast driver, not the example's sequential dense-output driver, and the deviation is not recorded** (D:1871-1904 vs `examples/solve_stiff.rs:57`). The brief's wording reads as the example's own call.
   - **Fix:** add a deviation entry to `docs/plan/nodes/a1-api.md` and the report, or get a controller ruling. The test itself can stay.

#### Minor (Nice to Have)
1. D:1313-1315: every unsuccessful run that has no fatal error and has `attempts < max_attempts` is labelled `StepSizeTooSmall`. Any other non-success cause the driver reports is silently relabelled. Consider checking `observed.message`, or adding a generic `DriverStopped { message }` variant.
2. D:1043: `Stats` is `Serialize` only. Reproducibility goes through `Options` plus problem flags, but the record cannot be deserialized whole. This is acceptable for now; deriving `Deserialize` would make the replay claim literal.
3. D:672-673: `BandJacobianFn` says "the buffer is zeroed before every call". The facade does not zero it (D:1564-1569) and relies on the driver doing so. Either zero it in the wrapper or document the driver as the source of the guarantee.
4. D:1478-1483 and D:2370-2392: there is no test of a Fatal or Recoverable error from the time derivative, Jacobian or JVP callbacks. Only RHS errors are covered.
5. Report: the clippy pre-existing warning count differs between runs ("4" at D:2535 vs "1 + 7" at D:2609). The difference is harmless, but it is unexplained.

### Assessment

**Task quality:** Needs fixes

**Reasoning:** The facade is well built. Routing, the FD and CPR Jacobians, the guard-based failure semantics and the bit-identity tests are correct and honestly scoped. The stats, however, omit the drivers' own reuse and band work counters and their policy ids, which the brief explicitly requires. The stiff-example comparator substitution also needs to be recorded as a deviation.
