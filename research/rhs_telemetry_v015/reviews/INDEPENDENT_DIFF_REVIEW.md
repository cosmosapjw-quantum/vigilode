# Independent diff review — RHS telemetry v0.15

## Scope

Reviewed changes from `vectorized-jf-common-w-v0.14.0-alpha1` to the v0.15 telemetry worktree, including the observer hook, telemetry mathematics, deterministic CLI, tests, analysis script, and failure semantics.

## Strengths

- Read-only feature boundary is explicit and mechanically checked.
- Actual SABR/homotopy RHS batches are instrumented, not synthetic replacement batches.
- Raw, transformed, directional, and drift quantities are separated.
- Hostile path failures are preserved.
- Standard/observed path equivalence is checked for both successful and failed outcomes and for work counters.
- Advice is conservative: high-risk batches never choose speculative block/seeded backends.

## Important issues found and disposition

1. **Observer lifetime compilation failure — fixed.** Repeated long mutable borrows were replaced by short reborrows through a mutable option reference.
2. **Concrete telemetry coupling in homotopy code — fixed.** The production path now depends only on a small observer trait/event contract.
3. **Magnitude-dominated rank misclassification — fixed.** Direction-normalized analysis is used for recommendation.
4. **Unrelated-batch drift — fixed.** Drift history is keyed by semantic phase and propagation level.
5. **Single-threshold fragility — not fixable inside telemetry.** Explicitly retained as an activation blocker.
6. **Reference path is not Jacobian-free — documented.** The report separates dispatcher counters from reference Jacobian/factorization counters.

## Assessment

PASS WITH ACTIVATION HOLD. Merge/tag is appropriate for an experimental telemetry checkpoint. Activating backend selection without shadow-mode calibration would be premature.
