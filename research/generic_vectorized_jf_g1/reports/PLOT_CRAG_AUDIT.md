# G1 Plot-based CRAG Adversarial Audit

## Plot reading

### `G1_FAST_FALLBACK_FRACTION.png`

One-step coverage is 87.5%, just below the 90% gate. Adaptive t1/t4 coverage collapses to 49.75%, with
50.25% protected fallback. The adaptive bars are identical scientifically; thread count does not alter lanes.

### `G1_CRITICAL_PATH_DEPTH.png`

One-step median is exactly the sequential depth 8 and P95 is 13.2. Adaptive median and P95 are both 16.
The central architectural objective—shorter within-step critical path—is not achieved.

### `G1_SAME_ERROR_SPEEDUP.png`

q1→q2 t1/t4 reach 95.8% of protected targets but have speedups 0.226 and 0.123. BDF2's plotted 0.425 is
based on only 4.2% coverage, while Radau IIA3 reaches 75% and gives 2.371× on that subset. Coverage labels
are load-bearing and prevent a coverage-biased headline.

### `G1_PAIRED_COST_RATIOS.png`

Step ratio is 1, but RHS is 5.125× and JVP work is 8.534×. Wall cost is 4.71× for t1 and 7.78× for t4.
This directly reproduces the original problem in a worse form: good step economy is overwhelmed by within-step work.

### `G1_FAST_FRACTION_BY_PROBLEM.png`

Complex Dahlquist, scalar linear, and noncommuting mass are fully handled by q2. PR, n=32 advection–diffusion,
and n=32 nonnormal nonlinear blocks fall back completely. Oscillatory PR is nearly all fallback. The failure is
therefore not a single isolated outlier but tracks nonlinear/nonnormal difficulty.

## CRAG

### Correctness

Fallback work is included, BDF/Radau coverage is annotated, and q2 depth parity is not called low depth. PASS.

### Retrieval/contrast

The plots agree with the protected RODAS contract: q2 is depth parity, fallback doubles depth, and independent
case-level Rayon throughput is not presented as solver-internal speedup. PASS.

### Augmentation

The negative result survives one-step and adaptive trajectories, same-control and same-error comparisons, and
strict sub-8 reinterpretation. Under the strict interpretation adaptive fast fraction is zero.

### Generation

Larger dimensions could improve batched linear algebra, but cannot remove q2 depth parity or fallback duplication.
A different high-order time-integration structure is predicted to be required.

## Adversarial mutations

- Remove q2 diagnostic batch: nominal depth 7, but correctness evidence is weakened; inadmissible.
- Ignore speculative work: violates the locked cost contract and still leaves q2 at depth parity.
- Count q2 as low depth: false because its accepted depth is 8.
- Restrict to easy cases: violates the sealed generic corpus.
- Headline Radau speed without coverage: misleading; only 18/24 targets are attained.

## Final claim status

- Surviving: strict matrix-free/Newton-free transactional state machine with protected fallback.
- Narrowed: q2 is a depth-parity correctness lane for several smooth/linear structures.
- Rejected: q1→q2 is a generic accelerated solver meeting the G1 objective.
