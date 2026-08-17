# PHYS–MATH–CODE AUDIT — persistence/budget accepted-grid preflight

## What is genuinely verified

- Six N=128 and six N=512 R-JF-only family shards ran successfully.
- New R-JF rows are exactly identical to the original canonical campaign JSON
  for `t_start`, `h`, transition level, embedded error, RHS count, JVP-vector
  count, and linear-matvec count: 413/413 rows at N=128 and 426/426 at N=512.
- Every new replay contains zero E-K shadow work, zero explicit Jacobian builds,
  zero direct factorizations, and zero Newton iterations.
- Debug wall time is excluded from policy scoring. Historical optimized G4-S5B0
  medians are joined only after exact R-JF parity.
- Frozen v2.5 policy: error-drop q50, k=1, delta=3%. Pooled development metrics
  remain similar across N=128→512.

## P0 — accepted-grid look-ahead blocker

The existing atlas stores accepted steps only. Runtime event logic observes the
completed accepted step, then sees the controller's next proposal *before* that
next R trial is accepted or rejected. Scoring the event against the eventually
accepted next atlas row can therefore use information that was unavailable at
runtime when intermediate rejections occur.

Observed rejected attempts:
- N=128: 18 total.
- N=512: 21 total.

**Required fix before N=2048:** attempt-level R-JF trace + actual next-proposal
prefix instrumentation, with rejected/speculative work retained.

## P1 — objective and group robustness

The frozen opportunity-first selection is not robust:
- v2.5 k=1: min(N128,N512) opportunity 0.8645; precision 0.8158.
- epsilon-opportunity k=2: opportunity 0.8564; precision 0.9444.
- safety-first k=3 diagnostic: opportunity 0.7910; pooled precision 1.0 and
  development family precision 1.0.

The N=512 pooled k=1 precision 0.825 hides family precision 0.50 for HIRES and
Van der Pol and 0.789 for nonautonomous forcing.

These are diagnostics only; v2.5 is not retuned after N=512.

## P1 — empirical wall cap terminology

`PrefixBudget` calls its input a certified upper bound, while the current value
is calibration max × 1.25. Rename/downgrade this in the next node or replace it
with a deterministic work-unit cap / interruptible prefix budget.
