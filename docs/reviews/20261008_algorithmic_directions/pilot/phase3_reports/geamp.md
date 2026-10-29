# PROBE B6: GE-AMP-DETECT on the corpus-v2 rows

This is an exploratory pilot for a future preregistered node. It has no ledger authority. I only read `/home/user/wt-speed` at a49f7e4 and built no Rust. Git status there shows only the `tools/__pycache__/` that was present before this probe.

**Result in brief:** the candidate as specified, with the cheap res5 propagator, fails its frozen kill criteria K1 and K3 on a stress family I declared before running it, so I close it. The exact-step-map variant passed every kill check on all 165 rows and is the only part worth a new preregistered node. Neither variant detects most tolerance exceedances: 20 of 27 are caused by accumulated local error, not amplification.

## Question
Does the detector flag exactly the runs where the flow amplifies local errors, and nothing else? The detector is A(t) = ‖ge‖_W / Σ err_k, with ge_{n+1} = M_n ge_n + U8_n, threshold A_max > 4, plus a validity guard. It was tested on all 18 corpus-v2 n = 96 rows, the tight-tolerance extensions, and Robertson, HIRES, vdP and Brusselator. I also asked what it costs and what it misses.

## Method
**Frozen protocol** (`protocol.json`, written before any detector run):
- **Propagators M_n:**
  - res5 = (I − hγJ)^-5, using five back-solves on the accepted attempt's own LU;
  - exact = the RODAS5P step map for y' = J_n y, using 8 back-solves and 8 J·v on the same LU.
- **A_n** = ‖ge_{n+1}‖ in the step weights sc_n, divided by Σ_{k≤n} err_k. This is the same definition as `hyp/ctrl/amp.py`.
- **Flags:**
  - AMP: A_max > 4.
  - INVALID: max over steps of rtol·‖ge‖ > 0.1.
  - FLAG = AMP or INVALID, evaluated separately for each propagator.

**Drivers:**
- Corpus rows use the `replica_check.py` dense arm: h0 = span/100, I controller 0.9·err^(-1/5) clamped to [0.2, 5] after acceptance and [0.2, 0.9] after rejection, analytic f_t, exact stage solves, H interpolant.
- Benchmark rows use the `r5_replica.py` rodas5p-fast driver: h0 = 1e-6, with the clip below the rejected step.
- Both run closed-loop. The transports are passive, so the solution is bitwise unchanged.

**Truth:**
- The reference is the exact solution for rotating, semilinear and the osc families. Otherwise it is Radau chained node to node, at rtol max(2.5e-13, 1e-5·rtol) and atol 1e-5·atol.
- Local errors are measured from the numerical state: le_k = y_{k+1} − Φ(t_k → t_{k+1}; y_k).
- **R_tot** = max node error / Σ of all true local errors. This is the judge's ratio. **R_run** is the true analogue of A_max.
- "Amplifying" means R_tot > 4.
- "Exceeds" means max-grid case error > 1 for corpus rows (repository rule) and max node error > 1 for benchmark rows. I also report grid-based exceedance for all rows.

**Arms:**
- the two direct propagators;
- matrix-free versions of both, using unrestarted GMRES with x0 = 0 at relative residual 1e-2 (the candidate's choice) and at 1e-4;
- **control arms:** the same transports driven by the true local errors, which separates linearization error from effectivity error;
- **rival:** a two-tolerance rerun at rtol/10, flagging when the grid estimate exceeds 1. This is the cheapest direct global-error check.
- **h0 seeds:** h0 × {0.3, 0.5, 2, 3} on 27 rows, giving 108 runs.
- **Declared stress family** (`extra_probs.py`, predictions written before running): osc-amp-AMP, 48 rotating 2×2 blocks with transient growth exp(0.3·AMP), at AMP = 14 and 6. Also semilinear at n = 384 and n = 1536, and rotating at n = 384.
- **Post hoc:** osc10. It was added after seeing the osc14 and osc6 results and is labelled post hoc.

**Cost model** (flops, not Ir):
- Dense LU: LU = 2n³/3, back-solve = 2n², J·v = 2n².
- Banded LU: LU = 2n·kl·ku + n·kl, back-solve = 2n·kl + 2n(kl+ku) + n, J·v = 2·nnz.
- Dots and axpys cost 2n each.
- **JVP or RHS cost per component** (the A2 hand counts, with one trig counted as 20 flops):

  | family | flops per component |
  |---|---|
  | robertson-ramped | 8.33 |
  | hires-ramped | 6.0 |
  | vdp-ramped | 5.5 |
  | rotating | 34 |
  | forcing | 28 |
  | semilinear | 22 |
  | Robertson / HIRES / vdP / Bruss (RHS) | 13, 50, 8 per problem; 19 per cell for Bruss |

- The matrix-free baseline is the recorded Rust campaign dense-arm counters. Its JVPs, orthogonalization dots and updates come to about 270–286 JVPs per attempt.

**Replica fidelity:**
- Benchmark driver against the harness Rust counters: 11 of 12 cells match exactly in attempts, accepted, rejected and final error. The exception is vdP at rtol 1e-3: 180 attempts vs 181.
- Corpus against `rust_recorded_rows`: attempts equal in 15 of 18 rows, rejections 16 of 18, max-grid error within 1% in 13 of 18 and within 10% in 16 of 18. The outlier is semilinear at 1e-4: 21.4 here vs 17.0 in Rust, because Rust solves stages inexactly.
- At n = 384, semilinear gives 24.8 / 71.0 / 261 against Rust's 22.8 / 71.0 / 260.8.
- Reference check, chained or exact final state against the stored or NATIVE final state: at most 1.1e-3 case units.
- The F-033 exact-state Σ local error is 1.49 / 2.76 / 6.94, against the repository's 1.37 / 2.67 / 6.83.

## Results: the 39 frozen rows
\* marks rows where threshold 4 was chosen (in-sample). Grid and node errors are in case or tolerance units. A_max is given as res5 / exact.

| row | att | grid | node | Σ le | R_tot | A_max | FLAG | exceeds | rival est |
|---|---|---|---|---|---|---|---|---|---|
| rob-ramped 1e-4/-6/-8/-10 | 16/25/74/185 | .466/3.70/2.28/1.51 | .311/1.63/2.25/1.51 | .80/3.19/5.41/7.27 | .39/.51/.42/.21 | 1/1 all | 0 | 0/1/1/1 | .45/3.68/2.14/1.18 |
| hires-ramped | 17/34/79/196 | .392/.916/.409/.385 | .27/.72/.34/.29 | 1.0/2.9/2.2/2.4 | .26/.25/.16/.12 | 1/1 | 0 | 0 | .34/.87/.41/.38 |
| vdp-ramped | 39/94/259/692 | .811/2.55/.510/.488 | .40/1.62/.21/.37 | 3.1/6.5/8.2/14.8 | .13/.25/.026/.025 | 1/1 | 0 | 0/1/0/0 | .80/2.54/.53/.36 |
| rotating | 56/142/422/1139 | 1.48/3.62/1.62/1.10 | .89/5.34/2.29/1.77 | 14.9/23.8/18.7/18.9 | .06/.22/.12/.09 | 1/1 | 0 | 1 | 1.45/3.32/1.50/1.00 |
| forcing | 91/253/617/1584 | 4.22/2.24/2.28/1.65 | 4.09/1.28/2.18/1.75 | 18–21 | .23/.08/.11/.08 | 1/1 (1.16/1.24 at 1e-10) | 0 | 1 | 3.89/1.97/2.10/1.50 |
| semilinear* 1e-4/-6/-8, 1e-10 | 20/35/66/154 | 21.4/53.2/207/73.4 | 21.2/53.1/207/73.4 | 1.50/2.76/6.94/2.83 | 14.2/19.3/29.9/26.0 | 7.15/8.97, 6.98/7.89, 16.6/16.6, 22.7/23.3 | 1 | 1 | 17.3/41.4/198/66.9 |
| Robertson* 1e-4/-6/-8, 1e-10 | 26/45/103/444 | .113/.649/.637/1.06 | .154/.89/4.74/8.61 | .65/3.1/12/75 | .24/.29/.39/.12 | 1/1 | 0 | 0/0/1/1 | .10/.60/.64/1.04 |
| HIRES* 1e-4/-6/-8, 1e-10 | 82/210/784/2853 | 3.77/7.82/.612/.205 | 1.20/8.58/.24/.25 | 7/20/45/143 | .17/.44/.005/.002 | 1/1 | 0 | 1/1/0/0 | 3.76/7.88/.60/.21 |
| vdP-μ1000* 1e-4/-6/-8 | 242/469/947 | 1.42/2.05/2.77 | 8.2e6/2.3e6/3.8e6 | 1650/23/27 | 5.0e3/9.7e4/1.4e5 | 8e9–1.4e11 / 5e7–2e9; guard 7.5e5–4.1e7 | 1 (AMP and INVALID) | 1 | 1.19/1.78/2.56 |
| Bruss-50* 1e-4/-6/-8 | 52/92/193 | 1.78/8.73/8.80 | 1.78/8.89/9.01 | 1.8/5.0/6.3 | .99/1.78/1.43 | 1.87/1.67, 2.00/1.76, 1.07/1.00 | 0 | 1 | 1.36/7.75/8.03 |
| Bruss-200 1e-6 | 92 | 8.65 | 8.80 | 4.97 | 1.77 | 2.00/1.75 | 0 | 1 | 7.66 |

## Confusion matrices (TP / FP / FN / TN)
| set | flag | vs amplifying | vs exceeds |
|---|---|---|---|
| main 39 | res5 | 7/0/0/32 | 7/0/20/12 (grid basis: 7/0/19/13) |
| main 39 | exact | 7/0/0/32 | 7/0/20/12 |
| main 39 | rival | 7/18/0/14 | 25/0/2/12 (grid basis: 25/0/1/13; the miss is rotating 1e-10, true 1.10, estimate 1.00) |
| out-of-sample 24 | res5 = exact | 1/0/0/23 | 1/0/14/9 |
| corpus 18 | res5 = exact | 3/0/0/15 | 3/0/9/6 |
| h0 seeds 108 | res5 = exact | 12/0/0/96 | 12/0/57/39 |
| h0 seeds 108 | rival | — | 65/0/5/38 on the grid basis (misses have true error 1.09–1.18) |
| declared stress 12 + post-hoc 3 | res5 | 5/0/4/6 | 5/0/10/0 |
| declared stress 12 + post-hoc 3 | exact | 9/1/0/5 (the FP is osc10 1e-4: R_tot 3.24, R_run 4.36, A 7.33) | 10/0/5/0 |

**Stress rows, R_tot → A_max res5 / exact:**

| row | R_tot | A_max res5 | A_max exact |
|---|---|---|---|
| osc14 1e-4 | 7.91 | 1.31 | 20.8 |
| osc14 1e-6 | 14.1 | 2.72 | 18.4 |
| osc14 1e-8 | 20.1 | 5.35 | 15.2 |
| osc10 1e-4 (post hoc) | 3.24 | 1.0 | 7.33 |
| osc10 1e-6 (post hoc) | 4.57 | 1.24 | 6.73 |
| osc10 1e-8 (post hoc) | 6.64 | 2.09 | 5.78 |
| osc6, all three rtols | 1.48–2.25 | 1.0–1.05 | 2.45–2.78 |
| semilinear n = 384 | 12.9 / 22.7 / 36.6 | 12.1 / 9.0 / 18.2 | 22.5 / 11.1 / 19.7 |
| semilinear n = 1536, 1e-6 | 28.0 | 14.3 | 17.5 |
| rotating n = 384 | 0.07 / 0.20 | 1 | 1 |

**Kill criteria:**
- **res5:**
  - K1 fires on osc14 1e-4 and osc14 1e-6 (declared rows) and on osc10 1e-8 (post hoc).
  - K3 (res5 and exact disagree on the flag) fires on osc14 1e-4 and 1e-6, and on all three osc10 rows.
- **exact:** K1 and K2 never fire across 165 rows (39 + 108 + 15).
- **Separation for exact:**
  - negatives reach at most 2.78 (osc6), 1.76 on the frozen set, 1.83 on the seeds;
  - positives are at least 5.78 (osc10 1e-8), 7.89 on corpus rows, 9.29 on the seeds.
- **Separation for res5 on the frozen set:** negatives at most 2.0, positives at least 6.98.

The mechanism is the one the math reviewer predicted: |(1 − γ·i·hω)^-5| < 1, so res5 damps rotating modes, and the U8 estimate sits mostly in the high-ω blocks.

## Attribution (control arms)
- **The linearization is valid on every row except benchmark vdP.** Transporting the true local errors with the exact map reproduces the true global error: T_true/G = 0.93–1.14. On benchmark vdP the ratio is 2.1e3–8.6e3, which is the relaxation-jump regime; the guard fires correctly there.
- **The under-read of A is entirely an effectivity effect.**
  - On semilinear, A driven by the true local errors is 31.9 / 29.1 / 33.8 / 36.5, against R_run of 33.4 / 29.9 / 34.1 / 36.7.
  - A driven by the U8 estimate is 8.97 / 7.89 / 16.6 / 23.3, so it under-reads by 1.6–3.8x.
  - The effectivity (estimate / true local error) spreads 10–48x between its 5th and 95th percentiles along the run.
  - It can also over-read: on osc10 1e-4, A = 7.33 against R_tot 3.24.
- **The flag shows only in the running maximum.** The endpoint A on amplifying rows is 0.10–1.43. The flag is raised at or before the time of the global error peak (semilinear rtol 1e-6: t = 0.659 for both; rtol 1e-4: 0.568 vs 0.687).
- **The exceedances the detector misses are accumulation, not amplification.** All 20 exceed-but-unflagged rows have R_tot ≤ 0.99, and their sums of true local errors are 3–143 tolerance units over hundreds of accepted steps.
- **Exploratory, not frozen:** a magnitude flag max ‖ge‖ > 1 scores 26/8/1/4 against exceedance, so it cannot be used.

## Matrix-free sensitivity (`mfdiag.py`)
- **res5 at linear tolerance 1e-2 is unsafe.**
  - Each application has a median relative error of 1.4–3.3%, and these errors compound over the steps.
  - On semilinear 1e-10, A_max becomes 1341 against 22.7 for the direct transport.
  - The largest value on a negative row rises to 3.03 (Bruss-200).
  - At 1e-4 it tracks the direct transport (22.0 against 22.7).
- **The exact map at 1e-2 tracks the direct one within about 2% on every row**, for example 23.3 against 23.3 and 21.1 against 20.8. In increment form, solve errors scale with ‖hJv‖ rather than ‖v‖.

## Cost (fraction added to the baseline flops of the same run)
| driver | res5 | exact | rival rerun at rtol/10 |
|---|---|---|---|
| dense, corpus n = 96 | +9.0–12.1% | +29.6–39.9% | +113–188% |
| dense, benchmark n = 2–8 | +11–26% | +63–106% | +127–251% |
| dense, Bruss-50/200 | +3.1–11.6% | +10–38% | +133–154% |
| banded, corpus | +4.4–29.3% | +34–87% | +113–187% |
| banded, benchmark | +14–26% | +61–96% | same as dense |
| matrix-free vs Rust campaign counters, corpus 18 rows | flops +0.7–2.4%, JVPs +3.5–7.0% (at 1e-4) | flops +1.0–3.0%, JVPs +5.7–10.4% (at 1e-2) | attempt ratio 1.13–1.88 (inferred) |

Counted directly:
- res5 adds 5 back-solves per accepted step, which is 0.40–0.63x the baseline back-solves.
- The exact map adds 8 back-solves and 8 J·v per accepted step.
- Neither variant adds an LU, an RHS evaluation or a J build.
- Matrix-free exact at 1e-2 adds about 2.2–3.0 GMRES iterations per solve.

## Decision
- **CLOSE GE-AMP-DETECT as specified (res5 propagator).** Frozen K1 and K3 fire on the declared oscillatory stress family, and the 1e-2 matrix-free form also drifts badly.
- **PROMOTE only the exact-step-map variant**, as an opt-in self-verification report field, through a new preregistered node with the same threshold of 4 and the same guard. It is not a global-error certificate and not a speed lever.

## Recommended rule (exact-map variant)
1. On each accepted step, using the step's J_n and W = I/(hγ) − J_n (LU on direct drivers):
   - compute V_i = W⁻¹[J_n(ge + Σ_{j<i} A_ij V_j) + Σ_{j<i} (C_ij/h) V_j] for i = 1…8;
   - update ge ← ge + Σ_j b_j V_j + U8.
2. Matrix-free: solve for each V_i with GMRES on (I − hγJ_n) with right-hand side hγ·(…), at relative residual 1e-2, x0 = 0. Never use res5 with loose solves.
3. Report, using running maxima rather than endpoint values:
   - A_max = max_n ‖ge‖_{sc_n} / Σ_{k≤n} err_k, with its time t(A_max);
   - g = max_n rtol·‖ge‖_{sc_n}.
4. Status:
   - `linearization-invalid` if g > 0.1;
   - otherwise `propagation-amplified` if A_max > 4;
   - otherwise `not-amplified`. This means local errors are not amplified. It does not mean the global error is within tolerance.
5. Use the two-tolerance rerun (+113–251%) as the companion check whenever a global-error claim is needed. It caught 25 of 26 grid exceedances with no false positives.

**Suggested KILL criteria for the new node:** a row with R_tot > 5 and A < 3, or a row with R_tot < 2 and A > 4 without INVALID, on held-out Rust rows. Include the osc families as declared positives.

## Threats to validity
- **Replica, not Rust.** Stages are solved exactly. 3 of 18 rows differ in attempts, and semilinear at 1e-4 has error 21.4 here against 17.0 in Rust. The transports were never implemented in Rust, and overhead is in flops, not Ir.
- **Narrow positive class.** In the frozen set the positives are one family (semilinear) plus vdP through the guard. The osc families are mine: synthetic, linear and with normal J. osc10 is post hoc.
- **Gray zone.** Rows with R_tot between 3 and 5 were sampled only by osc10, which produced one exact-map false positive (R_tot 3.24, A 7.33).
- **Truth definitions.** Node-based and grid-based truth differ on the benchmark rows: on HIRES the grid error is larger (interpolant), on Robertson the node error is larger (early transient). The vdP amplification is phase error across relaxation jumps; its node errors reach 1e6 units while its grid error is 1.4–2.8.
- **References.** The Radau chain was validated only at t_f (at most 1.1e-3 case units) and through T_true/G ≈ 1. Reference tolerance was not varied separately.
- **Matrix-free overhead.** The ratios use the over-solving Rust campaign arm as baseline (about 280 JVPs per attempt). Against a tuned baseline such as proj-stop + INO, the relative overhead would be about 2–4x larger.
- **Rival.** It assumes tolerance proportionality and misses marginal exceedances (true error ≤ 1.18).
- **Environment.** The CPUs were shared, so no timing claims are made.

## Files
All under `/tmp/claude-0/-home-user-vigilode/28b4ddd9-6979-5dcc-9dc3-5b21a0db99cc/scratchpad/algo/probe/geamp/`:
- `protocol.json` (frozen before the runs), `geamp.py`, `analyze.py`, `analyze_seeds.py`, `extra_probs.py`, `mfdiag.py`
- `rows/` (39 rows), `rows_seed/` (108), `rows_extra/` (15)
- `summary.json`, `summary_rows_seed.json`, `summary_rows_extra.json`, `fidelity_bench.json`, `mfdiag.json`
- `logs/`: `analyze.log`, `rows.log`, `extra.log`, `extra_posthoc.log`, `fidelity.log`, `mfdiag.log`, `seeds_summary.log`, `extra_summary.log`