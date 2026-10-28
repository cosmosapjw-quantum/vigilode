# L2 literature report: methods that can cut rejected steps and per-step cost together in a matrix-free RODAS5P solver

## How to read the source tags

- **[CODE]**: I read the full primary text of a reference implementation and its comments, mostly from GitHub mirrors. These are the strongest sources in this report:
  - Hairer's `rodas`, `radau5` and `dopri5` (Assimulo mirror, `thirdparty/hairer/*.f`)
  - Sommeijer–Shampine–Verwer `rkc.f` (odespy mirror)
  - Abdulle `rock4.f` (MoisaAndrew/StabilizedMethods)
  - LSODA/STODA (SciPy v1.10.0 `odepack`)
  - PETSc `src/ts/adapt/impls/dsp/adaptdsp.c`
  - OrdinaryDiffEq.jl source and docstrings
- **[SNIP]**: I saw only the abstract or a search summary. The network proxy blocked the full text: arxiv.org, epubs.siam.org, link.springer.com, ir.cwi.nl, netlib, unige.ch, petsc.org, researchgate and pub.h-brs.de were all blocked.
- **[MEM]**: a citation from my background knowledge that I did not re-check in this session. Treat these as **UNCERTAIN**.
- **[DER]**: my own derivation or inference. No cited paper makes these claims.

## Facts about the target method [CODE: OrdinaryDiffEq `rosenbrock_tableaus.jl`, Hairer `rodas_decsol.f`]

- Rodas5P has 8 stages and γ = 0.21193756319429014.
- `btilde = e_8`, so the error estimate is exactly the last stage increment k₈.
- Hairer's RODAS works the same way. Its error is `AK6`, which is itself produced by a solve with the stage matrix. RODAS recomputes the Jacobian every step and refactors on rejection.
- The Rodas5P paper (Steinebach 2023, BIT 63:27, doi:10.1007/s10543-023-00967-x [SNIP]) reports two problems with the older Rodas5:
  - its embedded method is not A-stable;
  - on y′ = cos t the main and embedded errors are identical, so step-size control fails.
- Rodas5P was built to fix both.

---

## 0. Cross-cutting cost model [DER]

I use this model to answer the key question for each spectrum.

**Assumptions.** Unpreconditioned Krylov, worst-case polynomial bounds, relative tolerance η, all 8 Rodas5P solves using the same matrix I − hγJ, and ρ = spectral radius.

**Real-negative (parabolic) spectrum, κ ≈ 1 + hγρ**
- Iterations per solve ≈ ½·√κ·ln(2/η), from the CG-type bound. GMRES on normal operators with real spectra obeys the same Chebyshev bound.
- Per Rodas5P step: ≈ 4·ln(2/η)·√γ·√(hρ). This is about 26.7√(hρ) JVPs at η = 1e‑6 and 14√(hρ) at η = 1e‑3.
- ROCK4 needs ≈ √((3+hρ)/0.353) + 4 ≈ 1.68√(hρ) f-evaluations per step [CODE degree formula]. RKC needs ≈ √(1.54 hρ) [CODE].
- Both cost per unit time scale like √(ρ/h). So at equal h, Rodas5P costs about 16× more operator applications than ROCK4 at η = 1e‑6, and about 8× at η = 1e‑3. That does not count GMRES orthogonalization or global reductions.
- Break-even needs Rodas5P's accuracy-limited step to be about 250× (η = 1e‑6) or about 70× (η = 1e‑3) larger than ROCK4's.
- Restarted GMRES(m) loses the √κ behaviour, which makes Rodas5P relatively worse.
- Krylov recycling or deflation helps little here. The stiff end of a discrete Laplacian's spectrum is dense near ρ.

**Purely oscillatory spectrum (on the imaginary axis)**
- The spectrum of I − hγJ is the segment 1 + i[−a, a] with a = hγρ.
- The GMRES asymptotic convergence factor is a/(1+√(1+a²)), from the exterior conformal map of a segment (potential-theory reasoning as in Driscoll–Toh–Trefethen, SIAM Rev. 1998 [MEM]). So iterations grow like a·ln(1/η), linearly in h.
- Cost per unit time is then ≈ 8γρ·ln(1/η) ≈ 23ρ at η = 1e‑6, independent of h. Larger steps buy nothing.
- Classical explicit RK4 at its imaginary-axis limit of 2√2 costs ≈ 1.4ρ per unit time.
- Stabilized-explicit methods cannot help here. The imaginary stability interval of an s-stage explicit method grows at most linearly in s (bound ≈ s−1, Vichnevetsky; Kinnmark–Gray [MEM, UNCERTAIN]).

**Nonnormal advection (first-order upwind)**
- I − hγJ = (1+ν)I − νS, with Courant-type number ν = hγc/Δx and S the shift.
- Its field of values lies in the disc centred at 1+ν with radius ν. So GMRES needs ≈ ν·ln(1/η) iterations, again linear in h, and cost per unit time is again h-independent (≈ 8γ(c/Δx)·ln(1/η)).
- With inflow boundaries, all eigenvalues equal −c/Δx while the pseudospectra fill a disc. Power-iteration ρ-estimates and eigenvalue-based stability arguments are then misleading (Reddy–Trefethen 1992 [SNIP]).

**Implications**
- An unpreconditioned Krylov-Rosenbrock pays off only when strong damping (L-stability) of unresolved stiff modes is needed, or when the slow dynamics allow enormous accuracy-limited steps.
- Otherwise its advantage is robustness, not speed.
- Any lever that removes Krylov iterations or wasted 8-solve attempts pays directly.

---

## 1. Stabilized explicit methods: RKC, ROCK2/ROCK4, SERK/ESERK, RKL, RKG and second-kind Chebyshev, plus ρ estimation

### (a) Core mathematics and guarantees

**RKC**
- s-stage explicit RK whose stability polynomial is a shifted, damped first-kind Chebyshev polynomial, evaluated by a three-term recurrence.
- In the code: `w0 = 1 + 2/(13 m²)` and `m = 1 + int(sqrt(1.54·h·ρ + 1))` [CODE rkc.f].
- The real stability interval is ≈ 0.653 s² [SNIP, citing Sommeijer, Shampine & Verwer, J. Comput. Appl. Math. 88 (1997/98) 315–326].
- Convergence theorem (Verwer, Hundsdorfer & Sommeijer, Numer. Math. 57 (1990) 157–178 [SNIP]):
  - for linear model problems the method converges under the stability step restriction alone;
  - the error bounds hold for any s and are independent of stiffness;
  - the key property is internal stability, meaning errors do not amplify across stages.

**ROCK2** (Abdulle & Medovikov, Numer. Math. 90 (2001) 1–18, doi:10.1007/s002110100292 [SNIP])
- Uses orthogonal polynomials to get a near-optimal second-order stability polynomial that still has a three-term recurrence.
- Stability domain ≈ 0.81 s² [SNIP; secondary sources].

**ROCK4** (Abdulle, SIAM J. Sci. Comput. 23 (2002) 2041–2054, doi:10.1137/S1064827500379549 [SNIP])
- Fourth order: an orthogonal-polynomial part composed with a 4-stage finishing procedure.
- Degree in the code: `mdeg = sqrt((3+hρ)/0.353)+1−4`, capped at 152 [CODE rock4.f].
- When the cap binds, the code reduces h: `h = 0.8(152²·0.353−3)/ρ`.

**RKL** (Meyer, Balsara & Aslam, J. Comput. Phys. 257 (2014) 594–626 [SNIP])
- Legendre-based, first or second order, step gain O(s²).
- No free damping parameter. A later code paper calls it "intrinsically stable" [SNIP].

**RKG** (O'Sullivan, J. Comput. Phys. 388 (2019) 209–223, arXiv:1712.03971 [SNIP])
- Gegenbauer stability polynomials of arbitrary order in closed form.
- The imaginary-axis extent increases with the Gegenbauer parameter.
- An ordering algorithm keeps internal amplification ≤ 10L².
- Tested only for mesh Péclet number < 1.

**Second-kind Chebyshev** (Almuslimani, arXiv:2201.10206; DOI 10.1007/s10543-023-00945-3, venue BIT per the DOI prefix — UNCERTAIN [SNIP])
- Second-order RKC-type methods stabilized with second-kind Chebyshev polynomials, an idea first used in the stochastic integrator SK-ROCK.
- Selects h, s and the damping parameter automatically.
- Claimed to beat existing schemes at relatively high Péclet numbers.

**SERK2v2 / ESERK4 / ESERK5** (Kleefeld & Martín-Vaquero, NMPDE 29 (2013) 170–185; ESERK5: J. Comput. Appl. Math. 356 (2019) 22–36 [SNIP])
- First-kind Chebyshev building blocks plus extrapolation.
- The stability region grows like O(nₜ²) while the order stays fixed.
- SciML describes ESERK4 as "smoothened to allow moderate complex eigenvalues" [SNIP of the docs].

**Spectrum-adapted polynomials**
- Ketcheson & Ahmadia (CAMCoS 7 (2012) 247–271 [SNIP]): convex-optimization design of the most stable polynomial for a given spectrum, with proven global convergence for order 1 or starlike regions.
- Torrilhon & Jeltsch (Numer. Math. 2007, doi:10.1007/s00211-006-0059-5 [SNIP]): thin-region spectra from hyperbolic-parabolic problems, so that the hyperbolic CFL alone sets the step.

**Nonnormal guarantee** (Reddy & Trefethen, Numer. Math. 62 (1992) 235–267 [SNIP])
- The method of lines is Lax-stable if and only if the ε-pseudospectra of the scaled operator lie within O(ε)+O(k) of the stability region.
- So an eigenvalue-only argument such as "the spectral radius fits in the strip" is insufficient for nonnormal J.

**Spectral-radius estimation**
- RKC `RKCRHO` [CODE]: nonlinear power method, up to 50 iterations, relative tolerance 0.01, result ×1.2 ("more likely to be an upper bound"), recomputed every 25 accepted steps or after a rejection, eigenvector stored as the next start.
- ROCK4 `rockfrho` [CODE]: up to 50 iterations, 5% relative test, ×1.2, refreshed about every 5 accepted steps and after a rejection.
- OrdinaryDiffEq `maxeig!` [CODE]: 50 iterations (100 for SERK2/ESERK4/5); the RKC family uses a 1% test with ×1.2 on convergence, the others a 5% test with safety 1.2. Recomputed on an interval, on the first step, at discontinuities and after rejections.
- Skvortsov (Comput. Math. Math. Phys. 65 (2025) 2567–2579 [SNIP]): Jacobian-free power-method procedures built into the integrator that track a changing ρ.
- Niemeyer & Sung [SNIP]: Gershgorin bounds overestimated ρ and gave too many stages.

### (b) Reported robustness

- Both RKC and ROCK4 pair the error estimate with a Gustafsson-type predictive controller [CODE].
  - RKC: `fac = 0.8·|h|·errold^{1/3} / (|hold|·err^{2/3})`, factor kept within [0.1, 10]; after a rejection `h ← 0.8h/err^{1/3}`.
  - ROCK4: `facp = errp^{1/4}·fac²·(h/hp)`, `fac = min(fac, facp)`; facmax is 5 initially, 2 after an accepted step, 1 after a rejection.
- The RKC header limits its scope: "modest accuracy of mildly stiff problems … eigenvalues … close to the negative real axis". It also says that if m is "far beyond 100 the problem is not mildly stiff" [CODE].
- RKL2 used for viscosity in a coronal MHD code produced slowly growing grid-scale oscillations, because high modes are only weakly damped (Caplan et al., arXiv:1610.01265 [SNIP]).

### (c) Reported cost

- **Caplan et al. [SNIP]:** RKL2 ran more than 2× faster than backward Euler with preconditioned CG for viscosity, and more than 3× faster in other tests not reported in detail. It scaled better at high core counts because it has no global dot products.
- **Niemeyer & Sung, J. Comput. Phys. 2014, doi:10.1016/j.jcp.2013.09.025 [SNIP]:**
  - CPU RKC beat VODE on moderately stiff kinetics.
  - RKC on GPU was up to 57× faster than six-core VODE (methane mechanism).
  - With more stiffness (larger steps) the GPU RKC was 2.5× *slower* than VODE.
- **ESERK [SNIP]:** cost about nₜ times lower than a classical explicit method.

### (d) Failure modes

- Dominant complex or oscillatory spectra. The real strip is narrow; SciML says these methods are "not for … large complex eigenvalues" [SNIP].
- Nonnormality (Reddy–Trefethen).
- Advection-dominated flow needs heavy damping, which shrinks the real stability length. This is reported for PIROCK (Abdulle & Vilmart, J. Comput. Phys. 242 (2013) 869–888) by Almuslimani [SNIP].
- The power method can fail to converge for a complex-conjugate dominant pair, and it underestimates when ρ jumps within a step. One τ-leap paper suggests using s+1 stages as a safety margin [SNIP].
- Stage caps: ROCK4's degree is capped at 152; RKC's `mmax = sqrt(rtol/(10·uround))` limits round-off growth [CODE].
- No L-stability, so unresolved stiff transients ring.
- Boundary-condition order reduction (Skaras, Saxton, Meyer & Aslam, J. Comput. Phys. 425 (2021) 109879 [SNIP]).

### (e) Relevance to a matrix-free RODAS5P solver

**What would change**
- Add a ROCK4 or RKC-family branch. It needs only f-evaluations, which the solver already has.
- An adjacent option is to keep RODAS5P but solve its stages with a Chebyshev semi-iteration on [1, 1+hγρ] instead of GMRES:
  - same √κ iteration count;
  - no inner products and no orthogonalization;
  - analogous to the "explicit stabilised implementation of SDIRK" idea (Almuslimani, Vilmart & Zygalakis, arXiv:2607.07497, 2026 preprint [SNIP]);
  - for complex spectra the enclosing ellipse needs adaptive Chebyshev in the style of Manteuffel 1977 [MEM].

**What it would buy**
- For real-negative-dominated spectra, roughly 8–16× fewer operator applications per unit time at equal h, before counting orthogonalization savings [DER §0].

**What could go wrong**
- Misclassifying a nonnormal or oscillatory regime.
- Loss of L-stability (the Caplan-type oscillations).
- ρ underestimation, which gives silent instability.
- Two separate error-estimate and dense-output paths to certify.

---

## 2. Automatic stiffness detection and method switching

### (a) Core mathematics and guarantees (all heuristic; no convergence theorem)

**LSODA** (Petzold, SIAM J. Sci. Stat. Comput. 4 (1983) 136–148 [SNIP abstract]; logic [CODE stoda.f])
- Adams → BDF only if the last step was stability-restricted (`irflag = 1`, i.e. rh·pdh ≥ `sm1(nq)`) and the BDF step-size advantage is `rh2 ≥ ratio·rh1` with `ratio = 5`.
- BDF → Adams if `rh1·ratio ≥ 5·rh2`, i.e. the Adams step is at least as large.
- A switch is followed by `icount = 20` steps of hysteresis.
- The Lipschitz estimate is `pdest = max(rate/|h·el(1)|)`, with `rate = del/delp` from the corrector iteration (capped at 1024). The BDF → Adams test uses the Jacobian norm `pdnorm`.

**Shampine 1977** (ACM TOMS 3(1), doi:10.1145/355719.355722 [SNIP]) and Shampine 1991 (SIAM J. Sci. Stat. Comput., doi:10.1137/0912015 [SNIP])
- Stages that share a node give a free difference-quotient estimate of the dominant eigenvalue.

**DOPRI5** [CODE dopri5.f]
- `HLAMB = h·‖k7−k6‖/‖y1−ysti‖`, testing whether HLAMB > 3.25.
- Stiff after 15 consecutive positive tests; the counter resets after 6 non-stiff tests.
- The test is activated every `NSTIFF` (default 1000) accepted steps, and runs continuously once a positive test has occurred.

**OrdinaryDiffEq AutoSwitch** [CODE]
- `stiffness = |eig_est·dt / stability_size|`, compared with 9/10.
- Switches to stiff after more than 10 consecutive stiff results and back after more than 3 non-stiff ones; dt is ×2 or ÷2 on the switch.
- The default algorithm switches Tsit5/Vern7 ↔ Rosenbrock23/Rodas5P for small systems, FBDF above 50 unknowns, and Krylov (GMRES) FBDF above 500.
- A NaN estimate is treated as stiff.

**Other approaches**
- Mathematica StiffnessSwitching [SNIP]: explicit modified-midpoint extrapolation ↔ linearly implicit Euler extrapolation; the stiffness test may fail a set number of times before switching.
- Rentrop 1985 (via Lang's review [SNIP]): coupling a Rosenbrock pair with an RK pair adds coupling conditions.
- Büttner, Schmitt & Weiner, Appl. Numer. Math. 13 (1993) 41–55 and SIAM J. Numer. Anal. 32 (1995) 260–284 [SNIP]:
  - an approximate Krylov solve of the W-method stage equations *is* an automatic partitioning: captured components are implicit, the rest explicit;
  - the Krylov dimension needs to be only slightly larger than the number of fast components that contribute non-negligibly;
  - examples used dimensions from 20 to 302.
- IMEX-RB (arXiv:2506.16470, 2025 [SNIP]): a self-adaptive IMEX approach.

### (b) Reported robustness

- Hysteresis (20 steps in LSODA, 15/6 in DOPRI5, 10/3 in AutoSwitch) prevents chattering [CODE].
- All of these detectors use a magnitude (ρ or a Lipschitz norm). None sees the *shape* of the spectrum: real vs imaginary vs nonnormal [DER].

### (c) Reported cost

- Petzold: many problems ran more efficiently with switching, and the cost of choosing methods was small [SNIP]. I found no numbers.

### (d) Failure modes

- Detection lag after regime changes.
- In the stability-limited regime the step-size sequence oscillates (Hall & Higham, IMA J. Numer. Anal. 8 (1988) 305–310 [SNIP]), which makes ρ̂·h noisy.
- Norm-based estimates overestimate badly for nonnormal J [DER].

### (e) Relevance

**Free spectral information**
- The Arnoldi relation from each stage GMRES gives Ritz values of J, since J ≈ (I − H)/(hγ) on the Krylov space. That is information about both ρ and the angle of the spectrum, at zero extra JVP cost.
- I found no paper that uses GMRES Hessenberg Ritz values for stiffness detection or stage selection. This is novel and untested.

**Critical caveat [DER]**
- Ritz values from a smooth right-hand side under-represent the extreme modes. That is acceptable for predicting Krylov cost, but it is *unsafe* for licensing an explicit method, because any mode destabilizes explicit integration through round-off.
- A switch to an explicit branch therefore needs an independent random-start power iteration with the ×1.2 margin, as RKC and ROCK do.

**Recommended rule**
- Use an LSODA-style cost ratio (≥ 5) with hysteresis (~20 steps), choosing among explicit RK, ROCK4/RKC and RODAS5P with the §0 cost formulas.

---

## 3. Step-size controller theory

### (a) Core mathematics and guarantees

**Model.** Asymptotically err ≈ φ·hᵏ. Controllers are linear feedback on log err; stability and smoothness come from the closed-loop poles.

**PI controller** (Gustafsson, Lundh & Söderlind, BIT 28 (1988) 270–287, doi:10.1007/BF01934091 [SNIP])
- Designed for the regime where stability, not accuracy, limits the step. Called "more robust at little extra cost".
- Hall & Higham 1988 [SNIP] give equilibrium criteria for smooth stability-limited behaviour.

**Predictive controller** (Gustafsson, ACM TOMS 20 (1994) 496–517, doi:10.1145/198429.198437 [SNIP])
- Hairer's codes implement a modified version [CODE rodas/radau5]: `FACGUS = (HACC/H)·(ERR²/ERRACC)^{1/4}/SAFE`, then `FAC = max(FAC, FACGUS)`. This takes the *smaller* of the classical and predictive steps.
- The RADAU5 comment says this "seems to produce safer results; for simple problems the classical [choice] often [gives] slightly faster runs" [CODE].
- PETSc's PC11 entry (β = (2, −1)/1, α₂ = −1) appears to be the same predictive form. I matched these coefficients myself [CODE PETSc, DER].

**Digital filters** (Söderlind, ACM TOMS 29 (2003) 1–26, doi:10.1145/641876.641877 [SNIP]; coefficients [CODE adaptdsp.c])
- General form: ρₙ = c₀^{b₁}·c₁^{b₂}·c₂^{b₃}·ρₙ₋₁^{−a₂}·ρₙ₋₂^{−a₃}, with c = (1/err)^{1/k}.
- Coefficient sets:
  - H211b = (1, 1, 0; α₂ = 1)/4
  - H312b = (1, 2, 1; 3, 1)/8
  - H211PI = (1, 1)/6
  - H312PID = (1, 2, 1)/18
  - PI42 = (3, −1)/5
  - PI34 = (7, −4)/10
- Limiter: 1 + atan(x−1). A step is accepted if the limited factor is ≥ 0.81.
- OrdinaryDiffEq's PIDController uses the same limiter and `accept_safety = 0.81`, with the factor kept within about [0.21, 2.57] [CODE].

**Söderlind & Wang** (J. Comput. Appl. Math. 185 (2006) 225–243, doi:10.1016/j.cam.2005.03.008 [SNIP])
- In modified DASSL, "relatively small algorithmic changes … vastly better computational stability at no extra expense."
- Related reviews: Söderlind, Numer. Algorithms 31 (2002) 281–310 and Appl. Numer. Math. 56 (2006) 488–502 [SNIP].

**Gustafsson & Söderlind** (SIAM J. Sci. Comput. 18 (1997) 23–40 [SNIP via a secondary description])
- Coordinate the Newton convergence rate α with the step-size controller.
- Force a Jacobian re-evaluation or a refactorization when α is likely to exceed its maximum, so that convergence failures do not occur.
- After a step increase, refactorization is often needed but the old Jacobian can be kept.

**Cost-aware control** (Einkemmer, arXiv:1709.10337, 2018, venue UNCERTAIN [SNIP])
- Minimizes cost per unit time using Krylov iteration counts, treated as a one-dimensional gradient search.
- Speedups of up to 5× on 1-D test problems: advection-diffusion, Burgers, porous medium, Allen–Cahn, Brusselator.
- Deka & Einkemmer (2021): up to 4× for exponential Rosenbrock methods [SNIP].

### (b) Reported robustness

- In an explicit SSP test on Van der Pol, PID and explicit-Gustafsson controllers gave much smoother step sequences and far fewer rejections than the I-controller [SNIP].
- Ranocha, Dalcin, Parsani & Ketcheson (CAMC 4 (2022) 1191–1228, doi:10.1007/s42967-021-00159-w [SNIP]): controller parameters matter, and optimized methods settle near the maximum stable CFL number at loose tolerances.

### (c) Reported cost

- Einkemmer: up to 5×. Söderlind–Wang: improvement "at no extra expense". I did not retrieve rejection-count tables.

### (d) Failure modes

- The asymptotic model breaks down under stiff order reduction (err ∝ h^q with q < p) and at transients.
- Estimator degeneracy can break control entirely, as in the Rodas5 cos t case [SNIP].
- **Hidden leniency [DER from CODE formulas]:** with `accept_safety = 0.81` and the atan limiter, the basic controller with k = min(5,4)+1 = 5 for Rodas5P accepts a step whenever err ≤ (1+tan(−0.19))^{−5} ≈ 2.9.
  - The tolerance becomes a target, not a bound.
  - This conflicts with any "certified ≤ tol" claim.
- Einkemmer's controller is deliberately non-smooth.

### (e) Relevance

**Matrix-free specific [DER]**
- There is no LU, and Krylov spaces are shift-invariant:
  - K_m(I−hγJ, b) = K_m(J, b);
  - (I − h′γJ)V_m = V_{m+1}(Ī − h′γH̄_m) for any new step h′.
- So a step-size change costs nothing for Krylov data. RADAU5's LU-saving dead band (keep h if 1.0 ≤ hnew/h ≤ 1.2 [CODE]) has no rationale here, and smooth filters can run freely.

**What to do**
- Use H211b or PI-type smoothing, with Gustafsson's predictive form after rejections and transients.
- Keep a strict err ≤ 1 acceptance, or make the leniency explicit and audited.
- Add a cost term from GMRES iteration counts.
- In the §0 model the cost per unit time of Rodas5P falls like h^{−1/2} (parabolic) or is flat (oscillatory/advective). So Einkemmer-type gains should appear mainly where restarts or stagnation make iterations superlinear in h [DER].
- Each rejection wastes 8 Krylov solves, so rejection reduction is worth more here than in explicit codes.

---

## 4. Robustness of the Rosenbrock error estimator (filtered estimates)

### (a) Core mathematics

**RADAU5 `ESTRAD`** [CODE]
- The raw estimate is multiplied by the existing LU of (I − hγ₀J), i.e. filtered.
- On FIRST or REJECTED steps with err ≥ 1, a second correction applies: err′ = E⁻¹(f(y₀+err) + …), costing one extra f-evaluation and one extra solve.
- Origin: Shampine & Baca, J. Comput. Appl. Math. 11 (1984) 197–207 [SNIP].
- Critique: de Swart & Söderlind, J. Comput. Appl. Math. 86 (1997) 347–358, doi:10.1016/S0377-0427(97)00166-0 [SNIP], call the filter ad hoc and "without firm theoretical grounding", and propose an implicit estimator with correct stiff behaviour.

**RODAS / Rodas5P** [CODE]
- The error is the last stage k_s, which itself solves (I/(hγ) − J)·k = … with the stage matrix.
- So it is *structurally* filtered at no extra cost, and no additional filter is applied (confirmed in both codes).

**Rodas5Pe** [CODE docs]: uses a stiffly accurate embedded scheme.

**Rodas5Pr** [CODE `rosenbrock_perform_step.jl`]
- Only when EEst < 1, it forms the dense-output midpoint, compares the interpolant's derivative with f(midpoint, t+dt/2), and sets EEst ← max(EEst, ‖defect‖/(atol + rtol·|·|)).
- Cost: one f-evaluation.

**ode23s** (Shampine & Reichelt, SIAM J. Sci. Comput. 18 (1997) 1–22 [SNIP]): a modified Rosenbrock (2,3) pair with no local extrapolation. OrdinaryDiffEq labels Rosenbrock23 a "Rosenbrock-W method" [CODE docstring].

### (b) Reported robustness

- Filtering keeps the estimate bounded as h → ∞ and removes the gross overestimation of stiff components [SNIP].

### (c) Cost

- In RADAU5 the filter is nearly free because the LU already exists.
- **In a matrix-free solver a filter is an extra Krylov solve [DER]**, so the RODAS structural filter is the right design.

### (d) Failure modes

- **Matrix-free specific [DER]:**
  - EEst = k₈ is only as accurate as the 8th GMRES solve, so a loose η can either under- or over-estimate the error.
  - Errors in stages 1–7 reach the estimate only through k₈'s right-hand side.
- Wang & Yu (J. Sci. Comput. 83 (2020) 39; arXiv:1904.04825 [SNIP]): Jacobian-free ROW schemes lost their nominal order at GMRES relative tolerances 1e‑2 and 1e‑4. At 1e‑6 they kept it, except ROW4 lost order at the finest refinement step. Going to 1e‑8 only added cost.

### (e) Relevance

- Certify the error-carrying stage (k₈), and the stages that feed y_{n+1}, to a tighter residual than the others.
- Add the Rodas5Pr defect test: one f-evaluation that also catches interpolation failures and some gross solve failures.
- Do *not* add a Shampine filter; it would double the solve cost for little gain.
- Optionally use the RADAU5 second-correction idea only on first and rejected steps.

---

## 5. Jacobian/LU reuse and when it is legal (W-methods)

### (a) Core mathematics

**RADAU5** [CODE]
- Skips the Jacobian update when the Newton contraction θ ≤ THET (default 0.001; "0.1 when Jacobians are costly").
- Keeps h and the LU when QUOT1 < hnew/h < QUOT2 (defaults 1.0/1.2; 0.99/2.0 suggested "for large full systems").

**RODAS** [CODE]: recomputes the Jacobian every step.

**W-methods** (Steihaug & Wolfbrandt, Math. Comp. 33 (1979) 521–534 [SNIP])
- Any matrix A may replace J, at the cost of many more order conditions and stages.
- Time-lagged A = J + O(h) (Scholz–Verwer 1983, via Lang's review arXiv:2002.12028 [SNIP]) needs fewer conditions.
- Directional consistency A·f = J·f + O(h), built from rank-1 updates, allows a 2-stage order-3 method and embedded 4-stage 4(3) pairs [SNIP].

**Krylov-W methods**
- Schmitt & Weiner, Appl. Numer. Math. 18 (1995) 307–320, and ROWMAP (Weiner, Schmitt & Podhaisky, Appl. Numer. Math. 25 (1997) 303–319) [SNIP]:
  - a multiple Arnoldi process over all stages gives order 4 with small Krylov dimensions independent of n;
  - stability may need larger dimensions, chosen adaptively;
  - compared favourably with VODPK.
- Rosenbrock-Krylov (ROK) (Tranquilli & Sandu, SIAM J. Sci. Comput. 36 (2014) A1313–A1338, arXiv:1305.5481 [SNIP]):
  - A is the Krylov projection built from f(yₙ); the number of basis vectors depends only on the order; far fewer order conditions;
  - with inaccurate finite-difference JVPs, Rok4a/b drop to order 2 (they satisfy the second-order W condition) and Rok4p drops to order 1;
  - the authors recommend W-methods when JVPs cannot be made accurate.
- Biorthogonal ROK (arXiv:1908.10531 [SNIP]): for large stiff problems m ≫ p is needed for stability, and Arnoldi becomes the bottleneck.

**The Rodas family** [CODE docstrings]
- Rodas4P2: "in case of inexact Jacobians a second order W method".
- W variants: Rodas4PW ("in preparation, 2026"), Rodas23W, ROS34PW1a/1b/2/3 and ROS34PRw.
- **Rodas5P has no inexact-Jacobian order statement in the code.** The Rodas5P paper's table, seen only in a snippet, lists Rodas5 at order 1 with an inexact Jacobian, and a fragment says Rodas5P does not beat Rodas5 there. **UNCERTAIN.**
- Jax & Steinebach (J. Comput. Appl. Math. 316, 2017 [SNIP]): new order conditions for inexact Jacobians on index-1 DAEs.

### (b) Reported robustness

- W-methods keep their order for any A, but stability then depends on A.
- With a Krylov A, modes not captured by the space are integrated explicitly (Büttner et al.).

### (c) Reported cost

- Wang & Yu [SNIP]:
  - ESDIRK tolerated a linear tolerance of 1e‑1, while ROW needed 1e‑6;
  - ROW was more efficient when the tolerances were equal;
  - the preconditioner strongly affects ROW efficiency.
- SciML docs: with Krylov linear solvers, SDIRK or BDF are preferred over Rosenbrock [CODE docs]. The default algorithm routes systems above 500 unknowns to Krylov FBDF [CODE].
- SymBoltz (arXiv:2509.24740 [SNIP]): Rodas5P was the most efficient and most stable; W-methods' Jacobian reuse is not used in OrdinaryDiffEq.

### (d) Failure modes

- A non-W Rosenbrock with an inexact J or inexact solves suffers order reduction. For DAEs it drops to order 1 even with per-step finite-difference Jacobians [SNIP, Lang review].
- **[DER]** Only the FOM/Galerkin solution xₘ = Vₘ(I − hγHₘ)⁻¹βe₁ is exactly a W-step with A = VHVᵀ. A truncated GMRES (minimal-residual) solution is not literally a W-step, so the ROWMAP and ROK legality arguments need a Galerkin projection.

### (e) Relevance [DER]

Matrix-free "reuse" means reusing the Arnoldi relation J·V = V·H̄, not an LU.

1. **Across the 8 stages: always legal.**
   - All stages use the same I − hγJ, and the Krylov space is shift-invariant.
   - A multiple-Arnoldi or augmented space extended by each new right-hand side cuts JVPs.
   - It stays RODAS5P-legal as long as every stage's *true* residual is certified.
2. **Across steps: legal for non-W RODAS5P only a posteriori.**
   - Check the true residual b − (I − hγJ(y_{n+1}))·x with one fresh JVP.
   - Otherwise switch to a W-legal tableau (Rodas4P2, Rodas4PW, ROS34PW2) or to a ROK formulation.
3. **Across h changes: free** (§3e).
4. **The same W/K story covers the exponential-Rosenbrock candidates.** EPIRK-W/K (Narayanamurthi, Tranquilli, Sandu & Tokman, arXiv:1701.06528 [SNIP]) give φ-action approximations that are legal under W/K order conditions.

---

## Key question: which ideas give fewer rejected or wasted steps *and* cheaper steps, by spectrum?

| Spectrum | Cheaper steps | Fewer rejected/wasted steps | Avoid |
|---|---|---|---|
| Real-dominated parabolic | ROCK4/RKC/RKL branch, or a Chebyshev stage iteration (~8–16× fewer JVPs per unit time at equal h [DER]); multiple-Arnoldi sharing across stages | Gustafsson predictive + H211b filtering; ρ re-estimate after rejection (RKC/ROCK practice) | Unpreconditioned GMRES with loose tolerance (order loss); recycling, which helps little here [DER] |
| Oscillatory | Nothing in L2 gives quadratic gains. Classical explicit RK at its stability limit is ~10–15× cheaper per unit time than Krylov-RODAS5P [DER] unless L-stable damping is required | PI-type controllers (Gustafsson 1988; Hall–Higham) to stop step oscillation at the stability boundary | Chebyshev stabilized methods; expecting larger h to reduce Krylov cost (it is h-independent [DER]) |
| Nonnormal advection-dominated | Damped or second-kind Chebyshev methods (PIROCK, Almuslimani) or RKG, only at moderate Péclet; else explicit RK at its CFL limit | Pseudospectrum-aware detection (Reddy–Trefethen); rejection-driven ρ refresh; RODAS5P with a certified error stage | Power-iteration ρ alone; eigenvalue-based switching |

---

## Ranked shortlist

1. **Spectral triage with LSODA-style switching between RODAS5P and ROCK4/RKC (and explicit RK).** Free Ritz values from the stage Arnoldi, plus a random-start power iteration with ×1.2 margin before any explicit switch, a cost ratio ≥ 5 and ~20-step hysteresis. This is the only lever with an order-of-magnitude JVP saving on real-dominated spectra [DER §0; RKC/ROCK/LSODA CODE; Caplan and Niemeyer–Sung SNIP], and robustness is kept by staying on RODAS5P whenever the spectrum is non-real or nonnormal.
2. **One shared Krylov space for all 8 stages, carried across step-size changes.** Use shift invariance and multiple Arnoldi as in ROWMAP, with every stage residual certified. It cuts Krylov work without touching RODAS5P's order or stability, because the matrix I − hγJ is identical in all stages [DER; ROWMAP SNIP].
3. **W-legal fallback tableau (Rodas4P2, Rodas4PW, ROS34PW2) or a ROK formulation.** Use it whenever Krylov data is reused across steps or a loose or truncated solve is wanted. It turns "inexact solve ⇒ order 1, silent accuracy loss" into a guaranteed order-2–3 W-method with small Krylov dimensions [Rodas docstrings CODE; Tranquilli–Sandu and Büttner et al. SNIP].
4. **Controller upgrade: Gustafsson predictive (already in RODAS/RADAU5) plus H211b/PI smoothing, a strict err ≤ 1 acceptance, and a Krylov-iteration cost term (Einkemmer).** With 8 Krylov solves per attempt, each avoided rejection is expensive. The dead-band rationale disappears matrix-free, and cost-aware control has reported up to 5× [CODE PETSc/OrdinaryDiffEq/Hairer; SNIP Söderlind, Einkemmer]. The 0.81 acceptance leniency (accepted err up to ~2.9 for the basic controller [DER]) must be removed or audited.
5. **Harden the error estimate for inexact solves.** Certify k₈ (the estimate) to a tighter residual, add the Rodas5Pr midpoint defect test (one f-evaluation), and skip any explicit Shampine filter because RODAS is already structurally filtered. These are cheap safeguards against solve-corrupted acceptance decisions, the dominant matrix-free robustness hole [CODE Rodas5Pr/RODAS/RADAU5; SNIP Wang–Yu, de Swart–Söderlind].
6. *(Lower priority)* **Localized-stiffness handling.** Explicit stabilized multirate mRKC (Abdulle, Grote & Rosilho de Souza, Math. Comp. 91 (2022) 2681–2714 [SNIP]) or Krylov automatic partitioning (Büttner et al.). It pays only when a few components carry the stiffness, for example under local mesh refinement.