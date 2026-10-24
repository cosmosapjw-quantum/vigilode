# D4 — owned effect journal and acknowledged delivery

Date: 2026-10-25 Asia/Seoul. Selected workflow: user-provided physmath-research-loop1.0.0, one full R1–R10 and C1–C10 loop following completed D3.

## Question and baseline
Connect the existing Session to a bounded, deeply owned callback-effect journal. Rejected trials discard their journal. An accepted trial owns one immutable pending batch; further solver attempts wait for its exact acknowledgement. A delivery failure retains the accepted scientific state and the same batch identity/payload.

D3 parent PR86 HEAD02060cc9dc4551dc5cf6ec86ad9b53b697a2c3ce, science96aee1829d72ce1cbd6bcd0c0e48ebf3b1b37455. D2 PR85 HEAD1c829f206ab67585191231be7f5135fd0d4c804e. Latest native public frontier aaf1f834d4c589e734ca7851a56c11bb44f7047a. Current native owner/reservation/live dirty/completion remains UNRESOLVED_INPUT_DEPENDENCY.

Existing Session cap/clock/sticky fault and native Transaction/generic_verified/output prepare/commit are already present. Reuse those mechanisms. New subtree research/d4_effect_journal_20261025, branch research/d4-effect-journal-20261025, based on exact D3 HEAD. Native and production files stay outside this change.

## Fixed construction
- Event payloads are deeply owned Vec<u8> with typed numeric channel. No arbitrary user payload destructors/shared references. Checked event/byte limits and sticky poison prevent ignored emit failures from permitting acceptance.
- A private Session issues the attempt lease. One producer invocation prepares candidate/error and trial events. The unchanged Session grants scientific acceptance. Rejection discards events; recoverable abort uses existing halving; fatal error/unwind aborts and becomes sticky.
- Acceptance creates one pending batch even when empty, identified by caller-bound run_id plus attempt/accepted epoch. Pending delivery takes priority even at final time; advance does not call begin or the producer.
- deliver_once invokes a sink at most once and preserves the pending batch on success/error/unwind. A separate acknowledge(exact_id) clears it. Stale acknowledgements cannot clear a different pending batch; duplicate last ACK is idempotent.
- Producer invocations are bounded by the existing cap A conditional on finite callback/internal-solve/destructor return. Internal RHS work is counted separately. Delivery retry has no hidden loop.
- Exactly-once remote effects additionally require an atomic durable sink that binds unique batch IDs to the same payload/effect and deduplicates replay. Here the guarantee is a live in-memory journal/barrier. Arbitrary captures/I/O, crash restore, external durable receipts and nonreturning callbacks are separate obligations.
- Retained payload byte caps exclude vector metadata/allocator overhead and arbitrary callback work.

## Frozen verification before compilation and tests
New boundary contracts only: accept/recoverable/fatal, norm/domain rejection, nonfinite candidate, budget/clock, ignored emit poison, event/byte limits, pending at final time, stale/duplicate ACK, empty batch, before/after-write delivery failure with controlled dedup sink, producer/sink unwind, and an external shared-counter discriminator. The initial drafted suite has22 tests; any correction is recorded with its actual failure/source delta.
Two newly instrumented stage_safe trials connect journal routing to the real existing eight-stage producer: one failure at RHS2 and one accepted attempt at RHS8. These are new adapter tests, not old D2/D3 test/campaign replays.
A separate new finite trace corpus and independent Python state oracle check transitions, prefix preservation, cap accounting and matching mutations. Plots use only saved new protocol traces.
New wrapper/test crates have crate-level forbid(unsafe_code). Reused admission with its known read-only MXCSR assembly is compiled as a separate research fixture dependency. The wrapper must compile safely; a direct -F unsafe-code fixture build is expected to fail at that inherited assembly. Production's forbid remains untouched.
No accuracy, tolerance improvement, wall-time speedup, actual BASS, arbitrary callback/Jacobian or general dense-domain result is inferred from protocol fixtures.

## Preservation and resources
No completed D2/D3 experiments/tests, old benchmarks, NCP or V3 harmonic replay. Desktop Commander calls0; its monthly quota rejection is not retried or bypassed. Existing archives, failure logs, source heads and closed budgets remain unchanged.
New D4 local subprocess cap120s, accounting toolchain restoration, compile, new tests/oracle/figures. This is a new unit, not a reset. Historical D3closed35.897308696992695s; D2closed25.104395919000012s; prior83.29566578299983s; native compile boundary14.951735806986107s; original historical S0 text108.40006170199984; older_remaining=null; reset=false.
Retrieval/drafting/storage/external CAS and automatic remote metadata CI are separate. An initial exploratory Wolfram call preceded S0 and produced a non-Boolean list conjunction for one dedup subcheck; preserve that result and correct its flattening rather than claim a pass.

## Stage checkpoints and runtime incident
S0 source survey/contract → S1 theory/implementation → S2 new execution/saved evidence → S3 independent review/report/handoff. Every R/C stage maps to its actual phase archive+commit; four consolidated phase backups follow the established project format, not20 duplicated uploads. Global completion=null; production=HOLD.
The scratch workspace disappeared after initial source/draft writes and before any new scientific subprocess. The branch still pointed at exact D3 base. Recovered source must match its already recorded hashes; no completed science is recreated. This published contract is the pre-execution authority; the earlier uncommitted local draft is not claimed as a remote checkpoint.
