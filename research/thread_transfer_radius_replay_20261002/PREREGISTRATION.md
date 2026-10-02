# Preregistration: native replay of the R4 homotopy radius cap (thread-transfer DAG node P0-RADIUS-REPLAY)

## Question

L-0017/L-0024 (R4 homotopy certificate cost study, `research/r4_homotopy_certificate_cost_20261001/STUDY.json`)
failed because neither the full nor the component-blocked doubling certificate closed its common state radius
within six attempts (`D = 0.001 * 4^k`, k = 0..5) at dimensions 8 and 16. The thread-transfer review
(`docs/reviews/thread_transfer_20261002/REVIEW_KO.md`, section 4.2) replayed the same fixtures in exact rational
arithmetic in Python and found closure at the 7th attempt (n = 8, D = 4.096) and the 8th (n = 16, D = 16.384). It
supplied a Rust template (`probes/native_replay_homotopy.rs`) that was never compiled.

Does the native Rust certificate, with its own directed binary64 arithmetic, reproduce that diagnosis?

This node only diagnoses the cap. It does not change the six-attempt gate of L-0017/L-0024, whose FAIL stands, and
it makes no efficiency or admission claim. The operational q2 admission path (`Q2Admission::NativeTargetCertificate`,
serial `certify_stage_target`) is a separate consumer and is not touched.

## Fixture (unchanged from `r4_studies::homotopy_cost_study`)

Dimensions n = 1, 2, 4, 8, 16; `StageTarget::sequential` of the unchanged coefficient snapshot; diagonal
`A_ii = -1 - i`; `q_i = -0.05 (1 + i mod 3)`; `y_i = 1 + 0.1 i`; `h = 0.05`; every candidate stage `h f(y)`;
`y_hat = y + h f(y)`; `e_hat = 0`; `InverseWitness::diagonal`; atol 1e-8, rtol 1e-6; initial radius 1e-3, factor 4;
`ParallelExecution::sequential()`.

## Test and command

`crates/rodas5p-integrators/tests/thread_transfer_radius.rs`, ported from the template (the unused
`Q2CertificateSource` import is dropped; n = 1, 2, 4 are added; the per-attempt ledger is written as JSON):

`THREAD_TRANSFER_RADIUS_OUTPUT=research/thread_transfer_radius_replay_20261002/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test thread_transfer_radius -- --nocapture`

For each n it runs the blocked and the full doubling certificate with 6 and with 8 attempts and records every
attempt (radius, closes, max state radius) plus the Python reference `required` values of
`docs/reviews/thread_transfer_20261002/results/homotopy_radius_probe.json`.

## Gate

**PASS** if all hold:

1. **Old failure preserved.** With 6 attempts, n = 8 and n = 16 return no certificate from both the blocked and the
   full path, with 6 recorded attempts each; n = 1, 2, 4 close (as in L-0017).
2. **Extended closure.** With 8 attempts, the first closing attempt (1-based) is 4, 5, 6, 7, 8 for n = 1, 2, 4, 8, 16,
   in both paths.
3. **Full = blocked.** At n = 8 and 16 (8 attempts), the full and blocked stage bounds are bit-identical, and every
   attempt's max state radius is bit-identical between the two paths.
4. **Native agrees with the exact reference.** At every attempt of every n, the native max state radius `R` and the
   Python reference value `P` (exact input rationals, upward to 2^-100) satisfy `-1e-12 <= R / P - 1 <= 1e-9`.
   Upward-rounded binary64 may sit slightly above the exact value, never materially below.

Otherwise **FAIL**, and the disagreement is classified as rounding, policy or model difference without promoting
anything (DAG stop condition).

## Prior information

- L-0017/L-0024 (FAIL): attempts 4/5/6 at n = 1/2/4; none closes at n = 8/16 within 6.
- The review's Python probe values above. No Rust run of this test exists before this commit; the template has never
  been compiled.
