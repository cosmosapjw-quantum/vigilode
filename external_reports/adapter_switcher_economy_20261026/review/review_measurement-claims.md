# Adversarial review — measurement and claim critique
Worktree /home/cosmosapjw/vigilode-adapter-perf, branch research/adapter-switcher-perf-20261026, diff 4fe67602..HEAD (0d3c7c51; execution commit 636cdc7a). Read-only; no builds, no runs.

## F1 MAJOR — gate evaluator edited after the run; the EXEC_SHA comparator reports FAIL
- research/adapter_switcher_economy_20261026/tools/compare.py:44 (HEAD); PREREGISTRATION.md:122-123, :161; research/LEDGER.jsonl:167
- Claim: "PASS only if G1-G5 all hold ... No threshold, cap or bound in this file is changed after the run"; results say suite:c10:switching "is the registered report-only cell".
- Evidence: `git diff 636cdc7a..HEAD -- tools/compare.py` replaces `validated = a["error"] is not None` by `attainable = error is not None and any direct candidate passes the cap`. AFTER_636cdc7a.json: suite:c10:switching error 3.085e-2, cap 1e-4, accuracy_pass False, so the 636cdc7a comparator gives `G3_success_accuracy_all = False` (verdict FAIL). GATES.json carries the keys `accuracy_gated_cells`/`accuracy_report_only_cells` that only the HEAD comparator emits. L-0170 inputs_sha256 pins run_campaign.sh and analyze.py but not compare.py (sha a34fd639… at 636cdc7a, 9e73aee1… at HEAD). The results text does not mention the post-run comparator change (only the commit message does). The implemented exemption is data-dependent (any cell whose direct candidates all miss the cap), broader than the registered cell-named rule.
- Scenario: re-running compare.py at the ledger's EXEC_SHA on the committed JSONs prints G3=False and FAIL; the recorded PASS exists only under a comparator edited after the numbers were seen, outside the hashed inputs. AGENTS.md step 3 requires the scripts producing outputs to be at EXEC_SHA.
- Fix: pin the report-only cell by name, hash compare.py in inputs_sha256, state in Results that the EXEC_SHA comparator reported G3=False and why, or supersede L-0170 with a row whose EXEC_SHA contains the corrected comparator.

## F2 MAJOR — recheck "trend re-arm" cannot advance a recheck; test 5's bound is vacuous; a late stiff-to-smooth transition is hidden
- crates/vigilode/tests/switching_economy_contracts.rs:353; crates/vigilode/src/switching.rs:590-605, :882-886; PREREGISTRATION.md:68-70
- Claim: "An implicit rate falling to 1/rearm_rate_ratio ... re-arms it" (doc: "re-arms the recheck at once"); test comment "the return happens within two grown intervals".
- Evidence: every implicit batch is sized to exactly `recheck_interval(probe_discards) - since_probe` windows (:590-605); when it completes, a probe is due at the next window whether or not :886 fires, so the re-arm only resets probe_discards. Test limit = 1.5 + (2*4*2)*(4/16) = 5.5 > t1 = 4.0, so `returned.start_time <= limit` is always true; with ts = 1.5 the return at the window-9 probe (t = 2.0) is the legacy schedule.
- Scenario: same test with ts = 2.75 (window 12/16): probes at windows 5 and 9 discarded, interval -> 8, implicit batch covers windows 9-16 to t1, no probe after the transition; `returned` is None and the test panics, while legacy probes at window 13 and returns at t = 3.0. For this family RODAS work is forcing-limited (exact solution cos 2t for any lambda), so the rate-drop re-arm cannot fire.
- Fix: make the re-arm actually cut a coalesced batch (per-window trend check at each landing) or drop the "re-arm" claim and register the delay bound (up to 4x probe_interval windows); set the test limit to ts + probe_interval*probe_backoff_factor*window (< t1) and add a case with ts beyond the second discarded probe.

## F3 MINOR — committed analysis JSONs are not the pinned analyze.py output
- research/adapter_switcher_economy_20261026/tools/analyze.py:117
- analyze.py writes per-cell `"segments": rows`; BASELINE_4fe67602.json, evidence/AFTER_636cdc7a.json, evidence/BEFORE2_4fe67602.json contain no per-cell segments (only totals.segments). The stripping step is undocumented (not in the ledger command, no script). outputs_sha256 therefore cannot be reproduced from the pinned tool on the retained raw; `reconciled` and bucket sums cannot be re-derived from the committed data.
- Fix: add a documented `--drop-segments` mode (or strip script) and hash it; or keep the rows.

## F4 MINOR — compare.py drops cells from G2/G3 silently; G1 is narrower than "all counters"
- compare.py:38-39 (`continue` before ok2/ok3), :9-10 (A ∩ B), :20 (8 summary fields from analyze.py:46-50)
- A switching cell with no completed direct candidate, or any cell missing from one JSON, is skipped from both work and accuracy gates while G1-G3 still print True; no check that the 19 registered cells are present; G1 compares attempts/accepted/rejected/rhs/jvp/jac/solves/ft, not the full counters object.
- Fix: assert the registered cell set in both files; a missing candidate fails the gate; compare full `counters`.

## F5 MINOR — "discarded pilots" bucket and column include passed pilots
- analyze.py:59; PREREGISTRATION.md:149/154/157
- A passing pilot is pushed with committed=false (the explicit record is the committed one, switching.rs:807-812), so it lands in implicit_pilot_discarded. stiff-forced `0:Rpilot![8] 1:D[32] 2:R[12]` is the passing pilot (PREREGISTRATION.md:37 "1 pilot (passes)") yet the results report "discarded pilots 1(8)->1(8)"; c5 "2(23)->2(6)" includes the 1-attempt passing pilot. G2 gates passed-pilot attempts as "discarded" (stricter than registered).
- Fix: split by evidence (implicit-cheaper-on-prefix) and relabel.

## F6 MINOR — test 3 can pass without the re-arm; no campaign cell has a late transition
- tests:289-292
- `entry - ts < 6e-4` also holds when the grown cooldown (pilots at batches 1, 6, 15, 32 ...) happens to expire in the stiff phase; nothing asserts the entering pilot ran before `batches_since_entry_pilot` reached the grown cooldown. All nine cells have switches <= 1 at t ~ 0, so the campaign cannot detect a broken re-arm.
- Fix: assert committed explicit batches between the last smooth-phase discard and the passing pilot < cooldown*2^(discards-1); add a control with rearm_rate_ratio = f64::MAX expecting a later entry; add a late-transition cell to run_campaign.sh.

## F7 MINOR — test 4 does not show the return ceiling engaged
- tests:324-328
- `rhs_calls <= cap.floor() + 7` also holds if the probe is ended by probe_attempt_budget (16 attempts ~ 97 RHS) whenever cap >= 90; no `decision.contains("ceiling")` / `attempts < budget` assertion (contrast test 1's ceiling_stops > 0). A silently-None return_ceiling would pass.
- Fix: assert a ceiling-stopped probe with attempts < budget and rhs_calls > cap.floor().

## F8 MINOR — no verdict-identity control for the "decision-preserving" ceilings; "legacy reachable" overstated
- PREREGISTRATION.md:52; switching.rs:762, :1270-1271; fd.rs:133-146
- Verified by reading: Guard stops only on `charged > ceiling` (strict), counters charge before user code, work_units = rhs+jvp+ft+n*jac+solves >= charged units, DOPRI work_units = rhs_calls so the RHS-only probe bound equals the work bound. But no test or cell compares verdict sequences with/without ceilings; the campaign conflates ceiling and back-off effects (rotation 6->3 pilots). pilot_backoff_factor=1 does not restore legacy because `(pilot_eligible || rearm)` still fires early; coalescing/mean-step seeding have no legacy switch.
- Fix: cfg(test) no-ceiling hook comparing per-checkpoint verdicts; qualify "legacy reachable".

## F9 NOTE — predictions post-dated by the disclosed dev run; back-off tuned on case5
- PREREGISTRATION.md:129-143. Dev run gave 892/66/1209 = the after values; policy changed after seeing case5; predictions are sanity bounds, not tests. Honest, but G2 on case5 is not independent.

## F10 NOTE — return rule omits the completion-within-budget condition
- switching.rs:687 (`record.success && ...`), :617. Return is impossible when a window needs > probe_attempt_budget explicit attempts (rotation pressure-only: ~110/window vs 16); the ceiling never binds there (16->16); adapter stays on RODAS-MF (19749 JVP). State it in the preregistration.

## F11 NOTE — fp asymmetry between return ceiling and rate test
- switching.rs:625-627 vs :687-688. Ceiling floor(fl(fl(r*old)*T)) vs pass fl(W/T) <= fl(r*old): at W = C+1 with q*T within an ulp below C+1 the probe would pass but is stopped. Entry side shares one product (exact). Use one expression.

## F12 NOTE — accuracy degradation on implicit cells not discussed
- PREREGISTRATION.md:154-157: stiff-forced 1.57e-10 -> 7.33e-10, c5 9.86e-10 -> 1.25e-8 (12.7x), from mean-step seeding and fewer h0 restarts; within cap; one sentence needed.

## F13 NOTE — allocation bound 12/batch undocumented; baseline not shown to fail
- tests:442-447: allows 12*64+48 = 816; no enumeration of the per-batch allocations and no baseline count, so G5 may not discriminate.
