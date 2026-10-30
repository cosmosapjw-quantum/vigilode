# RVJ N4 | source-first self-adversarial review

Date: 2026-10-30 KST. Reviewer = SAME_ASSISTANT. `native_subagent=NOT_RUN`; formal C9 `0/1_NOT_RUN`.

| Finding | Evidence / consequence | Resolution |
| --- | --- | --- |
| N4-ORACLE-01 (medium, numerical test oracle) | `a=1000,b=1000.5` gives an extremely small oscillatory integral. Plain `mpmath.quad` with insufficient relative precision initially disagreed with rigorous Fraction interval; treating it as truth produced false FAIL. | Replaced primary independent benchmark with high-precision integration-by-parts analytic recurrence. Kept direct quadrature discrepancy as diagnostic, did not adjust physical tolerance. |
| N4-SERIES-02 (low, code branch) | Theta exactly zero caused the generic interval phase padding to become slightly negative after outward addition of zero and triggered false rejection. | Specialized `theta=0` -> real nonnegative J_l. Native test and zero-phase symmetry now PASS. |
| N4-CLIPPY-03 (low, Rust style) | First strict lint rejected unused research-only code and needless borrows following opt-in transaction patch. | Made transaction run in research binary, changed borrowed arguments, reran 34/34 tests, strict Clippy/fmt PASS. |
| N4-SOURCE-04 (block-now for generalization) | `Rust n3h_source_bounded_copy.rs` is byte-locked N3H source plus tiny wrapper; kernel interval uses represented fixed-source parameters. Arbitrary OdeProblem callbacks are not cryptographically bound to it. | Keep research-only. Need native PR131 actual source+trial identity on every original stage, separately test wrong-owner substitutions and stale epoch. |
| N4-INTERVAL-05 (block-now for actual certified trajectory) | A verified interval for M_l does not certify actual cubic convolution, accepted carrier, whole-trajectory roundoff or exact-RHS physical error. | Must propagate interval through full N3H candidate, then add to inherited source-specific bound. No production acceptance. |
| N4-RAWU-06 (block-now for time-step admission) | All eight original raw-U residual enclosures are small but `embedded WRMS≈253200` for h=.25; force-accept would be false physical admission. | Added research-only fail-closed transactional gate with embedded<=1, full residual precondition, bit-exact carrier; h=.25 refused, h=1e-8 admitted, 16=8+8 direct solve charges. |
| N4-NATIVE-07 (runtime environment) | Attempted cargo test/check of previous offline `rodas5p-integrators` through actual workspace; complex dependency chain faer/serde exceeded bounded runtime. | Preserve logs, report NOT_RUN/INCOMPLETE for that specific native integration. Independent rust1.99.1 source-specific RODAS8 reference **is** compiled and tested; never substitute this for shipped PR131. |
| N4-SPEED-08 (not yet verified) | n12/24 equal-h comparison shows physical accuracy difference but not matched achieved accuracy or charged CPU cost of adaptive routes. | Require actual adaptive trajectories, n3/8/12/24/65/257, per-attempt residual/certificate time and total work. |

## Explicit scientific guards

- Correct energy identity for physical equations requires **real** `chi` and `phi`, positive weights and common source in both RHS/moments. Complex couplings or replacing mα callbacks can spoil dissipation.
- Exact-real source moments are treated as represented f64 inputs. This is **not** a general theorem for continuous parameter uncertainty.
- `I` interval assumes RN, gradual subnormal, no unsafe `fast-math`; scope supported source box, 240 tested points. Compiler flag and CPU semantic portability remain to check on other platforms.
- Floating original stage residual interval at one candidate gives consistency, not stability/forward error to the true implicit stage (requires resolvent/inverse bound) and not IVP temporal error.
- Legacy Rust `RODAS5P` sequential direct solve has fixed tableau; finite causal prefix identity `(I-L)^-1=(I+L)(I+L²)(I+L⁴)` is exact only for strictly lower triangular *affine* L. It is not permission to parallelize arbitrary nonlinear stage root unconditionally.

## Narrow verdict

**ACCEPT_SCOPED N4 kernel interval + small Rust source-specific original raw-U reference + opt-in reject/accept transaction.** `N4_FULL=PARTIAL`, because actual source-bound PR131 integration, floating full carrier and matched speed remain open. Official VigilODE 7/23, REI 28/88, successor admitted 0/12, RVJ N1–N3 3/8 with N4 partial. C9 0/1, C10 NOT_STARTED, production HOLD, no main/default mutation.
