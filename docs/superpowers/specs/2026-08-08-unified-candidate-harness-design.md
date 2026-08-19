# Unified RODAS5P Candidate Harness Design

Date: 2026-08-08
Branch: `unified-candidate-harness-v0.8`
Frozen predecessor: `homotopy-order-policy-v0.6.0-alpha1`

## Goal

Place every currently implemented RODAS5P-related candidate under one scientific,
numerical, certification, work-accounting, threading, and failure-semantics contract.
Future BDF, Radau/IRK, Peer/W, SDC, Rosenbrock–Krylov, and Leja candidates must enter
through the same registry rather than creating method-specific benchmark paths.

## Protected scientific endpoint

For one RODAS5P step, the protected endpoint is the official eight-stage Rosenbrock–Wanner
stage system and embedded 5(4) update already implemented by `sequential_step`.
No candidate may alter the tableau, sign convention, mass-matrix convention, error scale,
or original target residual.

## Candidate tiers

### Tier L — identical stage equations, alternative linear algebra

- sequential/direct;
- sequential/GMRES;
- sequential/LGMRES;
- sequential/GCRO-DR with OFF, STAGE, and PERSISTENT lifetime policies.

This tier isolates linear-solver effects. All candidates consume the same sequential stage
right-hand sides.

### Tier N — alternative all-at-once stage propagation

- protected sequential/direct control;
- SABR5P with Forward, Explicit, Nilpotent, and block-GMRES solves;
- partial-coupling homotopy for all registered `(theta,q,rounds,predictor,correction)`
  configurations;
- homotopy with protected sequential fallback.

The primary Tier-N comparison uses the same direct common-`W` backend so nonlinear-stage
strategy is not confounded with a different Krylov implementation.

### Tier I — complete integrators

The registry reserves entries for BDF, Radau/IRK, Peer/W, parallel SDC,
Rosenbrock–Krylov/BOROK, and exponential/Leja methods. They are marked `DEFERRED` until a
Rust implementation satisfies the same step-output and work-ledger contract. Deferred
entries are visible in the catalog but cannot appear in numerical rankings.

## Unified result contract

Every executed candidate returns:

- candidate and family identifiers;
- case, trace, step, and tolerance identifiers;
- raw candidate stages and step output;
- embedded error;
- original RODAS stage residual;
- certification mode and certification outcome;
- fast acceptance, fallback, rejection, and numerical-failure state;
- complete `WorkCounters` delta;
- batch depth and vector work;
- elapsed compute time excluding serialization;
- output WRMS against the protected oracle in research mode;
- deterministic checksum.

A failed or uncertified result is never converted to success by fallback without recording
both the failed fast-path work and the fallback work.

## Certification hierarchy

### C0 — protected sequential oracle

Research-only reference. The protected sequential/direct step is computed once per case.
Every candidate is compared in the output direction and, when required, stage space.
This is the canonical false-acceptance oracle and resolves the present research-comparison
blocker. It is not a production fast certificate.

### C1 — first correction

Existing

\[
E_1=\|(b^T\otimes I)J_R^{-1}R(K)\|_{\rm WRMS}.
\]

It remains a diagnostic only because the v0.6 corpus demonstrated severe underestimation.

### C2 — second-correction diagnostic

With a fixed or refreshed target Jacobian,

\[
J_R\delta K_1=-R(K),\qquad
J_R\delta K_2=-R(K+\delta K_1),
\]

record

\[
\rho_{\rm out}=\frac{\|B\delta K_2\|}{\|B\delta K_1\|},\qquad
\rho_R=\frac{\|R(K+\delta K_1)\|}{\|R(K)\|},
\quad B=b^T\otimes I.
\]

Nonfinite values, residual growth, or ratios at least one force rejection/fallback.
The geometric-tail estimate is labelled empirical unless a contraction proof is supplied.

### C3 — refined-root reference

A safeguarded target-root refinement iterates exact target residual/Jacobian corrections to
strict residual and correction tolerances. The candidate-to-refined-root output difference
is used as an independent reference when sequential and all-at-once formulations are being
cross-checked. Failure to refine means uncertified, never accepted.

### C4 — future validated bound

Kantorovich/Krawczyk/radii-polynomial and output-aware triangular bounds remain catalogued
research lanes. They are not called rigorous until their sufficient conditions and floating-
point enclosure are actually implemented.

## Common acceptance gates

For research promotion a candidate configuration must satisfy on calibration and holdout:

1. oracle false acceptance count exactly zero;
2. global order five before the roundoff floor;
3. stiff-decay regression no worse than protected RODAS5P within declared tolerance;
4. nonnormal and noncommuting-mass screens without silent failure;
5. index-1 DAE gate when that candidate claims DAE support;
6. deterministic one-thread/four-thread scientific identity;
7. fallback-inclusive work and time reported;
8. at least one measurable advantage: lower scalar work, lower batch critical depth with a
   plausible concurrent backend, or lower measured wall time;
9. no ranking of deferred/unimplemented candidates.

## Work ledger

All candidates report at least

\[
N_f,\ N_{Jv},\ N_{W^{-1}v},\ N_{W\text{-batch}},\ N_{\rm fact},
\ N_{\rm Krylov},\ N_{\rm orth},\ N_{\rm cert},\ N_{\rm fallback},
\]

and

\[
T_{\rm setup},\ T_{\rm candidate},\ T_{\rm certificate},\ T_{\rm fallback},\ T_{\rm total}.
\]

Batch calls and vectors are distinct. One eight-RHS call is not counted as one scalar solve.
Serialization time is excluded from the primary compute metric and reported separately.

## Parallel execution

A local Rayon pool executes independent cases. The order inside one case, one timestep, and
one stage graph remains deterministic. Rows are canonically sorted before hashing or
serialization. One-, two-, and four-thread runs must have identical scientific fields.
Nested parallelism is disabled for the strict comparison unless explicitly declared.

## Scenario matrix

The first integrated screen covers:

- affine noncommuting mass oracle;
- scalar linear stiffness ladder;
- Prothero–Robinson stiffness/nonlinearity ladder;
- manufactured nonnormal vector systems;
- nonlinear noncommuting-mass systems;
- fixed-step global-order trajectories;
- rejected-step/fallback transactional cases.

Calibration and holdout are split by deterministic case IDs, not by rows from the same
trajectory.

## Blocker disposition targeted by this cycle

- **False-acceptance ambiguity:** resolved for research rankings by C0 and C3 references;
  C1/C2 remain diagnostics.
- **Unequal solver criteria:** resolved by the unified result, work, and gate contracts.
- **Linear/nonlinear confounding:** resolved by Tier L and Tier N separation.
- **Fallback hiding cost:** resolved by additive fast-path plus fallback accounting.
- **Thread nondeterminism:** resolved by local-pool execution and canonical sorting.
- **Candidate proliferation:** resolved by a registry with executable versus deferred status.
- **Production cheap certificate:** not falsely claimed; remains a separate future research
  problem unless C2/C4 passes the holdout gate.

## Non-goals

- no claim that every prior-art family is implemented;
- no production default change;
- no GPU/MPI speedup claim;
- no DAE claim beyond candidates already supporting the regular mass-matrix path;
- no BDF/Radau ranking without complete Rust implementations;
- no use of the protected sequential oracle as a claimed fast certificate.
