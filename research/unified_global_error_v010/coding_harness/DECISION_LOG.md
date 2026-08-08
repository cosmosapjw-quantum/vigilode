# DECISION_LOG.md

## 2026-08-08 — external global error is authoritative

Decision: compare complete integrators by external global error at common output times, not by internal tolerance or timestep count.

## 2026-08-08 — no interpolation in P0

Decision: use nested fixed-step grids; missing output time is failure.

## 2026-08-08 — vector costs, no scalar score

Decision: retain wall/RHS/JVP/Jacobian/factorization/iteration/step/storage axes separately.

## 2026-08-08 — timing authority split

Decision: T1 randomized repeated timing is latency authority; T4 is scientific throughput/determinism only.

## 2026-08-08 — execution clock fix

Decision: separate one-pass scientific suite wall time from the full T1 timing campaign.

## 2026-08-08 — authority promote, solver rank hold

Decision: promote infrastructure only. Defer production ranking until adaptive output/storage and larger-corpus blockers close.
