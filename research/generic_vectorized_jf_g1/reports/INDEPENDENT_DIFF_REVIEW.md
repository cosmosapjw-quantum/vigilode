# G1 Independent Diff Review

## Scope

Reviewed the G0→G1 diff, new tests, deterministic raw artifacts, analysis script, plots, and declared
promotion gates. This review is independent in role, not in organizational identity.

## Steelman

The implementation is a faithful attempt to salvage the most promising RODAS-specific structure:
a q1 endpoint-correction attempt, retained-state q2 causal escalation, and exact protected matrix-free
fallback. It preserves accepted-step economy, explicit-J-free operation, Newton-free fast-path
semantics, and honest accounting of speculative work.

## Strongest objection

The method does not reduce the generic adaptive critical path. q1 is never the final accepted lane,
q2 is depth parity with sequential RODAS, and fallback doubles the path. The implementation is
correct enough to establish this negative result, but the result kills the primary acceleration claim.

## Code findings

### PASS

- strict matrix-free context rejects missing JVP instead of materializing an explicit Jacobian;
- q1 work/state is retained during q2 escalation;
- speculative work survives fallback and rejection;
- original-system fallback is used, not a surrogate;
- q1/q2/fallback lane semantics are explicit in reports;
- work-counter accumulation is saturating and fieldwise;
- BDF/Radau hash freeze and physical-client exclusion tests still pass;
- deterministic one-step and adaptive scientific checksums pass across T1/T4.

### MATERIAL SCIENTIFIC HOLD

- the operational contraction-tail estimate is empirical, not a rigorous nonlinear bound;
- q2 needs an eighth certificate batch, erasing the nominal depth-7 causal update;
- q1 has zero adaptive accepts in the final corpus;
- fallback duplicates eight speculative batches with eight protected stages;
- same-error performance is decisively below the 1.15 gate;
- the corpus is still small/dense enough that stage-level Rayon overhead is unfavorable.

### Non-G1 issue observed but not repaired

The expanded legacy adaptive suite exposed a pre-existing GCRO-DR generalized-eigen buffer panic.
A dedicated G1 candidate runner correctly isolates the G1 decision from that deferred comparator.
This issue does not change the G1 verdict and should not be repaired on the G1 branch.

## Decision

```text
CODE/CONTRACT REVIEW = PASS
SCIENTIFIC METHOD     = HOLD
NEXT DAG NODE         = PARALLEL FIFTH-ORDER EXPONENTIAL ROSENBROCK / EPIRK
```

No material code defect was found that would plausibly reverse the negative G1 performance result.
