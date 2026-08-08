# VALIDATION_MATRIX.md

| Requirement | Test/check | Level | Expected | Status | Evidence |
|---|---|---|---|---|---|
| frozen baseline | full release workspace tests | software | 26 pass | PASS | `receipts/BASELINE_RELEASE_TEST.log` |
| parameter validation | focused Rust tests | software | invalid values rejected | PASS | `receipts/TASK2-GREEN.log` |
| path endpoints | focused Rust tests | scientific | exact identities | PASS | `receipts/TASK2-GREEN.log` |
| affine FLRH limit | focused Rust tests | scientific | lambda-independent | PASS | `results/HOMOTOPY_DESIGN_CHECK.json` |
| nilpotent inverse | noncommuting M,J oracle | scientific | q=s-1 agrees with LU | PASS | q7 relative error `2.493e-16` |
| low-q semantics | chain screen | numerical | explicit truncation errors | PASS | integrator test + result JSON |
| output certificate | perturbed endpoint test | scientific | exact correction agreement | PASS | `receipts/TASK4-GREEN.log` |
| nonnormal counterexample | condition-growth screen | numerical | determinant-one does not imply benign | PASS | condition grows to `1.222e8` |
| independent implementation | NumPy reconstruction | scientific | all comparisons pass | PASS | `results/INDEPENDENT_NUMPY_CHECK.json` |
| deterministic artifact | repeat CLI + SHA-256 | reproducibility | identical bytes | PASS | SHA `cc6cb1...7743a` |
| protected regression | full release workspace tests | software | unchanged | PASS | 34 workspace/doc tests, 0 failures |
| format/lint/build | fmt, Clippy, release build | software | pass | PASS | `receipts/FULL_VALIDATION_STAGE1.log` |
| independent review | detached base-to-head diff | review | no blocking finding | PASS | `reviews/INDEPENDENT_DIFF_REVIEW.md` |

| exact code-head closeout | full release gates + deterministic CLI + Git audit | delivery | pass | PASS | `receipts/FINAL_VALIDATION_PASS1_CARGO_TEST.log`; `receipts/FINAL_VALIDATION_PASS1_REMAINDER_PASS.log` |
