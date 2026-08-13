# G3 Independent Diff Review

Review scope: base tag `generic-parallel-exponential-foundation-v1.2.0-alpha1` to G3 feature tree.

## Strongest plausible claim

The diff turns the G2 correctness-only exponential foundation into a real adaptive, matrix-free, Newton-free parallel pexprb54s4 candidate by fusing phi combinations, adding an Arnoldi residual diagnostic, preserving failure work, and measuring same-error performance against frozen comparators.

## Strongest objection

The diff also adds a research residual estimator and adaptive policy whose cost and nonnormal reliability are not production-certified. The aggregate speedup can conceal incomplete target coverage and catastrophic oscillatory-PR behavior. Therefore infrastructure promotion must be separated from solver promotion.

## Review findings

### PASS

- No explicit-J or Newton path was introduced into the primary exponential candidate.
- Fused action reuses one augmented Krylov process per linear combination.
- Current code distinguishes full-MGS authority from IOP2 research mode.
- Failed adaptive integration can now expose partial output and attempted work.
- BDF/Radau hashes and build lock remain unchanged.
- No physical client binding entered production Rust source.
- Tests cover order, dense-oracle parity, residual diagnostic, concurrency and failure semantics.

### MATERIAL LIMITATIONS

1. The first Arnoldi residual term is an estimator, not a rigorous arbitrary-nonnormal bound.
2. Current pexprb54s4 target coverage is 11/12, not complete.
3. Oscillatory PR is substantially slower and uses many more steps.
4. T4 is slower than T1 on every current small-state problem.
5. IOP2 success after the residual-estimator change does not erase the legacy failed-work evidence or establish production robustness.
6. The current corpus is calibration/early-comparator evidence, not a sealed expanded holdout.

## Decision

```text
CODE/INFRASTRUCTURE REVIEW    PASS
GENERIC PERFORMANCE CLAIM     HOLD
NEXT BRANCH                   EXPANDED SEALED HOLDOUT + EPIRK-K/W
```
