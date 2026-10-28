# Task 7 review: p0-api (6594d10..4945683)

### Spec Compliance

- ✅ Spec compliant (rulings applied: Options and Atol not non_exhaustive; vector-atol Stats docs-only; unit variants accepted).

Checks run:
- Refusal sites: `grep unsupported(` in solve.rs gives exactly 7 calls (677, 680, 686, 689, 692, 696, 725), each with a distinct variant. The 7 variants map 1:1 to the 7 sites. No `Unsupported(String)` or `.into()` construction remains in src, examples or bench.
- Display: each `#[error]` string is byte-identical to the removed literal (checked against the diff). `display_text_is_the_former_message` (p0_api_contracts.rs) asserts hand-typed copies of all 7 texts plus the `unsupported: ` prefix. It is a real check.
- Golden: RED commit d816b9f touches only lib.rs docs (the doctest), the golden JSON and the test file; no arithmetic. I built the untouched base 6594d10 (git archive into scratchpad) and ran `stats_json_unchanged` against the committed golden there: it passes. The golden therefore equals the base output, and still passes at HEAD. The golden holds no timing or machine-dependent fields (only counters and configuration). Commit order is correct (RED before GREEN).
- non_exhaustive list: 20 attributes in src. They match the brief's list exactly (Tier, InteriorError, Jacobian, FailureReason, Status, SolveError, CallbackError, Callback, JacobianRecord, DriverWork, Direction, Unsupported, Stats, Solution, RouteRecord, StopRecord, CallbackWork, FiniteDifferenceWork, KrylovStartRecord, EventRecord). Options and Atol are untouched.
- compile_fail doctest: all 7 `FailureReason` variants named in the match exist (solution.rs:51-91), and the paths are valid. I re-ran with `compile_fail,E0004` in a scratch copy of HEAD and it passes. It therefore fails for the right reason, the non-exhaustive match. RED evidence ("compiled successfully, but marked compile_fail") is credible.
- Robertson example: observed relative errors from my run are 3.6e-7 (y0), 1.1e-6 (y1) and 9.1e-7 (y2) against a 1e-4 bound, about 90x margin. The y1 reference has 7 digits (rounding alone up to about 5e-7 relative), still well inside. The check is per component and relative. The old check (1e-5 absolute + 1e-3 relative) allowed 9.2e-6 for y1, so it could not detect a y1 error. Toolchain flakiness risk is low: libm ulp differences move the adaptive path by far less than the 90x margin.
- Bench: `cargo check --manifest-path crates/vigilode-bench/Cargo.toml --locked -j 2 --all-targets` finishes cleanly (warnings only in pre-existing rodas5p-integrators). The bench has no match on these facade enums (R-C5/FP4), so this is a trivial pass until the T1 arms land, as the ruling expects.
- `cargo test -p vigilode --test p0_api_contracts`: 9 passed.
- The worktree was left unmodified; scratch copies are in the scratchpad only.

### Strengths
- Mechanical, arithmetic-free change. Existing refusal assertions now pin the specific variant (facade_contracts.rs, p0_contracts.rs, events_contracts.rs), which tightens coverage beyond the brief.
- The helper closures in the old tests were extended with an `expected` parameter, so no case became weaker.
- The golden was verified from the base tip, and the stability section names the exhaustive-struct exception for Options and Atol.

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
None.

#### Minor (Nice to Have)
1. lib.rs:127-129 (Stability section): "Every refusal is a typed `Unsupported` variant" is overstated. `InvalidInput(String)`, `StructureRefused(String)` and `Driver(String)` are still string-typed refusals (error.rs:92-107). Reword to "Every unsupported-feature refusal".
2. lib.rs:131-132: there is a stray empty `//!` line before the doctest paragraph (a double blank line in the rendered docs). Cosmetic.
3. lib.rs:136: use `compile_fail,E0004`. A bare `compile_fail` passes on any compile error, such as a later typo in the snippet; the code pins it to the intended cause. I confirmed E0004 is what fires.
4. error.rs:62-ff (`Unsupported` unit variants): adding a payload later changes the variant shape and is a breaking change, since a unit variant cannot gain fields without a break. Accepted by ruling; noted only so the Stability text states that "a future payload becomes a new variant" (the section currently does not say it).
5. error.rs:93-97: the pre-existing `SolveError::Unsupported` doc still says "for example a vector `atol`", although vector atol is supported on the fast routes now. Out of scope, but the doc is stale next to the new typed variants.
6. The vector-atol docs-only choice (solution.rs `Stats.adaptive`) is correct: `AtolSpec::max` (solve.rs:73) folds to the largest entry, and `options.atol` records the per-component vector. Nothing to fix.

### Assessment

**Task quality:** Approved

**Reasoning:** All 7 refusal sites are typed 1:1, Display is unchanged and tested against the former text, and the golden is verified against the untouched base. The non_exhaustive list matches the brief, and the doctest, Robertson example and bench all behave as claimed. Only documentation Minors remain.
