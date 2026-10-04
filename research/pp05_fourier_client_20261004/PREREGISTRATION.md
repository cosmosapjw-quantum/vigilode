# PP05 — native source-bound Fourier-Volterra client with the original-target certificate (prospective registration)

RVJ DAG node `PP05`. Base commit: the commit that adds this file. Source
of truth: `inputs/loop10/code/fourier_volterra.py` inside
`research/rvj_integration_20261004/inputs/rvj_independent_loop11_20261004.zip`
(SHA256 `a7d66500...6fe6`), functions `rhs`, `certificate`, `commit`,
`build`, `physical`, `phase_candidate`, `EP` and loop 9 `rotate_interval`.

## Client (ported, not changed)

Two coupled complex amplitudes on one step `[t, t+h]`, normalized time
`tau` in [0, 1]: `a' = i q c conj(a) b`, `b' = i (q/2) c a^2`,
`c = g + lambda eps Re(phase a)`, `q = sigma 5/4` (constant on the
invariant leaf `|a|^2 + 2|b|^2 = 9/16`, the archived scope), physical target
`lambda = 1`, model (omega, g = 1/8, eps = 1/8, sigma, epoch), state
(t, a, b, error, generation), initial state (1/4, 1/2). Paths are sparse
sums `c[k,j] tau^j e^{i k omega h tau}`.

## Native implementation (opt-in, `rodas5p-integrators`, new module)

- Directed complex-interval arithmetic for the certificate; candidate
  coefficients are binary64 points.
- `FourierPath` with exact noncyclic product, conjugation `(k, j) -> (-k, j)`,
  derivative, Bernstein-hull sup bound (triangle over harmonics),
  endpoint value enclosure with the rotation enclosure ported from
  `rotate_interval` (halving to `|r| <= 1/8`, Taylor with remainder,
  interval doubling) in directed binary64.
- `certificate(state, h, model, path, phase, phase_error)`: the same
  quantities as the source (`h growth < 1`, initial condition equality,
  `B <= 1`, the differential defect against the **lambda = 1** right side
  recomputed by the noncyclic product, phase defect `h |q eps| B^3
  phase_error`, growth `1 / (1 - h growth)`, endpoint rounding, error,
  closure defect reported), every operation rounded outward.
- `commit`: refuses unaccepted, over-budget, stale (state, model, epoch,
  h or path differ from the bound trial) candidates; returns the next state
  with generation + 1.
- A binary64 Picard predictor (direct noncyclic convolution, the source's
  `build` with `K, p, sweeps`) produces candidates; it has no acceptance
  field. The source's step driver (`(K,p,sweeps)` in `[(2,4,2), (3,6,4)]`,
  local and global budgets, step halving) is ported for the runs.
- `Q2CertificateSource` and every other existing certificate are not used.

## Runs (fixed now)

The source's cases with T = 1/2 and step cap 1/8: `resonance` (omega 0),
`unit` (1), `forty` (40), `negative` (-40, sigma -1), `fast` (1e4); tol
1e-8; and `forty` with tol 1e-11 (binary64 replaces the source's 112-bit
coefficients, so the source's 1e-12 case is not attempted).

## Gate

G1 Enclosure: for every committed state the reported error bounds the
   actual error against a reference of the true ODE (`q = sigma sqrt(1 +
   |a|^2 + 2|b|^2)`): mpmath `odefun` at 30 digits for omega in {0, 1, 40,
   -40}; for omega = 1e4 two scipy DOP853 runs at rtol 2e-13 and 8e-14 as
   in the source, with their difference reported (not rigorous; disclosed).
   Also three interior points per step against the growth bound.
G2 Source parity: for every committed native step, the source's exact
   `certificate` (Python, Fractions) evaluated on the native binary64 path
   accepts it, and the native error is >= the source's error (the native
   bound is outward of the exact-arithmetic one) and at most 1.01 times it.
G3 Negative controls fail closed: an independent-scalar replacement of the
   coupled right side (coupling terms dropped in the predictor); a phase
   witness omitted (`phase_error = 0` with a perturbed phase); a cyclic-alias
   candidate (wrapped harmonics); a stale binding (state, epoch, h, path);
   initial-condition mismatch; `h growth >= 1`; `B > 1`.
G4 Contract tests pass; fmt and clippy clean.

No default solver change, no timing, no claim beyond this invariant-leaf
client.

## Amendment before any run (2026-10-04, while writing the code)

1. Initial condition. The source requires the path's start `p(0)` to equal
   the state exactly, which its rational `enforce_start` guarantees. In
   binary64 the start coefficient `z - sum_k c[k,0]` is generally not
   representable, so exact equality would reject almost every candidate.
   The native certificate instead charges the mismatch: the start error is
   `state.error + max_j |a_j - p_j(0)|_1` (outward), which is the source's
   bound with the true start at distance at most that from `p(0)`; it is
   zero when they are equal. The G3 control "initial-condition mismatch"
   therefore fails closed by its charged bound exceeding the budget, not by
   an error.
2. Phase witness (finding A-PORT-03). The source's `certificate` takes the
   phase polynomial and `phase_error` from the caller. The native
   certificate builds the phase witness itself from (omega, t, h) with the
   rotation enclosure and does not accept a caller phase; the predictor
   computes its own binary64 phase, which is untrusted. G2 parity compares
   with the source certificate given the source's own phase for the same
   (omega, t, h) and the state with `a = p(0)` (so the source's equality
   check holds).

## Results (append only after the recorded run)
