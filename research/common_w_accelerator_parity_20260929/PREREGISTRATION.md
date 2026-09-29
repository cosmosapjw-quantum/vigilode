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
