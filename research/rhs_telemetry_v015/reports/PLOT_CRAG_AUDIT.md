# Plot-based CRAG audit — RHS telemetry v0.15

## Generated plots

1. `BACKEND_RECOMMENDATION_BY_CASE.png`
2. `TRANSFORMED_RANK_BY_METHOD_Q.png`
3. `SUBSPACE_DRIFT_BY_BACKEND.png`
4. `RANK_THRESHOLD_SENSITIVITY.png`

## Direct reading

- Mild/moderate cases contain substantial seeded/block opportunity; severe nonnormal manufactured cases are routed to independent fallback.
- SABR and homotopy q=0/1/2 all span multiple directional ranks. No single backend can be assigned from method name or q alone.
- Seeded-recommended batches have essentially zero matching-batch drift. Block-recommended batches have a benign median but a long tail up to 90 degrees.
- Rank classification changes materially when the relative singular-value threshold is moved by two to four decades.

## CRAG

### Correctness

Plots agree with the machine-readable tables. Fractions sum to one, p90/max tails are visible, and failed paths are listed separately rather than silently removed.

### Retrieval

The observed need for rank-deflation and guarded restart is consistent with multiple-RHS block-GMRES literature, but no external result licenses unconditional block dispatch for these RODAS traces.

### Augmented checks

- rank-threshold mutation: 22.6–38.8% of rows change classification;
- q mutation: every q has a 90-degree drift tail;
- risk mutation: high-risk policy collapses all such rows to protected independent solve;
- magnitude normalization: raw near-rank-one classification can become directional rank four.

### Generation

A production dispatcher should use cheap Gram/QR telemetry, hysteresis, online drift, residual growth, and same-step fallback. It should not perform a full SVD per batch.

## Claim status

- **Surviving:** actual traces justify a guarded transactional dispatcher experiment.
- **Narrowed:** seeded sharing is appropriate only for stable, strongly collinear rank-one batches.
- **Rejected:** “most batches are low rank, therefore block GMRES should be default.”
