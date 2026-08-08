# Phase 9 — Independent diff and result review

## Review target

Base: `homotopy-order-policy-v0.6.0-alpha1`
Functional head: `20bdda908326521f5993169263bcbf6a1e6d3a98`

## Findings

### Critical — false performance promotion

The first implementation treated lower batch depth as sufficient measurable advantage. This promoted SABR configurations that were 2–6 times slower and used substantially more RHS/JVP work. Performance was moved out of the scientific gate and into Tier-specific measured assessments with a locked 1.15x wall-speed threshold. Final joint promotions are zero.

### Critical — certification and candidate acceptance were conflated

C3/C0 output-budget pass previously forced a completed outcome. A candidate may independently reject on its embedded error even when its output is near the oracle. Regression tests now establish that method acceptance and reference certification are orthogonal.

### Important — unclassifiable C3 exhaustion

Four severe homotopy rows exhausted the C3 iteration budget. Because the protected C0 oracle is part of the research harness, these rows are now classified through an explicit C0 fallback and rejected. C3 fallback use remains visible and blocks promotion.

### Important — GMRES happy breakdown

A threshold mismatch could create a zero Arnoldi vector. The same scale-aware threshold now controls normalization and happy-breakdown termination.

### Important — roundoff-contaminated order estimate

The raw finest pair gave the protected method an apparent order near 3.2. Qualification now uses the finest pair above a deterministic roundoff floor; protected RODAS5P recovers the expected approximately fifth-order behavior.

## Result audit

The independent machine-readable audit verifies:

- exact catalog counts;
- one result for every catalog entry;
- no deferred candidate in a numerical ranking;
- zero joint promotions;
- all HOLD rows have explicit blockers;
- four C0 fallbacks are rejected and over budget;
- one-thread/four-thread scientific identity.

Receipt: `receipts/INDEPENDENT_RESULT_AUDIT.log`.

## Review disposition

**PASS WITH SCIENTIFIC AND IMPLEMENTATION HOLDS.**

No blocker remains in the common comparison infrastructure. Remaining blockers belong to the candidate methods themselves or to unimplemented deferred families.
