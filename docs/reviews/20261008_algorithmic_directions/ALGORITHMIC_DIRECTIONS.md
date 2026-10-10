# Method-level directions for robustness and speed (2026-10-08): exploratory cycle

Branch `audit/rvj-algorithmic-directions-20261008`, based on `a49f7e4` (head of `audit/rvj-speed-research-20261005`).

**Question.** The two speed cycles (SPD01-SPD09) worked at the programming level and kept results bit for bit. This
cycle asks a mathematical question: which method-level changes make the RODAS5P solver more robust (fewer failures,
rejections and order collapses; self-verifying behaviour) and faster (fewer JVPs, Krylov columns, factorizations,
attempts) at the same time?

**Status: exploratory.** Every number below comes from Python replicas of the Rust drivers, run in a session
scratch area. The replicas are checked against recorded Rust counters (section 6), but no Rust code was changed, no
research node was registered, no ledger row was written, and nothing here is a wall-time claim (the timing authority
stays on HOLD). The results are pilots: they decide which preregistered Rust nodes are worth running, and they supply
the predictions and gates for those nodes. The full pilot material is in [`pilot/`](pilot/README.md).

## 1. Method

Four multi-agent workflows ran in sequence, with the integrator reading every report between them.

| Phase | Agents | Output |
|---|---|---|
| 1. Map | 5 code/evidence readers, 3 literature reviewers | method facts with file:line and ledger ids, recorded failure modes and costs, the closed directions, a problem/measurement kit; literature on Rosenbrock-W/K, stabilized explicit methods, controllers, exponential integrators, preconditioning (`pilot/phase1_maps/`) |
| 2. Hypothesize and refute | 5 proposers with distinct lenses, 1 merger; then 5 adversarial reviewers (reproduction, mathematics/prior art, repository evidence) and 1 judge | 25 raw hypotheses, 16 merged candidates, three verdicts each, a judged disposition (`pilot/phase2_candidates/`) |
| 3. Discriminating probes | 8 probe agents and 1 completeness critic | closed-loop replica experiments on the open questions; stress tests of the recommended stack (`pilot/phase3_reports/`, `pilot/phase3_code/`) |
| 4. Synthesis | integrator | this document |

The phase-2 judge set five rules that every phase-3 probe followed:

1. Run closed loop: stage right-hand sides come from the inexact solves, and failures are fed back to the controller.
2. Count admitted work, not only JVPs. Orthogonalization dots and axpys, preconditioner solves and LU flops count too.
3. Compare against the strongest cheap rival, not only production.
4. Score matched accuracy by both the regression frontier and the harness cheapest-run rule, including tight and badly
   scaled cells.
5. Attribute every gain with control arms.

## 2. Main findings

### 2.1 The matrix-free stage target is mis-scaled; full GMRES cycles have been hiding it

The U-form matrix-free driver solves each stage to an L2-relative 1e-10 with an absolute floor of gamma*1e-14
(`rodas5p_matrix_free_fast.rs:411-416, 475-480`; `raw_stage_target.rs:292-304`). Restarted GMRES never stops inside a
cycle (`gmres.rs:110-186`), so every solve overshoots that target by 4-5 decades. Once the overshoot is removed with an
in-cycle projected-residual stop ("proj"), the target turns out to be uncoupled from the outer tolerance in both its
parts:

- The stage right-hand side in tolerance units grows roughly like rtol^-0.7. A fixed relative target therefore leaves
  stage residuals that grow in tolerance units. On HIRES the median goes from 2e-12 to 0.21 tolerance units over the
  ladder.
- The absolute floor binds on 17-40 % of HIRES solves and 44-58 % of Robertson solves, and at rtol 1e-11 it exceeds the
  outer atol.

What this costs in accuracy (probe A1, endpoint error / rtol for LU / base / proj):

| Cell | Exact LU | Production (base) | proj at the production target |
|---|---|---|---|
| HIRES 1e-8 | 0.104 | 0.092 | 0.235 |
| HIRES 1e-10 | 0.092 | 0.084 | 4.199 |
| HIRES 1e-11 | 0.098 | 0.075 | 6.921 |
| Robertson 1e-10 | 0.190 | 0.355 | 1.435 |

The production arm is not immune either:

- At Robertson 1e-10 it is 1.9x the exact-solve error.
- At Robertson 1e-11 it takes 165 rejections against LU's 3, because the absolute floor exceeds atol.
- On the fixed-step diagonal Prothero-Robinson ladder (n = 128) its error is 8.6x, 157x and 114x direct LU at k = 3, 4, 5.

**The repair is also the speed lever.** The tested rule (probe A1, `wE0.2`; `pilot/phase3_code/target/coupled_target.py`)
measures the stage residual in the outer WRMS metric and gives each step a share of a fixed contamination budget:

    ||b_i - W U_i||_WRMS <= eps_i,
    eps_i   = theta_n / (8 max(tau_y,i, tau_e,i))
    theta_n = Theta * (h_n / T) * min(1, e_hat / 0.5)^(6/5)
    eps_8   <= 0.1 * (0.9/5)^5

- Theta = 0.2.
- T is the integration span.
- tau are the tableau's residual-to-output transfer constants (sums 25.2 and 14.2).
- A round-off guard (16 eps ||Db||) and a stall rule (1024 eps) bound what GMRES is asked to reach.
- Summed over steps, the first-order contamination is at most M*Theta. That bound depends neither on rtol nor on the
  step count.

Results:

- Endpoint error stays within 1.15x of base on every cell of the 1e-3 to 1e-11 ladders (Robertson 1e-11, which is at the
  round-off floor even for LU, excluded).
- On the fixed-step ladders it equals direct LU at every rung (1.00x on diagonal PR k = 3-5; semilinear order slopes
  4.89 and 4.90 against LU's 4.92 and 4.91).
- The two controls each fail. Without the order factor the semilinear slopes are 1.09 and 0.52; without the U8 cap HIRES
  is 1.6-1.8x base.
- The judge's L2-coupling rival `l2c` (rtol_lin = min(1e-10, 1e-3 rtol), atol_lin = 1e-3 atol) passes the adaptive gate
  but fails the semilinear ladder (slopes 4.92, 4.45, -0.25).

Cost at matched accuracy, `wE0.2` against production:

| Problem | JVP | Flops |
|---|---|---|
| Bruss-50 | 0.21-0.31x | 0.06-0.15x |
| HIRES | 0.64-0.83x | — |
| Robertson | 0.60-0.73x | — |

Against `l2c` it is 0.87-0.99x on Bruss-50 and 1.00-1.06x on HIRES and Robertson.

This is the cleanest case in the cycle of one mechanism delivering both properties. A stage target that is consistent
with the outer tolerance is what lets the solver stop early, and early stopping is safe only with that target.

### 2.2 The integrated stack: large gains only where no structure is declared

Probe B1 measured the cumulative stack in closed loop against the SPD07 Rust counters. The recommended stack S is:

1. the duplicate final residual removed;
2. the in-cycle projected stop;
3. the coupled target of 2.1;
4. the predictive controller with a post-rejection cap (2.3);
5. a stage budget of 2,000 columns with a stagnation guard instead of 200.

Matched accuracy, frontier / cheapest-run rule:

| Problem | S / production, JVP | S / production, flops | S / proj-at-production-target, JVP |
|---|---|---|---|
| Bruss-50 (n = 100) | 0.26 / 0.27 | 0.10 / 0.10 | 0.75 / 0.80 |
| Bruss-160 (n = 320) | 0.43 / 0.46 | 0.35 / 0.37 | 0.61 / 0.66 |
| Bruss-300 (n = 600) | 0.49 (cheapest-run; production's frontier is invalid there) | 0.45 | 0.61 |
| HIRES | 0.71 / 0.73 | 0.74 / 0.77 | 0.88 / 0.91 |
| Robertson | 0.69 / 0.65 | 0.84 / 0.78 | 1.02 / 1.07 |
| van der Pol | 0.62 / 0.64 | 0.80 / 0.83 | 0.83 / 0.86 |

Attribution, by geometric mean of the frontier ratios:

- On Bruss-50 the projected stop gives 0.35x and the coupled target 0.79x.
- On Bruss-300 the coupled target gives 0.38x: it also removes most linear failures.
- The predictive controller gives 0.83x on van der Pol, 0.94-0.96x on the Brusselators and HIRES, and 1.00-1.04x on
  Robertson, PR-forced and quadratic-4.
- The duplicate-residual fix is 8 JVPs per attempt, which is most of the small-n gain.
- Uncertified absolute forcing (INO-FORCE-ABS), stacked on top of the coupled target, saves 0-2 % and breaks the
  accuracy gate on HIRES (up to 8.7x). It is closed as a layer.

Two scope limits decide where this stack matters:

- **Declared structure.** When a band is declared, the existing banded direct arm (SPD03) beats S by 15-27x (Bruss-50),
  72-132x (Bruss-160) and 215-273x (Bruss-300) in flops.
- **Small n.** For n <= 8, dense LU is 2-7x cheaper than any Krylov arm.

So the matrix-free improvements matter for problems that declare no structure, and for systems too large or too
irregular to factor. At the system level the larger lever is routing: use a direct arm whenever a pattern or band is
declared.

### 2.3 Predictive step-size control: fewer rejections and less work

The production integral controller rejects 15-37 % of van der Pol attempts at loose tolerances, against 1-14 % for
Hairer's RODAS. It also has no growth cap after a rejection (F-078, open).

Probe B5 used the dense replica (27 of 28 cells exact against Rust), 17-29-point ladders, 4 initial-step seeds and
both scoring rules. Results for the Hairer/Gustafsson predictive form (`min` of the classical and predictive factors)
with a post-rejection cap:

- **Rejections:** van der Pol 28.6 % -> 7.2 %, Bruss-50 13.8 % -> 6.0 %, HIRES 1.5 % -> 0.7 %.
- **Work at matched accuracy** (attempts = LU; geometric means over E):
  - van der Pol 0.68-0.84;
  - HIRES 0.93 (worst E 0.99);
  - Bruss-50 0.93;
  - Robertson 1.01 (worst E 1.03);
  - matrix-free Bruss-160 0.91 in JVPs.
- **Predictive term vs set-point.** The predictive term is the robust part. A uniform safety of 0.725 matches it on the
  Brusselator but makes HIRES and Robertson worse at tight E (up to 1.09 and 1.17). The default set-point should stay.
- **Noise.** Under tolerance-coupled stage targets the error-estimate noise is at most 5e-5 with zero accept/reject
  flips. Under uniform relative forcing the noise is O(1) and both controllers break. The predictive controller requires
  coupled targets.
- **Calibration risk.** At equal rtol the controller shifts the error calibration on some cells: Robertson 1e-10 is
  1.99-5.07x base across 3 seeds. The exact-solve twin under the same controller reproduces this, so it is the
  controller, not stage contamination. The Rust node needs a separate multi-seed equal-rtol calibration gate.
- **Closed or confirmed** (by the gates registered in phase 2):
  - CTRL-EXPANSIVE-GATE is closed: it fails all four keep criteria and fires on 41 % of rotating-nonnormal steps.
  - Same-state exponent adaptation, set-point-matched H211b/PI34 filters, Krylov-cost-aware step reduction,
    stage-prefix early abort, one-pass global-error feedback and stage-1 retry reuse are confirmed as under 5 %.
  - The catastrophe-only stage abort is confirmed as a guard: it catches every L-0049/L-0062 blow-up at stage 4-5 with 0
    false aborts in 8,021 attempts.
  - The automatic initial step gains 1-12 % at matched accuracy under the predictive controller. That is a harness
    choice, not a method gain.

### 2.4 Stress tests: two regimes where the stack fails, and the amended stack S'

The completeness critic stressed S on regimes no probe had covered. S held, and usually gained, on:

- stiff-oscillatory blocks (omega 1e2-1e4);
- forced oscillators with a resolved carrier;
- E-05 nonnormal operators (n = 256): 0 linear failures against 31-53 for production; 0.43-0.60x JVP at matched accuracy;
- long van der Pol and Brusselator runs;
- switched forcing.

Where it gained nothing or failed:

- **Strongly nonnormal operators** (VIG-A02-type blocks, k = 20, ||W^-1|| about 4.9e5 in the error-weighted norm):
  - S is 40-1,598x base error and costs 1.4-3.3x the JVPs.
  - The coupled target is the cause: without the predictive controller the arm is just as bad.
  - The residual-to-error constant that the target assumes (||W^-1|| <= 1/(1 - h gamma mu) with mu <= 0) is violated
    by five orders of magnitude.
  - Production survives only because it over-solves.
- **Long-span badly scaled run** (Robertson to t = 4e10):
  - S livelocks at all 8 rtols with 2,549-6,935 linear failures.
  - The stagnation guard (q >= 0.98) aborts 50-80 % of solves that would have converged, at residuals 1-14x above the
    stall threshold. The 16-eps attainable-accuracy floor underestimates the round-off floor of the badly scaled
    operator.
- **Misdeclared span.** A driver told the span is T/100 (for example, called once per output interval) loses up to 66x
  accuracy on HIRES.
- **Finite-difference JVPs:**
  - As calibrated, every arm livelocks, production included.
  - With an FD-aware floor (1e-8) S works at rtol >= 1e-9 on the Brusselator.
  - It still has 4-31 failures per run on HIRES, Robertson and van der Pol, where a production-form FD arm has 0-2.
- **No gain at omega = 1e4:** S costs 0.97-1.11x the maxit-2000 rival.

The critic's amended stack S' adds three rules (designed after seeing these data, so they are pilots):

1. **Production fallback.** Before a guard abort or maxit failure becomes a rejection, accept the iterate if the
   unscaled true residual meets production's own rule. On Robertson to 4e10 this gives 0 failures at all 8 rtols, with
   accuracy equal to the exact twin.
2. **Nonnormality guard.** Estimate nu = 1/sigma_min of the Givens-triangularized Hessenberg factor. When nu > 1, tighten
   the target by nu.
   - On VIG k = 20 this restores accuracy (0.26-1.03x the exact twin) at 1.08-1.55x base JVPs.
   - Elsewhere it costs +0-0.4 % JVPs (+3 % on E-05 s = 1).
3. **Span contract.** Use the per-unit-step budget only with the true remaining span. Otherwise fall back to a per-step
   budget of 1e-4, which passed A1's gates.

The finite-difference variant stays on HOLD.

### 2.5 Alternative method families: real but narrow niches

All three were tested against the improved matrix-free arm and against a sparse or banded direct arm with colored
Jacobians.

**ROCK4 hand-off (REG-EXPLICIT-TRIAGE, probe B2)**

- The switch as first specified (shadow cost referenced to the matrix-free arm) fires on every corpus family and loses to
  the direct arm on 5 of 6 by 1.2-10x. Its wins were against a comparator that is itself 4-158x the direct arm.
- A direct-referenced variant (`swd`) wins in two places:
  - 2-D Brusselator grids with no declared band: 0.08-0.38x direct;
  - the semilinear family at mid accuracy: 0.43-0.84x.
- Where `swd` does not fire, the overhead is 1.00-1.13x.
- It fails its own misroute gate (<= 1.25) at the loosest target on van der Pol (1.52), semilinear-96 (1.48) and
  rotating (1.47, cheapest run).
- A 16-column field-of-values step bound (Crouzeix-Palencia) keeps ROCK4 power-bounded on nonnormal advection.
  Without it, ||R^k|| reached 1e31.
- **Verdict: opt-in candidate; scope narrowed.**

**Exponential Rosenbrock at its own step size (EXPRB-OWN-H, probe B3)**

- PEXPRB54S4 with KIOPS-type phi actions and the coupled tolerance rule, against the improved matrix-free arm at matched
  accuracy:
  - rotating-nonnormal 0.47-0.71x;
  - semilinear 0.20-0.77x;
  - forcing 0.56-0.62x.
- Its extended Jawecki-Auzinger-Koch defect bound held on every phi action where mu_2(J) <= 0 is structural (bound /
  actual 1.0-2.2).
- Rivals dominate it elsewhere: the direct arm wherever a pattern is declared (1.8-14x cheaper), and ROCK4 or the
  guarded switch on real spectra.
- Rotating-nonnormal is the only family where it is the best admissible matrix-free choice.
- Both runtime admission screens failed: the shadow rule and a Jacobian-drift indicator.
- **Verdict: opt-in branch for declared semilinear, nonnormal, matrix-free problems; not an automatic switch.**

**Linear-part preconditioner (from PC-LAGGED-HWINDOW, probe B4)**

- The lagged full-Jacobian preconditioner as specified is never the cheapest arm in any 2-D cell.
- A preconditioner built only from the declared linear part (P = I - h_P gamma A, factored once per h-window in [1/2, 2])
  keeps 5-7 columns per stage at every grid size, with 0 failures in 467 runs.
- Against sparse direct LU on 2-D multi-species grids it is 0.64x at N = 128 and 0.33x at N = 256 in flops; in timed
  kernels 0.88x and 0.60x.
- It loses on the scalar semilinear family up to N = 256.
- Prerequisites: a sparse-LU kernel and right preconditioning (production preconditioning is left-only).
- Using any preconditioner voids the shift-invariant Krylov reuses.
- **Verdict: low-priority opt-in node.**

### 2.6 Self-verification without speed: two report fields

**GE-AMP-DETECT (probe B6).** This transports a linearized global-error estimate with the exact RODAS5P step map on
y' = J_n y and reports A_max = ||ge|| / sum of err_k.

- It separates the F-033 semilinear rows from the rest. Its confusion matrix against true amplification above 4 is
  7/0/0/32.
- The cheaper (I - h gamma J)^-5 propagator fires falsely on oscillatory stress families and is closed.
- **Verdict: opt-in report field, not a certificate and not a speed lever.**

**CERT-OSA-STALL (judged in phase 2, not re-probed).**

- The online first-order one-sided acceptance err_c +- B_e(h mu_hat) held with zero violations under exact mu_D, at
  +5-25 % cost.
- Its stall-acceptance speed claims were withdrawn: a larger Krylov budget does the same job.
- **Verdict: robustness-only, and only where mu is declared or certifiable.**

### 2.7 Closed in this cycle

| Candidate | Reason |
|---|---|
| KRY-SHARED-SPACE (one exact-image Krylov space for all 8 stages) | The JVP saving is real (closed loop 0.58-0.65x of proj-stop), but orthogonalization makes it 2.8-3.1x (Bruss-50) and 5.5-5.8x (Bruss-160) the flops of proj-stop. At small n it is dense assembly by n JVPs. Reopen only for expensive JVPs (above about 500-1,700 flops per state component) with an Ir gate |
| PC-SECANT-WOODBURY | 1.6-7x production and 5.6-11x proj-stop in flops; partial coverage adds failures |
| KLEGAL-ROK4-GATED | Under plain projection ROK4a is worse than RODAS5P on Bruss-160; dominated by ROCK4 on real spectra |
| SOLVE-LIMITED-SWITCH | A maxit-200 artefact; its trigger stops firing once the budget is raised; the remnant lives in the ROCK4 switch |
| INO-FORCE-ABS as a layer | Subsumed by the coupled target; 0-2 % extra saving with accuracy failures |
| CTRL-EXPANSIVE-GATE | See 2.3 |
| lagged full-Jacobian preconditioner | See 2.5 |

### 2.8 Side findings

- **F-033 amplification measured.** The v3 addendum said the amplification factor of the semilinear family "was not
  measured". Probe A2 measured G_2 = 210 (G_w = 596) over t in [0.44, 0.73]. That explains how accepted local errors of
  at most 1.36 units become 17-207 units globally. No other corpus family has G above 2.3.
- **Stored reference error.** The stored van-der-pol-ramped (n = 96) reference is off by 8.8e-6 at tight tolerance. Probe
  B2 rescored those cells against a fresh 1e-13 reference.
- **The repo's literature gap is filled where it matters.**
  - RODAS5P is W-order 2 (embedded 1) and K-order 3, so any stale or Krylov-projected operator collapses its order.
  - No 5th-order one-step W or K method was found.
  - Hence matrix-free reuse is legal only within a step (same W) or as a preconditioner, which is the design of every
    surviving candidate.
- **Most surviving levers are textbook.** In-cycle GMRES stopping (SPGMR), tolerance-coupled linear targets (CVODE's
  eps_L), Hairer's predictive controller, Lang-Verwer error transport, linear-part preconditioning. Their contribution
  here is adoption evidence under this repository's contracts, and they should be credited that way.

## 3. Disposition of the 16 phase-2 candidates

| Candidate | Phase-2 verdicts (repro / math / repo) | Phase-3 result | Disposition |
|---|---|---|---|
| KRY-PROJ-STOP | survives (all three, with corrections) | inside S; unsafe without the coupled target | **Tier 1** (with 2.1) |
| INO-FORCE-ABS | survives with corrections | subsumed by the coupled target; breaks HIRES | closed as a layer |
| CERT-OSA-STALL | survives with corrections | not re-probed | Tier 2, robustness only |
| KRY-SHARED-SPACE | survives / survives / refuted | — | closed (flops) |
| PC-SECANT-WOODBURY | survives / contested / refuted | — | closed |
| PC-LAGGED-HWINDOW | survives / survives / contested | full-J never cheapest; linear-part variant wins on 2-D N >= 128 | Tier 3 (linear-part variant) |
| KRY-BUDGET-PREDICT | survives with corrections | maxit 2000 + guard; guard needs the production fallback | **Tier 1** (in S') |
| SOLVE-LIMITED-SWITCH | survives / contested / refuted | — | closed (absorbed) |
| CTRL-PRED-CAP | survives with corrections | passes every PASS line, no KILL line | **Tier 1** |
| CTRL-EXPANSIVE-GATE | contested (all three) | fails all four keep criteria | closed |
| CTRL-NULLS | survives | (a)-(f) and catastrophe abort confirmed; (g) is a harness lever | recorded nulls |
| DENSE-OWNW | survives with corrections | not re-probed | Tier 2 |
| GE-AMP-DETECT | survives with corrections | exact-map variant separates F-033; res5 closed | Tier 2, report field |
| REG-EXPLICIT-TRIAGE | survives / survives / contested | direct-referenced switch wins on 2-D without band and semilinear mid accuracy | Tier 3 |
| EXPRB-OWN-H | survives / survives / contested | wins on rotating-nonnormal, certified where mu_2 <= 0 is structural | Tier 3 |
| KLEGAL-ROK4-GATED | survives / refuted / contested | — | closed |

## 4. Proposed next cycle (to be preregistered; nothing below is registered yet)

Each node must:

- be registered before its code;
- gate JVPs together with Ir and the orthogonalization counters;
- score matched accuracy by both rules;
- include the strongest cheap rival named here.

**Tier 1**

**N1. Coupled stage target with in-cycle exit (U-form matrix-free driver).**
- Arms: production; duplicate-residual fix; proj at the production target; `l2c`; `wE0.2`; `wE0.2` + nu-guard +
  production fallback (S' target).
- Cells: SPD07 (14); HIRES and Robertson at 1e-9 to 1e-11; the fixed-step ladders; VIG k = 10/20; E-05 s = 0/1;
  Robertson to 4e10.
- Gates:
  - error <= 1.5x the exact-solve twin;
  - contamination <= 0.5;
  - ladders equal to direct LU;
  - no new failures;
  - Bruss-50 JVP <= 0.35x production.
- An FD-JVP arm is reported but not gated.

**N2. Predictive controller with post-rejection cap.**
- Drivers: dense and matrix-free.
- Arms: I, I725, PRED, PRED+cap, H211b, PI34; 4 seeds.
- Predictions (from 2.3): van der Pol 0.68-0.84, HIRES 0.93, Bruss-50 0.93, Robertson about 1.01.
- Gates: as N1, plus a separate multi-seed equal-rtol calibration gate that includes Robertson 1e-10.

**N3. Stage budget 2,000 with stagnation guard and production fallback.**
- Cells: Bruss-160 at 1e-4, Bruss-300/500, Robertson to 4e10, stiff-oscillatory omega = 1e4.
- Gates:
  - bitwise identity where the budget never binds;
  - guard false aborts counted;
  - 0 livelocks.

**Tier 2**

- **N4. GE amplification report field** (exact-map transport, threshold 4, validity guard), on held-out rows.
- **N5. Dense output through the step's own factorization** (DENSE-OWNW Arm A), against Enforce + max feed, at matched
  max-grid error.
- **N6. Online one-sided acceptance certificate**, logged and checked offline against an exact frozen-J chain, on
  declared-structure problems only.

**Tier 3 (opt-in branches; each needs a corpus family in its niche first)**

- **N7. Direct-referenced ROCK4 hand-off** with the field-of-values step bound, on 2-D families without a declared band.
- **N8. PEXPRB54S4 at its own step size** on declared semilinear nonnormal problems at n >= 384, certified where mu_2 <= 0
  is structural.
- **N9. Linear-part preconditioner** on 2-D multi-species grids with N >= 128; needs a sparse LU and right
  preconditioning.

Two decisions belong to the owner, not to a node:

- **Router.** Whether the default solver should route to a direct arm whenever a sparsity pattern or band is declared.
  The evidence says this dominates every matrix-free improvement where it applies.
- **Corpus.** Whether to add calibration families in the Tier-3 niches: 2-D multi-species reaction-diffusion, and a
  nonnormal semilinear family at n >= 384. Without them, Tier 3 cannot be gated on-contract, and the holdout rules
  (`docs/HOLDOUT_HYGIENE.md`) forbid tuning on the holdouts.

## 5. What cannot be claimed

- Any Rust-level result. Every number is from a replica; Ir was not measured. Flops are hand-count models; only probe B4
  timed kernels.
- Any wall-time effect (timing authority on HOLD).
- Generality beyond the tested problems. Large-n evidence is the 1-D and 2-D Brusselator, corpus v2 at n = 96 to 1536,
  E-05 and synthetic stress operators. There is no DAE or mass-matrix evidence (the matrix-free U form refuses M), no 3-D
  evidence, and no expensive-JVP or GPU evidence. Section 3 of the critic report (`pilot/phase3_reports/critic.md`)
  lists which recommendation each gap could overturn.
- That the S' remedies are validated. They were designed after seeing the stress data.
- That Theta = 0.2, the guard thresholds or the switch ratios are anything but calibrations on the problems above.

## 6. Replica fidelity

| Replica | Check against recorded Rust | Result |
|---|---|---|
| Matrix-free U form (MGS2) | SPD07 `BASE.json` | Exact in attempts, rejections, JVPs, iterations, dots and RHS on van der Pol, HIRES, Bruss-50, Bruss-160, PR-forced and quadratic-4 at both rtols. Robertson JVPs within 0.3 % (1523 vs 1528; 3622 vs 3626), steps exact |
| Dense fast driver | `sb_fast.json` | 27 of 28 cells exact; van der Pol 1e-3 is 180/114/66 against Rust's 181/114/67 |
| Corpus v2 transcription | Stored references and Rust campaign rows | Every family within its stored uncertainty. Attempts equal in 15/18 (n = 96) and 14/18 (n = 384, 1536) rows; the exceed set (error > 1) agrees 18/18 |
| ROCK4 | OrdinaryDiffEq transcription of `rock4.f` | Order and consistency checks on all 50 degrees. The earlier phase-2 port evaluated every stage at t_n; that bug was found and fixed |

No new arm (proj-stop, coupled target, predictive controller, guard, ROCK4, PEXPRB54S4 at its own step, linear-part
preconditioner) has Rust counters yet.

# Tier-1 execution (2026-10-08): ALG01-ALG03

The user approved the plan of section 4. Nodes N1-N3 became ALG01-ALG03. Each followed the same order:

1. registration, pushed before any code (`b0d5ea3`);
2. a test-only commit and base exports recorded on the unmodified solver source;
3. implementation, all of it opt-in;
4. one recorded run per node with the registered command, gated by a checker.

**All three verdicts are FAIL.** Every default path is bitwise unchanged:
- `Integral` reproduces its base export on 276 of 276 dense cells.
- `Legacy` reproduces its base export on 65 adaptive cells and 18 ladder rungs, and SPD07's rows on 14 of 14.

| Node | Ledger | Verdict | Passed | Failed |
|---|---|---|---|---|
| ALG01 tolerance-coupled stage target with in-cycle exit, nu-guard and production fallback (`research/alg01_coupled_stage_target_20261008`) | L-0090, corrected by L-0093 | **FAIL** (items 2, 3) | Brusselator JVPs per accepted step: 0.31 / 0.26 (Bruss-50, inner products 0.11 / 0.07) and 0.45 / 0.51 (Bruss-160), against ProjL2's 0.47 / 0.30 and 0.73 / 0.62. Error within 1.5x of the dense twin on 29 of 31 cells, including HIRES and Robertson at 1e-9 and the guarded 32-block VIG k = 20 (1.00, 1.01 and 0.55x). Robustness item passes on 52 cells | Single-block VIG k = 20 is 37.1x the twin at 1e-4 and 4.4x at 1e-8. The fixed-step ladders cannot finish within the registered 200-column budget, and Legacy fails the same rungs (a registration defect). At the pilot's budget of 20,000 every rung is 1.00x LU |
| ALG02 predictive controller with post-rejection cap (`research/alg02_predictive_controller_20261008`) | L-0091 | **FAIL** (items 3, 5) | van der Pol attempts at matched accuracy 0.68-0.84x the Integral controller (5 of 5 E). Rejections 28.5 % -> 7.0 %. HIRES and Brusselator-50 never worse than 1.01x on the frontier. Matrix-free Brusselator-160 0.90x in JVPs, with linear failures 144 -> 81 | Robertson cheapest-run ratio 1.160 at E = 3.16e-9 (frontier 1.025; an error-threshold effect at one E). van der Pol at rtol 5.62e-7 exceeds the calibration bound on all 3 seeds (err/rtol 2.55-2.57 against 2.05-2.28) |
| ALG03 2,000-column budget with stagnation guard and production fallback (`research/alg03_stage_budget_guard_20261008`) | L-0092 | **FAIL** (item 4) | Bitwise neutral on 14 of 14 SPD07 cells. Completes Robertson to 4e10 at all three rtols, where the guard without fallback livelocks. 0 linear failures on Brusselator-160 and -300. Brusselator-300 1e-4: 0.655x B0's JVPs and 0.418x Rbig's, per trajectory | Robertson to 4e10 at rtol 1e-5 ends at 3.32x the twin's error (budget 2,000 without the guard: 5.77x) |

## What the Rust runs established

- **The Brusselator work cut of the coupled target reproduces the pilot** almost exactly (pilot 0.31 / 0.26 / 0.45 / 0.51). The gain on n <= 8 does not come from the target: it comes from not computing production's duplicate diagnostic residual, which is a programming-level change (L-0093).
- **The nonnormality guard is not enough for a single strongly nonnormal block.** It repairs the 32-block VIG k = 20 operator but not the single 2x2 block, where ||W^-1|| is about 5e5. A residual target needs a better residual-to-error bound there, or a fallback to over-solving.
- **The predictive controller is a robust work and rejection gain on van der Pol.** It fails only on two narrowly registered items. One is a cheapest-run threshold artefact at one E; the other is a real calibration shift on van der Pol at the tightest rung. Before adoption it needs an err = 0 cap and a calibration rule.
- **The production fallback is what makes a stagnation guard safe.** It removes the livelock. The remaining Robertson-4e10 error comes from accepting fallback iterates. With the guard off, budget 2,000 alone is worse (5.77x).

## Independent review

A reviewer who wrote none of the code checked the diff `b0d5ea3..4f423b2`, re-ran all three checkers (their outputs equal the committed RESULTS.json), ran a randomized probe of the staged solver (400 systems x 12 configurations), and verified every ledger hash. **No blocking finding.**

Four should-fix findings, all about attribution and disclosure. Each was answered by a correction appended to the node file, plus L-0093 for the ledger claim:
1. The nu-guard re-evaluates at later exits, which is a deviation from the registration.
2. The small-n passes are solver mechanics, not the target.
3. The ALG01/ALG03 checkers were committed together with their runs.
4. ALG01 item 3 was infeasible at the registered budget for every arm.

Minor findings, recorded in the node files and to be fixed in any follow-up node:
- the err = 0 cap bypass and the sliver rejection-flag reset in `PredictiveCapped`;
- the guard's overrun prediction uses the nominal cycle length;
- `I725` scales rejection proposals too.

## Validation (head `4f423b2`)

| Step | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| clippy `-D warnings`: workspace, `audit2-research`, `audit2-bateman-authority`, `audit2-stage-certificate` | pass |
| `cargo test --workspace --all-targets` | pass (911 tests) |
| `cargo test -p rodas5p-integrators --all-targets --features audit2-research` | pass (575 tests) |
| `cargo test --workspace --profile measurement -- --ignored` | pass (66 tests) |
| `tools/check-audit2-readiness.sh`, `tools/test_*.py` | pass |
| `tools/check-research-node.py --base a793ecd` | pass (92 rows, 3 new nodes) |
| `tools/check-authority-refs.py` | pass |
| `tools/check_ignored_tests_in_ci.py` | pass (62 ignored tests reachable) |

The correction commit after this run changes only the node files, the ledger (L-0093) and this document.

## Next (not registered)

1. **A coupled-target node without the registration defects.** Ladder budget 20,000, nu computed as registered, the duplicate residual removed as its own arm, and the single-block VIG cell with a sharper residual-to-error rule or an over-solve fallback.
2. **A controller node with an err = 0 cap and a calibration rule**, scored only by the frontier at E inside the measured range.
3. **The guard's overrun prediction with the effective cycle length**, and fallback acceptance limited to iterates whose unscaled residual also meets the coupled target's error budget.

# Second tests (2026-10-10): ALG04-ALG06

The user approved registering the three follow-ups of the Tier-1 section. All three were registered before any code
(`b8964fc`), with the checkers committed before the recorded runs. ALG04 and ALG06 reuse the base exports of ALG01 and
ALG03. ALG05 recorded a new base on fresh initial-step seeds.

| Node | Ledger | Verdict | What held | What failed or is not robust |
|---|---|---|---|---|
| ALG04: coupled target with small-system exhaustion, a DupFix attribution arm, ladder budget 20,000 | L-0094 | **FAIL** (item 2) | `vig1b-k20` at 1e-4 went from 37.1x to 1.00x the twin. Ladders 0.997-1.001x LU. The target's own work gain over ProjL2 on the Brusselators is a geometric mean of 0.738 (cells 0.61-0.87). Small n is within 0.91-1.006x DupFix | `vig1b-k20` at 1e-8 is 4.18x the twin; Legacy is 1.73x there |
| ALG05: predictive controller with the err = 0 cap and sliver-flag fixes, fresh seeds, frontier-only scoring | L-0095, corrected by L-0098 | **FAIL** (item 5, single-cell bound) | van der Pol 0.685-0.834 at 5 of 5 E. Worst frontier ratios 1.015, 1.033 and 0.969 on HIRES, Robertson and Brusselator-50. Rejections 28.3 % -> 6.8 %. Calibration fit ratios at most 1.56 | One van der Pol cell has err/rtol 65.17, where the reference arm I has 65.04. The two fixes never acted, because these cells have no interior output points |
| ALG06: guard with effective cycle length (G1), attainable-accuracy floor (G2) and fallback charge (G3) | L-0096, corrected by L-0097 | **PASS by the registered rule; item 4 not robust** | Every stress cell completes, with 0 linear failures. Brusselator-300 uses 0.418x Rbig's JVPs. G1 removed every Robertson overrun abort. G2 cut JVPs 19-26 % on Robertson | Under 1e-9 perturbations of h0 (unregistered reviewer reruns, 24 runs), B3 at Robertson-4e10 1e-5 is a median 2.39x the twin, and 15 of 24 runs are above 1.5x. G3 never acted |

## What the second tests established

- **DupFix attribution.** Omitting the duplicate diagnostic residual alone gives 0.63-0.89x Legacy's JVPs on small
  problems and about 0.98x on the Brusselators, with bitwise the same trajectory. This is a programming-level change.
  It can be adopted on its own, through its own promotion review.
- **The coupled target's own gain over the in-cycle exit is real on the Brusselators** (geometric mean 0.74). Its
  accuracy still fails on one strongly nonnormal single block at tight tolerance, a cell where every iterative arm,
  Legacy included, is above the twin.
- **The predictive controller replicated on fresh seeds** in work and rejections. Its calibration on the ladder fit is
  within 1.56x. The only failing item rests on an outlier that the reference controller shares.
- **The guard stack's accuracy on long badly scaled runs is not established.** The registered pass rests on one
  favourable trajectory. Any further guard test must gate accuracy over several h0 perturbations.

## Independent review

A reviewer who wrote none of the code reran all three registered exports at the merge head. The RUNS files came out
byte-identical, and the checkers reproduced the RESULTS files. The reviewer also confirmed:

- the git order (checkers committed before runs; ALG05 base before its source change);
- the bitwise reproduction of every earlier arm;
- the ledger hashes.

There was one blocking-for-claims finding: ALG06 item 4 passes by chance. It was answered with L-0097 and a correction
in the node file; the registered verdict is kept. There were three should-fix wording findings: the step sequences
differ, the word "real" is dropped, and L-0095's "mis-specified" became a factual statement in L-0098.

## Validation (head `d031a70`)

| Step | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| clippy `-D warnings`: workspace, `audit2-research`, `audit2-bateman-authority`, `audit2-stage-certificate` | pass |
| `cargo test --workspace --all-targets` | pass (950 tests) |
| `cargo test -p rodas5p-integrators --all-targets --features audit2-research` | pass (595 tests) |
| `cargo test --workspace --profile measurement -- --ignored` | pass (74 tests) |
| `tools/check-audit2-readiness.sh`, `tools/test_*.py`, `tools/check-research-node.py --base b8964fc`, `tools/check-authority-refs.py`, `tools/check_ignored_tests_in_ci.py` | pass |

The correction commit after this run changes only the node files, the ledger (L-0097, L-0098) and this document.

## Where the method-level program stands

Of the robustness-and-speed levers, these replicated in Rust:

| Lever | Result |
|---|---|
| Predictive controller | van der Pol 0.68-0.84x, rejections 4x fewer, in two independent seed sets |
| Coupled target on PDE-like matrix-free problems | 0.26-0.51x JVPs against Legacy; 0.74x against the in-cycle exit alone |
| Larger budget with fallback | removes failure cascades: Brusselator-300 0.42x Rbig |
| Duplicate-residual removal | 0.63-0.89x on small n, bitwise the same trajectory |

None is yet promotable as a default, for three reasons:

- **Nonnormal accuracy.** The coupled target's accuracy on strongly nonnormal operators at tight tolerance is not
  resolved. A residual-based target needs a residual-to-error bound that the Krylov space does not reveal.
- **Single-cell bounds.** The controller's single-cell bounds are noise-limited. A distributional calibration gate
  would decide it.
- **Long badly scaled runs.** The guard stack's accuracy there is trajectory-sensitive.

Proposed next steps, not registered:

1. A promotion review of the duplicate-residual removal alone.
2. A controller node with a calibration gate relative to the reference arm, and output grids with interior points so
   the two fixes are exercised.
3. A coupled-target node whose accuracy item is gated over h0 perturbations, with an over-solve fallback when the stage
   residual stagnates relative to its own history.
