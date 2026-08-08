# Phase 9 — Survivor Formalization

## F-001 — Complete-integrator comparison contract

A candidate declares:

- capability: ODE, nonsingular mass matrix, residual-form index-1 DAE, semilinear matrix-function;
- state/history requirements;
- nonlinear and linear solver requirements;
- native error estimator and controller;
- work ledger schema;
- failure/restart semantics.

Unsupported candidate/problem pairs are `NOT_APPLICABLE`.

The authoritative cross-family result is a global-error–cost Pareto set, not a scalar score.

## F-002 — Native anchor set

\[
\mathcal C_{\rm anchor}
=
\{\text{RODAS5P},\text{SABR},\text{homotopy},\text{BDF1},\text{BDF2},\text{RadauIIA1},\text{RadauIIA3}\}.
\]

- BDF2 starts with BDF1 and charges startup.
- RadauIIA1 is a cross-family identity check against backward Euler/BDF1.
- RadauIIA3 uses the exact fifth-order tableau and a full coupled Newton reference solve.

## F-003 — Nonlinear certificate metadata

Every nonlinear result stores:

```text
certificate_kind = operational | rigorous | oracle
rigorous = true | false
assumption_set_id
residual_norm
correction_norms
contraction_proxy
root_radius (optional)
output_bound (optional)
fallback_used
```

`rigorous=true` is forbidden unless an assumption set and validated derivative/interval bound are attached.

## F-004 — Implementation roadmap

1. Extend work counters and define a common dense Newton engine.
2. Implement BDF1/BDF2 fixed-step history and startup.
3. Implement Radau IIA1/3 coupled stages.
4. Add candidate registry execution and global order/stiff gates.
5. Add external error–cost table generation.
6. Leave variable-order BDF, SDC, ROK/BOROK, Peer/W and Leja as explicit next-cycle candidates.

## Claim labels

- Exact BDF/Radau identities: SUPPORTED by symbolic derivation.
- Mature production behavior of CVODE/RADAU5: SUPPORTED by official docs/literature, but not inherited by native anchors.
- Two-level certificate utility: HYPOTHESIS until holdout tests.
- Stage-recursive rigorous bound efficiency: HYPOTHESIS.
