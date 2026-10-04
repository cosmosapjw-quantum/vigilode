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

3. Rotation enclosure (second amendment, still before any run). The
   source's `rotate_interval` halves the angle to `|r| <= 1/8` and doubles
   back; with 112+ working bits that is harmless, but in binary64 interval
   arithmetic each doubling widened the enclosure by about 2.8, giving
   widths near 1e-10 at `omega t = 5000` (a unit test showed it). The native
   enclosure reduces modulo `2 pi` with a two-part Cody-Waite constant
   (`2 pi = A + B + d`, `A` with 26 trailing zero bits, `0 < d < ulp(B)`,
   checked at 60 digits by the node's check tool) and sums the Taylor series
   of `e^{ir}` with the tail `2 |r|^(n+1)/(n+1)!` once `|r|/(n+2) <= 1/2`.
   The enclosed quantity is the same.

## Results (append only after the recorded run)

### Executed result — 2026-10-04 (source `5c59328`)

Commands: `PP05_CASES=research/pp05_fourier_client_20261004/cases.json
cargo test --release -p rodas5p-integrators --locked --test
pp05_fourier_client export_fourier_client -- --ignored --nocapture`, then
`python3 tools/pp05_fourier_check.py --cases .../cases.json --output
.../RESULTS.json`. The checker first crashed (mpmath cannot take a
`Fraction`) before writing anything; the fix (a `Fraction -> mpf`
conversion helper, no gate or threshold change) is in the commit that
records these results. The exporter ran once.

**Verdict: FAIL (G2 and G3 as registered); G1 PASS.**
- All six runs reach T = 1/2: 4 steps each at tol 1e-8 (8 builds, 4
  rejected first candidates, 24 predictor calls), 8 steps with 7 halvings
  at tol 1e-11.
- G1 PASS: every committed state's bound encloses the actual error (worst
  actual / bound 0.18, resonance) and every interior point the growth bound
  (worst 0.13); final bounds 5.3e-13 to 2.8e-11, physical H-errors
  <= 4.8e-11. References: mpmath odefun 30 digits; omega 1e4 with scipy
  DOP853 (difference of the two tolerances 1.0e-14, not rigorous).
  `negative` (omega -40, sigma -1) gives the same numbers as `forty`, as the
  conjugation symmetry of the model predicts.
- G2 FAIL: the archived exact certificate accepts every native path and
  the native error is never below it (ratio >= 1.000006), but the ratio
  reaches 1.0112 in `forty_tight` (1.0011 at most elsewhere), above the
  registered 1.01. The native bound is outward and looser by up to 1.1 %.
- G3 FAIL: the independent-scalar and wrong-phase candidates are rejected
  (certified bounds 7.1e-4 and 7.2e-5 enclose actual errors 1.3e-4 and
  8.1e-6); stale state, epoch, h and path bindings, the start mismatch
  (charged 1.0e-3), `h growth >= 1` and `B > 1` all fail closed. The
  cyclic-alias candidate was **committed**: on the first step at omega 40,
  K = 3, the wrapped harmonics are below the tolerance, the candidate's
  actual error is 2.8e-14 and its certified bound 6.2e-13 encloses it. This
  is a correct acceptance of an accurate candidate, not a false
  certificate, but the registered control expected rejection and the
  control did not exercise aliasing at this step.
- No bound in any run or control is below the actual error.
