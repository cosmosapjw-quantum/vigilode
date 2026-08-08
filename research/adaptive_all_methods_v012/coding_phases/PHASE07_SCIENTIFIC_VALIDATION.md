# Phase 7 — Scientific Validation

## Canonical corpus

- executable candidates: 10;
- analytic-reference problems: 4;
- relative tolerances: \(10^{-4},10^{-6},10^{-8}\);
- absolute tolerance: `0.01 * rtol`;
- canonical runs: 120;
- successes: 120;
- failures: 0.

The external maximum-grid L2 error is nonincreasing as tolerance tightens in all

\[
10\times4=40
\]

candidate–problem combinations.

## Family-specific observations

### BDF1/Radau IIA1

The first-order reference lanes respond strongly to the tolerance ladder. Across the four canonical problems the median macro-step count rises from 9.5 at \(10^{-4}\) to 908 at \(10^{-8}\), while median maximum-grid L2 error falls from about \(1.92\times10^{-4}\) to \(1.98\times10^{-6}\).

### BDF2

The second-order lane responds to both rejection and tolerance. Median maximum-grid L2 error falls from about \(3.03\times10^{-5}\) to \(2.27\times10^{-7}\), with nonuniform-step BDF2 activated after startup.

### High-order RODAS5P/Radau/SABR/Homotopy lanes

These candidates use five requested output intervals over the short canonical horizon and reach approximately \(10^{-12}\)–\(10^{-11}\) external error at all three tolerance levels. This is a **step-clipping-limited screen**, not evidence that their adaptive estimator is tolerance-insensitive. Their adaptive rejection/tolerance behavior is separately tested in focused trajectory contracts.

## Validity boundary

This phase establishes a common adaptive correctness/reference layer for binary64 regular ODEs with nonsingular dense mass matrices. It does not establish production performance because:

1. BDF/Radau use three solves per macro-step through step doubling;
2. Homotopy q=7 is a correctness-heavy configuration;
3. the common output grid caps high-order step growth;
4. the canonical screen has one scientific timing pass, not a randomized repeated timing campaign;
5. sparse, DAE, MPI and GPU regimes are absent.
