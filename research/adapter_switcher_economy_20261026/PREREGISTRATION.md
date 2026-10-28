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

## Amendment after the adversarial review (2026-10-27; the registered text above is unchanged)

An independent three-lens review of commit `636cdc7a` (see `ANALYSIS_KO.md`, section 6) confirmed one
MAJOR defect and several MINOR items. The single fix loop changed code only where the review showed a
defect or an unsupported claim; no gate, threshold, cap or bound registered above changed, and the
second recorded run below supersedes the first.

- Entry ceiling and route verification. A `Jacobian::Dense` provider with a declared band is verified
  before any step by a band fill and a dense reference (`2n` charged units). A ceiling that counted
  that verification could end a bounded pilot as `StructureRefused`, which the adapter cannot treat as
  a discarded trial; the whole solve then stopped with incomplete accounting. The ceiling is now armed
  above the verification charge (`Guard::arm_ceiling` after the plan is known), so it bounds driver
  work only. Decision preservation is unchanged: any ceiling at or above the cap never stops a pilot
  that would pass. Regression contract `bounded_pilot_on_a_declared_band_is_discarded_not_refused`.
- Coalesced landings are deduplicated (strictly increasing), so window endpoints below the clock
  resolution collapse onto their neighbour instead of refusing the native run.
- A callback that returns a recoverable error is charged like a completed call (the driver counts
  it), still never above its work units.
- The return ceiling carries one unit of slack for the rounding difference between the product
  `return_cost_ratio * rate * window` and the rate comparison that decides the return.
- Recheck hysteresis baseline: with coalesced tenure, `implicit_rate` is the mean rate of the whole
  implicit run, not of its last window. The recheck re-arm cancels the back-off; the recheck itself
  still happens where the current implicit run ends, as scheduled. Both are documented behaviour, not
  a registered claim.
- Tests: the late-transition contract gained a negative control with the re-arm disabled (entry more
  than 1e-3 after the transition); the recheck contract asserts that the return ceiling actually ended
  a recheck; the stiff-to-smooth contract's bound is now inside the span (3.5 < 4.0); the allocation
  bound is 6 per batch plus 32, measured 376 allocations for 64 batches after and 568 before (packet
  head, same loop), so the bound refuses the pre-change loop.
- Disclosures: `tools/compare.py` at `636cdc7a` mis-gated the registered report-only cell
  (`suite:c10:switching`, default tolerance; every method fails its cap there); the committed
  comparator applies the registered rule (a cap is gated only where a direct candidate attains it);
  with the `636cdc7a` comparator G3 reads False on that one cell and nothing else. The committed
  analysis JSONs omit the per-segment `segments` arrays; the full analyzer outputs and all raw files
  are archived with hashes (`evidence/HOST.json`). Legacy (`carry_explicit_step == false`) rechecks
  now run in a scoped native session so their RHS work can be bounded; their committed-return path is
  unchanged in kind. The decision-preserving argument of the ceilings is mathematical (charged units
  are a lower bound of work units); no separate verdict-identity run was made. The adapter's explicit
  attempt budget for rechecks (16) is a pre-existing design limit: a window that needs more explicit
  attempts than that cannot be returned to. A pre-existing edge found while writing the regression
  contract and left for a follow-up: with `carry_explicit_step == false`, a remaining span below the
  driver's minimum step is refused by the per-batch driver instead of being treated as reached.

## Results, run 1 (L-0170, superseded by run 2 below)

**Verdict: PASS** (G1-G5; see `evidence/GATES.json`, `evidence/COMPARISON.md`, `evidence/TESTS.txt`). Execution commit `636cdc7a49011af3c10f8cf474cc4bb46e249700`; binaries in `evidence/BINARY_SHA256.json`; host and raw manifests in `evidence/HOST.json`. The timing table is **marked unreliable** by the registered drift rule: `bench:rotation:dopri` read 0.225 ms in the before campaign and 0.412 ms in the before-2 control (1.83x), so no wall-time ratio below is a claim; the counter columns carry the result.

| cell | attempts b→a | RHS b→a | JVP b→a | discarded pilots b→a (attempts) | discarded rechecks b→a (attempts) | implicit batches b→a (attempts) | error b→a (cap) | wall ms before / after / before-2 (diag.) | best direct ms b / a / b2 |
|---|---|---|---|---|---|---|---|---|---|
| bench:rotation:evidence | 1046→892 | 6597→5371 | 5246→355 | 6(169)→3(15) | 0(0)→0(0) | 0(0)→0(0) | 9.26e-07→9.26e-07 (1e-05) | 2.024 / 0.477 / 2.050 | dopri 0.225 / 0.219 / 0.412 |
| bench:rotation:screened-reset | 1274→1105 | 8069→6710 | 6312→833 | 8(204)→7(35) | 0(0)→0(0) | 0(0)→0(0) | 9.06e-07→9.06e-07 (1e-05) | 2.446 / 0.722 / 2.522 | dopri 0.225 / 0.219 / 0.412 |
| bench:rotation:pressure-only | 701→666 | 5430→5144 | 20677→19749 | 0(0)→0(0) | 1(16)→1(16) | 8(653)→3(618) | 8.32e-08→8.37e-08 (1e-05) | 6.589 / 6.384 / 6.535 | dopri 0.225 / 0.219 / 0.412 |
| bench:stiff-forced:evidence | 137→66 | 1002→456 | 0→0 | 1(8)→1(8) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | 0.212 / 0.134 / 0.225 | rodas 0.092 / 0.092 / 0.097 |
| bench:stiff-forced:screened-reset | 137→66 | 1002→456 | 0→0 | 1(8)→1(8) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | 0.206 / 0.132 / 0.207 | rodas 0.092 / 0.092 / 0.097 |
| bench:stiff-forced:pressure-only | 129→58 | 938→392 | 0→0 | 0(0)→0(0) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | 0.238 / 0.123 / 0.191 | rodas 0.092 / 0.092 / 0.097 |
| suite:c5:switching | 232→115 | 1633→748 | 3802→988 | 2(23)→2(6) | 3(48)→2(19) | 16(97)→4(26) | 9.86e-10→1.25e-08 (1e-04) | 2.405 / 0.833 / 3.394 | rodas5p-dense 0.137 / 0.183 / 0.176 |
| suite:c10t:switching | 4271→4159 | 26018→25102 | 5120→517 | 4(128)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.89e-05→3.89e-05 (1e-04) | 7.812 / 5.523 / 8.237 | dopri5 2.530 / 2.543 / 2.488 |
| suite:c10:switching | 1321→1209 | 8221→7309 | 5120→517 | 4(128)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.09e-02→3.09e-02 (1e-04) | 4.291 / 1.986 / 4.386 | dopri5 0.713 / 0.718 / 0.729 |

`suite:c10:switching` (suite default tolerance) is the registered report-only cell: every method, including direct DOPRI, exceeds the 1e-4 cap there (0.031), so it is work-gated but not cap-gated. All other cells meet their validated caps. Direct cells (`evidence/COMPARISON.md`, G1) are identical bit for bit in every counter and in the emitted final state.

**Predictions.** Rotation `evidence` 892 attempts (≤ 950) with 15 discarded pilot attempts (≤ 40); case5 115 attempts (≤ 200), 4 implicit batches (≤ 5), 19 discarded recheck attempts (≤ 30); stiff-forced `evidence` 66 attempts (≤ 80); case10 16 discarded pilot attempts at both tolerances (≤ 24); rotation `pressure-only` 666 attempts (before 701). All predictions held.

**Decision sequences after** (`D` explicit batch, `R` implicit batch, `!` discarded, attempts in brackets):

- `bench:rotation:evidence` (switches=0): `0:Rpilot![5] 1:D[32] 2:D[32] 3:D[32] 4:D[16] 5:D[32] 6:Rpilot![5] 7:D[32] 8:D[32] 9:D[12] 10:D[32] 11:D[32] 12:D[32] 13:D[14] 14:D[32] 15:D[32] 16:Rpilot![5] 17:D[32] 18:D[13] 19:D[32] 20:D[32] 21:D[32] 22:D[14] 23:D[32] 24:D[32] 25:D[32] 26:D[13] 27:D[32] 28:D[32] 29:D[32] 30:D[14] 31:D[32] 32:D[32] 33:D[32] 34:D[13]`
- `bench:rotation:screened-reset` (switches=0): `0:Rpilot![5] 1:D[32] 2:D[32] 3:D[32] 4:D[32] 5:D[7] 6:Rpilot![5] 7:D[32] 8:D[32] 9:D[32] 10:D[31] 11:D[32] 12:Rpilot![5] 13:D[32] 14:D[32] 15:D[31] 16:D[32] 17:D[32] 18:D[32] 19:D[32] 20:D[7] 21:Rpilot![5] 22:D[32] 23:D[32] 24:D[32] 25:D[32] 26:D[10] 27:Rpilot![5] 28:D[32] 29:D[32] 30:D[32] 31:D[32] 32:D[8] 33:Rpilot![5] 34:D[32] 35:D[32] 36:D[32] 37:D[32] 38:D[7] 39:Rpilot![5] 40:D[32] 41:D[32] 42:D[32] 43:D[32] 44:D[9]`
- `bench:rotation:pressure-only` (switches=1): `0:D[32] 1:R[141] 2:R[156] 3:Dprobe![16] 4:R[321]`
- `bench:stiff-forced:evidence` (switches=1): `0:Rpilot![8] 1:D[32] 2:R[12] 3:R[4] 4:Dprobe![5] 5:R[5]`
- `bench:stiff-forced:screened-reset` (switches=1): `0:Rpilot![8] 1:D[32] 2:R[12] 3:R[4] 4:Dprobe![5] 5:R[5]`
- `bench:stiff-forced:pressure-only` (switches=1): `0:D[32] 1:R[12] 2:R[4] 3:Dprobe![5] 4:R[5]`
- `suite:c5:switching` (switches=1): `0:Rpilot![5] 1:D[32] 2:Rpilot![1] 3:D[32] 4:R[9] 5:R[3] 6:Dprobe![10] 7:R[5] 8:Dprobe![9] 9:R[9]`
- `suite:c10t:switching` (switches=0): `0:Rpilot![4] 1:D[32] 2:Rpilot![4] 3:D[32] 4:D[32] 5:D[32] 6:Rpilot![4] 7:D[32] 8:D[32] 9:D[32] 10:D[32] 11:D[6] 12:Rpilot![4] 13:D[32] 14:D[32] 15:D[32] 16:D[32] 17:D[32] 18:D[32] 19:D[32] 20:D[32] 21:D[3] 22:D[32] 23:D[32] 24:D[32] 25:D[32] 26:D[32] 27:D[32] 28:D[32] 29:D[32] 30:D[6] 31:D[32] 32:D[32] 33:D[32] 34:D[32] 35:D[32] 36:D[32] 37:D[32] 38:D[32] 39:D[4] 40:D[32] 41:D[32] 42:D[32] 43:D[32] 44:D[32] 45:D[32] 46:D[32] 47:D[32] 48:D[5] 49:D[32] 50:D[32] 51:D[32] 52:D[32] 53:D[32] 54:D[32] 55:D[32] 56:D[31] 57:D[32] 58:D[32] 59:D[32] 60:D[32] 61:D[32] 62:D[32] 63:D[32] 64:D[32] 65:D[2] 66:D[32] 67:D[32] 68:D[32] 69:D[32] 70:D[32] 71:D[32] 72:D[32] 73:D[32] 74:D[3] 75:D[32] 76:D[32] 77:D[32] 78:D[32] 79:D[32] 80:D[32] 81:D[32] 82:D[32] 83:D[32] 84:D[32] 85:D[32] 86:D[32] 87:D[32] 88:D[32] 89:D[32] 90:D[32] 91:D[3] 92:D[32] 93:D[32] 94:D[32] 95:D[32] 96:D[32] 97:D[32] 98:D[32] 99:D[32] 100:D[6] 101:D[32] 102:D[32] 103:D[32] 104:D[32] 105:D[32] 106:D[32] 107:D[32] 108:D[32] 109:D[1] 110:D[32] 111:D[32] 112:D[32] 113:D[32] 114:D[32] 115:D[32] 116:D[32] 117:D[32] 118:D[4] 119:D[32] 120:D[32] 121:D[32] 122:D[32] 123:D[32] 124:D[32] 125:D[32] 126:D[32] 127:D[32] 128:D[32] 129:D[32] 130:D[32] 131:D[32] 132:D[32] 133:D[32] 134:D[32] 135:D[2] 136:D[32] 137:D[32] 138:D[32] 139:D[32] 140:D[32] 141:D[32] 142:D[32] 143:D[32] 144:D[3]`
- `suite:c10:switching` (switches=0): `0:Rpilot![4] 1:D[32] 2:Rpilot![4] 3:D[32] 4:D[8] 5:D[32] 6:Rpilot![4] 7:D[32] 8:D[10] 9:D[32] 10:D[32] 11:D[9] 12:Rpilot![4] 13:D[32] 14:D[32] 15:D[10] 16:D[32] 17:D[32] 18:D[14] 19:D[32] 20:D[32] 21:D[8] 22:D[32] 23:D[32] 24:D[13] 25:D[32] 26:D[32] 27:D[10] 28:D[32] 29:D[32] 30:D[11] 31:D[32] 32:D[32] 33:D[12] 34:D[32] 35:D[32] 36:D[13] 37:D[32] 38:D[32] 39:D[10] 40:D[32] 41:D[32] 42:D[9] 43:D[32] 44:D[32] 45:D[10] 46:D[32] 47:D[32] 48:D[10] 49:D[32] 50:D[32] 51:D[12]`

**Residual gap (reported, not claimed).** On the oscillatory cells the adapter still pays one explicit batch loop over DOPRI's own work (892 against 875 attempts on rotation; 4159 against 4067 on tight gyromotion) plus the bounded pilots; on case5 it pays the registered explicit-first phase (2 batches, 64 attempts) and runs the JVP route, whose direct run (`rodas5p-mf`, 0.635 ms before / 0.989 ms after by this unreliable clock) is itself several times the dense direct run (0.137 / 0.183 ms); the attempt count of the implicit tenure (26 over 4 batches) is now within 16 landings plus one short ramp of the direct 39. Entry on a persistently stiff problem still costs the explicit batches before pressure is observed; the adapter does not infer stiffness, by design.


## Results, run 2 (L-0171) (appended after the recorded run; the registered text above is unchanged)

**Verdict: PASS** on G1-G3 by `tools/compare.py` (`evidence/GATES.json`, `evidence/COMPARISON.md`); G4 and G5 by `evidence/TESTS.txt`. Execution commit `596dcdfae8b95d24f27b57975d9f4626267b8149`; binaries in `evidence/BINARY_SHA256.json`; host and raw manifests in `evidence/HOST.json`. Wall-time drift control (packet-head binaries run twice): worst direct-cell median ratio 1.83x, so the timing columns are **marked unreliable** by the registered rule; the counter columns carry the result.

| cell | attempts b→a | RHS b→a | JVP b→a | discarded pilots b→a (attempts) | discarded rechecks b→a (attempts) | implicit batches b→a (attempts) | error b→a (cap) | wall ms before / after / before-2 (diag.) | best direct ms b / a / b2 |
|---|---|---|---|---|---|---|---|---|---|
| bench:rotation:evidence | 1046→892 | 6597→5371 | 5246→355 | 6(169)→3(15) | 0(0)→0(0) | 0(0)→0(0) | 9.26e-07→9.26e-07 (1e-05) | 2.024 / 0.863 / 2.050 | dopri 0.225 / 0.223 / 0.412 |
| bench:rotation:screened-reset | 1274→1105 | 8069→6710 | 6312→833 | 8(204)→7(35) | 0(0)→0(0) | 0(0)→0(0) | 9.06e-07→9.06e-07 (1e-05) | 2.446 / 0.701 / 2.522 | dopri 0.225 / 0.223 / 0.412 |
| bench:rotation:pressure-only | 701→666 | 5430→5144 | 20677→19749 | 0(0)→0(0) | 1(16)→1(16) | 8(653)→3(618) | 8.32e-08→8.37e-08 (1e-05) | 6.589 / 6.290 / 6.535 | dopri 0.225 / 0.223 / 0.412 |
| bench:stiff-forced:evidence | 137→66 | 1002→456 | 0→0 | 1(8)→1(8) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | 0.212 / 0.212 / 0.225 | rodas 0.092 / 0.080 / 0.097 |
| bench:stiff-forced:screened-reset | 137→66 | 1002→456 | 0→0 | 1(8)→1(8) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | 0.206 / 0.140 / 0.207 | rodas 0.092 / 0.080 / 0.097 |
| bench:stiff-forced:pressure-only | 129→58 | 938→392 | 0→0 | 0(0)→0(0) | 1(16)→1(5) | 8(81)→3(21) | 1.57e-10→7.33e-10 (1e-05) | 0.238 / 0.121 / 0.191 | rodas 0.092 / 0.080 / 0.097 |
| suite:c5:switching | 232→115 | 1633→748 | 3802→988 | 2(23)→2(6) | 3(48)→2(19) | 16(97)→4(26) | 9.86e-10→1.25e-08 (1e-04) | 2.405 / 0.820 / 3.394 | rodas5p-dense 0.137 / 0.122 / 0.176 |
| suite:c10t:switching | 4271→4159 | 26018→25102 | 5120→517 | 4(128)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.89e-05→3.89e-05 (1e-04) | 7.812 / 9.216 / 8.237 | dopri5 2.530 / 4.613 / 2.488 |
| suite:c10:switching | 1321→1209 | 8221→7309 | 5120→517 | 4(128)→4(16) | 0(0)→0(0) | 0(0)→0(0) | 3.09e-02→3.09e-02 (1e-04) | 4.291 / 2.114 / 4.386 | dopri5 0.713 / 0.770 / 0.729 |

Report-only cells (no direct candidate attains the cap): `suite:c10:switching`. All other cells meet their validated caps; direct cells are identical bit for bit in every counter and in the emitted final state (G1).

**Decision sequences after** (`D` explicit batch, `R` implicit batch, `!` discarded, attempts in brackets):

- `bench:rotation:evidence` (switches=0): `0:Rpilot![5] 1:D[32] 2:D[32] 3:D[32] 4:D[16] 5:D[32] 6:Rpilot![5] 7:D[32] 8:D[32] 9:D[12] 10:D[32] 11:D[32] 12:D[32] 13:D[14] 14:D[32] 15:D[32] 16:Rpilot![5] 17:D[32] 18:D[13] 19:D[32] 20:D[32] 21:D[32] 22:D[14] 23:D[32] 24:D[32] 25:D[32] 26:D[13] 27:D[32] 28:D[32] 29:D[32] 30:D[14] 31:D[32] 32:D[32] 33:D[32] 34:D[13]`
- `bench:rotation:screened-reset` (switches=0): `0:Rpilot![5] 1:D[32] 2:D[32] 3:D[32] 4:D[32] 5:D[7] 6:Rpilot![5] 7:D[32] 8:D[32] 9:D[32] 10:D[31] 11:D[32] 12:Rpilot![5] 13:D[32] 14:D[32] 15:D[31] 16:D[32] 17:D[32] 18:D[32] 19:D[32] 20:D[7] 21:Rpilot![5] 22:D[32] 23:D[32] 24:D[32] 25:D[32] 26:D[10] 27:Rpilot![5] 28:D[32] 29:D[32] 30:D[32] 31:D[32] 32:D[8] 33:Rpilot![5] 34:D[32] 35:D[32] 36:D[32] 37:D[32] 38:D[7] 39:Rpilot![5] 40:D[32] 41:D[32] 42:D[32] 43:D[32] 44:D[9]`
- `bench:rotation:pressure-only` (switches=1): `0:D[32] 1:R[141] 2:R[156] 3:Dprobe![16] 4:R[321]`
- `bench:stiff-forced:evidence` (switches=1): `0:Rpilot![8] 1:D[32] 2:R[12] 3:R[4] 4:Dprobe![5] 5:R[5]`
- `bench:stiff-forced:screened-reset` (switches=1): `0:Rpilot![8] 1:D[32] 2:R[12] 3:R[4] 4:Dprobe![5] 5:R[5]`
- `bench:stiff-forced:pressure-only` (switches=1): `0:D[32] 1:R[12] 2:R[4] 3:Dprobe![5] 4:R[5]`
- `suite:c5:switching` (switches=1): `0:Rpilot![5] 1:D[32] 2:Rpilot![1] 3:D[32] 4:R[9] 5:R[3] 6:Dprobe![10] 7:R[5] 8:Dprobe![9] 9:R[9]`
- `suite:c10t:switching` (switches=0): `0:Rpilot![4] 1:D[32] 2:Rpilot![4] 3:D[32] 4:D[32] 5:D[32] 6:Rpilot![4] 7:D[32] 8:D[32] 9:D[32] 10:D[32] 11:D[6] 12:Rpilot![4] 13:D[32] 14:D[32] 15:D[32] 16:D[32] 17:D[32] 18:D[32] 19:D[32] 20:D[32] 21:D[3] 22:D[32] 23:D[32] 24:D[32] 25:D[32] 26:D[32] 27:D[32] 28:D[32] 29:D[32] 30:D[6] 31:D[32] 32:D[32] 33:D[32] 34:D[32] 35:D[32] 36:D[32] 37:D[32] 38:D[32] 39:D[4] 40:D[32] 41:D[32] 42:D[32] 43:D[32] 44:D[32] 45:D[32] 46:D[32] 47:D[32] 48:D[5] 49:D[32] 50:D[32] 51:D[32] 52:D[32] 53:D[32] 54:D[32] 55:D[32] 56:D[31] 57:D[32] 58:D[32] 59:D[32] 60:D[32] 61:D[32] 62:D[32] 63:D[32] 64:D[32] 65:D[2] 66:D[32] 67:D[32] 68:D[32] 69:D[32] 70:D[32] 71:D[32] 72:D[32] 73:D[32] 74:D[3] 75:D[32] 76:D[32] 77:D[32] 78:D[32] 79:D[32] 80:D[32] 81:D[32] 82:D[32] 83:D[32] 84:D[32] 85:D[32] 86:D[32] 87:D[32] 88:D[32] 89:D[32] 90:D[32] 91:D[3] 92:D[32] 93:D[32] 94:D[32] 95:D[32] 96:D[32] 97:D[32] 98:D[32] 99:D[32] 100:D[6] 101:D[32] 102:D[32] 103:D[32] 104:D[32] 105:D[32] 106:D[32] 107:D[32] 108:D[32] 109:D[1] 110:D[32] 111:D[32] 112:D[32] 113:D[32] 114:D[32] 115:D[32] 116:D[32] 117:D[32] 118:D[4] 119:D[32] 120:D[32] 121:D[32] 122:D[32] 123:D[32] 124:D[32] 125:D[32] 126:D[32] 127:D[32] 128:D[32] 129:D[32] 130:D[32] 131:D[32] 132:D[32] 133:D[32] 134:D[32] 135:D[2] 136:D[32] 137:D[32] 138:D[32] 139:D[32] 140:D[32] 141:D[32] 142:D[32] 143:D[32] 144:D[3]`
- `suite:c10:switching` (switches=0): `0:Rpilot![4] 1:D[32] 2:Rpilot![4] 3:D[32] 4:D[8] 5:D[32] 6:Rpilot![4] 7:D[32] 8:D[10] 9:D[32] 10:D[32] 11:D[9] 12:Rpilot![4] 13:D[32] 14:D[32] 15:D[10] 16:D[32] 17:D[32] 18:D[14] 19:D[32] 20:D[32] 21:D[8] 22:D[32] 23:D[32] 24:D[13] 25:D[32] 26:D[32] 27:D[10] 28:D[32] 29:D[32] 30:D[11] 31:D[32] 32:D[32] 33:D[12] 34:D[32] 35:D[32] 36:D[13] 37:D[32] 38:D[32] 39:D[10] 40:D[32] 41:D[32] 42:D[9] 43:D[32] 44:D[32] 45:D[10] 46:D[32] 47:D[32] 48:D[10] 49:D[32] 50:D[32] 51:D[12]`
