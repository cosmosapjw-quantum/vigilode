# D4: Owned effect journal and pending-delivery barrier

Status: DERIVED_SCOPED; actual driver correspondence checked in the single complementary review.
Date: 2026-10-25 (Asia/Seoul).
Workflow: user-selected physmath-research-loop 1.0.0, theory R3–R6.
The D2/D3 scientific evidence and closed budgets are reused. No D2/D3 numerical rerun
is part of this derivation. Production remains HOLD and global completion remains null.

## 1. Target and change from the reused contract

D2/D3 already have an attempt-limited Session, lease identity, fatal halt, no-progress
clock refusal, and an accepted-state update guarded by Session::finish. The native
frontier already has Transaction<S: Clone>, a non-refunding work budget, output
preparation, and numerical state rollback. D4 adds a controlled effect boundary:

1. A new trial owns a finite journal of effect data.
2. Rejection, callback failure, poisoning, or callback panic publishes no entry from
   that journal.
3. Acceptance creates one immutable pending batch bound to the accepted attempt.
4. A pending batch blocks further begin/producer calls.
5. Delivery failure or a mismatched acknowledgment preserves the pending batch.
6. Only the matching acknowledgment clears the barrier.

The state and the delivery status are distinct. A batch can be scientifically
accepted and still await an external acknowledgment. Delivering it again never
re-runs the ODE trial.

This is a protocol construction for managed effects. It does not change the
admission norm, domain predicate, RODAS tableau, dense certificate, physical model,
or tolerance. No new norm-only scientific node is introduced.

## 2. Definitions and assumptions

Let:

- \(S=(t,y,e,\nu)\) be the existing Session accepted snapshot: time, state vector,
  epoch, and last norm metadata.
- \(a\) be the cumulative count of successful Session.begin calls.
- \(M\in\mathbb N\) be the frozen total attempt cap.
- \(p\) be the count of outer producer invocations by the D4 driver.
- \(W\) be the attempted-work ledger. It is cumulative across rejection and delivery.
- \(J=(j_0,\ldots,j_{n-1})\) be the current trial's owned event sequence.
- \(P\) be either no pending batch or \(B=(b,J)\).
- \(b=(r,a,e)\) be BatchId { run_id: u128, attempt: u64, epoch: u64 }, consisting of the fixed run identity \(r\),
  successful-begin attempt index \(a\), and accepted epoch \(e\).
- \(F\) be the driver's sticky terminal fault, if any.
- \(N_{\max}\) and \(L_{\max}\) be immutable event-count and byte-count limits.

An **outer producer invocation** is the one callback the D4 advance operation uses
to prepare the candidate and error estimate for its active lease. A producer may
internally evaluate RHS, Jacobian, JVP, or linear-solver callbacks several times.
The theorem \(p\le M\) bounds outer producer calls; it does not identify \(p\) with
the number of internal RHS evaluations. Those remain in \(W\).

An Event is { kind: u32, payload: Vec<u8> }. The byte cap counts exactly payload.len(),
excluding Vec allocation overhead and the kind tag. The event-count cap also
limits empty-payload records. All retained-count additions are checked before
storage. A caller can allocate a payload before emit is called; the cap bounds
retained journal data, not all memory allocation or arbitrary callback work.

### Assumptions that carry the theorem

A1. Each producer, delivery callback, finite validator, and required allocation
returns normally or unwinds in the explicitly handled way. Process abort, fatal
allocation failure, nonreturning callbacks, and process crashes are outside the
live-object return theorem.

A2. The Session counter and run identity persist in this driver instance; retry,
yield, and delivery do not create a new Session or reset \(a\).

A3. The managed journal consists of independently owned immutable-to-caller bytes
after preparation. Producing a journal does not publish it to the external sink.

A4. No accepted-state field, output cursor, or managed published journal is exposed
for mutation by the producer. A snapshot supplied to the producer does not share
mutable scientific storage with the Session.

A5. Every accepted batch identity is prepared without wrapping integer arithmetic,
and acceptance is bound to the exact active lease. No caller can forge a Session
acceptance through the effect adapter.

A6. Pending-batch queries and delivery callbacks cannot mutate the stored batch.
The adapter clears \(P\) only after identity equality with the acknowledgment.

A7. The run identity is fixed for this live object. For a sink shared across runs,
the caller provides a run identity unique within the sink's deduplication lifetime,
or the sink rejects an identity reused for a different payload.

A8. Existing Session admission/environment assumptions are retained. The effect
adapter introduces no unsafe instruction. The inherited read-only MXCSR operation
stays in its research admission boundary; the production forbid(unsafe_code)
policy is unchanged.

The results below are exact discrete-state statements. They do not require a new
ODE truncation estimate, domain theorem, or wall-time speed claim.

## 3. State transitions

The intended sequential transition table is:

| Entry state and event | Session begin / producer | Scientific state \(S\) | Journal / pending batch | Exit |
|---|---:|---|---|---|
| Sticky terminal \(F\) | 0 / 0 | unchanged | unchanged | same terminal |
| Pending \(P=B\) and advance | 0 / 0 | unchanged | same \(B\) | pending-delivery refusal |
| begin refuses budget/clock/environment | 0 successful begin / 0 | unchanged | none | existing typed failure |
| Active lease, recoverable producer error | 1 / 1 | unchanged | discard \(J\) | existing abort/retry policy |
| Active lease, fatal producer error | 1 / 1 | unchanged | discard \(J\) | sticky terminal |
| Producer unwinds | 1 / 1 | unchanged | discard \(J\) | catch unwind, resolve lease, sticky terminal |
| Journal capacity error, even if ignored by producer | 1 / 1 | unchanged | discard poisoned \(J\) | refusal before finish can accept |
| Existing Session.finish rejects or is indeterminate | 1 / 1 | unchanged | discard \(J\) | existing refusal/terminal |
| Existing Session.finish accepts | 1 / 1 | exactly Session's new snapshot | one immutable \(B=(r,a,e,J)\) | pending delivery |
| deliver_once returns success, error, or caught panic | 0 / 0 | unchanged | same identity and bytes | pending delivery |
| acknowledge receives wrong identity | 0 / 0 | unchanged | same identity and bytes | acknowledgment mismatch |
| acknowledge receives current pending identity | 0 / 0 | unchanged | clear pending | ready for next advance |
| acknowledge repeats last acknowledged identity | 0 / 0 | unchanged | any newer pending batch retained | AlreadyAcknowledged |

"Publish" below means making journal events available through the accepted-batch
delivery path. It never includes diagnostics that the callback independently writes
to a captured file, network client, global object, or shared counter.

Every accepted trial creates a pending batch, including an empty journal. Empty
batches also require an explicit acknowledgment. The pending guard is before
terminal/Finished handling, so a final-time accepted batch still awaits ACK.

## 4. Lemma L1: finite number of outer producer calls

**Statement.** Under A1–A2, D4 makes at most \(M\) outer producer invocations during
the lifetime of one Session. On entry while \(P\ne\varnothing\), it makes none.

**Proof.** Existing Session.begin checks \(a\ge M\) before incrementing. A successful
begin changes \(a\) to \(a+1\), so \(0\le a\le M\) is inductive. D4 invokes the producer
only after one successful begin, and at most once for that lease. Therefore
\(p\le a\le M\). The pending check precedes begin, so a pending call changes neither
\(a\) nor \(p\). Rejection and callback failure resolve the lease without reducing
\(a\). Delivery likewise does not reduce or reset it. ∎

Each advance operation consists of finitely many wrapper operations plus its one
returning producer, so it returns in finite time under A1. A program may voluntarily
call advance forever on an unchanged pending batch or terminal fault; each call
returns without more numerical work. The theorem is bounded production, not
termination of an arbitrary external client loop.

This reuses the existing attempt cap. Halving alone is not used as a termination
proof. A floating-point no-progress step is separately refused by the existing
clock/minimum-step hook.

## 5. Lemma L2: rejection preserves the managed scientific prefix

**Statement.** Under A3–A5, if an attempt is not accepted by Session::finish, then
\(S'=S\), no accepted pending batch is created from its journal, and the attempted
work remains charged.

**Proof.** The producer writes the candidate, error estimate, and trial-owned
journal. It has no mutable access to \(S\). Recoverable/fatal failure, panic, and
poison are resolved through the existing active-lease abort path before an
accepting finish. A completed candidate reaches the unchanged Session::finish.
That function changes the accepted snapshot only in its Accept branch. If the
branch rejects, is indeterminate, or returns a typed fault, D4 discards the
journal and does not construct accepted pending output. Work was charged for the
attempt and is not part of the rollback target. ∎

The precise bitwise prefix can include Session time/state/epoch/last norm and any
driver-owned cursor/history whose commit is bound to the same transition. It
does not automatically include callback captures. The implementation must list
the managed fields instead of saying that all program state rolls back.

## 6. Lemma L3: poison closes the ignored-error path

**Statement.** Let journal emission use a sticky flag \(q\), initially false.
If an attempted emission would exceed \(N_{\max}\), \(L_{\max}\), or the integer
range used for byte accounting, emission returns an error and sets \(q=true\).
If acceptance requires \(q=false\), a producer cannot obtain an accepted batch
by ignoring an emission error.

**Proof.** Every failed capacity check sets \(q=true\), and no producer operation
can clear it. Thus at producer return, any failed emission implies \(q=true\).
The driver checks \(q\) before the accepting finish path and refuses the trial.
Even if the callback returned success after suppressing the emission error, its
journal cannot be published. ∎

The limit checks are performed before appending. An oversized event is not
partially appended. Empty events still count toward \(N_{\max}\), so zero-byte
events cannot create unbounded storage entries. Poison is an effect-admission
failure; it never relaxes the scientific tolerance.

## 7. Lemma L4: acceptance and batch identity are bound to the existing lease

**Statement.** Under A4–A5, every newly created pending batch corresponds to exactly
one existing Session Accept, and its identity \(b=(r,a,e)\) remains constant until
that batch is acknowledged.

**Proof.** D4 creates accepted pending output only in the existing Accept branch
for the currently active lease. The attempt component is the successful begin's
counter; the epoch is the committed Session epoch. A rejected attempt creates no
pending output. A successful begin strictly increases \(a\), so two different
accepted attempts within the same live run have different \(a\), even when
rejections occur between them. With fixed \(r\), their identities differ. Once
\(P=B\), no advance is admitted until acknowledgment, and delivery only reads
the immutable stored batch. ∎

Both accepted state and batch construction must be prepared before the final
observable commit sequence. Fallible formatting, allocation, payload conversion,
and identity validation must not occur after scientific acceptance if they could
lose the accepted batch. Moving an already prepared owned capsule is the intended
commit operation. The code review verifies this order.

A caller-supplied u128 run identifier is a namespace, not a global uniqueness proof. The
cross-run sink condition is A7.

## 8. Lemma L5: delivery is a barrier with retry-stable payload

**Statement.** Under A6, if \(P=B\), then:
\[
  \operatorname{advance}:\quad
  (\Delta a,\Delta p,\Delta S,\Delta B)=(0,0,0,0).
\]
A delivery success, error, caught panic, or nonmatching acknowledgment leaves
\(P=B\) exactly. Only acknowledge with the current pending identity clears \(P\)
without changing \(S\). Repeating the last acknowledged identity is idempotent
and does not clear a different newer pending batch.

**Proof.** The pending guard is before begin and the producer. It returns without
their side effects. deliver_once borrows the pending identity and payload and
never takes or clears it, for any returned/caught outcome. acknowledge first
checks the previous ACK for a harmless duplicate, then compares the current
pending identity; mismatch returns an error and equality clears \(P\). No branch
calls begin or Session.finish, so delivery/acknowledgment changes no scientific
snapshot. ∎

The acknowledge method treats the caller's argument as an assertion of sink
completion. It does not cryptographically authenticate a receipt and can be
called without deliver_once. Correct external completion therefore requires an
honest caller/sink acknowledgment contract in addition to identity matching.
This blocks duplicate scientific production while delivery is unresolved.
The sink may still have observed a delivery before returning an error. The next
section distinguishes that external event from the preserved in-memory batch.

## 9. Conditional external exactly-once result

The in-memory wrapper offers stable identity and payload for retry. Exactly-once
external effect additionally requires a sink contract.

Let a sink have durable state \(D\) mapping batch identities to payload identities.
For a batch \((b,J)\), require:

S1. Applying its effect and recording \((b,\operatorname{identity}(J))\) is atomic
with respect to durable recovery.
S2. A retry with a recorded \(b\) and matching payload returns the same acknowledgment
without applying the effect again.
S3. A recorded \(b\) with a different payload is rejected.
S4. A returned successful acknowledgment means the mapping and effect are durable.
S5. The run namespace satisfies A7.

Then retrying a fixed pending batch produces at most one external application.
If a delivery eventually returns its matching acknowledgment, it has applied
exactly once.

**Proof.** At the first atomic successful application, S1 writes the identity.
Every later attempt sees that identity and invokes S2, so no second application
occurs. S3 blocks accidental reuse with changed payload. S4 makes an accepted
acknowledgment meaningful after recovery. Eventual acknowledgment establishes
at least one application, combining with at-most-one to yield exactly one. ∎

**Lost-ACK example.** The sink applies batch \(b\), durably records it, but the
response is lost. D4 keeps \(P=B\) after receiving an error. It retries the
same bytes and identity. The sink sees the existing record, performs no new
effect, and returns the same ACK. The caller passes that exact identity to
acknowledge, which clears the barrier. No ODE trial is repeated.

Without S1–S4, the stable in-memory protocol states exactly what it knows:
scientific Accept occurred, the same batch remains pending, and remote application
may be unresolved. The implementation must not infer an external rollback from
a delivery error.

D4 persists no journal to disk unless explicitly implemented. Process crash,
restart, durable outbox recovery, and reconstruction of an identical run identity
remain a separate gate.

## 10. Constructed Clone-sharing boundary example

Consider a callback context containing
\(\operatorname{Arc}(\operatorname{Mutex}(c))\). Cloning it produces two references
to the same counter. A trial increments \(c\), then rejects. Discarding the clone
removes one reference; the original counter remains incremented. This follows from
the shared-storage semantics of Arc, independently of any ODE calculation.

Consequently:
\[
  S:\operatorname{Clone}
  \quad\not\Longrightarrow\quad
  \text{trial writes are isolated from committed reachable storage}.
\]
This is a scoped data-ownership counterexample to a possible API claim. It is not
a new physical impossibility theorem or novelty claim. The useful correction is
to state the managed owned-data boundary and reuse the existing transaction for
that boundary.

A producer may similarly mutate a file, network service, global state, or captured
counter before returning or panicking. D4 discards its owned journal but cannot
infer reversal of those effects. Diagnostics of attempted work may intentionally
remain cumulative.

## 11. Panic and fault treatment

For unwinding Rust panics, D4 places the producer inside catch_unwind, resolves the
live lease through the fatal abort path, discards its journal, and stores a sticky
terminal fault. No subsequent numerical producer runs on that driver instance.
Using AssertUnwindSafe to enter the boundary is not a theorem that arbitrary
captures are unchanged; it supports this terminal handling policy.

Process abort, panic=abort, forced termination, allocation abort, and a callback
that never returns are not catch_unwind outcomes. They are outside A1 and require
a different execution/recovery design. No such case is exercised by an infinite
loop test in this unit.

deliver_once also catches an unwinding sink panic, returns Delivery::Panicked(id),
and retains the identical pending batch. It does not terminally poison the
scientific Session, which remains blocked by that batch. Re-entering the sink
requires its own retry/idempotency contract because captured sink state may have
changed before the panic.

Handled-panic return claims assume panic-payload and callback destructors return;
a secondary panic from such a destructor is outside A1. This is distinct from
the tested ordinary string-payload unwind path.

## 12. Equation-to-source and proof obligations

| Result | Reused source / new obligation | Verification target |
|---|---|---|
| L1 finite producer count | admission::Session::begin/begin_to; wrapper ordering | cap hit, no reset, pending precheck |
| L2 refusal preservation | Session::abort_stage/finish | snapshot bits and journal sink invocation count |
| L3 poison | new owned Journal::emit | count/bytes/checked overflow and ignored error |
| L4 accepted identity | Session lease/epoch; prepared Batch | no batch before Accept; identity/payload stable |
| L5 barrier | new pending/deliver state | Err and wrong ACK preserve bytes; correct ACK clears |
| External exactly once | independent sink S1–S5 | apply-then-lost-ACK dedup model |
| Captured effect boundary | native Arc callback types; Rust Clone semantics | shared-capture counter example |
| Safe production boundary | unchanged production forbid(unsafe_code) | new wrapper contains no unsafe; inherited MXCSR isolated |

All fields and operation names are bound to actual implementation during the
single complementary math/code review. Pure protocol/model checks do not count as
an ODE trajectory rerun.

## 13. Source map and literature position

Primary sources read on 2026-10-25:

1. SUNDIALS CVODE User Guide, Usage §§3.4.3–3.4.4:
   https://sundials.readthedocs.io/en/latest/cvode/Usage/index.html
   Recoverable/unrecoverable RHS and Jacobian failures, repeated-failure controls,
   minimum step, and the scope of maxsteps.
2. Boost.Odeint Controlled Stepper:
   https://www.boost.org/doc/libs/latest/libs/numeric/odeint/doc/html/boost_numeric_odeint/concepts/controlled_stepper.html
   Failure preserves the numerical state and time while suggesting a smaller step.
3. SciML Common Solver Options:
   https://docs.sciml.ai/DiffEqDocs/stable/basics/common_solver_opts/
   dtmin/maxiters and force_dtmin semantics.
4. SciML Callback Functions:
   https://docs.sciml.ai/DiffEqDocs/stable/features/callback_functions/
   Event timing and the separate condition/affect callback interface.
5. Rust Clone:
   https://doc.rust-lang.org/std/clone/trait.Clone.html
   Shared storage under Arc/Rc clones.

These sources establish prior solver and language contracts. D4's contribution is
the narrow source-bound composition and its tests, not invention of transactional
messaging or finite attempt caps. SciSpace scholarly results are pointers recorded
in R2_R3_CALLBACK_CONTRACT_KO.md; their abstracts are not substituted for the
primary-source semantics above.

NG1–NG7 strong no-go admission is not requested or claimed here. All negative
statements are scoped ownership/counter examples or explicit unimplemented gates.

## 14. R3–R6 disposition

- R3: definitions, quantified managed-effect scope, assumptions, and omitted
  external obligations specified.
- R4: pure callback assumption, deep-clone transaction, and owned journal/barrier
  alternatives compared through actual existing source. The smallest missing
  boundary is the owned journal/barrier.
- R5: L1–L5 and the conditional sink theorem derived constructively above.
- R6: source/driver.rs was read after implementation and mapped to L1–L5.
  No source defect requiring a code patch was found in this pass. Documentation
  was narrowed to the actual separate deliver_once/acknowledge API, empty-batch
  ACK policy, retained-payload cap, and callback/destructor return assumptions.
  Execution evidence remains the root's C/R7 responsibility.
- Real BASS, generic callback/Jacobian correctness, general dense-domain validity,
  production FP-provider ownership, live native dirty/reservation receipt, and
  durable process-crash recovery remain independently tracked obligations.

