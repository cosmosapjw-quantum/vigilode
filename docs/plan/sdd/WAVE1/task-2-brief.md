## Task 2: A2-wfiltered — W-filtered interior error control (node id `a2-wfiltered`)

Plan §5 T3. Add `DenseErrorControl::WFilteredReport` and `DenseErrorControl::WFilteredEnforce` to
`crates/rodas5p-integrators/src/dense_output_v2.rs`.

- Estimate: E = (M − hγJ₀)⁻¹ (h/2) d(1/2), with d the existing midpoint defect of
  `scaled_midpoint_defect` (θ = 1/2), J₀ the Jacobian the step itself used and γ the RODAS5P diagonal
  coefficient. Reuse the step's own factorization of M − hγJ₀ when the driver has it (expose it from the
  step without changing the step's arithmetic); otherwise factor M − hγJ₀ once per sampled step and record
  that in the report's work counters (`direct_factorizations`). Either way no new Jacobian evaluation.
- `WFilteredReport` records samples, never changes the run. `WFilteredEnforce` uses
  `max(embedded_error, estimate)` in the controller for steps that contain an output time, so an interior
  failure shrinks the step through the normal controller rather than through a separate rejection rule.
- Existing `Off`/`ReportDefect`/`Report`/`Enforce` stay bit-for-bit unchanged.
- Tests (new file `tests/dense_wfiltered_contracts.rs`): (a) Lemma 3 numerically — for y' = λy with a
  constant defect, the ratio estimate/true interior error at θ = 1/2 lies in [1, 1/(2γ)] = [1, 2.3592] for
  z = hλ ∈ {−1e-3, −0.1, −1, −10, −1e3, −1e6}; (b) Prothero–Robinson λ = −1e5, t ∈ [0, 2], 201 output
  times, rtol ∈ {1e-4, 1e-6, 1e-8} (the cases of `docs/DENSE_OUTPUT_ERROR.md` and
  `tests/dense_error_control_contracts.rs`): under `WFilteredEnforce` the true interior max is ≤ 2
  tolerance units at every rtol, and the accepted-step count is reported next to `Off` and `Enforce`;
  (c) the nonlinear witness of `docs/reviews/20261018_production_audit/DEFAULT_PATH_KO.md` DP-01
  (y' = −1e6(y − sin t) + cos t + 10(y − sin t)², y(0) = 0, [0, 1], rtol 1e-8, atol 1e-11, 101 outputs):
  report max normalized interior error and Jacobian/LU counts for Off, Enforce, WFilteredEnforce.
  Kill gate (plan §6 A2): interior error > 2 tolerance units at rtol 1e-6 on Prothero–Robinson, or
  more than 1.3x the `Off` step count. If the kill gate fails, keep the code, mark the result FAIL in the
  node report and in the test (as an `#[ignore]`d measurement test with the numbers), do not loosen it.

