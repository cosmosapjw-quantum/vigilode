# L1 literature report: matrix-free RODAS5P, Rosenbrock-W/K, inexact stage solves

**Answer to the key question:** stage solves can be made cheaper without losing order or stability only in two ways:
- the Krylov information you reuse leaves the solved operator `I − hγJ(y_n)` unchanged, or
- the solve tolerance is set from the local error tolerance TOL with a known amplification constant for this tableau.

Any change that swaps in another operator (a fixed small Krylov space, or a stale Jacobian) turns RODAS5P into a W-method or a K-method. Computed from the repo's own coefficients, RODAS5P then has only **W-order 2 (embedded: 1)** or **K-order 3 (embedded: 3)**. One published 5th-order method that tolerates an approximate Jacobian has been confirmed: Tsit5DA (Steinebach 2025), but it is only half-linearly-implicit and does not fit this use (§5). No 5th-order one-step Rosenbrock-W or K method was found.

**Access limits.** The egress proxy blocked WebFetch and curl to arxiv, Springer, SIAM, ScienceDirect, OSTI, ResearchGate, university sites and journals. Only GitHub raw files could be read in full.
- **Read in full:**
  - PETSc `rosw.c`
  - OrdinaryDiffEq.jl `algorithms.jl`, `derivative_utils.jl` and `rosenbrock_tableaus.jl`
  - SUNDIALS `Mathematics.rst` and `cvode_ls.c`
- **Everything else** comes from abstracts or excerpts returned by the search tool. These are tagged **[EXCERPT]**.
- **[COMPUTED]** marks results I calculated from `/home/user/vigilode/fixtures/rodas5p_coefficients_snapshot.json`. Scripts are in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/scripts/`:
  - `trees.py`: tree counts
  - `kcheck.py`: ROS/K/W order conditions; set `EMB=1` for the embedded method
  - `erk.py`: stability of the explicit part
  - `pr.py`: Prothero–Robinson order
  - `sens.py`: sensitivity of the step to stage-solve errors

---

## 0. Computed facts about RODAS5P [COMPUTED]

**Order-condition counts (cumulative through order p, autonomous trees):**

| Family | p=1 | p=2 | p=3 | p=4 | p=5 | p=6 |
|---|---|---|---|---|---|---|
| ROS (exact J) | 1 | 2 | 4 | 8 | 17 | 37 |
| K (Krylov, M ≥ p) | 1 | 2 | 4 | 9 | **23** | 62 |
| W (arbitrary T) | 1 | 3 | 8 | 21 | **58** | 166 |

The K and W counts at p ≤ 4 match the published figures: 9 for K at order 4, 21 for W at order 4, and "4 extra conditions at order 3" for W (Narayanamurthi et al. 2019; Tranquilli & Sandu 2014) [EXCERPT]. The p=5 values (23 and 58) are my own enumeration; **UNCERTAIN** until checked against Tranquilli–Sandu's order-5 list.

**RODAS5P checked against these conditions** (the ROS sanity check passes to order 5):

| | Main method | Embedded method |
|---|---|---|
| ROS order | 5 | 4 |
| K order (single Krylov space, Galerkin, M ≥ 4) | 3 (the one extra order-4 K-tree has residual 1.1e-3) | 3 |
| W order | 2 | **1** |

**Explicit part.** The explicit RK underlying (α, b) is only 2nd order, with real stability interval about [−2.0, 0].
- In a ROK-style use (component orthogonal to the Krylov space treated explicitly), any stiff mode the space misses is integrated with |hλ| ≲ 2.

**Prothero–Robinson** (φ = sin, fixed steps h = 0.1 to 0.00625):
- Observed order is about 5 for λ = −1.
- It drops to about 3 for λ ≤ −1e4.
- Error constants shrink like 1/|λ| (for example 5e-12 at λ = −1e6, h = 0.1).
- This fits Steinebach's stated goal of limiting order reduction to Rodas4P2's level.

**Sensitivity of the step to stage-solve errors.** Inject an error δ_i into the transformed stage variable u_i (Hairer–Wanner form) on the test equation y' = λy:

| z = hλ | Σ_i ‖∂y₁/∂δ_i‖ | Σ_i ‖∂err/∂δ_i‖ |
|---|---|---|
| −0.1 | 23.2 | 7.0 |
| −1 | 21.6 | 8.7 |
| −10 | 11.9 | 7.3 |
| −1e2 | 2.8 | 3.0 |
| ≤ −1e4 | 1.0 (only the last stage matters) | 2.0 |

- max |c_ij| = 165, but the resolvent damps it.
- In very stiff directions, solve errors in early stages are damped away; only the last stage's error matters.

---

## 1. Rosenbrock-Krylov (K-methods): Tranquilli & Sandu 2014 and follow-ups

**(a) Mathematics**
- Build one Arnoldi space per step: K_M = span{f_n, J f_n, …, J^{M−1} f_n}. The Jacobian is replaced by A = V Vᵀ J V Vᵀ.
- Stages split into an implicit part and an explicit part:
  - (I − hγH) λ_i = h φ_i + hH Σ γ_ij λ_j, with φ_i = Vᵀ F_i (small M×M solve)
  - k_i = V λ_i + h(F_i − V φ_i) (the orthogonal complement is explicit; M = 0 gives explicit RK)
- The order theory uses TK-trees, obtained by recolouring fat-rooted linear subtrees of TW-trees, via the identity A^k f = J^k f for k < M.
- Theorem: a type-1 ROK method has order p **iff M ≥ p** and the TK conditions hold. Theorem 3.7 covers M < p.
- Order conditions were given up to order 5. [EXCERPT]
- Source: Tranquilli & Sandu, *SIAM J. Sci. Comput.* 36(3):A1313–A1338, 2014, doi:10.1137/130923336; arXiv:1305.5481.

**(b) Robustness**
- ROK4a is L-stable and its embedded method is strongly A-stable, **but only with the full Jacobian**. With the Krylov approximation, "exact stability requirements … undetermined". A Wensch result suggests M must be about the number of stiff variables. [EXCERPT]
- Subspace adaptivity paper: stability does not carry over unless a "stage stability term" stays small. Two remedies are given:
  - control the stage residuals directly;
  - add the stage right-hand sides to the basis.
- Adding the stage right-hand sides helps more, and *why* it works is left open. [EXCERPT]
- Source: Tranquilli, Glandon & Sandu, *J. Comput. Appl. Math.* 385:113188, 2021, doi:10.1016/j.cam.2020.113188; arXiv:1910.02514.
- BOROK (Lanczos biorthogonalization) makes larger M affordable for stiff problems. [EXCERPT; venue UNCERTAIN]
- Source: Glandon, Tranquilli & Sandu, arXiv:1908.10531.

**(c) Cost evidence**
- M is independent of the system dimension. A 3072-variable shallow-water case needed about 8 basis vectors. [EXCERPT]
- Methane/GRI-3.0 chemistry: ROK4E with M = 4, 6, 8 was cheaper than BDF and RKDP for splitting steps of 10 ns to 1 µs, with 4th-order error decay. Speedup numbers not retrieved. [EXCERPT]
  - Source: Wu, Ma & Ihme, arXiv:1712.00953.
- CFD study: ROK was best for flows with few stiff modes; inexact solves in the other implicit schemes caused loss of convergence. [EXCERPT]
  - Source: Sarshar et al., *Computers & Fluids* 159:53–63, 2017; arXiv:1607.06834; author list UNCERTAIN.

**(d) Failure modes**
- Stiff modes outside K_M are integrated explicitly.
- Arnoldi cost is O(M²N), and the basis cannot be restarted without breaking the order theory.
- DAEs: the Krylov dimension must exceed the number of algebraic variables. [EXCERPT]
  - Source: Wensch, *Appl. Numer. Math.* 53:527–541, 2005, doi:10.1016/j.apnum.2004.08.012.
- The theory assumes a Galerkin (FOM-type) projection. A GMRES solve in the same space is a different approximation and is not covered.

**(e) Relevance to the matrix-free RODAS5P solver**
- Plugging ROK projection into RODAS5P coefficients gives **order 3** [COMPUTED]. A 5th-order ROK would need a new tableau satisfying about 23 conditions plus Rodas DAE and PR conditions. None was found published (**UNCERTAIN**).
- Gain: about M JVPs per step instead of a full GMRES solve per stage.
- Risks:
  - loss of order 5;
  - loss of index-1 DAE capability;
  - explicit treatment of stiff modes outside the space (RODAS5P's explicit part is stable only for |hλ| ≲ 2) [COMPUTED].
- Usable form: grow the space with stage right-hand sides and accept a stage **only when its true residual meets the TOL budget** (§3). That keeps exact-RODAS5P semantics.

## 2. Rosenbrock-W methods, Krylov-W, Jacobian reuse, AMF-W

**(a) Mathematics**
- Steihaug–Wolfbrandt: an arbitrary T replaces J. Conditions are indexed by TW-trees (meagre f-nodes, fat T-nodes). The meagre-only conditions are explicit-RK conditions on (α, b).
- Counts are given in §0. Source: Steihaug & Wolfbrandt, *Math. Comp.* 33:521–534, 1979. [EXCERPT]
- Available orders:
  - ROS34PW1a/1b/2/3: W-order 3, 4 stages, index-1 PDAEs. ROS34PW2 has R(∞) = 0; its embedded order-2 method has R(∞) = 0.48. [read: PETSc]
    - Source: Rang & Angermann, *BIT* 45:761–787, 2005, doi:10.1007/s10543-005-0035-y.
  - ROS34PRw: W, order 3, B_PR consistency order 3 (earlier ROS34PRW had B_PR order 2). [read: PETSc]
    - Source: Rang, *JCAM* 286:128–144, 2015, doi:10.1016/j.cam.2015.03.010.
  - RosenbrockW6S4OS: 4th-order L-stable W-method, fixed step only. [read: OrdinaryDiffEq]
    - Source: Rahunanthan & Stanescu, *JCAM* 233:1798–1811, 2010, doi:10.1016/j.cam.2009.09.017.
  - Rodas4P2 "is, with inexact Jacobians, a second-order W-method". Rodas4PW is a W-adaptation of Rodas4P; Steinebach, in preparation (2026); its order is **UNCERTAIN**. [read: OrdinaryDiffEq]
- Krylov-W: a "multiple Arnoldi" process over all stages preserves the order with low dimensions that depend only on the method coefficients. Stability "usually requires larger dimensions … adaptively". B-consistency order 2 for semilinear problems with constant linear part. [EXCERPT]
  - Source: Weiner & Schmitt, *Computing* 61:69–89, 1998, doi:10.1007/BF02684451.
- Automatic partitioning (linear autonomous case): using one Krylov family for all stages adds error of the size of the discretization error. Dimensions need only slightly exceed the number of "fast components with non-negligible contribution". [EXCERPT]
  - Source: Büttner, Schmitt & Weiner, *SIAM J. Numer. Anal.* 32:260–284, 1995, doi:10.1137/0732010; author list partly unconfirmed.
- ROWMAP: Krylov-W code built on ROS4, order 4 with small Krylov spaces, order-3 embedding, beat VODPK on large stiff tests. Numbers not retrieved. [EXCERPT]
  - Source: Weiner, Schmitt & Podhaisky, *Appl. Numer. Math.* 25:303–319, 1997.

**(b), (d) Robustness and limits**
- "Stability analysis for W-methods with arbitrary matrices is still an open problem" (Tranquilli et al. 2021). [EXCERPT]
- PETSc developer list (informal): a W-method "becomes unstable if you lag too much".
- For W-methods, order reduction on PDEs is more severe than for Rosenbrock methods. [EXCERPT]
  - Sources: Lang survey, arXiv:2002.12028; Lubich & Ostermann, *IMA J. Numer. Anal.* 15:555–583, 1995.
- OrdinaryDiffEq Jacobian reuse [read source]:
  - only for W-methods;
  - recompute J after any rejection, when |Δ(hγ)|/(hγ) > 0.03, or after 20 accepted steps;
  - **always recompute for Krylov `WOperator`** ("Krylov convergence depends on W quality, stale J … degradation");
  - never reuse for mass-matrix DAEs ("stale Jacobians cause order reduction for DAEs", citing Steinebach 2024).

**(c) Cost evidence**
- The main saving reported is fewer Jacobian evaluations and factorizations.
- Blom et al. 2016: ROS34PW2 was best at most tolerances but lost at small TOL because of its lower order. [EXCERPT]

**AMF-W**
- Optimal ℓ2/ℓ∞ bounds for 1-stage AMF-W methods with time step of order grid size. [EXCERPT]
  - Source: González-Pinto, Hairer & Hernández-Abreu, *SIAM J. Numer. Anal.* 58:1117–1137, 2020.
- AMF-W needs a directional or additive splitting of J. That is **not applicable to a black-box JVP operator** unless the problem exposes structure.

**(e) Relevance**
- With JVPs, a "stale Jacobian" saves nothing: a JVP costs the same at any base point. The only W-type saving is reusing *Krylov data* (V, H) across steps.
- RODAS5P is only W-order 2 (embedded 1) [COMPUTED], so any stale operator lowers its order. The silver lining: the order-1 embedded estimate responds to the Jacobian mismatch, so the step controller sees it and shrinks h rather than silently accepting the error. This follows from the order counts, not from a measurement.
- A real W path means switching tableau (ROS34PRw, RosenbrockW6S4OS, Rodas4PW) and giving up order 5.

## 3. Rosenbrock methods with inexact stage solves (tolerance rules)

**(a) Theory**
- No published rigorous rule for Rosenbrock was found. Blom et al. state that "no theory is available" on how to choose the tolerance. [EXCERPT]
- Two partial bounds exist:
  - the Krylov-W "same family of spaces" bound (Büttner et al.);
  - perturbation analysis.
- Derived here: in u-form, a stage solve error is e_i = hγ(I − hγJ)⁻¹ r_i, so with a dissipative J (μ(J) ≤ 0) the error is at most hγ‖r_i‖. The total error added to the step is then about C_y·max‖e_i‖, with C_y ≈ 23 for y₁ and about 9 for the error estimate [COMPUTED; scalar normal test equation; non-normal J gives larger constants].
  - Implied budget: ‖e_i‖_WRMS ≤ θ/C_y. With θ = 0.1 this is about 4e-3, i.e. ~TOL/250 in absolute terms.

**(b), (c) Practice**
- CVODE (read source): linear tolerance = C·ε_L·ε_N·ε, with ε_L = 0.05 and ε_N = 0.1, on a weighted (WRMS-scaled) preconditioned residual. That makes the linear tolerance about 0.005× the local-error constant.
- Blom et al.: TOL/100 for Rosenbrock + JF-GMRES. Findings:
  - RODASP was the most robust scheme;
  - Rosenbrock schemes went unstable at large steps ("nonlinear stability reduced vs ESDIRK");
  - up to about 3× fewer GMRES iterations than ESDIRK at large steps.
  - Source: Blom, Birken, Bijl, Kessels, Meister & van Zuijlen, *Adv. Comput. Math.*, 2016, doi:10.1007/s10444-016-9468-x. [EXCERPT]
- Wang & Yu: "ROW linear tolerance needs to be tight". ROW is easier to push into order reduction than ESDIRK.
  - Source: Wang & Yu, *J. Sci. Comput.* 83:39, 2020; arXiv:1904.04825. [EXCERPT]

**(d) Failure mode**
- A fixed small number of minimal-residual iterations turns the implicit scheme into an explicit (polynomial) one, so the step size must be limited for stability.
- Botchev, Sleijpen & van der Vorst therefore control h by stability.
  - Source: Botchev, Sleijpen & van der Vorst, *Appl. Numer. Math.* 31:239–253, 1999. [EXCERPT]

**(e) Relevance**
- The repo's `RODAS5P_INNER_RESIDUAL_HEURISTIC_FRACTION = 0.1` is documented as lacking a resolvent bound. The C_y ≈ 23 and C_err ≈ 9 constants give it a tableau-specific scale.
- Stiff directions damp early-stage errors (sensitivity → 0 for z ≤ −1e4) and shrink the true error per unit residual. An error-based stopping rule could therefore stop earlier than a residual-based one. This is a **hypothesis** to test, not a published result.
- Risk: non-normal J breaks the resolvent ≤ 1 bound.

## 4. RODAS5P itself (Steinebach 2023)

**(a) Design.** [EXCERPT]
- 8 stages, stiffly accurate (b_i = β_8i, b_8 = γ, α_8 = 1); 7-stage stiffly accurate embedded method.
- Order 5(4) on index-1 DAEs.
- Order reduction limited to that of Rodas4P2.
- A-stable embedded method (the Rodas5 embedded method is not A-stable).
- Dense output of order ≥ 4.
- Same construction route as Rodas4P and Rodas4P2.
- Rodas4P and Rodas4P2 satisfy PR conditions 48 and 49 yet show order 3 on Prothero–Robinson.
- Source: *BIT Numer. Math.* 63, 2023, doi:10.1007/s10543-023-00967-x.

**(c) Benchmarks.** "For higher accuracy Rodas5P always belongs to the best methods within the Rodas family" [EXCERPT]. Tables were not retrieved.

**(d) Limitations**
- RODAS5P is a ROW method, not W [read source: the OrdinaryDiffEq `is_W` flag is false for Rodas5P, Rodas5Pr and Rodas5Pe].
- PR order is about 3 in the stiff limit [COMPUTED].
- OrdinaryDiffEq issue #2132: Rodas5P was "over-optimistic" on a non-smooth mass-matrix problem (21 steps vs 103 for ROS34PRw). [EXCERPT]
- Follow-ups exist: Rodas5Pe (modified embedded scheme), Rodas5Pr (additional residual control), Rodas6P (6th order, arXiv:2511.21252). Source for Rodas5Pe and Rodas5Pr: Steinebach, JuliaCon Proceedings preprint, 2024. [read: docstrings; content EXCERPT]

## 5. Is there a 5th-order W- or K-method?

- **One-step W:** none found. The highest confirmed is order 4 (RosenbrockW6S4OS). W needs 58 conditions vs 17 for ROW [COMPUTED].
- **K:** conditions to order 5 were published; no 5th-order tableau was found (**UNCERTAIN**). It would need about 23 conditions [COMPUTED]. EPIRK-K reaches order 4 with M = 4 (exponential family).
  - Source: Narayanamurthi, Tranquilli, Sandu & Tokman, *J. Sci. Comput.* 78:167–201, 2019. [EXCERPT]
- **Two-step peer W:** a subclass of order s−1 with Krylov, matrix-free (Schmitt, Weiner & Podhaisky, *BIT* 45:197–217, 2005). Rosenbrock-type peer methods have stage order s−1 and beat RODAS at stringent tolerances (Podhaisky, Weiner & Schmitt, *Appl. Numer. Math.* 53:409–420, 2005). Whether s ≥ 6 gives a usable order-5 W-method: **UNCERTAIN**. [EXCERPT]
- **Tsit5DA:** order 5, but only half-linearly-implicit — explicit (Tsit5) for the differential variables, linear-implicit only for the algebraic block. It is not a fit for stiff ODEs.
  - Source: Steinebach, arXiv:2511.21252, 2025. [EXCERPT]

## 6. Shift-invariance and inexact-JVP side results (directly usable)

- With no preconditioner, K_m(I − hγJ, b) = K_m(J, b) holds exactly. The Arnoldi data is reusable for any new h when the right-hand side is collinear. Stage 1's right-hand side ∝ f(y_n) for every h, so **a rejected-step retry gets stage 1 free in JVPs**.
  - Source: Frommer & Glässner, *SIAM J. Sci. Comput.* 19:15–26, 1998. [EXCERPT]
- JVPs may be computed less and less accurately as GMRES converges, with computable bounds.
  - Source: Simoncini & Szyld, *SIAM J. Sci. Comput.* 25:454–477, 2003. [EXCERPT]

---

## Ranked shortlist for the matrix-free RODAS5P solver

1. **Tie the stage residual target to TOL using RODAS5P's own amplification constants** (C_y ≈ 23, C_err ≈ 9, plus a WRMS absolute target in the style of CVODE). This keeps exact order-5 semantics. It is consistent with CVODE's 0.005 factor and Blom's TOL/100, and removes over-solving where the current heuristic fraction is too strict, while bounding damage where it is too loose.
2. **Shift-invariant Krylov reuse on step rejection and h changes** (exact with no preconditioner). This makes rejections nearly free in JVPs, so a more conservative and robust step controller costs little, with no order or stability risk.
3. **One growing Krylov space per step, augmented with stage right-hand sides, with per-stage true-residual certification** (ROWMAP multiple Arnoldi / Tranquilli et al. 2021), falling back to GCRO-DR. This is the safe version of "small Krylov space once per step". It keeps ROW order because acceptance is by certified residual, not by K-theory, which would give only order 3 for RODAS5P.
4. **Hardening the error estimator (Rodas5Pe-style modified embedding and Rodas5Pr-style residual or interpolation control).** It targets the documented over-optimistic failure mode at the cost of a few extra function evaluations.
5. **Opt-in ROK4-type K-mode with adaptive M** for problems whose stiffness is low-rank (no DAEs). It can cut JVPs sharply, but stability depends on M covering the stiff modes, and the order drops from 5 to 4. It is worth having only behind a stability and residual guard.
6. **(Lowest) W-mode with Krylov data reused across steps.** RODAS5P is W-order 2 with a W-order-1 estimator. It would need a different tableau (ROS34PRw or RosenbrockW6S4OS) and order loss. Stale-operator stability is an open problem, and OrdinaryDiffEq deliberately disables reuse for Krylov operators.

---

## Sources

**Read in full (GitHub raw)**
- PETSc `src/ts/impls/rosw/rosw.c`: https://raw.githubusercontent.com/petsc/petsc/main/src/ts/impls/rosw/rosw.c
- OrdinaryDiffEq.jl `lib/OrdinaryDiffEqRosenbrock/src/algorithms.jl`
- OrdinaryDiffEq.jl `lib/OrdinaryDiffEqRosenbrock/src/rosenbrock_tableaus.jl`
- OrdinaryDiffEq.jl `lib/OrdinaryDiffEqDifferentiation/src/derivative_utils.jl`
- SUNDIALS `doc/cvode/guide/source/Mathematics.rst` and `src/cvode/cvode_ls.c`

**Excerpts only (via search)**
- https://epubs.siam.org/doi/10.1137/130923336
- https://arxiv.org/abs/1910.02514
- https://arxiv.org/pdf/1908.10531
- https://arxiv.org/pdf/1701.06528
- https://arxiv.org/pdf/1607.06834
- https://arxiv.org/pdf/1712.00953
- https://link.springer.com/article/10.1007/BF02684451
- https://epubs.siam.org/doi/10.1137/0732010
- https://www.sciencedirect.com/science/article/abs/pii/S0168927497000676
- https://www.sciencedirect.com/science/article/abs/pii/S016892740400128X
- https://link.springer.com/article/10.1007/s10543-005-0035-y
- https://link.springer.com/article/10.1007/s10543-023-00967-x
- https://arxiv.org/abs/2511.21252
- https://www.sciencedirect.com/science/article/pii/S0377042709006396
- https://link.springer.com/content/pdf/10.1007/s10444-016-9468-x.pdf
- https://link.springer.com/article/10.1007/s10915-020-01222-z
- https://ir.cwi.nl/pub/66/0066D.pdf
- https://www.unige.ch/~hairer/preprints/convergence-W.pdf
- https://arxiv.org/pdf/2002.12028
- https://link.springer.com/doi/10.1007/s10543-005-2635-y
- https://www.sciencedirect.com/science/article/abs/pii/S0168927404001278
- https://epubs.siam.org/doi/10.1137/S1064827596304563
- https://www.ams.org/mcom/1979-33-146/S0025-5718-1979-0521273-8/
- https://lists.mcs.anl.gov/pipermail/petsc-dev/2014-March/015044.html
- https://github.com/SciML/OrdinaryDiffEq.jl/issues/2132