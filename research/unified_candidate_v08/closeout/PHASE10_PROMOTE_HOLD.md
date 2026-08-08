# Phase 10 — PROMOTE / HOLD / DEFER closeout

## PROMOTE — research infrastructure

- candidate registry and stable identifiers;
- common C0/C1/C2/C3 certification record;
- explicit C0 research fallback with C3 fallback blocker;
- common global-order, stiff, nonnormal, work, fallback, and thread gates;
- Tier-L and Tier-N separation;
- matched Rust performance overlays;
- Rayon case-level deterministic execution;
- canonical JSON, CSV tables, and scientific checksum;
- scalar GCRO-DR and GMRES happy-breakdown fixes.

## REFERENCE

- `sequential-direct-off`: protected nonlinear-stage and step-output reference;
- `sequential-gmres-off`: Tier-L wall/work reference.

## HOLD — implemented candidates

- six nonreference sequential variants;
- all four SABR variants;
- all 144 homotopy configurations.

Reasons vary by candidate, but the dominant blockers are failure to reach the 1.15x measured wall threshold, increased scalar work, fallback/C3 dependence, or failure of global order/stiff/nonnormal gates.

## DEFERRED — visible, unranked families

- BDF;
- Radau/IRK;
- Peer/W;
- parallel SDC;
- Rosenbrock–Krylov;
- BOROK;
- exponential Rosenbrock/Leja.

These entries are intentionally present so future implementations must join the same registry. They are not assigned synthetic scores.

## Claims prohibited

- every scientific blocker has been solved;
- no future candidate can outperform the references;
- GCRO-DR, SABR, or homotopy is generally inferior;
- the six-case Tier-N timing screen is a production benchmark;
- a protected C0 oracle is a deployable online certificate;
- deferred methods have been compared numerically;
- current Rayon case parallelism proves stage-parallel speedup.
