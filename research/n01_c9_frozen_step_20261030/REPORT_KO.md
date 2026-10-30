# VigilODE C9 H1/H2: source-bound frozen native RODAS8 reference step

**Date:** 2026-10-30 KST. **PR:** https://github.com/cosmosapjw-quantum/vigilode/pull/131 (Draft, unmerged). Parent frozen PR129 HEAD `1771fe51f1dfbc2395ca575a99a9f749918e08d3`. Selected bounded stage: C9 scientific/implementation risk-repair, not C10 and not a separately authored independent C9 reviewer.

## 1. Scientific invariant and protected construction

The user's primary algorithm remains **causal, parallel, derivative-free RHS-only Newton-free accelerated homotopy** of Rosenbrock/RK stages. A supplied matrix-free JVP is *optional fallback*, never primary. An explicit dense 3×3 Jacobian serves only the original RODAS5P small correctness oracle.

For an autonomous Robertson mass-action source `f(a,b,c)=(-k1 a+k2 bc,k1 a-k2 bc-k3 b²,k3 b²)`, positive rates have units of inverse time and concentrations are dimensionless here. The source identity is `(private owner, k, accepted t0,y0, generation g)` and an original RODAS8 one-step proposal has `(h, tableau)`. Ordinary stage evaluation is `Q_g(t,z) -> f(z;k_g),J(z;k_g)` with **generation and accepted state unchanged**. In contrast, the legacy `n01_mass_action_owned_problem` callback calls `advance(t,z)`, so even a temporary time change increments the epoch, witnessed in PR129. This distinction is about source identity, not physical temporal accuracy.

Constructed `N01FrozenSourceProblem` privately holds an immutable `Arc<N01MassActionOwner>` and its actual native `OdeProblem`. RHS/Jac/JVP closures are built from clones of **the same Arc**. No public method accepts a separate owner, a replacement `OdeProblem`, or caller-supplied reaction rates when executing `N01FrozenSourceStep::original_rodas8_direct_reference`. Before/after the attempt, the session checks its owner-created opaque trial and exact bitwise t0/y0. Temporary RHS/Jac/JVP probes verify the source reference box with outward intervals and do not call `advance`; thus they preserve g for this single step. The positive-source singleton is an **immutable research reference**, not a versioned accepted-step production object.

Original first raw-U target is `(I-h gamma J0) U0=h gamma f(y0)`, where U0=gamma*K0 for the existing native sequential K tableau. For y0=(.75,.125,.125) and k=(.5,10,200), exact f(y0)=(-7/32,-93/32,25/8), whose components sum to zero (Python Fraction and Wolfram exact algebra). Real native 8-stage direct reference checks the same source and original first raw-U defect; no synthetic stage equivalence from an auxiliary BE root is assumed.

## 2. Actual native execution and results

- **GitHub Actions** https://github.com/cosmosapjw-quantum/vigilode/actions/runs/41006566540 , job `123902686470`, native original source HEAD `6c060d963fa3257dd982d09a562c639bbd63fa3e`, Rust **1.94.1**, Ubuntu **24.04**, **SUCCESS**.
- Test targets: new `n01_frozen_source_bundle_contract` **3/3**; existing C8/C9/F01/H1/H2 **8/8**; C6/C7 **9/9**; PR91 original causal/RHS-only homotopy **29/29**; all **49/49 real tests PASS**. Strict `cargo clippy --locked --offline` with `-D warnings` PASS, rustfmt source-focused PASS. No zero-selected test filter counted.
- Exact native RHS: `[-0.21875,-2.90625,3.125]` at t0 and t0+h/2. **model_epoch=0** before and after both source probes. Separate independently constructed zero-rate bundle gives `[0,0,0]`, never substituted into the true bundle's callback.
- One true original native RODAS5P **8-stage direct reference**: first full-space raw-U0 residual max `1.35525271560688054e-20`, endpoint mass defect recorded 0, eight linear reference solves, `jvp_calls=0` and source epoch unchanged. This is **first stage residual**, not an interval certificate for all eight original stages, not global IVP error and not solver acceptance.
- Negative tests: wrong initial clock including -0.0, carrier mismatch, zero/NaN/Inf step size, negative/out-of-box callback state, nonfinite temporary clock refuse and preserve output on failure.
- Historical first `run41005875127` scientific 49/49/clippy PASS but new module rustfmt RED. Second `run41006235574` scientific 49/49/clippy PASS, only test rustfmt RED. Each was diagnosed and repaired with whitespace-only edits, never changing physics/tolerances. Their logs remain on GitHub.

## 3. Reproduction and artifact byte identity

Exact changed Rust source bytes in [GitHub Actions source artifact 12955522545](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/41006566540/artifacts/12955522545) (23429-byte ZIP) and in this checkpoint's `sources/`, with `FILE_SHA256.txt` and actual `git hash-object` for three source files. The final GitHub native results were generated before subsequent report/metadata commits. Current-turn full archive `VERIFY.py` checks all source byte identities; the previous full PR129 evidence archive is preserved and CRC-verified. Local Python Fraction 1024/1024 replay of prior source-substitution counterexamples remains byte-identical. External SciSpace/Exa/Wolfram methodology searches are NOT independently authored C9 reviews.

## 4. What is and is NOT established

**Established only for this source-specific one-step reference:** native callback source and opaque trial cannot be independently swapped through this bundle's step interface; transient stage evaluations do not mutate the source generation; the original RODAS8 direct small-reference sample and original PR91 regression suite are executable and green. The private immutable `Arc` is the key Rust construction invariant.

**Open:** generic arbitrary `OdeProblem`/BASS physical33 attestation; time-dependent closures, accepted-step commit and source generation transition, rollback/rejection and concurrency/parallel stage ownership, exact eight-stage source-owned outward residual and positivity-tube existence proofs beyond the first-stage raw-U check, global truncation/IVP accuracy, matched achieved-error performance, and full production derivative-free 8-stage solver. Old `n01_mass_action_owned_problem` is unchanged and still has transient `advance()` semantics. No source-authoritative production gate is attached to this research path.

**Verdict:** `C9_H1_H2_FROZEN_STEP_PROTOTYPE=1/1 SCOPED_NATIVE_PASS`. `C9_DISTINCT_AUTHOR=0/1 NOT_RUN`; no actual native external reviewer tool was available. `C10=NOT_STARTED`, `PRODUCTION=HOLD`. Official original DAGs VigilODE **7/23**, REI **28/88**, successor admitted **0/12** remain unchanged. No merge, force push or tolerance relaxation.
