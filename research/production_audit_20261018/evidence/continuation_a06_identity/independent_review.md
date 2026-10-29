# A06 native stage RHS convention review

Verdict: SCOPED PASS for the native matrix and first sequential-stage RHS mapping. Full A06 trajectory integration and matched-speed claims remain HOLD.

Reviewer role requested: `costroute-research-review / gpt-6-astra / xhigh`. Observed runtime identity: `NOT_INDEPENDENTLY_OBSERVED`. Review was read-only; no commands, edits, or test reruns were performed.

The source matrix is `ShiftedOperator` with identity mass: `A = I - h*gamma*J`, so `s=h*gamma`. Sequential stage zero supplies `r0=h*f(y0)=(-1/8,0)` for the frozen scalar damping fixture; the Woodbury solve receives `r0` directly. Multiplying by `s` again is incorrect.

The regression invokes `build_step_context_matrix_free` and `sequential_stages`, compares the actual first stage result with Woodbury, and checks the actual shifted residual. Its negative control solves with `s*r0` and confirms the wrong equation leaves `(1-s)*r0`; this proves the regression detects the scaling error. It does not show that `solve_into` or `residual_witness` returns a refusal for that wrong candidate.

Reviewed source identities matched the receipt. The pre-EXEC target passed 6/6 and clippy passed. The post-EXEC target is recorded separately. Formatting and diffcheck logs are empty; their exit 0 values are receipt-reported, not independently evidenced by those empty logs.
