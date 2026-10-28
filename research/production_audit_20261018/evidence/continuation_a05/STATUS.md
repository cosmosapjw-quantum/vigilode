# A05 bounded continuation checkpoint

Base checkout identity: `de990927d0f9197f0e37fcaeccee33bea6c7b8af` on `audit/production-readiness-20261018`; remote was `origin` at the initial check. Shared checkout is concurrently modified by other bounded workers; those files were not reverted or edited here. Per-file source hashes are in `source_sha256.txt`.

## Implemented in this slice

- `OdeProblem::eval_rhs` validates the state shape before charging work, then charges before entering user code. `eval_rhs_into` validates both input/output shape before charging; the batched callback path validates the scheduled input shapes before charging the attempted batch and its scheduled RHS evaluations. Callback `Err`, malformed output, and NaN/Inf output therefore remain charged, while library-side shape rejection is uncharged and does not claim a callback attempt.
- Added scalar Err/NaN, in-place NaN, invalid scalar shape, invalid batch shape (no callback/work), batch callback Err/success, scalar fallback success/failure, and zero-span invalid adaptive/linear configuration regressions.

## Verification

- Test-first witness: the added invalid-shape assertion failed because `eval_rhs([], ...)` returned a shape error but counters were `(1, 1)`; callback count was zero. Raw output: `a05_review_initial_failure.log`.
- After the repair, `cargo test --locked --offline -p rodas5p-integrators --test a05_dense_global_output -- --nocapture`: exit 0, 4 passed. Raw output: `a05_review_final_test.log`.
- `cargo check --locked --offline -p rodas5p-integrators`: exit 0. Raw output: `a05_review_check.log`.
- `rustfmt --edition 2024 --check crates/rodas5p-integrators/src/problem.rs crates/rodas5p-integrators/tests/a05_dense_global_output.rs`: exit 0.
- `git diff --check`: exit 0.
- Review slice source hashes are in `review_source_sha256.txt`.

## A05 remains open

The current `DenseErrorControl` provides Off, pointwise ReportDefect, and midpoint filtered Report/Enforce. None is a continuous interval enclosure. `OdeProblem` has point RHS/Jacobian/JVP callbacks, but no interval extension for enclosing `f(t,p(t))`, no verified derivative/residual tube, and no descriptor certifying the one-sided logarithmic norm over the full tube. There is therefore no sound basis in the current API to control every requested output and between-point error with T1. The frozen PR witness's 101 requested outputs and an independent continuous enclosure were not claimed or run as a pass. Global accuracy/production promotion remains HOLD; A05 requires a reviewed interval-model/gain witness interface before implementation can proceed without guessing.

## Quartic segment review and counter repair

- The immutable segment's interval methods are scoped to the represented stored quartic and its curve derivative. They do not certify the ODE residual or global trajectory; `ContinuousResidualProviderUnavailable` remains the deliberate fail-closed result, and A05/global accuracy remains HOLD.
- Review repair: successful interior value evaluation now charges 1 shared complement operation and 9 interval arithmetic operations per component before any fallible primitive. Derivative evaluation charges 2 shared operations and 14 per component before fallible primitives. This keeps refusal work charged and matches the counted interval arithmetic expression tree (point construction and exact unary negation are not counted as arithmetic operations).
- Allocation boundary by source inspection, not allocator instrumentation: `from_step` allocates endpoint copies, three coefficient vectors, and five identity snapshot vectors. Successful `evaluate_*` methods use caller-owned output buffers and contain no heap allocation on their success path. Error construction may allocate; no general zero-allocation or hot-loop claim is made for this public constructor/API.
- Refreshed source/test/log hashes for the counter-repair run are in `a05_counter_repair_receipt.json`. Older `source_sha256.txt` and `review_source_sha256.txt` are retained as historical receipts and do not bind the new run.
