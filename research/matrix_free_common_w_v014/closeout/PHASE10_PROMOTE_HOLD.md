# Phase 10 — PROMOTE / HOLD Closeout

## PROMOTE

- strict matrix-free common-W operator contract;
- batched operator/preconditioner work accounting;
- independent serial/Rayon GMRES controls;
- block GMRES research backend;
- seeded shared rank-one backend;
- unpreconditioned true-residual certificate;
- failure-preserving canonical matrix and plots.

## HOLD

- block GMRES as universal/default backend;
- seeded shared GMRES outside verified numerical-rank-one cases;
- any dispatch rule based only on synthetic exact rank;
- integrated homotopy/SABR speedup;
- production preconditioner, sparse, GPU, or DAE claims.

## Gate result

```text
STRICT MATRIX-FREE COMMON-W GATE: PASS
GENERAL BLOCK ROBUSTNESS GATE: FAIL/HOLD
SAFE MULTI-RHS FALLBACK GATE: PASS
HOMOTOPY INTEGRATION GATE: OPEN
```

## Why block remains alive

Among successful rows, block GMRES has median speedup 5.018 and median operator ratio 0.331; 353/396 successful rows exceed 1.15x. The failures are concentrated and diagnosable rather than universal. Therefore rejection is too strong, but unconditional promotion is unsupported.
