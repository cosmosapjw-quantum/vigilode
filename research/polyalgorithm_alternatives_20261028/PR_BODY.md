## Why this investigation is needed

Several proposed polynomial and causal optimizations already exist in earlier
PRs, while the remaining costs concern execution direction, distinct-vector
fusion, physical-error planning, session reconstruction and unproved future
cost assumptions. This supplement reads the latest PR92 source and four supplied
papers critically, then maps concrete alternatives to the six remaining tasks.

Stacked on PR92. Baseline: `7eb2dd0ffda39afb2871d76700a720986ff09fe0`.
The initial remote survey found no newer PR. This is a **theory/design and
supporting-computation PR**, with no Rust backend or default-route change.

## New mathematical constructions

- **Fused backward Clenshaw:** five distinct degree-m polynomial inputs can
  produce one fused result with m rather than5m operator products. The exact
  local-defect transfer is a sum of basis polynomials applied to the local
  defects. Laguerre uses the proved exp(L/2) bound on its declared self-adjoint
  chart; Chebyshev uses1 on [-1,1]. New backward local-error enclosures are still
  required. Current forward signed-adjoint certification remains the comparator.
- **Cubic finite-step planning:** a chord-integral proof improves the midpoint
  constant1/8 to1/12. Positive rational decay sums use O(logN) planning operations.
  For k=1,g=1/2,Y=T=1,epsilon1e-5, the exact minimum dyadic grid changes512 to256;
  changing the constant alone still selects512. Root/update/clock prefix gates
  remain necessary.
- **Broader polyalgorithm choices:** exact-degree SPD Chebyshev resolvents,
  near-structure polynomial correction without a commutation assumption,
  numerical-range ellipse/Faber bounds, two-channel physical transport,
  carrier-specific causal endpoint budgets, and geometry-specific routes.
- **Continuation and adapter:** an invariant-ball station margin, full-session
  trace-preserving reuse, and a conditional residence/payback theorem. Generic
  branch/domain and future cost-variation certificates remain explicit gaps.

The exact Verlet example has two individually stable steps but an unstable
two-step product. Nonnormal and wrong-root counterexamples are also retained.
The supplied algebraic-geometry papers are not mislabeled derivative-free;
the DAE symmetry theorem is not extended beyond its stated linear scope.

## Reuse and qualification boundaries

Reuse PR68/69 coefficient machinery, the newer signed-adjoint Laguerre path,
guarded polynomial router, conditioned polynomial relations, shared coupled
factorization, finite-causal theorem and existing session implementation.
The current polynomial provider is dense symmetric; a matrix-free port needs
its own provider/action-error authority. Historical scaling failures and PR92
adverse measurements are unchanged.

The update contains17 candidates within the existing six PR-N tasks, preserves
the23-node/34-edge DAG, native suite argv and correction budgets, and refreshes
the local Codex handoff. Seven existing Q-* qualification tasks remain open.
No claim of new native speedup, generic nonlinear order, or production release
qualification is made.

## Actual verification

The three CHECK.py programs and protocol were first published at
`866668348f840bd43558859d2795576e292b6ca7`, then executed. All passed:

- Polynomial degree0--8 identities/all impulses, exact minimal degree28 example,
  and Jordan counterexample.
- 108 cubic algebra fixtures, exact grid decisions, continuation-ball/branch
  controls and conditional residence.
- Four geometry checks: full-block versus scalar-adjoint transport, nonlinear
  causal majorant, inexact invariant drift and geometric counterexamples.

Ledger rows **L-0174--L-0176** preserve commands, source and output hashes.
Earlier exploratory CAS checks remain labeled as such. One cross-lane review
clarified the L=0 residence boundary without changing any checker or runtime.

Artifact verification passes; the node checker passes173 ledger rows with the
old prefix preserved. Scoped authority checking passes65 citations/27 distinct
published commits after fetching the actual PR branches and repairing the
remaining shallow boundary. Initial failures are retained; no allowlist or
checker changed. These checks are separate from full repository CI.

The **12-page REVTeX PDF** was rendered and inspected; no overfull box or
undefined reference remains. Native Rust tests, solver benchmarking and
profiling were **NOT_RUN** in this theory supplement.

Start at `research/polyalgorithm_alternatives_20261028/REPORT_KO.md`,
`LOCAL_DEVELOPMENT_PLAN.json` and `LOCAL_CODEX_HANDOFF_KO.md`.
