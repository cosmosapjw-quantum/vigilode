# Fresh-Context Review Contract

## Inputs

Provide only:

- authoritative implementation base SHA;
- final implementation SHA;
- `git diff BASE..FINAL`;
- this execution package;
- verification logs and explicit skipped checks;
- unresolved blockers.

The first pass is read-only. Output findings only in this form:

```yaml
finding:
  severity: P0|P1|P2|P3
  file: path
  line_or_symbol: location
  violated_invariant: INV-K0-XXX
  reproducer: exact command or minimal evidence
  explanation: why the delta violates the contract
```

Required questions:

1. Did telemetry add an operator/JVP application missing from named overhead?
2. Did observation change convergence, acceptance, output, recycle transaction, or production routing?
3. Are diagnostic applies included in total work using checked arithmetic?
4. Are COMPLETE/STOP_INVALID/ERROR receipts exhaustive and failure preserving?
5. Is the nonlinear remainder exactly `f_i - f_0 - J_n delta_i - c_i h f_t,n`?
6. Does the angle measure current-RHS novelty rather than nested-space self-overlap?
7. Is residual-sign coverage kept separate from norm telemetry?
8. Are the six families, LegacyFixed tolerance, both kernels, and historical evidence unchanged?
9. Do GitHub, PM-7, and the Confluence page point to the same implementation identity?

Pass condition: P0=0 and P1=0. Repair only reproduced P0/P1, then run a new fresh review on the repaired delta.
