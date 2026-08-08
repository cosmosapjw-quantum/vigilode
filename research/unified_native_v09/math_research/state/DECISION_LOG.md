# Phase 8 — External Decision Gate

| Object | Decision | Rationale | Remaining risk | Next gate |
|---|---|---|---|---|
| H-001 two-level certificate | PROMOTE | Honest separation of empirical online rejection and proof-producing offline validation; directly addresses current blocker | operational lane may be too conservative; rigorous lane may be costly | frozen false-accept corpus + radius validation |
| H-002 stage-recursive certificate | HOLD | Strong structure-specific possibility, but derivative bounds and nonnormal pessimism unresolved | can collapse to full expensive bound | derive/test on bounded polynomial systems |
| H-003 BDF1/BDF2 anchor | PROMOTE | Smallest decisive native multistep implementation; high information, bounded work | not production variable-order BDF | TDD implementation and order/stiff screens |
| H-004 Radau anchors | PROMOTE | Exact high-order stiff control on same backend | dense block Newton scalability | TDD implementation and exact tableau tests |
| H-005 external-error Pareto | PROMOTE | Prevents tolerance and step-count gaming | reference uncertainty | dual reference methods and confidence metadata |
| H-006 variable-order BDF | HOLD | Necessary production candidate but too large before anchors | controller/history complexity | close H-003 first |
| H-007 parallel SDC | HOLD | Reuses Radau collocation layer | sweeper design and parallel economics | close H-004 first |
| H-008 Peer/W | HOLD | Parallel by construction | startup/events/variable-step theory | separate cycle |
| H-009 ROK/BOROK | HOLD | Important large-scale lane | no representative large sparse trace yet | add matrix-free trace suite |
| H-010 Leja | HOLD | Important semilinear lane | unsupported general DAE/mass-matrix scope | capability-specific branch |

## Decision

Proceed to a bounded coding loop implementing the shared nonlinear contract, fixed-step BDF1/BDF2, Radau IIA1/3 and unified global-order/stiff gates. Do not implement adaptive production methods or all deferred families in the same change.
