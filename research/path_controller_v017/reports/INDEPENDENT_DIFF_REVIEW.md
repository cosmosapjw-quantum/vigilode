# Independent Diff Review — v0.15 → Path Controller v0.17

## Scope

Reviewed changes are limited to:

- schedule contract and reference engine
- path-controller research screen
- CLI and tests
- research artifacts

BDF/Radau are frozen.

## Strengths

1. Existing fixed-path API remains unchanged.
2. Fixed schedule equivalence is tested.
3. Partial numerical failure preserves work and path points.
4. Algebraic and timestep acceptance are explicitly separated.
5. Actual common-W batch/vector work is reported, preventing path-round overclaim.
6. Canonical artifact is deterministic.
7. q7 is retained as an oracle, not marketed as fast.

## Important findings

### Important — one-round survivor is not classical continuation

The strongest schedule jumps directly to \(\lambda=1\) and uses two corrections. Documentation correctly reclassifies it as endpoint preconditioned iteration. Any novelty claim must compare against SABR/Picard/SDC.

### Important — correctness engine is not strict JF

The path screen uses explicit common-W LU and dense offline target certification. It can select schedules but cannot support a Jacobian-free performance claim.

### Important — strict low-depth target remains unmet

Full non-hostile coverage requires 8 common-W batch applications. The 6-batch q1 lane fails PR.

### Important — hostile fallback remains mandatory

No tested schedule passes the hostile nonnormal case.

## Minor findings

- Multiple exploratory schedules share identical points except correction policy; this is acceptable for a bounded research screen but should not become a permanent registry explosion.
- Plot summaries must distinguish non-hostile and all-case quantiles because one hostile row dominates high percentiles.

## Verdict

```text
PASS WITH SCIENTIFIC HOLD
```

The reference infrastructure is ready for a strict matrix-free survivor-integration cycle. Production promotion is not supported.
