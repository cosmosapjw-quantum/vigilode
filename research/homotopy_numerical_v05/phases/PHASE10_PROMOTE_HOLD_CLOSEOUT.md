# Phase 10 — PROMOTE / HOLD closeout

## PROMOTE — experimental correctness and measurement layer

- read-only nonlinear RODAS block decomposition;
- exact explicit target-Jacobian/output-correction certificate;
- fixed-schedule partial-coupling experimental kernel;
- q=7 finite-inverse correctness oracle;
- q=0/1/2 low-depth measurement lanes;
- deterministic canonical/smoke screen and immutable JSON evidence;
- protected sequential RODAS5P and SABR controls on the same Rust backend;
- transactional sequential fallback;
- separate batch-depth, common-`W` vector-work, RHS, JVP, correction, and fallback ledgers.

## HOLD — method or production promotion

- fixed absolute output-WRMS budget as a global acceptance policy;
- general sufficiency of q<=2;
- fifth-order or stiff-decay preservation for low-depth paths;
- index-1 DAE constraint preservation;
- adaptive theta/lambda/q schedules;
- Anderson, Chebyshev/Padé, or nonstationary sweeps;
- sparse, MPI, GPU, SIMD, or threaded performance;
- default solver selection and BDF/Radau ranking.

## REJECTED claims

- zero one-step false accepts imply global order preservation;
- low batch-call depth implies lower scalar work or wall time;
- `direct_solve_calls` is comparable between scalar and batched solve APIs;
- q=7 is a practical fast path;
- this cycle demonstrates acceleration.

## Quantitative closeout

```text
cases                                      57
controls                                   114
candidates                                 8208
fast accepts                               3510
fallbacks                                  4698
failures                                   0
one-step false accepts                     0
low-depth fast rows                        401
low-depth represented cases                16
median low-depth common-W batches          5
median low-depth common-W vectors          40
fast candidate scalar-work dominance       0 vs sequential, 0 vs SABR
```

The next branch must first replace the fixed absolute output budget with order-aware policies and
repeat the global-order/stiffness kill screens before any broader tuning.
