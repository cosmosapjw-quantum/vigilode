# Phase 10 — Promote / Hold closeout

## PROMOTE — research reference layer

The following are promoted onto the homotopy research branch:

- validated `PartialCouplingParameters`;
- `AffinePartialCouplingOracle` for explicit dense reference systems;
- affine path/target operator and residual actions;
- normalized coupling action `Q=eta D^{-1}C`;
- truncated finite inverse action for `q=0,...,s-1`;
- original affine RODAS-output certificate;
- deterministic `homotopy-design-check` CLI and JSON schema;
- red/green, full validation, NumPy cross-check, and independent review receipts.

Promotion means “trusted as a correctness oracle and experiment-seeding API.” It does not mean production solver readiness.

## HOLD — next numerical-experiment branch

The following require actual numerical experiments:

- nonlinear remainder `N(K)` and its batched evaluation;
- lambda/theta path stepping;
- `q=0,1,2` tangent/corrector economics;
- path-step controller and rejection rules;
- original nonlinear output certificate;
- sequential fallback transaction;
- fifth-order global convergence;
- stiff decay and Prothero–Robinson behavior;
- nonnormal robustness;
- index-1 DAE constraint preservation;
- CPU/GPU batching and wall-time break-even;
- comparison with equal-work SABR5P.

## DEFER — common comparison cycle

GCRO-DR `(m,k)` tuning, adaptive recycle admission, BDF/Radau ranking, sparse/MPI/GPU generalization, and broad production tuning remain deferred until a homotopy candidate survives the initial kill screens.

## Prohibited claims

- the current code solves nonlinear RODAS stage systems by homotopy;
- `q<=2` is generally sufficient;
- the exact affine path result implies good nonlinear conditioning;
- deterministic design-check status implies method stability or speedup;
- RODAS5P now outperforms BDF/Radau;
- novelty of a final method is established.

## Closeout decision

**PROMOTE reference layer; HOLD numerical method.**


## Final delivery evidence

The complete functional tree at `934b49fc57656310ecc3aa39f241ee1e7eaee999` passed exact release validation before this documentation-only closeout commit:

- `cargo fmt --all -- --check`;
- strict release Clippy with `-D warnings`;
- 34 workspace and compile-fail doc tests;
- release build;
- byte-identical design-check outputs with SHA-256 `cc6cb1c963e75415f5b35ba879d83b081a08fd7103e0f2cc27c234e63ec7743a`;
- coding-harness, protected-diff, dynamic-dependency, source-guard, Git-fsck, and clean-tree checks.

The first monolithic validation command exceeded the container command window during fat-LTO test compilation, and the first remainder script used an incorrect JSON field name. Both failed transcripts are preserved. The same test command subsequently completed with exit code zero, and the corrected remainder validation passed.
