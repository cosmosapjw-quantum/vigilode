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

### Executed result — 2026-10-04 (source `675f55d`)

Commands: `PP15_CASES=research/pp15_fourier_comparator_20261004/cases.json
cargo test --release -p rodas5p-integrators --locked --test
pp15_fourier_comparator export -- --ignored --nocapture`, then `python3
tools/pp15_comparator_check.py --cases .../cases.json --output
.../RESULTS.json`. Contract `jacobian_matches_finite_differences` passes.

- G1 PASS: both client arms reach T = 1/2 in 4 steps for omega 1, 40 and
  1e4 and their certified bounds enclose the actual final errors (bounds
  2.2e-12 to 2.8e-11, actual 1.9e-13 to 2.9e-12; the FFT arm agrees with
  the direct arm to 1e-15).
- G2: RODAS5P is uncertified (an embedded estimate). Its actual final
  errors are 4.2e-10 / 3.9e-12 (omega 1, tol 1e-8 / 1e-10), 3.5e-10 /
  4.6e-12 (omega 40) and 9.3e-7 / 4.2e-10 (omega 1e4). The tolerance whose
  actual error is closest to the client's certified bound is 1e-10 for
  every omega. At omega 1e4 neither RODAS5P run reaches the client's
  certified level (best actual 4.2e-10 against a client bound of 2.2e-12).
- Counted work, each in its own unit and not comparable as cost: the
  client used 8 candidate builds, 24 predictor convolutions and 8
  certificates per run at every omega; RODAS5P at tol 1e-10 used 11, 72
  and 6,315 attempts with 88, 568 and 49,177 right-side evaluations and
  11, 72 and 6,315 LU factorizations of 4 x 4 matrices for omega 1, 40,
  1e4. The client's work does not grow with omega here because the
  Fourier path carries the carrier analytically; RODAS5P resolves it with
  steps. A convolution of sparse coefficient sets and a 4 x 4 right-side
  evaluation are different operations, so no work ratio or speed claim is
  made (timing authority HOLD).
- Scope: the client is the invariant-leaf model with constant q (PP05);
  RODAS5P solved the same leaf model. This is not evidence about general
  closures (PP14) or other problems.
- G3: see REVIEW.md.

### Independent review and corrections (after the draft above)

**Verdict: FAIL (G3).** G1 and G2 hold. The independent reviewer
(REVIEW.md, verbatim) found the draft above misleading on matched error and
carrying an unstated cost argument. The draft is kept as reviewed; these
corrections replace its G2 and work paragraphs:
- No matched physical error was reached. The certified client bound is
  2.8e-11, 2.9e-12 and 2.2e-12 for omega 1, 40, 1e4. RODAS5P at tol 1e-10
  reaches 3.9e-12 (omega 1, below the bound), 4.6e-12 (omega 40, 1.6x the
  bound) and 4.2e-10 (omega 1e4, 193x the bound and 2,280x the client's
  actual error). "Closest tolerance" only names the nearer of the two
  tolerances tried; at omega 40 and 1e4 neither reaches the client's level.
- The comparison is asymmetric: the client's number is a certified bound,
  RODAS5P's is its actual error against the reference (it has no bound).
- The work counts are at unmatched accuracy and in different units; they
  support no cost or scaling statement. The draft's sentence that the
  client's work "does not grow with omega because ... RODAS5P resolves it
  with steps" is an untested causal claim and is withdrawn.
- "8 certificates" was not a separate count: the exporter copies
  `certificate_calls` from `candidate_builds` (each build calls the
  certificate once). Candidate rejections were 4 of 8 at every omega;
  RODAS5P Jacobian builds were 11/64/4,972 accepted-step Jacobians at tol
  1e-10 (omega 1, 40, 1e4) and 10/31/1,696 at 1e-8.
- The omega = 1e4 reference is the non-rigorous scipy pair (difference
  3.4e-15 between its two tolerances).
