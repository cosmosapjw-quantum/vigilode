**PROBE B3 report: PEXPRB54S4 at its own step size (EXPRB-OWN-H) against the improved RODAS5P-MF arm**

This is an exploratory pilot. It feeds future preregistered nodes and has no ledger authority. I only read the worktree `/home/user/wt-speed` at a49f7e4: I built no Rust, and `git status` shows only the `tools/__pycache__/` that was there before. Everything is under `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/expo/`.

## Question

When PEXPRB54S4 runs at its own step size, with Krylov phi actions and its own controller, does it beat the improved matrix-free RODAS5P arm at matched accuracy in total admitted flops? If so:
- On which problems?
- Under what admission rule?
- Does the Jawecki–Auzinger–Koch defect-integral bound hold wherever mu_2(J) <= 0?

## Method

**Integrator** (`pexprb.py`):
- Tableau from exponential.rs:907-930.
- Non-autonomous problems are made autonomous exactly by carrying f_t as the u_2 column, so every action is on J itself. The order screen on a stiff problem started on its slow manifold gives global slopes 5.13 / 5.06 / 5.01 / 4.98 (autonomous) and 5.16 / 5.11 / 5.08 / 5.06 (non-autonomous).
- Four multi-output phi calls per attempt:
  - A: u = [0, f0, f_t] at tau = h/4, h/2, 9h/10, h.
  - B: u3 = 32 D2/h^2 at h/2 and 9h/10. This works because a32/c3^3 = a42/c4^3 = 32 exactly.
  - C: the main solution.
  - D: the error vector (b minus b-hat).
- Plus 3 remainder JVPs and 3 RHS evaluations.
- f0, f_t and J are reused after a rejection, and so is call A's Krylov basis.
- Controller: order-5 I controller, 0.9·err^(-1/5), clamped to [0.2, 5] after an accepted step and [0.2, 0.9] after a rejection, with the last-rejected-h cap. err = max(time error, phi error), as in adaptive_exponential.rs:290.
- Phi tolerance per call: the A1 EPUS rule, theta_n = 0.2·(h/T)·min(1, e_hat/0.5)^1.2 in WRMS units.

**Krylov phi engine** (`kiops.py`), following KIOPS (Gaudreault, Rainwater & Tokman 2018):
- Arnoldi on the augmented operator, using the exponential of the augmented projected Hessenberg.
- Saad first-term error estimate with KIOPS's error-per-unit-step test.
- Adaptive dimension m and step tau, substeps, and reuse of the basis on a rejected substep.
- Deviations from KIOPS:
  - full two-pass classical Gram–Schmidt instead of incomplete orthogonalisation;
  - happy breakdown only when h_{j+1,j} <= 100·eps·||A v_j||;
  - a squaring chain for the h/4 and h/2 outputs.
- Unit checks against dense augmented expm: 108 cases (symmetric, advection–diffusion, rotated 2x2 nonnormal blocks; n = 96 and 200), no case with error above 10·tol. Basis reuse costs 0 new JVPs at a smaller tau.

**Certificate.** I extended the JAK defect integral to the KIOPS augmentation:

  ||e(tau)|| <= ∫ ( |c(s)|·||q|| + ||U_flip a(s)|| ) ds

- Substep bounds add, since exp(sA) is contractive when mu(A) <= 0.
- The integral is computed by Simpson's rule on Q points of the exact small linear ODE.
- It is checked against the actual error of every phi output, computed by dense augmented expm.

**Cost counted** (lesson 1):
- Per operator application: F_jvp (+2n for the WRMS-scaled form) + 2pn for the augmentation.
- Orthogonalisation: 2(j+1) inner products and 2(j+1) vector updates per column, at 2(n+p) flops each, plus norms and scalings.
- Output assembly: 2jn per output.
- Small dense expm: Higham (2005) Padé model, (2·pi_m + 8/3 + 2s)·N^3, plus 2N^3 per squaring.
- Remainder JVPs, RHS and f_t evaluations, vector updates.
- Not counted: about 15n flops of WRMS scaling per attempt, under 0.5%.

JVP cost model, flops per component: F_jvp/n = rotating 34, forcing 28, semilinear 22, Robertson 8.33, HIRES 6, vdP 5.5, Brusselator 12, adv-diff 9.

**RODAS arms and rivals.** The `mf` and `base` arms are my copy of the B2 driver (`swdrv.py`); it reproduces the B2 rows bit-for-bit.
- `mf`: projected stop + coupled WRMS EPUS target + stall rule + maxit 2000.
- `base`: SPD07 production matrix-free arm.
- Context rivals from B2 or rerun here: direct (sparse LU, needs the sparsity pattern), ROCK4, and the guarded switched integrator `sw`.

**Scoring** (as in B2):
- Endpoint error, rescored from saved final states against one reference per problem:
  - exact solution for rotating, semilinear and adv-diff;
  - direct RODAS5P at rtol 1e-13 for rob/hires/vdp/forc;
  - NATIVE.json for Bruss-50; the B2 reference for Bruss-160.
- 15-point half-decade ladder from 1e-3 to 1e-10.
- [F] local regression frontier ("x" = extrapolated) and [C] cheapest run reaching E ("NR" = no run reaches it).
- Targets E_k = mf error at rtol 1e-k.

## Arms

| arm | definition |
|---|---|
| **E** | Primary arm, chosen before seeing results. KIOPS on D·J·D^-1 (WRMS norm), EPUS tolerance, mmax 100. |
| **E2** | Same as E but in the unscaled 2-norm with the F-043 conversion (sqrt(n)·min w); this is the norm the certificate can use. |
| **Emc** | E with a cost-balanced Krylov cap: mmax = clamp(ceil((2n + sqrt(4n² + 100·F_jvp))/50), 12, 100). |
| **Ed** | Exact dense phi (control: separates the method from the Krylov layer). |
| **Ef** | Fixed tolerance theta = 0.02 per step (control for the tolerance rule). |
| **Em40 / Em24 / Em16 / Em12** | Krylov cap ablation. |
| **E:0.3333, E:3 (and E2, mf)** | h0 seed variants. |
| **Ec / Esc** | Certification audits (2-norm with JAK bound; WRMS-scaled with actual errors). |

## Replica fidelity (item 6)

**RODAS arms, my copy against B2** (bit-identical):

| cell | attempts | JVPs | dots | flops | error |
|---|---|---|---|---|---|
| rot-96 mf 1e-6 | 142 | 8731 | 63028 | 76,499,040 | 3.4293 |
| hires-96 base 1e-6 | 34 | 11424 | 446080 | 186,270,720 | 2.8207 |
| bruss1d-50 mf 1e-6 | 92 | 9542 | 128332 | 73,715,200 | — |
| semi-96 mf 1e-8 | 66 | 5166 | 50246 | 37,250,784 | — |
| rot-96 sw 1e-6 | 170 | — | — | 54.654 M | 48.78 |

**B2's base arm against Rust SPD07 BASE.json**, exact:
- Bruss-50 at 1e-6: 92 attempts, 30666 JVPs, 726 RHS, 1,197,200 dots. At 1e-8: 193, 64561, 1540, 2,520,680.
- Bruss-160 at 1e-6: 92, 56906, 726. At 1e-8: 193, 78255, 1541.

**E side.** No Rust counters exist for PEXPRB54S4 at its own step size.
- Bruss-50 at 1e-6 with h0 = 1e-6 and dense phi: 87 accepted / 13 rejected, error 1.848e-6, the same as the earlier replica `hyp/regime/expo.py` (87/13, 1.85e-6). The KIOPS arm gives 87/13 and 1.819e-6.
- HIRES-8: 274 attempts, the same count as the candidate (error 4.10e-8 against 5.8e-8; the references differ).
- vdP μ = 1000 at 1e-6: 499 attempts (130 rejected), error 3.73e-5 against mf's 1.56e-6, i.e. 24x worse. This matches the candidate's "24–47x".

## Results

**T1. Ladders, Mflop @ endpoint tight-WRMS error, n = 96**

| problem | arm | rtol 1e-4 | 1e-6 | 1e-8 | 1e-10 |
|---|---|---|---|---|---|
| rot | base | 362.4 @ 794 | 919.5 @ 3.43 | 2732.5 @ 6.02e-2 | 7375.2 @ 7.66e-4 |
| rot | mf | 31.39 @ 794 | 76.50 @ 3.43 | 204.11 @ 6.02e-2 | 525.44 @ 7.67e-4 |
| rot | E | 23.78 @ 788 | 56.52 @ 5.03 | 139.65 @ 3.94e-2 | 324.88 @ 4.00e-4 |
| rot | E2 | 28.64 @ 788 | 64.31 @ 5.04 | 148.62 @ 3.95e-2 | 352.29 @ 4.01e-4 |
| semi | base | 120.2 @ 1.29e4 | 210.4 @ 255 | 396.8 @ 3.83 | 926.0 @ 3.38e-2 |
| semi | mf | 19.31 @ 1.30e4 | 26.80 @ 256 | 37.25 @ 3.83 | 70.48 @ 3.37e-2 |
| semi | E | 14.48 @ 1.97e3 | 11.82 @ 96.7 | 20.64 @ 1.26 | 39.94 @ 1.34e-2 |
| forc | base | 564.8 @ 724 | 1570.3 @ 9.91 | 3829.9 @ 6.51e-2 | 9832.4 @ 7.86e-4 |
| forc | mf | 25.51 @ 724 | 70.77 @ 9.91 | 176.64 @ 6.51e-2 | 443.21 @ 7.86e-4 |
| forc | E | 31.32 @ 11.3 | 73.73 @ 0.358 | 188.63 @ 4.34e-3 | 478.39 @ 5.70e-5 |

E has 0 run failures and 0 linear failures. One non-finite KIOPS case (vdP μ = 1000 at rtol 1e-3) is now fed back to the controller as a rejection.

**T2. E/mf flops at matched accuracy, [F] / [C], targets = mf error at 1e-4 / 1e-6 / 1e-8 / 1e-9 / 1e-10**

| problem | E/mf, [F] / [C] at k = 4 / 6 / 8 / 9 / 10 | other arms, h0 seeds |
|---|---|---|
| rot-96 | .62/.76, .71/.90, .65/.68, .59/.64, .55/.62 | E2 .60-.80 / .67-1.02. Over 3 h0 seeds: F .55-.71, C .51-.90 |
| rot-384 | .48/.63, .59/.64, .54/.61, .52/.58, .49/.56 | E2 .53-.66 / .61-.75 |
| rot-1536 | .47/.60, .57/.61, .54/.59, .51/.57, .48/.55 | E2 .52-.63 / .59-.71 |
| semi-96 | .77/.63, .49/.44, .50/.55, .50/.58, .51/.57 | E2 .55-.85; seeds F .48-.77 |
| semi-384 | .41/.28, .28/.28, .36/.41, .40/.43, .42/.46 | — |
| semi-1536 | .34/.20, .20/.21, .27/.30, .32/.34, .36/.38 | — |
| forc-96 | .96/.96, .57/.67, .56/.66, .60/.66, .62/.69 | seeds at k = 4: .81-1.21 |
| hires-96 | 1.25/1.45, 1.16/1.31, .96/.98, .89/.94, .84/.91 | Emc at k = 4 / 6: 1.59 / 1.26 |
| rob-96 | 1.33/1.20, 1.02/1.20, 1.20/1.24, 1.16/1.47, 1.12x/NR | — |
| vdp-96 | 1.84/2.30, 1.76/2.20, 1.58/1.60, 1.45/1.23, 1.34x/NR | E2 1.15-1.67 |
| bruss1d-50 | .97/.96, .82/.83, .78/.90, .81/.94, .85x/NR | Emc 1.00-1.27 |
| bruss1d-160 | 1.51/1.06, .92/.82, .60/.59, .53/.55, .49x/NR | Em24 .80-.88 everywhere |
| advdiff-128 (G1) | 10.7/1.62, 9.16/1.14, 2.97/.79, 1.30/.70 | Emc .27-.41 both rules; Em12 .25-.38 |
| vdP μ=1000 (n=2) | 25-30 | — |
| HIRES-8 | 4.6-8.4 | — |

Notes on the right-hand column:
- vdP μ = 1000 is dominated by dense flops (87%). Its JVP ratio is 1.5–1.9.
- HIRES-8 is also dense-dominated; its JVP ratio is 0.62–0.87.

Against SPD07 base, E/base is 0.03–0.16 on the n = 96 corpus families.

**JVP-only view.**
- E/mf JVPs: 0.20–0.55 on semilinear, rotating, forcing, Brusselator and adv-diff; 0.55–0.83 on HIRES and Robertson; 0.82–1.29 on vdP.
- With JVP and RHS cost x10: rotating 0.45–0.57 [F], semilinear-96 0.36–0.49, forcing 0.41–0.69, HIRES 0.66–0.91, vdP 1.02–1.37.

**T3. Attribution at equal rtol 1e-8 (E/mf), and controls**

| problem | attempts | endpoint error | flops per attempt |
|---|---|---|---|
| rot-96 | 0.90 | 0.65 | 0.76 |
| rot-1536 | 0.90 | 0.61 | 0.66 |
| semi-96 | 0.88 | 0.33 | 0.63 |
| semi-1536 | 0.91 | 0.66 | 0.32 |
| forc-96 | 1.22 | 0.067 | 0.88 |
| hires-96 | 1.13 | 0.93 | 0.87 |
| rob-96 | 1.01 | 2.83 | 0.97 |
| vdp-96 | 0.95 | 10.5 | 1.07 |

The gain comes from two sources:
- lower cost per attempt: about 30 operator applications per attempt against mf's 45–200 JVPs;
- a better global error per unit of tolerance on the families where RODAS5P under-delivers (forcing, semilinear; this is the F-033 effect).

Controls:
- **Ed:** attempts equal E's in every family except vdp-96. In the WRMS-scaled norm there, the phi errors meet their tolerance (max actual/tol 3.4) but the stage-error transfer through the remainders amplifies them, giving 10x the error of Ed. E2 and Ed agree on vdp-96.
- **Ef (fixed theta 0.02):** 10–35% cheaper on rotating, semilinear, forcing and Brusselator. It fails on vdp-96 (3.0–4.8x mf) and rob-96 (1.2–1.9x).
- **Krylov cap:** decisive when Krylov dimensions are large. Adv-diff goes from 10.7 to 0.25–0.38 with mmax 12. No single static cap is best everywhere: Bruss-50 goes from 0.78–0.97 to 1.00–1.27 with mmax 12.
- **Cost share** of E at 1e-6:
  - dense expm: rot-96 0.16, semi-96 0.29, Bruss-160 0.57, adv-diff 0.90, rot-1536 0.01;
  - orthogonalisation: 0.31–0.62.

**T4. Stronger cheap rivals, E/rival flops at matched accuracy [F]**

| problem | E/direct (needs sparsity pattern) | E/ROCK4 | E/sw (guarded switch, B2) |
|---|---|---|---|
| rot-96 | 2.1-3.1 | 2.1-2.4 | .42-1.20 (C .48-1.62) |
| rot-384 | 1.85-2.6 | 1.6-2.1 | .45-1.03 (C .43-1.16) |
| rot-1536 | 1.8-2.5 | 1.3-2.0 | .45-1.01 (C .42-1.13) |
| semi-96 / 384 / 1536 | 3.1-14 | 10-29 | 1.6-7.4 |
| forc-96 | 3.9-6.7 | 1.6-3.8 | 1.22-1.39 |
| bruss1d-160 | 22-239 | 22-145 | 19-26 |
| advdiff-128 | 100-5000 | 13-430 | mixed |

Wherever a sparsity pattern is declared, the direct arm dominates E. ROCK4 is also cheaper wherever it is admissible. Rotating-nonnormal is the only family where E beats the guarded switch, which the judge requires for nonnormal admission (lesson 12); it wins 3 of 5 cells at every n tested.

**T5. Certification audit**

2-norm KIOPS, every phi output compared with dense expm:

| run | steps | mu_2(J) max | actions | violations above rounding floor | bound/actual (min / median / max) |
|---|---|---|---|---|---|
| rot-96 1e-4 / 1e-6 / 1e-8 | 51 / 143 / 380 | -6.25 | 448 / 1168 / 3048 | 0 | 1.002-1.017 / 1.04-1.26 / 1.12-1.77 |
| forc-96 1e-4 / 1e-6 / 1e-8 | 101 / 279 / 744 | -27.05 | 920 / 2328 / 6008 | 0 | 1.004-1.018 / 1.04-1.19 / 1.11-2.18 |

- No action has bound/actual above 1e2 (so none above 1e4).
- The raw "violations" (12–1110 per run) are all at the rounding floor: actual about 1e-17 against a bound of about 1e-28. The bound has no rounding term.
- The declared-structure bound mu_struct = max_k s_k·lambda(eta) + max_i 2·nl·y_i (rotating), or the maximum diagonal (forcing), costs O(n) and equals mu_2 exactly on 100% of steps. So a matrix-free run with declared structure certifies these two families.
- **WRMS-scaled norm:** mu(D J D^-1) <= 0 on only 15% of rotating steps (maximum +9319) and 100% of forcing steps. The scaled-norm certificate is not usable on rotating.
- **EstimateOnly cases:** semi-96 (mu_2 > 0 on 100% of steps, maximum 37.5) and hires-96 (100%, maximum 19.9). On Bruss-50 mu_2 <= 0 on only 9% of steps.
- **Overhead** (rot / forc):

| quadrature points Q | 8 | 16 | 32 | 128 |
|---|---|---|---|---|
| overhead | 0.21 / 0.30 | 0.21 / 0.31 | 0.24 / 0.34 | 0.47 / 0.61 |

  No violation appears at any Q.
- Certified E2 against mf on rotating at n >= 384: about 0.63–0.85.

**T6. Runtime admission screens: both failed (negative results)**

- **Shadow rule.** At each accepted mf step, run one E2 attempt at the same (t, y, h) and form rho_hat = (C_E/C_R)·(err_E/err_R)^(1/5). It admits only semi-384 (rho_hat 0.73–0.76) and rejects rot, forc and semi-96 (1.09–1.98), where E actually wins. The gain is in global error per unit of tolerance, which local error estimates do not see. The consistency check kappa = ||y_E − y_R||_w / max(err) stayed at or below 0.17 (median) everywhere, including vdP.
- **Jacobian-drift indicator** gD = h·||D4||_w / ||U4 − y||_w. Medians and 90th percentiles overlap between winners and losers: forcing q90 0.70 and adv-diff 0.94 against Robertson 0.36 and HIRES-96 0.08. It flags vdP-type problems only (q90 0.47–28).

**Judge's criteria**
- PASS (<= 0.6x on rotating against proj+INO): met at n = 384 and 1536 on the frontier (0.47–0.59), but not at n = 96 (0.55–0.71 [F], 0.62–0.90 [C]) and not on the cheapest-run rule at n >= 384 (0.55–0.64).
- KILL (>= 0.8x on both rotating and semilinear): not met; semilinear is 0.20–0.77.
- No bound below the actual error where mu_2 <= 0 (above the rounding floor), and no action with bound/actual above 1e4.
- Error above 3x RODAS5P's at the same rtol, undetected: occurs on vdp-96 with the scaled norm (10.7x) and on rob-96 (2.8–3.1x). Both are families the admission rule excludes.

## Decision: PROMOTE, scoped to an opt-in matrix-free branch, not an automatic switch

Reasons:
- Against the specified comparator (the improved RODAS5P-MF arm), E is cheaper at matched accuracy under both rules, across 3 h0 seeds and every n tested, on the semilinear families:
  - rotating 0.47–0.71 [F];
  - semilinear 0.20–0.77;
  - forcing 0.56–0.62 at k >= 6;
  - Bruss-160 at rtol <= 1e-8: 0.49–0.60.
- The certificate is valid and tight where mu_2 <= 0 is structural.
- The scope is narrow, because stronger rivals dominate most of this regime:
  - the direct arm whenever a sparsity pattern is declared (1.8–14x cheaper than E on the corpus);
  - ROCK4 or the guarded switch on real or near-normal spectra (semilinear, forcing, Brusselator).
- The only regime where E is the best admissible matrix-free choice in this probe is rotating-nonnormal.

Sub-claims I close:
- the free level-1 actions from RODAS's stage-1 basis (not used at its own step size, and voided by PC-LAGGED);
- a Bruss-50 gain (0.78–1.27, i.e. parity);
- runtime shadow (zeta34-type) admission;
- the WRMS-norm certificate on nonnormal problems;
- any use on vdP or kinetics, including the harness problems with n <= 8, which are 5–34x worse.

## Recommended rule and algorithm for the preregistered node

**Admit E only if all of the following hold:**
1. The run is on the matrix-free path with no declared sparsity pattern or band (otherwise use the sparse or banded direct arm).
2. The problem declares semilinear structure f = L(t)y + N(t, y), with the stiffness carried by L. A runtime check could not detect this.
3. The REG field-of-values guard refuses ROCK4: a nonnormal or complex field of values.
4. Revert to RODAS-MF if the Krylov cap binds on more than 50% of calls or if dense plus orthogonalisation exceeds 0.6 of E's flops.

**Configuration:**
- PEXPRB54S4 with the 4 multi-output KIOPS calls listed in Method.
- 2-norm KIOPS (E2) when a certificate is required; otherwise the WRMS-scaled form, which is 5–15% cheaper.
- EPUS phi tolerance (Theta 0.2, e_ref 0.5, p 1.2); mmin 4; warm m per call slot.
- Krylov cap: clamp(ceil((2n + sqrt(4n² + 100·F_jvp))/50), 12, 100). A per-call cost-aware choice of (m, tau) that includes the m^3 dense term is the open lever.
- Order-5 I controller with err = max(time, phi).
- Certificate: JAK integral with Q = 16 where mu_2(L) + max diag(N') <= 0 is certified in O(n) from the declared structure; otherwise label the result EstimateOnly.

**Predictions for the node:**
- Rotating-type problems at n >= 384: 0.47–0.59 [F] and 0.55–0.64 [C] against T1-A; certified, about 0.63–0.85.
- Against the guarded switch: 0.42–1.16.

**Kill line:** kill if E >= 0.8x of T1-A at n >= 384, or if E loses to the guarded switch on nonnormal families in the majority of cells.

## Threats to validity

- **Python replica.** There are no Rust counters for E at its own step size. Its fidelity rests on the earlier replica and on dense phi only.
- **Flop models.** The dense-expm cost is a 1-norm Higham-2005 model, an upper bound relative to SciPy's 2009 algorithm. The JVP cost model is a hand count. B2's direct, ROCK4 and sw rows are themselves exploratory replicas.
- **Error metric.** Only endpoint error is scored. The corpus contract is the maximum over the 101-point grid, and E has no dense output.
- **Matched accuracy.** Targets are mf errors. Ladder granularity is half a decade. Some cells are extrapolated ("x") or never reached (NR): Robertson, vdP and Brusselator at k = 10.
- **Sensitivity to tuning.** Results move with the Krylov cap (advdiff ranges from 0.25 to 10.7x) and the tolerance rule (Ef is 10–35% cheaper but fails on vdP and Robertson). The primary arm E was fixed before seeing results.
- **Certificate.** It uses quadrature with no rounding term, holds in the 2-norm only, and covers the phi-action error, not stage-error propagation through the remainders.
- **Narrow niche.** It rests on one corpus family (rotating-nonnormal). The forcing and semilinear gains partly reflect RODAS5P's error-estimator calibration (F-033), not larger steps.
- **Only 3 h0 seeds on 4 families**, and no wall-clock measurements (CPUs were shared).

## Files

All in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/expo/`:

| file | contents |
|---|---|
| `kiops.py`, `pexprb.py` | the engine and the integrator |
| `run_e.py` | job runner; arms; harness problems |
| `analyze.py` | analysis → `res/analysis.txt`, `res/summary.json` |
| `cert.py`, `cert_q.py` | certification audit → `res/cert.json`, `res/cert_q.json` |
| `shadow.py`, `gd.py` | admission screens → `res/shadow_E2.json`, `res/gd.json` |
| `diag_vdp.py` | vdP diagnosis → `res/diag_vdp.json` |
| `fid_rodas.py`, `fid_expo.py`, `test_kiops.py` | fidelity checks and unit tests → `res/fid_*.json`, `res/test_kiops.json` |
| `res/e_*.jsonl` | all runs |
| `rodas/` | copy of the B2 RODAS driver and references |