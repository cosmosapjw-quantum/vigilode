# G1 Closeout — PROMOTE / HOLD

## PROMOTE

- strict matrix-free q1→q2 transaction state machine;
- retained-endpoint q2 escalation;
- original-system protected fallback;
- speculative/fallback work preservation;
- generic one-step and adaptive same-error gates;
- deterministic T1/T4 scientific reporting.

## HOLD

- q1→q2 as default generic solver;
- sub-8 critical-path claim;
- same-error acceleration claim;
- standalone fast-path fifth-order claim;
- BDF/Radau modification;
- physical-client execution.

## Canonical kill evidence

```text
one-step fast fraction       0.875 < 0.90
one-step fallback            0.125 > 0.10
adaptive fast fraction       0.4975 < 0.90
adaptive fallback            0.5025 > 0.10
adaptive q1 accepts          0
median / P95 depth           16 / 16
same-error speedup t1        0.233 < 1.15
same-error speedup t4        0.076 < 1.15
```

```text
G1_TRANSACTIONAL_Q1_Q2 = HOLD
G1_INFRASTRUCTURE       = PROMOTE
NEXT_CORE_NODE          = PARALLEL_FIFTH_ORDER_EXPONENTIAL_ROSENBROCK_EPIRK
```
