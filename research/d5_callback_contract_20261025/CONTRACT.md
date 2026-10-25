# D5 strict RHS/Jacobian callback contract

## Result to construct

An opt-in Rust boundary admits finite time/state points in a fixed closed box,
executes a full-write RHS or Jacobian callback into owned NaN-initialized scratch,
and publishes only a checked result. A paired evaluation publishes RHS and
Jacobian together after both succeed. A centered time-difference request
preflights both represented stencil points before entering user callbacks.
Returning failures leave the caller's prior published scientific value intact;
attempted-work counters and captured external effects remain observable.

This fills a specifically observed research-level gap. At the pinned native
frontier, eval_rhs_into writes directly into caller storage and the in-place
dense_jacobian_into branch delegates directly to fill. Those APIs do not promise
transactional scratch publication. Their existing semantics are preserved.
In particular with_jacobian_into uses previous-matrix/fixed-sparsity storage;
D5 is a separate FULL-WRITE opt-in contract. It is not a drop-in replacement for
that sparse contract. Existing Session, Transaction, output prepare/commit,
attempt cap and continuous model-provider hooks are reused or referenced.

## Fixed authority and scope

- Parent D4 final source: 3c8877ea0409495752dcfab42426e679e3ba92d3, draft PR87.
- Current native source: aaf1f834d4c589e734ca7851a56c11bb44f7047a.
- Native live owner/reservation/dirty/completion chain: UNRESOLVED_INPUT_DEPENDENCY.
- Isolated new subtree: research/d5_callback_contract_20261025/.
- Production code, its forbid(unsafe_code), and the native sparse API are unchanged.
- This unit does not promote pointwise finite Jacobians to derivatives or a
  general continuous certificate. UnsupportedUnboundModel remains the general
  unbound-model refusal. Actual BASS, full general callback/Jacobian correctness,
  dense-domain, nonreturning callbacks, durable effect/crash restore, and the
  production FP provider retain independent gates.
- Production HOLD; global completion=null; independent personnel NOT_RUN.

## Operational assumptions and guarantees

For const dimension N>0, all box endpoints are finite and lower<=upper.
The query time/state must be finite and inside the box (including its boundary).
The witness reports expected model epoch and the required FP environment before
and after each callback. Epoch honesty, no ABA, and no unobserved concurrent
model mutation are assumptions, not conclusions from integer equality.
The safe fixture probe reads MXCSR through the previously isolated research
dependency; it does not prove compiler flags or an unsafe-free dependency tree.

RHS/Jacobian callbacks must fill all entries. A never-written NaN entry, NaN/Inf,
fatal callback result, handled panic, failed witness, or counter overflow cannot
publish a result. Fatal conditions poison the gate. Explicit recoverable callback
errors and rejected input/stencil requests do not grant output or run an internal
retry. Each dispatched callback is charged before entry, even on failure.
The paired result is returned as one owned value only after both callbacks and
all witness checks succeed. A previous caller value changes only by assigning
that successful value. This cannot roll back arbitrary captured external state.

The finite-time statement is conditional on all callback/probe/destructor calls
returning or unwinding in a catchable way. There is no process-abort, allocation
failure or nonreturning-callback termination guarantee. Actual Session::begin,
abort_stage, snapshot and its original cap are used in the new integration
contract tests; no Session is restarted to recover a budget.

## Manufactured science input

Let k(t)=k0+eta*t^2 and
 f=(-lambda*x+k(t)*y^2, lambda*x-mu*y-k(t)*y^2).
Use nondimensional t in [0,1], x,y in [0,1], lambda=3, mu=1,
k0=2, eta=1/2. This is an explicitly manufactured two-species transfer/loss
model, not actual BASS physics. Its derivative and conservation identity will
be derived symbolically and checked independently with exact rational arithmetic.
There is no new ODE trajectory or benchmark campaign in D5.

The centered time-difference is an estimate at represented stencil points:
(f(t_plus,y)-f(t_minus,y))/(t_plus-t_minus). Both points must be distinct from
t and each other, finite, and in the domain. It is not a derivative certificate.

## Completion criteria

1. Constructive proofs of dispatch exclusion, owned-output atomicity, incomplete
   full-write refusal, conditional Session/counter behavior and FD domain safety.
2. Actual new Rust contract tests, including normal and hostile callbacks,
   with pinned compiler and safe dependency boundary.
3. One frozen 9x9x9=729 polynomial point grid and a 7x7=49 centered-time stencil
   sweep, checked from saved output by an independent exact Fraction oracle.
4. Deliberate transpose/sign/finite-but-wrong-J interpretations are separated
   from interface admission; at least one nonvacuous detecting row is retained.
5. Physics/math then equation/code review, actual saved-data plots inspected,
   all original failures retained, phase publications/backups and copyable handoff.

No completed D2/D3/D4 tests, trajectories, benchmarks, NCP or V3 harmonic runs
are replayed. Desktop Commander calls remain zero. Closed budgets stay closed.

## Cross-project pattern reuse established by the current survey

C21H already validates an RPC RHS/Jacobian before publishing owned values; C21I
already has cell-owned scratch, model-specific domain/provider checks and success-only
cache insertion. Those patterns are reused as evidence. D5 adds their explicit
composition for direct in-process generic strict full-write RHS/Jacobian, pair
publication and bilateral time-stencil admission. It is not the first staging or
domain guard and does not replace/replay either completed optical adapter.
See RELATED_ADDITIONAL_KO.md and evidence/REUSED_CR_PATTERN_PINS.json.
