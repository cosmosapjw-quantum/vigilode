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

### Estimand versions (re-audit R4, R4-STAT-DEV-03)

The contract id above says "case-session medians", but the statistic
`assess_paired_timing` computes, and that the R3 studies and campaigns used, pools a
case's pairs over all its sessions. The two targets are named and versioned so that
neither can stand in for the other:

| Estimand constant | Target | Used by |
|---|---|---|
| `POOLED_PAIR_MEDIAN_ESTIMAND` (`case-median-of-pooled-pair-log-speedup-median-v1`) | median over cases of the population median of the pooled pair `Y` of the case | `PairedTimingAssessment::estimand`; every R3 record (records without the field deserialize to it) |
| `SESSION_CELL_MEDIAN_ESTIMAND` (`case-median-of-session-cell-median-log-speedup-v1`) | median over cases of the population median `m_c` of the session-cell median `Z_sc` | `exact_session_median_interval` (R4-STAT-DEV-04) |

They coincide in the symmetric additive family of the coverage simulator: with
`Y = theta + v_c + u_s + e` for symmetric independent `u_s` (session) and `e` (pair),
`u_s + e` is symmetric about 0 and so is `u_s + median(e_1..e_P)`, so both targets are
`theta + v_c` per case. Off that family they can differ: with a session effect of 0 or
10 (probabilities 0.6, 0.4), a pair error of 0 or 3 (0.6, 0.4) and 31 pairs per cell,
the pooled-pair median is 3 and the median of cell medians 0
(`tests/r4_statistics_contracts.rs`). An interval, an authority and a study apply only
to the estimand they name.

### Statistical authority (re-audit R4, R4-STAT-DEV-01)

`PairedTimingEvidence::verified_decision` establishes integrity only.
`PairedTimingEvidence::admissible_decision` adds the authority the compiled study
registry (`timing_authority_registry`) assigns to the evidence's design (estimand,
assessment schema, sessions, cases): `Hold` when a covering study failed, `NotEvaluated`
when none covers it, `Admissible` only when every covering study passed and was reviewed.
Both R3 coverage studies failed (ledger L-0007, L-0010), so every pooled-pair design is
on hold; the CLI wall criterion acts only on an admissible decision, and
`rodas5p timing-authority --campaign <CAMPAIGN.json>` reports a published campaign's
diagnostic decision beside its authority without rewriting it. A receipt or a JSON field
cannot raise the status (`verify_timing_authority` accepts only registry entries).

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
imputed; since re-audit R4 (R4-STAT-DEV-02) the receipt records it as a failure
(`missing cell`), so the campaign never gates on the sessions that happened to complete.
Every raw cell is validated before merging: exactly `protocol.pairs` candidate,
reference and order entries, at least `protocol.warmups` finite nonnegative warmups (the
producer writes `2 * protocol.warmups`), the first session's batch calibrated from its own
warmups and run unchanged by every later session, finite ordered session timestamps and
nonempty arm and workload identities.
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
