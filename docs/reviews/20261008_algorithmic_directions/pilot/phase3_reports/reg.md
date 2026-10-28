**PROBE B2 report: REG-EXPLICIT-TRIAGE (switching between RODAS5P and ROCK4) tested against the strong rivals**

This is an exploratory pilot for future preregistered nodes. It carries no ledger authority. I only read from `/home/user/wt-speed` at a49f7e4 (fixtures, BASE.json, corpus files). I wrote nothing there and built no Rust. Everything is in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/reg/`.

## Question
Does a switched integrator pay off once cost is counted in total flops at matched accuracy? The integrator runs RODAS5P by default, estimates ROCK4's cost per unit time with a shadow step, and hands off to ROCK4 when that estimate wins by 5x for 3 consecutive steps and a nonnormality guard passes. It is compared with three arms: the SPD07 matrix-free arm, the improved matrix-free arm, and a direct arm (sparse LU, coloured J). The problems are every corpus-v2 family, Brusselator-50/160 and a 2-D reaction-diffusion problem with no declared band.

KILL rule: the trigger fires on no corpus family, or the switched integrator loses to the direct arm everywhere it fires.

## Method

**ROCK4 (`rock4x.py`)**
- The coefficients and degree formula come from OrdinaryDiffEq's `rkc_tableaus_rock4.jl`, which transcribes Abdulle's `rock4.f`. I could not fetch `rock4.f` itself: unige.ch is blocked (403) and GitHub code search is not available here.
- Coefficient integrity checks on all 50 degrees:
  - R_s(z) = e^z + O(z^5), observed order within 0.065 of 5;
  - the embedded polynomial is e^z + O(z^4), coefficient ≤ 3.6e-3;
  - consistency error ≤ 2.4e-15;
  - order 4 on a non-autonomous nonlinear test (4.10–4.20).
- One defect in the published polynomial: at 152 stages |R| reaches 1.0077 near x = −8. No run here used more than 75 stages.
- Bug found in the earlier port (`hyp/regime/rock4.py`): it evaluates every stage at t_n. On the non-autonomous corpus that makes it effectively first order. On semi-96 at rtol 1e-6 it needs 10,614 f-evals for error 5.1e4, against 176 for error 84.5 with corrected stage times (`logs/oldport_check.log`).
- The power iteration for ρ uses safety factor 1.2 and is refreshed every 25 accepted steps and after every rejection.

**Switched integrator (`swdrv.py`), per accepted RODAS step**
- **Settle rule.** Costs are compared only on accuracy-limited steps, i.e. err ≥ (0.9/5)^5 = 1.89e-4.
- **ρ̂.** 1.2 × the Ritz radius from the stage-1 Hessenberg of D W D⁻¹, which is free.
- **RODAS cost per unit time.** c_R = flops since the previous acceptance ÷ h_R*, where h_R* = h·min(10, 0.9 err^-1/5).
- **Prescreen.** Skip the shadow if c_R is less than 5x ROCK4's best-case cost. After any ratio below 2.5, back off 1, 2, 4 … 64 steps.
- **Shadow step.** One charged ROCK4 step at h_R gives err_E, then h_E* = h·min(10, 0.8 err_E^-1/4).
- **Hand-off.** After 3 consecutive ratios ≥ 5, run the real power iteration and the guard. Hand off only if the ratio is still ≥ 5 with the step capped at h_FOV.
- **Reverts.** Over a 20-attempt window: rejection rate > 15%, or realised cost > 0.5 × c_R,ref (the median c_R of the 3 triggering steps). Also h_FOV = 0, or the stiff Ritz angle growing past max(0.3, 3 × its hand-off value). After a revert there is a cool-down of 10·2^k steps.

**Guard (`guard.py`): a field-of-values step bound**
- Basis: Crouzeix–Palencia gives ‖R(hA)^k‖ ≤ 2.41 (max over W(hA) of |R|)^k, with no normality assumption. A = D J D⁻¹ in the WRMS norm.
- A 16-column Arnoldi is run from D·b₁ plus a seeded random vector; all columns are charged.
- The boundary of W(H₁₆) is traced from 16 support directions, with imaginary parts inflated ×1.5 and the real extent pushed out to −ρ̂.
- h_FOV is the largest h such that max over ∂W of |R_s(h)(hz)| ≤ e^{hω₊}(1 + ln10·h/T). It is used as a step cap, not just as a veto.
- Calibration (`logs/fovstart.log`): starting from the stage-1 vector alone was not conservative on 2 of 8 operators. On advdiff-mild it gave h_FOV 160x too large; on semi-384 the compressed |Im W| was 67 against 119 exact. The mixed start fixed both.
- The Ritz angle is reported as a diagnostic only. It triggered no revert.

**Cost model (`res/costmodel.json`)**
- Flops per JVP / per RHS, per state component:

| problem | JVP | RHS |
|---|---|---|
| Robertson | 8.33 | 8.33 |
| HIRES | 6 | 6 |
| van der Pol | 5.5 | 5.5 |
| rotating | 34 | 114 |
| forcing | 28 | 28 |
| semilinear | 22 | 27.2 (25.1 at n = 384) |
| Brusselator 1-D | 12 | 11 |
| Brusselator 2-D | 12 | 11.5 |
| advdiff | 9 | 14 |

- One trig call counts as 20 flops.
- The Krylov operator costs JVP + 2n (shift), plus 2n for the scaled form.
- dot, axpy and norm cost 2n each; scaling costs n.
- LU and solve flops are exact for the fill SuperLU produces. I used MMD_AT_PLUS_A, the cheapest of the three orderings checked (nested dissection, MMD_AT_PLUS_A, COLAMD); on bruss2d-64 one factorisation costs 32.65 vs 57.87 vs 93.31 Mflop.
- J build costs (number of colours) × JVP, using the structural CPR colouring (5 for semilinear).
- ROCK4 vector work is 5n per recurrence stage + 33n + 7n per step.
- Guard dense work is charged as 16·9m³ + 25m³.

**Protocol**
- Closed loop: linear failures and non-finite stages are fed back as rejections, and every Krylov exit is confirmed by a true residual.
- Ladder: 15 half-decade rtols from 1e-3 to 1e-10 (bruss2d-64: decades only).
- Matched accuracy is scored two ways. (F) is a local log-log regression frontier; "x" marks extrapolation. (C) is the harness rule: the cheapest run reaching E; NR means no run reached it.
- Targets E_k are the direct arm's endpoint error at rtol 1e-k.
- Error rules: corpus = tight-WRMS of the endpoint against the exact solution (rotating, semilinear) or the stored reference. vdp-96 at rtol ≤ 1e-8 was rescored against a fresh 1e-13 reference, because the stored one is off by 8.8e-6 (`res/corpus_refs.json`). Brusselator, bruss2d and advdiff use max relative error with a 1e-10 floor.

## Arms
- **base**: SPD07 production. GMRES(40), Zero start, maxit 200, relative 1e-10 target.
- **mf**: the improved arm. Projected stop, outer-coupled WRMS target, stall rule, maxit 2000.
- **direct**: sparse LU with coloured J.
- **rock4**: plain ROCK4, the control arm.
- **sw**: the switched integrator as specified, with the improved matrix-free arm as the RODAS side.
- **swng**: sw with the guard off (control).
- **swd**: switched integrator with the direct arm as the RODAS side; ρ̂ from a Gershgorin bound. Added by the probe.
- **swd-r2**: swd with switching ratio 2.
- **-ns**: no-settle-rule sensitivity arms.

## Replica fidelity (`res/fidelity.txt`)

| check | result |
|---|---|
| base arm vs SPD07 BASE.json, Bruss-50/160 at 1e-6/1e-8 | attempts, JVPs, RHS and inner products identical in all 4 cells (e.g. 92 / 30,666 / 726 / 1,197,200). Vector updates 2.6% higher (counting convention). |
| direct arm vs Rust v2 campaign, n = 96 | attempts equal in 15/18 rows; equal to probe A2's replica in 18/18. Endpoint errors equal to 4 digits in 7/18 rows (the rtol-1e-4 rows differ because the Rust stage solves are inexact, as A2 found). |
| base arm vs Rust campaign matrix-free counters | attempts equal in 15/18; JVPs 13–23% higher (different inner configuration: restart 32, Previous start) |
| ROCK4 Bruss-50 | 957 f-evals at 1.632e-6 and 2,037 at 1.405e-8, reproduced exactly |

## Results

**Absolute work at rtol 1e-6** (Mflop @ endpoint error)

| problem | base | mf | direct | rock4 | sw | swd |
|---|---|---|---|---|---|---|
| semi-96 | 210.4@255 | 26.8@256 | 2.36@255 | 0.60@84.5 | 3.51@107 | 2.06@142 |
| semi-384 | 841.4@179 | 186.9@181 | 12.09@179 | 2.97@78.7 | 16.02@66.0 | 8.24@41.4 |
| forc-96 | 1570@9.91 | 70.8@9.91 | 10.37@9.91 | 10.74@44.1 | 19.65@60.7 | 10.67@9.91 |
| hires-96 | 186.3@2.82 | 6.75@2.84 | 0.91@2.82 | 0.42@45.7 | 1.84@45.7 | 1.01@2.82 |
| bruss1d-50 | 539.5@2.14e-6 | 73.7@2.14e-6 | 3.41@2.14e-6 | 1.73@1.63e-6 | 6.38@1.75e-6 | 3.50@2.14e-6 |
| bruss1d-160 | 3230@2.14e-6 | 1202@2.13e-6 | 10.92@2.14e-6 | 14.27@3.87e-6 | 43.7@9.67e-6 | 10.94@2.14e-6 |
| bruss2d-64 | 42180@1.30e-6 | 12175@1.31e-6 | 2673@1.30e-6 | 196.8@1.07e-5 | 655.4@1.86e-5 | 559.5@1.86e-5 |
| advdiff-128 | – | 290.7@6.4e-6 | 0.57@6.6e-6 | 6.20@6.6e-5 | 162.4@1.5e-8 | 0.57@6.6e-6 |

**Matched-accuracy ratios, F/C, at E₄ / E₆ / E₈ / E₁₀**

| problem | sw / direct | swd / direct | rock4 / direct | sw / mf |
|---|---|---|---|---|
| semi-96 | 5.58/2.60, 1.63/1.48, 0.80/0.76, 0.50/0.52 | 1.48/1.38, 0.84/0.82, 0.89/1.19, 1.07/1.07 | 0.38x/0.32, 0.23/0.22, 0.24/0.26, 0.30/0.33 | 0.39/0.19 … 0.08/0.08 |
| semi-384 | 6.67/1.79, 1.69/1.16, 0.62/0.63, 0.38/0.39 | 1.06/0.98, 0.72/0.68, 0.55/0.43, 1.21/1.09 | 0.43–0.20 | 0.25/0.08 … 0.06x |
| forc-96 | 4.83/3.95, 2.82/2.82, 2.88/3.39, 3.40x/NR | 1.08, 1.03, 1.01, 1.01 (never fires) | 1.5–2.6 | 0.41–0.70 |
| rot-96 | 4.00/4.77, 2.60/2.47, 2.94/5.32, 4.92/4.06 | 1.11/1.47, 1.03, 1.01, 1.01 | 1.0–2.3 | 0.44–0.85, 1.34 (C at E₈), 1.30x (E₁₀) |
| hires-96 | 4.0, 2.3–2.5, 1.5, 1.50x | 1.11, 1.11, 1.06, 1.02 | 0.72–0.93 | 0.17–0.50 |
| rob-96 | 3.1–3.8, 4.0–5.7, 2.4, 1.21x | 1.13, 1.12, 1.10, 1.00 | 0.69–1.54 | 0.17–0.70 |
| vdp-96 | 8.0–10.1, 4.9–5.2, 2.5–2.9, 1.8–2.5 | 1.52/1.43, 1.18/1.09, 1.01, 1.01 | 2.8–4.9 | 0.19–0.74 |
| bruss1d-50 | 3.99/3.37, 2.01/1.87, 0.97/0.99, 0.69/0.77 | 1.01–1.05 (never fires) | 0.47–0.78 | 0.06–0.12 |
| bruss1d-160 | 12.8/6.6, 4.0/3.7, 1.9/1.8, 1.11x | 1.00–1.01 (never fires) | 1.24–1.79 (E₁₀ 0.99x) | 0.02–0.08 |
| bruss2d-32 | 0.55/0.49, 0.32/0.31, 0.18/0.18, 0.14x | 0.37/0.38, 0.26/0.27, 0.17/0.17, 0.15x | 0.10–0.14 | 0.07–0.12 |
| bruss2d-64 | 0.51/0.33, 0.23/0.21, 0.13/0.13, 0.07x | 0.33/0.34, 0.22/0.26, 0.14/0.15, 0.08x | 0.06–0.12 | 0.04–0.08 |
| advdiff-128 | 484x/160, 232/123, 40/73 | 1.00 (never fires) | 6.5–14.4 | 0.10–0.50 |

- sw / base is 0.005–0.15 everywhere. mf / direct is 4–158 everywhere: the matrix-free arms are not the right rival at these sizes.
- Without the settle rule, sw / direct is 0.06–0.18 on bruss2d and 0.59–1.43 on bruss1d-50. Removing the rule also produced one large misroute: swd-r2 on advdiff became 8.4–19x the direct arm. That misroute is why the settle rule was adopted; it is a post-hoc protocol change (see threats).

**Trigger behaviour (fired / 15 rtols, median share of time on ROCK4)**
- sw: 15/15 on every corpus family. Time on ROCK4: semi 0.80, forc 0.81, rot 0.47, hires 0.94, rob 0.63, vdp 0.50. Also 15/15 on Brusselator-1D and bruss2d (≈0.99), and 5/7 on advdiff.
- swd: semi-96 9/15, semi-384 10/15, rot 3/15, vdp 3/15, bruss2d 15/15. Zero on forc, hires, rob, bruss1d-50/160 and advdiff.
- Reverts in sw: forc 19 on cost, 1 on rejection rate; rob 12 on rejection rate; vdp 15 + 2.
- Guard cost as a share of sw work: 0.4% (bruss2d-64) up to 37% (hires-96), because the m³ cost is fixed regardless of n.
- At equal rtol, err(sw)/err(mf) has median 5.7 on forc and 6.0 on HIRES. ROCK4 at Bruss-160 has err/rtol of 3.9 at 1e-6 and 11.6 at 1e-8, confirming its estimator under-reports.

**ROCK4 on nonnormal operators** (`res/nonnormal.json`; Q = ‖R^k‖ / max(1, ‖e^{khA}‖), k ≤ 300)

| operator | plain ROCK4 steps, h_med / h_p90 (Q) | guard h_FOV (Q) | exact-FOV h_FOV |
|---|---|---|---|
| advdiff a=5 | 2.81e-3 (164) / 3.90e-3 (1.8e31) | 2.10e-3 (1.00) | 2.10e-3 |
| semi-96 | 2.5e-2 (1.0) / 7.18e-2 (24) | 4.35e-2 (1.004) | 7.32e-2 (43) |
| semi-384 | 1.69e-2 (1.0) / 6.07e-2 (1.2e22) | 1.83e-2 (1.002) | 2.18e-2 |
| rot-96 | 3.2e-3 (1.0) / 1.47e-2 (9.1) | 9.15e-3 (0.82) | 0.319 |
| advdiff-mild | 1.65e-2 (1.41) | 1.83e-2 (1.61; optimistic 1.5x) | 1.19e-2 |
| bruss1d-50 | (1.0) | 26.2 (0.45) | 26.2 |

- The guard does not keep ROCK4 off advdiff. It caps the step at h_FOV, where the propagator stays power-bounded (Q ≈ 1). The run then costs 8.8–162 Mflop at error 1.5e-8 to 1.9e-8, against direct's 0.57.
- With the guard off (swng) on advdiff, the hand-off happens at 4.5x h_FOV and the run reverts on a 20% rejection rate, costing 120–500x direct. Plain ROCK4 there gives err/rtol of 100 at rtol 1e-4.
- swd stays off advdiff on cost grounds alone.

**Fault injection, ρ̂ × 0.25**
- Plain ROCK4: no silent failure on any of Bruss-50/160, bruss2d-32, advdiff or semi. Rejections rise to 20–25%, cost rises 3–19x, and errors get smaller.
- sw with only the power estimate faulted: results are identical to the clean run, because the 1.2·ρ_Ritz floor takes over.
- sw with both the power and Krylov estimates faulted: the guard (whose FOV still sees the true extent) caps h. Cost rises 1.6–10x, with 0 rejections and smaller errors.

## Decision: PROMOTE, narrowed
The KILL rule is not met.
- The trigger fires on all six corpus families (sw), and on semilinear, rotating and vdp (swd).
- The switched integrator beats the direct arm where it fires: semi-96/384 at E₈ and E₁₀ (0.38–0.80), bruss1d-50 at E₈ and E₁₀ (0.69–0.99), and bruss2d-32/64 at every target (0.07–0.55 for sw, 0.08–0.38 for swd).

What is promoted is narrower than the candidate as written:
- The matrix-free-referenced trigger as specified should not be a default. It loses to direct on 5 of 6 corpus families by 1.2–10x. Its "wins" are against a matrix-free comparator that is itself 4–158x direct at these sizes.
- The direct-referenced variant does not lose on the corpus. Where it abstains the overhead is 1.00–1.13, with exceptions of 1.52 at vdp E₄, 1.47 (C) at rot E₄ and 1.48 at semi-96 E₄. It gains 0.43–0.84 on the 2-D semilinear family at mid accuracy and 0.08–0.38 on 2-D grids without a declared band.
- On 1-D banded problems (Bruss-160: ROCK4 is 1.24–1.79x direct) and on nonnormal advection, the direct arm is the answer.

## Recommended rule (for a preregistered node)
Run RODAS5P with the cheapest available stage solve: sparse direct with CPR-coloured J when a pattern exists, otherwise the improved matrix-free arm. Then:
1. On every accepted step with err ≥ (0.9/5)^5, and outside cool-down or back-off:
   - c_R = flops since the last acceptance ÷ h_R*;
   - ρ̂ = 1.2·ρ_Ritz (matrix-free) or the Gershgorin bound ‖J‖∞ (direct);
   - prescreen against ROCK4 at min(4h_R*, stage cap); back off after any ratio below 2.5;
   - shadow ROCK4 step to get h_E* and c_E = 1.05·(s·F_f + vector work)/h_E*.
2. After 3 consecutive ratios ≥ 5: run the power iteration, set ρ̂ = max(power, 1.2·ρ_Ritz), compute h_FOV from the 16-column guard (as above), and hand off only if the ratio is still ≥ 5 at min(h_E*, h_FOV).
3. On ROCK4: rock4.f controller; step ≤ h_FOV; refresh the guard whenever branch work since the last check is at least 20x the guard's cost.
4. Revert on (window of 20) rejection rate > 15%, or realised cost > 0.5·c_R,ref, or h_FOV = 0. Cool-down after a revert is 10·2^k accepted steps.
5. Add an internal ROCK4 tolerance factor. It is untested here; it is needed because the equal-rtol error is up to 6x larger.

Gates for the node:
- matched accuracy under both rules, against the SPD03 banded or sparse direct arm and the proj+INO arm;
- misroute ≤ 1.25 on every corpus family;
- at least 2 grid sizes of a 2-D/3-D family, including at least one on-contract family (semilinear at n ≥ 1536).

## Threats to validity
- **Replica, not Rust.** Flops come from an explicit model, not Ir. JVP and RHS costs are hand counts. The guard's dense work (16·9m³) is a model; it dominates on small n.
- **Post-hoc design changes.** The settle rule, shadow back-off, Krylov ρ floor, mixed guard start vector and swd arm were all added after seeing data. Each one changes small-n results by 2–5x and must be preregistered.
- **Ladder granularity.** F and C disagree by up to 3.7x at loose targets (semi-384 E₄: 6.67 vs 1.79), and several E₁₀ cells are extrapolated or NR.
- **Endpoint error only.** ROCK4 has no dense output, so the corpus's 101-point output grid (landing or interpolation cost) is not covered.
- **Guard.** It is heuristic: the compressed FOV is an inner approximation (1.5x optimistic on advdiff-mild, Q = 1.6, inside the 2.41 constant), J is frozen, and only one random seed was used.
- **Off-contract evidence.** The 2-D Brusselator cases are off-contract (not the corpus holdout). On-contract corpus evidence is n = 96 plus semi-384 only. Holdouts were not touched.
- **Direct comparator.** I used sparse MMD_AT_PLUS_A LU, which is at least as strong as banded on these problems. The Rust SPD03 arm was not run.
- **References.** vdp-96's stored reference had to be rescored. Rob-96 at E₁₀ (6.1e-5) is 44x the stored reference's endpoint error (1.4e-6), so it is admissible.
- **Shared CPUs.** Wall time is not used anywhere.

## Files
All in `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/reg/`:
- Code: `rock4x.py`, `guard.py`, `swdrv.py`, `pr.py`, `run.py`, `analyze.py`, `fidelity.py`, `nonnormal.py`, `fovtest.py`, `fovstart.py`, `gen_refs.py`, `corpus_refs.py`
- Copied modules: `corpus_v2.py`, `coupled_target.py`, `lucost.py`, `probs2d.py`
- Results: `res/analysis.txt`, `res/summary.json`, `res/fidelity.txt`, `res/nonnormal.json`, `res/costmodel.json`, `res/corpus_refs.json`
- Runs: `res/runs*.jsonl`
- Logs: `logs/`
- References: `refs/`