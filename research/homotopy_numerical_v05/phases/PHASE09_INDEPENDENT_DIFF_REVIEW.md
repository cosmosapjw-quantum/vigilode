# Phase 9 — Independent diff review

A separate clean worktree reviewed
`ce1cc51034186958da3e27f641f29aae777ceaf1..1704f2ce60c556f3bb577bcfc6d12c90e5650168`.

Result: **PASS WITH SCIENTIFIC AND PERFORMANCE HOLD**.

Blocking review findings fixed before closeout:

1. renamed the absolute normalized output error threshold from the misleading
   `defect_budget_fraction` to `output_wrms_budget` and added a failing-then-passing CLI contract;
2. separated common-`W` batch call depth from vector work and removed any equal-work conclusion
   that relied on incomparable `direct_solve_calls` semantics.

Accepted HOLD findings:

- q<7 begins from an approximate, not exact, lambda=0 path solution;
- the exact target-Jacobian certificate is a reference oracle rather than a cheap fast certificate;
- fixed output-WRMS 0.1 does not preserve fifth-order global convergence;
- no real concurrent batch backend or wall-time speedup was tested;
- q=7 is a correctness lane only.

Detailed record: `reviews/INDEPENDENT_DIFF_REVIEW.md`.
