# PP15 — bounded comparator on the Fourier client: certified client vs RODAS5P at matched physical error (prospective registration)

RVJ DAG node `PP15` (depends on PP07, done as abstain). Base commit: the
commit that adds this file. Timing authority stays HOLD: no timing is run
and no speed is claimed; this node compares correctness and counted work
in their own units.

## Arms (domain-admissible ones only)

- C1 native Fourier client, direct predictor (PP05 driver, tol 1e-8).
- C2 native Fourier client, FFT predictor (PP06).
- R RODAS5P (existing adaptive solver with an explicit Jacobian of the
  real 4-dimensional system in the same rotating variables, `q` constant
  5/4 as on the leaf), rtol = atol in {1e-8, 1e-10}, to T = 1/2.
Omitted, with reasons: the shared jet (PP07: no shifted solves), Laguerre
and Leja (not this problem's operator), q1/q2 homotopy paths (stage
targets of RODAS, not this client).

## Measured (fixed now)

omega in {1, 40, 1e4}: final error at T against the PP05 reference
(30-digit mpmath for omega <= 40, scipy pair for 1e4), whether the arm
provides a certified bound and whether it encloses, and counted work:
client predictor calls, candidate builds, rejections, certificate calls;
RODAS5P right-side calls, Jacobian builds, LU factorizations, accepted and
rejected steps.

## Gate

G1 Every certified arm's final bound encloses its actual error.
G2 Matched error is reported honestly: for each omega, the RODAS5P
   tolerance whose actual error is closest to the client's certified
   bound is identified; no conclusion about speed is drawn.
G3 An independent reviewer (a separate agent that did not write the code)
   reads the outputs and states whether the report separates correctness
   from speed; its statement is stored.

## Results (append only after the recorded run)
