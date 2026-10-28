# RHS-only homotopy experiment, 2026-10-27 KST

Baseline source: PR90 `68a28357d960c4c2743c299770f0a33ef63245b2`.
New algorithm: direct map `G_lambda=(1-lambda)anchor+lambda G`; plain Picard,
screened depth-one Anderson, and SlowOnly screened Anderson are implemented
in one module and evaluated in one binary. The prior projected coupled modes
used frozen JVPs and small matrices; this separate callback interface does not.

Before the first native execution, freeze these cases:

- Implicit midpoint target for `y'=-k y-g y^3`, initial vector components
  `1+0.01(i mod 7)`, `(n,k,g,h)` equal to `(1,1,0,.2)`, `(1,1,0,1.98)`,
  `(64,1,0,1.98)`, `(1,1,.5,.5)`, `(64,10,.5,.18)`, `(1,1,0,2.2)`.
- The nonlinear scalar case also uses lambda path `[.25,.5,1]` to expose
  continuation overhead. Every other case uses `[1]`.
- All three modes: residual L2 tolerance `1e-12`, 4096 updates/station,
  20000 map-call cap; extrapolation factor at most 1024, improvement 0.9,
  rejected-trial cap 2/station; SlowOnly threshold 0.5.
- Compare to monotone scalar bisection on `K+h(k(y+K/2)+g(y+K/2)^3)=0`,
  maximum component error cap `1e-10`. Preserve failed numerical cases.
- Actual classical RK4 causal stage target at `y=1,h=.25`, same nonlinear RHS,
  compared to its four-call sequential root. Expose extra all-stage map work.
- Actual implicit-midpoint trajectories to `T=1` for `k=1,g=.5,y0=1`,
  8/16/32/64 steps and all modes. Independently score every time-grid state
  against `exp(-t)/sqrt(1+.5(1-exp(-2t)))`; show order separately from root error.
  For each arm compare zero seed with reuse of the previous accepted stage
  (the step size is unchanged). Preserve all returned grid states in the output.

All map calls, including rejected extrapolation and continuation, are charged.
Reference bisection is validation work and is not included in candidate cost.
This experiment makes deterministic callback-work claims; it does not measure
wall-clock speed. No broad method invalidity is inferred from a failed cell.
