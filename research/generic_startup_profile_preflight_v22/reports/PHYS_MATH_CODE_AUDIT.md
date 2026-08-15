# PHYS–MATH–CODE AUDIT — G4-S5B2 Preflight

## Code-path reality

```text
immutable G4-S1/G4-S5B0 campaign data
  -> corrected G4-S5B1 unit-time cost rows
  -> future-perfect startup profile upper bounds
  -> optimistic safety-triggered re-entry upper bound
  -> no candidate source mutation
```

## Genuinely fixed

- The G4-S5B2 contract is killed before unnecessary prefix-integration code is added.
- Import/replay is deterministic and generates machine-readable trajectory tables.
- Regime-drift and short-horizon overhead are separated.
- Actual prefix kernels from G4-S4 are preserved unchanged.

## Still uncertain

- Runtime change-point detection without future information.
- Actual E→R controller/cache transfer.
- R→E re-entry using reusable Arnoldi prefix.
- Forced-switch fifth-order recovery.
- N=2048 sealed holdout.

## Regression risk

The main risk is reporting the optimistic re-entry upper bound as an implemented selector. It is not.
