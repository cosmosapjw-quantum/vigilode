# D4 complementary source and theory review

Reviewer: native agent /root/contract_literature. Date: 2026-10-25 Asia/Seoul.
Scope: one complementary pass over the new wrapper, research dependency boundary,
22 newly written contract tests, and the discrete protocol proof.
This reviewer wrote the literature/theory text and did not write driver.rs or
tests.rs. It is an agent source review, not independent-personnel certification.
No tests, solver steps, builds, benchmarks, NCP or Desktop Commander calls were
executed by this reviewer.

## Result

PASS_SCOPED_SOURCE_AND_MATH_REVIEW.
No implementation change was required by this source pass. Four documentation
clarifications were applied to THEORY.md, preserving the frozen scientific scope.
Runtime results are NOT_RUN by this reviewer and must be read from the root's
actual execution receipts before the unit is called implementation-verified.

Read source identities:

| File | SHA256 |
|---|---|
| source/driver.rs | 8d25917bb48c14a1af993e19b9cb15b7939c1b9db53e667a5d5ce784a98e8d1b |
| source/fixture.rs | aaf090ebbe504125244d8afcb2f1c959620216dc419a1459c58e466884a86f1a |
| source/tests.rs | 19f34783ff237c2c271607bba634b02dd17f3963b5c9360c2d2531f57db1de40 |

Also read the actual reused admission, stage_safe, trajectory source and current
native transaction/problem/generic_verified modules at their pinned identities.
The root's source-byte manifest is the authority for copy identity.

## A. Mathematical/physical contract review

1. The finite-count proof uses successful begin count a and frozen cap M.
   One outer producer call follows each successful begin. The pending precheck and
   sticky terminal paths precede begin. Thus p <= a <= M. This is a returned-call
   protocol result, with internal RHS/Jacobian/solve counts separately tracked.
2. Rejection preserves Session Accepted fields because the wrapper exposes only a
   copied snapshot and a private journal, and invokes the original finish/abort.
   The original Accept decision is the sole numerical-state authority.
3. Journal poison is sticky and checked before Session::finish. Ignoring emit Err
   cannot turn a partially stored journal into an accepted batch.
4. Accepted attempt identifiers are unique within one run because lease.id is the
   cumulative begin count; the accepted epoch is copied after the original Accept.
   Cross-run uniqueness is the caller's u128 run-id obligation.
5. Delivery retry preserves identity and byte payload. The primitive does not run
   the producer or mutate the Session.
6. A sink's exactly-once effect additionally depends on atomic durable deduplication,
   payload binding, run namespace uniqueness, and honest acknowledgment. The
   BTreeMap test is a controlled in-memory demonstration of that mechanism.
7. The Arc-sharing example is a concrete boundary on Clone isolation. It is not
   adopted as a new strong no-go or physical impossibility result.
8. No change in physical equations, tolerance, norm certificate, domain predicate,
   dense certificate, or production claims follows from the protocol result.
   Real BASS and generic callback/Jacobian physics retain their separate gates.

## B. Code correspondence review

| Obligation | Observed implementation | Source verdict |
|---|---|---|
| Existing Session reuse | Driver owns Session and calls new_nonnegative/begin/finish/abort_stage | PASS |
| Pending before production | advance first inspects pending, before terminal and begin | PASS |
| Single producer per lease | one catch_unwind-wrapped producer call after begin | PASS |
| Refusal journal isolation | private Vec<Event>; failure returns without installing pending | PASS |
| Ignored emission error | emit sets first poison; driver checks it before finish | PASS |
| Finite retained storage | checked count and payload byte sums before push | PASS |
| Accepted commit order | after finish Accept, only integer copies and Vec move build Batch | PASS |
| Stable retry payload | deliver_once borrows pending; no clear on any delivery outcome | PASS |
| Exact ACK clearing | acknowledge compares current BatchId before taking pending | PASS |
| Stale duplicate protection | last_ack equality returns harmless AlreadyAcknowledged before touching newer pending | PASS |
| Producer panic | fatal lease abort, discarded journal, sticky CallbackPanic | PASS for handled unwind scope |
| Delivery panic | caught, same pending retained, no Session operation | PASS for handled unwind scope |
| Work honesty | attempted-event/producer diagnostics are retained and saturating, not acceptance authority | PASS |
| Safe boundary | wrapper and tests forbid unsafe; inherited research fixture separately links MXCSR read | PASS source; compiler receipt pending |

The data moved after Session Accept contains Vec<u8>, Vec<Event>, primitive
integers, and no user-defined payload destructors. Formatting, callback invocation,
or new allocation is not performed in the branch installing pending. This closes
the local accept-then-batch-loss path under the return/allocation assumptions.

The fixture is explicitly a research dependency. A successful wrapper compile
with forbid(unsafe_code) proves the new crate's lint boundary; it does not make
the dependency's inline assembly safe Rust or authorize a production FP provider.

## C. Documentation findings and corrections

| Finding | Observed risk | Minimal correction | Status |
|---|---|---|---|
| F1 separate delivery/ACK | A draft transition table implied successful delivery itself cleared pending | Stated deliver_once always retains pending; acknowledge is separate | CLOSED |
| F2 empty/final batch | A draft left empty-journal acknowledgment as an implementation choice | Bound to actual behavior: every Accept creates pending, including empty/final | CLOSED |
| F3 cap scope | Unqualified memory/callback bounds could overstate retained-byte and outer-call controls | Distinguished retained payload from allocation overhead, and producer from internal RHS | CLOSED |
| F4 external receipt/panic scope | Identity equality alone does not authenticate delivery; arbitrary panic destructors may panic again | Explicit caller assertion, sink assumptions, returning-destructor boundary | CLOSED |

No tolerance or scope was relaxed by these corrections.

## D. Test-source adequacy and limits

The 22 new tests cover:
- accepted snapshot/event binding;
- recoverable, fatal, norm, domain and nonfinite-candidate refusal;
- attempt cap and represented-time no-progress;
- final-time and empty-batch barrier;
- wrong, repeated and stale acknowledgment;
- event/payload cap poison, ignored emit failure, and exact-cap order preservation;
- delivery error before write, lost ACK after write with controlled deduplication;
- delivery panic and producer panic with unchanged managed state;
- no-pending delivery;
- an Arc<AtomicUsize> external-capture counterexample;
- two newly instrumented connections to the unchanged stage_safe hook.

The last two tests call one failed stage trial (2 RHS entries, 1 solve) and one
successful stage trial (8 RHS entries, 8 solves). They are new effect-routing
checks, not reruns of D2/D3 full campaigns. Their work must be reported separately
if executed. Synthetic candidates in the other tests exercise protocol admission
paths and are not presented as ODE accuracy solutions.

The following remain outside this test source and are not inferred:
general callback mathematical correctness; generic Jacobian consistency; crash
recovery; distributed durable exactly-once delivery; callbacks that never return;
panic=abort; and independence of arbitrary captured I/O.

## E. Single-pass termination

All source-level findings above are closed. Reopen this review only for an actual
source change, a contradictory execution result, or a changed contract dependency.
The root may append execution-log readback for these exact identities without
running another numerical campaign.

## F. Actual execution receipt readback — same review pass

Readback performed on 2026-10-25 Asia/Seoul. This section connects the root's
already completed executions to the source reviewed above. The reviewer ran no
compiler, test binary, trace generator, oracle, ODE stage, or Wolfram calculation.

### Source-to-execution binding

The execution source was published as S1 commit
555a93e5bffbed4b488ee0496e73d8095b08eab4, tree
78a0f6df66fb85dcd60ef7d1d20b50a6e73c7772, on Draft PR87.
delivery/S1_PUBLICATION.json truthfully records that tests were NOT_RUN when that
source was published. Later BUDGET.json entries record the actual build/run commands,
exit statuses, elapsed times, and log hashes. These are different observation times.

Direct current-file hashes of driver.rs, fixture.rs, and tests.rs match all three
identities in the first table. The independent trace components match
SOURCE_IDENTITY.json and REVIEW.json:

- source/trace_corpus.rs:
  02f2b4acee54b58df43b23286efdb43b3419ff74a641b1322b7dc5653f42a05d
- review_traces.py:
  62638c3def276b9395d1579de30c91bcc2c818e0b701cbdb2324c1a2175332b9

REUSED_SOURCE_PINS.json records the six original D3/D2 files with exact Git blob
matches at parent 02060cc9dc4551dc5cf6ec86ad9b53b697a2c3ce. This review does not
replace those existing byte-identity receipts with a scientific rerun.

### Compilation and new contract results

| Execution | Actual evidence read | Result |
|---|---|---|
| research fixture compile | BUDGET command/exit 0; fixture_compile.log is empty successful output | PASS |
| wrapper compile with -F unsafe-code | BUDGET command/exit 0; safe_wrapper_compile.log is empty successful output | PASS |
| new contract compile | BUDGET command/exit 0; contracts_compile.log is empty successful output | PASS |
| new contract execution | new_contracts.log, all 22 named tests, exit 0 | 22 passed / 0 failed / 0 ignored / 0 filtered |
| inherited fixture under -F unsafe-code | BUDGET exit 1 and inherited_unsafe_boundary_expected_failure.log | EXPECTED_FAILURE at reused/admission.rs:15 stmxcsr unsafe block |
| new trace compile/run | BUDGET exits 0; frozen_new_trace_corpus.log | 16384 cases / 65536 rows |
| independent saved-trace oracle | BUDGET exit 0; independent_saved_trace_oracle.log; REVIEW.json | PASS_ALL_FROZEN_PROTOCOL_TRACES |

The unsafe-boundary failure is the requested negative compile check: the log
identifies exactly the inherited MXCSR assembly block and -F unsafe-code.
The safe wrapper passed without changing the production unsafe policy. This is
evidence of the dependency boundary, not proof that the inherited fixture contains
no unsafe code.

new_contracts.log contains the two intentional panic-hook messages followed by
successful tests. The test harness reports no failed test. These messages preserve
the actual producer/sink unwind observations rather than hiding them as noise.

### The two newly instrumented stage calls

The two stage test names appear as passed in new_contracts.log, and their actual
assertions were read from the same-hash tests.rs:

- d4_existing_d2_stage_callback_failure_drops_two_owned_rhs_events:
  Work { rhs: 2, solves: 1, checked: 2 }, no pending batch, accepted bits retained.
- d4_existing_d2_stage_success_publishes_eight_rhs_records_after_admission:
  Work { rhs: 8, solves: 8, checked: 8 }, ordered channel keys 0 through 7,
  64 retained payload bytes, and final-time pending barrier before ACK.

Thus the new effect-routing execution contains 2 stage trial calls, 10 internal
RHS calls, 9 solves, and 10 stage checks. Previously completed D2/D3 campaigns,
old regression suites, benchmarks, NCP, and V3 harmonic runs were not replayed.
The trace corpus itself reports ODE_stage_calls=0.

### Independent saved-trace results and exact scope

The corpus covers all length-four words over ARFBLSKW for caps 0,1,2,3:
4096 cases per cap, 16384 cases, 65536 recorded transitions. REVIEW.json gives:

- 3407872 field comparisons;
- 61552 scientific-prefix-preservation rows;
- 3984 accepted clock-progress rows;
- 65536 attempt-ledger identity rows;
- 2580 pending-admission-barrier rows;
- 2580 delivery-retained-identity rows;
- failure_count=0.

The three semantic counterfactuals are all detected against saved real traces:

| Counterfactual model | Differing rows | Cases detected | First sequence |
|---|---:|---:|---|
| clear pending on delivery error | 2692 | 1412 | AAAB |
| permit advance while pending | 3751 | 1914 | AAAA |
| non-idempotent receiver | 380 | 284 | AALL |

These are oracle-side semantic mutations, not a claim that three separately
patched solver binaries were compiled or benchmarked. Longer sequence claims
rely on the invariant proof; the exhaustive execution scope is exactly length 4.

The aggregate corpus explicitly records 706 exact-ID ACK events without a
corresponding observed sink effect and 780 without a prior delivery success.
They are permitted caller assertions in acknowledge(), not authenticated receipts.
Their presence is evidence that the bare API cannot be used to infer remote
exactly-once completion. The conditional sink theorem requires its stated
atomic/durable deduplication, payload binding, and honest ACK assumptions.

The trace input identity reported by the independent oracle is
3f9948f68c3da234859519788b34806145ee1f95dc69aa3302e74ba17fc9273b.
This reviewer read that receipt and the associated logs; it did not run a second
oracle or reread the 68 MB trace corpus as an additional audit layer.

### Wolfram record

Both evidence/WOLFRAM_INITIAL.json and WOLFRAM_CORRECTED.json were read.

The initial bounded model returned 23 reachable states, 51 edges including
stutter, True for the core invariants, and 0 pending-advance edges. Its dedup
conjunction remained an And expression of lists, so that field was not a Boolean
pass. The initial query and failure classification remain preserved.

The corrected query flattens that Table before applying And. Its recorded result
has 23 states / 51 edges, allInvariants=True, pendingAdvanceEdges=0,
atomicSetDedupIdempotent=True, and the clear-on-error witness violates the invariant.
Reduce returned attemptIncrementViolation=False. The
LinearProgramming::lpsnf and General::messages warnings remain in the raw record;
the review does not rewrite it as a warning-free invocation.

The CAS record checks this bounded abstraction and the stated arithmetic
counterexample condition. It complements the source induction and independent
saved-trace oracle; it is not a proof of arbitrary callbacks or durable I/O.

### Receipt disposition

IMPLEMENTATION_VERIFIED_SCOPED for the exact new source and frozen test/trace
contract above. This extends the earlier source-only verdict with actual root
execution receipts; independent_personnel remains NOT_RUN.

At this readback the new D4 local BUDGET was still open with 10 entries totaling
23.327209242008394 seconds. Final plotting/closeout may append new authorized
entries before closing it. No historical D2/D3/native-boundary ledger was reset.

No new unresolved source defect was identified by this receipt linkage, and no
additional test was requested. Plot review is the root's separate pending
deliverable. Production remains HOLD; global completion remains null.


