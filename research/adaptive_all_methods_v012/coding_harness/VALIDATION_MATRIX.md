# Validation Matrix

| Gate | Status | Evidence |
|---|---|---|
| Baseline full tests | PASS | `coding_receipts/BASELINE_FULL_TEST.log` |
| Common controller | PASS | `TASK1_*` receipts |
| Fixed-step regression | PASS | `TASK2_FULL_GREEN.log` |
| Variable-step BDF2 algebra | PASS | `TASK3_*` receipts |
| Adaptive BDF/Radau order and rejection | PASS | `TASK4_*`, `TASK5_*` |
| Adaptive Homotopy fallback | PASS | `TASK6_*` |
| Unified adaptive screen | PASS | `TASK7_*`, canonical results |
| T1/T4 determinism | PASS | identical checksum in canonical JSON |
| Exact-HEAD fmt/clippy/test/build | PENDING_FINAL | final validation receipt |
| Independent diff review | PASS_WITH_HOLD | `coding_phases/PHASE09_INDEPENDENT_DIFF_REVIEW.md` |
