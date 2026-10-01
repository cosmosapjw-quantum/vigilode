# Re-audit R4 closure (2026-10-01)

This document covers the external R4 re-audit package `research/adversarial_reaudit_20261001_r4/`. Its source is
`1c54194123ee6abc6daa512e8574922f510b4e2c`, its development plan is the 26-node `NEXT_DEVELOPMENT_DAG.json`, and its
ledger rows are L-0011 and L-0012. The stacked branch is `claude/jolly-wozniak-7wl15h-wu24-reaudit-r4`. For each node,
this document lists what the branch implements, the contract tests that check it, and the preregistered experiments
with their ledger rows. It also states what stays open.

Each node keeps the claim ceiling the DAG gives it. Nothing here is a production-readiness or speed promotion. The DAG
authority gate holds: no timing decision is admissible on this branch.

## Status legend

| Status | Meaning |
|---|---|
| **Closed** | Every acceptance item has a contract test on this branch. |
| **Closed (research)** | The node is implemented and tested, and its claim stays inside the research module or the declared domain. |
| **Measured: PASS / FAIL** | The preregistered experiment ran and its numeric gate decided. A FAIL is kept as a negative ledger row. |

## Matrix

| Node | Status | Implementation | Contract tests / evidence |
|---|---|---|---|
| ARITH-DEV-01 | Closed | `ExpBound::total_cmp`: ZERO sorts below every positive value and malformed bounds sort above all values. It is used for row and column maxima, the norm choice and the class minimum. The nilpotent class is an acyclic nonzero pattern, so permuted and transposed triangular matrices keep their bound. | `r4_transform_bound_contracts.rs`: the R4 witness `A=[[0,.25],[0,0]]`, `h=1000.1` is enclosed (it was 5.8e-11 against an exact 1.507e-9); 16 exact-rational cases, including 2x2 and 3x3 permutations and transposes; order axioms. |
| ARITH-DEV-02 | Closed | `reciprocal_factorials_upper`: the outward recurrence `r_k = r_(k-1)/k`, so `k!` is never formed. Orders go up to `MAX_TRANSFORM_ORDER = 1000`; higher orders get a typed error. | The same file checks `1/k!` for k = 0..1000 in exact integers, tight to a factor of 2. Every order 0..1000 of `A = 0`, `h = 1.1` is enclosed against `fixtures/r4_transform_oracle_fixtures.json`. |
| ARITH-DEV-03 | Closed | Order-0 stored weights are compared with their sign. A negative, NaN or infinite tolerance admits nothing, and a zero tolerance admits only a zero error. Non-finite stored weights are rejected. | The same file: `b0 = 1`, `stored = -1` gives an upper bound of at least 2, and the tolerance-domain cases. |
| POLY-DEV-01 | Closed (research) | `ExpBound::l2_norm_upper`, a norm scaled by powers of two. Inputs outside `2^+-500` are normalized by an exact power of two. The budget is rounded down. Lost input bits are propagated by `1/k!` and subnormal result rounding is added. Out-of-range cases give `POLYNOMIAL_RANGE_UNSUPPORTED`. | `r4_polynomial_contracts.rs`: eight exact norms, and seven mpmath actions with amplitudes from 5e-324 to 1e300, including a mixed lossy case. Each is enclosed and none gives a zero certificate. |
| POLY-DEV-02 | Closed (research) | `JointPhiReport::admit_total_error` admits only a Certified bound within an absolute budget in [0, inf). The relative bound and the conditioning are reported beside it. `execution` labels the unbounded timing path. | The same file: a truncation budget of 1e-30 is met but total admission is rejected (`TOTAL_ERROR_ABOVE_BUDGET`); a cancellation case; EstimateOnly and the timing path never admit. |
| POLY-DEV-03 | Closed (research), **Measured: PASS** (L-0013; L-0020) | `laguerre_majorant`: `E_(n+1) = d_n E_n + n/(n+1) E_(n-1) + eps_n`, reported as `recurrence_majorant` and `laguerre_majorant_total`. The status stays EstimateOnly. | The contract checks that the majorant encloses the seven fixture actions at L in {1, 2, 4, 8, 16}. The study's majorant encloses the error against its f64 reference at 44 of 44 points (31 distinct evaluations) and rejects 18% (4 distinct cases). This is an empirical check, not a certificate. It is loose at high degree, up to 3.5e65x. |
| POLY-DEV-04 | **Measured: PASS** (L-0015; L-0022) | `rodas5p r4-study --study polynomial-regimes`: eight arms (basis x certified or unbounded x cached or uncached) and an alternating-h cache control. | Every arm is accurate (at most 3.3e-15) and correctly labelled. Coefficients are reused only for identical keys. Timing is not evaluated (authority hold). |
| POLY-DEV-05 | **Measured: FAIL** (L-0014; L-0021) | `laguerre_scale_for_degree`: `L*(m) = min(16, 2(m+1) - h rho)`, the analytic minimiser. | The contract checks that the analytic tail is minimal at the returned scale. The study never gets a lower degree (0 of 22), so the candidate is rejected and the grid and the cap of 16 stay. |
| POLY-DEV-06 | Closed (research) | `taylor_phi` (schema `vigilode-scaled-taylor-phi-v1`): an augmented `exp(M)` computed by scaled Taylor. The domain witness is an explicit 1-norm with a content digest. The truncation bound is exact-arithmetic, the status is EstimateOnly, and `Certified` is reserved. | `r4_taylor_phi_contracts.rs`: diagonal, nilpotent and nonnormal 2x2 cases (with the departure from normality), plus typed rejections. Arnoldi stays the reference. |
| TIME-DEV-01 | Closed | `same_step` requires exact equality. Unequal steps take the variable coefficients of their actual ratio. | `r4_time_contracts.rs`: constant flow within 1e-12 at boundary powers -40, 0, 10 and 40 through the fixed, observed and dense wrappers; power-of-two unit invariance of values and classification; equal-step and restart controls. The old rule fails the first two. |
| TIME-DEV-02 | Closed | `step_doubling_divisor`: `(1 - eta)/eta` over the represented halves, used by Radau1. Equal halves are bit-identical. After a rejection the next represented step is strictly shorter, or the run fails with a typed time-resolution error. | The same file: the 3-ULP witness estimates 5/9; both Radau1 wrappers reject it or fail on time resolution, with no 100 identical retries; continuity near equal halves. |
| TIME-DEV-03 | Closed | Empty-history startup uses derived divisors. BDF1+BDF1 uses the Radau1 rule. BDF1+BDF2 uses `D = 2 h2/h1`: BDF2 carries the BDF1 error by `(1+r)^2/(1+2r)`, so `e_f = H^2 y''/2/(1+2r)`. Each case has its own estimator id. | The same file: estimates are exact on degree-1 and degree-2 primitives in both orders, for equal and 2+1 ULP halves. On cubics the ratio tends to 1. Rejected startups do not advance history. |
| TIME-DEV-04 | Closed | `MaxStepPolicy::{StrictRepresentedCap, AllowClockResolutionSlack}`, with slack as the default as before. Under the strict policy no accepted step exceeds `max_step`, and a sub-ULP cap is a typed error. | The same file: the cap grid 0.49, 0.75, 1, 1.5 and 2 ULP under both policies, with separate endpoint and cap assertions; a decimal-grid control. |
| HOM-DEV-01 | Closed | `InverseWitness` fields are sealed. `UnverifiedWitness::verify` rebuilds the witness (diagonal, exact-small, or approximate with its V) and requires the bound bit for bit. | `r4_homotopy_contracts.rs`: zero-upper, empty-row, negative, NaN, other-structure and other-operator witnesses are rejected; the round trip works, including for an approximate witness. |
| HOM-DEV-02 | Closed | `doubling_levels = ceil(log2 s)`. Zero stages, shape mismatches and bad radii are typed errors. | The same file: for the exact chains s = 1, 2, 4, 8, 9 and 16, every bound is at least s (it was 8 for s = 9 and 16). |
| HOM-DEV-03 | Closed (research) | `certificate_binding`: a length-prefixed SHA-256 over the target coefficients, `y`, `h`, `J`, `q`, the candidate, `y_hat`, `e_hat`, the witness identity and the scale. `QuadraticModel` generates both the ODE (named with its digest) and the certificate problem. A `GeneratedFromModel` binding holds only for that ODE. | The same file: every input change makes the certificate stale. A foreign ODE gets `SampledAgreement`. |
| HOM-DEV-04 | Closed | The q1/q2 step borrows a `ParallelExecution`. An integration builds at most one pool. The doubling certificate builds one pool per call, or none when given an execution context. | The same file: `pool_creations` is 0 at 1 thread and 1 at 3 threads, and the results are bit-identical. |
| HOM-DEV-05 | Closed (research) | `blocked_doubling_certificate_with_execution` handles diagonal J with a diagonal witness: n stage blocks in parallel, counting allocation, nonzeros and operations. | The same file: on the 12 diagonal (all scalar) R3 fixtures each path certifies exactly where the fixture says it closes (10 of 12), and the certifying bounds are bit-identical to the full ones and enclose the exact distances. A stacked n = 6 problem of six certifying rows certifies on both paths, bit for bit. Work scaling is checked on a separate n = 6 problem. Non-diagonal inputs are refused. |
| HOM-DEV-06 | Closed | `Q2CertificateSource::capability` builds the witness before any speculative work. DimensionCutoff, MathematicalReject and ProblemUnavailable skip the q2 escalation (`CERTIFICATE_CAPABILITY_UNAVAILABLE`). | The same file: coupled-linear-6 falls back without q2 batches. `q2_diagnostic_replacement_contracts.rs` is updated: the no-structure source spends fewer than 7 batches and 0 certificate attempts. |
| HOM-DEV-07 | **Measured: FAIL** (L-0017; L-0024) | `rodas5p r4-study --study homotopy-cost`. | Bit identity holds for n = 1, 2 and 4. At n = 8 and 16 neither doubling certificate closes. Serial is the cheapest. `SPEEDUP_UNPROVEN`. |
| STAT-DEV-01 | Closed | `timing_authority`: a compiled study registry. `AdmissibleTimingDecision` has private fields, is not deserializable and is built only by `admissible_decision` from the compiled registry. A caller-supplied registry yields only a `HypotheticalTimingDecision` (`would_admit`, `registry_verified`), and forged entries fail `verify_timing_authority`. The CLI wall criterion uses `admissible_decision`; its counterfactual registry exists only in test builds. `rodas5p timing-authority` reports a published campaign. | `r4_statistics_contracts.rs`; CLI unit tests: a synthetic Promote is recorded but not acted on; POLY03 (L-0009) keeps its numbers and reports a hold. |
| STAT-DEV-02 | Closed | Every raw cell is validated before merging. A missing cell with no explanation is recorded as a failure. | The same file: empty or single warmups, 29/31 shifts, end-before-start, a wrong later batch and a NaN start are rejected; the five R3 rejections still hold; the published POLY03 receipts stay valid. |
| STAT-DEV-03 | Closed | `POOLED_PAIR_MEDIAN_ESTIMAND` is on every assessment (records written before R4 deserialize to it). `SESSION_CELL_MEDIAN_ESTIMAND` is on the new interval. `docs/TIMING_DESIGN_CONTRACT.md` is updated. | The same file: an asymmetric law gives 3 against 0, a symmetric law gives equal values, and the registry does not let one estimand stand in for the other. |
| STAT-DEV-04 | Closed (research), **Measured: PASS** (L-0018; L-0025) | `session_median` uses exact integer tails with no `2^S` overflow, and is unbounded when infeasible. `session-median-coverage-study` runs the study. | The R4 table is reproduced: 32 designs, 6 enumerations, 12 controls and 4 invalid inputs; S6/C1 gives 31/32, S6/C5 is unbounded, S8/C5 gives 123/128. In the study, 96 in-domain scenarios and 40 exact checks pass (gate power: see the second review). The registry entry covers only the simulated session counts S in {6, 8, 12, 24}. It is held pending independent review. |
| STAT-DEV-05 | **Measured: PASS** (L-0016; L-0023, L-0027), acceptance partial | The cost-separation part of `polynomial-regimes`. The homotopy pool moved out of the step loop (HOM-DEV-04). | Timing arms do the same polynomial work (degrees, block products, coefficient setups) as the certified arms. Both arms run one routine, so this holds by construction. The enclosure arithmetic of the certified arm is not counted: no counter exists. In the warm wall diagnostic the certified arm takes 9.9x (cached Chebyshev, 0.0211 s against 0.00215 s), 2.4x (uncached Chebyshev), 22.8x (cached Laguerre) and 2.5x (uncached Laguerre) the unbounded arm's time. The formal eigensystem crossover is 3 actions, with no speed claim. |
| VERIFY-DEV-01 | **Measured: PASS** (L-0019; L-0026) | `tools/r4_run_named_tests.py`, which runs each test with `--exact` and a 3600 s budget. A timeout is never a pass. | `research/r4_runtime_remainder_20261001/`: all 60 named tests pass (longest 409 s, total 1585 s). The four ignored tests are listed and not run. |

## Experiments

Every experiment was preregistered and pushed (`95cb66e`) before its first output. The host is a shared 4-vCPU cloud
container, and no wall time is used as evidence. The ledger id in parentheses is the superseding row that corrects
the original row's provenance (see the second review). L-0027 further corrects the profile and the overhead figures
of L-0023. In the node matrix, the id after the semicolon is the current row.

| Ledger | Node | Verdict | Numbers |
|---|---|---|---|
| L-0013 (L-0020) | POLY-DEV-03 | PASS | Encloses 44/44 against an f64 reference (31 distinct); rejection 0.18 (4 distinct cases); median looseness 157x, maximum 3.5e65x |
| L-0014 (L-0021) | POLY-DEV-05 | FAIL | Lower degree on 0 of 22 points; majorant up to 27x lower at equal degree (diag24-narrow, h = 0.1, budget 1e-6) |
| L-0015 (L-0022) | POLY-DEV-04 | PASS | Maximum relative error 3.3e-15; reuse only on identical keys (2 setups, 18 reuses) |
| L-0016 (L-0023, L-0027) | STAT-DEV-05 | PASS | Same polynomial work in timing and certified arms; enclosure overhead not counted (2.4x to 22.8x in the warm wall diagnostic); eigensystem cheaper (formally) from action 3 |
| L-0017 (L-0024) | HOM-DEV-07 | FAIL | Doubling radius does not close at n = 8 and 16; serial costs 425 to 26000 operations |
| L-0018 (L-0025) | STAT-DEV-04 | PASS | 96/96 in-domain scenarios, worst margin -0.0023; exact checks 40/40; drift fails out of domain |
| L-0019 (L-0026) | VERIFY-DEV-01 | PASS | 60/60 named tests pass one at a time (0 FAIL, 0 TIMEOUT, 0 NOT_FOUND) |

### Process deviations (disclosed)

- **Laguerre study rerun.** The first run of the preregistered Laguerre command aborted before writing any output.
  The scalar phi enclosure supports `z >= -600`, and the study reached `z = -1000`. Only the study's reference code
  changed (`d700572`); the preregistration addendum records this before the rerun.
- **Duplicate debugging runs.** The polynomial-regimes and homotopy-cost commands wrote their official outputs on the
  first attempt. During the Laguerre debugging they were run a second time into a scratch directory, and those
  outputs were deleted unread. The studies are deterministic.
- **Interrupted test run.** One full workspace test run was stopped midway after the independent review, and repeated
  on the fixed code: 694 tests passed.
- **VERIFY-DEV-01 restart.** The first VERIFY-DEV-01 invocation ran under a tool with a 2-hour limit. It was stopped
  after 9 passes, before any output was written, and rerun as a detached process.
- **Ledger provenance.** Rows L-0013..L-0019 hashed their inputs at the branch head rather than at the execution
  commit. For L-0015..L-0019 each preregistration hash is that of the file after its results had been appended; the
  preregistration hash of L-0013 and L-0014 (`033767d4...`) matches no committed version of the file and cannot be
  traced. For L-0015..L-0017 the recorded
  `r4_studies.rs` hash was the `d700572` version, not the `95cb66e` code that ran; the change between them touches
  only `z < -600` and not these studies. The rows stay as written (append-only). L-0020..L-0026 supersede them with
  input hashes taken at each row's execution commit, where the preregistration is the text before its results.
- **Registry entry after the run.** The session-cell study was added to the authority registry as a hold after its
  run, as preregistered.

## Behaviour changes to existing results

The R4 fixes change some outputs that earlier records may hold. Each change has a new identifier, so none of them is
silent.

- **Adaptive BDF2.** The startup error estimate is halved at equal halves (`D = 2` instead of 1). Its estimator id is
  now `bdf-startup-bdf1-bdf2-mixed-v1`.
- **Adaptive Radau1.** Unequal represented halves use the geometry divisor.
- **Adaptive drivers.** After a rejection, the next represented step is strictly shorter. This changes step sequences
  at ULP scale only.
- **Fixed BDF.** Exact step equality changes which coefficients are used for steps that differ by less than
  `32 eps max(|h|, 1)`.
- **Certificate mode with an unavailable witness.** The q2 escalation is skipped, for example on the non-diagonal
  `n > 2` systems of the R3 HOM-06 campaign. Lane counts and batch counts of such runs change if they are repeated.
  The published L-0008 run is not modified.
- **Paired timing.** A verified Promote of a pooled-pair design is no longer acted on (authority hold), and missing
  cells are recorded as failures.

## Independent review

An independent adversarial review read the whole diff before the experiments.

**Blocker.** In certificate mode the HOM-06 skip broke an existing contract. The contract now asserts the new
semantics.

**Major findings:**

- The certificate binding omitted `y_hat`, `e_hat` and the witness identity. They were added, with length-prefixed
  fields.
- `ModelBinding::GeneratedFromModel` was self-declared. It is now checked against the integrated problem's digest.
- Three preregistered gates could not fail:
  - The HOM-07 identity gate passed when neither certificate existed. It now requires both to certify.
  - The HOM-07 cost gate compared mismatched counts. Costs are now reported, not gated.
  - The STAT-05 field checks were tautological. The gate now requires the timing arm's work to equal the certified
    path's.
  - The STAT-04 gate gained a two-sided exact check for C = 1, and the trivially passing unbounded designs are
    disclosed.

**Minor findings** (all fixed):

- Misplaced documentation.
- `capability` ignoring an overridden `witness`.
- Missing cells counted twice.
- First-batch binding after a failed first session.
- The median of an empty cell.
- Rational-to-float conversion for large S.
- Vacuous test branches.

The review found the following sound:

- The `ExpBound` order, norm and reciprocal factorials.
- The acyclic nilpotency claim.
- The normalization perturbation term.
- The Richardson, BDF-startup, Laguerre-majorant and continuous-scale derivations.
- Blocked bit identity.
- The exact binomial arithmetic.
- The authority gate.

## Second independent review

A second adversarial review read the closure, the ledger rows and the code since the first review. It recomputed
every published number from the outputs. The raw outputs and verdicts were right; some of the numbers quoted in the
text were not (the minor findings below). It found no blocker. A third, read-only check of the fix commit found no
blocker or major finding; its wording corrections (the per-arm overhead, the L-0023 profile through L-0027, the
untraceable L-0013/L-0014 preregistration hash, and the current ledger ids) are applied here.

**Major findings:**

- **STAT-DEV-05 overstated.** "Timing work equals verified work" held for polynomial work only, and by
  construction. The claim is narrowed (L-0023 and L-0027, this document); counting the enclosure work stays open.
- **Ledger provenance.** Input hashes were not those of the execution commit (see the process deviations). Fixed
  by superseding rows L-0020..L-0026.
- **HOM-DEV-05 test partly vacuous.** A `(None, None)` branch and an `Option` comparison passed when neither path
  certified. The test now requires each row to certify exactly when the fixture says it closes (10 of 12), and a
  stacked n = 6 problem must certify on both paths.
- **Public counterfactual registry.** `admissible_decision_in` accepted any registry, so a hand-built Admissible
  entry produced an admissible decision. It is removed. A caller-supplied registry now yields only a
  `HypotheticalTimingDecision`, and `AdmissibleTimingDecision` cannot be built or deserialized outside the library.

**Minor findings** (all addressed):

- STAT-04 gate power, in the L-0018 preregistration results and L-0025:
  - The one-sided tolerance of 0.0222 accepts a miss rate up to about 0.030 at S8/C1, about 3.8 times the nominal
    0.0078, and a zero-miss implementation passes that design.
  - At C = 1, three of the five continuous laws are one distribution and theta only shifts the location, so the 40
    exact checks cover about 3 laws x 4 session counts.
  - The binomial z-scores of the exact checks lie in [-2.27, 2.03].
- "Certified region" and "sound on the grid" wording for an empirical check against an f64 reference, and the
  median looseness: it is 157x; the published 162x is the upper median.
- The Laguerre results text placed the 27x reduction at h <= 0.01. It is at diag24-narrow, h = 0.1, budget 1e-6.
- The POLY-DEV-03 contract count: seven actions at L in {1, 2, 4, 8, 16}. The preregistration's prior information
  also says six; that text is preregistered and stays, and its results section records the correction.
- The formal eigensystem cost per action ignored `Q^T` on the five input columns. The model is now `6 n^2 + 25 n` =
  4056 at n = 24 (the published study shows 1752). The crossover stays at 3 actions (124416 / (60480 - 4056) = 2.2).
- The session-cell registry domain is narrowed to the simulated counts S in {6, 8, 12, 24}.
- The receipt warmup comment claimed the producer's `2 * warmups` count while the check enforces only `warmups`. The
  comment now states the lower bound. The published receipts hold `2 * warmups`; hand-built test receipts hold
  `warmups`.
- `ExactRatio::as_f64` is used for gate floats only. It rounds in no fixed direction and underflows for tiny q.
  No change.

## Open items

- **Timing authority.** No timing decision is admissible. The pooled-pair designs are held because L-0007 and L-0010
  failed. The session-cell interval passed its study (L-0018, corrected by L-0025) and waits for an independent domain review, as the
  R4 gate requires. No registry entry is admissible, and only a code change to the compiled registry can make one
  admissible.
- **HOM-DEV-07.** The doubling radius schedule does not close at n >= 8. Next steps are componentwise radii or the
  past-step radius predictor, measured before any speed campaign. The matched timing campaign is not run while timing
  is on hold.
- **STAT-DEV-05 enclosure accounting.** No work counter covers the certified arm's enclosure and recurrence
  arithmetic. The DAG acceptance for separated costs is therefore met for polynomial work only.
- **Laguerre certification.** The baseline majorant enclosed the error against an f64 reference at every grid point
  but is far too loose at high degree. Laguerre
  stays EstimateOnly, and a tighter transfer bound is still open.
- **POLY-DEV-06.** The scaled-Taylor backend stays EstimateOnly; `Certified` is reserved until a rounding bound is
  derived.
