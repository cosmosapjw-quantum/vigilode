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

