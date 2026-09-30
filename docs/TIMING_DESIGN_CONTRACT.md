# Paired timing design contract (re-audit R3, STAT-DEV-04)

Contract id: `vigilode-timing-design-v1/corpus-median-of-case-session-medians/balanced-cells`
(`rodas5p_fair_ab::TIMING_DESIGN_CONTRACT`). Implementation:
`crates/rodas5p-fair-ab/src/timing_design.rs`; decision rule:
`crates/rodas5p-fair-ab/src/paired_timing.rs`.

## Estimand

A paired observation is `Y = ln(reference seconds / candidate seconds)` for one ABBA pair.
For case `c` the case estimand is the population median of `Y` over independent sessions
(processes) and pairs. The campaign estimand is the median over the **declared case
corpus** of the case estimands, each case weighted equally. The corpus is the population
of the claim: nothing is said about cases outside it. The speedup is `exp` of the
estimand, and promotion needs its lower confidence limit to be at least
`PAIRED_TIMING_REQUIRED_SPEEDUP = 1.15`.

## Design

| Element | Fixed value |
|---|---|
| Independent unit | a session: one process, one provenance record (`SessionProvenance`) |
| Sessions | at least `PAIRED_TIMING_MIN_INDEPENDENT_BLOCKS = 6` distinct labels |
| Pairs per session and case (cell) | exactly `protocol.pairs` (at least 30), in the per-session seeded ABBA order |
| Case weights | equal (median of case medians) |
| Resampling | sessions and cases, two-way, `PAIRED_TIMING_BOOTSTRAP_RESAMPLES = 10000`, never extended after seeing data |
| Decision | percentile endpoints **and** the Monte-Carlo gate (`delta = 0.01`) must agree, otherwise Inconclusive |

Duplicating a session under its own label adds pairs, not an independent session.
Repeating pairs within a session changes no bit of the interval (the medians are
invariant). Both are contract tests (`tests/r3_timing_design_contracts.rs`). A session
relabelled with a fresh label is a fabricated provenance record; receipts are not
authenticated, so this is excluded only by the provenance fields the receipt binds (see
`docs/REAUDIT_R3_CLOSURE_20261001.md`).

## Missing cells and failures

A missing cell (a session that did not measure a case) stays missing and is never
imputed. A case is excluded only when fewer than `protocol.pairs` of its pairs remain.
Failed sessions stay in the receipt (`failed_sessions`) and count in the campaign
denominator. The coverage study measures the sensitivity to missing cells that are
unrelated to the outcome (MCAR 0.1 and 0.3) and to missing cells that flatter the
candidate (MNAR 0.3).

## Resampling error versus population coverage

The Monte-Carlo gate bounds only the simulation error of a finite number of resamples for
the observed data. Population coverage of the interval is a separate property of the
design and the dependence model. It is measured by
`rodas5p paired-timing-coverage-study`, and its numeric verdict is in the research ledger.
A decision is authoritative only under a design for which that study passed.

## Frozen campaign recipe

1. Commit a `PREREGISTRATION.md` with the candidate, the case corpus, the holdout cases,
   `protocol` (seed, pairs, warmups, resamples), the number of sessions, the command and
   the PASS/FAIL gate before the first timed session.
2. Run `rodas5p paired-timing-campaign` (one process per session). Its receipt keeps
   every session's provenance, including failed sessions.
3. Assess only the calibration cases. Open the holdout cases once, after the calibration
   verdict is recorded, with the same frozen protocol.
4. Append the numeric result and a `research/LEDGER.jsonl` row with verdict `PASS` or
   `FAIL`.
