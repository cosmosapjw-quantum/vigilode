## Task 3: P0-fastdense — continuous extension and W-filtered interior control on the fast drivers (node id `p0-fastdense`)

Plan §3 P0 (interior error, zero allocation per step), §5 T3, §9 (`fast` becomes the default dense path).

**Files:**
- Create: `crates/rodas5p-integrators/src/rodas5p_fast_dense.rs`, plus a `lib.rs` export.
- Modify: `rodas5p_fast.rs`: a per-step sink hook, additive only. No signature change of any `pub(crate)`
  item that `rodas5p_lagged.rs` uses (`Workspace::new`, `factor`, `solve`, `solve_dense_into`,
  `after_loop`, `failure_kind`). Add a banded-capable `pub(crate) fn solve_into(&self, b: &mut [f64])`.
- Modify: `dense_output_v2.rs`: make `rodas5p_dense_output_parts` `pub(crate)` (visibility only).
- Modify: `crates/vigilode/src/{solve.rs, solution.rs, lib.rs}`.
- Test: `crates/rodas5p-integrators/tests/fast_dense_contracts.rs`,
  `crates/rodas5p-integrators/tests/fast_dense_allocations.rs` (counting allocator, following
  `rodas5p_fast_allocations.rs`), and `crates/vigilode/tests/p0_contracts.rs` (updated).

**Interfaces (new):**
- `pub enum FastInteriorControl { Off, WFilteredEnforce }`.
- `pub struct FastDenseSegments` (flat storage: `t: Vec<f64>`, states `(m+1)·n`, stage rows `m·8·n`) with
  `len()`, `t_old(k)`, `t_new(k)` and
  `pub fn evaluate_into(&self, t: f64, out: &mut [f64]) -> CoreResult<()>`. `evaluate_into` returns the
  stored state bitwise at a step endpoint and otherwise computes θ exactly as `Rodas5pDenseSegment::evaluate`
  does.
- `pub struct FastInteriorWork { pub samples: u64, pub interior_rejections: u64, pub rhs_calls: u64, pub back_solves: u64, pub max_estimate: f64 }`.
- `pub struct Rodas5pFastDenseResult { pub fast: Rodas5pFastResult, pub segments: FastDenseSegments, pub interior: FastInteriorWork, pub partial: Option<DriverPartial> }`.
- `pub fn integrate_rodas5p_fast_dense_observed(problem: &OdeProblem, t_span: (f64, f64), y0: &[f64], adaptive: &AdaptiveStepConfig, interior: FastInteriorControl) -> CoreResult<Rodas5pFastDenseResult>`,
  and `integrate_rodas5p_fast_banded_dense_observed(problem, band: &BandedJacobian, t_span, y0, adaptive, interior)`
  with the same return type.
- `pub const RODAS5P_FAST_DENSE_DRIVER_ID: &str = "rodas5p-fast-dense-v1"`,
  `RODAS5P_FAST_BANDED_DENSE_DRIVER_ID = "rodas5p-fast-banded-dense-v1"`.
- Facade: `DriverWork::FastDense { banded: bool, interior: FastInteriorWork, segments: usize }`;
  `STATS_SCHEMA = "vigilode.stats.v3"`.

**Exact rules:**
- The output is `[t0, t1]` only (no grid clipping).
- On every attempt whose embedded error is ≤ 1 (the `InteriorSampling::EveryStep` rule), compute
  E = (I − hγJ₀)⁻¹(h/2)d(1/2) = W⁻¹((h/2)d)/(hγ), using the workspace's own LU of W = I/(hγ) − J. The cost
  is one RHS at the midpoint and one back-solve. There is no Jacobian and no factorization.
- Use the midpoint derivative formula of `scaled_midpoint_defect` (`(y_new − y_old + (d1 + d2)/4)/h`).
- Take the norm with `scaled_error_component` on the step's scale.
- Under `WFilteredEnforce` the controller sees `max(err_emb, est)` for acceptance and for the step proposal;
  `Off` skips the estimate entirely.
- The no-op sink is compiled out, so the existing entry points stay bit-identical.
- Facade routing:

  | Request | Route |
  | --- | --- |
  | `dense_output` with no mass matrix, interior `WFilteredEnforce` or `Off` | fast dense; banded twin for `Jacobian::Banded` or an FD band |
  | `Enforce`, or a mass matrix | sequential (unchanged) |
  | JVP-only with dense output | still refused |

- Facade dense Jacobian provider: switch `build()` in `solve.rs` from an allocating `JacobianFn` to
  `OdeProblem::with_jacobian_into` (removes the per-call `DenseMatrix` allocation E1 found, 8.7–18.5 MB per
  Brusselator run).

- [ ] **Step 1: Write failing tests** (`fast_dense_contracts.rs` unless noted):
  - (a) `off_matches_fast_driver_bitwise`: Robertson and HIRES at rtol {1e-4, 1e-6, 1e-8}. Endpoint bits,
    attempts, accepted, rejected and counters equal `integrate_rodas5p_fast_observed` with
    `OutputSchedule::new(vec![t0, t1])`; the same holds for the banded twin on Brusselator n = 100.
  - (b) `segment_endpoints_are_stored_states`: `evaluate_into(t_k)` gives the stored state bitwise for
    every k.
  - (c) `stage_rows_match_sequential_stages`: one step from the same (t, y, h) on Robertson; the fast `u`
    rows equal the sequential stage rows used by `Rodas5pDenseSegment` within 1e-12 relative (a lib unit
    test with `pub(crate)` access). If the coordinates differ, the test documents and applies the transform.
  - (d) `wfiltered_pr_interior_within_two_units`: Prothero–Robinson λ = −1e5, [0, 2], rtol 1e-6, atol 1e-8,
    201 evaluation points. The max interior error is ≤ 2 tolerance units; record the accepted steps next to
    the sequential values 15 (WFilteredEnforce) and 7 (Off), measured and not gated.
  - (e) `interior_rejection_counter_sum`: `accepted + rejected == attempts`, and `interior_rejections > 0`
    on PR at rtol 1e-6.
  - (f) `fatal_rhs_partial_prefix`: under fast dense output, the segments of a run with a fatal RHS after
    ≥ 3 steps are a bit-identical prefix of the clean run.
  - (g) `fast_dense_allocations_after_setup` (`fast_dense_allocations.rs`): allocations during the run are
    ≤ those of `integrate_rodas5p_fast_observed` on the same problem plus 3·⌈log₂(accepted + 1)⌉, which
    covers segment-store growth only.
  - (h) facade `p0_contracts.rs`: `dense_output_routes_to_fast_dense` (driver id, schema v3);
    `dense_output_bit_identical_to_direct_fast_dense`; `enforce_and_mass_stay_sequential`;
    `at_none_outside_span_and_after_failure`; `empty_span_dense_output` (t1 == t0: `at(t0) == y0`).
    Update the wave-2 dense-output tests to the new route; the PR λ = −1e5 test keeps "≤ 2 tolerance units"
    and updates the step count it records.
- [ ] **Step 2: Run** `cargo test -p rodas5p-integrators --locked -j 1 --test fast_dense_contracts --test fast_dense_allocations`
  and `cargo test -p vigilode --locked -j 1`; expect compile failures.
- [ ] **Step 3: Implement** the module, the sink hook, `solve_into`, the facade route and `with_jacobian_into`.
- [ ] **Step 4: Run** the new tests and the regression set:
  - `rodas5p_fast_contracts`, `rodas5p_fast_allocations`, `int03_banded_fast_contracts`,
    `p0_partial_contracts`, `dense_wfiltered_contracts`, `b1_lagged_contracts`, `b1b_gmres_contracts`;
  - `cargo test -p vigilode --locked -j 1` (all targets).

  All pass.
- [ ] **Step 5: Node report** with the PR and Robertson interior and step numbers (measured, deterministic)
  and the allocation counts.

