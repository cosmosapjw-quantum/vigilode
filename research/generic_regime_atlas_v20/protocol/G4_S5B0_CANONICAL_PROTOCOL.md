# G4-S5B0 Expanded Paired Regime Atlas — canonical protocol

Date: 2026-08-15 Asia/Seoul
Status: sealed before canonical output

## Purpose

Measure, without active switching, whether protected matrix-free RODAS5P (R-JF)
and fused full-MGS pexprb54s4 (E-K) exhibit economically meaningful and persistent
step-local complementarity on transition-bearing generic stiff ODE families.

## Frozen candidate paths

- Committed trajectory: protected sequential matrix-free RODAS5P.
- Read-only shadow: fused full-MGS pexprb54s4 on every accepted RODAS step.
- E-K is admissible only when its embedded/phi total-error estimate is finite and <= 1.
- No BDF/Radau source edits, no physical-client execution, no method switching.

## Corpus

Six transition-bearing atlas families:

1. Robertson-ramped
2. HIRES-ramped
3. van-der-Pol-ramped
4. rotating-nonnormal
5. nonautonomous-stiff-forcing
6. semilinear-advection-diffusion-ramped

Canonical dimensions: N=128 and N=512. N=2048 remains sealed.

These are scaled/replicated transition-bearing atlas variants, not exact work-precision
reproductions of the historical benchmark problems. This distinction is load-bearing.

## Numerical controls

- Rust 1.94.1, offline locked dependencies.
- Canonical tolerance: rtol=1e-5, atol=1e-7.
- R-JF GMRES: rtol=1e-10, atol=1e-12, restart=32, maxiter=256.
- E-K fused phi tolerance: 0.03*rtol relative, 3e-4*rtol absolute,
  full MGS, max Krylov dimension min(N+4,32), max 16 substeps.
- CPU affinity: CPU 0 when available.
- Seven independent campaigns; scientific/work fields must be identical after removing timings/failure strings.

## Residence labels

For each accepted RODAS step:

- E-K winner iff E-K is locally admissible and median E-K wall < median R-JF wall.
- Otherwise R-JF winner.
- A strong winner additionally requires >=5% median wall advantage.
- Consecutive same-winner accepted steps form a residence segment.

## Entry-adjusted oracle

Static control cost: sum of R-JF step wall.

Oracle cost: sum of the faster admissible method step wall, plus one measured E-K
prefix cost at the beginning of each E-K residence segment. R-JF is the protected default
and carries no entry-prefix charge.

The atlas continues to selector research only if all hold:

1. Both E-K and R-JF win at least one nontrivial residence segment.
2. At least one E-K residence has length >= 6 accepted steps.
3. Entry-adjusted oracle gain over static R-JF is >= 5% in aggregate.
4. Explicit Jacobian builds = 0, direct factorizations = 0, fast-path Newton = 0.
5. No active switching and no committed-state mutation by shadow E-K.

## Kill rule

If entry-adjusted oracle gain is <5% or favorable E-K residence is predominantly shorter
than six accepted steps, stop selector implementation and retain static protected R-JF.
Do not tune BDF/Radau, physical clients, or routine homotopy probes to rescue the result.
