# Phase 6 — Software validation

Date: 2026-08-08
Functional source commit: `20bdda908326521f5993169263bcbf6a1e6d3a98`

## Scope

The unified harness executes every currently implemented candidate through one Rust process and one result contract. The catalog contains 163 entries:

- 8 sequential RODAS5P variants;
- 4 SABR5P variants;
- 144 partial-coupling homotopy configurations;
- 7 visible but non-executable integrator families.

The executable paths use the same official RODAS5P tableau, mass-matrix convention, WRMS scale, protected direct oracle, failure taxonomy, work counters, and local Rayon case scheduler.

## Defects found and fixed

1. A 1x1 GCRO-DR recycle update reached a generalized-eigen backend not defined for the scalar case. A scalar exact path now bypasses the QZ backend.
2. Restarted GMRES used a scale-aware happy-breakdown threshold for normalization but a different absolute threshold for loop termination. The mismatch could append a zero Arnoldi vector and feed a rank-deficient projected system to QR. One threshold now controls both decisions.
3. The global-order gate used the finest error pair even after roundoff saturation. A deterministic roundoff floor now selects the finest uncontaminated order pair.
4. C3 refined-root exhaustion previously made a research row unclassifiable even though the protected C0 oracle was already available. The row now records an explicit research-only C0 fallback source. The C3 failure remains a promotion blocker.
5. C3/C0 output-budget pass was incorrectly treated as sufficient for method acceptance. Candidate acceptance, embedded-error rejection, and reference certification are now orthogonal decisions.
6. Scientific eligibility and performance promotion were mixed. Scientific gates now report correctness only; Tier-L and Tier-N performance assessments are separate overlays.
7. Batch critical-depth reduction could incorrectly promote a method that was slower and used more scalar work. Joint promotion now requires a measured median wall speedup of at least 1.15x under the matched Rust screen.

## TDD evidence

Red/green transcripts are stored under `research/unified_candidate_v08/receipts/`, including:

- `TASK10_HAPPY_BREAKDOWN_RED.log` / `TASK10_HAPPY_BREAKDOWN_GREEN.log`;
- `TASK11_NONLINEAR_PERF_RED.log` / `TASK11_NONLINEAR_PERF_GREEN.log`;
- `TASK12_C0_FALLBACK_RED.log` / `TASK12_C0_FALLBACK_GREEN.log`;
- `TASK13_REJECTION_ORTHOGONALITY_RED.log` / corresponding green logs.

## Result

The software layer is suitable as a research comparison harness. It is not a production integrator registry: seven method families remain deliberately deferred until native Rust implementations satisfy the same contract.
