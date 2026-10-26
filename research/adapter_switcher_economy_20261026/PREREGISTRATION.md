# Preregistration: bounded trials, back-off and coalesced implicit tenure in the switching adapter

Node `adapter_switcher_economy_20261026`, branch `research/adapter-switcher-perf-20261026`, from the audit
packet head `4fe67602502440d76aa9dd245c6b1d67558c14bf` (`research/adapter-local-followup-20261025`; packet
`external_reports/adapter_local_followup_20261025/`, measurement source `d914b4c8`). It answers the packet's
open items N05 (`HOLD_OPTIMIZATION`, repeated discarded pilots) and N03 (RODAS tenure reconstructed every
window) for the adapter in `crates/vigilode/src/switching.rs`.

**Claim boundary.**
- Deterministic `WorkCounters` (attempts, RHS, JVP, Jacobian builds, linear solves) are the authority.
  Single-process wall time is reported with `"timing_authority": "diagnostic_only"` and never gated.
- No local error test, tolerance, hysteresis fraction or entry/return threshold is relaxed. Every discarded
  trial stays fully charged. Accepted state, time and step history never change on a discarded trial.
- Scope is the opt-in switching adapter and the facade entry it needs. Direct `Dopri5::solve` and
  `Rodas5p::solve` must stay bit-identical. No production promotion; the adapter still classifies observed
  method fitness, not physical stiffness. No NCP, no holdout profile, no `Cargo.toml`/`Cargo.lock` change.

## Question

The packet measured the selected switching path against the best direct method at a common error cap and
found it slower in four of five cohorts (rotation 8.0x, gyromotion 3.1x, case5 14.6x, stiff-forced 2.0x by
wall time) while its RHS count was only 1.0-1.3x higher on the oscillatory cohorts. It attributed the loss to
"repeated pilots and the cost of trying the explicit method first" and held N05. Which adapter-side costs
account for the gap, and can they be removed without changing any entry or return decision that the
registered thresholds would make, and without missing a late stiff transition?

## Diagnosis basis (reproduced before this registration)

The packet's binaries (`BINARY_SHA256_FINAL.json`: `evidence_switch_bench ab812631…`,
`local_bottleneck_suite 12e66ce2…`) were re-run on this host (AMD Ryzen 9 5900X, `taskset -c 0`, warm-up 1 +
5 timed) and every switching segment was bucketed by method and disposition (`BASELINE_4fe67602.json`,
produced by `tools/analyze.py` from that raw). Findings, all deterministic counts:

| cell | total attempts | wasted trials | RODAS tenure | best direct |
|---|---|---|---|---|
| rotation, `evidence_driven` | 1046 | 6 discarded MF-RODAS pilots, 169 attempts, 1334 RHS, 5246 JVP | none | DOPRI 875 attempts |
| stiff-forced, `evidence_driven` | 137 | 1 pilot (passes), 1 discarded probe 16 attempts | 8 windows restarted from h0 = 1e-7: 81 attempts | RODAS 14 attempts |
| case5, suite `switching` | 232 | 2 pilots 23 attempts, 3 discarded probes 48 attempts | 16 windows restarted from h0: 97 attempts | RODAS-MF 39 attempts |
| case10 (gyro), suite `switching` | 1321 | 4 incomplete pilots, 128 attempts, 5120 JVP | none | DOPRI 1105 attempts |

Per attempt, a matrix-free RODAS pilot attempt costs roughly 10 µs against 0.3 µs for a DOPRI attempt on
these 2-6 dimensional problems, so the 169 pilot attempts explain about 85 % of the rotation wall-time gap
although they are 16 % of its attempts. The RODAS tenure restarts explain 58 of case5's 97 implicit attempts
(direct rate 19.5 attempts per unit time against 48.5 inside the adapter). The remaining costs are per-batch
allocation and record construction in the explicit tenure loop.

The packet's own claims hold with two corrections: (1) the wall-time gap is not an RHS-count gap, and the
packet never divided wall time by disposition, which hides that pilots dominate; (2) "RODAS tenure still
reconstructed" (N03 partial) is the dominant case5 cost, not a secondary one. The packet's refusal to lower
fixed thresholds is kept here.

## Change (every threshold unchanged; new controls default to the bounded behaviour, legacy reachable)

1. **Entry work ceiling (decision-preserving).** A same-checkpoint implicit pilot can pass only if its work
   units are at most `entry_cost_ratio` times the explicit batch's. `Rodas5p::solve_observed_bounded`
   (crate-private) runs the pilot with a callback work ceiling `floor(entry_cost_ratio * W_explicit)` in the
   facade `Guard` (RHS, JVP, time derivative 1 unit; dense/band Jacobian `n` units; charged after each
   completed call). Since work units are at least the charged callback units, a pilot stopped by the ceiling
   could not have passed; the stop is `FailureReason::WorkCeilingReached` and the segment decision names the
   ceiling. Every call made stays counted.
2. **Return work ceiling (decision-preserving).** An explicit recheck can return only if its work rate is at
   most `return_cost_ratio` times the last implicit rate; it now runs in a scoped native session one attempt
   at a time and stops once its RHS work exceeds `floor(return_cost_ratio * implicit_rate * window)`.
3. **Pilot back-off with trend re-arm.** After every discarded pilot beyond the first, the entry cooldown
   multiplies by `pilot_backoff_factor` (default 2; 1 restores the fixed cooldown). An explicit batch whose
   work rate is at least `rearm_rate_ratio` (default 2) times the rate observed at the last discarded pilot
   re-arms the pilot at once, so a late stiff transition is tried immediately.
4. **Recheck back-off with trend re-arm.** After every discarded explicit recheck beyond the first, the
   recheck interval multiplies by `probe_backoff_factor` (default 2), applied at most twice. An implicit rate
   falling to `1/rearm_rate_ratio` of the rate before the last discarded recheck re-arms it.
5. **Coalesced implicit tenure.** Between rechecks the RODAS route integrates every window up to the next
   scheduled recheck in one native run, landing on each window endpoint (every endpoint still appears in the
   output), and the next implicit run of the same tenure starts from the previous run's mean accepted step
   instead of the user's `h0`. `SwitchingSegment::windows` records the windows a batch covered.
6. **Allocation hygiene.** The explicit tenure batch loop builds its public driver record once from a
   copyable session snapshot, keeps a single request copy per tenure, no longer clones the checkpoint
   state for a pilot it may not run, and stores decisions as static strings.

New `SwitchingConfig` fields: `pilot_backoff_factor`, `probe_backoff_factor`, `rearm_rate_ratio`; validated
before any callback. New public variant `FailureReason::WorkCeilingReached`. `SwitchingSegment::decision`
is now `&'static str`.

## Cells

Bench (`evidence_switch_bench`, rtol 1e-7, atol 1e-10, h0 1e-7, span [0, 1], cap 1e-5): rotation and
stiff-forced, each in `evidence`, `screened-reset`, `pressure-only` (switch cells) and `dopri`, `rodas`
(direct cells). Suite (`local_bottleneck_suite --size standard --emit-state`): case 5 with the external
endpoint reference `research/local_frontier_native_diffsol_20261024/stop_audit_20261025/evidence/case_references/case05/EXTERNAL_REFERENCE_PRECOMMIT.json`
(SHA-256 `3430e710…`, cap 1e-4) in `switching`, `rodas5p-mf`, `rodas5p-dense`; case 10 at
`--rtol 1e-9 --atol 1e-12` (the packet's passing setting, cap 1e-4) in `switching`, `dopri5`, `rodas5p-mf`;
case 10 at the suite default tolerance in `switching`, `dopri5` (every method fails the cap there; reported,
not gated). The suite `switching` config is `ProvidedOnly, window_count 16, max_dense_dimension 256,
implicit_pilot_attempt_budget 32, implicit_pilot_limit 4`.

## Measurement

`tools/run_campaign.sh BIN RAW` runs every cell once as warm-up and five times timed, pinned to one core,
one process at a time, in a fresh raw directory (create-only). It is run three times: with the packet-head
binaries (before), with the binaries built from this node's execution commit (after), and with the
packet-head binaries again (before-2, a wall-time drift control). `tools/analyze.py RAW OUT.json BIN COMMIT`
buckets every segment and records counters, final states, timing and determinism across the six runs;
`tools/compare.py BEFORE AFTER OUT.md GATES.json` evaluates the gates. Raw files and per-segment rows stay
outside the repository; the node keeps the three analysis JSONs, the comparison, gate verdicts, binary hashes
and host identity.

## Gates (deterministic; evaluated by `tools/compare.py` and the test suite)

- **G1** Every direct cell is bit-identical before and after: all counters, success, and the emitted final
  state (bench record and suite `--emit-state`).
- **G2** On every switching cell: total attempts and RHS calls are not higher than before; discarded pilot
  attempts and discarded recheck attempts are not higher; the number of committed implicit batches is not
  higher.
- **G3** On every switching cell: the solve succeeds, the validated error cap is met (bench 1e-5; case 5 and
  tight case 10 1e-4), segment sums reconcile with the totals, and all six runs are deterministic.
- **G4** `cargo test -p vigilode` passes, including the eight new contracts in
  `crates/vigilode/tests/switching_economy_contracts.rs` (ceiling bound and exactness, geometric pilot gaps
  with immediate re-arm at a late stiff transition, recheck bound and back-off, return after a
  stiff-to-smooth transition, coalesced landings with step seeding, allocation bound, control validation)
  and every pre-existing switching, session and automatic-adapter contract.
- **G5** The allocation bound (at most 12 allocations per committed explicit batch plus 48) holds.

**Verdict.** PASS only if G1-G5 all hold; otherwise FAIL. No threshold, cap or bound in this file is changed
after the run. Wall-time ratios are reported with the before-2 drift control; if any direct cell's median
drifts by more than 1.5x between the two before campaigns, the timing table is marked unreliable (the
counter gates are unaffected).

## Prior information (disclosed)

A development measurement of the uncommitted source on 2026-10-26 (same host and protocol, same cells as the
baseline) was observed before this registration: rotation `evidence` 1046 to 892 attempts with 3 pilots of
5 attempts; stiff-forced `evidence` 137 to 66 attempts; case5 232 to 178 attempts with 4 implicit batches;
case10 1321 to 1209 attempts with 4 pilots of 4 attempts; all direct cells identical. That run used the
first version of the back-off (which doubled the cooldown already after the first discard and delayed the
case5 entry by two batches); the registered policy keeps the first discard's cooldown. Its wall-time column
was unusable for the matrix-free cells because the direct RODAS cell itself read 1.7x slower during that
run, which is why the before-2 control is part of the protocol.

## Predictions

Rotation `evidence`: at most 950 attempts and at most 40 discarded pilot attempts. Case5: at most 200
attempts, at most 5 committed implicit batches, at most 30 discarded recheck attempts. Stiff-forced
`evidence`: at most 80 attempts. Case10 (either tolerance): at most 24 discarded pilot attempts. Rotation
`pressure-only`: attempts not above before. Every error at or below its cap. Direct cells identical.
