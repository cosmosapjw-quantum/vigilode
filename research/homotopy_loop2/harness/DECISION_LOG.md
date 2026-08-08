# DECISION_LOG.md

## D-001 — Scope

Implement only affine/reference homotopy structure and deterministic screens. Do not implement the actual numerical path solver.

## D-002 — Oracle backend

Reuse existing `DenseMatrix` and `faer` LU so the reference path remains on the same Rust numerical backend.

## D-003 — Certification

Use original target residual and exact affine target correction. Homotopy residual alone is never an acceptance signal.

## D-004 — API isolation

Place all new behavior in a research module and one design-check CLI command; protected sequential/SABR code remains untouched.

## D-005 — Finite inverse scope

Expose exact and truncated inverse actions as affine/reference kernels only. Do not label `q=0,1,2` as a nonlinear solver or convergence guarantee.

## D-006 — Design-check status

A `pass` status certifies structural identities and deterministic screens, not runtime speedup, fifth-order preservation, stiff decay, or DAE validity.

## D-007 — Independent backend

Use NumPy only as an external scientific cross-check. Performance comparisons remain Rust-only in the next branch.
