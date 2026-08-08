# Independent diff review — homotopy order-policy + Rayon cycle

Date: 2026-08-08 Asia/Seoul
Base: `1e322cfbf5802dbabc6c94afce0e072a667f2081`
Reviewed branch: `homotopy-order-policy-v0.6`

## Review scope

The review compared the full base-to-head diff against the locked design and checked:

- protected sequential RODAS5P and SABR boundaries;
- dimensional consistency of all budget formulas;
- deterministic calibration/holdout splitting;
- policy ranking and false-accept semantics;
- trajectory order/stiff/low-depth gates;
- local Rayon pool isolation and result ordering;
- TDD receipts and scientific result reproducibility;
- manual SIMD/toolchain claims.

## Finding fixed during review

### Important — policy-wide false accepts were attributed to every path configuration

The initial `build_trajectory_gates` implementation attached the winner policy's total holdout
false-accept count to every trajectory method using that policy. This could reject a safe
`(theta,q,rounds,predictor,correction)` configuration because a different configuration failed.

A canonical regression test reproduced the mismatch:

```text
gate=homotopy-theta0-q0-r2-ab2-c0
policy=embedded-relative-eta2.000000e-1
reported=85
matching-path expected=0
```

The gate now filters replay evidence by the exact path configuration. RED and GREEN receipts are
preserved under `receipts/TASK7_GATE_SCOPE_*`. The corrected method-specific counts do not change
the final promotion result: no homotopy candidate passes all order, stiff, low-depth, and
false-accept gates.

## Strengths

- The protected RODAS5P and SABR source files are not algorithmically changed.
- Budget policies use a dimensionless `|h|/h_ref` ratio and reject invalid/nonfinite inputs.
- The local Rayon pool avoids mutating global process state and explicitly fixes the requested
  thread count.
- Scientific rows are stable-sorted and identical at one and four threads.
- Calibration and holdout case IDs are disjoint and deterministic.
- Policy replay preserves the exact cycle-1 candidates, so the policy experiment is not confounded
  by path retuning.
- Negative results are retained rather than optimized away.
- Manual nightly SIMD and unsafe architecture intrinsics were not introduced.

## Remaining scientific limitations

1. `J_R^{-1} R` is a first-order correction and not a rigorous nonlinear upper bound.
2. The FNV parity split is deterministic but not stratified by family or stiffness; a later large
   suite should add stratified holdout seeds.
3. Family-winner objects identify the best calibration candidate even when no candidate in that
   family is safe. Reports must distinguish "calibration winner" from "promoted policy".
4. The cycle-1 source candidate generation remains serial.
5. Full canonical JSON construction and serialization dominate much of the end-to-end timing.
6. The current low-depth candidates do not simultaneously preserve fifth order and stiff
   robustness.
7. AVX-512 instructions in the binary show that vectorized kernels are present, not that the
   measured hotspot used them.

## Disposition

**PASS WITH SCIENTIFIC HOLD.**

The code and evidence are suitable as a research checkpoint and as the input to a certificate
research branch. They are not suitable for enabling a production homotopy fast path.
