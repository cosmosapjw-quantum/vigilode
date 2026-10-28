# Task 4 review: p0-atol (75a0c6c..997fefe)

### Spec Compliance

- ✅ Spec compliant (brief as amended by R-C1, R-C2, R-C4).
- ⚠️ Cannot verify from diff: R-C4 ordering (T5 branching from the tip that holds T4) is a controller matter. The implementer's full-crate and workspace runs were not done (handoff list); the targeted runs below are the only independent evidence.

Checks run (targeted, worktree target dir): `--test fast_vector_atol_contracts` 4/4 pass; `--test rodas5p_fast_allocations` 2/2 pass. The 4 lib warnings are pre-existing dead-code ones, none in the touched lines.

### Strengths
- Scalar path: rodas5p_fast.rs:1035-1040 is the old loop verbatim behind `atol_vector.is_empty()`. The default is `Vec::new()` (:771), so there is no allocation and no arithmetic change. The vector branch (:1041-1045) uses the same expression with `atol_i`.
- `adaptive.atol` is read once in the loop (:1803, passed to `attempt`), and the only other reads are the public single-step (:1399, scalar by design). Nothing else needed redirecting (grep confirmed).
- R-C1 shape honoured: private last parameter `atol_vector`, stored in `Workspace`, empty = scalar, all 8 call sites updated, `attempt` signature unchanged, lagged `Workspace::new` untouched.
- Public entries reject an empty slice (:1694-1700), so the private "scalar" marker cannot be reached by accident from the public API. Test (c) covers `vec![]`.
- Refusal matrix (solve.rs:1113-1134) matches R-C2: any `dense_output` or mass, JVP-only, dense Jacobian + declared band all give `Unsupported` with "vector atol" in the message. Length/NaN/negative give `InvalidInput`, and those checks run before the refusals, so bad input is reported as such. Accepted: undeclared dense (analytic or FD) via new `Plan::DenseFast`, and banded (user band, FD band). Only one `Plan` variant added; the restructure is left to T3 as ruled.
- Entry-point recording is exact and tested against a direct call, bit for bit (facade_contracts.rs vector_atol_accepted_on_fast_routes).

### Specific questions

- Scalar path bit-identical: yes (see Strengths). No double rounding or reordering is introduced.
- Uniform-vector test: it genuinely compares bits and counters. Dense compares endpoint bits, `observed.counters`, attempts, accepted, rejected, jacobian_reuses. Banded adds `work` equality (fast_vector_atol_contracts.rs:526-532, :559-566). Weakness: only the endpoint row `y[1]` is bit-compared, and a test passing for a uniform vector cannot detect a driver that ignores the vector (the implementer notes this; (b) and (d) carry that load, and they fail under the reported mutant).
- `Stats.adaptive.atol` = max entry: acceptable, not a defect. It is documented (options.rs Atol::Vector, solve.rs `adaptive_atol`), it makes a uniform vector reproduce the scalar record exactly (asserted at facade_contracts `sol.stats.adaptive == adaptive`), and the true vector is in `Stats.options.atol`. Residual risk: a consumer who replays from `Stats.adaptive` alone gets a different run. See Minor 1.
- No-RED deviation: the mutation check (`if true` forcing the scalar branch) kills (b) and (d); (a) and (c) correctly pass under it. That substitutes adequately for a compile-fail RED on the behaviour that matters. It does not cover component-order bugs, see Minor 2.
- No hidden allocation: the only new allocation is `atol_vector.to_vec()` once at setup (:1762), outside the step loop, and zero-size for scalar callers. The vector branch is allocation-free (iterator zip). `rodas5p_fast_allocations.rs` exists and passes, but it never exercises a nonempty `atol_vector` (see Minor 3).

### Issues

#### Critical (Must Fix)
None.

#### Important (Should Fix)
None.

#### Minor (Nice to Have)
1. solve.rs:885-890, options.rs docs: `Stats.adaptive.atol` for a vector is a derived value that no driver read. The doc says so; T7 may prefer an explicit field or a `None`. Leave as is for T4, but flag it to T7 (the implementer also raised this).
2. fast_vector_atol_contracts.rs:581 uses the palindromic vector `[1e-8, 1e-14, 1e-8]` (brief-specified), so a reversed or mis-indexed zip would pass (b). The facade nonuniform banded check compares against the same driver, so it cannot catch an order bug either. Add one asymmetric vector case, for example `[1e-8, 1e-14, 1e-11]` against a hand-computed scale, or a unit test of `attempt` with a two-component error.
3. rodas5p_fast_allocations.rs: no case sets `work.atol_vector` nonempty. The vector branch is allocation-free by inspection (zip over slices), but the guard test does not prove it.
4. fast_vector_atol_contracts.rs:615: `run.accepted_steps > loose.accepted_steps` is a heuristic added to make (b) bite; it is sound for Robertson today but ties the test to controller behaviour. Acceptable; note only.
5. Process: no compile-failure RED (implementation preceded the tests). Covered by the mutation check; not a code defect.

### Assessment

**Task quality:** Approved

**Reasoning:** The scalar path is provably unchanged, the vector branch is a minimal, correctly indexed mirror of it, refusals and routing match R-C2, and the tests exercise real behaviour with a mutation check backing the missing RED. Only minor test-strength gaps remain.
