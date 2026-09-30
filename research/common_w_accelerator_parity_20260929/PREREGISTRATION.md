# Pre-registration: matrix-free common-W correction against the sequential baseline

Written 2026-09-29, before any run of this node. Source: external re-audit
of 2026-09-29, section 8 and `NEXT_LOOP_PROMPT_KO.md`.

## Question

On the same RODAS5P stage target and the same h, does a predictor plus one
or two exact-target matrix-free common-W corrections
(`run_audit2_matrix_free_common_w_correction`) reproduce the protected
sequential stages, and at what cost relative to the sequential baseline?

## Fixed before running

- **Target.** R_i(K) = W K_i - g_i(K_<i), W = M - h gamma J_n, from
  `build_step_context_matrix_free`. The target is not replaced.
- **Arms.** (1) sequential matrix-free baseline (`sequential_stages`,
  GMRES rtol 1e-11); (2) predictor + 1 correction; (3) predictor + 2
  corrections, then a charged sequential fallback when the causal
  acceptance test fails.
- **Predictor.** The previous accepted step's stages scaled by h / h_prev
  (zero on the first step). No reference information enters it.
- **Causal acceptance (arm 3).** Relative target residual
  ||R(K)|| / max(||g||, tiny) <= 1e-9 after the last correction. The
  baseline is never consulted for acceptance.
- **Parity metrics (not acceptance).** Against the baseline stages of the
  same step: endpoint, embedded-estimate and dense (theta = 1/2) WRMS
  differences in the case norm.
- **Cost.** RHS evaluations, JVP vectors, W applications (GMRES matvecs),
  preconditioner applications, W solves, corrections, fallbacks, and the
  incremental cost of every fallback, all counted from the same
  `WorkCounters` and correction work ledgers, failures included.
- **Mesh.** The accepted mesh of one adaptive sequential run per case; each
  step is replayed from the baseline state, so every arm sees the same
  (t, y, h).
- **Cases (new, synthetic; no holdout, no N = 320/384, no Oregonator).**
  (a) a 3-D dissipative nonlinear non-commuting ODE
      y1' = -y1 + y2^2, y2' = -10 y2 + y1 y3, y3' = -100 (y3 - y1);
  (b) semilinear advection-diffusion n = 32 (in-tree manufactured family).
  rtol 1e-4 and 1e-6, atol = 1e-2 rtol.
- **Stop rule.** If arms 2-3 do not reach parity or cost more than the
  baseline, the accelerator's domain is narrowed or left off. The target is
  not changed to make it win.

## Expected, stated before running

Each correction is a block-forward sweep of s = 8 common-W solves, like the
baseline itself, and GMRES stops on a relative criterion, so a small
correction right-hand side does not make a solve cheap. The expectation is
parity at a cost above the baseline in solves, with no span reduction. A
cheaper arm would need a correction that is not block-forward (a lagged,
stage-parallel sweep), which this node does not build.

## Not in scope

Timing (no wall-clock claim), holdouts, production dispatch, any change of
the default solver.

---

## Results (appended after the run, 2026-09-29)

Command, at commit `4b2594b3536501c31b53366f766dd0e4f8274367`:
`cargo run --profile measurement -p rodas5p-integrators --features audit2-research --example common_w_parity`
(`RAYON_NUM_THREADS=1`). Raw output: `RESULTS.txt`.

| Case, rtol | steps | arm | W-apply | W-solves | RHS | max endpoint diff (units) | fallbacks |
|---|---:|---|---:|---:|---:|---:|---:|
| dissipative-3, 1e-4 | 17 | sequential | 544 | 136 | 119 | 0 | - |
| | | pred + 1 corr | 544 | 136 | 272 | 4.16 | - |
| | | pred + 2 corr + fb | 1148 | 304 | 436 | 2.1e-7 | 4 |
| semilinear-32, 1e-4 | 5 | sequential | 1800 | 40 | 35 | 0 | - |
| | | pred + 1 corr | 1800 | 40 | 80 | 2.54 | - |
| | | pred + 2 corr + fb | 3225 | 80 | 120 | 8.4e-7 | 0 |
| dissipative-3, 1e-6 | 44 | sequential | 1408 | 352 | 308 | 0 | - |
| | | pred + 1 corr | 1408 | 352 | 704 | 4.57 | - |
| | | pred + 2 corr + fb | 2640 | 704 | 1056 | 3.6e-8 | 0 |
| semilinear-32, 1e-6 | 7 | sequential | 2400 | 56 | 49 | 0 | - |
| | | pred + 1 corr | 2400 | 56 | 112 | 1.40 | - |
| | | pred + 2 corr + fb | 4000 | 112 | 168 | 2.1e-8 | 0 |

Reading:

- One correction from the scaled previous-step predictor does not reach
  parity: endpoint differences of 1.4 to 4.6 tolerance units, relative
  target residual 2e-5 to 1.5e-3.
- Two corrections reach parity (endpoint, embedded and dense differences
  below 1e-6 units; causal acceptance residual below 1e-8), with 4 charged
  fallbacks in 17 steps on one row.
- Cost at parity: 1.8 to 2.1 times the baseline W applications and exactly
  twice the W solves, 2.4 to 3.4 times the RHS evaluations, and more JVPs.
  Each correction is a block-forward sweep of 8 common-W solves; GMRES
  stops on a relative criterion, so a small correction right-hand side
  costs as many matvecs as a baseline stage solve (arm 2 used exactly the
  baseline's W applications).
- There is no span reduction: the correction sweep is as sequential as the
  baseline.

Verdict per the stop rule: **FAIL** for this accelerator form. It stays off;
the sequential baseline remains the default. A cheaper arm would need a
correction that is not block-forward (a lagged, stage-parallel sweep with
a bounded number of rounds) or a predictor accurate enough that one
correction reaches parity; neither exists in the tree. No timing was run,
because parity at a higher operation count leaves nothing to time.
