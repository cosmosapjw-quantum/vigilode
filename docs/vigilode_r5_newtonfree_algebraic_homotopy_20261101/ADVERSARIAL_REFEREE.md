# R5 hostile same-author review and evidence gate

## Strongest viable assertion
For the *specific frozen original cooperative dissipative cubic source* and a diagonal implicit BE/RK stage, a coupling-strength homotopy from separable cubic inverses to the original coupled nonlinear stage has exactly one positive real root at each λ∈[0,1], no singular real-branch crossing, monotone increasing continuation state, and a global ∞-norm residual-to-root bound with constant mλ. See THEORY_R5.md for complete direct proof.

Original Rodas5P is linearly implicit, so its stage roots must be checked in the ORIGINAL U-form W U_i = r_i rather than the auxiliary BE nonlinear fixed point. A source-owner-accessible frozen cubic Jacobian permits an analytically represented M-matrix action with no runtime JVP callback for this limited source class; this is not a general unknown-RHS derivative-free method. The original first U-stage independent source-based calculation is not a native full 8-stage run.

## Attack and result
1. Wrong stage target: theorem A is on BE/diagonal RK, not original 8-stage RODAS. Prevented by separate theorem B and first U-stage raw residual calculation. Complete eight-stage native parity remains NOT_RUN.
2. False derivative-free marketing: deriving W from known polynomial source implicitly uses the derivative formula 3g y_n². The runtime test calls no Jacobian/JVP/AD/FD/Newton, but generic unknown source cannot construct W. Wording narrowed to callback-derivative-free source-specialized provider.
3. Singular/negative coefficients: negative g yields cubic 2z-z³ with THREE real roots; negative c fails source positivity. Both are retained explicit negative tests. The theorem only covers k>0,g>=0,c>=0,lambda∈[0,1],tau>0.
4. Complex paths: nonsingular real M-matrix guarantees no *real* singular branch crossing; does not prove no complex critical points or full algebraic homotopy path completeness. Scope explicitly narrowed.
5. Sufficient but non-sharp residual error: the lower bound mλ was initially approximated by binary64; that could overstate rigorous lower bound. REPAIRED by computing exactly as a Fraction of frozen represented inputs, comparing the computed rational bound against the up-rounded stored output in test_global_nonlinear_source_residual.
6. Root isolation: cubic inverses evaluated by monotone bisection, no Newton; when g=0 a plain division is now used, not 55 wasted cubic evaluations. This preserves the exact linear control.
7. Cost adversary: in every predeclared nontrivial source, extra homotopy stations require MORE scalar evaluations than direct λ=1 Newton-free Jacobi. Therefore NO speedup is claimed, and homotopy is rejected as a faster option on these specific cases. The strong protected RODAS route remains the intended fallback for a full solver; no native fallback execution occurred in this theory-only unit.
8. Source identity: Rust original cubic BE, full WP1 fixture, raw-U RODAS native candidate and reusable RHS homotopy source identities verified by Git blob SHA; original RODAS tableau SHA256 checked. Refined asymmetric_2 is one resolution control not an additional physical source.
9. Non-source-bound BDF/Radau/geometry: the proof does not transfer to arbitrary nonnormal J, signed exchange, mass matrices, or full general nonlinear ODEs. No fresh strong no-go theorem is asserted.
10. Exact-source vs native floating bound: Fraction eval at represented coefficients certifies a mathematical source-conditioned residual, but callback rounding, clock and requested-output global uncertainty are not enclosed; distinguish exact-real bracket from native interval production certificate.
11. Authority CI: restored 12 genuinely absent original PR synthetic-test-merge SHAs and reanchored 3 already-existing PR101–103 SHAs, all with live source-ref+archive-ref SHA readback. 20 of 32 previously missing unique ids still unresolved; full CI green NOT_VERIFIED. Existing strict checker, allowlist and source provenance unchanged.
12. Novelty: M-matrix monotonicity and polynomial discriminant are standard mathematical tools. Original contribution here is project-specific mapping and evidence separation, not established external publication novelty.

## Method and checks
- Constructor: same model, isolated Git branch, prospective preregistration pushed before new simulation.
- Adversarial review: same author, 12 Python assertions, including independent exact rational residues and branch monotonicity, negative g/c/invalid-source tests. Independent distinct-author C9 NOT_RUN.
- CAS: Wolfram Language symbolic Discriminant[a x^3+d x-b,x] = -a(4d³+27ab²) verified. Numerical source data 20 homotopy λ stations across original 4 fixtures (three distinct physical families), four source-conditioned original RODAS first U-stage checks.
- No native full-IVP run, order-five convergence or same-achieved-global-error wall speedup; no production route altered.

## Positive next action / no process recursion
Next genuine science unit is 8-stage source-bound raw-U same-runner study at fixed G0 inputs, with M2 causal critical-depth, full attempted work and protected fallback, followed by M1 full-IVP matched-achieved-error. This is a NEW science execution, not another correction to the completed R5 source-scoped theorem. Frozen official DAG remains 7/23, successor 0/12, production HOLD. The historic 20 missing SHAs are a separate bounded provenance workstream; do not hold R5 research results hostage to unrelated old citations and do not fabricate source objects.
