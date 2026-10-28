# L3 literature report: certified matrix-free exponential integrators, stage-system preconditioning and recycling, and mixed precision for a JVP-only RODAS5P solver

## 0. How to read this report

**Access limitation.** WebFetch was blocked by the egress proxy for arxiv.org, SIAM, Elsevier, Springer, PMC, NSF-PAR, KIT and OpenAlex. Only GitHub was reachable. Almost all literature claims below therefore come from search-engine abstracts and snippets. Each claim is tagged:
- **[SRC]**: I read the primary text or code.
- **[ABS]**: I saw only the abstract or a search snippet of the primary source.
- **[SEC]**: I saw only a secondary source restating it.
- **[MEM]**: standard result cited from memory and not fetched this session.
- **[DERIV]**: my own derivation, not taken from any source.
- **UNCERTAIN**: needs checking against the full text.

**Local facts I checked in the repo:**
- The RODAS5P tableau has s = 8 stages and γ ≈ 0.211938 (`/home/user/vigilode/research/adversarial_audit_20260927/experiments/E-07/coeff_f64.json`).
- The production stage loop in `/home/user/vigilode/crates/rodas5p-integrators/src/sequential.rs` (around lines 349–358) solves the K-form `(I − hγJ) k_i = h f(Y_i) + h J(Σ_{j<i} γ_ij k_j) + h² γ_i f_t`. That costs one extra JVP per stage i ≥ 2, so 7 per step. `research/.../fixes/G2_inner_forcing_rule.json:88` documents this.
- The coefficient snapshot also carries `c_matrix_input` and `a_input`, the Hairer–Wanner transformed-form coefficients (`/home/user/vigilode/crates/rodas5p-core/src/coefficients.rs`).

---

## 1. Baseline cost model and spectral scaling

These scalings are the reference point for every comparison below.

**Baseline cost.** Unpreconditioned RODAS5P costs Σ_{i=1..8} m_i GMRES JVPs per step, plus 7 K-form coupling JVPs, plus any certification JVPs. All 8 stage matrices are the same W = I − hγJ within a step, because the diagonal is a single γ (local tableau).

**GMRES on W, by spectral regime:**
- **Dissipative J (μ₂(J) ≤ 0).** The field of values of W lies in Re z ≥ 1, so ‖W⁻¹‖₂ ≤ 1. The GMRES residual is then a certified bound on the 2-norm error [DERIV].
  - Elman's bound gives ‖r_k‖ ≤ (1 − ν(W)²/‖W‖²)^{k/2}‖r₀‖ [SEC: Liesen–Tichý arXiv 1211.5969; Beckermann–Goreinov–Tyrtyshnikov, improved rate].
  - The worst case under field-of-values information alone is k = O((hγ‖J‖)² log 1/ε) [DERIV].
- **Normal J with real spectrum in [−ρ, 0].** k = O(√(hγρ) log 1/ε), by the Chebyshev bound on [1, 1+hγρ] [DERIV/MEM].
- **Disc-shaped or imaginary-axis spectrum of radius ρ.** k = O(hγρ log 1/ε) [DERIV/MEM].
- **μ₂(J) > 0 but J stable.** Embree builds a Lyapunov-weighted inner product in which Elman's bound applies again, at the cost of a constant [ABS: Embree, "Extending Elman's bound for GMRES", arXiv 2312.15022]. This is the theoretical counterpart of the solver's Osborne/diagonal metric.

**Polynomial Krylov for exp and φ actions:**
- Hochbruck & Lubich 1997 (SINUM 34(5):1911–1925, doi 10.1137/S0036142995280572) [ABS + SEC]:
  - For a Hermitian negative semidefinite matrix with spectrum in [−4ρ, 0], the error is small once m ≳ √(ρτ).
  - Convergence becomes superlinear once m ≳ τ‖A‖.
  - The exact constants are UNCERTAIN; the secondary restatement carries a (4ρτ)²/m² prefactor.
  - The abstract variant says convergence "is faster than that of corresponding Krylov methods for linear equations".
- Hochbruck, Lubich & Selhofer 1998 (SISC; KIT preprint): Krylov approximations of exp/φ "typically converge faster than those for the solution of the linear systems arising in standard stiff integrators" [ABS].

**Consequence [DERIV].** For advective or imaginary spectra, both unpreconditioned implicit and polynomial-exponential Krylov cost about ρ per unit time, independent of h. Large steps then buy nothing over explicit methods. For dissipative real spectra, cost per unit time falls like h^{−1/2}. This matches the advection-dominated studies cited in §2.4.

---

## 2. Family (i): matrix-free exponential integrators with robust φ actions

### 2.1 φ-action kernels

**(a) Core mathematics and guarantees**
- **phipm** (Niesen & Wright, ACM TOMS 38(3), 2012, Algorithm 919; arXiv 0907.4631) [ABS]. Arnoldi or Lanczos projection plus internal time-stepping. Both the substep τ and the Krylov dimension m adapt to an error estimate, which is heuristic rather than a bound.
- **KIOPS** (Gaudreault, Rainwater & Tokman, JCP 372:236–255, 2018, doi 10.1016/j.jcp.2018.06.026; corrigendum JCP 441:110443, 2021) [ABS/snippet]:
  - One exponential of an augmented matrix (Sidje-type) evaluates the whole linear combination of φ_k.
  - The basis uses incomplete orthogonalization, down to about 2 inner products per iteration.
  - τ and m are adapted by a new controller. The error estimate is heuristic.
- **expmv** (Al-Mohy & Higham, SISC 33(2):488–511, 2011) [SRC: code at github.com/higham/expmv]:
  - Truncated Taylor series with scaling, with parameters chosen by backward-error analysis.
  - `normAm.m` estimates ‖A^m‖₁ with `normest1`, which calls both `A*X` ('notransp') and `A'*X` ('transp').
  - It shifts by μ = trace(A)/n.
  - Tolerance tables exist for double, single and half precision.
- **Leja interpolation** (Caliari, Kandolf, Ostermann & Rainer, SISC 2016, doi 10.1137/15M1027620) [ABS]. A backward-error analysis sets the scaling, the interpolation points and the degree for a prescribed accuracy.
- **Matrix-free Leja** (Deka & Einkemmer, ApJS 2022, doi 10.3847/1538-4365/ac5177; arXiv 2108.13622) [ABS]:
  - JVPs come from forward finite differences.
  - The spectral bound comes from power iteration with a safety factor, refreshed every n steps.
  - LeXInt sets the left interval end to minus the power-iteration magnitude, which assumes a real-axis spectrum [ABS].
- **BAMPHI** (Caliari, Cassini & Zivcovich, JCAM 423:114973, 2023) [ABS]. Newton interpolation at "extended Ritz values", backward-stable, and explicitly matrix-free and transpose-free.
- **phimv** (Al-Mohy, arXiv 2509.26475, 2025) [ABS]. Scaling-and-recovering truncated Taylor series with a spectral shift and scaling chosen by a power-based objective. Whether it needs adjoint products is UNCERTAIN.
- **Certified a posteriori bounds:**
  - Jawecki, Auzinger & Koch (BIT 60:157–197, 2020, doi 10.1007/s10543-019-00771-6) [ABS]. The integral of the scalar Krylov defect is a rigorous upper bound on the exp/φ error when μ₂(σA) ≤ 0. It is computable inside the Krylov space and asymptotically correct as t → 0.
  - Botchev, Grimm & Hochbruck (SISC 35(3):A1376–A1397, 2013) [ABS]. An ODE residual interpreted as a backward error, giving reliable stopping and a Richardson-type restart.
  - Krieger & Schweitzer (ETNA 65:414–440, 2026; arXiv 2510.17538) [ABS]. A general Krylov ODE-residual framework covering rational and sketched Krylov.
  - Liu & Schweitzer (arXiv 2607.17819, 2026) [ABS]. The field of values of KIOPS-type augmented matrices grows with the operand-vector norms, so only "very pessimistic" bounds exist for them. The Al-Mohy–Liu block formulation has a vector-independent field of values and yields sharper bounds.
- **Rational / shift-and-invert Krylov:**
  - van den Eshof & Hochbruck (SISC 2006, doi 10.1137/040605461) [ABS]: error bounds independent of ‖A‖.
  - Bergermann & Stoll, rk2expint (arXiv 2303.09482) [ABS]: iteration counts stay roughly flat as the spectral radius grows.

**(b) Reported robustness**
- KIOPS beat phipm in accuracy and efficiency in all of its authors' tests [ABS].
- On a 108-matrix suite including highly nonnormal matrices (Al-Mohy 2025) [ABS]:
  - t = 0.1: KIOPS degrades and phipm "breaks down badly".
  - t = 1: only phimv and BAMPHI stay reliable; KIOPS and phipm are "unusable".
- Deka, Tokman & Einkemmer (arXiv 2211.08948) [ABS] found no uniformly best kernel–integrator pair; performance depends on how the two interact.

**(c) Reported cost**
- KIOPS vs phipm: 5–7× faster at fixed tolerance (KIOPS paper Figs 3(e), 4(e); tol 1e-14; m_min = 10, m_max = 128). With KIOPS, 4th-order EPIRK/EXPRB schemes need 2 calls per step and 5th-order schemes 3 calls [snippet].
- Leja vs Krylov:
  - Leja wins in an operation-count model, especially when inner products are expensive (arXiv 2410.12765) [ABS].
  - In CPU time, Krylov wins at small steps and Leja at large steps (arXiv 2512.03679) [ABS].
- rk2expint: wins only at large spectral radii. Its shifted linear solves take 60–90% of runtime and are preconditioned (tol 1e-7) [snippet].

**(d) Failure modes**
- expmv and the Leja backward-error machinery need norm estimates. expmv specifically needs Aᵀ products and trace(A), neither of which a JVP-only solver has [SRC].
- Power-iteration spectral bounds are not certified and assume a real interval.
- Augmented-matrix field-of-values bounds are pessimistic.
- The Jawecki bound requires μ ≤ 0 in the inner product actually used.
- Rational Krylov needs good solvers for (I − γhJ), which is exactly the RODAS bottleneck.

**(e) Relevance to the solver**
- **Do:** keep Krylov φ actions with defect-based certified bounds. Prefer the block formulation (vector-independent field of values) for any a priori guard.
- **Do:** run Arnoldi in the diagonal (Osborne) metric D where μ_D(hJ) ≤ 0, so the Jawecki bound certifies in the D-norm [DERIV; that the theorem transfers to a weighted inner product is UNCERTAIN but follows formally].
- **Avoid:** expmv, which is not JVP-only, and rational Krylov without a preconditioner.
- **Synergy [DERIV]:** RODAS stage 1 (rhs ∝ f(y_n) + hγ₁f_t) and the exponential Rosenbrock first stage φ₁(hJ)f(y_n) live in the same space K_m(J, [f(y_n), f_t]). One Arnoldi decomposition J V_m = V_{m+1}H̄_m gives:
  - exact GMRES for stage 1 at any h, through min ‖βe₁ − (Ĩ − hγH̄_m)y‖;
  - the φ approximation with its defect bound.

  The guarded exponential candidate's first projection is therefore free once RODAS stage 1 has been solved, or the reverse.

### 2.2 Exponential Rosenbrock and EPIRK

**(a) Core mathematics and guarantees**
- Exponential Rosenbrock (Hochbruck, Ostermann & Schweitzer, SINUM 47:786–803, 2009, doi 10.1137/080717717) [ABS]:
  - Linearize at u_n with the exact J_n; the remainder g_n then has zero derivative at u_n [DERIV/MEM].
  - Stiff convergence is proven in a semigroup framework with variable steps up to order 4.
  - exprb43 uses 3 stages with an embedded 3rd-order estimate.
- Luan & Ostermann (JCAM 255:417–431, 2014) [ABS + SEC]: stiff order theory for arbitrary order. Order 5 needs 4 conditions, against 16 for exponential Runge–Kutta with a fixed linearization.
- EPIRK (Tokman 2006, 2011) [MEM]: built to minimize the number of Krylov projections per step.
- Loffeld & Tokman (SISC 36(5):C591–C616, 2014) [ABS]: EPIRK5P1 uses 3 projections per step.

**(b) Robustness.** The methods are exact for linear constant-coefficient problems, and the stiff order theory is clean [ABS]. All of it depends on the exact Jacobian.

**(c) Cost.** See §2.4.

**(d) Failure modes**
- An inexact J or a truncated Krylov solve breaks g_n′(u_n) = 0 and reduces order. Narayanamurthi et al. state that "early truncation … is equivalent to the use of an approximation of the Jacobian … may suffer from order reduction" [ABS].
- Finite-difference JVPs make J inexact at the √u level.

**(e) Relevance.** Use exprb43 or EPIRK5-class schemes as the guarded candidate, at 2–3 φ-combination calls per step against 8 RODAS stage solves. Certify each call with a defect bound so that "exact Jacobian" stays operationally true.

### 2.3 Exponential-W and exponential-K variants (approximate Jacobian)

**(a) Core mathematics and guarantees**
- EPIRK-W / EPIRK-K (Narayanamurthi, Tranquilli, Sandu & Tokman, J Sci Comput 78:167–201, 2019, doi 10.1007/s10915-018-0761-3) [ABS]:
  - W: any Jacobian approximation is allowed; the practical method is order 3.
  - K: a specific Krylov projection V V^T J V V^T is used; the practical method is order 4.
- expK (Tranquilli & Sandu, JCP 278:31–46, 2014) [ABS]: one Krylov space per step. The basis size needed for order depends only on the order, not on the ODE.
- Hochbruck, Lubich & Selhofer 1998 proposed the exponential-W class [SEC via arXiv 1401.2125].

**(b) Robustness.** Order is guaranteed independent of the Krylov dimension M. Stability conditions are not established (§3.2).

**(c) Cost.** Reported as favorable against a representative EPIRK and a Rosenbrock–Krylov scheme; no numbers were seen [ABS].

**(d) Failure modes.** Many more order conditions (W). Stiff-mode stability needs M of the order of the number of stiff modes.

**(e) Relevance.** EPIRK-K is the exponential analogue of item 6 in §6: it legitimizes small-M φ actions without exact-J order loss, but must be gated by residual or stability checks.

### 2.4 Exponential vs implicit integrators at matched error

- Loffeld & Tokman (JCAM 241:45–67, 2013, doi 10.1016/j.cam.2012.09.038) [ABS]. Krylov-based EPIRK compares favorably with unpreconditioned Newton–Krylov implicit methods. The savings are attributed mainly to faster Krylov convergence for φ(hJ)v than for the implicit linear systems.
- Einkemmer, Tokman & Loffeld 2017, magnetohydrodynamics (arXiv 1604.02614) [snippet]:
  - EPIRK5P1 is about 2× faster than CVODE at loose tolerances (≥ 1e-4 or 1e-5; the snippets disagree).
  - CVODE is "significantly more efficient" at tight tolerances.
- Deka & Einkemmer, ApJS 2022 [ABS]: matrix-free Leja exponential integration outperformed Krylov exponential integrators, explicit methods, and CVODE on reconnection and Kelvin–Helmholtz tests.
- Advection-dominated problems (arXiv 2410.12765; AAMM 2026) [ABS]: exponential integrators are only comparable to explicit Runge–Kutta, and better only when part of the domain is diffusion-dominated.

**Takeaway.** The exponential advantage is regime-dependent: diffusion- or real-dominated spectra and loose-to-moderate tolerances. This argues for per-step method selection by cost rather than wholesale replacement.

---

## 3. Family (ii): preconditioning and reuse for the stage systems (I − hγJ)

### 3.1 JFNK and lagged or frozen preconditioners

**(a) Core.** Knoll & Keyes (JCP 193(2):357–397, 2004, doi 10.1016/j.jcp.2003.08.010) [ABS]: Jacobian-free Newton–Krylov succeeds only with adequate preconditioning; finite-difference perturbation size trades truncation against rounding error.

**(b, c) Matrix-free preconditioner updates and Rosenbrock practice**
- Duintjer Tebbens & Tůma (NLAA 17(6):997–1019, 2010) [ABS]: two fully algebraic matrix-free updates of a factorized preconditioner. They "often outperform" both recomputing and freezing.
- Blom et al. (Adv Comput Math 42:1401–1426, 2016, doi 10.1007/s10444-016-9468-x) [ABS]: Rosenbrock with preconditioned Jacobian-free GMRES, inner tolerance TOL/100. RODASP was the most dependable method; W-methods such as ROS34PW2 tolerate approximate J.

**(d) Failure modes.** A JVP-only setting has no matrix to factor. Every option needs either problem structure or one assembled Jacobian.

**(e) Relevance, conditional.** If the sparsity pattern is known (the banded pipeline exists), a colored compressed Jacobian costs about c JVPs, where c is the number of colors [MEM: Curtis–Powell–Reid 1974; Gebremedhin–Manne–Pothen, SIAM Review 2005].
- A lagged LU or ILU of I − hγJ_old then serves as a right preconditioner, so the true residual stays certifiable.
- This is the single largest potential JVP cut for PDE-like stiffness, but it departs from the "no preconditioner" design.

### 3.2 Krylov-W, ROWMAP and Rosenbrock–Krylov (method-level fusion)

**(a) Core mathematics and guarantees**
- Krylov-W:
  - Schmitt & Weiner, Appl Numer Math 18:307–320, 1995 [ABS].
  - Weiner & Schmitt, Computing 61:69–89, 1998, doi 10.1007/BF02684451 [ABS]: a multiple Arnoldi process across all stages preserves the order of the implicit scheme with low Krylov dimensions; the minimal dimensions depend only on method parameters.
  - ROWMAP (Appl Numer Math 25:303–319, 1997) [ABS]: order 4 with "fairly low" dimensions, about 4–15 according to a citing paper [SEC]; default maximum 70 [snippet of the Fortran source].
- Rosenbrock–Krylov (ROK) (Tranquilli & Sandu, SISC 36(3):A1313–A1338, 2014; arXiv 1305.5481) [ABS]:
  - One Arnoldi space K_M(J, f(y_n)) serves all stages.
  - Far fewer order conditions than W-methods, and M depends only on the order.
  - ROK4a: 4 stages, order 4, L-stable for the exact J.

**(b) Robustness and failure modes**
- Stability with a Krylov-projected J is "unresolved". A result attributed to Wensch says M must be about the number of stiff variables [SEC].
- For index-1 DAEs, M must exceed the number of algebraic variables (Wensch, Appl Numer Math 53, 2005) [SEC].
- Fixes in Tranquilli, Glandon & Sandu (JCAM 385:113188, 2021) [ABS]:
  - direct residual control with adaptive M;
  - extending the basis with the stage right-hand sides f(Y_i), reported as the more effective of the two.
- BOROK (Glandon et al., Appl Numer Math 2020) [ABS] uses Lanczos biorthogonalization to afford larger M. Basis extension doubled matvecs in their test and did not pay off unless JVPs are expensive [snippet].

**(c) Cost.** Favorable against Rosenbrock and Rosenbrock-W schemes [ABS]. ROWMAP was efficient against VODPK [ABS]. No numbers were seen.

**(e) Relevance**
- These are the theoretical license for item 2 of §6.
- Unlike ROK, the solver can keep RODAS5P's exact-J order conditions by converging each stage to a certified residual, while still reusing the basis across stages [DERIV].
- Adopting ROK4a or a ROWMAP-type scheme as a cheaper candidate means going down to order 4, and needs residual-gated M because stability is not guaranteed.

### 3.3 Shift invariance, shifted systems and recycling

**(a) Core**
- K_m(I − hγJ, b) = K_m(J, b). Unrestarted GMRES for any shift reuses one basis through a small least-squares problem [DERIV/MEM].
- Restarted shifted GMRES needs collinear residuals (Frommer & Glässner, SISC 19, 1998, doi 10.1137/S1064827596304563) [ABS/SEC].
- Recycling for shifted families (Soodhalter, Szyld & Xue, Appl Numer Math 2014, doi 10.1016/j.apnum.2014.02.006) [ABS]: one recycled subspace for all shifts cannot be combined with storage independent of the number of shifts.
- GCRO-DR (Parks et al., SISC 28(5):1651–1674, 2006) [ABS]: recycles subspaces across general slowly changing sequences.

**(c) Cost.** rGCROT used 30 matvecs per step against 100 for GMRES(50) on a CFD sequence (Amritkar et al., JCP 303:222–237, 2015) [snippet].

**(e) Relevance [DERIV]**
1. **Within a step W is identical in all 8 stages.** Augmenting stage i with all previous stage Krylov vectors U costs no extra JVPs, because W U = V_{m+1}(Ĩ − hγH̄) is already known from the Arnoldi relation. This is exact recycling, unlike Ritz-subset GCRO-DR.
2. **After a step rejection with the same y_n and J**, the stage-1 space K(J, [f(y_n), f_t]) is h-independent. The retry's stage 1 costs zero JVPs, and so does probing the cost or residual at a candidate h.

### 3.4 Spectral-deflation and low-rank quasi-Newton preconditioners

**(a) Core.** Deflation preconditioners map selected eigenvalues to fixed values:
- Erhel, Burrage & Pohl (JCAM 69:303–318, 1996) [ABS/SEC; implemented as PETSc KSPDGMRES];
- Baglama et al. (SISC 20:243–269, 1998);
- Burrage & Erhel (NLAA 5:101–121, 1998) [metadata only].

**Shift-updatable form [DERIV].** Let U be an orthonormal basis of a J-invariant subspace (Schur or Ritz vectors) and T = U*JU. Define M_h⁻¹ = I + U[(I − hγT)⁻¹ − I]U*.
- In the basis [U, U⊥], W M_h⁻¹ = [[I, −hγU*JU⊥], [0, I − hγU⊥*JU⊥]].
- The k deflated eigenvalues become exactly 1, and the rest of W's spectrum is unchanged.
- A new h or γ needs only a new k×k solve, with no JVPs.
- The same U gives exact φ(hJ)U = Uφ(hT), which deflates the exponential path as well.
- With right preconditioning the true residual is still computed, so certification is preserved.

**Quasi-Newton updates**
- Broyden rank-one updates of ILU or AINV preconditioners for inexact Newton (Bergamaschi, Bru, Martínez & Putti, ETNA 23:76–87, 2006) [ABS].
- L-BFGS preconditioning for SPD sequences from matvec information only (Morales & Nocedal, SIOPT 10(4):1079–1096, 2000) [ABS].
- Updates for shifted SPD sequences (A + αI) via LDLᵀ (Bellavia et al., SISC 33(4):1785–1809, 2011) [ABS].

**(d) Failure modes**
- Gains only when stiffness sits in k ≪ n separated modes; continuum spectra such as diffusion would need k = O(n).
- Under non-normality the coupling block remains [DERIV].
- Stale U as J drifts.
- Broyden updates built from Krylov secant pairs carry roughly the same information as augmentation [DERIV].

**(e) Relevance.** This is the most promising preconditioner that fits the solver's constraints: it uses only JVP-derived data, is h-reusable, certifiable, and shared with the exponential candidate.

### 3.5 Polynomial preconditioning

- Loe & Morgan (NLAA 29(4):e2427, 2022, doi 10.1002/nla.2427) [snippet]:
  - one test: daxpys −90%, dot products −94%, matvecs +1%;
  - higher degrees can also cut matvecs on hard problems.
- Ahmad, Szyld & van Gijzen (SIMAX 38(2):401–424, 2017) [ABS]: polynomials can be chosen so that shift invariance survives.

**Relevance.** It mainly cuts orthogonalization cost, not JVPs. Low value when f is expensive; useful for large m, GPUs, or low-synchronization settings.

### 3.6 Formulation level: transformed RODAS stage variables [MEM + local code]

- **Mathematics.** In the Hairer–Wanner transformed form, u_i = Σ_j γ_ij k_j and (1/(hγ)I − J)u_i = f(Y_i) + Σ_j (c_ij/h)u_j + γ_i h f_t. The J·(Σγ_ij k_j) product disappears.
- **Gain.** Exactly 7 JVPs per step. The coefficients (`c_matrix_input`, `a_input`) are already in the snapshot.
- **Risk [DERIV].** Under inexact solves, stage residuals leak into later right-hand sides through the c_ij/h coupling instead of through an exact JVP. The residual-to-output certificate must be re-derived.

---

## 4. Family (iii): mixed-precision Krylov and GMRES-IR

**(a) Core**
- GMRES-IR in three precisions (Carson & Higham, SISC 40(2):A817–A847, 2018) [ABS]. The κ thresholds come from a summary [SEC: Oktay & Carson, arXiv 2201.09827]: κ∞(A) ≤ u^{−1/2}u_f^{−1} with extra precision, κ∞(A) ≤ u^{−1/3}u_f^{−2/3} with uniform precision. The guarantees hold only for unrestarted GMRES.
- Inexact Krylov (Simoncini & Szyld, SISC 25(2):454–477, 2003) [ABS]: computable criteria allow matvec accuracy to relax as the residual decreases.

**(c) Cost**
- Mixed-precision restarted GMRES on GPUs (Lindquist, Luszczek & Dongarra, IEEE TPDS 33(4):1027–1037, 2022) [snippet]:
  - unpreconditioned mean speedups of 18% (MGS) and 61% (CGS with reorthogonalization);
  - pure fp32 reached the target on only 16 of 23 problems.
- Balos, Roberts & Gardner, HPEC 2023 (arXiv 2307.09498) [ABS]: low-precision φ actions with a reformulated exponential Rosenbrock–Euler, plus inexact and incomplete Arnoldi.
- Grant, mixed-precision Runge–Kutta (arXiv 2012.13055; J Sci Comput 2022) [SEC]: low-precision implicit stages plus corrections give O(εh^m) error.

**(d) Failure modes [DERIV]**
- No reduction in JVPs.
- Finite-difference JVPs in fp32 have relative error of order √u₃₂ ≈ 2e-4, so forward-mode AD JVPs are required.
- The final residual must be computed in fp64 to keep certification.
- Convergence needs κ(W)·u₃₂ ≲ 1. For μ ≤ 0, κ₂(W) ≤ 1 + hγ‖J‖.

**(e) Relevance.** Speed only, about 1.2–1.6× and bandwidth-bound. It does not reduce JVPs; lowest priority.

---

## 5. Key question: certifiable behavior AND fewer JVPs than unpreconditioned GMRES per stage

| Direction | Certifiable? | JVP reduction | Spectral or structural condition |
|---|---|---|---|
| Transformed u-form | Yes, after re-deriving the certificate | −7 per step, unconditionally | None |
| Exact within-step augmentation with all previous stage bases and f(Y_i) | Yes (true residual) | Large when stage right-hand sides are correlated | None for correctness |
| Stage-1 shift reuse on rejection or h-probing | Yes | m₁ saved per retry | Same y_n and J |
| Shift-updatable spectral deflation | Yes (right preconditioning) | Large | k ≪ n isolated stiff modes; ineffective for continuum spectra |
| Exponential candidate with certified defect bound and shared basis | Yes when μ_D(hJ) ≤ 0 | 2–3 φ calls vs 8 solves | Dissipative, real-dominated spectra; fails with strong non-normality or large ‖hJ‖ |
| ROK / Krylov-W | Order: yes; stability: no, needs a residual gate | One space per step | Small stiff subspace; for DAEs, M > number of algebraic variables |
| Polynomial preconditioning | Yes | ≈ 0 | — |
| Mixed precision | Yes, with fp64 final residual | 0 (wall clock only) | κ(W)·u_low ≲ 1 |
| Colored Jacobian with lagged LU (conditional) | Yes | Very large | Known sparsity, small number of colors |

---

## 6. Ranked shortlist

1. **Switch RODAS5P to the transformed u-form stage formulation.** This saves exactly 7 of the 8 per-stage coupling JVPs on every step without changing the method in exact arithmetic. The coefficients are already in the repo; only the stage-residual certificate needs re-deriving.
2. **Exact within-step Krylov reuse plus h-independent stage-1 reuse.** Because W is identical in all 8 stages, augmenting each stage with all previous stage bases and f(Y_i) is exact recycling at zero extra JVP cost (justified by Krylov-W, ROWMAP and ROK basis extension). It keeps RODAS5P's exact-J order and a certified true residual, and makes rejected-step retries and h-probing nearly free.
3. **Shift-updatable spectral (Schur/Ritz) deflation preconditioner, built from Arnoldi data already computed.** It re-targets any new h or γ with a k×k solve, preserves residual certification, and also deflates the φ path. It pays off only when stiffness sits in a few modes; for problems with known sparsity, the colored-Jacobian lagged preconditioner is the stronger conditional variant.
4. **Cost-aware step-size and candidate selection on measured JVPs** (Einkemmer, Appl Numer Math 132:182–204, 2018 [ABS]; Deka & Einkemmer, CAMWA 2022 [ABS], with 1–4× reported for EXPRB43 [ABS]). Error control is unchanged, so robustness is unaffected. Cheap shifted re-solves from item 2 make the cost per unit time at candidate h measurable.
5. **Exponential candidate with certified φ actions sharing RODAS stage-1 Arnoldi data.** It needs only 2–3 φ calls per step, uses the Jawecki defect bound in a metric where μ_D(hJ) ≤ 0, and uses the block field-of-values formulation for guards. Admit it only for real or dissipative spectra and loose-to-moderate tolerances, where EPIRK-class methods beat CVODE-type implicit solvers about 2× (§2.4). Avoid expmv, which needs Aᵀ products.
6. **Rosenbrock–Krylov / Krylov-W (ROK4a or ROWMAP-type) as a residual-gated order-4 fallback for small stiff subspaces.** Order is guaranteed independent of the Krylov dimension at one Krylov space per step. Stability is not guaranteed, so M must adapt to stage residuals as in Tranquilli, Glandon & Sandu 2021.

Mixed precision and polynomial preconditioning are left off the list because neither reduces JVPs.