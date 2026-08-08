# Phase 7 — Minimal decisive verification

1. Exact trajectory gives exactly zero error.
2. A known constant perturbation produces the expected endpoint/max/RMS norms.
3. A deliberately dominated point is excluded from every relevant Pareto front.
4. Target attainment selects the minimum-cost successful point below threshold.
5. Common-grid lookup rejects a missing output time rather than interpolating silently.
6. Exact-reference checksum is deterministic.
7. Five fixed anchors run on the same problem/step ladders.
8. BDF2 startup counters are nonzero and retained.
9. Failures are serialized and excluded only from successful fronts.
10. One-/four-thread scientific checksums are identical.
11. Repeated one-thread timing is randomized and summarized by median/IQR.
12. Existing solver regression suite remains unchanged.
