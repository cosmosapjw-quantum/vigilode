# Phase 5 — Implementation Receipt

Date: 2026-08-08
Baseline: `ea7303c280d9c2e4e8769d06f5660a42361318aa`
Branch: `adaptive-all-methods-v0.12`

## Implemented bounded architecture

### Common adaptive state machine

`crates/rodas5p-integrators/src/adaptive.rs` now provides:

- `AdaptiveStepConfig` with finite positive tolerances and bounded step factors;
- integral and PI controller policies;
- acceptance-only PI memory updates;
- rejection caps and minimum/maximum step guards;
- estimator-order-aware update exponents;
- method-independent diagnostics;
- step-doubling WRMS reference error.

### Existing RODAS5P endpoint family

Sequential RODAS5P, SABR5P and Homotopy RODAS use the existing RODAS5P 5(4) endpoint estimator and a shared controller through additive observed APIs. Existing fixed-step and legacy adaptive wrappers remain intact.

### BDF reference family

Implemented exact nonuniform BDF2 coefficients

\[
a_0=\frac{1+2r}{1+r},\qquad
 a_1=-(1+r),\qquad
 a_2=\frac{r^2}{1+r},\qquad
 r=\frac{h_n}{h_{n-1}},
\]

plus a history-aware predictor and transactional adaptive BDF1/2 reference lane based on one coarse step versus two half steps. BDF2 startup uses BDF1; the estimator order changes from 2 to 3 when the two-step method becomes active.

### Radau reference family

Implemented adaptive Radau IIA1 and three-stage fifth-order Radau IIA through the same coarse-versus-two-half-step reference estimator. The accepted state is the two-half-step endpoint; all coarse, fine and rejected work remains in the ledger.

### Homotopy family

Implemented an adaptive Homotopy RODAS wrapper using the native RODAS embedded endpoint error plus algebraic certificate, with protected sequential fallback and transactional state/recycle handling.

### Unified adaptive screen

`rodas5p-fair-ab` and the CLI now expose a deterministic adaptive global-error screen for ten executable candidates:

- sequential RODAS5P direct, GMRES, LGMRES and GCRO-DR;
- SABR5P;
- Homotopy RODAS q=7 reference lane;
- BDF1 and BDF2 adaptive reference lanes;
- Radau IIA1 and Radau IIA3 adaptive reference lanes.

The screen uses four analytic-reference problems, three tolerance levels and requested-output-only storage. Independent cases are parallelized with a local Rayon pool; result rows are canonically sorted before hashing.

## Commit chain

- `7ac5445` common adaptive controller;
- `e0b5610` RODAS/SABR controller refactor;
- `b485583` exact variable-step BDF2;
- `a76fde5` adaptive BDF reference lane;
- `3855700` adaptive Radau reference lane;
- `17d81c7` adaptive Homotopy lane;
- `2446518` unified adaptive screen;
- `4289653` lint/result-artifact closeout.
