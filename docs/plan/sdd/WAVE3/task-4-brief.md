## Task 4: P0-atol — per-component absolute tolerance on the fast routes (node id `p0-atol`)

Plan §3 P0 ("vector atol"); E1 deviation 2 (the corpus atol vectors had to be uniform).

**Files:**
- Modify: `crates/rodas5p-integrators/src/rodas5p_fast.rs`: the error-norm hunk of `Workspace::attempt` and
  new entry points. The existing `attempt(.., atol: f64, ..)` signature is unchanged.
- Modify: `crates/vigilode/src/{solve.rs, options.rs}`.
- Test: `crates/rodas5p-integrators/tests/fast_vector_atol_contracts.rs` (new),
  `crates/vigilode/tests/facade_contracts.rs`.

**Interfaces (new):**
- `Workspace` gains `pub(crate) atol_vector: Vec<f64>`. Empty means the scalar path, unchanged bit for bit.
  With entries, component i uses `atol_vector[i] + rtol·max(|y_i|, |y_new,i|)` through
  `scaled_error_component`. Every other use of `adaptive.atol` in the fast loop follows the vector when one
  is given.
- `pub fn integrate_rodas5p_fast_observed_with_atol(problem: &OdeProblem, t_span: (f64, f64), y0: &[f64], adaptive: &AdaptiveStepConfig, output: &OutputSchedule, atol: &[f64]) -> CoreResult<(Rodas5pFastResult, Option<DriverPartial>)>`.
- `pub fn integrate_rodas5p_fast_banded_observed_with_atol(problem: &OdeProblem, band: &BandedJacobian, t_span: (f64, f64), y0: &[f64], adaptive: &AdaptiveStepConfig, output: &OutputSchedule, atol: &[f64]) -> CoreResult<Rodas5pFastBandedResult>`.
- Validation: length n, finite, ≥ 0, otherwise `CoreError::InvalidInput`. `adaptive.atol` is still validated
  but is unused when a vector is given.
- Facade: `Atol::Vector` is accepted on the dense fast route (no declared band; direct call recorded in
  `RouteRecord::entry_point`) and on the banded fast route. It stays refused with `Unsupported` on the
  matrix-free route (the Krylov weights take a scalar), on the sequential route (mass or `Enforce`) and on
  dense + declared band (router). Task 8 extends it to fast dense output. Update the `Atol::Vector` docs.

- [ ] **Step 1: Write failing tests:**
  - (a) `uniform_vector_equals_scalar_bitwise`: Robertson dense and Brusselator banded at rtol 1e-6; endpoint
    bits and counters are equal.
  - (b) `robertson_vector_atol_accuracy`: atol = [1e-8, 1e-14, 1e-8] at rtol 1e-6. The endpoint error
    against the fast driver at rtol 1e-11, measured in the same vector-weighted WRMS, is ≤ 2 tolerance units.
  - (c) `invalid_vector_rejected` (wrong length, NaN, negative).
  - (d) `zero_component_follows_zero_scale_rule`: one identically-zero component with atol_i = 0 succeeds,
    and a component with atol_i = 0 and a nonzero error is rejected (+inf).
  - (e) facade: `vector_atol_accepted_on_fast_routes`, `vector_atol_refused_on_mf_sequential_and_declared_band`.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test fast_vector_atol_contracts`; expect a
  compile failure.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the new tests plus `rodas5p_fast_contracts`, `int03_banded_fast_contracts`,
  `a1_zero_tolerance_scale`, `rodas5p_fast_allocations`, `b1_lagged_contracts` and
  `cargo test -p vigilode --locked -j 1`; all pass.

