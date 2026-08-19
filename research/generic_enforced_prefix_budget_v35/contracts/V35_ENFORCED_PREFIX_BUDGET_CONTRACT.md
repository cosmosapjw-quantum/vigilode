# v3.5 Enforced Speculative Prefix Budget Contract

Parent authority: `4a78a6893a630bea0e565c698e064c66367a2681`.

## Frozen constants — no retuning

- absolute speculative-prefix JVP cap: `B_abs = 80`
- cumulative speculative fraction: `delta = 0.25`
- frozen zeta34 recommendation threshold: `tau = 13.39706618860016`
- upstream event gate: frozen k=3 R-JF-only event policy
- active switching: forbidden
- N=2048: sealed

## Per-event budget

Immediately before a speculative E-K prefix at event k, define

`B_k = min(80, floor(0.25 * R_k - S_k))`,

where `R_k` is cumulative committed R-JF JVP vectors and `S_k` is cumulative
speculative prefix JVP vectors already charged. Since both ledgers are integers
and delta=1/4 exactly, implementation authority is

`B_k = min(80, (R_k / 4).saturating_sub(S_k))`.

If `B_k == 0`, do not start a prefix and abstain to R-JF.

## Transactional JVP semantics

Every matrix-free JVP used by pexprb54s4 level1+2 prefix work is guarded by the
same per-event budget.  The guard MUST refuse the prospective `(B_k + 1)`th JVP
before invoking the underlying operator.

If the prefix exhausts its budget before level2 completion:

- return an explicit `budget-exhausted` outcome, not a numerical failure;
- charge every completed speculative JVP and all other already completed prefix
  work to the diagnostic ledger;
- emit no level2 zeta34 or other completed-prefix safety witness;
- execute no full-E endpoint work for that event;
- leave R-JF state, controller, requested output, and existing R-JF work counters
  unchanged;
- abstain to R-JF.

If the prefix completes within the cap, existing zeta34 and audit-label behavior
is unchanged and `tau` remains frozen.

## Replay / evidence policy

Consumed profiles N=96, 192, 256, 384 may be replayed only for semantic
regression.  They are not new holdouts.  A new unseen budget/safety holdout must
be contract-frozen before its first output if replay gates survive. N=2048 is
reserved as a later scaling holdout.

## Kill conditions

1. any underlying JVP operator is invoked more than `B_k` times;
2. any row records speculative JVP work greater than `B_k`;
3. budget exhaustion mutates committed R-JF trajectory/controller/output;
4. completed speculative work disappears from the ledger;
5. a budget-exhausted row emits zeta34 or performs full-E endpoint audit;
6. frozen `B_abs`, `delta`, `tau`, k=3 gate, BDF/Radau bytes, or Cargo lock change;
7. active switching or N=2048 execution occurs.
