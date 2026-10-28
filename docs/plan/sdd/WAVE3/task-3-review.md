# Task 3 review: p0-fastdense (e9c5bbd..169cbf2)

Reviewer: task reviewer (spec + quality), single pass over the review package
`review-t3-e9c5bbd..169cbf2.diff` (3457 lines), with one focused out-of-diff check per named risk. Paths are
relative to `crates/`. Line numbers are those of the diff file (`diff:N`) or of the worktree at 169cbf2.

### Spec Compliance

- ✅ **Spec compliant**, read against the brief as amended by R-C1, R-C2, R-C3, B3c–B3f, FP2, FP3 and the
  controller's two notes (schema v3 covers the p0-events fields; events must run on the fast route). Deviations
  D1–D9 are either ordered by those rulings (D5 = B3c, D8 = R-C2, D9 = R-C1/R-C3 guards) or are acceptable
  judgement calls (D1, D2, D3, D4, D6, D7). Details per question are below.
- ⚠️ **Cannot verify from diff:**
  - The brief's E1 figure (8.7–18.5 MB per Brusselator run) is removed only if E1's Brusselator used a *user
    dense* provider. The FD dense Jacobian still allocates per build (`dense_fd_jacobian`, report open
    question 2). The brief scopes the switch to the "facade dense Jacobian provider", so this is compliant. The
    controller should still confirm which provider E1 measured.
  - The 4 lib dead-code warnings in the test output (`DenseOutputPreparationWork`, `EndpointAdmission`, …) are
    not in any hunk of this diff, and the report calls them pre-existing. I did not check them at e9c5bbd.

#### 1. Math

- **The W-filtered estimate is correct.**
  - `rodas5p_fast_dense.rs` `estimate` (diff:801–852) forms the midpoint and `u'(t_m) = (y_new − y + (d1+d2)/4)/h`.
    It differentiates `(1−θ)y0 + θ(y1 + (1−θ)P(θ))` at θ = 1/2 by hand and gets `(d1+d2)/4`.
  - The midpoint RHS goes into `work.stage_rhs` through `eval_rhs_into` (diff:824). That buffer is safe to use:
    it is overwritten at the start of every attempt (`rodas5p_fast.rs:1046`), and the loop never reads it after
    an attempt.
  - The code then forms `(h/2)(u' − f)` and calls `solve_into` (diff:831) on the step's own factors. I checked
    `factor` at `rodas5p_fast.rs:801–821`: it forms `W = I/(hγ) − J` at `trial_h` on every attempt. The division
    by `hγ` (diff:834–842) then gives `(I − hγJ)⁻¹(h/2)d`, the same quantity the sequential route gets from its
    `M − hγJ` LU (`dense_output_v2.rs:1189–1195`). There is no Jacobian and no factorization.
- **The norm is equivalent to the sequential one.**
  - It is computed inline with `scaled_error_component` on `atol_i + rtol·max(|y|,|y_new|)`, reading
    `work.atol_vector` when it is set (diff:836–845).
  - `step_error_wrms` (`rodas5p-core/src/norms.rs:119–139`) does the same per-component arithmetic, returns
    +∞ on a non-finite term, and would need an allocated scale vector. Inlining it is the right choice, and it is
    what the brief asks for.
  - The output agrees with the sequential driver: PR gives 0.3495 units in 15 steps on both routes, and the max
    estimate is 0.6315. I reproduced this.
- **The dense extension is the sequential interpolant.**
  - `polynomial_into` (diff:755–770) is `Rodas5pDenseSegment`'s cubic in the `u = Γk` coordinates with `H` in
    place of `D = HΓ`.
  - Test (c) checks it against `rodas5p_dense_output_parts` within 1e-12 at five θ values. D2's conditioning
    argument for that tolerance is sound.
- **Off is bitwise the fast driver.**
  - `after_attempt` returns early under Off (diff:868). `accepted` only reads `work`. The `NoSink` hooks are
    `#[inline(always)]` identity functions on a monomorphized generic.
  - Test (a) passes: endpoint bits, attempts/accepted/rejected, counters, `jacobian_reuses`, `lu`, and for the
    banded twin also `BandedWork` and `partial`. I reran it, and the numbers match the report table.

#### 2. D3 (an error inside the estimate rejects on the fast route; the sequential route accepts)

- **Verdict: the divergence is acceptable; do not unify toward the sequential behaviour. "Reject" is the
  safer contract and the documented one.**
  - The facade's `CallbackError::Recoverable` contract (`vigilode/src/error.rs:10–15`) is "rejects the current
    step attempt, and the step-size controller retries with a smaller step".
  - The fast route honours that for the midpoint call, exactly as it does for every stage call.
  - The sequential route (`dense_output_v2.rs:1954`, `Err(_) => unavailable += 1`) accepts the step with *no*
    interior control. Under `WFilteredEnforce` that silently lowers the guarantee for that step. The defect sits
    on the sequential side, not on the fast one.
- **Can this drive h → 0 and end in MinimumStep where the sequential route succeeds?** Only pathologically.
  - The midpoint state tends to `y_old` as h → 0, and `f(t, y_old)` has already succeeded as `f0` on this
    attempt. Any error set that is closed in (t, y), such as a domain boundary or a non-finite output, is
    therefore left behind at small enough h.
  - The back-solve can fail only through overflow or NaN of `(h/2)d`, and that term goes to 0 with h. Faer's
    "not factored" error cannot occur after a successful attempt.
  - The remaining case is an error set that is not closed and touches `(t, y_old)` while sparing every stage
    state. That is not a realistic user RHS.
- **Fatal errors end the same way on both routes.** The guard latch (`vigilode/src/fd.rs:49–57`) fails every
  later call. Both routes then stop at the minimum step with `CallbackFailed`. The only difference is one step of
  partial prefix: the sequential route keeps the step whose midpoint call failed, the fast route does not.
- **Recommendation:**
  - (i) Keep "reject" as the fast-route contract.
  - (ii) Record the divergence in the node/plan as a known sequential-route weakness, to be aligned in a later
    node. That would change protected sequential behaviour only on error paths.
  - (iii) Document and pin the rule (see Minor 1).

#### 3. D1 (the test-(e) gate moved to nonlinear PR)

- **Confirmed at the brief's settings.** At rtol 1e-6 the sequential driver also gives 15 accepted in 15
  attempts on linear PR under `WFilteredEnforce`. This is recorded at
  `rodas5p-integrators/tests/dense_wfiltered_contracts.rs:300–302` ("WFilteredEnforce: 15 accepted, 15
  attempts"). I reran the fast route: 0 interior rejections in 15 attempts on linear PR, and 2 in 27 at μ = 1e5.
- **The general claim "linear PR never rejects in the interior on either route" is false for the sequential
  route.**
  - `dense_wfiltered_contracts.rs:450–482` gates `interior_rejections > 0` on *linear* PR λ = −1e5. That test
    uses rtol 1e-8, atol 1e-10 and an 11-point output grid ("14 of 23 samples"). Output clipping perturbs the
    controller history, and the fast dense route has no grid.
  - So the premise is false only at the brief's rtol 1e-6, which is how the test's doc comment states it
    (diff:1698–1704).
  - Moving the gate to nonlinear PR is acceptable. The plan should record the premise narrowly, at rtol 1e-6
    without an output grid, not as "never".

#### 4. D7 (banded vs dense providers now agree to tolerance only)

- **Not a regression of a contract.** At e9c5bbd the test carried no doc claim of bit identity.
  - On the sequential route, a `Jacobian::Banded` provider is assembled into the same dense matrix
    (`band_as_dense`) and factored by the same direct LU. Bit identity followed from that construction; it was
    incidental.
  - On the fast route the banded LU (band partial pivoting) and the dense in-place LU round differently, so
    tolerance agreement is the right assertion.
- **Coverage gap:** the sequential-route bit identity, which still holds by construction, is no longer tested
  anywhere (Minor 2).

#### 5. Allocations (test (g))

- **Formula:** the budget `fast + 3·⌈log₂(accepted+1)⌉` comes verbatim from the brief (Step 1 (g), line 72–73),
  and preflight Table B confirms it for T3.
- **Counting:** it is thread-local through a `Cell` in a `thread_local!`, counting `alloc`, `alloc_zeroed` and
  `realloc`, the same pattern as `rodas5p_fast_allocations.rs` (diff:1136–1171). The coefficient cache is warmed
  first, and the closures exclude the printing.
- **Not flaky:** the counts are deterministic. My rerun matches the report exactly:

  | rtol | Off | WFilteredEnforce | budget |
  | --- | --- | --- | --- |
  | 1e-6 | 61 | 62 | 64 |
  | 1e-9 | 67 | 69 | 70 |

- **The margin is structurally tight, though (Minor 3).**
  - About 4–5 setup allocations are charged against the log term: three store vectors, the span schedule, and
    the scratch under WFilteredEnforce.
  - Extrapolated, the store's doubling from 2 steps (and `t` from 16) leaves about one allocation of slack at
    roughly 1e5 accepted steps.

#### 6. Facade

- **`Plan::FastDense { band }`** (`solve.rs` diff:2490, 2539–2547, 2569–2625).
  - It re-plans `Routed{None}` → `FastDense{None}` and `Banded` → `FastDense{Some}` when the route is dense
    output and not sequential.
  - The rule is `sequential_route = mass ∥ (dense_output ∧ Enforce)` (diff:2756–2758).
  - The `entry_point` is recorded for all four combinations of dense/banded and scalar/vector atol.
  - The `unreachable!` on `Enforce` is guarded by `sequential_route`.
- **Refusal matrix:**
  - JVP-only with dense output or a mass matrix: refused, through `dense_or_mass` (diff:2708–2716).
  - Dense provider + declared band + dense output: refused under every interior mode (diff:2718, B3f).
  - Vector atol: refused only on the sequential route (diff:2727–2733, R-C2).
  - All of these are tested in `enforce_and_mass_stay_sequential` (diff:3382–3422) and in the D8 edit of
    `facade_contracts.rs` (diff:2909–2936).
- **`Extension`:**
  - The `enum Extension { Sequential, Fast }` is private (diff:2228–2281).
  - `truncate_after` works on both stores. The fast store truncates on the step starts, `times()[..len()]`.
  - p0-events' bound "`t > t.last()` ⇒ None" is kept in `at()` (diff:2309–2311).
  - The unit test `truncate_after_on_both_extension_stores` (diff:2360–2391) covers both stores.
- **Events:** `events.rs` is untouched and consumes the extension only through `Solution::at` and
  `truncate_after` (`events.rs:83, 260, 275, 298`). `events_do_not_change_integration` now asserts the
  `rodas5p-fast-dense-v1` route (diff:2855–2858), and the report lists the 15 events tests as passing.
- **Schema v3:** the doc note at `solution.rs` (diff:2139–2147) covers both the fast-dense route and p0-events'
  `terminal_event`/`event_evaluations`, as the controller ruled.
- **AtolSpec:**
  - Vector atol is accepted on fast dense output through `..._dense_observed_with_atol`. The estimate's scale
    reads the vector (diff:837–841).
  - `uniform_atol_vector_is_the_scalar_dense_run` checks dense and banded bitwise and checks the refusal of a
    wrong length.
- **`with_jacobian_into`:** it is wired up for the user dense provider. It zeroes the buffer before calling the
  user (B3d, diff:2795–2804, 2826–2829).
- **Docs (B3e):** the `options.rs` and `lib.rs` routing table and refusal list are updated.

#### 7. Merge correctness

- **Both merges have two parents:**
  - `4702b4f`: parents `90b8826` and `997fefe` (p0-atol).
  - `7b1eb8b`: parents `4702b4f` and `e9c5bbd` (p0-events and b1c).
- **No history was rewritten:** `e9c5bbd`, `997fefe` and `f65d968` are all ancestors of `169cbf2`.
- **No sibling code was regressed:**
  - `events.rs` and `fast_vector_atol_contracts.rs` are unchanged against e9c5bbd.
  - The only edits to p0-atol and p0-events tests are the R-C2-ordered change to the vector-atol refusal (D8)
    and the added route assertion in the events test.
  - The report lists `fast_vector_atol_contracts` 4/4, `events_contracts` 15/15 and `facade_contracts` 20/20 as
    passing at 169cbf2. I did not rerun the facade suite.

### Strengths

- **The sink design is clean.** A generic `StepSink` with an `#[inline(always)]` `NoSink` keeps every existing
  entry point bit-identical without touching the `pub(crate)` surface that lagged uses (`rodas5p_fast.rs`
  hunks). The hook sits exactly where the brief puts it, on attempts with embedded error ≤ 1.
- **The extension is computed in the fast coordinates.** Applying `H` to `u = Γk` avoids storing or rebuilding
  the `k` rows. Test (c) documents and checks the transform, and D2's conditioning argument is correct.
- **FP2 and FP3 are resolved properly.**
  - `solve_into(&mut self)` charges band operations to `BandedWork` and nothing to `WorkCounters`. The caller
    charges `linear_solves` and `direct_solve_calls` explicitly.
  - The midpoint and defect use a preallocated `2n` scratch buffer.
- **The tests are behavioural and broad.** They cover bitwise Off on three problems and three tolerances, the
  bitwise prefix after a fatal latch, a closed counter ledger (`samples = accepted + interior_rejections`), typed
  errors outside the span, the empty span, and the scalar-vs-uniform-vector seam.
- **The report is accurate.** Every number I reran matched it exactly.

### Issues

#### Critical (Must Fix)

None.

#### Important (Should Fix)

None.

#### Minor (Nice to Have)

1. **The D3 rule is neither documented nor pinned by a test.** `rodas5p_fast_dense.rs` `after_attempt` and
   `estimate` (diff:824, 831, 871) propagate the estimate's errors with `?`.
   - **Documentation:** the doc of `FastInteriorWork::samples` (diff:569–571) implies the rule, but neither the
     module doc nor `InteriorError::WFilteredEnforce` in `options.rs` states that an error in the midpoint RHS
     or the back-solve rejects the attempt like a stage error (while the sequential route counts it
     `unavailable` and accepts).
   - **Test:** no test exercises the path. Test (f)'s latch fires in a stage call.
   - **Fix:** add one sentence to both docs. Add a driver-level test with an RHS that fails only on its k-th call
     (the k-th call being a midpoint call). It should assert a `NonFinite` rejection and then an accepted retry at
     a smaller h.
2. **The sequential-route provider bit identity lost its only test** (`p0_contracts.rs` diff:3330–3348, D7).
   - **Fix:** keep the fast-route tolerance check, and add an `InteriorError::Enforce` variant of the same pair
     asserting bitwise `t`, `y` and accepted steps. The wave-2 Minor 7 property still holds there by
     construction, and it would then stay guarded.
3. **The allocation budget absorbs the setup allocations into the log term.**
   - Where: `fast_dense_allocations.rs` diff:1222; `FastDenseSegments::new` diff:603–629; `DenseSink::new`
     diff:783–797.
   - The 3–4 setup allocations (store vectors, span schedule, scratch) are charged against
     `3·⌈log₂(m+1)⌉`, which the brief meant for growth only. The margin is 1–3 now and about 1 at roughly 1e5
     steps.
   - The capacities are also inconsistent: `t` starts at capacity 16, while `states` and `stages` start at
     2 steps.
   - The test is deterministic, not flaky. Either reserve the three vectors with one shared growth policy (for
     example a 16-step start for all three), or state the setup constant in the test's doc.
4. **"Zero allocation per step" holds only on the in-place LUs.**
   - On the Faer LU (n above the small-LU threshold), `solve_into` allocates once per sample
     (`rodas5p_fast.rs` diff:143–150; its doc says so).
   - This is no regression relative to the fast driver, whose Faer stage solves already allocate.
   - Mention it in the `rodas5p_fast_dense.rs` module doc, which says "no allocation per step" (diff:530).
     Test (g) only covers Robertson on the in-place path.
5. **The θ = 1/2 polynomial is written out a second time** in `estimate` (diff:818), rather than calling the
   per-component form of `polynomial_into`. It is bitwise the same expression today, but the two copies can drift
   apart. Factor out a per-component `poly_at(c, θ)` used by both.
6. **An estimate error forces a redundant Jacobian rebuild.** After an `Err` from the estimate, the shared match
   in `integrate_fast` sets `fresh_state = false` (`rodas5p_fast.rs` diff:474–477). The retry from the same
   state therefore rebuilds `J`, `f0` and `f_t`, although they are still valid: the stage attempt succeeded.
   This is harmless, rare and bit-stable, but worth a comment or a `work.lu_chosen`-preserving branch if the
   sink grows.

### Assessment

**Task quality:** Approved

**Reasoning:** The W-filtered estimate, the u-coordinate extension and the facade routing are correct. They
match the sequential driver's numbers exactly where they should, and the Off path is verifiably bit-identical to
the fast driver. Each deviation is ruled or defensible, including D3, whose "reject" semantics follow the
documented `Recoverable` contract. The remaining items are documentation, coverage and DRY polish.

**Checks run:**
- `cargo test -p rodas5p-integrators --test fast_dense_allocations --test fast_dense_contracts -- --nocapture`
  at 169cbf2: 1 + 7 passed. The numbers are identical to the report.
- `git log --format='%h %p'` and `merge-base --is-ancestor` for the merge topology.
- Focused reads of:
  - `rodas5p_fast.rs:801–821` (`W` form) and `:1010–1114` (`stage_rhs` lifetime);
  - `dense_output_v2.rs:1185–1290` and `:1895–1990` (sequential estimate and the `unavailable` path);
  - `norms.rs:119–139`;
  - `vigilode/src/error.rs:5–30` and `fd.rs:43–73` (the callback contract and the latch);
  - `dense_wfiltered_contracts.rs:300–302` and `:445–482` (D1).

Finding counts: Critical 0, Important 0, Minor 6.
